use std::ops::Range;

use terminal::*;

use crate::{Scroll, ScrollData, ScrollMargins};

// TODO: Scrolling assumes all chars/graphemes are one width.

pub struct Prompt {
    input: String,
    placeholder: &'static str,
    cursor: usize,
    selector: Option<usize>,
    scroll: u16,
    margins: u16,
    disabled: bool,
    colors: PromptColors,
    area_pos: Pos,
    last_width: u16,
}

pub struct PromptColors {
    pub placeholder: Color,
    pub disabled: Color,
}

impl PromptColors {
    pub const fn new() -> Self {
        Self {
            placeholder: Color::Indexed(240),
            disabled: Color::Indexed(238),
        }
    }

    pub const fn all(color: Color) -> Self {
        Self {
            placeholder: color,
            disabled: color,
        }
    }
}

impl Default for PromptColors {
    fn default() -> Self {
        Self::new()
    }
}

impl Prompt {
    pub const fn new() -> Self {
        Self::from(String::new())
    }

    pub const fn from(s: String) -> Self {
        Self {
            input: s,
            placeholder: "",
            cursor: 0,
            selector: None,
            scroll: 0,
            margins: 0,
            disabled: false,
            colors: PromptColors::new(),
            area_pos: Pos::ZERO,
            last_width: 0,
        }
    }

    pub const fn with_placeholder(mut self, s: &'static str) -> Self {
        self.placeholder = s;
        self
    }

    pub const fn with_colors(mut self, colors: PromptColors) -> Self {
        self.colors = colors;
        self
    }

    pub const fn with_margins(mut self, horizontal: u16) -> Self {
        self.margins = horizontal;
        self
    }

    pub const fn with_disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    pub const fn set_colors(&mut self, colors: PromptColors) -> &mut Self {
        self.colors = colors;
        self
    }

    pub const fn set_disabled(&mut self, value: bool) -> &mut Self {
        self.disabled = value;
        self
    }

    pub const fn is_empty(&self) -> bool {
        self.input.is_empty()
    }

    pub fn is_empty_trim(&self) -> bool {
        self.input.as_str().trim().is_empty()
    }

    pub const fn has_selection(&self) -> bool {
        self.selector.is_some()
    }

    pub const fn as_str(&self) -> &str {
        self.input.as_str()
    }

    pub fn as_str_trim(&self) -> &str {
        self.input.as_str().trim()
    }

    pub fn hash(&self) -> u64 {
        utils::hash_fast(self.input.as_str())
    }

    pub fn hash_trim(&self) -> u64 {
        utils::hash_fast(self.input.as_str().trim())
    }

    pub fn get_cursor_pos(&self) -> Pos {
        self.cursor_render_pos()
    }

    pub const fn get_render_pos(&self) -> Pos {
        self.area_pos
    }

    pub fn input(&mut self, key_pressed: KeyCode, key_modifiers: KeyModifiers) -> bool {
        if self.disabled {
            return false;
        }

        let ctrl = key_modifiers.contains(KeyModifiers::CONTROL);
        let shift = key_modifiers.contains(KeyModifiers::SHIFT);

        match key_pressed {
            KeyCode::Right => self.move_forward(shift),
            KeyCode::Left => self.move_backward(shift),
            KeyCode::Home => self.move_to_start(shift),
            KeyCode::End => self.move_to_end(shift),
            KeyCode::Backspace => self.delete_backward(),
            KeyCode::Delete => self.delete_forward(),
            KeyCode::Char('a') => {
                if ctrl {
                    self.select_all()
                } else {
                    self.push_char('a');
                    true
                }
            }
            KeyCode::Char(c) => {
                self.push_char(c);
                true
            }
            _ => false,
        }
    }

    pub fn push_char(&mut self, c: char) {
        self.delete_selection();
        self.input.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub fn push_str(&mut self, s: &str) {
        self.delete_selection();
        self.input.insert_str(self.cursor, s);
        self.cursor += s.len();
    }

    pub fn move_forward(&mut self, shift: bool) -> bool {
        self.move_cursor(CursorMove::Forward, shift)
    }

    pub fn move_backward(&mut self, shift: bool) -> bool {
        self.move_cursor(CursorMove::Back, shift)
    }

    pub fn move_to_end(&mut self, shift: bool) -> bool {
        self.move_cursor(CursorMove::End, shift)
    }

    pub fn move_to_start(&mut self, shift: bool) -> bool {
        self.move_cursor(CursorMove::Start, shift)
    }

    pub fn select_all(&mut self) -> bool {
        let (old_cursor, old_selector) = (self.cursor, self.selector);

        self.cursor = self.input.len();
        self.selector = Some(0);

        self.cursor != old_cursor || self.selector != old_selector
    }

    pub fn delete_forward(&mut self) -> bool {
        if let Some(selector) = self.selector.take() {
            return self.try_delete_selection(selector);
        }

        if let Some(g) = utils::graphemes(&self.input[self.cursor..]).next() {
            let range = self.cursor..self.cursor + g.len();
            self.input.replace_range(range, "");
            return true;
        }

        false
    }

    pub fn delete_backward(&mut self) -> bool {
        if let Some(selector) = self.selector.take() {
            return self.try_delete_selection(selector);
        }

        if let Some(g) = utils::graphemes(&self.input[..self.cursor]).next_back() {
            self.cursor -= g.len();
            let range = self.cursor..self.cursor + g.len();
            self.input.replace_range(range, "");
            return true;
        }

        false
    }

    pub fn delete_selection(&mut self) -> bool {
        match self.selector.take() {
            Some(selector) => self.try_delete_selection(selector),
            None => false,
        }
    }

    pub fn clear(&mut self) {
        self.input.clear();
        self.cursor = 0;
        self.selector = None;
        self.scroll = 0;
    }

    pub fn render(&mut self, area: Rect, frame: &mut Framebuffer) {
        self.area_pos = area.pos;

        if area.is_empty() {
            return;
        }

        let last_width = self.last_width;

        // Disabled
        if self.disabled {
            if self.input.is_empty() {
                self.scroll = 0;
                self.render_placeholder(area, frame, self.colors.disabled);
            } else {
                self.update_scroll(last_width, area.size.cols, self.input_width());
                self.render_text(area, frame, Some(self.colors.disabled));
            }
            return;
        }

        // Placeholder
        if self.input.is_empty() {
            self.scroll = 0;
            self.render_placeholder(area, frame, self.colors.placeholder);
            frame.cursor_move(area.pos);
            return;
        }

        // Update state
        self.update_scroll(last_width, area.size.cols, self.input_width());
        self.last_width = area.size.cols;

        // Render
        match self.try_selection() {
            Some(range) => self.render_text_with_selection(area, frame, range),
            None => self.render_text(area, frame, None),
        }
        frame.cursor_move(self.cursor_render_pos());
    }

    fn render_placeholder(&self, area: Rect, frame: &mut Framebuffer, color: Color) {
        frame.push_fmt(Sgr::Fg(color));
        frame.push_str(self.placeholder);
        frame.push_fmt(Sgr::Fg(Color::Default));
        frame.render(area, TextOptions::span());
    }

    fn render_text(&self, area: Rect, frame: &mut Framebuffer, color: Option<Color>) {
        if let Some(color) = color {
            frame.print_fmt(Sgr::Fg(color));
        }

        frame.cursor_move(area.pos);
        for (_, g) in self.render_iter(area.size.cols) {
            frame.print_str(g);
        }

        if color.is_some() {
            frame.print_fmt(Sgr::Fg(Color::Default));
        }
    }

    fn render_text_with_selection(
        &self,
        area: Rect,
        frame: &mut Framebuffer,
        selector: Range<usize>,
    ) {
        let mut found_selector = false;
        let mut has_reset_selector = false;

        frame.cursor_move(area.pos);
        for (i, g) in self.render_iter(area.size.cols) {
            if i > selector.start && !found_selector {
                frame.print_fmt(Sgr::Reverse);
                found_selector = true
            } else if i > selector.end && found_selector {
                frame.print_fmt(Sgr::NotReverse);
                has_reset_selector = true;
            }

            frame.print_str(g);
        }

        if !has_reset_selector && found_selector {
            frame.print_fmt(Sgr::NotReverse);
        }
    }

    fn render_iter(&self, max_width: u16) -> impl Iterator<Item = (usize, &str)> {
        let mut width = 0;
        let mut index = self.start_index();

        utils::graphemes(&self.input[index..])
            .take_while(move |g| {
                let w = grapheme_width(g);
                let reached_max_width = width + w > max_width;
                width += w;

                !reached_max_width
            })
            .map(move |g| {
                index += g.len();
                (index, grapheme_render(g))
            })
    }

    fn cursor_render_pos(&self) -> Pos {
        let mut col = self.area_pos.col + self.cursor_col();
        col = col.saturating_sub(self.scroll);
        self.area_pos.with_col(col)
    }

    const fn update_scroll(&mut self, last_width: u16, width: u16, input_width: u16) {
        let scroll = if last_width != width {
            // Refresh scroll on window resize
            0
        } else {
            self.scroll
        };
        self.scroll = Scroll::calc(ScrollData {
            current_index: self.cursor,
            current_scroll: scroll as usize,
            total_lines: input_width as usize + 1,
            viewport_height: width,
            margins: ScrollMargins::ZERO,
        }) as u16;
    }

    fn move_cursor(&mut self, cm: CursorMove, shift: bool) -> bool {
        let (old_cursor, old_selector) = (self.cursor, self.selector);

        if shift {
            if self.selector.is_none() {
                self.selector = Some(self.cursor);
            }
        } else {
            self.selector = None;
        }

        match cm {
            CursorMove::Forward => {
                if let Some(g) = utils::graphemes(&self.input[self.cursor..]).next() {
                    self.cursor += g.len();
                }
            }
            CursorMove::Back => {
                if let Some(g) = utils::graphemes(&self.input[..self.cursor]).next_back() {
                    self.cursor -= g.len();
                }
            }
            CursorMove::Start => {
                self.cursor = 0;
            }
            CursorMove::End => {
                self.cursor = self.input.len();
            }
        }

        self.selector.take_if(|s| *s == self.cursor);

        self.cursor != old_cursor || self.selector != old_selector
    }

    fn selection_range(&self, selector: usize) -> Option<Range<usize>> {
        use std::cmp::Ordering;

        match self.cursor.cmp(&selector) {
            Ordering::Less => Some(self.cursor..selector),
            Ordering::Greater => Some(selector..self.cursor),
            Ordering::Equal => None,
        }
    }

    fn try_selection(&self) -> Option<Range<usize>> {
        self.selector
            .and_then(|selector| self.selection_range(selector))
    }

    fn try_delete_selection(&mut self, selector: usize) -> bool {
        let Some(range) = self.selection_range(selector) else {
            return false;
        };
        self.cursor = range.start;
        self.input.replace_range(range, "");
        true
    }

    fn input_width(&self) -> u16 {
        text_width(&self.input)
    }

    fn cursor_col(&self) -> u16 {
        text_width(&self.input[..self.cursor])
    }

    fn start_index(&self) -> usize {
        let (mut width, mut start) = (0, 0);
        for (i, g) in utils::grapheme_indices(&self.input) {
            if width >= self.scroll {
                return i;
            }

            width += grapheme_width(g);
            start = i;
        }

        start
    }
}

enum CursorMove {
    Forward,
    Back,
    Start,
    End,
}

fn text_width(s: &str) -> u16 {
    utils::graphemes(s).map(grapheme_width).sum()
}

fn grapheme_width(g: &str) -> u16 {
    if g.chars().all(char::is_whitespace) {
        1
    } else {
        utils::str_width(g)
    }
}

fn grapheme_render(g: &str) -> &str {
    if g.chars().all(char::is_whitespace) {
        " "
    } else {
        g
    }
}
