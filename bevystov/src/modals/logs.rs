use std::ops::Range;

use bevy_ecs::{
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Res, ResMut},
};
use bevy_state::condition::in_state;
use terminal::*;
use widgets2::Block;

use crate::{
    app::{Action, Actions, App, Colors, Frame, Input, InputState, RenderSet},
    modals::Modal,
};

pub(super) struct LogsModalPlugin;

impl LogsModalPlugin {
    pub(super) fn build(app: &mut App) {
        app.insert_resource(Logs::default())
            .add_input(InputState::Modal, input_logs.run_if(in_state(Modal::Logs)))
            .add_render(RenderSet::Modal, render_logs.run_if(in_state(Modal::Logs)));
    }
}

#[derive(Default, Resource)]
pub struct Logs {
    text: String,
    logs: Vec<Range<usize>>,
    len_new: usize,
    index: usize,
}

impl Logs {
    pub const fn is_empty(&self) -> bool {
        self.logs.is_empty()
    }

    pub const fn len(&self) -> usize {
        self.logs.len()
    }

    pub const fn len_new(&self) -> usize {
        self.len_new
    }

    pub fn push_err(&mut self, error: impl std::error::Error) {
        use std::fmt::Write;

        let start = self.text.len();
        let _ = write!(self.text, "{error}");
        let mut source = error.source();
        while let Some(cause) = source {
            let _ = write!(self.text, ": {cause}");
            source = cause.source();
        }
        self.add_log(start);
    }

    pub fn push_str(&mut self, s: impl AsRef<str>) {
        let start = self.text.len();
        self.text.push_str(s.as_ref());
        self.add_log(start);
    }

    pub fn push_fmt(&mut self, s: impl std::fmt::Display) {
        use std::fmt::Write;

        let start = self.text.len();
        let _ = write!(self.text, "{s}");
        self.add_log(start);
    }

    fn add_log(&mut self, start: usize) {
        self.logs.push(start..self.text.len());
        self.len_new += 1;
    }

    fn _iter(&self) -> impl ExactSizeIterator<Item = &str> {
        self.logs.iter().map(|range| &self.text[range.clone()])
    }

    fn current(&self) -> Option<&str> {
        self.logs
            .get(self.index)
            .map(|range| &self.text[range.clone()])
    }

    fn next(&mut self) {
        self.index += 1;

        if self.index >= self.logs.len() {
            self.index = 0;
        }
    }

    fn prev(&mut self) {
        if self.index == 0 {
            self.index = self.logs.len().saturating_sub(1);
        } else {
            self.index -= 1;
        }
    }

    fn remove_current(&mut self) {
        self.logs.remove(self.index);

        if self.index >= self.logs.len() {
            self.index = self.logs.len().saturating_sub(1);
        }
    }

    fn clear(&mut self) {
        self.text.clear();
        self.logs.clear();
    }
}

fn input_logs(input: Res<Input>, mut logs: ResMut<Logs>, mut actions: ResMut<Actions>) {
    let Some(key) = input.get() else {
        return;
    };

    match key.code {
        KeyCode::Right => {
            logs.next();
            actions.push(Action::Render);
        }
        KeyCode::Left => {
            logs.prev();
            actions.push(Action::Render);
        }
        KeyCode::Delete => {
            if key.ctrl() {
                logs.clear();
                actions.push(Action::Modal(Modal::Confirmed));
            } else {
                logs.remove_current();
                actions.push(Action::Render);
            }
        }
        KeyCode::Char('c') => {
            // TODO: copy log to clipboard
        }
        _ => {}
    }
}

fn render_logs(mut frame: ResMut<Frame>, colors: Res<Colors>, mut logs: ResMut<Logs>) {
    let area = frame.area().scale_and_center(0.6);

    let bg = frame.palette().background().slight_offset().as_color();
    frame.fill(area, bg, false);

    frame.print_fmt(Sgr::Fg(colors.secondary));
    Block::rectangle().render(area, &mut frame);
    frame.print_fmt(Sgr::Fg(colors.normal));
    frame.print_span_with_options(
        area,
        format_args!(" Logs: {} / {} ", logs.index + 1, logs.len()),
        SpanOptions::center_top(),
    );

    if logs.len_new > 0 {
        logs.len_new = 0;
        logs.index = logs.len().saturating_sub(1);
    }

    let Some(log) = logs.current() else {
        return;
    };

    frame.push_str(log);
    frame.render(
        area.inner(Margin::proportional(1)),
        TextOptions::paragraph_center(),
    );

    let shortcuts_area = Rect::new(area.pos.with_row(area.bottom_row()), area.size.with_rows(1));
    render_shortcuts(shortcuts_area, &mut frame, &colors);

    frame.print_fmt(Sgr::Reset);
}

fn render_shortcuts(area: Rect, frame: &mut Framebuffer, colors: &Colors) {
    let key_color = colors.secondary;
    let name_color = colors.normal;
    let gap = 1;

    frame.push_ch(' ');

    for (key, name, gap) in [
        ("⇄", "Next/Prev", gap),
        ("c", "Copy (todo)", gap),
        ("(^)Del", "Delete (all)", 0),
    ] {
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
