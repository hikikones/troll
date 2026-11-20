mod action;
mod logs;
mod search;

use action::*;
use logs::*;
use search::*;

use shared::logs::Logs;
use terminal::*;

use crate::{
    app::Action,
    database::{CardId, Database, TagId},
    settings::Colors,
};

pub struct Modals {
    current: Option<Modal>,
    search: SearchModal,
    logs: LogsModal,
    action: ActionModal,
}

#[derive(Debug, Clone, Copy)]
pub enum Modal {
    Search,
    Logs,
    AddTag,
    EditTag(TagId),
    DeleteTag(TagId),
    DeleteCard(CardId),
}

impl Modals {
    pub const fn new() -> Self {
        Self {
            current: None,
            search: SearchModal::new(),
            logs: LogsModal::new(),
            action: ActionModal::new(),
        }
    }

    pub const fn is_active(&self) -> bool {
        self.current.is_some()
    }

    pub const fn current(&self) -> Option<Modal> {
        self.current
    }

    pub const fn logs(&self) -> &Logs {
        &self.logs.logs
    }

    pub const fn logs_mut(&mut self) -> &mut Logs {
        &mut self.logs.logs
    }

    pub fn set(&mut self, modal: Option<Modal>, db: &Database) {
        match (self.current, modal) {
            (None, Some(modal)) => {
                self.enter(modal, db);
            }
            (Some(current), None) => {
                self.exit(current);
            }
            (Some(current), Some(next)) => {
                let is_same = std::mem::discriminant(&current) == std::mem::discriminant(&next);
                if is_same {
                    self.exit(current);
                } else {
                    self.exit(current);
                    self.enter(next, db);
                }
            }
            (None, None) => {}
        }
    }

    fn enter(&mut self, modal: Modal, db: &Database) {
        self.current = Some(modal);

        match modal {
            Modal::Search => self.search.enter(db),
            Modal::Logs => self.logs.enter(),
            Modal::AddTag => self.action.enter(ModalAction::AddTag, db),
            Modal::EditTag(id) => self.action.enter(ModalAction::EditTag(id), db),
            Modal::DeleteTag(id) => self.action.enter(ModalAction::DeleteTag(id), db),
            Modal::DeleteCard(id) => self.action.enter(ModalAction::DeleteCard(id), db),
        }
    }

    fn exit(&mut self, modal: Modal) {
        self.current = None;

        match modal {
            Modal::Search => self.search.exit(),
            Modal::Logs => self.logs.exit(),
            Modal::AddTag | Modal::EditTag(_) | Modal::DeleteTag(_) | Modal::DeleteCard(_) => {
                self.action.exit()
            }
        }
    }

    pub fn input(&mut self, modal: Modal, key: Key, db: &mut Database) -> Action {
        match modal {
            Modal::Search => self.search.input(key, db),
            Modal::Logs => self.logs.input(key),
            Modal::AddTag | Modal::EditTag(_) | Modal::DeleteTag(_) | Modal::DeleteCard(_) => {
                self.action.input(key, db)
            }
        }
    }

    pub fn render(&mut self, modal: Modal, frame: &mut Frame, colors: &Colors, db: &Database) {
        match modal {
            Modal::Search => self.search.render(frame, colors),
            Modal::Logs => self.logs.render(frame, colors),
            Modal::AddTag | Modal::EditTag(_) | Modal::DeleteTag(_) | Modal::DeleteCard(_) => {
                self.action.render(frame, colors, db)
            }
        }
    }
}
