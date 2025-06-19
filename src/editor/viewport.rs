use std::cmp;

use ratatui::layout::Position;

use crate::editor::CursorPosition;

#[derive(Default, Debug, Clone)]
pub struct Viewport {
    position: CursorPosition,
    width: usize,
    height: usize,
}

impl Viewport {
    pub fn update_size(&mut self, width: u16, height: u16) {
        self.width = width.into();
        self.height = height.into();
    }

    pub fn rect(&self) -> (CursorPosition, CursorPosition) {
        let CursorPosition { row: row_top, col: col_top } = self.position;

        let row_bottom = row_top.saturating_add(self.height).saturating_sub(1);
        let col_bottom = col_top.saturating_add(self.width).saturating_sub(1);

        (
            CursorPosition { row: row_top, col: col_top },
            CursorPosition {
                row: cmp::max(row_top, row_bottom),
                col: cmp::max(col_top, col_bottom),
            },
        )
    }

    pub fn terminal_cursor_position(&self, cursor: CursorPosition) -> Position {
        Position {
            x: (cursor.col - self.position.col).try_into().unwrap(),
            y: (cursor.row - self.position.row).try_into().unwrap(),
        }
    }

    // pub fn scroll(&mut self, rows: i64, cols: i64) {
    //     self.row.set(self.row.get().saturating_add_signed(rows));
    //     self.col.set(self.col.get().saturating_add_signed(cols));
    // }
}
