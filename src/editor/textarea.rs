use anyhow::Result;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::text::Text;
use ratatui::widgets::{Paragraph, Widget};
use regex::Regex;

use std::io::BufRead;
use std::{cmp, fs, io};

use crate::editor::cursor::CursorPosition;
use crate::editor::history::HistoryAction;
use crate::editor::indent::Indent;
use crate::editor::viewport::Viewport;
use crate::input::{Input, Key};

#[derive(Debug, Clone)]
pub struct TextArea {
    lines: Vec<String>,
    viewport: Viewport,
    cursor_start: CursorPosition,
    cursor_end: Option<CursorPosition>,

    history: Vec<HistoryAction>,
    clipboard: String,
    search: Option<Regex>,

    indent: Indent,
}

impl Default for TextArea {
    fn default() -> Self {
        Self {
            lines: vec![String::new()],
            viewport: Default::default(),
            cursor_start: Default::default(),
            cursor_end: Default::default(),
            history: Default::default(),
            clipboard: Default::default(),
            search: Default::default(),
            indent: Default::default(),
        }
    }
}

impl TextArea {
    pub fn new_from_file(file: &fs::File) -> Result<Self> {
        let mut file_reader = io::BufReader::new(file);

        let mut buf = String::new();
        let mut lines = Vec::new();
        let mut indent = None;
        let mut ends_in_newline = false;
        loop {
            buf.clear();
            match file_reader.read_line(&mut buf)? {
                0 => break,
                _n => {
                    if indent.is_none() {
                        if buf.starts_with('\t') {
                            indent = Some(Indent::Tabs);
                        } else if buf.starts_with(' ') {
                            let mut spaces = 1;
                            for char in buf.chars().skip(1) {
                                if char == ' ' {
                                    spaces += 1;
                                } else {
                                    break;
                                }
                            }
                            indent = Some(spaces.into());
                        }
                    }

                    ends_in_newline = buf.ends_with('\n');
                    if ends_in_newline {
                        buf.pop();
                        if buf.ends_with('\r') {
                            buf.pop();
                        }
                    }
                    lines.push(buf.clone());
                }
            };
        }

        if ends_in_newline {
            lines.push(String::new());
        }

        Ok(Self {
            lines,
            indent: indent.unwrap_or_default(),
            ..Default::default()
        })
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn cursor(&self) -> CursorPosition {
        self.cursor_start
    }

    pub fn input(&mut self, input: Input) -> bool {
        match input {
            Input {
                key: Key::Up,
                shift,
                alt: false,
                ctrl: false,
            } => {
                self.cursor_start.row = self.cursor_start.row.saturating_sub(1);
                self.viewport.update_view(self.cursor_start);
            }
            Input {
                key: Key::Down,
                shift,
                alt: false,
                ctrl: false,
            } => {
                self.cursor_start.row = self.cursor_start.row.saturating_add(1);
                self.viewport.update_view(self.cursor_start);
            }
            Input {
                key: Key::Left,
                shift,
                alt: false,
                ctrl: false,
            } => {
                self.cursor_start.col = self.cursor_start.col.saturating_sub(1);
                self.viewport.update_view(self.cursor_start);
            }
            Input {
                key: Key::Right,
                shift,
                alt: false,
                ctrl: false,
            } => {
                self.cursor_start.col = self.cursor_start.col.saturating_add(1);
                self.viewport.update_view(self.cursor_start);
            }
            Input {
                key: Key::Home,
                shift,
                alt: false,
                ctrl: false,
            } => {
                self.cursor_start.col = 0;
                self.viewport.update_view(self.cursor_start);
            }
            _ => {}
        }

        true
    }
}

// cursor & selection
impl TextArea {
    pub fn set_cursor_start(&mut self, cursor: CursorPosition) {
        if self.is_cursor_valid(cursor) {
            self.cursor_start = cursor;
        }
        self.viewport.update_view(cursor);
    }

    pub fn set_cursor_end(&mut self, cursor: Option<CursorPosition>) {
        if let Some(cursor) = cursor {
            if self.is_cursor_valid(cursor) {
                self.cursor_end = Some(cursor);
            }
        } else {
            self.cursor_end = None;
        }
    }

    fn is_cursor_valid(&self, CursorPosition { row, col }: CursorPosition) -> bool {
        row < self.lines.len() && col <= self.lines[row].len()
    }

    pub fn get_display_cursor_position(&self) -> Position {
        self.viewport.terminal_cursor_position(self.cursor_start)
    }

    pub fn take_selection(&self) -> Option<&str> {
        if let Some(cursor_end) = self.cursor_end {
            if self.cursor_start.row != cursor_end.row {
                return None;
            }

            Some(Self::get_char_slice(
                &self.lines[self.cursor_start.row],
                self.cursor_start.col,
                cursor_end.col,
            ))
        } else {
            None
        }
    }

    fn get_char_slice(line: &str, col_start: usize, col_end: usize) -> &str {
        let Some(start) = line.char_indices().nth(col_start).map(|(i, _)| i) else { return "" };
        let Some(end) = line.char_indices().nth(col_end).map(|(i, _)| i) else { return &line[start..] };
        &line[start..end]
    }
}

// search
impl TextArea {
    pub fn set_search_pattern(&mut self, pattern: &str) -> Result<()> {
        match &self.search {
            Some(r) if r.as_str() == pattern => {}
            _ if pattern.is_empty() => self.search = None,
            _ => self.search = Some(Regex::new(pattern)?),
        }
        Ok(())
    }

    pub fn search_forward(&self) -> Option<(CursorPosition, CursorPosition)> {
        let search_pattern = self.search.as_ref()?;

        let cursor_line = self.lines.get(self.cursor_start.row)?;
        let lines_after_cursor = self.lines.split_at_checked(self.cursor_start.row + 1)?.1;

        search_pattern
            .find_at(cursor_line, self.cursor_start.col + 1)
            .map(|m| (self.cursor_start.row, m, cursor_line))
            .or_else(|| {
                lines_after_cursor.iter().enumerate().find_map(|(i, line)| {
                    search_pattern
                        .find(line)
                        .map(|m| (self.cursor_start.row + 1 + i, m, line))
                })
            })
            .map(|(row, m, line)| {
                let start_col = line[0..m.start()].chars().count();
                let end_col = start_col + line[m.start()..m.end()].chars().count();
                (
                    CursorPosition { row, col: start_col },
                    CursorPosition { row, col: end_col },
                )
            })
    }

    pub fn search_backward(&self) -> Option<(CursorPosition, CursorPosition)> {
        let search_pattern = self.search.as_ref()?;

        let cursor_line = self
            .lines
            .get(self.cursor_start.row)?
            .split_at_checked(self.cursor_start.col)?
            .0;
        let lines_before_cursor = self.lines.split_at_checked(self.cursor_start.row)?.0;

        search_pattern
            .find_iter(cursor_line)
            .last()
            .map(|m| (self.cursor_start.row, m, cursor_line))
            .or_else(|| {
                lines_before_cursor.iter().rev().enumerate().find_map(|(i, line)| {
                    search_pattern
                        .find_iter(line)
                        .last()
                        .map(|m| (self.cursor_start.row + 1 + i, m, line.as_str()))
                })
            })
            .map(|(row, m, line)| {
                let start_col = line[0..m.start()].chars().count();
                let end_col = start_col + line[m.start()..m.end()].chars().count();
                (
                    CursorPosition { row, col: start_col },
                    CursorPosition { row, col: end_col },
                )
            })
    }
}

// viewport
impl TextArea {
    pub fn update_viewport_size(&mut self, area: Rect) {
        self.viewport.update_size(area.width.into(), area.height.into());
    }
}

impl Widget for &TextArea {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let (top_left, bottom_right) = self.viewport.rect();

        let lines = self.lines[cmp::min(top_left.row, self.lines.len())..cmp::min(bottom_right.row, self.lines.len())]
            .iter()
            .map(|line| TextArea::get_char_slice(line, top_left.col, bottom_right.col))
            .map(|line| line.replace('\t', self.indent.spaces()));

        Paragraph::new(Text::from_iter(lines)).render(area, buf);
    }
}
