use std::{
    fmt::{Display, Write},
    io::Stdout,
};

use crossterm::{
    Command,
    cursor::{Hide, MoveTo, Show},
    event::Event,
    execute,
    terminal::{Clear, ClearType, DisableLineWrap, EnterAlternateScreen, LeaveAlternateScreen},
};

use crate::{CellDims, HorizontalAlignment, Pos, Rect, Size};

pub struct Terminal {
    stdout: Stdout,
    buffer: Framebuffer,
}

impl Terminal {
    pub fn enter_tui() -> std::io::Result<Self> {
        let size = TermSize::from(crossterm::terminal::window_size()?);

        Self::set_panic_hook();

        let mut stdout = std::io::stdout();
        crossterm::terminal::enable_raw_mode()?;
        execute!(stdout, EnterAlternateScreen, Hide, DisableLineWrap)?;

        Ok(Self {
            stdout,
            buffer: Framebuffer {
                buf: String::new(),
                text: String::new(),
                size,
            },
        })
    }

    pub fn leave_tui(mut self) -> std::io::Result<()> {
        execute!(self.stdout, Show, LeaveAlternateScreen)?;
        crossterm::terminal::disable_raw_mode()
    }

    /// Reads a terminal event in a blocking manner.
    pub fn read_event() -> std::io::Result<Event> {
        crossterm::event::read()
    }

    /// Polls and reads a terminal event in a non-blocking manner.
    pub fn poll_event(timeout: std::time::Duration) -> std::io::Result<Option<Event>> {
        match crossterm::event::poll(timeout) {
            Ok(true) => crossterm::event::read().map(|ev| Some(ev)),
            Ok(false) => Ok(None),
            Err(err) => Err(err),
        }
    }

    pub fn frame(&mut self) -> &mut Framebuffer {
        &mut self.buffer
    }

    pub fn render(
        &mut self,
        f: impl FnOnce(&mut Framebuffer) -> std::io::Result<()>,
    ) -> std::io::Result<()> {
        self.buffer.size = TermSize::from(crossterm::terminal::window_size()?);

        f(&mut self.buffer)?;

        self.buffer.flush(&mut self.stdout.lock())
    }

    fn set_panic_hook() {
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            Self::restore();
            hook(info);
        }));
    }

    fn restore() {
        fn try_restore() -> std::io::Result<()> {
            crossterm::terminal::disable_raw_mode()?;
            execute!(std::io::stdout(), LeaveAlternateScreen)
        }

        if let Err(err) = try_restore() {
            std::eprintln!("Failed to restore terminal: {err}");
        }
    }
}

pub struct Framebuffer {
    buf: String,
    text: String,
    size: TermSize,
}

impl Framebuffer {
    pub const fn area(&self) -> Rect {
        Rect::new(Pos::ZERO, self.size.window_size())
    }

    pub const fn window_size(&self) -> Size {
        self.size.window_size()
    }

    pub const fn cell_dims(&self) -> CellDims {
        self.size.cell_dims()
    }

    pub fn cursor_hide(&mut self) {
        let _ = Hide.write_ansi(&mut self.buf);
    }

    pub fn cursor_show(&mut self) {
        let _ = Show.write_ansi(&mut self.buf);
    }

    pub fn cursor_move(&mut self, pos: impl Into<Pos>) {
        let Pos { col, row } = pos.into();
        let _ = MoveTo(col, row).write_ansi(&mut self.buf);
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

    pub fn print_fmt(&mut self, content: impl Display) {
        let _ = write!(self.buf, "{content}");
    }

    fn flush(&mut self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        use crossterm::QueueableCommand;

        if self.size.window_size().is_less(2) {
            self.clear();
            return Ok(());
        }

        for i in 0..self.size.rows {
            writer
                .queue(MoveTo(0, i))?
                .queue(Clear(ClearType::CurrentLine))?;
        }
        writer.write_all(self.buf.as_bytes())?;
        writer.flush()?;

        self.clear();

        Ok(())
    }

    fn clear(&mut self) {
        self.buf.clear();
        self.text.clear();
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

    pub fn push_fmt(&mut self, content: impl Display) {
        let _ = write!(self.text, "{content}");
    }

    pub fn render(&mut self, area: Rect, options: TextOptions) {
        if self.text.is_empty() {
            self.cursor_move(area.pos);
            return;
        }

        if area.is_empty() {
            self.cursor_move(area.pos);
            self.text.clear();
            return;
        }

        let mut text = std::mem::take(&mut self.text);

        match options.mode {
            TextMode::Span { fill } => {
                self.render_span(area.pos, area.size.cols, &text, options.align, fill);
            }
            TextMode::Paragraph { center_vertical } => {
                utils::text_wrap(&mut text, area.size.cols);
                let lines = text.lines().count() as u16;

                let Rect {
                    mut pos,
                    size: mut rect,
                } = area;

                if center_vertical {
                    rect.rows = area.size.rows.min(lines);
                    pos.row = area.pos.row + (area.size.rows.saturating_sub(lines)) / 2;
                }

                let max_row = pos.row + rect.rows;

                for line in text.lines() {
                    self.render_span(pos, rect.cols, line, options.align, false);
                    pos.row += 1;

                    if pos.row == max_row {
                        break;
                    }
                }
            }
        }

        text.clear();
        self.text = text;
    }

    fn render_span(
        &mut self,
        pos: Pos,
        max_width: u16,
        text: &str,
        align: HorizontalAlignment,
        fill: bool,
    ) {
        use std::cmp::Ordering;

        let display_width = utils::display_width(text) as u16;
        match display_width.cmp(&max_width) {
            Ordering::Less => {
                // Room to spare, apply alignment
                let start_text_col = align.calc(pos.col, max_width, display_width);

                if fill {
                    // Fill remaining empty cells with spaces
                    self.cursor_move(pos);

                    let empty_left_count = start_text_col - pos.col;
                    self.print_ch_repeat(' ', empty_left_count);

                    self.print_str(text);

                    let empty_right_count = max_width - (empty_left_count + display_width);
                    self.print_ch_repeat(' ', empty_right_count);
                } else {
                    self.cursor_move(pos.with_col(start_text_col));
                    self.print_str(text);
                }
            }
            Ordering::Equal => {
                // Perfect fit, just print
                self.cursor_move(pos);
                self.print_str(text);
            }
            Ordering::Greater => {
                // No fit, print what we can and keep ansi codes
                self.cursor_move(pos);

                let mut width = 0;
                for g in utils::GraphemeAnsiIter::new(text) {
                    match g {
                        utils::GraphemeOrAnsi::Grapheme(g) => {
                            if width == max_width {
                                continue;
                            }

                            let w = utils::str_width(g);

                            if width + w > max_width {
                                width = max_width;
                                continue;
                            }

                            width += w;
                            self.print_str(g);
                        }
                        utils::GraphemeOrAnsi::Ansi(s) => {
                            self.print_str(s);
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TermSize {
    cols: u16,
    rows: u16,
    width: u16,
    height: u16,
}

impl TermSize {
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

    pub const fn cell_dims(&self) -> CellDims {
        CellDims {
            width: self.width / self.cols,
            height: self.height / self.rows,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TextMode {
    Span { fill: bool },
    Paragraph { center_vertical: bool },
}

#[derive(Debug, Clone, Copy)]
pub struct TextOptions {
    pub mode: TextMode,
    pub align: HorizontalAlignment,
}

impl TextOptions {
    pub const fn span() -> Self {
        Self {
            mode: TextMode::Span { fill: false },
            align: HorizontalAlignment::Left,
        }
    }

    pub const fn span_fill() -> Self {
        Self {
            mode: TextMode::Span { fill: true },
            align: HorizontalAlignment::Left,
        }
    }

    pub const fn span_center() -> Self {
        Self {
            mode: TextMode::Span { fill: false },
            align: HorizontalAlignment::Center,
        }
    }

    pub const fn span_right() -> Self {
        Self {
            mode: TextMode::Span { fill: false },
            align: HorizontalAlignment::Right,
        }
    }

    pub const fn paragraph() -> Self {
        Self {
            mode: TextMode::Paragraph {
                center_vertical: false,
            },
            align: HorizontalAlignment::Left,
        }
    }

    pub const fn paragraph_center() -> Self {
        Self {
            mode: TextMode::Paragraph {
                center_vertical: false,
            },
            align: HorizontalAlignment::Center,
        }
    }

    pub const fn paragraph_right() -> Self {
        Self {
            mode: TextMode::Paragraph {
                center_vertical: false,
            },
            align: HorizontalAlignment::Right,
        }
    }
}

impl Default for TextOptions {
    fn default() -> Self {
        Self::span()
    }
}
