use std::path::Path;

use shared::sqlite::{
    FromSql, FromSqlResult, SqliteId, SqliteResult, ToSql, ToSqlOutput, Value, ValueRef,
};

use super::{AlbumId, AudioDuration, AudioRating};

#[derive(Debug)]
pub struct TrackInfo<'a> {
    pub title: &'a str,
    pub duration: AudioDuration,
    pub rating: AudioRating,
    pub bit_rate_kbps: u32,
    pub bit_depth: Option<u8>,
    pub sample_rate: Option<u32>,
    pub path: &'a Path,
    pub album_id: AlbumId,
}

#[derive(Debug)]
pub struct TrackTable<'a> {
    pub title: &'a str,
    pub artists: &'a str,
    pub album: &'a str,
    pub duration: AudioDuration,
    pub rating: AudioRating,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrackId(pub u32);

impl TrackId {
    pub const ZERO: Self = Self(0);
}

impl ToSql for TrackId {
    fn to_sql(&self) -> SqliteResult<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Integer(self.0 as SqliteId)))
    }
}

impl FromSql for TrackId {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value.as_i64().map(|n| Self(n as u32))
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum TrackSort {
    Title,
    Artist,
    #[default]
    Album,
    Time,
    Rating,
}

impl TrackSort {
    pub const fn next(self) -> Self {
        match self {
            Self::Title => Self::Artist,
            Self::Artist => Self::Album,
            Self::Album => Self::Time,
            Self::Time => Self::Rating,
            Self::Rating => Self::Title,
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::Title => Self::Rating,
            Self::Artist => Self::Title,
            Self::Album => Self::Artist,
            Self::Time => Self::Album,
            Self::Rating => Self::Time,
        }
    }

    pub const fn equals(self, other: Self) -> bool {
        self as u8 == other as u8
    }
}
