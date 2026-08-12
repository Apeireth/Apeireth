//! Audit log for protocol conversions.

use chrono::{DateTime, Utc};
use std::collections::VecDeque;

use crate::protocol::CompatProtocol;

#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub timestamp: DateTime<Utc>,
    pub protocol: CompatProtocol,
    pub direction: AuditDirection,
    pub status: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditDirection {
    Request,
    Response,
}

pub struct AuditLog {
    entries: VecDeque<AuditEntry>,
    max_size: usize,
}

impl AuditLog {
    pub fn new(max_size: usize) -> Self {
        Self { entries: VecDeque::new(), max_size }
    }
    pub fn record(&mut self, entry: AuditEntry) {
        if self.entries.len() >= self.max_size {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }
    pub fn entries(&self) -> impl Iterator<Item = &AuditEntry> {
        self.entries.iter()
    }
    pub fn count(&self) -> usize { self.entries.len() }
}

impl Default for AuditLog {
    fn default() -> Self { Self::new(1000) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_count() {
        let mut log = AuditLog::new(10);
        log.record(AuditEntry {
            timestamp: Utc::now(),
            protocol: CompatProtocol::OpenAIChatCompletions,
            direction: AuditDirection::Request,
            status: 200,
        });
        assert_eq!(log.count(), 1);
    }

    #[test]
    fn max_size_eviction() {
        let mut log = AuditLog::new(3);
        for _ in 0..5 {
            log.record(AuditEntry {
                timestamp: Utc::now(),
                protocol: CompatProtocol::Unknown,
                direction: AuditDirection::Request,
                status: 200,
            });
        }
        assert_eq!(log.count(), 3, "should evict oldest entries");
    }

    #[test]
    fn default_max_is_1000() {
        let log = AuditLog::default();
        assert_eq!(log.max_size, 1000);
    }
}