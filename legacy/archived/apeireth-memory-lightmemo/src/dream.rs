//! Dream subsystem: offline consolidation hook.

use std::sync::Mutex;

/// Callback for consolidating two items (returns merged content).
pub type DreamCallback<'a> = &'a dyn Fn(&str, &str) -> String;

pub struct DreamSubsystem {
    /// Number of consolidation operations performed.
    operations: Mutex<usize>,
}

impl DreamSubsystem {
    pub fn new() -> Self {
        Self { operations: Mutex::new(0) }
    }

    /// Run a dream cycle: iterate items and consolidate via callback.
    /// Returns number of operations performed.
    pub fn dream_cycle(&self, items: &[String], callback: DreamCallback) -> usize {
        let mut count = 0;
        // Pair items (0,1), (2,3), ...
        for pair in items.chunks(2) {
            if pair.len() == 2 {
                let merged = callback(&pair[0], &pair[1]);
                // Operation succeeded; count it
                let _ = merged;
                count += 1;
            }
        }
        let mut ops = self.operations.lock().expect("poisoned");
        *ops += count;
        count
    }

    pub fn total_operations(&self) -> usize {
        *self.operations.lock().expect("poisoned")
    }
}

impl Default for DreamSubsystem {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_cycle() {
        let d = DreamSubsystem::new();
        let n = d.dream_cycle(&[], &|a, b| format!("{}{}", a, b));
        assert_eq!(n, 0);
    }

    #[test]
    fn pair_consolidation() {
        let d = DreamSubsystem::new();
        let n = d.dream_cycle(&["hello".into(), "world".into(), "extra".into()], &|a, b| format!("{}{}", a, b));
        assert_eq!(n, 1, "1 pair (the third 'extra' is unpaired)");
        assert_eq!(d.total_operations(), 1);
    }

    #[test]
    fn two_pairs() {
        let d = DreamSubsystem::new();
        let items = vec!["a".into(), "b".into(), "c".into(), "d".into()];
        let n = d.dream_cycle(&items, &|a, b| format!("{}-{}", a, b));
        assert_eq!(n, 2);
    }
}