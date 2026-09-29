//! Brier 校准纯函数 —— 1:1 语义复制自
//! `crates/engine/memory/src/intent_brier.rs`（`brier_score` / `mean_brier`）。
//!
//! 语义锁定（与原文件逐行对齐）：
//! - `brier_score(p, hit)`：`p` 先 `clamp(0.0, 1.0)`；`hit` → `(p-1)^2`，否则 `p^2`。
//!   范围 `[0,1]`，0=完美校准，1=完全猜反。
//! - `mean_brier`：对已对账样本取均值；空样本返回 `0.0`
//!   （原文件对未反馈样本 `filter_map` 后为空返 0.0；本骨架阶段全部样本均已对账，
//!   故直接对 `&[(p, actual)]` 求均值，语义一致）。
//!
//! 注：本骨架（阶段 0/1/2）暂不调用这两个函数——它们预留给协议 §3 阶段 4
//! （PCF 校准）使用，属交付物而非本阶段执行路径。
#![allow(dead_code)] // 阶段 4 校准使用；本骨架阶段 0/1/2 暂不调用。

/// Brier 单条得分：`(p-1)^2` if hit else `p^2`（p 先 clamp [0,1]）。
pub fn brier_score(predicted_confidence: f64, hit: bool) -> f64 {
    let p = predicted_confidence.clamp(0.0, 1.0);
    if hit {
        (p - 1.0).powi(2)
    } else {
        p.powi(2)
    }
}

/// Brier 均值（无样本时返回 0.0）。
///
/// 样本单元为 `(预测概率 p, 实际发生与否 actual)`，对应协议 §2.4 的 PCF 对。
pub fn mean_brier(samples: &[(f64, bool)]) -> f64 {
    if samples.is_empty() {
        0.0
    } else {
        samples
            .iter()
            .map(|&(p, actual)| brier_score(p, actual))
            .sum::<f64>()
            / samples.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brier_boundaries_p_0_and_1() {
        // p=1 命中 → 0（完美）；p=0 未命中 → 0（完美）。
        assert_eq!(brier_score(1.0, true), 0.0);
        assert_eq!(brier_score(0.0, false), 0.0);
        // p=1 未命中 → 1（完全猜反）；p=0 命中 → 1。
        assert_eq!(brier_score(1.0, false), 1.0);
        assert_eq!(brier_score(0.0, true), 1.0);
    }

    #[test]
    fn brier_fifty_fifty_quarter() {
        assert!((brier_score(0.5, true) - 0.25).abs() < 1e-6);
        assert!((brier_score(0.5, false) - 0.25).abs() < 1e-6);
    }

    #[test]
    fn brier_clamps_out_of_range_confidence() {
        // 与原文件 `brier_clamps_confidence` 测试同语义。
        assert_eq!(brier_score(1.5, true), 0.0);
        assert_eq!(brier_score(-0.5, false), 0.0);
        assert_eq!(brier_score(1.5, false), 1.0);
        assert_eq!(brier_score(-0.5, true), 1.0);
    }

    #[test]
    fn mean_brier_empty_returns_zero() {
        assert_eq!(mean_brier(&[]), 0.0);
    }

    #[test]
    fn mean_brier_computes_average() {
        // (0.9, hit)=0.01 + (0.9, miss)=0.81 → 0.41（对齐原文件 mean_brier_computes_average）。
        let samples = [(0.9, true), (0.9, false)];
        assert!((mean_brier(&samples) - 0.41).abs() < 1e-4);
    }
}
