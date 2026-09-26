use std::fmt::Display;

use crossterm::{
    Command,
    cursor::{Hide, MoveTo, Show},
};

use crate::Pos;

#[derive(Debug, Clone, Copy)]
pub enum Cursor {
    Hide,
    Show,
    Move(u16, u16),
}

impl Display for Cursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Cursor::Hide => Hide.write_ansi(f),
            Cursor::Show => Show.write_ansi(f),
            Cursor::Move(col, row) => MoveTo(col, row).write_ansi(f),
        }
    }
}

impl From<Pos> for Cursor {
    fn from(pos: Pos) -> Self {
        let Pos { col, row } = pos;
        Self::Move(col, row)
    }
}

impl From<(u16, u16)> for Cursor {
    fn from((col, row): (u16, u16)) -> Self {
        Self::Move(col, row)
    }
}
