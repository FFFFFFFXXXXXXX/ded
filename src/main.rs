use anyhow::Result;

use crossterm::event::Event;
use ratatui::DefaultTerminal;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Paragraph;

use std::borrow::Cow;
use std::io::{self, Write};
use std::path::PathBuf;
use std::{env, fs};

use crate::editor::{CursorPosition, TextArea};
use crate::input::{Input, Key};
use crate::searchbox::SearchBox;

mod editor;
mod input;
mod searchbox;

fn main() -> Result<()> {
    let term = ratatui::init();
    let result = (|| Editor::new(env::args_os().skip(1))?.run(term))();
    ratatui::restore();

    result
}

#[derive(Debug, Default)]
struct Buffer<'a> {
    path: PathBuf,
    textarea: TextArea,
    searchbox: SearchBox<'a>,
    modified: bool,
}

impl<'a> Buffer<'a> {
    fn new(path: PathBuf) -> Result<Self> {
        let textarea = if path.exists() {
            TextArea::new_from_file(&fs::File::open(&path)?)?
        } else {
            TextArea::default()
        };

        Ok(Self {
            textarea,
            path,
            ..Default::default()
        })
    }

    fn save(&mut self) -> Result<()> {
        if !self.modified {
            return Ok(());
        }

        let mut f = io::BufWriter::new(fs::File::create(&self.path)?);

        let lines = self.textarea.lines();
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

#[derive(PartialEq, Eq)]
enum Status {
    Continue,
    Stop,
}

struct Editor<'a> {
    buffers: Vec<Buffer<'a>>,
    current: usize,
    message: Option<Cow<'static, str>>,
}

impl<'a> Editor<'a> {
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

            // wait for next userinput (blocking!)
            let event = crossterm::event::read()?;
            // manually re-render on window resize because Event::Resize(_, _) gets ignored by tui_textarea
            if let Event::Resize(width, height) = event {
                self.render(&mut terminal)?;
            }

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

            buffer.textarea.viewport.update_size(chunks[1].width, chunks[1].height);

            if buffer.searchbox.is_open() {
                f.render_widget(&buffer.searchbox, chunks[0]);
            }

            f.render_widget(&buffer.textarea, chunks[1]);

            // Render status line
            let modified = if buffer.modified { " [modified]" } else { "" };
            let slot = format!("[{}/{}]", self.current + 1, num_buffers);
            let path = format!(" {}{} ", buffer.path.display(), modified);
            let CursorPosition { row, col } = buffer.textarea.cursor();
            let cursor = format!("({},{})", row, col);
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

            f.set_cursor_position(
                buffer
                    .textarea
                    .viewport
                    .terminal_cursor_position(buffer.textarea.cursor()),
            );
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
                if buffer.textarea.search_forward().is_some() {
                    buffer.searchbox.set_error_message(None::<&'static str>);
                } else {
                    buffer.searchbox.set_error_message(Some("not found"));
                }
            }
            Input { key: Key::Up, .. } => {
                if buffer.textarea.search_backward().is_some() {
                    buffer.searchbox.set_error_message(None::<&'static str>);
                } else {
                    buffer.searchbox.set_error_message(Some("not found"));
                }
            }
            Input { key: Key::Enter, .. } => {
                if let Some((cursor_start, cursor_end)) = buffer.textarea.search_forward() {
                    buffer.textarea.set_cursor_start(cursor_start);
                    buffer.textarea.set_cursor_end(Some(cursor_end));
                } else {
                    buffer.searchbox.set_error_message(Some("not found"));
                }
                buffer.searchbox.close();
                buffer.textarea.set_search_pattern("").unwrap();
            }
            Input { key: Key::Esc, .. } => {
                buffer.searchbox.close();
                buffer.textarea.set_search_pattern("").unwrap();
            }
            input => {
                if let Some(query) = buffer.searchbox.input(input) {
                    let maybe_err = buffer.textarea.set_search_pattern(query).err();
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
                ..
            } => {
                let search_pattern = {
                    let prev_search_pattern = buffer.searchbox.open();
                    buffer
                        .textarea
                        .take_selection()
                        .unwrap_or(prev_search_pattern)
                        .to_owned()
                };

                buffer.searchbox.set_pattern(&search_pattern);
                let maybe_err = buffer.textarea.set_search_pattern(&search_pattern).err();
                buffer.searchbox.set_error_message(maybe_err);
            }
            input => {
                let buffer = &mut self.buffers[self.current];
                buffer.modified |= buffer.textarea.input(input);
            }
        }
    }
}
