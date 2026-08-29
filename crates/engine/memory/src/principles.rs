//! `apeireth-memory::principles` — 动态原则层与原则洋葱晋级候选 (Level 2/3 自成长信条机制).
//!
//! **设计哲学 (主人设想与安全洋葱边界)**:
//! - **Level 2 动态原则层 (洋葱外层)**: AI 提案原则候选 (`pending`) → 哲学评审 →
//!   主人签发提案绑定的短期批准凭据 → `active`（仅库内状态，不是 Runtime 规则）
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
//!
//! **生产接线禁令 (P2)**: 本模块的 `Active` 状态和
//! `PrincipleApprovalArtifact` 仅服务于库内演示/生命周期逻辑。未来任何生产
//! 激活都**必须**经 canonical Runtime 的 governance/approval 路径；不得从
//! Runtime 构造或消费本模块的 store、artifact 或 master token。

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::MemoryError;

/// 动态原则状态.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrincipleStatus {
    /// 待主人审查与批准
    Pending,
    /// 已在库内批准；尚非 canonical Runtime 生效状态
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
///
/// **诚实限制声明**: 循环次数取决于较长一侧的输入长度, 因此比较耗时仍会泄露
/// `max(len(a), len(b))`. 本实现保证的是:
/// - 比较耗时**与内容无关** (不泄露首个差异字节位置或前缀匹配深度);
/// - 移除了旧实现中长度不匹配时的提前返回分支.
///
/// 并非数学意义上完美的常数时间实现; 如需严格常数时间语义应改用专用密码学库.
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    // 以较长一侧决定循环次数, 短侧越界补 0: 循环体内不存在依赖内容或
    // 长度的提前退出分支.
    let max_len = a_bytes.len().max(b_bytes.len());
    let mut diff = 0u8;
    for i in 0..max_len {
        let x = a_bytes.get(i).copied().unwrap_or(0);
        let y = b_bytes.get(i).copied().unwrap_or(0);
        diff |= x ^ y;
    }
    // 长度差异折叠进最终判定: 纯零字节前缀 (如 "\0\0" vs "\0") 的字节折叠
    // 可能为 0, 必须由长度相等性兜底. 此为唯一最终分支, 只依赖长度相等性.
    let length_equal = a_bytes.len() == b_bytes.len();
    diff == 0 && length_equal
}

/// 默认的库内原则批准凭据有效期（毫秒）。
///
/// 调用方显式传入 `now_epoch_ms`，因此测试和嵌入方可以使用自己的时钟；本
/// 模块不会读取 `SystemTime`。该凭据不是 Runtime 批准令牌，当前没有任何
/// 生产 Runtime 路径消费它。
pub const DEFAULT_PRINCIPLE_APPROVAL_TTL_MS: i64 = 5 * 60 * 1000;

/// 库内原则批准凭据可配置的最长有效期（毫秒）。
pub const MAX_PRINCIPLE_APPROVAL_TTL_MS: i64 = 24 * 60 * 60 * 1000;

/// 用于一次原则激活的、不透明且不可重放的库内批准凭据。
///
/// 凭据只由 [`PrincipleStore::issue_approval`] 创建；其 nonce、提案 ID 和
/// 内容绑定均不对调用方公开，也不会由 `Debug` 输出。它是本模块尚未接线
/// 的本地生命周期能力，**不是** canonical Runtime 的批准路径或凭据。
pub struct PrincipleApprovalArtifact {
    nonce: String,
}

impl std::fmt::Debug for PrincipleApprovalArtifact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PrincipleApprovalArtifact")
            .field("nonce", &"<redacted>")
            .finish()
    }
}

#[derive(Clone)]
struct IssuedApproval {
    proposal_id: String,
    proposal_fingerprint: [u8; 32],
    issued_at_epoch_ms: i64,
    expires_at_epoch_ms: i64,
}

#[derive(Default)]
struct PrincipleState {
    principles: HashMap<String, DynamicPrinciple>,
    issued_approvals: HashMap<String, IssuedApproval>,
}

fn approval_error(message: &'static str) -> MemoryError {
    MemoryError::Invalid(message.to_string())
}

fn update_fingerprint_text(hasher: &mut Sha256, text: &str) {
    hasher.update((text.len() as u64).to_be_bytes());
    hasher.update(text.as_bytes());
}

/// Computes a domain-separated digest over every proposal identity and content
/// field. Any proposal mutation therefore invalidates a previously issued
/// approval artifact, including a stale revision selected by chain.
fn proposal_fingerprint(principle: &DynamicPrinciple) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"apeireth/principle-approval/v1\0");
    update_fingerprint_text(&mut hasher, &principle.id);
    update_fingerprint_text(&mut hasher, &principle.chain);
    hasher.update(principle.rev.to_be_bytes());
    update_fingerprint_text(&mut hasher, principle.status.as_str());
    update_fingerprint_text(&mut hasher, &principle.statement);
    update_fingerprint_text(&mut hasher, &principle.rationale);
    update_fingerprint_text(&mut hasher, &principle.source);
    hasher.update(principle.violations.to_be_bytes());
    hasher.update(principle.created_at_epoch_ms.to_be_bytes());
    hasher.update(principle.updated_at_epoch_ms.to_be_bytes());
    hasher.finalize().into()
}

/// Resolves either a chain or a historical version ID to the current latest
/// version of that chain. A caller cannot use an old version ID to bypass a
/// newer proposal revision.
fn latest_principle_for(state: &PrincipleState, chain_or_id: &str) -> Option<DynamicPrinciple> {
    let chain = state
        .principles
        .get(chain_or_id)
        .map(|principle| principle.chain.clone())
        .unwrap_or_else(|| chain_or_id.to_string());
    state
        .principles
        .values()
        .filter(|principle| principle.chain == chain)
        .max_by_key(|principle| principle.rev)
        .cloned()
}

fn fresh_approval_nonce(state: &PrincipleState) -> Result<String, MemoryError> {
    // UUID v4 collisions are already extraordinarily unlikely. The explicit
    // map check makes collision handling fail closed instead of permitting one
    // artifact's nonce to overwrite another proposal binding.
    for _ in 0..8 {
        let nonce = Uuid::new_v4().to_string();
        if !state.issued_approvals.contains_key(&nonce) {
            return Ok(nonce);
        }
    }
    Err(approval_error("无法分配唯一原则批准凭据"))
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

    /// 用正确的 master token 为当前 Pending 提案签发短期、提案绑定的批准凭据。
    ///
    /// 该步骤不激活原则。返回的凭据只能用于相同身份和内容的当前提案，且
    /// 有限期、单次成功使用。`now_epoch_ms` 由调用方提供，避免本库自行引入
    /// 时钟权威。
    fn issue_approval(
        &self,
        chain_or_id: &str,
        master_token: &str,
        now_epoch_ms: i64,
    ) -> Result<PrincipleApprovalArtifact, MemoryError>;

    /// 使用已签发的提案绑定凭据批准原则。
    ///
    /// **架构边界 (P2 硬化)**: 此操作是**库内本地**的原则生命周期操作，
    /// 当前未接入 canonical Runtime 审批/治理。未来生产激活必须路由到
    /// canonical Runtime governance/approval；不得把本机制接成第二审批
    /// 权威，也不得据此修改 Runtime 行为。
    fn approve(
        &self,
        chain_or_id: &str,
        approval: &PrincipleApprovalArtifact,
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
///
/// **架构边界 (P2 硬化)**: 本类型及其 master-token 认证和
/// [`PrincipleApprovalArtifact`] 是**库内本地**实现。当前 production Runtime
/// 无此 crate 依赖，故没有构造/调用路径，必须保持 UNWIRED；未来接入必须经
/// canonical Runtime governance/approval 路径，不得自行成为第二审批权威。
///
/// master token 与 artifact nonce 永不出现在 `Debug`/`Display`/错误/日志
/// 输出中，也不泄露 token 长度。
pub struct InMemoryPrincipleStore {
    state: Mutex<PrincipleState>,
    master_token: Option<String>,
    approval_ttl_ms: i64,
}

impl std::fmt::Debug for InMemoryPrincipleStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (principles_count, issued_approvals_count) = self
            .state
            .lock()
            .map(|state| (state.principles.len(), state.issued_approvals.len()))
            .unwrap_or((0, 0));
        f.debug_struct("InMemoryPrincipleStore")
            .field("principles_count", &principles_count)
            .field("issued_approvals_count", &issued_approvals_count)
            .field("master_token", &"<redacted>")
            .finish()
    }
}

impl InMemoryPrincipleStore {
    pub fn new(master_token: Option<String>) -> Self {
        Self {
            state: Mutex::new(PrincipleState::default()),
            master_token,
            approval_ttl_ms: DEFAULT_PRINCIPLE_APPROVAL_TTL_MS,
        }
    }

    pub fn with_token(token: impl Into<String>) -> Self {
        Self::new(Some(token.into()))
    }

    /// Creates a library-local store with a bounded custom approval lifetime.
    ///
    /// `now_epoch_ms` is supplied at issue/consume time; this constructor does
    /// not create a clock authority. Values outside the bounded window fail
    /// rather than allowing effectively permanent bearer artifacts.
    pub fn with_approval_ttl(
        master_token: Option<String>,
        approval_ttl_ms: i64,
    ) -> Result<Self, MemoryError> {
        if !(1..=MAX_PRINCIPLE_APPROVAL_TTL_MS).contains(&approval_ttl_ms) {
            return Err(approval_error("原则批准凭据有效期不在允许范围内"));
        }
        Ok(Self {
            state: Mutex::new(PrincipleState::default()),
            master_token,
            approval_ttl_ms,
        })
    }

    fn purge_expired_approvals(state: &mut PrincipleState, now_epoch_ms: i64) {
        state
            .issued_approvals
            .retain(|_, approval| approval.expires_at_epoch_ms >= now_epoch_ms);
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

        let mut state = self.state.lock().expect("principles mutex");
        state
            .principles
            .insert(principle.id.clone(), principle.clone());
        Ok(principle)
    }

    fn issue_approval(
        &self,
        chain_or_id: &str,
        master_token: &str,
        now_epoch_ms: i64,
    ) -> Result<PrincipleApprovalArtifact, MemoryError> {
        let expected = self
            .master_token
            .as_deref()
            .ok_or_else(|| approval_error("未配置 master token，无法签发原则批准凭据"))?;
        if !constant_time_eq(master_token, expected) {
            return Err(approval_error("Master Token 校验失败"));
        }

        let expires_at_epoch_ms = now_epoch_ms
            .checked_add(self.approval_ttl_ms)
            .ok_or_else(|| approval_error("原则批准凭据过期时间溢出"))?;
        let mut state = self.state.lock().expect("principles mutex");
        Self::purge_expired_approvals(&mut state, now_epoch_ms);
        let latest = latest_principle_for(&state, chain_or_id);

        let Some(p) = latest else {
            return Err(approval_error("原则不存在"));
        };

        if p.status != PrincipleStatus::Pending {
            return Err(approval_error("仅处于 Pending 状态的原则可签发批准凭据"));
        }

        let nonce = fresh_approval_nonce(&state)?;
        let proposal_id = p.id.clone();
        let proposal_fingerprint = proposal_fingerprint(&p);
        state.issued_approvals.insert(
            nonce.clone(),
            IssuedApproval {
                proposal_id,
                proposal_fingerprint,
                issued_at_epoch_ms: now_epoch_ms,
                expires_at_epoch_ms,
            },
        );
        Ok(PrincipleApprovalArtifact { nonce })
    }

    fn approve(
        &self,
        chain_or_id: &str,
        approval: &PrincipleApprovalArtifact,
        now_epoch_ms: i64,
    ) -> Result<DynamicPrinciple, MemoryError> {
        let mut state = self.state.lock().expect("principles mutex");
        let Some(issued) = state.issued_approvals.get(&approval.nonce).cloned() else {
            return Err(approval_error("原则批准凭据无效、已使用或已过期"));
        };
        if now_epoch_ms < issued.issued_at_epoch_ms {
            return Err(approval_error("原则批准凭据的调用时间早于签发时间"));
        }
        if now_epoch_ms > issued.expires_at_epoch_ms {
            state.issued_approvals.remove(&approval.nonce);
            return Err(approval_error("原则批准凭据已过期"));
        }

        let latest = latest_principle_for(&state, chain_or_id);
        let Some(p) = latest else {
            return Err(approval_error("原则不存在"));
        };

        if p.status != PrincipleStatus::Pending {
            return Err(approval_error("仅处于 Pending 状态的原则可被批准"));
        }
        if issued.proposal_id != p.id || issued.proposal_fingerprint != proposal_fingerprint(&p) {
            return Err(approval_error("原则批准凭据与当前提案不匹配"));
        }

        let new_rev = p
            .rev
            .checked_add(1)
            .ok_or_else(|| approval_error("原则版本号已耗尽"))?;
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

        // Consume only as part of the successful activation state transition;
        // the mutex makes simultaneous uses of the same bearer artifact a
        // single-winner operation.
        state.issued_approvals.remove(&approval.nonce);
        state.principles.insert(updated.id.clone(), updated.clone());
        Ok(updated)
    }

    fn record_violation(&self, chain_or_id: &str, now_epoch_ms: i64) -> Result<(), MemoryError> {
        let mut state = self.state.lock().expect("principles mutex");
        let latest = latest_principle_for(&state, chain_or_id);

        if let Some(p) = latest {
            let updated = DynamicPrinciple {
                id: format!("princ-{}", Uuid::new_v4()),
                chain: p.chain.clone(),
                rev: p
                    .rev
                    .checked_add(1)
                    .ok_or_else(|| approval_error("原则版本号已耗尽"))?,
                statement: p.statement,
                rationale: p.rationale,
                source: p.source,
                status: p.status,
                violations: p.violations + 1,
                created_at_epoch_ms: p.created_at_epoch_ms,
                updated_at_epoch_ms: now_epoch_ms,
            };
            state.principles.insert(updated.id.clone(), updated);
        }
        Ok(())
    }

    fn list(&self, status: Option<PrincipleStatus>) -> Result<Vec<DynamicPrinciple>, MemoryError> {
        let state = self.state.lock().expect("principles mutex");
        let mut by_chain: HashMap<String, DynamicPrinciple> = HashMap::new();

        for p in state.principles.values() {
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
    use std::sync::{mpsc, Arc, Barrier};
    use std::thread;

    use super::*;

    #[test]
    fn constant_time_eq_works() {
        assert!(constant_time_eq("secret123", "secret123"));
        assert!(!constant_time_eq("secret123", "secret456"));
        assert!(!constant_time_eq("short", "longer_secret"));
    }

    /// P1 硬化: 比较矩阵 — 移除长度提前返回后, 全部判定仍正确.
    #[test]
    fn constant_time_eq_full_matrix() {
        // 相等
        assert!(constant_time_eq("", ""));
        assert!(constant_time_eq("abc", "abc"));
        // 多字节 UTF-8 按字节相等
        assert!(constant_time_eq("密钥token", "密钥token"));
        // 同长度不同内容
        assert!(!constant_time_eq("abc", "abd"));
        assert!(!constant_time_eq("密钥token", "密钥tOken"));
        // 不同长度 (含前缀关系)
        assert!(!constant_time_eq("abc", "abcd"));
        assert!(!constant_time_eq("abcd", "abc"));
        assert!(!constant_time_eq("", "a"));
        assert!(!constant_time_eq("a", ""));
        // 全零字节 + 长度不同: 字节折叠为 0, 必须由长度相等性兜底拒绝
        assert!(!constant_time_eq("\0\0", "\0"));
        assert!(!constant_time_eq("\0", "\0\0"));
        assert!(constant_time_eq("\0\0", "\0\0"));
    }

    /// P1/P2 硬化: master token 与 artifact nonce 不得出现在 Debug 输出中.
    #[test]
    fn approval_secrets_never_appear_in_debug() {
        let token = "super-secret-master-token-12345";
        let store = InMemoryPrincipleStore::with_token(token);
        let proposal = store
            .propose("禁止未经验证执行系统命令", "安全性保护", "安全守则", 1000)
            .unwrap();
        let artifact = store.issue_approval(&proposal.chain, token, 1000).unwrap();
        let dbg = format!("{store:?}");
        assert!(!dbg.contains(token), "Debug 不得包含完整 token: {dbg}");
        assert!(
            !dbg.contains("super-secret"),
            "Debug 不得包含 token 片段: {dbg}"
        );
        assert!(
            !dbg.contains("master-token-12345"),
            "Debug 不得包含 token 片段: {dbg}"
        );
        // 不泄露长度 (token 长 31 字节)
        assert!(!dbg.contains("31"), "Debug 不得泄露 token 长度: {dbg}");
        assert!(
            dbg.contains("<redacted>"),
            "Debug 应显式显示 redacted: {dbg}"
        );

        let artifact_dbg = format!("{artifact:?}");
        assert!(
            !artifact_dbg.contains(&artifact.nonce),
            "Debug 不得包含 artifact nonce: {artifact_dbg}"
        );
        assert!(
            artifact_dbg.contains("<redacted>"),
            "artifact Debug 应显式显示 redacted: {artifact_dbg}"
        );
    }

    /// P1/P2 硬化: 批准签发失败的错误文本不得回显尝试的 token 值.
    #[test]
    fn approval_failure_error_does_not_leak_attempted_token() {
        let store = InMemoryPrincipleStore::with_token("valid_token");
        let p = store
            .propose("禁止未经验证执行系统命令", "安全性保护", "安全守则", 1000)
            .unwrap();
        let err = store
            .issue_approval(&p.chain, "WRONG_SECRET_ATTEMPT_XY", 2000)
            .unwrap_err();
        let err_text = err.to_string();
        assert!(
            !err_text.contains("WRONG_SECRET_ATTEMPT_XY"),
            "错误文本不得回显 token: {err_text}"
        );
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
        let err = store.issue_approval(&p.chain, "wrong_token", 2000);
        assert!(err.is_err());

        // 正确 token 先签发精确绑定的短期凭据，再由凭据批准。
        let approval = store.issue_approval(&p.chain, "valid_token", 2000).unwrap();
        let approved = store.approve(&p.chain, &approval, 2000).unwrap();
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
        let approval = store.issue_approval(&p.chain, "token", 2000).unwrap();
        store.approve(&p.chain, &approval, 2000).unwrap();

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
        let p = DynamicPrinciple {
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

    #[test]
    fn approval_for_proposal_a_cannot_activate_proposal_b() {
        let store = InMemoryPrincipleStore::with_token("valid_token");
        let proposal_a = store.propose("A 原则", "A 理由", "测试", 1000).unwrap();
        let proposal_b = store.propose("B 原则", "B 理由", "测试", 1000).unwrap();
        let approval_a = store
            .issue_approval(&proposal_a.chain, "valid_token", 1000)
            .unwrap();

        let err = store
            .approve(&proposal_b.chain, &approval_a, 1001)
            .unwrap_err();
        assert!(matches!(err, MemoryError::Invalid(_)));

        // A mismatch must not consume A's valid artifact; it remains bound only
        // to proposal A and can still activate that exact proposal.
        let approved_a = store.approve(&proposal_a.chain, &approval_a, 1001).unwrap();
        assert_eq!(approved_a.chain, proposal_a.chain);
        assert_eq!(approved_a.status, PrincipleStatus::Active);
    }

    #[test]
    fn modified_pending_proposal_invalidates_prior_approval_artifact() {
        let store = InMemoryPrincipleStore::with_token("valid_token");
        let proposal = store.propose("原始声明", "原始理由", "测试", 1000).unwrap();
        let approval = store
            .issue_approval(&proposal.chain, "valid_token", 1000)
            .unwrap();

        // Simulate a changed pending proposal without adding a production
        // mutation API: any content mutation must invalidate its old artifact.
        let mut state = store.state.lock().expect("principles mutex");
        let pending = state.principles.get_mut(&proposal.id).unwrap();
        pending.statement = "被篡改的声明".to_string();
        pending.updated_at_epoch_ms = 1001;
        drop(state);

        let err = store.approve(&proposal.chain, &approval, 1002).unwrap_err();
        assert!(matches!(err, MemoryError::Invalid(_)));
        assert!(
            !err.to_string().contains(&approval.nonce),
            "错误不得泄露 artifact nonce"
        );
    }

    #[test]
    fn expired_approval_artifact_is_rejected_without_sleeping() {
        let store =
            InMemoryPrincipleStore::with_approval_ttl(Some("valid_token".into()), 1).unwrap();
        let proposal = store.propose("声明", "理由", "测试", 1000).unwrap();
        let approval = store
            .issue_approval(&proposal.chain, "valid_token", 1000)
            .unwrap();

        let err = store.approve(&proposal.chain, &approval, 1002).unwrap_err();
        assert!(matches!(err, MemoryError::Invalid(_)));
        assert!(store
            .list(Some(PrincipleStatus::Active))
            .unwrap()
            .is_empty());
    }

    #[test]
    fn approval_artifact_ttl_is_strictly_bounded() {
        assert!(matches!(
            InMemoryPrincipleStore::with_approval_ttl(Some("valid_token".into()), 0),
            Err(MemoryError::Invalid(_))
        ));
        assert!(matches!(
            InMemoryPrincipleStore::with_approval_ttl(
                Some("valid_token".into()),
                MAX_PRINCIPLE_APPROVAL_TTL_MS + 1,
            ),
            Err(MemoryError::Invalid(_))
        ));
    }

    #[test]
    fn successful_approval_artifact_cannot_be_replayed() {
        let store = InMemoryPrincipleStore::with_token("valid_token");
        let proposal = store.propose("声明", "理由", "测试", 1000).unwrap();
        let approval = store
            .issue_approval(&proposal.chain, "valid_token", 1000)
            .unwrap();

        assert!(store.approve(&proposal.chain, &approval, 1001).is_ok());
        let replay = store.approve(&proposal.chain, &approval, 1002);
        assert!(replay.is_err(), "a successful artifact must be single-use");
    }

    #[test]
    fn concurrent_uses_of_one_approval_artifact_have_one_winner() {
        let store = Arc::new(InMemoryPrincipleStore::with_token("valid_token"));
        let proposal = store.propose("声明", "理由", "测试", 1000).unwrap();
        let approval = Arc::new(
            store
                .issue_approval(&proposal.chain, "valid_token", 1000)
                .unwrap(),
        );
        let barrier = Arc::new(Barrier::new(3));
        let (tx, rx) = mpsc::channel();
        let mut workers = Vec::new();

        for _ in 0..2 {
            let store = Arc::clone(&store);
            let approval = Arc::clone(&approval);
            let barrier = Arc::clone(&barrier);
            let chain = proposal.chain.clone();
            let tx = tx.clone();
            workers.push(thread::spawn(move || {
                barrier.wait();
                tx.send(store.approve(&chain, approval.as_ref(), 1001).is_ok())
                    .unwrap();
            }));
        }
        drop(tx);

        barrier.wait();
        let successes = rx.into_iter().filter(|success| *success).count();
        for worker in workers {
            worker.join().unwrap();
        }
        assert_eq!(successes, 1, "one artifact must have exactly one winner");
    }

    #[test]
    fn library_only_principle_approval_has_no_runtime_dependency() {
        // The canonical Runtime manifest has no apeireth-memory dependency, so
        // production Runtime code cannot construct or consume this local store
        // or artifact. Future production wiring must use Runtime governance.
        let runtime_manifest = include_str!("../../runtime/Cargo.toml");
        assert!(
            !runtime_manifest.contains("apeireth-memory"),
            "canonical Runtime must not depend on the library-only principle approval mechanism"
        );
    }
}
