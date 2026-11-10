mod editor;
mod review;
mod settings;

use editor::*;
use review::*;
use settings::*;

use terminal::*;

use crate::{app::Action, settings::Colors};

pub struct Pages {
    route: Route,
    review: ReviewPage,
    editor: EditorPage,
    settings: SettingsPage,
}

#[derive(Debug, Clone, Copy)]
pub enum Route {
    Review,
    Editor,
    Settings,
}

impl Route {
    pub const fn next(self) -> Self {
        match self {
            Self::Review => Self::Editor,
            Self::Editor => Self::Settings,
            Self::Settings => Self::Review,
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::Review => Self::Settings,
            Self::Editor => Self::Review,
            Self::Settings => Self::Editor,
        }
    }
}

impl Pages {
    pub const fn new(route: Route) -> Self {
        Self {
            route,
            review: ReviewPage::new(),
            editor: EditorPage::new(),
            settings: SettingsPage::new(),
        }
    }

    pub const fn current(&self) -> Route {
        self.route
    }

    pub const fn forward(&self) -> Route {
        self.route.next()
    }

    pub const fn backward(&self) -> Route {
        self.route.prev()
    }

    pub fn set(&mut self, route: Route) {
        self.exit();
        self.route = route;
        self.enter();
    }

    fn enter(&self) {
        match self.route {
            Route::Review => self.review.enter(),
            Route::Editor => self.editor.enter(),
            Route::Settings => self.settings.enter(),
        }
    }

    fn exit(&self) {
        match self.route {
            Route::Review => self.review.exit(),
            Route::Editor => self.editor.exit(),
            Route::Settings => self.settings.exit(),
        }
    }

    pub fn refresh(&self) {
        match self.route {
            Route::Review => self.review.refresh(),
            Route::Editor => self.editor.refresh(),
            Route::Settings => self.settings.refresh(),
        }
    }

    pub fn input(&mut self, key: Key) -> Action {
        match self.route {
            Route::Review => self.review.input(key),
            Route::Editor => self.editor.input(key),
            Route::Settings => self.settings.input(key),
        }
    }

    pub fn render(&self, area: Rect, frame: &mut Framebuffer, colors: &Colors) {
        match self.route {
            Route::Review => self.review.render(area, frame, colors),
            Route::Editor => self.editor.render(area, frame, colors),
            Route::Settings => self.settings.render(area, frame, colors),
        }
    }

    pub fn render_navigation(&self, area: Rect, frame: &mut Framebuffer, colors: &Colors) {
        for (route, name, gap) in [
            (Route::Review, "Review", 3),
            (Route::Editor, "Editor", 3),
            (Route::Settings, "Settings", 0),
        ] {
            let is_current = std::mem::discriminant(&self.route) == std::mem::discriminant(&route);
            if is_current {
                frame.push_fmt(Sgrs([Sgr::Bold, Sgr::Fg(colors.primary)]));
                frame.push_str(name);
                frame.push_fmt(Sgr::Reset);
            } else {
                frame.push_str_fg(name, colors.normal);
            }
            frame.push_ch_repeat(' ', gap);
        }

        frame.render(area, TextOptions::span_center_top());
    }
}
