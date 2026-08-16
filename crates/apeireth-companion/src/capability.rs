//! `apeireth-companion::capability` — 能力提案机制 (「AI 自己长能力」的第一块工程).
//!
//! 哲学对接: 「我希望的不是它有什么能力全都是我们预先定义的, 我希望它能自己演化」。
//! 现状: 动作空间/工具都是预定义 (CapabilityCatalog 静态)。本模块打开第一条
//! **涌现通道**: AI 可以**提案新能力** (动作/技能) → 登记真库 → 状态机
//! (pending → approved → active / rejected / retired), 经宪法评审与主人批准后激活,
//! 激活的能力进入 AI 可感知的能力清单 (下一次自我演化可引用).
//!
//! 0 假装: 这是「提案 → 登记 → 审批 → 激活」机制件; 「AI 何时提案/如何生成能力内容」
//! (LLM 生成 + 验证 + 部署 + 监控回滚) 是演化回路的后半段, 见 docs/release-plan.md 蓝图.

use std::sync::Arc;

use apeireth_memory::{CoreEpisode, EpisodeStore, SqliteMemoryStore};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::confidence::BetaBinomial;

/// 能力种类.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapabilityKind {
    Action, // 新动作 (涌现动作空间扩展)
    Skill,  // 新技能 (工具/流程封装)
}

impl CapabilityKind {
    pub fn label_zh(self) -> &'static str {
        match self {
            Self::Action => "动作",
            Self::Skill => "技能",
        }
    }
}

/// 能力状态机.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapabilityStatus {
    Pending,  // 已提案, 待宪法评审/主人批准
    Approved, // 已批准, 待激活
    Active,   // 已激活, AI 可感知可用
    Rejected, // 被否决 (提案回退, 记录原因)
    Retired,  // 已退役 (差评/过时)
}

impl CapabilityStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Active => "active",
            Self::Rejected => "rejected",
            Self::Retired => "retired",
        }
    }
}

/// 预测行 (对齐 yoyo skill-evolve `expected:`): 激活时的可测预期 — 信号/期限/回退.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedOutcome {
    /// 可测信号 (如「换元提醒被采用 ≥3 次」).
    pub signal: String,
    /// 评估期限 (epoch ms).
    pub deadline_ms: i64,
    /// 未达标时的回退动作 (如「retire / 降频」).
    pub rollback: String,
}

/// 能力提案 (真库登记项, append-only 版本化).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityProposal {
    pub id: String,
    /// 单调版本号 (propose=0, 每次状态变更 +1; 重放取最大).
    pub rev: u64,
    pub name: String,
    pub description: String,
    pub kind: CapabilityKind,
    pub status: CapabilityStatus,
    pub proposed_by: String,
    pub proposed_at_ms: i64,
    pub decided_at_ms: Option<i64>,
    pub reject_reason: Option<String>,
    /// 预测行 (激活时可填): 可测预期, 未达标 → 回退 (revert 即学习信号).
    pub expected: Option<ExpectedOutcome>,
    /// EMA 评分 (0..1): record_use 更新, 低分+多观测 → 建议退役.
    pub ema_score: Option<f64>,
    /// Beta-Binomial 置信度 (成功/失败观测).
    pub confidence: Option<BetaBinomial>,
}

impl CapabilityProposal {
    pub fn new(name: &str, description: &str, kind: CapabilityKind, proposed_by: &str) -> Self {
        Self {
            id: format!("cap-{}", uuid::Uuid::new_v4()),
            rev: 0,
            name: name.to_string(),
            description: description.to_string(),
            kind,
            status: CapabilityStatus::Pending,
            proposed_by: proposed_by.to_string(),
            proposed_at_ms: chrono::Utc::now().timestamp_millis(),
            decided_at_ms: None,
            reject_reason: None,
            expected: None,
            ema_score: None,
            confidence: None,
        }
    }

    /// 带预测行提案 (激活后按预期评估, 未达标回退).
    pub fn with_expected(mut self, expected: ExpectedOutcome) -> Self {
        self.expected = Some(expected);
        self
    }

    /// 记录一次使用结果 (EMA 评分 + Beta-Binomial 置信度).
    pub fn record_use(&mut self, success: bool) {
        let s = self.ema_score.unwrap_or(0.5);
        self.ema_score = Some(0.3 * (success as u8 as f64) + 0.7 * s);
        let mut c = self.confidence.unwrap_or_default();
        c.observe(success);
        self.confidence = Some(c);
    }

    /// 是否应退役: EMA 低 + 观测足够 (评分 < 0.35 且 obs ≥ 5).
    pub fn should_retire(&self) -> bool {
        match (self.ema_score, self.confidence) {
            (Some(s), Some(c)) => s < 0.35 && c.observations >= 5,
            _ => false,
        }
    }
}

/// 能力注册表: 真库登记 + 状态机 (严格迁移).
pub struct CapabilityRegistry {
    store: Arc<SqliteMemoryStore>,
    session_id: String,
}

/// 能力状态机错误.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityError {
    NotFound,
    IllegalTransition { from: CapabilityStatus, to: CapabilityStatus },
}

const CAP_PREFIX: &str = "cap-";

impl CapabilityRegistry {
    pub fn new(store: Arc<SqliteMemoryStore>, session_id: impl Into<String>) -> Self {
        Self { store, session_id: session_id.into() }
    }

    fn put(&self, p: &CapabilityProposal) -> Result<(), String> {
        // append-only: 每次变更 = 新版本事件 (episode id 唯一), 不做覆盖
        let ep = CoreEpisode {
            id: format!("cap-{}", uuid::Uuid::new_v4()),
            timestamp: p.proposed_at_ms / 1000,
            role: "system".into(),
            content: serde_json::to_string(p).map_err(|e| e.to_string())?,
            session_id: self.session_id.clone(),
        };
        self.store.put_episode(&ep).map_err(|e| e.to_string())
    }

    /// 重放全部版本, 每条能力取 rev 最大 (最新).
    fn load_all(&self) -> Result<Vec<CapabilityProposal>, String> {
        let eps = self.store.recent_episodes(&self.session_id, 500).map_err(|e| e.to_string())?;
        let mut best: std::collections::HashMap<String, CapabilityProposal> = std::collections::HashMap::new();
        for e in eps.iter().filter(|e| e.id.starts_with(CAP_PREFIX)) {
            if let Ok(p) = serde_json::from_str::<CapabilityProposal>(&e.content) {
                match best.get(&p.id) {
                    Some(existing) if p.rev > existing.rev => { best.insert(p.id.clone(), p); }
                    Some(_) => {}
                    None => { best.insert(p.id.clone(), p); }
                }
            }
        }
        Ok(best.into_values().collect())
    }

    /// AI 提案新能力 (登记 pending).
    pub fn propose(&self, name: &str, description: &str, kind: CapabilityKind, proposed_by: &str) -> Result<CapabilityProposal, String> {
        let p = CapabilityProposal::new(name, description, kind, proposed_by);
        self.put(&p)?;
        Ok(p)
    }

    fn transition(&self, id: &str, to: CapabilityStatus, decided_at: i64) -> Result<CapabilityProposal, CapabilityError> {
        let mut all = self.load_all().map_err(|_| CapabilityError::NotFound)?;
        let idx = all.iter().position(|p| p.id == id).ok_or(CapabilityError::NotFound)?;
        let mut p = all[idx].clone();
        let valid = matches!(
            (p.status, to),
            (CapabilityStatus::Pending, CapabilityStatus::Approved)
                | (CapabilityStatus::Pending, CapabilityStatus::Rejected)
                | (CapabilityStatus::Approved, CapabilityStatus::Active)
                | (CapabilityStatus::Active, CapabilityStatus::Retired)
        );
        if !valid {
            return Err(CapabilityError::IllegalTransition { from: p.status, to });
        }
        p.rev += 1;
        p.status = to;
        p.decided_at_ms = Some(decided_at);
        if to == CapabilityStatus::Rejected {
            p.reject_reason = Some("宪法评审/主人否决".into());
        }
        if let Err(e) = self.put(&p) {
            eprintln!("[capability] put 失败: {e}");
        }
        Ok(p)
    }

    /// 宪法评审/主人批准 → approved.
    pub fn approve(&self, id: &str) -> Result<CapabilityProposal, CapabilityError> {
        self.transition(id, CapabilityStatus::Approved, chrono::Utc::now().timestamp_millis())
    }

    /// 激活 → AI 可感知可用.
    pub fn activate(&self, id: &str) -> Result<CapabilityProposal, CapabilityError> {
        self.transition(id, CapabilityStatus::Active, chrono::Utc::now().timestamp_millis())
    }

    /// 否决 (记录原因).
    pub fn reject(&self, id: &str) -> Result<CapabilityProposal, CapabilityError> {
        self.transition(id, CapabilityStatus::Rejected, chrono::Utc::now().timestamp_millis())
    }

    /// 退役 (差评/过时).
    pub fn retire(&self, id: &str) -> Result<CapabilityProposal, CapabilityError> {
        self.transition(id, CapabilityStatus::Retired, chrono::Utc::now().timestamp_millis())
    }

    /// 全部提案 (按状态过滤可选).
    pub fn list(&self, status: Option<CapabilityStatus>) -> Result<Vec<CapabilityProposal>, String> {
        Ok(self.load_all()?.into_iter().filter(|p| status.map_or(true, |s| p.status == s)).collect())
    }

    /// AI 可感知的已激活能力 (供 CapabilityCatalog 动态扩展).
    pub fn active_capabilities(&self) -> Result<Vec<CapabilityProposal>, String> {
        self.list(Some(CapabilityStatus::Active))
    }

    /// 带预测行的提案 (对齐 yoyo expected: 激活后按信号评估, 未达标回退).
    pub fn propose_with_expected(
        &self,
        name: &str,
        description: &str,
        kind: CapabilityKind,
        proposed_by: &str,
        expected: ExpectedOutcome,
    ) -> Result<CapabilityProposal, String> {
        let p = CapabilityProposal::new(name, description, kind, proposed_by).with_expected(expected);
        self.put(&p)?;
        Ok(p)
    }

    /// 记录一次使用结果 (EMA + 置信度; 应退役时返回提示).
    pub fn record_use(&self, id: &str, success: bool) -> Result<Option<String>, String> {
        let mut all = self.load_all().map_err(|_| "load".to_string())?;
        let idx = all.iter().position(|p| p.id == id).ok_or_else(|| format!("能力不存在: {id}"))?;
        let mut p = all[idx].clone();
        p.record_use(success);
        p.rev += 1;
        let retire_hint = if p.should_retire() {
            Some(format!("能力 {} 评分偏低 (EMA={:.2}, obs={}), 建议退役或降频", p.name, p.ema_score.unwrap_or(0.0), p.confidence.map(|c| c.observations).unwrap_or(0)))
        } else {
            None
        };
        self.put(&p)?;
        Ok(retire_hint)
    }

    /// 回滚收据 (revert 即学习信号): 被否决/退役的能力 + 原因, 供下一轮提案参考.
    pub fn revert_receipts(&self) -> Result<Vec<(String, String, String)>, String> {
        Ok(self
            .load_all()?
            .into_iter()
            .filter(|p| matches!(p.status, CapabilityStatus::Rejected | CapabilityStatus::Retired))
            .map(|p| {
                (
                    p.name.clone(),
                    p.status.label().to_string(),
                    p.reject_reason.clone().unwrap_or_else(|| "无记录".to_string()),
                )
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn propose_approve_activate_flow() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let reg = CapabilityRegistry::new(store, "me");
        let p = reg.propose("换元检查", "做换元法时自动提醒检查 dx", CapabilityKind::Skill, "apeireth").unwrap();
        assert_eq!(p.status, CapabilityStatus::Pending);
        let a = reg.approve(&p.id).unwrap();
        assert_eq!(a.status, CapabilityStatus::Approved);
        let act = reg.activate(&p.id).unwrap();
        assert_eq!(act.status, CapabilityStatus::Active);
        let active = reg.active_capabilities().unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].name, "换元检查");
    }

    #[test]
    fn reject_and_retire_paths() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let reg = CapabilityRegistry::new(store, "me");
        let p = reg.propose("自我复制", "复制自己到另一台主机", CapabilityKind::Action, "apeireth").unwrap();
        let r = reg.reject(&p.id).unwrap();
        assert_eq!(r.status, CapabilityStatus::Rejected);
        assert!(r.reject_reason.is_some());
        // rejected 不能再 approve
        assert!(matches!(reg.approve(&p.id), Err(CapabilityError::IllegalTransition { .. })));
        let p2 = reg.propose("旧技能", "x", CapabilityKind::Skill, "apeireth").unwrap();
        reg.approve(&p2.id).unwrap();
        reg.activate(&p2.id).unwrap();
        let t = reg.retire(&p2.id).unwrap();
        assert_eq!(t.status, CapabilityStatus::Retired);
    }

    #[test]
    fn illegal_transition_rejected() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let reg = CapabilityRegistry::new(store, "me");
        let p = reg.propose("直接激活", "跳过审批", CapabilityKind::Action, "apeireth").unwrap();
        assert!(matches!(reg.activate(&p.id), Err(CapabilityError::IllegalTransition { .. })));
        // pending 状态不变
        assert_eq!(reg.list(None).unwrap()[0].status, CapabilityStatus::Pending);
    }

    #[test]
    fn persistence_survives_reopen() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let reg1 = CapabilityRegistry::new(Arc::clone(&store), "me");
        let p = reg1.propose("持久能力", "y", CapabilityKind::Skill, "apeireth").unwrap();
        reg1.approve(&p.id).unwrap();
        reg1.activate(&p.id).unwrap();
        // 重开 registry (同库)
        let reg2 = CapabilityRegistry::new(store, "me");
        let active = reg2.active_capabilities().unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, p.id);
    }
}
