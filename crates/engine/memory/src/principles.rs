//! `apeireth-memory::principles` — 动态原则层与原则洋葱晋级候选 (Level 2/3 自成长信条机制).
//!
//! **设计哲学 (主人设想与安全洋葱边界)**:
//! - **Level 2 动态原则层 (洋葱外层)**: AI 提案原则候选 (`pending`) → 哲学评审 → 主人批准 (master token) → `active` (运行时规则)
//! - **Level 3 内层晋级候选**: `active` 原则长期生效且 0 违反 → 生成晋级补丁建议 (文档/JSON),
//!   **只能由主人侧工程动作写入编译期内层** (原则根 / 9 锚 / 13 键)，AI 永不直写内层
//! - **安全隔离 (O-1 & O-5)**:
//!   - 主人批准权在主人手里 (通过 constant-time master token 校验)
//!   - 0 假装: 编译期规则表自身不可被动态代码修改，内层晋级为纯导出建议报告
//!
//! **O-6 三阶审查**:
//! 1. 总体: 实现 AI 系统的自主原则反思与自成长沉淀机制
//! 2. 系统: 放置在 `apeireth-memory`, 与 `HistoryStream` 和 `governance` 契约对齐
//! 3. 架构: 强类型链式版本追踪 (`chain` + `rev`), 常数时间 token 比较, 0 unsafe

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::MemoryError;

/// 动态原则状态.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrincipleStatus {
    /// 待主人审查与批准
    Pending,
    /// 已批准并在运行时生效
    Active,
    /// 主人明确拒绝
    Rejected,
    /// 已退役废弃
    Retired,
}

impl PrincipleStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Rejected => "rejected",
            Self::Retired => "retired",
        }
    }
}

/// 动态原则条目 (链式版本追踪).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DynamicPrinciple {
    /// 唯一版本 ID
    pub id: String,
    /// 逻辑原则链标识 (同一原则的不同版本共享相同 chain)
    pub chain: String,
    /// 链内单调递增版本号 (rev 越大表示版本越新)
    pub rev: u64,
    /// 准则声明内容
    pub statement: String,
    /// 提出理由
    pub rationale: String,
    /// 来源 (如 "经验反思", "安全审查", "主人指令")
    pub source: String,
    /// 当前状态
    pub status: PrincipleStatus,
    /// active 生效后累计触发或违反次数
    pub violations: u64,
    /// 创建时间戳 (毫秒)
    pub created_at_epoch_ms: i64,
    /// 更新时间戳 (毫秒)
    pub updated_at_epoch_ms: i64,
}

/// 晋级候选 (长期 active 且 0 违规的优秀信条).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromotionCandidate {
    pub principle: DynamicPrinciple,
    /// 生效时长 (天数)
    pub active_days: i64,
}

/// 常数时间字符串比较 (防时序攻击).
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    if a_bytes.len() != b_bytes.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a_bytes.iter().zip(b_bytes.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// 动态原则持久化与审查 Trait.
pub trait PrincipleStore: Send + Sync {
    /// 提案一条新原则 (初始为 Pending 状态).
    fn propose(
        &self,
        statement: &str,
        rationale: &str,
        source: &str,
        now_epoch_ms: i64,
    ) -> Result<DynamicPrinciple, MemoryError>;

    /// 主人批准原则 (需要提供正确的 master token).
    fn approve(
        &self,
        chain_or_id: &str,
        master_token: &str,
        now_epoch_ms: i64,
    ) -> Result<DynamicPrinciple, MemoryError>;

    /// 记录一条规则违规或触发.
    fn record_violation(&self, chain_or_id: &str, now_epoch_ms: i64) -> Result<(), MemoryError>;

    /// 列出所有原则 (可选按状态过滤, 自动按 chain 折叠最新 rev).
    fn list(&self, status: Option<PrincipleStatus>) -> Result<Vec<DynamicPrinciple>, MemoryError>;

    /// 筛选晋级候选原则 (Active 且 0 违规).
    fn promotion_candidates(
        &self,
        now_epoch_ms: i64,
    ) -> Result<Vec<PromotionCandidate>, MemoryError> {
        let active = self.list(Some(PrincipleStatus::Active))?;
        let candidates = active
            .into_iter()
            .filter(|p| p.violations == 0)
            .map(|p| {
                let diff_ms = (now_epoch_ms - p.created_at_epoch_ms).max(0);
                let days = diff_ms / (1000 * 86400);
                PromotionCandidate {
                    principle: p,
                    active_days: days,
                }
            })
            .collect();
        Ok(candidates)
    }

    /// 导出内层晋级建议报告 (供主人工程审计参考).
    fn export_promotion_report(&self, now_epoch_ms: i64) -> Result<String, MemoryError> {
        let candidates = self.promotion_candidates(now_epoch_ms)?;
        if candidates.is_empty() {
            return Ok(String::new());
        }
        let mut report = String::from("# 原则洋葱内层晋级候选清单 (Level 3 Promotion Report)\n\n");
        report.push_str("> 以下原则长期生效且零违规记录。内层写入需由主人侧人工审计合并。\n\n");
        for c in candidates {
            report.push_str(&format!(
                "### 准则 ID: {}\n- **准则内容**: {}\n- **理由**: {}\n- **来源**: {}\n- **生效时长**: {} 天 (0 次违规)\n\n",
                c.principle.chain,
                c.principle.statement,
                c.principle.rationale,
                c.principle.source,
                c.active_days
            ));
        }
        Ok(report)
    }
}

/// 内存版动态原则存储器.
#[derive(Debug)]
pub struct InMemoryPrincipleStore {
    principles: Mutex<HashMap<String, DynamicPrinciple>>,
    master_token: Option<String>,
}

impl InMemoryPrincipleStore {
    pub fn new(master_token: Option<String>) -> Self {
        Self {
            principles: Mutex::new(HashMap::new()),
            master_token,
        }
    }

    pub fn with_token(token: impl Into<String>) -> Self {
        Self::new(Some(token.into()))
    }
}

impl PrincipleStore for InMemoryPrincipleStore {
    fn propose(
        &self,
        statement: &str,
        rationale: &str,
        source: &str,
        now_epoch_ms: i64,
    ) -> Result<DynamicPrinciple, MemoryError> {
        let stmt = statement.trim();
        if stmt.is_empty() {
            return Err(MemoryError::Json(serde::de::Error::custom(
                "原则声明不能为空",
            )));
        }
        let chain_id = format!("princ-{}", Uuid::new_v4());
        let principle = DynamicPrinciple {
            id: chain_id.clone(),
            chain: chain_id.clone(),
            rev: 1,
            statement: stmt.to_string(),
            rationale: rationale.trim().to_string(),
            source: source.trim().to_string(),
            status: PrincipleStatus::Pending,
            violations: 0,
            created_at_epoch_ms: now_epoch_ms,
            updated_at_epoch_ms: now_epoch_ms,
        };

        let mut guard = self.principles.lock().expect("principles mutex");
        guard.insert(principle.id.clone(), principle.clone());
        Ok(principle)
    }

    fn approve(
        &self,
        chain_or_id: &str,
        master_token: &str,
        now_epoch_ms: i64,
    ) -> Result<DynamicPrinciple, MemoryError> {
        let expected = self.master_token.as_deref().ok_or_else(|| {
            MemoryError::Json(serde::de::Error::custom("未配置 master token，无法批准"))
        })?;
        if !constant_time_eq(master_token, expected) {
            return Err(MemoryError::Json(serde::de::Error::custom(
                "Master Token 校验失败",
            )));
        }

        let mut guard = self.principles.lock().expect("principles mutex");
        let latest = guard
            .values()
            .filter(|p| p.chain == chain_or_id || p.id == chain_or_id)
            .max_by_key(|p| p.rev)
            .cloned();

        let Some(p) = latest else {
            return Err(MemoryError::Json(serde::de::Error::custom("原则不存在")));
        };

        if p.status != PrincipleStatus::Pending {
            return Err(MemoryError::Json(serde::de::Error::custom(
                "仅处于 Pending 状态的原则可被批准",
            )));
        }

        let new_rev = p.rev + 1;
        let updated = DynamicPrinciple {
            id: format!("princ-{}", Uuid::new_v4()),
            chain: p.chain.clone(),
            rev: new_rev,
            statement: p.statement,
            rationale: p.rationale,
            source: p.source,
            status: PrincipleStatus::Active,
            violations: p.violations,
            created_at_epoch_ms: p.created_at_epoch_ms,
            updated_at_epoch_ms: now_epoch_ms,
        };

        guard.insert(updated.id.clone(), updated.clone());
        Ok(updated)
    }

    fn record_violation(&self, chain_or_id: &str, now_epoch_ms: i64) -> Result<(), MemoryError> {
        let mut guard = self.principles.lock().expect("principles mutex");
        let latest = guard
            .values()
            .filter(|p| p.chain == chain_or_id || p.id == chain_or_id)
            .max_by_key(|p| p.rev)
            .cloned();

        if let Some(p) = latest {
            let updated = DynamicPrinciple {
                id: format!("princ-{}", Uuid::new_v4()),
                chain: p.chain.clone(),
                rev: p.rev + 1,
                statement: p.statement,
                rationale: p.rationale,
                source: p.source,
                status: p.status,
                violations: p.violations + 1,
                created_at_epoch_ms: p.created_at_epoch_ms,
                updated_at_epoch_ms: now_epoch_ms,
            };
            guard.insert(updated.id.clone(), updated);
        }
        Ok(())
    }

    fn list(&self, status: Option<PrincipleStatus>) -> Result<Vec<DynamicPrinciple>, MemoryError> {
        let guard = self.principles.lock().expect("principles mutex");
        let mut by_chain: HashMap<String, DynamicPrinciple> = HashMap::new();

        for p in guard.values() {
            if let Some(existing) = by_chain.get(&p.chain) {
                if p.rev > existing.rev {
                    by_chain.insert(p.chain.clone(), p.clone());
                }
            } else {
                by_chain.insert(p.chain.clone(), p.clone());
            }
        }

        let mut out: Vec<DynamicPrinciple> = by_chain
            .into_values()
            .filter(|p| status.map_or(true, |s| p.status == s))
            .collect();
        out.sort_by_key(|p| p.created_at_epoch_ms);
        Ok(out)
    }
}

/// 检查动作内容是否包含触发特定原则的内容 (字符串前缀或包含匹配).
pub fn check_dynamic_principles<'a>(
    action: &str,
    rules: &'a [DynamicPrinciple],
) -> Option<&'a DynamicPrinciple> {
    rules.iter().find(|r| action.contains(&r.statement))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_works() {
        assert!(constant_time_eq("secret123", "secret123"));
        assert!(!constant_time_eq("secret123", "secret456"));
        assert!(!constant_time_eq("short", "longer_secret"));
    }

    #[test]
    fn propose_and_approve_lifecycle() {
        let store = InMemoryPrincipleStore::with_token("valid_token");
        let p = store
            .propose("禁止未经验证执行系统命令", "安全性保护", "安全守则", 1000)
            .unwrap();

        assert_eq!(p.status, PrincipleStatus::Pending);
        assert_eq!(p.rev, 1);

        // 错误 token 拒绝
        let err = store.approve(&p.chain, "wrong_token", 2000);
        assert!(err.is_err());

        // 正确 token 批准
        let approved = store.approve(&p.chain, "valid_token", 2000).unwrap();
        assert_eq!(approved.status, PrincipleStatus::Active);
        assert_eq!(approved.rev, 2);
        assert_eq!(approved.chain, p.chain);

        // 列出 active 原则
        let active = store.list(Some(PrincipleStatus::Active)).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].statement, "禁止未经验证执行系统命令");
    }

    #[test]
    fn record_violation_and_promotion_candidates() {
        let store = InMemoryPrincipleStore::with_token("token");
        let p = store
            .propose("始终输出结构化 JSON", "工程规范", "团队规范", 1000)
            .unwrap();
        store.approve(&p.chain, "token", 2000).unwrap();

        // 此时 0 违规，且活跃 10 天 (10 * 86400 * 1000 ms)
        let now = 1000 + 10 * 86400 * 1000;
        let cands = store.promotion_candidates(now).unwrap();
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].active_days, 10);

        let report = store.export_promotion_report(now).unwrap();
        assert!(report.contains("原则洋葱内层晋级候选清单"));
        assert!(report.contains("始终输出结构化 JSON"));

        // 发生违规后不再属于晋级候选
        store.record_violation(&p.chain, now).unwrap();
        let cands_after = store.promotion_candidates(now).unwrap();
        assert!(cands_after.is_empty());
    }

    #[test]
    fn check_dynamic_principles_matcher() {
        let mut p = DynamicPrinciple {
            id: "p1".into(),
            chain: "p1".into(),
            rev: 1,
            statement: "rm -rf /".into(),
            rationale: "危险操作".into(),
            source: "审计".into(),
            status: PrincipleStatus::Active,
            violations: 0,
            created_at_epoch_ms: 1000,
            updated_at_epoch_ms: 1000,
        };
        let rules = vec![p];
        let matched = check_dynamic_principles("执行命令 rm -rf / 危险", &rules);
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().statement, "rm -rf /");
    }
}
