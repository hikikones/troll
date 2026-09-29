use std::fmt::{Display, Write};

use crossterm::{
    Command,
    cursor::{Hide, MoveTo, Show},
};

use crate::{Color, Pos};

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

#[derive(Debug, Clone, Copy)]
pub enum Sgr {
    // Reset
    Reset,

    // Style on
    Bold,
    Faint,
    Italic,
    Underline,
    // SlowBlink,
    // RapidBlink,
    Reverse,
    Conceal,
    CrossedOut,
    // Framed,
    // Encircled,
    // Overlined,

    // Style off
    NotBold,
    NotItalic,
    NotUnderline,
    // NotBlink,
    NotReverse,
    NotConceal,
    NotCrossedOut,
    // NotFramedOrEncircled,
    // NotOverlined,

    // Colors
    Fg(Color),
    Bg(Color),
}

impl Sgr {
    pub const fn reset_all() -> Self {
        Self::Reset
    }

    pub const fn reset_fg() -> Self {
        Self::Fg(Color::Default)
    }

    pub const fn reset_bg() -> Self {
        Self::Bg(Color::Default)
    }

    pub(crate) fn write_ansi_code(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reset => f.write_char('0'),

            Self::Bold => f.write_char('1'),
            Self::Faint => f.write_char('2'),
            Self::Italic => f.write_char('3'),
            Self::Underline => f.write_char('4'),
            // Self::SlowBlink => f.write_char('5'),
            // Self::RapidBlink => f.write_char('6'),
            Self::Reverse => f.write_char('7'),
            Self::Conceal => f.write_char('8'),
            Self::CrossedOut => f.write_char('9'),
            // Self::Framed => f.write_str("51"),
            // Self::Encircled => f.write_str("52"),
            // Self::Overlined => f.write_str("53"),

            //
            Self::NotBold => f.write_str("22"),
            Self::NotItalic => f.write_str("23"),
            Self::NotUnderline => f.write_str("24"),
            // Self::NotBlink => f.write_str("25"),
            Self::NotReverse => f.write_str("27"),
            Self::NotConceal => f.write_str("28"),
            Self::NotCrossedOut => f.write_str("29"),
            // Self::NotFramedOrEncircled => f.write_str("54"),
            // Self::NotOverlined => f.write_str("55"),

            //
            Self::Fg(Color::Black) => f.write_str("30"),
            Self::Fg(Color::Red) => f.write_str("31"),
            Self::Fg(Color::Green) => f.write_str("32"),
            Self::Fg(Color::Yellow) => f.write_str("33"),
            Self::Fg(Color::Blue) => f.write_str("34"),
            Self::Fg(Color::Magenta) => f.write_str("35"),
            Self::Fg(Color::Cyan) => f.write_str("36"),
            Self::Fg(Color::White) => f.write_str("37"),

            Self::Fg(Color::Default) => f.write_str("39"),

            Self::Fg(Color::BrightBlack) => f.write_str("90"),
            Self::Fg(Color::BrightRed) => f.write_str("91"),
            Self::Fg(Color::BrightGreen) => f.write_str("92"),
            Self::Fg(Color::BrightYellow) => f.write_str("93"),
            Self::Fg(Color::BrightBlue) => f.write_str("94"),
            Self::Fg(Color::BrightMagenta) => f.write_str("95"),
            Self::Fg(Color::BrightCyan) => f.write_str("96"),
            Self::Fg(Color::BrightWhite) => f.write_str("97"),

            Self::Bg(Color::Black) => f.write_str("40"),
            Self::Bg(Color::Red) => f.write_str("41"),
            Self::Bg(Color::Green) => f.write_str("42"),
            Self::Bg(Color::Yellow) => f.write_str("43"),
            Self::Bg(Color::Blue) => f.write_str("44"),
            Self::Bg(Color::Magenta) => f.write_str("45"),
            Self::Bg(Color::Cyan) => f.write_str("46"),
            Self::Bg(Color::White) => f.write_str("47"),

            Self::Bg(Color::Default) => f.write_str("49"),

            Self::Bg(Color::BrightBlack) => f.write_str("100"),
            Self::Bg(Color::BrightRed) => f.write_str("101"),
            Self::Bg(Color::BrightGreen) => f.write_str("102"),
            Self::Bg(Color::BrightYellow) => f.write_str("103"),
            Self::Bg(Color::BrightBlue) => f.write_str("104"),
            Self::Bg(Color::BrightMagenta) => f.write_str("105"),
            Self::Bg(Color::BrightCyan) => f.write_str("106"),
            Self::Bg(Color::BrightWhite) => f.write_str("107"),

            Self::Fg(Color::Indexed(n)) => f.write_fmt(format_args!("38;5;{}", n)),
            Self::Bg(Color::Indexed(n)) => f.write_fmt(format_args!("48;5;{}", n)),

            Self::Fg(Color::Rgb(r, g, b)) => f.write_fmt(format_args!("38;2;{};{};{}", r, g, b)),
            Self::Bg(Color::Rgb(r, g, b)) => f.write_fmt(format_args!("48;2;{};{};{}", r, g, b)),
        }
    }
}

impl Display for Sgr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("\x1b[")?;
        self.write_ansi_code(f)?;
        f.write_char('m')
    }
}

pub struct Sgrs<T>(pub T);

impl<T> Display for Sgrs<T>
where
    for<'a> &'a T: IntoIterator<Item = &'a Sgr>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("\x1b[")?;

        for sgr in self.0.into_iter() {
            sgr.write_ansi_code(f)?;
            f.write_char(';')?;
        }

        f.write_char('m')
    }
}
