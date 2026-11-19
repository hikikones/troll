use std::path::PathBuf;

use terminal::*;
use widgets2::{KittyDeleteAll, Markup};

use crate::{
    AppDir,
    database::Database,
    modals::{Modal, Modals},
    pages::{Pages, Route},
    settings::Settings,
};

pub type AppResult<T> = Result<T, AppError>;

pub struct App {
    database: Database,
    settings: Settings,
    frame: Frame,
    pages: Pages,
    modals: Modals,
    markup: Markup,
    is_running: bool,
}

impl App {
    pub fn new(args: crate::cli::Args, app_dir: AppDir) -> AppResult<Self> {
        let database =
            Database::open(args.database.as_deref(), app_dir.as_deref()).map_err(|error| {
                AppError::Database {
                    path: args.database,
                    error,
                }
            })?;

        let mut modals = Modals::new();

        let mut settings = Settings::read_or_create(args.settings.as_deref(), app_dir.as_deref())
            .unwrap_or_else(|error| {
                modals.logs_mut().push_err(AppError::Config {
                    path: args.settings.clone(),
                    error,
                });
                Settings::new(args.settings)
            });
        settings.colors_mut().background =
            Rgb::query_bg(&mut std::io::stdout().lock(), &mut std::io::stdin().lock())
                .map(|rgb| rgb.slight_offset().as_color())
                .unwrap_or_else(|error| {
                    modals.logs_mut().push_err(AppError::QueryBg(error));
                    Color::Default
                });
        settings.colors_mut().foreground =
            Rgb::query_fg(&mut std::io::stdout().lock(), &mut std::io::stdin().lock())
                .unwrap_or_else(|error| {
                    modals.logs_mut().push_err(AppError::QueryFg(error));
                    Rgb::WHITE
                });

        let pages = Pages::new(Route::Review, &settings);
        let markup = Markup::new(args.assets)
            .with_colors(settings.colors().markup())
            .with_scrollbar(0)
            .with_padding(Margin::horizontal(1));

        Ok(Self {
            database,
            settings,
            frame: Frame::new(),
            pages,
            modals,
            markup,
            is_running: true,
        })
    }

    pub fn run(&mut self, terminal: &mut Terminal) -> AppResult<()> {
        self.frame
            .query_size()
            .map_err(|err| AppError::QuerySize(err))?;
        self.pages.enter(&mut self.database, &mut self.markup);
        self.render(terminal)?;

        while self.is_running {
            let Some(event) = TerminalEvent::read().map_err(|err| AppError::Event(err))? else {
                continue;
            };

            let action = self.handle_event(event, terminal)?;
            self.apply_action(action, terminal)?;
        }

        Ok(())
    }

    fn handle_event(&mut self, event: TerminalEvent, terminal: &mut Terminal) -> AppResult<Action> {
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
                KeyCode::Char('l') if key.ctrl() => {
                    if self.modals.logs().is_empty() {
                        Action::None
                    } else {
                        Action::Modal(Some(Modal::Logs))
                    }
                }
                _ => match self.modals.current() {
                    Some(modal) => self.modals.input(modal, key, &mut self.database),
                    None => self.pages.input(
                        key,
                        &mut self.database,
                        &mut self.markup,
                        &mut self.settings,
                        terminal,
                        self.modals.logs_mut(),
                    ),
                },
            },
            TerminalEvent::Resize => {
                self.frame
                    .query_size()
                    .map_err(|err| AppError::QuerySize(err))?;
                Action::Render
            }
        };

        Ok(action)
    }

    fn apply_action(&mut self, action: Action, terminal: &mut Terminal) -> AppResult<()> {
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
                self.pages.refresh(&mut self.database, &mut self.markup);
                self.render(terminal)?;
            }
            Action::Clear => {
                self.frame.increment_generation();
                self.frame.reset_cursor_visibility();
                self.render(terminal)?;
            }
            Action::Quit => {
                self.is_running = false;
            }
        }

        Ok(())
    }

    fn render(&mut self, terminal: &mut Terminal) -> AppResult<()> {
        self.frame.set_cursor_visibility(false);
        self.frame.print_fmt(KittyDeleteAll);

        let (top, body, bottom) = self.frame.area().split_ends(1, 2);

        // Swap colors for modal
        let colors = self
            .modals
            .is_active()
            .then(|| self.settings.replace_colors_with_dimmed());

        // Navigation
        self.pages
            .render_navigation(top, &mut self.frame, self.settings.colors());

        // Page
        self.pages.render(
            body.inner(Margin::all(1)),
            &mut self.frame,
            &self.settings,
            &self.database,
            &mut self.markup,
        );

        let [page_shortcuts_area, app_shortcuts_area] = bottom.split_lines();
        self.pages
            .render_shortcuts(page_shortcuts_area, &mut self.frame, &self.settings);
        self.render_shortcuts(app_shortcuts_area);

        // Modal
        if let Some((modal, colors)) = self.modals.current().zip(colors) {
            self.frame.set_cursor_visibility(false);
            self.settings.set_colors(colors);
            self.modals.render(
                modal,
                &mut self.frame,
                self.settings.colors(),
                &self.database,
            );
        }

        self.frame
            .flush(&mut terminal.lock())
            .map_err(|err| AppError::Render(err))
    }

    fn render_shortcuts(&mut self, area: Rect) {
        let (key_color, name_color) = {
            let colors = self.settings.colors();
            (colors.primary, colors.normal)
        };
        let gap = 1;

        for (key, name, gap) in [
            ("Esc", "Quit", gap),
            ("(⇧)Tab", "Navigate", gap),
            ("^f", "Find", 0),
        ] {
            self.frame.push_fmt(Sgr::Fg(key_color));
            self.frame.push_str(key);
            self.frame.push_fmt(Sgr::Fg(name_color));
            self.frame.push_ch(' ');
            self.frame.push_str(name);
            self.frame.push_ch_repeat(' ', gap);
        }

        if !self.modals.logs().is_empty() {
            self.frame.push_ch_repeat(' ', gap);
            self.frame.push_fmt(Sgr::Fg(key_color));
            self.frame.push_str("^l");
            self.frame.push_fmt(Sgr::Fg(name_color));
            self.frame.push_ch(' ');
            self.frame.push_str("Logs");

            let len = self.modals.logs().len_new();
            if len > 0 {
                let mut buf = std::fmt::NumBuffer::new();
                self.frame.push_iter(["(", len.format_into(&mut buf), ")"]);
            }
        }

        self.frame.push_fmt(Sgr::reset_fg());
        self.frame.render(area, TextOptions::span_center_top());
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

#[derive(Debug)]
pub enum AppError {
    Database {
        path: Option<PathBuf>,
        error: crate::database::DatabaseError,
    },
    Config {
        path: Option<PathBuf>,
        error: crate::settings::ConfigError,
    },
    QueryBg(RgbQueryError),
    QueryFg(RgbQueryError),
    QuerySize(std::io::Error),
    Render(std::io::Error),
    Event(std::io::Error),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database { path, .. } => {
                f.write_fmt(format_args!("Failed to open database at {path:?}."))
            }
            Self::Config { path, .. } => f.write_fmt(format_args!(
                "Failed to read or create settings at {path:?}."
            )),
            Self::QueryBg(_) => f.write_str(
                "Failed to query background color. Using default instead. Images will not be blocked in modals.",
            ),
            Self::QueryFg(_) => f.write_str(
                "Failed to query foreground color. Using white color instead for math.",
            ),
            Self::QuerySize(_) => f.write_str("Failed to query terminal for window size."),
            Self::Render(_) => f.write_str("Failed to render/flush to stdout."),
            Self::Event(_) => f.write_str("Failed to read event from stdin."),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database { error, .. } => Some(error),
            Self::Config { error, .. } => Some(error),
            Self::QueryBg(error) => Some(error),
            Self::QueryFg(error) => Some(error),
            Self::QuerySize(error) => Some(error),
            Self::Render(error) => Some(error),
            Self::Event(error) => Some(error),
        }
    }
}
