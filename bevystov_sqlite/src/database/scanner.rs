use std::{
    path::{Path, PathBuf},
    range::Range,
    sync::mpsc::TryRecvError,
};

use utils::StringExt;

use crate::database::{
    AudioDuration, AudioFile, AudioFileError, AudioFileExt, AudioMetadata, AudioProperties,
    AudioRating, AudioResult,
};

type Sender = std::sync::mpsc::Sender<ScanEvent>;
type Receiver = std::sync::mpsc::Receiver<ScanEvent>;

pub struct Scanner {
    sender: Option<(Sender, PathBuf)>,
    receiver: Receiver,
}

pub enum ScanEvent {
    Files(u32),
    Track(u32, ScannedTrack),
    Error(PathBuf, AudioFileError),
    Done,
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
            let files = collect_files(dir, true);
            let _ = sender.send(ScanEvent::Files(files.len() as u32));

            let mut track = ScannedTrack::default();
            let mut i = 1;

            for (path, ext) in files {
                let event = match process_file(&path, ext, &mut track) {
                    Ok(track) => ScanEvent::Track(i, track),
                    Err(err) => ScanEvent::Error(path, err),
                };
                let _ = sender.send(event);

                i += 1;
                track.clear();
            }

            // TODO: remove
            for t in ScannedTrack::test() {
                let _ = sender.send(ScanEvent::Track(i, t));
                i += 1;
            }
        });
    }

    pub fn recv(&self) -> Option<ScanEvent> {
        match self.receiver.try_recv() {
            Ok(event) => Some(event),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(ScanEvent::Done),
        }
    }
}

#[derive(Default, Clone)]
pub struct ScannedTrack {
    path: Range<usize>,
    metadata: AudioMetadata,
    properties: AudioProperties,
}

impl ScannedTrack {
    // TODO: Remove
    fn test() -> Vec<ScannedTrack> {
        let mut v = vec![];

        let t = v.push_mut(Self::default());
        t.metadata.artists.push(Range::default());

        let t = v.push_mut(Self::default());
        t.metadata.title = t.metadata.text.push_str_get_range("title");
        t.metadata.album = t.metadata.text.push_str_get_range("album");
        t.metadata
            .artists
            .push(t.metadata.text.push_str_get_range("a1"));
        t.metadata
            .artists
            .push(t.metadata.text.push_str_get_range("a2"));
        t.metadata
            .artists
            .push(t.metadata.text.push_str_get_range("a3"));

        v
    }

    pub const fn number(&self) -> u8 {
        self.metadata.number
    }

    pub const fn rating(&self) -> AudioRating {
        self.metadata.rating
    }

    pub const fn duration(&self) -> AudioDuration {
        self.properties.duration
    }

    pub const fn bit_rate_kbps(&self) -> u32 {
        self.properties.bit_rate_kbps
    }

    pub const fn bit_depth(&self) -> Option<u8> {
        self.properties.bit_depth
    }

    pub const fn sample_rate(&self) -> Option<u32> {
        self.properties.sample_rate
    }

    pub fn title(&self) -> &str {
        self.slice(self.metadata.title)
    }

    pub fn artists(&self) -> impl Iterator<Item = &str> {
        self.metadata.artists.iter().map(|r| self.slice(*r))
    }

    pub fn album(&self) -> &str {
        self.slice(self.metadata.album)
    }

    pub fn album_artist(&self) -> &str {
        self.slice(self.metadata.album_artist)
    }

    pub fn date(&self) -> &str {
        self.slice(self.metadata.date)
    }

    pub fn path(&self) -> &str {
        self.slice(self.path)
    }

    fn slice(&self, range: Range<usize>) -> &str {
        &self.metadata.text[range]
    }

    fn clear(&mut self) {
        self.path = Range::default();
        self.metadata.clear();
        self.properties = AudioProperties::default();
    }
}

fn process_file(
    path: &Path,
    ext: AudioFileExt,
    track: &mut ScannedTrack,
) -> AudioResult<ScannedTrack> {
    let audio = AudioFile::read_metadata(path, ext)?;

    audio.metadata_in_place(&mut track.metadata)?;
    track.properties = audio.properties();
    track.path = track
        .metadata
        .text
        .push_str_get_range(&path.to_string_lossy());

    if track.metadata.artists.is_empty() {
        track.metadata.artists.push(Range::default());
    }

    Ok(track.clone())
}

fn collect_files(root: impl AsRef<Path>, follow_links: bool) -> Vec<(PathBuf, AudioFileExt)> {
    walkdir::WalkDir::new(root)
        .follow_links(follow_links)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            AudioFileExt::from_path(entry.path()).map(|ext| (entry.path().to_path_buf(), ext))
        })
        .collect()
}
