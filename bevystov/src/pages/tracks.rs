use ecs::*;
use terminal::*;
use widgets2::{Block, List, ListIndex, TableLayout, Text, TextOptions};

use crate::{
    app::{Action, Actions, App, Input, InputSet, PageArea, RenderSet, ShortcutsArea},
    database::{AudioRating, Database, TrackId, TrackSort},
    jukebox::Jukebox,
    modals::Modal,
    pages::Page,
    settings::{Colors, Settings},
};

pub struct TracksPagePlugin;

impl TracksPagePlugin {
    pub fn build(app: &mut App) {
        app.insert_resource(TracksPage::default())
            .add_input(InputSet::Page, input_tracks.run_if(in_state(Page::Tracks)))
            .add_input(
                InputSet::Modal,
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
    pub const fn set_params(&mut self, id: TrackId) {
        self.params = Some(id);
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
    mut database: NonSendMut<Database>,
    mut jukebox: NonSendMut<Jukebox>,
    mut page: ResMut<TracksPage>,
    mut actions: ResMut<Actions>,
) {
    let Some(key) = input.get() else {
        return;
    };

    match key.code {
        KeyCode::Enter => {
            if let Some(id) = database.get_id_from_index(page.list.index()) {
                jukebox.play_id(id, &database);
            }
        }
        KeyCode::Char(c) => match c {
            '0' | '1' | '2' | '3' | '4' | '5' => {
                let rating = AudioRating::from_char(c).unwrap();
                for i in page.list.selection_inclusive() {
                    if let Some(id) = database.get_id_from_index(i) {
                        database.write_rating(id, rating);
                    }
                }
            }
            'q' => {
                let ids = page
                    .list
                    .selection_inclusive()
                    .filter_map(|i| database.get_id_from_index(i));
                jukebox.extend(ids);
            }
            'n' => {
                for i in page.list.selection_inclusive().rev() {
                    if let Some(id) = database.get_id_from_index(i) {
                        jukebox.enqueue_next(id);
                    }
                }
            }
            's' | 'S' => {
                let id = database.get_id_from_index(page.list.index());

                if c == 's' {
                    let sort = database.get_sort().next();
                    database.sort(sort, page.reverse_sort);
                } else {
                    let sort = database.get_sort();
                    page.reverse_sort = !page.reverse_sort;
                    database.sort(sort, page.reverse_sort);
                }

                if page.keep_on_sort
                    && let Some(id) = id
                    && let Some(i) = database.get_index_from_id(id)
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

fn enter_tracks(mut page: ResMut<TracksPage>, database: NonSend<Database>) {
    if let Some(id) = page.params.take()
        && let Some(index) = database.get_index_from_id(id)
    {
        page.list.set_index(index).set_selector(None);
    };
}

fn render_tracks(
    area: Res<PageArea>,
    shortcuts_area: Res<ShortcutsArea>,
    mut frame: ResMut<Framebuffer>,
    mut text: ResMut<Text>,
    settings: Res<Settings>,
    database: NonSend<Database>,
    jukebox: NonSend<Jukebox>,
    mut page: ResMut<TracksPage>,
) {
    let area = **area;
    let colors = settings.colors();

    if database.is_empty() {
        frame.print_fmt(Sgr::Fg(colors.neutral));
        frame.print_span_with(area, "No tracks to be found", SpanOptions::center());
        frame.print_fmt(Sgr::reset_fg());
        return;
    }

    frame.print_fmt(Sgr::Fg(colors.secondary));
    Block::rectangle().render(area, &mut frame);
    text.push_fmt_fg(
        format_args!(" All Tracks ({}) ", database.len()),
        colors.normal,
    );
    text.render(area, &mut frame, TextOptions::span_center_top());
    frame.print_fmt(Sgr::reset_fg());

    let inner = area.inner(Margin::all(1));
    let gap = if inner.cols() < 10 { 0 } else { 2 };
    let sort = database.get_sort();
    let reverse = page.reverse_sort;
    let current = jukebox.current_track_id();

    page.list.set_colors(colors.list()).render_table(
        inner,
        &mut frame,
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
                    if reverse { "⌃" } else { "⌄" }
                } else {
                    ""
                }
            }

            frame.print_fmt(Sgr::Fg(colors.normal));

            text.push_str("Title");
            text.push_str(sort_symbol(sort, TrackSort::Title, reverse));
            text.render(title_area, frame, TextOptions::span());
            text.push_str("Artist");
            text.push_str(sort_symbol(sort, TrackSort::Artist, reverse));
            text.render(artist_area, frame, TextOptions::span());
            text.push_str("Album");
            text.push_str(sort_symbol(sort, TrackSort::Album, reverse));
            text.render(album_area, frame, TextOptions::span());
            text.push_str("Time");
            text.push_str(sort_symbol(sort, TrackSort::Time, reverse));
            text.render(time_area, frame, TextOptions::span());
            text.push_str("Rating");
            text.push_str(sort_symbol(sort, TrackSort::Rating, reverse));
            text.render(rating_area, frame, TextOptions::span());

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

    render_shortcuts(**shortcuts_area, &mut frame, &mut text, &colors);
}

fn render_shortcuts(area: Rect, frame: &mut Framebuffer, text: &mut Text, colors: &Colors) {
    let key_color = colors.secondary;
    let name_color = colors.normal;
    let gap = 1;

    for (key, name, gap) in [
        ("↵", "Play", gap),
        ("q", "Enqueue", gap),
        ("n", "Play next", gap),
        ("0-5", "Rating", gap),
        ("(⇧)s", "Sort", 0),
    ] {
        text.push_fmt(Sgr::Fg(key_color));
        text.push_str(key);
        text.push_fmt(Sgr::Fg(name_color));
        text.push_ch(' ');
        text.push_str(name);
        text.push_ch_repeat(' ', gap);
    }

    text.push_fmt(Sgr::reset_fg());
    text.render(area, frame, TextOptions::span_center_top());
}

fn input_tracks_modal(input: Res<Input>, mut actions: ResMut<Actions>) {
    let Some(key) = input.get() else {
        return;
    };

    match key.code {
        KeyCode::Char('m') => {
            actions.push(Action::Modal(Modal::None));
        }
        _ => {}
    }
}

// TODO: remove
fn render_tracks_modal(
    mut frame: ResMut<Framebuffer>,
    mut text: ResMut<Text>,
    settings: Res<Settings>,
) {
    let area = frame.area();
    let area = area.with_size(area.size / 2).center(area);
    let colors = settings.colors();

    frame.fill(area, colors.background, false);

    frame.print_fmt(Sgr::Fg(colors.normal));
    Block::rectangle().render(area, &mut frame);
    frame.print_span_with(area, " Custom ", SpanOptions::center_top());

    text.push_str("A custom modal only for the tracks page.");
    text.render(
        area.inner(Margin::proportional(1)),
        &mut frame,
        TextOptions::paragraph_center(),
    );
    frame.print_fmt(Sgr::Reset);
}
