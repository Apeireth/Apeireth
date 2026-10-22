//! immune_memory —— 安全事件免疫记忆与炎症收口内核 v0（生物机制族 W1，2026-10-19）。
//!
//! # 生物原型
//!
//! 先天免疫快而不特异：看到可疑就立刻压制，但不留记忆；适应免疫慢而特异：识别具体抗原、
//! 建立记忆，二次暴露时反应更快更轻。免疫系统同时要做 self / non-self 判别，对无害的自我
//! 成分保持耐受，避免「狼来了」式的长期全身炎症；炎症本身也不是二值开关，而是分级
//! （静默 / 记录 / 守备 / 封锁）且必须能收口（resolve），否则免疫系统会一直空转，把正常
//! 流量也当成敌情。
//!
//! 本模块把这四件事压成一个纯内核：
//!
//! - self / non-self 判别：`is_self_origin` 且提示严重度非高时走耐受，不建立抗原记忆；
//! - 先天分级：新签名按调用方给的 `severity_hint` 分级，高严重度再升一级；
//! - 适应记忆：同签名第二次起降一级并抑制升级（`tolerance_level` 累积）；
//! - 炎症收口：`resolve` 关闭事件并进入冷却窗，窗内同签名只静默计数，窗外复发重新开启。
//!
//! # 工程边界（能力等级：IMPLEMENTED，未接线）
//!
//! - **IMPLEMENTED 未接线**：本文件只有内核类型与单元测试；未挂 guard hook（`crate::hook`
//!   无调用点）、未进治理链（无 `GovernanceHook` 实现）、无自述节（不向 introspection 上报），
//!   也没有任何装配或默认开关。调用方要显式持有 `ImmuneMemory` 才会生效。
//! - **不替代审批**：免疫记忆只做响应分级与教训复用，不能放行或阻断任何动作；真正的放行 /
//!   阻断仍由审批与治理链决定。记忆只能让「已知模式」的响应更轻，「轻」不等于「允许」。
//! - **不跨进程持久化**：`InMemoryImmuneStore` 是进程内存储，容量上限固定，进程重启即遗忘；
//!   跨会话记忆需要另接持久化 store（本文件不提供）。
//! - **归一化不是万能**：它只能消除数字、绝对路径目录、零宽字符、大小写这类实例噪声；若随机性
//!   落在末段文件名或主机名里，归一化无法消除，此时调用方应传入检测器族的稳定键（例如规则 id），
//!   而不是原始文件路径。
//! - **时钟注入**：所有时间都由调用方的 `now_ms` 注入，内核不读系统时钟，因此行为可复现；
//!   时钟回拨时按「仍在冷却期」保守处理。
//!
//! # 签名归一化规则
//!
//! `normalize(kind, raw_key)` 对原始键依次做：
//!
//! 1. 全量小写（Unicode `to_lowercase`）；
//! 2. 删除零宽字符（U+200B / U+200C / U+200D / U+2060 / U+FEFF / U+180E）；
//! 3. 删除所有 ASCII 数字（编号、时间戳、随机后缀等可变段）；
//! 4. 按空白切 token：URL 只保留 host（`url:<host>`）；绝对路径 / 含分隔符的路径丢弃目录段、
//!    只保留末段（`path:<basename>`）；其余 token 原样保留；
//! 5. 以单个空格拼回 canonical 串，做 SHA-256，取前 16 个 hex 字符作为 `normalized_key`。
//!
//! 同类事件的不同实例（大小写、零宽字符、编号、上传目录不同）因此得到同一签名；不同 `kind`
//! 或不同语义形态得到不同签名。
//!
//! # 响应分级与收口规则
//!
//! | 场景 | 响应 tier | escalated |
//! | --- | --- | --- |
//! | 新签名，`severity_hint >= Tier2Guard` | `hint.raise()` | true |
//! | 新签名，`severity_hint < Tier2Guard` | `hint` | false |
//! | 已知签名（Open，第二次起） | `hint.lower()` | false，且 `tolerance_level += 1` |
//! | `is_self_origin` 且 hint 非高 | `Tier0Quiet` | false，不记录 |
//! | 已收口且在冷却窗内 | `Tier0Quiet` | false，只计数 |
//! | 已收口且冷却窗已过（复发） | 高 hint 则 `hint.raise()`，否则 `max(hint, Tier1Note)` | 高 hint 时为 true |
//!
//! `resolve` 把记录置为 `Resolved` 并从 `now_ms` 起开启 `cooldown_ms` 冷却窗；`reopen` 是
//! 操作者的显式撤销，恢复 `Open` 并清空冷却窗与耐受度。
//!
//! # W2 接线点（本文件不实现）
//!
//! 在 guard hook 的判定出口调用 `observe`，把 `ImmuneResponse.tier` 并入既有决策作为分级输入；
//! 不改变审批结论本身。
//!
//! # 确定性
//!
//! 无随机数、无系统时钟、无浮点；全部输入（含 `now_ms`）显式注入。容量淘汰按
//! `last_seen_ms` 升序，同刻用 `(kind, normalized_key)` 全序打破平局，与 `HashMap` 迭代顺序无关。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// `InMemoryImmuneStore` 的默认容量上限（条记录）。
pub const DEFAULT_CAPACITY: usize = 256;

/// 默认炎症冷却窗：6 小时（毫秒）。收口后窗内同签名响应降为 `Tier0Quiet`。
pub const DEFAULT_COOLDOWN_MS: i64 = 6 * 60 * 60 * 1000;

/// 安全事件类别（抗原大类）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum IncidentKind {
    /// 提示注入 / 指令劫持。
    PromptInjection,
    /// 凭据泄露。
    CredentialLeak,
    /// 不可信内容越界（不可信信封内容逃逸到可信上下文）。
    UntrustedContentEscape,
    /// 工具投毒（工具描述 / 参数被污染）。
    ToolPoison,
    /// 出网异常（目标、频次或载荷越界）。
    EgressAnomaly,
    /// 未归类事件：仍参与记忆，但不假设其语义。
    Unknown,
}

impl IncidentKind {
    /// 稳定的字符串标识，用于记录键前缀与跨进程审计。
    pub const fn as_str(self) -> &'static str {
        match self {
            IncidentKind::PromptInjection => "prompt_injection",
            IncidentKind::CredentialLeak => "credential_leak",
            IncidentKind::UntrustedContentEscape => "untrusted_content_escape",
            IncidentKind::ToolPoison => "tool_poison",
            IncidentKind::EgressAnomaly => "egress_anomaly",
            IncidentKind::Unknown => "unknown",
        }
    }
}

/// 炎症分级（响应强度）。
///
/// 顺序即强度：`Tier0Quiet < Tier1Note < Tier2Guard < Tier3Lockdown`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SeverityTier {
    /// 静默：仅计数 / 不打扰。
    Tier0Quiet,
    /// 记录：写审计，不触发额外动作。
    Tier1Note,
    /// 守备：加强校验 / 提高采样。
    Tier2Guard,
    /// 封锁：最强响应，需人工介入。
    Tier3Lockdown,
}

impl SeverityTier {
    /// 强度秩：0..=3。
    pub const fn rank(self) -> u8 {
        match self {
            SeverityTier::Tier0Quiet => 0,
            SeverityTier::Tier1Note => 1,
            SeverityTier::Tier2Guard => 2,
            SeverityTier::Tier3Lockdown => 3,
        }
    }

    /// 是否高严重度（`>= Tier2Guard`）。self 耐受只对非高严重度生效。
    pub const fn is_high(self) -> bool {
        self.rank() >= SeverityTier::Tier2Guard.rank()
    }

    /// 升一级（已到顶则保持 `Tier3Lockdown`）。
    pub const fn raise(self) -> SeverityTier {
        match self {
            SeverityTier::Tier0Quiet => SeverityTier::Tier1Note,
            SeverityTier::Tier1Note => SeverityTier::Tier2Guard,
            SeverityTier::Tier2Guard => SeverityTier::Tier3Lockdown,
            SeverityTier::Tier3Lockdown => SeverityTier::Tier3Lockdown,
        }
    }

    /// 降一级（已到底则保持 `Tier0Quiet`）。
    pub const fn lower(self) -> SeverityTier {
        match self {
            SeverityTier::Tier0Quiet => SeverityTier::Tier0Quiet,
            SeverityTier::Tier1Note => SeverityTier::Tier0Quiet,
            SeverityTier::Tier2Guard => SeverityTier::Tier1Note,
            SeverityTier::Tier3Lockdown => SeverityTier::Tier2Guard,
        }
    }
}

/// 事件状态。`Resolved` 表示已收口（进入冷却窗），`Open` 表示仍在观察。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IncidentStatus {
    /// 未收口。
    Open,
    /// 已收口。
    Resolved,
}

/// 抗原签名：`kind` + 归一化键。
///
/// 由 [`normalize`] 生成。`kind` 参与 `Hash` / `Eq`，因此同 key 不同类不会互相覆盖记忆。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct IncidentSignature {
    /// 事件类别。
    pub kind: IncidentKind,
    /// SHA-256 前 16 hex 的归一化键。
    pub normalized_key: String,
}

impl IncidentSignature {
    /// 记录键，形如 `prompt_injection:1a2b3c4d5e6f7a8b`。
    pub fn record_key(&self) -> String {
        format!("{}:{}", self.kind.as_str(), self.normalized_key)
    }
}

/// 免疫记忆中的一条事件记录。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncidentRecord {
    /// 抗原签名（与存储键一致）。
    pub signature: IncidentSignature,
    /// 事件类别（冗余保存，便于记录脱离签名单独消费）。
    pub kind: IncidentKind,
    /// 最近一次响应分级。
    pub tier: SeverityTier,
    /// 首次观察时间（毫秒，调用方注入）。
    pub first_seen_ms: i64,
    /// 最近观察时间（毫秒，调用方注入；容量淘汰按此字段排序）。
    pub last_seen_ms: i64,
    /// 累计观察次数（含冷却期内的静默计数）。
    pub occurrences: u64,
    /// 事件状态。
    pub status: IncidentStatus,
    /// 耐受等级：同签名每多观察一次 +1（饱和于 `u8::MAX`），是「已知模式」的强度证据。
    pub tolerance_level: u8,
}

/// 一次观察的响应分级结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImmuneResponse {
    /// 建议的响应分级。
    pub tier: SeverityTier,
    /// 是否升级（仅新签名高严重度与冷却后高严重度复发为 true）。
    pub escalated: bool,
    /// 分级理由（中文，直接用于审计 / 可观测）。
    pub reason: String,
    /// 对应的记录键；即使因 self 耐受未记录也会返回，便于审计关联。
    pub record_key: String,
}

/// 免疫记忆存储抽象（`get` / `put` / `list`）。
///
/// 内核只依赖这三个读接口和 `len`，不假设存储介质；持久化实现可由调用方在 W2 之后注入。
pub trait ImmuneMemoryStore {
    /// 读取一条记录（不更新访问时间）。
    fn get(&self, signature: &IncidentSignature) -> Option<&IncidentRecord>;
    /// 写入 / 覆盖一条记录，返回被覆盖的旧值。
    fn put(
        &mut self,
        signature: IncidentSignature,
        record: IncidentRecord,
    ) -> Option<IncidentRecord>;
    /// 列出全部记录（顺序由实现定义，应保持确定性）。
    fn list(&self) -> Vec<(IncidentSignature, IncidentRecord)>;
    /// 当前记录条数。
    fn len(&self) -> usize;
    /// 是否为空。
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// 进程内免疫记忆存储：固定容量上限，超限淘汰 `last_seen_ms` 最旧者（近似 LRU）。
#[derive(Debug, Clone)]
pub struct InMemoryImmuneStore {
    records: HashMap<IncidentSignature, IncidentRecord>,
    capacity: usize,
}

impl InMemoryImmuneStore {
    /// 以给定容量建立存储；容量为 0 时按 1 处理。
    pub fn new(capacity: usize) -> Self {
        Self {
            records: HashMap::new(),
            capacity: capacity.max(1),
        }
    }

    /// 容量上限。
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    fn evict_until_within_capacity(&mut self) {
        while self.records.len() > self.capacity {
            let victim = self
                .records
                .iter()
                .min_by(|(sig_a, rec_a), (sig_b, rec_b)| {
                    rec_a
                        .last_seen_ms
                        .cmp(&rec_b.last_seen_ms)
                        .then_with(|| sig_a.kind.cmp(&sig_b.kind))
                        .then_with(|| sig_a.normalized_key.cmp(&sig_b.normalized_key))
                })
                .map(|(signature, _)| signature.clone());
            let Some(victim) = victim else { break };
            self.records.remove(&victim);
        }
    }
}

impl ImmuneMemoryStore for InMemoryImmuneStore {
    fn get(&self, signature: &IncidentSignature) -> Option<&IncidentRecord> {
        self.records.get(signature)
    }

    fn put(
        &mut self,
        signature: IncidentSignature,
        record: IncidentRecord,
    ) -> Option<IncidentRecord> {
        let previous = self.records.insert(signature, record);
        self.evict_until_within_capacity();
        previous
    }

    fn list(&self) -> Vec<(IncidentSignature, IncidentRecord)> {
        let mut items: Vec<(IncidentSignature, IncidentRecord)> = self
            .records
            .iter()
            .map(|(signature, record)| (signature.clone(), record.clone()))
            .collect();
        items.sort_by(|(sig_a, _), (sig_b, _)| {
            sig_a
                .kind
                .cmp(&sig_b.kind)
                .then_with(|| sig_a.normalized_key.cmp(&sig_b.normalized_key))
        });
        items
    }

    fn len(&self) -> usize {
        self.records.len()
    }
}

/// 免疫记忆内核：适应记忆 + self 耐受 + 炎症分级与收口。
///
/// 泛型参数 `S` 是存储后端，默认 [`InMemoryImmuneStore`]；所有时间由调用方注入。
#[derive(Debug, Clone)]
pub struct ImmuneMemory<S: ImmuneMemoryStore = InMemoryImmuneStore> {
    store: S,
    cooldown_ms: i64,
    resolved_at: HashMap<IncidentSignature, i64>,
}

impl ImmuneMemory<InMemoryImmuneStore> {
    /// 以默认容量 [`DEFAULT_CAPACITY`] 建立进程内记忆。
    pub fn with_capacity(capacity: usize) -> Self {
        Self::new(InMemoryImmuneStore::new(capacity))
    }
}

impl<S: ImmuneMemoryStore> ImmuneMemory<S> {
    /// 以默认冷却窗 [`DEFAULT_COOLDOWN_MS`] 包裹一个存储后端。
    pub fn new(store: S) -> Self {
        Self::with_cooldown_ms(store, DEFAULT_COOLDOWN_MS)
    }

    /// 指定冷却窗（毫秒）包裹存储后端；负值按 0 处理（等价于无冷却）。
    pub fn with_cooldown_ms(store: S, cooldown_ms: i64) -> Self {
        Self {
            store,
            cooldown_ms: cooldown_ms.max(0),
            resolved_at: HashMap::new(),
        }
    }

    /// 冷却窗（毫秒）。
    pub fn cooldown_ms(&self) -> i64 {
        self.cooldown_ms
    }

    /// 只读访问存储后端。
    pub fn store(&self) -> &S {
        &self.store
    }

    /// 读取一条事件记录（拷贝）。
    pub fn record(&self, signature: &IncidentSignature) -> Option<IncidentRecord> {
        self.store.get(signature).cloned()
    }

    /// 观察一次安全事件，返回响应分级。
    ///
    /// 参数：
    /// - `kind`：事件类别；
    /// - `raw_key`：原始键（规则 id、文件路径、目标等），经 [`normalize`] 归一化；
    /// - `severity_hint`：上游检测器给的初始严重度；
    /// - `is_self_origin`：事件是否源于 self（本机 / 自身进程 / 已授权内部路径）；
    /// - `now_ms`：调用方注入的时间（毫秒），内核不读系统时钟。
    ///
    /// 分级规则见模块文档表格。self 耐受不写记录；冷却期内的重复只计数不升级。
    pub fn observe(
        &mut self,
        kind: IncidentKind,
        raw_key: &str,
        severity_hint: SeverityTier,
        is_self_origin: bool,
        now_ms: i64,
    ) -> ImmuneResponse {
        let signature = normalize(kind, raw_key);
        let record_key = signature.record_key();
        let high = severity_hint.is_high();

        if is_self_origin && !high {
            return ImmuneResponse {
                tier: SeverityTier::Tier0Quiet,
                escalated: false,
                reason: "自我耐受：来源为 self 且提示严重度非高，不建立抗原记忆".to_owned(),
                record_key,
            };
        }

        if let Some(mut record) = self.store.get(&signature).cloned() {
            if record.status == IncidentStatus::Resolved {
                if self.within_cooldown(&signature, now_ms) {
                    record.occurrences = record.occurrences.saturating_add(1);
                    record.last_seen_ms = now_ms;
                    self.store.put(signature, record);
                    return ImmuneResponse {
                        tier: SeverityTier::Tier0Quiet,
                        escalated: false,
                        reason: "冷却期：同签名已收口且冷却窗未过，降为 Tier0 静默计数".to_owned(),
                        record_key,
                    };
                }

                // 冷却窗已过：复发，重新开启事件。
                self.resolved_at.remove(&signature);
                record.status = IncidentStatus::Open;
                record.occurrences = record.occurrences.saturating_add(1);
                record.last_seen_ms = now_ms;
                record.tolerance_level = 0;
                let tier = if high {
                    severity_hint.raise()
                } else {
                    severity_hint.max(SeverityTier::Tier1Note)
                };
                record.tier = tier;
                self.store.put(signature, record);
                return ImmuneResponse {
                    tier,
                    escalated: high,
                    reason: "冷却期结束：同签名复发，重新开启事件".to_owned(),
                    record_key,
                };
            }

            // 已知模式（Open）：适应记忆，降一级且不升级。
            record.occurrences = record.occurrences.saturating_add(1);
            record.last_seen_ms = now_ms;
            record.tolerance_level = record.tolerance_level.saturating_add(1);
            let tier = severity_hint.lower();
            record.tier = tier;
            let occurrences = record.occurrences;
            self.store.put(signature, record);
            return ImmuneResponse {
                tier,
                escalated: false,
                reason: format!(
                    "已知模式（第 {occurrences} 次）：适应记忆生效，响应下调一级且不升级"
                ),
                record_key,
            };
        }

        // 新签名：先天分级，高严重度升一级并标记升级。
        let tier = if high {
            severity_hint.raise()
        } else {
            severity_hint
        };
        let record = IncidentRecord {
            signature: signature.clone(),
            kind,
            tier,
            first_seen_ms: now_ms,
            last_seen_ms: now_ms,
            occurrences: 1,
            status: IncidentStatus::Open,
            tolerance_level: 0,
        };
        self.store.put(signature, record);
        ImmuneResponse {
            tier,
            escalated: high,
            reason: if high {
                "新签名 + 高严重度：建立抗原记忆并升级响应".to_owned()
            } else {
                "新签名：按提示分级建立抗原记忆".to_owned()
            },
            record_key,
        }
    }

    /// 收口一条事件：状态置 `Resolved` 并从 `now_ms` 起开启冷却窗。
    ///
    /// 返回是否找到了对应记录；对已收口的记录再次调用会刷新冷却起点。
    pub fn resolve(&mut self, signature: &IncidentSignature, now_ms: i64) -> bool {
        let Some(mut record) = self.store.get(signature).cloned() else {
            return false;
        };
        record.status = IncidentStatus::Resolved;
        self.store.put(signature.clone(), record);
        self.resolved_at.insert(signature.clone(), now_ms);
        self.prune_cooldowns(now_ms);
        true
    }

    /// 显式撤销收口：恢复 `Open`、清空冷却窗、把耐受度归零，重新开始适应。
    ///
    /// 返回是否真的从 `Resolved` 状态重开；记录不存在或本就 `Open` 时返回 false。
    pub fn reopen(&mut self, signature: &IncidentSignature) -> bool {
        let Some(mut record) = self.store.get(signature).cloned() else {
            return false;
        };
        let was_resolved = record.status == IncidentStatus::Resolved;
        record.status = IncidentStatus::Open;
        record.tolerance_level = 0;
        self.store.put(signature.clone(), record);
        self.resolved_at.remove(signature);
        was_resolved
    }

    /// 未收口（`Open`）的事件记录数。
    pub fn open_count(&self) -> usize {
        self.store
            .list()
            .iter()
            .filter(|(_, record)| record.status == IncidentStatus::Open)
            .count()
    }

    /// 已建立适应记忆的模式数：观察次数 `>= 2` 的签名数（含已收口但仍在记忆中的模式）。
    pub fn known_patterns(&self) -> usize {
        self.store
            .list()
            .iter()
            .filter(|(_, record)| record.occurrences >= 2)
            .count()
    }

    fn within_cooldown(&self, signature: &IncidentSignature, now_ms: i64) -> bool {
        self.resolved_at
            .get(signature)
            .is_some_and(|resolved_at| now_ms.saturating_sub(*resolved_at) < self.cooldown_ms)
    }

    fn prune_cooldowns(&mut self, now_ms: i64) {
        let cooldown_ms = self.cooldown_ms;
        self.resolved_at
            .retain(|_, resolved_at| now_ms.saturating_sub(*resolved_at) < cooldown_ms);
    }
}

/// 把 `(kind, raw_key)` 归一化为抗原签名。
///
/// 规则见模块文档「签名归一化规则」。同类事件的不同实例得到同一签名；不同 `kind` 即使
/// `normalized_key` 相同也视为不同签名。
pub fn normalize(kind: IncidentKind, raw_key: &str) -> IncidentSignature {
    let lowered = raw_key.to_lowercase();
    let mut stripped = String::with_capacity(lowered.len());
    for character in lowered.chars() {
        if !character.is_ascii_digit() && !is_zero_width(character) {
            stripped.push(character);
        }
    }

    let mut canonical = String::with_capacity(stripped.len());
    for token in stripped.split_whitespace() {
        if !canonical.is_empty() {
            canonical.push(' ');
        }
        canonical.push_str(&canonicalize_token(token));
    }

    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let digest = sha256_hex(&hasher.finalize());
    let normalized_key = digest.chars().take(16).collect();
    IncidentSignature {
        kind,
        normalized_key,
    }
}

fn is_zero_width(character: char) -> bool {
    matches!(
        character,
        '\u{200b}' | '\u{200c}' | '\u{200d}' | '\u{2060}' | '\u{feff}' | '\u{180e}'
    )
}

fn canonicalize_token(token: &str) -> String {
    if let Some(rest) = token
        .strip_prefix("https://")
        .or_else(|| token.strip_prefix("http://"))
    {
        let host = rest
            .split(|character| character == '/' || character == '?' || character == '#')
            .next()
            .unwrap_or(rest);
        return format!("url:{host}");
    }

    if is_path_like(token) {
        let basename = token
            .rsplit(|character| character == '/' || character == '\\')
            .next()
            .unwrap_or(token);
        if basename.is_empty() {
            return "path:root".to_owned();
        }
        return format!("path:{basename}");
    }

    token.to_owned()
}

fn is_path_like(token: &str) -> bool {
    token.contains('/') || token.contains('\\') || token.starts_with('~') || looks_like_drive(token)
}

fn looks_like_drive(token: &str) -> bool {
    let mut characters = token.chars();
    matches!(
        (characters.next(), characters.next()),
        (Some(letter), Some(':')) if letter.is_ascii_alphabetic()
    )
}

fn sha256_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_memory(capacity: usize, cooldown_ms: i64) -> ImmuneMemory {
        ImmuneMemory::with_cooldown_ms(InMemoryImmuneStore::new(capacity), cooldown_ms)
    }

    #[test]
    fn normalize_same_kind_different_instances_is_identical() {
        let kind = IncidentKind::CredentialLeak;
        let variants = [
            "CredentialLeak at /home/alice/.aws/creds.txt#123",
            "credentialleak at C:\\Users\\bob\\.aws\\creds.txt#456",
            "credential\u{200b}leak at /var/tmp/run-9/creds.txt#789",
            "CREDENTIALLEAK AT /mnt/backup/CREDS.TXT#42",
        ];
        let first = normalize(kind, variants[0]);
        for variant in &variants[1..] {
            assert_eq!(
                normalize(kind, variant),
                first,
                "instance survived normalization: {variant}"
            );
        }
        assert_eq!(first.normalized_key.len(), 16);
        assert!(first
            .normalized_key
            .chars()
            .all(|character| character.is_ascii_hexdigit()));
    }

    #[test]
    fn normalize_distinct_kinds_and_shapes_differ() {
        let same_key = "tool_poison: shell rm -rf /tmp/x";
        let prompt = normalize(IncidentKind::PromptInjection, same_key);
        let tool = normalize(IncidentKind::ToolPoison, same_key);
        assert_ne!(prompt, tool, "kind must participate in signature identity");
        assert_eq!(prompt.normalized_key, tool.normalized_key);

        let egress = normalize(
            IncidentKind::EgressAnomaly,
            "post to https://evil.example/a",
        );
        let other_egress = normalize(
            IncidentKind::EgressAnomaly,
            "post to https://safe.example/a",
        );
        assert_ne!(
            egress, other_egress,
            "different antigen host must yield different signature"
        );
    }

    #[test]
    fn normalize_url_keeps_host_and_drops_variable_path() {
        let first = normalize(
            IncidentKind::EgressAnomaly,
            "GET https://evil.example/run-1/a?x=1",
        );
        let second = normalize(
            IncidentKind::EgressAnomaly,
            "GET https://evil.example/run-9/b?x=2",
        );
        let third = normalize(
            IncidentKind::EgressAnomaly,
            "GET https://other.example/run-1/a?x=1",
        );
        assert_eq!(first, second);
        assert_ne!(first, third);
    }

    #[test]
    fn new_high_severity_escalates() {
        let mut memory = new_memory(16, 1_000);
        let response = memory.observe(
            IncidentKind::PromptInjection,
            "ignore previous instructions in /tmp/upload/a1.md",
            SeverityTier::Tier2Guard,
            false,
            1_000,
        );
        assert_eq!(response.tier, SeverityTier::Tier3Lockdown);
        assert!(response.escalated);
        assert!(response.reason.contains("升级"));
        assert_eq!(memory.open_count(), 1);
        assert_eq!(memory.known_patterns(), 0);
    }

    #[test]
    fn new_low_severity_stays_quiet() {
        let mut memory = new_memory(16, 1_000);
        let note = memory.observe(
            IncidentKind::EgressAnomaly,
            "post https://example.com/a",
            SeverityTier::Tier1Note,
            false,
            10,
        );
        assert_eq!(note.tier, SeverityTier::Tier1Note);
        assert!(!note.escalated);
        assert_eq!(memory.open_count(), 1);
    }

    #[test]
    fn second_occurrence_downgrades_and_suppresses_escalation() {
        let mut memory = new_memory(16, 1_000);
        let first = memory.observe(
            IncidentKind::ToolPoison,
            "tool description /tmp/a/tool.json",
            SeverityTier::Tier2Guard,
            false,
            1,
        );
        assert_eq!(first.tier, SeverityTier::Tier3Lockdown);
        assert!(first.escalated);

        let second = memory.observe(
            IncidentKind::ToolPoison,
            "tool description /tmp/b/tool.json",
            SeverityTier::Tier2Guard,
            false,
            2,
        );
        assert_eq!(second.tier, SeverityTier::Tier1Note);
        assert!(!second.escalated);
        assert!(second.reason.contains("已知模式"));
        assert_eq!(second.record_key, first.record_key);

        let signature = normalize(
            IncidentKind::ToolPoison,
            "tool description /tmp/c/tool.json",
        );
        let record = memory.record(&signature).expect("record must exist");
        assert_eq!(record.occurrences, 2);
        assert_eq!(record.tolerance_level, 1);
    }

    #[test]
    fn same_pattern_across_instances_reuses_memory() {
        let mut memory = new_memory(16, 1_000);
        let first = memory.observe(
            IncidentKind::PromptInjection,
            "prompt injection at /tmp/run-1/doc1.md",
            SeverityTier::Tier1Note,
            false,
            100,
        );
        let second = memory.observe(
            IncidentKind::PromptInjection,
            "PROMPT INJECTION AT /var/run/doc9.md",
            SeverityTier::Tier1Note,
            false,
            200,
        );
        assert_eq!(first.record_key, second.record_key);
        assert_eq!(second.tier, SeverityTier::Tier0Quiet);
        assert!(!second.escalated);
        assert_eq!(memory.known_patterns(), 1);
    }

    #[test]
    fn self_origin_low_severity_is_tolerated_without_record() {
        let mut memory = new_memory(16, 1_000);
        let response = memory.observe(
            IncidentKind::EgressAnomaly,
            "sync to https://internal.example/health",
            SeverityTier::Tier1Note,
            true,
            10,
        );
        assert_eq!(response.tier, SeverityTier::Tier0Quiet);
        assert!(!response.escalated);
        assert!(response.reason.contains("自我耐受"));
        assert_eq!(memory.open_count(), 0);
        assert_eq!(memory.known_patterns(), 0);
        let signature = normalize(
            IncidentKind::EgressAnomaly,
            "sync to https://internal.example/health",
        );
        assert!(memory.record(&signature).is_none());
    }

    #[test]
    fn self_origin_high_severity_still_records() {
        let mut memory = new_memory(16, 1_000);
        let response = memory.observe(
            IncidentKind::CredentialLeak,
            "/etc/shadow leaked by /usr/bin/self-probe",
            SeverityTier::Tier3Lockdown,
            true,
            10,
        );
        assert_eq!(response.tier, SeverityTier::Tier3Lockdown);
        assert!(response.escalated);
        assert_eq!(memory.open_count(), 1);
    }

    #[test]
    fn resolve_then_cooldown_suppresses_response() {
        let mut memory = new_memory(16, 1_000);
        let signature = normalize(
            IncidentKind::UntrustedContentEscape,
            "escape at /tmp/envelope/envelope.txt",
        );
        let first = memory.observe(
            signature.kind,
            "escape at /tmp/envelope/envelope.txt",
            SeverityTier::Tier3Lockdown,
            false,
            0,
        );
        assert!(first.escalated);
        assert!(memory.resolve(&signature, 10));
        assert_eq!(memory.open_count(), 0);

        let suppressed = memory.observe(
            signature.kind,
            "escape at /var/envelope/envelope.txt",
            SeverityTier::Tier3Lockdown,
            false,
            500,
        );
        assert_eq!(suppressed.tier, SeverityTier::Tier0Quiet);
        assert!(!suppressed.escalated);
        assert!(suppressed.reason.contains("冷却期"));
        assert_eq!(suppressed.record_key, first.record_key);

        let record = memory.record(&signature).expect("record must exist");
        assert_eq!(record.status, IncidentStatus::Resolved);
        assert_eq!(record.occurrences, 2);
        assert_eq!(memory.open_count(), 0);
    }

    #[test]
    fn cooldown_expiry_reopens_incident() {
        let mut memory = new_memory(16, 1_000);
        let signature = normalize(IncidentKind::ToolPoison, "poisoned tool /tmp/a/tool.json");
        memory.observe(
            signature.kind,
            "poisoned tool /tmp/a/tool.json",
            SeverityTier::Tier2Guard,
            false,
            0,
        );
        assert!(memory.resolve(&signature, 10));

        // 冷却窗恰好结束（1010 - 10 == 1000，不再 < cooldown）。
        let reopened = memory.observe(
            signature.kind,
            "poisoned tool /tmp/b/tool.json",
            SeverityTier::Tier2Guard,
            false,
            1_010,
        );
        assert_eq!(reopened.tier, SeverityTier::Tier3Lockdown);
        assert!(reopened.escalated);
        assert!(reopened.reason.contains("冷却期结束"));
        assert_eq!(memory.open_count(), 1);
        let record = memory.record(&signature).expect("record must exist");
        assert_eq!(record.status, IncidentStatus::Open);
        assert_eq!(record.occurrences, 2);
        assert_eq!(record.tolerance_level, 0);
    }

    #[test]
    fn explicit_reopen_restores_open_state() {
        let mut memory = new_memory(16, 1_000);
        let signature = normalize(IncidentKind::CredentialLeak, "token in /tmp/a/env.txt");
        memory.observe(
            signature.kind,
            "token in /tmp/a/env.txt",
            SeverityTier::Tier1Note,
            false,
            1,
        );
        assert!(memory.resolve(&signature, 2));
        assert!(memory.reopen(&signature));
        assert_eq!(memory.open_count(), 1);
        assert!(!memory.reopen(&signature), "already open");

        let known = memory.observe(
            signature.kind,
            "token in /tmp/b/env.txt",
            SeverityTier::Tier2Guard,
            false,
            3,
        );
        assert!(!known.escalated, "reopened signature is already known");
        assert_eq!(known.tier, SeverityTier::Tier1Note);
    }

    #[test]
    fn capacity_evicts_oldest_touched_record() {
        let mut memory = new_memory(2, 1_000);
        let alpha = normalize(IncidentKind::ToolPoison, "alpha /tmp/a/report.txt");
        let beta = normalize(IncidentKind::ToolPoison, "beta /tmp/b/report.txt");
        let gamma = normalize(IncidentKind::ToolPoison, "gamma /tmp/c/report.txt");

        memory.observe(
            IncidentKind::ToolPoison,
            "alpha /tmp/a/report.txt",
            SeverityTier::Tier1Note,
            false,
            100,
        );
        memory.observe(
            IncidentKind::ToolPoison,
            "beta /tmp/b/report.txt",
            SeverityTier::Tier1Note,
            false,
            200,
        );
        // 再次观察 alpha：刷新 last_seen，使其比 beta 更新。
        memory.observe(
            IncidentKind::ToolPoison,
            "alpha /tmp/a/report.txt",
            SeverityTier::Tier1Note,
            false,
            300,
        );
        memory.observe(
            IncidentKind::ToolPoison,
            "gamma /tmp/c/report.txt",
            SeverityTier::Tier1Note,
            false,
            400,
        );

        assert_eq!(memory.store().len(), 2);
        assert!(memory.record(&alpha).is_some());
        assert!(memory.record(&gamma).is_some());
        assert!(
            memory.record(&beta).is_none(),
            "oldest touched record evicted"
        );
    }

    #[test]
    fn stats_open_count_and_known_patterns() {
        let mut memory = new_memory(16, 1_000);
        let signature = normalize(IncidentKind::PromptInjection, "pattern /tmp/a/doc.txt");
        assert_eq!(memory.open_count(), 0);
        assert_eq!(memory.known_patterns(), 0);

        memory.observe(
            IncidentKind::PromptInjection,
            "pattern /tmp/a/doc.txt",
            SeverityTier::Tier1Note,
            false,
            1,
        );
        assert_eq!(memory.open_count(), 1);
        assert_eq!(memory.known_patterns(), 0);

        memory.observe(
            IncidentKind::PromptInjection,
            "pattern /tmp/b/doc.txt",
            SeverityTier::Tier1Note,
            false,
            2,
        );
        assert_eq!(memory.known_patterns(), 1);

        assert!(memory.resolve(&signature, 3));
        assert_eq!(memory.open_count(), 0);
        assert_eq!(memory.known_patterns(), 1);

        memory.observe(
            IncidentKind::PromptInjection,
            "pattern /tmp/c/doc.txt",
            SeverityTier::Tier1Note,
            false,
            4,
        );
        assert_eq!(memory.open_count(), 0, "cooldown hit keeps record resolved");
    }

    #[test]
    fn tolerance_level_saturates() {
        let mut memory = new_memory(16, 1_000);
        let signature = normalize(IncidentKind::Unknown, "noise /tmp/a/file.bin");
        for index in 0..300 {
            memory.observe(
                IncidentKind::Unknown,
                "noise /tmp/a/file.bin",
                SeverityTier::Tier1Note,
                false,
                index,
            );
        }
        let record = memory.record(&signature).expect("record must exist");
        assert_eq!(record.occurrences, 300);
        assert_eq!(record.tolerance_level, u8::MAX);
    }

    #[test]
    fn extreme_inputs_and_clock_bounds_do_not_panic() {
        let mut memory = new_memory(1, 1_000);
        let empty = normalize(IncidentKind::Unknown, "");
        assert_eq!(empty.normalized_key.len(), 16);

        let huge = "x".repeat(64 * 1024);
        let huge_signature = normalize(IncidentKind::PromptInjection, &huge);
        assert_eq!(huge_signature.normalized_key.len(), 16);

        let low = memory.observe(
            IncidentKind::PromptInjection,
            &huge,
            SeverityTier::Tier3Lockdown,
            false,
            i64::MIN,
        );
        assert!(low.escalated);
        assert!(low.tier.rank() <= SeverityTier::Tier3Lockdown.rank());

        assert!(memory.resolve(&huge_signature, i64::MAX));
        // 时钟回拨：保守按仍在冷却期处理，不 panic、不溢出。
        let skewed = memory.observe(
            IncidentKind::PromptInjection,
            &huge,
            SeverityTier::Tier3Lockdown,
            false,
            i64::MIN,
        );
        assert_eq!(skewed.tier, SeverityTier::Tier0Quiet);
        assert!(skewed.reason.contains("冷却期"));

        // 容量为 0 时按 1 处理；未知签名收口 / 重开返回 false。
        let mut tiny = new_memory(0, 0);
        assert_eq!(tiny.store().capacity(), 1);
        let unknown = normalize(IncidentKind::ToolPoison, "never seen");
        assert!(!tiny.resolve(&unknown, 0));
        assert!(!tiny.reopen(&unknown));
        let first = tiny.observe(
            IncidentKind::ToolPoison,
            "one /tmp/a/f.txt",
            SeverityTier::Tier2Guard,
            false,
            i64::MAX,
        );
        let second = tiny.observe(
            IncidentKind::ToolPoison,
            "two /tmp/b/f.txt",
            SeverityTier::Tier2Guard,
            false,
            i64::MAX,
        );
        assert_eq!(tiny.store().len(), 1);
        assert!(first.tier.rank() <= second.tier.rank());
    }

    #[test]
    fn deterministic_given_injected_time() {
        let run = || {
            let mut memory = new_memory(16, 1_000);
            memory.observe(
                IncidentKind::CredentialLeak,
                "leak /tmp/a/creds.txt",
                SeverityTier::Tier2Guard,
                false,
                10,
            );
            memory.observe(
                IncidentKind::CredentialLeak,
                "leak /tmp/b/creds.txt",
                SeverityTier::Tier3Lockdown,
                false,
                20,
            );
            memory.resolve(
                &normalize(IncidentKind::CredentialLeak, "leak /tmp/a/creds.txt"),
                30,
            );
            memory.observe(
                IncidentKind::CredentialLeak,
                "leak /tmp/c/creds.txt",
                SeverityTier::Tier1Note,
                false,
                40,
            );
            memory
        };
        let left = run();
        let right = run();
        assert_eq!(left.store().list(), right.store().list());
        assert_eq!(left.open_count(), right.open_count());
        assert_eq!(left.known_patterns(), right.known_patterns());
    }

    #[test]
    fn serde_roundtrip_preserves_memory_types() {
        let signature = normalize(IncidentKind::UntrustedContentEscape, "escape /tmp/a/x.txt");
        let record = IncidentRecord {
            signature: signature.clone(),
            kind: signature.kind,
            tier: SeverityTier::Tier2Guard,
            first_seen_ms: 1,
            last_seen_ms: 2,
            occurrences: 3,
            status: IncidentStatus::Resolved,
            tolerance_level: 4,
        };
        let response = ImmuneResponse {
            tier: SeverityTier::Tier1Note,
            escalated: false,
            reason: "roundtrip".to_owned(),
            record_key: signature.record_key(),
        };

        let signature_json = serde_json::to_string(&signature).expect("signature serializes");
        let record_json = serde_json::to_string(&record).expect("record serializes");
        let response_json = serde_json::to_string(&response).expect("response serializes");

        assert_eq!(
            serde_json::from_str::<IncidentSignature>(&signature_json).expect("signature parses"),
            signature
        );
        assert_eq!(
            serde_json::from_str::<IncidentRecord>(&record_json).expect("record parses"),
            record
        );
        assert_eq!(
            serde_json::from_str::<ImmuneResponse>(&response_json).expect("response parses"),
            response
        );
    }
}
