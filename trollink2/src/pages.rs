mod editor;
mod review;
mod settings;

use editor::*;
use review::*;
use settings::*;

use terminal::*;

use crate::settings::Colors;

pub struct Pages {
    pub route: Route,
    pub review: ReviewPage,
    pub editor: EditorPage,
    pub settings: SettingsPage,
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
