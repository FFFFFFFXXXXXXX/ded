use std::cmp;

#[derive(Debug, Clone, Copy, Default, Eq)]
pub struct CursorPosition {
    pub row: usize,
    pub col: usize,
}

impl PartialEq for CursorPosition {
    fn eq(&self, other: &Self) -> bool {
        self.row == other.row && self.col == other.col
    }
}

impl PartialOrd for CursorPosition {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(Self::cmp(self, other))
    }
}

impl Ord for CursorPosition {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        if self == other {
            cmp::Ordering::Equal
        } else if self.row < other.row || (self.row == other.row && self.col < other.col) {
            cmp::Ordering::Less
        } else {
            cmp::Ordering::Greater
        }
    }
}
