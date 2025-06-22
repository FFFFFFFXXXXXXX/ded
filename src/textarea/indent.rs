#[derive(Debug, Clone)]
pub enum Indent {
    Tabs,
    Spaces(String),
}

impl Indent {
    pub fn spaces(&self) -> &str {
        match self {
            Indent::Tabs => "    ",
            Indent::Spaces(spaces) => spaces,
        }
    }
}

impl Default for Indent {
    fn default() -> Self {
        4.into()
    }
}

impl From<usize> for Indent {
    fn from(spaces: usize) -> Self {
        Self::Spaces((0..spaces).map(|_| ' ').collect())
    }
}
