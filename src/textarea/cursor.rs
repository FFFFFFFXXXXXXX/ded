use std::cmp;

#[derive(Debug, Clone, Copy, Default, Eq, Ord)]
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
        if self == other {
            Some(cmp::Ordering::Equal)
        } else if self.row < other.row || (self.row == other.row && self.col < other.col) {
            Some(cmp::Ordering::Less)
        } else {
            Some(cmp::Ordering::Greater)
        }
    }
}
