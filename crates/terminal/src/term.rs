use std::{
    fmt::{Display, Write},
    io::Stdout,
};

use crossterm::{
    Command,
    cursor::{Hide, MoveTo, Show},
    execute,
    terminal::{Clear, ClearType, DisableLineWrap, EnterAlternateScreen, LeaveAlternateScreen},
};

use crate::{
    Color, Cursor, HorizontalAlignment, Pos, Rect, Sgr, Size, TerminalEvent, VerticalAlignment,
};

pub struct Terminal {
    backend: Stdout,
    buffer: Framebuffer,
}

impl Terminal {
    pub fn new() -> std::io::Result<Self> {
        let size = TerminalSize::query()?;

        let mut backend = std::io::stdout();
        let palette = TerminalPalette::query(&mut backend, &mut std::io::stdin())?;

        Ok(Self {
            backend,
            buffer: Framebuffer::new(size, palette),
        })
    }

    pub const fn with_thresholds(mut self, thresholds: ScreenSizeThresholds) -> Self {
        self.buffer.thresholds = thresholds;
        self
    }

    pub fn enter<T>(
        mut self,
        f: impl FnOnce(&mut Self) -> Result<T, Box<dyn std::error::Error>>,
    ) -> Result<T, Box<dyn std::error::Error>> {
        Self::set_panic_hook();

        Self::enter_alternate_screen(&mut self.backend)?;
        let res = f(&mut self);
        Self::leave_alternate_screen(&mut self.backend)?;

        res
    }

    /// Reads a terminal event in a blocking manner.
    pub fn read() -> std::io::Result<Option<TerminalEvent>> {
        crossterm::event::read().map(|ev| TerminalEvent::from(ev))
    }

    /// Polls and reads a terminal event in a non-blocking manner.
    pub fn poll(timeout: std::time::Duration) -> std::io::Result<Option<TerminalEvent>> {
        match crossterm::event::poll(timeout) {
            Ok(true) => Self::read(),
            Ok(false) => Ok(None),
            Err(err) => Err(err),
        }
    }

    pub const fn frame(&mut self) -> &mut Framebuffer {
        &mut self.buffer
    }

    pub fn render(
        &mut self,
        f: impl FnOnce(&mut Framebuffer) -> std::io::Result<()>,
    ) -> std::io::Result<()> {
        // Move this to a method on buffer? Caller can then decide when to query again.
        self.buffer.size = TerminalSize::from(crossterm::terminal::window_size()?);

        f(&mut self.buffer)?;

        self.buffer.flush(&mut self.backend.lock())
    }

    pub fn temp_leave<T>(&mut self, f: impl FnOnce() -> std::io::Result<T>) -> std::io::Result<T> {
        Self::leave_alternate_screen(&mut self.backend)?;
        let t = f();
        Self::enter_alternate_screen(&mut self.backend)?;
        t
    }

    fn enter_alternate_screen(backend: &mut impl std::io::Write) -> std::io::Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        execute!(backend, EnterAlternateScreen, Hide, DisableLineWrap)
    }

    fn leave_alternate_screen(backend: &mut impl std::io::Write) -> std::io::Result<()> {
        crossterm::terminal::disable_raw_mode()?;
        execute!(backend, LeaveAlternateScreen)
    }

    fn set_panic_hook() {
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if let Err(err) = Self::leave_alternate_screen(&mut std::io::stdout()) {
                std::eprintln!("Failed to restore terminal: {err}");
            }
            hook(info);
        }));
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TerminalSize {
    cols: u16,
    rows: u16,
    width: u16,
    height: u16,
}

impl TerminalSize {
    fn query() -> std::io::Result<Self> {
        let win_size = crossterm::terminal::window_size()?;

        if win_size.width == 0 || win_size.height == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "terminal does not support query for window size in pixel dimensions",
            ));
        }

        Ok(Self::from(win_size))
    }

    const fn from(win_size: crossterm::terminal::WindowSize) -> Self {
        Self {
            cols: win_size.columns,
            rows: win_size.rows,
            width: win_size.width,
            height: win_size.height,
        }
    }

    pub const fn window_size(&self) -> Size {
        Size {
            cols: self.cols,
            rows: self.rows,
        }
    }

    pub const fn window_dims(&self) -> Dims {
        Dims {
            width: self.width,
            height: self.height,
        }
    }

    pub const fn cell_dims(&self) -> CellDims {
        CellDims {
            width: self.width / self.cols,
            height: self.height / self.rows,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenSize {
    Small,
    Medium,
    Large,
}

impl ScreenSize {
    pub const fn new(window_size: Size, thresholds: ScreenSizeThresholds) -> ScreenSize {
        let ScreenSizeThresholds { medium, large } = thresholds;
        match (window_size.cols, window_size.rows) {
            (w, h) if w < medium.cols || h < medium.rows => Self::Small,
            (w, h) if w < large.cols || h < large.rows => Self::Medium,
            _ => Self::Large,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ScreenSizeThresholds {
    pub medium: Size,
    pub large: Size,
}

impl ScreenSizeThresholds {
    pub const DEFAULT: Self = Self {
        medium: Size::new(68, 20),
        large: Size::new(108, 30),
    };

    pub const fn new(medium: Size, large: Size) -> Self {
        Self { medium, large }
    }
}

impl Default for ScreenSizeThresholds {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Pixel dimensions.
#[derive(Debug, Clone, Copy)]
pub struct Dims {
    pub width: u16,
    pub height: u16,
}

impl Dims {
    pub const ZERO: Self = Self {
        width: 0,
        height: 0,
    };

    pub const SD: Self = Self {
        width: 720,
        height: 480,
    };

    pub const HD: Self = Self {
        width: 1280,
        height: 720,
    };

    pub const FULL_HD: Self = Self {
        width: 1920,
        height: 1080,
    };

    pub const ULTRA_HD: Self = Self {
        width: 3840,
        height: 2160,
    };

    pub const fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }

    pub const fn with_width(mut self, width: u16) -> Self {
        self.width = width;
        self
    }

    pub const fn with_height(mut self, height: u16) -> Self {
        self.height = height;
        self
    }

    pub const fn into_u32(self) -> (u32, u32) {
        (self.width as u32, self.height as u32)
    }

    pub fn resize(self, max: Dims) -> Self {
        // https://docs.rs/image/0.25.10/src/image/math/utils.rs.html
        fn resize_dimensions(
            width: u32,
            height: u32,
            nwidth: u32,
            nheight: u32,
            fill: bool,
        ) -> (u32, u32) {
            use std::cmp::max;

            let wratio = f64::from(nwidth) / f64::from(width);
            let hratio = f64::from(nheight) / f64::from(height);

            let ratio = if fill {
                f64::max(wratio, hratio)
            } else {
                f64::min(wratio, hratio)
            };

            let nw = max((f64::from(width) * ratio).round() as u64, 1);
            let nh = max((f64::from(height) * ratio).round() as u64, 1);

            if nw > u64::from(u32::MAX) {
                let ratio = f64::from(u32::MAX) / f64::from(width);
                (u32::MAX, max((f64::from(height) * ratio).round() as u32, 1))
            } else if nh > u64::from(u32::MAX) {
                let ratio = f64::from(u32::MAX) / f64::from(height);
                (max((f64::from(width) * ratio).round() as u32, 1), u32::MAX)
            } else {
                (nw as u32, nh as u32)
            }
        }

        let (rw, rh) = resize_dimensions(
            self.width as u32,
            self.height as u32,
            max.width as u32,
            max.height as u32,
            false,
        );

        Self {
            width: rw as u16,
            height: rh as u16,
        }
    }
}

impl From<(u32, u32)> for Dims {
    fn from((w, h): (u32, u32)) -> Self {
        Self {
            width: w as u16,
            height: h as u16,
        }
    }
}

/// The pixel dimensions of a single cell in the terminal.
#[derive(Debug, Clone, Copy)]
pub struct CellDims {
    pub width: u16,
    pub height: u16,
}

impl CellDims {
    pub const DEFAULT: Self = Self {
        width: 10,
        height: 20,
    };

    pub const fn width(&self, cols: u16) -> u16 {
        self.width * cols
    }

    pub const fn height(&self, rows: u16) -> u16 {
        self.height * rows
    }

    pub const fn cols(&self, width: u16) -> u16 {
        width.div_ceil(self.width)
    }

    pub const fn rows(&self, height: u16) -> u16 {
        height.div_ceil(self.height)
    }

    pub const fn size(&self, dims: Dims) -> Size {
        Size {
            cols: self.cols(dims.width),
            rows: self.rows(dims.height),
        }
    }

    pub const fn dims(&self, size: Size) -> Dims {
        Dims {
            width: self.width(size.cols),
            height: self.height(size.rows),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Rgb(u8, u8, u8);

impl Rgb {
    pub const BLACK: Self = Self(0, 0, 0);
    pub const WHITE: Self = Self(255, 255, 255);

    fn query_fg(
        writer: &mut impl std::io::Write,
        reader: &mut impl std::io::Read,
    ) -> std::io::Result<Self> {
        let Some(fg) = Self::query("\x1b]10;?\x07", writer, reader)? else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "terminal does not support OSC 10 color query",
            ));
        };
        Ok(fg)
    }

    fn query_bg(
        writer: &mut impl std::io::Write,
        reader: &mut impl std::io::Read,
    ) -> std::io::Result<Self> {
        let Some(bg) = Self::query("\x1b]11;?\x07", writer, reader)? else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "terminal does not support OSC 11 color query",
            ));
        };
        Ok(bg)
    }

    fn query_cursor(
        writer: &mut impl std::io::Write,
        reader: &mut impl std::io::Read,
    ) -> std::io::Result<Self> {
        let Some(cursor) = Self::query("\x1b]12;?\x07", writer, reader)? else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "terminal does not support OSC 12 color query",
            ));
        };
        Ok(cursor)
    }

    /// Returns true if the color is perceived as dark.
    pub const fn is_dark(self) -> bool {
        let Self(r, g, b) = self;
        let brightness = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
        brightness < 128.0
    }

    pub const fn as_color(self) -> Color {
        let Self(r, g, b) = self;
        Color::Rgb(r, g, b)
    }

    pub const fn slight_offset(self) -> Self {
        Self(self.0, self.1, if self.2 == 255 { 254 } else { self.2 + 1 })
    }

    fn query(
        osc: &str,
        writer: &mut impl std::io::Write,
        reader: &mut impl std::io::Read,
    ) -> std::io::Result<Option<Self>> {
        /// Device Status Report control sequence that most terminals implement.
        /// Makes sure that stdin responds.
        const DEVICE_STATUS_REPORT: &str = "\x1b[5n";

        crossterm::terminal::enable_raw_mode()?;

        // Write query
        writer.write_fmt(format_args!("{osc}{DEVICE_STATUS_REPORT}"))?;
        writer.flush()?;

        // Read response
        let mut buffer = [0; 32];
        let n = reader.read(&mut buffer)?;

        crossterm::terminal::disable_raw_mode()?;

        let response = String::from_utf8_lossy(&buffer[..n]);
        Ok(Self::parse(&response))
    }

    fn parse(response: &str) -> Option<Self> {
        // Parse response of pattern "\u{1b}]10;rgb:c4c4/c4c4/b5b5\u{1b}\\\u{1b}"
        let start = response.find(':')? + 1;
        let end = response[start..].find('\x1b')?;
        let payload = &response[start..start + end];
        let mut parts = payload.split('/');

        let parse_channel = |s: &str| match s.len() {
            2 => u8::from_str_radix(s, 16).ok(),
            4 => Some((u16::from_str_radix(s, 16).ok()? >> 8) as u8),
            _ => None,
        };

        let r = parse_channel(parts.next()?)?;
        let g = parse_channel(parts.next()?)?;
        let b = parse_channel(parts.next()?)?;

        Some(Self(r, g, b))
    }
}

#[derive(Debug, Clone)]
pub struct TerminalPalette {
    fg: Rgb,
    bg: Rgb,
    cursor: Rgb,
}

impl TerminalPalette {
    fn query(
        writer: &mut impl std::io::Write,
        reader: &mut impl std::io::Read,
    ) -> std::io::Result<Self> {
        Ok(Self {
            fg: Rgb::query_fg(writer, reader)?,
            bg: Rgb::query_bg(writer, reader)?,
            cursor: Rgb::query_cursor(writer, reader)?,
        })
    }

    pub const fn foreground(&self) -> Rgb {
        self.fg
    }

    pub const fn background(&self) -> Rgb {
        self.bg
    }

    pub const fn cursor(&self) -> Rgb {
        self.cursor
    }

    pub const fn theme(&self) -> TerminalTheme {
        if self.bg.is_dark() {
            TerminalTheme::Dark
        } else {
            TerminalTheme::Light
        }
    }
}

impl Default for TerminalPalette {
    fn default() -> Self {
        Self {
            fg: Rgb::WHITE,
            bg: Rgb::BLACK,
            cursor: Rgb::WHITE,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TerminalTheme {
    Dark,
    Light,
}

#[derive(Debug, Default)]
pub struct Framebuffer {
    buf: String,
    text: String,
    size: TerminalSize,
    thresholds: ScreenSizeThresholds,
    palette: TerminalPalette,
    cursor_state: CursorState,
    cursor_pos_at_end: Option<Pos>,
}

#[derive(Debug, Default, Clone, Copy)]
pub enum CursorState {
    #[default]
    Hide,
    Show,
}

impl Framebuffer {
    const fn new(size: TerminalSize, palette: TerminalPalette) -> Self {
        Self {
            buf: String::new(),
            text: String::new(),
            size,
            thresholds: ScreenSizeThresholds::DEFAULT,
            palette,
            cursor_state: CursorState::Hide,
            cursor_pos_at_end: None,
        }
    }

    pub const fn term_size(&self) -> TerminalSize {
        self.size
    }

    pub const fn set_term_size(&mut self, size: TerminalSize) {
        self.size = size;
    }

    pub const fn screen_size(&self) -> ScreenSize {
        ScreenSize::new(self.size.window_size(), self.thresholds)
    }

    pub const fn palette(&self) -> &TerminalPalette {
        &self.palette
    }

    pub const fn area(&self) -> Rect {
        Rect::new(Pos::ZERO, self.size.window_size())
    }

    pub const fn cursor_state(&self) -> CursorState {
        self.cursor_state
    }

    pub fn set_cursor_state(&mut self, state: CursorState) {
        let _ = match (self.cursor_state, state) {
            (CursorState::Hide, CursorState::Show) => Show.write_ansi(&mut self.buf),
            (CursorState::Show, CursorState::Hide) => Hide.write_ansi(&mut self.buf),
            (_, _) => Ok(()),
        };
        self.cursor_state = state;
    }

    pub const fn set_cursor_pos_at_end(&mut self, pos: Pos) {
        self.cursor_pos_at_end = Some(pos);
    }

    pub fn cursor(&mut self, cursor: impl Into<Cursor>) {
        let _ = write!(self.buf, "{}", cursor.into());
    }

    pub fn print_ch(&mut self, ch: char) {
        self.buf.push(ch);
    }

    pub fn print_ch_repeat(&mut self, ch: char, n: u16) {
        self.buf.extend(std::iter::repeat_n(ch, n as usize));
    }

    pub fn print_str(&mut self, s: &str) {
        self.buf.push_str(s);
    }

    pub fn print_str_fg(&mut self, s: &str, fg: Color) {
        let _ = write!(self.buf, "{}{s}{}", Sgr::Fg(fg), Sgr::reset_fg());
    }

    pub fn print_fmt(&mut self, text: impl Display) {
        let _ = write!(self.buf, "{text}");
    }

    pub fn print_fmt_fg(&mut self, text: impl Display, fg: Color) {
        let _ = write!(self.buf, "{}{text}{}", Sgr::Fg(fg), Sgr::reset_fg());
    }

    pub fn print_span(&mut self, area: Rect, text: impl Display) {
        self.print_span_with_options(area, text, SpanOptions::left());
    }

    pub fn print_span_with_options(
        &mut self,
        area: Rect,
        text: impl Display,
        options: SpanOptions,
    ) {
        if area.is_empty() {
            return;
        }

        // TODO: Newlines should be ignored?

        let start = self.text.len();
        self.push_fmt(text);
        let text = &self.text[start..];

        let row = options.vertical.calc(area.pos.row, area.size.rows, 1);
        print_span(
            area.pos.with_row(row),
            area.size.cols,
            text,
            options.horizontal,
            options.fill,
            &mut self.buf,
        );

        self.text.truncate(start);
    }

    pub fn fill(&mut self, area: Rect, color: Color) {
        self.print_fmt(Sgr::Bg(color));

        for i in 0..area.size.rows {
            self.cursor(area.pos.with_row(area.pos.row + i));
            self.print_ch_repeat(' ', area.size.cols);
        }

        self.print_fmt(Sgr::reset_bg());
    }

    fn flush(&mut self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        use crossterm::QueueableCommand;

        if self.size.window_size().is_either_less(2) {
            self.clear();
            return Ok(());
        }

        for i in 0..self.size.rows {
            writer
                .queue(MoveTo(0, i))?
                .queue(Clear(ClearType::CurrentLine))?;
        }

        if let Some(cpos) = self.cursor_pos_at_end.take() {
            self.cursor(cpos);
        }

        writer.write_all(self.buf.as_bytes())?;
        writer.flush()?;

        self.clear();

        Ok(())
    }

    fn clear(&mut self) {
        self.buf.clear();
        self.text.clear();
        self.cursor_pos_at_end = None;
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

    pub fn render(&mut self, mut area: Rect, opts: TextOptions) {
        if area.is_empty() {
            self.text.clear();
            return;
        }

        match opts.mode {
            TextMode::Span => {
                let row = opts.vertical.calc(area.pos.row, area.size.rows, 1);
                print_span(
                    area.pos.with_row(row),
                    area.size.cols,
                    &self.text, // TODO: Newlines should be ignored?
                    opts.horizontal,
                    opts.fill,
                    &mut self.buf,
                );
            }
            TextMode::Paragraph => {
                utils::text_wrap(&mut self.text, area.size.cols);
                let lines = self.text.lines().count() as u16;

                area.pos.row = opts.vertical.calc(area.pos.row, area.size.rows, lines);
                area.size.rows = area.size.rows.min(lines);

                for line in self.text.lines().take(area.size.rows as usize) {
                    print_span(
                        area.pos,
                        area.size.cols,
                        line,
                        opts.horizontal,
                        opts.fill,
                        &mut self.buf,
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
}

fn print_span(
    pos: Pos,
    max_width: u16,
    text: &str,
    align: HorizontalAlignment,
    fill: bool,
    output: &mut String,
) {
    use std::{cmp::Ordering, fmt::Write};

    let display_width = utils::display_width(text) as u16;
    match display_width.cmp(&max_width) {
        Ordering::Less => {
            // Room to spare, apply alignment
            let start_text_col = align.calc(pos.col, max_width, display_width);

            if fill {
                // Fill remaining empty cells with spaces
                let _ = write!(output, "{}", Cursor::Move(pos.col, pos.row));

                let empty_left_count = start_text_col - pos.col;
                output.extend(std::iter::repeat_n(' ', empty_left_count as usize));

                output.push_str(text);

                let empty_right_count = max_width - (empty_left_count + display_width);
                output.extend(std::iter::repeat_n(' ', empty_right_count as usize));
            } else {
                let _ = write!(output, "{}", Cursor::Move(start_text_col, pos.row));
                output.push_str(text);
            }
        }
        Ordering::Equal => {
            // Perfect fit, just print
            let _ = write!(output, "{}", Cursor::Move(pos.col, pos.row));
            output.push_str(text);
        }
        Ordering::Greater => {
            // No fit, print what we can and keep ansi codes
            let _ = write!(output, "{}", Cursor::Move(pos.col, pos.row));

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
