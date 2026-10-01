use terminal::*;
use widgets2::{KittyDeleteAll, KittyGraphics};

use crate::{
    database::{Database, DatabaseEvent},
    events::{Event, EventHandler},
    jukebox::{Jukebox, JukeboxEvent},
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
            database: Database::new(std::path::PathBuf::from("/home/danny/Downloads/songs2")),
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
        self.pages.on_enter(terminal.frame(), &self.database);
        self.render(terminal)?;

        // Start reading events and load music
        self.events.start();
        self.database.load();

        // Run event loop
        while self.is_running {
            let action = match self.events.next()? {
                Event::Update => self.update(),
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
                    None => self.input_page(key),
                },
            },
            TerminalEvent::Resize => Action::Render,
        }
    }

    fn apply_action(&mut self, action: Action, terminal: &mut Terminal) -> std::io::Result<()> {
        match action {
            Action::None => {}
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

    fn update(&mut self) -> Action {
        let mut render = false;

        // Update database
        self.database.update(|event| {
            render = true;
            match event {
                DatabaseEvent::Rating(_) => {}
                DatabaseEvent::Error(err) => {
                    // self.pages.logs.enqueue(Log::new(err)); //TODO
                }
            }
        });

        // Update jukebox
        self.jukebox.update(&self.database, |event| {
            render = true;
            match event {
                JukeboxEvent::Play(id) => {
                    match id.and_then(|id| self.database.get(id)) {
                        Some(track) => {
                            //TODO
                            // // Start loading front cover
                            // let path = track.path().to_path_buf();
                            // let picker = self.picker.clone();
                            // let handle = load_front_cover(path, picker);
                            // self.front_cover_handle = Some(handle);

                            // // Update metadata and playback status for system media
                            // self.events.set_media(
                            //     track.title(),
                            //     track.artist(),
                            //     MediaPlayback::Playing,
                            // );
                        }
                        None => {
                            // Update only playback status for system media
                            // TODO
                            // self.events.set_playback(MediaPlayback::Playing);
                        }
                    }
                }
                JukeboxEvent::Pause => {
                    // self.events.set_playback(MediaPlayback::Paused);
                }
                JukeboxEvent::Stop => {
                    // self.front_cover = FrontCover::default();
                    // self.front_cover_handle = None;
                    // self.events.reset_media();
                }
                JukeboxEvent::Error(err) => {
                    // self.pages.logs.enqueue(Log::new(err));
                }
            }
        });

        // Poll thread for finished image loading
        // if let Some(handle) = self.front_cover_handle.as_ref() {
        //     if handle.is_finished() {
        //         render = true;
        //         let handle = self.front_cover_handle.take().unwrap();
        //         match handle.join().unwrap() {
        //             Ok(cover) => {
        //                 self.front_cover = cover;
        //             }
        //             Err(err) => {
        //                 self.front_cover = FrontCover::empty();
        //                 self.pages.logs.enqueue(Log::new(err));
        //             }
        //         }
        //     }
        // }

        self.pages
            .update(&self.database, &self.jukebox, &mut self.kitty);

        if render { Action::Render } else { Action::None }
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
        self.pages.on_exit(frame);
        self.pages.set_route(route);
        self.pages.on_enter(frame, &self.database);
    }

    fn render_page(&mut self, area: Rect, frame: &mut Framebuffer, colors: &Colors) {
        self.pages.render(
            area,
            frame,
            colors,
            &self.database,
            &self.jukebox,
            &self.kitty,
        );
    }

    fn input_page(&mut self, key: Key) -> Action {
        self.pages.input(key, &mut self.database, &mut self.jukebox)
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
                self.pages.refresh();
                Action::Modal(None)
            }
            ModalAction::Cancel => Action::Modal(None),
        }
    }
}
