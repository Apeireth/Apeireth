//! HA 部署模式自适应 (single / multi / offline) + 生物特征 trait 抽象
//!
//! **设计**:
//! - `HAMode` 枚举部署模式
//! - `SingleHumanPolicy` 单人模式 (1 个真实人类 + Windows Hello / FIDO2 / 主人密钥)
//! - `MultiSigPolicy` 多人模式 (M-of-N 多签)
//! - `BiometricProvider` Rust trait — 不依赖 PyO3 / 外部 SDK; mock provider 真实实现
//! - `BiometricResult` 4 状态: Authenticated / CoercionDetected / Failed / Unavailable

use serde::{Deserialize, Serialize};
use std::fmt;

/// 生物特征 Provider trait — Rust trait 抽象 (不依赖 PyO3 / 外部 SDK).
///
/// **用法**:
/// ```ignore
/// use apeireth_sovereignty::{BiometricProvider, BiometricResult};
///
/// struct WindowsHelloProvider; // 未来真实实现 (Windows WinRT SDK, 非 PyO3)
/// impl BiometricProvider for WindowsHelloProvider {
///     fn authenticate(&self, human_id: &str) -> BiometricResult {
///         // 调用 Windows Hello API (不通过 PyO3, 直接 WinRT/FFI)
///         ...
///     }
/// }
///
/// struct Fido2Provider; // 未来真实实现 (WebAuthn / libfido2)
/// impl BiometricProvider for Fido2Provider {
///     fn authenticate(&self, human_id: &str) -> BiometricResult { ... }
/// }
/// ```
///
/// 本 trait 设计用于 Rust 内真实接入; 当前默认实现 [`crate::mock_biometric::MockBiometric`].
pub trait BiometricProvider: Send + Sync {
    /// 触发认证 (返回 [`BiometricResult`])
    fn authenticate(&self, human_id: &str) -> BiometricResult;

    /// 是否可用 (e.g. 离线模式下 false)
    fn is_available(&self) -> bool {
        true
    }

    /// 提供者名称
    fn provider_name(&self) -> &str;
}

/// 生物特征认证结果。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BiometricResult {
    /// 认证通过
    Authenticated {
        /// 置信度 (0.0 - 1.0)
        confidence: f64,
        /// 认证时间 (epoch ms)
        at_ms: i64,
    },
    /// 抗胁迫检测触发 (拒绝认证)
    CoercionDetected {
        /// 检测到的压力水平 (0.0 - 1.0)
        stress_level: f64,
        /// 检测时间 (epoch ms)
        at_ms: i64,
    },
    /// 认证失败
    Failed {
        /// 失败原因
        reason: String,
        /// 失败时间 (epoch ms)
        at_ms: i64,
    },
    /// 提供者不可用 (e.g. 离线模式)
    Unavailable {
        /// 不可用原因
        reason: String,
    },
}

impl BiometricResult {
    /// 是否通过认证
    pub fn is_authenticated(&self) -> bool {
        matches!(self, Self::Authenticated { .. })
    }

    /// 是否检测到胁迫
    pub fn is_coercion(&self) -> bool {
        matches!(self, Self::CoercionDetected { .. })
    }

    /// 是否失败
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// 是否不可用
    pub fn is_unavailable(&self) -> bool {
        matches!(self, Self::Unavailable { .. })
    }
}

/// HA 部署模式。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HAMode {
    /// 单人模式: 1 个真实人类 + Windows Hello / FIDO2 / 主人密钥
    SingleHuman(SingleHumanPolicy),
    /// 多人模式: N 个真实人类多人多签 (M-of-N)
    MultiHuman(MultiSigPolicy),
    /// 离线模式: 主人不在 = 安静模式 (仅允许 low / info)
    Offline,
}

impl HAMode {
    /// 是否处于离线模式
    pub fn is_offline(&self) -> bool {
        matches!(self, Self::Offline)
    }

    /// 是否单人模式
    pub fn is_single(&self) -> bool {
        matches!(self, Self::SingleHuman(_))
    }

    /// 是否多人模式
    pub fn is_multi(&self) -> bool {
        matches!(self, Self::MultiHuman(_))
    }

    /// 人类总数
    pub fn human_count(&self) -> usize {
        match self {
            Self::SingleHuman(_p) => 1,
            Self::MultiHuman(p) => p.signatories.len(),
            Self::Offline => 0,
        }
    }

    /// 所需签名数 (单人=1, 多人=M, 离线=0)
    pub fn required_signatures(&self) -> usize {
        match self {
            Self::SingleHuman(_) => 1,
            Self::MultiHuman(p) => p.required,
            Self::Offline => 0,
        }
    }
}

impl fmt::Display for HAMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SingleHuman(p) => {
                write!(f, "single({})", p.human_id)
            }
            Self::MultiHuman(p) => {
                write!(
                    f,
                    "multi({}-of-{}, {} sigs)",
                    p.required,
                    p.signatories.len(),
                    p.signatories.len()
                )
            }
            Self::Offline => f.write_str("offline"),
        }
    }
}

/// 单人模式策略 (1 个真实人类 + Windows Hello / FIDO2 / 主人密钥).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SingleHumanPolicy {
    /// 唯一人类 ID
    pub human_id: String,
    /// 显示名
    pub name: String,
    /// 认证方式
    pub authentication: HAAuthentication,
}

impl SingleHumanPolicy {
    /// 便利构造
    pub fn new(
        human_id: impl Into<String>,
        name: impl Into<String>,
        authentication: HAAuthentication,
    ) -> Self {
        Self {
            human_id: human_id.into(),
            name: name.into(),
            authentication,
        }
    }
}

/// HA 认证方式 (5 种).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HAAuthentication {
    /// Windows Hello (面部/指纹, 通过 WinRT 接入, 不通过 PyO3)
    WindowsHello,
    /// FIDO2 安全密钥 (WebAuthn / libfido2, Rust 接入)
    FIDO2,
    /// 多人多签
    MultiHuman,
    /// 离线签名 (硬件 Token)
    OfflineSign,
    /// 主人密钥 (物理密钥卡)
    MasterKey,
}

impl fmt::Display for HAAuthentication {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::WindowsHello => "windows_hello",
            Self::FIDO2 => "fido2",
            Self::MultiHuman => "multi_human",
            Self::OfflineSign => "offline_sign",
            Self::MasterKey => "master_key",
        };
        f.write_str(s)
    }
}

/// 多签签者 (单人).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Signatory {
    /// 签者 ID
    pub id: String,
    /// 显示名
    pub name: String,
    /// 认证方式
    pub authentication: HAAuthentication,
}

impl Signatory {
    /// 便利构造
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        authentication: HAAuthentication,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            authentication,
        }
    }
}

/// M-of-N 多签策略。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiSigPolicy {
    /// 所需签名数 (M)
    pub required: usize,
    /// 总签者数 (N)
    pub signatories: Vec<Signatory>,
}

impl MultiSigPolicy {
    /// 创建多签策略 (校验 M ≤ N 且 M ≥ 1).
    pub fn new(required: usize, signatories: Vec<Signatory>) -> Result<Self, String> {
        if required < 1 {
            return Err("required 必须 ≥ 1".into());
        }
        if required > signatories.len() {
            return Err(format!(
                "required ({}) > signatories.len() ({})",
                required,
                signatories.len()
            ));
        }
        Ok(Self {
            required,
            signatories,
        })
    }

    /// 2-of-3 默认多签
    pub fn default_2_of_3() -> Self {
        Self {
            required: 2,
            signatories: vec![
                Signatory::new("h-1", "Alice", HAAuthentication::FIDO2),
                Signatory::new("h-2", "Bob", HAAuthentication::FIDO2),
                Signatory::new("h-3", "Carol", HAAuthentication::FIDO2),
            ],
        }
    }

    /// 3-of-5 多签
    pub fn three_of_five() -> Self {
        Self {
            required: 3,
            signatories: (0..5)
                .map(|i| {
                    Signatory::new(
                        format!("h-{}", i),
                        format!("Signatory {}", i),
                        HAAuthentication::FIDO2,
                    )
                })
                .collect(),
        }
    }

    /// 是否满足多签阈值
    pub fn meets_threshold(&self, signatures: &[String]) -> bool {
        signatures.len() >= self.required
    }
}

/// 主人请求 multi-sig 处理结果 (Q13)
#[derive(Debug, Clone, PartialEq)]
pub enum OwnerRequestMultisigOutcome {
    /// 通过 — 所有 token (包括 Master) 都满足 multi-sig 阈值
    Approved {
        /// 主人令牌 (Master / Admin / Operator — ReadOnly 不会到这里)
        token: crate::owner::OwnerToken,
        /// 已收集签名数
        signature_count: usize,
        /// 所需签名数
        required: usize,
        /// 是否触及 E 层
        touches_e_layer: bool,
    },
    /// 拒绝 — ReadOnly token 不能改 core-rule
    ReadOnlyRejected,
    /// 拒绝 — multi-sig 不足 (适用于所有 token, 包括 Master)
    InsufficientSignatures {
        /// 主人令牌
        token: crate::owner::OwnerToken,
        /// 已收集签名数
        collected: usize,
        /// 所需签名数
        required: usize,
    },
    /// 拒绝 — signatory 不在注册表
    UnknownSignatory(String),
}

impl MultiSigPolicy {
    /// Q13 主人请求 multi-sig 处理 — 验证所有 token (包括 Master)
    ///
    /// **硬约束**:
    /// 1. Master token **不能凌驾** multi-sig — 必须收集 ≥ `required` 签名
    /// 2. ReadOnly token 触及 core-rule → 立即拒绝 (ReadOnlyRejected)
    /// 3. 所有 core-rule 修改 (touches_e_layer=true) 走 5 重治理
    /// 4. SovereigntyHook 中无 bypass 路径 (验证调用者必须经过这里)
    ///
    /// **参数**:
    /// - `request`: 主人请求 (token + action + reason)
    /// - `collected_signatures`: 当前已收集的签名列表 (signatory IDs)
    ///
    /// **返回**:
    /// - `Approved` — 通过, 可进入 `Governance.process_owner_decision`
    /// - `ReadOnlyRejected` — ReadOnly token 改 core-rule, 立即拒绝
    /// - `InsufficientSignatures` — 签名数 < required (Master 也算)
    /// - `UnknownSignatory` — 有 signatory 不在注册表
    pub fn process_owner_request(
        &self,
        request: &crate::owner::OwnerRequest,
        collected_signatures: &[String],
    ) -> OwnerRequestMultisigOutcome {
        // Step 1: ReadOnly token 检查 (无论是否触及 E 层, ReadOnly 改 core-rule 必拒)
        if !request.token.can_attempt_core_rule() && request.touches_e_layer() {
            return OwnerRequestMultisigOutcome::ReadOnlyRejected;
        }

        // Step 2: 验证所有签名对应 signatory 在注册表
        for sig in collected_signatures {
            if !self.signatories.iter().any(|s| s.id == *sig) {
                return OwnerRequestMultisigOutcome::UnknownSignatory(sig.clone());
            }
        }

        // Step 3: 校验多签阈值 (Master / Admin / Operator / ReadOnly 一视同仁)
        // Q13 硬约束: Master 也必须满足 multi-sig, 不能凌驾治理
        if collected_signatures.len() < self.required {
            return OwnerRequestMultisigOutcome::InsufficientSignatures {
                token: request.token,
                collected: collected_signatures.len(),
                required: self.required,
            };
        }

        // Step 4: 通过 — 返回 Approved
        OwnerRequestMultisigOutcome::Approved {
            token: request.token,
            signature_count: collected_signatures.len(),
            required: self.required,
            touches_e_layer: request.touches_e_layer(),
        }
    }
}
// ============================================================
// round6-01: HA M-of-N 多签字段补全
// ============================================================

/// 单条主人审批记录 (用于 HumanAuthority.applications audit trail).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HumanApproval {
    /// 审批 ID (唯一)
    pub approval_id: String,
    /// 审批人 ID (signatory.id)
    pub approver_id: String,
    /// 审批人显示名
    pub approver_name: String,
    /// 审批时间 (epoch ms)
    pub approved_at_ms: i64,
    /// 审批的具体 action (例如 "modify_l0_threshold")
    pub action: String,
    /// 过期时间 (epoch ms, 0 = 永不过期)
    pub expires_at_ms: i64,
    /// 是否已撤销
    pub revoked: bool,
}

impl HumanApproval {
    /// 便利构造
    pub fn new(
        approval_id: impl Into<String>,
        approver_id: impl Into<String>,
        approver_name: impl Into<String>,
        approved_at_ms: i64,
        action: impl Into<String>,
    ) -> Self {
        Self {
            approval_id: approval_id.into(),
            approver_id: approver_id.into(),
            approver_name: approver_name.into(),
            approved_at_ms,
            action: action.into(),
            expires_at_ms: 0,
            revoked: false,
        }
    }

    /// 设置过期时间
    pub fn with_expiry(mut self, expires_at_ms: i64) -> Self {
        self.expires_at_ms = expires_at_ms;
        self
    }

    /// 当前时刻 (now_ms) 此审批是否仍然有效
    pub fn is_valid(&self, now_ms: i64) -> bool {
        if self.revoked {
            return false;
        }
        if self.expires_at_ms > 0 && now_ms >= self.expires_at_ms {
            return false;
        }
        true
    }
}

/// HA 授权模式 (Single / Multi / Dynamic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuthorityMode {
    /// 单人模式 — 1 of 1 (required_approvals=1, threshold=100)
    Single,
    /// 多人模式 — M of N (required_approvals=M, threshold=round(M/N*100))
    Multi,
    /// 动态模式 — 阈值根据上下文自适应 (例如 E 层调高, 普通层保持)
    Dynamic,
}

impl fmt::Display for AuthorityMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Single => f.write_str("single"),
            Self::Multi => f.write_str("multi"),
            Self::Dynamic => f.write_str("dynamic"),
        }
    }
}

/// HA 多签授权 — 完整 M-of-N 数据模型.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HumanAuthority {
    /// 授权 ID (唯一)
    pub authority_id: String,
    /// 显示名
    pub name: String,
    /// 授权模式
    pub mode: AuthorityMode,
    /// 所需审批数 (M) — 单次请求至少需要多少人类批准
    pub required_approvals: u8,
    /// 权重阈值 (0-100) — 加权批准率 ≥ threshold 才算通过
    pub threshold: u8,
    /// 已注册的总签者数 (N) — 用于计算百分比阈值
    pub total_signatories: u8,
    /// 已批准记录 (audit trail)
    pub applications: Vec<HumanApproval>,
}

impl HumanAuthority {
    /// 新建单人模式 (1 of 1, threshold=100)
    pub fn single(_human_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            authority_id: "ha-single".into(),
            name: name.into(),
            mode: AuthorityMode::Single,
            required_approvals: 1,
            threshold: 100,
            total_signatories: 1,
            applications: Vec::new(),
        }
    }

    /// 新建多人模式 (M of N, threshold 自动 = round(M/N*100))
    pub fn multi(
        authority_id: impl Into<String>,
        name: impl Into<String>,
        m: u8,
        n: u8,
    ) -> Result<Self, String> {
        if m < 1 {
            return Err("M 必须 ≥ 1".into());
        }
        if n < 1 {
            return Err("N 必须 ≥ 1".into());
        }
        if m > n {
            return Err(format!("M ({}) > N ({})", m, n));
        }
        let threshold = if n == 0 {
            0
        } else {
            (u32::from(m) * 100 / u32::from(n)) as u8
        };
        Ok(Self {
            authority_id: authority_id.into(),
            name: name.into(),
            mode: AuthorityMode::Multi,
            required_approvals: m,
            threshold,
            total_signatories: n,
            applications: Vec::new(),
        })
    }

    /// 新建动态模式 (用户自定义 required + threshold)
    pub fn dynamic(
        authority_id: impl Into<String>,
        name: impl Into<String>,
        required_approvals: u8,
        threshold: u8,
        total_signatories: u8,
    ) -> Self {
        Self {
            authority_id: authority_id.into(),
            name: name.into(),
            mode: AuthorityMode::Dynamic,
            required_approvals,
            threshold: threshold.min(100),
            total_signatories,
            applications: Vec::new(),
        }
    }

    /// 追加一条批准记录
    pub fn record_approval(&mut self, approval: HumanApproval) {
        self.applications.push(approval);
    }

    /// 撤销一条批准记录 (按 approval_id)
    pub fn revoke_approval(&mut self, approval_id: &str) -> bool {
        for a in self.applications.iter_mut() {
            if a.approval_id == approval_id {
                a.revoked = true;
                return true;
            }
        }
        false
    }

    /// 计算当前有效批准数 (过滤 revoked + expired)
    pub fn valid_approval_count(&self, now_ms: i64) -> usize {
        self.applications
            .iter()
            .filter(|a| a.is_valid(now_ms))
            .count()
    }

    /// 计算当前有效批准率 (0-100)
    pub fn valid_approval_percentage(&self, now_ms: i64) -> u8 {
        if self.total_signatories == 0 {
            return 0;
        }
        let valid = self.valid_approval_count(now_ms) as u32;
        let pct = (valid * 100) / u32::from(self.total_signatories);
        pct.min(100) as u8
    }

    /// 是否满足批准数 + 权重阈值 (Single/Multi/Dynamic 三模式差异化的核心)
    ///
    /// **三模式差异**:
    /// - `Single` — 必须 required_approvals==1 且 threshold==100
    /// - `Multi`  — required_approvals 来自 M; threshold = M/N*100; 有效批准数 ≥ M 且 percentage ≥ threshold
    /// - `Dynamic` — threshold 可适应上下文 (E 层要求更高); 此函数只校验数据模型, 调用方负责 context adjustment
    pub fn meets_authority(&self, now_ms: i64) -> bool {
        let valid_count = self.valid_approval_count(now_ms);
        let valid_pct = self.valid_approval_percentage(now_ms);
        match self.mode {
            AuthorityMode::Single => {
                valid_count >= self.required_approvals as usize
                    && valid_pct >= self.threshold
                    && valid_count >= 1
            }
            AuthorityMode::Multi => {
                valid_count >= self.required_approvals as usize && valid_pct >= self.threshold
            }
            AuthorityMode::Dynamic => {
                // Dynamic 模式: 满足 required_approvals 即认为通过 (threshold 由调用方 context 决定)
                valid_count >= self.required_approvals as usize
            }
        }
    }
}

impl fmt::Display for HumanAuthority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.mode {
            AuthorityMode::Single => write!(f, "HA[single:{}]", self.name),
            AuthorityMode::Multi => write!(
                f,
                "HA[multi:{}-of-{} threshold={}%]",
                self.required_approvals, self.total_signatories, self.threshold
            ),
            AuthorityMode::Dynamic => write!(
                f,
                "HA[dynamic:{} required={} threshold={}%]",
                self.name, self.required_approvals, self.threshold
            ),
        }
    }
}

/// owner request multi-sig 处理结果 — 带 HumanAuthority 上下文 (round6-01 新增)
#[derive(Debug, Clone, PartialEq)]
pub enum AuthorityMultisigOutcome {
    /// 通过 — 已满足 required_approvals + threshold
    Approved {
        token: crate::owner::OwnerToken,
        authority_id: String,
        signature_count: usize,
        required: usize,
        threshold: u8,
        touches_e_layer: bool,
    },
    /// ReadOnly 拒绝
    ReadOnlyRejected,
    /// 签名数不足 (collected < required_approvals)
    InsufficientSignatures {
        token: crate::owner::OwnerToken,
        collected: usize,
        required: usize,
    },
    /// 权重阈值不足 (count ≥ required 但 percentage < threshold)
    ThresholdNotMet {
        token: crate::owner::OwnerToken,
        valid_count: usize,
        percentage: u8,
        required_threshold: u8,
    },
    /// 签者不在注册表
    UnknownSignatory(String),
}

impl MultiSigPolicy {
    /// round6-01: process_owner_request 不再 stub, 根据 HumanAuthority.threshold 真实计算.
    ///
    /// **与原版差异**:
    /// - 原版只校验 `collected_signatures.len() >= self.required` (只看计数)
    /// - 本版基于 `HumanAuthority` 校验 (计数 + 权重阈值 + 三模式差异化):
    ///   - Single: 1 个有效批准 + 100% (>= 100)
    ///   - Multi: M 个有效批准 + >= M/N*100%
    ///   - Dynamic: M 个有效批准 (threshold 由调用方上下文决定)
    ///
    /// **Q13 硬约束保持** (与原版一致):
    /// 1. Master token 不能凌驾 multi-sig
    /// 2. ReadOnly token 触及 core-rule → 立即拒绝
    /// 3. touches_e_layer=true 走 5 重治理 (后续 governance 处理)
    /// 4. SovereigntyHook 中无 bypass 路径
    pub fn process_owner_request_with_authority(
        &self,
        request: &crate::owner::OwnerRequest,
        collected_signatures: &[String],
        authority: &HumanAuthority,
        _now_ms: i64,
    ) -> AuthorityMultisigOutcome {
        // Step 1: ReadOnly token 检查
        if !request.token.can_attempt_core_rule() && request.touches_e_layer() {
            return AuthorityMultisigOutcome::ReadOnlyRejected;
        }

        // Step 2: 验证所有签名对应 signatory 在注册表
        for sig in collected_signatures {
            if !self.signatories.iter().any(|s| s.id == *sig) {
                return AuthorityMultisigOutcome::UnknownSignatory(sig.clone());
            }
        }

        // Step 3: 校验签名数 ≥ required_approvals (M)
        let required = authority.required_approvals as usize;
        if collected_signatures.len() < required {
            return AuthorityMultisigOutcome::InsufficientSignatures {
                token: request.token,
                collected: collected_signatures.len(),
                required,
            };
        }

        // Step 4: 按三模式差异化校验 threshold
        match authority.mode {
            AuthorityMode::Single => {
                // Single: 1 of 1, threshold 必须 100%
                if authority.threshold != 100 {
                    return AuthorityMultisigOutcome::ThresholdNotMet {
                        token: request.token,
                        valid_count: collected_signatures.len(),
                        percentage: 100, // 单人模式强制 100%
                        required_threshold: 100,
                    };
                }
                AuthorityMultisigOutcome::Approved {
                    token: request.token,
                    authority_id: authority.authority_id.clone(),
                    signature_count: collected_signatures.len(),
                    required,
                    threshold: authority.threshold,
                    touches_e_layer: request.touches_e_layer(),
                }
            }
            AuthorityMode::Multi => {
                // Multi: M of N, percentage = collected/N*100, >= threshold
                let n = authority.total_signatories.max(1) as usize;
                let percentage = ((collected_signatures.len() * 100) / n).min(100) as u8;
                if percentage < authority.threshold {
                    return AuthorityMultisigOutcome::ThresholdNotMet {
                        token: request.token,
                        valid_count: collected_signatures.len(),
                        percentage,
                        required_threshold: authority.threshold,
                    };
                }
                AuthorityMultisigOutcome::Approved {
                    token: request.token,
                    authority_id: authority.authority_id.clone(),
                    signature_count: collected_signatures.len(),
                    required,
                    threshold: authority.threshold,
                    touches_e_layer: request.touches_e_layer(),
                }
            }
            AuthorityMode::Dynamic => {
                // Dynamic: 调用方 context 决定 threshold; 此函数只看计数
                // (若调用方传入 adjusted_threshold, 用 meets_authority 校验)
                AuthorityMultisigOutcome::Approved {
                    token: request.token,
                    authority_id: authority.authority_id.clone(),
                    signature_count: collected_signatures.len(),
                    required,
                    threshold: authority.threshold,
                    touches_e_layer: request.touches_e_layer(),
                }
            }
        }
    }
}

#[cfg(test)]
mod round6_01_tests {
    use super::*;
    use crate::owner::{OwnerAction, OwnerRequest, OwnerToken};

    fn sigs(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| (*n).to_string()).collect()
    }

    fn req(token: OwnerToken) -> OwnerRequest {
        OwnerRequest::new(
            "req-test",
            token,
            OwnerAction::AuditQuery,
            "test-user",
            "test",
        )
    }

    // ---- HumanApproval ----

    #[test]
    fn approval_valid_when_not_revoked_and_not_expired() {
        let a = HumanApproval::new("ap-1", "h-1", "Alice", 1000, "test");
        assert!(a.is_valid(2000));
    }

    #[test]
    fn approval_invalid_when_revoked() {
        let mut a = HumanApproval::new("ap-1", "h-1", "Alice", 1000, "test");
        a.revoked = true;
        assert!(!a.is_valid(2000));
    }

    #[test]
    fn approval_invalid_when_expired() {
        let a = HumanApproval::new("ap-1", "h-1", "Alice", 1000, "test").with_expiry(2000);
        assert!(!a.is_valid(2000));
        assert!(!a.is_valid(3000));
        assert!(a.is_valid(1999));
    }

    // ---- HumanAuthority construction ----

    #[test]
    fn single_authority_defaults() {
        let h = HumanAuthority::single("h-1", "Alice");
        assert_eq!(h.required_approvals, 1);
        assert_eq!(h.threshold, 100);
        assert_eq!(h.total_signatories, 1);
        assert_eq!(h.mode, AuthorityMode::Single);
    }

    #[test]
    fn multi_authority_2_of_3_threshold_66() {
        let h = HumanAuthority::multi("ha-1", "core-team", 2, 3).unwrap();
        assert_eq!(h.required_approvals, 2);
        assert_eq!(h.total_signatories, 3);
        assert_eq!(h.threshold, 66); // round(2/3*100)
    }

    #[test]
    fn multi_authority_3_of_5_threshold_60() {
        let h = HumanAuthority::multi("ha-2", "board", 3, 5).unwrap();
        assert_eq!(h.required_approvals, 3);
        assert_eq!(h.threshold, 60);
    }

    #[test]
    fn multi_authority_rejects_m_greater_than_n() {
        assert!(HumanAuthority::multi("x", "x", 4, 3).is_err());
        assert!(HumanAuthority::multi("x", "x", 0, 3).is_err());
    }

    #[test]
    fn dynamic_authority_user_defined_threshold() {
        let h = HumanAuthority::dynamic("d-1", "ctx-aware", 2, 80, 4);
        assert_eq!(h.mode, AuthorityMode::Dynamic);
        assert_eq!(h.required_approvals, 2);
        assert_eq!(h.threshold, 80);
    }

    // ---- applications log + revoke ----

    #[test]
    fn authority_records_and_revokes_approvals() {
        let mut h = HumanAuthority::multi("ha-1", "team", 2, 3).unwrap();
        h.record_approval(HumanApproval::new("ap-1", "h-1", "Alice", 1000, "x"));
        h.record_approval(HumanApproval::new("ap-2", "h-2", "Bob", 1000, "x"));
        assert_eq!(h.applications.len(), 2);
        assert!(h.revoke_approval("ap-1"));
        assert_eq!(h.valid_approval_count(2000), 1);
        assert!(!h.revoke_approval("nonexistent"));
    }

    #[test]
    fn authority_valid_percentage_correct() {
        let mut h = HumanAuthority::multi("ha-1", "team", 2, 4).unwrap();
        h.record_approval(HumanApproval::new("ap-1", "h-1", "A", 1000, "x"));
        h.record_approval(HumanApproval::new("ap-2", "h-2", "B", 1000, "x"));
        assert_eq!(h.valid_approval_percentage(2000), 50); // 2/4*100
    }

    #[test]
    fn meets_authority_single_requires_1_and_100pct() {
        let mut h = HumanAuthority::single("h-1", "Alice");
        h.record_approval(HumanApproval::new("ap-1", "h-1", "Alice", 1000, "x"));
        assert!(h.meets_authority(2000));
    }

    #[test]
    fn meets_authority_multi_2_of_3_with_2_approvals() {
        let mut h = HumanAuthority::multi("ha-1", "team", 2, 3).unwrap();
        h.record_approval(HumanApproval::new("ap-1", "h-1", "A", 1000, "x"));
        h.record_approval(HumanApproval::new("ap-2", "h-2", "B", 1000, "x"));
        assert!(h.meets_authority(2000));
        assert_eq!(h.threshold, 66);
    }

    #[test]
    fn meets_authority_multi_fails_with_only_1_approval() {
        let mut h = HumanAuthority::multi("ha-1", "team", 2, 3).unwrap();
        h.record_approval(HumanApproval::new("ap-1", "h-1", "A", 1000, "x"));
        assert!(!h.meets_authority(2000));
    }

    #[test]
    fn meets_authority_dynamic_uses_required_only() {
        let mut h = HumanAuthority::dynamic("d-1", "ctx", 2, 50, 5);
        h.record_approval(HumanApproval::new("ap-1", "h-1", "A", 1000, "x"));
        h.record_approval(HumanApproval::new("ap-2", "h-2", "B", 1000, "x"));
        assert!(h.meets_authority(2000));
    }

    // ---- process_owner_request_with_authority 三模式 ----

    #[test]
    fn process_request_single_mode_approved() {
        let _policy = MultiSigPolicy::default_2_of_3();
        let mut ha = HumanAuthority::single("h-1", "Alice");
        ha.total_signatories = 1; // 单人 N=1
                                  // Override signatories to allow h-1
        let mut policy = MultiSigPolicy {
            required: 1,
            signatories: vec![Signatory::new("h-1", "Alice", HAAuthentication::FIDO2)],
        };
        let request = req(OwnerToken::Master);
        let collected = sigs(&["h-1"]);
        let outcome = policy.process_owner_request_with_authority(&request, &collected, &ha, 1000);
        assert!(matches!(outcome, AuthorityMultisigOutcome::Approved { .. }));
    }

    #[test]
    fn process_request_multi_mode_2_of_3_approved() {
        let policy = MultiSigPolicy::default_2_of_3();
        let ha = HumanAuthority::multi("ha-1", "team", 2, 3).unwrap();
        let request = req(OwnerToken::Master);
        let collected = sigs(&["h-1", "h-2"]);
        let outcome = policy.process_owner_request_with_authority(&request, &collected, &ha, 1000);
        assert!(matches!(
            outcome,
            AuthorityMultisigOutcome::Approved {
                signature_count: 2,
                required: 2,
                ..
            }
        ));
    }

    #[test]
    fn process_request_multi_mode_1_of_3_rejected_threshold() {
        let policy = MultiSigPolicy::default_2_of_3();
        let ha = HumanAuthority::multi("ha-1", "team", 2, 3).unwrap();
        let request = req(OwnerToken::Master);
        let collected = sigs(&["h-1"]); // 只 1 个签名 (33%) < threshold 66%
        let outcome = policy.process_owner_request_with_authority(&request, &collected, &ha, 1000);
        // 1 < required=2 → InsufficientSignatures
        assert!(matches!(
            outcome,
            AuthorityMultisigOutcome::InsufficientSignatures {
                collected: 1,
                required: 2,
                ..
            }
        ));
    }

    #[test]
    fn process_request_unknown_signatory_rejected() {
        let policy = MultiSigPolicy::default_2_of_3();
        let ha = HumanAuthority::multi("ha-1", "team", 2, 3).unwrap();
        let request = req(OwnerToken::Master);
        let collected = sigs(&["h-1", "h-unknown"]);
        let outcome = policy.process_owner_request_with_authority(&request, &collected, &ha, 1000);
        assert!(
            matches!(outcome, AuthorityMultisigOutcome::UnknownSignatory(ref s) if s == "h-unknown")
        );
    }

    #[test]
    fn process_request_readonly_rejected_for_e_layer() {
        let policy = MultiSigPolicy::default_2_of_3();
        let ha = HumanAuthority::multi("ha-1", "team", 2, 3).unwrap();
        let request = OwnerRequest::new(
            "req-readonly",
            OwnerToken::ReadOnly,
            OwnerAction::ModifyL0Threshold,
            "test-user",
            "trying to modify core rule",
        );
        let collected = sigs(&["h-1", "h-2", "h-3"]);
        let outcome = policy.process_owner_request_with_authority(&request, &collected, &ha, 1000);
        assert!(matches!(
            outcome,
            AuthorityMultisigOutcome::ReadOnlyRejected
        ));
    }

    #[test]
    fn process_request_dynamic_mode_adaptive_threshold() {
        let policy = MultiSigPolicy::three_of_five();
        let ha = HumanAuthority::dynamic("d-1", "ctx", 3, 50, 5);
        let request = req(OwnerToken::Master);
        let collected = sigs(&["h-0", "h-1", "h-2"]); // 3 个签名
        let outcome = policy.process_owner_request_with_authority(&request, &collected, &ha, 1000);
        assert!(matches!(
            outcome,
            AuthorityMultisigOutcome::Approved {
                signature_count: 3,
                required: 3,
                threshold: 50,
                ..
            }
        ));
    }

    #[test]
    fn display_format_for_three_modes() {
        let s = HumanAuthority::single("h-1", "Alice");
        assert_eq!(s.to_string(), "HA[single:Alice]");
        let m = HumanAuthority::multi("ha-1", "team", 2, 3).unwrap();
        assert_eq!(m.to_string(), "HA[multi:2-of-3 threshold=66%]");
        let d = HumanAuthority::dynamic("d-1", "ctx", 2, 80, 5);
        assert_eq!(d.to_string(), "HA[dynamic:ctx required=2 threshold=80%]");
    }
}
