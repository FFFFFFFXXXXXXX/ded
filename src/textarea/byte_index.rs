pub trait ByteIndex {
    fn byte_index(&self, char_idx: usize) -> usize;
}

impl ByteIndex for str {
    #[inline(always)]
    fn byte_index(&self, char_idx: usize) -> usize {
        self.char_indices()
            .nth(char_idx)
            .map(|(byte_idx, _)| byte_idx)
            .unwrap_or_else(|| self.len())
    }
}
