use bevy_state::state::States;

use crate::{app::App, database::TrackId};

mod playing;
mod settings;
mod tracks;

pub use playing::*;
pub use settings::*;
pub use tracks::*;

pub struct PagesPlugin;

impl PagesPlugin {
    pub fn build(app: &mut App) {
        app.add_state(Page::default());

        TracksPagePlugin::build(app);
        PlayingPagePlugin::build(app);
        SettingsPagePlugin::build(app);
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
    Tracks(Option<TrackId>),
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
