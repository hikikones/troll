use shared::symbols;
use terminal::*;
use widgets2::{Block, Image, ImageOptions, KittyGraphics, List, ListIndex, ScrollMargins};

use crate::{
    app::{Action, Colors},
    database::{Database, TrackId},
    jukebox::Jukebox,
    modals::ModalAction,
};

type LoadImageHandle = std::thread::JoinHandle<Result<Option<Vec<u8>>, String>>;

fn load_front_cover(path: &std::path::Path) -> LoadImageHandle {
    let path = path.to_path_buf();
    std::thread::spawn(move || {
        let front_cover = crate::database::AudioFrontCover::read(path)?;
        match front_cover.bytes() {
            Some(bytes) => Ok(Some(bytes.to_vec())),
            None => Ok(None),
        }
    })
}

pub struct PlayingPage {
    list: List,
    current_id: Option<TrackId>,
    current_qi: Option<usize>,
    image: Image,
    image_handle: Option<LoadImageHandle>,
    image_loaded: bool,
}

impl PlayingPage {
    pub const fn new() -> Self {
        Self {
            list: List::new()
                .with_scrollbar(0)
                .with_padding(Margin::horizontal(1)),
            current_id: None,
            current_qi: None,
            image: Image::new(1),
            image_handle: None,
            image_loaded: false,
        }
    }

    pub fn on_enter(&self) {}

    pub fn on_exit(&self) {}

    pub fn update(&mut self, db: &Database, jb: &Jukebox, kitty: &mut KittyGraphics) {
        let current_id = jb.current_track_id();

        // Check for new track and start image load if so
        if self.current_id != current_id {
            self.current_id = current_id;
            if let Some(id) = current_id
                && let Some(path) = db.get(id).map(|t| t.path())
            {
                self.image_handle = Some(load_front_cover(path));
            }
        }

        // Poll thread for finished image loading
        if let Some(handle) = self.image_handle.as_ref() {
            if handle.is_finished() {
                let handle = self.image_handle.take().unwrap();
                match handle.join().unwrap() {
                    Ok(Some(bytes)) => {
                        // TODO: log error
                        match self.image.load_from_bytes(bytes, kitty) {
                            Ok(_) => {
                                self.image_loaded = true;
                            }
                            Err(_) => {
                                self.image_loaded = false;
                                // TODO: log error
                            }
                        }
                    }
                    Ok(None) => {
                        self.image_loaded = false;
                    }
                    Err(err) => {
                        // TODO: log error
                    }
                }
            }
        }
    }

    pub fn refresh(&self) {}

    pub fn render(
        &mut self,
        area: Rect,
        frame: &mut Framebuffer,
        colors: &Colors,
        db: &Database,
        jb: &Jukebox,
        kitty: &KittyGraphics,
    ) {
        self.update_scroll_on_new_track(jb);

        let (left, right) = area.split_vertically_with_gap(0);
        self.render_cover(left, frame, colors, db, jb, kitty);
        self.render_queue(right, frame, colors, db, jb);
    }

    fn render_cover(
        &mut self,
        area: Rect,
        frame: &mut Framebuffer,
        colors: &Colors,
        db: &Database,
        jb: &Jukebox,
        kitty: &KittyGraphics,
    ) {
        let Some(rating) = self
            .current_id
            .and_then(|id| db.get(id).map(|t| t.rating()))
        else {
            frame.push_str_fg("No track currently playing", colors.neutral);
            frame.render(area, TextOptions::span_center());
            return;
        };

        let (_, cover_area, stars_area) = area.split_ends(1, 1);

        if self.image_loaded {
            self.image
                .render(cover_area, frame, kitty, ImageOptions::fit_and_center());
        } else {
            frame.push_str_fg("No image", colors.neutral);
            frame.render(area, TextOptions::span_center());
        }

        let (filled_stars, empty_stars) = rating.stars_split();
        frame.push_str_fg(filled_stars, colors.primary);
        frame.push_str_fg(empty_stars, colors.neutral);
        frame.render(stars_area, TextOptions::span_center_top());
    }

    fn render_queue(
        &mut self,
        area: Rect,
        frame: &mut Framebuffer,
        colors: &Colors,
        db: &Database,
        jb: &Jukebox,
    ) {
        Block::rectangle()
            .with_color(colors.secondary)
            .render(area, frame);
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
        self.list.set_scrolloff(ScrollMargins::all(scrolloff));

        let hlen = jb.history();
        let current_qi = jb.current_queue_index();

        self.list
            .render(inner, frame, jb.iter(), |line, frame, (id, qi), index| {
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

    pub fn input(&mut self, key: Key, db: &mut Database, jb: &mut Jukebox) -> Action {
        match key.code {
            KeyCode::Enter => {
                let index = self.list.index();
                jb.play_index(index, db);
            }
            _ => {
                if self.list.input(key) {
                    return Action::Render;
                }
            }
        }
        Action::None
    }

    pub fn render_modal(&self, area: Rect, frame: &mut Framebuffer, colors: &Colors) {
        //todo
    }

    pub fn input_modal(&self, key: Key) -> ModalAction {
        //todo
        ModalAction::None
    }

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
}
