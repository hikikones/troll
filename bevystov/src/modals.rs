use bevy_ecs::system::{Res, ResMut};
use bevy_state::state::{State, States};
use terminal::*;
use widgets2::Block;

use crate::app::{Action, Actions, App, Colors, Frame, Input, InputState, RenderSet};

pub struct ModalsPlugin;

impl ModalsPlugin {
    pub fn build(app: &mut App) {
        app.add_state(Modal::default());

        app.add_input(InputState::Modal, input_modals)
            .add_render(RenderSet::Modal, render_modals);
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
}

fn input_modals(key: Res<Input>, mut actions: ResMut<Actions>, modal: Res<State<Modal>>) {
    if let KeyCode::Esc = key.code {
        actions.push(Action::Quit);
        return;
    }

    match **modal {
        Modal::Search => match key.code {
            KeyCode::Char('f') if key.ctrl() => {
                actions.push(Action::Modal(Modal::Canceled));
            }
            _ => {}
        },
        Modal::Logs => match key.code {
            KeyCode::Char('l') if key.ctrl() => {
                actions.push(Action::Modal(Modal::Canceled));
            }
            _ => {}
        },
        Modal::Custom | Modal::Confirmed | Modal::Canceled => {}
    }
}

fn render_modals(modal: Res<State<Modal>>, mut frame: ResMut<Frame>, colors: Res<Colors>) {
    let area = frame.area();

    match **modal {
        Modal::Search => {
            let area = area.with_size(area.size / 2).center(area);

            let bg = frame.palette().background().slight_offset().as_color();
            Block::fill(Style::bg(bg)).render(area, &mut *frame);
            Block::rectangle(Style::fg(colors.normal).with_bg(bg)).render(area, &mut *frame);
            frame.push_str(" Search ");
            frame.render(area, TextOptions::span_center_top());

            frame.print_fmt(Sgrs([Sgr::Fg(colors.secondary), Sgr::Bold, Sgr::Reverse]));
            frame.push_str("YOLO");
            frame.render(
                area.inner(Margin::proportional(1)),
                TextOptions::span_center().with_fill(),
            );
            frame.print_fmt(Sgr::Reset);
        }
        Modal::Logs => {
            let area = area.with_size(area.size / 2).center(area);

            let bg = frame.palette().background().slight_offset().as_color();
            Block::fill(Style::bg(bg)).render(area, &mut *frame);
            Block::rectangle(Style::fg(colors.normal).with_bg(bg)).render(area, &mut *frame);
            frame.push_str(" Logs ");
            frame.render(area, TextOptions::span_center_top());
        }
        Modal::Custom | Modal::Confirmed | Modal::Canceled => {}
    }
}
