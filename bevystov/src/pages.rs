use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Res, ResMut},
};
use bevy_state::{
    condition::in_state,
    state::{NextState, States},
};
use terminal::*;
use widgets2::{Block, List, ListIndex};

use crate::app::{App, Colors, Frame, Input, PageArea, RenderSet};

pub struct PagesPlugin;

impl PagesPlugin {
    pub fn build(app: &mut App) {
        app.insert_resource(TracksParam::default())
            .insert_resource(TracksList::default())
            .add_state(Route::Tracks)
            .add_enter(Route::Tracks, enter_tracks)
            .add_render(
                RenderSet::Page,
                (
                    render_tracks.run_if(in_state(Route::Tracks)),
                    render_playing.run_if(in_state(Route::NowPlaying)),
                    render_settings.run_if(in_state(Route::Settings)),
                ),
            )
            .add_input((
                input_tracks.run_if(in_state(Route::Tracks)),
                input_playing.run_if(in_state(Route::NowPlaying)),
                input_settings.run_if(in_state(Route::Settings)),
            ));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States)]
pub enum Route {
    Tracks,
    NowPlaying,
    Settings,
}

impl Route {
    pub const fn next(self) -> Self {
        match self {
            Self::Tracks => Self::NowPlaying,
            Self::NowPlaying => Self::Settings,
            Self::Settings => Self::Tracks,
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::Tracks => Self::Settings,
            Self::NowPlaying => Self::Tracks,
            Self::Settings => Self::NowPlaying,
        }
    }
}

#[derive(Debug, Default, Resource, Deref, DerefMut)]
pub struct TracksParam(Option<usize>);

#[derive(Debug, Resource, Deref, DerefMut)]
struct TracksList(List);

impl Default for TracksList {
    fn default() -> Self {
        Self(
            List::new()
                .with_scrollbar(0)
                .with_padding(Margin::horizontal(1)),
        )
    }
}

fn enter_tracks(mut params: ResMut<TracksParam>, mut list: ResMut<TracksList>) {
    if let Some(i) = params.take() {
        list.set_index(i);
    }
}

// pub fn enter_tracks(&mut self, id: Option<TrackId>, db: &Database) {
//     if let Some(id) = id
//         && let Some(index) = db.get_index_from_id(id)
//     {
//         self.list.set_index(index).set_selector(None);
//     };
// }

fn render_tracks(
    area: Res<PageArea>,
    mut frame: ResMut<Frame>,
    colors: Res<Colors>,
    mut list: ResMut<TracksList>,
) {
    let area = **area;

    let tracks = [
        "yoyo here be dragons",
        "nope im out",
        "another one",
        "yolo",
        "hoho",
        "pudding",
        "yes",
    ];

    Block::rectangle(colors.secondary).render(area, &mut frame);
    frame.push_fmt_fg(
        format_args!(" All Tracks ({}) ", tracks.len()),
        colors.normal,
    );
    frame.render(area, TextOptions::span_center_top());

    let inner = area.inner(Margin::all(1));
    list.render(inner, &mut frame, tracks, |line, frame, track, index| {
        let style = match index {
            ListIndex::Selected => Style::fg(colors.primary).with_reverse(),
            ListIndex::Selection => Style::fg(colors.neutral).with_reverse(),
            ListIndex::Normal => Style::fg(colors.normal),
        };

        frame.print_fmt(style);
        frame.push_str(track);
        frame.render(line, TextOptions::span().with_fill());
        frame.print_fmt(Sgr::Reset);
    });
}

fn input_tracks(key: Res<Input>, mut list: ResMut<TracksList>) {
    match key.code {
        _ => {
            if list.input(**key) {
                // TODO: send render event
            }
        }
    }
}

fn render_playing(area: Res<PageArea>, mut frame: ResMut<Frame>, colors: Res<Colors>) {
    let area = **area;

    Block::rectangle(colors.primary).render(area, &mut frame);
    frame.push_str("PLAYING");
    frame.render(area, TextOptions::span_center());
}

fn input_playing(
    key: Res<Input>,
    mut params: ResMut<TracksParam>,
    mut next_route: ResMut<NextState<Route>>,
) {
    match key.code {
        KeyCode::Char('2') => {
            **params = Some(2);
            next_route.set(Route::Tracks);
        }
        _ => {
            // TODO
        }
    }
}

fn render_settings(area: Res<PageArea>, mut frame: ResMut<Frame>, colors: Res<Colors>) {
    let area = **area;

    Block::rectangle(colors.neutral).render(area, &mut frame);
    frame.push_str("SETTINGS");
    frame.render(area, TextOptions::span_center());
}

fn input_settings(key: Res<Input>) {
    match key.code {
        _ => {
            // TODO
        }
    }
}
