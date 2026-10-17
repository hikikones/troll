use std::{collections::HashSet, path::Path};

use shared::sqlite::*;

mod audio2;
mod track2;

pub use audio2::*;
pub use track2::*;

pub type DatabaseResult<T> = Result<T, DatabaseError>;

pub struct Database {
    sqlite: Sqlite,
}

impl Database {
    fn new(sqlite: Sqlite) -> Self {
        Self { sqlite }
    }

    pub fn open(path: impl AsRef<Path>) -> DatabaseResult<Self> {
        let db = Self::new(Sqlite::open(path)?);
        db.migrate()?;
        Ok(db)
    }

    pub fn open_in_memory(music_dir: &Path) -> DatabaseResult<Self> {
        let mut db = Self::new(Sqlite::open_in_memory()?);
        db.migrate()?;

        db.sqlite.transaction(|tx| {
            // let mut artist_ids = HashSet::new();

            traverse_files(music_dir, true, |path, ext| {
                let Ok(audio) = AudioFile::read(path, ext) else {
                    // TODO: write log to db
                    return;
                };

                let Some(tag) = audio.vorbis_comments() else {
                    // TODO: write log to db
                    return;
                };
                let p = audio.properties();

                // Track
                let title = tag.get("TITLE").unwrap_or("");
                let rating = tag
                    .get("RATING")
                    .map(TrackRating::from_vorbis_comments)
                    .unwrap_or(TrackRating::None);
                let duration = TrackDuration::from_duration(p.duration);

                tx.execute("INSERT INTO tracks (title, rating, duration, path) VALUES (?1, ?2, ?3, ?4)", (title, rating, duration, path.to_string_lossy())).unwrap();

                let track_id = tx.last_insert_rowid();

                // Artists
                for artist in tag.get_all("ARTIST") {
                    let rows_changed = tx.execute("INSERT INTO artists (name) VALUES (?1) ON CONFLICT(name) DO NOTHING", [artist]).unwrap();
                    let artist_id: SqliteId = if rows_changed > 0 {
                        tx.last_insert_rowid()
                    } else {
                        tx.query_row("SELECT id FROM artists WHERE name = ?1",[artist], |row| row.get(0)).unwrap()
                    };
                    tx.execute("INSERT INTO artist_tracks (artist_id, track_id) VALUES (?1, ?2) ON CONFLICT DO NOTHING", (artist_id, track_id)).unwrap();
                }

                // Album
                let album = tag.get("ALBUM").unwrap_or("");
                // TODO: Albums can have same name. Need to compute uniqueness from more data.
                tx.execute("INSERT INTO albums (name) VALUES (?1) ON CONFLICT(name) DO NOTHING", [album]).unwrap();

                // println!("------------------------");

                // for (key, value) in tag.items() {
                //     match key {
                //         "TITLE" | "ARTIST" | "ARTISTS" | "ALBUM" | "ALBUMARTIST" => {
                //             println!("{key} -> {value}")
                //         }
                //         _ => {}
                //     }
                // }
            });

            Ok(())
        })?;

        Ok(db)
    }

    pub fn path(&self) -> Option<&Path> {
        self.sqlite.path().map(Path::new)
    }

    pub fn print_all(&self) {
        println!("TITLES");
        println!("-------------------------------------------------------------");

        self.sqlite
            .query("SELECT * FROM tracks", [], |row| {
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
            .query("SELECT * FROM artists", [], |row| {
                let id: SqliteId = row.get(0).unwrap();
                let name: &str = row.get_ref(1).unwrap().as_str().unwrap();

                println!("Artist{id}: {name}");

                Ok(())
            })
            .unwrap();

        println!("\nALBUMS");
        println!("-------------------------------------------------------------");

        self.sqlite
            .query("SELECT * FROM albums", [], |row| {
                let id: SqliteId = row.get(0).unwrap();
                let name: &str = row.get_ref(1).unwrap().as_str().unwrap();

                println!("Album{id}: {name}");

                Ok(())
            })
            .unwrap();

        println!("\nARTIST_TRACKS");
        println!("-------------------------------------------------------------");

        self.sqlite
            .query("SELECT * FROM artist_tracks", [], |row| {
                let aid: SqliteId = row.get(0).unwrap();
                let tid: SqliteId = row.get(1).unwrap();

                println!("ArtistTracks: ArtistId({aid}) -> TrackId({tid})");

                Ok(())
            })
            .unwrap();

        println!("\nALBUM_TRACKS");
        println!("-------------------------------------------------------------");

        self.sqlite
            .query("SELECT * FROM album_tracks", [], |row| {
                let aid: SqliteId = row.get(0).unwrap();
                let tid: SqliteId = row.get(1).unwrap();

                println!("ArtistTracks: AlbumId({aid}) -> TrackId({tid})");

                Ok(())
            })
            .unwrap();
    }

    fn migrate(&self) -> DatabaseResult<()> {
        const VERSION: SqliteId = 1;

        let version = self.sqlite.version();
        match version {
            0 => {
                self.sqlite
                    .execute_batch(include_str!("database2/schema.sql"))?;
                self.sqlite.set_version(VERSION);
            }
            VERSION => {}
            _ => {
                // TODO: Schema has changed since last time. Rebuild from scratch.
            }
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

fn traverse_files(root: &Path, follow_links: bool, f: impl Fn(&Path, AudioExt)) {
    walkdir::WalkDir::new(root)
        .follow_links(follow_links)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| entry.file_type().is_file())
        .for_each(|entry| {
            let path = entry.path();
            if let Some(ext) = AudioExt::from_path(path) {
                f(path, ext);
            }
        });
}
