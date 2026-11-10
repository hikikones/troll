use terminal::*;
use widgets2::Block;

use crate::{app::Action, settings::Colors};

pub struct SearchModal {}

impl SearchModal {
    pub const fn new() -> Self {
        Self {}
    }

    pub fn enter(&self) {}

    pub fn exit(&self) {}

    pub fn input(&mut self, key: Key) -> Action {
        Action::None
    }

    pub fn render(&self, frame: &mut Framebuffer, colors: &Colors) {
        let area = frame.area().scale_and_center(0.7);

        frame.fill(area, colors.background, false);

        let (top, body, bottom) = area.split_ends(1, 1);

        frame.print_fmt(Sgr::Fg(colors.secondary));
        Block::rectangle().render(area, frame);

        frame.print_fmt(Sgr::Fg(colors.normal));
        frame.push_str(" Search ");
        frame.render(top, TextOptions::span_center_top());

        // todo
        // render_shortcuts(bottom, &mut frame, colors);

        frame.print_fmt(Sgr::Reset);
    }
}
