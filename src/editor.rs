use anyhow::Result;

use crate::input::{Input, Key};
use crate::textarea::{BytePosition, CharSlice, CursorPosition, HistoryAction, Indent, TextArea};

#[derive(Default, Debug, Clone)]
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

        let mut textarea = TextArea::default();
        textarea.lines = lines;
        textarea.indent = indent.unwrap_or_default();

        Ok(Self { textarea })
    }

    pub fn input(&mut self, input: Input) -> bool {
        match input {
            Input {
                key: Key::Char(char),
                ctrl: false,
                alt: false,
                ..
            } => {
                let cursor = self.textarea.cursor();
                let selection = self.textarea.selection();

                match self.selected_text_unselect().zip(selection) {
                    Some((selected_text, selection)) => {
                        let start = if cursor < selection { cursor } else { selection };

                        let cursor = self.textarea.do_action(HistoryAction::RemoveLines {
                            lines: selected_text,
                            position: BytePosition::from_line(start, &self.textarea.lines[start.row]),
                            cursor: (cursor, start),
                        });

                        let cursor = self.textarea.do_action(HistoryAction::InsertChar {
                            char,
                            position: BytePosition::from_line(
                                CursorPosition { col: cursor.col, ..cursor },
                                &self.textarea.lines[cursor.row],
                            ),
                            cursor: (cursor, CursorPosition { col: cursor.col + 1, ..cursor }),
                        });
                        self.textarea.set_cursor(cursor, false);
                    }
                    None => {
                        let cursor = self.textarea.do_action(HistoryAction::InsertChar {
                            char,
                            position: BytePosition::from_line(
                                CursorPosition { col: cursor.col, ..cursor },
                                &self.textarea.lines[cursor.row],
                            ),
                            cursor: (cursor, CursorPosition { col: cursor.col + 1, ..cursor }),
                        });
                        self.textarea.set_cursor(cursor, false);
                    }
                }

                true
            }

            Input {
                key: Key::Backspace,
                ctrl: false,
                alt: false,
                ..
            } => {
                let cursor = self.textarea.cursor();
                let selection = self.textarea.selection();
                let selected_text = self.selected_text_unselect();

                let lines = &self.textarea.lines;

                if let Some((selected_text, selection)) = selected_text.zip(selection) {
                    let start = if cursor < selection { cursor } else { selection };

                    let cursor = self.textarea.do_action(HistoryAction::RemoveLines {
                        lines: selected_text,
                        position: BytePosition::from_line(start, &lines[start.row]),
                        cursor: (cursor, start),
                    });
                    self.textarea.set_cursor(cursor, false);

                    true
                } else {
                    match cursor {
                        CursorPosition { row: 0, col: 0 } => false,
                        CursorPosition { col: 0, .. } => {
                            let cursor = self.textarea.do_action(HistoryAction::RemoveLinebreak {
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
                            self.textarea.set_cursor(cursor, false);
                            true
                        }
                        _ => {
                            let cursor = self.textarea.do_action(HistoryAction::RemoveChar {
                                char: self.textarea.lines[cursor.row].chars().nth(cursor.col).unwrap(),
                                position: BytePosition::from_line(cursor, &self.textarea.lines[cursor.row]),
                                cursor: (
                                    cursor,
                                    CursorPosition {
                                        row: cursor.row,
                                        col: cursor.col - 1,
                                    },
                                ),
                            });
                            self.textarea.set_cursor(cursor, false);
                            true
                        }
                    }
                }
            }
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

            _ => self.textarea.input(input),
        }
    }
}

// cursor & selection
impl Editor {
    pub fn selected_text_unselect(&mut self) -> Option<Vec<String>> {
        let selection = self.textarea.selection()?;
        self.textarea.set_selection(None);

        let lines = &self.textarea.lines;
        let cursor = self.textarea.cursor();

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

    pub fn selected_text(&self) -> Option<&str> {
        let lines = &self.textarea.lines;
        let cursor = self.textarea.cursor();
        let selection = self.textarea.selection();

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
