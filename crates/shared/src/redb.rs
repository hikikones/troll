pub use redb::{Key as RedbKey, TypeName as RedbTypeName, Value as RedbValue};
use redb::{MultimapTableDefinition, ReadableDatabase, ReadableTable, TableDefinition};

pub trait RedbTable: 'static {
    type Key: RedbKey + Copy;
    type Value: RedbValue;

    const NAME: &str;

    const TABLE: TableDefinition<'_, Self::Key, Self::Value> = TableDefinition::new(Self::NAME);
}

pub trait RedbMultimap: 'static {
    type Key: RedbKey + Copy;
    type Value: RedbKey + Copy;

    const NAME: &str;

    const TABLE: MultimapTableDefinition<'_, Self::Key, Self::Value> =
        MultimapTableDefinition::new(Self::NAME);
}

type Key<'a, T> = <T as RedbValue>::SelfType<'a>;
type Value<'a, T> = <T as RedbValue>::SelfType<'a>;

type Version = u8;

pub struct Redb(redb::Database);

impl Redb {
    const VERSION_TABLE: TableDefinition<'_, (), Version> = TableDefinition::new("redb_version");

    pub fn in_memory() -> Result<Self, RedbError> {
        let db = Self(
            redb::Database::builder().create_with_backend(redb::backends::InMemoryBackend::new())?,
        );

        // Create version table
        let write = db.0.begin_write()?;
        write.open_table(Self::VERSION_TABLE)?.insert((), 0)?;
        write.commit()?;

        Ok(db)
    }

    // TODO: Make init_table<T:RedbTable>.
    // TODO: Add a bulk writer.

    pub fn version(&self) -> Result<Version, RedbError> {
        Ok(self
            .0
            .begin_read()?
            .open_table(Self::VERSION_TABLE)?
            .get(())?
            .map(|guard| guard.value())
            .unwrap())
    }

    pub fn set_version(&self, version: Version) -> Result<(), RedbError> {
        let write = self.0.begin_write()?;
        {
            let mut table = write.open_table(Self::VERSION_TABLE)?;
            table.insert((), version)?;
        }
        write.commit()?;

        Ok(())
    }

    pub fn get<T: RedbTable>(
        &self,
        key: Key<'_, T::Key>,
        f: impl FnOnce(Value<'_, T::Value>),
    ) -> Result<(), RedbError> {
        let read = self.0.begin_read()?;
        let table = read.open_table(T::TABLE)?;

        if let Some(guard) = table.get(key)? {
            f(guard.value());
        }

        Ok(())
    }

    pub fn insert<T: RedbTable>(
        &self,
        key: Key<'_, T::Key>,
        value: Value<'_, T::Value>,
    ) -> Result<(), RedbError> {
        let write = self.0.begin_write()?;
        {
            let mut table = write.open_table(T::TABLE)?;
            table.insert(key, value)?;
        }
        write.commit()?;

        Ok(())
    }

    pub fn for_each<T: RedbTable>(
        &self,
        mut f: impl for<'a> FnMut(Key<'a, T::Key>, Value<'a, T::Value>),
    ) -> Result<(), RedbError> {
        let read = self.0.begin_read()?;
        let table = read.open_table(T::TABLE)?;

        for entry in table.iter()? {
            let (key, value) = entry?;
            f(key.value(), value.value());
        }

        Ok(())
    }

    pub fn get_multi<M: RedbMultimap>(
        &self,
        key: Key<'_, M::Key>,
        mut f: impl for<'a> FnMut(Key<'a, M::Value>),
    ) -> Result<(), RedbError> {
        let read = self.0.begin_read()?;
        let table = read.open_multimap_table(M::TABLE)?;

        for entry in table.get(key)? {
            let guard = entry?;
            f(guard.value());
        }

        Ok(())
    }

    pub fn insert_multi<M: RedbMultimap>(
        &self,
        key: Key<'_, M::Key>,
        value: Value<'_, M::Value>,
    ) -> Result<(), RedbError> {
        let write = self.0.begin_write()?;
        {
            let mut table = write.open_multimap_table(M::TABLE)?;
            table.insert(key, value)?;
        }
        write.commit()?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum RedbError {
    Error(redb::Error),
    Table(redb::TableError),
    Commit(redb::CommitError),
    Storage(redb::StorageError),
    Database(redb::DatabaseError),
    Savepoint(redb::SavepointError),
    Compaction(redb::CompactionError),
    Transaction(redb::TransactionError),
}

impl From<redb::Error> for RedbError {
    fn from(err: redb::Error) -> Self {
        Self::Error(err)
    }
}

impl From<redb::TableError> for RedbError {
    fn from(err: redb::TableError) -> Self {
        Self::Table(err)
    }
}

impl From<redb::CommitError> for RedbError {
    fn from(err: redb::CommitError) -> Self {
        Self::Commit(err)
    }
}

impl From<redb::StorageError> for RedbError {
    fn from(err: redb::StorageError) -> Self {
        Self::Storage(err)
    }
}

impl From<redb::DatabaseError> for RedbError {
    fn from(err: redb::DatabaseError) -> Self {
        Self::Database(err)
    }
}

impl From<redb::SavepointError> for RedbError {
    fn from(err: redb::SavepointError) -> Self {
        Self::Savepoint(err)
    }
}

impl From<redb::CompactionError> for RedbError {
    fn from(err: redb::CompactionError) -> Self {
        Self::Compaction(err)
    }
}

impl From<redb::TransactionError> for RedbError {
    fn from(err: redb::TransactionError) -> Self {
        Self::Transaction(err)
    }
}

impl std::fmt::Display for RedbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RedbError::Error(err) => err.fmt(f),
            RedbError::Table(err) => err.fmt(f),
            RedbError::Commit(err) => err.fmt(f),
            RedbError::Storage(err) => err.fmt(f),
            RedbError::Database(err) => err.fmt(f),
            RedbError::Savepoint(err) => err.fmt(f),
            RedbError::Compaction(err) => err.fmt(f),
            RedbError::Transaction(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for RedbError {}

// Example usage
// use shared::redb::*;

// fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let db = Redb::in_memory()?;

//     println!("Version: {}", db.version()?);
//     db.set_version(1)?;
//     println!("Version: {}", db.version()?);

//     db.insert::<Tracks>(TrackId(1), Track::new("Track1"))?;
//     db.insert::<Tracks>(TrackId(2), Track::new("Track2"))?;
//     db.insert::<Tracks>(TrackId(3), Track::new("Track3"))?;

//     db.insert::<Tracks>(TrackId(4), Track::new("Track Unknown"))?;

//     db.insert::<Albums>(AlbumId(1), Album::new("Album"))?;

//     db.insert_multi::<AlbumTracks>(AlbumId(1), TrackId(1))?;
//     db.insert_multi::<AlbumTracks>(AlbumId(1), TrackId(2))?;
//     db.insert_multi::<AlbumTracks>(AlbumId(1), TrackId(3))?;
//     db.insert_multi::<AlbumTracks>(AlbumId(1), TrackId(3))?;

//     println!("\nTracks:");
//     db.for_each::<Tracks>(|id, track| {
//         println!("Id: {id:?}, Title: {}", track.title);
//     })?;

//     println!("\nAlbums:");
//     db.for_each::<Albums>(|id, track| {
//         println!("Id: {id:?}, Title: {}", track.title);
//     })?;

//     println!("\nTracks for AlbumId(1):");
//     db.get_multi::<AlbumTracks>(AlbumId(1), |tid| {
//         println!("TrackId: {tid:?}");
//     })?;

//     Ok(())
// }

// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// struct TrackId(u64);

// #[derive(Debug)]
// struct Track<'a> {
//     title: &'a str,
// }

// impl<'a> Track<'a> {
//     fn new(title: &'a str) -> Self {
//         Self { title }
//     }
// }

// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// struct AlbumId(u64);

// #[derive(Debug)]
// struct Album<'a> {
//     title: &'a str,
// }

// impl<'a> Album<'a> {
//     fn new(title: &'a str) -> Self {
//         Self { title }
//     }
// }

// struct AlbumTracks;

// impl RedbMultimap for AlbumTracks {
//     type Key = AlbumId;
//     type Value = TrackId;
//     const NAME: &str = "AlbumTracks";
// }

// struct Tracks;

// impl RedbTable for Tracks {
//     type Key = TrackId;
//     type Value = TrackValue;
//     const NAME: &str = "Tracks";
// }

// #[derive(Debug)]
// struct TrackValue;

// impl RedbValue for TrackValue {
//     type SelfType<'a> = Track<'a>;
//     type AsBytes<'a> = &'a str;

//     fn fixed_width() -> Option<usize> {
//         None
//     }

//     fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
//     where
//         Self: 'a,
//     {
//         Track {
//             title: <&str as RedbValue>::from_bytes(data),
//         }
//     }

//     fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
//     where
//         Self: 'b,
//     {
//         value.title
//     }

//     fn type_name() -> RedbTypeName {
//         RedbTypeName::new("Track")
//     }
// }

// impl RedbValue for TrackId {
//     type SelfType<'a> = Self;
//     type AsBytes<'a> = <u64 as RedbValue>::AsBytes<'a>;

//     fn fixed_width() -> Option<usize> {
//         <u64 as RedbValue>::fixed_width()
//     }

//     fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
//     where
//         Self: 'a,
//     {
//         Self(<u64 as RedbValue>::from_bytes(data))
//     }

//     fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
//     where
//         Self: 'b,
//     {
//         <u64 as RedbValue>::as_bytes(&value.0)
//     }

//     fn type_name() -> RedbTypeName {
//         RedbTypeName::new("TrackId")
//     }
// }

// impl RedbKey for TrackId {
//     fn compare(data1: &[u8], data2: &[u8]) -> std::cmp::Ordering {
//         <u64 as RedbKey>::compare(data1, data2)
//     }
// }

// struct Albums;

// impl RedbTable for Albums {
//     type Key = AlbumId;
//     type Value = AlbumValue;
//     const NAME: &str = "Albums";
// }

// #[derive(Debug)]
// struct AlbumValue;

// impl RedbValue for AlbumValue {
//     type SelfType<'a> = Album<'a>;
//     type AsBytes<'a> = &'a str;

//     fn fixed_width() -> Option<usize> {
//         None
//     }

//     fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
//     where
//         Self: 'a,
//     {
//         Album {
//             title: <&str as RedbValue>::from_bytes(data),
//         }
//     }

//     fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
//     where
//         Self: 'b,
//     {
//         value.title
//     }

//     fn type_name() -> RedbTypeName {
//         RedbTypeName::new("Album")
//     }
// }

// impl RedbValue for AlbumId {
//     type SelfType<'a> = Self;
//     type AsBytes<'a> = <u64 as RedbValue>::AsBytes<'a>;

//     fn fixed_width() -> Option<usize> {
//         <u64 as RedbValue>::fixed_width()
//     }

//     fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
//     where
//         Self: 'a,
//     {
//         Self(<u64 as RedbValue>::from_bytes(data))
//     }

//     fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
//     where
//         Self: 'b,
//     {
//         <u64 as RedbValue>::as_bytes(&value.0)
//     }

//     fn type_name() -> RedbTypeName {
//         RedbTypeName::new("AlbumId")
//     }
// }

// impl RedbKey for AlbumId {
//     fn compare(data1: &[u8], data2: &[u8]) -> std::cmp::Ordering {
//         <u64 as RedbKey>::compare(data1, data2)
//     }
// }
