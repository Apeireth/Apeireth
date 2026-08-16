//! 可注入时钟 — 让时间敏感机制可测试、可模拟快进.
//!
//! - [`SystemClock`] — 真实时钟 (生产默认).
//! - [`VirtualClock`] — 虚拟时钟 (测试/模拟: `advance` 快进, 0 等待).
//!
//! 用法: 机制持 `Arc<dyn Clock>`, 默认 `SystemClock`; 模拟/测试注入
//! `VirtualClock` 并 `advance(...)`, 即可毫秒级触发「等待型」逻辑
//! (如 SleepCycle 安静期、包过期、淘汰窗口).

use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex};

/// 时间源抽象.
pub trait Clock: Send + Sync {
    /// 当前 UTC 时间.
    fn now(&self) -> DateTime<Utc>;
}

/// 真实时钟 (生产默认).
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// 虚拟时钟: 内部持可变时刻, `advance` 快进 (模拟/测试用).
#[derive(Debug, Clone)]
pub struct VirtualClock {
    now: Arc<Mutex<DateTime<Utc>>>,
}

impl VirtualClock {
    /// 从给定时刻开始.
    pub fn new(start: DateTime<Utc>) -> Self {
        Self {
            now: Arc::new(Mutex::new(start)),
        }
    }

    /// 快进 `d` (虚拟时间, 不真等待).
    pub fn advance(&self, d: chrono::Duration) {
        *self.now.lock().expect("virtual clock poisoned") += d;
    }

    /// 直接设置时刻.
    pub fn set(&self, t: DateTime<Utc>) {
        *self.now.lock().expect("virtual clock poisoned") = t;
    }

    /// 当前虚拟时刻.
    pub fn current(&self) -> DateTime<Utc> {
        *self.now.lock().expect("virtual clock poisoned")
    }
}

impl Clock for VirtualClock {
    fn now(&self) -> DateTime<Utc> {
        self.current()
    }
}

/// `Arc<dyn Clock>` 便捷构造.
pub fn system_clock() -> Arc<dyn Clock> {
    Arc::new(SystemClock)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, TimeZone, Timelike};

    #[test]
    fn system_clock_returns_now() {
        let before = Utc::now();
        let n = SystemClock.now();
        let after = Utc::now();
        assert!(n >= before && n <= after);
    }

    #[test]
    fn virtual_clock_advances_without_waiting() {
        let vc = VirtualClock::new(Utc.with_ymd_and_hms(2026, 8, 16, 6, 0, 0).single().unwrap());
        assert_eq!(vc.now().hour(), 6);
        vc.advance(chrono::Duration::days(90)); // 快进 90 天 (虚拟, 0 等待)
        assert_eq!(vc.current().date_naive().to_string(), "2026-11-14");
        vc.advance(chrono::Duration::days(3));
        assert_eq!(vc.current().date_naive().to_string(), "2026-11-17");
    }

    #[test]
    fn virtual_clock_set() {
        let vc = VirtualClock::new(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).single().unwrap());
        vc.set(
            Utc.with_ymd_and_hms(2026, 8, 15, 22, 36, 0)
                .single()
                .unwrap(),
        );
        assert_eq!(vc.current().year(), 2026);
        assert_eq!(vc.current().month(), 8);
        assert_eq!(vc.current().day(), 15);
    }
}
