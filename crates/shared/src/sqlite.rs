use std::path::Path;

use rusqlite::{Connection, OptionalExtension};

pub use rusqlite::{Params, Row, Transaction, types::*};

pub type SqliteId = i64;
pub type SqliteError = rusqlite::Error;
pub type SqliteResult<T> = Result<T, SqliteError>;

pub struct Sqlite(Connection);

impl Sqlite {
    pub fn open(path: impl AsRef<Path>) -> SqliteResult<Self> {
        Ok(Self(Connection::open(path)?))
    }

    pub fn open_in_memory() -> SqliteResult<Self> {
        Ok(Self(Connection::open_in_memory()?))
    }

    pub fn path(&self) -> Option<&str> {
        self.0.path()
    }

    pub fn version(&self) -> SqliteId {
        self.0
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap()
    }

    pub fn set_version(&self, version: SqliteId) {
        self.0.pragma_update(None, "user_version", version).unwrap();
    }

    pub fn execute(&self, sql: &str, args: impl Params) -> SqliteResult<usize> {
        self.0.execute(sql, args)
    }

    pub fn execute_batch(&self, sql: &str) -> SqliteResult<()> {
        self.0.execute_batch(sql)
    }

    pub fn transaction(
        &mut self,
        f: impl FnOnce(&Transaction) -> SqliteResult<()>,
    ) -> SqliteResult<()> {
        let transaction = self.0.transaction()?;
        f(&transaction)?;
        transaction.commit()
    }

    pub fn iter<'a, T, F>(
        &'a self,
        sql: &str,
        args: impl Params,
        f: F,
    ) -> SqliteResult<impl Iterator<Item = SqliteResult<T>> + 'a>
    where
        F: for<'row> FnMut(&Row<'row>) -> SqliteResult<T> + 'a,
    {
        let statement = self.0.prepare(sql)?;
        let iter = SqliteIterTryBuilder {
            statement,
            iter_builder: |statement| statement.query_map(args, f),
        }
        .try_build()?;

        Ok(iter)
    }

    pub fn query<T>(
        &self,
        sql: &str,
        args: impl Params,
        mut f: impl FnMut(&Row) -> SqliteResult<T>,
    ) -> SqliteResult<()> {
        let mut statement = self.0.prepare(sql)?;
        let mut rows = statement.query(args)?;

        while let Some(row) = rows.next()? {
            f(row)?;
        }

        Ok(())
    }

    pub fn query_first<T>(
        &self,
        sql: &str,
        args: impl Params,
        f: impl FnOnce(&Row) -> SqliteResult<T>,
    ) -> SqliteResult<Option<T>> {
        self.0.query_row(sql, args, f).optional()
    }

    pub fn query_single<T>(
        &self,
        sql: &str,
        args: impl Params,
        f: impl FnOnce(&Row) -> SqliteResult<T>,
    ) -> SqliteResult<T> {
        self.0.query_one(sql, args, f)
    }

    pub fn last_insert_rowid(&self) -> SqliteId {
        self.0.last_insert_rowid()
    }
}

#[ouroboros::self_referencing]
pub struct SqliteIter<'conn, F>
where
    F: 'conn,
{
    statement: rusqlite::Statement<'conn>,

    #[borrows(mut statement)]
    #[covariant]
    iter: rusqlite::MappedRows<'this, F>,
}

impl<'conn, T, F> Iterator for SqliteIter<'conn, F>
where
    F: for<'row> FnMut(&Row<'row>) -> SqliteResult<T> + 'conn,
{
    type Item = SqliteResult<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.with_iter_mut(|iter| iter.next())
    }
}

// #[ouroboros::self_referencing]
// pub struct SqliteIter<'conn, F> {
//     statement: Statement<'conn>,

//     #[borrows(mut statement)]
//     #[covariant]
//     iter: MappedRows<'this, F>,
// }

// impl<T, F> Iterator for SqliteIter<'_, F>
// where
//     F: for<'a> FnMut(&Row<'a>) -> SqliteResult<T>,
// {
//     type Item = SqliteResult<T>;

//     fn next(&mut self) -> Option<Self::Item> {
//         self.with_iter_mut(|iter| iter.next())
//     }
// }

// impl<'conn, T, F> Iterator for SqliteIter<'conn, F>
// where
//     F: for<'row> FnMut(&Row<'row>) -> SqliteResult<T>,
// {
//     type Item = SqliteResult<T>;

//     fn next(&mut self) -> Option<Self::Item> {
//         self.with_iter_mut(|iter| iter.next())
//     }
// }

// #[ouroboros::self_referencing]
// struct IterInner<'conn, F> {
//     stmt: Statement<'conn>,

//     #[borrows(mut stmt)]
//     #[covariant]
//     rows: Rows<'this>,

//     map: F,
// }

// pub struct SqliteIter<'conn, T, F> {
//     inner: IterInner<'conn, F>,
//     _marker: std::marker::PhantomData<T>,
// }

// impl<T, F> Iterator for SqliteIter<'_, T, F>
// where
//     F: FnMut(&Row<'_>) -> SqliteResult<T>,
// {
//     type Item = SqliteResult<T>;

//     fn next(&mut self) -> Option<Self::Item> {
//         self.inner.with_mut(|fields| match fields.rows.next() {
//             Ok(Some(row)) => Some((fields.map)(row)),
//             Ok(None) => None,
//             Err(err) => Some(Err(err)),
//         })
//     }
// }
