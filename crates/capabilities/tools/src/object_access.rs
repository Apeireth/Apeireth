//! 对象级访问判定 (E-P3 · 威胁模型 G3「沙箱对象级判定」)。
//!
//! # 问题:「换 API 换权力」
//! 词法/命令级判定按 **API 形态**记账: 拦得住这一种写法, 换一种等价写法就穿。
//! 本模块把判定升到**对象级** —— 权力挂在「哪个对象、哪个动作」上, 与调用
//! 形态无关: 同一对象的同一动作, 不管经哪条 API 路径落地, 判定逐一相同。
//!
//! # 判定模型 (两个互补面)
//! 1. **请求时判定** ([`ObjectAccessJudge::check`] + [`ObjectAccessPolicy`]):
//!    声明面动作 (枚举/预声明的对象操作) 按 [`ObjectAccessRequest`] 直接裁决 ——
//!    谁 ([`ObjectSubject`]) 对哪个对象 ([`Path`]) 做什么 ([`ObjectAction`])。
//! 2. **事实判定** ([`ObjectAccessJudge::guard`] / [`ObjectWindow`]): 受控执行
//!    **前后对象状态对比** ([`diff_object_states`]) ——「动了哪些对象」是事实,
//!    与形态无关; 每个事实变更对象再过一次请求时判定, 未授权即违规。
//!    词法面明示的漏放边界 (编码负载 / 解释器内写入 / 动态成员调用等) 在这里
//!    按对象事实兜底。
//!
//! # 越界即帧 + 审计
//! 事实判定发现未授权对象操作 → [`ObjectGuardReport::failure`] 走**闭合失败
//! 词表**出帧 (`pipeline.post_block`, source=`object_access_judge`, 理由串逐条
//! 列对象/动作/进程) + 每条违规落一行 `sandbox.object_violation` 审计
//! ([`ObjectAuditSink`], 对象/动作/进程/域/理由)。失败即帧, 不静默降级。
//!
//! # 挂载点 (星港线深望智能体接此契约)
//! [`ObjectAccessPolicy`] 是对象判定的唯一裁决接口: 实现一个策略、装进
//! [`ObjectAccessJudge`]、挂到执行面 ([`crate::TrustedShellConfig::with_object_judgment`])
//! 或直接以 [`ObjectAccessJudge::check`] / [`ObjectAccessJudge::guard`] 消费。
//! [`ObjectPolicyChain`] 把附加策略叠加在默认策略之上 (顺序判定, 第一个拒绝即拒绝)。
//!
//! # 边界与残余 (如实)
//! - **事实判定是事后判定**: 违规对象变更已发生在盘上, 判定给出失败帧 + 问责
//!   审计; 回滚不在本层 (可逆性由受控写工具/审批链负责);
//! - **事实判定有域界**: 快照只覆盖 [`ObjectDomain`] 域内对象, 域外对象变更
//!   不入差分 (域外访问由请求时判定 + 沙箱物理围栏兜住);
//! - **枚举零状态痕迹**: 枚举 (读目录/读对象集合) 不改对象状态, 不入事实差分,
//!   只在请求时判定面裁决 (见 [`ObjectAction::Enumerate`]);
//! - **指纹口径**: 对象指纹 = (种类, 长度, mtime) 元数据三元组, 不读文件内容
//!   (与读前观测门禁同口径); 同长同 mtime 粒度内的原地改写不可见;
//! - **同域并发窗口互相可见**: 差分是窗口级事实, 同一域上并发执行的变更会互相
//!   进入对方窗口的差分 (归属是窗口级, 不是进程级); 并发安全面 = 快照是不可变
//!   值、判定器无共享可变态, 不同域并发互不串扰 (测试钉住);
//! - **审计面**: [`ObjectAuditSink`] 是挂载点; 默认 [`MemoryObjectAuditSink`]
//!   为进程内记录 (重启即重置, 与读前观测表同一边界), 落盘 sink 由宿主接线。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::exec_pipeline::PipelineFailure;
use crate::observed_gate::path_key;

/// 失败帧 source 名 (闭合失败词表里指认本判定器)。
pub const OBJECT_JUDGE_SOURCE: &str = "object_access_judge";

/// `sandbox.object_violation` 审计事件名: 对象越界事实的问责行 (对象/动作/进程)。
pub const OBJECT_VIOLATION_AUDIT_EVENT: &str = "sandbox.object_violation";

/// 对象动作词表 (冻结四动作)。
///
/// 事实差分 ([`diff_object_states`]) 只产生前三类; [`ObjectAction::Enumerate`]
/// 不改对象状态, 只在请求时判定面出现。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectAction {
    /// 创建对象。
    Create,
    /// 修改对象。
    Modify,
    /// 删除对象。
    Delete,
    /// 枚举对象集合 (零状态痕迹, 只走请求时判定)。
    Enumerate,
}

impl ObjectAction {
    /// 全词表 (矩阵测试按此展开, 新增动作必入此表)。
    pub const ALL: [Self; 4] = [Self::Create, Self::Modify, Self::Delete, Self::Enumerate];

    /// 稳定词面 (审计行 / 帧文案用)。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Modify => "modify",
            Self::Delete => "delete",
            Self::Enumerate => "enumerate",
        }
    }
}

impl fmt::Display for ObjectAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 对象种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectKind {
    /// 文件 (含符号链接等非目录对象; 快照不跟随符号链接)。
    File,
    /// 目录。
    Directory,
}

/// 对象状态指纹: (种类, 长度, mtime) 元数据三元组 —— 不读文件内容。
///
/// 目录指纹只看种类: 目录成员集合的变化由子对象差分承担, 不重复计入。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectFingerprint {
    /// 对象种类。
    pub kind: ObjectKind,
    /// 字节数 (目录恒为 0)。
    pub len: u64,
    /// 修改时间 (目录恒为纪元)。
    pub mtime: SystemTime,
}

impl ObjectFingerprint {
    /// 从已取得的文件元数据构造指纹 (不追加 IO)。
    pub fn from_metadata(metadata: &std::fs::Metadata) -> Self {
        if metadata.is_dir() {
            Self::directory()
        } else {
            Self {
                kind: ObjectKind::File,
                len: metadata.len(),
                mtime: metadata.modified().unwrap_or(UNIX_EPOCH),
            }
        }
    }

    /// 目录指纹 (成员集合由子对象差分承担)。
    pub const fn directory() -> Self {
        Self {
            kind: ObjectKind::Directory,
            len: 0,
            mtime: UNIX_EPOCH,
        }
    }
}

/// 一个对象域的快照: 域内对象 → 指纹, 键为归一化相对路径。
///
/// 快照是**不可变值** ([`ObjectSnapshot::complete`] 只标注覆盖是否完整),
/// 事实判定的并发安全就建立在「快照即值」上。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectSnapshot {
    root: PathBuf,
    entries: BTreeMap<String, ObjectFingerprint>,
    complete: bool,
}

impl ObjectSnapshot {
    /// 空快照 (覆盖完整)。假探针/测试可据此逐个 [`ObjectSnapshot::insert`] 构造。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            entries: BTreeMap::new(),
            complete: true,
        }
    }

    /// 记一个对象 (键 = 归一化相对路径)。
    pub fn insert(&mut self, key: impl Into<String>, fingerprint: ObjectFingerprint) {
        self.entries.insert(key.into(), fingerprint);
    }

    /// 标注覆盖是否完整 (false = 触达条目上限或有子树不可读)。
    pub fn set_complete(&mut self, complete: bool) {
        self.complete = complete;
    }

    /// 域根。
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 对象表 (键序 = 差分的确定性来源)。
    pub fn entries(&self) -> &BTreeMap<String, ObjectFingerprint> {
        &self.entries
    }

    /// 覆盖是否完整。
    pub fn complete(&self) -> bool {
        self.complete
    }
}

/// 一个事实对象变更 (事实差分只产生创建/修改/删除三类)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectChange {
    /// 事实动作 (Create / Modify / Delete)。
    pub action: ObjectAction,
    /// 被动对象 (域根拼接的绝对路径)。
    pub object: PathBuf,
}

/// 事实差分: 执行前后对象状态对比 ——「动了哪些对象」的事实判定, 与 API 形态无关。
///
/// 纯函数 (零 IO、零策略), 输出按对象键序确定; 同一对象同一动作不论经哪条
/// API 路径落地, 只要状态事实相同, 差分逐一相同。
pub fn diff_object_states(before: &ObjectSnapshot, after: &ObjectSnapshot) -> Vec<ObjectChange> {
    let keys: BTreeSet<&String> = before
        .entries()
        .keys()
        .chain(after.entries().keys())
        .collect();
    let mut changes = Vec::new();
    for key in keys {
        let action = match (before.entries().get(key), after.entries().get(key)) {
            (None, Some(_)) => Some(ObjectAction::Create),
            (Some(_), None) => Some(ObjectAction::Delete),
            (Some(before_fp), Some(after_fp)) if before_fp != after_fp => {
                Some(ObjectAction::Modify)
            }
            _ => None,
        };
        if let Some(action) = action {
            changes.push(ObjectChange {
                action,
                object: before.root().join(key),
            });
        }
    }
    changes
}

/// 路径对象域: 判定的域界 (域内 / 敏感 / 域外)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectDomain {
    root: PathBuf,
}

impl ObjectDomain {
    /// 以域根构造。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// 域根。
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 对象落在哪个域 (词法归一判定, 与读前观测门禁同一口径)。
    ///
    /// 域外 / 拼写对不上域根 = [`ObjectZone::OutsideWorkspace`] (fail-closed);
    /// 域内凭据/密钥面 = [`ObjectZone::Sensitive`] (敏感面拒绝词表)。
    pub fn zone_of(&self, path: &Path) -> ObjectZone {
        let Some(relative) = self.relative_to_root(path) else {
            return ObjectZone::OutsideWorkspace;
        };
        if relative.is_empty() {
            return ObjectZone::InsideWorkspace;
        }
        if crate::sensitive_path::is_sensitive_path(Path::new(""), Path::new(&relative)) {
            return ObjectZone::Sensitive;
        }
        ObjectZone::InsideWorkspace
    }

    /// 组一次请求时判定输入 (域随之填好)。
    pub fn request<'a>(
        &'a self,
        subject: &'a ObjectSubject,
        object: &'a Path,
        action: ObjectAction,
    ) -> ObjectAccessRequest<'a> {
        ObjectAccessRequest {
            subject,
            object,
            action,
            zone: self.zone_of(object),
        }
    }

    fn relative_to_root(&self, path: &Path) -> Option<String> {
        let root_key = path_key(&self.root);
        let key = path_key(path);
        if key == root_key {
            return Some(String::new());
        }
        let prefix = format!("{root_key}/");
        key.strip_prefix(prefix.as_str()).map(str::to_string)
    }
}

/// 路径对象域分区。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectZone {
    /// 域内普通对象 (工作区内、非敏感面)。
    InsideWorkspace,
    /// 敏感对象域 (凭据/密钥面, 复用敏感面拒绝词表)。
    Sensitive,
    /// 域外对象 (路径对象域之外, 含拼写对不上域根的形态)。
    OutsideWorkspace,
}

impl ObjectZone {
    /// 稳定词面 (审计行用)。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InsideWorkspace => "workspace",
            Self::Sensitive => "sensitive",
            Self::OutsideWorkspace => "outside_domain",
        }
    }
}

/// 受控执行的问责主体 (进程/执行体标识)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectSubject {
    /// 进程/执行体的可问责名 (入 `sandbox.object_violation` 审计行 `process=`)。
    pub process: String,
}

impl ObjectSubject {
    /// 一枚主体。
    pub fn new(process: impl Into<String>) -> Self {
        Self {
            process: process.into(),
        }
    }
}

/// 一次对象级访问判定输入: 谁对哪个对象做什么 (域随附)。
#[derive(Debug, Clone, Copy)]
pub struct ObjectAccessRequest<'a> {
    /// 谁 (受控进程 / 智能体)。
    pub subject: &'a ObjectSubject,
    /// 哪个对象 (路径对象)。
    pub object: &'a Path,
    /// 做什么 (四动作词表)。
    pub action: ObjectAction,
    /// 对象落在哪个路径对象域。
    pub zone: ObjectZone,
}

/// 对象级裁决 (闭合二值: 放行 / 拒绝 + 理由)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectVerdict {
    /// 放行。
    Allow,
    /// 拒绝, 携带策略原话 (入帧与审计)。
    Deny {
        /// 拒绝理由。
        reason: String,
    },
}

impl ObjectVerdict {
    /// 是否放行。
    pub const fn is_allow(&self) -> bool {
        matches!(self, Self::Allow)
    }

    /// 一枚拒绝裁决。
    pub fn deny(reason: impl Into<String>) -> Self {
        Self::Deny {
            reason: reason.into(),
        }
    }

    /// 拒绝理由 (放行 = `None`)。
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Allow => None,
            Self::Deny { reason } => Some(reason.as_str()),
        }
    }
}

/// 对象级访问策略挂载点 (星港线深望智能体接此契约)。
///
/// 裁决输入是**对象** (谁/对象/动作/域), 不是 API 形态 —— 实现方按对象与
/// 动作立规则, 与调用路径无关; 判定必须是纯裁决 (无 IO、无副作用)。
pub trait ObjectAccessPolicy: Send + Sync {
    /// 策略稳定名 (入帧文案与审计行)。
    fn name(&self) -> &str;

    /// 裁决一次对象访问。
    fn decide(&self, request: &ObjectAccessRequest<'_>) -> ObjectVerdict;
}

/// 默认工作区对象策略: 域内普通对象放行; 敏感对象一律拒; 域外对象一律拒。
#[derive(Debug, Clone, Copy, Default)]
pub struct WorkspaceObjectPolicy;

impl ObjectAccessPolicy for WorkspaceObjectPolicy {
    fn name(&self) -> &str {
        "workspace_object_policy"
    }

    fn decide(&self, request: &ObjectAccessRequest<'_>) -> ObjectVerdict {
        match request.zone {
            ObjectZone::InsideWorkspace => ObjectVerdict::Allow,
            ObjectZone::Sensitive => ObjectVerdict::deny(format!(
                "敏感对象域不可{}（对象级安全契约, 凭据/密钥面 fail-closed）",
                request.action.as_str()
            )),
            ObjectZone::OutsideWorkspace => ObjectVerdict::deny(format!(
                "对象越界: 路径对象域之外的对象不可{}（工作区对象域）",
                request.action.as_str()
            )),
        }
    }
}

/// 策略链: 顺序判定, **第一个拒绝即拒绝**, 全部放行才放行。
///
/// 用来把附加策略 (如深望智能体的对象规则) 叠加在默认策略之上, 不替换、不覆盖。
#[derive(Default)]
pub struct ObjectPolicyChain {
    policies: Vec<Arc<dyn ObjectAccessPolicy>>,
}

impl fmt::Debug for ObjectPolicyChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObjectPolicyChain")
            .field(
                "policies",
                &self.policies.iter().map(|p| p.name()).collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl ObjectPolicyChain {
    /// 以给定顺序构造策略链。
    pub fn new(policies: Vec<Arc<dyn ObjectAccessPolicy>>) -> Self {
        Self { policies }
    }

    /// 链上的策略 (判定顺序)。
    pub fn policies(&self) -> &[Arc<dyn ObjectAccessPolicy>] {
        &self.policies
    }
}

impl ObjectAccessPolicy for ObjectPolicyChain {
    fn name(&self) -> &str {
        "object_policy_chain"
    }

    fn decide(&self, request: &ObjectAccessRequest<'_>) -> ObjectVerdict {
        for policy in &self.policies {
            if let ObjectVerdict::Deny { reason } = policy.decide(request) {
                return ObjectVerdict::deny(format!("[{}] {reason}", policy.name()));
            }
        }
        ObjectVerdict::Allow
    }
}

/// 对象状态探针 (执行前后各取一次域快照)。
pub trait ObjectStateProbe: Send + Sync {
    /// 取一个域快照。失败 = 对象事实不可核验 (调用方 fail-closed 处置)。
    fn snapshot(&self, domain: &ObjectDomain) -> std::io::Result<ObjectSnapshot>;
}

/// 生产对象状态探针: 域内全树**元数据**快照 (不读文件内容, 不跟随符号链接)。
///
/// 覆盖上限 [`FsObjectStateProbe::DEFAULT_MAX_ENTRIES`]: 触达上限或子树不可读
/// 时快照标注覆盖不完整 ([`ObjectSnapshot::complete`] = false), 如实打折、不假装全量。
#[derive(Debug, Clone, Copy)]
pub struct FsObjectStateProbe {
    max_entries: usize,
}

impl Default for FsObjectStateProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl FsObjectStateProbe {
    /// 默认对象条目上限。
    pub const DEFAULT_MAX_ENTRIES: usize = 200_000;

    /// 默认探针 (元数据三元组指纹)。
    pub fn new() -> Self {
        Self {
            max_entries: Self::DEFAULT_MAX_ENTRIES,
        }
    }

    /// 自定对象条目上限。
    pub fn with_max_entries(mut self, max_entries: usize) -> Self {
        self.max_entries = max_entries;
        self
    }
}

impl ObjectStateProbe for FsObjectStateProbe {
    fn snapshot(&self, domain: &ObjectDomain) -> std::io::Result<ObjectSnapshot> {
        let mut snapshot = ObjectSnapshot::new(domain.root());
        let mut stack = vec![(domain.root().to_path_buf(), String::new())];
        let mut seen = 0usize;
        'walk: while let Some((dir, prefix)) = stack.pop() {
            let entries = match std::fs::read_dir(&dir) {
                Ok(entries) => entries,
                Err(error) => {
                    if prefix.is_empty() {
                        return Err(error);
                    }
                    snapshot.set_complete(false);
                    continue;
                }
            };
            for entry in entries {
                let Ok(entry) = entry else {
                    snapshot.set_complete(false);
                    continue;
                };
                if seen >= self.max_entries {
                    snapshot.set_complete(false);
                    break 'walk;
                }
                seen += 1;
                let name = entry.file_name().to_string_lossy().into_owned();
                let key = if prefix.is_empty() {
                    name
                } else {
                    format!("{prefix}/{name}")
                };
                let Ok(metadata) = entry.metadata() else {
                    snapshot.set_complete(false);
                    continue;
                };
                if metadata.is_dir() {
                    snapshot.insert(key.clone(), ObjectFingerprint::directory());
                    stack.push((entry.path(), key));
                } else {
                    snapshot.insert(key, ObjectFingerprint::from_metadata(&metadata));
                }
            }
        }
        Ok(snapshot)
    }
}

/// 一条未授权对象操作 (对象越界事实)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectViolation {
    /// 越界对象 (路径对象)。
    pub object: PathBuf,
    /// 未授权动作 (四动作词表)。
    pub action: ObjectAction,
    /// 进程/执行体 (问责主体)。
    pub process: String,
    /// 对象落在哪个路径对象域。
    pub zone: ObjectZone,
    /// 策略拒绝原话。
    pub reason: String,
}

impl ObjectViolation {
    /// `sandbox.object_violation` 审计行细节: 对象/动作/进程/域/理由。
    pub fn audit_detail(&self) -> String {
        format!(
            "object={} action={} process={} zone={} reason={}",
            self.object.display(),
            self.action.as_str(),
            self.process,
            self.zone.as_str(),
            self.reason
        )
    }
}

/// 对象审计挂载点: 写审计行 (`事件名 空格 细节`, 与网关审计行同形)。
///
/// 无审计面的实现 = 不写、不伪造; 进程内可查面见 [`ObjectAuditSink::rows`]。
pub trait ObjectAuditSink: Send + Sync {
    /// 写一条审计行。
    fn record(&self, event: &str, detail: &str);

    /// 已记审计行 (记录顺序)。落盘/远端 sink 无回读面时如实返回空。
    fn rows(&self) -> Vec<String> {
        Vec::new()
    }
}

/// 进程内审计 sink (默认落点): 行进内存表, 重启即重置 —— 与读前观测表同一边界。
#[derive(Debug, Default)]
pub struct MemoryObjectAuditSink {
    rows: Mutex<Vec<String>>,
}

impl MemoryObjectAuditSink {
    /// 空审计表。
    pub fn new() -> Self {
        Self::default()
    }
}

impl ObjectAuditSink for MemoryObjectAuditSink {
    fn record(&self, event: &str, detail: &str) {
        let row = if detail.is_empty() {
            event.to_string()
        } else {
            format!("{event} {detail}")
        };
        self.rows
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(row);
    }

    fn rows(&self) -> Vec<String> {
        self.rows
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

/// 判定窗开启失败 —— fail-closed: 受控执行**不启动**。
#[derive(Debug, thiserror::Error)]
pub enum ObjectGuardError {
    /// 执行前快照不可得 (域根不可读), 对象事实无法核验。
    #[error("object judgment window unavailable (fail-closed, execution not started): {0}")]
    Unobservable(String),
}

/// 事实核验报告: 动了哪些对象 (事实) + 哪些越界 (违规)。
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectGuardReport {
    changes: Vec<ObjectChange>,
    violations: Vec<ObjectViolation>,
    probe_error: Option<String>,
    complete: bool,
}

impl ObjectGuardReport {
    /// 事实变更集 (创建/修改/删除, 键序确定)。
    pub fn changes(&self) -> &[ObjectChange] {
        &self.changes
    }

    /// 未授权对象操作 (越界事实, 已随判定落审计行)。
    pub fn violations(&self) -> &[ObjectViolation] {
        &self.violations
    }

    /// 执行后快照失败的原因 (失败即帧的 fail-closed 面)。
    pub fn probe_error(&self) -> Option<&str> {
        self.probe_error.as_deref()
    }

    /// 快照覆盖是否完整 (false = 触达条目上限或有子树不可读)。
    pub fn complete(&self) -> bool {
        self.complete
    }

    /// 是否存在真违规 (对象越界事实)。
    pub fn is_violation(&self) -> bool {
        !self.violations.is_empty()
    }

    /// 是否判失败 (违规, 或判定面不可用 —— 后者 fail-closed)。
    pub fn is_failure(&self) -> bool {
        self.failure().is_some()
    }

    /// 失败即帧: 走闭合失败词表出帧 (`pipeline.post_block`,
    /// source = `object_access_judge`), 理由串以 `object violation:` 起头、
    /// 逐条列对象/动作/进程。合规执行返回 `None` (结果零变化)。
    pub fn failure(&self) -> Option<PipelineFailure> {
        if let Some(error) = &self.probe_error {
            return Some(PipelineFailure::PostBlocked {
                source: OBJECT_JUDGE_SOURCE.to_string(),
                reason: format!("object judgment unavailable (fail-closed): {error}"),
            });
        }
        if self.violations.is_empty() {
            return None;
        }
        let details = self
            .violations
            .iter()
            .map(ObjectViolation::audit_detail)
            .collect::<Vec<_>>()
            .join("; ");
        Some(PipelineFailure::PostBlocked {
            source: OBJECT_JUDGE_SOURCE.to_string(),
            reason: format!("object violation: {details}"),
        })
    }
}

/// 对象级判定器: 策略 (挂载点) + 状态探针 + 审计 sink。
///
/// 无可变共享状态 —— 并发安全面; 快照即值, 各执行窗互不串扰。
pub struct ObjectAccessJudge {
    policy: Arc<dyn ObjectAccessPolicy>,
    probe: Arc<dyn ObjectStateProbe>,
    audit: Arc<dyn ObjectAuditSink>,
}

impl fmt::Debug for ObjectAccessJudge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObjectAccessJudge")
            .field("policy", &self.policy.name())
            .finish_non_exhaustive()
    }
}

impl ObjectAccessJudge {
    /// 以给定策略/探针/审计面构造判定器 (全部可挂载替换)。
    pub fn new(
        policy: Arc<dyn ObjectAccessPolicy>,
        probe: Arc<dyn ObjectStateProbe>,
        audit: Arc<dyn ObjectAuditSink>,
    ) -> Self {
        Self {
            policy,
            probe,
            audit,
        }
    }

    /// 默认判定器: 工作区对象策略 + 元数据快照探针 + 进程内审计。
    pub fn with_defaults() -> Self {
        Self::new(
            Arc::new(WorkspaceObjectPolicy),
            Arc::new(FsObjectStateProbe::new()),
            Arc::new(MemoryObjectAuditSink::new()),
        )
    }

    /// 挂载的策略。
    pub fn policy(&self) -> &Arc<dyn ObjectAccessPolicy> {
        &self.policy
    }

    /// 挂载的探针。
    pub fn probe(&self) -> &Arc<dyn ObjectStateProbe> {
        &self.probe
    }

    /// 挂载的审计面。
    pub fn audit(&self) -> &Arc<dyn ObjectAuditSink> {
        &self.audit
    }

    /// 已记审计行 (进程内可查面)。
    pub fn audit_rows(&self) -> Vec<String> {
        self.audit.rows()
    }

    /// 请求时判定 (声明面: 枚举/预声明动作) —— 与 API 形态无关。
    pub fn check(&self, request: &ObjectAccessRequest<'_>) -> ObjectVerdict {
        self.policy.decide(request)
    }

    /// 开一个事实判定窗 (执行前快照)。快照不可得 = [`ObjectGuardError`] (fail-closed)。
    pub fn open(
        &self,
        subject: ObjectSubject,
        domain: ObjectDomain,
    ) -> Result<ObjectWindow<'_>, ObjectGuardError> {
        let before = self
            .probe
            .snapshot(&domain)
            .map_err(|error| ObjectGuardError::Unobservable(error.to_string()))?;
        Ok(ObjectWindow {
            judge: self,
            subject,
            domain,
            before,
        })
    }

    /// 事实判定一把罩: 开窗 → 执行闭包 (受控执行本体, 形态无关) → 核验。
    ///
    /// 窗开不出时闭包**不执行** (fail-closed); 窗内执行照常返回, 核验结果在报告里。
    pub fn guard<T>(
        &self,
        subject: ObjectSubject,
        domain: ObjectDomain,
        run: impl FnOnce() -> T,
    ) -> Result<(T, ObjectGuardReport), ObjectGuardError> {
        let window = self.open(subject, domain)?;
        Ok(window.run(run))
    }
}

/// 一个事实判定窗: 执行前快照 → (受控执行) → 执行后核验。
pub struct ObjectWindow<'j> {
    judge: &'j ObjectAccessJudge,
    subject: ObjectSubject,
    domain: ObjectDomain,
    before: ObjectSnapshot,
}

impl<'j> ObjectWindow<'j> {
    /// 问责主体。
    pub fn subject(&self) -> &ObjectSubject {
        &self.subject
    }

    /// 对象域。
    pub fn domain(&self) -> &ObjectDomain {
        &self.domain
    }

    /// 执行前快照。
    pub fn before(&self) -> &ObjectSnapshot {
        &self.before
    }

    /// 包住一段执行再核验 (形态无关: 闭包内外动了什么对象就是什么事实)。
    pub fn run<T>(self, run: impl FnOnce() -> T) -> (T, ObjectGuardReport) {
        let output = run();
        let report = self.verify();
        (output, report)
    }

    /// 执行后核验: 取后快照 → 事实差分 → 逐对象过策略 → 越界即审计。
    ///
    /// 执行后快照失败 = 判定面不可用 → 报告带 [`ObjectGuardReport::probe_error`],
    /// 调用方按 fail-closed 判失败。
    pub fn verify(self) -> ObjectGuardReport {
        let after = match self.judge.probe.snapshot(&self.domain) {
            Ok(after) => after,
            Err(error) => {
                return ObjectGuardReport {
                    changes: Vec::new(),
                    violations: Vec::new(),
                    probe_error: Some(error.to_string()),
                    complete: false,
                };
            }
        };
        let complete = self.before.complete() && after.complete();
        let changes = diff_object_states(&self.before, &after);
        let mut violations = Vec::new();
        for change in &changes {
            let request = ObjectAccessRequest {
                subject: &self.subject,
                object: &change.object,
                action: change.action,
                zone: self.domain.zone_of(&change.object),
            };
            if let ObjectVerdict::Deny { reason } = self.judge.check(&request) {
                let violation = ObjectViolation {
                    object: change.object.clone(),
                    action: change.action,
                    process: self.subject.process.clone(),
                    zone: request.zone,
                    reason,
                };
                self.judge
                    .audit
                    .record(OBJECT_VIOLATION_AUDIT_EVENT, &violation.audit_detail());
                violations.push(violation);
            }
        }
        ObjectGuardReport {
            changes,
            violations,
            probe_error: None,
            complete,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fp(len: u64) -> ObjectFingerprint {
        ObjectFingerprint {
            kind: ObjectKind::File,
            len,
            mtime: UNIX_EPOCH + std::time::Duration::from_secs(1),
        }
    }

    #[test]
    fn action_vocabulary_is_frozen_at_four_words() {
        // 词表即承诺: 四动作逐一有稳定词面, 矩阵按 ALL 展开。
        let words: Vec<&str> = ObjectAction::ALL.iter().map(|a| a.as_str()).collect();
        assert_eq!(words, ["create", "modify", "delete", "enumerate"]);
    }

    #[test]
    fn diff_is_a_pure_object_fact_table() {
        let before = {
            let mut s = ObjectSnapshot::new("root");
            s.insert("a.txt", fp(1));
            s.insert("b.txt", fp(1));
            s.insert("c.txt", fp(1));
            s
        };
        let after = {
            let mut s = ObjectSnapshot::new("root");
            s.insert("a.txt", fp(2));
            s.insert("b.txt", fp(1));
            s.insert("d.txt", fp(1));
            s
        };
        let changes = diff_object_states(&before, &after);
        let rendered: Vec<(ObjectAction, String)> = changes
            .iter()
            .map(|c| (c.action, c.object.to_string_lossy().replace('\\', "/")))
            .collect();
        assert_eq!(
            rendered,
            [
                (ObjectAction::Modify, "root/a.txt".to_string()),
                (ObjectAction::Delete, "root/c.txt".to_string()),
                (ObjectAction::Create, "root/d.txt".to_string()),
            ]
        );
    }

    #[test]
    fn zones_classify_inside_sensitive_and_outside() {
        let domain = ObjectDomain::new("root");
        assert_eq!(
            domain.zone_of(Path::new("root/notes.txt")),
            ObjectZone::InsideWorkspace
        );
        assert_eq!(
            domain.zone_of(Path::new("root/.env")),
            ObjectZone::Sensitive
        );
        assert_eq!(
            domain.zone_of(Path::new("root/data/creds.json")),
            ObjectZone::Sensitive
        );
        assert_eq!(
            domain.zone_of(Path::new("other/notes.txt")),
            ObjectZone::OutsideWorkspace
        );
    }
}
