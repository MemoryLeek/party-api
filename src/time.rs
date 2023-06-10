use chrono::{DateTime, Utc};

pub trait TimeService: Clone + Send + Sync + 'static {
    fn now(self) -> DateTime<Utc>;
}

#[derive(Clone)]
pub struct SystemTimeService {}

impl TimeService for SystemTimeService {
    fn now(self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[cfg(test)]
#[derive(Clone)]
pub struct ConstantTimeService {
    value: DateTime<Utc>,
}

#[cfg(test)]
impl ConstantTimeService {
    pub fn new() -> Self {
        Self { value: Utc::now() }
    }
}

#[cfg(test)]
impl TimeService for ConstantTimeService {
    fn now(self) -> DateTime<Utc> {
        self.value
    }
}
