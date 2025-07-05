use crate::textarea::CursorPosition;

#[derive(Debug, Copy, Clone)]
pub struct BytePosition {
    pub row: usize,
    pub col: usize,
}

impl BytePosition {
    pub fn new(cursor: CursorPosition, line: &str) -> Self {
        Self {
            row: cursor.row,
            col: line
                .char_indices()
                .nth(cursor.col)
                .map(|(idx, _)| idx)
                .unwrap_or_else(|| line.len()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum HistoryAction {
    InsertChar { char: char, position: BytePosition },
    InsertString { string: String, position: BytePosition },
    RemoveChar { char: char, position: BytePosition },
    RemoveString { string: String, position: BytePosition },
}

impl HistoryAction {
    pub fn invert(self) -> Self {
        match self {
            HistoryAction::InsertChar { char, position } => HistoryAction::RemoveChar { char, position },
            HistoryAction::RemoveChar { char, position } => HistoryAction::InsertChar { char, position },
            _ => todo!(),
        }
    }

    pub fn apply(&self, lines: &mut [String]) {
        match self {
            HistoryAction::InsertChar { char, position } => {
                lines[position.row].insert(position.col, *char);
            }
            HistoryAction::RemoveChar { position, .. } => {
                lines[position.row].remove(position.col);
            }
            _ => {}
        }
    }
}
