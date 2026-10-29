use ecs::*;
use shared::symbols;
use terminal::*;
use widgets2::{
    Block, Image, ImageLoadOptions, ImageOptions, KittyDeleteAll, KittyError, ScrollMargins,
    list2::List,
};

use crate::{
    app::{Action, Actions, App, Input, InputSet, PageArea, RenderSet, ShortcutsArea},
    database::{AudioExt, AudioFile, AudioFileError, Database, TrackId, TrackRating},
    jukebox::Jukebox,
    modals::Logs,
    pages::{Page, Route},
    settings::{Colors, Settings},
};

type ImageResult = Result<Option<Image>, LoadImageError>;
type ImageHandle = std::thread::JoinHandle<ImageResult>;

pub struct PlayingPagePlugin;

impl PlayingPagePlugin {
    pub fn build(app: &mut App) {
        app.insert_resource(PlayingPage::default())
            .insert_non_send(FrontCover::default())
            .add_input(
                InputSet::Page,
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
    view: ViewLayout,
}

impl PlayingPage {
    fn update_scroll_on_new_track(&mut self, jb: &Jukebox) {
        let current_queue_index = jb.queue_index();

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
            view: ViewLayout::Horizontal,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ViewLayout {
    Cover,
    Horizontal,
    Vertical,
}

impl ViewLayout {
    const fn from(screen_size: (ScreenWidth, ScreenHeight)) -> Self {
        match screen_size {
            (ScreenWidth::Narrow, ScreenHeight::Short) => Self::Cover,
            (ScreenWidth::Narrow, ScreenHeight::Normal) => Self::Vertical,
            (ScreenWidth::Narrow, ScreenHeight::Tall) => Self::Vertical,
            (ScreenWidth::Normal, ScreenHeight::Tall) => Self::Vertical,
            (_, _) => Self::Horizontal,
        }
    }

    const fn shows_queue(self) -> bool {
        !matches!(self, Self::Cover)
    }
}

#[derive(Debug, Default)]
struct FrontCover {
    state: CoverState,
    handle: Option<ImageHandle>,
}

#[derive(Debug, Default)]
enum CoverState {
    #[default]
    None,
    Loading,
    NoImage,
    Error,
    Image(Image),
}

impl FrontCover {
    fn clear(&mut self) {
        self.state = CoverState::default();
        self.handle = None;
    }

    fn render(&self, area: Rect, frame: &mut Framebuffer, colors: &Colors) -> Option<Rect> {
        let msg = match &self.state {
            CoverState::None => return None,
            CoverState::Loading => {
                frame.print_fmt(Sgr::Fg(colors.neutral));
                Block::rectangle().render(area, frame);
                frame.print_span_with(
                    area.inner(Margin::proportional(1)),
                    "Loading",
                    SpanOptions::center(),
                );
                frame.print_fmt(Sgr::reset_fg());
                return None;
            }
            CoverState::NoImage => "No image",
            CoverState::Error => "Failed to load image",
            CoverState::Image(image) => {
                return Some(image.render(area, frame, ImageOptions::fit_center()));
            }
        };

        frame.print_fmt(Sgr::Fg(colors.neutral));
        Block::rectangle().render(area, frame);
        frame.push_str(msg);
        frame.render(
            area.inner(Margin::proportional(1)),
            TextOptions::paragraph_center(),
        );
        frame.print_fmt(Sgr::reset_fg());

        Some(area)
    }
}

fn input_playing(
    input: Res<Input>,
    mut page: ResMut<PlayingPage>,
    mut actions: ResMut<Actions>,
    mut database: NonSendMut<Database>,
    mut jukebox: NonSendMut<Jukebox>,
) {
    if jukebox.is_empty() {
        return;
    }

    let Some(key) = input.get() else {
        return;
    };

    let shows_queue = page.view.shows_queue();

    match key.code {
        KeyCode::Enter => {
            let index = page.list.index();
            jukebox.play_index(index, &database);
        }
        KeyCode::Char(c) => match c {
            '0' | '1' | '2' | '3' | '4' | '5' => {
                if let Some(id) = jukebox.track_id() {
                    let rating = TrackRating::from_char(c).unwrap();
                    if database.write_rating(id, rating) {
                        actions.push(Action::Render);
                    }
                }
            }
            'c' if shows_queue => {
                if jukebox.clear() {
                    actions.push(Action::Render);
                }
            }
            's' if shows_queue => {
                if jukebox.shuffle() {
                    actions.push(Action::Render);
                }
            }
            'g' => {
                let index = page.list.index();
                let id = jukebox.get(index);
                actions.push(Action::Route(Route::Tracks(id)));
            }
            'm' if shows_queue => {
                let (start, end) = page.list.selection_inclusive().into_inner();

                let success = if start == end {
                    jukebox.move_down(start)
                } else {
                    jukebox.move_down_range(start, end)
                };

                if success {
                    page.current_qi = jukebox.queue_index();
                    page.list.move_selection_down();
                    actions.push(Action::Render);
                }
            }
            'M' if shows_queue => {
                let (start, end) = page.list.selection_inclusive().into_inner();

                let success = if start == end {
                    jukebox.move_up(start)
                } else {
                    jukebox.move_up_range(start, end)
                };

                if success {
                    page.current_qi = jukebox.queue_index();
                    page.list.move_selection_up();
                    actions.push(Action::Render);
                }
            }
            'r' if shows_queue => {
                let (start, end) = page.list.selection_inclusive().into_inner();

                let success = if start == end {
                    jukebox.remove(start)
                } else {
                    jukebox.remove_range(start, end)
                };

                if success {
                    page.current_qi = jukebox.queue_index();
                    page.list.set_index(start).set_selector(None);
                    actions.push(Action::Render);
                }
            }
            _ => {
                if shows_queue && page.list.input(key) {
                    actions.push(Action::Render);
                }
            }
        },
        _ => {
            if shows_queue && page.list.input(key) {
                actions.push(Action::Render);
            }
        }
    }
}

fn render_playing(
    area: Res<PageArea>,
    shortcuts_area: Res<ShortcutsArea>,
    mut frame: ResMut<Framebuffer>,
    screen_size: Res<ScreenSize>,
    settings: Res<Settings>,
    database: NonSend<Database>,
    jukebox: NonSend<Jukebox>,
    mut page: ResMut<PlayingPage>,
    cover: NonSend<FrontCover>,
) {
    let colors = settings.colors();

    // Always delete image
    frame.print_fmt(KittyDeleteAll);

    // Render layout based on screen size
    page.view = ViewLayout::from(screen_size.both());
    let (cover_area, queue_area) = match page.view {
        ViewLayout::Cover => {
            render_cover(**area, &mut frame, colors, &database, &jukebox, &cover);
            return;
        }
        ViewLayout::Horizontal => area.split_vertically_with_gap(0),
        ViewLayout::Vertical => area.split_top(area.rows() * 60 / 100, 1),
    };

    page.update_scroll_on_new_track(&jukebox);

    render_cover(cover_area, &mut frame, colors, &database, &jukebox, &cover);
    render_queue(
        queue_area,
        **shortcuts_area,
        &mut frame,
        colors,
        &database,
        &jukebox,
        &mut page.list,
    );
}

fn render_cover(
    area: Rect,
    frame: &mut Framebuffer,
    colors: &Colors,
    database: &Database,
    jukebox: &Jukebox,
    cover: &FrontCover,
) {
    let Some(rating) = jukebox.track_id().map(|id| database.track(id).rating) else {
        frame.push_str_fg("No track currently playing", colors.neutral);
        frame.render(
            area.inner(Margin::proportional(1)),
            TextOptions::paragraph_center(),
        );
        return;
    };

    if area.size.rows > 15 {
        if let Some(image_area) = cover.render(area.inner(Margin::vertical(1)), frame, colors) {
            let (filled_stars, empty_stars) = rating.stars_split();
            frame.push_fmt(Sgr::Fg(colors.primary));
            frame.push_str(filled_stars);
            frame.push_fmt(Sgr::Fg(colors.neutral));
            frame.push_str(empty_stars);
            frame.push_fmt(Sgr::reset_fg());
            frame.render(
                image_area.with_row(image_area.bottom_out()),
                TextOptions::span_center_top(),
            );
        }
    } else {
        cover.render(area, frame, colors);
    }
}

fn render_queue(
    area: Rect,
    shortcuts_area: Rect,
    frame: &mut Framebuffer,
    colors: &Colors,
    database: &Database,
    jukebox: &Jukebox,
    list: &mut List,
) {
    frame.print_fmt(Sgr::Fg(colors.secondary));
    Block::rectangle().render(area, frame);

    frame.print_fmt(Sgr::Fg(colors.normal));
    frame.push_fmt(format_args!(
        " History ({}) / Queue ({}) ",
        jukebox.history(),
        jukebox.queue()
    ));
    frame.render(area, TextOptions::span_center_top());

    if jukebox.is_empty() {
        frame.print_fmt(Sgr::Fg(colors.neutral));
        frame.print_span_with(
            area.inner(Margin::proportional(1)),
            "No tracks in the queue",
            SpanOptions::center(),
        );
        frame.print_fmt(Sgr::reset_fg());
        return;
    }

    let inner = area.inner(Margin::all(1));
    let scrolloff = inner.rows() / 2;

    let Some((list_area, lines)) = list
        .set_scrolloff(ScrollMargins::all(scrolloff))
        .set_len(jukebox.len())
        .set_colors(colors.list())
        .render(inner, frame)
    else {
        return;
    };

    let line = list_area.with_rows(1);
    let selected = list.index();
    let selection = list.selection_inclusive();
    let hlen = jukebox.history();
    let current_qi = jukebox.queue_index();
    let mut row = list_area.pos.row;

    for (index, (id, qi)) in lines.zip(jukebox.iter().skip(list.scroll())) {
        let fg = if qi < hlen {
            colors.neutral
        } else if current_qi == Some(qi) {
            colors.primary
        } else {
            colors.normal
        };

        let symbol = if index == selected {
            symbols::concat!(symbols::SELECTED, " ")
        } else if selection.contains(&index) {
            symbols::concat!(symbols::SELECTION, " ")
        } else {
            ""
        };

        frame.push_fmt(Sgr::Fg(fg));
        frame.push_str(symbol);

        if jukebox.is_faulty(id) {
            frame.push_fmt(Sgr::CrossedOut);
        }

        frame.push_str(database.track_title(id));

        frame.push_fmt(Sgr::Reset);
        frame.render(line.with_row(row), TextOptions::span());

        row += 1;
    }

    render_shortcuts(shortcuts_area, frame, colors);
}

fn render_shortcuts(area: Rect, frame: &mut Framebuffer, colors: &Colors) {
    let key_color = colors.secondary;
    let name_color = colors.normal;
    let gap = 1;

    for (key, name, gap) in [
        ("↵", "Play", gap),
        ("0-5", "Rating", gap),
        ("(⇧)m", "Move", gap),
        ("s", "Shuffle", gap),
        ("r", "Remove", gap),
        ("c", "Clear", gap),
        ("g", "Goto", 0),
    ] {
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

fn exit_playing(mut frame: ResMut<Framebuffer>) {
    frame.print_fmt(KittyDeleteAll);
}

fn update_cover(
    mut page: ResMut<PlayingPage>,
    mut cover: NonSendMut<FrontCover>,
    jukebox: NonSend<Jukebox>,
) {
    let id = jukebox.track_id();

    if page.current_id == id {
        return;
    }

    page.current_id = id;

    let Some(track) = jukebox.track() else {
        cover.clear();
        return;
    };

    cover.state = CoverState::Loading;
    cover.handle = Some(load_front_cover(
        track.path().to_path_buf(),
        track.ext(),
        page.next_image_id(),
    ));
}

fn poll_cover(
    mut cover: NonSendMut<FrontCover>,
    mut logs: ResMut<Logs>,
    mut actions: ResMut<Actions>,
) {
    if let Some(handle) = cover.handle.as_ref()
        && handle.is_finished()
        && let Some(handle) = cover.handle.take()
        && let Ok(result) = handle.join()
    {
        match result {
            Ok(image) => {
                cover.state = image
                    .map(|img| CoverState::Image(img))
                    .unwrap_or(CoverState::NoImage);
            }
            Err(err) => {
                logs.push_err(err);
                cover.state = CoverState::Error;
            }
        }

        actions.push(Action::Render);
    }
}

fn load_front_cover(path: std::path::PathBuf, ext: AudioExt, image_id: u32) -> ImageHandle {
    std::thread::spawn(move || {
        let audio_file = AudioFile::read_cover(&path, ext).map_err(|err| LoadImageError {
            message: format!("Failed to read front cover image from '{}'", path.display()),
            source: Some(LoadErrorSource::Audio(err)),
        })?;

        audio_file
            .front_cover()
            .map(|bytes| {
                Image::from_bytes(
                    bytes,
                    &mut std::io::stdout().lock(),
                    image_id,
                    ImageLoadOptions::max(Dims::all(512)),
                )
            })
            .transpose()
            .map_err(|err| LoadImageError {
                message: format!("Failed to load front cover image from '{}'", path.display()),
                source: Some(LoadErrorSource::Image(err)),
            })
    })
}

#[derive(Debug)]
struct LoadImageError {
    message: String,
    source: Option<LoadErrorSource>,
}

#[derive(Debug)]
enum LoadErrorSource {
    Audio(AudioFileError),
    Image(KittyError),
}

impl std::fmt::Display for LoadImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for LoadImageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|err| match err {
            LoadErrorSource::Audio(err) => err as _,
            LoadErrorSource::Image(err) => err as _,
        })
    }
}
