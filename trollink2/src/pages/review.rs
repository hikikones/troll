use terminal::*;
use widgets2::{Markup, MarkupScroll};

use crate::{
    app::Action,
    database::{CardId, Database},
    pages::Route,
    settings::Colors,
};

pub struct ReviewPage {
    total: u32,
    progress: u32,
    state: ReviewState,
    review: ReviewCard,
}

enum ReviewState {
    None,
    Review(CardId),
    Done,
}

impl ReviewPage {
    pub const fn new() -> Self {
        Self {
            total: 0,
            progress: 0,
            state: ReviewState::None,
            review: ReviewCard::new(),
        }
    }

    pub fn enter(&mut self, db: &Database, markup: &mut Markup) {
        self.total = db.get_due_count();

        if self.total > 0 {
            self.next_card(db, markup);
        }
    }

    pub fn exit(&mut self) {
        self.total = 0;
        self.progress = 0;
        self.state = ReviewState::None;
        self.review.clear();
    }

    pub fn refresh(&self) {}

    pub fn input(
        &mut self,
        key: Key,
        db: &mut Database,
        markup: &mut Markup,
        desired_retention: f32,
    ) -> Action {
        match self.state {
            ReviewState::Review(id) => match key.code {
                KeyCode::Char('e') => return Action::Route(Route::Editor(Some(id))),
                KeyCode::Delete => {
                    db.delete_card(id);
                    self.total = self.total.saturating_sub(1);
                    self.next_card(db, markup);
                    return Action::Render;
                }
                KeyCode::Char(' ') => {
                    if !self.review.is_fully_revealed() {
                        self.review.reveal_more();
                        markup.set_desired_scroll(MarkupScroll::End);
                        return Action::Render;
                    }
                }
                KeyCode::Char('y' | 'n') => {
                    if self.review.is_fully_revealed() {
                        let success = key.code == KeyCode::Char('y');
                        db.review_card(id, success, desired_retention);
                        self.progress += 1;
                        if self.is_done() {
                            self.state = ReviewState::Done;
                        } else {
                            self.next_card(db, markup);
                        }
                        return Action::Render;
                    }
                }
                KeyCode::Right => {
                    if self.has_more_cards() {
                        self.next_card(db, markup);
                        return Action::Render;
                    }
                }
                _ => {
                    if markup.input(key.code) {
                        return Action::Render;
                    }
                }
            },
            ReviewState::None | ReviewState::Done => {}
        }

        Action::None
    }

    pub fn render(
        &self,
        mut area: Rect,
        frame: &mut Framebuffer,
        colors: &Colors,
        markup: &mut Markup,
    ) {
        match self.state {
            ReviewState::None => {
                frame.print_span_with(area, "No cards to review", SpanOptions::center());
            }
            ReviewState::Review(_id) => {
                frame.push_fmt_fg(
                    format_args!("{} / {}", self.progress + 1, self.total),
                    colors.neutral,
                );
                frame.render(area, TextOptions::span_center_top());

                area.shrink_down(2);

                markup.render(area, frame, self.review.content());
            }
            ReviewState::Done => {
                frame.print_span_with(area, "Good job!", SpanOptions::center());
            }
        }
    }

    pub fn render_shortcuts(&self, area: Rect, frame: &mut Framebuffer, colors: &Colors) {
        if let ReviewState::Done = self.state {
            return;
        }

        let gap = 1;

        for (key, name, gap, _show) in [
            ("y", "Yes", gap, self.review.is_fully_revealed()),
            ("n", "No", gap, self.review.is_fully_revealed()),
            ("Space", "Show", gap, !self.review.is_fully_revealed()),
            ("➔", "Skip", gap, self.has_more_cards()),
            ("e", "Edit", gap, true),
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

    fn next_card(&mut self, db: &Database, markup: &mut Markup) {
        let next_card = if let ReviewState::Review(id) = self.state {
            db.get_due_card_random_except(id)
        } else {
            db.get_due_card_random()
        };

        match next_card {
            Some(id) => {
                db.get_card_content(id, |content| {
                    self.review.start(content);
                });
                self.state = ReviewState::Review(id);
                markup.scroll(MarkupScroll::Start);
            }
            None => {
                self.state = ReviewState::Done;
            }
        }
    }

    const fn is_done(&self) -> bool {
        self.progress == self.total
    }

    const fn has_more_cards(&self) -> bool {
        (self.total - self.progress) > 1
    }
}

struct ReviewCard {
    content: String,
    reveals: Vec<usize>,
    reveal_len: usize,
}

impl ReviewCard {
    const fn new() -> Self {
        Self {
            content: String::new(),
            reveals: Vec::new(),
            reveal_len: 0,
        }
    }

    fn content(&self) -> &str {
        &self.content[..self.reveal_len]
    }

    fn start(&mut self, content: &str) {
        self.clear();

        self.content.push_str(content);
        self.reveals.extend(
            widgets2::MarkupBlockParser::new(content)
                .filter(|(b, _)| matches!(b, widgets2::MarkupBlock::Break))
                .map(|(_, range)| range.start),
        );
        self.reveals.reverse();
        self.reveal_more();
    }

    fn reveal_more(&mut self) {
        self.reveal_len = self.reveals.pop().unwrap_or(self.content.len());
    }

    const fn is_fully_revealed(&self) -> bool {
        self.reveal_len == self.content.len()
    }

    fn clear(&mut self) {
        self.content.clear();
        self.reveals.clear();
        self.reveal_len = 0;
    }
}
