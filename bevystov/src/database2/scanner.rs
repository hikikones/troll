use std::{
    ops::Range,
    path::{Path, PathBuf},
    sync::mpsc::TryRecvError,
};

use utils::StringExt;

use crate::database2::{
    AudioExt, AudioFile, AudioFileError, AudioResult, TrackDuration, TrackRating,
};

type Sender = std::sync::mpsc::Sender<ScanResult>;
type Receiver = std::sync::mpsc::Receiver<ScanResult>;

pub struct Scanner {
    scan: Option<(Sender, PathBuf)>,
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
            scan: Some((sender, dir)),
            receiver,
        }
    }

    pub fn start(&mut self) {
        let Some((sender, dir)) = self.scan.take() else {
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
    rating: TrackRating,
    duration: TrackDuration,
    bit_rate_kbps: u32,
    bit_depth: Option<u8>,
    sample_rate: Option<u32>,
    title: Range<usize>,
    album: Range<usize>,
    path: Range<usize>,
}

impl ScannedTrack {
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
    let audio = AudioFile::read_metadata(path, ext)?;
    let tag = audio.vorbis_comments()?;
    let p = audio.properties();

    // Track
    track.data.title = track
        .text
        .push_str_get_range(tag.get("TITLE").unwrap_or(""));
    track.data.rating = tag
        .get("RATING")
        .map(TrackRating::from_vorbis_comments)
        .unwrap_or(TrackRating::None);
    track.data.duration = TrackDuration::from_duration(p.duration);
    track.data.bit_rate_kbps = p.bit_rate_kbps;
    track.data.bit_depth = p.bit_depth;
    track.data.sample_rate = p.sample_rate;

    // Artists
    // TODO: What if empty?
    for artist in tag.get_all("ARTIST") {
        let range = track.text.push_str_get_range(artist);
        track.artists.push(range);
    }

    // Album
    track.data.album = track
        .text
        .push_str_get_range(tag.get("ALBUM").unwrap_or(""));

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
