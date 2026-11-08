use std::{path::PathBuf, range::Range};
use terminal::*;
use utils::StringExt;

use crate::{
    ImageOptions, KittyError, KittyGraphics, KittyImage, ResizeMode, Scroll, Scrollbar,
    ScrollbarColors, ScrollbarData,
};

type MarkupResult<T> = Result<T, MarkupError>;

pub struct Markup {
    text: String,
    scroll: u16,
    desired_scroll: Option<MarkupScroll>,
    plain: Vec<MarkupPlain>,
    rich: Vec<MarkupRich>,
    highlighter: CodeHighlighter,
    images: MarkupImages,
    cell: CellDims,
    view: Size,
    size: Size,
    hash: u64,
    total_lines: u16,
    scroll_area: Option<Rect>,
    options: MarkupOptions,
    colors: MarkupColors,
}

#[derive(Debug, Clone, Copy)]
pub enum MarkupScroll {
    Up,
    Down,
    PageUp,
    PageDown,
    Start,
    End,
}

pub struct MarkupOptions {
    pub padding: Margin,
    pub scrollbar: bool,
    pub scrollbar_margin: u16,
    pub break_char: char,
}

impl MarkupOptions {
    pub const fn new() -> Self {
        Self {
            padding: Margin::ZERO,
            scrollbar: false,
            scrollbar_margin: 1,
            break_char: '─',
        }
    }
}

impl Default for MarkupOptions {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MarkupColors {
    pub syntax_theme: MarkupCodeTheme,
    pub scrollbar: ScrollbarColors,
    pub heading: Color,
    pub break_char: Color,
    pub math: Rgb,
}

impl MarkupColors {
    pub const fn new() -> Self {
        Self {
            syntax_theme: MarkupCodeTheme::Base16EightiesDark,
            scrollbar: ScrollbarColors::DEFAULT,
            heading: Color::Yellow,
            break_char: Color::Indexed(240),
            math: Rgb::WHITE,
        }
    }
}

impl Default for MarkupColors {
    fn default() -> Self {
        Self::new()
    }
}

impl Markup {
    pub fn new(assets: PathBuf) -> Self {
        Self {
            text: String::new(),
            scroll: 0,
            desired_scroll: None,
            plain: Vec::new(),
            rich: Vec::new(),
            images: MarkupImages::new(assets),
            highlighter: CodeHighlighter::new(),
            cell: CellDims::DEFAULT,
            view: Size::ZERO,
            size: Size::ZERO,
            hash: 0,
            total_lines: 0,
            scroll_area: None,
            options: MarkupOptions::new(),
            colors: MarkupColors::new(),
        }
    }

    pub const fn with_options(mut self, options: MarkupOptions) -> Self {
        self.options = options;
        self
    }

    pub const fn with_scrollbar(mut self, margin: u16) -> Self {
        self.options.scrollbar = true;
        self.options.scrollbar_margin = margin;
        self
    }

    pub const fn with_padding(mut self, padding: Margin) -> Self {
        self.options.padding = padding;
        self
    }

    pub const fn with_colors(mut self, colors: MarkupColors) -> Self {
        self.colors = colors;
        self
    }

    pub const fn scroll_index(&self) -> u16 {
        self.scroll
    }

    pub const fn set_desired_scroll(&mut self, sm: MarkupScroll) -> &mut Self {
        self.desired_scroll = Some(sm);
        self
    }

    pub const fn set_colors(&mut self, colors: MarkupColors) -> &mut Self {
        self.colors = colors;
        self
    }

    pub const fn set_cell_dims(&mut self, size: CellDims) -> &mut Self {
        self.cell = size;
        self
    }

    pub fn input(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Down => self.scroll(MarkupScroll::Down),
            KeyCode::Up => self.scroll(MarkupScroll::Up),
            KeyCode::PageDown => self.scroll(MarkupScroll::PageDown),
            KeyCode::PageUp => self.scroll(MarkupScroll::PageUp),
            KeyCode::Home => self.scroll(MarkupScroll::Start),
            KeyCode::End => self.scroll(MarkupScroll::End),
            _ => false,
        }
    }

    pub fn scroll(&mut self, sm: MarkupScroll) -> bool {
        let old_scroll = self.scroll;
        self.set_scroll(sm);
        self.scroll != old_scroll
    }

    pub fn reset(&mut self) {
        self.scroll = 0;
        self.desired_scroll = None;
    }

    pub fn render(
        &mut self,
        area: Rect,
        frame: &mut Framebuffer,
        markup: &str,
        kitty: &mut KittyGraphics,
    ) {
        let inner = area.inner(self.options.padding);

        if inner.is_empty() {
            return;
        }

        // Prepare markup
        self.parse_and_load(markup, kitty);
        self.process_markup(area, inner, frame);

        // Scroll
        self.update_scroll();
        self.render_scrollbar(frame);

        // Render
        self.render_markup(inner.with_size(self.view), frame, kitty);
    }

    fn render_markup(
        &mut self,
        mut area: Rect,
        frame: &mut Framebuffer,
        kitty: &mut KittyGraphics,
    ) {
        let top_row = area.row();
        let view_top = self.scroll;
        let view_bot = self.scroll + area.rows();
        let mut curr_line = 0;

        const fn is_in_view(curr_line: u16, top: u16, bot: u16) -> bool {
            curr_line >= top && curr_line < bot
        }

        for markup in self.rich.iter().copied() {
            match markup {
                MarkupRich::Text { range, alignment } => {
                    for line in self.text.slice(range).lines() {
                        let is_in_view = is_in_view(curr_line, view_top, view_bot);

                        utils::GraphemeAnsiIter::new(line).for_each(|ga| match ga {
                            utils::GraphemeOrAnsi::Grapheme(g) => {
                                if is_in_view {
                                    frame.push_str(g.0);
                                }
                            }
                            utils::GraphemeOrAnsi::Ansi(s) => match is_in_view {
                                true => frame.push_str(s),
                                false => frame.print_str(s),
                            },
                        });

                        if is_in_view {
                            frame.render(area, TextOptions::span().with_horizontal(alignment));
                            area.shrink_down(1);
                        }

                        curr_line += 1;
                    }
                }
                MarkupRich::Image { index } => {
                    let dims = self.images.dims(index);
                    let max_width = self.cell.width(area.cols());
                    let resized_dims = dims.resize(dims.with_width(max_width));
                    let resized_rows = self.cell.rows(resized_dims.height);

                    if is_in_view(
                        curr_line,
                        view_top.saturating_sub(resized_rows - 1),
                        view_bot,
                    ) {
                        let is_at_top = area.row() == top_row;
                        let rows_outside_top = if is_at_top {
                            curr_line.abs_diff(self.scroll)
                        } else {
                            0
                        };
                        let image_rows = (resized_rows - rows_outside_top).min(area.rows());
                        let image_area = area.with_rows(image_rows);

                        self.images
                            .render(index, image_area, rows_outside_top, frame, kitty);

                        area.shrink_down(image_rows);
                    }

                    curr_line += resized_rows;
                }
                MarkupRich::Break => {
                    if is_in_view(curr_line, view_top, view_bot) {
                        frame.cursor(area.pos);
                        frame.print_fmt(Sgr::Fg(self.colors.break_char));
                        frame.print_ch_repeat(self.options.break_char, area.cols());
                        frame.print_fmt(Sgr::reset_fg());
                        area.shrink_down(1);
                    }

                    curr_line += 1;
                }
                MarkupRich::EmptyLine => {
                    if is_in_view(curr_line, view_top, view_bot) {
                        area.shrink_down(1);
                    }

                    curr_line += 1;
                }
            }

            if area.rows() == 0 {
                break;
            }
        }
    }

    fn clear(&mut self) {
        self.text.clear();
        self.plain.clear();
        self.rich.clear();
        self.total_lines = 0;
        self.view = Size::ZERO;
        self.size = Size::ZERO;
        self.hash = 0;
    }

    fn parse_and_load(&mut self, markup: &str, kitty: &mut KittyGraphics) {
        let hash = utils::hash_fast(markup);

        if self.hash == hash {
            return;
        }

        self.clear();
        self.hash = hash;

        let mut image_index = 0;

        // Parse and load markup
        for (block, _) in MarkupBlockParser::new(markup) {
            match block {
                MarkupBlock::Paragraph { text, alignment } => {
                    self.plain.push(MarkupPlain::Paragraph {
                        text: self.text.push_str_get_range(text),
                        alignment,
                    });
                    self.plain.push(MarkupPlain::EmptyLine);
                }
                MarkupBlock::Heading {
                    text,
                    alignment,
                    newline,
                } => {
                    self.plain.push(MarkupPlain::Heading {
                        text: self.text.push_str_get_range(text),
                        alignment,
                    });
                    if newline {
                        self.plain.push(MarkupPlain::EmptyLine);
                    }
                }
                MarkupBlock::List { items } => {
                    for item in items {
                        self.plain.push(MarkupPlain::ListItem {
                            text: self.text.push_str_get_range(item),
                        });
                    }
                    self.plain.push(MarkupPlain::EmptyLine);
                }
                MarkupBlock::Code { language, text } => {
                    self.plain.push(MarkupPlain::Code {
                        text: self.text.push_str_get_range(text),
                        _language: self.text.push_str_get_range(language),
                    });
                    self.plain.push(MarkupPlain::EmptyLine);
                }
                MarkupBlock::Image { description, path } => {
                    match self
                        .images
                        .load(image_index, ImageSource::Filename(path), kitty)
                    {
                        Ok(_) => {
                            self.plain.push(MarkupPlain::Image { index: image_index });
                        }
                        Err(err) => {
                            self.plain.push(MarkupPlain::ImageError {
                                text: self.text.write_get_range(format_args!("ERROR\n{err}")),
                            });
                        }
                    }

                    image_index += 1;

                    if !description.is_empty() {
                        self.plain.push(MarkupPlain::ImageDescription {
                            text: self.text.push_str_get_range(description),
                        });
                    }

                    self.plain.push(MarkupPlain::EmptyLine);
                }
                MarkupBlock::Math { text } => {
                    match self.images.load(
                        image_index,
                        ImageSource::Math(text, self.colors.math, self.cell),
                        kitty,
                    ) {
                        Ok(_) => {
                            self.plain.push(MarkupPlain::Image { index: image_index });
                        }
                        Err(err) => {
                            self.plain.push(MarkupPlain::ImageError {
                                text: self.text.write_get_range(format_args!("ERROR\n{err}")),
                            });
                        }
                    }

                    image_index += 1;

                    self.plain.push(MarkupPlain::EmptyLine);
                }
                MarkupBlock::Break => {
                    self.plain.push(MarkupPlain::Break);
                    self.plain.push(MarkupPlain::EmptyLine);
                }
                MarkupBlock::Comment { .. } => continue,
            }
        }

        // Remove last empty line
        let is_last_empty_line = self
            .plain
            .last()
            .map(|mp| matches!(mp, MarkupPlain::EmptyLine))
            .unwrap_or(false);
        if is_last_empty_line {
            self.plain.pop();
        }
    }

    fn process_markup(&mut self, area: Rect, inner: Rect, frame: &mut Framebuffer) {
        if self.size == area.size {
            return;
        }

        self.size = area.size;
        self.view = inner.size;
        self.scroll_area = None;

        self.relayout(frame);

        if self.is_scrollable() {
            let scroll_area = area.with_col(area.right_col()).with_cols(1);
            self.view
                .sub_cols(scroll_area.cols() + self.options.scrollbar_margin);
            self.scroll_area = Some(scroll_area);
            self.relayout(frame);
        }
    }

    fn relayout(&mut self, frame: &mut Framebuffer) {
        self.rich.clear();

        let width = self.view.cols;
        let writer = frame.text_mut();

        fn markup_to_ansi<'a>(markup: &str, width: u16, writer: &'a mut String) -> &'a str {
            InlineParser::new(markup).for_each(|event| match event {
                InlineEvent::Text(text) => writer.push_str(text),
                InlineEvent::Tag(tag) => writer.write(tag.as_sgr()),
            });
            utils::text_wrap(writer, width);
            writer.as_str()
        }

        let plain_to_rich_iter = self.plain.iter().copied().map(|plain| {
            let rich = match plain {
                MarkupPlain::Paragraph { text, alignment } => {
                    let markup = self.text.slice(text);
                    let ansi = markup_to_ansi(markup, width, writer);
                    let range = self.text.push_str_get_range(ansi);
                    MarkupRich::Text { range, alignment }
                }
                MarkupPlain::Heading { text, alignment } => {
                    let markup = self.text.slice(text);
                    writer.write(format_args!(
                        "{}{markup}{}",
                        Sgrs([Sgr::Bold, Sgr::Fg(self.colors.heading)]),
                        Sgr::Reset
                    ));
                    utils::text_wrap(writer, width);
                    let range = self.text.push_str_get_range(writer.as_str());
                    MarkupRich::Text { range, alignment }
                }
                MarkupPlain::ListItem { text } => {
                    let markup = self.text.slice(text);
                    let ansi = markup_to_ansi(markup, width.saturating_sub(4), writer);

                    let start = self.text.len();
                    for (i, line) in ansi.lines().enumerate() {
                        let indent = if i == 0 { "  • " } else { "    " };
                        self.text.extend([indent, line, "\n"]);
                    }
                    let end = self.text.len();

                    MarkupRich::Text {
                        range: Range { start, end },
                        alignment: HorizontalAlignment::Left,
                    }
                }
                MarkupPlain::Code { text, _language } => {
                    let lang = self.text.slice(_language);
                    let code = self.text.slice(text);

                    self.highlighter.highlight(
                        lang,
                        code,
                        self.colors.syntax_theme,
                        |span, color| match color {
                            Some(Rgb(r, g, b)) => {
                                writer.write(Sgr::Fg(Color::Rgb(r, g, b)));
                                writer.push_str(span);
                                writer.write(Sgr::Reset);
                            }
                            None => {
                                writer.push_str(span);
                            }
                        },
                    );

                    MarkupRich::Text {
                        range: self.text.push_str_get_range(writer.as_str()),
                        alignment: HorizontalAlignment::Left,
                    }
                }
                MarkupPlain::Image { index } => MarkupRich::Image { index },
                MarkupPlain::ImageError { text } => {
                    let error = self.text.slice(text);
                    writer.write(format_args!(
                        "{}{error}{}",
                        Sgr::Fg(Color::Red),
                        Sgr::reset_fg()
                    ));
                    utils::text_wrap(writer, width);
                    let range = self.text.push_str_get_range(writer.as_str());
                    MarkupRich::Text {
                        range,
                        alignment: HorizontalAlignment::Center,
                    }
                }
                MarkupPlain::ImageDescription { text } => {
                    let markup = self.text.slice(text);
                    let ansi = markup_to_ansi(markup, width, writer);
                    let range = self.text.push_str_get_range(ansi);
                    MarkupRich::Text {
                        range,
                        alignment: HorizontalAlignment::Center,
                    }
                }
                MarkupPlain::Break => MarkupRich::Break,
                MarkupPlain::EmptyLine => MarkupRich::EmptyLine,
            };

            writer.clear();
            rich
        });

        self.rich.extend(plain_to_rich_iter);
        self.total_lines = self.compute_total_lines();
    }

    const fn is_scrollable(&self) -> bool {
        self.options.scrollbar && Scroll::is_scrollable(self.total_lines as usize, self.view)
    }

    fn update_scroll(&mut self) {
        match self.desired_scroll.take() {
            Some(sm) => {
                self.set_scroll(sm);
            }
            None => {
                self.scroll = self
                    .scroll
                    .min(self.total_lines.saturating_sub(self.view.rows));
            }
        }
    }

    fn set_scroll(&mut self, ms: MarkupScroll) {
        let height = self.view.rows;
        self.scroll = match ms {
            MarkupScroll::Up => self.scroll.saturating_sub(1),
            MarkupScroll::Down => (self.scroll + 1).min(self.total_lines.saturating_sub(height)),
            MarkupScroll::PageUp => self.scroll.saturating_sub(height),
            MarkupScroll::PageDown => {
                (self.scroll + height).min(self.total_lines.saturating_sub(height))
            }
            MarkupScroll::Start => 0,
            MarkupScroll::End => self.total_lines.saturating_sub(height),
        };
    }

    fn render_scrollbar(&self, frame: &mut Framebuffer) {
        let Some(scroll_area) = self.scroll_area else {
            return;
        };

        Scrollbar::new(ScrollbarData {
            viewport_height: self.view.rows,
            current_scroll: self.scroll as usize,
            total_items: self.total_lines as usize,
        })
        .with_colors(self.colors.scrollbar)
        .render(scroll_area, frame);
    }

    fn compute_total_lines(&self) -> u16 {
        self.rich
            .iter()
            .copied()
            .map(|item| match item {
                MarkupRich::Text { range, .. } => self.text.slice(range).lines().count() as u16,
                MarkupRich::Image { index } => {
                    let dims = self.images.dims(index);
                    let max_width = self.cell.width(self.size.cols);
                    let resized_dims = dims.resize(dims.with_width(max_width));
                    self.cell.rows(resized_dims.height)
                }
                MarkupRich::Break | MarkupRich::EmptyLine => 1,
            })
            .sum()
    }
}

#[derive(Debug, Clone, Copy)]
enum MarkupPlain {
    Paragraph {
        text: Range<usize>,
        alignment: HorizontalAlignment,
    },
    Heading {
        text: Range<usize>,
        alignment: HorizontalAlignment,
    },
    ListItem {
        text: Range<usize>,
    },
    Code {
        text: Range<usize>,
        _language: Range<usize>,
    },
    Image {
        index: u8,
    },
    ImageError {
        text: Range<usize>,
    },
    ImageDescription {
        text: Range<usize>,
    },
    Break,
    EmptyLine,
}

#[derive(Debug, Clone, Copy)]
enum MarkupRich {
    Text {
        range: Range<usize>,
        alignment: HorizontalAlignment,
    },
    Image {
        index: u8,
    },
    Break,
    EmptyLine,
}

#[derive(Debug, Clone, Copy)]
pub enum MarkupCodeTheme {
    Base16OceanDark,
    Base16OceanLight,
    Base16MochaDark,
    Base16EightiesDark,
    InspiredGitHub,
    SolarizedDark,
    SolarizedLight,
}

impl MarkupCodeTheme {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Base16OceanDark => "base16-ocean.dark",
            Self::Base16OceanLight => "base16-ocean.light",
            Self::Base16MochaDark => "base16-mocha.dark",
            Self::Base16EightiesDark => "base16-eighties.dark",
            Self::InspiredGitHub => "InspiredGitHub",
            Self::SolarizedDark => "Solarized (dark)",
            Self::SolarizedLight => "Solarized (light)",
        }
    }
}

struct CodeHighlighter {
    syntax_set: syntect::parsing::SyntaxSet,
    theme_set: syntect::highlighting::ThemeSet,
}

impl CodeHighlighter {
    fn new() -> Self {
        Self {
            syntax_set: syntect::parsing::SyntaxSet::load_defaults_newlines(),
            theme_set: syntect::highlighting::ThemeSet::load_defaults(),
        }
    }

    fn highlight(
        &self,
        language: &str,
        code: &str,
        theme: MarkupCodeTheme,
        mut f: impl FnMut(&str, Option<Rgb>),
    ) {
        let syntax = if language.is_empty() {
            self.syntax_set.find_syntax_plain_text()
        } else {
            self.syntax_set
                .find_syntax_by_token(language)
                .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        };

        let mut highlighter =
            syntect::easy::HighlightLines::new(syntax, &self.theme_set.themes[theme.as_str()]);
        for code_line in syntect::util::LinesWithEndings::from(code) {
            match highlighter.highlight_line(code_line, &self.syntax_set) {
                Ok(spans) => {
                    for (style, span) in spans {
                        let syntect::highlighting::Color { r, g, b, .. } = style.foreground;
                        f(span, Some(Rgb(r, g, b)));
                    }
                }
                Err(_) => {
                    f(code_line, None);
                }
            }
        }
    }
}

struct MarkupImages {
    images: Vec<MarkupImage>,
    id_start: u8,
    dir: PathBuf,
}

impl MarkupImages {
    const fn new(dir: PathBuf) -> Self {
        Self {
            images: Vec::new(),
            id_start: 90,
            dir,
        }
    }

    fn dims(&self, index: u8) -> Dims {
        self.images[index as usize].image.dims()
    }

    fn load(
        &mut self,
        index: u8,
        source: ImageSource,
        kitty: &mut KittyGraphics,
    ) -> MarkupResult<()> {
        if index == self.images.len() as u8 {
            self.images
                .push(MarkupImage::new((self.id_start + index) as u32));
        }

        match source {
            ImageSource::Filename(name) => {
                if name.is_empty() {
                    return Err(MarkupError::Image(ImageError::NoFilename));
                }

                let path = self.dir.join(name);
                let hash = utils::hash_fast(&path);
                let image = &mut self.images[index as usize];

                if hash == image.hash {
                    return Ok(());
                }

                image
                    .image
                    .load_from_path(&path, kitty)
                    .map_err(|err| MarkupError::Image(ImageError::Kitty(path, err)))?;

                image.hash = hash;
            }
            ImageSource::Math(math, color, cell) => {
                let hash = utils::hash_fast(math);
                let image = &mut self.images[index as usize];

                if hash == image.hash {
                    return Ok(());
                }

                let png = math_to_png(math, color, cell)?;
                image
                    .image
                    .load_from_bytes(png, kitty)
                    .map_err(|err| MarkupError::Math(MathError::Kitty(err)))?;

                image.hash = hash;
            }
        }

        Ok(())
    }

    fn render(
        &mut self,
        index: u8,
        area: Rect,
        rows_outside_top: u16,
        frame: &mut Framebuffer,
        kitty: &mut KittyGraphics,
    ) {
        self.images[index as usize].image.render(
            area,
            frame,
            kitty,
            ImageOptions {
                resize: ResizeMode::FitWidthCropHeight { rows_outside_top },
                horizontal: HorizontalAlignment::Center,
                vertical: VerticalAlignment::Top,
            },
        );
    }
}

struct MarkupImage {
    hash: u64,
    image: KittyImage,
}

impl MarkupImage {
    const fn new(id: u32) -> Self {
        Self {
            hash: 0,
            image: KittyImage::new(id),
        }
    }
}

enum ImageSource<'a> {
    Filename(&'a str),
    Math(&'a str, Rgb, CellDims),
}

fn math_to_png(math: &str, color: Rgb, size: CellDims) -> MarkupResult<Vec<u8>> {
    let layout_options = ratex_layout::LayoutOptions {
        color: ratex_types::Color {
            r: color.0 as f32 / 255.0,
            g: color.1 as f32 / 255.0,
            b: color.2 as f32 / 255.0,
            a: 1.0,
        },
        ..Default::default()
    };
    let render_options = ratex_render::RenderOptions {
        font_size: size.width as f32,
        padding: 0.0,
        background_color: ratex_types::Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        },
        device_pixel_ratio: size.height as f32 / size.width as f32,
        ..Default::default()
    };

    let ast = ratex_parser::parse(math).map_err(|err| MarkupError::Math(MathError::Parse(err)))?;
    let layout = ratex_layout::layout(&ast, &layout_options);
    let display_list = ratex_layout::to_display_list(&layout);
    ratex_render::render_to_png(&display_list, &render_options)
        .map_err(|err| MarkupError::Math(MathError::Render(err)))
}

#[derive(Debug)]
enum MarkupError {
    Image(ImageError),
    Math(MathError),
}

#[derive(Debug)]
enum ImageError {
    NoFilename,
    Kitty(PathBuf, KittyError),
}

#[derive(Debug)]
enum MathError {
    Parse(ratex_parser::ParseError),
    Render(String),
    Kitty(KittyError),
}

impl std::fmt::Display for MarkupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Image(ImageError::NoFilename) => f.write_str("No image filename found"),
            Self::Image(ImageError::Kitty(path, err)) => match err {
                KittyError::Load(err) => f.write_fmt(format_args!(
                    "Failed to load image\n'{}'\ndue to\n\"{}\"",
                    path.display(),
                    err
                )),
                KittyError::Encode(err) => f.write_fmt(format_args!(
                    "Failed to encode image\n'{}'\ndue to\n\"{}\"",
                    path.display(),
                    err
                )),
            },
            Self::Math(MathError::Parse(err)) => {
                f.write_fmt(format_args!("Failed to parse math due to\n\"{}\"", err))
            }
            Self::Math(MathError::Render(msg)) => f.write_fmt(format_args!(
                "Failed to render math as png due to\n\"{}\"",
                msg
            )),
            Self::Math(MathError::Kitty(err)) => f.write_fmt(format_args!(
                "Failed to load or encode math image due to\n\"{}\"",
                err
            )),
        }
    }
}

impl std::error::Error for MarkupError {}

#[derive(Debug)]
pub enum MarkupBlock<'a> {
    Paragraph {
        text: &'a str,
        alignment: HorizontalAlignment,
    },
    Heading {
        text: &'a str,
        alignment: HorizontalAlignment,
        newline: bool,
    },
    List {
        items: ListItems<'a>,
    },
    Code {
        language: &'a str,
        text: &'a str,
    },
    // TODO: Table
    Image {
        description: &'a str,
        path: &'a str,
    },
    Math {
        text: &'a str,
    },
    Comment {
        _text: &'a str,
    },
    Break,
}

pub struct MarkupBlockParser<'a> {
    input: &'a str,
    graphemes: utils::PeekableGraphemesPrevious<'a>,
}

const MARKUP_CENTER: &str = ":";
const MARKUP_RIGHT: &str = ";";
const MARKUP_HEADING: &str = "=";
const MARKUP_COMMENT: &str = "#";
const MARKUP_IMAGE: &str = "!";
const MARKUP_CODE: &str = "`";
const MARKUP_LIST: &str = "-";
const MARKUP_MATH: &str = "$";

impl<'a> Iterator for MarkupBlockParser<'a> {
    type Item = (MarkupBlock<'a>, Range<usize>);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((i, g)) = self.graphemes.next() {
            if g.chars().any(char::is_whitespace) {
                continue;
            }

            if let Some("\n") | Some("\r\n") | None = self.graphemes.previous() {
                let (block, range) = match g {
                    MARKUP_CENTER => self.parse_paragraph(i, HorizontalAlignment::Center),
                    MARKUP_RIGHT => self.parse_paragraph(i, HorizontalAlignment::Right),
                    MARKUP_HEADING => self.parse_heading(i),
                    MARKUP_COMMENT => self.parse_comment(i),
                    MARKUP_IMAGE => self.parse_image(i),
                    MARKUP_CODE => {
                        let ticks = 1 + self.graphemes.count_consecutive(MARKUP_CODE, usize::MAX);
                        if ticks >= 3 {
                            self.parse_code_block(i, ticks)
                        } else {
                            self.parse_paragraph(i, HorizontalAlignment::Left)
                        }
                    }
                    MARKUP_LIST => {
                        let dashes = 1 + self.graphemes.count_consecutive(MARKUP_LIST, usize::MAX);
                        if dashes == 1 {
                            self.parse_list(i)
                        } else if dashes == 3
                            && self.graphemes.count_consecutive_by(|g| g.contains('\n'), 2) == 2
                        {
                            (
                                MarkupBlock::Break,
                                Range {
                                    start: i,
                                    end: i + dashes + 2,
                                },
                            )
                        } else {
                            self.parse_paragraph(i, HorizontalAlignment::Left)
                        }
                    }
                    MARKUP_MATH => {
                        if let Some((_, MARKUP_MATH)) = self.graphemes.next() {
                            self.parse_math(i)
                        } else {
                            self.parse_paragraph(i, HorizontalAlignment::Left)
                        }
                    }
                    _ => self.parse_paragraph(i, HorizontalAlignment::Left),
                };
                return Some((block, range));
            } else {
                return Some(self.parse_paragraph(i, HorizontalAlignment::Left));
            }
        }

        None
    }
}

impl<'a> MarkupBlockParser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            graphemes: utils::PeekableGraphemesPrevious::new(input),
        }
    }

    fn parse_paragraph(
        &mut self,
        start: usize,
        alignment: HorizontalAlignment,
    ) -> (MarkupBlock<'a>, Range<usize>) {
        let start_offset = match alignment {
            HorizontalAlignment::Left => 0,
            HorizontalAlignment::Center | HorizontalAlignment::Right => 1,
        };
        let paragraph_start = start + start_offset;
        let (paragraph_end, end) = match self.graphemes.find_consecutive_by(|g| g.contains('\n'), 2)
        {
            Some((i, g)) => (i, i + g.len()),
            None => (self.input.len(), self.input.len()),
        };

        return (
            MarkupBlock::Paragraph {
                text: self.input[paragraph_start..paragraph_end].trim(),
                alignment,
            },
            Range { start, end },
        );
    }

    fn parse_heading(&mut self, start: usize) -> (MarkupBlock<'a>, Range<usize>) {
        let count = self.graphemes.count_consecutive(MARKUP_HEADING, usize::MAX);
        let (alignment, offset) = match self.graphemes.next() {
            Some((_, g)) => match g {
                MARKUP_CENTER => (HorizontalAlignment::Center, 1),
                MARKUP_RIGHT => (HorizontalAlignment::Right, 1),
                _ => (HorizontalAlignment::Left, 0),
            },
            None => return self.parse_paragraph(start, HorizontalAlignment::Left),
        };

        let heading_start = start + 1 + count + offset;
        let (heading_end, end, newline) = match self.graphemes.find_newline() {
            Some((i, g)) => {
                let newline = self
                    .graphemes
                    .peek()
                    .map(|g| g.contains('\n'))
                    .unwrap_or(false);
                (i, i + g.len(), newline)
            }
            None => (self.input.len(), self.input.len(), false),
        };

        return (
            MarkupBlock::Heading {
                text: self.input[heading_start..heading_end].trim(),
                alignment,
                newline,
            },
            Range { start, end },
        );
    }

    fn parse_list(&mut self, start: usize) -> (MarkupBlock<'a>, Range<usize>) {
        let list_start = start + 1;
        let (list_end, end) = match self.graphemes.find_consecutive_by(|g| g.contains('\n'), 2) {
            Some((i, g)) => (i, i + g.len()),
            None => (self.input.len(), self.input.len()),
        };

        return (
            MarkupBlock::List {
                items: ListItems::new(self.input[list_start..list_end].trim()),
            },
            Range { start, end },
        );
    }

    fn parse_comment(&mut self, start: usize) -> (MarkupBlock<'a>, Range<usize>) {
        let comment_start = start + 1;
        let (comment_end, end) = match self.graphemes.find_by(|g| g.contains('\n')) {
            Some((i, g)) => (i, i + g.len()),
            None => (self.input.len(), self.input.len()),
        };

        return (
            MarkupBlock::Comment {
                _text: self.input[comment_start..comment_end].trim(),
            },
            Range { start, end },
        );
    }

    fn parse_code_block(&mut self, start: usize, ticks: usize) -> (MarkupBlock<'a>, Range<usize>) {
        let lang_start = start + ticks;
        let Some((i, g)) = self.graphemes.find_by(|g| g.contains('\n')) else {
            return (
                MarkupBlock::Code {
                    language: self.input[lang_start..].trim(),
                    text: "",
                },
                Range {
                    start,
                    end: self.input.len(),
                },
            );
        };

        let language = self.input[lang_start..i].trim();
        let code_start = i + g.len();
        loop {
            let Some((code_end, g)) = self.graphemes.find_by(|g| g.contains('\n')) else {
                return (
                    MarkupBlock::Code {
                        language,
                        text: &self.input[code_start..],
                    },
                    Range {
                        start,
                        end: self.input.len(),
                    },
                );
            };

            let end_ticks = self.graphemes.count_consecutive("`", usize::MAX);
            if end_ticks == ticks {
                let end = code_end + g.len() + end_ticks;
                let mut graphemes = utils::grapheme_indices(&self.input[end..]);

                let Some((i, g)) = graphemes.next() else {
                    return (
                        MarkupBlock::Code {
                            language,
                            text: &self.input[code_start..code_end],
                        },
                        Range { start, end },
                    );
                };

                if g.contains('\n') {
                    let Some((i, g)) = graphemes.next() else {
                        return (
                            MarkupBlock::Code {
                                language,
                                text: &self.input[code_start..code_end],
                            },
                            Range {
                                start,
                                end: i + g.len(),
                            },
                        );
                    };

                    if g.contains('\n') {
                        self.graphemes.next();
                        self.graphemes.next();

                        return (
                            MarkupBlock::Code {
                                language,
                                text: &self.input[code_start..code_end],
                            },
                            Range {
                                start,
                                end: i + g.len(),
                            },
                        );
                    }
                }
            }
        }
    }

    fn parse_image(&mut self, start: usize) -> (MarkupBlock<'a>, Range<usize>) {
        let Some((descr_start, "[")) = self.graphemes.next() else {
            return self.parse_paragraph(start, HorizontalAlignment::Left);
        };
        let Some((descr_end, _)) = self.graphemes.find("]") else {
            return self.parse_paragraph(start, HorizontalAlignment::Left);
        };

        let Some((path_start, "(")) = self.graphemes.next() else {
            return self.parse_paragraph(start, HorizontalAlignment::Left);
        };
        let Some((path_end, _)) = self.graphemes.find(")") else {
            return self.parse_paragraph(start, HorizontalAlignment::Left);
        };

        let description = self.input[descr_start + 1..descr_end].trim();
        let path = self.input[path_start + 1..path_end].trim();

        let Some((end, g)) = self.graphemes.next() else {
            return (
                MarkupBlock::Image { description, path },
                Range {
                    start,
                    end: self.input.len(),
                },
            );
        };

        if !g.contains("\n") {
            return self.parse_paragraph(start, HorizontalAlignment::Left);
        }

        let Some((_, g)) = self.graphemes.next() else {
            return (
                MarkupBlock::Image { description, path },
                Range { start, end },
            );
        };

        if !g.contains("\n") {
            return self.parse_paragraph(start, HorizontalAlignment::Left);
        }

        (
            MarkupBlock::Image { description, path },
            Range { start, end },
        )
    }

    fn parse_math(&mut self, start: usize) -> (MarkupBlock<'a>, Range<usize>) {
        let Some((end_math, _)) = self.graphemes.find_consecutive(MARKUP_MATH, 2) else {
            return self.parse_paragraph(start, HorizontalAlignment::Left);
        };

        let start_math = start + 2;
        let end_math = end_math - 1;
        let text = self.input[start_math..end_math].trim();

        if text.is_empty() {
            return self.parse_paragraph(start, HorizontalAlignment::Left);
        }

        let Some((end, g)) = self.graphemes.next() else {
            return (
                MarkupBlock::Math { text },
                Range {
                    start,
                    end: self.input.len(),
                },
            );
        };

        if !g.contains("\n") {
            return self.parse_paragraph(start, HorizontalAlignment::Left);
        }

        let Some((_, g)) = self.graphemes.next() else {
            return (MarkupBlock::Math { text }, Range { start, end });
        };

        if !g.contains("\n") {
            return self.parse_paragraph(start, HorizontalAlignment::Left);
        }

        (MarkupBlock::Math { text }, Range { start, end })
    }
}

#[derive(Debug)]
pub struct ListItems<'a> {
    text: &'a str,
    graphemes: utils::PeekableGraphemesPrevious<'a>,
    start: usize,
}

impl<'a> ListItems<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            text,
            graphemes: utils::PeekableGraphemesPrevious::new(text),
            start: 0,
        }
    }
}

impl<'a> Iterator for ListItems<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start == self.text.len() {
            return None;
        }

        let (end, next_start) = match self
            .graphemes
            .find_pattern_by(|prev, next| prev.contains('\n') && next == "-")
        {
            Some((i, p, n)) => (i - p.len(), i + n.len()),
            None => (self.text.len(), self.text.len()),
        };

        let item = self.text[self.start..end].trim();
        self.start = next_start;
        Some(item)
    }
}

#[derive(Debug, Clone, Copy)]
enum InlineEvent<'a> {
    Text(&'a str),
    Tag(InlineTag),
}

#[derive(Debug, Clone, Copy)]
enum InlineTag {
    BoldStart,
    BoldEnd,
    ItalicStart,
    ItalicEnd,
}

impl InlineTag {
    const fn as_sgr(self) -> Sgr {
        match self {
            InlineTag::BoldStart => Sgr::Bold,
            InlineTag::BoldEnd => Sgr::NotBold,
            InlineTag::ItalicStart => Sgr::Italic,
            InlineTag::ItalicEnd => Sgr::NotItalic,
        }
    }
}

struct InlineParser<'a> {
    input: &'a str,
    chars: utils::PeekableCharsPrevious<'a>,
    start: usize,
    tag: Option<InlineTag>,
    bold: bool,
    italic: bool,
}

impl<'a> InlineParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: utils::PeekableCharsPrevious::new(input),
            start: 0,
            tag: None,
            bold: false,
            italic: false,
        }
    }
}

impl<'a> Iterator for InlineParser<'a> {
    type Item = InlineEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(tag) = self.tag.take() {
            return Some(InlineEvent::Tag(tag));
        }

        /// Looks for valid inline tag from the current, previous and next chars.
        /// If valid, either a start tag or end tag is returned based on the tag condition.
        /// That is, if tag condition is true, then the end tag will be returned,
        /// since this implies that a start tag has been found earlier.
        fn parse_tag(
            curr: char,
            prev: Option<char>,
            next: Option<char>,
            tag_start: InlineTag,
            tag_end: InlineTag,
            tag_cond: &mut bool,
        ) -> Option<InlineTag> {
            match (prev, next) {
                // Look for both tags based on tag condition
                (Some(prev), Some(next)) => {
                    if *tag_cond {
                        if !prev.is_whitespace() && prev != curr {
                            *tag_cond = false;
                            return Some(tag_end);
                        }
                    } else {
                        if !next.is_whitespace() && next != curr {
                            *tag_cond = true;
                            return Some(tag_start);
                        }
                    }
                }
                // First char, only need to look for start tag
                (None, Some(next)) => {
                    if !*tag_cond {
                        if !next.is_whitespace() && next != curr {
                            *tag_cond = true;
                            return Some(tag_start);
                        }
                    }
                }
                // Last char, only need to look for end tag
                (Some(prev), None) => {
                    if *tag_cond {
                        if !prev.is_whitespace() && prev != curr {
                            *tag_cond = false;
                            return Some(tag_end);
                        }
                    }
                }
                // Nothing to look for
                (None, None) => {}
            }

            None
        }

        while let Some((i, c)) = self.chars.next() {
            let tag = match c {
                '*' => parse_tag(
                    c,
                    self.chars.previous(),
                    self.chars.peek(),
                    InlineTag::BoldStart,
                    InlineTag::BoldEnd,
                    &mut self.bold,
                ),
                '_' => parse_tag(
                    c,
                    self.chars.previous(),
                    self.chars.peek(),
                    InlineTag::ItalicStart,
                    InlineTag::ItalicEnd,
                    &mut self.italic,
                ),
                _ => None,
            };

            if let Some(tag) = tag {
                let text = &self.input[self.start..i];
                self.start = i + c.len_utf8();
                if text.is_empty() {
                    return Some(InlineEvent::Tag(tag));
                } else {
                    self.tag = Some(tag);
                    return Some(InlineEvent::Text(text));
                }
            }
        }

        let remaining = &self.input[self.start..];
        if remaining.is_empty() {
            None
        } else {
            self.start = self.input.len();
            Some(InlineEvent::Text(remaining))
        }
    }
}
