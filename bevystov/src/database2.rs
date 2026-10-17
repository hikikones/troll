use std::path::Path;

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
        let db = Self::new(Sqlite::open_in_memory()?);
        db.migrate()?;

        // TODO: remove
        traverse_files(music_dir, true, |path, ext| {
            let Ok(audio) = AudioFile::read(path, ext) else {
                // TODO: write log to db
                return;
            };

            let Some(vorbis_comments) = audio.vorbis_comments() else {
                // TODO: write log to db
                return;
            };

            println!("------------------------");

            for (key, value) in vorbis_comments.items() {
                match key {
                    "TITLE" | "ARTIST" | "ARTISTS" | "ALBUM" | "ALBUMARTIST" => {
                        println!("{key} -> {value}")
                    }
                    _ => {}
                }
            }
        });

        Ok(db)
    }

    pub fn path(&self) -> Option<&Path> {
        self.sqlite.path().map(Path::new)
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
            _ => return Err(DatabaseError::Version(version)),
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
