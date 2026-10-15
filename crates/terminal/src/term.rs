use std::io::Stdout;

use crossterm::{
    cursor::Hide,
    execute,
    terminal::{DisableLineWrap, EnterAlternateScreen, LeaveAlternateScreen},
};

use crate::{Color, Size, TerminalEvent};

pub struct Terminal<W: std::io::Write = Stdout>(W);

impl Terminal<Stdout> {
    pub fn stdout() -> Self {
        Self(std::io::stdout())
    }

    pub fn enter<T>(
        mut self,
        f: impl FnOnce(&mut Self) -> Result<T, Box<dyn std::error::Error>>,
    ) -> Result<T, Box<dyn std::error::Error>> {
        Self::set_panic_hook();

        Self::enter_alternate_screen(&mut self.0)?;
        let res = f(&mut self);
        Self::leave_alternate_screen(&mut self.0)?;

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

    pub fn temp_leave<T>(&mut self, f: impl FnOnce() -> std::io::Result<T>) -> std::io::Result<T> {
        Self::leave_alternate_screen(&mut self.0)?;
        let t = f();
        Self::enter_alternate_screen(&mut self.0)?;

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

impl<W: std::io::Write> std::ops::Deref for Terminal<W> {
    type Target = W;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<W: std::io::Write> std::ops::DerefMut for Terminal<W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug)]
pub enum TerminalError {
    Io(std::io::Error),
    Query(RgbQueryError),
}

impl std::fmt::Display for TerminalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TerminalError::Io(err) => err.fmt(f),
            TerminalError::Query(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for TerminalError {}

impl From<std::io::Error> for TerminalError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<RgbQueryError> for TerminalError {
    fn from(err: RgbQueryError) -> Self {
        Self::Query(err)
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
    pub const fn new() -> Self {
        Self {
            cols: 0,
            rows: 0,
            width: 0,
            height: 0,
        }
    }

    pub fn query() -> std::io::Result<Self> {
        let win_size = crossterm::terminal::window_size()?;
        Ok(Self {
            cols: win_size.columns,
            rows: win_size.rows,
            width: win_size.width,
            height: win_size.height,
        })
    }

    pub const fn window_size(&self) -> Size {
        Size {
            cols: self.cols,
            rows: self.rows,
        }
    }

    pub const fn window_dims(&self) -> Option<Dims> {
        if self.width == 0 || self.height == 0 {
            return None;
        }

        Some(Dims {
            width: self.width,
            height: self.height,
        })
    }

    pub const fn cell_dims(&self) -> Option<CellDims> {
        if self.width == 0 || self.height == 0 {
            return None;
        }

        Some(CellDims {
            width: self.width / self.cols,
            height: self.height / self.rows,
        })
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ScreenWidth {
    Narrow,
    #[default]
    Normal,
    Wide,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ScreenHeight {
    Short,
    #[default]
    Normal,
    Tall,
}

#[derive(Debug, Clone, Copy)]
pub struct ScreenSizeThresholds {
    pub width_normal: u16,
    pub width_wide: u16,
    pub height_normal: u16,
    pub height_tall: u16,
}

impl ScreenSizeThresholds {
    pub const DEFAULT: Self = Self {
        width_normal: 60,
        width_wide: 80,
        height_normal: 20,
        height_tall: 30,
    };
}

impl Default for ScreenSizeThresholds {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, Default, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::resource::Resource))]
pub struct ScreenSize {
    width: ScreenWidth,
    height: ScreenHeight,
}

impl ScreenSize {
    pub const fn new(window_size: Size, thresholds: ScreenSizeThresholds) -> ScreenSize {
        let ScreenSizeThresholds {
            width_normal,
            width_wide,
            height_normal,
            height_tall,
        } = thresholds;

        let width = match window_size.cols {
            w if w < width_normal => ScreenWidth::Narrow,
            w if w < width_wide => ScreenWidth::Normal,
            _ => ScreenWidth::Wide,
        };
        let height = match window_size.rows {
            h if h < height_normal => ScreenHeight::Short,
            h if h < height_tall => ScreenHeight::Normal,
            _ => ScreenHeight::Tall,
        };

        Self { width, height }
    }

    pub const fn width(&self) -> ScreenWidth {
        self.width
    }

    pub const fn height(&self) -> ScreenHeight {
        self.height
    }

    pub const fn both(&self) -> (ScreenWidth, ScreenHeight) {
        (self.width, self.height)
    }
}

/// Pixel dimensions.
#[derive(Debug, Default, Clone, Copy)]
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

    pub const fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
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

impl Default for CellDims {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::resource::Resource))]
pub struct Rgb(u8, u8, u8);

impl Rgb {
    pub const BLACK: Self = Self(0, 0, 0);
    pub const WHITE: Self = Self(255, 255, 255);

    pub fn query_fg(
        writer: &mut impl std::io::Write,
        reader: &mut impl std::io::Read,
    ) -> Result<Self, RgbQueryError> {
        let fg = Self::query("\x1b]10;?\x07", writer, reader)?;
        Ok(fg)
    }

    pub fn query_bg(
        writer: &mut impl std::io::Write,
        reader: &mut impl std::io::Read,
    ) -> Result<Self, RgbQueryError> {
        let bg = Self::query("\x1b]11;?\x07", writer, reader)?;
        Ok(bg)
    }

    pub fn query_cursor(
        writer: &mut impl std::io::Write,
        reader: &mut impl std::io::Read,
    ) -> Result<Self, RgbQueryError> {
        let cursor = Self::query("\x1b]12;?\x07", writer, reader)?;
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
        let Self(r, g, b) = self;
        Self(r, g, if b == 255 { 254 } else { b + 1 })
    }

    fn query(
        osc: &str,
        writer: &mut impl std::io::Write,
        reader: &mut impl std::io::Read,
    ) -> Result<Self, RgbQueryError> {
        let mut buffer = [0; 32];

        let n = Self::do_query(osc, &mut buffer, writer, reader).map_err(|err| RgbQueryError {
            message: format!("Failed to run OSC color query '{osc}'."),
            source: Some(RgbQueryErrorSource::Io(err)),
        })?;

        let response = String::from_utf8_lossy(&buffer[..n]);

        let Some(rgb) = Self::get_hex_parts(&response) else {
            return Err(RgbQueryError {
                message: format!(
                    "Failed to find hex color parts of response '{response}' from OSC color query '{osc}'."
                ),
                source: None,
            });
        };

        let rgb = Self::parse_hex_parts(rgb).map_err(|err| RgbQueryError {
            message: format!(
                "Failed to parse hex colors from parts '{rgb:?}' obtained by the OSC color query '{osc}'."
            ),
            source: Some(RgbQueryErrorSource::Parse(err)),
        })?;

        Ok(rgb)
    }

    fn do_query(
        osc: &str,
        buffer: &mut [u8],
        writer: &mut impl std::io::Write,
        reader: &mut impl std::io::Read,
    ) -> std::io::Result<usize> {
        /// Device Status Report control sequence that most terminals implement.
        /// Makes sure that stdin responds.
        const DEVICE_STATUS_REPORT: &str = "\x1b[5n";

        crossterm::terminal::enable_raw_mode()?;

        // Write query
        writer.write_fmt(format_args!("{osc}{DEVICE_STATUS_REPORT}"))?;
        writer.flush()?;

        // Read response
        let n = reader.read(buffer)?;

        crossterm::terminal::disable_raw_mode()?;

        Ok(n)
    }

    fn get_hex_parts(response: &str) -> Option<(&str, &str, &str)> {
        // Parse response of pattern "\u{1b}]10;rgb:c4c4/c4c4/b5b5\u{1b}\\\u{1b}"
        let start = response.find(':')? + 1;
        let end = response[start..].find('\x1b')?;
        let payload = &response[start..start + end];
        let mut parts = payload.split('/');

        match (parts.next(), parts.next(), parts.next()) {
            (Some(r), Some(g), Some(b)) => Some((r, g, b)),
            _ => None,
        }
    }

    fn parse_hex_parts((r, g, b): (&str, &str, &str)) -> Result<Self, std::num::ParseIntError> {
        fn parse_channel(hex: &str) -> Result<u8, std::num::ParseIntError> {
            let value = u32::from_str_radix(hex, 16)?;
            let max = (1u32 << (hex.len() * 4)) - 1;
            Ok(((value * 255 + max / 2) / max) as u8)
        }

        let r = parse_channel(r)?;
        let g = parse_channel(g)?;
        let b = parse_channel(b)?;

        Ok(Self(r, g, b))
    }
}

#[derive(Debug)]
pub struct RgbQueryError {
    message: String,
    source: Option<RgbQueryErrorSource>,
}

impl std::fmt::Display for RgbQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message.as_str())
    }
}

impl std::error::Error for RgbQueryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|source| match source {
            RgbQueryErrorSource::Io(err) => err as _,
            RgbQueryErrorSource::Parse(err) => err as _,
        })
    }
}

#[derive(Debug)]
enum RgbQueryErrorSource {
    Io(std::io::Error),
    Parse(std::num::ParseIntError),
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::resource::Resource))]
pub struct TerminalPalette {
    fg: Rgb,
    bg: Rgb,
    cursor: Rgb,
}

impl TerminalPalette {
    pub fn query(
        writer: &mut impl std::io::Write,
        reader: &mut impl std::io::Read,
    ) -> Result<Self, RgbQueryError> {
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
#[cfg_attr(feature = "bevy", derive(bevy_ecs::resource::Resource))]
pub enum TerminalTheme {
    Dark,
    Light,
}
