use std::{path::Path, time::Duration};

use shared::sqlite::{
    FromSql, FromSqlError, FromSqlResult, SqliteId, SqliteResult, ToSql, ToSqlOutput, Value,
    ValueRef,
};

use crate::database::AlbumId;

#[derive(Debug)]
pub struct TrackInfo<'a> {
    pub title: &'a str,
    pub duration: TrackDuration,
    pub rating: TrackRating,
    pub bit_rate_kbps: u32,
    pub bit_depth: Option<u8>,
    pub sample_rate: Option<u32>,
    pub path: &'a Path,
    pub album_id: AlbumId,
}

#[derive(Debug)]
pub struct TrackTable<'a> {
    pub index: usize,
    pub id: TrackId,
    pub title: &'a str,
    pub artists: &'a str,
    pub album: &'a str,
    pub duration: TrackDuration,
    pub rating: TrackRating,
}

#[derive(Debug)]
pub struct TrackText<'a> {
    pub title: &'a str,
    pub artists: &'a str,
    pub album: &'a str,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrackId(pub SqliteId);

impl TrackId {
    pub const ZERO: Self = Self(0);
}

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

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde_repr::Serialize_repr,
    serde_repr::Deserialize_repr,
)]
#[repr(u8)]
pub enum TrackRating {
    #[default]
    None = 0,
    Awful = 1,
    Bad = 2,
    Ok = 3,
    Good = 4,
    Amazing = 5,
}

impl TrackRating {
    pub fn from_vorbis_comments(value: &str) -> Self {
        value
            .parse::<u8>()
            .map(|num| match num {
                0 => Self::None,
                ..=20 => Self::Awful,
                ..=40 => Self::Bad,
                ..=60 => Self::Ok,
                ..=80 => Self::Good,
                ..=255 => Self::Amazing,
            })
            .unwrap_or_default()
    }

    pub const fn into_vorbis_comments(self) -> &'static str {
        match self {
            Self::None => "0",
            Self::Awful => "20",
            Self::Bad => "40",
            Self::Ok => "60",
            Self::Good => "80",
            Self::Amazing => "100",
        }
    }

    pub const fn from_char(c: char) -> Option<Self> {
        match c {
            '0' => Some(Self::None),
            '1' => Some(Self::Awful),
            '2' => Some(Self::Bad),
            '3' => Some(Self::Ok),
            '4' => Some(Self::Good),
            '5' => Some(Self::Amazing),
            _ => None,
        }
    }

    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    pub const fn stars(self) -> &'static str {
        use shared::symbols::{STAR, repeat};
        match self {
            Self::None => "",
            Self::Awful => STAR,
            Self::Bad => repeat!(STAR, 2),
            Self::Ok => repeat!(STAR, 3),
            Self::Good => repeat!(STAR, 4),
            Self::Amazing => repeat!(STAR, 5),
        }
    }

    pub const fn stars_split(self) -> (&'static str, &'static str) {
        use shared::symbols::{STAR_BIG, repeat};
        match self {
            Self::None => (repeat!(STAR_BIG, 0), repeat!(STAR_BIG, 5)),
            Self::Awful => (repeat!(STAR_BIG, 1), repeat!(STAR_BIG, 4)),
            Self::Bad => (repeat!(STAR_BIG, 2), repeat!(STAR_BIG, 3)),
            Self::Ok => (repeat!(STAR_BIG, 3), repeat!(STAR_BIG, 2)),
            Self::Good => (repeat!(STAR_BIG, 4), repeat!(STAR_BIG, 1)),
            Self::Amazing => (repeat!(STAR_BIG, 5), repeat!(STAR_BIG, 0)),
        }
    }
}

impl ToSql for TrackRating {
    fn to_sql(&self) -> SqliteResult<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Integer(*self as SqliteId)))
    }
}

impl FromSql for TrackRating {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value.as_i64().and_then(|n| match n {
            0 => Ok(Self::None),
            1 => Ok(Self::Awful),
            2 => Ok(Self::Bad),
            3 => Ok(Self::Ok),
            4 => Ok(Self::Good),
            5 => Ok(Self::Amazing),
            _ => Err(FromSqlError::Other(
                format!("Track rating {n} is out of range (0-5)").into(),
            )),
        })
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TrackDuration(u16);

impl TrackDuration {
    pub const fn new(seconds: u16) -> Self {
        Self(seconds)
    }

    pub const fn from_duration(duration: Duration) -> Self {
        Self(duration.as_secs() as u16)
    }

    pub const fn progress(self, other: Self) -> f32 {
        if self.0 >= other.0 || other.0 == 0 {
            return 1.0;
        }

        self.as_f32() / other.as_f32()
    }

    pub const fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }

    pub const fn sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    pub const fn mul_f32(self, v: f32) -> Self {
        Self((self.0 as f32 * v) as u16)
    }

    pub const fn max(self, other: Self) -> Self {
        if self.0 < other.0 { other } else { self }
    }

    pub const fn as_f32(self) -> f32 {
        self.0 as f32
    }

    pub const fn as_duration(self) -> Duration {
        Duration::from_secs(self.0 as u64)
    }
}

impl From<Duration> for TrackDuration {
    fn from(value: Duration) -> Self {
        Self::from_duration(value)
    }
}

impl From<TrackDuration> for Duration {
    fn from(value: TrackDuration) -> Self {
        value.as_duration()
    }
}

impl std::fmt::Display for TrackDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let seconds = self.0 % 60;
        let minutes = (self.0 - seconds) / 60;
        f.write_fmt(format_args!("{:02}:{:02}", minutes, seconds))
    }
}

impl ToSql for TrackDuration {
    fn to_sql(&self) -> SqliteResult<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Integer(self.0 as SqliteId)))
    }
}

impl FromSql for TrackDuration {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value.as_i64().map(|n| Self(n as u16))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrackDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl TrackDate {
    /// Parses dates in ISO 8601 "YYYY-MM-DD".
    /// Months and days defaults to zero when not found.
    pub fn parse(s: &str) -> Option<Self> {
        let mut parts = s.split('-');

        let year = parts.next()?.parse().ok()?;
        let month = parts
            .next()
            .map(|s| s.parse().unwrap_or_default())
            .unwrap_or_default();
        let day = parts
            .next()
            .map(|s| s.parse().unwrap_or_default())
            .unwrap_or_default();

        Some(Self { year, month, day })
    }

    pub const fn from_u32(yyyymmdd: u32) -> Self {
        Self {
            year: (yyyymmdd / 10_000) as u16,
            month: ((yyyymmdd / 100) % 100) as u8,
            day: (yyyymmdd % 100) as u8,
        }
    }

    pub const fn as_u32(self) -> u32 {
        self.year as u32 * 10_000 + self.month as u32 * 100 + self.day as u32
    }
}

impl std::fmt::Display for TrackDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{:04}-{:02}-{:02}",
            self.year, self.month, self.day
        ))
    }
}

impl ToSql for TrackDate {
    fn to_sql(&self) -> SqliteResult<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(
            Value::Integer(self.as_u32() as SqliteId),
        ))
    }
}

impl FromSql for TrackDate {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value.as_i64().map(|n| Self::from_u32(n as u32))
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

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Title => "Title",
            Self::Artist => "Artist",
            Self::Album => "Album",
            Self::Time => "Time",
            Self::Rating => "Rating",
        }
    }

    pub const fn equals(self, other: Self) -> bool {
        self as u8 == other as u8
    }
}
