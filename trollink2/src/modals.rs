mod logs;
mod search;

pub use logs::*;
use search::*;

use terminal::*;

use crate::{app::Action, settings::Colors};

pub struct Modals {
    current: Option<Modal>,
    search: SearchModal,
    logs: LogsModal,
}

#[derive(Debug, Clone, Copy)]
pub enum Modal {
    Search,
    Logs,
}

impl Modals {
    pub const fn new() -> Self {
        Self {
            current: None,
            search: SearchModal::new(),
            logs: LogsModal::new(),
        }
    }

    pub const fn is_active(&self) -> bool {
        self.current.is_some()
    }

    pub const fn current(&self) -> Option<Modal> {
        self.current
    }

    pub const fn logs(&self) -> &LogsModal {
        &self.logs
    }

    pub const fn logs_mut(&mut self) -> &mut LogsModal {
        &mut self.logs
    }

    pub fn set(&mut self, modal: Option<Modal>) {
        match (self.current, modal) {
            (None, Some(modal)) => {
                self.enter(modal);
            }
            (Some(current), None) => {
                self.exit(current);
            }
            (Some(current), Some(next)) => match (current, next) {
                (Modal::Search, Modal::Search) => {
                    self.exit(current);
                }
                (Modal::Logs, Modal::Logs) => {
                    self.exit(current);
                }
                (_, _) => {
                    self.exit(current);
                    self.enter(next);
                }
            },
            (None, None) => {}
        }
    }

    fn enter(&mut self, modal: Modal) {
        self.current = Some(modal);

        match modal {
            Modal::Search => self.search.enter(),
            Modal::Logs => self.logs.enter(),
        }
    }

    fn exit(&mut self, modal: Modal) {
        self.current = None;

        match modal {
            Modal::Search => self.search.exit(),
            Modal::Logs => self.logs.exit(),
        }
    }

    pub fn input(&mut self, modal: Modal, key: Key) -> Action {
        match modal {
            Modal::Search => self.search.input(key),
            Modal::Logs => self.logs.input(key),
        }
    }

    pub fn render(&self, modal: Modal, frame: &mut Framebuffer, colors: &Colors) {
        match modal {
            Modal::Search => self.search.render(frame, colors),
            Modal::Logs => self.logs.render(frame, colors),
        }
    }
}
