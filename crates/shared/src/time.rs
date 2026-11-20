use std::time::{Duration, Instant};

use crate::sqlite::{FromSql, FromSqlResult, SqliteResult, ToSql, ToSqlOutput, Value, ValueRef};

pub struct Timer {
    interval: Duration,
    last_tick: Option<Instant>,
    timeout: Duration,
    looping: bool,
}

impl Timer {
    pub fn new(fps: u8) -> Self {
        Self {
            interval: Duration::from_secs_f64(1.0 / fps as f64),
            last_tick: None,
            timeout: Duration::ZERO,
            looping: false,
        }
    }

    pub const fn with_loop(mut self) -> Self {
        self.set_loop(true);
        self
    }

    pub fn with_start(mut self) -> Self {
        self.start();
        self
    }

    pub const fn set_loop(&mut self, enabled: bool) {
        self.looping = enabled;
    }

    pub const fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn start(&mut self) {
        self.last_tick = Some(Instant::now());
    }

    pub const fn stop(&mut self) {
        self.last_tick = None;
    }

    pub fn tick(&mut self) -> bool {
        let Some(last_tick) = self.last_tick else {
            return false;
        };

        self.timeout = self.interval.saturating_sub(last_tick.elapsed());

        if !self.timeout.is_zero() {
            return false;
        }

        if self.looping {
            self.start();
        } else {
            self.stop();
        }

        true
    }

    pub fn reset(&mut self) {
        if self.last_tick.is_none() {
            return;
        }

        self.start();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnixTime(u64);

impl UnixTime {
    const SECONDS_PER_DAY: u64 = 86400;

    pub fn now() -> Self {
        let secs_since_unix_epoch = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self(secs_since_unix_epoch)
    }

    pub const fn days_since(self, other: Self) -> u32 {
        (self.0.abs_diff(other.0) / Self::SECONDS_PER_DAY) as u32
    }

    pub const fn add_days(self, days: f32) -> Self {
        let days_in_secs = days * Self::SECONDS_PER_DAY as f32;
        Self(self.0 + days_in_secs as u64)
    }
}

impl ToSql for UnixTime {
    fn to_sql(&self) -> SqliteResult<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Integer(self.0 as i64)))
    }
}

impl FromSql for UnixTime {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value.as_i64().map(|n| Self(n as u64))
    }
}
