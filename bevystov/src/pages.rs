use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    resource::Resource,
    schedule::{IntoScheduleConfigs, SystemCondition},
    system::{Res, ResMut},
};
use bevy_state::{condition::in_state, state::States};
use terminal::*;
use widgets2::{Block, List, ListIndex};

use crate::{
    app::{Action, Actions, App, Colors, Frame, Input, InputState, PageArea, RenderSet},
    modals::Modal,
};

pub struct PagesPlugin;

impl PagesPlugin {
    pub fn build(app: &mut App) {
        app.insert_resource(TracksParam::default())
            .insert_resource(TracksList::default())
            .add_state(Page::default())
            .add_input(
                InputState::Normal,
                (
                    input_tracks.run_if(in_state(Page::Tracks)),
                    input_playing.run_if(in_state(Page::NowPlaying)),
                    input_settings.run_if(in_state(Page::Settings)),
                ),
            )
            .add_input(
                InputState::Modal,
                input_tracks_modal.run_if(in_state(Page::Tracks).and_then(in_state(Modal::Custom))),
            )
            .add_enter(Page::Tracks, enter_tracks)
            .add_render(
                RenderSet::Page,
                (
                    render_tracks.run_if(in_state(Page::Tracks)),
                    render_playing.run_if(in_state(Page::NowPlaying)),
                    render_settings.run_if(in_state(Page::Settings)),
                ),
            )
            .add_render(
                RenderSet::Modal,
                render_tracks_modal
                    .run_if(in_state(Page::Tracks).and_then(in_state(Modal::Custom))),
            );
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, States)]
pub enum Page {
    #[default]
    Tracks,
    NowPlaying,
    Settings,
}

impl Page {
    pub const fn next(self) -> Route {
        match self {
            Self::Tracks => Route::NowPlaying,
            Self::NowPlaying => Route::Settings,
            Self::Settings => Route::Tracks(None),
        }
    }

    pub const fn prev(self) -> Route {
        match self {
            Self::Tracks => Route::Settings,
            Self::NowPlaying => Route::Tracks(None),
            Self::Settings => Route::NowPlaying,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Route {
    Tracks(Option<usize>),
    NowPlaying,
    Settings,
}

impl Route {
    pub const fn as_page(self) -> Page {
        match self {
            Self::Tracks(_) => Page::Tracks,
            Self::NowPlaying => Page::NowPlaying,
            Self::Settings => Page::Settings,
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

fn input_tracks(key: Res<Input>, mut actions: ResMut<Actions>, mut list: ResMut<TracksList>) {
    match key.code {
        KeyCode::Char('m') => {
            actions.push(Action::Modal(Modal::Custom));
        }
        _ => {
            if list.input(**key) {
                actions.push(Action::Render);
            }
        }
    }
}

fn input_tracks_modal(key: Res<Input>, mut actions: ResMut<Actions>) {
    match key.code {
        KeyCode::Char('m') => {
            actions.push(Action::Modal(Modal::Canceled));
        }
        _ => {}
    }
}

fn enter_tracks(mut params: ResMut<TracksParam>, mut list: ResMut<TracksList>) {
    if let Some(i) = params.take() {
        list.set_index(i);
    }
}

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

fn render_tracks_modal(mut frame: ResMut<Frame>, colors: Res<Colors>) {
    let area = frame.area();
    let area = area.with_size(area.size / 2).center(area);

    let bg = frame.palette().background().slight_offset().as_color();
    Block::fill(Style::bg(bg)).render(area, &mut *frame);
    Block::rectangle(Style::fg(colors.normal).with_bg(bg)).render(area, &mut *frame);
    frame.push_str(" Custom ");
    frame.render(area, TextOptions::span_center_top());

    frame.push_str("A custom modal only for the tracks page.");
    frame.render(
        area.inner(Margin::proportional(1)),
        TextOptions::paragraph_center(),
    );
}

fn input_playing(key: Res<Input>, mut actions: ResMut<Actions>) {
    match key.code {
        KeyCode::Char('2') => {
            actions.push(Action::Route(Route::Tracks(Some(2))));
        }
        _ => {
            // TODO
        }
    }
}

fn render_playing(area: Res<PageArea>, mut frame: ResMut<Frame>, colors: Res<Colors>) {
    let area = **area;

    Block::rectangle(colors.primary).render(area, &mut frame);
    frame.push_str("PLAYING");
    frame.render(area, TextOptions::span_center());
}

fn input_settings(key: Res<Input>) {
    match key.code {
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
