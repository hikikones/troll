use terminal::*;

use crate::{Scroll, Scrollbar, ScrollbarColors, ScrollbarData};

#[derive(Debug)]
pub struct AnsiViewer {
    ansi: String,
    scroll: u16,
    hash: u64,
    size: Size,
    view: Size,
    lines: u16,
    scroll_area: Option<Rect>,
    options: AnsiViewerOptions,
    colors: AnsiViewerColors,
}

#[derive(Debug, Clone, Copy)]
pub struct AnsiViewerOptions {
    pub padding: Margin,
    pub scrollbar: bool,
    pub scrollbar_margin: u16,
    pub alignment: HorizontalAlignment,
}

impl AnsiViewerOptions {
    pub const fn new() -> Self {
        Self {
            padding: Margin::ZERO,
            scrollbar: false,
            scrollbar_margin: 1,
            alignment: HorizontalAlignment::Left,
        }
    }
}

impl Default for AnsiViewerOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AnsiViewerColors {
    pub scrollbar: ScrollbarColors,
}

impl AnsiViewerColors {
    pub const fn new() -> Self {
        Self {
            scrollbar: ScrollbarColors::DEFAULT,
        }
    }
}

impl Default for AnsiViewerColors {
    fn default() -> Self {
        Self::new()
    }
}

impl AnsiViewer {
    pub const fn new() -> Self {
        Self {
            ansi: String::new(),
            scroll: 0,
            hash: 0,
            size: Size::ZERO,
            view: Size::ZERO,
            lines: 0,
            scroll_area: None,
            options: AnsiViewerOptions::new(),
            colors: AnsiViewerColors::new(),
        }
    }

    pub const fn with_padding(mut self, padding: Margin) -> Self {
        self.options.padding = padding;
        self
    }

    pub const fn with_scrollbar(mut self, margin: u16) -> Self {
        self.options.scrollbar = true;
        self.options.scrollbar_margin = margin;
        self
    }

    pub const fn with_alignment(mut self, alignment: HorizontalAlignment) -> Self {
        self.options.alignment = alignment;
        self
    }

    pub const fn with_colors(mut self, colors: AnsiViewerColors) -> Self {
        self.colors = colors;
        self
    }

    pub const fn set_padding(&mut self, padding: Margin) -> &mut Self {
        self.options.padding = padding;
        self
    }

    pub const fn set_scrollbar(&mut self, enabled: bool) -> &mut Self {
        self.options.scrollbar = enabled;
        self
    }

    pub const fn set_alignment(&mut self, alignment: HorizontalAlignment) -> &mut Self {
        self.options.alignment = alignment;
        self
    }

    pub const fn set_colors(&mut self, colors: AnsiViewerColors) -> &mut Self {
        self.colors = colors;
        self
    }

    pub const fn input(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Down => self.move_down(1),
            KeyCode::Up => self.move_up(1),
            KeyCode::PageDown => self.move_down(self.view.rows),
            KeyCode::PageUp => self.move_up(self.view.rows),
            KeyCode::End => self.move_to_end(),
            KeyCode::Home => self.move_to_start(),
            _ => false,
        }
    }

    pub const fn move_down(&mut self, n: u16) -> bool {
        self.set_scroll(self.scroll + n)
    }

    pub const fn move_up(&mut self, n: u16) -> bool {
        self.set_scroll(self.scroll.saturating_sub(n))
    }

    pub const fn move_to_end(&mut self) -> bool {
        let max = self.max_scroll();

        if self.scroll == max {
            return false;
        }

        self.scroll = max;
        true
    }

    pub const fn move_to_start(&mut self) -> bool {
        if self.scroll == 0 {
            return false;
        }

        self.scroll = 0;
        true
    }

    pub fn render(&mut self, area: Rect, frame: &mut Framebuffer, ansi: &str) {
        let inner = area.inner(self.options.padding);

        if inner.is_empty() {
            return;
        }

        self.process(ansi, area, inner);
        self.update_scroll();

        let view_start = self.scroll;
        let view_end = self.scroll + inner.size.rows;
        let align = self.options.alignment;

        let mut pos = inner.pos;
        let mut i = 0;

        for line in self.ansi.lines() {
            let col = align.calc_from(pos.col, self.view.cols, line);
            frame.cursor(pos.with_col(col));

            let is_in_view = i >= view_start && i < view_end;

            for ga in utils::GraphemeAnsiIter::new(line) {
                match ga {
                    utils::GraphemeOrAnsi::Grapheme(g) => {
                        if is_in_view {
                            frame.print_str(g.0);
                        }
                    }
                    utils::GraphemeOrAnsi::Ansi(s) => {
                        frame.print_str(s);
                    }
                }
            }

            if is_in_view {
                pos.row += 1;
            }

            i += 1;
        }

        self.render_scrollbar(frame);
    }

    fn process(&mut self, ansi: &str, mut area: Rect, mut inner: Rect) {
        let hash = utils::hash_fast(ansi);

        if self.hash != hash || self.size != area.size {
            self.scroll_area = None;

            // First pass
            self.relayout(ansi, inner.cols());

            if self.is_scrollable(inner.size) {
                let scroll_area = Scroll::make_area(&mut area, self.options.scrollbar_margin);
                inner.sub_cols(scroll_area.cols() + self.options.scrollbar_margin);
                self.scroll_area = Some(scroll_area);

                // Second pass for scroll area
                self.relayout(ansi, inner.cols());
            }

            self.hash = hash;
            self.size = area.size;
            self.view = inner.size;
        }
    }

    fn relayout(&mut self, ansi: &str, max_width: u16) {
        self.ansi.clear();

        self.ansi.push_str(ansi);
        utils::text_wrap(&mut self.ansi, max_width);
        self.lines = self.ansi.lines().count() as u16;
    }

    const fn set_scroll(&mut self, v: u16) -> bool {
        let old = self.scroll;
        let max = self.max_scroll();

        self.scroll = if v > max { max } else { v };

        self.scroll != old
    }

    const fn update_scroll(&mut self) {
        let max = self.max_scroll();
        if self.scroll > max {
            self.scroll = max;
        }
    }

    const fn max_scroll(&self) -> u16 {
        self.lines.saturating_sub(self.view.rows)
    }

    const fn is_scrollable(&self, list_view: Size) -> bool {
        self.options.scrollbar && Scroll::is_scrollable(self.lines as usize, list_view, 10)
    }

    fn render_scrollbar(&self, frame: &mut Framebuffer) {
        let Some(scroll_area) = self.scroll_area else {
            return;
        };

        Scrollbar::colors(self.colors.scrollbar).render(
            scroll_area,
            frame,
            ScrollbarData {
                viewport_height: self.view.rows,
                current_scroll: self.scroll as usize,
                total_items: self.lines as usize,
            },
        );
    }
}
