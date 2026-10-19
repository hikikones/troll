use std::{
    ops::Range,
    path::{Path, PathBuf},
};

use shared::sqlite::*;

mod audio2;
mod scanner;
mod track2;

pub use audio2::*;
use scanner::*;
pub use track2::*;

pub type DatabaseResult<T> = Result<T, DatabaseError>;

pub struct Database {
    sqlite: Sqlite,
    receiver: Option<ScanReceiver>,
}

pub enum DatabaseEvent {
    FinishedLoading,
    Rating(TrackId),
    Error(AudioFileError),
}

impl Database {
    fn new(sqlite: Sqlite) -> Self {
        Self {
            sqlite,
            receiver: None,
        }
    }

    pub fn open(path: impl AsRef<Path>, music_dir: PathBuf) -> DatabaseResult<Self> {
        let path = path.as_ref();
        let mut db = Self::new(Sqlite::open(path)?);
        db.migrate(music_dir)?;

        Ok(db)
    }

    pub fn open_in_memory(music_dir: PathBuf) -> DatabaseResult<Self> {
        let mut db = Self::new(Sqlite::open_in_memory()?);
        db.migrate(music_dir)?;

        Ok(db)
    }

    pub fn path(&self) -> Option<&Path> {
        self.sqlite.path().map(Path::new)
    }

    pub const fn has_finished_loading(&self) -> bool {
        self.receiver.is_none()
    }

    pub fn iter<'a, T: 'a>(
        &'a self,
        sql: &str,
        args: impl Params,
        f: impl FnMut(&Row<'_>) -> SqliteResult<T> + 'a,
    ) -> impl Iterator<Item = T> + 'a {
        self.sqlite.iter(sql, args, f).unwrap().map(Result::unwrap)
    }

    pub fn for_each(&self, sql: &str, args: impl Params, f: impl FnMut(&Row) -> SqliteResult<()>) {
        self.sqlite.for_each(sql, args, f).unwrap()
    }

    pub fn first<T>(
        &self,
        sql: &str,
        args: impl Params,
        f: impl FnOnce(&Row) -> SqliteResult<T>,
    ) -> Option<T> {
        self.sqlite.first(sql, args, f).unwrap()
    }

    pub fn single<T>(
        &self,
        sql: &str,
        args: impl Params,
        f: impl FnOnce(&Row) -> SqliteResult<T>,
    ) -> T {
        self.sqlite.single(sql, args, f).unwrap()
    }

    pub fn is_tracks_empty(&self) -> bool {
        self.single("SELECT NOT EXISTS (SELECT 1 FROM tracks)", (), |row| {
            row.get(0)
        })
    }

    pub fn tracks_len(&self) -> usize {
        let len: SqliteId = self.single("SELECT COUNT(*) FROM tracks", (), |row| row.get(0));
        len as usize
    }

    pub fn tracks_id(&self, index: usize) -> TrackId {
        self.single(
            "SELECT id FROM tracks ORDER BY id LIMIT 1 OFFSET ?",
            [index as SqliteId],
            |row| row.get(0),
        )
    }

    pub fn tracks_index(&self, id: TrackId) -> usize {
        let index: SqliteId = self.single(
            "SELECT ROW_NUMBER() OVER (ORDER BY id) - 1 FROM tracks WHERE id = ?",
            [id],
            |row| row.get(0),
        );
        index as usize
    }

    pub fn track_random_id(&self) -> TrackId {
        let len: SqliteId = self.single("SELECT COUNT(*) FROM tracks", (), |row| row.get(0));
        // TODO: Store rng somewhere for reuse
        TrackId(fastrand::i64(0..len))
    }

    pub fn track_title(&self, id: TrackId, mut f: impl FnMut(&str)) {
        self.single("SELECT title FROM tracks WHERE id = ?", [id], |row| {
            Ok(f(row.get_ref(0)?.as_str()?))
        });
    }

    pub fn track_rating(&self, id: TrackId) -> TrackRating {
        self.single("SELECT rating FROM tracks WHERE id = ?", [id], |row| {
            row.get(0)
        })
    }

    pub fn track_path(&self, id: TrackId) -> PathBuf {
        self.single("SELECT path FROM tracks WHERE id = ?", [id], |row| {
            Ok(PathBuf::from(row.get_ref(0)?.as_str()?))
        })
    }

    pub fn tracks_range(&self, range: Range<usize>) -> impl Iterator<Item = TrackId> {
        let (start, end) = (range.start as SqliteId, range.end as SqliteId);
        self.iter(
            "SELECT id FROM tracks LIMIT ?1 OFFSET ?2",
            [end - start, start],
            |row| row.get(0),
        )
    }

    pub fn tracks_table(&self, range: Range<usize>, mut f: impl FnMut(TrackTable)) {
        let (start, end) = (range.start as SqliteId, range.end as SqliteId);
        self.for_each(
            "SELECT
                    ROW_NUMBER() OVER (ORDER BY t.id) - 1,
                    t.id,
                    t.title,
                    GROUP_CONCAT(ar.name, ', ') AS artists,
                    al.name AS album,
                    t.duration,
                    t.rating
                FROM tracks t
                LEFT JOIN artist_tracks atr ON atr.track_id = t.id
                LEFT JOIN artists ar ON ar.id = atr.artist_id
                LEFT JOIN album_tracks atl ON atl.track_id = t.id
                LEFT JOIN albums al ON al.id = atl.album_id
                GROUP BY t.id
                ORDER BY t.id
                LIMIT ?1 OFFSET ?2",
            [end - start, start],
            |row| {
                Ok(f(TrackTable {
                    index: row.get::<_, SqliteId>(0)? as usize,
                    id: row.get(1)?,
                    title: row.get_ref(2)?.as_str()?,
                    artists: row.get_ref(3)?.as_str()?,
                    album: row.get_ref(4)?.as_str()?,
                    duration: row.get(5)?,
                    rating: row.get(6)?,
                }))
            },
        );
    }

    pub fn track_info(&self, id: TrackId, mut f: impl FnMut(TrackInfo)) {
        self.single(
            "SELECT title, rating, duration FROM tracks WHERE id = ?",
            [id],
            |row| {
                f(TrackInfo {
                    title: row.get_ref(0)?.as_str()?,
                    rating: row.get(1)?,
                    duration: row.get(2)?,
                });
                Ok(())
            },
        )
    }

    pub fn track_artists(&self, id: TrackId, mut f: impl FnMut(&str)) {
        self.for_each(
            "SELECT a.name FROM artists AS a \
                JOIN artist_tracks AS at ON at.artist_id = a.id \
                WHERE at.track_id = ?",
            [id],
            |row| Ok(f(row.get_ref(0)?.as_str()?)),
        );
    }

    pub fn track_album(&self, id: TrackId, mut f: impl FnMut(&str)) {
        self.single(
            "SELECT a.name FROM albums AS a \
                JOIN album_tracks AS at ON at.album_id = a.id \
                WHERE at.track_id = ?",
            [id],
            |row| Ok(f(row.get_ref(0)?.as_str()?)),
        );
    }

    pub fn update(&mut self, mut on_event: impl FnMut(DatabaseEvent)) {
        use std::sync::mpsc::TryRecvError;

        let Some(recv) = self.receiver.as_ref() else {
            return;
        };

        loop {
            match recv.try_recv() {
                Ok(res) => match res {
                    Ok(track) => {
                        self.sqlite
                            .transaction(|tx| Self::add_track(tx, track))
                            .unwrap(); // TODO
                    }
                    Err(err) => {
                        on_event(DatabaseEvent::Error(err));
                    }
                },
                Err(err) => match err {
                    TryRecvError::Empty => {
                        return;
                    }
                    TryRecvError::Disconnected => {
                        self.receiver = None;
                        on_event(DatabaseEvent::FinishedLoading);
                        return;
                    }
                },
            }
        }
    }

    fn add_track(tx: &Transaction, track: ScannedTrack) -> SqliteResult<()> {
        // Track
        tx.execute(
            "INSERT INTO tracks (title, rating, duration, path) VALUES (?1, ?2, ?3, ?4)",
            (
                track.title(),
                track.rating(),
                track.duration(),
                track.path(),
            ),
        )?;
        let track_id = tx.last_insert_rowid();

        // Artists
        // TODO: Artists can have same name?
        for artist in track.artists() {
            let rows_changed = tx.execute(
                "INSERT INTO artists (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
                [artist],
            )?;
            let artist_id: SqliteId = if rows_changed > 0 {
                tx.last_insert_rowid()
            } else {
                tx.query_row("SELECT id FROM artists WHERE name = ?1", [artist], |row| {
                    row.get(0)
                })?
            };
            tx.execute("INSERT INTO artist_tracks (artist_id, track_id) VALUES (?1, ?2) ON CONFLICT DO NOTHING", (artist_id, track_id))?;
        }

        // Album
        // TODO: Albums can have same name. Need to compute uniqueness from more data.
        let rows_changed = tx.execute(
            "INSERT INTO albums (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
            [track.album()],
        )?;
        let album_id: SqliteId = if rows_changed > 0 {
            tx.last_insert_rowid()
        } else {
            tx.query_row(
                "SELECT id FROM albums WHERE name = ?1",
                [track.album()],
                |row| row.get(0),
            )?
        };
        tx.execute(
            "INSERT INTO album_tracks (album_id, track_id) VALUES (?1, ?2) ON CONFLICT DO NOTHING",
            (album_id, track_id),
        )?;

        Ok(())
    }

    pub fn print_all(&self) {
        println!("TITLES");
        println!("-------------------------------------------------------------");

        self.sqlite
            .for_each("SELECT * FROM tracks", [], |row| {
                let id: SqliteId = row.get(0).unwrap();
                let title: &str = row.get_ref(1).unwrap().as_str().unwrap();
                let rating: TrackRating = row.get(2).unwrap();
                let dur: TrackDuration = row.get(3).unwrap();
                let path: &str = row.get_ref(4).unwrap().as_str().unwrap();

                println!("Title({id}): {title} -> {rating:?} -> {dur} -> {path}");

                Ok(())
            })
            .unwrap();

        println!("\nARTISTS");
        println!("-------------------------------------------------------------");

        self.sqlite
            .for_each("SELECT * FROM artists", [], |row| {
                let id: SqliteId = row.get(0).unwrap();
                let name: &str = row.get_ref(1).unwrap().as_str().unwrap();

                println!("Artist{id}: {name}");

                Ok(())
            })
            .unwrap();

        println!("\nALBUMS");
        println!("-------------------------------------------------------------");

        self.sqlite
            .for_each("SELECT * FROM albums", [], |row| {
                let id: SqliteId = row.get(0).unwrap();
                let name: &str = row.get_ref(1).unwrap().as_str().unwrap();

                println!("Album{id}: {name}");

                Ok(())
            })
            .unwrap();

        println!("\nARTIST_TRACKS");
        println!("-------------------------------------------------------------");

        self.sqlite
            .for_each("SELECT * FROM artist_tracks", [], |row| {
                let aid: SqliteId = row.get(0).unwrap();
                let tid: SqliteId = row.get(1).unwrap();

                println!("ArtistTracks: ArtistId({aid}) -> TrackId({tid})");

                Ok(())
            })
            .unwrap();

        println!("\nALBUM_TRACKS");
        println!("-------------------------------------------------------------");

        self.sqlite
            .for_each("SELECT * FROM album_tracks", [], |row| {
                let aid: SqliteId = row.get(0).unwrap();
                let tid: SqliteId = row.get(1).unwrap();

                println!("ArtistTracks: AlbumId({aid}) -> TrackId({tid})");

                Ok(())
            })
            .unwrap();
    }

    fn migrate(&mut self, dir: PathBuf) -> DatabaseResult<()> {
        const VERSION: SqliteId = 1;

        let version = self.sqlite.version();
        let rescan = match version {
            0 => {
                self.sqlite
                    .execute_batch(include_str!("database2/schema.sql"))?;
                true
            }
            VERSION => false,
            _ => {
                // TODO: Either delete or purge current db.
                true
            }
        };

        if rescan {
            self.receiver = Some(Scanner::start(dir.to_path_buf()));
            self.sqlite.set_version(VERSION);
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArtistId(SqliteId);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlbumId(SqliteId);

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
pub enum DatabaseError {
    Sqlite(SqliteError),
    Version(SqliteId),
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseError::Sqlite(err) => err.fmt(f),
            DatabaseError::Version(v) => f.write_fmt(format_args!("Unknown version: {v}")),
        }
    }
}

impl std::error::Error for DatabaseError {}

impl From<SqliteError> for DatabaseError {
    fn from(err: SqliteError) -> Self {
        Self::Sqlite(err)
    }
}
