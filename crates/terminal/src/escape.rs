use std::fmt::{Display, NumBuffer, Write};

use crate::{Color, Pos, Rgb};

pub(crate) const ESC: &str = "\x1b";
pub(crate) const BEL: &str = "\x07";

pub(crate) const CSI: &str = utils::concat!(ESC, "[");
pub(crate) const OSC: &str = utils::concat!(ESC, "]");
pub(crate) const SGR_END: &str = "m";
pub(crate) const SGR_END_CH: char = 'm';

#[derive(Debug, Clone, Copy)]
pub enum Cursor {
    Visibility(CursorVisibility),
    Move(CursorMove),
    Erase(u16),
    Clear(CursorClear),
}

impl Display for Cursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Visibility(cv) => cv.fmt(f),
            Self::Move(cm) => cm.fmt(f),
            Self::Erase(cols) => CursorErase(cols).fmt(f),
            Self::Clear(clear) => clear.fmt(f),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CursorVisibility {
    Hide,
    Show,
}

impl CursorVisibility {
    pub const fn from_bool(show: bool) -> Self {
        if show { Self::Show } else { Self::Hide }
    }
}

impl Display for CursorVisibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CursorVisibility::Hide => CursorHide.fmt(f),
            CursorVisibility::Show => CursorShow.fmt(f),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CursorHide;

impl Display for CursorHide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(CSI)?;
        f.write_str("?25l")
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CursorShow;

impl Display for CursorShow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(CSI)?;
        f.write_str("?25h")
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CursorMove {
    Home,
    Left(u16),
    Right(u16),
    Up(u16),
    Down(u16),
    Column(u16),
    Row(u16),
    Position(Pos),
}

impl Display for CursorMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Home => CursorToHome.fmt(f),
            Self::Left(n) => CursorMoveLeft(n).fmt(f),
            Self::Right(n) => CursorMoveRight(n).fmt(f),
            Self::Up(n) => CursorMoveUp(n).fmt(f),
            Self::Down(n) => CursorMoveDown(n).fmt(f),
            Self::Column(col) => CursorToColumn(col).fmt(f),
            Self::Row(row) => CursorToRow(row).fmt(f),
            Self::Position(pos) => CursorToPos(pos).fmt(f),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CursorToHome;

impl Display for CursorToHome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(CSI)?;
        f.write_char('H')
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CursorMoveLeft(pub u16);

impl Display for CursorMoveLeft {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(CSI)?;
        f.write_str(self.0.format_into(&mut NumBuffer::new()))?;
        f.write_char('D')
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CursorMoveRight(pub u16);

impl Display for CursorMoveRight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(CSI)?;
        f.write_str(self.0.format_into(&mut NumBuffer::new()))?;
        f.write_char('C')
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CursorMoveUp(pub u16);

impl Display for CursorMoveUp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(CSI)?;
        f.write_str(self.0.format_into(&mut NumBuffer::new()))?;
        f.write_char('A')
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CursorMoveDown(pub u16);

impl Display for CursorMoveDown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(CSI)?;
        f.write_str(self.0.format_into(&mut NumBuffer::new()))?;
        f.write_char('B')
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CursorToColumn(pub u16);

impl Display for CursorToColumn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(CSI)?;
        f.write_str((self.0 + 1).format_into(&mut NumBuffer::new()))?;
        f.write_char('G')
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CursorToRow(pub u16);

impl Display for CursorToRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(CSI)?;
        f.write_str((self.0 + 1).format_into(&mut NumBuffer::new()))?;
        f.write_char('d')
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CursorToPos(pub Pos);

impl Display for CursorToPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(CSI)?;

        let buf = &mut NumBuffer::new();
        f.write_str((self.0.row + 1).format_into(buf))?;
        f.write_char(';')?;
        f.write_str((self.0.col + 1).format_into(buf))?;
        f.write_char('H')
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CursorErase(pub u16);

impl Display for CursorErase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(CSI)?;
        f.write_str((self.0).format_into(&mut NumBuffer::new()))?;
        f.write_char('X')
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CursorClear {
    ToLineEnd,
    FromLineStart,
    EntireLine,
    ToScreenEnd,
    FromScreenStart,
    EntireScreen,
}

impl Display for CursorClear {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(CSI)?;

        match self {
            Self::ToLineEnd => f.write_char('K'),
            Self::FromLineStart => f.write_str("1K"),
            Self::EntireLine => f.write_str("2K"),
            Self::ToScreenEnd => f.write_char('J'),
            Self::FromScreenStart => f.write_str("1J"),
            Self::EntireScreen => f.write_str("2J"),
        }
    }
}

impl From<bool> for CursorVisibility {
    fn from(show: bool) -> Self {
        Self::from_bool(show)
    }
}

impl From<bool> for Cursor {
    fn from(show: bool) -> Self {
        Self::Visibility(CursorVisibility::from_bool(show))
    }
}

impl From<Pos> for CursorMove {
    fn from(pos: Pos) -> Self {
        Self::Position(pos)
    }
}

impl From<Pos> for Cursor {
    fn from(pos: Pos) -> Self {
        Self::Move(CursorMove::Position(pos))
    }
}

impl From<CursorMove> for Cursor {
    fn from(cm: CursorMove) -> Self {
        Self::Move(cm)
    }
}

impl From<CursorClear> for Cursor {
    fn from(clear: CursorClear) -> Self {
        Self::Clear(clear)
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
        match *self {
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

            Self::Fg(Color::BlackBright) => f.write_str("90"),
            Self::Fg(Color::RedBright) => f.write_str("91"),
            Self::Fg(Color::GreenBright) => f.write_str("92"),
            Self::Fg(Color::YellowBright) => f.write_str("93"),
            Self::Fg(Color::BlueBright) => f.write_str("94"),
            Self::Fg(Color::MagentaBright) => f.write_str("95"),
            Self::Fg(Color::CyanBright) => f.write_str("96"),
            Self::Fg(Color::WhiteBright) => f.write_str("97"),

            Self::Bg(Color::Black) => f.write_str("40"),
            Self::Bg(Color::Red) => f.write_str("41"),
            Self::Bg(Color::Green) => f.write_str("42"),
            Self::Bg(Color::Yellow) => f.write_str("43"),
            Self::Bg(Color::Blue) => f.write_str("44"),
            Self::Bg(Color::Magenta) => f.write_str("45"),
            Self::Bg(Color::Cyan) => f.write_str("46"),
            Self::Bg(Color::White) => f.write_str("47"),

            Self::Bg(Color::Default) => f.write_str("49"),

            Self::Bg(Color::BlackBright) => f.write_str("100"),
            Self::Bg(Color::RedBright) => f.write_str("101"),
            Self::Bg(Color::GreenBright) => f.write_str("102"),
            Self::Bg(Color::YellowBright) => f.write_str("103"),
            Self::Bg(Color::BlueBright) => f.write_str("104"),
            Self::Bg(Color::MagentaBright) => f.write_str("105"),
            Self::Bg(Color::CyanBright) => f.write_str("106"),
            Self::Bg(Color::WhiteBright) => f.write_str("107"),

            Self::Fg(Color::Indexed(n)) => {
                f.write_str("38;5;")?;
                f.write_str(n.format_into(&mut NumBuffer::new()))
            }
            Self::Bg(Color::Indexed(n)) => {
                f.write_str("48;5;")?;
                f.write_str(n.format_into(&mut NumBuffer::new()))
            }

            Self::Fg(Color::Rgb(Rgb(r, g, b))) => {
                f.write_str("38;2;")?;

                let buf = &mut NumBuffer::new();
                f.write_str(r.format_into(buf))?;
                f.write_char(';')?;
                f.write_str(g.format_into(buf))?;
                f.write_char(';')?;
                f.write_str(b.format_into(buf))
            }
            Self::Bg(Color::Rgb(Rgb(r, g, b))) => {
                f.write_str("48;2;")?;

                let buf = &mut NumBuffer::new();
                f.write_str(r.format_into(buf))?;
                f.write_char(';')?;
                f.write_str(g.format_into(buf))?;
                f.write_char(';')?;
                f.write_str(b.format_into(buf))
            }
        }
    }
}

impl Display for Sgr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(CSI)?;
        self.write_ansi_code(f)?;
        f.write_char(SGR_END_CH)
    }
}

pub struct Sgrs<T>(pub T);

impl<T> Display for Sgrs<T>
where
    for<'a> &'a T: IntoIterator<Item = &'a Sgr>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(CSI)?;

        for sgr in self.0.into_iter() {
            sgr.write_ansi_code(f)?;
            f.write_char(';')?;
        }

        f.write_char(SGR_END_CH)
    }
}

/// Control Sequence Introducer (CSI) sequences.
#[derive(Debug, Clone, Copy)]
pub enum Csi {
    /// Query the terminal's status.
    DeviceStatusReport,
}

impl Display for Csi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeviceStatusReport => f.write_str(utils::concat!(CSI, "5n")),
        }
    }
}

/// Operating System Command (OSC) sequences.
#[derive(Debug, Clone, Copy)]
pub enum Osc {
    /// OSC 10: query the terminal's default foreground color.
    ForegroundColor,
    /// OSC 11: query the terminal's default background color.
    BackgroundColor,
    /// OSC 12: query the terminal's cursor color.
    CursorColor,
}

impl Display for Osc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ForegroundColor => f.write_str(utils::concat!(OSC, "10;?", BEL)),
            Self::BackgroundColor => f.write_str(utils::concat!(OSC, "11;?", BEL)),
            Self::CursorColor => f.write_str(utils::concat!(OSC, "12;?", BEL)),
        }
    }
}
