use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[non_exhaustive]
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    /// Normal letter key input
    Char(char),
    /// F1, F2, F3, ... keys
    F(u8),
    /// Backspace key
    Backspace,
    /// Enter or return key
    Enter,
    /// Left arrow key
    Left,
    /// Right arrow key
    Right,
    /// Up arrow key
    Up,
    /// Down arrow key
    Down,
    /// Tab key
    Tab,
    /// Tab key
    BackTab,
    /// Delete key
    Delete,
    /// Home key
    Home,
    /// End key
    End,
    /// Page up key
    PageUp,
    /// Page down key
    PageDown,
    /// Escape key
    Esc,
    /// Copy key. This key is supported by termwiz only
    Copy,
    /// Cut key. This key is supported by termwiz only
    Cut,
    /// Paste key. This key is supported by termwiz only
    Paste,
    /// Virtual key to scroll down by mouse
    MouseScrollDown,
    /// Virtual key to scroll up by mouse
    MouseScrollUp,
    /// An invalid key input (this key is always ignored by [`TextArea`](crate::TextArea))
    #[default]
    Null,
}

#[derive(Debug, Clone, Default, PartialEq, Hash, Eq)]
pub struct Input {
    /// Typed key.
    pub key: Key,
    /// Ctrl modifier key. `true` means Ctrl key was pressed.
    pub ctrl: bool,
    /// Alt modifier key. `true` means Alt key was pressed.
    pub alt: bool,
    /// Shift modifier key. `true` means Shift key was pressed.
    pub shift: bool,
}

impl From<Event> for Input {
    /// Convert [`crossterm::event::Event`] into [`Input`].
    fn from(event: Event) -> Self {
        match event {
            Event::Key(key) => Self::from(key),
            // Event::Mouse(mouse) => Self::from(mouse),
            _ => Self::default(),
        }
    }
}

impl From<KeyCode> for Key {
    /// Convert [`crossterm::event::KeyCode`] into [`Key`].
    fn from(code: KeyCode) -> Self {
        match code {
            KeyCode::Char(c) => Key::Char(c),
            KeyCode::Backspace => Key::Backspace,
            KeyCode::Enter => Key::Enter,
            KeyCode::Left => Key::Left,
            KeyCode::Right => Key::Right,
            KeyCode::Up => Key::Up,
            KeyCode::Down => Key::Down,
            KeyCode::Tab => Key::Tab,
            KeyCode::BackTab => Key::BackTab,
            KeyCode::Delete => Key::Delete,
            KeyCode::Home => Key::Home,
            KeyCode::End => Key::End,
            KeyCode::PageUp => Key::PageUp,
            KeyCode::PageDown => Key::PageDown,
            KeyCode::Esc => Key::Esc,
            KeyCode::F(x) => Key::F(x),
            _ => Key::Null,
        }
    }
}

impl From<KeyEvent> for Input {
    /// Convert [`crossterm::event::KeyEvent`] into [`Input`].
    fn from(key: KeyEvent) -> Self {
        if key.kind == KeyEventKind::Release {
            // On Windows or when `crossterm::event::PushKeyboardEnhancementFlags` is set,
            // key release event can be reported. Ignore it. (#14)
            return Self::default();
        }

        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        let key = Key::from(key.code);

        Self { key, ctrl, alt, shift }
    }
}
