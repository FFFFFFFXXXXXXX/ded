use anyhow::Result;

use std::io::BufRead;
use std::num::NonZeroU8;
use std::{fs, io};

use crate::editor::cursor::CursorPosition;
use crate::editor::history::HistoryAction;
use crate::editor::settings::{Indent, Settings};
use crate::editor::viewport::Viewport;

#[derive(Debug, Clone, Default)]
pub struct TextArea {
    lines: Vec<String>,
    viewport: Viewport,
    selection_start: Option<CursorPosition>,
    cursor: CursorPosition,

    history: Vec<HistoryAction>,
    clipboard: String,
    search: (), // todo

    settings: Settings,
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
                            indent = Some(Indent::Spaces(
                                NonZeroU8::new(spaces).unwrap_or(NonZeroU8::new(4).unwrap()),
                            ));
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
            settings: Settings {
                indent: indent.unwrap_or_default(),
                ..Default::default()
            },
            ..Default::default()
        })
    }
}
