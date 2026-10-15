use std::{
    cmp::Ordering,
    fmt::{Display, Write},
};

use crossterm::{
    Command,
    cursor::{Hide, MoveTo, Show},
    terminal::{Clear, ClearType},
};

use crate::{
    CellDims, Color, Cursor, Dims, HorizontalAlignment, Pos, Rect, Sgr, Size, TerminalSize,
    VerticalAlignment,
};

#[derive(Debug, Default)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::resource::Resource))]
pub struct Framebuffer {
    size: TerminalSize,
    buffer: String,
    text: String,
    cursor_state: CursorState,
}

impl Framebuffer {
    pub const fn new() -> Self {
        Self {
            size: TerminalSize::new(),
            buffer: String::new(),
            text: String::new(),
            cursor_state: CursorState::new(),
        }
    }

    pub fn query_size(&mut self) -> std::io::Result<()> {
        self.size = TerminalSize::query()?;
        Ok(())
    }

    pub const fn window_size(&self) -> Size {
        self.size.window_size()
    }

    pub const fn window_dims(&self) -> Option<Dims> {
        self.size.window_dims()
    }

    pub const fn cell_dims(&self) -> Option<CellDims> {
        self.size.cell_dims()
    }

    pub const fn area(&self) -> Rect {
        Rect::new(Pos::ZERO, self.size.window_size())
    }

    pub const fn set_cursor_visibility(&mut self, value: bool) {
        self.cursor_state.set_visible(value);
    }

    pub const fn set_cursor_pos_at_end(&mut self, pos: Pos) {
        self.cursor_state.set_pos(pos);
    }

    pub fn cursor(&mut self, cursor: impl Into<Cursor>) {
        let _ = write!(self.buffer, "{}", cursor.into());
    }

    pub fn print_ch(&mut self, ch: char) {
        self.buffer.push(ch);
    }

    pub fn print_ch_repeat(&mut self, ch: char, n: u16) {
        self.buffer.extend(std::iter::repeat_n(ch, n as usize));
    }

    pub fn print_str(&mut self, s: &str) {
        self.buffer.push_str(s);
    }

    pub fn print_str_fg(&mut self, s: &str, fg: Color) {
        let _ = write!(self.buffer, "{}{s}{}", Sgr::Fg(fg), Sgr::reset_fg());
    }

    pub fn print_fmt(&mut self, text: impl Display) {
        let _ = write!(self.buffer, "{text}");
    }

    pub fn print_fmt_fg(&mut self, text: impl Display, fg: Color) {
        let _ = write!(self.buffer, "{}{text}{}", Sgr::Fg(fg), Sgr::reset_fg());
    }

    pub fn print_iter<I>(&mut self, iter: I)
    where
        I: IntoIterator,
        String: Extend<I::Item>,
    {
        self.buffer.extend(iter);
    }

    pub fn print_span(&mut self, area: Rect, text: &str) {
        self.print_span_with(area, text, SpanOptions::left());
    }

    pub fn print_span_with(&mut self, area: Rect, text: &str, options: SpanOptions) {
        let SpanOptions {
            horizontal,
            vertical,
            fill,
        } = options;

        // TODO: Newlines should be ignored?

        let row = vertical.calc(area.pos.row, area.size.rows, 1);
        let pos = area.pos.with_row(row);
        let max_width = area.size.cols;

        let text_width = utils::str_width(text);
        match text_width.cmp(&max_width) {
            Ordering::Less => {
                // Room to spare, apply alignment
                let start_text_col = horizontal.calc(pos.col, max_width, text_width);

                if fill {
                    // Fill remaining empty cells with spaces
                    let _ = MoveTo(pos.col, pos.row).write_ansi(&mut self.buffer);

                    let empty_left_count = start_text_col - pos.col;
                    self.print_ch_repeat(' ', empty_left_count);

                    self.buffer.push_str(text);

                    let empty_right_count = max_width - (empty_left_count + text_width);
                    self.print_ch_repeat(' ', empty_right_count);
                } else {
                    let _ = MoveTo(start_text_col, pos.row).write_ansi(&mut self.buffer);
                    self.buffer.push_str(text);
                }
            }
            Ordering::Equal => {
                // Perfect fit, just print
                let _ = MoveTo(pos.col, pos.row).write_ansi(&mut self.buffer);
                self.buffer.push_str(text);
            }
            Ordering::Greater => {
                // No fit, print what we can
                let _ = MoveTo(pos.col, pos.row).write_ansi(&mut self.buffer);

                let mut width = 0;
                for g in utils::graphemes(text) {
                    let w = utils::str_width(g.0);

                    if width + w > max_width {
                        if fill {
                            let remaining = max_width - width;
                            self.print_ch_repeat(' ', remaining);
                        }
                        break;
                    }

                    width += w;
                    self.buffer.push_str(g.0);
                }
            }
        }
    }

    pub fn fill(&mut self, area: Rect, bg: Color, reset: bool) {
        self.print_fmt(Sgr::Bg(bg));

        for i in 0..area.size.rows {
            let _ = MoveTo(area.pos.col, area.pos.row + i).write_ansi(&mut self.buffer);
            self.print_ch_repeat(' ', area.size.cols);
        }

        if reset {
            self.print_fmt(Sgr::reset_bg());
        }
    }

    pub fn flush(&mut self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        use crossterm::QueueableCommand;

        let size = self.size.window_size();
        if size.is_either_less(2) {
            self.clear();
            return Ok(());
        }

        for i in 0..size.rows {
            writer
                .queue(MoveTo(0, i))?
                .queue(Clear(ClearType::CurrentLine))?;
        }

        let _ = write!(self.buffer, "{}", self.cursor_state.take());

        writer.write_all(self.buffer.as_bytes())?;
        writer.flush()?;

        self.clear();

        Ok(())
    }

    pub fn push_ch(&mut self, ch: char) {
        self.text.push(ch);
    }

    pub fn push_ch_repeat(&mut self, ch: char, n: u16) {
        self.text.extend(std::iter::repeat_n(ch, n as usize));
    }

    pub fn push_str(&mut self, s: &str) {
        self.text.push_str(s);
    }

    pub fn push_str_fg(&mut self, s: &str, fg: Color) {
        let _ = write!(self.text, "{}{s}{}", Sgr::Fg(fg), Sgr::reset_fg());
    }

    pub fn push_fmt(&mut self, text: impl Display) {
        let _ = write!(self.text, "{text}");
    }

    pub fn push_fmt_fg(&mut self, text: impl Display, fg: Color) {
        let _ = write!(self.text, "{}{text}{}", Sgr::Fg(fg), Sgr::reset_fg());
    }

    pub fn push_iter<I>(&mut self, iter: I)
    where
        I: IntoIterator,
        String: Extend<I::Item>,
    {
        self.text.extend(iter);
    }

    pub fn render(&mut self, mut area: Rect, opts: TextOptions) {
        if area.is_empty() {
            self.text.clear();
            return;
        }

        match opts.mode {
            TextMode::Span => {
                let row = opts.vertical.calc(area.pos.row, area.size.rows, 1);
                print_span_with_ansi(
                    area.pos.with_row(row),
                    area.size.cols,
                    &self.text, // TODO: Newlines should be ignored?
                    opts.horizontal,
                    opts.fill,
                    &mut self.buffer,
                );
            }
            TextMode::Paragraph => {
                utils::text_wrap(&mut self.text, area.size.cols);
                let lines = self.text.lines().count() as u16;

                area.pos.row = opts.vertical.calc(area.pos.row, area.size.rows, lines);
                area.size.rows = area.size.rows.min(lines);

                for line in self.text.lines().take(area.size.rows as usize) {
                    print_span_with_ansi(
                        area.pos,
                        area.size.cols,
                        line,
                        opts.horizontal,
                        opts.fill,
                        &mut self.buffer,
                    );
                    area.pos.row += 1;
                }

                if lines > area.size.rows {
                    self.print_fmt(Sgr::Reset);
                }
            }
        }

        self.text.clear();
    }

    fn clear(&mut self) {
        self.buffer.clear();
        self.text.clear();
    }
}

fn print_span_with_ansi(
    pos: Pos,
    max_width: u16,
    text: &str,
    align: HorizontalAlignment,
    fill: bool,
    output: &mut String,
) {
    let display_width = utils::display_width(text);
    match display_width.cmp(&max_width) {
        Ordering::Less => {
            // Room to spare, apply alignment
            let start_text_col = align.calc(pos.col, max_width, display_width);

            if fill {
                // Fill remaining empty cells with spaces
                let _ = MoveTo(pos.col, pos.row).write_ansi(output);

                let empty_left_count = start_text_col - pos.col;
                output.extend(std::iter::repeat_n(' ', empty_left_count as usize));

                output.push_str(text);

                let empty_right_count = max_width - (empty_left_count + display_width);
                output.extend(std::iter::repeat_n(' ', empty_right_count as usize));
            } else {
                let _ = MoveTo(start_text_col, pos.row).write_ansi(output);
                output.push_str(text);
            }
        }
        Ordering::Equal => {
            // Perfect fit, just print
            let _ = MoveTo(pos.col, pos.row).write_ansi(output);
            output.push_str(text);
        }
        Ordering::Greater => {
            // No fit, print what we can and keep ansi codes
            let _ = MoveTo(pos.col, pos.row).write_ansi(output);

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
                                output.extend(std::iter::repeat_n(' ', remaining as usize));
                            }
                            width = max_width;
                            continue;
                        }

                        width += w;
                        output.push_str(g.0);
                    }
                    utils::GraphemeOrAnsi::Ansi(s) => {
                        output.push_str(s);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
struct CursorState {
    current_visibility: bool,
    next_visibility: bool,
    position: Option<Pos>,
}

impl CursorState {
    const fn new() -> Self {
        Self {
            current_visibility: false,
            next_visibility: false,
            position: None,
        }
    }

    const fn set_visible(&mut self, value: bool) {
        self.next_visibility = value;
    }

    const fn set_pos(&mut self, pos: Pos) {
        self.position = Some(pos);
    }

    const fn take(&mut self) -> CursorStateCommand {
        if self.current_visibility != self.next_visibility {
            self.current_visibility = self.next_visibility;
            CursorStateCommand {
                visibility: Some(self.next_visibility),
                position: self.position.take(),
            }
        } else {
            CursorStateCommand {
                visibility: None,
                position: self.position.take(),
            }
        }
    }
}

impl Default for CursorState {
    fn default() -> Self {
        Self::new()
    }
}

struct CursorStateCommand {
    visibility: Option<bool>,
    position: Option<Pos>,
}

impl std::fmt::Display for CursorStateCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(visible) = self.visibility {
            match visible {
                true => Show.write_ansi(f)?,
                false => Hide.write_ansi(f)?,
            }
        }

        if let Some(Pos { col, row }) = self.position {
            MoveTo(col, row).write_ansi(f)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SpanOptions {
    pub horizontal: HorizontalAlignment,
    pub vertical: VerticalAlignment,
    pub fill: bool,
}

impl SpanOptions {
    pub const fn left() -> Self {
        Self {
            horizontal: HorizontalAlignment::Left,
            vertical: VerticalAlignment::Top,
            fill: false,
        }
    }

    pub const fn center() -> Self {
        Self {
            horizontal: HorizontalAlignment::Center,
            vertical: VerticalAlignment::Center,
            fill: false,
        }
    }

    pub const fn center_top() -> Self {
        Self {
            horizontal: HorizontalAlignment::Center,
            vertical: VerticalAlignment::Top,
            fill: false,
        }
    }

    pub const fn right() -> Self {
        Self {
            horizontal: HorizontalAlignment::Right,
            vertical: VerticalAlignment::Top,
            fill: false,
        }
    }

    pub const fn fill() -> Self {
        Self {
            horizontal: HorizontalAlignment::Left,
            vertical: VerticalAlignment::Top,
            fill: true,
        }
    }

    pub const fn with_fill(mut self) -> Self {
        self.fill = true;
        self
    }
}

impl Default for SpanOptions {
    fn default() -> Self {
        Self::left()
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
