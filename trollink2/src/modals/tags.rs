use terminal::*;
use widgets2::{Block, Prompt};

use crate::{
    app::Action,
    database::{Database, TagId},
    settings::Colors,
};

pub struct TagsModal {
    op: TagOp,
    prompt: Prompt,
    message: Option<Message>,
}

#[derive(Debug, Clone, Copy)]
pub enum TagOp {
    Add,
    Edit(TagId),
    Delete(TagId),
}

impl TagOp {
    const fn modal_title(self) -> &'static str {
        match self {
            Self::Add => " New Tag ",
            Self::Edit(_) => " Edit Tag ",
            Self::Delete(_) => " Delete Tag ",
        }
    }

    const fn rows(self) -> u16 {
        match self {
            Self::Add | Self::Edit(_) => 3,
            Self::Delete(_) => 12,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Message {
    Empty,
    Taken,
}

impl Message {
    const fn get(&self) -> &str {
        match self {
            Self::Empty => "Empty tag",
            Self::Taken => "Tag taken",
        }
    }
}

impl TagsModal {
    pub const fn new() -> Self {
        Self {
            op: TagOp::Add,
            prompt: Prompt::new().with_placeholder("Tag name..."),
            message: None,
        }
    }

    pub fn enter(&mut self, op: TagOp, db: &Database) {
        self.op = op;

        if let TagOp::Edit(id) = op {
            db.get_tag_name(id, |name| self.prompt.push_str(name));
        }
    }

    pub fn exit(&mut self) {
        self.prompt.clear();
        self.message = None;
    }

    pub fn input(&mut self, key: Key, db: &mut Database) -> Action {
        match self.op {
            TagOp::Add => match key.code {
                KeyCode::Enter => {
                    let name = self.prompt.as_str_trim();

                    if name.is_empty() {
                        self.message = Some(Message::Empty);
                        return Action::Render;
                    }

                    if db.add_tag(name).is_some() {
                        return Action::Refresh;
                    }

                    self.message = Some(Message::Taken);
                    return Action::Render;
                }
                _ => {
                    if self.prompt.input(key) {
                        self.message = None;
                        return Action::Render;
                    }
                }
            },
            TagOp::Edit(id) => match key.code {
                KeyCode::Enter => {
                    let name = self.prompt.as_str_trim();

                    if name.is_empty() {
                        self.message = Some(Message::Empty);
                        return Action::Render;
                    }

                    if db.update_tag(id, name) {
                        return Action::Refresh;
                    }

                    self.message = Some(Message::Taken);
                    return Action::Render;
                }
                _ => {
                    if self.prompt.input(key) {
                        self.message = None;
                        return Action::Render;
                    }
                }
            },
            TagOp::Delete(id) => {
                //todo
            }
        }

        Action::None
    }

    pub fn render(&mut self, frame: &mut Framebuffer, colors: &Colors, db: &Database) {
        let area = frame
            .area()
            .with_rows_min(self.op.rows())
            .scale_cols(0.5)
            .center(frame.area());

        frame.fill(area, colors.background, false);

        frame.print_fmt(Sgr::Fg(colors.secondary));
        Block::rectangle().render(area, frame);
        frame.print_fmt(Sgr::reset_fg());

        let (top, mut body, bottom) = area.inner(Margin::horizontal(2)).split_ends(1, 1);

        frame.print_span_with(top, self.op.modal_title(), SpanOptions::center_top());

        match self.op {
            TagOp::Add | TagOp::Edit(_) => {
                if let Some(msg) = self.message {
                    let (left, right) = body.split_vertically_with_gap(0);
                    frame.push_str_fg(msg.get(), colors.red);
                    frame.render(right, TextOptions::span_center_top());
                    body = left;
                }

                self.prompt.set_colors(colors.prompt()).render(body, frame);

                frame.push_fmt(format_args!(
                    " {}↵{} Confirm ",
                    Sgr::Fg(colors.secondary),
                    Sgr::Fg(colors.normal),
                ));
                frame.render(bottom, TextOptions::span_center_top());
            }
            TagOp::Delete(id) => {
                //todo
            }
        }

        frame.print_fmt(Sgr::Reset);
    }
}
