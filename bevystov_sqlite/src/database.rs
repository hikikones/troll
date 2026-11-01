use std::{
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

pub struct Database {
    sqlite: Sqlite,
    sql: String,
    sort: TrackSort,
    reverse_sort: bool,
    scanner: Option<Scanner>,
    writer: RatingsWriter,
    tracks_len: u32,
}

pub enum DatabaseEvent {
    Track(TrackId),
    Error(DatabaseError),
    FinishedLoading,
}

pub enum DatabaseLoadEvent {
    Start(u32),
    Progress(u32),
    ScanError(DatabaseError),
}

impl Database {
    pub fn new(music_dir: PathBuf, in_memory: bool) -> DatabaseResult<Self> {
        let sqlite = if in_memory {
            Sqlite::open_in_memory().map_err(|err| DatabaseError {
                message: String::from("Failed to open database in-memory"),
                source: Some(DatabaseErrorSource::Sqlite(err)),
            })
        } else {
            let path =
                std::env::temp_dir().join(shared::symbols::concat!(crate::APP_NAME, ".db.tmp"));
            let _ = std::fs::File::create(&path);
            Sqlite::open(&path).map_err(|err| DatabaseError {
                message: format!("Failed to open database at '{}'", path.display()),
                source: Some(DatabaseErrorSource::Sqlite(err)),
            })
        }?;

        sqlite
            .execute_batch(include_str!("database/schema.sql"))
            .map_err(|err| DatabaseError {
                message: String::from("Failed to execute schema"),
                source: Some(DatabaseErrorSource::Sqlite(err)),
            })?;

        Ok(Self {
            sqlite,
            sql: String::new(),
            sort: TrackSort::default(),
            reverse_sort: false,
            scanner: Some(Scanner::new(music_dir)),
            writer: RatingsWriter::new(),
            tracks_len: 0,
        })
    }

    pub fn load(&mut self) {
        if let Some(scanner) = self.scanner.as_mut() {
            scanner.start();
        }
    }

    pub const fn has_finished_loading(&self) -> bool {
        self.scanner.is_none()
    }

    pub fn path(&self) -> Option<&Path> {
        self.sqlite.path().map(Path::new)
    }

    pub const fn get_sort(&self) -> TrackSort {
        self.sort
    }

    pub const fn get_reverse_sort(&self) -> bool {
        self.reverse_sort
    }

    pub const fn next_sort(&mut self) {
        self.sort = self.sort.next();
    }

    pub const fn toggle_reverse_sort(&mut self) {
        self.reverse_sort = !self.reverse_sort;
    }

    pub const fn is_tracks_empty(&self) -> bool {
        self.tracks_len == 0
    }

    pub const fn tracks_len(&self) -> u32 {
        self.tracks_len
    }

    pub fn track_title(&self, id: TrackId, mut f: impl FnMut(&str)) {
        self.sqlite
            .single("SELECT title FROM tracks WHERE id = ?", [id], |row| {
                Ok(f(row.get_ref(0)?.as_str()?))
            })
            .unwrap();
    }

    pub fn track_rating(&self, id: TrackId) -> TrackRating {
        self.sqlite
            .single("SELECT rating FROM tracks WHERE id = ?", [id], |row| {
                row.get(0)
            })
            .unwrap()
    }

    pub fn track_path(&self, id: TrackId) -> PathBuf {
        self.sqlite
            .single("SELECT path FROM tracks WHERE id = ?", [id], |row| {
                Ok(PathBuf::from(row.get_ref(0)?.as_str()?))
            })
            .unwrap()
    }

    pub fn track_info(&self, id: TrackId, mut f: impl FnMut(TrackInfo)) {
        self.sqlite
            .single(
                "SELECT \
            title, rating, duration, bit_rate_kbps, bit_depth, sample_rate, path, album_id \
            FROM tracks WHERE id = ?",
                [id],
                |row| {
                    Ok(f(TrackInfo {
                        title: row.get_ref(0)?.as_str()?,
                        rating: row.get(1)?,
                        duration: row.get(2)?,
                        bit_rate_kbps: row.get(3)?,
                        bit_depth: row.get(4)?,
                        sample_rate: row.get(5)?,
                        path: Path::new(row.get_ref(6)?.as_str()?),
                        album_id: row.get(7)?,
                    }))
                },
            )
            .unwrap();
    }

    pub fn track_text(&self, id: TrackId, mut f: impl FnMut(TrackText)) {
        self.sqlite
            .single(
                "SELECT t.title, GROUP_CONCAT(ar.name, ', '), al.name \
                FROM tracks t \
                JOIN artist_tracks ar_t ON ar_t.track_id = t.id \
                JOIN artists ar ON ar.id = ar_t.artist_id \
                JOIN albums al ON al.id = t.album_id \
                WHERE t.id = ? \
                GROUP BY t.id",
                [id],
                |row| {
                    Ok(f(TrackText {
                        title: row.get_ref(0)?.as_str()?,
                        artists: row.get_ref(1)?.as_str()?,
                        album: row.get_ref(2)?.as_str()?,
                    }))
                },
            )
            .unwrap();
    }

    pub fn track_artists(&self, id: TrackId, mut f: impl FnMut(&str)) {
        self.sqlite
            .for_each(
                "SELECT ar.name FROM artists ar \
                JOIN artist_tracks ar_t ON ar_t.artist_id = ar.id \
                WHERE ar_t.track_id = ?",
                [id],
                |row| Ok(f(row.get_ref(0)?.as_str()?)),
            )
            .unwrap();
    }

    pub fn track_table(&mut self, id: TrackId, mut f: impl FnMut(TrackTable)) {
        self.sql.clear();

        let artists = {
            if self.sort == TrackSort::Artist {
                if self.reverse_sort {
                    "GROUP_CONCAT(ar.name, ', ' ORDER BY ar.name COLLATE NOCASE DESC)"
                } else {
                    "GROUP_CONCAT(ar.name, ', ' ORDER BY ar.name COLLATE NOCASE ASC)"
                }
            } else {
                "GROUP_CONCAT(ar.name, ', ')"
            }
        };
        self.sql.write(format_args!(
            "SELECT t.title, {artists}, al.name, t.duration, t.rating FROM tracks t \
            JOIN artist_tracks ar_t ON ar_t.track_id = t.id \
            JOIN artists ar ON ar.id = ar_t.artist_id \
            JOIN albums al ON al.id = t.album_id \
            WHERE t.id = ? \
            GROUP BY t.id"
        ));
        self.sqlite
            .single(&self.sql, [id], |row| {
                Ok(f(TrackTable {
                    title: row.get_ref(0)?.as_str()?,
                    artists: row.get_ref(1)?.as_str()?,
                    album: row.get_ref(2)?.as_str()?,
                    duration: row.get(3)?,
                    rating: row.get(4)?,
                }))
            })
            .unwrap();
    }

    pub fn tracks_sort(&mut self) -> impl Iterator<Item = TrackId> {
        self.sql.clear();

        let order = if self.reverse_sort { "DESC" } else { "ASC" };
        let query = match self.sort {
            TrackSort::Title => {
                format_args!(
                    "SELECT id, title FROM tracks \
                    ORDER BY title COLLATE NOCASE {order}, id {order}"
                )
            }
            TrackSort::Artist => {
                format_args!(
                    "SELECT \
                        t.id, \
                        GROUP_CONCAT(ar.name, ', ' ORDER BY ar.name COLLATE NOCASE {order}) \
                    FROM tracks t \
                    JOIN artist_tracks ar_t ON ar_t.track_id = t.id \
                    JOIN artists ar ON ar.id = ar_t.artist_id \
                    GROUP BY t.id \
                    ORDER BY ar.name COLLATE NOCASE {order}, t.id {order}"
                )
            }
            TrackSort::Album => {
                format_args!(
                    "SELECT t.id, al.name FROM tracks t \
                    JOIN albums al ON al.id = t.album_id \
                    ORDER BY al.name COLLATE NOCASE {order}, t.album_id {order}, t.number {order}, t.id {order}"
                )
            }
            TrackSort::Time => {
                format_args!(
                    "SELECT id, duration FROM tracks \
                    ORDER BY duration {order}, id {order}"
                )
            }
            TrackSort::Rating => {
                format_args!(
                    "SELECT id, rating FROM tracks \
                    ORDER BY rating {order}, id {order}"
                )
            }
        };

        self.sql.write(query);
        self.sqlite
            .iter(&self.sql, (), |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
    }

    pub fn album_name(&self, id: AlbumId, mut f: impl FnMut(&str)) {
        self.sqlite
            .single("SELECT name FROM albums WHERE id = ?", [id], |row| {
                Ok(f(row.get_ref(0)?.as_str()?))
            })
            .unwrap();
    }

    pub fn album_tracks(&self, id: AlbumId) -> impl Iterator<Item = TrackId> {
        self.sqlite
            .iter("SELECT id FROM tracks WHERE album_id = ?", [id], |row| {
                row.get(0)
            })
            .unwrap()
            .map(Result::unwrap)
    }

    pub fn search(
        &mut self,
        input: &str,
        include_path: bool,
        mut f: impl FnMut(TrackId),
    ) -> SqliteResult<()> {
        self.sql.clear();

        let param = if !include_path {
            self.sql.extend(["-path:", input]);
            self.sql.as_str()
        } else {
            input
        };

        self.sqlite.for_each(
            "SELECT rowid FROM track_fts \
                WHERE track_fts MATCH ? \
                ORDER BY rank",
            [param],
            |row| Ok(f(row.get(0)?)),
        )
    }

    pub fn search_highlight(
        &mut self,
        id: TrackId,
        input: &str,
        markup_start: impl std::fmt::Display,
        markup_end: impl std::fmt::Display,
        f: impl FnOnce(&str),
    ) -> SqliteResult<()> {
        self.sql.clear();

        self.sql.write(format_args!(
            "SELECT
                rowid, \
                highlight(track_fts, 0, '{markup_start}', '{markup_end}') || ' ' || \
                highlight(track_fts, 1, '{markup_start}', '{markup_end}') || ' ' || \
                highlight(track_fts, 2, '{markup_start}', '{markup_end}') \
            FROM track_fts \
            WHERE rowid = ?1 AND track_fts MATCH ?2",
        ));

        self.sqlite.single(&self.sql, (id, input), |row| {
            Ok(f(row.get_ref(1)?.as_str()?))
        })
    }

    pub fn random_track_id(&self) -> TrackId {
        self.sqlite
            .single(
                "SELECT id FROM tracks ORDER BY RANDOM() LIMIT 1",
                (),
                |row| row.get(0),
            )
            .unwrap()
    }

    pub fn random_track_id_except(&self, id: TrackId) -> Option<TrackId> {
        self.sqlite
            .first(
                "SELECT id FROM tracks WHERE id != ? ORDER BY RANDOM() LIMIT 1",
                [id],
                |row| row.get(0),
            )
            .unwrap()
    }

    pub fn write_rating(&mut self, id: TrackId, rating: TrackRating) -> bool {
        let current_rating = self.track_rating(id);

        if current_rating == rating {
            return false;
        }

        let path = self.track_path(id);
        self.writer.push(WriteRequest {
            id,
            old_rating: current_rating,
            new_rating: rating,
            path,
        });

        // Update new rating immediately
        self.set_track_rating(id, rating);
        true
    }

    pub fn poll_receiver(&mut self, mut on_event: impl FnMut(DatabaseLoadEvent)) {
        let Some(scanner) = self.scanner.take() else {
            return;
        };

        let mut is_done = false;
        let mut progress = None;

        while let Some(event) = scanner.recv() {
            match event {
                ScanEvent::Files(len) => {
                    on_event(DatabaseLoadEvent::Start(len));
                }
                ScanEvent::Track(i, track) => {
                    self.sqlite
                        .transaction(|tx| Ok(Self::add_track(tx, track)))
                        .unwrap();
                    progress = Some(i);
                }
                ScanEvent::Error(path, err) => {
                    on_event(DatabaseLoadEvent::ScanError(DatabaseError {
                        message: format!("Failed to scan audio file '{}'", path.display()),
                        source: Some(DatabaseErrorSource::Audio(err)),
                    }));
                }
                ScanEvent::Done => {
                    is_done = true;
                    break;
                }
            }
        }

        if let Some(progress) = progress {
            on_event(DatabaseLoadEvent::Progress(progress));
        }

        if is_done {
            self.set_tracks_len();
            return;
        }

        self.scanner = Some(scanner);
    }

    pub fn poll_writer(
        &mut self,
        current_track_id: Option<TrackId>,
        mut on_error: impl FnMut(DatabaseError),
    ) {
        if let Some(Err(WriteFailure {
            id,
            old_rating,
            path,
            error,
        })) = self.writer.poll(current_track_id)
        {
            // Restore old rating
            self.set_track_rating(id, old_rating);

            // Notify
            on_error(DatabaseError {
                message: format!("Failed to write audio rating to '{}'", path.display()),
                source: Some(DatabaseErrorSource::Audio(error)),
            });
        }
    }

    fn set_tracks_len(&mut self) {
        self.tracks_len = self
            .sqlite
            .single("SELECT COUNT(*) FROM tracks", (), |row| row.get(0))
            .unwrap();
    }

    fn set_track_rating(&self, id: TrackId, rating: TrackRating) {
        self.sqlite
            .execute("UPDATE tracks SET rating = ?1 WHERE id = ?2", (rating, id))
            .unwrap();
    }

    fn add_track(tx: &Transaction, track: ScannedTrack) {
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
        )
        .unwrap();

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
        )
        .unwrap();
        let track_id = tx.last_insert_rowid() as u32;

        // Artists
        for artist in track.artists() {
            let rows_changed = tx
                .execute(
                    "INSERT INTO artists (name) VALUES (?1) \
                            ON CONFLICT(name) DO NOTHING",
                    [artist],
                )
                .unwrap();
            let artist_id: SqliteId = if rows_changed > 0 {
                tx.last_insert_rowid()
            } else {
                tx.query_row("SELECT id FROM artists WHERE name = ?1", [artist], |row| {
                    row.get(0)
                })
                .unwrap()
            };
            tx.execute(
                "INSERT INTO artist_tracks (artist_id, track_id) \
                VALUES (?1, ?2) ON CONFLICT DO NOTHING",
                (artist_id, track_id),
            )
            .unwrap();
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
        )
        .unwrap();
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArtistId(pub SqliteId);

impl ArtistId {
    pub const ZERO: Self = Self(0);
}

impl ToSql for ArtistId {
    fn to_sql(&self) -> SqliteResult<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Integer(self.0)))
    }
}

impl FromSql for ArtistId {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value.as_i64().map(|n| Self(n))
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlbumId(pub SqliteId);

impl AlbumId {
    pub const ZERO: Self = Self(0);
}

impl ToSql for AlbumId {
    fn to_sql(&self) -> SqliteResult<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Integer(self.0)))
    }
}

impl FromSql for AlbumId {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value.as_i64().map(|n| Self(n))
    }
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
