pub trait CharSlice<'a> {
    fn get_char_slice(&'a self, col_start: usize, col_end: usize) -> &'a str;
}

impl<'a> CharSlice<'a> for str {
    fn get_char_slice(&'a self, col_start: usize, col_end: usize) -> &'a str {
        let Some(start) = self.char_indices().nth(col_start).map(|(i, _)| i) else { return "" };
        let Some(end) = self.char_indices().nth(col_end).map(|(i, _)| i) else { return &self[start..] };
        &self[start..end]
    }
}
