use bevy_ecs::{
    schedule::IntoScheduleConfigs,
    system::{Res, ResMut},
};
use bevy_state::{condition::in_state, state::States};
use terminal::*;
use widgets2::Block;

use crate::app::{App, Colors, Frame, Input, PageArea, RenderSet};

pub struct PagesPlugin;

impl PagesPlugin {
    pub fn build(app: &mut App) {
        app.add_state(Route::Tracks)
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

fn render_tracks(area: Res<PageArea>, mut frame: ResMut<Frame>, colors: Res<Colors>) {
    let area = **area;

    Block::rectangle(colors.secondary).render(area, &mut frame);
    frame.push_str("TRACKS");
    frame.render(area, TextOptions::span_center());
}

fn input_tracks(key: Res<Input>) {
    match key.code {
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

fn input_playing(key: Res<Input>) {
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

fn input_settings(key: Res<Input>) {
    match key.code {
        _ => {
            // TODO
        }
    }
}
