use terminal::*;
use widgets2::*;

mod playing;
mod settings;
mod tracks;

use playing::*;
use settings::*;
use tracks::*;

use crate::{
    app::{Action, Colors},
    database::TrackId,
    modals::ModalAction,
};

pub struct Pages {
    route: Route,
    tracks: TracksPage,
    playing: PlayingPage,
    settings: SettingsPage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Tracks(Option<TrackId>),
    NowPlaying,
    Settings,
}

impl Route {
    pub const fn next(self) -> Self {
        match self {
            Self::Tracks(_) => Self::NowPlaying,
            Self::NowPlaying => Self::Settings,
            Self::Settings => Self::Tracks(None),
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::Tracks(_) => Self::Settings,
            Self::NowPlaying => Self::Tracks(None),
            Self::Settings => Self::NowPlaying,
        }
    }
}

impl Pages {
    pub const fn new(route: Route) -> Self {
        Self {
            route,
            tracks: TracksPage::new(),
            playing: PlayingPage::new(),
            settings: SettingsPage::new(),
        }
    }

    pub const fn route(&self) -> Route {
        self.route
    }

    pub const fn next(&self) -> Route {
        self.route.next()
    }

    pub const fn prev(&self) -> Route {
        self.route.prev()
    }

    pub const fn set_route(&mut self, route: Route) {
        self.route = route;
    }

    pub fn on_enter(&mut self, frame: &mut Framebuffer) {
        match self.route {
            Route::Tracks(id) => self.tracks.on_enter(id),
            Route::NowPlaying => self.playing.on_enter(),
            Route::Settings => self.settings.on_enter(),
        }
    }

    pub fn on_exit(&mut self, frame: &mut Framebuffer) {
        match self.route {
            Route::Tracks(id) => self.tracks.on_exit(),
            Route::NowPlaying => self.playing.on_exit(),
            Route::Settings => self.settings.on_exit(),
        }
    }

    pub fn update(&self) {
        match self.route {
            Route::Tracks(id) => self.tracks.on_update(),
            Route::NowPlaying => self.playing.on_update(),
            Route::Settings => self.settings.on_update(),
        }
    }

    pub fn render(
        &mut self,
        area: Rect,
        frame: &mut Framebuffer,
        colors: &Colors,
        kitty: &KittyGraphics,
    ) {
        match self.route {
            Route::Tracks(id) => self.tracks.render(area, frame, colors),
            Route::NowPlaying => self.playing.render(area, frame, colors),
            Route::Settings => self.settings.render(area, frame, colors),
        }
    }

    pub fn input(&mut self, key: Key, terminal: &mut Terminal) -> Action {
        match self.route {
            Route::Tracks(id) => self.tracks.input(key),
            Route::NowPlaying => self.playing.input(key),
            Route::Settings => self.settings.input(key),
        }
    }

    pub fn render_modal(&mut self, area: Rect, frame: &mut Framebuffer, colors: &Colors) {
        match self.route {
            Route::Tracks(id) => self.tracks.render_modal(area, frame, colors),
            Route::NowPlaying => self.playing.render_modal(area, frame, colors),
            Route::Settings => self.settings.render_modal(area, frame, colors),
        }
    }

    pub fn input_modal(&mut self, key: Key) -> ModalAction {
        match self.route {
            Route::Tracks(id) => self.tracks.input_modal(key),
            Route::NowPlaying => self.playing.input_modal(key),
            Route::Settings => self.settings.input_modal(key),
        }
    }

    pub fn render_navigation(&self, area: Rect, frame: &mut Framebuffer, colors: &Colors) {
        frame.print_fmt(Sgr::Fg(colors.normal));
        frame.push_str("TODO TOP");
        frame.render(area, TextOptions::span_center_top());
        frame.print_fmt(Sgr::reset_fg());
    }
}
