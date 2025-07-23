use anyhow::Result;

use crate::input::{Input, Key};
use crate::textarea::{BytePosition, CursorPosition, HistoryAction, Indent, TextArea};

#[derive(Default)]
pub struct Editor {
    pub textarea: TextArea,
}

impl Editor {
    pub fn new_from_file(file: &std::fs::File) -> Result<Self> {
        use std::io::BufRead;

        let mut file_reader = std::io::BufReader::new(file);

        let mut buf = String::new();
        let mut lines = Vec::new();
        let mut indent = None;
        let mut ends_in_newline = false;
        loop {
            buf.clear();
            match file_reader.read_line(&mut buf)? {
                0 => break,
                _ => {
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

        if lines.is_empty() {
            lines.push(String::new());
        }

        let mut textarea = TextArea::default();
        textarea.lines = lines;
        textarea.indent = indent.unwrap_or_default();

        Ok(Self { textarea })
    }

    pub fn input(&mut self, input: Input) -> bool {
        match input {
            Input {
                key: Key::Enter,
                ctrl: false,
                alt: false,
                ..
            } => {
                let lines = &self.textarea.lines;
                let cursor = self.textarea.cursor();

                let cursor = self.textarea.do_action(HistoryAction::InsertLinebreak {
                    position: BytePosition::from_line(cursor, &lines[cursor.row]),
                    cursor: (cursor, CursorPosition { row: cursor.row + 1, col: 0 }),
                });
                self.textarea.set_cursor(cursor, false);

                true
            }
            Input {
                key: Key::Tab,
                ctrl: false,
                alt: false,
                ..
            } => {
                let cursor = self.textarea.cursor();
                let selection = self.textarea.selection();

                match selection {
                    Some(selection) if cursor.row != selection.row => {
                        let selection_range = if cursor < selection {
                            cursor.row + 1..selection.row
                        } else {
                            selection.row + 1..cursor.row
                        };

                        let action = if cursor < selection {
                            match &self.textarea.indent {
                                Indent::Tabs => HistoryAction::InsertChar {
                                    char: '\t',
                                    position: BytePosition { row: cursor.row, col: 0 },
                                    cursor: (cursor, CursorPosition { col: cursor.col + 1, ..cursor }),
                                },
                                Indent::Spaces(spaces) => HistoryAction::InsertLines {
                                    lines: vec![spaces.clone()],
                                    position: BytePosition { row: cursor.row, col: 0 },
                                    cursor: (
                                        cursor,
                                        CursorPosition {
                                            col: cursor.col + spaces.len(),
                                            ..cursor
                                        },
                                    ),
                                },
                            }
                        } else {
                            match &self.textarea.indent {
                                Indent::Tabs => HistoryAction::InsertChar {
                                    char: '\t',
                                    position: BytePosition { row: selection.row, col: 0 },
                                    cursor: (cursor, cursor),
                                },
                                Indent::Spaces(spaces) => HistoryAction::InsertLines {
                                    lines: vec![spaces.clone()],
                                    position: BytePosition { row: selection.row, col: 0 },
                                    cursor: (cursor, cursor),
                                },
                            }
                        };
                        let cursor = self.textarea.do_action(action);
                        self.textarea.set_cursor(cursor, false);

                        let mut cursor = cursor;
                        for row in selection_range {
                            let action = match &self.textarea.indent {
                                Indent::Tabs => HistoryAction::InsertChar {
                                    char: '\t',
                                    position: BytePosition { row, col: 0 },
                                    cursor: (cursor, cursor),
                                },
                                Indent::Spaces(spaces) => HistoryAction::InsertLines {
                                    lines: vec![spaces.clone()],
                                    position: BytePosition { row, col: 0 },
                                    cursor: (cursor, cursor),
                                },
                            };
                            cursor = self.textarea.do_action_chain(action);
                        }

                        let action = if cursor < selection {
                            match &self.textarea.indent {
                                Indent::Tabs => HistoryAction::InsertChar {
                                    char: '\t',
                                    position: BytePosition { row: selection.row, col: 0 },
                                    cursor: (cursor, cursor),
                                },
                                Indent::Spaces(spaces) => HistoryAction::InsertLines {
                                    lines: vec![spaces.clone()],
                                    position: BytePosition { row: selection.row, col: 0 },
                                    cursor: (cursor, cursor),
                                },
                            }
                        } else {
                            match &self.textarea.indent {
                                Indent::Tabs => HistoryAction::InsertChar {
                                    char: '\t',
                                    position: BytePosition { row: cursor.row, col: 0 },
                                    cursor: (cursor, CursorPosition { col: cursor.col + 1, ..cursor }),
                                },
                                Indent::Spaces(spaces) => HistoryAction::InsertLines {
                                    lines: vec![spaces.clone()],
                                    position: BytePosition { row: cursor.row, col: 0 },
                                    cursor: (
                                        cursor,
                                        CursorPosition {
                                            col: cursor.col + spaces.len(),
                                            ..cursor
                                        },
                                    ),
                                },
                            }
                        };
                        let cursor = self.textarea.do_action_chain(action);
                        self.textarea.set_cursor(cursor, false);

                        let selection_increment = match &self.textarea.indent {
                            Indent::Tabs => 1,
                            Indent::Spaces(spaces) => spaces.len(),
                        };
                        self.textarea.set_selection(Some(CursorPosition {
                            col: selection.col + selection_increment,
                            ..selection
                        }));
                    }
                    Some(selection) => {
                        let action = match &self.textarea.indent {
                            Indent::Tabs => HistoryAction::InsertChar {
                                char: '\t',
                                position: BytePosition { row: cursor.row, col: 0 },
                                cursor: (cursor, CursorPosition { col: cursor.col + 1, ..cursor }),
                            },
                            Indent::Spaces(spaces) => HistoryAction::InsertLines {
                                lines: vec![spaces.clone()],
                                position: BytePosition::from_line(cursor, &self.textarea.lines[cursor.row]),
                                cursor: (
                                    cursor,
                                    CursorPosition {
                                        col: cursor.col + spaces.len(),
                                        ..cursor
                                    },
                                ),
                            },
                        };
                        let selection_increment = match &self.textarea.indent {
                            Indent::Tabs => 1,
                            Indent::Spaces(spaces) => spaces.len(),
                        };

                        let cursor = self.textarea.do_action(action);
                        self.textarea.set_cursor(cursor, false);
                        self.textarea.set_selection(Some(CursorPosition {
                            col: selection.col + selection_increment,
                            ..selection
                        }));
                    }
                    None => {
                        let action = match &self.textarea.indent {
                            Indent::Tabs => HistoryAction::InsertChar {
                                char: '\t',
                                position: BytePosition::from_line(cursor, &self.textarea.lines[cursor.row]),
                                cursor: (cursor, CursorPosition { col: cursor.col + 1, ..cursor }),
                            },
                            Indent::Spaces(spaces) => HistoryAction::InsertLines {
                                lines: vec![spaces.clone()],
                                position: BytePosition::from_line(cursor, &self.textarea.lines[cursor.row]),
                                cursor: (
                                    cursor,
                                    CursorPosition {
                                        col: cursor.col + spaces.len(),
                                        ..cursor
                                    },
                                ),
                            },
                        };

                        let cursor = self.textarea.do_action(action);
                        self.textarea.set_cursor(cursor, false);
                    }
                }

                true
            }
            Input {
                key: Key::BackTab,
                ctrl: false,
                alt: false,
                ..
            } => {
                let cursor = self.textarea.cursor();
                let selection = self.textarea.selection();

                match selection {
                    Some(selection) if cursor.row != selection.row => {
                        let selection_range = if cursor < selection {
                            cursor.row + 1..selection.row
                        } else {
                            selection.row + 1..cursor.row
                        };

                        let mut first_action = true;
                        let action = if cursor < selection {
                            match &self.textarea.indent {
                                Indent::Tabs => {
                                    if self.textarea.lines[cursor.row].starts_with('\t') {
                                        Some(HistoryAction::RemoveChar {
                                            char: '\t',
                                            position: BytePosition { row: cursor.row, col: 0 },
                                            cursor: (
                                                cursor,
                                                CursorPosition {
                                                    col: cursor.col.saturating_sub(1),
                                                    ..cursor
                                                },
                                            ),
                                        })
                                    } else {
                                        None
                                    }
                                }
                                Indent::Spaces(spaces) => {
                                    if self.textarea.lines[cursor.row].starts_with('\t')
                                        || self.textarea.lines[cursor.row].starts_with(spaces)
                                    {
                                        Some(HistoryAction::RemoveLines {
                                            lines: vec![spaces.clone()],
                                            position: BytePosition { row: cursor.row, col: 0 },
                                            cursor: (
                                                cursor,
                                                CursorPosition {
                                                    col: cursor.col.saturating_sub(spaces.len()),
                                                    ..cursor
                                                },
                                            ),
                                        })
                                    } else {
                                        None
                                    }
                                }
                            }
                        } else {
                            match &self.textarea.indent {
                                Indent::Tabs => {
                                    if self.textarea.lines[selection.row].starts_with('\t') {
                                        Some(HistoryAction::RemoveChar {
                                            char: '\t',
                                            position: BytePosition { row: selection.row, col: 0 },
                                            cursor: (cursor, cursor),
                                        })
                                    } else {
                                        None
                                    }
                                }
                                Indent::Spaces(spaces) => {
                                    if self.textarea.lines[selection.row].starts_with('\t')
                                        || self.textarea.lines[selection.row].starts_with(spaces)
                                    {
                                        Some(HistoryAction::RemoveLines {
                                            lines: vec![spaces.clone()],
                                            position: BytePosition { row: selection.row, col: 0 },
                                            cursor: (cursor, cursor),
                                        })
                                    } else {
                                        None
                                    }
                                }
                            }
                        };

                        let cursor = match action {
                            Some(action) => {
                                first_action = false;
                                self.textarea.do_action(action)
                            }
                            None => cursor,
                        };

                        let mut cursor = cursor;
                        for row in selection_range {
                            let action = match &self.textarea.indent {
                                Indent::Tabs => {
                                    if self.textarea.lines[row].starts_with('\t') {
                                        Some(HistoryAction::RemoveChar {
                                            char: '\t',
                                            position: BytePosition { row, col: 0 },
                                            cursor: (cursor, cursor),
                                        })
                                    } else {
                                        None
                                    }
                                }
                                Indent::Spaces(spaces) => {
                                    if self.textarea.lines[row].starts_with('\t')
                                        || self.textarea.lines[row].starts_with(spaces)
                                    {
                                        Some(HistoryAction::RemoveLines {
                                            lines: vec![spaces.clone()],
                                            position: BytePosition { row, col: 0 },
                                            cursor: (cursor, cursor),
                                        })
                                    } else {
                                        None
                                    }
                                }
                            };

                            cursor = match action {
                                Some(action) => {
                                    if first_action {
                                        first_action = false;
                                        self.textarea.do_action(action)
                                    } else {
                                        first_action = false;
                                        self.textarea.do_action_chain(action)
                                    }
                                }
                                None => cursor,
                            };
                        }

                        let action = if cursor < selection {
                            match &self.textarea.indent {
                                Indent::Tabs => {
                                    if self.textarea.lines[selection.row].starts_with('\t') {
                                        Some(HistoryAction::RemoveChar {
                                            char: '\t',
                                            position: BytePosition { row: selection.row, col: 0 },
                                            cursor: (cursor, cursor),
                                        })
                                    } else {
                                        None
                                    }
                                }
                                Indent::Spaces(spaces) => {
                                    if self.textarea.lines[selection.row].starts_with('\t')
                                        || self.textarea.lines[selection.row].starts_with(spaces)
                                    {
                                        Some(HistoryAction::RemoveLines {
                                            lines: vec![spaces.clone()],
                                            position: BytePosition { row: selection.row, col: 0 },
                                            cursor: (cursor, cursor),
                                        })
                                    } else {
                                        None
                                    }
                                }
                            }
                        } else {
                            match &self.textarea.indent {
                                Indent::Tabs => {
                                    if self.textarea.lines[cursor.row].starts_with('\t') {
                                        Some(HistoryAction::RemoveChar {
                                            char: '\t',
                                            position: BytePosition { row: cursor.row, col: 0 },
                                            cursor: (
                                                cursor,
                                                CursorPosition {
                                                    col: cursor.col.saturating_sub(1),
                                                    ..cursor
                                                },
                                            ),
                                        })
                                    } else {
                                        None
                                    }
                                }
                                Indent::Spaces(spaces) => {
                                    if self.textarea.lines[cursor.row].starts_with('\t')
                                        || self.textarea.lines[cursor.row].starts_with(spaces)
                                    {
                                        Some(HistoryAction::RemoveLines {
                                            lines: vec![spaces.clone()],
                                            position: BytePosition { row: cursor.row, col: 0 },
                                            cursor: (
                                                cursor,
                                                CursorPosition {
                                                    col: cursor.col.saturating_sub(spaces.len()),
                                                    ..cursor
                                                },
                                            ),
                                        })
                                    } else {
                                        None
                                    }
                                }
                            }
                        };

                        let selection_increment = if action.is_some() {
                            match &self.textarea.indent {
                                Indent::Tabs => 1,
                                Indent::Spaces(spaces) => spaces.len(),
                            }
                        } else {
                            0
                        };

                        let cursor = match action {
                            Some(action) => {
                                if first_action {
                                    first_action = false;
                                    self.textarea.do_action(action)
                                } else {
                                    first_action = false;
                                    self.textarea.do_action_chain(action)
                                }
                            }
                            None => cursor,
                        };

                        self.textarea.set_cursor(cursor, false);
                        self.textarea.set_selection(Some(CursorPosition {
                            col: selection.col + selection_increment,
                            ..selection
                        }));

                        !first_action
                    }
                    _ => {
                        let action = match &self.textarea.indent {
                            Indent::Tabs => {
                                if self.textarea.lines[cursor.row].starts_with('\t') {
                                    Some(HistoryAction::RemoveChar {
                                        char: '\t',
                                        position: BytePosition { row: cursor.row, col: 0 },
                                        cursor: (
                                            cursor,
                                            CursorPosition {
                                                col: cursor.col.saturating_sub(1),
                                                ..cursor
                                            },
                                        ),
                                    })
                                } else {
                                    None
                                }
                            }
                            Indent::Spaces(spaces) => {
                                if self.textarea.lines[cursor.row].starts_with('\t')
                                    || self.textarea.lines[cursor.row].starts_with(spaces)
                                {
                                    Some(HistoryAction::RemoveLines {
                                        lines: vec![spaces.clone()],
                                        position: BytePosition { row: cursor.row, col: 0 },
                                        cursor: (
                                            cursor,
                                            CursorPosition {
                                                col: cursor.col.saturating_sub(spaces.len()),
                                                ..cursor
                                            },
                                        ),
                                    })
                                } else {
                                    None
                                }
                            }
                        };

                        match action {
                            Some(action) => {
                                let cursor = self.textarea.do_action(action);
                                self.textarea.set_cursor(cursor, false);

                                let selection_increment = match &self.textarea.indent {
                                    Indent::Tabs => 1,
                                    Indent::Spaces(spaces) => spaces.len(),
                                };
                                self.textarea.set_selection(selection.map(|selection| CursorPosition {
                                    col: selection.col - selection_increment,
                                    ..selection
                                }));

                                true
                            }
                            None => false,
                        }
                    }
                }
            }
            Input {
                key: key @ (Key::Up | Key::Down),
                ctrl: false,
                alt: true,
                shift: false,
            } => {
                let cursor = self.textarea.cursor();
                let selection = self.textarea.selection();

                let (start, end) = match selection {
                    Some(selection) if cursor < selection => (cursor, selection),
                    Some(selection) if cursor > selection => (selection, cursor),
                    _ => (cursor, cursor),
                };

                if key == Key::Up && start.row > 0 {
                    let mut cursor = self.textarea.do_action(HistoryAction::SwapLines {
                        lines: (start.row, start.row - 1),
                        cursor: (cursor, CursorPosition { row: cursor.row - 1, ..cursor }),
                    });

                    for row in (start.row..=end.row).skip(1) {
                        cursor = self.textarea.do_action_chain(HistoryAction::SwapLines {
                            lines: (row, row - 1),
                            cursor: (cursor, cursor),
                        });
                    }

                    self.textarea.set_cursor(cursor, false);
                    self.textarea.set_selection(selection.map(|selection| CursorPosition {
                        row: selection.row - 1,
                        ..selection
                    }));

                    true
                } else if key == Key::Down && end.row < self.textarea.lines.len() - 1 {
                    let mut cursor = self.textarea.do_action(HistoryAction::SwapLines {
                        lines: (end.row, end.row + 1),
                        cursor: (cursor, CursorPosition { row: cursor.row + 1, ..cursor }),
                    });

                    for row in (start.row..end.row).rev() {
                        cursor = self.textarea.do_action_chain(HistoryAction::SwapLines {
                            lines: (row, row + 1),
                            cursor: (cursor, cursor),
                        });
                    }

                    self.textarea.set_cursor(cursor, false);
                    self.textarea.set_selection(selection.map(|selection| CursorPosition {
                        row: selection.row + 1,
                        ..selection
                    }));

                    true
                } else {
                    false
                }
            }
            Input {
                key: Key::Char('d'),
                ctrl: true,
                alt: false,
                shift: false,
            } => {
                let cursor = self.textarea.cursor();

                let cursor = self.textarea.do_action(HistoryAction::InsertLines {
                    lines: vec![self.textarea.lines[cursor.row].clone(), "".to_string()],
                    position: BytePosition { row: cursor.row, col: 0 },
                    cursor: (cursor, CursorPosition { row: cursor.row + 1, ..cursor }),
                });
                self.textarea.set_cursor(cursor, false);

                true
            }
            Input {
                key: Key::Char(char @ ('(' | '[' | '{' | '\'' | '"')),
                ..
            } if !(char == '\'' && self.textarea.selection().is_none()) => {
                let cursor = self.textarea.cursor();
                let selection = self.textarea.selection();

                let closing_char = match char {
                    '(' => ')',
                    '[' => ']',
                    '{' => '}',
                    '\'' => '\'',
                    '"' => '"',
                    _ => unreachable!(),
                };

                match selection {
                    Some(selection) => {
                        let (c1, c2) = if cursor < selection {
                            (
                                cursor,
                                CursorPosition {
                                    col: selection.col + 1,
                                    ..selection
                                },
                            )
                        } else {
                            (selection, CursorPosition { col: cursor.col + 1, ..cursor })
                        };

                        let (cursor_after, selection_after) = if cursor.row == selection.row {
                            (
                                CursorPosition { col: cursor.col + 1, ..cursor },
                                CursorPosition {
                                    col: selection.col + 1,
                                    ..selection
                                },
                            )
                        } else if cursor < selection {
                            (CursorPosition { col: cursor.col + 1, ..cursor }, selection)
                        } else {
                            (
                                cursor,
                                CursorPosition {
                                    col: selection.col + 1,
                                    ..selection
                                },
                            )
                        };

                        let cursor = self.textarea.do_action(HistoryAction::InsertChar {
                            char,
                            position: BytePosition::from_line(c1, &self.textarea.lines[cursor.row]),
                            cursor: (cursor, cursor_after),
                        });
                        let cursor = self.textarea.do_action_chain(HistoryAction::InsertChar {
                            char: closing_char,
                            position: BytePosition::from_line(c2, &self.textarea.lines[cursor.row]),
                            cursor: (cursor, cursor),
                        });
                        self.textarea.set_cursor(cursor, false);
                        self.textarea.set_selection(Some(selection_after));
                    }
                    None => {
                        let cursor = self.textarea.do_action(HistoryAction::InsertChar {
                            char,
                            position: BytePosition::from_line(cursor, &self.textarea.lines[cursor.row]),
                            cursor: (cursor, CursorPosition { col: cursor.col + 1, ..cursor }),
                        });
                        let cursor = self.textarea.do_action_chain(HistoryAction::InsertChar {
                            char: closing_char,
                            position: BytePosition::from_line(cursor, &self.textarea.lines[cursor.row]),
                            cursor: (cursor, cursor),
                        });
                        self.textarea.set_cursor(cursor, false);
                    }
                }

                true
            }

            _ => self.textarea.input(input),
        }
    }
}
