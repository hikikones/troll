use ecs::*;
use terminal::*;
use widgets2::{Block, list2::List};

use crate::{
    app::{Action, Actions, App, Input, InputSet, PageArea, RenderSet, ShortcutsArea},
    database::{Database, TrackId, TrackRating, TrackSort},
    jukebox::Jukebox,
    pages::Page,
    settings::{Colors, Settings},
};

pub struct TracksPagePlugin;

impl TracksPagePlugin {
    pub fn build(app: &mut App) {
        app.insert_resource(TracksParam::default())
            .insert_resource(TracksList::default())
            .insert_resource(TracksSorted::default())
            .add_input(InputSet::Page, input_tracks.run_if(in_state(Page::Tracks)))
            .add_enter(Page::Tracks, enter_tracks)
            .add_render(
                RenderSet::Page,
                render_tracks.run_if(in_state(Page::Tracks)),
            );
    }
}

#[derive(Debug, Default, Resource)]
pub struct TracksParam(Option<TrackId>);

impl TracksParam {
    pub const fn set(&mut self, id: TrackId) {
        self.0 = Some(id);
    }
}

#[derive(Debug, Resource, Deref, DerefMut)]
struct TracksList(List);

impl Default for TracksList {
    fn default() -> Self {
        Self(
            List::new()
                .with_scrollbar(0)
                .with_padding(Margin::horizontal(1).with_top(1)),
        )
    }
}

#[derive(Debug, Default, Resource, Deref)]
pub struct TracksSorted(Vec<TrackId>);

impl TracksSorted {
    fn id(&self, index: usize) -> TrackId {
        self[index]
    }

    fn index(&self, id: TrackId) -> usize {
        self.iter()
            .enumerate()
            .find(|(_, tid)| id == **tid)
            .map(|(i, _)| i)
            .unwrap()
    }

    pub fn push(&mut self, id: TrackId) {
        self.0.push(id);
    }

    pub fn sort(&mut self, database: &mut Database) {
        self.0.clear();
        self.0.extend(database.tracks_sort());
    }
}

struct TableAreas([Rect; 5]);

impl TableAreas {
    const fn new(gap: u16, line: Rect) -> Self {
        Self(line.split_horizontal(
            gap,
            [
                Constraint::Percent(35),
                Constraint::Percent(30),
                Constraint::Percent(35),
                Constraint::Fixed(5),
                Constraint::Fixed(7),
            ],
        ))
    }

    const fn title(&self) -> Rect {
        self.0[0]
    }

    const fn artist(&self) -> Rect {
        self.0[1]
    }

    const fn album(&self) -> Rect {
        self.0[2]
    }

    const fn time(&self) -> Rect {
        self.0[3]
    }

    const fn rating(&self) -> Rect {
        self.0[4]
    }

    const fn set_row(&mut self, row: u16) {
        let mut i = 0;
        while i < self.0.len() {
            self.0[i].set_row(row);
            i += 1;
        }
    }
}

fn enter_tracks(
    mut param: ResMut<TracksParam>,
    mut list: ResMut<TracksList>,
    tracks: Res<TracksSorted>,
) {
    let Some(id) = param.0.take() else {
        return;
    };

    let index = tracks.index(id);
    list.set_index(index).set_selector(None);
}

fn input_tracks(
    input: Res<Input>,
    mut tracks: ResMut<TracksSorted>,
    mut database: NonSendMut<Database>,
    mut jukebox: NonSendMut<Jukebox>,
    mut list: ResMut<TracksList>,
    mut actions: ResMut<Actions>,
    settings: Res<Settings>,
) {
    let Some(key) = input.get() else {
        return;
    };

    match key.code {
        KeyCode::Enter => {
            let id = tracks.id(list.index());
            jukebox.play_id(id, &database);
        }
        KeyCode::Char(c) => match c {
            '0' | '1' | '2' | '3' | '4' | '5' => {
                let rating = TrackRating::from_char(c).unwrap();
                let mut render = false;
                for i in list.selection_inclusive() {
                    let id = tracks.id(i);
                    if database.write_rating(id, rating) {
                        render = true;
                    }
                }

                if render {
                    actions.push(Action::Render);
                }
            }
            'q' => {
                jukebox.extend(list.selection_inclusive().map(|i| tracks.id(i)));
            }
            'n' => {
                for i in list.selection_inclusive().rev() {
                    let id = tracks.id(i);
                    jukebox.add_next(id);
                }
            }
            's' => {
                if settings.keep_selected() {
                    let id = tracks.id(list.index());
                    database.next_sort();
                    tracks.sort(&mut database);
                    list.set_index(tracks.index(id));
                } else {
                    database.next_sort();
                    tracks.sort(&mut database);
                }
                actions.push(Action::Render);
            }
            'S' => {
                if settings.keep_selected() {
                    let id = tracks.id(list.index());
                    database.toggle_reverse_sort();
                    tracks.0.reverse();
                    list.set_index(tracks.index(id));
                } else {
                    database.toggle_reverse_sort();
                    tracks.0.reverse();
                }
                actions.push(Action::Render);
            }
            _ => {
                if list.input(key) {
                    actions.push(Action::Render);
                }
            }
        },
        _ => {
            if list.input(key) {
                actions.push(Action::Render);
            }
        }
    }
}

fn render_tracks(
    area: Res<PageArea>,
    shortcuts_area: Res<ShortcutsArea>,
    mut frame: ResMut<Framebuffer>,
    settings: Res<Settings>,
    mut list: ResMut<TracksList>,
    tracks: Res<TracksSorted>,
    mut database: NonSendMut<Database>,
    jukebox: NonSend<Jukebox>,
) {
    let area = **area;
    let colors = settings.colors();

    if tracks.is_empty() {
        frame.print_fmt(Sgr::Fg(colors.neutral));
        frame.print_span_with(area, "No tracks to be found", SpanOptions::center());
        frame.print_fmt(Sgr::reset_fg());
        return;
    }

    // Block
    frame.print_fmt(Sgr::Fg(colors.secondary));
    Block::rectangle().render(area, &mut frame);
    frame.push_fmt_fg(
        format_args!(" All Tracks ({}) ", tracks.len()),
        colors.normal,
    );
    frame.render(area, TextOptions::span_center_top());
    frame.print_fmt(Sgr::reset_fg());

    render_table(
        area.inner(Margin::all(1)),
        &mut frame,
        colors,
        &mut list,
        &tracks,
        &mut database,
        &jukebox,
    );

    render_shortcuts(**shortcuts_area, &mut frame, &colors);
}

fn render_table(
    area: Rect,
    frame: &mut Framebuffer,
    colors: &Colors,
    list: &mut List,
    tracks: &TracksSorted,
    database: &mut Database,
    jukebox: &Jukebox,
) {
    let Some((table_area, lines)) = list
        .set_len(tracks.len())
        .set_colors(colors.list())
        .render(area, frame)
    else {
        return;
    };

    let gap = if area.cols() < 10 { 0 } else { 2 };
    let mut areas = TableAreas::new(gap, table_area.with_rows(1));

    // Header
    const fn sort_symbol<'a>(current_sort: TrackSort, sort: TrackSort, reverse: bool) -> &'a str {
        if current_sort.equals(sort) {
            if reverse { "⌃" } else { "⌄" }
        } else {
            ""
        }
    }

    let sort = database.get_sort();
    let reverse = database.get_reverse_sort();

    areas.set_row(area.row());

    frame.print_fmt(Sgr::Fg(colors.normal));

    frame.push_str("Title");
    frame.push_str(sort_symbol(sort, TrackSort::Title, reverse));
    frame.render(areas.title(), TextOptions::span());
    frame.push_str("Artist");
    frame.push_str(sort_symbol(sort, TrackSort::Artist, reverse));
    frame.render(areas.artist(), TextOptions::span());
    frame.push_str("Album");
    frame.push_str(sort_symbol(sort, TrackSort::Album, reverse));
    frame.render(areas.album(), TextOptions::span());
    frame.push_str("Time");
    frame.push_str(sort_symbol(sort, TrackSort::Time, reverse));
    frame.render(areas.time(), TextOptions::span());
    frame.push_str("Rating");
    frame.push_str(sort_symbol(sort, TrackSort::Rating, reverse));
    frame.render(areas.rating(), TextOptions::span());

    frame.print_fmt(Sgr::reset_fg());

    // List
    let line = table_area.with_rows(1);
    let selected = list.index();
    let selection = list.selection_inclusive();
    let current_track = jukebox.track_id();
    let mut row = table_area.pos.row;

    for (index, id) in lines.zip(tracks.iter().copied().skip(list.scroll())) {
        let mut style = if index == selected {
            frame.fill(line.with_row(row), colors.primary, true);
            Style::fg(colors.primary).with_reverse()
        } else if selection.contains(&index) {
            frame.fill(line.with_row(row), colors.neutral, true);
            Style::fg(colors.neutral).with_reverse()
        } else {
            Style::fg(colors.normal)
        };

        if current_track == Some(id) {
            style.insert(Attributes::BOLD);
        }

        if jukebox.is_faulty(id) {
            style.insert(Attributes::CROSSED_OUT);
        }

        areas.set_row(row);
        frame.print_fmt(style);

        database.track_table(id, |track| {
            frame.print_span(areas.title(), track.title);
            frame.print_span(areas.artist(), track.artists);
            frame.print_span(areas.album(), track.album);
            frame.push_fmt(track.duration);
            frame.render(areas.time(), TextOptions::span());
            frame.print_span(areas.rating(), track.rating.stars());
        });

        frame.print_fmt(Sgr::Reset);
        row += 1;
    }
}

fn render_shortcuts(area: Rect, frame: &mut Framebuffer, colors: &Colors) {
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
        frame.push_fmt(Sgr::Fg(key_color));
        frame.push_str(key);
        frame.push_fmt(Sgr::Fg(name_color));
        frame.push_ch(' ');
        frame.push_str(name);
        frame.push_ch_repeat(' ', gap);
    }

    frame.push_fmt(Sgr::reset_fg());
    frame.render(area, TextOptions::span_center_top());
}
