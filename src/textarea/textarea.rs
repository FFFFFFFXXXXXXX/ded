use std::{cell::Cell, cmp, num::NonZeroU8};

use anyhow::Result;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Widget};
use regex::Regex;

use super::char_slice::CharSlice;
use super::cursor::CursorPosition;
use super::history::HistoryAction;
use super::indent::Indent;
use super::word::Word;
use crate::input::{Input, Key};

#[derive(Default, Debug, Clone)]
struct View {
    position: Cell<CursorPosition>,
    width: Cell<usize>,
    height: Cell<usize>,
}

#[derive(Debug, Clone)]
pub struct TextArea {
    pub lines: Vec<String>,
    cursor: CursorPosition,
    selection: Option<CursorPosition>,
    view: View,

    history: Vec<HistoryAction>,
    clipboard: String,
    search_pattern: Option<Regex>,

    pub indent: Indent,
    pub line_numbers: bool,
}

impl Default for TextArea {
    fn default() -> Self {
        Self {
            lines: vec![String::new()],
            cursor: Default::default(),
            selection: Default::default(),
            view: Default::default(),

            history: Default::default(),
            clipboard: Default::default(),
            search_pattern: Default::default(),

            indent: Default::default(),
            line_numbers: true,
        }
    }
}

impl TextArea {
    #[inline(always)]
    pub fn cursor(&self) -> CursorPosition {
        self.cursor
    }

    #[inline(always)]
    pub fn selection(&self) -> Option<CursorPosition> {
        self.selection
    }

    pub fn set_cursor(&mut self, cursor: CursorPosition, shift: bool) {
        match self.selection {
            Some(_) if !shift => self.selection = None,
            None if shift => self.selection = Some(self.cursor),
            _ => {}
        }

        self.cursor = cursor;
    }

    pub fn set_selection(&mut self, selection: Option<CursorPosition>) {
        self.selection = selection;
    }

    pub fn update_size(&self, width: usize, height: usize) -> (CursorPosition, CursorPosition) {
        self.view.width.set(width);
        self.view.height.set(height);

        self.view.position.set(CursorPosition {
            row: self.view.position.get().row.clamp(
                self.cursor.row.saturating_sub(self.view.height.get() - 1),
                self.cursor.row,
            ),
            col: self
                .view
                .position
                .get()
                .col
                .clamp(self.cursor.col.saturating_sub(self.view.width.get()), self.cursor.col),
        });

        (
            self.view.position.get(),
            CursorPosition {
                row: self.view.position.get().row.saturating_add(self.view.height.get()),
                col: self.view.position.get().col.saturating_add(self.view.width.get()),
            },
        )
    }

    pub fn terminal_cursor_position(&self) -> Position {
        let offset = if self.line_numbers {
            u16::from(num_digits(self.lines.len()))
        } else {
            0
        };
        Position {
            x: u16::try_from(self.cursor.col - self.view.position.get().col).unwrap() + offset + 1,
            y: u16::try_from(self.cursor.row - self.view.position.get().row).unwrap(),
        }
    }

    pub fn set_search_pattern(&mut self, pattern: &str) -> Result<()> {
        match &self.search_pattern {
            Some(r) if r.as_str() == pattern => {}
            _ if pattern.is_empty() => self.search_pattern = None,
            _ => self.search_pattern = Some(Regex::new(pattern)?),
        }
        Ok(())
    }

    pub fn search_forward(&self) -> Option<(CursorPosition, CursorPosition)> {
        let search_pattern = self.search_pattern.as_ref()?;

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
        let search_pattern = self.search_pattern.as_ref()?;

        let cursor_line = self
            .lines
            .get(self.cursor.row)?
            .split_at_checked(self.cursor.col.saturating_sub(1))?
            .0;
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
                        .map(|m| (self.cursor.row - i - 1, m, line.as_str()))
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

    pub fn push_history_action(&mut self, history_action: HistoryAction) {
        history_action.apply(&mut self.lines);
        self.history.push(history_action);
    }

    pub fn pop_history_action(&mut self) {
        if let Some(history_action) = self.history.pop() {
            history_action.invert().apply(&mut self.lines);
        }
    }

    pub fn input(&mut self, input: Input) {
        match input {
            Input {
                key: Key::Up,
                shift,
                alt: false,
                ctrl: false,
            } => {
                let lines = &self.lines;
                let cursor = self.cursor();
                if cursor.row > 0 {
                    self.set_cursor(
                        CursorPosition {
                            row: cursor.row - 1,
                            col: cursor.col.min(lines[cursor.row].len()),
                        },
                        shift,
                    );
                }
            }
            Input {
                key: Key::Up,
                shift,
                alt: false,
                ctrl: true,
            } => {
                let lines = &self.lines;
                let cursor = self.cursor();

                let row = lines[..cursor.row]
                    .iter()
                    .enumerate()
                    .rev()
                    .skip_while(|(_, line)| line.is_empty())
                    .skip_while(|(_, line)| !line.is_empty())
                    .next()
                    .map(|(idx, _)| idx + 1)
                    .unwrap_or(0);
                let col = cursor.col.min(lines[cursor.row].len());

                self.set_cursor(CursorPosition { row, col }, shift);
            }
            Input {
                key: Key::Up,
                shift,
                alt: true,
                ctrl: true,
            } => {
                let lines = &self.lines;
                let cursor = self.cursor();
                self.set_cursor(
                    CursorPosition {
                        row: 0,
                        col: cursor.col.min(lines[0].len()),
                    },
                    shift,
                );
            }
            Input {
                key: Key::Down,
                shift,
                alt: false,
                ctrl: false,
            } => {
                let lines = &self.lines;
                let cursor = self.cursor();
                if cursor.row < lines.len() - 1 {
                    self.set_cursor(
                        CursorPosition {
                            row: cursor.row + 1,
                            col: cursor.col.min(lines[cursor.row].len()),
                        },
                        shift,
                    );
                }
            }
            Input {
                key: Key::Down,
                shift,
                alt: false,
                ctrl: true,
            } => {
                let lines = &self.lines;
                let cursor = self.cursor();

                let row = if lines[cursor.row].is_empty() {
                    lines[cursor.row..]
                        .iter()
                        .enumerate()
                        .skip(1)
                        .skip_while(|(_, line)| line.is_empty())
                        .next()
                        .map(|(idx, _)| cursor.row + idx)
                        .unwrap_or_else(|| lines.len().saturating_sub(1))
                } else {
                    lines[cursor.row..]
                        .iter()
                        .enumerate()
                        .skip(1)
                        .skip_while(|(_, line)| !line.is_empty())
                        .next()
                        .map(|(idx, _)| cursor.row + idx)
                        .unwrap_or_else(|| lines.len().saturating_sub(1))
                };

                self.set_cursor(
                    CursorPosition {
                        row,
                        col: cursor.col.min(lines[cursor.row].len()),
                    },
                    shift,
                );
            }
            Input {
                key: Key::Down,
                shift,
                alt: true,
                ctrl: true,
            } => {
                let lines = &self.lines;
                let cursor = self.cursor();
                self.set_cursor(
                    CursorPosition {
                        row: lines.len().saturating_sub(1),
                        col: cursor.col.min(lines[0].len()),
                    },
                    shift,
                );
            }
            Input {
                key: Key::Left,
                shift,
                alt: false,
                ctrl: false,
            } => {
                let lines = &self.lines;
                let cursor = self.cursor();
                let selection = self.selection();

                match selection {
                    Some(selection) if !shift => {
                        if cursor > selection {
                            self.set_cursor(selection, shift);
                        } else {
                            self.set_cursor(cursor, shift);
                        }
                    }
                    _ => {
                        if cursor.col == 0 {
                            if cursor.row > 0 {
                                self.set_cursor(
                                    CursorPosition {
                                        row: cursor.row - 1,
                                        col: lines[cursor.row].len(),
                                    },
                                    shift,
                                );
                            }
                        } else {
                            self.set_cursor(CursorPosition { col: cursor.col - 1, ..cursor }, shift);
                        }
                    }
                };
            }
            Input {
                key: Key::Left,
                shift,
                alt: false,
                ctrl: true,
            } => {
                let lines = &self.lines;
                let cursor = self.cursor();
                self.set_cursor(
                    CursorPosition {
                        col: lines[cursor.row].previous_word(cursor.col).unwrap_or(0),
                        ..cursor
                    },
                    shift,
                );
            }
            Input {
                key: Key::Right,
                shift,
                alt: false,
                ctrl: false,
            } => {
                let lines = &self.lines;
                let cursor = self.cursor();
                let selection = self.selection();

                match selection {
                    Some(selection) if !shift => {
                        if cursor < selection {
                            self.set_cursor(selection, shift);
                        } else {
                            self.set_cursor(cursor, shift);
                        }
                    }
                    _ => {
                        if cursor.col == lines[cursor.row].len() {
                            if cursor.row < lines.len() - 1 {
                                self.set_cursor(CursorPosition { row: cursor.row + 1, col: 0 }, shift);
                            }
                        } else {
                            self.set_cursor(CursorPosition { col: cursor.col + 1, ..cursor }, shift);
                        }
                    }
                };
            }
            Input {
                key: Key::Right,
                shift,
                alt: false,
                ctrl: true,
            } => {
                let lines = &self.lines;
                let cursor = self.cursor();

                let col = lines[cursor.row]
                    .next_word(cursor.col)
                    .unwrap_or_else(|| lines[cursor.row].len());
                self.set_cursor(CursorPosition { col, ..cursor }, shift);
            }
            Input {
                key: Key::Home,
                shift,
                alt: false,
                ctrl: false,
            } => {
                let cursor = self.cursor();
                self.set_cursor(CursorPosition { col: 0, ..cursor }, shift);
            }
            Input {
                key: Key::End,
                shift,
                alt: false,
                ctrl: false,
            } => {
                let lines = &self.lines;
                let cursor = self.cursor();
                self.set_cursor(
                    CursorPosition {
                        col: lines[cursor.row].chars().count(),
                        ..cursor
                    },
                    shift,
                );
            }
            _ => {}
        };
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

            match selected_range {
                Some((start, end)) if start == 0 && end == 0 && line.len() == 0 => {
                    return Line::from_iter([Span::from(line_info), Span::from(" ").style(SELECT)]);
                }
                Some((start, end)) => {
                    return match &self.search_pattern {
                        Some(pattern) => {
                            let mut spans = Vec::new();
                            spans.push(Span::from(line_info));

                            Self::mark_matches(&mut spans, line.get_char_slice(0, start), pattern);
                            spans.push(Span::from(line.get_char_slice(start, end)).style(SELECT));
                            Self::mark_matches(&mut spans, line.get_char_slice(end, line.len()), pattern);

                            Line::from(spans)
                        }
                        None => Line::from_iter([
                            Span::from(line_info),
                            Span::from(line.get_char_slice(0, start)),
                            Span::from(line.get_char_slice(start, end)).style(SELECT),
                            Span::from(line.get_char_slice(end, line.len())),
                        ]),
                    };
                }
                _ => {}
            }
        }

        match &self.search_pattern {
            Some(pattern) => {
                let mut spans = Vec::new();
                spans.push(Span::from(line_info));
                Self::mark_matches(&mut spans, line, pattern);

                Line::from(spans)
            }
            None => Line::from_iter([Span::from(line_info), Span::from(line)]),
        }
    }

    fn mark_matches<'l>(spans: &mut Vec<Span<'l>>, line: &'l str, pattern: &Regex) {
        const FOUND: Style = Style::new().bg(Color::Magenta);

        let mut prev_end = 0;
        for m in pattern.find_iter(line) {
            spans.push(Span::from(&line[prev_end..m.start()]));
            spans.push(Span::from(&line[m.start()..m.end()]).style(FOUND));
            prev_end = m.end();
        }
        spans.push(Span::from(&line[prev_end..]));
    }
}

impl Widget for &TextArea {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let (top_left, bottom_right) = self.update_size(area.width.into(), area.height.into());

        let start = cmp::min(top_left.row, self.lines.len());
        let end = cmp::min(bottom_right.row, self.lines.len());

        let line_number_len = if self.line_numbers {
            num_digits(self.lines.len()).try_into().ok()
        } else {
            None
        };

        let lines = self.lines[start..end]
            .iter()
            .map(|line| {
                line.get_char_slice(
                    top_left.col,
                    bottom_right.col - line_number_len.map(|ln| usize::from(u8::from(ln))).unwrap_or_default(),
                )
            })
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
    line_number_len: Option<NonZeroU8>,
    current_line: bool,
}

impl From<LineNumber> for Span<'static> {
    fn from(value: LineNumber) -> Self {
        const LINE_NUMBER_STYLE_SELECTED: Style = Style::new().fg(Color::DarkGray);
        const LINE_NUMBER_STYLE: Style = LINE_NUMBER_STYLE_SELECTED.add_modifier(Modifier::DIM);

        match value.line_number_len {
            Some(line_number_len) => Span::styled(
                format!(
                    "{}{} ",
                    spaces(u8::from(line_number_len) - num_digits(value.line_number)),
                    value.line_number
                ),
                if value.current_line {
                    LINE_NUMBER_STYLE_SELECTED
                } else {
                    LINE_NUMBER_STYLE
                },
            ),
            None => Span::from(""),
        }
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
    const SPACES: &str = "                                                                                                                                                                                                                                                                      ";
    &SPACES[..size.into()]
}
