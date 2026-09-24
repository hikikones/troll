use terminal::*;

#[derive(Debug, Clone, Copy)]
pub struct Block {
    shape: Shape,
    color: Option<Color>,
}

#[derive(Debug, Clone, Copy)]
pub enum Shape {
    /// ```text
    /// ─────
    /// ```
    LineHorizontal,
    /// ```text
    /// │
    /// │
    /// │
    /// ```
    LineVertical,
    /// ```text
    /// ─────
    ///
    /// ─────
    /// ```
    Horizontals,
    /// ```text
    /// │   │
    /// │   │
    /// │   │
    /// ```
    Verticals,
    /// ```text
    /// ┌───┐
    /// │   │
    /// └───┘
    /// ```
    Rectangle,
    // /// ```text
    // /// ┌──────
    // /// │
    // /// │         │
    // ///           │
    // ///     ──────┘
    // /// ```
    // RectangleFancy,
    /// ```text
    /// ┌   ┐
    ///
    /// └   ┘
    /// ```
    Corners,
    /// ```text
    /// █████
    /// █████
    /// █████
    /// ```
    Fill,
}

impl Block {
    pub const fn new(shape: Shape) -> Self {
        Self { shape, color: None }
    }

    pub const fn rectangle() -> Self {
        Self::new(Shape::Rectangle)
    }

    pub const fn fill(color: Color) -> Self {
        Self::new(Shape::Fill).with_color(color)
    }

    pub const fn clear() -> Self {
        Self::new(Shape::Fill)
    }

    pub const fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn render(self, area: Rect, frame: &mut Framebuffer) {
        match self.shape {
            Shape::LineHorizontal => {
                if area.is_empty() {
                    return;
                }

                if let Some(color) = self.color {
                    frame.print_fmt(SetForegroundColor(color));
                    draw_line_horizontal(area.pos, area.size.cols, frame);
                    frame.print_fmt(SetForegroundColor(Color::Reset));
                } else {
                    draw_line_horizontal(area.pos, area.size.cols, frame);
                }
            }
            Shape::LineVertical => {
                if area.is_empty() {
                    return;
                }

                if let Some(color) = self.color {
                    frame.print_fmt(SetForegroundColor(color));
                    draw_line_vertical(area.pos, area.size.rows, frame);
                    frame.print_fmt(SetForegroundColor(Color::Reset));
                } else {
                    draw_line_vertical(area.pos, area.size.rows, frame);
                }
            }
            Shape::Horizontals => {
                if area.size.cols == 0 {
                    return;
                }

                match area.size.rows {
                    0 => return,
                    1 => {
                        if let Some(color) = self.color {
                            frame.print_fmt(SetForegroundColor(color));
                            draw_line_horizontal(area.pos, area.size.cols, frame);
                            frame.print_fmt(SetForegroundColor(Color::Reset));
                        } else {
                            draw_line_horizontal(area.pos, area.size.cols, frame);
                        }
                    }
                    _ => {
                        if let Some(color) = self.color {
                            frame.print_fmt(SetForegroundColor(color));
                            draw_horizontals(area, frame);
                            frame.print_fmt(SetForegroundColor(Color::Reset));
                        } else {
                            draw_horizontals(area, frame);
                        }
                    }
                }
            }
            Shape::Verticals => {
                if area.size.rows == 0 {
                    return;
                }

                match area.size.cols {
                    0 => return,
                    1 => {
                        if let Some(color) = self.color {
                            frame.print_fmt(SetForegroundColor(color));
                            draw_line_vertical(area.pos, area.size.rows, frame);
                            frame.print_fmt(SetForegroundColor(Color::Reset));
                        } else {
                            draw_line_vertical(area.pos, area.size.rows, frame);
                        }
                    }
                    _ => {
                        if let Some(color) = self.color {
                            frame.print_fmt(SetForegroundColor(color));
                            draw_verticals(area, frame);
                            frame.print_fmt(SetForegroundColor(Color::Reset));
                        } else {
                            draw_verticals(area, frame);
                        }
                    }
                }
            }
            Shape::Rectangle => {
                if area.size.is_less(2) {
                    return;
                }

                if let Some(color) = self.color {
                    frame.print_fmt(SetForegroundColor(color));
                    draw_rectangle(area, frame);
                    frame.print_fmt(SetForegroundColor(Color::Reset));
                } else {
                    draw_rectangle(area, frame);
                }
            }
            Shape::Corners => {
                if area.size.is_less(2) {
                    return;
                }

                if let Some(color) = self.color {
                    frame.print_fmt(SetForegroundColor(color));
                    draw_corners(area, frame);
                    frame.print_fmt(SetForegroundColor(Color::Reset));
                } else {
                    draw_corners(area, frame);
                }
            }
            Shape::Fill => {
                if area.is_empty() {
                    return;
                }

                fill(area, frame, self.color.unwrap_or(Color::Reset));
            }
        }
    }
}

fn draw_line_horizontal(pos: Pos, width: u16, frame: &mut Framebuffer) {
    frame.cursor_move(pos);
    frame.print_ch_repeat('─', width);
}

fn draw_line_vertical(mut pos: Pos, height: u16, frame: &mut Framebuffer) {
    for _ in 0..height {
        frame.cursor_move(pos);
        frame.print_ch('│');
        pos.row += 1;
    }
}

fn draw_horizontals(area: Rect, frame: &mut Framebuffer) {
    let Size { cols, rows } = area.size;
    let bottom = area.pos.with_row(area.pos.row + rows - 1);
    draw_line_horizontal(area.pos, cols, frame);
    draw_line_horizontal(bottom, cols, frame);
}

fn draw_verticals(area: Rect, frame: &mut Framebuffer) {
    let Size { cols, rows } = area.size;
    let right = area.pos.with_col(area.pos.col + cols - 1);
    draw_line_vertical(area.pos, rows, frame);
    draw_line_vertical(right, rows, frame);
}

fn draw_rectangle(area: Rect, frame: &mut Framebuffer) {
    let mut pos = area.pos;
    let Size { cols, rows } = area.size;
    let horizontals = cols - 2;

    // Top line
    frame.cursor_move(area.pos);
    frame.print_ch('┌');
    frame.print_ch_repeat('─', horizontals);
    frame.print_ch('┐');

    pos.row += 1;

    // Middle lines
    for _ in 0..rows - 2 {
        frame.cursor_move(pos);

        frame.print_ch('│');
        pos.col += cols - 1;
        frame.cursor_move(pos);
        frame.print_ch('│');

        pos.col = area.pos.col;
        pos.row += 1;
    }

    // Bottom line
    frame.cursor_move(pos);
    frame.print_ch('└');
    frame.print_ch_repeat('─', horizontals);
    frame.print_ch('┘');
}

fn draw_corners(area: Rect, frame: &mut Framebuffer) {
    let right = area.pos.col + area.size.cols - 1;
    let bottom = area.pos.row + area.size.rows - 1;

    frame.cursor_move(area.pos);
    frame.print_ch('┌');

    frame.cursor_move(area.pos.with_col(right));
    frame.print_ch('┐');

    frame.cursor_move(area.pos.with_row(bottom));
    frame.print_ch('└');

    frame.cursor_move(Pos::new(right, bottom));
    frame.print_ch('┘');
}

fn fill(area: Rect, frame: &mut Framebuffer, bg: Color) {
    frame.print_fmt(SetBackgroundColor(bg));

    for i in 0..area.size.rows {
        frame.cursor_move(area.pos.with_row(area.pos.row + i));
        frame.print_ch_repeat(' ', area.size.cols);
    }

    frame.print_fmt(SetBackgroundColor(Color::Reset));
}
