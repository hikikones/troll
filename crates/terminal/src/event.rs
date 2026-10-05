use crossterm::event::{
    Event as CrosstermEvent, KeyCode, KeyEvent as CrosstermKeyEvent, KeyModifiers,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerminalEvent {
    Key(Key),
    Resize,
}

impl TerminalEvent {
    pub(crate) fn from(event: CrosstermEvent) -> Option<Self> {
        match event {
            CrosstermEvent::Key(key) if key.is_press() => Some(Self::Key(Key::from(key))),
            CrosstermEvent::Resize(_, _) => Some(Self::Resize),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl Key {
    const fn from(key_event: CrosstermKeyEvent) -> Self {
        Self {
            code: key_event.code,
            modifiers: key_event.modifiers,
        }
    }

    pub const fn ctrl(&self) -> bool {
        self.modifiers.contains(KeyModifiers::CONTROL)
    }

    pub const fn shift(&self) -> bool {
        self.modifiers.contains(KeyModifiers::SHIFT)
    }

    pub const fn alt(&self) -> bool {
        self.modifiers.contains(KeyModifiers::ALT)
    }
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.modifiers {
            KeyModifiers::SHIFT => write!(f, "Shift")?,
            KeyModifiers::CONTROL => write!(f, "Ctrl")?,
            KeyModifiers::ALT => write!(f, "Alt")?,
            KeyModifiers::SUPER => write!(f, "Super")?,
            KeyModifiers::HYPER => write!(f, "Hyper")?,
            KeyModifiers::META => write!(f, "Meta")?,
            _ => unreachable!(),
        }

        match self.code {
            KeyCode::Backspace => write!(f, "Backspace"),
            KeyCode::Delete => write!(f, "Del"),
            KeyCode::Enter => write!(f, "Enter"),
            KeyCode::Left => write!(f, "Left"),
            KeyCode::Right => write!(f, "Right"),
            KeyCode::Up => write!(f, "Up"),
            KeyCode::Down => write!(f, "Down"),
            KeyCode::Home => write!(f, "Home"),
            KeyCode::End => write!(f, "End"),
            KeyCode::PageUp => write!(f, "Page Up"),
            KeyCode::PageDown => write!(f, "Page Down"),
            KeyCode::Tab => write!(f, "Tab"),
            KeyCode::BackTab => write!(f, "Back Tab"),
            KeyCode::Insert => write!(f, "Insert"),
            KeyCode::F(n) => write!(f, "F{}", n),
            KeyCode::Char(' ') => write!(f, "Space"),
            KeyCode::Char(c) => write!(f, "{}", c),
            KeyCode::Null => write!(f, "Null"),
            KeyCode::Esc => write!(f, "Esc"),
            KeyCode::CapsLock => write!(f, "Caps Lock"),
            KeyCode::ScrollLock => write!(f, "Scroll Lock"),
            KeyCode::NumLock => write!(f, "Num Lock"),
            KeyCode::PrintScreen => write!(f, "Print Screen"),
            KeyCode::Pause => write!(f, "Pause"),
            KeyCode::Menu => write!(f, "Menu"),
            KeyCode::KeypadBegin => write!(f, "Begin"),
            KeyCode::Media(media) => write!(f, "{}", media),
            KeyCode::Modifier(modifier) => write!(f, "{}", modifier),
        }
    }
}
