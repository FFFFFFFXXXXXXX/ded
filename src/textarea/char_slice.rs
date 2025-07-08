use std::ops::{Bound, RangeBounds};

pub trait CharSlice<'a> {
    fn char_slice(&'a self, range: impl RangeBounds<usize>) -> &'a str;
}

impl<'a> CharSlice<'a> for str {
    #[inline(always)]
    fn char_slice(&'a self, range: impl RangeBounds<usize>) -> &'a str {
        let Some(start) = (match range.start_bound() {
            Bound::Included(&col) => self.char_indices().nth(col).map(|(i, _)| i),
            Bound::Unbounded => Some(0),
            _ => unreachable!(),
        }) else {
            return "";
        };

        let Some(end) = (match range.end_bound() {
            Bound::Included(&col) => self.char_indices().nth(col + 1).map(|(i, _)| i),
            Bound::Excluded(&col) => self.char_indices().nth(col).map(|(i, _)| i),
            Bound::Unbounded => return &self[start..],
        }) else {
            return &self[start..];
        };

        &self[start..end]
    }
}
