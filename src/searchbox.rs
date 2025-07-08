use std::fmt::Display;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Widget};

use crate::input::Input;
use crate::textarea::{CursorPosition, TextArea};

pub struct SearchBox<'a> {
    pub textarea: TextArea,
    border_block: Block<'a>,
    open: bool,
}

impl<'a> Default for SearchBox<'a> {
    fn default() -> Self {
        let mut textarea = TextArea::default();
        textarea.line_numbers = false;

        Self {
            textarea,
            border_block: Block::default().borders(Borders::ALL).title(" Search: "),
            open: false,
        }
    }
}

impl<'a> SearchBox<'a> {
    pub fn open(&mut self) -> &str {
        self.open = true;
        &self.textarea.lines[0]
    }

    pub fn close(&mut self) {
        self.open = false;
        self.border_block = Block::default().borders(Borders::ALL).title(" Search: ");
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn text(&self) -> &str {
        &self.textarea.lines[0]
    }

    pub fn set_text(&mut self, pattern: &str) {
        self.textarea.lines[0] = pattern.to_string();
        self.textarea
            .set_cursor(CursorPosition { row: 0, col: pattern.len() }, false);
    }

    pub fn input(&mut self, input: Input) -> Option<&'_ str> {
        self.textarea.input(input).then_some(self.text())
    }

    pub fn set_error_message(&mut self, error_message: Option<impl Display>) {
        self.border_block = match error_message {
            Some(err_msg) => Block::default()
                .borders(Borders::ALL)
                .title(format!(" Search: {err_msg} "))
                .style(Style::default().fg(Color::Red)),
            None => Block::default().borders(Borders::ALL).title(" Search: "),
        };
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
