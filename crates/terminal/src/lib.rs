mod layout;
mod style;
mod term;

pub use layout::*;
pub use style::*;
pub use term::*;

pub use crossterm::{
    event::{Event, KeyCode, KeyEventKind, KeyModifiers},
    style::{
        Attribute, Color, ResetColor as Reset, SetAttribute, SetBackgroundColor, SetColors,
        SetForegroundColor,
    },
};
