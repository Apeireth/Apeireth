//! Stage 4: Default throttle.

pub const MAX_BURST: u32 = 100;
pub const MAX_CONCURRENT: u32 = 16;
pub const MAX_QPS: u32 = 50;
pub const TOKEN_BUCKET_REFILL_SECS: u64 = 1;

#[derive(Debug, Default, Clone)]
pub struct DefaultThrottle;

impl DefaultThrottle {
    pub fn new() -> Self { Self }
    pub fn admit(&self, current: u32) -> bool {
        current < MAX_BURST
    }
    pub fn qps_ok(&self, qps: u32) -> bool {
        qps <= MAX_QPS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants() {
        assert_eq!(MAX_BURST, 100);
        assert_eq!(MAX_CONCURRENT, 16);
        assert_eq!(MAX_QPS, 50);
        assert_eq!(TOKEN_BUCKET_REFILL_SECS, 1);
    }

    #[test]
    fn admit_under_burst() {
        let t = DefaultThrottle::new();
        assert!(t.admit(50));
        assert!(!t.admit(100));
        assert!(!t.admit(200));
    }

    #[test]
    fn qps_ok() {
        let t = DefaultThrottle::new();
        assert!(t.qps_ok(30));
        assert!(t.qps_ok(50));
        assert!(!t.qps_ok(100));
    }
}
