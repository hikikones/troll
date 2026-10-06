use bevy_ecs::{
    resource::Resource,
    schedule::{IntoScheduleConfigs, SystemCondition},
    system::{NonSend, NonSendMut, Res, ResMut},
};
use bevy_state::condition::in_state;
use shared::symbols;
use terminal::*;
use widgets2::{Block, List, ListIndex, TableLayout};

use crate::{
    app::{Action, Actions, App, Colors, Frame, Input, InputState, PageArea, RenderSet},
    database::{AudioRating, Database, TrackId, TrackSort},
    jukebox::Jukebox,
    modals::Modal,
    pages::Page,
};

pub struct TracksPagePlugin;

impl TracksPagePlugin {
    pub fn build(app: &mut App) {
        app.insert_resource(TracksPage::default())
            .add_input(
                InputState::Normal,
                input_tracks.run_if(in_state(Page::Tracks)),
            )
            .add_input(
                InputState::Modal,
                input_tracks_modal.run_if(in_state(Page::Tracks).and_then(in_state(Modal::Custom))),
            )
            .add_enter(Page::Tracks, enter_tracks)
            .add_render(
                RenderSet::Page,
                render_tracks.run_if(in_state(Page::Tracks)),
            )
            .add_render(
                RenderSet::Modal,
                render_tracks_modal
                    .run_if(in_state(Page::Tracks).and_then(in_state(Modal::Custom))),
            );
    }
}

#[derive(Debug, Resource)]
pub struct TracksPage {
    list: List,
    reverse_sort: bool,
    keep_on_sort: bool,
    params: Option<TrackId>,
}

impl TracksPage {
    pub const fn set_params(&mut self, id: Option<TrackId>) {
        self.params = id;
    }
}

impl Default for TracksPage {
    fn default() -> Self {
        Self {
            list: List::new()
                .with_scrollbar(0)
                .with_padding(Margin::horizontal(1)),
            reverse_sort: false,
            keep_on_sort: false,
            params: None,
        }
    }
}

fn input_tracks(
    input: Res<Input>,
    mut db: NonSendMut<Database>,
    mut jb: NonSendMut<Jukebox>,
    mut page: ResMut<TracksPage>,
    mut actions: ResMut<Actions>,
) {
    let Some(key) = input.get() else {
        return;
    };

    match key.code {
        KeyCode::Enter => {
            if let Some(id) = db.get_id_from_index(page.list.index()) {
                jb.play_id(id, &db);
            }
        }
        KeyCode::Char(c) => match c {
            '0' | '1' | '2' | '3' | '4' | '5' => {
                let rating = AudioRating::from_char(c).unwrap();
                for i in page.list.selection_inclusive() {
                    if let Some(id) = db.get_id_from_index(i) {
                        db.write_rating(id, rating);
                    }
                }
            }
            'q' => {
                let ids = page
                    .list
                    .selection_inclusive()
                    .filter_map(|i| db.get_id_from_index(i));
                jb.extend(ids);
            }
            'n' => {
                for i in page.list.selection_inclusive().rev() {
                    if let Some(id) = db.get_id_from_index(i) {
                        jb.enqueue_next(id);
                    }
                }
            }
            's' | 'S' => {
                let id = db.get_id_from_index(page.list.index());

                if c == 's' {
                    let sort = db.get_sort().next();
                    db.sort(sort, page.reverse_sort);
                } else {
                    let sort = db.get_sort();
                    page.reverse_sort = !page.reverse_sort;
                    db.sort(sort, page.reverse_sort);
                }

                if page.keep_on_sort
                    && let Some(id) = id
                    && let Some(i) = db.get_index_from_id(id)
                {
                    page.list.set_index(i).set_selector(None);
                }

                actions.push(Action::Render);
            }
            'm' => {
                // TODO: remove
                actions.push(Action::Modal(Modal::Custom));
            }
            _ => {
                if page.list.input(key) {
                    actions.push(Action::Render);
                }
            }
        },
        _ => {
            if page.list.input(key) {
                actions.push(Action::Render);
            }
        }
    }
}

fn input_tracks_modal(input: Res<Input>, mut actions: ResMut<Actions>) {
    let Some(key) = input.get() else {
        return;
    };

    match key.code {
        KeyCode::Char('m') => {
            actions.push(Action::Modal(Modal::Canceled));
        }
        _ => {}
    }
}

fn enter_tracks(mut page: ResMut<TracksPage>, database: NonSend<Database>) {
    if let Some(id) = page.params.take()
        && let Some(index) = database.get_index_from_id(id)
    {
        page.list.set_index(index).set_selector(None);
    };
}

fn render_tracks(
    area: Res<PageArea>,
    mut frame: ResMut<Frame>,
    colors: Res<Colors>,
    database: NonSend<Database>,
    jukebox: NonSend<Jukebox>,
    mut page: ResMut<TracksPage>,
) {
    let area = **area;

    if database.is_empty() {
        frame.push_str_fg("No tracks to be found", colors.neutral);
        frame.render(area, TextOptions::span_center());
        return;
    }

    Block::rectangle(colors.secondary).render(area, &mut frame);
    frame.push_fmt_fg(
        format_args!(" All Tracks ({}) ", database.len()),
        colors.normal,
    );
    frame.render(area, TextOptions::span_center_top());

    let inner = area.inner(Margin::all(1));
    let gap = if inner.cols() < 10 { 0 } else { 2 };
    let sort = database.get_sort();
    let reverse = page.reverse_sort;
    let current = jukebox.current_track_id();

    page.list.render_table(
        inner,
        &mut *frame,
        database.iter(),
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
            frame.push_str(sort_symbol(sort, TrackSort::Rating, reverse));
            frame.render(rating_area, TextOptions::span());

            frame.print_fmt(Sgr::reset_fg());
        },
        |line, frame, areas, (id, track), idx| {
            let [title_area, artist_area, album_area, time_area, rating_area] = areas;

            let mut style = match idx {
                ListIndex::Selected => {
                    frame.fill(line, colors.primary);
                    Style::fg(colors.primary).with_reverse()
                }
                ListIndex::Selection => {
                    frame.fill(line, colors.neutral);
                    Style::fg(colors.neutral).with_reverse()
                }
                ListIndex::Normal => Style::fg(colors.normal),
            };

            if current == Some(id) {
                style.insert(Attributes::BOLD);
            }

            if jukebox.is_faulty(id) {
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

fn render_tracks_modal(mut frame: ResMut<Frame>, colors: Res<Colors>) {
    let area = frame.area();
    let area = area.with_size(area.size / 2).center(area);

    let bg = frame.palette().background().slight_offset().as_color();
    frame.fill(area, bg);

    Block::rectangle(Style::fg(colors.normal).with_bg(bg)).render(area, &mut *frame);
    frame.push_str(" Custom ");
    frame.render(area, TextOptions::span_center_top());

    frame.push_str("A custom modal only for the tracks page.");
    frame.render(
        area.inner(Margin::proportional(1)),
        TextOptions::paragraph_center(),
    );
}
