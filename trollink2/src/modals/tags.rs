use terminal::*;
use widgets2::{Block, Prompt};

use crate::{app::Action, database::Database, settings::Colors};

pub struct NewTagModal {
    prompt: Prompt,
    message: Message,
}

enum Message {
    None,
    Empty,
    Taken,
}

impl NewTagModal {
    pub const fn new() -> Self {
        Self {
            prompt: Prompt::new().with_placeholder("Enter a tag name..."),
            message: Message::None,
        }
    }

    pub fn enter(&self) {}

    pub fn exit(&mut self) {
        self.prompt.clear();
        self.message = Message::None;
    }

    pub fn input(&mut self, key: Key, db: &mut Database) -> Action {
        match key.code {
            KeyCode::Enter => {
                let name = self.prompt.as_str_trim();

                if name.is_empty() {
                    self.message = Message::Empty;
                    return Action::Render;
                }

                if db.add_tag(name).is_some() {
                    return Action::Refresh;
                }

                self.message = Message::Taken;
                return Action::Render;
            }
            _ => {
                if self.prompt.input(key) {
                    return Action::Render;
                }
            }
        }

        Action::None
    }

    pub fn render(&mut self, frame: &mut Framebuffer, colors: &Colors) {
        let area = frame.area().scale_and_center(0.5);

        frame.fill(area, colors.background, false);

        frame.print_fmt(Sgr::Fg(colors.secondary));
        Block::rectangle().render(area, frame);
        frame.print_fmt(Sgr::reset_fg());

        let (top, body, bottom) = area.split_ends(1, 1);

        // Prompt
        let prompt_area = top.scale_cols_and_center(0.5);
        frame.cursor(prompt_area.pos);
        frame.print_ch_repeat(' ', prompt_area.cols());
        self.prompt
            .set_colors(colors.prompt())
            .render(prompt_area.inner(Margin::horizontal(1)), frame);

        // Message
        let (msg, color) = match self.message {
            Message::None => ("Please enter a tag name.", colors.neutral),
            Message::Empty => ("Empty tag name.", colors.red),
            Message::Taken => ("Tag name is already taken.", colors.red),
        };
        frame.push_str_fg(msg, color);
        frame.render(body, TextOptions::paragraph_center());

        self.render_shortcuts(bottom, frame, colors);

        frame.print_fmt(Sgr::Reset);
    }

    fn render_shortcuts(&self, area: Rect, frame: &mut Framebuffer, colors: &Colors) {
        frame.push_fmt(format_args!(
            " {}↵{} Confirm ",
            Sgr::Fg(colors.secondary),
            Sgr::Fg(colors.normal),
        ));
        frame.render(area, TextOptions::span_center_top());
    }
}
