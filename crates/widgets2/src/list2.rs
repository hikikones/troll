use std::{
    cmp::Ordering,
    ops::{Range, RangeInclusive},
};

use terminal::*;

use crate::{Scroll, ScrollData, ScrollMargins};

#[derive(Debug)]
pub struct List {
    index: usize,
    selector: Option<usize>,
    scroll: usize,
    scrolloff: u16,
    list_height: u16,
    last_height: u16,
    len: usize,
}

impl Default for List {
    fn default() -> Self {
        Self::new()
    }
}

impl List {
    pub const fn new() -> Self {
        Self {
            index: 0,
            selector: None,
            scroll: 0,
            scrolloff: 0,
            list_height: 0,
            last_height: 0,
            len: 0,
        }
    }

    pub const fn with_index(mut self, index: usize) -> Self {
        self.index = index;
        self
    }

    pub const fn with_scroll(mut self, scroll: usize) -> Self {
        self.scroll = scroll;
        self
    }

    pub const fn with_scrolloff(mut self, vertical: u16) -> Self {
        self.scrolloff = vertical;
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

    pub const fn set_len(&mut self, len: usize) -> &mut Self {
        self.len = len;
        self
    }

    pub const fn set_index(&mut self, i: usize) -> &mut Self {
        self.index = i;
        self
    }

    pub const fn set_selector(&mut self, s: Option<usize>) -> &mut Self {
        self.selector = s;
        self
    }

    pub fn input(&mut self, key: Key) -> bool {
        match key.code {
            KeyCode::Down => self.move_down(1, key.shift()),
            KeyCode::Up => self.move_up(1, key.shift()),
            KeyCode::PageDown => self.move_down(self.list_height as usize, key.shift()),
            KeyCode::PageUp => self.move_up(self.list_height as usize, key.shift()),
            KeyCode::End => self.move_to_end(key.shift()),
            KeyCode::Home => self.move_to_start(key.shift()),
            KeyCode::Char('a') => {
                if key.ctrl() {
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

    pub fn render(&mut self, area: Rect) -> Range<usize> {
        self.list_height = area.size.rows;

        if area.is_empty() {
            return 0..0;
        }

        let height = area.size.rows;

        self.clamp_index_and_selector();
        self.update_scroll(height);

        let start = self.scroll;
        let end = (start + height as usize).min(self.len);
        start..end
    }

    pub const fn reset(&mut self) {
        self.index = 0;
        self.scroll = 0;
        self.selector = None;
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
            viewport_height: area_height,
            margins: ScrollMargins::vertical(self.scrolloff),
        });
        self.last_height = area_height;
    }
}
