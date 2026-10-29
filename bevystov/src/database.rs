use std::{
    collections::HashMap,
    hash::Hasher,
    path::{Path, PathBuf},
};

use shared::sqlite::*;

mod audio;
mod scanner;
mod track;
mod writer;

pub use audio::*;
use scanner::*;
pub use track::*;
use utils::StringExt;
use writer::*;

pub type DatabaseResult<T> = Result<T, DatabaseError>;

pub type TextRange = std::range::Range<usize>;

pub struct Track<'a> {
    pub title: &'a str,
    pub duration: TrackDuration,
    pub rating: TrackRating,
    pub bit_rate_kbps: u32,
    pub bit_depth: Option<u8>,
    pub sample_rate: Option<u32>,
    pub album: &'a str,
    pub path: &'a str,
}

pub struct TrackTable<'a> {
    pub title: &'a str,
    // pub artists: &'a str, TODO
    pub album: &'a str,
    pub duration: TrackDuration,
    pub rating: TrackRating,
}

struct TrackData {
    title: TextRange,
    duration: TrackDuration,
    rating: TrackRating,
    bit_rate_kbps: u32,
    bit_depth: Option<u8>,
    sample_rate: Option<u32>,
    path: TextRange,
    album_id: AlbumId,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrackId(pub u32);

impl TrackId {
    const fn from_idx(i: usize) -> Self {
        Self(i as u32)
    }

    const fn idx(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone)]
pub struct Artist {
    pub name: TextRange,
}

#[derive(Debug, Clone)]
pub struct Album {
    pub name: TextRange,
}

pub struct Database {
    text: String,
    tracks: Vec<TrackData>,
    tracks_sorted: Vec<TrackId>,
    artists: HashMap<ArtistId, Artist>,
    albums: HashMap<AlbumId, Album>,
    sort: TrackSort,
    reverse_sort: bool,
    scanner: Option<Scanner>,
    writer: RatingsWriter,
}

pub enum DatabaseEvent {
    FinishedLoading,
    Error(DatabaseError),
}

impl Database {
    pub fn new(music_dir: PathBuf) -> Self {
        Self {
            text: String::new(),
            tracks: Vec::new(),
            tracks_sorted: Vec::new(),
            artists: HashMap::new(),
            albums: HashMap::new(),
            sort: TrackSort::Title, // TODO
            reverse_sort: false,
            scanner: Some(Scanner::new(music_dir)),
            writer: RatingsWriter::new(),
        }
    }

    pub fn load(&mut self) {
        if let Some(scanner) = self.scanner.as_mut() {
            scanner.start();
        }
    }

    pub const fn get_sort(&self) -> TrackSort {
        self.sort
    }

    pub const fn get_reverse_sort(&self) -> bool {
        self.reverse_sort
    }

    pub fn next_sort(&mut self) {
        // self.sort = self.sort.next();

        self.tracks_sorted.sort_unstable_by_key(|id| {
            (
                {
                    let track = &self.tracks[id.idx()];
                    let title = &self.text[track.title];
                    title
                },
                id.0,
            )
        });

        if self.reverse_sort {
            self.tracks_sorted.reverse();
        }
    }

    pub fn toggle_reverse_sort(&mut self) {
        self.reverse_sort = !self.reverse_sort;
        self.tracks_sorted.reverse();
    }

    pub fn slice(&self, range: TextRange) -> &str {
        &self.text[range]
    }

    pub fn track(&self, id: TrackId) -> Track<'_> {
        let data = &self.tracks[id.idx()];
        Track {
            title: self.slice(data.title),
            duration: data.duration,
            rating: data.rating,
            bit_rate_kbps: data.bit_rate_kbps,
            bit_depth: data.bit_depth,
            sample_rate: data.sample_rate,
            album: self.slice(self.albums[&data.album_id].name),
            path: self.slice(data.path),
        }
    }

    pub fn track_title(&self, id: TrackId) -> &str {
        let data = &self.tracks[id.idx()];
        self.slice(data.title)
    }

    pub fn track_table(&self, id: TrackId) -> TrackTable<'_> {
        let data = &self.tracks[id.idx()];
        TrackTable {
            title: self.slice(data.title),
            album: self.slice(self.albums[&data.album_id].name),
            duration: data.duration,
            rating: data.rating,
        }
    }

    pub fn track_id(&self, index: usize) -> TrackId {
        self.tracks_sorted[index]
    }

    pub fn track_index(&self, id: TrackId) -> usize {
        self.tracks_sorted
            .iter()
            .enumerate()
            .find(|(_, tid)| id == **tid)
            .map(|(i, _)| i)
            .unwrap()
    }

    pub fn random_track_id(&self) -> TrackId {
        // TODO
        TrackId(fastrand::u32(0..self.tracks.len() as u32))
    }

    pub fn track_path(&self, id: TrackId) -> &Path {
        let data = &self.tracks[id.idx()];
        Path::new(self.slice(data.path))
    }

    pub fn tracks_iter2(&self) -> impl Iterator<Item = &TrackData> {
        self.tracks_sorted.iter().map(|i| &self.tracks[i.idx()])
    }

    pub fn tracks_range2(&self, range: TextRange) -> impl Iterator<Item = &TrackData> {
        self.tracks_sorted[range]
            .iter()
            .map(|i| &self.tracks[i.idx()])
    }

    pub const fn is_tracks_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub const fn tracks_len(&self) -> usize {
        self.tracks.len()
    }

    pub fn write_rating(&mut self, id: TrackId, rating: TrackRating) -> bool {
        let track = &self.tracks[id.idx()];

        if track.rating == rating {
            return false;
        }

        self.writer.push(WriteRequest {
            id,
            old_rating: track.rating,
            new_rating: rating,
            path: PathBuf::from(&self.text[track.path]),
        });

        // Update new rating immediately
        // TODO
        // self.set_track_rating(id, rating);

        true
    }

    pub fn update(
        &mut self,
        current_track_id: Option<TrackId>,
        mut on_event: impl FnMut(DatabaseEvent),
    ) {
        self.receive_files(&mut on_event);
        self.poll_writer(current_track_id, &mut on_event);
    }

    fn receive_files(&mut self, mut on_event: impl FnMut(DatabaseEvent)) {
        let Some(scanner) = self.scanner.take() else {
            return;
        };

        let mut is_done = false;

        while let Some(event) = scanner.recv() {
            match event {
                ScanEvent::Track(track) => {
                    self.add_track(track);
                }
                ScanEvent::Error(path, err) => {
                    on_event(DatabaseEvent::Error(DatabaseError {
                        message: format!("Failed to scan audio file '{}'", path.display()),
                        source: Some(DatabaseErrorSource::Audio(err)),
                    }));
                }
                ScanEvent::Done => {
                    // TODO: remove
                    for track in ScannedTrack::test() {
                        self.add_track(track);
                    }

                    on_event(DatabaseEvent::FinishedLoading);
                    is_done = true;
                    break;
                }
            }
        }

        if !is_done {
            self.scanner = Some(scanner);
        }
    }

    fn poll_writer(
        &mut self,
        current_track_id: Option<TrackId>,
        mut on_event: impl FnMut(DatabaseEvent),
    ) {
        if let Some(Err(WriteFailure {
            id,
            old_rating,
            path,
            error,
        })) = self.writer.poll(current_track_id)
        {
            // Restore old rating
            // TODO
            // self.set_track_rating(id, old_rating);

            // Notify
            on_event(DatabaseEvent::Error(DatabaseError {
                message: format!("Failed to write audio rating to '{}'", path.display()),
                source: Some(DatabaseErrorSource::Audio(error)),
            }));
        }
    }

    fn add_track(&mut self, track: ScannedTrack) {
        // Album
        let album_id = {
            let id = utils::hasher_fast(|hasher| {
                hasher.write(track.album().as_bytes());
                hasher.write(track.album_artist().as_bytes());
                hasher.write(track.date().as_bytes());
            });
            AlbumId(id)
        };

        if !self.albums.contains_key(&album_id) {
            let range = self.text.push_str_get_range_new(track.album());
            self.albums.insert(album_id, Album { name: range });
        }

        let track_id = TrackId::from_idx(self.tracks.len());
        self.tracks.push(TrackData {
            title: self.text.push_str_get_range_new(track.title()),
            duration: track.duration(),
            rating: track.rating(),
            bit_rate_kbps: track.bit_rate_kbps(),
            bit_depth: track.bit_depth(),
            sample_rate: track.sample_rate(),
            path: self.text.push_str_get_range_new(track.path()),
            album_id,
        });
        self.tracks_sorted.push(track_id);

        // Artists
        for artist in track.artists() {
            let artist_id = ArtistId(utils::hash_fast(artist));
            if !self.artists.contains_key(&artist_id) {
                let range = self.text.push_str_get_range_new(artist);
                self.artists.insert(artist_id, Artist { name: range });
            }
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArtistId(pub u64);

impl ArtistId {
    pub const ZERO: Self = Self(0);
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlbumId(pub u64);

impl AlbumId {
    pub const ZERO: Self = Self(0);
}

#[derive(Debug)]
pub struct DatabaseError {
    message: String,
    source: Option<DatabaseErrorSource>,
}

#[derive(Debug)]
enum DatabaseErrorSource {
    Sqlite(SqliteError),
    Audio(AudioFileError),
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for DatabaseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|err| match err {
            DatabaseErrorSource::Sqlite(err) => err as _,
            DatabaseErrorSource::Audio(err) => err as _,
        })
    }
}
