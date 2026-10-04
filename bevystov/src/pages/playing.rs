use bevy_ecs::{
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{NonSend, NonSendMut, Res, ResMut},
};
use bevy_state::condition::in_state;
use shared::symbols;
use terminal::*;
use widgets2::{
    Block, Image, ImageLoadOptions, ImageOptions, KittyDeleteAll, KittyError, List, ListIndex,
    ScrollMargins,
};

use crate::{
    app::{Action, Actions, App, Colors, Frame, Input, InputState, PageArea, RenderSet},
    database::{AudioRating, Database, TrackId},
    jukebox::Jukebox,
    pages::{Page, Route},
};

type ImageResult = Result<Option<Image>, KittyError>;
type ImageHandle = std::thread::JoinHandle<ImageResult>;

pub struct PlayingPagePlugin;

impl PlayingPagePlugin {
    pub fn build(app: &mut App) {
        app.insert_resource(PlayingPage::default())
            .insert_non_send(FrontCover::default())
            .add_input(
                InputState::Normal,
                input_playing.run_if(in_state(Page::NowPlaying)),
            )
            .add_enter(Page::NowPlaying, update_cover)
            .add_exit(Page::NowPlaying, exit_playing)
            .add_render(
                RenderSet::Page,
                render_playing.run_if(in_state(Page::NowPlaying)),
            )
            .add_update((update_cover, poll_cover).chain());
    }
}

#[derive(Debug, Resource)]
struct PlayingPage {
    list: List,
    current_id: Option<TrackId>,
    current_qi: Option<usize>,
    image_id: u8,
}

impl PlayingPage {
    fn update_scroll_on_new_track(&mut self, jb: &Jukebox) {
        let current_queue_index = jb.current_queue_index();

        if self.current_qi == current_queue_index {
            return;
        }

        self.current_qi = current_queue_index;

        if let Some(idx) = current_queue_index {
            self.list.set_index(idx).set_selector(None);
        }
    }

    const fn next_image_id(&mut self) -> u32 {
        self.image_id = if self.image_id == 1 { 2 } else { 1 };
        self.image_id as u32
    }
}

impl Default for PlayingPage {
    fn default() -> Self {
        Self {
            list: List::new()
                .with_scrollbar(0)
                .with_padding(Margin::horizontal(1)),
            current_id: None,
            current_qi: None,
            image_id: 1,
        }
    }
}

#[derive(Debug, Default)]
struct FrontCover {
    result: Option<ImageResult>,
    handle: Option<ImageHandle>,
}

impl FrontCover {
    fn clear(&mut self) {
        self.result = None;
        self.handle = None;
    }
}

fn input_playing(
    key: Res<Input>,
    mut page: ResMut<PlayingPage>,
    mut actions: ResMut<Actions>,
    mut db: NonSendMut<Database>,
    mut jb: NonSendMut<Jukebox>,
) {
    if jb.is_empty() {
        return;
    }

    match key.code {
        KeyCode::Enter => {
            let index = page.list.index();
            jb.play_index(index, &db);
        }
        KeyCode::Char(c) => match c {
            '0' | '1' | '2' | '3' | '4' | '5' => {
                if let Some(id) = jb.current_track_id() {
                    let rating = AudioRating::from_char(c).unwrap();
                    db.write_rating(id, rating);
                }
            }
            'c' => {
                if jb.clear() {
                    actions.push(Action::Render);
                }
            }
            's' => {
                if jb.shuffle() {
                    actions.push(Action::Render);
                }
            }
            'g' => {
                let index = page.list.index();
                let id = jb.get(index);
                actions.push(Action::Route(Route::Tracks(id)));
            }
            'm' => {
                let (start, end) = {
                    let selection = page.list.selection_inclusive();
                    (*selection.start(), *selection.end())
                };

                let success = if start == end {
                    jb.move_down(start)
                } else {
                    jb.move_down_range(start, end)
                };

                if success {
                    page.current_qi = jb.current_queue_index();
                    page.list.move_selection_down();
                    actions.push(Action::Render);
                }
            }
            'M' => {
                let (start, end) = {
                    let selection = page.list.selection_inclusive();
                    (*selection.start(), *selection.end())
                };

                let success = if start == end {
                    jb.move_up(start)
                } else {
                    jb.move_up_range(start, end)
                };

                if success {
                    page.current_qi = jb.current_queue_index();
                    page.list.move_selection_up();
                    actions.push(Action::Render);
                }
            }
            'r' => {
                let (start, end) = {
                    let selection = page.list.selection_inclusive();
                    (*selection.start(), *selection.end())
                };

                let success = if start == end {
                    jb.remove(start)
                } else {
                    jb.remove_range(start, end)
                };

                if success {
                    page.current_qi = jb.current_queue_index();
                    page.list.set_index(start).set_selector(None);
                    actions.push(Action::Render);
                }
            }
            _ => {
                if page.list.input(**key) {
                    actions.push(Action::Render);
                }
            }
        },
        _ => {
            if page.list.input(**key) {
                actions.push(Action::Render);
            }
        }
    }
}

fn render_playing(
    area: Res<PageArea>,
    mut frame: ResMut<Frame>,
    colors: Res<Colors>,
    database: NonSend<Database>,
    jukebox: NonSend<Jukebox>,
    mut page: ResMut<PlayingPage>,
    cover: NonSend<FrontCover>,
) {
    page.update_scroll_on_new_track(&jukebox);

    let (left, right) = area.split_left(area.cols() * 40 / 100, 2);
    render_cover(left, &mut *frame, &colors, &database, &jukebox, &cover);
    render_queue(
        right,
        &mut *frame,
        &colors,
        &database,
        &jukebox,
        &mut page.list,
    );
}

fn render_cover(
    area: Rect,
    frame: &mut Framebuffer,
    colors: &Colors,
    db: &Database,
    jb: &Jukebox,
    cover: &FrontCover,
) {
    // Always delete image
    frame.print_fmt(KittyDeleteAll);

    let Some(rating) = jb
        .current_track_id()
        .and_then(|id| db.get(id).map(|t| t.rating()))
    else {
        frame.push_str_fg("No track currently playing", colors.neutral);
        frame.render(
            area.inner(Margin::proportional(1)),
            TextOptions::paragraph_center(),
        );
        return;
    };

    let (_, cover_area, mut stars_area) = area.split_ends(1, 1);
    let mut image_area = stars_area;

    match cover.result.as_ref() {
        Some(Ok(Some(image))) => {
            image_area = image.render(cover_area, frame, ImageOptions::fit_center());
        }
        Some(Ok(None)) => {
            Block::rectangle(colors.neutral).render(cover_area, frame);
            frame.print_fmt(Sgr::Fg(colors.neutral));
            frame.push_str("No Image");
            frame.render(
                cover_area.inner(Margin::proportional(1)),
                TextOptions::span_center(),
            );
            frame.print_fmt(Sgr::reset_fg());
        }
        Some(Err(err)) => {
            Block::rectangle(colors.red).render(cover_area, frame);
            frame.print_fmt(Sgr::Fg(colors.red));
            frame.push_fmt(format_args!("ERROR\n{err}"));
            frame.render(
                cover_area.inner(Margin::proportional(1)),
                TextOptions::paragraph_center(),
            );
            frame.print_fmt(Sgr::reset_fg());
        }
        None => {
            frame.push_str_fg("Loading", colors.neutral);
            frame.render(
                cover_area.inner(Margin::proportional(1)),
                TextOptions::span_center(),
            );
        }
    }

    stars_area.pos.row = stars_area.row().min(image_area.bottom_out());

    let (filled_stars, empty_stars) = rating.stars_split();
    frame.push_str_fg(filled_stars, colors.primary);
    frame.push_str_fg(empty_stars, colors.neutral);
    frame.render(stars_area, TextOptions::span_center_top());
}

fn render_queue(
    area: Rect,
    frame: &mut Framebuffer,
    colors: &Colors,
    db: &Database,
    jb: &Jukebox,
    list: &mut List,
) {
    Block::rectangle(colors.secondary).render(area, frame);
    frame.push_fmt_fg(
        format_args!(" History ({}) / Queue ({}) ", jb.history(), jb.queue()),
        colors.normal,
    );
    frame.render(area, TextOptions::span_center_top());

    if jb.is_empty() {
        frame.push_str_fg("No tracks in the queue", colors.neutral);
        frame.render(
            area.inner(Margin::proportional(1)),
            TextOptions::span_center(),
        );
        return;
    }

    let inner = area.inner(Margin::all(1));
    let scrolloff = inner.rows() / 2;
    list.set_scrolloff(ScrollMargins::all(scrolloff));

    let hlen = jb.history();
    let current_qi = jb.current_queue_index();

    list.render(inner, frame, jb.iter(), |line, frame, (id, qi), index| {
        let Some(track) = db.get(id) else {
            return;
        };

        let fg = if qi < hlen {
            colors.neutral
        } else if current_qi == Some(qi) {
            colors.primary
        } else {
            colors.normal
        };

        let symbol = match index {
            ListIndex::Selected => symbols::concat!(symbols::SELECTED, " "),
            ListIndex::Selection => symbols::concat!(symbols::SELECTION, " "),
            ListIndex::Normal => "",
        };

        frame.push_fmt(Sgr::Fg(fg));
        frame.push_str(symbol);

        if jb.is_faulty(id) {
            frame.push_fmt(Sgr::CrossedOut);
        }

        frame.push_fmt(track.title());
        frame.push_ch(' ');
        frame.push_fmt(track.artist());
        frame.push_ch(' ');
        frame.push_fmt(track.album());
        frame.push_ch(' ');
        frame.push_fmt(Sgr::Reset);
        frame.render(line, TextOptions::span());
    });
}

fn exit_playing(mut frame: ResMut<Frame>) {
    frame.print_fmt(KittyDeleteAll);
}

fn update_cover(
    mut page: ResMut<PlayingPage>,
    mut cover: NonSendMut<FrontCover>,
    jukebox: NonSend<Jukebox>,
    database: NonSend<Database>,
) {
    let id = jukebox.current_track_id();

    if page.current_id == id {
        return;
    }

    page.current_id = id;

    let Some(id) = id else {
        cover.clear();
        return;
    };

    let Some(track) = database.get(id) else {
        cover.clear();
        return;
    };

    cover.handle = Some(load_front_cover(track.path(), page.next_image_id()));

    fn load_front_cover(path: &std::path::Path, image_id: u32) -> ImageHandle {
        let path = path.to_path_buf();
        std::thread::spawn(move || {
            // TODO: Remove unwrap by reworking the AudioFileReport error.
            let cover = crate::database::AudioFrontCover::read(path).unwrap();
            match cover.bytes() {
                Some(bytes) => Image::from_bytes(
                    bytes,
                    &mut std::io::stdout().lock(),
                    image_id,
                    ImageLoadOptions::max(Dims::SD),
                )
                .map(|img| Some(img)),
                None => Ok(None),
            }
        })
    }
}

fn poll_cover(mut cover: NonSendMut<FrontCover>, mut actions: ResMut<Actions>) {
    if let Some(handle) = cover.handle.as_ref()
        && handle.is_finished()
        && let Some(handle) = cover.handle.take()
        && let Ok(image) = handle.join()
    {
        cover.result = Some(image);
        actions.push(Action::Render);
    }
}
