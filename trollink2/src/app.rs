use terminal::*;
use widgets2::{KittyDeleteAll, Markup};

use crate::{
    AppDir,
    database::Database,
    modals::{Modal, Modals},
    pages::{Pages, Route},
    settings::{Colors, Settings},
};

pub type AppResult<T> = Result<T, AppError>;

pub struct App {
    database: Database,
    settings: Settings,
    frame: Framebuffer,
    pages: Pages,
    modals: Modals,
    markup: Markup,
    is_running: bool,
}

impl App {
    pub fn new(args: crate::cli::Args, app_dir: AppDir) -> AppResult<Self> {
        let database = Database::open(args.database.as_deref(), app_dir.as_deref())
            .map_err(|err| AppError::from_database(args.database.as_deref(), err))?;

        let mut modals = Modals::new();
        let settings = Settings::read_or_create(args.settings.as_deref(), app_dir.as_deref())
            .unwrap_or_else(|err| {
                modals
                    .logs_mut()
                    .push_err(AppError::from_config(args.settings.as_deref(), err));
                Settings::new(args.settings)
            });

        Ok(Self {
            database,
            settings,
            frame: Framebuffer::new(),
            pages: Pages::new(Route::Review),
            modals,
            markup: Markup::new(args.assets)
                .with_scrollbar(0)
                .with_padding(Margin::horizontal(1)),
            is_running: true,
        })
    }

    pub fn run(&mut self, terminal: &mut Terminal) -> std::io::Result<()> {
        self.frame.query_size()?;
        self.pages.enter(&mut self.database, &mut self.markup);
        self.render(terminal)?;

        while self.is_running {
            let Some(event) = Terminal::read()? else {
                continue;
            };

            let action = self.handle_event(event, terminal)?;
            self.apply_action(action, terminal)?;
        }

        Ok(())
    }

    fn handle_event(
        &mut self,
        event: TerminalEvent,
        terminal: &mut Terminal,
    ) -> std::io::Result<Action> {
        let action = match event {
            TerminalEvent::Key(key) => match key.code {
                KeyCode::Esc => Action::Quit,
                KeyCode::Tab => {
                    if self.modals.is_active() {
                        Action::Modal(None)
                    } else {
                        Action::Route(self.pages.forward())
                    }
                }
                KeyCode::BackTab => {
                    if self.modals.is_active() {
                        Action::Modal(None)
                    } else {
                        Action::Route(self.pages.backward())
                    }
                }
                KeyCode::Char('f') if key.ctrl() => Action::Modal(Some(Modal::Search)),
                KeyCode::Char('l') if key.ctrl() => Action::Modal(Some(Modal::Logs)),
                _ => match self.modals.current() {
                    Some(modal) => {
                        self.modals
                            .input(modal, key, &mut self.database, self.settings.colors())
                    }
                    None => self.pages.input(
                        key,
                        &mut self.database,
                        &mut self.markup,
                        &self.settings,
                        terminal,
                        self.modals.logs_mut(),
                    ),
                },
            },
            TerminalEvent::Resize => {
                self.frame.query_size()?;
                Action::Render
            }
        };

        Ok(action)
    }

    fn apply_action(&mut self, action: Action, terminal: &mut Terminal) -> std::io::Result<()> {
        match action {
            Action::None => {}
            Action::Render => {
                self.render(terminal)?;
            }
            Action::Route(route) => {
                if self.modals.is_active() {
                    self.modals.set(None, &self.database);
                }

                self.pages.set(route, &mut self.database, &mut self.markup);
                self.render(terminal)?;
            }
            Action::Modal(modal) => {
                self.modals.set(modal, &self.database);
                self.render(terminal)?;
            }
            Action::Refresh => {
                self.modals.set(None, &self.database);
                self.pages.refresh();
                self.render(terminal)?;
            }
            Action::Clear => {
                self.frame.increment_generation();
                self.render(terminal)?;
            }
            Action::Quit => {
                self.is_running = false;
            }
        }

        Ok(())
    }

    fn render(&mut self, terminal: &mut Terminal) -> std::io::Result<()> {
        let mut frame = std::mem::take(&mut self.frame);

        frame.set_cursor_visibility(false);
        frame.print_fmt(KittyDeleteAll);

        let (top, body, bottom) = frame.area().split_ends(1, 2);

        // TODO: swap modal colors
        let colors = self.settings.colors();

        // Navigation
        self.pages.render_navigation(top, &mut frame, colors);

        // Page
        self.pages.render(
            body.inner(Margin::all(1)),
            &mut frame,
            colors,
            &self.database,
            &mut self.markup,
        );

        // TODO
        let [page_shortcuts_area, app_shortcuts_area] = bottom.split_lines();

        self.pages
            .render_shortcuts(page_shortcuts_area, &mut frame, colors);
        self.render_shortcuts(app_shortcuts_area, &mut frame, colors);

        // Modal
        if let Some(modal) = self.modals.current() {
            frame.set_cursor_visibility(false);
            self.modals.render(modal, &mut frame, colors);
        }

        frame.flush(&mut terminal.lock())?;
        self.frame = frame;

        Ok(())
    }

    fn render_shortcuts(&self, area: Rect, frame: &mut Framebuffer, colors: &Colors) {
        let key_color = colors.primary;
        let name_color = colors.normal;
        let gap = 1;

        for (key, name, gap) in [
            ("Esc", "Quit", gap),
            ("(⇧)Tab", "Navigate", gap),
            ("^f", "Find", 0),
        ] {
            frame.push_fmt(Sgr::Fg(key_color));
            frame.push_str(key);
            frame.push_fmt(Sgr::Fg(name_color));
            frame.push_ch(' ');
            frame.push_str(name);
            frame.push_ch_repeat(' ', gap);
        }

        if !self.modals.logs().is_empty() {
            frame.push_ch_repeat(' ', gap);
            frame.push_fmt(Sgr::Fg(key_color));
            frame.push_str("^l");
            frame.push_fmt(Sgr::Fg(name_color));
            frame.push_ch(' ');
            frame.push_str("Logs");

            let len = self.modals.logs().len_new();
            if len > 0 {
                let mut buf = std::fmt::NumBuffer::new();
                frame.push_iter(["(", len.format_into(&mut buf), ")"]);
            }
        }

        frame.push_fmt(Sgr::reset_fg());
        frame.render(area, TextOptions::span_center_top());
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    None,
    Render,
    Route(Route),
    Modal(Option<Modal>),
    Refresh,
    Clear,
    Quit,
}

// TODO: Rework error to just an enum. Print message in impl Display.
#[derive(Debug)]
pub struct AppError {
    message: String,
    source: Option<AppErrorSource>,
}

impl AppError {
    pub fn new(message: impl Into<String>, source: Option<AppErrorSource>) -> Self {
        Self {
            message: message.into(),
            source,
        }
    }

    fn from_database(path: Option<&std::path::Path>, err: crate::database::DatabaseError) -> Self {
        match path {
            Some(path) => Self {
                message: format!("Failed to open database at '{}'.", path.display()),
                source: Some(AppErrorSource::Database(err)),
            },
            None => Self {
                message: String::from("Failed to open database."),
                source: Some(AppErrorSource::Database(err)),
            },
        }
    }

    fn from_config(path: Option<&std::path::Path>, err: crate::settings::ConfigError) -> Self {
        match path {
            Some(path) => Self {
                message: format!("Failed to read of create settings at '{}'.", path.display()),
                source: Some(AppErrorSource::Config(err)),
            },
            None => Self {
                message: String::from("Failed to read of create settings."),
                source: Some(AppErrorSource::Config(err)),
            },
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message.as_str())
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|source| match source {
            AppErrorSource::Io(err) => err as _,
            AppErrorSource::Database(err) => err as _,
            AppErrorSource::Config(err) => err as _,
        })
    }
}

#[derive(Debug)]
pub enum AppErrorSource {
    Io(std::io::Error),
    Database(crate::database::DatabaseError),
    Config(crate::settings::ConfigError),
}
