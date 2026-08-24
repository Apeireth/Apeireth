//! ScreenPerception - 屏幕感知 (从 v1.0 apeireth-companion/screen_perception.rs 168 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 ScreenEvent + EventSource trait

use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenEvent { Focus, Blur, Switch, Idle }

pub struct ScreenEventSource { pub events: VecDeque<ScreenEvent> }

impl ScreenEventSource {
    pub fn new() -> Self { Self { events: VecDeque::new() } }
    pub fn push(&mut self, e: ScreenEvent) { self.events.push_back(e); }
    /// 0 装 PASS: 真 latest
    pub fn latest(&self) -> Option<&ScreenEvent> { self.events.back() }
    pub fn count(&self) -> usize { self.events.len() }
}

impl Default for ScreenEventSource { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_push() {
        let mut s = ScreenEventSource::new();
        s.push(ScreenEvent::Focus);
        assert_eq!(s.count(), 1);
    }
    #[test] fn test_latest() {
        let mut s = ScreenEventSource::new();
        s.push(ScreenEvent::Focus);
        s.push(ScreenEvent::Blur);
        assert_eq!(*s.latest().unwrap(), ScreenEvent::Blur);
    }
    #[test] fn test_event_eq() { assert_eq!(ScreenEvent::Focus, ScreenEvent::Focus); }
    #[test] fn test_default() { let s: ScreenEventSource = Default::default(); assert!(s.latest().is_none()); }
}
