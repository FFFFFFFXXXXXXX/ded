use std::{cell::Cell, cmp, num::NonZeroU8};

use anyhow::Result;
use arboard::Clipboard;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Widget};
use regex::Regex;
use unicode_width::UnicodeWidthStr;

use super::char_slice::CharSlice;
use super::cursor::CursorPosition;
use super::history::HistoryAction;
use super::indent::Indent;
use super::word::Word;
use crate::input::{Input, Key};
use crate::textarea::{ByteIndex, BytePosition};

#[derive(Default, Debug, Clone)]
struct View {
    position: Cell<CursorPosition>,
    width: Cell<usize>,
    height: Cell<usize>,
}

pub struct TextArea {
    pub lines: Vec<String>,
    cursor: CursorPosition,
    selection: Option<CursorPosition>,
    view: View,

    undo_history: Vec<(HistoryAction, bool)>,
    redo_history: Vec<(HistoryAction, bool)>,

    clipboard: Clipboard,
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

            undo_history: Default::default(),
            redo_history: Default::default(),
            clipboard: Clipboard::new().unwrap(),
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

        let cursor = self.cursor();
        let position = self.view.position.get();

        let slice = self.lines[cursor.row].char_slice(..cursor.col);
        let tabs = slice.chars().filter(|&c| c == '\t').count();
        let tab_width = self.indent.spaces().len();
        let x = slice.width() + tabs * (tab_width - 1);

        self.view.position.set(CursorPosition {
            row: position.row.clamp(cursor.row.saturating_sub(height - 1), cursor.row),
            col: position.col.clamp(
                x.saturating_sub(width - usize::from(num_digits(self.lines.len())) - 1),
                x,
            ),
        });

        let position = self.view.position.get();
        (
            position,
            CursorPosition {
                row: position.row.saturating_add(height),
                col: position.col.saturating_add(width),
            },
        )
    }

    pub fn terminal_cursor_position(&self) -> Position {
        let offset = if self.line_numbers {
            u16::from(num_digits(self.lines.len())) + 1
        } else {
            0
        };

        let position = self.view.position.get();
        let cursor = self.cursor();
        let tab_width = self.indent.spaces().len();

        let col = {
            let slice = self.lines[cursor.row].char_slice(..cursor.col);
            let tabs = slice.chars().filter(|&c| c == '\t').count();
            slice.width() + tabs * (tab_width - 1)
        };

        let line = self.lines[cursor.row].replace("\t", self.indent.spaces());
        let slice = line.as_str().char_slice(position.col..col);

        let tabs = slice.chars().filter(|&c| c == '\t').count();
        let line_width = slice.width() + tabs * (tab_width - 1);

        Position {
            x: offset + u16::try_from(line_width).unwrap(),
            y: u16::try_from(cursor.row - position.row).unwrap(),
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

    pub fn do_action(&mut self, history_action: HistoryAction) -> CursorPosition {
        self.redo_history.clear();

        let cursor = history_action.apply(&mut self.lines);
        self.undo_history.push((history_action, false));
        cursor
    }

    pub fn do_action_chain(&mut self, history_action: HistoryAction) -> CursorPosition {
        self.redo_history.clear();

        let cursor = history_action.apply(&mut self.lines);
        self.undo_history.push((history_action, true));
        cursor
    }

    pub fn undo_action(&mut self) -> Option<CursorPosition> {
        let mut chain;
        loop {
            let (action, next_chain) = self.undo_history.pop()?;
            chain = next_chain;

            let inverse_action = action.invert();
            let cursor = inverse_action.apply(&mut self.lines);
            self.redo_history.push((inverse_action, chain));

            if !chain {
                return Some(cursor);
            }
        }
    }

    pub fn redo_action(&mut self) -> Option<CursorPosition> {
        let mut chain;
        loop {
            let (action, next_chain) = self.redo_history.pop()?;
            chain = next_chain;

            let inverse_action = action.invert();
            let cursor = inverse_action.apply(&mut self.lines);
            self.undo_history.push((inverse_action, chain));

            if !chain {
                return Some(cursor);
            }
        }
    }

    pub fn input(&mut self, input: Input) -> bool {
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
                            col: cursor.col.min(lines[cursor.row - 1].len()),
                        },
                        shift,
                    );
                }
                false
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
                    .skip_while(|(_, line)| line.trim_start().is_empty())
                    .find_map(|(idx, line)| line.trim_start().is_empty().then_some(idx + 1))
                    .unwrap_or(0);
                let col = cursor.col.min(lines[row].len());

                self.set_cursor(CursorPosition { row, col }, shift);
                false
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
                false
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
                            col: cursor.col.min(lines[cursor.row + 1].len()),
                        },
                        shift,
                    );
                }
                false
            }
            Input {
                key: Key::Down,
                shift,
                alt: false,
                ctrl: true,
            } => {
                let lines = &self.lines;
                let cursor = self.cursor();

                let row = if lines[cursor.row].trim_start().is_empty() {
                    lines[cursor.row..]
                        .iter()
                        .enumerate()
                        .skip(1)
                        .find_map(|(idx, line)| (!line.trim_start().is_empty()).then_some(cursor.row + idx))
                        .unwrap_or_else(|| lines.len().saturating_sub(1))
                } else {
                    lines[cursor.row..]
                        .iter()
                        .enumerate()
                        .skip(1)
                        .find_map(|(idx, line)| line.trim_start().is_empty().then_some(cursor.row + idx))
                        .unwrap_or_else(|| lines.len().saturating_sub(1))
                };

                self.set_cursor(
                    CursorPosition {
                        row,
                        col: cursor.col.min(lines[row].len()),
                    },
                    shift,
                );
                false
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
                false
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
                                        col: lines[cursor.row - 1].len(),
                                    },
                                    shift,
                                );
                            }
                        } else {
                            self.set_cursor(CursorPosition { col: cursor.col - 1, ..cursor }, shift);
                        }
                    }
                };
                false
            }
            Input {
                key: Key::Left,
                shift,
                alt: false,
                ctrl: true,
            } => {
                let lines = &self.lines;
                let cursor = self.cursor();

                let cursor = match lines[cursor.row].previous_word(cursor.col) {
                    Some(col) => CursorPosition { col, ..cursor },
                    None if cursor.col > 0 => CursorPosition { col: 0, ..cursor },
                    None if cursor.row > 0 => CursorPosition {
                        row: cursor.row - 1,
                        col: lines[cursor.row - 1].len(),
                    },
                    None => cursor,
                };
                self.set_cursor(cursor, shift);
                false
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
                false
            }
            Input {
                key: Key::Right,
                shift,
                alt: false,
                ctrl: true,
            } => {
                let lines = &self.lines;
                let cursor = self.cursor();

                let cursor = match lines[cursor.row].next_word(cursor.col) {
                    Some(col) => CursorPosition { col, ..cursor },
                    None if cursor.col < lines[cursor.row].len() => CursorPosition {
                        col: lines[cursor.row].len(),
                        ..cursor
                    },
                    None if cursor.row < lines.len() - 1 => CursorPosition { row: cursor.row + 1, col: 0 },
                    None => cursor,
                };

                self.set_cursor(cursor, shift);
                false
            }
            Input {
                key: Key::Home,
                shift,
                alt: false,
                ctrl: false,
            } => {
                let cursor = self.cursor();
                self.set_cursor(CursorPosition { col: 0, ..cursor }, shift);
                false
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
                false
            }
            Input {
                key: Key::PageUp,
                shift,
                alt: false,
                ctrl: false,
            } => {
                let lines = &self.lines;
                let cursor = self.cursor();

                let row = cursor.row.saturating_sub(self.view.height.get());
                self.set_cursor(
                    CursorPosition {
                        row,
                        col: cursor.col.min(lines[row].chars().count()),
                    },
                    shift,
                );
                false
            }
            Input {
                key: Key::PageDown,
                shift,
                alt: false,
                ctrl: false,
            } => {
                let lines = &self.lines;
                let cursor = self.cursor();

                let row = std::cmp::min(lines.len() - 1, cursor.row + self.view.height.get());
                self.set_cursor(
                    CursorPosition {
                        row,
                        col: cursor.col.min(lines[row].chars().count()),
                    },
                    shift,
                );
                false
            }
            Input {
                key: Key::Char('a'),
                ctrl: true,
                alt: false,
                shift: false,
            } => {
                self.set_cursor(
                    CursorPosition {
                        row: self.lines.len() - 1,
                        col: self.lines.last().unwrap().len(),
                    },
                    false,
                );
                self.set_selection(Some(CursorPosition { row: 0, col: 0 }));
                false
            }
            Input {
                key: Key::Char('z'),
                ctrl: true,
                alt: false,
                shift: false,
            } => {
                if let Some(cursor) = self.undo_action() {
                    self.set_cursor(cursor, false);
                    true
                } else {
                    false
                }
            }
            Input {
                key: Key::Char('y'),
                ctrl: true,
                alt: false,
                shift: false,
            } => {
                if let Some(cursor) = self.redo_action() {
                    self.set_cursor(cursor, false);
                    true
                } else {
                    false
                }
            }
            Input {
                key: Key::Char('c'),
                ctrl: true,
                alt: false,
                shift: false,
            } => {
                if let Some(selected_text) = self.selected_text(false) {
                    _ = self.clipboard.set_text(selected_text.join("\n"));
                }
                false
            }
            Input {
                key: Key::Char('x'),
                ctrl: true,
                alt: false,
                shift: false,
            } => {
                if let Some((selection, selected_text)) = self.selection().zip(self.selected_text(false)) {
                    let lines = &self.lines;
                    let cursor = self.cursor();

                    let start = if cursor < selection { cursor } else { selection };

                    _ = self.clipboard.set_text(selected_text.join("\n"));
                    let cursor = self.do_action(HistoryAction::RemoveLines {
                        lines: selected_text,
                        position: BytePosition {
                            row: cursor.row,
                            col: lines[cursor.row].byte_index(cursor.col),
                        },
                        cursor: (cursor, start),
                    });
                    self.set_cursor(cursor, false);

                    true
                } else {
                    false
                }
            }
            Input {
                key: Key::Char('v'),
                ctrl: true,
                alt: false,
                shift: false,
            } => {
                if let Ok(text) = self.clipboard.get_text() {
                    let text = text.lines().map(|l| l.to_string()).collect::<Vec<_>>();
                    let cursor = self.cursor();

                    let (cursor, chain) = match self.selection().zip(self.selected_text(true)) {
                        Some((selection, selected_text)) => {
                            let start = if cursor < selection { cursor } else { selection };
                            (
                                self.do_action(HistoryAction::RemoveLines {
                                    lines: selected_text,
                                    position: BytePosition::from_line(start, &self.lines[start.row]),
                                    cursor: (cursor, start),
                                }),
                                true,
                            )
                        }
                        None => (cursor, false),
                    };

                    let cursor_after = if text.len() > 1 {
                        CursorPosition {
                            row: cursor.row + text.len() - 1,
                            col: text.last().unwrap().chars().count(),
                        }
                    } else {
                        CursorPosition {
                            col: cursor.col + text[0].len(),
                            ..cursor
                        }
                    };

                    let cursor = if chain {
                        self.do_action_chain(HistoryAction::InsertLines {
                            lines: text,
                            position: BytePosition {
                                row: cursor.row,
                                col: self.lines[cursor.row].byte_index(cursor.col),
                            },
                            cursor: (cursor, cursor_after),
                        })
                    } else {
                        self.do_action(HistoryAction::InsertLines {
                            lines: text,
                            position: BytePosition {
                                row: cursor.row,
                                col: self.lines[cursor.row].byte_index(cursor.col),
                            },
                            cursor: (cursor, cursor_after),
                        })
                    };
                    self.set_cursor(cursor, false);

                    true
                } else {
                    false
                }
            }
            Input { key: Key::Char(char), .. } => {
                let cursor = self.cursor();
                let selection = self.selection();

                match self.selected_text(true).zip(selection) {
                    Some((selected_text, selection)) => {
                        let start = if cursor < selection { cursor } else { selection };

                        let cursor = self.do_action(HistoryAction::RemoveLines {
                            lines: selected_text,
                            position: BytePosition::from_line(start, &self.lines[start.row]),
                            cursor: (cursor, start),
                        });

                        let cursor = self.do_action_chain(HistoryAction::InsertChar {
                            char,
                            position: BytePosition::from_line(cursor, &self.lines[cursor.row]),
                            cursor: (cursor, CursorPosition { col: cursor.col + 1, ..cursor }),
                        });
                        self.set_cursor(cursor, false);
                    }
                    None => {
                        let cursor = self.do_action(HistoryAction::InsertChar {
                            char,
                            position: BytePosition::from_line(cursor, &self.lines[cursor.row]),
                            cursor: (cursor, CursorPosition { col: cursor.col + 1, ..cursor }),
                        });
                        self.set_cursor(cursor, false);
                    }
                }

                true
            }
            Input {
                key: Key::Backspace,
                alt: false,
                ctrl,
                ..
            } => {
                let cursor = self.cursor();
                let selection = self.selection();
                let selected_text = self.selected_text(true);

                let lines = &self.lines;

                if let Some((selected_text, selection)) = selected_text.zip(selection) {
                    let start = if cursor < selection { cursor } else { selection };

                    let cursor = self.do_action(HistoryAction::RemoveLines {
                        lines: selected_text,
                        position: BytePosition::from_line(start, &lines[start.row]),
                        cursor: (cursor, start),
                    });
                    self.set_cursor(cursor, false);

                    true
                } else if ctrl {
                    let action = match lines[cursor.row].previous_word(cursor.col) {
                        Some(col) => Some(HistoryAction::RemoveLines {
                            lines: vec![lines[cursor.row].char_slice(col..cursor.col).to_string()],
                            position: BytePosition {
                                row: cursor.row,
                                col: lines[cursor.row].byte_index(col),
                            },
                            cursor: (cursor, CursorPosition { col, ..cursor }),
                        }),
                        None if cursor.col > 0 => Some(HistoryAction::RemoveLines {
                            lines: vec![lines[cursor.row].char_slice(0..cursor.col).to_string()],
                            position: BytePosition { row: cursor.row, col: 0 },
                            cursor: (cursor, CursorPosition { col: 0, ..cursor }),
                        }),
                        None if cursor.row > 0 => Some(HistoryAction::RemoveLinebreak {
                            position: BytePosition {
                                row: cursor.row - 1,
                                col: lines[cursor.row - 1].len(),
                            },
                            cursor: (
                                cursor,
                                CursorPosition {
                                    row: cursor.row - 1,
                                    col: lines[cursor.row - 1].chars().count(),
                                },
                            ),
                        }),
                        None => None,
                    };

                    if let Some(action) = action {
                        let cursor = self.do_action(action);
                        self.set_cursor(cursor, false);
                    }

                    true
                } else {
                    match cursor {
                        CursorPosition { row: 0, col: 0 } => false,
                        CursorPosition { col: 0, .. } => {
                            let cursor = self.do_action(HistoryAction::RemoveLinebreak {
                                position: BytePosition {
                                    row: cursor.row - 1,
                                    col: lines[cursor.row - 1].len(),
                                },
                                cursor: (
                                    cursor,
                                    CursorPosition {
                                        row: cursor.row - 1,
                                        col: lines[cursor.row - 1].chars().count(),
                                    },
                                ),
                            });
                            self.set_cursor(cursor, false);
                            true
                        }
                        _ => {
                            let cursor = self.do_action(HistoryAction::RemoveChar {
                                char: self.lines[cursor.row].chars().nth(cursor.col - 1).unwrap(),
                                position: BytePosition {
                                    row: cursor.row,
                                    col: lines[cursor.row].byte_index(cursor.col - 1),
                                },
                                cursor: (
                                    cursor,
                                    CursorPosition {
                                        row: cursor.row,
                                        col: cursor.col - 1,
                                    },
                                ),
                            });
                            self.set_cursor(cursor, false);
                            true
                        }
                    }
                }
            }
            Input {
                key: Key::Delete,
                alt: false,
                ctrl,
                ..
            } => {
                let cursor = self.cursor();
                let selection = self.selection();
                let selected_text = self.selected_text(true);

                let lines = &self.lines;

                if let Some((selected_text, selection)) = selected_text.zip(selection) {
                    let start = if cursor < selection { cursor } else { selection };

                    let cursor = self.do_action(HistoryAction::RemoveLines {
                        lines: selected_text,
                        position: BytePosition::from_line(start, &lines[start.row]),
                        cursor: (cursor, start),
                    });
                    self.set_cursor(cursor, false);

                    true
                } else if ctrl {
                    let action = match lines[cursor.row].next_word(cursor.col) {
                        Some(col) => Some(HistoryAction::RemoveLines {
                            lines: vec![lines[cursor.row].char_slice(cursor.col..col).to_string()],
                            position: BytePosition {
                                row: cursor.row,
                                col: lines[cursor.row].byte_index(cursor.col),
                            },
                            cursor: (cursor, cursor),
                        }),
                        None if cursor.col < lines[cursor.row].len() => Some(HistoryAction::RemoveLines {
                            lines: vec![lines[cursor.row].char_slice(cursor.col..).to_string()],
                            position: BytePosition {
                                row: cursor.row,
                                col: lines[cursor.row].byte_index(cursor.col),
                            },
                            cursor: (cursor, cursor),
                        }),
                        None if cursor.row < lines.len() - 1 => Some(HistoryAction::RemoveLinebreak {
                            position: BytePosition {
                                row: cursor.row,
                                col: lines[cursor.row].byte_index(cursor.col),
                            },
                            cursor: (cursor, cursor),
                        }),
                        None => None,
                    };

                    if let Some(action) = action {
                        let cursor = self.do_action(action);
                        self.set_cursor(cursor, false);
                    }

                    true
                } else {
                    match cursor {
                        CursorPosition { row, col } if row == lines.len() - 1 && col == lines.last().unwrap().len() => {
                            false
                        }
                        CursorPosition { col, .. } if col == lines[cursor.row].len() => {
                            let cursor = self.do_action(HistoryAction::RemoveLinebreak {
                                position: BytePosition {
                                    row: cursor.row,
                                    col: lines[cursor.row].len(),
                                },
                                cursor: (cursor, cursor),
                            });
                            self.set_cursor(cursor, false);
                            true
                        }
                        _ => {
                            let cursor = self.do_action(HistoryAction::RemoveChar {
                                char: self.lines[cursor.row].chars().nth(cursor.col).unwrap(),
                                position: BytePosition {
                                    row: cursor.row,
                                    col: lines[cursor.row].byte_index(cursor.col),
                                },
                                cursor: (cursor, cursor),
                            });
                            self.set_cursor(cursor, false);
                            true
                        }
                    }
                }
            }

            _ => false,
        }
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
                    line.chars().count()
                };

                let tabs_before_selection = self.lines[line_info.line_number]
                    .char_slice(..start)
                    .chars()
                    .filter(|&c| c == '\t')
                    .count();
                let tabs_in_selection = self.lines[line_info.line_number]
                    .char_slice(start..end)
                    .chars()
                    .filter(|&c| c == '\t')
                    .count();
                let tab_width = self.indent.spaces().len();

                Some((
                    start + (tabs_before_selection * (tab_width - 1)),
                    end + ((tabs_before_selection + tabs_in_selection) * (tab_width - 1)),
                ))
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
                    line.chars().count()
                };

                let tabs_before_selection = self.lines[line_info.line_number]
                    .char_slice(..start)
                    .chars()
                    .filter(|&c| c == '\t')
                    .count();
                let tabs_in_selection = self.lines[line_info.line_number]
                    .char_slice(start..end)
                    .chars()
                    .filter(|&c| c == '\t')
                    .count();
                let tab_width = self.indent.spaces().len();

                Some((
                    start + (tabs_before_selection * (tab_width - 1)),
                    end + ((tabs_before_selection + tabs_in_selection) * (tab_width - 1)),
                ))
            } else {
                None
            };

            match selected_range {
                Some((start, end)) if start == 0 && end == 0 && line.is_empty() => {
                    return Line::from_iter([Span::from(line_info), Span::from(" ").style(SELECT)]);
                }
                Some((start, end)) => {
                    return match &self.search_pattern {
                        Some(pattern) => {
                            let mut spans = Vec::new();
                            spans.push(Span::from(line_info));

                            Self::mark_matches(&mut spans, line.char_slice(..start), pattern);
                            spans.push(Span::from(line.char_slice(start..end)).style(SELECT));
                            Self::mark_matches(&mut spans, line.char_slice(end..), pattern);

                            Line::from(spans)
                        }
                        None => Line::from_iter([
                            Span::from(line_info),
                            Span::from(line.char_slice(..start)),
                            Span::from(line.char_slice(start..end)).style(SELECT),
                            Span::from(line.char_slice(end..)),
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

    pub fn selected_text(&mut self, unselect: bool) -> Option<Vec<String>> {
        let selection = self.selection()?;
        if unselect {
            self.set_selection(None);
        }

        let lines = &self.lines;
        let cursor = self.cursor();

        let (start, end) = if cursor < selection {
            (cursor, selection)
        } else {
            (selection, cursor)
        };

        if start.row == end.row {
            return Some(vec![lines[start.row].char_slice(start.col..end.col).to_string()]);
        }

        let mut text = Vec::with_capacity(end.row - start.row + 1);
        text.push(lines[start.row].char_slice(start.col..).to_string());
        lines[start.row + 1..end.row]
            .iter()
            .for_each(|line| text.push(line.to_string()));
        text.push(lines[end.row].char_slice(..end.col).to_string());

        Some(text)
    }

    pub fn selected_text_single_line(&self) -> Option<&str> {
        let lines = &self.lines;
        let cursor = self.cursor();
        let selection = self.selection();

        if let Some(selection) = selection {
            if cursor.row != selection.row {
                return None;
            }

            if selection < cursor {
                Some(lines[cursor.row].char_slice(selection.col..cursor.col))
            } else {
                Some(lines[cursor.row].char_slice(cursor.col..selection.col))
            }
        } else {
            None
        }
    }
}

impl Widget for &TextArea {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let (top_left, bottom_right) = self.update_size(usize::from(area.width), area.height.into());

        let start = cmp::min(top_left.row, self.lines.len());
        let end = cmp::min(bottom_right.row, self.lines.len());

        let lines = self.lines[start..end]
            .iter()
            .map(|line| {
                let trimmed = line.trim_end();
                let tabs = line[trimmed.len()..].chars().filter(|&c| c == '\t').count();
                let tab_width = self.indent.spaces().len();

                String::from_iter([
                    &trimmed.replace('\t', self.indent.spaces()),
                    dots(
                        (line.chars().count() - trimmed.chars().count() + (tabs * (tab_width - 1)))
                            .try_into()
                            .unwrap(),
                    ),
                ])
            })
            .collect::<Vec<_>>();

        let line_number_len: Option<NonZeroU8> = if self.line_numbers {
            num_digits(self.lines.len()).try_into().ok()
        } else {
            None
        };

        let lines = lines.iter().zip(start..end).map(|(line, line_number)| {
            self.render_line(
                line.char_slice(top_left.col..bottom_right.col),
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
    const SPACES: &str = "                                                                                                                                                                                                                                                                ";
    &SPACES[..size.into()]
}

pub fn dots(size: u8) -> &'static str {
    const DOTS: &str = "································································································································································································································································";
    &DOTS[..('·'.len_utf8() * usize::from(size))]
}
