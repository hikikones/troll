use terminal::*;
use widgets2::{Block, List, Prompt, ScrollMargins};

use crate::{
    app::Action,
    modals::LogsModal,
    settings::{Colors, Config, Settings},
};

pub struct SettingsPage {
    config: Config,
    primary: Prompt,
    secondary: Prompt,
    neutral: Prompt,
    list: List,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Setting {
    General,
    DesiredRetention,
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
            Self::DesiredRetention => true,
            Self::Colors => false,
            Self::PrimaryColor => true,
            Self::SecondaryColor => true,
            Self::NeutralColor => true,
            Self::Empty => false,
        }
    }
}

const SETTINGS: [Setting; 9] = [
    Setting::General,
    Setting::Empty,
    Setting::DesiredRetention,
    Setting::Empty,
    Setting::Colors,
    Setting::Empty,
    Setting::PrimaryColor,
    Setting::SecondaryColor,
    Setting::NeutralColor,
];

impl SettingsPage {
    pub fn new(settings: &Settings) -> Self {
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
            list: List::new()
                .with_index(2)
                .with_scrollbar(0)
                .with_scrolloff(ScrollMargins::vertical(3))
                .with_padding(Margin::proportional(1)),
        }
    }

    pub fn enter(&self) {}

    pub fn exit(&self) {}

    pub fn refresh(&self) {}

    pub fn input(&mut self, key: Key, settings: &mut Settings, logs: &mut LogsModal) -> Action {
        match key.code {
            KeyCode::Down => {
                if self.down() {
                    return Action::Render;
                }
            }
            KeyCode::Up => {
                if self.up() {
                    return Action::Render;
                }
            }
            KeyCode::Char('s') if key.ctrl() => {
                if settings.hash() != self.config.hash() {
                    match settings.save() {
                        Ok(_) => {
                            self.config = settings.config().clone();
                        }
                        Err(err) => {
                            logs.push_err(err);
                        }
                    }
                    return Action::Render;
                }
            }
            KeyCode::Char('r') if key.ctrl() => {
                self.config = Config::default();
                settings.set_config(&self.config);

                self.primary.clear();
                self.secondary.clear();
                self.neutral.clear();

                let colors = settings.colors();
                self.primary.push_fmt(colors.primary);
                self.secondary.push_fmt(colors.secondary);
                self.neutral.push_fmt(colors.neutral);

                return Action::Render;
            }
            _ => {
                return self.handle_setting(key, settings, logs);
            }
        }

        Action::None
    }

    pub fn render(&mut self, area: Rect, frame: &mut Framebuffer, settings: &Settings) {
        let colors = settings.colors();

        frame.print_fmt(Sgr::Fg(colors.secondary));
        Block::rectangle().render(area, frame);
        frame.print_fmt(Sgr::Fg(colors.normal));

        let (top, body, bottom) = area.inner(Margin::horizontal(1)).split_ends(1, 1);

        frame.print_span(top, " Settings ", SpanOptions::center_top());

        let Some((list_area, lines)) = self
            .list
            .set_len(SETTINGS.len())
            .set_colors(colors.list())
            .render(body, frame)
        else {
            return;
        };

        let mut line = list_area.with_rows(1);
        let (mut left, mut right) = line.split_vertically_with_gap(0);
        let selected = self.list.index();
        let current = self.current();
        let mut row = line.row();

        for (index, setting) in lines.zip(SETTINGS.into_iter().skip(self.list.scroll())) {
            line.set_row(row);
            left.set_row(row);
            right.set_row(row);

            if index == selected {
                frame.print_fmt(Sgr::Bold);
                frame.push_str("> ");
            }

            frame.print_fmt(Sgr::Fg(colors.normal));

            match setting {
                Setting::General => {
                    print_section("GENERAL", line, frame, colors);
                }
                Setting::DesiredRetention => {
                    frame.push_str("Desired retention:");
                    frame.render(left, TextOptions::span_right());

                    frame.push_fmt(format_args!("{}%", settings.desired_retention()));
                    frame.render(right, TextOptions::span());
                }
                Setting::Colors => {
                    print_section("COLORS", line, frame, colors);
                }
                Setting::PrimaryColor => {
                    print_color(
                        "Primary:",
                        &mut self.primary,
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
                        &mut self.secondary,
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
                        &mut self.neutral,
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
            row += 1;
        }

        let description = match current {
            Setting::DesiredRetention => "Set desired retention in percent",
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
    }

    pub fn render_shortcuts(&self, area: Rect, frame: &mut Framebuffer, settings: &Settings) {
        let gap = 1;
        let key_color = settings.colors().secondary;
        let name_color = settings.colors().normal;
        let current = self.current();
        let is_dr = current == Setting::DesiredRetention;
        let is_color = matches!(
            current,
            Setting::PrimaryColor | Setting::SecondaryColor | Setting::NeutralColor
        );
        let is_hash_neq = settings.hash() != self.config.hash();

        for (key, name, gap, _show) in [
            ("⇄", "Increment/Decrement", gap, is_dr),
            ("↵", "Set color", gap, is_color),
            ("^s", "Save", gap, is_hash_neq),
            ("^r", "Reset all", 0, is_hash_neq),
        ]
        .into_iter()
        .filter(|s| s.3)
        {
            frame.push_fmt(Sgr::Fg(key_color));
            frame.push_str(key);
            frame.push_fmt(Sgr::Fg(name_color));
            frame.push_ch(' ');
            frame.push_str(name);
            frame.push_ch_repeat(' ', gap);
        }

        frame.push_fmt(Sgr::reset_fg());
        frame.render(area, TextOptions::span_center_top());
    }

    fn handle_setting(
        &mut self,
        key: Key,
        settings: &mut Settings,
        logs: &mut LogsModal,
    ) -> Action {
        match self.current() {
            Setting::DesiredRetention => {
                if let KeyCode::Left | KeyCode::Right = key.code {
                    let current_retention = settings.desired_retention();
                    let increment = key.code == KeyCode::Right;
                    let new_retention = if increment {
                        (current_retention + 1).min(100)
                    } else {
                        current_retention.saturating_sub(1)
                    };
                    if current_retention != new_retention {
                        settings.set_desired_retention(new_retention);
                        return Action::Render;
                    }
                }
            }
            Setting::PrimaryColor => match key.code {
                KeyCode::Enter => {
                    match self.primary.as_str_trim().parse::<Color>() {
                        Ok(color) => {
                            settings.colors_mut().primary = color;
                        }
                        Err(err) => {
                            logs.push_err(err);
                        }
                    }
                    return Action::Render;
                }
                _ => {
                    if self.primary.input(key) {
                        return Action::Render;
                    }
                }
            },
            Setting::SecondaryColor => match key.code {
                KeyCode::Enter => {
                    match self.secondary.as_str_trim().parse::<Color>() {
                        Ok(color) => {
                            settings.colors_mut().secondary = color;
                        }
                        Err(err) => {
                            logs.push_err(err);
                        }
                    }
                    return Action::Render;
                }
                _ => {
                    if self.secondary.input(key) {
                        return Action::Render;
                    }
                }
            },
            Setting::NeutralColor => match key.code {
                KeyCode::Enter => {
                    match self.neutral.as_str_trim().parse::<Color>() {
                        Ok(color) => {
                            settings.colors_mut().neutral = color;
                        }
                        Err(err) => {
                            logs.push_err(err);
                        }
                    }
                    return Action::Render;
                }
                _ => {
                    if self.neutral.input(key) {
                        return Action::Render;
                    }
                }
            },
            Setting::General | Setting::Colors | Setting::Empty => {}
        }

        Action::None
    }

    const fn current(&self) -> Setting {
        SETTINGS[self.list.index()]
    }

    const fn down(&mut self) -> bool {
        let mut next = self.list.index() + 1;
        while next < SETTINGS.len() {
            if SETTINGS[next].filter() {
                self.list.set_index(next);
                return true;
            }
            next += 1;
        }

        false
    }

    const fn up(&mut self) -> bool {
        if self.list.index() == 0 {
            return false;
        }

        let mut prev = self.list.index().saturating_sub(1);
        loop {
            if SETTINGS[prev].filter() {
                self.list.set_index(prev);
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

fn print_section(title: &str, line: Rect, frame: &mut Framebuffer, colors: &Colors) {
    frame.print_span_fg(line, title, colors.neutral, SpanOptions::center_top());
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
}
