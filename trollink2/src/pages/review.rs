use terminal::*;

use crate::{app::Action, settings::Colors};

pub struct ReviewPage {}

impl ReviewPage {
    pub const fn new() -> Self {
        Self {}
    }

    pub fn enter(&self) {}

    pub fn exit(&self) {}

    pub fn refresh(&self) {}

    pub fn input(&mut self, key: Key) -> Action {
        Action::None
    }

    pub fn render(&self, area: Rect, frame: &mut Framebuffer, colors: &Colors) {
        frame.print_span_with(area, "REVIEW", SpanOptions::center());
    }
}
