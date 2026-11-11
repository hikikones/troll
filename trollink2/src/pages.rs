mod cards;
mod editor;
mod review;
mod settings;

use cards::*;
use editor::*;
use review::*;
use settings::*;

use terminal::*;
use widgets2::Markup;

use crate::{
    app::Action,
    database::{CardId, Database, TagId},
    settings::{Colors, Settings},
};

pub struct Pages {
    route: Route,
    review: ReviewPage,
    editor: EditorPage,
    cards: CardsPage,
    settings: SettingsPage,
}

#[derive(Debug, Clone, Copy)]
pub enum Route {
    Review,
    Editor(Option<CardId>),
    Cards(Option<CardsParam>),
    Settings,
}

#[derive(Debug, Clone, Copy)]
pub enum CardsParam {
    Card(CardId),
    Tag(TagId),
}

impl Route {
    pub const fn next(self) -> Self {
        match self {
            Self::Review => Self::Editor(None),
            Self::Editor(_) => Self::Cards(None),
            Self::Cards(_) => Self::Settings,
            Self::Settings => Self::Review,
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::Review => Self::Settings,
            Self::Editor(_) => Self::Review,
            Self::Cards(_) => Self::Editor(None),
            Self::Settings => Self::Cards(None),
        }
    }
}

impl Pages {
    pub fn new(route: Route) -> Self {
        Self {
            route,
            review: ReviewPage::new(),
            editor: EditorPage::new(),
            cards: CardsPage::new(),
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

    pub fn set(&mut self, route: Route, db: &mut Database, markup: &mut Markup) {
        self.exit();
        self.route = route;
        self.enter(db, markup);
    }

    pub fn enter(&mut self, db: &mut Database, markup: &mut Markup) {
        match self.route {
            Route::Review => self.review.enter(db, markup),
            Route::Editor(id) => self.editor.enter(id, db),
            Route::Cards(param) => self.cards.enter(db, param),
            Route::Settings => self.settings.enter(),
        }
    }

    fn exit(&mut self) {
        match self.route {
            Route::Review => self.review.exit(),
            Route::Editor(id) => self.editor.exit(),
            Route::Cards(param) => self.cards.exit(),
            Route::Settings => self.settings.exit(),
        }
    }

    pub fn refresh(&self) {
        match self.route {
            Route::Review => self.review.refresh(),
            Route::Editor(id) => self.editor.refresh(),
            Route::Cards(param) => self.cards.refresh(),
            Route::Settings => self.settings.refresh(),
        }
    }

    pub fn input(
        &mut self,
        key: Key,
        db: &mut Database,
        markup: &mut Markup,
        settings: &Settings,
        terminal: &mut Terminal,
    ) -> Action {
        match self.route {
            Route::Review => {
                self.review
                    .input(key, db, markup, settings.desired_retention_as_fraction())
            }
            Route::Editor(id) => self.editor.input(key, db, markup, terminal),
            Route::Cards(param) => self.cards.input(key, db, markup),
            Route::Settings => self.settings.input(key),
        }
    }

    pub fn render(
        &mut self,
        area: Rect,
        frame: &mut Framebuffer,
        colors: &Colors,
        db: &Database,
        markup: &mut Markup,
    ) {
        match self.route {
            Route::Review => self.review.render(area, frame, colors, markup),
            Route::Editor(id) => self.editor.render(area, frame, colors, db, markup),
            Route::Cards(param) => self.cards.render(area, frame, db, colors, markup),
            Route::Settings => self.settings.render(area, frame, colors),
        }
    }

    pub fn render_shortcuts(&mut self, area: Rect, frame: &mut Framebuffer, colors: &Colors) {
        match self.route {
            Route::Review => self.review.render_shortcuts(area, frame, colors),
            Route::Editor(id) => self.editor.render_shortcuts(area, frame, colors),
            Route::Cards(param) => self.cards.render_shortcuts(area, frame, colors),
            Route::Settings => self.settings.render_shortcuts(area, frame, colors),
        }
    }

    pub fn render_navigation(&self, area: Rect, frame: &mut Framebuffer, colors: &Colors) {
        for (route, name, gap) in [
            (Route::Review, "Review", 3),
            (Route::Editor(None), "Editor", 3),
            (Route::Cards(None), "Cards", 3),
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
