use bevy_ecs::{
    schedule::IntoScheduleConfigs,
    system::{Res, ResMut},
};
use bevy_state::condition::in_state;
use terminal::*;
use widgets2::Block;

use crate::{
    app::{App, Colors, Frame, Input, InputState, PageArea, RenderSet},
    pages::Page,
};

pub struct SettingsPagePlugin;

impl SettingsPagePlugin {
    pub fn build(app: &mut App) {
        app.add_input(
            InputState::Normal,
            input_settings.run_if(in_state(Page::Settings)),
        )
        .add_render(
            RenderSet::Page,
            render_settings.run_if(in_state(Page::Settings)),
        );
    }
}

fn input_settings(key: Res<Input>) {
    match key.code {
        _ => {
            // TODO
        }
    }
}

fn render_settings(area: Res<PageArea>, mut frame: ResMut<Frame>, colors: Res<Colors>) {
    let area = **area;

    Block::rectangle(colors.neutral).render(area, &mut frame);
    frame.push_str("SETTINGS");
    frame.render(area, TextOptions::span_center());
}
