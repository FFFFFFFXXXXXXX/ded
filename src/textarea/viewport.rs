use std::cell::Cell;

use ratatui::layout::Position;

use super::CursorPosition;

#[derive(Default, Debug, Clone)]
pub struct Viewport {
    position: Cell<CursorPosition>,
    width: Cell<usize>,
    height: Cell<usize>,
}

impl Viewport {
    pub fn update_size(&self, width: usize, height: usize) -> (CursorPosition, CursorPosition) {
        self.width.set(width);
        self.height.set(height);

        (
            self.position.get(),
            CursorPosition {
                row: self.position.get().row.saturating_add(self.height.get()),
                col: self.position.get().col.saturating_add(self.width.get()),
            },
        )
    }

    pub fn update_view(&mut self, cursor: CursorPosition) {
        self.position.set(CursorPosition {
            row: self
                .position
                .get()
                .row
                .clamp(cursor.row.saturating_sub(self.height.get() - 1), cursor.row),
            col: self
                .position
                .get()
                .col
                .clamp(cursor.col.saturating_sub(self.width.get()), cursor.col),
        });
    }

    pub fn terminal_cursor_position(&self, cursor: CursorPosition) -> Position {
        Position {
            x: (cursor.col - self.position.get().col).try_into().unwrap(),
            y: (cursor.row - self.position.get().row).try_into().unwrap(),
        }
    }
}
