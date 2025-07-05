pub trait Word {
    fn next_word(&self, start: usize) -> Option<usize>;
    fn previous_word(&self, start: usize) -> Option<usize>;
}

impl Word for str {
    fn next_word(&self, start: usize) -> Option<usize> {
        let mut iter = self.char_indices().skip(start).skip_while(|(_, c)| c.is_whitespace());

        if iter.next().is_some_and(|(_, c)| c.is_ascii_punctuation()) {
            iter.find_map(|(idx, c)| (!c.is_ascii_punctuation()).then_some(idx))
        } else {
            iter.find_map(|(idx, c)| (c.is_ascii_punctuation() || c.is_ascii_whitespace()).then_some(idx))
        }
    }
    fn previous_word(&self, start: usize) -> Option<usize> {
        let mut iter = self
            .char_indices()
            .rev()
            .skip(self.len() - start)
            .skip_while(|(_, c)| c.is_whitespace());

        if iter.next().is_some_and(|(_, c)| c.is_ascii_punctuation()) {
            iter.find_map(|(idx, c)| (!c.is_ascii_punctuation()).then_some(idx + 1))
        } else {
            iter.find_map(|(idx, c)| (c.is_ascii_punctuation() || c.is_ascii_whitespace()).then_some(idx + 1))
        }
    }
}

#[test]
fn test() {
    assert_eq!("   abc ".next_word(0), Some(6));
    assert_eq!("   a!bc ".next_word(0), Some(4));
    assert_eq!("   !!bc ".next_word(0), Some(5));
    assert_eq!("   !!   ".next_word(0), Some(5));

    assert_eq!("   abc  ".previous_word(8), Some(3));
    assert_eq!("   a!bc ".previous_word(8), Some(5));
    assert_eq!("   bc!! ".previous_word(8), Some(5));
    assert_eq!("   !!   ".previous_word(8), Some(3));
}
