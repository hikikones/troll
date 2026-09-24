use std::fmt::Display;

use crossterm::{
    Command,
    style::{Attributes, ContentStyle, SetStyle},
};

pub use crossterm::style::{
    Attribute, Color, ResetColor as Reset, SetAttribute, SetBackgroundColor, SetColors,
    SetForegroundColor,
};

#[derive(Debug, Clone, Copy)]
pub struct Style(ContentStyle);

impl Style {
    pub const fn empty() -> Self {
        Self(ContentStyle {
            foreground_color: None,
            background_color: None,
            underline_color: None,
            attributes: Attributes::none(),
        })
    }

    pub const fn fg(color: Color) -> Self {
        Self::empty().with_fg(color)
    }

    pub const fn bg(color: Color) -> Self {
        Self::empty().with_bg(color)
    }

    pub const fn bold() -> Self {
        Self::empty().with_bold()
    }

    pub const fn italic() -> Self {
        Self::empty().with_italic()
    }

    pub const fn reverse() -> Self {
        Self::empty().with_reverse()
    }

    pub const fn with_fg(mut self, color: Color) -> Self {
        self.0.foreground_color = Some(color);
        self
    }

    pub const fn with_bg(mut self, color: Color) -> Self {
        self.0.background_color = Some(color);
        self
    }

    pub const fn with_bold(self) -> Self {
        self.set_attr(Attribute::Bold)
    }

    pub const fn with_italic(self) -> Self {
        self.set_attr(Attribute::Italic)
    }

    pub const fn with_reverse(self) -> Self {
        self.set_attr(Attribute::Reverse)
    }

    pub const fn with_no_bold(self) -> Self {
        self.set_attr(Attribute::NoBold)
    }

    pub const fn with_no_italic(self) -> Self {
        self.set_attr(Attribute::NoItalic)
    }

    pub const fn with_no_reverse(self) -> Self {
        self.set_attr(Attribute::NoReverse)
    }

    const fn set_attr(mut self, attr: Attribute) -> Self {
        self.0.attributes = self.0.attributes.with(attr);
        self
    }

    pub const fn text<D: Display>(self, text: D) -> Styled<D> {
        Styled::new(text, self)
    }
}

impl Display for Style {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        SetStyle(self.0).write_ansi(f)
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
        Reset.write_ansi(f)
    }
}
