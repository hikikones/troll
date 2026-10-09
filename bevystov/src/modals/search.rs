use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{NonSend, NonSendMut, Res, ResMut},
};
use bevy_state::condition::in_state;
use terminal::*;
use widgets2::{Block, List, ListIndex, Prompt};

use crate::{
    app::{Action, Actions, App, Colors, Frame, Input, InputState, RenderSet},
    database::{Database, TrackId},
    jukebox::Jukebox,
    modals::Modal,
    pages::Route,
};

pub(super) struct SearchModalPlugin;

impl SearchModalPlugin {
    pub(super) fn build(app: &mut App) {
        app.insert_resource(SearchModal::default())
            .insert_resource(TrackResults::default())
            .add_enter(Modal::Search, enter_search)
            .add_exit(Modal::Search, exit_search)
            .add_input(
                InputState::Modal,
                input_search.run_if(in_state(Modal::Search)),
            )
            .add_render(
                RenderSet::Modal,
                render_search.run_if(in_state(Modal::Search)),
            )
            .add_update(update_search.run_if(in_state(Modal::Search)));
    }
}

#[derive(Debug, Resource)]
struct SearchModal {
    state: State,
    prompt: Prompt,
    prompt_hash: u64,
    include_path: bool,
    list: List,
}

impl SearchModal {
    fn reset(&mut self) {
        self.state = State::Search;
        self.prompt.clear();
        self.prompt.set_disabled(false);
        self.prompt_hash = 0;
        self.list.reset();
    }
}

impl Default for SearchModal {
    fn default() -> Self {
        Self {
            state: State::Search,
            prompt: Prompt::new().with_placeholder("Search..."),
            prompt_hash: 0,
            include_path: false,
            list: List::new()
                .with_scrollbar(0)
                .with_padding(Margin::horizontal(1)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum State {
    Search,
    Browse,
}

#[derive(Debug, Default, Resource, Deref, DerefMut)]
struct TrackResults(Vec<(TrackId, u16)>);

fn enter_search(mut frame: ResMut<Frame>, database: NonSend<Database>) {
    if database.is_empty() {
        return;
    }

    frame.cursor(Cursor::Show);
}

fn exit_search(
    mut frame: ResMut<Frame>,
    database: NonSend<Database>,
    mut modal: ResMut<SearchModal>,
    mut results: ResMut<TrackResults>,
) {
    if database.is_empty() {
        return;
    }

    modal.reset();
    results.clear();
    frame.cursor(Cursor::Hide);
}

fn input_search(
    input: Res<Input>,
    mut frame: ResMut<Frame>,
    mut modal: ResMut<SearchModal>,
    mut jukebox: NonSendMut<Jukebox>,
    mut actions: ResMut<Actions>,
    results: Res<TrackResults>,
    database: NonSend<Database>,
) {
    if database.is_empty() {
        return;
    }

    let Some(key) = input.get() else {
        return;
    };

    match modal.state {
        State::Search => match key.code {
            KeyCode::Enter | KeyCode::Down => {
                if !results.is_empty() {
                    modal.state = State::Browse;
                    modal.prompt.set_disabled(true);
                    modal.list.reset();
                    frame.cursor(Cursor::Hide);
                    actions.push(Action::Render);
                }
            }
            _ => {
                if modal.prompt.input(key) {
                    actions.push(Action::Render);
                }
            }
        },
        State::Browse => match key.code {
            KeyCode::Enter => {
                if let Some((id, _)) = results.get(modal.list.index()).copied() {
                    jukebox.play_id(id, &database);
                    actions.push(Action::Modal(Modal::Confirmed));
                }
            }
            KeyCode::Up => {
                if modal.list.index() == 0 {
                    modal.state = State::Search;
                    modal.prompt.set_disabled(false);
                    frame.cursor(Cursor::Show);
                    actions.push(Action::Render);
                } else {
                    if modal.list.input(key) {
                        actions.push(Action::Render);
                    }
                }
            }
            KeyCode::Char('q') => {
                let ids = modal
                    .list
                    .selection_inclusive()
                    .filter_map(|i| results.get(i).map(|(id, _)| *id));
                jukebox.extend(ids);
            }
            KeyCode::Char('n') => {
                modal
                    .list
                    .selection_inclusive()
                    .rev()
                    .filter_map(|i| results.get(i).map(|(id, _)| *id))
                    .for_each(|id| {
                        jukebox.enqueue_next(id);
                    });
            }
            KeyCode::Char('g') => {
                let index = modal.list.index();
                let id = results.get(index).map(|(id, _)| *id);
                // TODO: Goto does not work since app is in modal state.
                // Need to rework routing a bit.
                actions.push(Action::Route(Route::Tracks(id)));
            }
            KeyCode::Char('s') => {
                modal.state = State::Search;
                modal.prompt.set_disabled(false);
                frame.cursor(Cursor::Show);
                actions.push(Action::Render);
            }
            _ => {
                if modal.list.input(key) {
                    actions.push(Action::Render);
                }
            }
        },
    }
}

fn render_search(
    mut frame: ResMut<Frame>,
    colors: Res<Colors>,
    mut modal: ResMut<SearchModal>,
    track_results: Res<TrackResults>,
    database: NonSend<Database>,
    jukebox: NonSend<Jukebox>,
) {
    let area = frame.area().scale_and_center(0.6);
    let bg = frame.palette().background().slight_offset().as_color();
    frame.fill(area, bg, false);

    frame.print_fmt(Sgr::Fg(colors.secondary));
    Block::rectangle().render(area, &mut frame);

    // Prompt
    let prompt_area = area
        .with_size(Size::new(area.cols() / 2, 1))
        .center_horizontal(area);
    frame.cursor(prompt_area.pos);
    frame.print_ch_repeat(' ', prompt_area.cols());
    let prompt_area = prompt_area.inner(Margin::horizontal(1));
    modal.prompt.render(prompt_area, &mut frame);

    if database.is_empty() {
        frame.print_fmt(Sgr::Fg(colors.neutral));
        frame.push_str("No tracks to search for");
        frame.render(
            area.inner(Margin::proportional(1)),
            TextOptions::paragraph_center(),
        );
        frame.print_fmt(Sgr::Reset);
        return;
    }

    // Results
    let shortcuts_area = Rect::new(area.pos.with_row(area.bottom_row()), area.size.with_rows(1));
    let results_area = area.inner(Margin::all(1));
    let state = modal.state;

    modal.list.render(
        results_area,
        &mut frame,
        track_results.iter().copied(),
        |line, frame, (id, _), index| {
            let Some(track) = database.get(id) else {
                return;
            };

            let mut style = match state {
                State::Search => Style::fg(colors.neutral).with_bg(bg),
                State::Browse => match index {
                    ListIndex::Selected => {
                        frame.fill(line, colors.primary, true);
                        Style::fg(colors.primary).with_reverse()
                    }
                    ListIndex::Selection => {
                        frame.fill(line, colors.neutral, true);
                        Style::fg(colors.neutral).with_reverse()
                    }
                    ListIndex::Normal => Style::fg(colors.normal).with_bg(bg),
                },
            };

            if jukebox.is_faulty(id) {
                style.insert(Attributes::CROSSED_OUT);
            }

            frame.push_fmt(style);
            frame.push_str(track.title());
            frame.push_ch(' ');
            frame.push_str(track.artist());
            frame.push_ch(' ');
            frame.push_str(track.album());
            frame.push_fmt(Sgr::Reset);
            frame.render(line, TextOptions::span());
        },
    );

    if !track_results.is_empty() {
        frame.print_fmt(Sgr::Bg(bg));
        render_shortcuts(shortcuts_area, &mut frame, &colors, state);
        frame.print_fmt(Sgr::reset_bg());
    }

    frame.print_fmt(Sgr::Reset);

    frame.set_cursor_pos_at_end(modal.prompt.get_cursor_pos());
}

fn render_shortcuts(area: Rect, frame: &mut Framebuffer, colors: &Colors, state: State) {
    match state {
        State::Search => {
            frame.print_span_with_options(
                area,
                format_args!(
                    " {}↵{} Browse {}",
                    Sgr::Fg(colors.secondary),
                    Sgr::Fg(colors.normal),
                    Sgr::reset_fg()
                ),
                SpanOptions::center_top(),
            );
        }
        State::Browse => {
            let key_color = colors.secondary;
            let name_color = colors.normal;
            let gap = 1;

            frame.push_ch(' ');

            for (key, name, gap) in [
                ("↵", "Play", gap),
                ("q", "Enqueue", gap),
                ("n", "Play next", gap),
                ("g", "Goto", gap),
                ("s", "Search", 0),
            ] {
                frame.push_fmt(Sgr::Fg(key_color));
                frame.push_str(key);
                frame.push_fmt(Sgr::Fg(name_color));
                frame.push_ch(' ');
                frame.push_str(name);
                frame.push_ch_repeat(' ', gap);
            }

            frame.push_ch(' ');
            frame.push_fmt(Sgr::reset_fg());
            frame.render(area, TextOptions::span_center_top());
        }
    }
}

fn update_search(
    mut modal: ResMut<SearchModal>,
    mut results: ResMut<TrackResults>,
    mut database: NonSendMut<Database>,
    mut actions: ResMut<Actions>,
) {
    let hash = modal.prompt.hash_trim();

    if hash == modal.prompt_hash {
        return;
    }

    results.clear();
    modal.prompt_hash = hash;

    if modal.prompt.is_empty_trim() {
        actions.push(Action::Render);
        return;
    }

    results.extend(database.search(modal.prompt.as_str_trim(), modal.include_path));
    results.sort_by_key(|(_, score)| std::cmp::Reverse(*score));

    modal.list.reset();

    actions.push(Action::Render);
}
