use ecs::*;
use terminal::*;
use widgets2::{Block, Scrollbar, ScrollbarData, list2::List};

use crate::{
    app::{Action, Actions, App, Input, InputSet, PageArea, RenderSet, ShortcutsArea},
    database2::{Database, TrackId, TrackRating, TrackSort, TrackTable},
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

#[derive(Debug, Default, Resource)]
pub struct TracksPage {
    list: List,
    params: Option<TrackId>,
}

impl TracksPage {
    pub const fn set_params(&mut self, id: TrackId) {
        self.params = Some(id);
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
            let id = database.tracks_id(page.list.index());
            jukebox.play_id(id, &database);
        }
        KeyCode::Char(c) => match c {
            '0' | '1' | '2' | '3' | '4' | '5' => {
                // TODO
                // let rating = TrackRating::from_char(c).unwrap();
                // for i in page.list.selection_inclusive() {
                //     let id = database.tracks_id(i);
                //     database.write_rating(id, rating);
                // }
            }
            'q' => {
                // TODO
                jukebox.extend(
                    page.list
                        .selection_inclusive()
                        .map(|i| database.tracks_id(i)),
                );
            }
            'n' => {
                // TODO
                // for i in page.list.selection_inclusive().rev() {
                //     if let Some(id) = database.get_id_from_index(i) {
                //         jukebox.enqueue_next(id);
                //     }
                // }
            }
            's' => {
                database.next_sort();
                actions.push(Action::Render);
            }
            'S' => {
                database.toggle_reverse_sort();
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
    let Some(id) = page.params.take() else {
        return;
    };

    // TODO
    // let index = database.tracks_index(id);
    // page.list.set_index(index).set_selector(None);
}

fn render_tracks(
    area: Res<PageArea>,
    shortcuts_area: Res<ShortcutsArea>,
    mut frame: ResMut<Framebuffer>,
    settings: Res<Settings>,
    mut page: ResMut<TracksPage>,
    mut database: NonSendMut<Database>,
    jukebox: NonSend<Jukebox>,
) {
    let area = **area;
    let colors = settings.colors();
    let tracks_len = database.tracks_len();

    if tracks_len == 0 {
        frame.print_fmt(Sgr::Fg(colors.neutral));
        frame.print_span_with(area, "No tracks to be found", SpanOptions::center());
        frame.print_fmt(Sgr::reset_fg());
        return;
    }

    // Block
    frame.print_fmt(Sgr::Fg(colors.secondary));
    Block::rectangle().render(area, &mut frame);
    frame.push_fmt_fg(format_args!(" All Tracks ({}) ", tracks_len), colors.normal);
    frame.render(area, TextOptions::span_center_top());
    frame.print_fmt(Sgr::reset_fg());

    // Table
    let (mut table_area, scoll_area) = {
        let inner = area.inner(Margin::all(1));
        let is_scrollable = tracks_len > inner.rows() as usize && inner.cols() > 16;
        if is_scrollable {
            let (ta, sa) = inner.split_right(1, 0);
            (ta.inner(Margin::horizontal(1)), Some(sa))
        } else {
            (inner.inner(Margin::horizontal(1)), None)
        }
    };

    let gap = if table_area.cols() < 10 { 0 } else { 2 };
    let mut areas = TableAreas::new(gap, table_area.with_rows(1));

    render_header(&areas, &mut frame, colors, &database);

    table_area.shrink_down(1);

    render_list(
        table_area,
        &mut areas,
        &mut frame,
        colors,
        &mut page.list,
        tracks_len,
        &mut database,
        &jukebox,
    );

    if let Some(scroll_area) = scoll_area {
        Scrollbar::new().with_colors(colors.scrollbar()).render(
            scroll_area,
            &mut frame,
            ScrollbarData {
                current_scroll: page.list.scroll(),
                total_items: tracks_len,
                viewport_height: table_area.rows(),
            },
        );
    }

    render_shortcuts(**shortcuts_area, &mut frame, &colors);
}

fn render_header(
    areas: &TableAreas,
    frame: &mut Framebuffer,
    colors: &Colors,
    database: &Database,
) {
    const fn sort_symbol<'a>(current_sort: TrackSort, sort: TrackSort, reverse: bool) -> &'a str {
        if current_sort.equals(sort) {
            if reverse { "⌃" } else { "⌄" }
        } else {
            ""
        }
    }

    let sort = database.get_sort();
    let reverse = database.get_reverse_sort();

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
}

fn render_list(
    area: Rect,
    areas: &mut TableAreas,
    frame: &mut Framebuffer,
    colors: &Colors,
    tracks_list: &mut List,
    tracks_len: usize,
    database: &mut Database,
    jukebox: &Jukebox,
) {
    let line = area.with_rows(1);
    let lines = tracks_list.set_len(tracks_len).render(area);
    let selected = tracks_list.index();
    let selection = tracks_list.selection_inclusive();
    let current_track = jukebox.current_track_id();
    let mut row = area.pos.row;

    database.tracks_table(lines, |track| {
        let TrackTable {
            index,
            id,
            title,
            artists,
            album,
            rating,
            duration,
        } = track;

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

        frame.print_span(areas.title(), title);
        frame.print_span(areas.artist(), artists);
        frame.print_span(areas.album(), album);
        frame.push_fmt(duration);
        frame.render(areas.time(), TextOptions::span());
        frame.print_span(areas.rating(), rating.stars());

        frame.print_fmt(Sgr::Reset);
        row += 1;
    });
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
fn render_tracks_modal(mut frame: ResMut<Framebuffer>, settings: Res<Settings>) {
    let area = frame.area();
    let area = area.with_size(area.size / 2).center(area);
    let colors = settings.colors();

    frame.fill(area, colors.background, false);

    frame.print_fmt(Sgr::Fg(colors.normal));
    Block::rectangle().render(area, &mut frame);
    frame.print_span_with(area, " Custom ", SpanOptions::center_top());

    frame.push_str("A custom modal only for the tracks page.");
    frame.render(
        area.inner(Margin::proportional(1)),
        TextOptions::paragraph_center(),
    );
    frame.print_fmt(Sgr::Reset);
}
