use ecs::*;
use terminal::*;
use widgets2::{Block, List, ListIndex, Prompt, Text, TextOptions};

use crate::{
    app::{Action, Actions, App, Input, InputSet, RenderSet},
    database::{Database, TrackId},
    jukebox::Jukebox,
    modals::Modal,
    pages::Route,
    settings::{Colors, Settings},
};

pub(super) struct SearchModalPlugin;

impl SearchModalPlugin {
    pub(super) fn build(app: &mut App) {
        app.insert_resource(SearchModal::default())
            .insert_resource(SearchResults::default())
            .add_exit(Modal::Search, exit_search)
            .add_input(
                InputSet::Modal,
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
    state: SearchState,
    prompt: Prompt,
    prompt_hash: u64,
    include_path: bool,
    list: List,
}

impl SearchModal {
    fn reset(&mut self) {
        self.state = SearchState::Prompt;
        self.prompt.clear();
        self.prompt.set_disabled(false);
        self.prompt_hash = 0;
        self.list.reset();
    }
}

impl Default for SearchModal {
    fn default() -> Self {
        Self {
            state: SearchState::Prompt,
            prompt: Prompt::new().with_placeholder("Search..."),
            prompt_hash: 0,
            include_path: false,
            list: List::new().with_scrollbar(0).with_padding(Margin::all(1)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchState {
    Prompt,
    Browse,
}

#[derive(Debug, Default, Resource, Deref, DerefMut)]
struct SearchResults(Vec<(TrackId, u16)>);

fn exit_search(
    database: NonSend<Database>,
    mut modal: ResMut<SearchModal>,
    mut results: ResMut<SearchResults>,
) {
    if database.is_empty() {
        return;
    }

    modal.reset();
    results.clear();
}

fn input_search(
    input: Res<Input>,
    mut modal: ResMut<SearchModal>,
    mut jukebox: NonSendMut<Jukebox>,
    mut actions: ResMut<Actions>,
    results: Res<SearchResults>,
    database: NonSend<Database>,
) {
    if database.is_empty() {
        return;
    }

    let Some(key) = input.get() else {
        return;
    };

    match modal.state {
        SearchState::Prompt => match key.code {
            KeyCode::Enter | KeyCode::Down => {
                if !results.is_empty() {
                    modal.state = SearchState::Browse;
                    modal.list.reset();
                    actions.push(Action::Render);
                }
            }
            _ => {
                if modal.prompt.input(key) {
                    actions.push(Action::Render);
                }
            }
        },
        SearchState::Browse => match key.code {
            KeyCode::Enter => {
                if let Some((id, _)) = results.get(modal.list.index()).copied() {
                    jukebox.play_id(id, &database);
                    actions.push(Action::Modal(Modal::None));
                }
            }
            KeyCode::Up => {
                if modal.list.index() == 0 {
                    modal.state = SearchState::Prompt;
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
                actions.push(Action::Route(Route::Tracks(id)));
            }
            KeyCode::Char('s') => {
                modal.state = SearchState::Prompt;
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
    mut frame: ResMut<Framebuffer>,
    mut text: ResMut<Text>,
    settings: Res<Settings>,
    mut modal: ResMut<SearchModal>,
    results: Res<SearchResults>,
    database: NonSend<Database>,
    jukebox: NonSend<Jukebox>,
) {
    let area = frame.area().scale_and_center(0.7);
    let colors = settings.colors();
    let state = modal.state;
    let bg = colors.background;

    frame.fill(area, bg, false);

    frame.print_fmt(Sgr::Fg(colors.secondary));
    Block::rectangle().render(area, &mut frame);

    let (prompt_area, results_area, shortcuts_area) = area.split_ends(1, 1);

    // Prompt
    let prompt_area = prompt_area.scale_cols_and_center(0.5);
    frame.cursor(prompt_area.pos);
    frame.print_ch_repeat(' ', prompt_area.cols());
    modal
        .prompt
        .set_disabled(state == SearchState::Browse)
        .render(prompt_area.inner(Margin::horizontal(1)), &mut frame);

    if database.is_empty() {
        frame.set_cursor_visibility(false);
        modal.prompt.set_disabled(true);

        frame.print_fmt(Sgr::Fg(colors.neutral));
        text.push_str("No tracks to search for");
        text.render(
            area.inner(Margin::proportional(1)),
            &mut frame,
            TextOptions::paragraph_center(),
        );
        frame.print_fmt(Sgr::Reset);
        return;
    }

    // Results
    modal.list.set_colors(colors.list()).render(
        results_area.inner(Margin::horizontal(1)),
        &mut frame,
        results.iter().copied(),
        |line, frame, (id, _), index| {
            let Some(track) = database.get(id) else {
                return;
            };

            let mut style = match state {
                SearchState::Prompt => Style::fg(colors.neutral).with_bg(bg),
                SearchState::Browse => match index {
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

            text.push_fmt(style);
            text.push_str(track.title());
            text.push_ch(' ');
            text.push_str(track.artist());
            text.push_ch(' ');
            text.push_str(track.album());
            text.push_fmt(Sgr::Reset);
            text.render(line, frame, TextOptions::span());
        },
    );

    if !results.is_empty() {
        frame.print_fmt(Sgr::Bg(bg));
        render_shortcuts(shortcuts_area, &mut frame, &mut text, colors, state);
    }

    frame.print_fmt(Sgr::Reset);

    // Update cursor state
    frame.set_cursor_visibility(modal.state == SearchState::Prompt);
    frame.set_cursor_pos_at_end(modal.prompt.get_cursor_pos());
}

fn render_shortcuts(
    area: Rect,
    frame: &mut Framebuffer,
    text: &mut Text,
    colors: &Colors,
    state: SearchState,
) {
    match state {
        SearchState::Prompt => {
            text.push_fmt(format_args!(
                " {}↵{} Browse {}",
                Sgr::Fg(colors.secondary),
                Sgr::Fg(colors.normal),
                Sgr::reset_fg()
            ));
            text.render(area, frame, TextOptions::span_center_top());
        }
        SearchState::Browse => {
            let key_color = colors.secondary;
            let name_color = colors.normal;
            let gap = 1;

            text.push_ch(' ');

            for (key, name, gap) in [
                ("↵", "Play", gap),
                ("q", "Enqueue", gap),
                ("n", "Play next", gap),
                ("g", "Goto", gap),
                ("s", "Search", 0),
            ] {
                text.push_fmt(Sgr::Fg(key_color));
                text.push_str(key);
                text.push_fmt(Sgr::Fg(name_color));
                text.push_ch(' ');
                text.push_str(name);
                text.push_ch_repeat(' ', gap);
            }

            text.push_ch(' ');
            text.render(area, frame, TextOptions::span_center_top());
        }
    }
}

fn update_search(
    mut modal: ResMut<SearchModal>,
    mut results: ResMut<SearchResults>,
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
