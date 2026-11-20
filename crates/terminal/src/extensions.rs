use std::fmt::Display;

use crate::{Color, Sgr, Style, Styled};

pub trait AnsiExt {
    fn push_str_fg(&mut self, text: &str, fg: Color);
    fn push_fmt_fg(&mut self, text: impl Display, fg: Color);
}

impl AnsiExt for String {
    fn push_str_fg(&mut self, text: &str, fg: Color) {
        self.push_fmt_fg(text, fg);
    }

    fn push_fmt_fg(&mut self, text: impl Display, fg: Color) {
        let fmt = format_args!("{}{text}{}", Sgr::Fg(fg), Sgr::reset_fg());
        let _ = std::fmt::Write::write_fmt(self, fmt);
    }
}

pub trait Stylize<D: Display> {
    fn fg(self, color: Color) -> Styled<D>;
    fn bg(self, color: Color) -> Styled<D>;
    fn colors(self, fg: Color, bg: Color) -> Styled<D>;
    fn bold(self) -> Styled<D>;
    fn italic(self) -> Styled<D>;
    fn bold_fg(self, color: Color) -> Styled<D>;
    fn italic_fg(self, color: Color) -> Styled<D>;
}

impl<D: Display> Stylize<D> for D {
    fn fg(self, color: Color) -> Styled<D> {
        Styled::new(self, Style::fg(color))
    }

    fn bg(self, color: Color) -> Styled<D> {
        Styled::new(self, Style::bg(color))
    }

    fn colors(self, fg: Color, bg: Color) -> Styled<D> {
        Styled::new(self, Style::colors(fg, bg))
    }

    fn bold(self) -> Styled<D> {
        Styled::new(self, Style::bold())
    }

    fn italic(self) -> Styled<D> {
        Styled::new(self, Style::italic())
    }

    fn bold_fg(self, color: Color) -> Styled<D> {
        Styled::new(self, Style::bold_fg(color))
    }

    fn italic_fg(self, color: Color) -> Styled<D> {
        Styled::new(self, Style::italic_fg(color))
    }
}
