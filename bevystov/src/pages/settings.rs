use bevy_ecs::{
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Res, ResMut},
};
use bevy_state::condition::in_state;
use terminal::*;
use widgets2::{AnsiViewer, Block};

use crate::{
    app::{
        Action, Actions, App, Colors, Frame, Input, InputState, PageArea, RenderSet, ShortcutsArea,
    },
    pages::Page,
};

pub struct SettingsPagePlugin;

impl SettingsPagePlugin {
    pub fn build(app: &mut App) {
        app.insert_resource(Reader::default())
            .insert_resource(Lorem::default())
            .add_input(
                InputState::Normal,
                input_settings.run_if(in_state(Page::Settings)),
            )
            .add_render(
                RenderSet::Page,
                render_settings.run_if(in_state(Page::Settings)),
            );
    }
}

fn input_settings(input: Res<Input>, mut reader: ResMut<Reader>, mut actions: ResMut<Actions>) {
    let Some(key) = input.get() else {
        return;
    };

    if reader.0.input(key.code) {
        actions.push(Action::Render);
    }
}

fn render_settings(
    area: Res<PageArea>,
    shortcuts_area: Res<ShortcutsArea>,
    mut frame: ResMut<Frame>,
    colors: Res<Colors>,
    mut reader: ResMut<Reader>,
    lorem: Res<Lorem>,
) {
    let area = **area;

    Block::rectangle().render(area, &mut frame);
    reader
        .0
        .render(area.inner(Margin::all(1)), &mut frame, &lorem.0);

    render_shortcuts(**shortcuts_area, &mut frame, &colors);
}

fn render_shortcuts(area: Rect, frame: &mut Framebuffer, _colors: &Colors) {
    frame.print_span_with_options(area, "TODO", SpanOptions::center_top());
}

#[derive(Debug, Resource)]
struct Reader(AnsiViewer);

impl Default for Reader {
    fn default() -> Self {
        Self(
            AnsiViewer::new()
                .with_alignment(HorizontalAlignment::Center)
                .with_scrollbar(0)
                .with_padding(Margin::horizontal(1)),
        )
    }
}

#[derive(Debug, Resource)]
struct Lorem(String);

impl Default for Lorem {
    fn default() -> Self {
        use std::fmt::Write;

        let mut s = String::new();

        write!(s, "First line.\n\n").unwrap();

        write!(s, "{}", Sgr::Fg(Color::Red)).unwrap();
        write!(s, "Duis ac orci nec ex bibendum dignissim at nec velit. Ut porttitor elit orci, et luctus velit facilisis sit amet. Duis consectetur fermentum lectus in aliquet. In dictum tempus magna eget rhoncus. Nullam dolor diam, laoreet id mauris mattis, porttitor suscipit turpis. Integer vestibulum metus tortor, et consectetur odio facilisis eu. Maecenas orci ante, vulputate vel luctus ut, blandit consequat justo. Curabitur vehicula vitae erat ac gravida. Sed vitae lacinia odio.").unwrap();
        write!(s, "{}", Sgr::reset_fg()).unwrap();

        write!(
            s,
            "\n\nHere is some {}BOLD REVERSED TEXT{}.\n\n",
            Sgrs([Sgr::Bold, Sgr::Reverse]),
            Sgr::Reset
        )
        .unwrap();

        write!(s, "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Integer magna eros, imperdiet a erat consectetur, commodo suscipit tellus. In ac luctus sem, quis eleifend libero. Duis finibus est nec aliquet pulvinar. In pharetra, massa a interdum fermentum, ligula lorem suscipit ante, at tincidunt diam est sed nisl. In hac habitasse platea dictumst. Sed ornare nulla a ullamcorper suscipit. Aenean euismod, dui eget consectetur vulputate, eros nibh hendrerit ipsum, at semper enim elit in tellus. Sed eget maximus elit, eget vehicula purus. In ut dolor eu ipsum accumsan euismod nec sit amet risus. Nulla facilisi. Fusce faucibus sollicitudin dui at ultricies. Maecenas interdum ipsum ac vestibulum vehicula. Phasellus rutrum quam metus, eget pulvinar lorem sagittis ac. Suspendisse nec magna a mauris viverra posuere. Pellentesque ac massa sed arcu tempor mollis. Donec id nisi quis ligula blandit consequat quis sed nibh.").unwrap();
        write!(s, "\n\n").unwrap();
        write!(s, "Donec efficitur orci ac porttitor bibendum. Phasellus placerat magna id lorem dictum, ut consequat orci tincidunt. Interdum et malesuada fames ac ante ipsum primis in faucibus. Praesent lobortis a mauris mollis consectetur. Ut hendrerit eget urna quis euismod. Nulla vel iaculis nibh, id consequat lectus. Nullam vel ullamcorper nisl, vel facilisis odio. Class aptent taciti sociosqu ad litora torquent per conubia nostra, per inceptos himenaeos. Lorem ipsum dolor sit amet, consectetur adipiscing elit. Curabitur condimentum lectus lectus. Aliquam sodales in mi ut tempus. Curabitur turpis urna, tempus eleifend leo in, gravida luctus magna. Maecenas ac nunc magna.").unwrap();

        write!(s, "\n\nLast line.").unwrap();

        Self(s)
    }
}
