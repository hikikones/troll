use std::range::Range;

use terminal::*;
use widgets2::Block;

use crate::{app::Action, settings::Colors};

pub struct LogsModal {
    text: String,
    logs: Vec<Range<usize>>,
    len_new: usize,
    index: usize,
}

impl LogsModal {
    pub const fn new() -> Self {
        Self {
            text: String::new(),
            logs: Vec::new(),
            len_new: 0,
            index: 0,
        }
    }

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

    pub fn enter(&mut self) {
        if self.len_new > 0 {
            self.len_new = 0;
            self.index = self.len().saturating_sub(1);
        }
    }

    pub fn exit(&self) {}

    pub fn input(&mut self, key: Key) -> Action {
        match key.code {
            KeyCode::Right => {
                self.next();
                Action::Render
            }
            KeyCode::Left => {
                self.prev();
                Action::Render
            }
            KeyCode::Delete => {
                if key.ctrl() {
                    self.clear();
                    Action::Modal(None)
                } else {
                    self.remove_current();
                    if self.is_empty() {
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

    pub fn render(&self, frame: &mut Framebuffer, colors: &Colors) {
        let area = frame.area().scale_and_center(0.7);

        frame.fill(area, colors.background, false);

        let Some(log) = self.current() else {
            frame.print_fmt(Sgr::Fg(colors.secondary));
            Block::rectangle().render(area, frame);
            frame.print_fmt(Sgr::Fg(colors.normal));
            frame.print_span_with(area, " Logs ", SpanOptions::center_top());
            frame.print_fmt(Sgr::Fg(colors.neutral));
            frame.print_span_with(
                area.inner(Margin::proportional(1)),
                "No logs",
                SpanOptions::center(),
            );
            frame.print_fmt(Sgr::reset_fg());
            return;
        };

        let (top, body, bottom) = area.split_ends(1, 1);

        frame.print_fmt(Sgr::Fg(colors.secondary));
        Block::rectangle().render(area, frame);

        frame.print_fmt(Sgr::Fg(colors.normal));
        frame.push_fmt(format_args!(
            " Logs: {} / {} ",
            self.index + 1,
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

    fn add_log(&mut self, start: usize) {
        self.logs.push(Range {
            start,
            end: self.text.len(),
        });
        self.len_new += 1;
    }

    fn _iter(&self) -> impl ExactSizeIterator<Item = &str> {
        self.logs.iter().copied().map(|range| &self.text[range])
    }

    fn current(&self) -> Option<&str> {
        self.logs
            .get(self.index)
            .copied()
            .map(|range| &self.text[range])
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
