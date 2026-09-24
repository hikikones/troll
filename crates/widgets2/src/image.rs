use terminal::*;

// TODO: Remove generation stuff?
// Since terminal no longer clears entire buffer when window resizes,
// images do not need to be retransmitted anymore.

pub struct Image {
    id: u32,
    dims: ImageDims,
    encoded: String,
    generation: u32,
}

impl Image {
    pub const fn new(id: u32) -> Self {
        Self {
            id,
            dims: ImageDims::ZERO,
            encoded: String::new(),
            generation: 0,
        }
    }

    pub const fn dims(&self) -> ImageDims {
        self.dims
    }

    pub fn load_from_path(
        &mut self,
        path: impl AsRef<std::path::Path>,
        kitty: &mut KittyGraphics,
    ) -> Result<(), KittyError> {
        kitty.load_from_path(path.as_ref())?;
        kitty.encode(self)?;
        Ok(())
    }

    pub fn load_from_png_bytes(
        &mut self,
        bytes: impl AsRef<[u8]>,
        kitty: &mut KittyGraphics,
    ) -> Result<(), KittyError> {
        kitty.load_png_from_bytes(bytes.as_ref())?;
        kitty.encode(self)?;
        Ok(())
    }

    pub fn render(
        &mut self,
        area: Rect,
        frame: &mut Framebuffer,
        kitty: &KittyGraphics,
        options: ImageOptions,
    ) {
        let cell_dims = frame.cell_dims();
        let ResizeResult { size, render } = options
            .resize
            .map(|r| r.calc(self.id, self.dims, area.size, cell_dims))
            .unwrap_or_else(|| ResizeResult {
                size: cell_dims.size(self.dims),
                render: KittyRender {
                    id: self.id,
                    scale: None,
                    crop: None,
                },
            });

        let pos = match (options.horizontal, options.vertical) {
            (None, None) => area.pos,
            (Some(horz), None) => {
                area.pos
                    .with_col(horz.calc(area.pos.col, area.size.cols, size.cols))
            }
            (None, Some(vert)) => {
                area.pos
                    .with_row(vert.calc(area.pos.row, area.size.rows, size.rows))
            }
            (Some(horz), Some(vert)) => Pos {
                col: horz.calc(area.pos.col, area.size.cols, size.cols),
                row: vert.calc(area.pos.row, area.size.rows, size.rows),
            },
        };

        kitty.render(self, frame, pos, render);
    }

    fn clear(&mut self) {
        self.dims = ImageDims::ZERO;
        self.encoded.clear();
        self.generation = 0;
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ResizeMode {
    Fit,
    Stretch,
    FitWidthCropHeight { rows_outside_top: u16 },
}

impl ResizeMode {
    fn calc(
        self,
        image_id: u32,
        image_dims: ImageDims,
        image_size: Size,
        cell_dims: CellDims,
    ) -> ResizeResult {
        match self {
            ResizeMode::Fit => {
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

                ResizeResult {
                    size: resized,
                    render: KittyRender {
                        id: image_id,
                        scale: Some(scale),
                        crop: None,
                    },
                }
            }
            ResizeMode::Stretch => {
                let scale = KittyScale::Stretch(image_size.cols, image_size.rows);
                ResizeResult {
                    size: image_size,
                    render: KittyRender {
                        id: image_id,
                        scale: Some(scale),
                        crop: None,
                    },
                }
            }
            ResizeMode::FitWidthCropHeight { rows_outside_top } => {
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

                    ResizeResult {
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
                    ResizeResult {
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

impl Default for ResizeMode {
    fn default() -> Self {
        Self::Fit
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ImageOptions {
    pub resize: Option<ResizeMode>,
    pub horizontal: Option<HorizontalAlignment>,
    pub vertical: Option<VerticalAlignment>,
}

impl ImageOptions {
    pub const fn new() -> Self {
        Self {
            resize: None,
            horizontal: None,
            vertical: None,
        }
    }

    pub const fn fit_and_center() -> Self {
        Self {
            resize: Some(ResizeMode::Fit),
            horizontal: Some(HorizontalAlignment::Center),
            vertical: Some(VerticalAlignment::Center),
        }
    }

    pub const fn with_resize(mut self, resize: ResizeMode) -> Self {
        self.resize = Some(resize);
        self
    }

    pub const fn with_horizontal(mut self, horizontal: HorizontalAlignment) -> Self {
        self.horizontal = Some(horizontal);
        self
    }

    pub const fn with_vertical(mut self, vertical: VerticalAlignment) -> Self {
        self.vertical = Some(vertical);
        self
    }
}

impl Default for ImageOptions {
    fn default() -> Self {
        Self::fit_and_center()
    }
}

struct ResizeResult {
    size: Size,
    render: KittyRender,
}

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
            "{KITTY_START}{},{},{},{},{}",
            KittyAction::Display,
            KittyId(self.id),
            KittyPlacement(self.id),
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

pub struct KittyGraphics {
    frames: Vec<image::Frame>,
    deflate: Deflate,
    base64: Base64,
    verbosity: KittyVerbosity,
    generation: u32,
}

impl KittyGraphics {
    pub fn new() -> Self {
        Self {
            frames: Vec::new(),
            deflate: Deflate::new(),
            base64: Base64::new(),
            verbosity: KittyVerbosity::Silent,
            generation: 1,
        }
    }

    pub const fn increase_generation(&mut self) {
        self.generation += 1;
    }

    fn load_from_path(&mut self, path: &std::path::Path) -> Result<(), KittyLoadError> {
        use image::AnimationDecoder;

        let reader = image::ImageReader::open(path)?.with_guessed_format()?;

        let Some(image_format) = reader.format() else {
            let image = reader.decode()?.to_rgba8();
            self.frames.push(image::Frame::new(image));
            return Ok(());
        };

        self.frames.clear();

        match image_format {
            image::ImageFormat::Png => {
                let png_decoder = image::codecs::png::PngDecoder::new(reader.into_inner())?;
                if png_decoder.is_apng()? {
                    for frame_res in png_decoder.apng()?.into_frames() {
                        match frame_res {
                            Ok(frame) => self.frames.push(frame),
                            Err(err) => {
                                self.frames.clear();
                                return Err(err)?;
                            }
                        }
                    }
                } else {
                    let mut reader = image::ImageReader::open(path)?;
                    reader.set_format(image::ImageFormat::Png);
                    let image = reader.decode()?.to_rgba8();
                    self.frames.push(image::Frame::new(image));
                }
            }
            image::ImageFormat::Gif => {
                for frame_res in
                    image::codecs::gif::GifDecoder::new(reader.into_inner())?.into_frames()
                {
                    match frame_res {
                        Ok(frame) => self.frames.push(frame),
                        Err(err) => {
                            self.frames.clear();
                            return Err(err)?;
                        }
                    }
                }
            }
            image::ImageFormat::WebP => {
                for frame_res in
                    image::codecs::webp::WebPDecoder::new(reader.into_inner())?.into_frames()
                {
                    match frame_res {
                        Ok(frame) => self.frames.push(frame),
                        Err(err) => {
                            self.frames.clear();
                            return Err(err)?;
                        }
                    }
                }
            }
            _ => {
                let image = reader.decode()?.to_rgba8();
                self.frames.push(image::Frame::new(image));
            }
        }

        Ok(())
    }

    fn load_png_from_bytes(&mut self, bytes: &[u8]) -> Result<(), image::error::ImageError> {
        let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)?;
        self.frames.clear();
        self.frames.push(image::Frame::new(image.to_rgba8()));

        Ok(())
    }

    fn encode(&mut self, image: &mut Image) -> Result<(), KittyEncodeError> {
        let id = image.id;

        debug_assert_ne!(id, 0);

        image.clear();

        let rgba = self.frames[0].buffer();
        let dims = ImageDims::from(rgba.dimensions());
        let compressed = self.deflate.compress(rgba.as_raw())?;
        let b64 = self.base64.encode(compressed);

        // Encode first frame
        struct StaticRoot {
            id: u32,
            dims: ImageDims,
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

        Self::write_chunks(
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
            &mut image.encoded,
        )?;

        // Animated image
        if self.frames.len() > 1 {
            for i in 1..self.frames.len() {
                let delay = self.frames[i].delay().numer_denom_ms().0 as i32;
                let rgba = self.frames[i].buffer();
                let dims = ImageDims::from(rgba.dimensions());
                let compressed = self.deflate.compress(rgba.as_raw())?;
                let b64 = self.base64.encode(compressed);

                struct AnimatedRoot {
                    id: u32,
                    dims: ImageDims,
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

                Self::write_chunks(
                    AnimatedRoot {
                        id,
                        dims,
                        delay,
                        verbosity: KittyVerbosity::Silent,
                    },
                    AnimatedChunk {
                        id,
                        verbosity: KittyVerbosity::Silent,
                    },
                    b64,
                    &mut image.encoded,
                )?;
            }

            // Set animation controls
            std::fmt::Write::write_fmt(
                &mut image.encoded,
                format_args!(
                    "{KITTY_START}{},{},{},{},{}{KITTY_END}",
                    KittyAction::AnimationControl,
                    KittyId(id),
                    KittyAnimationState::RunNormal,
                    KittyAnimationLoop::Forever,
                    self.verbosity
                ),
            )?;
        }

        image.dims = dims;

        Ok(())
    }

    fn render(&self, image: &mut Image, frame: &mut Framebuffer, pos: Pos, kitty: KittyRender) {
        // Retransmit image
        if image.generation != self.generation {
            frame.print_str(&image.encoded);
            image.generation = self.generation;
        }

        // Render
        frame.cursor_move(pos);
        frame.print_fmt(kitty);
    }

    fn write_chunks(
        root_header: impl std::fmt::Display,
        chunk_header: impl std::fmt::Display,
        base64: &str,
        writer: &mut impl std::fmt::Write,
    ) -> std::fmt::Result {
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
}

#[derive(Debug, Clone, Copy)]
enum KittyImageFormat {
    // Rgb24(Dimensions),
    Rgba32(ImageDims),
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
        &self.encoded.as_str()
    }
}

#[derive(Debug)]
pub enum KittyLoadError {
    Io(std::io::Error),
    Image(image::error::ImageError),
}

impl std::fmt::Display for KittyLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => err.fmt(f),
            Self::Image(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for KittyLoadError {}

impl From<std::io::Error> for KittyLoadError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<image::error::ImageError> for KittyLoadError {
    fn from(value: image::error::ImageError) -> Self {
        Self::Image(value)
    }
}

#[derive(Debug)]
pub enum KittyEncodeError {
    Fmt(std::fmt::Error),
    Compress(DeflateError),
}

impl std::fmt::Display for KittyEncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fmt(err) => err.fmt(f),
            Self::Compress(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for KittyEncodeError {}

impl From<std::fmt::Error> for KittyEncodeError {
    fn from(value: std::fmt::Error) -> Self {
        Self::Fmt(value)
    }
}

impl From<DeflateError> for KittyEncodeError {
    fn from(value: DeflateError) -> Self {
        Self::Compress(value)
    }
}

#[derive(Debug)]
pub enum KittyError {
    Load(KittyLoadError),
    Encode(KittyEncodeError),
}

impl std::fmt::Display for KittyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Load(err) => err.fmt(f),
            Self::Encode(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for KittyError {}

impl From<image::error::ImageError> for KittyError {
    fn from(value: image::error::ImageError) -> Self {
        Self::Load(KittyLoadError::Image(value))
    }
}

impl From<KittyLoadError> for KittyError {
    fn from(value: KittyLoadError) -> Self {
        Self::Load(value)
    }
}

impl From<KittyEncodeError> for KittyError {
    fn from(value: KittyEncodeError) -> Self {
        Self::Encode(value)
    }
}
