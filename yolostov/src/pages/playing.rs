use shared::symbols;
use terminal::*;
use widgets2::{Block, KittyGraphics, List, ListIndex, ScrollMargins};

use crate::{
    app::{Action, Colors},
    database::Database,
    jukebox::Jukebox,
    modals::ModalAction,
};

pub struct PlayingPage {
    list: List,
    current_qi: Option<usize>,
}

impl PlayingPage {
    pub const fn new() -> Self {
        Self {
            list: List::new()
                .with_scrollbar(0)
                .with_padding(Margin::horizontal(1)),
            current_qi: None,
        }
    }

    pub fn on_enter(&self) {}

    pub fn on_exit(&self) {}

    pub fn on_update(&self) {}

    pub fn render(
        &mut self,
        area: Rect,
        frame: &mut Framebuffer,
        colors: &Colors,
        db: &Database,
        jb: &Jukebox,
        kitty: &KittyGraphics,
    ) {
        self.update_scroll_on_new_track(jb);

        let (left, right) = area.split_vertically_with_gap(0);
        self.render_cover(left, frame, colors);
        self.render_queue(right, frame, colors, db, jb);
    }

    fn render_cover(&self, area: Rect, frame: &mut Framebuffer, colors: &Colors) {
        frame.push_str_fg("TODO", colors.neutral);
        frame.render(area, TextOptions::span_center());
    }

    fn render_queue(
        &mut self,
        area: Rect,
        frame: &mut Framebuffer,
        colors: &Colors,
        db: &Database,
        jb: &Jukebox,
    ) {
        Block::rectangle()
            .with_color(colors.secondary)
            .render(area, frame);
        frame.push_fmt_fg(
            format_args!(" History ({}) / Queue ({}) ", jb.history(), jb.queue()),
            colors.normal,
        );
        frame.render(area, TextOptions::span_center_top());

        if jb.is_empty() {
            frame.push_str_fg("No tracks in the queue", colors.neutral);
            frame.render(
                area.inner(Margin::proportional(1)),
                TextOptions::span_center(),
            );
            return;
        }

        let inner = area.inner(Margin::all(1));
        let scrolloff = inner.rows() / 2;
        self.list.set_scrolloff(ScrollMargins::all(scrolloff));

        let hlen = jb.history();
        let current_qi = jb.current_queue_index();

        self.list
            .render(inner, frame, jb.iter(), |line, frame, (id, qi), index| {
                let Some(track) = db.get(id) else {
                    return;
                };

                let fg = if qi < hlen {
                    colors.neutral
                } else if current_qi == Some(qi) {
                    colors.primary
                } else {
                    colors.normal
                };

                let symbol = match index {
                    ListIndex::Selected => symbols::concat!(symbols::SELECTED, " "),
                    ListIndex::Selection => symbols::concat!(symbols::SELECTION, " "),
                    ListIndex::Normal => "",
                };

                frame.push_fmt(Sgr::Fg(fg));
                frame.push_str(symbol);

                if jb.is_faulty(id) {
                    frame.push_fmt(Sgr::CrossedOut);
                }

                frame.push_fmt(track.title());
                frame.push_ch(' ');
                frame.push_fmt(track.artist());
                frame.push_ch(' ');
                frame.push_fmt(track.album());
                frame.push_ch(' ');
                frame.push_fmt(Sgr::Reset);
                frame.render(line, TextOptions::span());
            });
    }

    pub fn input(&mut self, key: Key, db: &mut Database, jb: &mut Jukebox) -> Action {
        match key.code {
            KeyCode::Enter => {
                let index = self.list.index();
                jb.play_index(index, db);
            }
            _ => {
                if self.list.input(key) {
                    return Action::Render;
                }
            }
        }
        Action::None
    }

    pub fn render_modal(&self, area: Rect, frame: &mut Framebuffer, colors: &Colors) {
        //todo
    }

    pub fn input_modal(&self, key: Key) -> ModalAction {
        //todo
        ModalAction::None
    }

    fn update_scroll_on_new_track(&mut self, jb: &Jukebox) {
        let current_queue_index = jb.current_queue_index();

        if self.current_qi == current_queue_index {
            return;
        }

        self.current_qi = current_queue_index;

        if let Some(idx) = current_queue_index {
            self.list.set_index(idx).set_selector(None);
        }
    }
}
