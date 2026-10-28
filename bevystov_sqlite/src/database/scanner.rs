use std::{
    ops::Range,
    path::{Path, PathBuf},
    sync::mpsc::TryRecvError,
};

use utils::StringExt;

use crate::database::{
    AudioExt, AudioFile, AudioFileError, AudioResult, TrackDuration, TrackRating,
};

type Sender = std::sync::mpsc::Sender<ScanResult>;
type Receiver = std::sync::mpsc::Receiver<ScanResult>;

pub struct Scanner {
    sender: Option<(Sender, PathBuf)>,
    receiver: Receiver,
}

pub enum ScanEvent {
    Track(ScannedTrack),
    Error(PathBuf, AudioFileError),
    Done,
}

enum ScanResult {
    Success(ScannedTrack),
    Error(PathBuf, AudioFileError),
}

impl Scanner {
    pub fn new(dir: PathBuf) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        Self {
            sender: Some((sender, dir)),
            receiver,
        }
    }

    pub fn start(&mut self) {
        let Some((sender, dir)) = self.sender.take() else {
            return;
        };

        std::thread::spawn(move || {
            let mut track = ScannedTrack::default();

            traverse_files(dir, true, |path, ext| {
                let result = match process_file(path, ext, &mut track) {
                    Ok(track) => ScanResult::Success(track),
                    Err(err) => ScanResult::Error(path.to_path_buf(), err),
                };
                let _ = sender.send(result);

                track.clear();
            });
        });
    }

    pub fn recv(&self) -> Option<ScanEvent> {
        match self.receiver.try_recv() {
            Ok(result) => match result {
                ScanResult::Success(track) => Some(ScanEvent::Track(track)),
                ScanResult::Error(path, err) => Some(ScanEvent::Error(path, err)),
            },
            Err(err) => match err {
                TryRecvError::Empty => None,
                TryRecvError::Disconnected => Some(ScanEvent::Done),
            },
        }
    }
}

#[derive(Default, Clone)]
pub struct ScannedTrack {
    text: String,
    artists: Vec<Range<usize>>,
    data: Data,
}

#[derive(Default, Clone)]
struct Data {
    number: u8,
    rating: TrackRating,
    duration: TrackDuration,
    bit_rate_kbps: u32,
    bit_depth: Option<u8>,
    sample_rate: Option<u32>,
    title: Range<usize>,
    album: Range<usize>,
    album_artist: Range<usize>,
    date: Range<usize>,
    path: Range<usize>,
}

impl ScannedTrack {
    // TODO: Remove
    pub fn test() -> Vec<ScannedTrack> {
        let mut v = vec![];

        let t = v.push_mut(Self::default());
        t.artists.push(Range::default());

        let t = v.push_mut(Self::default());
        t.artists.push(t.text.push_str_get_range("a1"));
        t.artists.push(t.text.push_str_get_range("a2"));
        t.artists.push(t.text.push_str_get_range("a3"));

        v
    }

    pub const fn number(&self) -> u8 {
        self.data.number
    }

    pub const fn rating(&self) -> TrackRating {
        self.data.rating
    }

    pub const fn duration(&self) -> TrackDuration {
        self.data.duration
    }

    pub const fn bit_rate_kbps(&self) -> u32 {
        self.data.bit_rate_kbps
    }

    pub const fn bit_depth(&self) -> Option<u8> {
        self.data.bit_depth
    }

    pub const fn sample_rate(&self) -> Option<u32> {
        self.data.sample_rate
    }

    pub fn title(&self) -> &str {
        self.slice(self.data.title.clone())
    }

    pub fn artists(&self) -> impl Iterator<Item = &str> {
        self.artists.iter().map(|r| self.slice(r.clone()))
    }

    pub fn album(&self) -> &str {
        self.slice(self.data.album.clone())
    }

    pub fn album_artist(&self) -> &str {
        self.slice(self.data.album_artist.clone())
    }

    pub fn date(&self) -> &str {
        self.slice(self.data.date.clone())
    }

    pub fn path(&self) -> &str {
        self.slice(self.data.path.clone())
    }

    fn slice(&self, range: Range<usize>) -> &str {
        &self.text[range]
    }

    fn clear(&mut self) {
        self.text.clear();
        self.artists.clear();
        self.data = Data::default();
    }
}

fn process_file(path: &Path, ext: AudioExt, track: &mut ScannedTrack) -> AudioResult<ScannedTrack> {
    let mut audio = AudioFile::read_metadata(path, ext)?;
    let vorbis_comments = audio.vorbis_comments_mut()?;

    // Metadata
    for (mut key, value) in vorbis_comments.take_items() {
        key.make_ascii_uppercase();
        let val = value.as_str();

        match key.as_str() {
            "TITLE" => track.data.title = track.text.push_str_get_range(val),
            "ARTIST" => {
                let range = track.text.push_str_get_range(val);
                track.artists.push(range)
            }
            "ALBUM" => track.data.album = track.text.push_str_get_range(val),
            "ALBUMARTIST" => track.data.album_artist = track.text.push_str_get_range(val),
            "RATING" => track.data.rating = TrackRating::from_vorbis_comments(val),
            "TRACKNUMBER" => track.data.number = val.parse::<u8>().unwrap_or_default(),
            "DATE" => track.data.date = track.text.push_str_get_range(val),
            _ => {}
        }
    }

    if track.artists.is_empty() {
        track.artists.push(Range::default());
    }

    // Properties
    let p = audio.properties();
    track.data.duration = p.duration.into();
    track.data.bit_rate_kbps = p.bit_rate_kbps;
    track.data.bit_depth = p.bit_depth;
    track.data.sample_rate = p.sample_rate;

    // Path
    track.data.path = track.text.push_str_get_range(&path.to_string_lossy());

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
