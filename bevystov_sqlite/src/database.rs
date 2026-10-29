use std::{
    hash::Hasher,
    ops::Range,
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
}

pub enum DatabaseEvent {
    FinishedLoading,
    Error(DatabaseError),
}

impl Database {
    fn new(sqlite: Sqlite) -> Self {
        Self {
            sqlite,
            sql: String::new(),
            sort: TrackSort::default(),
            reverse_sort: false,
            scanner: None,
            writer: RatingsWriter::new(),
        }
    }

    pub fn open(music_dir: PathBuf, in_memory: bool) -> DatabaseResult<Self> {
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

        let mut db = Self::new(sqlite);
        db.migrate(music_dir)?;

        Ok(db)
    }

    pub fn load(&mut self) {
        if let Some(scanner) = self.scanner.as_mut() {
            scanner.start();
        }
    }

    pub fn path(&self) -> Option<&Path> {
        self.sqlite.path().map(Path::new)
    }

    pub const fn has_finished_loading(&self) -> bool {
        self.scanner.is_none()
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

    pub fn is_tracks_empty(&self) -> bool {
        self.sqlite
            .single("SELECT NOT EXISTS (SELECT 1 FROM tracks)", (), |row| {
                row.get(0)
            })
            .unwrap()
    }

    pub fn tracks_len(&self) -> usize {
        self.sqlite
            .single::<SqliteId>("SELECT COUNT(*) FROM tracks", (), |row| row.get(0))
            .map(|len| len as usize)
            .unwrap()
    }

    pub fn track_id(&mut self, index: usize) -> TrackId {
        let query = self.sorted_tracks_query();
        self.sql
            .write(format_args!("{query} LIMIT 1 OFFSET {index}"));

        let id = self
            .sqlite
            .single(&self.sql, (), |row| query.id(row))
            .unwrap();
        self.sql.clear();
        id
    }

    pub fn track_index(&mut self, id: TrackId) -> usize {
        let query = self.sorted_tracks_query();
        self.sql.write(format_args!(
            "SELECT row_num FROM ({query}) WHERE id = {}",
            id.0
        ));

        let index = self
            .sqlite
            .single(&self.sql, (), |row| query.index(row))
            .unwrap();
        self.sql.clear();
        index
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

    pub fn tracks_range(&self, range: Range<usize>) -> impl Iterator<Item = TrackId> {
        let (start, end) = (range.start as SqliteId, range.end as SqliteId);
        self.sqlite
            .iter(
                "SELECT id FROM tracks LIMIT ?1 OFFSET ?2",
                [end - start, start],
                |row| row.get(0),
            )
            .unwrap()
            .map(Result::unwrap)
    }

    pub fn tracks_table(&mut self, range: Range<usize>, mut f: impl FnMut(TrackTable)) {
        let offset = range.start;
        let limit = range.end - offset;
        let query = self.sorted_tracks_query();

        self.sql
            .write(format_args!("{query} LIMIT {limit} OFFSET {offset}"));
        self.sqlite
            .for_each(&self.sql, (), |row| Ok(f(query.table(row)?)))
            .unwrap();

        self.sql.clear();
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
        let param = if !include_path {
            self.sql.extend(["-path:", input]);
            self.sql.as_str()
        } else {
            input
        };

        let res = self.sqlite.for_each(
            "SELECT rowid FROM track_fts \
                WHERE track_fts MATCH ? \
                ORDER BY rank",
            [param],
            |row| Ok(f(row.get(0)?)),
        );
        self.sql.clear();
        res
    }

    pub fn search_highlight(
        &mut self,
        id: TrackId,
        input: &str,
        markup_start: impl std::fmt::Display,
        markup_end: impl std::fmt::Display,
        f: impl FnOnce(&str),
    ) -> SqliteResult<()> {
        self.sql.write(format_args!(
            "SELECT
                rowid, \
                highlight(track_fts, 0, '{markup_start}', '{markup_end}') || ' ' || \
                highlight(track_fts, 1, '{markup_start}', '{markup_end}') || ' ' || \
                highlight(track_fts, 2, '{markup_start}', '{markup_end}') \
            FROM track_fts \
            WHERE rowid = ?1 AND track_fts MATCH ?2",
        ));

        let res = self.sqlite.single(&self.sql, (id, input), |row| {
            Ok(f(row.get_ref(1)?.as_str()?))
        });
        self.sql.clear();
        res
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

    pub fn update(
        &mut self,
        current_track_id: Option<TrackId>,
        mut on_event: impl FnMut(DatabaseEvent),
    ) {
        self.receive_files(&mut on_event);
        self.poll_writer(current_track_id, &mut on_event);
    }

    fn receive_files(&mut self, mut on_event: impl FnMut(DatabaseEvent)) {
        let Some(scanner) = self.scanner.as_ref() else {
            return;
        };

        while let Some(event) = scanner.recv() {
            match event {
                ScanEvent::Track(track) => {
                    let _ = self.sqlite.transaction(|tx| Ok(Self::add_track(tx, track)));
                }
                ScanEvent::Error(path, err) => {
                    on_event(DatabaseEvent::Error(DatabaseError {
                        message: format!("Failed to scan audio file '{}'", path.display()),
                        source: Some(DatabaseErrorSource::Audio(err)),
                    }));
                }
                ScanEvent::Done => {
                    // TODO: remove
                    let _ = self.sqlite.transaction(|tx| {
                        for track in ScannedTrack::test() {
                            Self::add_track(tx, track);
                        }
                        Ok(())
                    });

                    self.scanner = None;
                    on_event(DatabaseEvent::FinishedLoading);
                    break;
                }
            }
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
            self.set_track_rating(id, old_rating);

            // Notify
            on_event(DatabaseEvent::Error(DatabaseError {
                message: format!("Failed to write audio rating to '{}'", path.display()),
                source: Some(DatabaseErrorSource::Audio(error)),
            }));
        }
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
            "INSERT INTO albums (id, name) VALUES (?1, ?2) ON CONFLICT(id) DO NOTHING",
            (album_id, track.album()),
        )
        .unwrap();

        // Track
        tx.execute(
            "INSERT INTO tracks \
            (title, number, rating, duration, bit_rate_kbps, bit_depth, sample_rate, path, album_id) \
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
        let track_id = tx.last_insert_rowid();

        // Artists
        for artist in track.artists() {
            let rows_changed = tx
                .execute(
                    "INSERT INTO artists (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
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
            tx.execute("INSERT INTO artist_tracks (artist_id, track_id) VALUES (?1, ?2) ON CONFLICT DO NOTHING", (artist_id, track_id)).unwrap();
        }

        tx.execute(
            "INSERT INTO track_fts (rowid, title, artist, album, path)
        SELECT
            t.id,
            t.title,
            COALESCE((
                SELECT group_concat(ar.name, ' ')
                FROM artist_tracks ar_t
                JOIN artists ar ON ar.id = ar_t.artist_id
                WHERE ar_t.track_id = t.id
            ), ''),
            al.name,
            t.path
        FROM tracks t
        JOIN albums al ON al.id = t.album_id
        WHERE t.id = ?1",
            [track_id],
        )
        .unwrap();
    }

    fn migrate(&mut self, music_dir: PathBuf) -> DatabaseResult<()> {
        const VERSION: SqliteId = 1;

        let version = self.sqlite.version();
        let rescan = match version {
            0 => {
                self.sqlite
                    .execute_batch(include_str!("database/schema.sql"))
                    .map_err(|err| DatabaseError {
                        message: String::from("Failed to execute schema"),
                        source: Some(DatabaseErrorSource::Sqlite(err)),
                    })?;
                // TODO: Add dir metadata.
                true
            }
            VERSION => {
                // TODO: Compare dir metadata -> either keep or rescan.
                false
            }
            _ => {
                // TODO: Either delete or purge current db. Rescan.
                true
            }
        };

        if rescan {
            self.scanner = Some(Scanner::new(music_dir));
            self.sqlite.set_version(VERSION);
        }

        Ok(())
    }

    const fn sorted_tracks_query(&self) -> SortedTracksQuery {
        SortedTracksQuery {
            sort: self.sort,
            reverse_sort: self.reverse_sort,
            case_sensitive: false,
        }
    }
}

struct SortedTracksQuery {
    sort: TrackSort,
    reverse_sort: bool,
    case_sensitive: bool,
}

impl SortedTracksQuery {
    fn index(self, row: &Row) -> SqliteResult<usize> {
        Ok(row.get::<_, SqliteId>(0)? as usize)
    }

    fn id(self, row: &Row) -> SqliteResult<TrackId> {
        row.get(1)
    }

    fn table<'a>(&self, row: &'a Row) -> SqliteResult<TrackTable<'a>> {
        Ok(TrackTable {
            index: row.get::<_, SqliteId>(0)? as usize,
            id: row.get(1)?,
            title: row.get_ref(2)?.as_str()?,
            artists: row.get_ref(3)?.as_str()?,
            album: row.get_ref(4)?.as_str()?,
            duration: row.get(5)?,
            rating: row.get(6)?,
        })
    }
}

impl std::fmt::Display for SortedTracksQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let order = if self.reverse_sort { "DESC" } else { "ASC" };
        let case = if self.case_sensitive {
            ""
        } else {
            "COLLATE NOCASE"
        };

        let sort_by = match self.sort {
            TrackSort::Title => format_args!("t.title {case} {order}, t.id {order}"),
            TrackSort::Artist => format_args!("ar.name {case} {order}, t.id {order}"),
            TrackSort::Album => format_args!(
                "al.name {case} {order}, t.album_id {order}, t.number {order}, t.id {order}"
            ),
            TrackSort::Time => format_args!("t.duration {case} {order}, t.id {order}"),
            TrackSort::Rating => format_args!("t.rating {case} {order}, t.id {order}"),
        };

        f.write_fmt(format_args!(
            "SELECT
                ROW_NUMBER() OVER (ORDER BY {sort_by}) - 1 AS row_num,
                t.id,
                t.title,
                GROUP_CONCAT(ar.name, ', ' ORDER BY ar.name {case} {order}) AS artists,
                al.name AS album,
                t.duration,
                t.rating,
                t.album_id,
                t.number
            FROM tracks t
            JOIN artist_tracks ar_t ON ar_t.track_id = t.id
            JOIN artists ar ON ar.id = ar_t.artist_id
            JOIN albums al ON al.id = t.album_id
            GROUP BY t.id
            ORDER BY {sort_by}"
        ))
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
