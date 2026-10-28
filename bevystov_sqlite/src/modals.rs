use ecs::States;

use crate::app::App;

mod logs;
mod search;

pub use logs::*;

pub struct ModalsPlugin;

impl ModalsPlugin {
    pub fn build(app: &mut App) {
        app.add_state(Modal::default());

        search::SearchModalPlugin::build(app);
        logs::LogsModalPlugin::build(app);
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, States)]
pub enum Modal {
    #[default]
    None,
    Search,
    Logs,
}

impl Modal {
    pub const fn is_active(self) -> bool {
        match self {
            Self::None => false,
            _ => true,
        }
    }
}
