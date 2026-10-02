use shared::symbols;
use terminal::*;
use widgets2::{
    Block, Image, ImageLoadOptions, ImageOptions, KittyError, KittyGraphics, List, ListIndex,
    ScrollMargins,
};

use crate::{
    app::{Action, Colors},
    database::{Database, TrackId},
    jukebox::Jukebox,
    modals::ModalAction,
};

type ImageResult = Result<Option<Image>, KittyError>;
type ImageHandle = std::thread::JoinHandle<ImageResult>;

const IMAGE_ID: u32 = 1;

pub struct PlayingPage {
    list: List,
    current_id: Option<TrackId>,
    current_qi: Option<usize>,
    image: Option<ImageResult>,
    image_handle: Option<ImageHandle>,
}

impl PlayingPage {
    pub const fn new() -> Self {
        Self {
            list: List::new()
                .with_scrollbar(0)
                .with_padding(Margin::horizontal(1)),
            current_id: None,
            current_qi: None,
            image: None,
            image_handle: None,
        }
    }

    pub fn on_enter(&self) {}

    pub fn on_exit(&self) {}

    pub fn update(&mut self, db: &Database, jb: &Jukebox, kitty: &mut KittyGraphics) -> bool {
        // Poll thread for finished image loading.
        // When finished, take the handle and join thread.

        let Some(handle) = self.image_handle.as_ref() else {
            return false;
        };

        if !handle.is_finished() {
            return false;
        }

        let Some(handle) = self.image_handle.take() else {
            return false;
        };

        let Ok(image) = handle.join() else {
            return false;
        };

        self.image = Some(image);
        true
    }

    pub fn refresh(&self) {}

    pub fn render(
        &mut self,
        area: Rect,
        frame: &mut Framebuffer,
        colors: &Colors,
        db: &Database,
        jb: &Jukebox,
    ) {
        self.update_scroll_on_new_track(jb);

        let (left, right) = area.split_left(area.cols() * 40 / 100, 2);
        self.render_cover(left, frame, colors, db, jb);
        self.render_queue(right, frame, colors, db, jb);
    }

    fn render_cover(
        &mut self,
        area: Rect,
        frame: &mut Framebuffer,
        colors: &Colors,
        db: &Database,
        jb: &Jukebox,
    ) {
        let Some(rating) = jb
            .current_track_id()
            .and_then(|id| db.get(id).map(|t| t.rating()))
        else {
            self.image = None;
            frame.push_str_fg("No track currently playing", colors.neutral);
            frame.render(
                area.inner(Margin::proportional(1)),
                TextOptions::paragraph_center(),
            );
            return;
        };

        let (_, cover_area, stars_area) = area.split_ends(1, 1);

        match self.image.as_ref() {
            Some(Ok(Some(image))) => {
                image.render(cover_area, frame, ImageOptions::fit_and_center());
            }
            Some(Ok(None)) => {
                frame.print_fmt(Sgr::Fg(colors.neutral));
                Block::rectangle().render(cover_area, frame);

                frame.push_str("No Image");
                frame.render(
                    cover_area.inner(Margin::proportional(1)),
                    TextOptions::span_center(),
                );

                frame.print_fmt(Sgr::reset_fg());
            }
            Some(Err(err)) => {
                frame.print_fmt(Sgr::Fg(colors.red));
                Block::rectangle().render(cover_area, frame);

                frame.push_fmt(format_args!("ERROR\n{err}"));
                frame.render(
                    cover_area.inner(Margin::proportional(1)),
                    TextOptions::paragraph_center(),
                );

                frame.print_fmt(Sgr::reset_fg());
            }
            None => {
                frame.print_fmt(Sgr::Fg(colors.neutral));
                Block::rectangle().render(cover_area, frame);

                frame.push_str("Loading");
                frame.render(
                    cover_area.inner(Margin::proportional(1)),
                    TextOptions::span_center(),
                );

                frame.print_fmt(Sgr::reset_fg());
            }
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

    pub fn load_front_cover(&mut self, path: &std::path::Path) {
        let path = path.to_path_buf();
        let handle = std::thread::spawn(move || {
            // TODO: Remove unwrap by reworking the AudioFileReport error.
            let cover = crate::database::AudioFrontCover::read(path).unwrap();
            match cover.bytes() {
                Some(bytes) => Image::from_bytes(
                    bytes,
                    &mut std::io::stdout().lock(),
                    IMAGE_ID,
                    ImageLoadOptions::max(Dims::SD),
                )
                .map(|img| Some(img)),
                None => Ok(None),
            }
        });
        self.image_handle = Some(handle);
    }
}
