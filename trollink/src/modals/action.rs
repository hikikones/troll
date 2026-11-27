use terminal::*;
use widgets::{Block, Prompt};

use crate::{
    app::Action,
    database::{CardId, Database, TagId},
    settings::Colors,
};

pub struct ActionModal {
    action: ModalAction,
    prompt: Prompt,
    message: Option<Message>,
}

#[derive(Debug, Clone, Copy)]
pub enum ModalAction {
    AddTag,
    EditTag(TagId),
    DeleteTag(TagId),
    DeleteCard(CardId),
}

impl ModalAction {
    const fn title(self) -> &'static str {
        match self {
            Self::AddTag => " New Tag ",
            Self::EditTag(_) => " Edit Tag ",
            Self::DeleteTag(_) => " Delete Tag ",
            Self::DeleteCard(_) => " Delete Card ",
        }
    }

    const fn rows(self) -> u16 {
        match self {
            Self::AddTag | Self::EditTag(_) => 3,
            Self::DeleteTag(_) => 10,
            Self::DeleteCard(_) => 5,
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

impl ActionModal {
    pub const fn new() -> Self {
        Self {
            action: ModalAction::AddTag,
            prompt: Prompt::new().with_placeholder("Tag name..."),
            message: None,
        }
    }

    pub fn enter(&mut self, op: ModalAction, db: &Database) {
        self.action = op;

        if let ModalAction::EditTag(id) = op {
            db.get_tag_name(id, |name| self.prompt.push_str(name));
        }
    }

    pub fn exit(&mut self) {
        self.prompt.clear();
        self.message = None;
    }

    pub fn input(&mut self, key: Key, db: &mut Database) -> Action {
        match self.action {
            ModalAction::AddTag => match key.code {
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
            ModalAction::EditTag(id) => match key.code {
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
            ModalAction::DeleteTag(id) => {
                // TODO: Add option for also deleting related cards.
                match key.code {
                    KeyCode::Char('y') => {
                        db.delete_tag(id);
                        return Action::Refresh;
                    }
                    KeyCode::Char('n') => {
                        return Action::Modal(None);
                    }
                    _ => {}
                }
            }
            ModalAction::DeleteCard(id) => match key.code {
                KeyCode::Char('y') => {
                    db.delete_card(id);
                    return Action::Refresh;
                }
                KeyCode::Char('n') => {
                    return Action::Modal(None);
                }
                _ => {}
            },
        }

        Action::None
    }

    pub fn render(&mut self, frame: &mut Frame, colors: &Colors, db: &Database) {
        let area = frame
            .area()
            .scale_cols(0.6)
            .with_size_min(Size::new(Size::SMALL.cols, self.action.rows()))
            .center(frame.area());

        frame.fill(area, colors.background, false);

        frame.print_fmt(Sgr::Fg(colors.secondary));
        Block::rectangle().render(area, frame);
        frame.print_fmt(Sgr::reset_fg());

        let (top, mut body, bottom) = area.inner(Margin::horizontal(2)).split_ends(1, 1);

        frame.print_span(top, self.action.title(), SpanOptions::center_top());

        match self.action {
            ModalAction::AddTag | ModalAction::EditTag(_) => {
                if let Some(msg) = self.message {
                    let (left, right) = body.split_vertically(0, 1);
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
            ModalAction::DeleteTag(id) => {
                frame.push_str("You are about to delete the following tag:\n\n");
                db.get_tag_name(id, |name| {
                    frame.push_fmt(Sgrs([Sgr::Bold, Sgr::Fg(colors.secondary)]));
                    frame.push_str(name);
                    frame.push_fmt(Sgr::Reset);
                });
                frame.push_str("\n\nAre you sure you want to continue?");
                frame.render(body, TextOptions::paragraph_center());

                frame.push_fmt(format_args!(
                    " {}y{} Yes {}n{} No ",
                    Sgr::Fg(colors.secondary),
                    Sgr::Fg(colors.normal),
                    Sgr::Fg(colors.secondary),
                    Sgr::Fg(colors.normal),
                ));
                frame.render(bottom, TextOptions::span_center_top());
            }
            ModalAction::DeleteCard(_) => {
                frame.push_str("Are you sure you want to delete this card?");
                frame.render(body, TextOptions::paragraph_center());

                frame.push_fmt(format_args!(
                    " {}y{} Yes {}n{} No ",
                    Sgr::Fg(colors.secondary),
                    Sgr::Fg(colors.normal),
                    Sgr::Fg(colors.secondary),
                    Sgr::Fg(colors.normal),
                ));
                frame.render(bottom, TextOptions::span_center_top());
            }
        }

        frame.print_fmt(Sgr::Reset);
    }
}
