use std::{
    fs::File,
    io::{BufRead, BufReader, Seek},
    path::Path,
};

use terminal::*;

pub use image::{ImageFormat, imageops::FilterType};

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
pub struct Image {
    id: u32,
    dims: Dims,
}

impl Image {
    pub fn from_path(
        &mut self,
        path: impl AsRef<Path>,
        writer: &mut impl std::io::Write,
        image_id: u32,
        options: ImageLoadOptions,
    ) -> Result<Self, KittyError> {
        let bytes = std::fs::read(path).map_err(|err| KittyLoadError::Read(err))?;
        Self::from_bytes(bytes, writer, image_id, options)
    }

    pub fn from_bytes(
        bytes: impl AsRef<[u8]>,
        writer: &mut impl std::io::Write,
        image_id: u32,
        options: ImageLoadOptions,
    ) -> Result<Self, KittyError> {
        debug_assert_ne!(image_id, 0);

        let mut frames = Vec::new();
        load_image(std::io::Cursor::new(bytes), &mut frames, options.format)?;

        if let Some(resize) = options.resize {
            resize_image(&mut frames, resize);
        }

        let mut deflate = Deflate::new();
        let mut base64 = Base64::new();
        let dims = encode_image(
            image_id,
            frames.iter(),
            &mut deflate,
            &mut base64,
            writer,
            KittyVerbosity::Silent,
        )?;

        Ok(Self { id: image_id, dims })
    }

    pub fn render(&self, area: Rect, frame: &mut Framebuffer, options: ImageOptions) -> Rect {
        let cell_dims = frame.size().cell_dims();
        let ImageOptions {
            resize,
            horizontal,
            vertical,
        } = options;

        // Resize
        let ResizeResult { size, render } = resize.calc(self.id, self.dims, area.size, cell_dims);

        // Alignment
        let area = area.with_size(size).align(area, horizontal, vertical);

        // Render
        frame.cursor(area.pos);
        frame.print_fmt(render);
        area
    }
}

#[derive(Debug)]
pub struct ImageLoadOptions {
    pub format: Option<ImageFormat>,
    pub resize: Option<ImageResize>,
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
        Self {
            format: None,
            resize: Some(ImageResize {
                max,
                filter: FilterType::Lanczos3,
            }),
        }
    }
}

impl Default for ImageLoadOptions {
    fn default() -> Self {
        Self::none()
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
pub enum ResizeMode {
    None,
    Fit,
    Stretch,
    FitWidthCropHeight { rows_outside_top: u16 },
}

impl ResizeMode {
    fn calc(
        self,
        image_id: u32,
        image_dims: Dims,
        image_size: Size,
        cell_dims: CellDims,
    ) -> ResizeResult {
        match self {
            ResizeMode::None => ResizeResult {
                size: cell_dims.size(image_dims),
                render: KittyRender {
                    id: image_id,
                    scale: None,
                    crop: None,
                },
            },
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
    pub resize: ResizeMode,
    pub horizontal: HorizontalAlignment,
    pub vertical: VerticalAlignment,
}

impl ImageOptions {
    pub const fn new() -> Self {
        Self {
            resize: ResizeMode::Fit,
            horizontal: HorizontalAlignment::Left,
            vertical: VerticalAlignment::Top,
        }
    }

    pub const fn fit_center() -> Self {
        Self {
            resize: ResizeMode::Fit,
            horizontal: HorizontalAlignment::Center,
            vertical: VerticalAlignment::Center,
        }
    }

    pub const fn with_resize(mut self, resize: ResizeMode) -> Self {
        self.resize = resize;
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

impl Default for ImageOptions {
    fn default() -> Self {
        Self::fit_center()
    }
}

struct ResizeResult {
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

    // Decode images into frames
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
                return Ok(());
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
            return Ok(());
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
) -> Result<Dims, KittyEncodeError> {
    debug_assert_ne!(id, 0);

    let mut frames = frames.into_iter();

    let Some(frame) = frames.next() else {
        return Err(KittyEncodeError::Empty);
    };

    let rgba = frame.buffer();
    let dims = Dims::from(rgba.dimensions());
    let compressed = deflate.compress(rgba.as_raw())?;
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
    )?;

    // If more frames, encode animation
    let mut animation = false;
    for frame in frames {
        animation = true;

        let delay = frame.delay().numer_denom_ms().0 as i32;
        let rgba = frame.buffer();
        let dims = Dims::from(rgba.dimensions());
        let compressed = deflate.compress(rgba.as_raw())?;
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
        )?;
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
        )?;
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

pub struct KittyImage {
    id: u32,
    dims: Dims,
    encoded: String,
    generation: u32,
}

impl KittyImage {
    pub const fn new(id: u32) -> Self {
        Self {
            id,
            dims: Dims::ZERO,
            encoded: String::new(),
            generation: 0,
        }
    }

    pub const fn dims(&self) -> Dims {
        self.dims
    }

    pub fn load_from_path(
        &mut self,
        path: impl AsRef<Path>,
        kitty: &mut KittyGraphics,
    ) -> Result<(), KittyError> {
        kitty.load_from_path(path)?;
        kitty.encode(self)?;
        Ok(())
    }

    pub fn load_from_bytes(
        &mut self,
        bytes: impl AsRef<[u8]>,
        kitty: &mut KittyGraphics,
    ) -> Result<(), KittyError> {
        kitty.load_from_bytes(bytes)?;
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
        let cell_dims = frame.size().cell_dims();
        let ImageOptions {
            resize,
            horizontal,
            vertical,
        } = options;

        let ResizeResult { size, render } = resize.calc(self.id, self.dims, area.size, cell_dims);
        let pos = area.with_size(size).align(area, horizontal, vertical).pos;
        kitty.render(self, frame, pos, render);
    }

    fn clear(&mut self) {
        self.dims = Dims::ZERO;
        self.encoded.clear();
        self.generation = 0;
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

    fn load_from_path(&mut self, path: impl AsRef<Path>) -> Result<(), image::error::ImageError> {
        self.load_from_reader(BufReader::new(File::open(path)?))
    }

    fn load_from_bytes(&mut self, bytes: impl AsRef<[u8]>) -> Result<(), image::error::ImageError> {
        self.load_from_reader(std::io::Cursor::new(bytes))
    }

    fn load_from_reader<R>(&mut self, reader: R) -> Result<(), image::error::ImageError>
    where
        R: BufRead + Seek,
    {
        self.frames.clear();
        self.load(reader).inspect_err(|_| {
            // Clear all loaded frames if any failed
            self.frames.clear();
        })
    }

    fn load<R>(&mut self, reader: R) -> Result<(), image::error::ImageError>
    where
        R: BufRead + Seek,
    {
        use image::{
            AnimationDecoder, DynamicImage, Frame, ImageReader,
            codecs::{gif::GifDecoder, png::PngDecoder, webp::WebPDecoder},
        };

        let reader = ImageReader::new(reader).with_guessed_format()?;

        let Some(format) = reader.format() else {
            let image = reader.decode()?.to_rgba8();
            self.frames.push(Frame::new(image));
            return Ok(());
        };

        match format {
            ImageFormat::Png => {
                let decoder = PngDecoder::new(reader.into_inner())?;
                if decoder.is_apng()? {
                    for frame in decoder.apng()?.into_frames() {
                        self.frames.push(frame?);
                    }
                } else {
                    let image = DynamicImage::from_decoder(decoder)?.to_rgba8();
                    self.frames.push(Frame::new(image));
                }
            }

            ImageFormat::Gif => {
                let decoder = GifDecoder::new(reader.into_inner())?;
                for frame in decoder.into_frames() {
                    self.frames.push(frame?);
                }
            }

            ImageFormat::WebP => {
                let decoder = WebPDecoder::new(reader.into_inner())?;
                for frame in decoder.into_frames() {
                    self.frames.push(frame?);
                }
            }

            _ => {
                let image = reader.decode()?.to_rgba8();
                self.frames.push(Frame::new(image));
            }
        }

        Ok(())
    }

    fn encode(&mut self, image: &mut KittyImage) -> Result<(), KittyEncodeError> {
        let id = image.id;

        debug_assert_ne!(id, 0);

        image.clear();

        let rgba = self.frames[0].buffer();
        let dims = Dims::from(rgba.dimensions());
        let compressed = self.deflate.compress(rgba.as_raw())?;
        let b64 = self.base64.encode(compressed);

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
                let dims = Dims::from(rgba.dimensions());
                let compressed = self.deflate.compress(rgba.as_raw())?;
                let b64 = self.base64.encode(compressed);

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

    fn render(
        &self,
        image: &mut KittyImage,
        frame: &mut Framebuffer,
        pos: Pos,
        kitty: KittyRender,
    ) {
        // Retransmit image
        if image.generation != self.generation {
            frame.print_str(&image.encoded);
            image.generation = self.generation;
        }

        // Render
        frame.cursor(pos);
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
    Read(std::io::Error),
    Image(image::error::ImageError),
}

impl std::fmt::Display for KittyLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read(err) => err.fmt(f),
            Self::Image(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for KittyLoadError {}

impl From<std::io::Error> for KittyLoadError {
    fn from(err: std::io::Error) -> Self {
        Self::Read(err)
    }
}

impl From<image::error::ImageError> for KittyLoadError {
    fn from(err: image::error::ImageError) -> Self {
        Self::Image(err)
    }
}

#[derive(Debug)]
pub enum KittyEncodeError {
    Empty,
    Io(std::io::Error),
    Fmt(std::fmt::Error),
    Compress(DeflateError),
}

impl std::fmt::Display for KittyEncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => f.write_str("no image frames to encode"),
            Self::Io(err) => err.fmt(f),
            Self::Fmt(err) => err.fmt(f),
            Self::Compress(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for KittyEncodeError {}

impl From<std::io::Error> for KittyEncodeError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<std::fmt::Error> for KittyEncodeError {
    fn from(err: std::fmt::Error) -> Self {
        Self::Fmt(err)
    }
}

impl From<DeflateError> for KittyEncodeError {
    fn from(err: DeflateError) -> Self {
        Self::Compress(err)
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
    fn from(err: image::error::ImageError) -> Self {
        Self::Load(KittyLoadError::Image(err))
    }
}

impl From<KittyLoadError> for KittyError {
    fn from(err: KittyLoadError) -> Self {
        Self::Load(err)
    }
}

impl From<KittyEncodeError> for KittyError {
    fn from(err: KittyEncodeError) -> Self {
        Self::Encode(err)
    }
}
