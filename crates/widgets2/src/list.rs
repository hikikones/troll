use std::{
    cmp::Ordering,
    ops::{Range, RangeInclusive},
};

use terminal::*;

use crate::{Scroll, ScrollData, ScrollMargins, Scrollbar, ScrollbarColors, ScrollbarData};

pub struct List {
    index: usize,
    selector: Option<usize>,
    scroll: usize,
    options: ListOptions,
    colors: ListColors,
    last_height: u16,
    list_height: u16,
    len: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct ListOptions {
    pub scrolloff: ScrollMargins,
    pub padding: Margin,
    pub scrollbar: bool,
    pub scrollbar_margin: u16,
}

impl ListOptions {
    pub const fn new() -> Self {
        Self {
            scrolloff: ScrollMargins::ZERO,
            padding: Margin::ZERO,
            scrollbar: false,
            scrollbar_margin: 1,
        }
    }
}

impl Default for ListOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ListColors {
    pub scrollbar: ScrollbarColors,
}

impl ListColors {
    pub const fn new() -> Self {
        Self {
            scrollbar: ScrollbarColors::DEFAULT,
        }
    }
}

impl Default for ListColors {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListIndex {
    Selected,
    Selection,
    Normal,
}

pub struct TableLayout<const N: usize> {
    gap: u16,
    constraints: [Constraint; N],
}

impl<const N: usize> TableLayout<N> {
    pub const fn new(gap: u16, constraints: [Constraint; N]) -> Self {
        Self { gap, constraints }
    }

    const fn areas(self, area: Rect) -> [Rect; N] {
        area.split_horizontal(self.gap, self.constraints)
    }
}

impl List {
    pub const fn new() -> Self {
        Self {
            index: 0,
            selector: None,
            scroll: 0,
            options: ListOptions::new(),
            colors: ListColors::new(),
            last_height: 0,
            list_height: 0,
            len: 0,
        }
    }

    pub const fn with_index(mut self, index: usize) -> Self {
        self.index = index;
        self
    }

    pub const fn with_scrolloff(mut self, margins: ScrollMargins) -> Self {
        self.set_scrolloff(margins);
        self
    }

    pub const fn with_padding(mut self, padding: Margin) -> Self {
        self.set_padding(padding);
        self
    }

    pub const fn with_scrollbar(mut self, margin: u16) -> Self {
        self.options.scrollbar = true;
        self.options.scrollbar_margin = margin;
        self
    }

    pub const fn with_colors(mut self, colors: ListColors) -> Self {
        self.set_colors(colors);
        self
    }

    pub const fn index(&self) -> usize {
        self.index
    }

    pub const fn selector(&self) -> Option<usize> {
        self.selector
    }

    pub const fn scroll(&self) -> usize {
        self.scroll
    }

    pub const fn set_index(&mut self, i: usize) -> &mut Self {
        self.index = i;
        self
    }

    pub const fn set_selector(&mut self, s: Option<usize>) -> &mut Self {
        self.selector = s;
        self
    }

    pub const fn set_scrolloff(&mut self, margins: ScrollMargins) -> &mut Self {
        self.options.scrolloff = margins;
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

    pub const fn set_colors(&mut self, colors: ListColors) -> &mut Self {
        self.colors = colors;
        self
    }

    pub fn input(&mut self, key_pressed: KeyCode, key_modifiers: KeyModifiers) -> bool {
        let ctrl = key_modifiers.contains(KeyModifiers::CONTROL);
        let shift = key_modifiers.contains(KeyModifiers::SHIFT);

        match key_pressed {
            KeyCode::Down => self.move_down(1, shift),
            KeyCode::Up => self.move_up(1, shift),
            KeyCode::PageDown => self.move_down(self.list_height as usize, shift),
            KeyCode::PageUp => self.move_up(self.list_height as usize, shift),
            KeyCode::End => self.move_to_end(shift),
            KeyCode::Home => self.move_to_start(shift),
            KeyCode::Char('a') => {
                if ctrl {
                    self.select_all()
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn move_down(&mut self, n: usize, shift: bool) -> bool {
        self.set_index_and_selector(self.index + n, shift)
    }

    pub fn move_up(&mut self, n: usize, shift: bool) -> bool {
        self.set_index_and_selector(self.index.saturating_sub(n), shift)
    }

    pub fn move_to_end(&mut self, shift: bool) -> bool {
        self.set_index_and_selector(usize::MAX, shift)
    }

    pub fn move_to_start(&mut self, shift: bool) -> bool {
        self.set_index_and_selector(0, shift)
    }

    pub fn move_selection_up(&mut self) -> bool {
        let Some(selector) = self.selector else {
            return self.set_index_and_selector(self.index.saturating_sub(1), false);
        };

        let index = self.index;
        let i = index.saturating_sub(1);
        let s = selector.saturating_sub(1);

        if i == index || s == selector {
            return false;
        }

        self.index = i;
        self.selector = Some(s);
        true
    }

    pub fn move_selection_down(&mut self) -> bool {
        let Some(selector) = self.selector else {
            return self.set_index_and_selector(self.index + 1, false);
        };

        let index = self.index;
        let max_index = self.len.saturating_sub(1);
        let i = usize::min(index + 1, max_index);
        let s = usize::min(selector + 1, max_index);

        if i == index || s == selector {
            return false;
        }

        self.index = i;
        self.selector = Some(s);
        true
    }

    pub fn select_all(&mut self) -> bool {
        let old_index = self.index;
        let old_selector = self.selector;

        self.index = 0;
        self.selector = Some(self.len.saturating_sub(1));
        self.selector.take_if(|s| *s == self.index);

        old_index != self.index || old_selector != self.selector
    }

    pub fn selection(&self) -> Option<Range<usize>> {
        self.selector
            .and_then(|selector| match self.index.cmp(&selector) {
                Ordering::Less => Some((self.index + 1)..(selector + 1)),
                Ordering::Greater => Some(selector..self.index),
                Ordering::Equal => None,
            })
    }

    pub fn selection_inclusive(&self) -> RangeInclusive<usize> {
        self.selector
            .map(|selector| {
                if self.index < selector {
                    self.index..=selector
                } else {
                    selector..=self.index
                }
            })
            .unwrap_or(self.index..=self.index)
    }

    pub const fn reset(&mut self) {
        self.index = 0;
        self.scroll = 0;
        self.selector = None;
    }

    pub fn render<T>(
        &mut self,
        mut area: Rect,
        frame: &mut Framebuffer,
        items: impl IntoIterator<Item = T, IntoIter: ExactSizeIterator>,
        mut render_line: impl FnMut(Rect, &mut Framebuffer, T, ListIndex),
    ) {
        let mut inner = area.inner(self.options.padding);

        if inner.is_empty() {
            return;
        }

        let items = items.into_iter();
        self.len = items.len();

        // Prepare scrollbar
        let scroll_area = self.prepare_scrollbar(&mut area, &mut inner);

        // Update state
        self.list_height = inner.size.rows;
        self.clamp_index_and_selector();
        self.update_scroll(area.size.rows);
        self.last_height = area.size.rows;

        // Render list items
        let selection = self.selection_inclusive();
        let mut line = inner.with_rows(1);

        items
            .enumerate()
            .skip(self.scroll)
            .take(inner.size.rows as usize)
            .for_each(|(i, item)| {
                let list_index = if i == self.index {
                    ListIndex::Selected
                } else if selection.contains(&i) {
                    ListIndex::Selection
                } else {
                    ListIndex::Normal
                };

                render_line(line, frame, item, list_index);

                line.pos.row += 1;
            });

        // Render scrollbar
        if let Some(scroll_area) = scroll_area {
            self.render_scrollbar(scroll_area, frame);
        }
    }

    pub fn render_table<T, const N: usize>(
        &mut self,
        mut area: Rect,
        frame: &mut Framebuffer,
        items: impl IntoIterator<Item = T, IntoIter: ExactSizeIterator>,
        layout: TableLayout<N>,
        render_header: impl FnOnce(Rect, &mut Framebuffer, [Rect; N]),
        mut render_row: impl FnMut(Rect, &mut Framebuffer, [Rect; N], T, ListIndex),
    ) {
        let mut inner = area.inner(self.options.padding);

        if inner.is_empty() {
            return;
        }

        let items = items.into_iter();
        self.len = items.len();

        let header_row = area.row();

        // Prepare scrollbar
        let scroll_area = self.prepare_scrollbar(&mut area, &mut inner);

        inner.shrink_down(1);

        // Render header
        let header_area = inner.with_row(header_row);
        let mut areas = layout.areas(header_area);
        render_header(header_area, frame, areas);

        // Update state
        self.list_height = inner.size.rows;
        self.clamp_index_and_selector();
        self.update_scroll(area.size.rows);
        self.last_height = area.size.rows;

        // Render table items
        let selection = self.selection_inclusive();
        let mut line = inner.with_rows(1);

        items
            .enumerate()
            .skip(self.scroll)
            .take(inner.size.rows as usize)
            .for_each(|(i, item)| {
                let list_index = if i == self.index {
                    ListIndex::Selected
                } else if selection.contains(&i) {
                    ListIndex::Selection
                } else {
                    ListIndex::Normal
                };

                areas.iter_mut().for_each(|a| a.add_row(1));
                render_row(line, frame, areas, item, list_index);
                line.add_row(1);
            });

        // Render scrollbar
        if let Some(scroll_area) = scroll_area {
            self.render_scrollbar(scroll_area, frame);
        }
    }

    fn set_index_and_selector(&mut self, i: usize, shift: bool) -> bool {
        let old_index = self.index;
        let old_selector = self.selector;

        if shift {
            if self.selector.is_none() {
                self.selector = Some(self.index);
            }
        } else {
            self.selector = None;
        }

        self.index = usize::min(i, self.len.saturating_sub(1));
        self.selector.take_if(|s| *s == self.index);

        old_index != self.index || old_selector != self.selector
    }

    fn clamp_index_and_selector(&mut self) {
        let max_idx = self.len.saturating_sub(1);
        self.index = self.index.min(max_idx);
        self.selector = self.selector.map(|selector| selector.min(max_idx));
    }

    const fn prepare_scrollbar(&self, area: &mut Rect, inner: &mut Rect) -> Option<Rect> {
        let is_scrollable =
            self.options.scrollbar && Scroll::is_scrollable(self.len, inner.size, 10);

        if !is_scrollable {
            return None;
        }

        let scroll_area = Scroll::make_scroll_area(area, self.options.scrollbar_margin);
        inner.sub_cols(scroll_area.cols() + self.options.scrollbar_margin);
        Some(scroll_area)
    }

    const fn update_scroll(&mut self, area_height: u16) {
        let scroll = if self.last_height != area_height {
            // Refresh scroll on window resize
            0
        } else {
            self.scroll
        };
        self.scroll = Scroll::calc(ScrollData {
            current_index: self.index,
            current_scroll: scroll,
            total_lines: self.len,
            viewport_height: self.list_height,
            margins: self.options.scrolloff,
        });
    }

    fn render_scrollbar(&self, area: Rect, frame: &mut Framebuffer) {
        Scrollbar::colors(self.colors.scrollbar).render(
            area,
            frame,
            ScrollbarData {
                current_scroll: self.scroll,
                total_items: self.len,
                viewport_height: self.list_height,
            },
        );
    }
}
