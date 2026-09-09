//! B2 · RA-15 P1-B 测点适配器（opt-in，默认关闭）：把 canonical 回合的实测
//! 数字（token / 延迟 / context_rot 失真）记进统一成本账本。
//!
//! 吸收自 arXiv:2607.08032（rate-distortion 统一）：账本本身在
//! `apeireth_orchestration::research_cost_ledger`（纯内存，默认关闭）；
//! 本模块是**回合级测点**——只有 Context 层的失真可由回合数据诚实代理
//! （context_rot 段平均腐烂分）。Summary/Cache/Vault 各有子系统专有测点
//! （账本已备适配器：`record_summary` / `record_prompt_cache` /
//! `record_vault_retention`），回合级无法诚实代报（0 装）。
//!
//! 生产面板接线（composition root 持账本 + B 块前端面板消费曲线）留
//! B 块；本模块为纯适配器，不挂任何生产路径。

use apeireth_orchestration::research_cost_ledger::{CostTriple, MemoryLayer, ResearchCostLedger};
use apeireth_protocol::canonical::NormalizedUsage;

/// 记一笔回合级成本：Context 层，预算 = prompt tokens，成本 = 总 tokens，
/// 失真 = context_rot 段平均腐烂分（无 rot 数据时失真 0，诚实不编）。
pub fn record_turn_cost(
    ledger: &mut ResearchCostLedger,
    usage: &NormalizedUsage,
    latency_ms: f64,
    rot_scores: &[f32],
    ts_ms: i64,
) -> usize {
    let distortion = if rot_scores.is_empty() {
        0.0
    } else {
        (rot_scores.iter().sum::<f32>() / rot_scores.len() as f32).clamp(0.0, 1.0)
    };
    ledger.record(
        MemoryLayer::Context,
        f64::from(usage.prompt_tokens),
        CostTriple {
            tokens: f64::from(usage.total_tokens),
            latency_ms,
            distortion,
        },
        ts_ms,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 回合测点: 预算=prompt, 成本=total, 失真=rot 均值; 无 rot 失真为 0.
    #[test]
    fn turn_cost_records_context_layer() {
        let mut ledger = ResearchCostLedger::default();
        let usage = NormalizedUsage::new(4000, 500);
        let idx = record_turn_cost(&mut ledger, &usage, 1200.0, &[0.1, 0.3], 7);
        let entry = &ledger.entries()[idx];
        assert_eq!(entry.layer, MemoryLayer::Context);
        assert_eq!(entry.budget_tokens, 4000.0);
        assert_eq!(entry.cost.tokens, 4500.0);
        assert_eq!(entry.cost.latency_ms, 1200.0);
        assert!((entry.cost.distortion - 0.2).abs() < 1e-6);

        // 无 rot 数据: 失真 0 (诚实不编).
        let idx2 = record_turn_cost(&mut ledger, &usage, 900.0, &[], 8);
        assert_eq!(ledger.entries()[idx2].cost.distortion, 0.0);
    }

    /// 两档回合 → 效用-成本曲线可画 (P1-B 面板后端的最小闭环).
    #[test]
    fn two_budget_turns_form_utility_cost_curve() {
        let mut ledger = ResearchCostLedger::default();
        record_turn_cost(
            &mut ledger,
            &NormalizedUsage::new(2000, 300),
            500.0,
            &[0.8],
            1,
        );
        record_turn_cost(
            &mut ledger,
            &NormalizedUsage::new(8000, 500),
            1500.0,
            &[0.2],
            2,
        );
        let curve = ledger.utility_cost_curve(MemoryLayer::Context);
        assert_eq!(curve.len(), 2);
        assert!(curve[0].1 > curve[1].1, "低预算失真必须更高");
        assert!(curve[0].2 < curve[1].2, "低预算成本必须更低");
    }
}
