mod logs;

pub use logs::*;

#[derive(Debug, Clone, Copy)]
pub enum Modal {
    Search,
    Logs,
    Custom,
}

pub struct Modals {
    pub current: Option<Modal>,
    pub logs: LogsModal,
}

impl Modals {
    pub const fn new() -> Self {
        Self {
            current: None,
            logs: LogsModal::new(),
        }
    }
}
