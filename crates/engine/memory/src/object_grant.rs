//! 记忆**对象级授权**（C-D1 · `tool.object_grant`）的账本与可见性原语。
//!
//! # 一句话
//!
//! 把**一条**记忆（episode / fact）以**只读**授予某主体；被授方可读不可写；
//! 撤销**即时生效**；授予/撤销**逐次人批 + 留审计**。
//!
//! # 语义（lead 拍板，**不得改**）：**叠加 ≠ 替代**
//!
//! ```text
//! 读可见 = 域可见 ∨ (存在有效对象 grant)
//! ```
//!
//! * **域可见** = 既有 [`MemoryScope::is_visible_in`]（**一个字都不改**）；
//! * **对象 grant** 是**叠加层**：只在「域可见为 false」时补一条授权；
//! * **域级隔离不被突破**：对象 grant **不扩大**任何域的可见范围
//!   （只对**那一个对象 id** 生效），也**不绕过**域级闸；
//! * **跨分区披露 = R4 硬门**：对象所在分区 ≠ 主体所在分区时，授予
//!   **必须逐次人批**才生效（请求先落 `pending`，人批后才 `active`），
//!   且授予 / 撤销 / 人批 / 拒绝**全部留审计行**。
//!
//! # 账本
//!
//! * **append-only**：每次状态变更 = **追加一行**，旧行永不改写；行号单调
//!   （`version` / `seq`）⇒ 追加顺序即裁决顺序，可回放可对账；
//! * **坏行按不存在处理**：解析失败 / 关键字段缺失 / 关键字段为空的行**跳过**
//!   （既不报错也**不放行**）⇒ 伪造行、截断行都不会变成有效授权。
//!
//! # 撤销即时生效（复用既有热撤销语义，**不新造轮询/缓存失效**）
//!
//! 判定**每次现读账本**（`Mutex<ObjectGrantLedger>`：无快照、无 TTL、无后台轮询）
//! ⇒ 撤销落账后**下一次判定**立刻生效。与既有 `PermissionPolicy::revoke` +
//! 共享 live 策略槽（面板热撤销）同一口径：**状态在权威账上，判定读权威账**。
//!
//! # 读路径并联点（**叠加**的落法）
//!
//! 既有域级过滤点在 `crates/engine/memory/src/coordinator.rs`（工作记忆层 / 情景区 /
//! 无 scoped backend 的回退路径）与 `crates/engine/memory/src/retrieval_pipeline.rs`
//! （重排过滤）；本文件给出**并联判定**原语 [`GrantVisibility::allows`]（用法见其文档）
//! ⇒ 域可见为 true 时**行为逐字节不变**；为 false 时才去看对象授权。候选生成侧另需
//! 并入 [`ObjectGrantRegistry::granted_object_ids`]（否则被授对象进不了候选集）。
//!
//! # 依赖边界
//!
//! 本 crate **不依赖** `apeireth-governance`（依赖边预算）⇒ 审批链不落在这里：
//! 本文件只管**账本 + 判定原语**；治理档（逐次人批）与模型面工具在
//! `crates/engine/runtime-assembly/src/canonical/object_grant.rs`。

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::scope::MemoryScope;

/// 授权的**生效范围**：本族**只有只读**。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantEffect {
    /// 只读授予（本族唯一的效果；**写范围不存在**）。
    ReadOnly,
}

/// 授权状态机（**每次变更 = 追加一行**，旧行永不改写）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantStatus {
    /// 已请求，等待人批（**未生效**：被授方读不到）。
    Pending,
    /// 已人批生效。
    Active,
    /// 人批否决。
    Denied,
    /// 已撤销（**即时生效**：被授方立刻读不到）。
    Revoked,
}

impl GrantStatus {
    /// 该状态是否**生效**（**只有 `Active` 生效**）。
    pub const fn is_effective(self) -> bool {
        matches!(self, Self::Active)
    }
}

/// 一条对象授权账本行（**append-only**）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrantLedgerEntry {
    /// 账本单调版本号（从 1 起，追加即 +1）。
    pub version: u64,
    /// 授权单号：同一授权的历次状态变更共用一个单号。
    pub grant_id: String,
    /// 记忆对象 id（episode / fact）。
    pub object_id: String,
    /// 被授主体（不透明字符串；生产取命名空间键）。
    pub subject: String,
    /// 授予人（发起这次授权请求的主体）。
    pub grantor: String,
    /// 授予请求时间（毫秒；后续状态行沿用）。
    pub granted_at_ms: i64,
    /// 本行时间（毫秒）。
    pub at_ms: i64,
    /// 生效范围（**恒为只读**）。
    pub effect: GrantEffect,
    /// 状态。
    pub status: GrantStatus,
    /// 对象所在分区（域）标识：授权**绑定**在该分区上，分区被偷换 ⇒ 授权失效。
    pub partition: String,
    /// 是否跨分区披露（`true` ⇒ R4 硬门 ⇒ 必须逐次人批 + 审计）。
    pub cross_partition: bool,
    /// 理由（请求理由 / 撤销理由）。
    pub reason: Option<String>,
    /// 人批执行者（批准 / 否决 / 撤销的人）。
    pub decided_by: Option<String>,
    /// 人批时间（毫秒）。
    pub decided_at_ms: Option<i64>,
}

impl GrantLedgerEntry {
    /// 这一行是否**结构完好**（关键字段非空）；坏行在读回时按**不存在**处理。
    fn is_structurally_sound(&self) -> bool {
        !self.grant_id.trim().is_empty()
            && !self.object_id.trim().is_empty()
            && !self.subject.trim().is_empty()
            && !self.grantor.trim().is_empty()
    }
}

/// 审计行种类（授予 / 撤销 / 人批 / 拒绝**全部留痕**）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantAuditKind {
    /// 授权请求已登记（**未生效**）。
    GrantRequested,
    /// 人批通过（授权生效）。
    GrantApproved,
    /// 人批否决。
    GrantDenied,
    /// 撤销（**即时生效**）。
    GrantRevoked,
    /// 请求被拒（对象不存在 / 主体为空 / 状态不对）。
    GrantRefused,
}

/// 一行审计。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrantAuditRow {
    /// 审计序号（单调）。
    pub seq: u64,
    /// 时间（毫秒）。
    pub at_ms: i64,
    /// 审计种类。
    pub kind: GrantAuditKind,
    /// 授权单号（被拒时可能为空）。
    pub grant_id: Option<String>,
    /// 对象 id。
    pub object_id: String,
    /// 主体。
    pub subject: String,
    /// 行为人（发起人 / 人批执行者）。
    pub actor: String,
    /// 明文（给人看的理由）。
    pub detail: String,
}

/// 账本文件里的一行（JSONL：一行一条记录）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "record", rename_all = "snake_case")]
enum LedgerLine {
    /// 账本行。
    Grant(GrantLedgerEntry),
    /// 审计行。
    Audit(GrantAuditRow),
}

/// 对象授权账本（**append-only**，可落文件）。
///
/// 每次状态变更 = **追加一行**（旧行永不改写）；行号单调递增 ⇒ 追加顺序即裁决
/// 顺序；**坏行按不存在处理**（解析失败 / 字段缺失 / 字段为空 ⇒ 跳过）。
#[derive(Debug)]
pub struct ObjectGrantLedger {
    /// 落盘路径（`None` = 纯内存账本）。
    path: Option<PathBuf>,
    /// 账本行（追加顺序）。
    entries: Vec<GrantLedgerEntry>,
    /// 审计行（追加顺序）。
    audits: Vec<GrantAuditRow>,
    /// 下一个账本版本号。
    next_version: u64,
    /// 下一个审计序号。
    next_seq: u64,
}

impl Default for ObjectGrantLedger {
    fn default() -> Self {
        Self::in_memory()
    }
}

impl ObjectGrantLedger {
    /// 纯内存账本（不落盘；进程内即时生效）。
    #[must_use]
    pub fn in_memory() -> Self {
        Self {
            path: None,
            entries: Vec::new(),
            audits: Vec::new(),
            next_version: 1,
            next_seq: 1,
        }
    }

    /// 打开（或新建）账本文件；**已存在的坏行按不存在处理**（跳过）。
    pub fn open(path: impl AsRef<Path>) -> Result<Self, GrantError> {
        let path = path.as_ref().to_path_buf();
        let mut ledger = Self {
            path: Some(path.clone()),
            ..Self::in_memory()
        };
        // 账本文件不存在 = 空账本（首次授予时创建）。
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Ok(ledger);
        };
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            // 坏行（解析失败）按不存在处理：跳过，既不报错也不放行。
            let Ok(record) = serde_json::from_str::<LedgerLine>(trimmed) else {
                continue;
            };
            match record {
                LedgerLine::Grant(entry) => {
                    if !entry.is_structurally_sound() {
                        continue;
                    }
                    ledger.next_version = ledger.next_version.max(entry.version.saturating_add(1));
                    ledger.entries.push(entry);
                }
                LedgerLine::Audit(row) => {
                    ledger.next_seq = ledger.next_seq.max(row.seq.saturating_add(1));
                    ledger.audits.push(row);
                }
            }
        }
        Ok(ledger)
    }

    /// 账本文件路径（`None` = 纯内存）。
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// 账本行（只读快照，追加顺序）。
    #[must_use]
    pub fn entries(&self) -> &[GrantLedgerEntry] {
        &self.entries
    }

    /// 审计行（只读快照，追加顺序）。
    #[must_use]
    pub fn audits(&self) -> &[GrantAuditRow] {
        &self.audits
    }

    /// 某个授权单的**当前状态**（该单号的最后一行）。
    #[must_use]
    pub fn latest_state(&self, grant_id: &str) -> Option<&GrantLedgerEntry> {
        self.entries
            .iter()
            .rev()
            .find(|entry| entry.grant_id == grant_id)
    }

    /// 追加一条账本行（**先落盘再入内存**：落盘失败 ⇒ 内存也不留 ⇒ 不产生授权）。
    pub fn append_grant(
        &mut self,
        mut entry: GrantLedgerEntry,
    ) -> Result<GrantLedgerEntry, GrantError> {
        entry.version = self.next_version;
        self.write_line(&LedgerLine::Grant(entry.clone()))?;
        self.next_version = self.next_version.saturating_add(1);
        self.entries.push(entry.clone());
        Ok(entry)
    }

    /// 追加一条审计行。
    pub fn append_audit(&mut self, mut row: GrantAuditRow) -> Result<GrantAuditRow, GrantError> {
        row.seq = self.next_seq;
        self.write_line(&LedgerLine::Audit(row.clone()))?;
        self.next_seq = self.next_seq.saturating_add(1);
        self.audits.push(row.clone());
        Ok(row)
    }

    fn write_line(&self, line: &LedgerLine) -> Result<(), GrantError> {
        let Some(path) = self.path.as_ref() else {
            return Ok(());
        };
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|error| GrantError::LedgerWrite(error.to_string()))?;
        let mut text = serde_json::to_string(line)
            .map_err(|error| GrantError::LedgerWrite(error.to_string()))?;
        text.push('\n');
        file.write_all(text.as_bytes())
            .map_err(|error| GrantError::LedgerWrite(error.to_string()))?;
        file.flush()
            .map_err(|error| GrantError::LedgerWrite(error.to_string()))?;
        Ok(())
    }
}

/// 记忆对象解析面（宿主注入）：回答「这个对象存在吗 / 在哪个分区 / 主体在哪个分区」。
///
/// 记忆 crate **不认识**存储实现 ⇒ 判定靠宿主注入的解析面；解析面缺失时一律
/// **fail-closed**（查不出来 = 不可判定 = 不放行）。
pub trait MemoryObjectResolver: Send + Sync {
    /// 对象是否存在。**不存在 ⇒ 授权一律不生效**（对抗：伪造对象 id）。
    fn object_exists(&self, object_id: &str) -> bool;

    /// 对象所在分区（域）标识；`None` = 查不到 ⇒ 授权不生效。
    fn object_partition(&self, object_id: &str) -> Option<String>;

    /// 主体所属分区（域）标识；`None` = 查不到 ⇒ **按跨分区处理**（fail-closed）。
    fn subject_partition(&self, _subject: &str) -> Option<String> {
        None
    }

    /// 主体在**域级**是否已可见（缺省 = 不可见 ⇒ 可见性只可能来自对象授权）。
    fn domain_visible(&self, _subject: &str, _object_id: &str) -> bool {
        false
    }
}

/// 对象授权的错误（**一律 fail-closed**）。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GrantError {
    /// 对象 id 为空。
    #[error("对象 id 为空")]
    EmptyObjectId,
    /// 主体为空。
    #[error("主体为空")]
    EmptySubject,
    /// 授予人 / 人批执行者为空。
    #[error("授予人为空")]
    EmptyGrantor,
    /// 对象不存在或不可解析。
    #[error("对象不存在或不可解析：{0}")]
    UnknownObject(String),
    /// 授权单不存在。
    #[error("授权单不存在：{0}")]
    UnknownGrant(String),
    /// 授权单当前状态不可做这个操作。
    #[error("授权单状态不可操作：{0}")]
    BadState(String),
    /// 账本写入失败。
    #[error("账本写入失败：{0}")]
    LedgerWrite(String),
}

/// 对象授权登记处（**热读**：每次判定现读账本 ⇒ 撤销**即时生效**）。
pub struct ObjectGrantRegistry {
    /// 账本（唯一的权威状态）。
    ledger: Mutex<ObjectGrantLedger>,
    /// 对象解析面（`None` ⇒ 无法校验对象 ⇒ 授权永不生效）。
    resolver: Option<Arc<dyn MemoryObjectResolver>>,
}

impl ObjectGrantRegistry {
    /// 造一个登记处（**无解析面** ⇒ 无法校验对象 ⇒ 授权永不生效，fail-closed）。
    #[must_use]
    pub fn new(ledger: ObjectGrantLedger) -> Self {
        Self {
            ledger: Mutex::new(ledger),
            resolver: None,
        }
    }

    /// 造一个登记处（注入对象解析面）。
    #[must_use]
    pub fn with_resolver(
        ledger: ObjectGrantLedger,
        resolver: Arc<dyn MemoryObjectResolver>,
    ) -> Self {
        Self {
            ledger: Mutex::new(ledger),
            resolver: Some(resolver),
        }
    }

    /// 账本文件路径（`None` = 纯内存账本）。
    #[must_use]
    pub fn ledger_path(&self) -> Option<PathBuf> {
        self.lock().path.clone()
    }

    /// 登记一条**授权请求**（**不生效**：等人批 ⇒ 域级隔离此刻未被突破）。
    ///
    /// 对象必须存在（不存在 ⇒ 拒绝 + 审计）；主体 / 授予人不得为空；跨分区
    /// （对象分区 ≠ 主体分区，或主体分区查不到）⇒ 标 `cross_partition`
    /// ⇒ R4 硬门 ⇒ 必须逐次人批才生效。
    pub fn request_grant(
        &self,
        grantor: &str,
        object_id: &str,
        subject: &str,
        reason: Option<&str>,
        at_ms: i64,
    ) -> Result<GrantLedgerEntry, GrantError> {
        let grantor = grantor.trim();
        let object_id = object_id.trim();
        let subject = subject.trim();
        if object_id.is_empty() {
            return Err(GrantError::EmptyObjectId);
        }
        if subject.is_empty() {
            return Err(GrantError::EmptySubject);
        }
        if grantor.is_empty() {
            return Err(GrantError::EmptyGrantor);
        }

        let mut ledger = self.lock();
        let Some(partition) = self.resolve_partition(object_id) else {
            // 对象不存在 / 不可解析 ⇒ 拒绝（对抗：伪造对象 id）并留审计。
            let _ = ledger.append_audit(GrantAuditRow {
                seq: 0,
                at_ms,
                kind: GrantAuditKind::GrantRefused,
                grant_id: None,
                object_id: object_id.to_string(),
                subject: subject.to_string(),
                actor: grantor.to_string(),
                detail: "对象不存在或不可解析 ⇒ 拒绝授权".to_string(),
            });
            return Err(GrantError::UnknownObject(object_id.to_string()));
        };
        let subject_partition = self
            .resolver
            .as_ref()
            .and_then(|resolver| resolver.subject_partition(subject));
        // 主体分区查不到 ⇒ 按跨分区处理（fail-closed ⇒ 必须人批）。
        let cross_partition = subject_partition
            .map(|own| own != partition)
            .unwrap_or(true);
        let version = ledger.next_version;
        let seed = format!("{object_id}|{subject}|{version}");
        let grant_id = Uuid::new_v5(&Uuid::NAMESPACE_OID, seed.as_bytes()).to_string();
        let entry = GrantLedgerEntry {
            version: 0,
            grant_id,
            object_id: object_id.to_string(),
            subject: subject.to_string(),
            grantor: grantor.to_string(),
            granted_at_ms: at_ms,
            at_ms,
            effect: GrantEffect::ReadOnly,
            status: GrantStatus::Pending,
            partition: partition.clone(),
            cross_partition,
            reason: reason.map(str::to_string),
            decided_by: None,
            decided_at_ms: None,
        };
        let entry = ledger.append_grant(entry)?;
        let detail = if cross_partition {
            "授权请求已登记（未生效；跨分区披露 ⇒ R4 硬门 ⇒ 逐次人批后才生效）".to_string()
        } else {
            "授权请求已登记（未生效；只读授予 ⇒ 待人批）".to_string()
        };
        let _ = ledger.append_audit(GrantAuditRow {
            seq: 0,
            at_ms,
            kind: GrantAuditKind::GrantRequested,
            grant_id: Some(entry.grant_id.clone()),
            object_id: entry.object_id.clone(),
            subject: entry.subject.clone(),
            actor: grantor.to_string(),
            detail,
        });
        Ok(entry)
    }

    /// 人批通过 ⇒ 授权**生效**（只读）。
    pub fn approve_grant(
        &self,
        approver: &str,
        grant_id: &str,
        at_ms: i64,
    ) -> Result<GrantLedgerEntry, GrantError> {
        self.transition(
            approver,
            grant_id,
            GrantStatus::Active,
            &[GrantStatus::Pending],
            None,
            at_ms,
            GrantAuditKind::GrantApproved,
            "人批通过（只读生效）",
        )
    }

    /// 人批否决。
    pub fn deny_grant(
        &self,
        approver: &str,
        grant_id: &str,
        reason: Option<&str>,
        at_ms: i64,
    ) -> Result<GrantLedgerEntry, GrantError> {
        self.transition(
            approver,
            grant_id,
            GrantStatus::Denied,
            &[GrantStatus::Pending],
            reason,
            at_ms,
            GrantAuditKind::GrantDenied,
            "人批否决",
        )
    }

    /// 撤销（**即时生效**：落账后下一次判定立刻读不到）。
    pub fn revoke_grant(
        &self,
        approver: &str,
        grant_id: &str,
        reason: Option<&str>,
        at_ms: i64,
    ) -> Result<GrantLedgerEntry, GrantError> {
        self.transition(
            approver,
            grant_id,
            GrantStatus::Revoked,
            &[GrantStatus::Pending, GrantStatus::Active],
            reason,
            at_ms,
            GrantAuditKind::GrantRevoked,
            "撤销（即时生效）",
        )
    }

    /// 状态迁移（批准 / 否决 / 撤销共用）：**追加一行**新状态 + **追加一行**审计；
    /// 旧行永不改写 ⇒ 裁决历史可回放；迁移只允许从 `allowed_from` 出发。
    fn transition(
        &self,
        approver: &str,
        grant_id: &str,
        target: GrantStatus,
        allowed_from: &[GrantStatus],
        reason: Option<&str>,
        at_ms: i64,
        audit: GrantAuditKind,
        detail: &str,
    ) -> Result<GrantLedgerEntry, GrantError> {
        let approver = approver.trim();
        let grant_id = grant_id.trim();
        if approver.is_empty() {
            return Err(GrantError::EmptyGrantor);
        }
        if grant_id.is_empty() {
            return Err(GrantError::UnknownGrant(grant_id.to_string()));
        }
        let mut ledger = self.lock();
        let previous = ledger
            .latest_state(grant_id)
            .cloned()
            .ok_or_else(|| GrantError::UnknownGrant(grant_id.to_string()))?;
        if !allowed_from.contains(&previous.status) {
            return Err(GrantError::BadState(format!(
                "{grant_id} 当前状态 {:?} 不可迁移到 {target:?}",
                previous.status
            )));
        }
        let entry = ledger.append_grant(GrantLedgerEntry {
            status: target,
            at_ms,
            reason: reason.map(str::to_string).or(previous.reason),
            decided_by: Some(approver.to_string()),
            decided_at_ms: Some(at_ms),
            ..previous
        })?;
        let detail = reason.map(str::to_string).unwrap_or_else(|| {
            if previous.cross_partition && audit == GrantAuditKind::GrantApproved {
                format!("{detail}；跨分区披露 ⇒ R4 硬门已过")
            } else {
                detail.to_string()
            }
        });
        let _ = ledger.append_audit(GrantAuditRow {
            seq: 0,
            at_ms,
            kind: audit,
            grant_id: Some(entry.grant_id.clone()),
            object_id: entry.object_id.clone(),
            subject: entry.subject.clone(),
            actor: approver.to_string(),
            detail,
        });
        Ok(entry)
    }

    /// 撤销某主体对某对象的**全部**授权（按对象 + 主体定位）；返回被撤销的单数。
    pub fn revoke_pair(
        &self,
        approver: &str,
        object_id: &str,
        subject: &str,
        reason: Option<&str>,
        at_ms: i64,
    ) -> Result<usize, GrantError> {
        let approver = approver.trim();
        let object_id = object_id.trim();
        let subject = subject.trim();
        if approver.is_empty() {
            return Err(GrantError::EmptyGrantor);
        }
        if object_id.is_empty() {
            return Err(GrantError::EmptyObjectId);
        }
        if subject.is_empty() {
            return Err(GrantError::EmptySubject);
        }
        let mut revoked = 0usize;
        loop {
            let target = {
                let ledger = self.lock();
                latest_rows(&ledger, subject, Some(object_id))
                    .into_iter()
                    .find(|entry| {
                        matches!(entry.status, GrantStatus::Pending | GrantStatus::Active)
                    })
                    .map(|entry| entry.grant_id.clone())
            };
            let Some(grant_id) = target else {
                break;
            };
            self.revoke_grant(approver, &grant_id, reason, at_ms)?;
            revoked += 1;
        }
        Ok(revoked)
    }

    /// 该主体对某对象是否有**有效**对象授权（只读）：主体 / 对象非空 → 对象存在
    /// → 分区绑定未被偷换 → 有 `Active` 单。
    pub fn read_grant_allows(&self, subject: &str, object_id: &str) -> bool {
        let subject = subject.trim();
        let object_id = object_id.trim();
        if subject.is_empty() || object_id.is_empty() {
            return false;
        }
        let ledger = self.lock();
        latest_rows(&ledger, subject, Some(object_id))
            .into_iter()
            .any(|entry| entry.status.is_effective() && self.binding_holds(entry))
    }

    /// **叠加**判定：读可见 = 域可见 ∨ 有效对象授权（域可见为 true ⇒ **直接放行**，
    /// 域级语义一字不改；为 false ⇒ 才看授权）。
    pub fn read_visible(&self, subject: &str, object_id: &str, domain_visible: bool) -> bool {
        domain_visible || self.read_grant_allows(subject, object_id)
    }

    /// 对象授权**只读** ⇒ 写路径恒不放行（被授方永远拿不到写权）。
    pub fn allows_write(&self, _subject: &str, _object_id: &str) -> bool {
        false
    }

    /// 该主体当前**生效**的授权对象 id 列表（候选生成接线原语）。
    #[must_use]
    pub fn granted_object_ids(&self, subject: &str) -> Vec<String> {
        let subject = subject.trim();
        if subject.is_empty() {
            return Vec::new();
        }
        let ledger = self.lock();
        let mut ids: Vec<String> = latest_rows(&ledger, subject, None)
            .into_iter()
            .filter(|entry| entry.status.is_effective() && self.binding_holds(entry))
            .map(|entry| entry.object_id.clone())
            .collect();
        ids.sort();
        ids.dedup();
        ids
    }

    /// 账本行（只读快照）。
    #[must_use]
    pub fn entries(&self) -> Vec<GrantLedgerEntry> {
        self.lock().entries().to_vec()
    }

    /// 审计行（只读快照）。
    #[must_use]
    pub fn audit_rows(&self) -> Vec<GrantAuditRow> {
        self.lock().audits().to_vec()
    }

    /// 主体在**域级**是否已可见（委托解析面；无解析面 ⇒ 不可见）。
    pub fn domain_visible(&self, subject: &str, object_id: &str) -> bool {
        self.resolver
            .as_ref()
            .is_some_and(|resolver| resolver.domain_visible(subject, object_id))
    }

    /// 授权绑定是否仍然成立（对象在 + 分区没被偷换）。
    fn binding_holds(&self, entry: &GrantLedgerEntry) -> bool {
        if entry.object_id.trim().is_empty() || entry.subject.trim().is_empty() {
            return false;
        }
        // 无解析面 ⇒ 无法校验对象 ⇒ 不放行（fail-closed）。
        let Some(resolver) = self.resolver.as_ref() else {
            return false;
        };
        if !resolver.object_exists(&entry.object_id) {
            return false;
        }
        match resolver.object_partition(&entry.object_id) {
            Some(partition) => partition == entry.partition,
            None => false,
        }
    }

    /// 解析对象所在分区（`None` = 不存在 / 不可解析）。
    fn resolve_partition(&self, object_id: &str) -> Option<String> {
        let resolver = self.resolver.as_ref()?;
        if !resolver.object_exists(object_id) {
            return None;
        }
        resolver.object_partition(object_id)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, ObjectGrantLedger> {
        self.ledger
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// 同一主体（可选限定对象）下**每个授权单的最后一行**（append-only ⇒ 最后一行 = 当前状态）。
fn latest_rows<'a>(
    ledger: &'a ObjectGrantLedger,
    subject: &str,
    object_id: Option<&str>,
) -> Vec<&'a GrantLedgerEntry> {
    let mut latest: BTreeMap<&str, &GrantLedgerEntry> = BTreeMap::new();
    for entry in ledger.entries() {
        if entry.subject == subject && object_id.is_none_or(|id| entry.object_id == id) {
            latest.insert(entry.grant_id.as_str(), entry);
        }
    }
    latest.into_values().collect()
}

/// 读路径的**叠加**判定（域可见 ∨ 有效对象授权）。
///
/// 接线到既有域级过滤点：`GrantVisibility::new(&registry, &subject)` 之后
/// `overlay.allows(&scope, &query.visible_scopes, &ep.id)` 替换域级判定即可。
pub struct GrantVisibility<'a> {
    registry: &'a ObjectGrantRegistry,
    subject: &'a str,
}

impl<'a> GrantVisibility<'a> {
    /// 造一个叠加判定（主体 = 命名空间键等不透明主体标识）。
    #[must_use]
    pub fn new(registry: &'a ObjectGrantRegistry, subject: &'a str) -> Self {
        Self { registry, subject }
    }

    /// 读可见 = 域可见（**既有语义不改**） ∨ 有效对象授权。
    #[must_use]
    pub fn allows(
        &self,
        scope: &MemoryScope,
        visible_scopes: &[MemoryScope],
        object_id: &str,
    ) -> bool {
        scope.is_visible_in(visible_scopes)
            || self.registry.read_grant_allows(self.subject, object_id)
    }

    /// 该主体当前生效的授权对象 id（候选生成侧接线用）。
    #[must_use]
    pub fn granted_object_ids(&self) -> Vec<String> {
        self.registry.granted_object_ids(self.subject)
    }
}
