use terminal::*;
use utils::StringExt;
use widgets2::{AnsiViewer, Block, Prompt};

use crate::{
    app::Action,
    database::{CardId, Database},
    pages::{CardsParam, Route},
    settings::Colors,
};

pub struct SearchModal {
    state: State,
    search: Prompt,
    results: Vec<CardId>,
    index: usize,
    query: String,
    writer: String,
    viewer: AnsiViewer,
    is_empty: bool,
}

enum State {
    Search,
    Browse,
}

impl SearchModal {
    pub const fn new() -> Self {
        Self {
            state: State::Search,
            search: Prompt::new().with_placeholder("Search..."),
            results: Vec::new(),
            index: 0,
            query: String::new(),
            writer: String::new(),
            viewer: AnsiViewer::new(),
            is_empty: false,
        }
    }

    pub fn enter(&mut self, db: &Database) {
        self.state = State::Search;
        self.is_empty = db.is_cards_empty();
        self.refresh(db);
    }

    pub fn exit(&self) {}

    pub fn input(&mut self, key: Key, db: &Database, colors: &Colors) -> Action {
        if self.is_empty {
            return Action::None;
        }

        match self.state {
            State::Search => match key.code {
                KeyCode::Enter => {
                    let input = self.search.as_str_trim();
                    if input.is_empty() {
                        return Action::None;
                    }

                    self.index = 0;
                    self.results.clear();
                    self.writer.clear();
                    self.query.clear();
                    self.query.push_str(input);

                    if let Err(err) = db.search(input, |id| self.results.push(id)) {
                        self.writer.write(format_args!(
                            "{}{err}{}",
                            Sgr::Fg(colors.red),
                            Sgr::reset_fg()
                        ));
                        return Action::Render;
                    }

                    if input.chars().count() < 3 {
                        self.writer.write(format_args!(
                            "{}Search query must be at least 3 characters{}",
                            Sgr::Fg(colors.primary),
                            Sgr::reset_fg()
                        ));
                        return Action::Render;
                    }

                    if self.results.is_empty() {
                        self.writer.write(format_args!(
                            "{}No cards found from query\n{}'{input}'{}",
                            Sgr::Fg(colors.neutral),
                            Sgr::Italic,
                            Sgr::Reset,
                        ));
                        return Action::Render;
                    }

                    self.highlight(self.results[0], db);
                    self.viewer.move_to_start();
                    self.state = State::Browse;
                    return Action::Render;
                }
                KeyCode::Down => {
                    if !self.results.is_empty() {
                        self.state = State::Browse;
                        return Action::Render;
                    }
                }
                KeyCode::Up => {}
                _ => {
                    if self.search.input(key) {
                        return Action::Render;
                    }
                }
            },
            State::Browse => match key.code {
                KeyCode::Up => {
                    if self.viewer.current_scroll() == 0 {
                        self.state = State::Search;
                        return Action::Render;
                    } else if self.viewer.move_up(1) {
                        return Action::Render;
                    }
                }
                KeyCode::Right => {
                    if self.results.len() > 1 {
                        self.index = (self.index + 1) % self.results.len();
                        self.highlight(self.current_card().unwrap(), db);
                        self.viewer.move_to_start();
                        return Action::Render;
                    }
                }
                KeyCode::Left => {
                    if self.results.len() > 1 {
                        if self.index == 0 {
                            self.index = self.results.len() - 1;
                        } else {
                            self.index -= 1;
                        }
                        self.highlight(self.current_card().unwrap(), db);
                        self.viewer.move_to_start();
                        return Action::Render;
                    }
                }
                KeyCode::Char('e') => {
                    if let Some(id) = self.current_card() {
                        return Action::Route(Route::Editor(Some(id)));
                    }
                }
                KeyCode::Char('g') => {
                    if let Some(id) = self.current_card() {
                        return Action::Route(Route::Cards(Some(CardsParam::Card(id))));
                    }
                }
                KeyCode::Char('s') => {
                    self.state = State::Search;
                    return Action::Render;
                }
                _ => {
                    if self.viewer.input(key.code) {
                        return Action::Render;
                    }
                }
            },
        }

        Action::None
    }

    pub fn render(&mut self, frame: &mut Framebuffer, colors: &Colors) {
        let area = frame.area().scale_and_center(0.7);
        let bg = colors.background;

        frame.fill(area, bg, false);

        let (prompt_area, results_area, shortcuts_area) = area.split_ends(1, 1);

        // Prompt
        let prompt_area = prompt_area.scale_cols_and_center(0.5);
        frame.cursor(prompt_area.pos);
        frame.print_ch_repeat(' ', prompt_area.cols());
        self.search
            .set_colors(colors.prompt())
            .set_disabled(matches!(self.state, State::Browse) || self.is_empty)
            .render(prompt_area.inner(Margin::horizontal(1)), frame);

        if self.is_empty {
            frame.print_fmt(Sgr::Fg(colors.neutral));
            Block::rectangle().render(results_area, frame);
            frame.push_str("No cards to search for");
            frame.render(
                results_area.inner(Margin::proportional(1)),
                TextOptions::paragraph_center(),
            );
            frame.print_fmt(Sgr::Reset);
            return;
        }

        // Results
        let border_color = match self.state {
            State::Search => colors.neutral,
            State::Browse => colors.secondary,
        };
        frame.print_fmt(Sgr::Fg(border_color));
        Block::rectangle().render(results_area, frame);
        frame.print_fmt(Sgr::reset_fg());

        if !self.results.is_empty() {
            frame.push_fmt_fg(
                format_args!(" {} / {} ", self.index + 1, self.results.len()),
                colors.neutral,
            );
            frame.render(results_area, TextOptions::span_center_top());
        }

        self.viewer.render(
            results_area.inner(Margin::proportional(1)),
            frame,
            self.writer.as_str(),
        );

        // todo
        // render_shortcuts(bottom, &mut frame, colors);
    }

    fn current_card(&self) -> Option<CardId> {
        self.results.get(self.index).copied()
    }

    fn highlight(&mut self, id: CardId, db: &Database) {
        db.search_highlight(id, &self.query, |content| {
            self.writer.clear();
            self.writer.push_str(content);
        })
        .unwrap();
    }

    fn refresh(&mut self, db: &Database) {
        if self.is_empty || self.query.is_empty() {
            self.results.clear();
            self.index = 0;
            return;
        }

        self.results.clear();
        let _ = db.search(self.query.as_str(), |id| self.results.push(id));
        self.index = self.index.min(self.results.len().saturating_sub(1));

        if let Some(id) = self.current_card() {
            self.highlight(id, db);
        }
    }
}
