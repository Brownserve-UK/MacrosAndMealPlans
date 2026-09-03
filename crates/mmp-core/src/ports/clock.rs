use std::sync::Arc;

use time::OffsetDateTime;

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
