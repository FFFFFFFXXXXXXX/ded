use std::cmp;

use crate::editor::CursorPosition;

#[derive(Default, Debug, Clone)]
pub struct Viewport {
    position: CursorPosition,
}

impl Viewport {
    pub fn rect(&self, width: u16, height: u16) -> (CursorPosition, CursorPosition) {
        let CursorPosition { row: row_top, col: col_top } = self.position;

        let row_bottom = row_top.saturating_add(height.into()).saturating_sub(1);
        let col_bottom = col_top.saturating_add(width.into()).saturating_sub(1);

        (
            CursorPosition { row: row_top, col: col_top },
            CursorPosition {
                row: cmp::max(row_top, row_bottom),
                col: cmp::max(col_top, col_bottom),
            },
        )
    }

    // pub fn scroll(&mut self, rows: i64, cols: i64) {
    //     self.row.set(self.row.get().saturating_add_signed(rows));
    //     self.col.set(self.col.get().saturating_add_signed(cols));
    // }
}
