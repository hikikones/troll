use std::ops::Range;

use ecs::*;
use terminal::*;
use widgets2::Block;

use crate::{
    app::{Action, Actions, App, Input, InputSet, RenderSet},
    modals::Modal,
    settings::{Colors, Settings},
};

pub(super) struct LogsModalPlugin;

impl LogsModalPlugin {
    pub(super) fn build(app: &mut App) {
        app.insert_resource(Logs::default())
            .add_enter(Modal::Logs, enter_logs)
            .add_input(InputSet::Modal, input_logs.run_if(in_state(Modal::Logs)))
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
        let _ = writeln!(self.text, "{error}");
        let mut source = error.source();
        while let Some(cause) = source {
            let _ = writeln!(self.text, "Caused by: {cause}");
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
                actions.push(Action::Modal(Modal::None));
            } else {
                logs.remove_current();
                if logs.is_empty() {
                    actions.push(Action::Modal(Modal::None));
                } else {
                    actions.push(Action::Render);
                }
            }
        }
        KeyCode::Char('c') => {
            // TODO: copy log to clipboard
        }
        _ => {}
    }
}

fn enter_logs(mut logs: ResMut<Logs>) {
    if logs.len_new > 0 {
        logs.len_new = 0;
        logs.index = logs.len().saturating_sub(1);
    }
}

fn render_logs(mut frame: ResMut<Framebuffer>, settings: Res<Settings>, logs: Res<Logs>) {
    let area = frame.area().scale_and_center(0.7);
    let colors = settings.colors();

    frame.fill(area, colors.background, false);

    let (top, body, bottom) = area.split_ends(1, 1);

    frame.print_fmt(Sgr::Fg(colors.secondary));
    Block::rectangle().render(area, &mut frame);

    frame.print_fmt(Sgr::Fg(colors.normal));
    frame.push_fmt(format_args!(" Logs: {} / {} ", logs.index + 1, logs.len()));
    frame.render(top, TextOptions::span_center_top());

    let Some(log) = logs.current() else {
        return;
    };

    frame.push_str(log);
    frame.render(
        body.inner(Margin::proportional(1)),
        TextOptions::paragraph_center(),
    );

    render_shortcuts(bottom, &mut frame, colors);

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
