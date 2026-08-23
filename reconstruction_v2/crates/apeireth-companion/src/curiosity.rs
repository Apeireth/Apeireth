pub struct CuriosityEngine {
    pub base_bias: f64,
}

impl CuriosityEngine {
    pub fn evaluate(&self, relevance: f64, brier_surprise: f64, novelty: f64) -> f64 {
        self.base_bias + (relevance * 0.4) + (brier_surprise * 0.4) + (novelty * 0.2)
    }
}
