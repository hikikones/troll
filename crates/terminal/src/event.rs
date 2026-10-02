use crossterm::event::{Event as CrosstermEvent, KeyEvent as CrosstermKeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerminalEvent {
    Key(Key),
    Resize,
}

impl TerminalEvent {
    pub(crate) fn from(event: CrosstermEvent) -> Option<Self> {
        match event {
            CrosstermEvent::Key(key) if key.kind.is_press() => Some(Self::Key(Key::from(key))),
            CrosstermEvent::Resize(_, _) => Some(Self::Resize),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key {
    pub code: crossterm::event::KeyCode,
    pub modifiers: crossterm::event::KeyModifiers,
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
