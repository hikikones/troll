use shared::sqlite::SqliteError;
use terminal::*;
use widgets::{AnsiViewer, Block, Prompt};

use crate::{
    app::Action,
    database::{CardId, Database},
    pages::{CardsParam, Route},
    settings::Colors,
};

pub struct SearchModal {
    state: State,
    prompt: Prompt,
    cards: Vec<CardId>,
    ansi: AnsiViewer,
    message: Option<Message>,
    no_cards: bool,
    index: usize,
}

enum State {
    Search,
    Browse,
}

enum Message {
    Help,
    MoreChars,
    NoCardsFound,
    Error(SqliteError),
}

impl SearchModal {
    pub const fn new() -> Self {
        Self {
            state: State::Search,
            prompt: Prompt::new().with_placeholder("Search..."),
            cards: Vec::new(),
            ansi: AnsiViewer::new(),
            message: None,
            no_cards: false,
            index: 0,
        }
    }

    pub fn enter(&mut self, db: &Database) {
        self.message = Some(Message::Help);
        self.no_cards = db.is_cards_empty();
    }

    pub fn exit(&mut self) {
        self.state = State::Search;
        self.prompt.clear();
        self.cards.clear();
        self.ansi.clear();
        self.message = None;
        self.index = 0;
    }

    pub fn input(&mut self, key: Key, db: &Database) -> Action {
        if self.no_cards {
            return Action::None;
        }

        match self.state {
            State::Search => match key.code {
                KeyCode::Enter => {
                    self.index = 0;
                    self.message = None;
                    self.cards.clear();

                    let prompt = self.prompt.as_str_trim();

                    if prompt.is_empty() {
                        self.message = Some(Message::Help);
                        return Action::Render;
                    }

                    if prompt.chars().count() < 3 {
                        self.message = Some(Message::MoreChars);
                        return Action::Render;
                    }

                    if let Err(err) = db.search(prompt, |id| self.cards.push(id)) {
                        self.message = Some(Message::Error(err));
                        return Action::Render;
                    }

                    if self.cards.is_empty() {
                        self.message = Some(Message::NoCardsFound);
                        return Action::Render;
                    }

                    self.highlight(self.cards[0], db);
                    self.ansi.move_to_start();
                    self.state = State::Browse;
                    return Action::Render;
                }
                KeyCode::Down => {
                    if !self.cards.is_empty() {
                        self.state = State::Browse;
                        return Action::Render;
                    }
                }
                KeyCode::Up => {}
                _ => {
                    if self.prompt.input(key) {
                        return Action::Render;
                    }
                }
            },
            State::Browse => match key.code {
                KeyCode::Up => {
                    if self.ansi.current_scroll() == 0 {
                        self.state = State::Search;
                        return Action::Render;
                    } else if self.ansi.move_up(1) {
                        return Action::Render;
                    }
                }
                KeyCode::Right => {
                    if self.cards.len() > 1 {
                        self.index = (self.index + 1) % self.cards.len();
                        self.highlight(self.current_card().unwrap(), db);
                        self.ansi.move_to_start();
                        return Action::Render;
                    }
                }
                KeyCode::Left => {
                    if self.cards.len() > 1 {
                        if self.index == 0 {
                            self.index = self.cards.len() - 1;
                        } else {
                            self.index -= 1;
                        }
                        self.highlight(self.current_card().unwrap(), db);
                        self.ansi.move_to_start();
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
                    if self.ansi.input(key.code) {
                        return Action::Render;
                    }
                }
            },
        }

        Action::None
    }

    pub fn render(&mut self, frame: &mut Frame, colors: &Colors) {
        let area = frame
            .area()
            .scale(0.8)
            .with_size_min(Size::LARGE)
            .center(frame.area());

        frame.fill(area, colors.background, false);

        frame.print_fmt(Sgr::Fg(colors.secondary));
        Block::rectangle().render(area, frame);
        frame.print_fmt(Sgr::reset_fg());

        let (top, body, bottom) = area.inner(Margin::horizontal(2)).split_ends(1, 1);

        // Prompt
        match self.state {
            State::Search => {
                let prompt_area = top.scale_cols_and_center(0.5);
                frame.cursor(prompt_area.pos);
                frame.print_ch_repeat(' ', prompt_area.cols());
                self.prompt
                    .set_colors(colors.prompt())
                    .set_disabled(self.no_cards)
                    .render(prompt_area.inner(Margin::horizontal(1)), frame);
            }
            State::Browse => {
                frame.push_fmt_fg(
                    format_args!(" {} / {} ", self.index + 1, self.cards.len()),
                    colors.normal,
                );
                frame.render(top, TextOptions::span_center_top());
            }
        }

        // Results
        if self.no_cards {
            frame.print_fmt(Sgr::Fg(colors.neutral));
            frame.push_str("No cards to search for");
            frame.render(body, TextOptions::paragraph_center());
            frame.print_fmt(Sgr::Reset);
            return;
        }

        match self.message.as_ref() {
            Some(msg) => {
                match msg {
                    Message::Help => {
                        frame.push_str_fg(USAGE, colors.neutral);
                    }
                    Message::MoreChars => {
                        frame.push_str_fg(
                            "Search query must be at least 3 characters",
                            colors.neutral,
                        );
                    }
                    Message::NoCardsFound => {
                        frame.push_str_fg("No cards found from search query", colors.neutral);
                    }
                    Message::Error(error) => {
                        frame.push_str("Failed to search:\n\n");
                        frame.push_fmt_fg(error, colors.red);
                    }
                }
                frame.render(body, TextOptions::paragraph_center());
            }
            None => {
                self.ansi.set_colors(colors.ansi()).render(body, frame);
            }
        }

        self.render_shortcuts(bottom, frame, colors);

        frame.print_fmt(Sgr::Reset);
    }

    fn render_shortcuts(&self, area: Rect, frame: &mut Frame, colors: &Colors) {
        let key_color = colors.secondary;
        let name_color = colors.normal;

        match self.state {
            State::Search => {
                frame.push_fmt(format_args!(
                    " {}↵{} Confirm ",
                    Sgr::Fg(key_color),
                    Sgr::Fg(name_color),
                ));
                frame.render(area, TextOptions::span_center_top());
            }
            State::Browse => {
                let gap = 1;

                frame.push_ch(' ');

                for (key, name, gap) in [("e", "Edit", gap), ("g", "Goto", gap), ("s", "Search", 0)]
                {
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
        }
    }

    fn current_card(&self) -> Option<CardId> {
        self.cards.get(self.index).copied()
    }

    fn highlight(&mut self, id: CardId, db: &Database) {
        db.search_highlight(id, self.prompt.as_str_trim(), |content| {
            self.ansi.clear();
            self.ansi.push_str(content);
        })
        .unwrap();
    }
}

const USAGE: &str = "Usage

Search for words and phrases across your cards. \
The word or phrase in a query must be at least 3 characters long.

Enclose your query in double quotes for exact matches, \
and exclude a word with NOT such as \"apple NOT green\".

The available operators are AND, NOT, OR, * (prefix) and ^ (start-of-column).

Special symbols like \"#,:%.;\" must also be enclosed in double quotes. \
If something is wrong with the query, an error will be displayed.";
