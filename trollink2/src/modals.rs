mod logs;
mod search;
mod tags;

pub use logs::*;
use search::*;
use tags::*;

use terminal::*;

use crate::{
    app::Action,
    database::{Database, TagId},
    settings::Colors,
};

pub struct Modals {
    current: Option<Modal>,
    search: SearchModal,
    logs: LogsModal,
    tags: TagsModal,
}

#[derive(Debug, Clone, Copy)]
pub enum Modal {
    Search,
    Logs,
    AddTag,
    EditTag(TagId),
    DeleteTag(TagId),
}

impl Modals {
    pub const fn new() -> Self {
        Self {
            current: None,
            search: SearchModal::new(),
            logs: LogsModal::new(),
            tags: TagsModal::new(),
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

    pub fn set(&mut self, modal: Option<Modal>, db: &Database) {
        match (self.current, modal) {
            (None, Some(modal)) => {
                self.enter(modal, db);
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
                    self.enter(next, db);
                }
            },
            (None, None) => {}
        }
    }

    fn enter(&mut self, modal: Modal, db: &Database) {
        self.current = Some(modal);

        match modal {
            Modal::Search => self.search.enter(db),
            Modal::Logs => self.logs.enter(),
            Modal::AddTag => self.tags.enter(TagOp::Add, db),
            Modal::EditTag(id) => self.tags.enter(TagOp::Edit(id), db),
            Modal::DeleteTag(id) => self.tags.enter(TagOp::Delete(id), db),
        }
    }

    fn exit(&mut self, modal: Modal) {
        self.current = None;

        match modal {
            Modal::Search => self.search.exit(),
            Modal::Logs => self.logs.exit(),
            Modal::AddTag | Modal::EditTag(_) | Modal::DeleteTag(_) => self.tags.exit(),
        }
    }

    pub fn input(&mut self, modal: Modal, key: Key, db: &mut Database, colors: &Colors) -> Action {
        match modal {
            Modal::Search => self.search.input(key, db, colors),
            Modal::Logs => self.logs.input(key),
            Modal::AddTag | Modal::EditTag(_) | Modal::DeleteTag(_) => self.tags.input(key, db),
        }
    }

    pub fn render(
        &mut self,
        modal: Modal,
        frame: &mut Framebuffer,
        colors: &Colors,
        db: &Database,
    ) {
        match modal {
            Modal::Search => self.search.render(frame, colors),
            Modal::Logs => self.logs.render(frame, colors),
            Modal::AddTag | Modal::EditTag(_) | Modal::DeleteTag(_) => {
                self.tags.render(frame, colors, db)
            }
        }
    }
}
