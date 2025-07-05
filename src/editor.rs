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
                self.textarea.push_history_action(HistoryAction::InsertChar {
                    char,
                    position: dbg!(BytePosition::new(
                        CursorPosition { col: cursor.col, ..cursor },
                        &self.textarea.lines[cursor.row]
                    )),
                });
                self.textarea
                    .set_cursor(CursorPosition { col: cursor.col + 1, ..cursor }, false);

                true
            }

            Input {
                key: Key::Backspace,
                ctrl: false,
                alt: false,
                ..
            } => {
                let mut cursor = self.textarea.cursor();
                cursor.col = cursor.col.saturating_sub(1);

                self.textarea.push_history_action(HistoryAction::RemoveChar {
                    char: self.textarea.lines[cursor.row].chars().nth(cursor.col).unwrap(),
                    position: BytePosition::new(cursor, &self.textarea.lines[cursor.row]),
                });

                self.textarea.set_cursor(cursor, false);

                true
            }

            Input {
                key: Key::Char('z'),
                ctrl: true,
                alt: false,
                shift: false,
            } => {
                self.textarea.pop_history_action();
                true
            }
            _ => {
                self.textarea.input(input);
                false
            }
        }
    }
}

// cursor & selection
impl Editor {
    pub fn selected_text(&mut self) -> Option<&str> {
        let lines = &self.textarea.lines;
        let cursor = self.textarea.cursor();
        let selection = self.textarea.selection();

        if let Some(selection) = selection {
            if cursor.row != selection.row {
                return None;
            }

            if selection < cursor {
                Some(lines[cursor.row].get_char_slice(selection.col, cursor.col))
            } else {
                Some(lines[cursor.row].get_char_slice(cursor.col, selection.col))
            }
        } else {
            None
        }
    }
}
