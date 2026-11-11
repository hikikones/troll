use std::collections::HashMap;

use terminal::*;
use widgets2::{Editor, List, Markup};

use crate::{
    app::Action,
    database::{CardId, Database, TagId},
    settings::Colors,
};

pub struct EditorPage {
    editor: Editor,
    preview: bool,
    show_tags: bool,
    tags: TagsSidebar,
    card: Option<CardId>,
}

impl EditorPage {
    pub fn new() -> Self {
        Self {
            editor: Editor::new().with_placeholder("Content..."),
            preview: false,
            show_tags: false,
            tags: TagsSidebar::new(),
            card: None,
        }
    }

    pub fn enter(&mut self, id: Option<CardId>, db: &Database) {
        self.preview = false;
        self.tags.update(db);
        self.card = id;

        if let Some(id) = id {
            self.editor.clear();
            db.get_card_content(id, |content| {
                self.editor.push_str(content);
                self.editor.move_to_start(false);
                self.tags.reset_toggles(db, Some(id));
            });
        }
    }

    pub fn exit(&mut self) {
        if self.card.is_some() {
            self.editor.clear();
            self.show_tags = false;
            self.tags.clear_toggles();
        }
    }

    pub fn refresh(&self) {}

    pub fn input(
        &mut self,
        key: Key,
        db: &mut Database,
        markup: &mut Markup,
        terminal: &mut Terminal,
    ) -> Action {
        match key.code {
            KeyCode::Char('e') if key.ctrl() => {
                match terminal.temp_leave(|| {
                    edit::edit_with_builder(
                        self.editor.as_str(),
                        edit::Builder::new().suffix(".md"),
                    )
                }) {
                    Ok(content) => {
                        self.editor.clear();
                        self.editor.push_str(&content);
                        self.editor.move_to_start(false);
                        return Action::Clear;
                    }
                    Err(err) => {
                        // TODO: Have AppError as action. Also, clear?
                        // return Action::EnqueueLog(Log::new(err));
                    }
                }
            }
            KeyCode::Char('p') if key.ctrl() => {
                self.preview = !self.preview;
                return Action::Render;
            }
            KeyCode::Char('s') if key.ctrl() && !self.editor.is_empty() => {
                self.save(db);
                // markup.clear(); todo?
                return Action::Render;
            }
            KeyCode::Char('t') if key.ctrl() => {
                self.show_tags = !self.show_tags;
                return Action::Render;
            }
            KeyCode::Char(' ') if self.show_tags => {
                if self.tags.toggle_selection() {
                    return Action::Render;
                }
            }
            KeyCode::Char('r') if self.show_tags => {
                if self.tags.reset_toggles(db, self.card) {
                    return Action::Render;
                }
            }
            _ => {
                if self.show_tags {
                    if self.tags.list.input(key) {
                        return Action::Render;
                    }
                } else if self.preview {
                    if markup.input(key.code) {
                        return Action::Render;
                    }
                } else if self.editor.input(key) {
                    return Action::Render;
                }
            }
        }

        Action::None
    }

    pub fn render(
        &mut self,
        mut area: Rect,
        frame: &mut Framebuffer,
        colors: &Colors,
        db: &Database,
        markup: &mut Markup,
    ) {
        if self.show_tags {
            let (left, right) = area.split_left(area.cols() / 3, 2);
            self.tags.render(left, frame, db, colors);
            area = right;
        }

        frame.push_str_fg(
            if self.card.is_some() {
                "Edit Card"
            } else {
                "New Card"
            },
            colors.neutral,
        );
        frame.render(area, TextOptions::span_center_top());

        area.shrink_down(2);

        if self.preview {
            markup.render(area, frame, self.editor.as_str());
        } else {
            self.editor
                // .set_colors(colors.text_editor()) // TODO
                .render(area, frame);
        }
    }

    pub fn render_shortcuts(&self, area: Rect, frame: &mut Framebuffer, colors: &Colors) {
        // todo
    }

    fn save(&mut self, db: &Database) {
        let cid = match self.card.take() {
            Some(cid) => {
                db.update_card(cid, self.editor.as_str());

                // Remove tags
                for tid in self.tags.to_remove() {
                    db.delete_tag_for_card(cid, tid);
                }

                cid
            }
            None => db.add_card(self.editor.as_str()),
        };

        // Add tags
        for tid in self.tags.to_add() {
            db.add_tag_for_card(cid, tid);
        }

        self.preview = false;
        self.show_tags = false;
        self.editor.clear();
        self.tags.clear_toggles();
    }
}

struct TagsSidebar {
    tags: Vec<TagId>,
    toggles: HashMap<TagId, TagState>,
    list: List,
}

#[derive(Debug, Clone, Copy)]
enum TagState {
    Add,
    Keep,
    Remove,
}

impl TagsSidebar {
    fn new() -> Self {
        Self {
            tags: Vec::new(),
            toggles: HashMap::new(),
            list: List::new(),
        }
    }

    fn update(&mut self, db: &Database) {
        self.tags.clear();
        db.get_tags(|tid| self.tags.push(tid));
    }

    const fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }

    fn to_add(&self) -> impl Iterator<Item = TagId> {
        self.toggles
            .iter()
            .filter(|(_, state)| matches!(state, TagState::Add))
            .map(|(id, _)| *id)
    }

    fn to_remove(&self) -> impl Iterator<Item = TagId> {
        self.toggles
            .iter()
            .filter(|(_, state)| matches!(state, TagState::Remove))
            .map(|(id, _)| *id)
    }

    fn toggle(&mut self, id: TagId) {
        match self.toggles.get_mut(&id) {
            Some(state) => match state {
                TagState::Add => {
                    self.toggles.remove(&id);
                }
                TagState::Keep => {
                    *state = TagState::Remove;
                }
                TagState::Remove => {
                    *state = TagState::Keep;
                }
            },
            None => {
                self.toggles.insert(id, TagState::Add);
            }
        }
    }

    fn toggle_selection(&mut self) -> bool {
        if self.tags.is_empty() {
            return false;
        }

        for i in self.list.selection_inclusive() {
            let id = self.tags[i];
            self.toggle(id);
        }

        true
    }

    fn reset_toggles(&mut self, db: &Database, cid: Option<CardId>) -> bool {
        let is_empty = self.toggles.is_empty();
        self.toggles.clear();

        if let Some(cid) = cid {
            db.get_tags_for_card(cid, |tid| {
                self.toggles.insert(tid, TagState::Keep);
            });
        }

        !is_empty
    }

    fn clear_toggles(&mut self) {
        self.toggles.clear();
    }

    fn render(&mut self, mut area: Rect, frame: &mut Framebuffer, db: &Database, colors: &Colors) {
        frame.print_span_with(area, "Tags", SpanOptions::center_top());

        if self.tags.is_empty() {
            frame.push_str_fg("No tags", colors.neutral);
            frame.render(area, TextOptions::span_center());
            return;
        }

        area.shrink_down(2);

        // Render tags
        let Some((area, lines)) = self
            .list
            .set_len(self.tags.len())
            .set_colors(colors.list())
            .render(area, frame)
        else {
            return;
        };

        let mut line = area.with_rows(1);
        let selected = self.list.index();
        let selection = self.list.selection_inclusive();

        for (index, tid) in lines.zip(self.tags.iter().copied().skip(self.list.scroll())) {
            let color = self
                .toggles
                .get(&tid)
                .map(|state| match state {
                    TagState::Add | TagState::Keep => Color::Green,
                    TagState::Remove => Color::Red,
                })
                .unwrap_or(Color::Default);

            frame.push_fmt(Sgr::Fg(color));

            if index == selected {
                frame.push_str("> ");
            } else if selection.contains(&index) {
                frame.push_str("| ");
            }

            db.get_tag_name(tid, |name| {
                frame.push_str(name);
            });

            frame.push_fmt(Sgr::reset_fg());
            frame.render(line, TextOptions::span());

            line.add_row(1);
        }
    }
}
