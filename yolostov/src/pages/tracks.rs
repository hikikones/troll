use shared::symbols;
use terminal::*;
use widgets2::{Block, List, ListIndex, TableLayout};

use crate::{
    app::{Action, Colors},
    database::{AudioRating, Database, TrackId, TrackSort},
    jukebox::Jukebox,
    modals::ModalAction,
};

pub struct TracksPage {
    list: List,
    reverse_sort: bool,
    keep_on_sort: bool,
}

impl TracksPage {
    pub const fn new() -> Self {
        Self {
            list: List::new()
                .with_scrollbar(0)
                .with_padding(Margin::horizontal(1)),
            reverse_sort: false,
            keep_on_sort: false,
        }
    }

    pub fn on_enter(&mut self, id: Option<TrackId>, db: &Database) {
        if let Some(id) = id
            && let Some(index) = db.get_index_from_id(id)
        {
            self.list.set_index(index).set_selector(None);
        };
    }

    pub fn on_exit(&self) {}

    pub fn update(&self) {}

    pub fn refresh(&self) {}

    pub fn render(
        &mut self,
        area: Rect,
        frame: &mut Framebuffer,
        colors: &Colors,
        db: &Database,
        jb: &Jukebox,
    ) {
        if db.is_empty() {
            frame.push_str_fg("No tracks to be found", colors.neutral);
            frame.render(area, TextOptions::span_center());
            return;
        }

        frame.print_fmt(Sgr::Fg(colors.secondary));
        Block::rectangle().render(area, frame);
        frame.push_fmt_fg(format_args!(" All Tracks ({}) ", db.len()), colors.normal);
        frame.render(area, TextOptions::span_center_top());

        let inner = area.inner(Margin::all(1));
        let gap = if inner.cols() < 10 { 0 } else { 2 };
        let sort = db.get_sort();
        let reverse = self.reverse_sort;
        let current = jb.current_track_id();

        self.list.render_table(
            inner,
            frame,
            db.iter(),
            TableLayout::new(
                gap,
                [
                    Constraint::Percent(35),
                    Constraint::Percent(30),
                    Constraint::Percent(35),
                    Constraint::Fixed(5),
                    Constraint::Fixed(7),
                ],
            ),
            |_line, frame, areas| {
                let [title_area, artist_area, album_area, time_area, rating_area] = areas;

                const fn sort_symbol<'a>(
                    current_sort: TrackSort,
                    sort: TrackSort,
                    reverse: bool,
                ) -> &'a str {
                    if current_sort.equals(sort) {
                        if reverse {
                            symbols::ARROW_HEAD_UP
                        } else {
                            symbols::ARROW_HEAD_DOWN
                        }
                    } else {
                        ""
                    }
                }

                frame.print_fmt(Sgr::Fg(colors.normal));

                frame.push_str("Title");
                frame.push_str(sort_symbol(sort, TrackSort::Title, reverse));
                frame.render(title_area, TextOptions::span());
                frame.push_str("Artist");
                frame.push_str(sort_symbol(sort, TrackSort::Artist, reverse));
                frame.render(artist_area, TextOptions::span());
                frame.push_str("Album");
                frame.push_str(sort_symbol(sort, TrackSort::Album, reverse));
                frame.render(album_area, TextOptions::span());
                frame.push_str("Time");
                frame.push_str(sort_symbol(sort, TrackSort::Time, reverse));
                frame.render(time_area, TextOptions::span());
                frame.push_str("Rating");
                frame.push_str(sort_symbol(sort, TrackSort::Time, reverse));
                frame.render(rating_area, TextOptions::span());

                frame.print_fmt(Sgr::reset_fg());
            },
            |line, frame, areas, (id, track), idx| {
                let [title_area, artist_area, album_area, time_area, rating_area] = areas;

                let mut style = match idx {
                    ListIndex::Selected => {
                        frame.fill(line, colors.primary, true);
                        Style::fg(colors.primary).with_reverse()
                    }
                    ListIndex::Selection => {
                        frame.fill(line, colors.neutral, true);
                        Style::fg(colors.neutral).with_reverse()
                    }
                    ListIndex::Normal => Style::fg(colors.normal),
                };

                if current == Some(id) {
                    style.insert(Attributes::BOLD);
                }

                if jb.is_faulty(id) {
                    style.insert(Attributes::CROSSED_OUT);
                }

                frame.print_fmt(style);
                frame.print_span(title_area, track.title());
                frame.print_span(artist_area, track.artist());
                frame.print_span(album_area, track.album());
                frame.print_span(time_area, track.duration_display());
                frame.print_span(rating_area, track.rating().stars());
                frame.print_fmt(Sgr::Reset);
            },
        );
    }

    pub fn input(&mut self, key: Key, db: &mut Database, jb: &mut Jukebox) -> Action {
        match key.code {
            KeyCode::Enter => {
                if let Some(id) = db.get_id_from_index(self.list.index()) {
                    jb.play_id(id, db);
                }
            }
            KeyCode::Char(c) => match c {
                '0' | '1' | '2' | '3' | '4' | '5' => {
                    let rating = AudioRating::from_char(c).unwrap();
                    for i in self.list.selection_inclusive() {
                        if let Some(id) = db.get_id_from_index(i) {
                            db.write_rating(id, rating);
                        }
                    }
                }
                'q' => {
                    let ids = self
                        .list
                        .selection_inclusive()
                        .filter_map(|i| db.get_id_from_index(i));
                    jb.extend(ids);
                }
                'n' => {
                    for i in self.list.selection_inclusive().rev() {
                        if let Some(id) = db.get_id_from_index(i) {
                            jb.enqueue_next(id);
                        }
                    }
                }
                's' | 'S' => {
                    let id = db.get_id_from_index(self.list.index());

                    if c == 's' {
                        db.sort(db.get_sort().next(), self.reverse_sort);
                    } else {
                        self.reverse_sort = !self.reverse_sort;
                        db.sort(db.get_sort(), self.reverse_sort);
                    }

                    if self.keep_on_sort
                        && let Some(id) = id
                        && let Some(i) = db.get_index_from_id(id)
                    {
                        self.list.set_index(i).set_selector(None);
                    }
                    return Action::Render;
                }
                _ => {
                    if self.list.input(key) {
                        return Action::Render;
                    }
                }
            },
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
}
