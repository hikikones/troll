use shared::sqlite::{
    FromSql, FromSqlError, FromSqlResult, SqliteId, SqliteResult, ToSql, ToSqlOutput, Value,
    ValueRef,
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Debug, Clone, Copy)]
pub struct TrackDuration(u16);

impl TrackDuration {
    pub const fn new(seconds: u16) -> Self {
        Self(seconds)
    }

    pub const fn from_duration(duration: std::time::Duration) -> Self {
        Self(duration.as_secs() as u16)
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
