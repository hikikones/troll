use std::fmt::{Display, Write};

use crate::{Color, Sgr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub attributes: Attributes,
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

    pub const fn with_crossed_out(self) -> Self {
        self.with_attribute(Attributes::CROSSED_OUT)
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

    pub const fn with_no_crossed_out(self) -> Self {
        self.with_attribute(Attributes::NOT_CROSSED_OUT)
    }

    pub const fn text<D: Display>(self, text: D) -> Styled<D> {
        Styled::new(text, self)
    }

    pub fn insert(&mut self, attr: Attributes) {
        self.attributes.insert(attr);
    }

    pub fn remove(&mut self, attr: Attributes) {
        self.attributes.remove(attr);
    }

    fn write_ansi_codes(
        &self,
        colors: impl IntoIterator<Item = Sgr>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        for sgr in colors.into_iter().chain(self.attributes.iter_sgr()) {
            sgr.write_ansi_code(f)?;
            f.write_char(';')?;
        }

        Ok(())
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

impl From<Color> for Style {
    fn from(color: Color) -> Self {
        Self::fg(color)
    }
}

impl From<Attributes> for Style {
    fn from(attr: Attributes) -> Self {
        Self::attributes(attr)
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

impl Attributes {
    fn as_sgr(self) -> Sgr {
        match self {
            Self::RESET => Sgr::Reset,
            Self::BOLD => Sgr::Bold,
            Self::FAINT => Sgr::Faint,
            Self::ITALIC => Sgr::Italic,
            Self::UNDERLINE => Sgr::Underline,
            Self::REVERSE => Sgr::Reverse,
            Self::CONCEAL => Sgr::Conceal,
            Self::CROSSED_OUT => Sgr::CrossedOut,
            Self::NOT_BOLD => Sgr::NotBold,
            Self::NOT_ITALIC => Sgr::NotItalic,
            Self::NOT_UNDERLINE => Sgr::NotUnderline,
            Self::NOT_REVERSE => Sgr::NotReverse,
            Self::NOT_CONCEAL => Sgr::NotConceal,
            Self::NOT_CROSSED_OUT => Sgr::NotCrossedOut,
            _ => unreachable!(),
        }
    }

    fn iter_sgr(self) -> impl Iterator<Item = Sgr> {
        self.iter().map(Self::as_sgr)
    }
}

impl Display for Attributes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("\x1b[")?;

        for sgr in self.iter_sgr() {
            sgr.write_ansi_code(f)?;
            f.write_char(';')?;
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
