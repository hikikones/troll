use std::{cmp::Ordering, fmt::Display};

use utils::StringExt;

use crate::{
    AnsiExt, CellDims, Color, Cursor, CursorClear, CursorErase, CursorMoveDown, CursorToHome,
    CursorToPos, CursorVisibility, Dims, HorizontalAlignment, Pos, Rect, ScreenHeight, ScreenSize,
    ScreenSizeThresholds, ScreenWidth, Sgr, Size, TerminalSize, VerticalAlignment,
};

#[derive(Debug, Default)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::resource::Resource))]
pub struct Frame {
    buffer: String,
    text: String,
    size: TerminalSize,
    thresholds: ScreenSizeThresholds,
    screen_size: ScreenSize,
    cursor_state: CursorState,
    generation: i32,
}

impl Frame {
    pub const fn new() -> Self {
        Self {
            buffer: String::new(),
            text: String::new(),
            size: TerminalSize::new(),
            thresholds: ScreenSizeThresholds::DEFAULT,
            screen_size: ScreenSize::DEFAULT,
            cursor_state: CursorState::new(),
            generation: 0,
        }
    }

    pub const fn with_thresholds(mut self, thresholds: ScreenSizeThresholds) -> Self {
        self.thresholds = thresholds;
        self
    }

    pub fn query_size(&mut self) -> std::io::Result<()> {
        self.size = TerminalSize::query()?;
        self.screen_size = ScreenSize::new(self.size.window_size(), self.thresholds);

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

    pub const fn screen_width(&self) -> ScreenWidth {
        self.screen_size.width
    }

    pub const fn screen_height(&self) -> ScreenHeight {
        self.screen_size.height
    }

    pub const fn screen_size(&self) -> ScreenSize {
        self.screen_size
    }

    pub const fn area(&self) -> Rect {
        Rect::new(Pos::ZERO, self.size.window_size())
    }

    pub const fn generation(&self) -> i32 {
        self.generation
    }

    pub const fn increment_generation(&mut self) {
        self.generation += 1;
    }

    pub const fn set_cursor_visibility(&mut self, value: bool) {
        self.cursor_state.set_visible(value);
    }

    pub const fn reset_cursor_visibility(&mut self) {
        self.cursor_state.current_visibility = false;
    }

    pub const fn set_cursor_pos_at_end(&mut self, pos: Pos) {
        self.cursor_state.set_pos(pos);
    }

    pub fn cursor(&mut self, cursor: impl Into<Cursor>) {
        self.buffer.push_fmt(format_args!("{}", cursor.into()));
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
        self.buffer.push_str_fg(s, fg);
    }

    pub fn print_fmt(&mut self, text: impl Display) {
        self.buffer.push_fmt(text);
    }

    pub fn print_fmt_fg(&mut self, text: impl Display, fg: Color) {
        self.buffer.push_fmt_fg(text, fg);
    }

    pub fn print_iter<I>(&mut self, iter: I)
    where
        I: IntoIterator,
        String: Extend<I::Item>,
    {
        self.buffer.extend(iter);
    }

    pub fn print_span(&mut self, area: Rect, text: &str, options: SpanOptions) {
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
                    self.print_fmt(CursorToPos(pos));

                    let empty_left_count = start_text_col - pos.col;
                    self.print_ch_repeat(' ', empty_left_count);

                    self.buffer.push_str(text);

                    let empty_right_count = max_width - (empty_left_count + text_width);
                    self.print_ch_repeat(' ', empty_right_count);
                } else {
                    self.print_fmt(CursorToPos(pos.with_col(start_text_col)));
                    self.buffer.push_str(text);
                }
            }
            Ordering::Equal => {
                // Perfect fit, just print
                self.print_fmt(CursorToPos(pos));
                self.buffer.push_str(text);
            }
            Ordering::Greater => {
                // No fit, print what we can
                self.print_fmt(CursorToPos(pos));

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

    pub fn print_span_fg(&mut self, area: Rect, text: &str, color: Color, options: SpanOptions) {
        self.print_fmt(Sgr::Fg(color));
        self.print_span(area, text, options);
        self.print_fmt(Sgr::reset_fg());
    }

    pub fn fill(&mut self, area: Rect, bg: Color, reset: bool) {
        self.print_fmt(CursorToPos(area.pos));
        self.print_fmt(Sgr::Bg(bg));

        for _ in 0..area.size.rows {
            self.print_fmt(CursorErase(area.size.cols));
            self.print_fmt(CursorMoveDown(1));
        }

        if reset {
            self.print_fmt(Sgr::reset_bg());
        }
    }

    pub fn flush(&mut self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        if self.size.window_size().is_either_less(2) {
            self.clear();
            return Ok(());
        }

        let cursor_state = self.cursor_state.take();
        self.print_fmt(cursor_state);

        writer.write_fmt(format_args!("{}{}", CursorToHome, CursorClear::ToScreenEnd))?;
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

    pub fn push_str_fg(&mut self, text: &str, fg: Color) {
        self.text.push_str_fg(text, fg);
    }

    pub fn push_fmt(&mut self, text: impl Display) {
        self.text.push_fmt(text);
    }

    pub fn push_fmt_fg(&mut self, text: impl Display, fg: Color) {
        self.text.push_fmt_fg(text, fg);
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

    pub const fn text(&self) -> &String {
        &self.text
    }

    pub const fn text_mut(&mut self) -> &mut String {
        &mut self.text
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
                output.push_fmt(CursorToPos(pos));

                let empty_left_count = start_text_col - pos.col;
                output.extend(std::iter::repeat_n(' ', empty_left_count as usize));

                output.push_str(text);

                let empty_right_count = max_width - (empty_left_count + display_width);
                output.extend(std::iter::repeat_n(' ', empty_right_count as usize));
            } else {
                output.push_fmt(CursorToPos(pos.with_col(start_text_col)));
                output.push_str(text);
            }
        }
        Ordering::Equal => {
            // Perfect fit, just print
            output.push_fmt(CursorToPos(pos));
            output.push_str(text);
        }
        Ordering::Greater => {
            // No fit, print what we can and keep ansi codes
            output.push_fmt(CursorToPos(pos));

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
        if let Some(show_or_hide) = self.visibility {
            CursorVisibility::from_bool(show_or_hide).fmt(f)?;
        }

        if let Some(pos) = self.position {
            CursorToPos(pos).fmt(f)?;
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

    pub const fn with_horizontal(mut self, align: HorizontalAlignment) -> Self {
        self.horizontal = align;
        self
    }

    pub const fn with_vertical(mut self, align: VerticalAlignment) -> Self {
        self.vertical = align;
        self
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
