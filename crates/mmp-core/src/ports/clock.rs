use std::sync::Arc;

use time::{Date, OffsetDateTime};
use time_tz::{OffsetDateTimeExt, Tz, timezones};

pub const DEFAULT_TIMEZONE: &str = "Etc/UTC";

pub trait Clock: Send + Sync + 'static {
    fn now(&self) -> OffsetDateTime;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

// This clock is used so that tests can check timestamps
#[derive(Debug, Clone)]
pub struct FixedClock(OffsetDateTime);

impl FixedClock {
    pub fn new(at: OffsetDateTime) -> Self {
        Self(at)
    }
}

impl Clock for FixedClock {
    fn now(&self) -> OffsetDateTime {
        self.0
    }
}

impl<T: Clock + ?Sized> Clock for Arc<T> {
    fn now(&self) -> OffsetDateTime {
        (**self).now()
    }
}

pub fn resolve_timezone(name: &str) -> Option<&'static Tz> {
    timezones::get_by_name(name)
}

pub struct HouseholdCalendar {
    clock: Arc<dyn Clock>,
    tz: &'static Tz,
}

impl HouseholdCalendar {
    pub fn new(clock: Arc<dyn Clock>, timezone: &str) -> Self {
        let tz = resolve_timezone(timezone)
            .or_else(|| resolve_timezone(DEFAULT_TIMEZONE))
            .expect("Etc/UTC is always resolvable");
        Self { clock, tz }
    }

    pub fn now(&self) -> OffsetDateTime {
        self.clock.now().to_timezone(self.tz)
    }

    pub fn today(&self) -> Date {
        self.now().date()
    }
}
