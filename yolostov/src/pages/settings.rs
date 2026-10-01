use terminal::*;

use crate::{
    app::{Action, Colors},
    modals::ModalAction,
};

pub struct SettingsPage;

impl SettingsPage {
    pub const fn new() -> Self {
        Self
    }

    pub fn on_enter(&self) {}

    pub fn on_exit(&self) {}

    pub fn on_update(&self) {}

    pub fn render(&self, area: Rect, frame: &mut Framebuffer, colors: &Colors) {
        // TODO
    }

    pub fn input(&self, key: Key) -> Action {
        //todo
        Action::None
    }

    pub fn render_modal(&self, area: Rect, frame: &mut Framebuffer, colors: &Colors) {
        //todo
    }

    pub fn input_modal(&self, key: Key) -> ModalAction {
        //todo
        ModalAction::None
    }
}
