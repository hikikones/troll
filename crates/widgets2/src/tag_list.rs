use terminal::*;

use crate::{Scroll, ScrollData, ScrollMargins, Scrollbar, ScrollbarColors, ScrollbarData};

pub struct TagList {
    index: usize,
    index_col: u16,
    index_row: u16,
    scroll: u16,
    total_lines: u16,
    total_items: usize,
    list_width: u16,
    last_size: Size,
    options: TagListOptions,
    colors: TagListColors,
}

pub trait TagItem {
    fn width(&self) -> u16;
}

#[derive(Debug, Clone, Copy)]
pub struct TagListOptions {
    pub gap: u16,
    pub padding: Margin,
    pub scrollbar: bool,
    pub scrollbar_margin: u16,
}

impl TagListOptions {
    pub const fn new() -> Self {
        Self {
            gap: 2,
            padding: Margin::ZERO,
            scrollbar: false,
            scrollbar_margin: 1,
        }
    }
}

impl Default for TagListOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TagListColors {
    pub scrollbar: ScrollbarColors,
}

impl TagListColors {
    pub const fn new() -> Self {
        Self {
            scrollbar: ScrollbarColors::DEFAULT,
        }
    }
}

impl Default for TagListColors {
    fn default() -> Self {
        Self::new()
    }
}

impl TagList {
    pub const fn new() -> Self {
        Self {
            index: 0,
            index_col: 0,
            index_row: 0,
            scroll: 0,
            total_lines: 0,
            total_items: 0,
            list_width: 0,
            last_size: Size::ZERO,
            options: TagListOptions::new(),
            colors: TagListColors::new(),
        }
    }

    pub const fn with_gap(mut self, gap: u16) -> Self {
        self.options.gap = gap;
        self
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

    pub const fn with_colors(mut self, colors: TagListColors) -> Self {
        self.colors = colors;
        self
    }

    pub const fn index(&self) -> usize {
        self.index
    }

    pub const fn set_index(&mut self, i: usize) -> &mut Self {
        self.index = i;
        self
    }

    pub const fn set_colors(&mut self, colors: TagListColors) -> &mut Self {
        self.colors = colors;
        self
    }

    pub fn input<T: TagItem>(&mut self, key: KeyCode, items: impl IntoIterator<Item = T>) -> bool {
        let old_index = self.index;

        match key {
            KeyCode::Right => {
                self.index = (self.index + 1).min(self.total_items.saturating_sub(1));
            }
            KeyCode::Left => {
                self.index = self.index.saturating_sub(1);
            }
            KeyCode::Down => {
                self.index = if self.index_row == self.total_lines.saturating_sub(1) {
                    self.total_items.saturating_sub(1)
                } else {
                    let (mut next_index, mut distance) = (0, u16::MAX);
                    for (i, x, y, _) in
                        iter_items_in_col_row(self.list_width, self.options.gap, items)
                            .skip(self.index + 1)
                    {
                        if y == self.index_row + 1 {
                            let d = self.index_col.abs_diff(x);
                            if d <= distance {
                                next_index = i;
                                distance = d;
                            }
                        } else if y > self.index_row + 1 {
                            break;
                        }
                    }
                    next_index
                };
            }
            KeyCode::Up => {
                self.index = if self.index_row == 0 {
                    0
                } else {
                    let (mut next_index, mut distance) = (0, u16::MAX);
                    for (i, x, y, _) in
                        iter_items_in_col_row(self.list_width, self.options.gap, items)
                    {
                        if y == self.index_row.saturating_sub(1) {
                            let d = self.index_col.abs_diff(x);
                            if d <= distance {
                                next_index = i;
                                distance = d;
                            }
                        } else if y >= self.index_row {
                            break;
                        }
                    }
                    next_index
                };
            }
            KeyCode::Home => {
                self.index = 0;
            }
            KeyCode::End => {
                self.index = self.total_items.saturating_sub(1);
            }
            _ => {}
        }

        self.index != old_index
    }

    pub fn render<T: TagItem>(
        &mut self,
        mut area: Rect,
        frame: &mut Framebuffer,
        items: impl IntoIterator<Item = T, IntoIter: Clone>,
        mut render_item: impl FnMut(Rect, &mut Framebuffer, T, bool),
    ) {
        let mut inner = area.inner(self.options.padding);

        if inner.is_empty() {
            return;
        }

        let items = items.into_iter();

        // First pass
        self.process_items(inner, items.clone());

        let scroll_area = if self.is_scrollable(inner.size) {
            let scroll_area = Scroll::make_scroll_area(&mut area, self.options.scrollbar_margin);
            inner.sub_cols(scroll_area.cols() + self.options.scrollbar_margin);

            // Second pass after scrollbar
            self.process_items(inner, items.clone());
            Some(scroll_area)
        } else {
            None
        };

        self.update_scroll(area.size, inner.rows());

        self.last_size = area.size;
        self.list_width = inner.cols();

        // Render tags
        for (i, x, y, item) in iter_items_in_col_row(inner.cols(), self.options.gap, items) {
            if y >= inner.size.rows + self.scroll {
                break;
            }

            if y >= self.scroll {
                let tag_area = Rect {
                    pos: Pos {
                        col: inner.col() + x,
                        row: inner.row() + y.saturating_sub(self.scroll),
                    },
                    size: Size {
                        cols: item.width().min(inner.cols().saturating_sub(x)),
                        rows: 1,
                    },
                };
                render_item(tag_area, frame, item, self.index == i);
            }
        }

        // Render scrollbar
        if let Some(scroll_area) = scroll_area {
            Scrollbar::colors(self.colors.scrollbar).render(
                scroll_area,
                frame,
                ScrollbarData {
                    viewport_height: inner.size.rows,
                    current_scroll: self.scroll as usize,
                    total_items: self.total_lines as usize,
                },
            );
        }
    }

    fn process_items<T: TagItem>(
        &mut self,
        area: Rect,
        items: impl IntoIterator<Item = T>,
    ) -> &mut Self {
        self.total_items = 0;

        for (i, x, y, _) in iter_items_in_col_row(area.cols(), self.options.gap, items) {
            if self.index == i {
                self.index_col = x;
                self.index_row = y;
            }
            self.total_items += 1;
            self.total_lines = y + 1;
        }

        self
    }

    const fn is_scrollable(&self, list_size: Size) -> bool {
        self.options.scrollbar && Scroll::is_scrollable(self.total_lines as usize, list_size, 10)
    }

    fn update_scroll(&mut self, area_size: Size, list_height: u16) {
        let scroll = if self.last_size != area_size {
            // Refresh scroll on window resize
            0
        } else {
            self.scroll
        };
        self.scroll = Scroll::calc(ScrollData {
            current_index: self.index_row as usize,
            current_scroll: scroll as usize,
            total_lines: self.total_lines as usize,
            viewport_height: list_height,
            margins: ScrollMargins::ZERO,
        }) as u16;
    }
}

fn iter_items_in_col_row<T: TagItem>(
    max_width: u16,
    item_gap: u16,
    items: impl IntoIterator<Item = T>,
) -> impl Iterator<Item = (usize, u16, u16, T)> {
    let (mut x, mut y) = (0, 0);
    items.into_iter().enumerate().map(move |(i, item)| {
        let item_width = item.width();
        if x + item_width > max_width {
            x = 0;
            if i > 0 {
                y += 1;
            }
        }

        let (col, row) = (x, y);

        x += item_width + item_gap;

        (i, col, row, item)
    })
}
