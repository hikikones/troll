use std::ops::Range;

use bevy_ecs::{
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Res, ResMut},
};
use bevy_state::condition::in_state;
use terminal::*;
use widgets2::{Block, List, ListIndex};

use crate::{
    app::{Action, Actions, App, Colors, Frame, Input, InputState, RenderSet},
    modals::Modal,
};

pub(super) struct LogsModalPlugin;

impl LogsModalPlugin {
    pub(super) fn build(app: &mut App) {
        app.insert_resource(LogsModal::default())
            .insert_resource(Logs::default())
            .add_input(InputState::Modal, input_logs.run_if(in_state(Modal::Logs)))
            .add_render(RenderSet::Modal, render_logs.run_if(in_state(Modal::Logs)));
    }
}

#[derive(Resource)]
pub struct LogsModal {
    list: List,
}

impl Default for LogsModal {
    fn default() -> Self {
        Self {
            list: List::new()
                .with_scrollbar(0)
                .with_padding(Margin::horizontal(1)),
        }
    }
}

#[derive(Default, Resource)]
pub struct Logs {
    text: String,
    logs: Vec<Log>,
}

impl Logs {
    pub const fn is_empty(&self) -> bool {
        self.logs.is_empty()
    }

    pub const fn len(&self) -> usize {
        self.logs.len()
    }

    pub fn push(&mut self, error: impl std::error::Error) {
        use std::fmt::Write;

        let start = self.text.len();

        let _ = write!(self.text, "{error}");
        let mut source = error.source();
        while let Some(cause) = source {
            let _ = write!(self.text, ": {cause}");
            source = cause.source();
        }

        let range = start..self.text.len();
        let width = utils::str_width(&self.text[start..]);
        self.logs.push(Log {
            range,
            _width: width,
        });
    }

    fn iter(&self) -> impl ExactSizeIterator<Item = &str> {
        self.logs.iter().map(|log| &self.text[log.range.clone()])
    }

    fn clear(&mut self) {
        self.text.clear();
        self.logs.clear();
    }
}

struct Log {
    range: Range<usize>,
    _width: u16,
}

fn input_logs(
    input: Res<Input>,
    mut modal: ResMut<LogsModal>,
    mut logs: ResMut<Logs>,
    mut actions: ResMut<Actions>,
) {
    let Some(key) = input.get() else {
        return;
    };

    match key.code {
        KeyCode::Char('c') => {
            logs.clear();
            actions.push(Action::Modal(Modal::Confirmed));
        }
        _ => {
            if modal.list.input(key) {
                actions.push(Action::Render);
            }
        }
    }
}

fn render_logs(
    mut frame: ResMut<Frame>,
    colors: Res<Colors>,
    mut modal: ResMut<LogsModal>,
    logs: Res<Logs>,
) {
    let area = frame.area().scale_and_center(0.8);

    let bg = frame.palette().background().slight_offset().as_color();
    frame.fill(area, bg, false);

    frame.print_fmt(Sgr::Fg(colors.secondary));
    Block::rectangle().render(area, &mut frame);
    frame.print_fmt(Sgr::Fg(colors.normal));
    frame.print_span_with_options(area, " Logs ", SpanOptions::center_top());

    modal.list.render(
        area.inner(Margin::all(1)),
        &mut frame,
        logs.iter(),
        |line, frame, log, index| {
            let style = match index {
                ListIndex::Selected => Style::fg(colors.primary).with_reverse(),
                ListIndex::Selection => Style::fg(colors.normal).with_bg(bg),
                ListIndex::Normal => Style::fg(colors.normal).with_bg(bg),
            };

            frame.print_fmt(style);
            frame.print_span_with_options(line, log, SpanOptions::fill());
            frame.print_fmt(Sgr::Reset);
        },
    );

    let shortcuts_area = Rect::new(area.pos.with_row(area.bottom_row()), area.size.with_rows(1));
    render_shortcuts(shortcuts_area, &mut frame, &colors);

    frame.print_fmt(Sgr::Reset);
}

fn render_shortcuts(area: Rect, frame: &mut Framebuffer, colors: &Colors) {
    let key_color = colors.secondary;
    let name_color = colors.normal;
    let gap = 0;

    frame.push_ch(' ');

    for (key, name, gap) in [("c", "Clear", gap)] {
        frame.push_fmt(Sgr::Fg(key_color));
        frame.push_str(key);
        frame.push_fmt(Sgr::Fg(name_color));
        frame.push_ch(' ');
        frame.push_str(name);
        frame.push_ch_repeat(' ', gap);
    }

    frame.push_ch(' ');
    frame.render(area, TextOptions::span_center_top());
}
