use crate::textarea::{ByteIndex, CursorPosition};

#[derive(Debug, Copy, Clone)]
pub struct BytePosition {
    pub row: usize,
    pub col: usize,
}

impl BytePosition {
    pub fn from_line(cursor: CursorPosition, line: &str) -> Self {
        Self {
            row: cursor.row,
            col: line.byte_index(cursor.col),
        }
    }
}

#[derive(Debug, Clone)]
pub enum HistoryAction {
    InsertChar {
        char: char,
        position: BytePosition,
        cursor: (CursorPosition, CursorPosition),
    },
    RemoveChar {
        char: char,
        position: BytePosition,
        cursor: (CursorPosition, CursorPosition),
    },
    InsertLinebreak {
        position: BytePosition,
        cursor: (CursorPosition, CursorPosition),
    },
    RemoveLinebreak {
        position: BytePosition,
        cursor: (CursorPosition, CursorPosition),
    },
    InsertLines {
        lines: Vec<String>,
        position: BytePosition,
        cursor: (CursorPosition, CursorPosition),
    },
    RemoveLines {
        lines: Vec<String>,
        position: BytePosition,
        cursor: (CursorPosition, CursorPosition),
    },
    SwapLines {
        lines: (usize, usize),
        cursor: (CursorPosition, CursorPosition),
    },
}

impl HistoryAction {
    pub fn invert(self) -> Self {
        match self {
            HistoryAction::InsertChar {
                char,
                position,
                cursor: (c1, c2),
            } => HistoryAction::RemoveChar {
                char,
                position,
                cursor: (c2, c1),
            },
            HistoryAction::RemoveChar {
                char,
                position,
                cursor: (c1, c2),
            } => HistoryAction::InsertChar {
                char,
                position,
                cursor: (c2, c1),
            },
            HistoryAction::InsertLinebreak { position, cursor: (c1, c2) } => {
                HistoryAction::RemoveLinebreak { position, cursor: (c2, c1) }
            }
            HistoryAction::RemoveLinebreak { position, cursor: (c1, c2) } => {
                HistoryAction::InsertLinebreak { position, cursor: (c2, c1) }
            }
            HistoryAction::InsertLines {
                lines,
                position,
                cursor: (c1, c2),
            } => HistoryAction::RemoveLines {
                lines,
                position,
                cursor: (c2, c1),
            },
            HistoryAction::RemoveLines {
                lines,
                position,
                cursor: (c1, c2),
            } => HistoryAction::InsertLines {
                lines,
                position,
                cursor: (c2, c1),
            },
            HistoryAction::SwapLines {
                lines: (l1, l2),
                cursor: (c1, c2),
            } => HistoryAction::SwapLines {
                lines: (l2, l1),
                cursor: (c2, c1),
            },
        }
    }

    pub fn apply(&self, lines: &mut Vec<String>) -> CursorPosition {
        match self {
            HistoryAction::InsertChar { char, position, cursor: (_, c) } => {
                lines[position.row].insert(position.col, *char);
                *c
            }
            HistoryAction::RemoveChar { position, cursor: (_, c), .. } => {
                lines[position.row].remove(position.col);
                *c
            }
            HistoryAction::InsertLinebreak { position, cursor: (_, c) } => {
                lines.insert(position.row + 1, lines[position.row][position.col..].to_string());
                lines[position.row].drain(position.col..);
                *c
            }
            HistoryAction::RemoveLinebreak { position, cursor: (_, c) } => {
                let (a, b) = lines.split_at_mut(position.row + 1);
                a.last_mut().unwrap().push_str(b.first().unwrap());
                lines.remove(position.row + 1);
                *c
            }
            HistoryAction::InsertLines {
                lines: ls,
                position,
                cursor: (_, c),
            } => {
                match ls.len() {
                    0 => {}
                    1 => {
                        lines[position.row].insert_str(position.col, ls.first().unwrap());
                    }
                    _ => {
                        if let Some((l1, l2)) = lines.get(position.row).map(|l| l.split_at(position.col)) {
                            let mut first_line = String::from(l1);
                            first_line.push_str(ls.first().unwrap());

                            let mut last_line = String::from(ls.last().unwrap());
                            last_line.push_str(l2);

                            lines[position.row] = first_line;
                            lines.splice(
                                position.row + 1..position.row + 1,
                                ls.iter().skip(1).map(|l| l.to_string()),
                            );
                            lines[position.row + ls.len() - 1] = last_line;
                        } else {
                            lines.splice(
                                position.row..position.row,
                                ls.iter().take(ls.len() - 1).map(|l| l.to_string()),
                            );
                        }
                    }
                };

                *c
            }
            HistoryAction::RemoveLines {
                lines: ls,
                position,
                cursor: (_, c),
            } => {
                match ls.len() {
                    0 => {}
                    1 => {
                        lines[position.row].drain(position.col..position.col + ls.first().unwrap().len());
                    }
                    _ => {
                        let (a, b) = lines.split_at_mut(position.row + 1);
                        a.last_mut().unwrap().replace_range(
                            position.col..,
                            b.get(ls.len() - 2)
                                .map(|l| &l[ls.last().unwrap().len()..])
                                .unwrap_or(""),
                        );

                        lines.drain((position.row + 1).min(lines.len())..(position.row + ls.len()).min(lines.len()));
                    }
                }

                *c
            }
            HistoryAction::SwapLines {
                lines: (l1, l2),
                cursor: (_, c2),
            } => {
                lines.swap(*l1, *l2);
                *c2
            }
        }
    }
}
