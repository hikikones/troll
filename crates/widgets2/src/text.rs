use std::{
    cmp::Ordering,
    fmt::{Display, Write},
};

use terminal::*;

#[derive(Debug, Default)]
pub struct Text(String);

impl Text {
    pub const fn new() -> Self {
        Self(String::new())
    }

    pub fn push_ch(&mut self, ch: char) {
        self.0.push(ch);
    }

    pub fn push_ch_repeat(&mut self, ch: char, n: u16) {
        self.0.extend(std::iter::repeat_n(ch, n as usize));
    }

    pub fn push_str(&mut self, s: &str) {
        self.0.push_str(s);
    }

    pub fn push_str_fg(&mut self, s: &str, fg: Color) {
        let _ = write!(self.0, "{}{s}{}", Sgr::Fg(fg), Sgr::reset_fg());
    }

    pub fn push_fmt(&mut self, text: impl Display) {
        let _ = write!(self.0, "{text}");
    }

    pub fn push_fmt_fg(&mut self, text: impl Display, fg: Color) {
        let _ = write!(self.0, "{}{text}{}", Sgr::Fg(fg), Sgr::reset_fg());
    }

    pub fn push_iter<I>(&mut self, iter: I)
    where
        I: IntoIterator,
        String: Extend<I::Item>,
    {
        self.0.extend(iter);
    }

    pub fn render(&mut self, mut area: Rect, frame: &mut Framebuffer, opts: TextOptions) {
        if area.is_empty() {
            self.clear();
            return;
        }

        match opts.mode {
            TextMode::Span => {
                let row = opts.vertical.calc(area.pos.row, area.size.rows, 1);
                print_span(
                    area.pos.with_row(row),
                    area.size.cols,
                    &self.0, // TODO: Newlines should be ignored?
                    opts.horizontal,
                    opts.fill,
                    frame,
                );
            }
            TextMode::Paragraph => {
                utils::text_wrap(&mut self.0, area.size.cols);
                let lines = self.0.lines().count() as u16;

                area.pos.row = opts.vertical.calc(area.pos.row, area.size.rows, lines);
                area.size.rows = area.size.rows.min(lines);

                for line in self.0.lines().take(area.size.rows as usize) {
                    print_span(
                        area.pos,
                        area.size.cols,
                        line,
                        opts.horizontal,
                        opts.fill,
                        frame,
                    );
                    area.pos.row += 1;
                }

                if lines > area.size.rows {
                    frame.print_fmt(Sgr::Reset);
                }
            }
        }

        self.clear();
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}

fn print_span(
    pos: Pos,
    max_width: u16,
    text: &str,
    align: HorizontalAlignment,
    fill: bool,
    frame: &mut Framebuffer,
) {
    let display_width = utils::display_width(text) as u16;
    match display_width.cmp(&max_width) {
        Ordering::Less => {
            // Room to spare, apply alignment
            let start_text_col = align.calc(pos.col, max_width, display_width);

            if fill {
                // Fill remaining empty cells with spaces
                frame.cursor(pos);

                let empty_left_count = start_text_col - pos.col;
                frame.print_ch_repeat(' ', empty_left_count);

                frame.print_str(text);

                let empty_right_count = max_width - (empty_left_count + display_width);
                frame.print_ch_repeat(' ', empty_right_count);
            } else {
                frame.cursor(pos.with_col(start_text_col));
                frame.print_str(text);
            }
        }
        Ordering::Equal => {
            // Perfect fit, just print
            frame.cursor(pos);
            frame.print_str(text);
        }
        Ordering::Greater => {
            // No fit, print what we can and keep ansi codes
            frame.cursor(pos);

            let mut width = 0;
            for g in utils::GraphemeAnsiIter::new(text) {
                match g {
                    utils::GraphemeOrAnsi::Grapheme(g) => {
                        if width == max_width {
                            continue;
                        }

                        let w = utils::str_width(g.0);

                        if width + w > max_width {
                            if fill {
                                let remaining = max_width - width;
                                frame.print_ch_repeat(' ', remaining);
                            }
                            width = max_width;
                            continue;
                        }

                        width += w;
                        frame.print_str(g.0);
                    }
                    utils::GraphemeOrAnsi::Ansi(s) => {
                        frame.print_str(s);
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TextMode {
    Span,
    Paragraph,
}

#[derive(Debug, Clone, Copy)]
pub struct TextOptions {
    pub mode: TextMode,
    pub horizontal: HorizontalAlignment,
    pub vertical: VerticalAlignment,
    pub fill: bool,
}

impl TextOptions {
    pub const fn span() -> Self {
        Self {
            mode: TextMode::Span,
            horizontal: HorizontalAlignment::Left,
            vertical: VerticalAlignment::Top,
            fill: false,
        }
    }

    pub const fn span_center() -> Self {
        Self {
            mode: TextMode::Span,
            horizontal: HorizontalAlignment::Center,
            vertical: VerticalAlignment::Center,
            fill: false,
        }
    }

    pub const fn span_center_top() -> Self {
        Self {
            mode: TextMode::Span,
            horizontal: HorizontalAlignment::Center,
            vertical: VerticalAlignment::Top,
            fill: false,
        }
    }

    pub const fn span_right() -> Self {
        Self {
            mode: TextMode::Span,
            horizontal: HorizontalAlignment::Right,
            vertical: VerticalAlignment::Top,
            fill: false,
        }
    }

    pub const fn paragraph() -> Self {
        Self {
            mode: TextMode::Paragraph,
            horizontal: HorizontalAlignment::Left,
            vertical: VerticalAlignment::Top,
            fill: false,
        }
    }

    pub const fn paragraph_center() -> Self {
        Self {
            mode: TextMode::Paragraph,
            horizontal: HorizontalAlignment::Center,
            vertical: VerticalAlignment::Center,
            fill: false,
        }
    }

    pub const fn paragraph_right() -> Self {
        Self {
            mode: TextMode::Paragraph,
            horizontal: HorizontalAlignment::Right,
            vertical: VerticalAlignment::Top,
            fill: false,
        }
    }

    pub const fn with_fill(mut self) -> Self {
        self.fill = true;
        self
    }
}

impl Default for TextOptions {
    fn default() -> Self {
        Self::span()
    }
}
