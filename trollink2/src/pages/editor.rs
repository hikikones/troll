use terminal::*;

use crate::settings::Colors;

pub struct EditorPage {}

impl EditorPage {
    pub const fn new() -> Self {
        Self {}
    }

    pub fn render(&self, area: Rect, frame: &mut Framebuffer, colors: &Colors) {
        frame.print_span_with(area, "EDITOR", SpanOptions::center());
    }
}
