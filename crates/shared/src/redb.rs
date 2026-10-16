use std::borrow::Borrow;

use redb::{Key, ReadableDatabase, ReadableTable, TableDefinition, Value};

pub trait Item:
    Value
    + Sized
    + for<'a> Value<SelfType<'a> = Self>
    + for<'a> Borrow<<Self as Value>::SelfType<'a>>
    + 'static
{
    type Id: Key
        + for<'a> Key<SelfType<'a> = Self::Id>
        + for<'a> Borrow<<Self::Id as Value>::SelfType<'a>>
        + Increment
        + Default
        + Copy;

    const TABLE: TableDefinition<'_, Self::Id, Self>;
}

trait Increment {
    fn increment(self) -> Self;
}

pub struct ReDb(redb::Database);

type Version = u8;

impl ReDb {
    const VERSION_TABLE: TableDefinition<'_, (), Version> = TableDefinition::new("version");

    pub fn in_memory() -> Result<Self, DbError> {
        let db = Self(
            redb::Database::builder().create_with_backend(redb::backends::InMemoryBackend::new())?,
        );

        // Create version table
        let write = db.0.begin_write()?;
        write.open_table(Self::VERSION_TABLE)?.insert((), 0)?;
        write.commit()?;

        Ok(db)
    }

    pub fn version(&self) -> Result<Version, DbError> {
        Ok(self
            .0
            .begin_read()?
            .open_table(Self::VERSION_TABLE)?
            .get(())?
            .map(|guard| guard.value())
            .unwrap_or_default())
    }

    pub fn get<T: Item>(&self, id: T::Id) -> Result<Option<T>, DbError> {
        let value = self
            .0
            .begin_read()?
            .open_table(T::TABLE)?
            .get(id)?
            .map(|guard| guard.value());

        Ok(value)
    }

    pub fn iter<T: Item>(&self) -> Result<impl Iterator<Item = (T::Id, T)>, DbError> {
        let iter = self
            .0
            .begin_read()?
            .open_table(T::TABLE)?
            .range::<T::Id>(..)?
            .flat_map(|result| result.map(|(id, value)| (id.value(), value.value())));

        Ok(iter)
    }

    pub fn _iter_keys<T: Item>(&self) -> Result<impl Iterator<Item = T::Id>, DbError> {
        let iter = self
            .0
            .begin_read()?
            .open_table(T::TABLE)?
            .range::<T::Id>(..)?
            .flat_map(|result| result.map(|(id, _)| id.value()));

        Ok(iter)
    }

    pub fn _iter_raw<T: Item>(
        &self,
    ) -> Result<impl Iterator<Item = (T::Id, redb::AccessGuard<'_, T>)>, DbError> {
        let iter = self
            .0
            .begin_read()?
            .open_table(T::TABLE)?
            .range::<T::Id>(..)?
            .flat_map(|result| result.map(|(id, value)| (id.value(), value)));

        Ok(iter)
    }

    pub fn add<T: Item>(&self, value: T) -> Result<T::Id, DbError> {
        let write = self.0.begin_write()?;
        let next_id = {
            let mut table = write.open_table(T::TABLE)?;
            let next_id = table
                .last()?
                .map(|(id, _)| id.value())
                .unwrap_or_default()
                .increment();
            table.insert(next_id, value)?;
            next_id
        };
        write.commit()?;

        Ok(next_id)
    }

    pub fn remove<T: Item>(&self, id: T::Id) -> Result<Option<T>, DbError> {
        let write = self.0.begin_write()?;
        let value = {
            let mut table = write.open_table(T::TABLE)?;
            let value = table.remove(id)?.map(|guard| guard.value());
            value
        };
        write.commit()?;

        Ok(value)
    }

    pub fn _update<T: Item, F>(&self, id: T::Id, f: F) -> Result<(), DbError>
    where
        F: FnOnce(&mut T),
    {
        if let Some(mut value) = self.get::<T>(id)? {
            f(&mut value);
            self.insert(id, value)?;
        }

        Ok(())
    }

    fn insert<T: Item>(&self, id: T::Id, value: T) -> Result<Option<T>, DbError> {
        let write = self.0.begin_write()?;
        let value = {
            let mut table = write.open_table(T::TABLE)?;
            let value = table.insert(id, value)?.map(|guard| guard.value());
            value
        };
        write.commit()?;

        Ok(value)
    }

    pub fn _last_id<T: Item>(&self) -> Result<T::Id, DbError> {
        let id = match self.0.begin_read()?.open_table(T::TABLE) {
            Ok(table) => table
                .last()?
                .map(|(id, _)| id.value())
                .unwrap_or_default()
                .increment(),
            Err(table_error) => {
                if let redb::TableError::TableDoesNotExist(_) = table_error {
                    T::Id::default()
                } else {
                    Err(table_error)?
                }
            }
        };

        Ok(id)
    }
}

#[derive(Debug)]
pub enum DbError {
    Error(redb::Error),
    Table(redb::TableError),
    Commit(redb::CommitError),
    Storage(redb::StorageError),
    Database(redb::DatabaseError),
    Savepoint(redb::SavepointError),
    Compaction(redb::CompactionError),
    Transaction(redb::TransactionError),
}

impl From<redb::Error> for DbError {
    fn from(err: redb::Error) -> Self {
        Self::Error(err)
    }
}

impl From<redb::TableError> for DbError {
    fn from(err: redb::TableError) -> Self {
        Self::Table(err)
    }
}

impl From<redb::CommitError> for DbError {
    fn from(err: redb::CommitError) -> Self {
        Self::Commit(err)
    }
}

impl From<redb::StorageError> for DbError {
    fn from(err: redb::StorageError) -> Self {
        Self::Storage(err)
    }
}

impl From<redb::DatabaseError> for DbError {
    fn from(err: redb::DatabaseError) -> Self {
        Self::Database(err)
    }
}

impl From<redb::SavepointError> for DbError {
    fn from(err: redb::SavepointError) -> Self {
        Self::Savepoint(err)
    }
}

impl From<redb::CompactionError> for DbError {
    fn from(err: redb::CompactionError) -> Self {
        Self::Compaction(err)
    }
}

impl From<redb::TransactionError> for DbError {
    fn from(err: redb::TransactionError) -> Self {
        Self::Transaction(err)
    }
}
