use std::{
    io::{BufRead, Seek},
    path::Path,
};

use terminal::*;

pub use image::{ImageFormat, imageops::FilterType};

pub type ImageResult<T> = Result<T, ImageError>;

pub struct Image {
    id: u32,
    dims: Dims,
    encode: ImageEncode,
    generation: i32,
}

impl Default for Image {
    fn default() -> Self {
        Self::new(1)
    }
}

impl Image {
    pub const fn new(id: u32) -> Self {
        Self::new_with_encode(id, ImageEncode(Vec::new()))
    }

    pub const fn new_with_encode(id: u32, encode: ImageEncode) -> Self {
        Self {
            id,
            dims: Dims::ZERO,
            encode,
            generation: -1,
        }
    }

    pub const fn id(&self) -> u32 {
        self.id
    }

    pub const fn dims(&self) -> Dims {
        self.dims
    }

    pub const fn encode(&self) -> &ImageEncode {
        &self.encode
    }

    pub fn take_encode(&mut self) -> ImageEncode {
        std::mem::take(&mut self.encode)
    }

    pub fn load_from_path(
        &mut self,
        path: impl AsRef<Path>,
        opts: ImageLoadOptions,
    ) -> ImageResult<()> {
        let bytes = std::fs::read(path).map_err(|err| ImageError::Read(err))?;
        self.load_from_bytes(bytes, opts)
    }

    pub fn load_from_bytes(
        &mut self,
        bytes: impl AsRef<[u8]>,
        opts: ImageLoadOptions,
    ) -> ImageResult<()> {
        let mut frames = Vec::new();
        load_image(std::io::Cursor::new(bytes), &mut frames, opts.format)
            .map_err(|err| ImageError::Load(err))?;

        if let Some(resize) = opts.resize {
            resize_image(&mut frames, resize);
        }

        self.encode.0.clear();

        let dims = encode_image(
            self.id,
            frames.iter(),
            &mut Deflate::new(),
            &mut Base64::new(),
            &mut self.encode.0,
            KittyVerbosity::Silent,
        )?;

        self.dims = dims;
        self.generation = -1;

        Ok(())
    }

    pub fn transmit(&mut self, generation: i32) -> std::io::Result<()> {
        use std::io::Write;

        let mut stdout = std::io::stdout().lock();
        stdout.write_all(&self.encode.0)?;
        stdout.flush()?;

        self.generation = generation;

        Ok(())
    }

    pub fn render(&mut self, area: Rect, frame: &mut Frame, opts: ImageRenderOptions) -> Rect {
        // Resize
        let FitResult { size, render } = opts.fit.calc(
            self.id,
            self.dims,
            area.size,
            frame.cell_dims().unwrap_or_default(),
        );

        // Alignment
        let area = area
            .with_size(size)
            .align(area, opts.horizontal, opts.vertical);

        // Retransmit image
        let generation = frame.generation();
        if self.generation != generation {
            frame.print_str(self.encode.base64());
            self.generation = generation;
        }

        // Render
        frame.cursor(area.pos);
        frame.print_fmt(render);

        area
    }
}

#[derive(Default)]
pub struct ImageEncode(Vec<u8>);

impl ImageEncode {
    pub const fn bytes(&self) -> &[u8] {
        self.0.as_slice()
    }

    pub const fn base64(&self) -> &str {
        // SAFETY: Base64-encoded data, which is always valid UTF-8 (ASCII).
        unsafe { std::str::from_utf8_unchecked(self.bytes()) }
    }
}

#[derive(Debug)]
pub struct ImageLoadOptions {
    pub format: Option<ImageFormat>,
    pub resize: Option<ImageResize>,
}

impl Default for ImageLoadOptions {
    fn default() -> Self {
        Self::none()
    }
}

impl ImageLoadOptions {
    pub const fn new(format: ImageFormat, resize: ImageResize) -> Self {
        Self {
            format: Some(format),
            resize: Some(resize),
        }
    }

    pub const fn none() -> Self {
        Self {
            format: None,
            resize: None,
        }
    }

    pub const fn format(format: ImageFormat) -> Self {
        Self {
            format: Some(format),
            resize: None,
        }
    }

    pub const fn resize(resize: ImageResize) -> Self {
        Self {
            format: None,
            resize: Some(resize),
        }
    }

    pub const fn max(max: Dims) -> Self {
        Self::resize(ImageResize {
            max,
            filter: FilterType::Lanczos3,
        })
    }
}

#[derive(Debug)]
pub struct ImageResize {
    pub max: Dims,
    pub filter: FilterType,
}

impl ImageResize {
    pub const fn new(max: Dims, filter: FilterType) -> Self {
        Self { max, filter }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ImageRenderOptions {
    pub fit: ImageFitMode,
    pub horizontal: HorizontalAlignment,
    pub vertical: VerticalAlignment,
}

impl Default for ImageRenderOptions {
    fn default() -> Self {
        Self::fit_center()
    }
}

impl ImageRenderOptions {
    pub const fn new() -> Self {
        Self {
            fit: ImageFitMode::Fit,
            horizontal: HorizontalAlignment::Left,
            vertical: VerticalAlignment::Top,
        }
    }

    pub const fn fit_center() -> Self {
        Self {
            fit: ImageFitMode::Fit,
            horizontal: HorizontalAlignment::Center,
            vertical: VerticalAlignment::Center,
        }
    }

    pub const fn with_resize(mut self, fit: ImageFitMode) -> Self {
        self.fit = fit;
        self
    }

    pub const fn with_horizontal(mut self, horizontal: HorizontalAlignment) -> Self {
        self.horizontal = horizontal;
        self
    }

    pub const fn with_vertical(mut self, vertical: VerticalAlignment) -> Self {
        self.vertical = vertical;
        self
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub enum ImageFitMode {
    None,
    #[default]
    Fit,
    Stretch,
    FitWidthCropHeight {
        rows_outside_top: u16,
    },
}

impl ImageFitMode {
    fn calc(
        self,
        image_id: u32,
        image_dims: Dims,
        image_size: Size,
        cell_dims: CellDims,
    ) -> FitResult {
        match self {
            ImageFitMode::None => FitResult {
                size: cell_dims.size(image_dims),
                render: KittyRender {
                    id: image_id,
                    scale: None,
                    crop: None,
                },
            },
            ImageFitMode::Fit => {
                let max_dims = cell_dims.dims(image_size);
                let resized_dims = image_dims.resize(max_dims);
                let resized = cell_dims.size(resized_dims);

                let width_ratio = image_dims.width as f64 / max_dims.width as f64;
                let height_ratio = image_dims.height as f64 / max_dims.height as f64;

                // Scale by either columns or rows to maintain aspect ratio
                let scale = if width_ratio >= height_ratio {
                    KittyScale::Columns(resized.cols)
                } else {
                    KittyScale::Rows(resized.rows)
                };

                FitResult {
                    size: resized,
                    render: KittyRender {
                        id: image_id,
                        scale: Some(scale),
                        crop: None,
                    },
                }
            }
            ImageFitMode::Stretch => {
                let scale = KittyScale::Stretch(image_size.cols, image_size.rows);
                FitResult {
                    size: image_size,
                    render: KittyRender {
                        id: image_id,
                        scale: Some(scale),
                        crop: None,
                    },
                }
            }
            ImageFitMode::FitWidthCropHeight { rows_outside_top } => {
                let max_width = cell_dims.width(image_size.cols);
                let resized_dims = image_dims.resize(image_dims.with_width(max_width));
                let resized = cell_dims.size(resized_dims);

                if resized.rows > image_size.rows {
                    // Need to crop height with source rectangle x, y, width, height
                    let y = cell_dims.height(rows_outside_top);
                    let height = cell_dims.height(image_size.rows);
                    let h_ratio = image_dims.height as f64 / resized_dims.height as f64;
                    let scale = KittyScale::Columns(image_size.cols.min(resized.cols));
                    let crop = KittyCrop {
                        x: 0,
                        y: (y as f64 * h_ratio).round() as u32,
                        width: image_dims.width as u32,
                        height: (height as f64 * h_ratio).round() as u32,
                    };

                    FitResult {
                        size: resized.with_rows(image_size.rows),
                        render: KittyRender {
                            id: image_id,
                            scale: Some(scale),
                            crop: Some(crop),
                        },
                    }
                } else {
                    // No vertical cropping needed, just scale by width
                    let scale =
                        KittyScale::Columns(image_size.cols.min(cell_dims.cols(image_dims.width)));
                    FitResult {
                        size: resized.with_rows(cell_dims.rows(image_dims.height)),
                        render: KittyRender {
                            id: image_id,
                            scale: Some(scale),
                            crop: None,
                        },
                    }
                }
            }
        }
    }
}

struct FitResult {
    size: Size,
    render: KittyRender,
}

fn load_image<R>(
    reader: R,
    frames: &mut Vec<image::Frame>,
    format: Option<ImageFormat>,
) -> Result<(), image::error::ImageError>
where
    R: BufRead + Seek,
{
    use image::{
        AnimationDecoder, DynamicImage, Frame, ImageReader,
        codecs::{gif::GifDecoder, png::PngDecoder, webp::WebPDecoder},
    };

    let (reader, format) = match format {
        Some(format) => {
            let mut reader = ImageReader::new(reader);
            reader.set_format(format);
            (reader, format)
        }
        None => {
            let reader = ImageReader::new(reader).with_guessed_format()?;
            match reader.format() {
                Some(format) => (reader, format),
                None => {
                    let image = reader.decode()?.into_rgba8();
                    frames.push(Frame::new(image));
                    return Ok(());
                }
            }
        }
    };

    // Decode image into frames
    match format {
        ImageFormat::Png => {
            let decoder = PngDecoder::new(reader.into_inner())?;
            if decoder.is_apng()? {
                for frame in decoder.apng()?.into_frames() {
                    frames.push(frame?);
                }
            } else {
                let image = DynamicImage::from_decoder(decoder)?.into_rgba8();
                frames.push(Frame::new(image));
            }
        }
        ImageFormat::Gif => {
            let decoder = GifDecoder::new(reader.into_inner())?;
            for frame in decoder.into_frames() {
                frames.push(frame?);
            }
        }
        ImageFormat::WebP => {
            let decoder = WebPDecoder::new(reader.into_inner())?;
            for frame in decoder.into_frames() {
                frames.push(frame?);
            }
        }
        _ => {
            let image = reader.decode()?.into_rgba8();
            frames.push(Frame::new(image));
        }
    }

    Ok(())
}

fn resize_image(frames: &mut Vec<image::Frame>, options: ImageResize) {
    let (max_width, max_height) = options.max.into_u32();
    for frame in frames.iter_mut() {
        let (width, height) = frame.buffer().dimensions();

        if width <= max_width && height <= max_height {
            continue;
        }

        let buffer = std::mem::take(frame.buffer_mut());
        let resized = image::DynamicImage::ImageRgba8(buffer)
            .resize(max_width, max_height, options.filter)
            .into_rgba8();
        *frame.buffer_mut() = resized;
    }
}

fn encode_image<'a>(
    id: u32,
    frames: impl IntoIterator<Item = &'a image::Frame>,
    deflate: &mut Deflate,
    base64: &mut Base64,
    writer: &mut impl std::io::Write,
    verbosity: KittyVerbosity,
) -> ImageResult<Dims> {
    debug_assert_ne!(id, 0);

    let mut frames = frames.into_iter();

    let Some(frame) = frames.next() else {
        return Err(ImageError::EncodeEmpty);
    };

    let rgba = frame.buffer();
    let dims = Dims::from(rgba.dimensions());
    let compressed = deflate
        .compress(rgba.as_raw())
        .map_err(|err| ImageError::Compress(err))?;
    let b64 = base64.encode(compressed);

    // Encode first frame
    struct StaticRoot {
        id: u32,
        dims: Dims,
        verbosity: KittyVerbosity,
    }

    struct StaticChunk {
        id: u32,
        verbosity: KittyVerbosity,
    }

    impl std::fmt::Display for StaticRoot {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_fmt(format_args!(
                "{},{},{},{},{},{}",
                KittyAction::Transmit,
                KittyImageFormat::Rgba32(self.dims),
                KittyTransfer::Direct,
                KittyId(self.id),
                KittyCompression::ZlibDeflate,
                self.verbosity
            ))
        }
    }

    impl std::fmt::Display for StaticChunk {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_fmt(format_args!("{},{}", KittyId(self.id), self.verbosity))
        }
    }

    write_chunks(
        StaticRoot {
            id,
            dims,
            verbosity: KittyVerbosity::Silent,
        },
        StaticChunk {
            id,
            verbosity: KittyVerbosity::Silent,
        },
        b64,
        writer,
    )
    .map_err(|err| ImageError::Write(err))?;

    // If more frames, encode animation
    let mut animation = false;
    for frame in frames {
        animation = true;

        let delay = frame.delay().numer_denom_ms().0 as i32;
        let rgba = frame.buffer();
        let dims = Dims::from(rgba.dimensions());
        let compressed = deflate
            .compress(rgba.as_raw())
            .map_err(|err| ImageError::Compress(err))?;
        let b64 = base64.encode(compressed);

        struct AnimatedRoot {
            id: u32,
            dims: Dims,
            delay: i32,
            verbosity: KittyVerbosity,
        }

        struct AnimatedChunk {
            id: u32,
            verbosity: KittyVerbosity,
        }

        impl std::fmt::Display for AnimatedRoot {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_fmt(format_args!(
                    "{},{},{},{},{},{},{}",
                    KittyAction::AnimationTransmitFrame,
                    KittyImageFormat::Rgba32(self.dims),
                    KittyTransfer::Direct,
                    KittyId(self.id),
                    KittyAnimationGap(self.delay),
                    KittyCompression::ZlibDeflate,
                    self.verbosity
                ))
            }
        }

        impl std::fmt::Display for AnimatedChunk {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_fmt(format_args!(
                    "{},{},{}",
                    KittyAction::AnimationTransmitFrame,
                    KittyId(self.id),
                    self.verbosity
                ))
            }
        }

        write_chunks(
            AnimatedRoot {
                id,
                dims,
                delay,
                verbosity,
            },
            AnimatedChunk { id, verbosity },
            b64,
            writer,
        )
        .map_err(|err| ImageError::Write(err))?;
    }

    // Set animation controls
    if animation {
        write!(
            writer,
            "{KITTY_START}{},{},{},{},{}{KITTY_END}",
            KittyAction::AnimationControl,
            KittyId(id),
            KittyAnimationState::RunNormal,
            KittyAnimationLoop::Forever,
            verbosity
        )
        .map_err(|err| ImageError::Write(err))?;
    }

    Ok(dims)
}

fn write_chunks(
    root_header: impl std::fmt::Display,
    chunk_header: impl std::fmt::Display,
    base64: &str,
    writer: &mut impl std::io::Write,
) -> std::io::Result<()> {
    const CHUNK_SIZE: usize = 4096;
    let b64_len = base64.len();

    if b64_len <= CHUNK_SIZE {
        return write!(writer, "{KITTY_START}{root_header};{base64}{KITTY_END}");
    }

    write!(
        writer,
        "{KITTY_START}{root_header},m=1;{}{KITTY_END}",
        &base64[0..CHUNK_SIZE]
    )?;

    let mut start = CHUNK_SIZE;
    let mut end = CHUNK_SIZE * 2;

    while end < b64_len {
        write!(
            writer,
            "{KITTY_START}{chunk_header},m=1;{}{KITTY_END}",
            &base64[start..end]
        )?;
        start = end;
        end += CHUNK_SIZE;
    }

    write!(
        writer,
        "{KITTY_START}{chunk_header},m=0;{}{KITTY_END}",
        &base64[start..]
    )
}

struct Deflate(Vec<u8>);

impl Deflate {
    const fn new() -> Self {
        Self(Vec::new())
    }

    fn compress(&mut self, input: &[u8]) -> Result<&[u8], DeflateError> {
        // Reset buffer with zeroes
        let zeroes = zlib_rs::compress_bound(input.len());
        self.0.clear();
        self.0.extend(std::iter::repeat_n(0, zeroes));

        // Compress
        let config = zlib_rs::DeflateConfig::default();
        let (compressed, rc) = zlib_rs::compress_slice(&mut self.0, input, config);
        match rc {
            zlib_rs::ReturnCode::Ok => Ok(compressed),
            _ => Err(DeflateError(rc)),
        }
    }
}

#[derive(Debug)]
pub struct DeflateError(zlib_rs::ReturnCode);

impl std::fmt::Display for DeflateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let error = unsafe { std::ffi::CStr::from_ptr(self.0.error_message()) };
        f.write_fmt(format_args!(
            "{} in deflate compression",
            error.to_string_lossy()
        ))
    }
}

impl std::error::Error for DeflateError {}

struct Base64 {
    engine: base64::engine::Simd,
    encoded: String,
}

impl Base64 {
    fn new() -> Self {
        Self {
            engine: base64::engine::Simd::standard(base64::engine::general_purpose::PAD),
            encoded: String::new(),
        }
    }

    fn encode(&mut self, input: &[u8]) -> &str {
        self.encoded.clear();
        base64::Engine::encode_string(&self.engine, input, &mut self.encoded);
        self.encoded.as_str()
    }
}

#[derive(Debug)]
pub enum ImageError {
    Read(std::io::Error),
    Load(image::error::ImageError),
    EncodeEmpty,
    Compress(DeflateError),
    Write(std::io::Error),
}

impl std::fmt::Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read(err) => err.fmt(f),
            Self::Load(err) => err.fmt(f),
            Self::EncodeEmpty => f.write_str("No image frames to encode"),
            Self::Compress(err) => err.fmt(f),
            Self::Write(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for ImageError {}

// TODO: Move most kitty stuff to terminal module?

const KITTY_START: &str = "\x1b_G";
const KITTY_END: &str = "\x1b\\";

pub struct KittyDeleteAll;

impl std::fmt::Display for KittyDeleteAll {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{KITTY_START}{},{}{KITTY_END}",
            KittyAction::Delete(KittyDelete::AllVisible),
            KittyVerbosity::Silent,
        ))
    }
}

struct KittyRender {
    id: u32,
    scale: Option<KittyScale>,
    crop: Option<KittyCrop>,
}

impl std::fmt::Display for KittyRender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{KITTY_START}{},{},{},{},{},{}",
            KittyAction::Display,
            KittyId(self.id),
            KittyPlacement(self.id),
            KittyLayer::BEHIND_ALL,
            KittyCursorMovement::NoMovement,
            KittyVerbosity::Silent,
        ))?;

        if let Some(scale) = self.scale {
            f.write_fmt(format_args!(",{}", scale))?;
        }

        if let Some(crop) = self.crop {
            f.write_fmt(format_args!(",{}", crop))?;
        }

        f.write_str(KITTY_END)
    }
}

#[derive(Debug, Clone, Copy)]
enum KittyImageFormat {
    // Rgb24(Dimensions),
    Rgba32(Dims),
    // Png,
}

impl std::fmt::Display for KittyImageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rgba32(dims) => {
                f.write_fmt(format_args!("f=32,s={},v={}", dims.width, dims.height))
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum KittyTransfer {
    Direct,
    // File,
    // TempFile,
    // SharedMemory
}

impl std::fmt::Display for KittyTransfer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KittyTransfer::Direct => f.write_str("t=d"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum KittyAction {
    Transmit,
    // TransmitAndDisplay,
    // QueryTerminal,
    Display,
    Delete(KittyDelete),
    AnimationTransmitFrame,
    // AnimationComposeFrame,
    AnimationControl,
}

impl std::fmt::Display for KittyAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Transmit => f.write_str("a=t"),
            Self::Display => f.write_str("a=p"),
            Self::Delete(delete) => f.write_fmt(format_args!("a=d,{}", delete)),
            Self::AnimationTransmitFrame => f.write_str("a=f"),
            Self::AnimationControl => f.write_str("a=a"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct KittyId(u32);

impl std::fmt::Display for KittyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("i={}", self.0))
    }
}

#[derive(Debug, Clone, Copy)]
struct KittyPlacement(u32);

impl std::fmt::Display for KittyPlacement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("p={}", self.0))
    }
}

#[derive(Debug, Clone, Copy)]
enum KittyScale {
    Columns(u16),
    Rows(u16),
    Stretch(u16, u16),
}

impl std::fmt::Display for KittyScale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Columns(cols) => f.write_fmt(format_args!("c={}", cols)),
            Self::Rows(rows) => f.write_fmt(format_args!("r={}", rows)),
            Self::Stretch(cols, rows) => f.write_fmt(format_args!("c={},r={}", cols, rows)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct KittyCrop {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl std::fmt::Display for KittyCrop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "x={},y={},w={},h={}",
            self.x, self.y, self.width, self.height
        ))
    }
}

/// The z-index vertical stacking order of the image.
#[derive(Debug, Clone, Copy)]
struct KittyLayer(i32);

impl KittyLayer {
    // const DEFAULT: Self = Self(0);
    // const BEHIND_TEXT: Self = Self(-1);
    const BEHIND_ALL: Self = Self(i32::MIN);
}

impl std::fmt::Display for KittyLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("z={}", self.0))
    }
}

#[derive(Debug, Clone, Copy)]
enum KittyDelete {
    AllVisible,
    // Id,
    // Range(u32, u32),
}

impl std::fmt::Display for KittyDelete {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::AllVisible => f.write_str("d=a"),
            // Self::Id => f.write_str("d=i"),
            // Self::Range(min, max) => f.write_fmt(format_args!("d=r,x={},y={}", min, max)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum KittyCompression {
    ZlibDeflate,
}

impl std::fmt::Display for KittyCompression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZlibDeflate => f.write_str("o=z"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum KittyCursorMovement {
    // MoveToAfterImage = 0,
    NoMovement = 1,
}

impl std::fmt::Display for KittyCursorMovement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("C={}", *self as u8))
    }
}

#[derive(Debug, Clone, Copy)]
struct KittyAnimationGap(i32);

impl std::fmt::Display for KittyAnimationGap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("z={}", self.0))
    }
}

#[derive(Debug, Clone, Copy)]
enum KittyAnimationState {
    // Stop = 1,
    // RunWaitLoad = 2,
    RunNormal = 3,
}

impl std::fmt::Display for KittyAnimationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("s={}", *self as u8))
    }
}

#[derive(Debug, Clone, Copy)]
enum KittyAnimationLoop {
    // Ignore,
    Forever,
    // Amount(u32),
}

impl std::fmt::Display for KittyAnimationLoop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            // Self::Ignore => f.write_str("v=0"),
            Self::Forever => f.write_str("v=1"),
            // Self::Amount(n) => f.write_fmt(format_args!("v={}", n + 1)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum KittyVerbosity {
    // All = 0,
    // ErrorsOnly = 1,
    Silent = 2,
}

impl std::fmt::Display for KittyVerbosity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("q={}", *self as u8))
    }
}
