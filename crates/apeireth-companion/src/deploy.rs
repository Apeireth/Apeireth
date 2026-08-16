//! `apeireth-companion::deploy` — 能力演化回路: 部署 → 监控 → 回滚 (激活后的后半段, A1).
//!
//! 哲学对接: 「我希望它能自己演化」。完整回路 = 提案 → 生成 → 验证 → 部署 → 监控 → 回滚。
//! capability.rs 落「提案→评审→激活」, evolution_gate.rs 落「验证闸门」(其 LoopAction
//! 指向本模块), 本模块落最后三段:
//! - **部署**: 激活的能力经部署通道上线 (trait 抽象; 测试走 MockDeployChannel)
//! - **监控**: 部署后运行观测登记 (调用计数/失败率/差评信号 + 预测线期限)
//! - **回滚**: 差评或失败率越限 → 状态机回退 (active → rolled_back, 留痕 + 收据)
//!
//! 0 假装 PASS: ① 部署通道只有 mock 实现, 真执行体 (exec_worker/sandbox 隔离执行)
//! 是接线点 = 实现 DeployChannel trait, 本模块不内置真部署执行; ② 制品 = 文本描述,
//! 真制品形态 (代码/工具注册项) 待真执行体接入; ③「生成」段 (LLM 生成能力内容)
//! 未机制化, 不在本模块范围。

use std::sync::Arc;

use apeireth_core::clock::{Clock, SystemClock};
use apeireth_memory::{CoreEpisode, SqliteMemoryStore};
use serde::{Deserialize, Serialize};

use crate::capability::{CapabilityError, CapabilityRegistry, CapabilityStatus};

/// 部署通道抽象 (mock 可测; 真执行体挂 exec_worker/sandbox 隔离口 = 实现本 trait).
pub trait DeployChannel: Send + Sync {
    /// 执行部署: 把能力制品 (artifact) 上线。Err = 部署失败 (能力保持 active, 可重试).
    fn deploy(&self, capability_name: &str, artifact: &str) -> Result<(), String>;
    /// 通道名 (留痕用).
    fn name(&self) -> &str;
}

/// Mock 部署通道 (测试/模拟: 可配失败, 计调用次数).
#[derive(Default)]
pub struct MockDeployChannel {
    fail: std::sync::atomic::AtomicBool,
    calls: std::sync::atomic::AtomicU64,
}

impl MockDeployChannel {
    pub fn ok() -> Self {
        Self::default()
    }
    pub fn failing() -> Self {
        let s = Self::default();
        s.set_fail(true);
        s
    }
    pub fn set_fail(&self, v: bool) {
        self.fail.store(v, std::sync::atomic::Ordering::SeqCst);
    }
    pub fn call_count(&self) -> u64 {
        self.calls.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl DeployChannel for MockDeployChannel {
    fn deploy(&self, _capability_name: &str, _artifact: &str) -> Result<(), String> {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if self.fail.load(std::sync::atomic::Ordering::SeqCst) {
            Err("mock 通道故障".into())
        } else {
            Ok(())
        }
    }
    fn name(&self) -> &str {
        "mock"
    }
}

/// 部署状态.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeployStatus {
    Live,       // 部署成功, 监控期
    Failed,     // 通道执行失败 (能力保持 active, 可重试)
    RolledBack, // 已回滚 (能力状态机 rolled_back, 留痕)
}

/// 运行观测指标 (监控: 调用计数 / 失败率 / 差评信号).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonitorMetrics {
    /// 调用计数 (观测总次数; 差评也计一次调用+失败).
    pub calls: u64,
    pub failures: u64,
    /// 差评信号 (用户负反馈显式信号, 与失败分开计数).
    pub negative_signals: u64,
}

impl MonitorMetrics {
    pub fn failure_rate(&self) -> f64 {
        if self.calls == 0 {
            0.0
        } else {
            self.failures as f64 / self.calls as f64
        }
    }
}

/// 部署记录 (真库登记项, append-only 版本化, 对齐 capability.rs 写法).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deployment {
    pub id: String,
    /// 单调版本号 (deploy=0, 每次观测/回滚 +1; 重放取最大).
    pub rev: u64,
    pub capability_id: String,
    pub status: DeployStatus,
    /// 通道名 (mock / 将来的 exec_worker...).
    pub channel: String,
    /// 能力制品描述 (0 装 PASS: 文本; 真制品形态是真执行体接线点).
    pub artifact: String,
    pub deployed_at_ms: i64,
    /// 监控登记 (调用计数/失败率/差评信号).
    pub metrics: MonitorMetrics,
    pub rollback_reason: Option<String>,
}

/// 部署错误.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeployError {
    NotFound,
    /// 能力不在可部署状态 (仅 active 可部署).
    IllegalState { status: CapabilityStatus },
    /// 部署通道执行失败 (已留痕 Failed 部署, 能力保持 active 可重试).
    ChannelFailed(String),
    /// 部署非 live (未部署/已失败/已回滚), 不接受观测.
    NotLive,
    Store(String),
}

/// 一次观测的结果.
#[derive(Debug, Clone)]
pub struct ObserveOutcome {
    pub metrics: MonitorMetrics,
    /// 本次观测是否触发了回滚.
    pub rolled_back: bool,
    /// 回滚收据 (rolled_back=true 时; revert 即学习信号).
    pub receipt: Option<String>,
}

const DEP_PREFIX: &str = "dep-";

/// 部署管理器: 部署执行 + 监控观测登记 + 回滚触发 (机制件).
///
/// 接线: 持 CapabilityRegistry (状态机) + SqliteMemoryStore (留痕) +
/// DeployChannel (执行体) + Clock (时间源, 测试注入 VirtualClock).
pub struct DeployManager {
    registry: Arc<CapabilityRegistry>,
    store: Arc<SqliteMemoryStore>,
    session_id: String,
    channel: Arc<dyn DeployChannel>,
    clock: Arc<dyn Clock>,
    /// 失败率回滚上限 (calls >= min_observations 且 failure_rate >= 此值 → 回滚).
    pub max_failure_rate: f64,
    /// 失败率判定前的最小观测数 (防小样本误杀).
    pub min_observations: u64,
    /// 差评信号回滚上限 (negative_signals >= 此值 → 回滚, 不看样本量).
    pub max_negative_signals: u64,
}

impl DeployManager {
    pub fn new(
        registry: Arc<CapabilityRegistry>,
        store: Arc<SqliteMemoryStore>,
        session_id: impl Into<String>,
        channel: Arc<dyn DeployChannel>,
    ) -> Self {
        Self {
            registry,
            store,
            session_id: session_id.into(),
            channel,
            clock: Arc::new(SystemClock),
            max_failure_rate: 0.5,
            min_observations: 4,
            max_negative_signals: 2,
        }
    }

    /// 注入时钟 (测试/模拟: VirtualClock 快进期限检查, 0 真等待).
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    fn now_ms(&self) -> i64 {
        self.clock.now().timestamp_millis()
    }

    fn put(&self, d: &Deployment) -> Result<(), DeployError> {
        // append-only: 每次变更 = 新版本事件 (对齐 capability.rs), 不做覆盖
        let ep = CoreEpisode {
            id: format!("{}{}", DEP_PREFIX, uuid::Uuid::new_v4()),
            timestamp: d.deployed_at_ms / 1000,
            role: "system".into(),
            content: serde_json::to_string(d).map_err(|e| DeployError::Store(e.to_string()))?,
            session_id: self.session_id.clone(),
        };
        self.store.put_episode(&ep).map_err(|e| DeployError::Store(e.to_string()))
    }

    /// 重放全部版本, 每条部署取 rev 最大 (最新).
    fn load_latest_all(&self) -> Result<Vec<Deployment>, DeployError> {
        let eps = self.store.recent_episodes(&self.session_id, 500).map_err(|e| DeployError::Store(e.to_string()))?;
        let mut best: std::collections::HashMap<String, Deployment> = std::collections::HashMap::new();
        for e in eps.iter().filter(|e| e.id.starts_with(DEP_PREFIX)) {
            if let Ok(d) = serde_json::from_str::<Deployment>(&e.content) {
                match best.get(&d.id) {
                    Some(existing) if d.rev > existing.rev => {
                        best.insert(d.id.clone(), d);
                    }
                    Some(_) => {}
                    None => {
                        best.insert(d.id.clone(), d);
                    }
                }
            }
        }
        Ok(best.into_values().collect())
    }

    fn capability(&self, id: &str) -> Result<crate::capability::CapabilityProposal, DeployError> {
        self.registry
            .list(None)
            .map_err(|e| DeployError::Store(e))?
            .into_iter()
            .find(|p| p.id == id)
            .ok_or(DeployError::NotFound)
    }

    /// 能力当前最新部署记录 (任意状态).
    pub fn deployment_of(&self, capability_id: &str) -> Result<Option<Deployment>, DeployError> {
        Ok(self
            .load_latest_all()?
            .into_iter()
            .filter(|d| d.capability_id == capability_id)
            .max_by_key(|d| d.deployed_at_ms))
    }

    /// 部署: 仅 active 能力可部署; 通道失败 → Failed 留痕, 能力保持 active 可重试.
    pub fn deploy(&self, capability_id: &str, artifact: &str) -> Result<Deployment, DeployError> {
        let cap = self.capability(capability_id)?;
        if cap.status != CapabilityStatus::Active {
            return Err(DeployError::IllegalState { status: cap.status });
        }
        let mut d = Deployment {
            id: format!("{}{}", DEP_PREFIX, uuid::Uuid::new_v4()),
            rev: 0,
            capability_id: capability_id.to_string(),
            status: DeployStatus::Live,
            channel: self.channel.name().to_string(),
            artifact: artifact.to_string(),
            deployed_at_ms: self.now_ms(),
            metrics: MonitorMetrics::default(),
            rollback_reason: None,
        };
        match self.channel.deploy(&cap.name, artifact) {
            Ok(()) => {
                self.put(&d)?;
                Ok(d)
            }
            Err(e) => {
                d.status = DeployStatus::Failed;
                self.put(&d)?; // 失败也留痕
                Err(DeployError::ChannelFailed(e))
            }
        }
    }

    /// 监控观测: 记录一次运行结果 (成功/失败), 越限自动回滚.
    pub fn observe(&self, capability_id: &str, success: bool) -> Result<ObserveOutcome, DeployError> {
        self.observe_inner(capability_id, success, false)
    }

    /// 差评信号: 用户负反馈 (计一次调用+失败+差评), 越限自动回滚.
    pub fn observe_negative(&self, capability_id: &str) -> Result<ObserveOutcome, DeployError> {
        self.observe_inner(capability_id, false, true)
    }

    fn observe_inner(&self, capability_id: &str, success: bool, negative: bool) -> Result<ObserveOutcome, DeployError> {
        let cap = self.capability(capability_id)?;
        if cap.status != CapabilityStatus::Active {
            return Err(DeployError::NotLive);
        }
        let mut dep = self.deployment_of(capability_id)?.ok_or(DeployError::NotLive)?;
        if dep.status != DeployStatus::Live {
            return Err(DeployError::NotLive);
        }
        dep.metrics.calls += 1;
        if !success {
            dep.metrics.failures += 1;
        }
        if negative {
            dep.metrics.negative_signals += 1;
        }
        // capability 自身 EMA/置信度台账同步 (监控双登记, 复用已有机制).
        let _ = self.registry.record_use(capability_id, success);
        if let Some(reason) = self.rollback_trigger(&dep.metrics) {
            let receipt = self.rollback(capability_id, &reason)?;
            return Ok(ObserveOutcome { metrics: dep.metrics, rolled_back: true, receipt: Some(receipt) });
        }
        dep.rev += 1;
        self.put(&dep)?;
        Ok(ObserveOutcome { metrics: dep.metrics, rolled_back: false, receipt: None })
    }

    fn rollback_trigger(&self, m: &MonitorMetrics) -> Option<String> {
        if m.negative_signals >= self.max_negative_signals {
            return Some(format!("差评信号 {} ≥ {} (用户负反馈触发)", m.negative_signals, self.max_negative_signals));
        }
        if m.calls >= self.min_observations && m.failure_rate() >= self.max_failure_rate {
            return Some(format!(
                "失败率 {:.0}% ≥ {:.0}% (obs={}, 越限触发)",
                m.failure_rate() * 100.0,
                self.max_failure_rate * 100.0,
                m.calls
            ));
        }
        None
    }

    /// 期限检查 (对齐预测行 ExpectedOutcome): deadline 已过且零观测 (信号未启动) → 回滚.
    /// 时间敏感测试用 VirtualClock 快进, 0 真等待.
    pub fn check_deadline(&self, capability_id: &str) -> Result<Option<String>, DeployError> {
        let cap = self.capability(capability_id)?;
        let Some(expected) = &cap.expected else { return Ok(None) };
        if self.now_ms() < expected.deadline_ms || cap.status != CapabilityStatus::Active {
            return Ok(None);
        }
        let Some(dep) = self.deployment_of(capability_id)?.filter(|d| d.status == DeployStatus::Live) else {
            return Ok(None);
        };
        if dep.metrics.calls > 0 {
            return Ok(None); // 信号已有观测, 由观测指标判定, 不在期限通道重复判
        }
        let reason = format!("预期未达标 (信号: {}; 期限已过): {}", expected.signal, expected.rollback);
        let receipt = self.rollback(capability_id, &reason)?;
        Ok(Some(receipt))
    }

    /// 回滚: active → rolled_back (能力状态机) + 部署留痕 + 收据 (revert 即学习信号).
    pub fn rollback(&self, capability_id: &str, reason: &str) -> Result<String, DeployError> {
        let cap = self.registry.rollback(capability_id, reason).map_err(|e| match e {
            CapabilityError::NotFound => DeployError::NotFound,
            CapabilityError::IllegalTransition { from, .. } => DeployError::IllegalState { status: from },
        })?;
        let metrics = if let Some(mut dep) = self.deployment_of(capability_id)? {
            let m = dep.metrics;
            if dep.status == DeployStatus::Live {
                dep.status = DeployStatus::RolledBack;
                dep.rollback_reason = Some(reason.to_string());
                dep.rev += 1;
                self.put(&dep)?;
            }
            m
        } else {
            MonitorMetrics::default()
        };
        Ok(format!(
            "[deploy-revert] 能力 {} ({}) 原因: {} | 指标: calls={} failures={} negatives={} — 回滚即学习信号, 下轮提案须更小",
            cap.name, capability_id, reason, metrics.calls, metrics.failures, metrics.negative_signals
        ))
    }

    /// 全部最新部署记录 (可按能力过滤).
    pub fn deployments(&self, capability_id: Option<&str>) -> Result<Vec<Deployment>, DeployError> {
        Ok(self
            .load_latest_all()?
            .into_iter()
            .filter(|d| capability_id.map_or(true, |c| d.capability_id == c))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::clock::VirtualClock;
    use crate::capability::CapabilityKind;
    use crate::evolution_gate::{EvalGate, GateDecision, LoopAction, VerifyOutcome};

    fn fixture(channel: Arc<dyn DeployChannel>) -> (Arc<CapabilityRegistry>, Arc<SqliteMemoryStore>, DeployManager) {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let reg = Arc::new(CapabilityRegistry::new(Arc::clone(&store), "me"));
        let mgr = DeployManager::new(Arc::clone(&reg), Arc::clone(&store), "me", channel);
        (reg, store, mgr)
    }

    fn activated(reg: &CapabilityRegistry, name: &str) -> String {
        let p = reg.propose(name, "测试能力", CapabilityKind::Skill, "apeireth").unwrap();
        reg.approve(&p.id).unwrap();
        reg.activate(&p.id).unwrap();
        p.id
    }

    #[test]
    fn deploy_active_capability_live() {
        let (reg, _, mgr) = fixture(Arc::new(MockDeployChannel::ok()));
        let id = activated(&reg, "换元检查");
        let d = mgr.deploy(&id, "制品描述").unwrap();
        assert_eq!(d.status, DeployStatus::Live);
        assert_eq!(d.channel, "mock");
        assert!(d.metrics.calls == 0);
        assert_eq!(mgr.deployments(Some(&id)).unwrap().len(), 1);
    }

    #[test]
    fn deploy_requires_active_state() {
        let (reg, _, mgr) = fixture(Arc::new(MockDeployChannel::ok()));
        // pending 不能部署
        let p = reg.propose("未批准", "x", CapabilityKind::Skill, "apeireth").unwrap();
        assert!(matches!(mgr.deploy(&p.id, "a"), Err(DeployError::IllegalState { status: CapabilityStatus::Pending })));
        // approved 不能部署 (须先激活)
        reg.approve(&p.id).unwrap();
        assert!(matches!(mgr.deploy(&p.id, "a"), Err(DeployError::IllegalState { status: CapabilityStatus::Approved })));
        // 不存在
        assert!(matches!(mgr.deploy("cap-404", "a"), Err(DeployError::NotFound)));
    }

    #[test]
    fn channel_failure_leaves_failed_record_and_retries() {
        let ch = Arc::new(MockDeployChannel::failing());
        let (reg, _, mgr) = fixture(ch.clone());
        let id = activated(&reg, "故障通道");
        assert!(matches!(mgr.deploy(&id, "a"), Err(DeployError::ChannelFailed(_))));
        assert_eq!(ch.call_count(), 1);
        // 失败留痕 (Failed 部署)
        assert_eq!(mgr.deployment_of(&id).unwrap().unwrap().status, DeployStatus::Failed);
        // 能力保持 active, 通道恢复后可重试
        assert_eq!(reg.list(None).unwrap().into_iter().find(|c| c.id == id).unwrap().status, CapabilityStatus::Active);
        ch.set_fail(false);
        let d = mgr.deploy(&id, "a").unwrap();
        assert_eq!(d.status, DeployStatus::Live);
        assert_eq!(ch.call_count(), 2);
    }

    #[test]
    fn observe_accumulates_metrics() {
        let (reg, _, mgr) = fixture(Arc::new(MockDeployChannel::ok()));
        let id = activated(&reg, "观测累积");
        mgr.deploy(&id, "a").unwrap();
        let o1 = mgr.observe(&id, true).unwrap();
        assert!(!o1.rolled_back);
        assert_eq!(o1.metrics.calls, 1);
        let o2 = mgr.observe(&id, false).unwrap();
        assert_eq!(o2.metrics.calls, 2);
        assert_eq!(o2.metrics.failures, 1);
        assert!((o2.metrics.failure_rate() - 0.5).abs() < 1e-9);
        // 能力自身 EMA/置信度台账同步
        let cap = reg.list(None).unwrap().into_iter().find(|c| c.id == id).unwrap();
        assert_eq!(cap.confidence.unwrap().observations, 2);
    }

    #[test]
    fn negative_signals_trigger_rollback() {
        let (reg, _, mgr) = fixture(Arc::new(MockDeployChannel::ok()));
        let id = activated(&reg, "差评回滚");
        mgr.deploy(&id, "a").unwrap();
        let o1 = mgr.observe_negative(&id).unwrap();
        assert!(!o1.rolled_back, "单次差评不触发 (上限 2)");
        let o2 = mgr.observe_negative(&id).unwrap();
        assert!(o2.rolled_back);
        assert!(o2.receipt.unwrap().contains("[deploy-revert]"));
        // 状态机回退留痕: active → rolled_back
        let cap = reg.list(None).unwrap().into_iter().find(|c| c.id == id).unwrap();
        assert_eq!(cap.status, CapabilityStatus::RolledBack);
        assert!(cap.rollback_reason.unwrap().contains("差评信号"));
        // 部署记录回滚留痕
        assert_eq!(mgr.deployment_of(&id).unwrap().unwrap().status, DeployStatus::RolledBack);
        // 回滚后不接受观测
        assert!(matches!(mgr.observe(&id, true), Err(DeployError::NotLive)));
    }

    #[test]
    fn failure_rate_trigger_rollback() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let reg = Arc::new(CapabilityRegistry::new(Arc::clone(&store), "me"));
        let mut mgr = DeployManager::new(Arc::clone(&reg), Arc::clone(&store), "me", Arc::new(MockDeployChannel::ok()));
        mgr.min_observations = 3;
        mgr.max_failure_rate = 0.6;
        let id = activated(&reg, "失败率回滚");
        mgr.deploy(&id, "a").unwrap();
        // 前 2 次失败: 样本不足 (min_observations=3), 不回滚
        assert!(!mgr.observe(&id, false).unwrap().rolled_back);
        assert!(!mgr.observe(&id, false).unwrap().rolled_back);
        // 第 3 次失败: 失败率 100% ≥ 60%, 触发
        let o3 = mgr.observe(&id, false).unwrap();
        assert!(o3.rolled_back);
        assert!(o3.receipt.unwrap().contains("失败率"));
        let cap = reg.list(None).unwrap().into_iter().find(|c| c.id == id).unwrap();
        assert_eq!(cap.status, CapabilityStatus::RolledBack);
    }

    #[test]
    fn observe_requires_live_deployment() {
        let (reg, _, mgr) = fixture(Arc::new(MockDeployChannel::ok()));
        let id = activated(&reg, "未部署");
        // 未部署不能观测
        assert!(matches!(mgr.observe(&id, true), Err(DeployError::NotLive)));
        // 已退役不能观测
        let id2 = activated(&reg, "退役");
        mgr.deploy(&id2, "a").unwrap();
        reg.retire(&id2).unwrap();
        assert!(matches!(mgr.observe(&id2, true), Err(DeployError::NotLive)));
    }

    #[test]
    fn deadline_triggers_rollback_with_virtual_clock() {
        let vc = Arc::new(VirtualClock::new(chrono::Utc::now()));
        let deadline = vc.current().timestamp_millis() + 3_600_000; // 1h 后
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let reg = Arc::new(CapabilityRegistry::new(Arc::clone(&store), "me"));
        let mgr = DeployManager::new(Arc::clone(&reg), Arc::clone(&store), "me", Arc::new(MockDeployChannel::ok()))
            .with_clock(vc.clone());
        let expected = crate::capability::ExpectedOutcome {
            signal: "被采用 ≥3 次".into(),
            deadline_ms: deadline,
            rollback: "retire".into(),
        };
        let p = reg.propose_with_expected("期限能力", "y", CapabilityKind::Skill, "apeireth", expected).unwrap();
        reg.approve(&p.id).unwrap();
        reg.activate(&p.id).unwrap();
        mgr.deploy(&p.id, "a").unwrap();
        // 期限未到: 不回滚
        assert!(mgr.check_deadline(&p.id).unwrap().is_none());
        // 快进 2h (虚拟时间, 0 真等待): 零观测 → 预期未达标 → 回滚
        vc.advance(chrono::Duration::hours(2));
        let receipt = mgr.check_deadline(&p.id).unwrap();
        assert!(receipt.unwrap().contains("[deploy-revert]"));
        let cap = reg.list(None).unwrap().into_iter().find(|c| c.id == p.id).unwrap();
        assert_eq!(cap.status, CapabilityStatus::RolledBack);
        assert!(cap.rollback_reason.unwrap().contains("预期未达标"));
    }

    #[test]
    fn deadline_skipped_when_observations_exist() {
        let vc = Arc::new(VirtualClock::new(chrono::Utc::now()));
        let deadline = vc.current().timestamp_millis() + 1000;
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let reg = Arc::new(CapabilityRegistry::new(Arc::clone(&store), "me"));
        let mgr = DeployManager::new(Arc::clone(&reg), Arc::clone(&store), "me", Arc::new(MockDeployChannel::ok()))
            .with_clock(vc.clone());
        let expected = crate::capability::ExpectedOutcome { signal: "s".into(), deadline_ms: deadline, rollback: "r".into() };
        let p = reg.propose_with_expected("有观测", "y", CapabilityKind::Skill, "apeireth", expected).unwrap();
        reg.approve(&p.id).unwrap();
        reg.activate(&p.id).unwrap();
        mgr.deploy(&p.id, "a").unwrap();
        mgr.observe(&p.id, true).unwrap(); // 信号已有观测 → 期限通道不重复判
        vc.advance(chrono::Duration::hours(1));
        assert!(mgr.check_deadline(&p.id).unwrap().is_none());
    }

    #[test]
    fn deployment_persistence_survives_reopen() {
        let (reg, store, mgr) = fixture(Arc::new(MockDeployChannel::ok()));
        let id = activated(&reg, "持久部署");
        mgr.deploy(&id, "a").unwrap();
        mgr.observe(&id, true).unwrap();
        // 重开 manager (同库): 重放取最新 rev
        let mgr2 = DeployManager::new(Arc::clone(&reg), store, "me", Arc::new(MockDeployChannel::ok()));
        let d = mgr2.deployment_of(&id).unwrap().unwrap();
        assert_eq!(d.status, DeployStatus::Live);
        assert_eq!(d.metrics.calls, 1);
    }

    /// 状态机全链证据: 提案 → (生成*) → 验证闸门 → 部署 → 监控 → 回滚.
    /// (*生成 = LLM 段未机制化, 以提案代替; 见模块头 0 假装标注)
    #[test]
    fn full_evolution_loop_propose_verify_deploy_monitor_rollback() {
        // 1. 提案
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let reg = Arc::new(CapabilityRegistry::new(Arc::clone(&store), "me"));
        let p = reg.propose("全链能力", "演化回路全链测试", CapabilityKind::Skill, "apeireth").unwrap();
        // 2. 验证闸门 (evolution_gate): 首轮通过 → Promoted → LoopAction::Deploy
        let gate = EvalGate::new("pre-sha");
        let (decision, _, _) = gate.run_until_conclusion(
            |_| VerifyOutcome { passed: true, changed: true, error: None },
            std::time::Instant::now(),
        );
        assert_eq!(decision, GateDecision::Promoted);
        assert_eq!(gate.loop_action(&decision), LoopAction::Deploy);
        let r = gate.deploy_receipt(&p.name);
        assert!(r.contains("[agent-deploy]"));
        // 3. 批准 + 激活 + 部署
        reg.approve(&p.id).unwrap();
        reg.activate(&p.id).unwrap();
        let mgr = DeployManager::new(Arc::clone(&reg), Arc::clone(&store), "me", Arc::new(MockDeployChannel::ok()));
        let d = mgr.deploy(&p.id, "制品").unwrap();
        assert_eq!(d.status, DeployStatus::Live);
        // 4. 监控: 两次差评 → 5. 自动回滚
        assert!(!mgr.observe_negative(&p.id).unwrap().rolled_back);
        let o = mgr.observe_negative(&p.id).unwrap();
        assert!(o.rolled_back && o.receipt.unwrap().contains("[deploy-revert]"));
        let cap = reg.list(None).unwrap().into_iter().find(|c| c.id == p.id).unwrap();
        assert_eq!(cap.status, CapabilityStatus::RolledBack);
        // 回滚收据进学习信号 (供下一轮提案参考)
        assert!(reg.revert_receipts().unwrap().iter().any(|(_, s, _)| s == "rolled_back"));
        // 验证失败路径对照: Rejected → LoopAction::Rollback (不部署)
        let (d2, _, _) = gate.run_until_conclusion(
            |_| VerifyOutcome { passed: false, changed: false, error: Some("编译失败".into()) },
            std::time::Instant::now(),
        );
        assert!(matches!(gate.loop_action(&d2), LoopAction::Rollback { .. }));
    }
}
