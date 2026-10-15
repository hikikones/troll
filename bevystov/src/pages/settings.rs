use ecs::*;
use shared::symbols;
use terminal::*;
use widgets2::{Block, List, ListIndex, Prompt, ScrollMargins};

use crate::{
    app::{Action, Actions, App, Input, InputSet, PageArea, RenderSet, ShortcutsArea},
    database::AudioRating,
    modals::Logs,
    pages::Page,
    settings::{Colors, Config, Settings},
};

pub struct SettingsPagePlugin;

impl SettingsPagePlugin {
    pub fn build(app: &mut App) {
        let page = SettingsPage::new(app.world().resource::<Settings>());

        app.insert_resource(page)
            .insert_resource(SettingsList::default())
            .add_input(
                InputSet::Page,
                input_settings.run_if(in_state(Page::Settings)),
            )
            .add_render(
                RenderSet::Page,
                render_settings.run_if(in_state(Page::Settings)),
            );
    }
}

#[derive(Debug, Resource)]
struct SettingsPage {
    config: Config,
    primary: Prompt,
    secondary: Prompt,
    neutral: Prompt,
}

impl SettingsPage {
    fn new(settings: &Settings) -> Self {
        let config = settings.config().clone();
        let mut primary = Prompt::from(config.colors.primary.to_string());
        primary.move_to_end(false);
        let mut secondary = Prompt::from(config.colors.secondary.to_string());
        secondary.move_to_end(false);
        let mut neutral = Prompt::from(config.colors.neutral.to_string());
        neutral.move_to_end(false);

        Self {
            config,
            primary,
            secondary,
            neutral,
        }
    }
}

#[derive(Debug, Resource, Deref, DerefMut)]
struct SettingsList(List);

impl SettingsList {
    const fn current(&self) -> Setting {
        SETTINGS[self.0.index()]
    }

    const fn down(&mut self) -> bool {
        let mut next = self.0.index() + 1;
        while next < SETTINGS.len() {
            if SETTINGS[next].filter() {
                self.0.set_index(next);
                return true;
            }
            next += 1;
        }

        false
    }

    const fn up(&mut self) -> bool {
        if self.0.index() == 0 {
            return false;
        }

        let mut prev = self.0.index().saturating_sub(1);
        loop {
            if SETTINGS[prev].filter() {
                self.0.set_index(prev);
                return true;
            }

            if prev == 0 {
                break;
            }

            prev -= 1;
        }

        false
    }
}

impl Default for SettingsList {
    fn default() -> Self {
        let mut list = Self(
            List::new()
                .with_scrollbar(0)
                .with_scrolloff(ScrollMargins::vertical(3))
                .with_padding(Margin::proportional(1)),
        );

        if !SETTINGS[0].filter() {
            list.down();
        }

        list
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Setting {
    General,
    SkipRating,
    KeepTrackSort,
    SearchByPath,
    Colors,
    PrimaryColor,
    SecondaryColor,
    NeutralColor,
    Empty,
}

impl Setting {
    const fn filter(&self) -> bool {
        match self {
            Self::General => false,
            Self::SkipRating => true,
            Self::KeepTrackSort => true,
            Self::SearchByPath => true,
            Self::Colors => false,
            Self::PrimaryColor => true,
            Self::SecondaryColor => true,
            Self::NeutralColor => true,
            Self::Empty => false,
        }
    }

    const fn is_color(self) -> bool {
        matches!(
            self,
            Self::PrimaryColor | Self::SecondaryColor | Self::NeutralColor
        )
    }
}

const SETTINGS: [Setting; 11] = [
    Setting::General,
    Setting::Empty,
    Setting::SkipRating,
    Setting::KeepTrackSort,
    Setting::SearchByPath,
    Setting::Empty,
    Setting::Colors,
    Setting::Empty,
    Setting::PrimaryColor,
    Setting::SecondaryColor,
    Setting::NeutralColor,
];

fn input_settings(
    input: Res<Input>,
    mut list: ResMut<SettingsList>,
    mut settings: ResMut<Settings>,
    mut page: ResMut<SettingsPage>,
    mut logs: ResMut<Logs>,
    mut actions: ResMut<Actions>,
) {
    let Some(key) = input.get() else {
        return;
    };

    match key.code {
        KeyCode::Down => {
            if list.down() {
                actions.push(Action::Render);
            }
        }
        KeyCode::Up => {
            if list.up() {
                actions.push(Action::Render);
            }
        }
        KeyCode::Char('s') if key.ctrl() => {
            if settings.hash() != page.config.hash() {
                match settings.save() {
                    Ok(_) => {
                        page.config = settings.config().clone();
                    }
                    Err(err) => {
                        logs.push_err(err);
                    }
                }
                actions.push(Action::Render);
            }
        }
        KeyCode::Char('r') if key.ctrl() => {
            page.config = Config::default();
            settings.set_config(&page.config);

            page.primary.clear();
            page.primary.push_fmt(&settings.colors().primary);
            page.secondary.clear();
            page.secondary.push_fmt(&settings.colors().secondary);
            page.neutral.clear();
            page.neutral.push_fmt(&settings.colors().neutral);

            actions.push(Action::Render);
        }
        _ => {
            handle_setting(
                key,
                list.current(),
                &mut settings,
                &mut page,
                &mut logs,
                &mut actions,
            );
        }
    }
}

fn handle_setting(
    key: Key,
    current: Setting,
    settings: &mut Settings,
    page: &mut SettingsPage,
    logs: &mut Logs,
    actions: &mut Actions,
) {
    match current {
        Setting::SkipRating => {
            if let KeyCode::Char(c) = key.code
                && let Some(rating) = AudioRating::from_char(c)
                && settings.general().skip_tracks_with_rating != rating
            {
                settings.general_mut().skip_tracks_with_rating = rating;
                actions.push(Action::Render);
            }
        }
        Setting::KeepTrackSort => {
            if let KeyCode::Char(' ') = key.code {
                let toggle = !settings.general().keep_selected_track_on_sort;
                settings.general_mut().keep_selected_track_on_sort = toggle;
                actions.push(Action::Render);
            }
        }
        Setting::SearchByPath => {
            if let KeyCode::Char(' ') = key.code {
                let toggle = !settings.general().search_by_path;
                settings.general_mut().search_by_path = toggle;
                actions.push(Action::Render);
            }
        }
        Setting::PrimaryColor => match key.code {
            KeyCode::Enter => match page.primary.as_str_trim().parse::<Color>() {
                Ok(color) => {
                    settings.colors_mut().primary = color;
                    actions.push(Action::Render);
                }
                Err(err) => {
                    logs.push_err(err);
                    actions.push(Action::Render);
                }
            },
            _ => {
                if page.primary.input(key) {
                    actions.push(Action::Render);
                }
            }
        },
        Setting::SecondaryColor => match key.code {
            KeyCode::Enter => match page.secondary.as_str_trim().parse::<Color>() {
                Ok(color) => {
                    settings.colors_mut().secondary = color;
                    actions.push(Action::Render);
                }
                Err(err) => {
                    logs.push_err(err);
                    actions.push(Action::Render);
                }
            },
            _ => {
                if page.secondary.input(key) {
                    actions.push(Action::Render);
                }
            }
        },
        Setting::NeutralColor => match key.code {
            KeyCode::Enter => match page.neutral.as_str_trim().parse::<Color>() {
                Ok(color) => {
                    settings.colors_mut().neutral = color;
                    actions.push(Action::Render);
                }
                Err(err) => {
                    logs.push_err(err);
                    actions.push(Action::Render);
                }
            },
            _ => {
                if page.neutral.input(key) {
                    actions.push(Action::Render);
                }
            }
        },
        Setting::General | Setting::Colors | Setting::Empty => {}
    }
}

fn render_settings(
    area: Res<PageArea>,
    shortcuts_area: Res<ShortcutsArea>,
    mut frame: ResMut<Framebuffer>,
    mut list: ResMut<SettingsList>,
    mut page: ResMut<SettingsPage>,
    settings: Res<Settings>,
) {
    let area = **area;
    let colors = settings.colors();

    frame.print_fmt(Sgr::Fg(colors.secondary));
    Block::rectangle().render(area, &mut frame);

    let (top, body, bottom) = area.split_ends(1, 1);

    frame.print_span_with(top, " Settings ", SpanOptions::center_top());
    frame.print_fmt(Sgr::reset_fg());

    let current = list.current();
    list.set_colors(colors.list()).render(
        body.inner(Margin::horizontal(1)),
        &mut frame,
        SETTINGS.into_iter(),
        |line, frame, setting, index| {
            let (left, right) = line.split_vertically_with_gap(0);

            if index == ListIndex::Selected {
                frame.print_fmt(Sgr::Bold);
                frame.push_str("> ");
            }

            frame.print_fmt(Sgr::Fg(colors.normal));

            match setting {
                Setting::General => {
                    print_section("GENERAL", line, frame, colors);
                }
                Setting::SkipRating => {
                    frame.push_str("Skip tracks with rating:");
                    frame.render(left, TextOptions::span_right());

                    let (filled_stars, empty_stars) =
                        settings.general().skip_tracks_with_rating.stars_split();
                    frame.push_fmt(Sgr::Fg(colors.primary));
                    frame.push_str(filled_stars);
                    frame.push_fmt(Sgr::Fg(colors.neutral));
                    frame.push_str(empty_stars);
                    frame.push_fmt(Sgr::reset_fg());
                    frame.render(right, TextOptions::span());
                }
                Setting::KeepTrackSort => {
                    print_toggle(
                        "Keep selected track on sort:",
                        settings.general().keep_selected_track_on_sort,
                        left,
                        right,
                        frame,
                        colors,
                    );
                }
                Setting::SearchByPath => {
                    print_toggle(
                        "Search by path:",
                        settings.general().search_by_path,
                        left,
                        right,
                        frame,
                        colors,
                    );
                }
                Setting::Colors => {
                    print_section("COLORS", line, frame, colors);
                }
                Setting::PrimaryColor => {
                    print_color(
                        "Primary:",
                        &mut page.primary,
                        left,
                        right,
                        frame,
                        colors,
                        current == Setting::PrimaryColor,
                    );
                }
                Setting::SecondaryColor => {
                    print_color(
                        "Secondary:",
                        &mut page.secondary,
                        left,
                        right,
                        frame,
                        colors,
                        current == Setting::SecondaryColor,
                    );
                }
                Setting::NeutralColor => {
                    print_color(
                        "Neutral:",
                        &mut page.neutral,
                        left,
                        right,
                        frame,
                        colors,
                        current == Setting::NeutralColor,
                    );
                }
                Setting::Empty => {}
            }

            frame.print_fmt(Sgr::NotBold);
        },
    );

    let description = match current {
        Setting::SkipRating => "Skips tracks that are less than or equal to set rating",
        Setting::KeepTrackSort => "Scrolls to selected track when sorting",
        Setting::SearchByPath => "Includes directories and filename when searching",
        Setting::PrimaryColor | Setting::SecondaryColor | Setting::NeutralColor => {
            "Set color by name, hex code or indexed value"
        }
        Setting::General | Setting::Colors | Setting::Empty => "",
    };

    frame.print_fmt(Sgr::Fg(colors.neutral));
    frame.push_ch(' ');
    frame.push_str(description);
    frame.push_ch(' ');
    frame.render(bottom, TextOptions::span_center_top());

    render_shortcuts(
        **shortcuts_area,
        &mut frame,
        colors,
        current,
        &settings,
        &page.config,
    );

    frame.set_cursor_visibility(current.is_color());
}

fn render_shortcuts(
    area: Rect,
    frame: &mut Framebuffer,
    colors: &Colors,
    current: Setting,
    settings: &Settings,
    config: &Config,
) {
    let key_color = colors.secondary;
    let name_color = colors.normal;
    let gap = 1;

    fn push_shortcut(
        key: &str,
        name: &str,
        key_color: Color,
        name_color: Color,
        frame: &mut Framebuffer,
    ) {
        frame.push_fmt(Sgr::Fg(key_color));
        frame.push_str(key);
        frame.push_fmt(Sgr::Fg(name_color));
        frame.push_ch(' ');
        frame.push_str(name);
    }

    match current {
        Setting::SkipRating => {
            push_shortcut("0-5", "Rating", key_color, name_color, frame);
            frame.push_ch_repeat(' ', gap);
        }
        Setting::KeepTrackSort => {
            push_shortcut("Space", "Toggle", key_color, name_color, frame);
            frame.push_ch_repeat(' ', gap);
        }
        Setting::SearchByPath => {
            push_shortcut("Space", "Toggle", key_color, name_color, frame);
            frame.push_ch_repeat(' ', gap);
        }
        Setting::PrimaryColor | Setting::SecondaryColor | Setting::NeutralColor => {
            push_shortcut("↵", "Set Color", key_color, name_color, frame);
            frame.push_ch_repeat(' ', gap);
        }
        Setting::General | Setting::Colors | Setting::Empty => {}
    };

    if settings.hash() != config.hash() {
        push_shortcut("^s", "Save", key_color, name_color, frame);
        frame.push_ch_repeat(' ', gap);
    }

    push_shortcut("^r", "Reset All", key_color, name_color, frame);

    frame.push_fmt(Sgr::reset_fg());
    frame.render(area, TextOptions::span_center_top());
}

fn print_section(title: &str, line: Rect, frame: &mut Framebuffer, colors: &Colors) {
    frame.print_fmt(Sgr::Fg(colors.neutral));
    frame.print_span_with(line, title, SpanOptions::center_top());
    frame.print_fmt(Sgr::reset_fg());
}

fn print_toggle(
    description: &str,
    value: bool,
    left: Rect,
    right: Rect,
    frame: &mut Framebuffer,
    colors: &Colors,
) {
    frame.push_str(description);
    frame.render(left, TextOptions::span_right());

    let (toggle, color) = symbols::checkmark_with_value(value, colors.primary, colors.neutral);
    frame.print_fmt(Sgr::Fg(color));
    frame.print_span(right, toggle);
    frame.print_fmt(Sgr::reset_fg());
}

fn print_color(
    description: &str,
    prompt: &mut Prompt,
    left: Rect,
    right: Rect,
    frame: &mut Framebuffer,
    colors: &Colors,
    is_active: bool,
) {
    frame.push_str(description);
    frame.render(left, TextOptions::span_right());

    prompt
        .set_colors(colors.prompt())
        .set_disabled(!is_active)
        .render(right, frame);

    if is_active {
        frame.set_cursor_pos_at_end(prompt.get_cursor_pos());
    }
}
