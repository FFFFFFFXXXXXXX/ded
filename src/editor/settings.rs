use std::num::NonZeroU8;

#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub indent: Indent,
    pub line_numbers: bool,
    pub fullscreen: Fullscreen,
}

#[derive(Debug, Clone)]
pub enum Indent {
    Tabs,
    Spaces(NonZeroU8),
}

impl Default for Indent {
    fn default() -> Self {
        Self::Spaces(NonZeroU8::new(4).unwrap())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Fullscreen {
    #[default]
    Off,
    Half,
    Full,
}

impl Fullscreen {
    pub fn toggle(&self) -> Self {
        match self {
            Fullscreen::Off => Fullscreen::Half,
            Fullscreen::Half => Fullscreen::Full,
            Fullscreen::Full => Fullscreen::Off,
        }
    }
}
