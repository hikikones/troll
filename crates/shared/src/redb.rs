use std::borrow::Borrow;

pub use redb::{Key as RedbKey, TypeName as RedbTypeName, Value as RedbValue};
use redb::{ReadableDatabase, TableDefinition};

// pub trait RedbTable:
//     RedbValue
//     + Sized
//     + for<'a> RedbValue<SelfType<'a> = Self>
//     + for<'a> Borrow<<Self as RedbValue>::SelfType<'a>>
//     + 'static
// {
//     type Id: RedbKey
//         + for<'a> RedbKey<SelfType<'a> = Self::Id>
//         + for<'a> Borrow<<Self::Id as RedbValue>::SelfType<'a>>
//         + Copy;

//     const NAME: &str;
//     const TABLE: TableDefinition<'_, Self::Id, Self> = TableDefinition::new(Self::NAME);
// }

pub trait RedbTable: 'static {
    type Id: RedbKey
        // + for<'a> RedbKey<SelfType<'a> = Self::Id>
        // + for<'a> Borrow<<Self::Id as RedbValue>::SelfType<'a>>
        + Copy;

    // type Value: RedbValue
    //     + for<'a> RedbKey<SelfType<'a> = Self::Value>
    //     + for<'a> Borrow<<Self::Value as RedbValue>::SelfType<'a>>;
    type Value: RedbValue;

    const NAME: &str;
    const TABLE: TableDefinition<'_, Self::Id, Self::Value> = TableDefinition::new(Self::NAME);
}

type Value<'a, T> = <T as RedbValue>::SelfType<'a>;

pub struct Redb(redb::Database);

type Version = u8;

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

    // pub fn get<T: RedbTable>(&self, id: T::Id) -> Result<Option<T::Value>, RedbError> {
    //     let value = self
    //         .0
    //         .begin_read()?
    //         .open_table(T::TABLE)?
    //         .get(id)?
    //         .map(|guard| guard.value());

    //     Ok(value)
    // }

    pub fn get<T: RedbTable>(
        &self,
        id: Value<'_, T::Id>,
        f: impl FnOnce(Value<'_, T::Value>),
    ) -> Result<(), RedbError> {
        let read = self.0.begin_read()?;
        let table = read.open_table(T::TABLE)?;

        if let Some(guard) = table.get(id)? {
            f(guard.value());
        }

        Ok(())
    }

    // pub fn get<T: RedbTable>(
    //     &self,
    //     id: <T::Id as RedbValue>::SelfType<'_>,
    // ) -> Result<Option<&<T::Value as RedbValue>::SelfType<'_>>, RedbError> {
    //     let read = self.0.begin_read()?;
    //     let table = read.open_table(T::TABLE)?;
    //     let value = table.get(id)?;
    //     match value {
    //         Some(guard) => Ok(Some(&guard.value())),
    //         None => Ok(None),
    //     }
    // }

    pub fn insert<T: RedbTable>(
        &self,
        id: T::Id,
        value: T::Value,
    ) -> Result<Option<T::Value>, RedbError> {
        let write = self.0.begin_write()?;
        let value = {
            let mut table = write.open_table(T::TABLE)?;
            let value = table.insert(id, value)?.map(|guard| guard.value());
            value
        };
        write.commit()?;

        Ok(value)
    }

    pub fn iter<T: RedbTable>(&self) -> Result<impl Iterator<Item = (T::Id, T::Value)>, RedbError> {
        let iter = self
            .0
            .begin_read()?
            .open_table(T::TABLE)?
            .range::<T::Id>(..)?
            .flat_map(|result| result.map(|(id, value)| (id.value(), value.value())));

        Ok(iter)
    }

    pub fn _iter_keys<T: RedbTable>(&self) -> Result<impl Iterator<Item = T::Id>, RedbError> {
        let iter = self
            .0
            .begin_read()?
            .open_table(T::TABLE)?
            .range::<T::Id>(..)?
            .flat_map(|result| result.map(|(id, _)| id.value()));

        Ok(iter)
    }

    pub fn _iter_raw<T: RedbTable>(
        &self,
    ) -> Result<impl Iterator<Item = (T::Id, redb::AccessGuard<'_, T::Value>)>, RedbError> {
        let iter = self
            .0
            .begin_read()?
            .open_table(T::TABLE)?
            .range::<T::Id>(..)?
            .flat_map(|result| result.map(|(id, value)| (id.value(), value)));

        Ok(iter)
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
