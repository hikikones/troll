use std::fmt::Display;

use terminal::*;
use utils::StringExt;

use crate::{Scroll, Scrollbar, ScrollbarColors, ScrollbarData};

#[derive(Debug)]
pub struct AnsiViewer {
    ansi: String,
    wrapped: String,
    scroll: u16,
    hash: u64,
    size: Size,
    view: Size,
    lines: u16,
    scroll_area: Option<Rect>,
    options: AnsiViewerOptions,
    colors: AnsiViewerColors,
}

impl Default for AnsiViewer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AnsiViewerOptions {
    pub padding: Margin,
    pub scrollbar: bool,
    pub scrollbar_margin: u16,
    pub alignment: HorizontalAlignment,
    // pub disabled: bool, TODO
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
    // pub disabled: Color, TODO
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
            wrapped: String::new(),
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

    pub const fn current_scroll(&self) -> u16 {
        self.scroll
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

    pub fn push(&mut self, ch: char) {
        self.ansi.push(ch);
    }

    pub fn push_repeat(&mut self, ch: char, n: u16) {
        self.ansi.push_repeat(ch, n as usize);
    }

    pub fn push_str(&mut self, text: &str) {
        self.ansi.push_str(text);
    }

    pub fn push_str_fg(&mut self, text: &str, fg: Color) {
        self.ansi.push_str_fg(text, fg);
    }

    pub fn push_fmt(&mut self, text: impl Display) {
        self.ansi.push_fmt(text);
    }

    pub fn push_fmt_fg(&mut self, text: impl Display, fg: Color) {
        self.ansi.push_fmt_fg(text, fg);
    }

    pub fn push_iter<I>(&mut self, iter: I)
    where
        I: IntoIterator,
        String: Extend<I::Item>,
    {
        self.ansi.extend(iter);
    }

    pub fn clear(&mut self) {
        self.ansi.clear();
    }

    pub fn render(&mut self, area: Rect, frame: &mut Frame) {
        let inner = area.inner(self.options.padding);

        if inner.is_empty() {
            return;
        }

        self.process(area, inner);
        self.update_scroll();

        let view_start = self.scroll;
        let view_end = self.scroll + inner.size.rows;

        let max_width = self.view.cols;
        let mut pos = inner.pos;

        for (line, i) in self.wrapped.lines().zip(0..) {
            let col = self.options.alignment.calc_line(pos.col, max_width, line);
            frame.cursor(pos.with_col(col));

            let is_in_view = i >= view_start && i < view_end;

            if is_in_view {
                frame.print_str(line);
                pos.add_row(1);
            } else {
                let sgr = SgrIter::new(line).with_filter(true);
                frame.print_iter(sgr);
            }
        }

        self.render_scrollbar(frame);
    }

    fn process(&mut self, area: Rect, mut inner: Rect) {
        let hash = utils::hash_fast(self.ansi.as_str());

        if !(self.hash != hash || self.size != area.size) {
            return;
        }

        self.scroll_area = None;

        // First pass
        self.relayout(inner.cols());

        if self.is_scrollable(inner.size) {
            let scroll_area = area.right_edge();
            inner.sub_cols(scroll_area.cols() + self.options.scrollbar_margin);
            self.scroll_area = Some(scroll_area);

            // Second pass for scroll area
            self.relayout(inner.cols());
        }

        self.hash = hash;
        self.size = area.size;
        self.view = inner.size;
    }

    fn relayout(&mut self, max_width: u16) {
        self.wrapped.clear();

        self.wrapped.push_str(self.ansi.as_str());
        utils::text_wrap(&mut self.wrapped, max_width);
        self.lines = self.wrapped.lines().count() as u16;
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
        self.options.scrollbar && Scroll::is_scrollable(self.lines as usize, list_view)
    }

    fn render_scrollbar(&self, frame: &mut Frame) {
        let Some(scroll_area) = self.scroll_area else {
            return;
        };

        Scrollbar::new(ScrollbarData {
            viewport_height: self.view.rows,
            current_scroll: self.scroll as usize,
            total_items: self.lines as usize,
        })
        .with_colors(self.colors.scrollbar)
        .render(scroll_area, frame);
    }
}
