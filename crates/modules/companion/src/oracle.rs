//! `apeireth-companion::oracle` — 预测决策一体沙盘 (oracle-suite 核心机制件).
//!
//! 哲学 (docs/oracle-suite-design.md):
//! - 预测 = 可证伪断言「在 T 前 X 发生概率 P」+ 对照 resolve + 校准评分 (Brier)
//! - 决策 = 分支推演中选优 (期望值 Σ P×V)
//! - 共用沙盘底座: 虚拟时钟快进分支 + 事件溯源 (SessionLog) + 校准数学 (confidence)
//!
//! 0 假装: v1 = 一层决策树 (expectimax-lite) + 规则层 apply; 不确定性裁决已接真
//! (`CalibratedResolver`: BetaBinomial 校准追踪 + ForecastRegistry 对照), LLM 裁决
//! 仍留 `UncertaintyResolver` trait 口; 多轮 MCTS 是下一步.

use std::collections::HashMap;
use std::sync::Arc;

use apeireth_memory::{CoreEpisode, EpisodeStore, SqliteMemoryStore};
use async_trait::async_trait;

use crate::confidence::BetaBinomial;

// ============================================================
// 世界状态 + 情景引擎
// ============================================================

/// 世界实体 (沙盘中的对象/主体).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Entity {
    pub id: String,
    pub name: String,
    pub props: HashMap<String, f64>,
}

/// 世界状态: 实体集 + 虚拟 tick (由推演快进).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct WorldState {
    pub entities: Vec<Entity>,
    pub tick: u64,
}

impl WorldState {
    pub fn entity(&self, id: &str) -> Option<&Entity> {
        self.entities.iter().find(|e| e.id == id)
    }
    pub fn entity_mut(&mut self, id: &str) -> Option<&mut Entity> {
        self.entities.iter_mut().find(|e| e.id == id)
    }
    pub fn prop(&self, id: &str, key: &str) -> Option<f64> {
        self.entity(id).and_then(|e| e.props.get(key)).copied()
    }
}

/// 不确定性裁决口子: 真实现 [`CalibratedResolver`] (校准数学, 0 LLM 依赖);
/// 需要语义推理时可在外部注入 LLM 实现 (trait 保持开放).
#[async_trait]
pub trait UncertaintyResolver: Send + Sync {
    async fn resolve(&self, state: &WorldState, question: &str) -> Result<f64, String>;
}

/// 情景引擎: 事件注入 → 规则层 apply → 状态演进.
pub struct ScenarioEngine {
    pub state: WorldState,
}

/// 规则层 apply (确定性, 调用方注入; 例: "rain+0.3" 改属性).
pub type ApplyFn = Box<dyn Fn(&mut WorldState, &str) -> Result<(), String> + Send + Sync>;

impl ScenarioEngine {
    pub fn new(state: WorldState) -> Self {
        Self { state }
    }

    /// 注入事件 (规则层 apply; 失败返回错误, 状态不变).
    pub fn inject(&mut self, event: &str, apply: &ApplyFn) -> Result<(), String> {
        apply(&mut self.state, event)?;
        self.state.tick += 1;
        Ok(())
    }

    /// 推演: 依次注入事件序列, 返回每一步后的状态 (快照, 供分支评估).
    pub fn simulate(
        &mut self,
        events: &[String],
        apply: &ApplyFn,
    ) -> Result<Vec<WorldState>, String> {
        let mut snaps = Vec::with_capacity(events.len());
        for e in events {
            self.inject(e, apply)?;
            snaps.push(self.state.clone());
        }
        Ok(snaps)
    }
}

// ============================================================
// 预测断言 + 校准
// ============================================================

/// 预测断言: 「在 deadline 前, statement 发生概率 p」— 可证伪.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Forecast {
    pub id: String,
    pub statement: String,
    /// 预测概率 (0..1).
    pub probability: f64,
    /// 期限 (epoch ms).
    pub deadline_ms: i64,
    /// 结果 (None=未决).
    pub resolved: Option<bool>,
    /// Brier score: (p-1)² 若发生, p² 若未发生 (越低越准).
    pub brier: Option<f64>,
    pub created_at_ms: i64,
    /// 单调版本号 (register=0, resolve 写新版本 +1; 重放取最大).
    pub rev: u64,
}

impl Forecast {
    pub fn new(statement: impl Into<String>, probability: f64, deadline_ms: i64) -> Self {
        Self {
            id: format!("forecast-{}", uuid::Uuid::new_v4()),
            statement: statement.into(),
            probability: probability.clamp(0.0, 1.0),
            deadline_ms,
            resolved: None,
            brier: None,
            created_at_ms: chrono::Utc::now().timestamp_millis(),
            rev: 0,
        }
    }

    /// 对照真实结果: resolve + Brier score.
    pub fn resolve(&mut self, actual: bool) {
        let p = self.probability;
        self.resolved = Some(actual);
        self.brier = Some(if actual { (p - 1.0).powi(2) } else { p.powi(2) });
    }
}

/// 预测登记表 (真库, forecast- 前缀; 校准积累).
#[derive(Debug, Clone)]
pub struct ForecastRegistry {
    store: Arc<SqliteMemoryStore>,
    session_id: String,
}

const FORECAST_PREFIX: &str = "forecast-";

impl ForecastRegistry {
    pub fn new(store: Arc<SqliteMemoryStore>, session_id: impl Into<String>) -> Self {
        Self {
            store,
            session_id: session_id.into(),
        }
    }

    pub fn register(&self, f: &Forecast) -> Result<(), String> {
        let ep = CoreEpisode {
            id: f.id.clone(),
            timestamp: f.created_at_ms / 1000,
            role: "system".into(),
            content: serde_json::to_string(f).map_err(|e| e.to_string())?,
            session_id: self.session_id.clone(),
        };
        self.store.put_episode(&ep).map_err(|e| e.to_string())
    }

    fn load_all(&self) -> Result<Vec<Forecast>, String> {
        let eps = self
            .store
            .recent_episodes(&self.session_id, 500)
            .map_err(|e| e.to_string())?;
        // 按 forecast.id 分组, 取 rev 最大 (最新版本)
        let mut best: std::collections::HashMap<String, Forecast> =
            std::collections::HashMap::new();
        for e in eps.iter().filter(|e| e.id.starts_with(FORECAST_PREFIX)) {
            if let Ok(f) = serde_json::from_str::<Forecast>(&e.content) {
                match best.get(&f.id) {
                    Some(existing) if f.rev > existing.rev => {
                        best.insert(f.id.clone(), f);
                    }
                    Some(_) => {}
                    None => {
                        best.insert(f.id.clone(), f);
                    }
                }
            }
        }
        Ok(best.into_values().collect())
    }

    /// 对照结果 (更新 + 写回新版本).
    pub fn resolve(&self, id: &str, actual: bool) -> Result<f64, String> {
        let all = self.load_all()?;
        let idx = all
            .iter()
            .position(|f| f.id == id)
            .ok_or_else(|| format!("预测不存在: {id}"))?;
        let mut f = all[idx].clone();
        if f.resolved.is_some() {
            return Err("预测已 resolve".into());
        }
        f.resolve(actual);
        f.rev += 1; // 版本单调, 分组取最新
        f.created_at_ms = chrono::Utc::now().timestamp_millis();
        let brier = f.brier.unwrap();
        let ep = CoreEpisode {
            id: format!("{FORECAST_PREFIX}{}", uuid::Uuid::new_v4()),
            timestamp: chrono::Utc::now().timestamp(),
            role: "system".into(),
            content: serde_json::to_string(&f).map_err(|e| e.to_string())?,
            session_id: self.session_id.clone(),
        };
        self.store.put_episode(&ep).map_err(|e| e.to_string())?;
        Ok(brier)
    }

    /// 校准统计: (已解决数, 平均 Brier, 校准度提示).
    pub fn calibration(&self) -> Result<(usize, f64, String), String> {
        let all = self.load_all()?;
        let resolved: Vec<&Forecast> = all.iter().filter(|f| f.resolved.is_some()).collect();
        if resolved.is_empty() {
            return Ok((0, 0.0, "暂无已对照预测".into()));
        }
        let mean_brier: f64 =
            resolved.iter().filter_map(|f| f.brier).sum::<f64>() / resolved.len() as f64;
        // 校准度: 概率 70% 组的实际发生率 vs 70%
        let bands = [(0.6, 0.8), (0.4, 0.6), (0.8, 1.01), (0.0, 0.4)];
        let mut hint = String::new();
        for (lo, hi) in bands {
            let group: Vec<&Forecast> = resolved
                .iter()
                .filter(|f| f.probability >= lo && f.probability < hi)
                .copied()
                .collect();
            if !group.is_empty() {
                let actual_rate = group.iter().filter(|f| f.resolved == Some(true)).count() as f64
                    / group.len() as f64;
                hint.push_str(&format!(
                    "p∈[{:.0}%,{:.0}%) 实际 {:.0}%; ",
                    lo * 100.0,
                    (hi.min(1.01) * 100.0).min(100.0),
                    actual_rate * 100.0
                ));
            }
        }
        Ok((resolved.len(), mean_brier, hint))
    }
}

// ============================================================
// 不确定性裁决: 校准追踪真实现 (backlog P2#13, oracle-suite 就绪后接线)
// ============================================================

/// 校准裁决的量化状态: 概率点估计 + 95% 区间 + 证据强度 + 对照规模.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CalibrationStatus {
    /// 校准后的概率点估计 (0..1).
    pub probability: f64,
    /// Wilson 95% 区间 (下, 上).
    pub interval: (f64, f64),
    /// 证据强度 (按已对照观测数分档).
    pub strength: crate::confidence::Strength,
    /// 已对照预测数.
    pub resolved_count: usize,
    /// 平均 Brier score (越低越准).
    pub mean_brier: f64,
}

/// 校准裁决器: 用历史已对照预测的 BetaBinomial 追踪, 把不确定性量化为数学化概率.
///
/// 原理 (docs/oracle-suite-design.md §三/§四「校准 = Brier + BetaBinomial」):
/// - 每个已 resolve 的 [`Forecast`] 的真实结果是一条校准观测 (`success = actual`),
///   累积进 [`BetaBinomial`] (均匀先验 (1,1) 起步) → 后验均值 = 模型「预测成真率」.
/// - 0 历史 → 0.5 (均匀先验, 诚实「不知道」, 与 `BetaBinomial::default` 同语义).
/// - 区间 = Wilson 95% (观测越多越窄), strength 按观测数分档.
///
/// LLM 语义裁决仍留 [`UncertaintyResolver`] trait 口 (外部可注入真 LLM);
/// 本实现 0 LLM 依赖、纯确定性、可测试 (与项目「0 装 PASS」风格一致: 留口诚实标注).
#[derive(Debug, Clone)]
pub struct CalibratedResolver {
    registry: ForecastRegistry,
}

impl CalibratedResolver {
    pub fn new(registry: ForecastRegistry) -> Self {
        Self { registry }
    }

    /// 构建校准 BetaBinomial: 从已对照预测的真实结果累积观测.
    pub fn calibrated_beta(&self) -> Result<BetaBinomial, String> {
        let all = self.registry.load_all()?;
        let mut bb = BetaBinomial::default();
        for f in all.iter().filter(|f| f.resolved.is_some()) {
            bb.observe(f.resolved == Some(true));
        }
        Ok(bb)
    }

    /// 校准状态: 概率 + 区间 + 强度 + 已对照数 + 平均 Brier.
    pub fn status(&self) -> Result<CalibrationStatus, String> {
        let bb = self.calibrated_beta()?;
        let (n, mean_brier, _hint) = self.registry.calibration()?;
        Ok(CalibrationStatus {
            probability: bb.mean(),
            interval: bb.interval95(),
            strength: bb.strength(),
            resolved_count: n,
            mean_brier,
        })
    }
}

#[async_trait]
impl UncertaintyResolver for CalibratedResolver {
    async fn resolve(&self, _state: &WorldState, _question: &str) -> Result<f64, String> {
        // 校准概率 = 后验均值 (0 历史 → 0.5 均匀先验, 诚实「不知道」).
        Ok(self.calibrated_beta()?.mean())
    }
}

// ============================================================
// 分支推演 + 决策 (expectimax-lite)
// ============================================================

/// 一个推演分支: 事件序列 + 概率 + 价值.
#[derive(Debug, Clone)]
pub struct Branch {
    pub name: String,
    pub probability: f64,
    pub value: f64,
    pub events: Vec<String>,
}

/// 决策引擎: 期望值选优 (v1 一层决策树; 多轮 MCTS 下一步).
pub struct DecisionEngine;

impl DecisionEngine {
    /// 分支期望值 = Σ P(branch)×V(branch).
    pub fn expected_value(branches: &[Branch]) -> f64 {
        branches.iter().map(|b| b.probability * b.value).sum()
    }

    /// 选优: 返回期望值最大的分支下标 (空 → None).
    pub fn choose(branches: &[Branch]) -> Option<usize> {
        let ev = |b: &Branch| b.probability * b.value;
        (0..branches.len()).max_by(|&a, &b| {
            ev(&branches[a])
                .partial_cmp(&ev(&branches[b]))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> WorldState {
        WorldState {
            entities: vec![Entity {
                id: "master".into(),
                name: "主人".into(),
                props: HashMap::from([("进度".into(), 0.3f64), ("焦虑".into(), 0.6f64)]),
            }],
            tick: 0,
        }
    }

    #[test]
    fn simulate_applies_events_and_advances() {
        let apply: ApplyFn = Box::new(|s: &mut WorldState, e: &str| {
            if let Some((id, val)) = e.split_once('+') {
                if let Some(en) = s.entity_mut(id) {
                    *en.props.entry("进度".into()).or_insert(0.0) +=
                        val.parse::<f64>().map_err(|_| "bad".to_string())?;
                }
            }
            Ok(())
        });
        let mut eng = ScenarioEngine::new(world());
        let snaps = eng
            .simulate(&["master+0.2".into(), "master+0.1".into()], &apply)
            .unwrap();
        assert_eq!(snaps.len(), 2);
        assert_eq!(eng.state.tick, 2);
        assert!(
            (eng.state.prop("master", "进度").unwrap() - 0.6).abs() < 1e-9,
            "0.3+0.2+0.1"
        );
    }

    #[test]
    fn forecast_resolve_brier() {
        let mut f = Forecast::new("明天交作业", 0.7, 1_800_000_000_000);
        assert!(f.resolved.is_none());
        f.resolve(true);
        assert_eq!(f.resolved, Some(true));
        assert!(
            (f.brier.unwrap() - 0.09).abs() < 1e-9,
            "Brier=(0.7-1)²=0.09"
        );
        let mut f2 = Forecast::new("x", 0.7, 0);
        f2.resolve(false);
        assert!((f2.brier.unwrap() - 0.49).abs() < 1e-9, "Brier=0.7²=0.49");
    }

    #[test]
    fn forecast_registry_resolve_and_calibration() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let reg = ForecastRegistry::new(store, "me");
        let f1 = Forecast::new("线代作业明天交", 0.7, 1_800_000_000_000);
        reg.register(&f1).unwrap();
        let f2 = Forecast::new("高数错题本写完", 0.9, 1_800_000_000_000);
        reg.register(&f2).unwrap();
        reg.resolve(&f1.id, true).unwrap();
        reg.resolve(&f2.id, true).unwrap();
        let (n, mean_brier, hint) = reg.calibration().unwrap();
        assert_eq!(n, 2);
        assert!(
            mean_brier < 0.2,
            "两个高概率都发生 → Brier 低: {mean_brier}"
        );
        assert!(hint.contains("实际"), "校准提示含实际发生率: {hint}");
        // 重复 resolve 拒绝
        assert!(reg.resolve(&f1.id, false).is_err());
    }

    // ---- UncertaintyResolver 接真 (P2#13): CalibratedResolver ----

    fn registry_with(store: Arc<SqliteMemoryStore>) -> ForecastRegistry {
        ForecastRegistry::new(store, "me")
    }

    fn seed_resolved(reg: &ForecastRegistry, probabilities: &[f64], outcomes: &[bool]) {
        for (i, (p, o)) in probabilities.iter().zip(outcomes.iter()).enumerate() {
            let f = Forecast::new(format!("断言 {i}"), *p, 1_800_000_000_000);
            reg.register(&f).unwrap();
            reg.resolve(&f.id, *o).unwrap();
        }
    }

    #[test]
    fn calibrated_resolver_no_history_is_uninformative() {
        let reg = registry_with(Arc::new(SqliteMemoryStore::open_in_memory().unwrap()));
        let r = CalibratedResolver::new(reg);
        let st = r.status().unwrap();
        assert!(
            (st.probability - 0.5).abs() < 1e-9,
            "无历史 → 均匀先验 0.5: {}",
            st.probability
        );
        assert_eq!(st.resolved_count, 0);
        assert_eq!(st.interval, (0.0, 1.0));
        assert_eq!(st.strength, crate::confidence::Strength::Weak);
    }

    #[test]
    fn calibrated_resolver_tracks_forecast_outcomes() {
        let reg = registry_with(Arc::new(SqliteMemoryStore::open_in_memory().unwrap()));
        // 3 条高概率预测全部成真 → 后验均值 = (1+3)/(2+3) = 0.8
        seed_resolved(&reg, &[0.8, 0.8, 0.8], &[true, true, true]);
        let r = CalibratedResolver::new(reg);
        let st = r.status().unwrap();
        assert!(
            (st.probability - 0.8).abs() < 1e-9,
            "3/3 成真 → 0.8: {}",
            st.probability
        );
        assert_eq!(st.resolved_count, 3);
        assert_eq!(st.strength, crate::confidence::Strength::Weak);
        assert!(
            (st.mean_brier - 0.04).abs() < 1e-9,
            "Brier=(0.8-1)²=0.04: {}",
            st.mean_brier
        );
        assert!(
            st.interval.0 < st.probability && st.probability < st.interval.1,
            "区间应包住点估计: {:?} vs {}",
            st.interval,
            st.probability
        );
    }

    #[test]
    fn calibrated_resolver_mixed_outcomes_pull_toward_rate() {
        let reg = registry_with(Arc::new(SqliteMemoryStore::open_in_memory().unwrap()));
        // 2 真 1 假 → (1+2)/(2+3) = 0.6
        seed_resolved(&reg, &[0.6, 0.6, 0.6], &[true, true, false]);
        let r = CalibratedResolver::new(reg);
        let p = r.calibrated_beta().unwrap().mean();
        assert!((p - 0.6).abs() < 1e-9, "2/3 成真 → 0.6: {p}");
    }

    #[tokio::test]
    async fn calibrated_resolver_implements_uncertainty_resolver_trait() {
        let reg = registry_with(Arc::new(SqliteMemoryStore::open_in_memory().unwrap()));
        let r = CalibratedResolver::new(reg);
        // 0 历史 → trait 口返回 0.5
        let p = r
            .resolve(&WorldState::default(), "今晚能写完作业吗")
            .await
            .unwrap();
        assert!((p - 0.5).abs() < 1e-9, "无历史 → 0.5: {p}");
        // 有历史 → trait 口返回校准后验均值
        let reg2 = registry_with(Arc::new(SqliteMemoryStore::open_in_memory().unwrap()));
        seed_resolved(&reg2, &[0.9, 0.9, 0.9, 0.9], &[true, true, true, true]);
        let r2 = CalibratedResolver::new(reg2);
        let p2 = r2
            .resolve(&WorldState::default(), "明天会下雨吗")
            .await
            .unwrap();
        assert!((p2 - 5.0 / 6.0).abs() < 1e-9, "4/4 成真 → 5/6: {p2}");
    }

    #[test]
    fn decision_engine_expectimax() {
        let branches = vec![
            Branch {
                name: "保守".into(),
                probability: 0.8,
                value: 5.0,
                events: vec![],
            },
            Branch {
                name: "激进".into(),
                probability: 0.2,
                value: 30.0,
                events: vec![],
            },
        ];
        // 保守 4.0 vs 激进 6.0 → 激进期望更高
        assert!((DecisionEngine::expected_value(&branches) - 10.0).abs() < 1e-9);
        assert_eq!(DecisionEngine::choose(&branches), Some(1));
        // 低概率高价值 vs 高概率中价值: 0.9*8=7.2 vs 0.1*100=10 → 期望值选「赌」(数学正确)
        let b2 = vec![
            Branch {
                name: "稳".into(),
                probability: 0.9,
                value: 8.0,
                events: vec![],
            },
            Branch {
                name: "赌".into(),
                probability: 0.1,
                value: 100.0,
                events: vec![],
            },
        ];
        assert_eq!(
            DecisionEngine::choose(&b2),
            Some(1),
            "期望值 10 > 7.2, 应选赌"
        );
    }
}
