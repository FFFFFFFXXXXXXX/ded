use anyhow::Result;

use crossterm::event::Event;
use ratatui::DefaultTerminal;
use ratatui::layout::{Constraint, Direction, Layout, Position};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Paragraph;

use std::borrow::Cow;
use std::io::{self, Write};
use std::path::PathBuf;
use std::{env, fs};

use crate::editor::Editor;
use crate::input::{Input, Key};
use crate::searchbox::SearchBox;

mod editor;
mod input;
mod searchbox;
mod textarea;

fn main() -> Result<()> {
    let term = ratatui::init();
    let result = (|| App::new(env::args_os().skip(1))?.run(term))();
    ratatui::restore();

    result
}

struct App<'a> {
    buffers: Vec<Buffer<'a>>,
    current: usize,
    message: Option<Cow<'static, str>>,
}

impl<'a> App<'a> {
    fn new<I>(paths: I) -> Result<Self>
    where
        I: Iterator,
        I::Item: Into<PathBuf>,
    {
        let buffers = paths.map(|p| Buffer::new(p.into())).collect::<Result<Vec<_>>>()?;
        if buffers.is_empty() {
            anyhow::bail!("USAGE: ded FILE1 [FILE2...]");
        }

        Ok(Self {
            buffers,
            current: 0,
            message: None,
        })
    }

    fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        loop {
            // render state to terminal
            self.render(&mut terminal)?;

            match crossterm::event::read()? {
                Event::Resize(_, _) => self.render(&mut terminal)?,
                Event::Key(event) => {
                    let event = event.into();
                    // ignore Key::Null so we don't rerender unnecessarily
                    if let Input { key: Key::Null, .. } = event {
                        continue;
                    }

                    // process input / change state
                    if self.process_input(event)? == Status::Stop {
                        break;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn render(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        terminal.draw(|f| {
            let num_buffers = self.buffers.len();
            let buffer = &mut self.buffers[self.current];

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(if buffer.searchbox.is_open() { 3 } else { 0 }),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(f.area());

            if buffer.searchbox.is_open() {
                f.render_widget(&buffer.searchbox, chunks[0]);
            }

            f.render_widget(
                &buffer.editor.textarea,
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Min(1), Constraint::Length(1)])
                    .split(chunks[1])[0],
            );

            // Render status line
            let modified = if buffer.modified { " [modified]" } else { "" };
            let slot = format!("[{}/{}]", self.current + 1, num_buffers);
            let path = format!(" {}{} ", buffer.path.display(), modified);
            let cursor = buffer.editor.textarea.cursor();
            let cursor = match buffer.editor.textarea.selection() {
                Some(selection) => format!(
                    "({},{}) - ({},{})",
                    selection.row, selection.col, cursor.row, cursor.col
                ),
                None => format!("({},{})", cursor.row, cursor.col),
            };
            let status_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Length(slot.len().try_into().unwrap()),
                        Constraint::Min(1),
                        Constraint::Length(cursor.len().try_into().unwrap()),
                    ]
                    .as_ref(),
                )
                .split(chunks[2]);
            let status_style = Style::default().add_modifier(Modifier::REVERSED);
            f.render_widget(Paragraph::new(slot).style(status_style), status_chunks[0]);
            f.render_widget(Paragraph::new(path).style(status_style), status_chunks[1]);
            f.render_widget(Paragraph::new(cursor).style(status_style), status_chunks[2]);

            if buffer.searchbox.is_open() {
                f.set_cursor_position(Position::new(buffer.searchbox.textarea.terminal_cursor_position().x, 1));
            } else {
                f.set_cursor_position(buffer.editor.textarea.terminal_cursor_position());
            }
        })?;

        Ok(())
    }

    fn process_input(&mut self, event: Input) -> Result<Status> {
        let buffer = &mut self.buffers[self.current];

        match event {
            Input {
                key: Key::Char('q'),
                ctrl: true,
                alt: false,
                shift: false,
            } => return Ok(Status::Stop),
            Input {
                key: Key::Char(char),
                alt: true,
                ctrl: false,
                shift: false,
            } if char.is_ascii_digit() => {
                let buf_idx = char.to_digit(10).unwrap().saturating_sub(1).try_into().unwrap();
                if buf_idx < self.buffers.len() {
                    self.current = buf_idx;
                }
            }
            Input {
                key: Key::Char('s'),
                ctrl: true,
                ..
            } => {
                buffer.save()?;
                self.message = Some("Saved!".into());
            }
            event => {
                if buffer.searchbox.is_open() {
                    self.process_searchbox_input(event);
                } else {
                    self.process_textarea_input(event);
                }
            }
        };

        Ok(Status::Continue)
    }

    fn process_searchbox_input(&mut self, event: Input) {
        let buffer = &mut self.buffers[self.current];

        match event {
            Input { key: Key::Down, .. } => {
                if !buffer.searchbox.textarea.lines[0].is_empty() {
                    if let Some((cursor, selection)) = buffer.editor.textarea.search_forward() {
                        buffer.searchbox.set_error_message(None::<&str>);
                        buffer.editor.textarea.set_cursor(cursor, false);
                        buffer.editor.textarea.set_selection(Some(selection));
                    } else {
                        buffer.searchbox.set_error_message(Some("not found"));
                    }
                }
            }
            Input { key: Key::Up, .. } => {
                if !buffer.searchbox.textarea.lines[0].is_empty() {
                    if let Some((cursor, selection)) = buffer.editor.textarea.search_backward() {
                        buffer.searchbox.set_error_message(None::<&str>);
                        buffer.editor.textarea.set_cursor(cursor, false);
                        buffer.editor.textarea.set_selection(Some(selection));
                    } else {
                        buffer.searchbox.set_error_message(Some("not found"));
                    }
                }
            }
            Input { key: Key::Enter, .. } => {
                if !buffer.searchbox.textarea.lines[0].is_empty() && buffer.editor.textarea.selection().is_none() {
                    if let Some((cursor_start, cursor_end)) = buffer.editor.textarea.search_forward() {
                        buffer.editor.textarea.set_cursor(cursor_start, false);
                        buffer.editor.textarea.set_selection(Some(cursor_end));
                    } else {
                        buffer.searchbox.set_error_message(Some("not found"));
                    }
                }

                buffer.searchbox.close();
                buffer.editor.textarea.set_search_pattern("").unwrap();
            }
            Input { key: Key::Esc, .. } => {
                buffer.searchbox.close();
                buffer.editor.textarea.set_search_pattern("").unwrap();
            }
            input => {
                if let Some(query) = buffer.searchbox.input(input) {
                    let maybe_err = buffer.editor.textarea.set_search_pattern(query).err();
                    buffer.searchbox.set_error_message(maybe_err);
                }
            }
        }
    }

    fn process_textarea_input(&mut self, event: Input) {
        let buffer = &mut self.buffers[self.current];

        match event {
            Input {
                key: Key::Char('f'),
                ctrl: true,
                alt: false,
                shift: false,
            } => {
                let search_pattern = {
                    let prev_search_pattern = buffer.searchbox.open();
                    buffer
                        .editor
                        .textarea
                        .selected_text_single_line()
                        .unwrap_or(prev_search_pattern)
                        .to_owned()
                };

                buffer.searchbox.set_text(&search_pattern);
                let maybe_err = buffer.editor.textarea.set_search_pattern(&search_pattern).err();
                buffer.searchbox.set_error_message(maybe_err);
            }
            input => {
                let buffer = &mut self.buffers[self.current];
                buffer.modified |= buffer.editor.input(input);
            }
        }
    }
}

#[derive(PartialEq, Eq)]
enum Status {
    Continue,
    Stop,
}

#[derive(Default)]
struct Buffer<'a> {
    path: PathBuf,
    searchbox: SearchBox<'a>,
    editor: Editor,
    modified: bool,
}

impl<'a> Buffer<'a> {
    fn new(path: PathBuf) -> Result<Self> {
        let textarea = if path.exists() {
            Editor::new_from_file(&fs::File::open(&path)?)?
        } else {
            Editor::default()
        };

        Ok(Self {
            editor: textarea,
            path,
            ..Default::default()
        })
    }

    fn save(&mut self) -> Result<()> {
        if !self.modified {
            return Ok(());
        }

        let mut f = io::BufWriter::new(fs::File::create(&self.path)?);

        let lines = &self.editor.textarea.lines;
        for line in lines.iter().take(lines.len() - 1) {
            f.write_all(line.as_bytes())?;
            f.write_all(b"\n")?;
        }

        if let Some(last_line) = lines.last() {
            f.write_all(last_line.as_bytes())?;
            if !last_line.is_empty() {
                f.write_all(b"\n")?;
            }
        }

        self.modified = false;
        Ok(())
    }
}
