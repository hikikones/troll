use terminal::*;
use widgets2::{KittyDeleteAll, KittyGraphics};

use crate::{
    database::Database,
    events::{Event, EventHandler},
    jukebox::Jukebox,
    modals::{Modal, ModalAction, Modals},
    pages::{Pages, Route},
};

pub struct App {
    pages: Pages,
    modals: Modals,
    events: EventHandler,
    database: Database,
    jukebox: Jukebox,
    kitty: KittyGraphics,
    colors: Colors,
    is_running: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    None,
    Update,
    Render,
    Route(Route),
    Modal(Option<Modal>),
    Clear,
    Quit,
}

#[derive(Debug, Clone)]
pub struct Colors {
    pub normal: Color,
    pub primary: Color,
    pub secondary: Color,
    pub neutral: Color,
    pub red: Color,
}

impl Colors {
    const fn all(color: Color) -> Self {
        Self {
            normal: color,
            primary: color,
            secondary: color,
            neutral: color,
            red: color,
        }
    }
}

impl App {
    pub fn new(jukebox: Jukebox) -> Self {
        Self {
            pages: Pages::new(Route::Tracks(None)),
            modals: Modals::new(),
            events: EventHandler::new(),
            database: Database::new(std::path::PathBuf::from("~/Downloads/songs2")),
            jukebox,
            kitty: KittyGraphics::new(),
            colors: Colors {
                normal: Color::Default,
                primary: Color::BrightYellow,
                secondary: Color::Yellow,
                neutral: Color::Indexed(240),
                red: Color::Red,
            },
            is_running: true,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal) -> Result<(), Box<dyn std::error::Error>> {
        // Render default page
        self.pages.on_enter(self.pages.route, terminal.frame());
        self.render(terminal)?;

        // Start reading events and load music
        self.events.start();
        self.database.load();

        // Run event loop
        while self.is_running {
            let action = match self.events.next()? {
                Event::Update => Action::Update,
                Event::Render => Action::Render,
                Event::Terminal(event) => self.handle_event(event, terminal),
            };
            self.apply_action(action, terminal)?;
        }

        Ok(())
    }

    fn handle_event(&mut self, event: TerminalEvent, terminal: &mut Terminal) -> Action {
        match event {
            TerminalEvent::Key(key) => match key.code {
                KeyCode::Esc => Action::Quit,
                KeyCode::Tab if self.modals.is_none() => Action::Route(self.pages.next()),
                KeyCode::BackTab if self.modals.is_none() => Action::Route(self.pages.prev()),
                KeyCode::Char('f') if key.ctrl() => Action::Modal(Some(Modal::Search)),
                _ => match self.modals.current {
                    Some(modal) => self.modal_input(key, modal),
                    None => self.input_page(key, terminal),
                },
            },
            TerminalEvent::Resize => Action::Render,
        }
    }

    fn apply_action(&mut self, action: Action, terminal: &mut Terminal) -> std::io::Result<()> {
        match action {
            Action::None => {}
            Action::Update => {
                //todo
            }
            Action::Render => {
                self.render(terminal)?;
            }
            Action::Route(route) => {
                self.set_route(route, terminal.frame());
                self.render(terminal)?;
            }
            Action::Modal(modal) => {
                self.set_modal(modal, terminal.frame());
                self.render(terminal)?;
            }
            Action::Clear => {
                self.kitty.increase_generation();
                self.render(terminal)?;
            }
            Action::Quit => {
                self.is_running = false;
            }
        }

        Ok(())
    }

    fn render(&mut self, terminal: &mut Terminal) -> std::io::Result<()> {
        terminal.render(|frame| {
            let area = frame.area();

            frame.print_fmt(KittyDeleteAll);

            let colors = if self.modals.current.is_none() {
                self.colors.clone()
            } else {
                Colors::all(self.colors.neutral)
            };

            let (top, body, bottom) = area.split_ends(1, 1);

            self.pages.render_navigation(top, frame, &colors);

            self.render_page(body.inner(Margin::proportional(1)), frame, &colors);

            frame.push_fmt(Sgr::Fg(colors.normal));
            frame.push_str("TODO BOTTOM");
            frame.push_fmt(Sgr::reset_fg());
            frame.render(bottom, TextOptions::span_center_top());

            if let Some(modal) = self.modals.current {
                self.modal_render(area, frame, modal);
            }

            Ok(())
        })
    }

    fn set_route(&mut self, route: Route, frame: &mut Framebuffer) {
        self.pages.on_exit(self.pages.route, frame);
        self.pages.route = route;
        self.pages.on_enter(route, frame);
    }

    fn render_page(&mut self, area: Rect, frame: &mut Framebuffer, colors: &Colors) {
        self.pages.render(area, frame, colors, &self.kitty);
    }

    fn input_page(&mut self, key: Key, terminal: &mut Terminal) -> Action {
        self.pages.input(key, terminal)
    }

    fn set_modal(&mut self, modal: Option<Modal>, frame: &mut Framebuffer) {
        match (self.modals.current, modal) {
            (None, None) => {}
            (None, Some(modal)) => {
                self.modal_enter(modal, frame);
            }
            (Some(current), None) => {
                self.modal_exit(current, frame);
            }
            (Some(current), Some(next)) => match (current, next) {
                (Modal::Search, Modal::Search) => {
                    self.modal_exit(current, frame);
                }
                (Modal::Custom, _) => {
                    self.modal_exit(current, frame);
                    self.modal_enter(next, frame);
                }
                (_, Modal::Custom) => {}
            },
        }
    }

    fn modal_enter(&mut self, modal: Modal, frame: &mut Framebuffer) {
        self.modals.current = Some(modal);

        self.modals.cursor_state = frame.cursor_state();
        frame.set_cursor_state(CursorState::Hide);

        match modal {
            Modal::Search => self.modals.search.on_enter(),
            Modal::Custom => {}
        }
    }

    fn modal_exit(&mut self, modal: Modal, frame: &mut Framebuffer) {
        self.modals.current = None;

        frame.set_cursor_state(self.modals.cursor_state);

        match modal {
            Modal::Search => self.modals.search.on_exit(),
            Modal::Custom => {}
        }
    }

    fn modal_render(&mut self, area: Rect, frame: &mut Framebuffer, modal: Modal) {
        let colors = &self.colors;

        match modal {
            Modal::Search => {
                self.modals.search.render(area, frame, colors);
            }
            Modal::Custom => {
                self.pages.render_modal(area, frame, colors);
            }
        }
    }

    fn modal_input(&mut self, key: Key, modal: Modal) -> Action {
        let action = match modal {
            Modal::Search => self.modals.search.input(key),
            Modal::Custom => self.pages.input_modal(key),
        };

        match action {
            ModalAction::None => Action::None,
            ModalAction::Render => Action::Render,
            ModalAction::Confirm => {
                self.pages.update();
                Action::Modal(None)
            }
            ModalAction::Cancel => Action::Modal(None),
        }
    }
}
