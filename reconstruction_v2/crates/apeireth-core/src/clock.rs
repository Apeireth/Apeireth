use chrono::{DateTime, Duration, Utc};
use std::sync::atomic::{AtomicI64, Ordering};

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

pub struct VirtualClock {
    epoch_ms: AtomicI64,
}

impl VirtualClock {
    pub fn new(initial: DateTime<Utc>) -> Self {
        Self {
            epoch_ms: AtomicI64::new(initial.timestamp_millis()),
        }
    }

    pub fn advance(&self, duration: Duration) {
        self.epoch_ms.fetch_add(duration.num_milliseconds(), Ordering::SeqCst);
    }

    pub fn set(&self, new_time: DateTime<Utc>) {
        self.epoch_ms.store(new_time.timestamp_millis(), Ordering::SeqCst);
    }
}

impl Clock for VirtualClock {
    fn now(&self) -> DateTime<Utc> {
        let ms = self.epoch_ms.load(Ordering::SeqCst);
        DateTime::from_timestamp_millis(ms).unwrap_or_else(Utc::now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_clock() {
        let clock = SystemClock;
        let t1 = clock.now();
        let t2 = clock.now();
        assert!(t2 >= t1);
    }

    #[test]
    fn test_virtual_clock_advance() {
        let base = Utc::now();
        let vclock = VirtualClock::new(base);
        assert_eq!(vclock.now().timestamp(), base.timestamp());

        vclock.advance(Duration::seconds(60));
        assert_eq!(vclock.now().timestamp(), base.timestamp() + 60);

        vclock.advance(Duration::hours(2));
        assert_eq!(vclock.now().timestamp(), base.timestamp() + 60 + 7200);
    }
}

