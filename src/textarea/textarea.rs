use anyhow::Result;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Widget};
use regex::Regex;

use std::io::BufRead;
use std::{cmp, fs, io};

use super::cursor::CursorPosition;
use super::history::HistoryAction;
use super::indent::Indent;
use super::viewport::Viewport;
use crate::input::{Input, Key};

#[derive(Debug, Clone)]
pub struct TextArea {
    lines: Vec<String>,
    viewport: Viewport,
    cursor: CursorPosition,
    pub selection: Option<CursorPosition>,

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
            cursor: Default::default(),
            selection: Default::default(),
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
        self.cursor
    }

    pub fn input(&mut self, input: Input) -> bool {
        match input {
            Input {
                key: Key::Up,
                shift,
                alt: false,
                ctrl: false,
            } => {
                if self.cursor.row > 0 {
                    self.handle_selection(shift);
                    self.cursor.row = self.cursor.row - 1;
                    self.cursor.col = self.cursor.col.min(self.lines[self.cursor.row].len());
                    self.viewport.update_view(self.cursor);
                }
            }
            Input {
                key: Key::Down,
                shift,
                alt: false,
                ctrl: false,
            } => {
                if self.cursor.row < self.lines.len() - 1 {
                    self.handle_selection(shift);
                    self.cursor.row = self.cursor.row + 1;
                    self.cursor.col = self.cursor.col.min(self.lines[self.cursor.row].len());
                    self.viewport.update_view(self.cursor);
                }
            }
            Input {
                key: Key::Left,
                shift,
                alt: false,
                ctrl: false,
            } => match self.selection {
                Some(selection) if !shift => {
                    if self.cursor > selection {
                        self.cursor = selection;
                    }
                    self.handle_selection(shift);
                }
                _ => {
                    if self.cursor.col == 0 {
                        if self.cursor.row > 0 {
                            self.handle_selection(shift);
                            self.cursor.row = self.cursor.row - 1;
                            self.cursor.col = self.lines[self.cursor.row].len();
                            self.viewport.update_view(self.cursor);
                        }
                    } else {
                        self.handle_selection(shift);
                        self.cursor.col = self.cursor.col - 1;
                        self.viewport.update_view(self.cursor);
                    }
                }
            },
            Input {
                key: Key::Right,
                shift,
                alt: false,
                ctrl: false,
            } => match self.selection {
                Some(selection) if !shift => {
                    if self.cursor < selection {
                        self.cursor = selection;
                    }
                    self.handle_selection(shift);
                }
                _ => {
                    if self.cursor.col == self.lines[self.cursor.row].len() {
                        if self.cursor.row < self.lines.len() - 1 {
                            self.handle_selection(shift);
                            self.cursor.row = self.cursor.row + 1;
                            self.cursor.col = 0;
                            self.viewport.update_view(self.cursor);
                        }
                    } else {
                        self.handle_selection(shift);
                        self.cursor.col = self.cursor.col + 1;
                        self.viewport.update_view(self.cursor);
                    }
                }
            },
            Input {
                key: Key::Home,
                shift,
                alt: false,
                ctrl: false,
            } => {
                self.handle_selection(shift);
                self.cursor.col = 0;
                self.viewport.update_view(self.cursor);
            }
            Input {
                key: Key::End,
                shift,
                alt: false,
                ctrl: false,
            } => {
                self.handle_selection(shift);
                self.cursor.col = self.lines[self.cursor.row].chars().count();
                self.viewport.update_view(self.cursor);
            }
            _ => {}
        }

        true
    }
}

// cursor & selection
impl TextArea {
    fn handle_selection(&mut self, shift: bool) {
        match self.selection {
            Some(_) if !shift => self.selection = None,
            None if shift => self.selection = Some(self.cursor),
            _ => {}
        }
    }

    pub fn set_cursor_start(&mut self, cursor: CursorPosition) {
        if self.is_cursor_valid(cursor) {
            self.cursor = cursor;
        }
        self.viewport.update_view(cursor);
    }

    pub fn set_cursor_end(&mut self, cursor: Option<CursorPosition>) {
        if let Some(cursor) = cursor {
            if self.is_cursor_valid(cursor) {
                self.selection = Some(cursor);
            }
        } else {
            self.selection = None;
        }
    }

    fn is_cursor_valid(&self, CursorPosition { row, col }: CursorPosition) -> bool {
        row < self.lines.len() && col <= self.lines[row].len()
    }

    pub fn get_display_cursor_position(&self) -> Position {
        let mut cursor_position = self.viewport.terminal_cursor_position(self.cursor);
        cursor_position.x += u16::from(num_digits(self.lines.len())) + 1;
        cursor_position
    }

    pub fn take_selection(&self) -> Option<&str> {
        if let Some(cursor_end) = self.selection {
            if self.cursor.row != cursor_end.row {
                return None;
            }

            Some(self.lines[self.cursor.row].get_char_slice(self.cursor.col, cursor_end.col))
        } else {
            None
        }
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

        let cursor_line = self.lines.get(self.cursor.row)?;
        let lines_after_cursor = self.lines.split_at_checked(self.cursor.row + 1)?.1;

        search_pattern
            .find_at(cursor_line, self.cursor.col + 1)
            .map(|m| (self.cursor.row, m, cursor_line))
            .or_else(|| {
                lines_after_cursor
                    .iter()
                    .enumerate()
                    .find_map(|(i, line)| search_pattern.find(line).map(|m| (self.cursor.row + 1 + i, m, line)))
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

        let cursor_line = self.lines.get(self.cursor.row)?.split_at_checked(self.cursor.col)?.0;
        let lines_before_cursor = self.lines.split_at_checked(self.cursor.row)?.0;

        search_pattern
            .find_iter(cursor_line)
            .last()
            .map(|m| (self.cursor.row, m, cursor_line))
            .or_else(|| {
                lines_before_cursor.iter().rev().enumerate().find_map(|(i, line)| {
                    search_pattern
                        .find_iter(line)
                        .last()
                        .map(|m| (self.cursor.row + 1 + i, m, line.as_str()))
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

// render Widget
impl TextArea {
    fn render_line<'l>(&self, line: &'l str, line_info: LineNumber) -> Line<'l> {
        const SELECT: Style = Style::new().bg(Color::LightBlue);

        if let Some(selection) = self.selection {
            let selected_range = if self.cursor < selection
                && self.cursor.row <= line_info.line_number
                && line_info.line_number <= selection.row
            {
                let start = if self.cursor.row == line_info.line_number {
                    self.cursor.col
                } else {
                    0
                };

                let end = if selection.row == line_info.line_number {
                    selection.col
                } else {
                    self.lines[line_info.line_number].len()
                };

                Some((start, end))
            } else if selection < self.cursor
                && selection.row <= line_info.line_number
                && line_info.line_number <= self.cursor.row
            {
                let start = if selection.row == line_info.line_number {
                    selection.col
                } else {
                    0
                };

                let end = if self.cursor.row == line_info.line_number {
                    self.cursor.col
                } else {
                    self.lines[line_info.line_number].len()
                };

                Some((start, end))
            } else {
                None
            };

            if let Some((start, end)) = selected_range {
                return Line::from_iter([
                    Span::from(line_info),
                    Span::from(line.get_char_slice(0, start)),
                    Span::from(line.get_char_slice(start, end)).style(SELECT),
                    Span::from(line.get_char_slice(end, line.len())),
                ]);
            }
        }

        Line::from_iter([Span::from(line_info), Span::from(line)])
    }
}

impl Widget for &TextArea {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let (top_left, bottom_right) = self.viewport.update_size(area.width.into(), area.height.into());

        let start = cmp::min(top_left.row, self.lines.len());
        let end = cmp::min(bottom_right.row, self.lines.len());

        let line_number_len = num_digits(self.lines.len());

        let lines = self.lines[start..end]
            .iter()
            .map(|line| line.get_char_slice(top_left.col, bottom_right.col - usize::from(line_number_len)))
            .map(|line| line.replace('\t', self.indent.spaces()))
            .collect::<Vec<_>>();

        let lines = lines.iter().zip(start..end).map(|(line, line_number)| {
            self.render_line(
                line,
                LineNumber {
                    line_number,
                    line_number_len,
                    current_line: line_number == self.cursor().row,
                },
            )
        });

        Paragraph::new(Text::from_iter(lines)).render(area, buf);
    }
}

struct LineNumber {
    line_number: usize,
    line_number_len: u8,
    current_line: bool,
}

impl From<LineNumber> for Span<'_> {
    fn from(value: LineNumber) -> Self {
        const LINE_NUMBER_STYLE_SELECTED: Style = Style::new().fg(Color::DarkGray);
        const LINE_NUMBER_STYLE: Style = LINE_NUMBER_STYLE_SELECTED.add_modifier(Modifier::DIM);

        Span::styled(
            format!(
                "{}{} ",
                spaces(value.line_number_len - num_digits(value.line_number)),
                value.line_number
            ),
            if value.current_line {
                LINE_NUMBER_STYLE_SELECTED
            } else {
                LINE_NUMBER_STYLE
            },
        )
    }
}

pub fn num_digits(i: usize) -> u8 {
    const { assert!(usize::ilog10(usize::MAX) <= (u8::MAX as u32)) }

    if i == 0 {
        return 1;
    }

    (usize::ilog10(i) + 1) as u8
}

pub fn spaces(size: u8) -> &'static str {
    const SPACES: &str = "                                                                                                                                                                                                                                                                ";
    &SPACES[..size.into()]
}

trait CharSlice<'a> {
    fn get_char_slice(&'a self, col_start: usize, col_end: usize) -> &'a str;
}

impl<'a> CharSlice<'a> for str {
    fn get_char_slice(&'a self, col_start: usize, col_end: usize) -> &'a str {
        let Some(start) = self.char_indices().nth(col_start).map(|(i, _)| i) else { return "" };
        let Some(end) = self.char_indices().nth(col_end).map(|(i, _)| i) else { return &self[start..] };
        &self[start..end]
    }
}
