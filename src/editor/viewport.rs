use std::{cell::Cell, cmp};

#[derive(Default, Debug, Clone)]
pub struct Viewport {
    row: Cell<u64>,
    col: Cell<u64>,
    width: Cell<u16>,
    height: Cell<u16>,
}

impl Viewport {
    fn store(&self, row: u64, col: u64, width: u16, height: u16) {
        self.width.set(width);
        self.height.set(height);
        self.row.set(row);
        self.col.set(col);
    }

    pub fn scroll_top(&self) -> (u64, u64) {
        (self.row.get(), self.col.get())
    }

    pub fn rect(&self) -> (u64, u64, u16, u16) {
        (self.row.get(), self.col.get(), self.width.get(), self.height.get())
    }

    pub fn position(&self) -> (u64, u64, u64, u64) {
        let (row_top, col_top, width, height) = self.rect();
        let row_bottom = row_top.saturating_add(height.into()).saturating_sub(1);
        let col_bottom = col_top.saturating_add(width.into()).saturating_sub(1);

        (
            row_top,
            col_top,
            cmp::max(row_top, row_bottom),
            cmp::max(col_top, col_bottom),
        )
    }

    pub fn scroll(&mut self, rows: i64, cols: i64) {
        self.row.set(self.row.get().saturating_add_signed(rows));
        self.col.set(self.col.get().saturating_add_signed(cols));
    }
}
