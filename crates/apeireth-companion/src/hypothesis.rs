//! `apeireth-companion::hypothesis` — F4 假设检验闭环 (HypothesisStore + 验证调度 + 对账).
//!
//! ## 哲学 (主人 2026-08-18: "这个非常重要, 有了它她才能闭环想法进步")
//!
//! 好奇心决定探索哪 → 世界模型提供推演载体 → **假设检验设计验证** → 记忆提供证据库
//! → 验证结果更新她. 本模块是四原型串链的中枢.
//!
//! 与 W2 的关系: W2 因果边统计验证是**被动**版 (记忆时间线里数次数),
//! 本模块是**主动**版 (她主动提出"如果 X 则 Y", 设计验证, 对账更新).
//!
//! ## 机制 (确定性, 无 LLM)
//!
//! - **HypothesisStore**: 猜想 → 验证中 → 确认/证伪 状态机, 证据加权 (确认 +w / 反证 -w).
//! - **VerifyPlanner**: 验证方式选择 — 低成本观察窗 / 问主人更快 (E4 疑问路由同哲学)
//!   / 可证伪预测 (喂 oracle).
//! - **ReconcileSink**: 对账 trait 口 — 确认/证伪结果写回记忆图 (W2 因果边).
//!   0 装 PASS: 默认 NoopSink, 真写回由调用方接 (不假装已对账).
//!
//! ## 与 E4 的衔接
//!
//! E4 好奇引擎产出探索目标 → 本模块把探索中的可证伪猜想登记为 Hypothesis →
//! 验证 → 对账 → 记忆图因果边 (W3 挖边机制的主动来源).

use std::collections::HashMap;

/// 假设状态 (猜想 → 验证中 → 确认/证伪).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HypothesisStatus {
    Conjecture,
    Verifying,
    Confirmed,
    Refuted,
}

/// 证据来源 (诚实标注).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceSource {
    /// 低成本观察 (本机事件/数据).
    Observation,
    /// 主人回答 (疑问路由: 问主人更快).
    MasterAnswer,
    /// oracle 可证伪预测结算.
    OracleResolve,
}

/// 一条证据 (确认方向 +w, 反证方向 -w).
#[derive(Debug, Clone)]
pub struct Evidence {
    pub source: EvidenceSource,
    /// 正 = 支持假设, 负 = 反驳假设.
    pub weight: f64,
    pub detail: String,
    pub at_ms: i64,
}

impl Evidence {
    pub fn supporting(source: EvidenceSource, weight: f64, detail: impl Into<String>) -> Self {
        Self {
            source,
            weight: weight.abs(),
            detail: detail.into(),
            at_ms: chrono::Utc::now().timestamp_millis(),
        }
    }
    pub fn refuting(source: EvidenceSource, weight: f64, detail: impl Into<String>) -> Self {
        Self {
            source,
            weight: -weight.abs(),
            detail: detail.into(),
            at_ms: chrono::Utc::now().timestamp_millis(),
        }
    }
}

/// 一条假设 (确定性状态机).
#[derive(Debug, Clone)]
pub struct Hypothesis {
    pub id: u64,
    pub statement: String,
    pub status: HypothesisStatus,
    pub evidence: Vec<Evidence>,
    /// 加权证据分 (确认证据和 - 反证证据和).
    pub score: f64,
    pub created_ms: i64,
    pub updated_ms: i64,
}

/// 对账配置.
#[derive(Debug, Clone)]
pub struct HypothesisConfig {
    /// 确认阈值: score ≥ 此值 → Confirmed.
    pub confirm_threshold: f64,
    /// 证伪阈值: score ≤ 此值 → Refuted.
    pub refute_threshold: f64,
    /// 最小证据数才可确认 (防单条大权重拍板).
    pub min_evidence_to_settle: usize,
}

impl Default for HypothesisConfig {
    fn default() -> Self {
        Self {
            confirm_threshold: 2.0,   // 待拟合
            refute_threshold: -2.0,   // 待拟合
            min_evidence_to_settle: 2, // 待拟合
        }
    }
}

/// 假设库 (确定性).
#[derive(Debug)]
pub struct HypothesisStore {
    config: HypothesisConfig,
    items: HashMap<u64, Hypothesis>,
    next_id: u64,
}

impl HypothesisStore {
    pub fn new(config: HypothesisConfig) -> Self {
        Self {
            config,
            items: HashMap::new(),
            next_id: 1,
        }
    }

    /// 登记猜想 (好奇/探索中发现的可证伪命题).
    pub fn conjecture(&mut self, statement: impl Into<String>) -> Hypothesis {
        let now = chrono::Utc::now().timestamp_millis();
        let h = Hypothesis {
            id: self.next_id,
            statement: statement.into(),
            status: HypothesisStatus::Conjecture,
            evidence: Vec::new(),
            score: 0.0,
            created_ms: now,
            updated_ms: now,
        };
        self.next_id += 1;
        self.items.insert(h.id, h.clone());
        h
    }

    /// 开始验证 (Conjecture → Verifying).
    pub fn start_verify(&mut self, id: u64) -> Result<(), String> {
        let h = self.items.get_mut(&id).ok_or("假设不存在")?;
        match h.status {
            HypothesisStatus::Conjecture => {
                h.status = HypothesisStatus::Verifying;
                h.updated_ms = chrono::Utc::now().timestamp_millis();
                Ok(())
            }
            s => Err(format!("状态 {s:?} 不能开始验证 (仅 Conjecture 可)")),
        }
    }

    /// 加证据 → 加权更新状态 (确定性状态机).
    /// 返回更新后的状态.
    pub fn add_evidence(&mut self, id: u64, ev: Evidence) -> Result<HypothesisStatus, String> {
        let h = self.items.get_mut(&id).ok_or("假设不存在")?;
        if matches!(h.status, HypothesisStatus::Confirmed | HypothesisStatus::Refuted) {
            return Err(format!("假设已定论 ({:?}), 不再接受证据", h.status));
        }
        h.score += ev.weight;
        h.evidence.push(ev);
        h.updated_ms = chrono::Utc::now().timestamp_millis();
        // 定论判定: 需要最小证据数 (防单条大权重拍板)
        if h.evidence.len() >= self.config.min_evidence_to_settle {
            if h.score >= self.config.confirm_threshold {
                h.status = HypothesisStatus::Confirmed;
            } else if h.score <= self.config.refute_threshold {
                h.status = HypothesisStatus::Refuted;
            } else {
                h.status = HypothesisStatus::Verifying;
            }
        } else {
            h.status = HypothesisStatus::Verifying;
        }
        Ok(h.status)
    }

    pub fn get(&self, id: u64) -> Option<&Hypothesis> {
        self.items.get(&id)
    }

    pub fn list(&self, status: Option<HypothesisStatus>) -> Vec<&Hypothesis> {
        let mut out: Vec<&Hypothesis> = self
            .items
            .values()
            .filter(|h| status.map_or(true, |s| h.status == s))
            .collect();
        out.sort_by_key(|h| std::cmp::Reverse(h.updated_ms));
        out
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}

/// 验证计划 (VerifyPlanner 产出).
#[derive(Debug, Clone, PartialEq)]
pub enum VerifyPlan {
    /// 低成本观察窗: 观察 N 小时内的相关信号 (本机事件/数据).
    ObserveWindow { hours: f64 },
    /// 问主人更快 (E4 疑问路由哲学: 不硬分线).
    AskMaster { question: String },
    /// 可证伪预测: 喂 oracle, deadline 内结算.
    OracleResolve { deadline_ms: i64 },
}

/// 验证方式选择配置.
#[derive(Debug, Clone)]
pub struct PlannerConfig {
    /// 观察窗成本 (token 量级).
    pub observe_cost: f64,
    /// 问主人成本 (打扰主人, 高).
    pub ask_cost: f64,
    /// 预算阈值: 低于此成本倾向观察窗.
    pub budget: f64,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            observe_cost: 50.0,
            ask_cost: 300.0,
            budget: 400.0, // > ask_cost: 预算内问主人可行 (否则永不 AskMaster)
        }
    }
}

/// 验证调度器 (确定性: 成本最低的验证方式优先, 主人可答且观察不可行的才问).
#[derive(Debug)]
pub struct VerifyPlanner {
    config: PlannerConfig,
}

impl VerifyPlanner {
    pub fn new(config: PlannerConfig) -> Self {
        Self { config }
    }

    pub fn plan(&self, h: &Hypothesis, observable: bool) -> VerifyPlan {
        if observable && self.config.observe_cost <= self.config.budget {
            VerifyPlan::ObserveWindow { hours: 24.0 }
        } else if self.config.ask_cost <= self.config.budget {
            VerifyPlan::AskMaster {
                question: format!("关于『{}』, 想确认一下——", h.statement),
            }
        } else {
            VerifyPlan::OracleResolve {
                deadline_ms: 7 * 24 * 3600 * 1000, // 7 天
            }
        }
    }
}

/// 对账 sink: 定论结果写回记忆图 (W2 因果边) — trait 口, 默认 no-op.
/// 0 装 PASS: 不假装已对账; 由调用方决定是否接 memory_graph.
pub trait ReconcileSink: Send + Sync {
    fn write_back(&mut self, h: &Hypothesis) -> Result<(), String>;
}

/// 默认 no-op sink (诚实: 未接真对账).
#[derive(Debug, Default)]
pub struct NoopSink;

impl ReconcileSink for NoopSink {
    fn write_back(&mut self, _h: &Hypothesis) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conjecture_to_verify_to_confirm() {
        let mut store = HypothesisStore::new(HypothesisConfig::default());
        let h = store.conjecture("主人熬夜 → 次日效率低");
        assert_eq!(h.status, HypothesisStatus::Conjecture);
        assert!(store.start_verify(h.id).is_ok());
        assert_eq!(store.get(h.id).unwrap().status, HypothesisStatus::Verifying);

        store
            .add_evidence(h.id, Evidence::supporting(EvidenceSource::Observation, 1.2, "7 次熬夜记录中 5 次效率低"))
            .unwrap();
        store
            .add_evidence(h.id, Evidence::supporting(EvidenceSource::MasterAnswer, 1.0, "主人确认: 熬夜后确实没精神"))
            .unwrap();
        assert_eq!(store.get(h.id).unwrap().status, HypothesisStatus::Confirmed);
    }

    #[test]
    fn refuting_evidence_outweighs() {
        let mut store = HypothesisStore::new(HypothesisConfig::default());
        let h = store.conjecture("雨天 → 主人心情差");
        store.start_verify(h.id).unwrap();
        store
            .add_evidence(h.id, Evidence::supporting(EvidenceSource::Observation, 1.0, "一次雨天低落"))
            .unwrap();
        store
            .add_evidence(h.id, Evidence::refuting(EvidenceSource::MasterAnswer, 3.0, "主人: 下雨天其实很舒服"))
            .unwrap();
        assert_eq!(store.get(h.id).unwrap().status, HypothesisStatus::Refuted);
    }

    #[test]
    fn settled_hypothesis_rejects_evidence() {
        let mut store = HypothesisStore::new(HypothesisConfig::default());
        let h = store.conjecture("X");
        store.start_verify(h.id).unwrap();
        store.add_evidence(h.id, Evidence::supporting(EvidenceSource::Observation, 1.5, "a")).unwrap();
        store.add_evidence(h.id, Evidence::supporting(EvidenceSource::Observation, 1.0, "b")).unwrap();
        assert_eq!(store.get(h.id).unwrap().status, HypothesisStatus::Confirmed);
        assert!(
            store
                .add_evidence(h.id, Evidence::refuting(EvidenceSource::Observation, 5.0, "late"))
                .is_err(),
            "已定论假设不接受新证据"
        );
    }

    #[test]
    fn min_evidence_prevents_single_big_weight_settlement() {
        let mut store = HypothesisStore::new(HypothesisConfig::default());
        let h = store.conjecture("Y");
        store.start_verify(h.id).unwrap();
        // 单条 5.0 支持证据, 但 min_evidence_to_settle=2 → 不能确认
        store.add_evidence(h.id, Evidence::supporting(EvidenceSource::MasterAnswer, 5.0, "一锤定音")).unwrap();
        assert_ne!(store.get(h.id).unwrap().status, HypothesisStatus::Confirmed);
    }

    #[test]
    fn planner_prefers_low_cost_observation() {
        let planner = VerifyPlanner::new(PlannerConfig::default());
        let mut store = HypothesisStore::new(HypothesisConfig::default());
        let h = store.conjecture("可观察命题");
        let plan = planner.plan(&h, true);
        assert_eq!(plan, VerifyPlan::ObserveWindow { hours: 24.0 });
        // 不可观察 → 问主人 (成本在预算内)
        let plan2 = planner.plan(&h, false);
        assert!(matches!(plan2, VerifyPlan::AskMaster { .. }));
    }

    #[test]
    fn noop_sink_is_honest_noop() {
        let mut sink = NoopSink;
        let mut store = HypothesisStore::new(HypothesisConfig::default());
        let h = store.conjecture("noop");
        assert!(sink.write_back(&h).is_ok());
    }
}
