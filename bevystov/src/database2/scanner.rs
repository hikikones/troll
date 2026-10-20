use std::{
    ops::Range,
    path::{Path, PathBuf},
};

use utils::StringExt;

use crate::database2::{AudioExt, AudioFile, AudioResult, TrackDuration, TrackRating};

pub type ScanReceiver = std::sync::mpsc::Receiver<ScannedResult>;
pub type ScannedResult = AudioResult<ScannedTrack>;

// TODO: recv -> scan event

pub struct Scanner;

impl Scanner {
    pub fn start(dir: PathBuf) -> ScanReceiver {
        let (sender, receiver) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let mut scan = ScannedTrack::new();

            traverse_files(dir, true, |path, ext| {
                let result = process_file(path, ext, &mut scan);
                let _ = sender.send(result);

                scan.clear();
            });
        });

        receiver
    }
}

#[derive(Clone)]
pub struct ScannedTrack {
    data: String,
    title: Range<usize>,
    rating: TrackRating,
    duration: TrackDuration,
    artists: Vec<Range<usize>>,
    album: Range<usize>,
    path: Range<usize>,
}

impl ScannedTrack {
    const fn new() -> Self {
        Self {
            data: String::new(),
            title: 0..0,
            rating: TrackRating::None,
            duration: TrackDuration::new(0),
            artists: Vec::new(),
            album: 0..0,
            path: 0..0,
        }
    }

    pub fn title(&self) -> &str {
        &self.data[self.title.clone()]
    }

    pub const fn rating(&self) -> TrackRating {
        self.rating
    }

    pub const fn duration(&self) -> TrackDuration {
        self.duration
    }

    pub fn artists(&self) -> impl Iterator<Item = &str> {
        self.artists.iter().map(|r| &self.data[r.clone()])
    }

    pub fn album(&self) -> &str {
        &self.data[self.album.clone()]
    }

    pub fn path(&self) -> &str {
        &self.data[self.path.clone()]
    }

    fn clear(&mut self) {
        self.data.clear();
        self.title = 0..0;
        self.rating = TrackRating::None;
        self.duration = TrackDuration::new(0);
        self.artists.clear();
        self.album = 0..0;
        self.path = 0..0;
    }
}

fn process_file(path: &Path, ext: AudioExt, track: &mut ScannedTrack) -> ScannedResult {
    let audio = AudioFile::read(path, ext)?;
    let tag = audio.vorbis_comments()?;
    let p = audio.properties();

    // Track
    track.title = track
        .data
        .push_str_get_range(tag.get("TITLE").unwrap_or(""));
    track.rating = tag
        .get("RATING")
        .map(TrackRating::from_vorbis_comments)
        .unwrap_or(TrackRating::None);
    track.duration = TrackDuration::from_duration(p.duration);

    // Artists
    for artist in tag.get_all("ARTIST") {
        let range = track.data.push_str_get_range(artist);
        track.artists.push(range);
    }

    // Album
    track.album = track
        .data
        .push_str_get_range(tag.get("ALBUM").unwrap_or(""));

    // Path
    track.path = track.data.push_str_get_range(&path.to_string_lossy());

    Ok(track.clone())
}

fn traverse_files(root: impl AsRef<Path>, follow_links: bool, mut f: impl FnMut(&Path, AudioExt)) {
    walkdir::WalkDir::new(root)
        .follow_links(follow_links)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .for_each(|entry| {
            let path = entry.path();
            if let Some(ext) = AudioExt::from_path(path) {
                f(path, ext);
            }
        });
}
