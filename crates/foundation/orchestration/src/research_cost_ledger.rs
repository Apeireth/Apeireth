//! B2 · RA-15 P1-B: 统一内存成本账本（Research 前缀，默认关闭）。
//!
//! # 学术账本（铁律 3）
//! - **问题定义**（吸收自 arXiv:2607.08032）: KV 逐出 / 提示压缩 / 架构状态 /
//!   智能体记忆四个社区各自压缩各自的层, 无人统一度量——这些其实都是同一个
//!   **rate-distortion 决策**: 在资源预算下保留/丢弃哪些上下文衍生信息、以何种
//!   保真度、如何保住下游任务效用。
//! - **对手贡献**: 单一压缩目标 + 层不可知下界 + 七轴分类法；跨层机制迁移；
//!   指出每层信号 (attention 幅度 / recency) 以同样方式失败 (查询未知前丢弃)。
//! - **工程版本**: 四层 (context / summary / cache / vault) 的统一记账——
//!   成本三元组 (token 成本, 延迟, 效用失真) + 效用-成本曲线 + 边际失真率
//!   (每省 1 token 的失真代价) 驱动的压缩顺序推荐。context_rot / prompt cache /
//!   VaultLRU / summary-fold 经适配器接进同一账本（对齐 ra3/ra10 在线成本模型）。
//! - **默认关闭（铁律 1）**: 纯内存记账器, 不挂任何生产路径。
//! - **引用**: arXiv:2607.08032；逐行对照见
//!   `docs/03-reference/absorption-2026-09.md` §P1-B。
//! - **短期价值**: 一个面板就能看到"预算从 8k 到 2k 到底损失多少效用、省多少
//!   成本"——论文图表的后端。

use serde::{Deserialize, Serialize};

/// 成本三元组: (token 成本, 延迟, 效用失真)。
/// distortion ∈ [0,1], 0 = 无损, 1 = 全损（rate-distortion 的 D 项）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CostTriple {
    pub tokens: f64,
    pub latency_ms: f64,
    pub distortion: f32,
}

/// 记忆层（七轴分类在本架构下的工程映射——外部 API 无 KV 层,
/// KV 逐出/压缩类吸收挂 P2 本地推理路线）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryLayer {
    /// 上下文折叠/衰减 (context_rot / context_budget / fold)。
    Context,
    /// 在途摘要与滚动压缩。
    Summary,
    /// prompt 缓存命中/失效。
    Cache,
    /// 长期记忆保留 (VaultLRU / FTRL)。
    Vault,
}

impl MemoryLayer {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Context => "context",
            Self::Summary => "summary",
            Self::Cache => "cache",
            Self::Vault => "vault",
        }
    }
}

/// 单条账目: 某层在某 token 预算档下的实测成本。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub layer: MemoryLayer,
    pub budget_tokens: f64,
    pub cost: CostTriple,
    pub ts_ms: i64,
}

/// 统一内存成本账本（纯内存, 确定性; 默认关闭）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResearchCostLedger {
    entries: Vec<LedgerEntry>,
}

impl ResearchCostLedger {
    /// 记一笔账, 返回条目索引（审计可溯源）。
    pub fn record(
        &mut self,
        layer: MemoryLayer,
        budget_tokens: f64,
        cost: CostTriple,
        ts_ms: i64,
    ) -> usize {
        self.entries.push(LedgerEntry {
            layer,
            budget_tokens,
            cost,
            ts_ms,
        });
        self.entries.len() - 1
    }

    pub fn entries(&self) -> &[LedgerEntry] {
        &self.entries
    }

    /// 某层的汇总三元组: token 与延迟求和, 失真取均值。
    pub fn layer_summary(&self, layer: MemoryLayer) -> CostTriple {
        let mut tokens = 0.0f64;
        let mut latency = 0.0f64;
        let mut distortion = 0.0f32;
        let mut n = 0usize;
        for e in &self.entries {
            if e.layer == layer {
                tokens += e.cost.tokens;
                latency += e.cost.latency_ms;
                distortion += e.cost.distortion;
                n += 1;
            }
        }
        if n == 0 {
            return CostTriple {
                tokens: 0.0,
                latency_ms: 0.0,
                distortion: 0.0,
            };
        }
        CostTriple {
            tokens,
            latency_ms: latency,
            distortion: distortion / n as f32,
        }
    }

    /// 效用-成本曲线: 某层按预算档聚合 → [(预算, 平均失真, 总 token 成本)]。
    /// 预算档升序 = 面板的"从 2k 加到 8k 效用曲线"（论文图表后端）。
    pub fn utility_cost_curve(&self, layer: MemoryLayer) -> Vec<(f64, f32, f64)> {
        let mut buckets: Vec<(f64, Vec<&LedgerEntry>)> = Vec::new();
        for e in &self.entries {
            if e.layer != layer {
                continue;
            }
            match buckets
                .iter_mut()
                .find(|(b, _)| (*b - e.budget_tokens).abs() < f64::EPSILON)
            {
                Some((_, v)) => v.push(e),
                None => buckets.push((e.budget_tokens, vec![e])),
            }
        }
        buckets.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        buckets
            .into_iter()
            .map(|(budget, v)| {
                let n = v.len() as f32;
                let distortion: f32 = v.iter().map(|e| e.cost.distortion).sum::<f32>() / n;
                let tokens: f64 = v.iter().map(|e| e.cost.tokens).sum();
                (budget, distortion, tokens)
            })
            .collect()
    }

    /// 边际失真率: 曲线端点间的 Δdistortion / Δtokens_saved（每省 1 token 的失真代价）。
    /// 曲线少于两点时返回 None（诚实标注：单档无法估计边际）。
    pub fn marginal_distortion_per_token(&self, layer: MemoryLayer) -> Option<f32> {
        let curve = self.utility_cost_curve(layer);
        if curve.len() < 2 {
            return None;
        }
        let (_lo_budget, lo_d, lo_tokens) = curve[0];
        let (_hi_budget, hi_d, hi_tokens) = curve[curve.len() - 1];
        let d_tokens = hi_tokens - lo_tokens;
        if d_tokens <= 0.0 {
            return None;
        }
        // 压缩方向 = 省钱: 每省 1 token 换来的失真上升 = (低预算失真 − 高预算失真) / 省下的 token.
        let d_distortion = (lo_d - hi_d).max(0.0);
        Some(d_distortion / d_tokens as f32)
    }

    /// rate-distortion 贪心: 先压"每省 1 token 失真最小"的层。
    /// 返回按推荐顺序排序的 (层, 边际失真率)；无法估计的层排最后（0 装）。
    pub fn recommend_compaction_order(&self) -> Vec<(MemoryLayer, Option<f32>)> {
        let layers = [
            MemoryLayer::Context,
            MemoryLayer::Summary,
            MemoryLayer::Cache,
            MemoryLayer::Vault,
        ];
        let mut scored: Vec<(MemoryLayer, Option<f32>)> = layers
            .iter()
            .map(|l| (*l, self.marginal_distortion_per_token(*l)))
            .collect();
        scored.sort_by(|a, b| match (a.1, b.1) {
            (Some(x), Some(y)) => x
                .partial_cmp(&y)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.as_str().cmp(b.0.as_str())),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.0.as_str().cmp(b.0.as_str()),
        });
        scored
    }

    // ------------------------------------------------------------------
    // 适配器: 把各子系统既有输出映射进统一账本（均只记账, 不改变子系统）。
    // ------------------------------------------------------------------

    /// context_rot 适配: 失真 = 段平均腐烂分（腐烂 = 已损失/将被剪除的效用）。
    pub fn record_context_rot(
        &mut self,
        budget_tokens: f64,
        latency_ms: f64,
        rot_scores: &[f32],
        ts_ms: i64,
    ) -> usize {
        let distortion = if rot_scores.is_empty() {
            0.0
        } else {
            rot_scores.iter().sum::<f32>() / rot_scores.len() as f32
        };
        self.record(
            MemoryLayer::Context,
            budget_tokens,
            CostTriple {
                tokens: budget_tokens,
                latency_ms,
                distortion: distortion.clamp(0.0, 1.0),
            },
            ts_ms,
        )
    }

    /// prompt cache 适配: 失真 = 1 − 命中率（miss 即重算, 效用损失由重算失真代理）。
    pub fn record_prompt_cache(
        &mut self,
        budget_tokens: f64,
        latency_ms: f64,
        hit_rate: f32,
        ts_ms: i64,
    ) -> usize {
        self.record(
            MemoryLayer::Cache,
            budget_tokens,
            CostTriple {
                tokens: budget_tokens,
                latency_ms,
                distortion: (1.0 - hit_rate).clamp(0.0, 1.0),
            },
            ts_ms,
        )
    }

    /// Vault 保留适配: 失真 = 1 − 保留命中率（被逐出的证据会话占比）。
    pub fn record_vault_retention(
        &mut self,
        budget_tokens: f64,
        latency_ms: f64,
        retention_hit_rate: f32,
        ts_ms: i64,
    ) -> usize {
        self.record(
            MemoryLayer::Vault,
            budget_tokens,
            CostTriple {
                tokens: budget_tokens,
                latency_ms,
                distortion: (1.0 - retention_hit_rate).clamp(0.0, 1.0),
            },
            ts_ms,
        )
    }

    /// summary/fold 适配: 失真由调用方提供（丢失摘要比例）。
    pub fn record_summary(
        &mut self,
        budget_tokens: f64,
        latency_ms: f64,
        distortion: f32,
        ts_ms: i64,
    ) -> usize {
        self.record(
            MemoryLayer::Summary,
            budget_tokens,
            CostTriple {
                tokens: budget_tokens,
                latency_ms,
                distortion: distortion.clamp(0.0, 1.0),
            },
            ts_ms,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ledger() -> ResearchCostLedger {
        ResearchCostLedger::default()
    }

    /// 效用-成本曲线: 高预算低失真高成本, 低预算高失真低成本 (8k vs 2k 面板).
    #[test]
    fn utility_cost_curve_shows_budget_tradeoff() {
        let mut l = ledger();
        l.record_context_rot(8000.0, 40.0, &[0.1, 0.2], 1);
        l.record_context_rot(8000.0, 45.0, &[0.15, 0.25], 2);
        l.record_context_rot(2000.0, 20.0, &[0.6, 0.7], 3);
        let curve = l.utility_cost_curve(MemoryLayer::Context);
        assert_eq!(curve.len(), 2);
        assert_eq!(curve[0].0, 2000.0); // 预算档升序
        assert_eq!(curve[1].0, 8000.0);
        assert!(curve[0].1 > curve[1].1, "低预算失真必须更高");
        assert!(curve[0].2 < curve[1].2, "低预算 token 成本必须更低");
    }

    /// 边际失真率 = Δ失真 / Δ省下的 token.
    #[test]
    fn marginal_distortion_per_token_is_delta_over_tokens_saved() {
        let mut l = ledger();
        l.record_context_rot(2000.0, 20.0, &[0.8], 1);
        l.record_context_rot(8000.0, 50.0, &[0.2], 2);
        let m = l
            .marginal_distortion_per_token(MemoryLayer::Context)
            .unwrap();
        // Δ失真 0.6 / Δtokens 6000 = 1e-4.
        assert!((m - 1e-4).abs() < 1e-7);
    }

    /// 贪心推荐: 边际失真率小的层先压.
    #[test]
    fn recommend_compaction_order_prefers_low_distortion_per_token() {
        let mut l = ledger();
        // context: 0.6/6000 = 1e-4; cache: 0.3/6000 = 5e-5 → cache 先压.
        l.record_context_rot(2000.0, 10.0, &[0.8], 1);
        l.record_context_rot(8000.0, 30.0, &[0.2], 2);
        l.record_prompt_cache(2000.0, 5.0, 0.4, 3);
        l.record_prompt_cache(8000.0, 15.0, 0.7, 4);
        let order = l.recommend_compaction_order();
        assert_eq!(order[0].0, MemoryLayer::Cache);
        assert_eq!(order[1].0, MemoryLayer::Context);
    }

    /// 单档曲线无法估计边际 (0 装 None), 排序落在有估计值的层之后.
    #[test]
    fn single_bucket_layer_has_no_marginal() {
        let mut l = ledger();
        l.record_summary(4000.0, 10.0, 0.3, 1);
        assert_eq!(l.marginal_distortion_per_token(MemoryLayer::Summary), None);
        l.record_context_rot(2000.0, 10.0, &[0.8], 2);
        l.record_context_rot(8000.0, 30.0, &[0.2], 3);
        let order = l.recommend_compaction_order();
        assert_eq!(order[0].0, MemoryLayer::Context);
        assert!(order
            .iter()
            .any(|(l, m)| *l == MemoryLayer::Summary && m.is_none()));
    }

    /// 适配器映射: prompt cache 命中率 → 失真 = 1 - hit_rate; vault 同理.
    #[test]
    fn adapters_map_hit_rates_to_distortion() {
        let mut l = ledger();
        l.record_prompt_cache(1000.0, 1.0, 0.75, 1);
        l.record_vault_retention(1000.0, 1.0, 0.9, 2);
        let cache = l.layer_summary(MemoryLayer::Cache);
        let vault = l.layer_summary(MemoryLayer::Vault);
        assert!((cache.distortion - 0.25).abs() < 1e-6);
        assert!((vault.distortion - 0.1).abs() < 1e-6);
    }
}
