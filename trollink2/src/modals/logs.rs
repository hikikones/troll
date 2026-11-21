use shared::logs::Logs;
use terminal::*;
use widgets2::Block;

use crate::{app::Action, settings::Colors};

pub struct LogsModal {
    pub logs: Logs,
}

impl LogsModal {
    pub const fn new(logs: Logs) -> Self {
        Self { logs }
    }

    pub fn enter(&mut self) {
        if self.logs.len_new() > 0 {
            self.logs.reset_new();
            self.logs.last();
        }
    }

    pub fn exit(&self) {}

    pub fn input(&mut self, key: Key) -> Action {
        match key.code {
            KeyCode::Right => {
                self.logs.next();
                Action::Render
            }
            KeyCode::Left => {
                self.logs.prev();
                Action::Render
            }
            KeyCode::Delete => {
                if key.ctrl() {
                    self.logs.clear();
                    Action::Modal(None)
                } else {
                    self.logs.remove_current();
                    if self.logs.is_empty() {
                        Action::Modal(None)
                    } else {
                        Action::Render
                    }
                }
            }
            KeyCode::Char('c') => {
                // TODO: copy log to clipboard
                Action::None
            }
            _ => Action::None,
        }
    }

    pub fn render(&self, frame: &mut Frame, colors: &Colors) {
        let area = frame.area().scale_and_center(0.7);

        frame.fill(area, colors.background, false);

        let Some(log) = self.logs.current() else {
            frame.print_fmt(Sgr::Fg(colors.secondary));
            Block::rectangle().render(area, frame);
            frame.print_fmt(Sgr::Fg(colors.normal));
            frame.print_span(area, " Logs ", SpanOptions::center_top());
            frame.print_fmt(Sgr::Fg(colors.neutral));
            frame.print_span(
                area.inner(Margin::proportional(1)),
                "No logs",
                SpanOptions::center(),
            );
            frame.print_fmt(Sgr::Reset);
            return;
        };

        let (top, body, bottom) = area.split_ends(1, 1);

        frame.print_fmt(Sgr::Fg(colors.secondary));
        Block::rectangle().render(area, frame);

        frame.print_fmt(Sgr::Fg(colors.normal));
        frame.push_fmt(format_args!(
            " Logs: {} / {} ",
            self.logs.index() + 1,
            self.logs.len()
        ));
        frame.render(top, TextOptions::span_center_top());

        frame.push_str(log);
        frame.render(
            body.inner(Margin::proportional(1)),
            TextOptions::paragraph_center(),
        );

        render_shortcuts(bottom, frame, colors);

        frame.print_fmt(Sgr::Reset);
    }
}

fn render_shortcuts(area: Rect, frame: &mut Frame, colors: &Colors) {
    let gap = 1;

    frame.push_ch(' ');

    for (key, name, gap) in [
        ("⇄", "Next/Prev", gap),
        ("c", "Copy (todo)", gap),
        ("(^)Del", "Delete (all)", 0),
    ] {
        frame.push_fmt(Sgr::Fg(colors.secondary));
        frame.push_str(key);
        frame.push_fmt(Sgr::Fg(colors.normal));
        frame.push_ch(' ');
        frame.push_str(name);
        frame.push_ch_repeat(' ', gap);
    }

    frame.push_ch(' ');
    frame.render(area, TextOptions::span_center_top());
}
