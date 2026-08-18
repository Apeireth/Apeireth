//! `apeireth-companion::confidence` — Beta-Binomial 置信度 (吸收 hydra genome 置信度数学, 重写).
//!
//! 用途: 能力提案/自测的**数学化自信度** — `conf=91% [89%-93%] obs=25000 strength=STRONG`
//! 纯 Rust 计算, 不依赖 LLM 自报.
//!
//! 模型: 成功数 k / 观察数 n, 先验 (α₀, β₀) = (1, 1) (均匀),
//! 后验均值 E[θ]=(α₀+k)/(α₀+β₀+n), 区间用 Wilson 近似, strength 按观测数分档.

/// Beta-Binomial 置信度估计.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BetaBinomial {
    pub alpha0: f64,
    pub beta0: f64,
    pub successes: u64,
    pub observations: u64,
}

impl Default for BetaBinomial {
    fn default() -> Self {
        Self {
            alpha0: 1.0,
            beta0: 1.0,
            successes: 0,
            observations: 0,
        }
    }
}

/// strength 分档 (按观测数).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strength {
    Weak,
    Moderate,
    Strong,
    VeryStrong,
}

impl Strength {
    pub fn label(self) -> &'static str {
        match self {
            Self::Weak => "WEAK",
            Self::Moderate => "MODERATE",
            Self::Strong => "STRONG",
            Self::VeryStrong => "VERY_STRONG",
        }
    }
}

impl BetaBinomial {
    pub fn new(alpha0: f64, beta0: f64) -> Self {
        Self {
            alpha0: alpha0.max(0.001),
            beta0: beta0.max(0.001),
            successes: 0,
            observations: 0,
        }
    }

    /// 记录一次结果 (成功/失败).
    pub fn observe(&mut self, success: bool) {
        self.observations += 1;
        if success {
            self.successes += 1;
        }
    }

    /// 后验均值 (期望成功率).
    pub fn mean(&self) -> f64 {
        let a = self.alpha0 + self.successes as f64;
        let b = self.beta0 + (self.observations - self.successes) as f64;
        a / (a + b)
    }

    /// Wilson 近似的 95% 区间 (下, 上).
    pub fn interval95(&self) -> (f64, f64) {
        let n = self.observations as f64;
        if n == 0.0 {
            return (0.0, 1.0);
        }
        let p = self.mean();
        let z = 1.96;
        let z2 = z * z;
        let denom = 1.0 + z2 / n;
        let center = (p + z2 / (2.0 * n)) / denom;
        let half = z * ((p * (1.0 - p) + z2 / (4.0 * n)) / n).sqrt() / denom;
        ((center - half).max(0.0), (center + half).min(1.0))
    }

    pub fn strength(&self) -> Strength {
        match self.observations {
            0..=4 => Strength::Weak,
            5..=49 => Strength::Moderate,
            50..=999 => Strength::Strong,
            _ => Strength::VeryStrong,
        }
    }

    /// 一句话自信度报告 (对齐 hydra CCA 格式).
    pub fn report(&self) -> String {
        let (lo, hi) = self.interval95();
        format!(
            "conf={:.0}% [{:.0}%-{:.0}%] obs={} strength={}",
            self.mean() * 100.0,
            lo * 100.0,
            hi * 100.0,
            self.observations,
            self.strength().label(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_is_uninformative() {
        let b = BetaBinomial::default();
        assert!((b.mean() - 0.5).abs() < 1e-9, "无观测 → 均匀先验 0.5");
        assert_eq!(b.strength(), Strength::Weak);
    }

    #[test]
    fn success_drives_mean_up() {
        let mut b = BetaBinomial::default();
        for _ in 0..9 {
            b.observe(true);
        }
        b.observe(false);
        // 均匀先验拉偏: E[θ] = (1+9)/(2+10) = 0.833
        assert!(
            (b.mean() - 10.0 / 12.0).abs() < 0.01,
            "9/10 成功 → ≈0.833: {}",
            b.mean()
        );
        assert_eq!(b.strength(), Strength::Moderate);
        // 大样本下趋近真实值
        let mut many = BetaBinomial::default();
        for _ in 0..99 {
            many.observe(true);
        }
        many.observe(false);
        assert!(
            (many.mean() - 0.98).abs() < 0.01,
            "99/100 → ≈0.98: {}",
            many.mean()
        );
    }

    #[test]
    fn interval_narrows_with_data() {
        let mut few = BetaBinomial::default();
        for _ in 0..5 {
            few.observe(true);
        }
        let (lo1, hi1) = few.interval95();
        let mut many = BetaBinomial::default();
        for _ in 0..500 {
            many.observe(true);
        }
        let (lo2, hi2) = many.interval95();
        assert!(hi2 - lo2 < hi1 - lo1, "观测越多区间越窄");
        assert_eq!(many.strength(), Strength::Strong);
    }

    #[test]
    fn report_format() {
        let mut b = BetaBinomial::default();
        for _ in 0..100 {
            b.observe(true);
        }
        let r = b.report();
        assert!(r.starts_with("conf="), "格式: {r}");
        assert!(r.contains("obs=100"));
        assert!(r.contains("strength=STRONG"));
    }
}
