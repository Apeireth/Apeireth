//! 授权策略对象（Authorization Policy Object）—— **可携带 / 可委托 / 可过期 / 可追溯**。
//!
//! 上位：ADR-0014 §4「授权应做成可携带/可委托/可过期/可追溯的**策略对象**」，
//! 以及 §3 的承诺分级（我们只承诺自己控制范围内的停止与撤回 —— 前提是我们
//! **能说出**「当前什么被授权了、谁授的、到什么时候为止」）。
//!
//! # 与既有 `PermissionPolicy` 的关系
//!
//! 两者**不是同一件事**，故意并存：
//!
//! - [`crate::permission::PermissionPolicy`] = **进程内、静态**的权限集
//!   （「这个能力能不能调」），是运行期的快路径；
//! - 本模块 = **带元数据**的授权凭证（「谁、什么时候、授了什么范围、到什么时候失效、
//!   能不能转授」），是**可携带**的那一层。
//!
//! 把两者混为一谈会犯一个具体错误：**授权了就永远有效**、**转授能扩大范围**。
//! 本模块的四条不变式就是防这两件事的。
//!
//! # 四条不变式（都有测试钉死）
//!
//! 1. **委托只能收窄**：子授权的 `scope` 必须是父授权的子集 —— 扩大即拒绝。
//! 2. **过期即失效**：`expires_at` 过了就是无效凭证（不是「建议不用」）。
//! 3. **可携带**：serde JSON 往返无损（可以带去另一台机器/另一个进程）。
//! 4. **可追溯**：每条授权带 `issuer` + `issued_at`；策略整体带内容指纹；
//!    委托链完整记录「谁授给谁、什么时候、是否收窄」。

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// 授权动作（粗粒度；与具体工具名解耦）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorizationAction {
    /// 读取。
    Read,
    /// 写入 / 修改。
    Write,
    /// 出网（含「模型上下文出境」这一路）。
    Egress,
    /// 委派 / 转授。
    Delegate,
    /// 管理（改策略本身）。
    Admin,
}

impl AuthorizationAction {
    /// 稳定标签（报告面用）。
    pub fn label(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Egress => "egress",
            Self::Delegate => "delegate",
            Self::Admin => "admin",
        }
    }
}

/// 一条带范围的授权。
///
/// `scope` 为空 = 「该 resource 的全部」；非空 = **只限这些具体项**（收窄用）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopedGrant {
    /// 资源（对象或用途，如 `provider:anthropic` / `memory:recall`）。
    pub resource: String,
    /// 动作。
    pub action: AuthorizationAction,
    /// 范围（空 = 全部；非空 = 子集）。
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub scope: BTreeSet<String>,
}

impl ScopedGrant {
    /// 新建一条「该资源全部」的授权。
    pub fn new(resource: impl Into<String>, action: AuthorizationAction) -> Self {
        Self {
            resource: resource.into(),
            action,
            scope: BTreeSet::new(),
        }
    }

    /// 新建一条限范围的授权。
    pub fn scoped(
        resource: impl Into<String>,
        action: AuthorizationAction,
        scope: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            resource: resource.into(),
            action,
            scope: scope.into_iter().collect(),
        }
    }

    /// 是否允许访问 `item`。
    ///
    /// - scope 为空 → 该资源全部允许；
    /// - scope 非空 → **逐项匹配**（父 scope 空时子项必然 ⊆）。
    pub fn allows(&self, item: &str) -> bool {
        self.scope.is_empty() || self.scope.contains(item)
    }

    /// 是否是 `other` 的**收窄**（`other` 的范围 ⊆ 自己的范围）。
    ///
    /// 空 scope = 「全部」，因此它**包含**任何非空 scope ⇒ 收窄成立。
    pub fn narrows_to(&self, other: &ScopedGrant) -> bool {
        if self.resource != other.resource || self.action != other.action {
            return false;
        }
        if self.scope.is_empty() {
            return true;
        }
        other.scope.is_subset(&self.scope)
    }
}

/// 一次委托记录（谁授给谁、什么时候、是否收窄）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationRecord {
    /// 被授方。
    pub to: String,
    /// 委托时刻（epoch 毫秒）。
    pub at_ms: i64,
    /// 本次委托是否收窄了范围（**false 只允许出现在显式记录里，不允许发生**——
    /// 扩大范围的委托在 `delegate()` 处就被拒绝了）。
    pub narrowed: bool,
}

/// 授权策略对象。
///
/// 拿到它的人（或进程）能回答四个问题：
/// **谁授的？授了什么？到什么时候为止？转过给谁？**
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorizationPolicy {
    /// 策略版本（预留演进位；当前恒为 1）。
    pub version: u32,
    /// 持有者（这份策略属于谁）。
    pub subject: String,
    /// 签发者（谁给的权）。
    pub issuer: String,
    /// 签发时刻（epoch 毫秒）。
    pub issued_at_ms: i64,
    /// 失效时刻（`None` = 不过期）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at_ms: Option<i64>,
    /// 撤销时刻（`Some` = 已撤销，任何时刻都不再有效）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoked_at_ms: Option<i64>,
    /// 授权条目。
    pub grants: Vec<ScopedGrant>,
    /// 委托链（从直接签发者往后排）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub delegations: Vec<DelegationRecord>,
}

impl AuthorizationPolicy {
    /// 新建策略（不过期、不撤销）。
    pub fn new(
        subject: impl Into<String>,
        issuer: impl Into<String>,
        issued_at_ms: i64,
        grants: Vec<ScopedGrant>,
    ) -> Self {
        Self {
            version: 1,
            subject: subject.into(),
            issuer: issuer.into(),
            issued_at_ms,
            expires_at_ms: None,
            revoked_at_ms: None,
            grants,
            delegations: Vec::new(),
        }
    }

    /// 设置失效时刻（可过期）。
    pub fn with_expiry(mut self, expires_at_ms: i64) -> Self {
        self.expires_at_ms = Some(expires_at_ms);
        self
    }

    /// 撤销（自此刻起无效）。
    pub fn revoke(mut self, at_ms: i64) -> Self {
        self.revoked_at_ms = Some(at_ms);
        self
    }

    /// 此刻是否有效（未撤销、未过期）。
    pub fn is_valid_at(&self, now_ms: i64) -> bool {
        if self.revoked_at_ms.is_some() {
            return false;
        }
        match self.expires_at_ms {
            Some(expiry) => now_ms < expiry,
            None => true,
        }
    }

    /// 在此刻是否允许 `action` 访问 `resource` 的 `item`。
    ///
    /// 先判策略有效性（撤销 / 过期）→ 再找匹配的 grant → 判 scope。
    pub fn allows_at(
        &self,
        resource: &str,
        item: &str,
        action: &AuthorizationAction,
        now_ms: i64,
    ) -> bool {
        if !self.is_valid_at(now_ms) {
            return false;
        }
        self.grants.iter().any(|grant| {
            grant.resource == resource && grant.action == *action && grant.allows(item)
        })
    }

    /// 签发一份**子授权**（委托）。
    ///
    /// **只能收窄**：`child` 的每一条 grant 都必须是本策略某条 grant 的子集，
    /// 否则整体拒绝（不部分签发 —— 部分成功比整体失败更难查）。
    ///
    /// 返回子策略；失败时给出原因。
    pub fn delegate(
        &self,
        to: &str,
        now_ms: i64,
        child_grants: Vec<ScopedGrant>,
    ) -> Result<AuthorizationPolicy, String> {
        if !self.is_valid_at(now_ms) {
            return Err("策略已失效（撤销或过期）—— 不可委托".to_string());
        }
        for child in &child_grants {
            let covered = self.grants.iter().any(|parent| parent.narrows_to(child));
            if !covered {
                return Err(format!(
                    "委托被拒：子授权 {}:{} 超出父策略范围（委托只能收窄，不能扩大）",
                    child.resource,
                    child.action.label()
                ));
            }
        }
        // 允许委派动作本身必须被显式授予。
        if !child_grants.is_empty()
            && !self.grants.iter().any(|grant| {
                grant.action == AuthorizationAction::Delegate
                    && grant.allows(&child_grants[0].resource)
            })
        {
            return Err("委托被拒：父策略未授予 delegate 动作".to_string());
        }
        let narrowed = child_grants
            .iter()
            .all(|child| self.grants.iter().any(|parent| parent.narrows_to(child)));
        Ok(AuthorizationPolicy {
            version: 1,
            subject: to.to_string(),
            issuer: self.subject.clone(),
            issued_at_ms: now_ms,
            expires_at_ms: self.expires_at_ms,
            revoked_at_ms: None,
            grants: child_grants,
            delegations: {
                let mut chain = self.delegations.clone();
                chain.push(DelegationRecord {
                    to: to.to_string(),
                    at_ms: now_ms,
                    narrowed,
                });
                chain
            },
        })
    }

    /// 内容指纹（可追溯：改了内容，指纹就变）。
    pub fn fingerprint(&self) -> String {
        let mut hasher = Sha256::new();
        // 字段顺序固定 ⇒ 指纹稳定。
        hasher.update(self.version.to_le_bytes());
        hasher.update(self.subject.as_bytes());
        hasher.update(self.issuer.as_bytes());
        hasher.update(self.issued_at_ms.to_le_bytes());
        hasher.update(self.expires_at_ms.unwrap_or(i64::MIN).to_le_bytes());
        hasher.update(self.revoked_at_ms.unwrap_or(i64::MIN).to_le_bytes());
        for grant in &self.grants {
            hasher.update(grant.resource.as_bytes());
            hasher.update(grant.action.label().as_bytes());
            for item in &grant.scope {
                hasher.update(item.as_bytes());
            }
            hasher.update([0u8]);
        }
        let digest = hasher.finalize();
        digest
            .iter()
            .take(16)
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    /// 可携带形式（JSON 字符串）—— 带走后用 [`AuthorizationPolicy::from_carryable`] 复原。
    pub fn to_carryable(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// 从可携带形式复原。
    pub fn from_carryable(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|error| format!("授权策略解析失败: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> AuthorizationPolicy {
        AuthorizationPolicy::new(
            "user-1",
            "owner",
            1_000,
            vec![
                ScopedGrant::new("provider:anthropic", AuthorizationAction::Egress),
                ScopedGrant::new("memory:recall", AuthorizationAction::Read),
                ScopedGrant::new("scope:everything", AuthorizationAction::Delegate),
            ],
        )
    }

    #[test]
    fn valid_policy_allows_granted_action() {
        let policy = base();
        assert!(policy.allows_at(
            "provider:anthropic",
            "chat",
            &AuthorizationAction::Egress,
            2_000
        ));
        assert!(!policy.allows_at(
            "provider:openai",
            "chat",
            &AuthorizationAction::Egress,
            2_000
        ));
    }

    #[test]
    fn expired_policy_is_invalid() {
        let policy = base().with_expiry(1_500);
        assert!(policy.is_valid_at(1_499));
        assert!(!policy.is_valid_at(1_500), "过期即失效");
        assert!(!policy.allows_at(
            "provider:anthropic",
            "chat",
            &AuthorizationAction::Egress,
            1_500
        ));
    }

    #[test]
    fn revoked_policy_is_invalid() {
        let policy = base().revoke(1_200);
        assert!(!policy.is_valid_at(1_199));
        assert!(!policy.is_valid_at(1_200), "撤销即失效");
        assert!(!policy.is_valid_at(9_999), "撤销不可复活");
    }

    #[test]
    fn delegation_may_narrow_but_not_widen() {
        let policy = base();
        // 收窄：同一资源，scope 是子集。
        let child = policy
            .delegate(
                "user-2",
                2_000,
                vec![ScopedGrant::scoped(
                    "provider:anthropic",
                    AuthorizationAction::Egress,
                    vec!["chat".to_string()],
                )],
            )
            .expect("收窄应允许");
        assert!(child.allows_at(
            "provider:anthropic",
            "chat",
            &AuthorizationAction::Egress,
            2_100
        ));
        assert!(
            !child.allows_at(
                "provider:anthropic",
                "email",
                &AuthorizationAction::Egress,
                2_100
            ),
            "子授权不得超出收窄范围"
        );

        // 扩大：换资源 → 拒绝。
        let err = policy
            .delegate(
                "user-3",
                2_000,
                vec![ScopedGrant::new(
                    "provider:openai",
                    AuthorizationAction::Egress,
                )],
            )
            .expect_err("扩大应拒绝");
        assert!(err.contains("委托只能收窄"));
    }

    #[test]
    fn delegation_requires_delegate_action() {
        let policy = AuthorizationPolicy::new(
            "user-1",
            "owner",
            1_000,
            vec![ScopedGrant::new(
                "provider:anthropic",
                AuthorizationAction::Egress,
            )],
        );
        let err = policy
            .delegate(
                "user-2",
                2_000,
                vec![ScopedGrant::new(
                    "provider:anthropic",
                    AuthorizationAction::Egress,
                )],
            )
            .expect_err("未授 delegate 应拒绝");
        assert!(err.contains("delegate"));
    }

    #[test]
    fn expired_policy_cannot_delegate() {
        let policy = base().with_expiry(1_500);
        let err = policy
            .delegate(
                "user-2",
                1_600,
                vec![ScopedGrant::new(
                    "provider:anthropic",
                    AuthorizationAction::Egress,
                )],
            )
            .expect_err("过期策略不可委托");
        assert!(err.contains("失效"));
    }

    #[test]
    fn policy_is_carryable() {
        let policy = base().with_expiry(9_999);
        let carried = policy.to_carryable();
        let restored = AuthorizationPolicy::from_carryable(&carried).expect("应可复原");
        assert_eq!(restored, policy);
        assert_eq!(restored.fingerprint(), policy.fingerprint());
    }

    #[test]
    fn fingerprint_changes_with_content() {
        let policy = base();
        let mut changed = policy.clone();
        changed.grants[0].scope.insert("chat".to_string());
        assert_ne!(policy.fingerprint(), changed.fingerprint());
    }

    #[test]
    fn delegation_chain_is_traceable() {
        let policy = base();
        let child = policy
            .delegate(
                "user-2",
                2_000,
                vec![ScopedGrant::new(
                    "provider:anthropic",
                    AuthorizationAction::Egress,
                )],
            )
            .expect("应允许");
        assert_eq!(child.delegations.len(), 1);
        assert_eq!(child.delegations[0].to, "user-2");
        assert_eq!(child.delegations[0].at_ms, 2_000);
        assert!(child.delegations[0].narrowed);
        // 签发者链条：子策略的签发者是父策略的持有者。
        assert_eq!(child.issuer, "user-1");
    }
}
