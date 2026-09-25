use terminal::*;

#[derive(Debug, Clone, Copy)]
pub struct Scrollbar {
    colors: ScrollbarColors,
}

#[derive(Debug, Clone, Copy)]
pub struct ScrollbarColors {
    pub thumb: Color,
    pub track: Option<Color>,
}

impl ScrollbarColors {
    pub const DEFAULT: Self = Self {
        thumb: Color::Indexed(240),
        track: None,
    };

    pub const fn new(thumb: Color, track: Option<Color>) -> Self {
        Self { thumb, track }
    }
}

impl Default for ScrollbarColors {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ScrollbarData {
    pub current_scroll: usize,
    pub total_items: usize,
    pub viewport_height: u16,
}

impl Scrollbar {
    pub const fn new() -> Self {
        Self {
            colors: ScrollbarColors::DEFAULT,
        }
    }

    pub const fn colors(colors: ScrollbarColors) -> Self {
        Self { colors }
    }

    pub const fn with_colors(mut self, colors: ScrollbarColors) -> Self {
        self.colors = colors;
        self
    }

    pub fn render(self, area: Rect, frame: &mut Framebuffer, data: ScrollbarData) {
        if data.total_items == 0 || area.is_empty() {
            return;
        }

        let ScrollbarData {
            viewport_height,
            current_scroll,
            total_items,
        } = data;

        let visible = viewport_height as f32 / total_items as f32;
        let size = ((visible * area.size.rows as f32).floor() as u16).max(1);
        let progress = (current_scroll as f32
            / total_items.saturating_sub(viewport_height as usize) as f32)
            .min(1.0);
        let range = area.size.rows.saturating_sub(size);
        let start = (progress * range as f32).floor() as u16;
        let end = start + size;

        // TODO: Rework rendering.
        let mut pos = area.pos;
        match self.colors.track {
            // Render both track and thumb
            Some(track_color) => {
                for i in 0..area.size.rows {
                    let is_thumb = i >= start && i < end;

                    let (ch, color) = if is_thumb {
                        ('┃', self.colors.thumb)
                    } else {
                        ('│', track_color)
                    };

                    frame.cursor_move(pos);
                    frame.print_fmt(Sgr::Fg(color));
                    frame.print_ch(ch);

                    pos.row += 1;
                }

                frame.print_fmt(Sgr::Fg(Color::Default));
            }
            // Render only thumb
            None => {
                frame.print_fmt(Sgr::Fg(self.colors.thumb));

                for i in 0..area.size.rows {
                    let is_thumb = i >= start && i < end;

                    if is_thumb {
                        frame.cursor_move(pos);
                        frame.print_ch('│');
                    }

                    pos.row += 1;
                }

                frame.print_fmt(Sgr::Fg(Color::Default));
            }
        }
    }
}

impl Default for Scrollbar {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Scroll;

impl Scroll {
    pub const fn is_scrollable(lines: usize, viewport: Size, min_width: u16) -> bool {
        lines > viewport.rows as usize && viewport.cols > min_width
    }

    /// Calculates the scroll offset for provided data.
    /// Assumes the same height of one for each line.
    pub const fn calc(data: ScrollData) -> usize {
        let ScrollData {
            current_index,
            current_scroll,
            total_lines,
            viewport_height,
            margins:
                ScrollMargins {
                    margin_top,
                    margin_bottom,
                    padding_bottom,
                },
        } = data;

        const fn min(a: usize, b: usize) -> usize {
            if a < b { a } else { b }
        }

        let height = viewport_height as usize;
        let max_offset = (total_lines + padding_bottom as usize).saturating_sub(height);

        let available = height.saturating_sub(1);
        let margin_top = min(margin_top as usize, available);
        let margin_bottom = min(margin_bottom as usize, available - margin_top);

        let top_boundary = current_scroll + margin_top;
        let bottom_boundary = current_scroll + height.saturating_sub(margin_bottom + 1);

        if current_index < top_boundary {
            // Scroll up
            current_scroll.saturating_sub(top_boundary - current_index)
        } else if current_index > bottom_boundary {
            // Scroll down
            let delta = current_index - bottom_boundary;
            min(current_scroll + delta, max_offset)
        } else {
            // No scroll
            current_scroll
        }
    }

    pub const fn make_scroll_area(area: &mut Rect, margin: u16) -> Rect {
        let scroll_area = Rect {
            pos: Pos {
                col: area.pos.col + area.size.cols.saturating_sub(1),
                row: area.pos.row,
            },
            size: Size {
                cols: 1,
                rows: area.size.rows,
            },
        };
        area.size.cols = area.size.cols.saturating_sub(1 + margin);
        scroll_area
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ScrollData {
    pub current_index: usize,
    pub current_scroll: usize,
    pub total_lines: usize,
    pub viewport_height: u16,
    pub margins: ScrollMargins,
}

#[derive(Debug, Clone, Copy)]
pub struct ScrollMargins {
    pub margin_top: u16,
    pub margin_bottom: u16,
    pub padding_bottom: u16,
}

impl ScrollMargins {
    pub const ZERO: Self = Self::all(0);

    pub const fn new(top: u16, bottom: u16, padding_bottom: u16) -> Self {
        Self {
            margin_top: top,
            margin_bottom: bottom,
            padding_bottom,
        }
    }

    pub const fn all(v: u16) -> Self {
        Self::new(v, v, v)
    }

    pub const fn vertical(v: u16) -> Self {
        Self::new(v, v, 0)
    }

    pub const fn with_padding(mut self, v: u16) -> Self {
        self.padding_bottom = v;
        self
    }
}

impl Default for ScrollMargins {
    fn default() -> Self {
        Self::all(0)
    }
}
