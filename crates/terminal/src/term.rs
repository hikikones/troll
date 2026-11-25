use std::{fmt::Display, io::Stdout};

use crossterm::{
    cursor::Hide,
    execute,
    terminal::{DisableLineWrap, EnterAlternateScreen, LeaveAlternateScreen},
};

use crate::{Color, Size};

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
        execute!(backend, LeaveAlternateScreen)?;
        crossterm::terminal::disable_raw_mode()
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

impl ScreenWidth {
    pub const fn is_wide(&self) -> bool {
        matches!(self, ScreenWidth::Wide)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ScreenHeight {
    Short,
    #[default]
    Normal,
    Tall,
}

impl ScreenHeight {
    pub const fn is_tall(&self) -> bool {
        matches!(self, ScreenHeight::Tall)
    }
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
        width_normal: 56,
        width_wide: 80,
        height_normal: 16,
        height_tall: 24,
    };
}

impl Default for ScreenSizeThresholds {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ScreenSize {
    pub width: ScreenWidth,
    pub height: ScreenHeight,
}

impl ScreenSize {
    pub const DEFAULT: Self = Self {
        width: ScreenWidth::Normal,
        height: ScreenHeight::Normal,
    };

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

    pub const fn is_wide(&self) -> bool {
        self.width.is_wide()
    }

    pub const fn is_tall(&self) -> bool {
        self.height.is_tall()
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

    pub const fn all(value: u16) -> Self {
        Self {
            width: value,
            height: value,
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub const BLACK: Self = Self(0, 0, 0);
    pub const WHITE: Self = Self(255, 255, 255);

    pub const fn all(v: u8) -> Self {
        Self(v, v, v)
    }

    pub fn query_fg() -> Result<Self, RgbQueryError> {
        Self::query(crate::Osc::ForegroundColor)
    }

    pub fn query_bg() -> Result<Self, RgbQueryError> {
        Self::query(crate::Osc::BackgroundColor)
    }

    pub fn query_cursor() -> Result<Self, RgbQueryError> {
        Self::query(crate::Osc::CursorColor)
    }

    /// Returns true if the color is perceived as dark.
    pub const fn is_dark(self) -> bool {
        let Self(r, g, b) = self;
        let brightness = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
        brightness < 128.0
    }

    pub const fn as_color(self) -> Color {
        Color::Rgb(self)
    }

    /// Offsets the blue channel by one.
    /// Useful for default bg so images behind are blocked.
    pub const fn slight_offset(self) -> Self {
        let Self(r, g, b) = self;
        Self(r, g, if b == 255 { 254 } else { b + 1 })
    }

    /// Returns a color with a contrast that is readable on `self`.
    pub fn readable(self) -> Self {
        let rl = self.relative_luminance();
        let contrast_black = (rl + 0.05) / 0.05;
        let contrast_white = (1.05) / (rl + 0.05);
        if contrast_black > contrast_white {
            Self::BLACK
        } else {
            Self::WHITE
        }
    }

    pub fn relative_luminance(self) -> f64 {
        fn srgb_to_linear(c: u8) -> f64 {
            let c = c as f64 / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }

        let r = srgb_to_linear(self.0);
        let g = srgb_to_linear(self.1);
        let b = srgb_to_linear(self.2);

        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    fn query(osc: impl Display) -> Result<Self, RgbQueryError> {
        let mut buffer = [0; 32];
        let n = Self::query_color(osc, &mut buffer).map_err(RgbQueryError::Io)?;
        let response = String::from_utf8_lossy(&buffer[..n]);

        let hex = Self::get_hex_parts(&response).ok_or_else(|| RgbQueryError::ParseResponse {
            response: format!("{response}"),
        })?;
        let rgb = Self::parse_hex_parts(hex).map_err(|err| RgbQueryError::ParseRgb {
            hex_parts: format!("{hex:?}"),
            error: err,
        })?;

        Ok(rgb)
    }

    fn query_color(osc: impl Display, buffer: &mut [u8]) -> std::io::Result<usize> {
        use std::io::{Read, Write};

        crossterm::terminal::enable_raw_mode()?;

        let stdout = &mut std::io::stdout().lock();
        let stdin = &mut std::io::stdin().lock();

        // Also add DSR to make sure that stdin responds.
        stdout.write_fmt(format_args!("{osc}{}", crate::Csi::DeviceStatusReport))?;
        stdout.flush()?;

        // Read response
        let n = stdin.read(buffer)?;

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

    fn parse_hex_parts(hex: (&str, &str, &str)) -> Result<Self, std::num::ParseIntError> {
        fn parse_channel(hex: &str) -> Result<u8, std::num::ParseIntError> {
            let value = u32::from_str_radix(hex, 16)?;
            let max = (1u32 << (hex.len() * 4)) - 1;
            Ok(((value * 255 + max / 2) / max) as u8)
        }

        let r = parse_channel(hex.0)?;
        let g = parse_channel(hex.1)?;
        let b = parse_channel(hex.2)?;

        Ok(Self(r, g, b))
    }
}

impl Default for Rgb {
    fn default() -> Self {
        Self::WHITE
    }
}

#[derive(Debug)]
pub enum RgbQueryError {
    Io(std::io::Error),
    ParseResponse {
        response: String,
    },
    ParseRgb {
        hex_parts: String,
        error: std::num::ParseIntError,
    },
}

impl std::fmt::Display for RgbQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => err.fmt(f),
            Self::ParseResponse { response } => f.write_fmt(format_args!(
                "Unable to extract hex parts from query response: '{response}'."
            )),
            Self::ParseRgb { hex_parts, .. } => {
                f.write_fmt(format_args!("Unable to parse hex parts from {hex_parts}."))
            }
        }
    }
}

impl std::error::Error for RgbQueryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::ParseResponse { .. } => None,
            Self::ParseRgb { error, .. } => Some(error),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub enum TermTheme {
    #[default]
    Dark,
    Light,
}

impl TermTheme {
    pub const fn from_bg(bg: Rgb) -> Self {
        if bg.is_dark() {
            Self::Dark
        } else {
            Self::Light
        }
    }
}
