use bevy_ecs::{
    schedule::IntoScheduleConfigs,
    system::{Res, ResMut},
};
use bevy_state::condition::in_state;
use terminal::*;
use widgets2::Block;

use crate::{
    app::{App, Colors, Frame, RenderSet},
    modals::Modal,
};

pub(super) struct LogsModalPlugin;

impl LogsModalPlugin {
    pub(super) fn build(app: &mut App) {
        app.add_render(RenderSet::Modal, render_logs.run_if(in_state(Modal::Logs)));
    }
}

fn render_logs(mut frame: ResMut<Frame>, colors: Res<Colors>) {
    let area = frame.area().inner(Margin::proportional(6));

    let bg = frame.palette().background().slight_offset().as_color();
    frame.fill(area, bg);

    let style = Style::fg(colors.normal).with_bg(bg);
    Block::rectangle(style).render(area, &mut *frame);
    frame.print_span_with_options(
        area,
        format_args!("{} Logs {}", style, Sgr::Reset),
        SpanOptions::center_top(),
    );
}
