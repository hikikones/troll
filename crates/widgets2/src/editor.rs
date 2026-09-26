use std::ops::Range;

use terminal::*;
use utils::StringExt;

use crate::{Scroll, ScrollData, ScrollMargins};

// TODO: Add scrollbar and padding.
// TODO: Use ansi codes for syntax highlighting.

pub struct Editor {
    input: String,
    cursor: usize,
    selector: Option<usize>,
    placeholder: &'static str,
    wrapped: String,
    lines: Vec<VisualLine>,
    preferred_column: u16,
    scroll: u16,
    margins: u16,
    disabled: bool,
    colors: EditorColors,
    area_pos: Pos,
    last_size: Size,
    last_hash: u64,
}

pub struct EditorColors {
    pub placeholder: Color,
    pub disabled: Color,
}

impl EditorColors {
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

impl Default for EditorColors {
    fn default() -> Self {
        Self::new()
    }
}

impl Editor {
    pub const fn new() -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            selector: None,
            placeholder: "",
            wrapped: String::new(),
            lines: Vec::new(),
            preferred_column: 0,
            scroll: 0,
            margins: 0,
            disabled: false,
            colors: EditorColors::new(),
            area_pos: Pos::ZERO,
            last_size: Size::ZERO,
            last_hash: 0,
        }
    }

    pub const fn with_placeholder(mut self, s: &'static str) -> Self {
        self.placeholder = s;
        self
    }

    pub const fn with_colors(mut self, colors: EditorColors) -> Self {
        self.colors = colors;
        self
    }

    pub const fn with_margins(mut self, vertical: u16) -> Self {
        self.margins = vertical;
        self
    }

    pub const fn with_disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    pub const fn set_colors(&mut self, colors: EditorColors) -> &mut Self {
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

    pub const fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub const fn has_selection(&self) -> bool {
        self.selector.is_some()
    }

    pub const fn as_str(&self) -> &str {
        self.input.as_str()
    }

    pub const fn get_scroll(&self) -> u16 {
        self.scroll
    }

    pub fn get_cursor_pos(&self) -> Pos {
        self.cursor_render_pos()
    }

    pub const fn get_render_pos(&self) -> Pos {
        self.area_pos
    }

    pub fn is_cursor_on_first_row(&self) -> bool {
        self.scroll == 0 && self.area_pos.row == self.get_cursor_pos().row
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
            KeyCode::Up => self.move_up(shift),
            KeyCode::Down => self.move_down(shift),
            KeyCode::Home => self.move_to_start(shift),
            KeyCode::End => self.move_to_end(shift),
            KeyCode::Backspace => self.delete_backward(),
            KeyCode::Delete => self.delete_forward(),
            KeyCode::Enter => {
                self.push_newline();
                true
            }
            KeyCode::Char(c) => match c {
                'a' => {
                    if ctrl {
                        return self.select_all();
                    }

                    self.push_char(c);
                    true
                }
                _ => {
                    self.push_char(c);
                    true
                }
            },
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

    pub fn push_newline(&mut self) {
        #[cfg(target_os = "windows")]
        self.push_str("\r\n");
        #[cfg(not(target_os = "windows"))]
        self.push_char('\n');
    }

    pub fn move_forward(&mut self, shift: bool) -> bool {
        self.move_cursor(CursorMove::Forward, shift)
    }

    pub fn move_backward(&mut self, shift: bool) -> bool {
        self.move_cursor(CursorMove::Back, shift)
    }

    pub fn move_up(&mut self, shift: bool) -> bool {
        self.move_cursor(CursorMove::Up, shift)
    }

    pub fn move_down(&mut self, shift: bool) -> bool {
        self.move_cursor(CursorMove::Down, shift)
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
        self.preferred_column = self.cursor_pos().col;

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

    pub fn render(&mut self, area: Rect, frame: &mut Framebuffer) {
        self.area_pos = area.pos;

        if area.is_empty() {
            return;
        }

        let last_size = self.last_size;

        // Disabled
        if self.disabled {
            if self.input.is_empty() {
                self.scroll = 0;
                self.render_placeholder(area, frame, self.colors.disabled);
            } else {
                self.process_input(last_size, area.size);
                self.update_scroll(last_size.rows, area.size.rows);
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
        self.process_input(last_size, area.size);
        self.update_scroll(last_size.rows, area.size.rows);

        // Render
        match self.try_selection() {
            Some(range) => self.render_text_with_selection(area, frame, range),
            None => self.render_text(area, frame, None),
        }
        frame.cursor_move(self.cursor_render_pos());
    }

    pub fn clear(&mut self) {
        self.input.clear();
        self.cursor = 0;
        self.selector = None;
        self.wrapped.clear();
        self.lines.clear();
        self.preferred_column = 0;
        self.scroll = 0;
        self.last_size = Size::ZERO;
        self.last_hash = 0;
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

        for (i, line) in self
            .lines
            .iter()
            .skip(self.scroll as usize)
            .take(area.size.rows as usize)
            .enumerate()
        {
            frame.cursor_move(Pos::new(area.pos.col, area.pos.row + i as u16));
            for g in utils::graphemes(self.wrapped.slice(line.range())) {
                frame.print_str(grapheme_render(g));
            }
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

        for (i, line) in self
            .lines
            .iter()
            .skip(self.scroll as usize)
            .take(area.size.rows as usize)
            .enumerate()
        {
            frame.cursor_move(Pos::new(area.pos.col, area.pos.row + i as u16));

            let mut index = line.start;

            for g in utils::graphemes(self.wrapped.slice(line.range())) {
                if index >= selector.start && !found_selector {
                    frame.print_fmt(Sgr::Reverse);
                    found_selector = true
                } else if index >= selector.end && found_selector {
                    frame.print_fmt(Sgr::NotReverse);
                    has_reset_selector = true;
                }

                index += g.len();

                frame.print_str(grapheme_render(g));
            }
        }

        if !has_reset_selector && found_selector {
            frame.print_fmt(Sgr::NotReverse);
        }
    }

    fn cursor_render_pos(&self) -> Pos {
        let mut cpos = self.area_pos + self.cursor_pos();
        cpos.row = cpos.row.saturating_sub(self.scroll);
        cpos
    }

    fn update_scroll(&mut self, last_height: u16, height: u16) {
        let scroll = if last_height != height {
            // Refresh scroll on window resize
            0
        } else {
            self.scroll
        };
        self.scroll = Scroll::calc(ScrollData {
            current_index: self.index_to_row(self.cursor) as usize,
            current_scroll: scroll as usize,
            total_lines: self.lines.len(),
            viewport_height: height,
            margins: ScrollMargins::ZERO,
        }) as u16;
    }

    fn process_input(&mut self, last_size: Size, size: Size) {
        let hash = utils::hash_fast(self.input.as_str());
        if last_size.cols != size.cols || self.last_hash != hash {
            self.last_size = size;
            self.last_hash = hash;
            self.relayout(size.cols);
        }
    }

    fn relayout(&mut self, max_width: u16) {
        self.wrapped.clear();
        self.lines.clear();

        self.wrapped.push_str(self.input.as_str());
        utils::text_wrap(&mut self.wrapped, max_width);

        let mut start = 0;
        let mut column = 0;

        for (i, g) in utils::grapheme_indices(self.wrapped.as_str()) {
            if g.contains('\n') {
                self.lines.push(VisualLine::new(start..i + g.len()));

                start = i + g.len();
                column = 0;
                continue;
            }

            let width = grapheme_width(g);
            if column + width > max_width {
                self.lines.push(VisualLine::new(start..i));

                start = i;
                column = width;
            } else {
                column += width;
            }
        }

        self.lines.push(VisualLine::new(start..self.wrapped.len()));
        self.preferred_column = self.cursor_pos().col;
    }

    fn move_cursor(&mut self, cm: CursorMove, shift: bool) -> bool {
        if self.input.is_empty() {
            return false;
        }

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
                    self.preferred_column = self.cursor_pos().col;
                }
            }
            CursorMove::Back => {
                if let Some(g) = utils::graphemes(&self.input[..self.cursor]).next_back() {
                    self.cursor -= g.len();
                    self.preferred_column = self.cursor_pos().col;
                }
            }
            CursorMove::Up => {
                let row = self.cursor_row();
                if row > 0 {
                    self.cursor = self.col_to_index(row - 1, self.preferred_column);
                }
            }
            CursorMove::Down => {
                let row = self.cursor_row();
                if row + 1 < self.lines.len() as u16 {
                    self.cursor = self.col_to_index(row + 1, self.preferred_column);
                }
            }
            CursorMove::Start => {
                self.cursor = 0;
                self.preferred_column = 0;
            }
            CursorMove::End => {
                self.cursor = self.input.len();
                self.preferred_column = self.cursor_pos().col;
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

    fn cursor_row(&self) -> u16 {
        self.index_to_row(self.cursor)
    }

    fn cursor_pos(&self) -> Pos {
        self.index_to_pos(self.cursor)
    }

    fn index_to_row(&self, index: usize) -> u16 {
        self.lines
            .partition_point(|line| line.end <= index)
            .min(self.lines.len().saturating_sub(1)) as u16
    }

    fn index_to_pos(&self, index: usize) -> Pos {
        if self.lines.is_empty() {
            return Pos::ZERO;
        };

        let row = self.index_to_row(index);
        let line = &self.lines[row as usize];
        let text = self.wrapped.slice(line.range());
        let col = text_width(&text[..index - line.start]);
        Pos { col, row }
    }

    fn col_to_index(&self, row: u16, target: u16) -> usize {
        let line = &self.lines[row as usize];

        let mut column = 0;
        let mut index = line.start;

        for (i, g) in utils::grapheme_indices(self.wrapped.slice(line.range())) {
            if g.contains('\n') {
                break;
            }

            let width = grapheme_width(g);
            if column + width > target {
                break;
            }

            column += width;
            index = line.start + i + g.len();
        }

        index
    }
}

enum CursorMove {
    Forward,
    Back,
    Up,
    Down,
    Start,
    End,
}

struct VisualLine {
    start: usize,
    end: usize,
}

impl VisualLine {
    fn new(range: Range<usize>) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }

    const fn range(&self) -> Range<usize> {
        self.start..self.end
    }
}

fn text_width(s: &str) -> u16 {
    utils::graphemes(s).map(grapheme_width).sum()
}

fn grapheme_width(g: &str) -> u16 {
    match g {
        "\t" => 4,
        _ => utils::str_width(g),
    }
}

fn grapheme_render(g: &str) -> &str {
    match g {
        "\t" => "    ",
        "\r\n" | "\n" => " ",
        _ => g,
    }
}
