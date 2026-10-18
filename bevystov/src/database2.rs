use std::path::Path;

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

impl Database {
    fn new(sqlite: Sqlite) -> Self {
        Self {
            sqlite,
            receiver: None,
        }
    }

    pub fn open(path: impl AsRef<Path>) -> DatabaseResult<Self> {
        let path = path.as_ref();
        let mut db = Self::new(Sqlite::open(path)?);
        db.migrate(path)?;

        Ok(db)
    }

    pub fn open_in_memory(path: impl AsRef<Path>) -> DatabaseResult<Self> {
        let mut db = Self::new(Sqlite::open_in_memory()?);
        db.migrate(path.as_ref())?;

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

    pub fn update(&mut self) {
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
                    Err(_err) => {
                        // TODO: Log error to db.
                    }
                },
                Err(err) => match err {
                    TryRecvError::Empty => {
                        return;
                    }
                    TryRecvError::Disconnected => {
                        self.receiver = None;
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
        tx.execute(
            "INSERT INTO albums (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
            [track.album()],
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

    fn migrate(&mut self, dir: &Path) -> DatabaseResult<()> {
        const VERSION: SqliteId = 1;

        let version = self.sqlite.version();
        let rescan = match version {
            0 => {
                self.sqlite
                    .execute_batch(include_str!("database2/schema.sql"))?;
                self.sqlite.set_version(VERSION);
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
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrackId(SqliteId);

impl ToSql for TrackId {
    fn to_sql(&self) -> SqliteResult<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Integer(self.0)))
    }
}

impl FromSql for TrackId {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value.as_i64().map(|n| Self(n))
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
