use bevy_ecs::system::{Res, ResMut};
use bevy_state::state::{State, States};
use terminal::*;
use widgets2::Block;

use crate::app::{App, Colors, Frame, InputState, RenderSet};

pub struct ModalsPlugin;

impl ModalsPlugin {
    pub fn build(app: &mut App) {
        app.add_state(Modal::default())
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

    pub const fn as_input_state(self) -> InputState {
        match self {
            Self::Search | Self::Logs | Self::Custom => InputState::Modal,
            Self::Confirmed | Self::Canceled => InputState::Normal,
        }
    }
}

fn render_modals(modal: Res<State<Modal>>, mut frame: ResMut<Frame>, colors: Res<Colors>) {
    let area = frame.area();

    match **modal {
        Modal::Search => {
            let area = area.with_size(area.size / 2).center(area);

            let bg = frame.palette().background().slight_offset().as_color();
            frame.fill(area, bg);

            Block::rectangle(Style::fg(colors.normal).with_bg(bg)).render(area, &mut *frame);
            frame.print_span_with_options(area, " Search ", SpanOptions::center_top());

            frame.print_span_with_options(
                area.inner(Margin::proportional(1)),
                Styled::new(
                    "YOLO",
                    Style::fg(colors.secondary).with_bold().with_reverse(),
                ),
                SpanOptions::center().with_fill(),
            );
        }
        Modal::Logs => {
            let area = area.with_size(area.size / 2).center(area);

            let bg = frame.palette().background().slight_offset().as_color();
            frame.fill(area, bg);

            Block::rectangle(Style::fg(colors.normal).with_bg(bg)).render(area, &mut *frame);
            frame.push_str(" Logs ");
            frame.render(area, TextOptions::span_center_top());
        }
        Modal::Custom | Modal::Confirmed | Modal::Canceled => {}
    }
}
