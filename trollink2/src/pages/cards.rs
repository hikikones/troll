use std::collections::HashSet;

use terminal::*;
use utils::StringExt;
use widgets2::{List, Markup, MarkupScroll};

use crate::{
    app::Action,
    database::{CardId, Database, TagId},
    modals::Modal,
    pages::{CardsParam, Route},
    settings::Colors,
};

pub struct CardsPage {
    cards: Vec<CardId>,
    index: usize,
    show_tags: bool,
    tags: TagsSidebar,
}

impl CardsPage {
    pub fn new() -> Self {
        Self {
            cards: Vec::new(),
            index: 0,
            show_tags: false,
            tags: TagsSidebar::new(),
        }
    }

    pub fn enter(&mut self, db: &mut Database, param: Option<CardsParam>) {
        match param {
            Some(param) => {
                match param {
                    CardsParam::Card(cid) => {
                        self.tags.clear();
                        self.tags.refresh(db);
                        db.get_tags_for_card(cid, |tid| {
                            self.tags.includes.insert(tid);
                        });
                        self.update_cards(db);
                        self.select_card(cid);
                    }
                    CardsParam::Tag(tid) => {
                        self.tags.clear();
                        self.tags.refresh(db);
                        self.tags.includes.insert(tid);
                        self.tags.select(tid);
                        self.update_cards(db);
                    }
                }
                self.show_tags = true;
            }
            None => {
                self.tags.refresh(db);
                self.update_cards(db);
            }
        }
    }

    pub fn exit(&self) {}

    pub fn refresh(&mut self, db: &mut Database) {
        self.tags.refresh(db);
        self.update_cards(db);
    }

    pub fn input(&mut self, key: Key, db: &mut Database, markup: &mut Markup) -> Action {
        // TODO: impl remaining shortcut actions for tags
        match key.code {
            KeyCode::Right => {
                return self.next_card(markup);
            }
            KeyCode::Left => {
                return self.previous_card(markup);
            }
            KeyCode::Delete => {
                return self.delete_card(db, markup);
            }
            KeyCode::Char('e') => {
                if let Some(id) = self.current_card() {
                    return Action::Route(Route::Editor(Some(id)));
                }
            }
            KeyCode::Char('n') if self.show_tags => return Action::Modal(Some(Modal::TagNew)),
            KeyCode::Char('t') => {
                return self.toggle_show_tags();
            }
            KeyCode::Char(' ') => {
                if self.show_tags {
                    if self.tags.toggle_selection() {
                        self.update_cards(db);
                        return Action::Render;
                    }
                }
            }
            KeyCode::Char('r') => {
                if self.show_tags && !self.tags.is_empty() {
                    if self.tags.reset() {
                        self.update_cards(db);
                        return Action::Render;
                    }
                }
            }
            _ => {
                if self.show_tags {
                    if self.tags.list.input(key) {
                        return Action::Render;
                    }
                } else {
                    if markup.input(key.code) {
                        return Action::Render;
                    }
                }
            }
        }

        Action::None
    }

    pub fn render(
        &mut self,
        mut area: Rect,
        frame: &mut Framebuffer,
        db: &Database,
        colors: &Colors,
        markup: &mut Markup,
    ) {
        if self.show_tags {
            let (left, right) = area.split_left(area.cols() / 3, 2);
            self.tags.render(left, frame, colors);
            area = right;
        }

        match self.current_card() {
            Some(id) => {
                frame.push_fmt_fg(
                    format_args!("{} / {}", self.index + 1, self.cards.len()),
                    colors.neutral,
                );
                frame.render(area, TextOptions::span_center_top());

                area.shrink_down(2);

                db.get_card_content(id, |content| {
                    markup.render(area, frame, content);
                });
            }
            None => {
                frame.push_str_fg("No cards", colors.neutral);
                frame.render(area, TextOptions::span_center());
            }
        }
    }

    pub fn render_shortcuts(&self, area: Rect, frame: &mut Framebuffer, colors: &Colors) {
        let gap = 1;
        let has_tags = self.show_tags && !self.tags.is_empty();

        for (key, name, gap, _show) in [
            ("n", "New", gap, self.show_tags),
            ("e", "Edit", gap, true),
            ("Space", "Toggle", gap, has_tags),
            ("r", "Reset", gap, has_tags),
            ("t", "Tags", gap, true),
            ("Del", "Delete", 0, true),
        ]
        .into_iter()
        .filter(|s| s.3)
        {
            frame.push_fmt(Sgr::Fg(colors.secondary));
            frame.push_str(key);
            frame.push_fmt(Sgr::Fg(colors.normal));
            frame.push_ch(' ');
            frame.push_str(name);
            frame.push_ch_repeat(' ', gap);
        }

        frame.push_fmt(Sgr::reset_fg());
        frame.render(area, TextOptions::span_center_top());
    }

    fn current_card(&self) -> Option<CardId> {
        self.cards.get(self.index).copied()
    }

    fn next_card(&mut self, markup: &mut Markup) -> Action {
        if self.cards.len() > 1 {
            self.index = (self.index + 1) % self.cards.len();
            markup.scroll(MarkupScroll::Start);
            return Action::Render;
        }
        Action::None
    }

    fn previous_card(&mut self, markup: &mut Markup) -> Action {
        if self.cards.len() > 1 {
            if self.index == 0 {
                self.index = self.cards.len() - 1;
            } else {
                self.index -= 1;
            }
            markup.scroll(MarkupScroll::Start);
            return Action::Render;
        }
        Action::None
    }

    fn select_card(&mut self, id: CardId) {
        if let Some(i) = self
            .cards
            .iter()
            .copied()
            .enumerate()
            .find(|(_, cid)| id == *cid)
            .map(|(i, _)| i)
        {
            self.index = i;
        }
    }

    fn delete_card(&mut self, db: &Database, markup: &mut Markup) -> Action {
        if self.index < self.cards.len() {
            let id = self.cards.remove(self.index);
            db.delete_card(id);

            if !self.cards.is_empty() {
                self.index = self.index.min(self.cards.len() - 1);
                markup.scroll(MarkupScroll::Start);
            }

            return Action::Render;
        }
        Action::None
    }

    fn update_cards(&mut self, db: &mut Database) {
        self.cards.clear();
        self.index = 0;

        db.get_cards_with_tags(self.tags.includes(), self.tags.excludes(), |id| {
            self.cards.push(id);
        });
    }

    fn toggle_show_tags(&mut self) -> Action {
        self.show_tags = !self.show_tags;
        return Action::Render;
    }
}

struct TagsSidebar {
    tags: Vec<TagItem>,
    names: String,
    includes: HashSet<TagId>,
    excludes: HashSet<TagId>,
    list: List,
}

impl TagsSidebar {
    fn new() -> Self {
        Self {
            tags: Vec::new(),
            names: String::new(),
            includes: HashSet::new(),
            excludes: HashSet::new(),
            list: List::new(),
        }
    }

    fn refresh(&mut self, db: &Database) {
        self.tags.clear();
        self.names.clear();

        db.get_tags_and_cards_count(|id, name, cards_count| {
            self.tags
                .push(TagItem::new(id, name, cards_count, &mut self.names))
        });

        self.includes
            .retain(|id| self.tags.iter().any(|tag| tag.id == *id));
        self.excludes
            .retain(|id| self.tags.iter().any(|tag| tag.id == *id));
    }

    const fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }

    fn toggle(&mut self, id: TagId) {
        if self.includes.remove(&id) {
            self.excludes.insert(id);
            return;
        }

        if self.excludes.remove(&id) {
            return;
        }

        self.includes.insert(id);
    }

    fn toggle_selection(&mut self) -> bool {
        if self.tags.is_empty() {
            return false;
        }

        for i in self.list.selection_inclusive() {
            let tag = &self.tags[i];
            self.toggle(tag.id);
        }

        true
    }

    fn includes(&self) -> impl ExactSizeIterator<Item = TagId> {
        self.includes.iter().copied()
    }

    fn excludes(&self) -> impl ExactSizeIterator<Item = TagId> {
        self.excludes.iter().copied()
    }

    fn select(&mut self, id: TagId) {
        if let Some(i) = self
            .tags
            .iter()
            .enumerate()
            .find(|(_, tag)| tag.id == id)
            .map(|(i, _)| i)
        {
            self.list.reset();
            self.list.set_index(i);
        }
    }

    fn reset(&mut self) -> bool {
        let has_includes = !self.includes.is_empty();
        let has_excludes = !self.excludes.is_empty();
        self.includes.clear();
        self.excludes.clear();
        has_includes || has_excludes
    }

    fn clear(&mut self) {
        self.tags.clear();
        self.names.clear();
        self.includes.clear();
        self.excludes.clear();
        self.list.reset();
    }

    fn render(&mut self, mut area: Rect, frame: &mut Framebuffer, colors: &Colors) {
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

        for (index, tag) in lines.zip(self.tags.iter().skip(self.list.scroll())) {
            let id = tag.id;

            let color = if self.includes.contains(&id) {
                Color::Green
            } else if self.excludes.contains(&id) {
                Color::Red
            } else {
                Color::Default
            };

            frame.push_fmt(Sgr::Fg(color));

            if index == selected {
                frame.push_str("> ");
            } else if selection.contains(&index) {
                frame.push_str("| ");
            }

            frame.push_str(self.names.slice(tag.name));
            frame.push_fmt(Sgr::reset_fg());
            frame.render(line, TextOptions::span());

            line.add_row(1);
        }
    }
}

struct TagItem {
    id: TagId,
    name: std::range::Range<usize>,
}

impl TagItem {
    fn new(id: TagId, name: &str, cards_count: u32, writer: &mut String) -> Self {
        Self {
            id,
            name: writer.write_get_range(format_args!("{name} ({cards_count})")),
        }
    }
}
