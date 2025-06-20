use ratatui::layout::Position;

use crate::editor::CursorPosition;

#[derive(Default, Debug, Clone)]
pub struct Viewport {
    position: CursorPosition,
    width: usize,
    height: usize,
}

impl Viewport {
    pub fn update_size(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
    }

    pub fn rect(&self) -> (CursorPosition, CursorPosition) {
        (
            CursorPosition {
                row: self.position.row,
                col: self.position.col,
            },
            CursorPosition {
                row: self.position.row.saturating_add(self.height),
                col: self.position.col.saturating_add(self.width),
            },
        )
    }

    pub fn update_view(&mut self, cursor: CursorPosition) {
        self.position.row = self
            .position
            .row
            .clamp(cursor.row.saturating_sub(self.height - 1), cursor.row);
        self.position.col = self
            .position
            .col
            .clamp(cursor.col.saturating_sub(self.width), cursor.col);
    }

    pub fn terminal_cursor_position(&self, cursor: CursorPosition) -> Position {
        Position {
            x: (cursor.col - self.position.col).try_into().unwrap(),
            y: (cursor.row - self.position.row).try_into().unwrap(),
        }
    }
}
