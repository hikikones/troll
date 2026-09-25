use std::fmt::{Display, Write};

#[derive(Debug, Clone, Copy)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Debug, Clone, Copy)]
pub struct Style {
    fg: Option<Color>,
    bg: Option<Color>,
    attributes: Attributes,
}

impl Style {
    pub const fn empty() -> Self {
        Self {
            fg: None,
            bg: None,
            attributes: Attributes::empty(),
        }
    }

    pub const fn fg(color: Color) -> Self {
        Self {
            fg: Some(color),
            bg: None,
            attributes: Attributes::empty(),
        }
    }

    pub const fn bg(color: Color) -> Self {
        Self {
            fg: None,
            bg: Some(color),
            attributes: Attributes::empty(),
        }
    }

    pub const fn attributes(attributes: Attributes) -> Self {
        Self {
            fg: None,
            bg: None,
            attributes,
        }
    }

    pub const fn bold() -> Self {
        Self::attributes(Attributes::BOLD)
    }

    pub const fn italic() -> Self {
        Self::attributes(Attributes::ITALIC)
    }

    pub const fn reverse() -> Self {
        Self::attributes(Attributes::REVERSE)
    }

    pub const fn with_fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    pub const fn with_bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    pub const fn with_attribute(mut self, attr: Attributes) -> Self {
        self.attributes = self.attributes.union(attr);
        self
    }

    pub const fn with_bold(self) -> Self {
        self.with_attribute(Attributes::BOLD)
    }

    pub const fn with_italic(self) -> Self {
        self.with_attribute(Attributes::ITALIC)
    }

    pub const fn with_reverse(self) -> Self {
        self.with_attribute(Attributes::REVERSE)
    }

    pub const fn with_no_bold(self) -> Self {
        self.with_attribute(Attributes::NOT_BOLD)
    }

    pub const fn with_no_italic(self) -> Self {
        self.with_attribute(Attributes::NOT_ITALIC)
    }

    pub const fn with_no_reverse(self) -> Self {
        self.with_attribute(Attributes::NOT_REVERSE)
    }

    pub const fn text<D: Display>(self, text: D) -> Styled<D> {
        Styled::new(text, self)
    }

    fn write_ansi_codes(
        &self,
        colors: impl IntoIterator<Item = Sgr>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        for sgr in colors.into_iter().chain(self.attributes_as_sgr()) {
            sgr.write_ansi_code(f)?;
            f.write_char(';')?;
        }

        Ok(())
    }

    pub fn attributes_as_sgr(&self) -> impl Iterator<Item = Sgr> {
        self.attributes.iter().map(|a| match a.bits() {
            1 => Sgr::Reset,
            2 => Sgr::Bold,
            4 => Sgr::Faint,
            8 => Sgr::Italic,
            16 => Sgr::Underline,
            // => Sgr::SlowBlink,
            // => Sgr::RapidBlink,
            32 => Sgr::Reverse,
            64 => Sgr::Conceal,
            128 => Sgr::CrossedOut,
            // => Sgr::Framed,
            // => Sgr::Encircled,
            // => Sgr::Overlined,
            256 => Sgr::NotBold,
            512 => Sgr::NotItalic,
            1024 => Sgr::NotUnderline,
            // => Sgr::NotBlink,
            2048 => Sgr::NotReverse,
            4096 => Sgr::NotConceal,
            8192 => Sgr::NotCrossedOut,
            // => Sgr::NotFramedOrEncircled,
            // => Sgr::NotOverlined,
            _ => unreachable!(),
        })
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct Attributes: u16 {
        const RESET = 1;
        const BOLD = 2;
        const FAINT = 4;
        const ITALIC = 8;
        const UNDERLINE = 16;
        // const SLOW_BLINK
        // const RAPID_BLINK
        const REVERSE = 32;
        const CONCEAL = 64;
        const CROSSED_OUT = 128;
        // const FRAMED
        // const ENCIRCLED
        // const OVERLINED
        const NOT_BOLD = 256;
        const NOT_ITALIC = 512;
        const NOT_UNDERLINE = 1024;
        // const NOT_BLINK
        const NOT_REVERSE = 2048;
        const NOT_CONCEAL = 4096;
        const NOT_CROSSED_OUT = 8192;
        // const NOT_FRAMED_OR_ENCIRCLED
        // const NOT_OVERLINED
    }
}

impl Display for Style {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("\x1b[")?;

        match (self.fg, self.bg) {
            (None, None) => {
                self.write_ansi_codes([], f)?;
            }
            (Some(fg), None) => {
                self.write_ansi_codes([Sgr::Fg(fg)], f)?;
            }
            (None, Some(bg)) => {
                self.write_ansi_codes([Sgr::Bg(bg)], f)?;
            }
            (Some(fg), Some(bg)) => {
                self.write_ansi_codes([Sgr::Fg(fg), Sgr::Bg(bg)], f)?;
            }
        }

        f.write_char('m')
    }
}

#[derive(Debug)]
pub struct Styled<D: Display> {
    text: D,
    style: Style,
}

impl<D: Display> Styled<D> {
    pub const fn new(text: D, style: Style) -> Self {
        Self { text, style }
    }
}

impl<D: Display> Display for Styled<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.style.fmt(f)?;
        self.text.fmt(f)?;
        Sgr::Reset.fmt(f)
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

    fn write_ansi_code(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

pub struct SetSgr<T>(pub T);

impl<T> Display for SetSgr<T>
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
