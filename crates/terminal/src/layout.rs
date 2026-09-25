#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub pos: Pos,
    pub size: Size,
}

impl Rect {
    pub const fn new(pos: Pos, size: Size) -> Self {
        Self { pos, size }
    }

    pub const fn with_pos(mut self, pos: Pos) -> Self {
        self.pos = pos;
        self
    }

    pub const fn with_col(mut self, col: u16) -> Self {
        self.pos.col = col;
        self
    }

    pub const fn with_row(mut self, row: u16) -> Self {
        self.pos.row = row;
        self
    }

    pub const fn with_size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub const fn with_cols(mut self, cols: u16) -> Self {
        self.size.cols = cols;
        self
    }

    pub const fn with_rows(mut self, rows: u16) -> Self {
        self.size.rows = rows;
        self
    }

    pub const fn is_empty(&self) -> bool {
        self.size.is_empty()
    }

    pub const fn col(&self) -> u16 {
        self.pos.col
    }

    pub const fn row(&self) -> u16 {
        self.pos.row
    }

    pub const fn cols(&self) -> u16 {
        self.size.cols
    }

    pub const fn rows(&self) -> u16 {
        self.size.rows
    }

    pub const fn set_pos(&mut self, p: Pos) {
        self.pos = p;
    }

    pub const fn set_col(&mut self, v: u16) {
        self.pos.col = v;
    }

    pub const fn set_row(&mut self, v: u16) {
        self.pos.row = v;
    }

    pub const fn set_cols(&mut self, v: u16) {
        self.size.cols = v;
    }

    pub const fn set_rows(&mut self, v: u16) {
        self.size.rows = v;
    }

    pub const fn set_size(&mut self, s: Size) {
        self.size = s;
    }

    pub const fn add_pos(&mut self, p: Pos) {
        self.pos.add(p);
    }

    pub const fn add_col(&mut self, v: u16) {
        self.pos.add_col(v);
    }

    pub const fn add_row(&mut self, v: u16) {
        self.pos.add_row(v);
    }

    pub const fn sub_pos(&mut self, p: Pos) {
        self.pos.sub(p);
    }

    pub const fn sub_col(&mut self, v: u16) {
        self.pos.sub_col(v);
    }

    pub const fn sub_row(&mut self, v: u16) {
        self.pos.sub_row(v);
    }

    pub const fn add_size(&mut self, s: Size) {
        self.size.add(s);
    }

    pub const fn add_cols(&mut self, v: u16) {
        self.size.add_cols(v);
    }

    pub const fn add_rows(&mut self, v: u16) {
        self.size.add_rows(v);
    }

    pub const fn sub_size(&mut self, s: Size) {
        self.size.sub(s);
    }

    pub const fn sub_cols(&mut self, v: u16) {
        self.size.sub_cols(v);
    }

    pub const fn sub_rows(&mut self, v: u16) {
        self.size.sub_rows(v);
    }

    pub const fn shrink_down(&mut self, rows: u16) {
        self.pos.row += rows;
        self.size.rows = self.size.rows.saturating_sub(rows);
    }

    pub const fn right_out(&self) -> u16 {
        self.pos.col + self.size.cols
    }

    pub const fn right_col(&self) -> u16 {
        self.right_out().saturating_sub(1)
    }

    pub const fn bottom_out(&self) -> u16 {
        self.pos.row + self.size.rows
    }

    pub const fn bottom_row(&self) -> u16 {
        self.bottom_out().saturating_sub(1)
    }

    pub const fn right_bottom(&self) -> Pos {
        Pos::new(self.right_col(), self.bottom_row())
    }

    pub const fn inner(self, margin: Margin) -> Self {
        Self {
            pos: Pos {
                col: self.pos.col + margin.left,
                row: self.pos.row + margin.top,
            },
            size: Size {
                cols: self.size.cols.saturating_sub(margin.left + margin.right),
                rows: self.size.rows.saturating_sub(margin.top + margin.bottom),
            },
        }
    }

    pub const fn align(
        mut self,
        outer: Self,
        horz: HorizontalAlignment,
        vert: VerticalAlignment,
    ) -> Self {
        self.pos.col = horz.calc(outer.pos.col, outer.size.cols, self.size.cols);
        self.pos.row = vert.calc(outer.pos.row, outer.size.rows, self.size.rows);
        self
    }

    pub const fn align_horz(mut self, outer: Self, alignment: HorizontalAlignment) -> Self {
        self.pos.col = alignment.calc(outer.pos.col, outer.size.cols, self.size.cols);
        self
    }

    pub const fn align_vert(mut self, outer: Self, alignment: VerticalAlignment) -> Self {
        self.pos.row = alignment.calc(outer.pos.row, outer.size.rows, self.size.rows);
        self
    }

    pub const fn center(self, outer: Self) -> Self {
        self.align(
            outer,
            HorizontalAlignment::Center,
            VerticalAlignment::Center,
        )
    }

    pub const fn center_horz(self, outer: Self) -> Self {
        self.align_horz(outer, HorizontalAlignment::Center)
    }

    pub const fn center_vert(self, outer: Self) -> Self {
        self.align_vert(outer, VerticalAlignment::Center)
    }

    pub const fn split_horizontally(self) -> (Self, Self) {
        let top_rows = self.size.rows / 2;
        (
            self.with_rows(top_rows),
            Self {
                pos: self.pos.with_row(self.pos.row + top_rows),
                size: self.size.with_rows(self.size.rows - top_rows),
            },
        )
    }

    pub const fn split_vertically(self) -> (Self, Self) {
        let left_cols = self.size.cols / 2;
        (
            self.with_cols(left_cols),
            Self {
                pos: self.pos.with_col(self.pos.col + left_cols),
                size: self.size.with_cols(self.size.cols - left_cols),
            },
        )
    }

    pub const fn split_horizontally_with_gap(self, extra_gaps: u16) -> (Self, Self) {
        let top_rows = (self.size.rows / 2).saturating_sub(extra_gaps);
        (
            self.with_rows(top_rows),
            Self {
                pos: self
                    .pos
                    .with_row(self.pos.row + top_rows + 1 + extra_gaps * 2),
                size: self.size.with_rows(top_rows),
            },
        )
    }

    pub const fn split_vertically_with_gap(self, extra_gaps: u16) -> (Self, Self) {
        let left_cols = (self.size.cols / 2).saturating_sub(extra_gaps);
        (
            self.with_cols(left_cols),
            Self {
                pos: self
                    .pos
                    .with_col(self.pos.col + left_cols + 1 + extra_gaps * 2),
                size: self.size.with_cols(left_cols),
            },
        )
    }

    pub const fn split_horizontal<const N: usize>(
        self,
        gap: u16,
        constraints: [Constraint; N],
    ) -> [Self; N] {
        let right_out = self.right_out();
        let mut pos = self.pos;

        let splits = split(self.size.cols, gap, constraints);
        let mut rects = [self; N];

        let mut i = 0;
        while i < N {
            let w = splits[i];
            let s = Size::new(w, self.size.rows);
            let r = Rect::new(pos, s);
            let next_col = pos.col + w + gap;
            pos.col = if next_col < right_out {
                next_col
            } else {
                right_out
            };
            rects[i] = r;
            i += 1;
        }

        rects
    }

    pub const fn split_vertical<const N: usize>(
        self,
        gap: u16,
        constraints: [Constraint; N],
    ) -> [Self; N] {
        let bottom_out = self.bottom_out();
        let mut pos = self.pos;

        let splits = split(self.size.rows, gap, constraints);
        let mut rects = [self; N];

        let mut i = 0;
        while i < N {
            let h = splits[i];
            let s = Size::new(self.size.cols, h);
            let r = Rect::new(pos, s);
            let next_row = pos.row + h + gap;
            pos.row = if next_row < bottom_out {
                next_row
            } else {
                bottom_out
            };
            rects[i] = r;
            i += 1;
        }

        rects
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Constraint {
    Fixed(u8),
    Percent(u8),
}

const fn split<const N: usize>(width: u16, gap: u16, constraints: [Constraint; N]) -> [u16; N] {
    let total_gap = gap * N.saturating_sub(1) as u16;
    let mut available = width.saturating_sub(total_gap);
    let fixed = {
        let mut i = 0;
        let mut sum = 0;
        while i < N {
            sum += match constraints[i] {
                Constraint::Fixed(w) => w as u16,
                Constraint::Percent(_) => 0,
            };
            i += 1;
        }
        sum
    };
    let remaining = available.saturating_sub(fixed);

    let mut widths = [0; N];

    let mut i = 0;
    while i < N {
        let cw = match constraints[i] {
            Constraint::Fixed(w) => w as u16,
            Constraint::Percent(p) => (remaining * p as u16) / 100,
        };
        let w = if cw < available { cw } else { available };
        available = available.saturating_sub(cw);
        widths[i] = w;
        i += 1;
    }

    if available > 0 {
        // TODO: How to apply/distribute available width? Should I even apply it?
    }

    widths
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub col: u16,
    pub row: u16,
}

impl Pos {
    pub const ZERO: Self = Self { col: 0, row: 0 };

    pub const fn new(col: u16, row: u16) -> Self {
        Self { col, row }
    }

    pub const fn with_col(mut self, col: u16) -> Self {
        self.col = col;
        self
    }

    pub const fn with_row(mut self, row: u16) -> Self {
        self.row = row;
        self
    }

    pub const fn add(self, rhs: Self) -> Self {
        Self {
            col: self.col + rhs.col,
            row: self.row + rhs.row,
        }
    }

    pub const fn sub(self, rhs: Self) -> Self {
        Self {
            col: self.col.saturating_sub(rhs.col),
            row: self.row.saturating_sub(rhs.row),
        }
    }

    pub const fn add_col(&mut self, v: u16) {
        self.col += v;
    }

    pub const fn add_row(&mut self, v: u16) {
        self.row += v;
    }

    pub const fn sub_col(&mut self, v: u16) {
        self.col = self.col.saturating_sub(v);
    }

    pub const fn sub_row(&mut self, v: u16) {
        self.row = self.row.saturating_sub(v);
    }
}

impl From<(u16, u16)> for Pos {
    fn from((col, row): (u16, u16)) -> Self {
        Self { col, row }
    }
}

impl std::ops::Add for Pos {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.add(rhs)
    }
}

impl std::ops::Add<u16> for Pos {
    type Output = Self;

    fn add(self, rhs: u16) -> Self::Output {
        Self {
            col: self.col + rhs,
            row: self.row + rhs,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub cols: u16,
    pub rows: u16,
}

impl Size {
    pub const ZERO: Self = Self { cols: 0, rows: 0 };

    pub const fn new(cols: u16, rows: u16) -> Self {
        Self { cols, rows }
    }

    pub const fn with_cols(mut self, cols: u16) -> Self {
        self.cols = cols;
        self
    }

    pub const fn with_rows(mut self, rows: u16) -> Self {
        self.rows = rows;
        self
    }

    pub const fn is_empty(&self) -> bool {
        self.cols == 0 || self.rows == 0
    }

    pub const fn is_less(&self, v: u16) -> bool {
        self.cols < v || self.rows < v
    }

    pub const fn eq(&self, other: Self) -> bool {
        self.cols == other.cols && self.rows == other.rows
    }

    pub const fn neq(&self, other: Self) -> bool {
        self.cols != other.cols || self.rows != other.rows
    }

    pub const fn add(self, rhs: Self) -> Self {
        Self {
            cols: self.cols + rhs.cols,
            rows: self.rows + rhs.rows,
        }
    }

    pub const fn sub(self, rhs: Self) -> Self {
        Self {
            cols: self.cols.saturating_sub(rhs.cols),
            rows: self.rows.saturating_sub(rhs.rows),
        }
    }

    pub const fn mul(self, rhs: Self) -> Self {
        Self {
            cols: self.cols * rhs.cols,
            rows: self.rows * rhs.rows,
        }
    }

    pub const fn mul_f32(self, cols: f32, rows: f32) -> Self {
        Self {
            cols: (self.cols as f32 * cols) as u16,
            rows: (self.rows as f32 * rows) as u16,
        }
    }

    pub const fn div(self, rhs: Self) -> Self {
        Self {
            cols: self.cols / rhs.cols,
            rows: self.rows / rhs.rows,
        }
    }

    pub const fn add_cols(&mut self, v: u16) {
        self.cols += v;
    }

    pub const fn add_rows(&mut self, v: u16) {
        self.rows += v;
    }

    pub const fn sub_cols(&mut self, v: u16) {
        self.cols = self.cols.saturating_sub(v);
    }

    pub const fn sub_rows(&mut self, v: u16) {
        self.rows = self.rows.saturating_sub(v);
    }

    pub const fn mul_cols(&mut self, v: u16) {
        self.cols *= v;
    }

    pub const fn mul_cols_f32(&mut self, v: f32) {
        self.cols = (self.cols as f32 * v) as u16
    }

    pub const fn mul_rows(&mut self, v: u16) {
        self.rows *= v;
    }

    pub const fn mul_rows_f32(&mut self, v: f32) {
        self.rows = (self.rows as f32 * v) as u16
    }

    pub const fn div_cols(&mut self, v: u16) {
        self.cols /= v;
    }

    pub const fn div_rows(&mut self, v: u16) {
        self.rows /= v;
    }
}

impl std::ops::Add for Size {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            cols: self.cols + rhs.cols,
            rows: self.rows + rhs.cols,
        }
    }
}

impl std::ops::Add<u16> for Size {
    type Output = Self;

    fn add(self, rhs: u16) -> Self::Output {
        Self {
            cols: self.cols + rhs,
            rows: self.rows + rhs,
        }
    }
}

impl std::ops::Add<(u16, u16)> for Size {
    type Output = Self;

    fn add(self, (cols, rows): (u16, u16)) -> Self::Output {
        Self {
            cols: self.cols + cols,
            rows: self.rows + rows,
        }
    }
}

impl std::ops::Div<u16> for Size {
    type Output = Self;

    fn div(self, rhs: u16) -> Self::Output {
        Self {
            cols: self.cols / rhs,
            rows: self.rows / rhs,
        }
    }
}

impl std::ops::Div<(u16, u16)> for Size {
    type Output = Self;

    fn div(self, (cols, rows): (u16, u16)) -> Self::Output {
        Self {
            cols: self.cols / cols,
            rows: self.rows / rows,
        }
    }
}

/// The pixel dimensions of a single cell in the terminal.
#[derive(Debug, Clone, Copy)]
pub struct CellDims {
    pub width: u16,
    pub height: u16,
}

impl CellDims {
    pub const fn width(&self, cols: u16) -> u16 {
        self.width * cols
    }

    pub const fn height(&self, rows: u16) -> u16 {
        self.height * rows
    }

    pub const fn cols(&self, width: u16) -> u16 {
        width.div_ceil(self.width)
    }

    pub const fn rows(&self, height: u16) -> u16 {
        height.div_ceil(self.height)
    }

    pub const fn size(&self, dims: ImageDims) -> Size {
        Size {
            cols: self.cols(dims.width),
            rows: self.rows(dims.height),
        }
    }

    pub const fn dims(&self, size: Size) -> ImageDims {
        ImageDims {
            width: self.width(size.cols),
            height: self.height(size.rows),
        }
    }
}

/// The pixel dimensions of an image.
#[derive(Debug, Clone, Copy)]
pub struct ImageDims {
    pub width: u16,
    pub height: u16,
}

impl ImageDims {
    pub const ZERO: Self = Self {
        width: 0,
        height: 0,
    };

    pub const fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }

    pub const fn with_width(mut self, width: u16) -> Self {
        self.width = width;
        self
    }

    pub const fn with_height(mut self, height: u16) -> Self {
        self.height = height;
        self
    }

    pub fn resize(self, max: ImageDims) -> Self {
        // https://docs.rs/image/0.25.10/src/image/math/utils.rs.html
        fn resize_dimensions(
            width: u32,
            height: u32,
            nwidth: u32,
            nheight: u32,
            fill: bool,
        ) -> (u32, u32) {
            use std::cmp::max;

            let wratio = f64::from(nwidth) / f64::from(width);
            let hratio = f64::from(nheight) / f64::from(height);

            let ratio = if fill {
                f64::max(wratio, hratio)
            } else {
                f64::min(wratio, hratio)
            };

            let nw = max((f64::from(width) * ratio).round() as u64, 1);
            let nh = max((f64::from(height) * ratio).round() as u64, 1);

            if nw > u64::from(u32::MAX) {
                let ratio = f64::from(u32::MAX) / f64::from(width);
                (u32::MAX, max((f64::from(height) * ratio).round() as u32, 1))
            } else if nh > u64::from(u32::MAX) {
                let ratio = f64::from(u32::MAX) / f64::from(height);
                (max((f64::from(width) * ratio).round() as u32, 1), u32::MAX)
            } else {
                (nw as u32, nh as u32)
            }
        }

        let (rw, rh) = resize_dimensions(
            self.width as u32,
            self.height as u32,
            max.width as u32,
            max.height as u32,
            false,
        );

        Self {
            width: rw as u16,
            height: rh as u16,
        }
    }
}

impl From<(u32, u32)> for ImageDims {
    fn from((w, h): (u32, u32)) -> Self {
        Self {
            width: w as u16,
            height: h as u16,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum HorizontalAlignment {
    Left,
    Center,
    Right,
}

impl HorizontalAlignment {
    pub const fn calc(self, outer_x: u16, outer_width: u16, inner_width: u16) -> u16 {
        match self {
            Self::Left => outer_x,
            Self::Center => outer_x + (outer_width.saturating_sub(inner_width)) / 2,
            Self::Right => outer_x + outer_width.saturating_sub(inner_width),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

impl VerticalAlignment {
    pub const fn calc(self, outer_y: u16, outer_height: u16, inner_height: u16) -> u16 {
        match self {
            Self::Top => outer_y,
            Self::Center => outer_y + (outer_height.saturating_sub(inner_height)) / 2,
            Self::Bottom => outer_y + outer_height.saturating_sub(inner_height),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Margin {
    pub left: u16,
    pub right: u16,
    pub top: u16,
    pub bottom: u16,
}

impl Margin {
    pub const ZERO: Self = Self {
        left: 0,
        right: 0,
        top: 0,
        bottom: 0,
    };

    pub const fn new(left: u16, right: u16, top: u16, bottom: u16) -> Self {
        Self {
            left,
            right,
            top,
            bottom,
        }
    }

    pub const fn all(value: u16) -> Self {
        Self {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }

    pub const fn horizontal(value: u16) -> Self {
        Self {
            left: value,
            right: value,
            top: 0,
            bottom: 0,
        }
    }

    pub const fn vertical(value: u16) -> Self {
        Self {
            left: 0,
            right: 0,
            top: value,
            bottom: value,
        }
    }

    pub const fn symmetric(horizontal: u16, vertical: u16) -> Self {
        Self {
            left: horizontal,
            right: horizontal,
            top: vertical,
            bottom: vertical,
        }
    }

    pub const fn proportional(value: u16) -> Self {
        Self::symmetric(value * 2, value)
    }

    pub const fn left(value: u16) -> Self {
        Self {
            left: value,
            right: 0,
            top: 0,
            bottom: 0,
        }
    }

    pub const fn right(value: u16) -> Self {
        Self {
            left: 0,
            right: value,
            top: 0,
            bottom: 0,
        }
    }

    pub const fn top(value: u16) -> Self {
        Self {
            left: 0,
            right: 0,
            top: value,
            bottom: 0,
        }
    }

    pub const fn bottom(value: u16) -> Self {
        Self {
            left: 0,
            right: 0,
            top: 0,
            bottom: value,
        }
    }

    pub const fn with_left(mut self, value: u16) -> Self {
        self.left = value;
        self
    }

    pub const fn with_right(mut self, value: u16) -> Self {
        self.right = value;
        self
    }

    pub const fn with_top(mut self, value: u16) -> Self {
        self.top = value;
        self
    }

    pub const fn with_bottom(mut self, value: u16) -> Self {
        self.bottom = value;
        self
    }
}
