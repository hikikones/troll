use ecs::*;
use terminal::*;
use widgets2::{Block, Prompt, list2::List};

use crate::{
    app::{Action, Actions, App, Input, InputSet, RenderSet},
    database2::{Database, TrackId},
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
    _include_path: bool, // TODO
    list: List,
}

impl SearchModal {
    fn reset(&mut self) {
        self.state = SearchState::Prompt;
        self.prompt.clear();
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
            _include_path: false,
            list: List::new()
                .with_scrollbar(0)
                .with_padding(Margin::horizontal(1)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchState {
    Prompt,
    Browse,
}

#[derive(Debug, Default, Resource, Deref, DerefMut)]
struct SearchResults(Vec<TrackId>);

fn exit_search(
    database: NonSend<Database>,
    mut modal: ResMut<SearchModal>,
    mut results: ResMut<SearchResults>,
) {
    if database.is_tracks_empty() {
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
    let Some(key) = input.get() else {
        return;
    };

    if database.is_tracks_empty() {
        return;
    }

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
                if let Some(id) = results.get(modal.list.index()).copied() {
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
                    .filter_map(|i| results.get(i).map(|id| *id));
                jukebox.extend(ids);
            }
            KeyCode::Char('n') => {
                modal
                    .list
                    .selection_inclusive()
                    .rev()
                    .filter_map(|i| results.get(i).map(|id| *id))
                    .for_each(|id| {
                        jukebox.enqueue_next(id);
                    });
            }
            KeyCode::Char('g') => {
                let index = modal.list.index();
                let id = results.get(index).copied();
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
    settings: Res<Settings>,
    mut modal: ResMut<SearchModal>,
    results: Res<SearchResults>,
    mut database: NonSendMut<Database>,
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
    let no_tracks = database.is_tracks_empty();

    // Prompt
    let prompt_area = prompt_area.scale_cols_and_center(0.5);
    frame.cursor(prompt_area.pos);
    frame.print_ch_repeat(' ', prompt_area.cols());
    modal
        .prompt
        .set_colors(colors.prompt())
        .set_disabled(state == SearchState::Browse || no_tracks)
        .render(prompt_area.inner(Margin::horizontal(1)), &mut frame);

    if no_tracks {
        frame.print_fmt(Sgr::Fg(colors.neutral));
        frame.push_str("No tracks to search for");
        frame.render(
            area.inner(Margin::proportional(1)),
            TextOptions::paragraph_center(),
        );
        frame.print_fmt(Sgr::Reset);
        return;
    }

    let Some((list_area, lines)) = modal
        .list
        .set_len(results.len())
        // .set_colors(colors.list()) TODO
        .render(results_area.inner(Margin::horizontal(1)), &mut frame)
    else {
        return;
    };

    let mut line = list_area.with_rows(1);
    let selected = modal.list.index();
    let selection = modal.list.selection_inclusive();
    let input = modal.prompt.as_str_trim();

    for (index, id) in lines.zip(results.iter().copied().skip(modal.list.scroll())) {
        let (mut style, highlight) = match state {
            SearchState::Prompt => (
                Style::fg(colors.neutral).with_bg(bg),
                Some((Sgr::Reverse, Sgr::NotReverse)),
            ),
            SearchState::Browse => {
                if index == selected {
                    frame.fill(line, colors.primary, true);
                    (Style::fg(colors.primary).with_reverse(), None)
                } else if selection.contains(&index) {
                    frame.fill(line, colors.neutral, true);
                    (Style::fg(colors.neutral).with_reverse(), None)
                } else {
                    (
                        Style::fg(colors.normal).with_bg(bg),
                        Some((Sgr::Reverse, Sgr::NotReverse)),
                    )
                }
            }
        };

        if jukebox.is_faulty(id) {
            style.insert(Attributes::CROSSED_OUT);
        }

        frame.push_fmt(style);

        match highlight {
            Some((start, end)) => {
                let _ = database.search_highlight(id, input, start, end, |s| frame.push_str(s));
            }
            None => {
                database.track_text(id, |track| {
                    frame.push_str(track.title);
                    frame.push_ch(' ');
                    frame.push_str(track.artists);
                    frame.push_ch(' ');
                    frame.push_str(track.album);
                });
            }
        }

        frame.push_fmt(Sgr::Reset);
        frame.render(line, TextOptions::span());

        line.add_row(1);
    }

    if !results.is_empty() {
        frame.print_fmt(Sgr::Bg(bg));
        render_shortcuts(shortcuts_area, &mut frame, colors, state);
    }

    frame.print_fmt(Sgr::Reset);
}

fn render_shortcuts(area: Rect, frame: &mut Framebuffer, colors: &Colors, state: SearchState) {
    match state {
        SearchState::Prompt => {
            frame.push_fmt(format_args!(
                " {}↵{} Browse {}",
                Sgr::Fg(colors.secondary),
                Sgr::Fg(colors.normal),
                Sgr::reset_fg()
            ));
            frame.render(area, TextOptions::span_center_top());
        }
        SearchState::Browse => {
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
            frame.render(area, TextOptions::span_center_top());
        }
    }
}

fn update_search(
    mut modal: ResMut<SearchModal>,
    mut results: ResMut<SearchResults>,
    database: NonSendMut<Database>,
    mut actions: ResMut<Actions>,
) {
    let hash = modal.prompt.hash_trim();

    if hash == modal.prompt_hash {
        return;
    }

    results.clear();
    modal.prompt_hash = hash;

    let input = modal.prompt.as_str_trim();

    if input.is_empty() {
        actions.push(Action::Render);
        return;
    }

    if let Err(_err) = database.search(input, |id| results.push(id)) {
        // TODO
    }

    modal.list.reset();

    actions.push(Action::Render);
}
