use bevy_state::state::States;

use crate::app::{App, InputState};

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
    Search,
    Logs,
    Custom,
    Confirmed,
    #[default]
    Canceled,
}

impl Modal {
    pub const fn is_active(self) -> bool {
        match self {
            Self::Search | Self::Logs | Self::Custom => true,
            Self::Confirmed | Self::Canceled => false,
        }
    }

    pub const fn as_input_state(self) -> InputState {
        match self {
            Self::Search | Self::Logs | Self::Custom => InputState::Modal,
            Self::Confirmed | Self::Canceled => InputState::Normal,
        }
    }
}
