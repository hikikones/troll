use terminal::*;

use crate::settings::Colors;

pub struct SettingsPage {}

impl SettingsPage {
    pub const fn new() -> Self {
        Self {}
    }

    pub fn render(&self, area: Rect, frame: &mut Framebuffer, colors: &Colors) {
        frame.print_span_with(area, "SETTINGS", SpanOptions::center());
    }
}
