use std::fmt::Display;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};

use crate::{
    input::{Input, Key},
    textarea::TextArea,
};

#[derive(Debug)]
pub struct SearchBox<'a> {
    textarea: TextArea,
    border_block: Block<'a>,
    open: bool,
}

impl<'a> Default for SearchBox<'a> {
    fn default() -> Self {
        Self {
            textarea: TextArea::default(),
            border_block: Block::default().borders(Borders::ALL).title(" Search: "),
            open: false,
        }
    }
}

impl<'a> SearchBox<'a> {
    pub fn open(&mut self) -> &str {
        self.open = true;
        &self.textarea.lines()[0]
    }

    pub fn close(&mut self) {
        self.open = false;
        self.border_block = Block::default().borders(Borders::ALL).title(" Search: ");
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn input(&mut self, input: Input) -> Option<&'_ str> {
        match input {
            Input { key: Key::Enter, .. } => None,
            input => {
                let modified = self.textarea.input(input);
                modified.then(|| self.textarea.lines()[0].as_str())
            }
        }
    }

    pub fn set_error_message(&mut self, error_message: Option<impl Display>) {
        self.border_block = match error_message {
            Some(err_msg) => Block::default()
                .borders(Borders::ALL)
                .title(format!(" Search: {} ", err_msg))
                .style(Style::default().fg(Color::Red)),
            None => Block::default().borders(Borders::ALL).title(" Search: "),
        };
    }

    pub fn set_pattern(&mut self, pattern: &str) {
        // self.textarea.delete_line(false);
        // self.textarea.insert_str(pattern);
    }
}

impl<'a> Widget for &SearchBox<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        if area.is_empty() {
            return;
        }

        (&self.border_block).render(area, buf);
        self.textarea.render(self.border_block.inner(area), buf);
    }
}
