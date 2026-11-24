use std::{
    hash::Hasher,
    path::{Path, PathBuf},
    range::Range,
};

use shared::{
    logs::Logs,
    sqlite::{SqliteError, SqliteId, SqliteResult, Transaction},
    time::Timer,
};
use utils::StringExt;

use crate::database::{
    AudioFile, AudioFileExt, AudioMetadata, AudioProperties, AudioResult, Database, TrackDuration,
    TrackNumber, TrackRating,
};

type Receiver = std::sync::mpsc::Receiver<ScanEvent>;
type Handle = std::thread::JoinHandle<(Database, Logs)>;

pub struct Scanner(PathBuf);

impl Scanner {
    pub const fn new(dir: PathBuf) -> Self {
        Self(dir)
    }

    pub fn spawn(self, mut database: Database, mut logs: Logs) -> (ScannerRecv, ScannerHandle) {
        let Scanner(dir) = self;
        let (sender, receiver) = std::sync::mpsc::channel();

        let handle = std::thread::spawn(move || {
            let files = collect_files(dir, true);
            let _ = sender.send(ScanEvent::TotalFiles(files.len() as u32));

            let mut track = ScannedTrack::default();
            let mut timer = Timer::new(4).with_loop().with_start();

            for ((path, ext), i) in files.into_iter().zip(1..) {
                if let Err(error) = process_file(&path, ext, &mut track) {
                    logs.push_context(
                        format_args!("Failed to scan audio file '{}'.", path.display()),
                        error,
                    );
                    track.clear();
                    continue;
                }

                if let Err(error) = database.sqlite.transaction(|tx| add_track(tx, &track)) {
                    let _ = sender.send(ScanEvent::Error(error));
                    track.clear();
                    continue;
                }

                if timer.tick() {
                    let _ = sender.send(ScanEvent::Progress(i));
                }

                track.clear();
            }

            let _ = sender.send(ScanEvent::Done);

            database.set_tracks_len();
            database.sort_tracks();

            (database, logs)
        });

        (ScannerRecv(receiver), ScannerHandle(handle))
    }
}

pub struct ScannerRecv(Receiver);

impl ScannerRecv {
    pub fn recv(&self) -> Option<ScanEvent> {
        self.0.try_recv().ok()
    }
}

pub struct ScannerHandle(Handle);

impl ScannerHandle {
    pub fn join(self) -> (Database, Logs) {
        self.0.join().unwrap()
    }
}

pub enum ScanEvent {
    TotalFiles(u32),
    Progress(u32),
    Done,
    Error(SqliteError),
}

#[derive(Default)]
struct ScannedTrack {
    path: Range<usize>,
    metadata: AudioMetadata,
    properties: AudioProperties,
}

impl ScannedTrack {
    const fn number(&self) -> TrackNumber {
        self.metadata.number
    }

    const fn rating(&self) -> TrackRating {
        self.metadata.rating
    }

    const fn duration(&self) -> TrackDuration {
        self.properties.duration
    }

    const fn bit_rate_kbps(&self) -> u32 {
        self.properties.bit_rate_kbps
    }

    const fn bit_depth(&self) -> Option<u8> {
        self.properties.bit_depth
    }

    const fn sample_rate(&self) -> Option<u32> {
        self.properties.sample_rate
    }

    fn title(&self) -> &str {
        self.slice(self.metadata.title)
    }

    fn artists(&self) -> impl Iterator<Item = &str> {
        self.metadata.artists.iter().map(|r| self.slice(*r))
    }

    fn album(&self) -> &str {
        self.slice(self.metadata.album)
    }

    fn album_artist(&self) -> &str {
        self.slice(self.metadata.album_artist)
    }

    fn date(&self) -> &str {
        self.slice(self.metadata.date)
    }

    fn path(&self) -> &str {
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

fn add_track(tx: &Transaction, track: &ScannedTrack) -> SqliteResult<()> {
    // Album
    let album_id = utils::hasher_fast(|hasher| {
        hasher.write(track.album().as_bytes());
        hasher.write(track.album_artist().as_bytes());
        hasher.write(track.date().as_bytes());
    }) as SqliteId;

    tx.execute(
        "INSERT INTO albums (id, name) \
                VALUES (?1, ?2) ON CONFLICT(id) DO NOTHING",
        (album_id, track.album()),
    )?;

    // Track
    tx.execute(
        "INSERT INTO tracks \
                (title, \
                number, \
                rating, \
                duration, \
                bit_rate_kbps, \
                bit_depth, \
                sample_rate, \
                path, album_id) \
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        (
            track.title(),
            track.number(),
            track.rating(),
            track.duration(),
            track.bit_rate_kbps(),
            track.bit_depth(),
            track.sample_rate(),
            track.path(),
            album_id,
        ),
    )?;
    let track_id = tx.last_insert_rowid() as u32;

    // Artists
    for artist in track.artists() {
        let rows_changed = tx.execute(
            "INSERT INTO artists (name) VALUES (?1) \
                            ON CONFLICT(name) DO NOTHING",
            [artist],
        )?;
        let artist_id: SqliteId = if rows_changed > 0 {
            tx.last_insert_rowid()
        } else {
            tx.query_row("SELECT id FROM artists WHERE name = ?1", [artist], |row| {
                row.get(0)
            })?
        };
        tx.execute(
            "INSERT INTO artist_tracks (artist_id, track_id) \
                VALUES (?1, ?2) ON CONFLICT DO NOTHING",
            (artist_id, track_id),
        )?;
    }
    tx.execute(
        "INSERT INTO track_fts (rowid, title, artist, album, path) \
            SELECT \
                t.id, \
                t.title, \
                COALESCE(( \
                    SELECT group_concat(ar.name, ' ') \
                    FROM artist_tracks ar_t \
                    JOIN artists ar ON ar.id = ar_t.artist_id \
                    WHERE ar_t.track_id = t.id \
                ), ''), \
                al.name, \
                t.path \
            FROM tracks t \
            JOIN albums al ON al.id = t.album_id \
            WHERE t.id = ?1",
        [track_id],
    )?;

    Ok(())
}

fn process_file(path: &Path, ext: AudioFileExt, track: &mut ScannedTrack) -> AudioResult<()> {
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

    Ok(())
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
