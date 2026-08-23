use std::collections::VecDeque;

pub struct SlidingWindow {
    size: usize,
    scores: VecDeque<f64>,
}

impl SlidingWindow {
    pub fn new(size: usize) -> Self {
        Self { size, scores: VecDeque::with_capacity(size) }
    }

    pub fn add(&mut self, score: f64) {
        if self.scores.len() == self.size {
            self.scores.pop_front();
        }
        self.scores.push_back(score);
    }

    pub fn average(&self) -> f64 {
        if self.scores.is_empty() { return 0.0; }
        self.scores.iter().sum::<f64>() / self.scores.len() as f64
    }
}

pub struct IntentBrierTracker {
    pub w30: SlidingWindow,
    pub w100: SlidingWindow,
    pub w300: SlidingWindow,
}

impl IntentBrierTracker {
    pub fn new() -> Self {
        Self {
            w30: SlidingWindow::new(30),
            w100: SlidingWindow::new(100),
            w300: SlidingWindow::new(300),
        }
    }

    pub fn record(&mut self, predicted: f64, actual: f64) {
        let brier = (predicted - actual).powi(2);
        self.w30.add(brier);
        self.w100.add(brier);
        self.w300.add(brier);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_sliding_window() {
        let mut tracker = IntentBrierTracker::new();
        tracker.record(0.8, 1.0);
        assert!(tracker.w30.average() > 0.0);
    }
}
