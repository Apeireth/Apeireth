//! 进程树快照面（C-B1 · `tool.process_tree` 的读面 + 幂等取消面）。
//!
//! 本模块只补**两件事**（既有面不重建）：
//!
//! 1. **快照枚举**（只读）：列出**受控**子进程树 —— pid / 父 pid / 命令 / 状态 /
////    是否逃逸收容组。
//! 2. **按 pid 幂等取消**：取消一棵受控子进程树；重复取消**零副作用**；目标已退出
//!    时返回**明确结果**（不是错误）。
//!
//! 既有面（不重建）：`IsolationCapability::ProcessTreeContainment`（杀整树的隔离档位）
//! 与执行后端里真正的整树终止逻辑，仍在 `process/mod.rs` 与平台后端里。
//!
//! # 平台覆盖（0 装诚实）
//!
//! | 平台 | 快照枚举 | 幂等取消 | 收容组归属判定 |
//! |---|---|---|---|
//! | Linux | ✅ 真实现（读 `/proc`） | ✅ 真实现 | ✅ 可判定（进程组 id） |
//! | Windows | ✅ 真实现（系统进程枚举） | ✅ 真实现 | ⚠️ **不可判定**（收容容器句柄不对外暴露 ⇒ 记 `Undetermined`，不谎报） |
//! | macOS | ❌ **显式未实现** | ❌ **显式未实现** | ❌ 未实现 |
//! | 其他 | ❌ 显式未实现 | ❌ 显式未实现 | ❌ |
//!
//! 未实现的平台**显式报错**（[`ProcessTreeError::Unsupported`]），**绝不返回空快照冒充成功**。
//!
//! # 归属判定（越权防线）
//!
//! 「受控」= **本会话的进程树**：根 pid 必须是当前进程自身或其后代。
//! 判定沿父进程链上溯（链上出现当前进程 pid 即命中）；一路上溯到会话初始进程仍未命中
//! ⇒ **不是本会话的树 ⇒ 拒绝**（[`ProcessTreeError::NotControlled`]）。
//! ⇒ 取消面**不会**动到别的会话 / 别的账号起的进程。
//!
//! 判定失败一律**保守拒绝**（fail-closed）：父进程已退出而被重新挂靠的孤儿进程、
//! 命名空间外的 pid、pid 复用等情形，一律当作「不可取消」。
//!
//! # 幂等口径
//!
//! 取消台账记下（根 pid + 进程启动时刻）⇒ 同一棵树的第二次取消**直接返回台账结果**，
//! **不再发任何信号**；若 pid 已被系统复用（启动时刻不同）⇒ 视为新树，重新判定归属。
//!
//! # 治理口径
//!
//! 本面是**只读 + 受控取消**：读树不需要任何人工确认；取消只作用于**本会话自己的**树，
//! 且逐 pid 显式回报结果（终止 / 已退出 / 失败）。

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{LazyLock, Mutex};

use serde::{Deserialize, Serialize};

use super::PlatformKind;

/// 父进程链上溯的最大跳数（防御环与异常深度）。
const MAX_ANCESTOR_HOPS: usize = 256;

/// 取消台账的容量上限（只保留最近若干条，避免无界增长）。
const LEDGER_CAPACITY: usize = 64;

// ===========================================================================
// 公共契约（平台无关）
// ===========================================================================

/// 进程在快照里的状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessTreeStatus {
    /// 正在运行。
    Running,
    /// 可中断睡眠。
    Sleeping,
    /// 不可中断睡眠（等待内核资源）。
    Blocked,
    /// 已停止。
    Stopped,
    /// 已终止但尚未被收尸。
    Zombie,
    /// 已退出。
    Exited,
    /// 平台未提供或查询失败（不猜）。
    Unknown,
}

/// 进程相对**本树收容组**的归属。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupContainment {
    /// 仍在根进程所属的收容组内（整组终止能覆盖到它）。
    Contained,
    /// **已确认**脱离根进程所属的收容组（自建组 ⇒ 整组终止覆盖不到它）。
    Escaped,
    /// 该平台无法判定（**不谎报**未逃逸）。
    Undetermined,
}

/// 快照里的一行：一个受控子进程。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessTreeEntry {
    /// 进程 id。
    pub pid: u32,
    /// 父进程 id。
    pub parent_pid: u32,
    /// 命令行（可执行文件 + 参数；内核线程无命令行时退化为进程名）。
    pub command_line: String,
    /// 进程状态。
    pub status: ProcessTreeStatus,
    /// 该进程相对本树收容组的归属。
    pub group_containment: GroupContainment,
}

impl ProcessTreeEntry {
    /// 该进程是否**已确认**逃出本树的收容组。
    pub fn escaped_group(&self) -> bool {
        self.group_containment == GroupContainment::Escaped
    }
}

/// 一棵受控子进程树的快照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessTreeSnapshot {
    /// 树根 pid。
    pub root_pid: u32,
    /// 树内成员（根在前，其余按层级 breadth-first 排列）。
    pub entries: Vec<ProcessTreeEntry>,
}

impl ProcessTreeSnapshot {
    /// 树内成员数（含根）。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 树是否为空（恒为 `false`：根自身也是成员）。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 按 pid 查一个成员。
    pub fn find(&self, pid: u32) -> Option<&ProcessTreeEntry> {
        self.entries.iter().find(|entry| entry.pid == pid)
    }

    /// 根之外的所有成员。
    pub fn descendants(&self) -> impl Iterator<Item = &ProcessTreeEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.pid != self.root_pid)
    }

    /// 已确认逃出收容组的成员。
    pub fn escaped(&self) -> impl Iterator<Item = &ProcessTreeEntry> {
        self.entries.iter().filter(|entry| entry.escaped_group())
    }
}

/// 平台对进程树快照面的支持档。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessTreeSupport {
    /// 平台后端**真实现**了枚举 + 取消。
    Enumerable,
    /// 平台后端**未实现**该面（显式未实现 ⇒ 调用显式报错，不静默通过）。
    Unsupported,
}

/// 取消一棵受控子进程树的结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessTreeCancel {
    /// 本次取消**真实生效**。
    Cancelled {
        /// 树根 pid。
        root_pid: u32,
        /// 收到终止信号并确认存活的 pid。
        terminated: Vec<u32>,
        /// 取消时**已经退出**的 pid（无需再动）。
        already_exited: Vec<u32>,
        /// 取消**失败**（例如权限不足）的 pid。
        failed: Vec<u32>,
    },
    /// 幂等重复：同一棵树此前已取消过，本次**零副作用**（未发任何信号）。
    AlreadyCancelled {
        /// 树根 pid。
        root_pid: u32,
        /// 首次取消时终止的 pid（原样回放）。
        terminated: Vec<u32>,
    },
    /// 目标**已不存在**（已退出）⇒ 明确结果，不是错误。
    NotRunning {
        /// 树根 pid。
        root_pid: u32,
    },
}

impl ProcessTreeCancel {
    /// 本次调用是否**真实产生了终止副作用**。
    pub fn caused_termination(&self) -> bool {
        matches!(self, ProcessTreeCancel::Cancelled { .. })
    }
}

/// 进程树快照面的错误。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProcessTreeError {
    /// 平台后端**未实现**该面（显式未实现，不静默通过）。
    #[error("process tree snapshot is not implemented on this platform: {platform:?}")]
    Unsupported {
        /// 未实现的平台。
        platform: PlatformKind,
    },
    /// 目标**不属于本会话**的受控树 ⇒ 拒绝（越权防线）。
    #[error("pid {pid} does not belong to this session's controlled process tree")]
    NotControlled {
        /// 被拒的 pid。
        pid: u32,
    },
    /// 目标不存在。
    #[error("pid {pid} was not found")]
    NotFound {
        /// 未找到的 pid。
        pid: u32,
    },
    /// 拒绝取消（越权或目标非法）。
    #[error("refused to cancel pid {pid}: {reason}")]
    Refused {
        /// 被拒的 pid。
        pid: u32,
        /// 拒绝原因。
        reason: String,
    },
    /// 平台查询 / 终止失败。
    #[error("process tree operation failed: {message}")]
    Io {
        /// 失败详情。
        message: String,
    },
}

// ===========================================================================
// 公共入口
// ===========================================================================

/// 该平台是否实现了进程树快照面。
///
/// 未实现的平台返回 [`ProcessTreeSupport::Unsupported`]，且
/// [`process_tree_snapshot`] / [`cancel_process_tree`] 会**显式报错**。
pub fn process_tree_support() -> ProcessTreeSupport {
    backend::support()
}

/// 列出以 `root_pid` 为根的**受控**子进程树快照（只读面）。
///
/// * 根必须是**本会话**的进程（当前进程自身或其后代），否则
///   [`ProcessTreeError::NotControlled`]（越权防线）。
/// * 目标不存在 ⇒ [`ProcessTreeError::NotFound`]。
/// * 平台未实现 ⇒ [`ProcessTreeError::Unsupported`]。
pub fn process_tree_snapshot(root_pid: u32) -> Result<ProcessTreeSnapshot, ProcessTreeError> {
    if backend::support() == ProcessTreeSupport::Unsupported {
        return Err(ProcessTreeError::Unsupported {
            platform: platform_kind(),
        });
    }
    if root_pid == 0 {
        return Err(ProcessTreeError::Refused {
            pid: root_pid,
            reason: "pid 0 is not a process root".into(),
        });
    }

    let processes = backend::enumerate()?;
    let index = index_by_pid(&processes);
    let Some(root) = index.get(&root_pid) else {
        return Err(ProcessTreeError::NotFound { pid: root_pid });
    };
    if !owned_by_session(&index, root_pid) {
        return Err(ProcessTreeError::NotControlled { pid: root_pid });
    }

    let root_group = root.group_id;
    let tree = collect_tree(&index, root_pid);
    let entries = tree
        .iter()
        .map(|process| ProcessTreeEntry {
            pid: process.pid,
            parent_pid: process.parent_pid,
            command_line: process.command_line.clone(),
            status: process
                .status
                .unwrap_or_else(|| backend::status_of(process.pid)),
            group_containment: containment_of(root_group, process.group_id),
        })
        .collect();

    Ok(ProcessTreeSnapshot { root_pid, entries })
}

/// 按 pid **幂等**取消一棵受控子进程树。
///
/// * 重复取消同一棵树 ⇒ [`ProcessTreeCancel::AlreadyCancelled`]，**零副作用**。
/// * 目标已退出 ⇒ [`ProcessTreeCancel::NotRunning`]（明确结果，不是错误）。
/// * 目标不属于本会话 ⇒ [`ProcessTreeError::NotControlled`]（**拒绝即拒**，不越权）。
/// * 平台未实现 ⇒ [`ProcessTreeError::Unsupported`]。
pub fn cancel_process_tree(root_pid: u32) -> Result<ProcessTreeCancel, ProcessTreeError> {
    if backend::support() == ProcessTreeSupport::Unsupported {
        return Err(ProcessTreeError::Unsupported {
            platform: platform_kind(),
        });
    }
    if root_pid == 0 {
        return Err(ProcessTreeError::Refused {
            pid: root_pid,
            reason: "pid 0 is not a process root".into(),
        });
    }

    let processes = backend::enumerate()?;
    let index = index_by_pid(&processes);
    let current = index.get(&root_pid).copied();

    // 幂等台账：同一棵树（启动时刻一致）⇒ 直接回放首次结果，**不再发信号**。
    if let Some(record) = ledger_peek(root_pid, current.and_then(|process| process.start_time)) {
        return Ok(ProcessTreeCancel::AlreadyCancelled {
            root_pid,
            terminated: record.terminated,
        });
    }

    let Some(root) = current else {
        return Ok(ProcessTreeCancel::NotRunning { root_pid });
    };
    if !owned_by_session(&index, root_pid) {
        return Err(ProcessTreeError::NotControlled { pid: root_pid });
    }

    let tree = collect_tree(&index, root_pid);
    let self_pid = std::process::id();
    if tree.iter().any(|process| process.pid == self_pid) {
        return Err(ProcessTreeError::Refused {
            pid: root_pid,
            reason: "the target tree contains the current process".into(),
        });
    }

    // 逐个终止（深层优先：逆序遍历 ⇒ 子进程先于父进程）。
    let mut terminated = Vec::new();
    let mut already_exited = Vec::new();
    let mut failed = Vec::new();
    for process in tree.iter().rev() {
        match backend::terminate(process.pid) {
            Ok(true) => terminated.push(process.pid),
            Ok(false) => already_exited.push(process.pid),
            Err(_) => failed.push(process.pid),
        }
    }

    // 收容组兜底终止：抓「枚举之后才 fork 出来」的成员。
    // 只有在「根自己是组长」且「该组不是本进程所在组」且「不是 1 号组」时才发信号，
    // 避免误伤本进程所在的组或会话初始进程所在的组。
    if let Some(group) = root.group_id {
        if group == root_pid && Some(group) != backend::own_group() && group > 1 {
            let _ = backend::terminate_group(group);
        }
    }

    ledger_record(root_pid, root.start_time, terminated.clone());
    Ok(ProcessTreeCancel::Cancelled {
        root_pid,
        terminated,
        already_exited,
        failed,
    })
}

// ===========================================================================
// 平台无关的共用逻辑
// ===========================================================================

/// 平台后端提供的原始进程记录（平台机制名只落在平台后端里）。
struct RawProcess {
    pid: u32,
    parent_pid: u32,
    command_line: String,
    /// 平台若在枚举时即可给出状态则填 `Some`；否则由 [`backend::status_of`] 补查。
    status: Option<ProcessTreeStatus>,
    /// 收容组标识（平台不可判定时为 `None`）。
    group_id: Option<u32>,
    /// 进程启动时刻（用于识别 pid 复用；平台不提供时为 `None`）。
    start_time: Option<u64>,
}

/// 当前平台种类。
fn platform_kind() -> PlatformKind {
    super::platform_backend().0
}

/// 按 pid 建索引。
fn index_by_pid(processes: &[RawProcess]) -> BTreeMap<u32, &RawProcess> {
    processes
        .iter()
        .map(|process| (process.pid, process))
        .collect()
}

/// 目标是否属于**本会话**（当前进程自身或其后代）。
///
/// 沿父进程链上溯，链上出现当前进程 pid 即命中；一路上溯到会话初始进程仍未命中
/// ⇒ 不属于本会话 ⇒ 拒绝（fail-closed）。
fn owned_by_session(index: &BTreeMap<u32, &RawProcess>, root_pid: u32) -> bool {
    let self_pid = std::process::id();
    let mut cursor = root_pid;
    for _ in 0..MAX_ANCESTOR_HOPS {
        if cursor == self_pid {
            return true;
        }
        let Some(process) = index.get(&cursor) else {
            return false;
        };
        if process.parent_pid == cursor {
            // 自环（脏数据）⇒ 不可判定 ⇒ 拒绝。
            return false;
        }
        if process.parent_pid == 0 {
            return false;
        }
        cursor = process.parent_pid;
    }
    false
}

/// 收集以 `root_pid` 为根的整棵树（根在前，breadth-first）。
fn collect_tree<'a>(
    index: &'a BTreeMap<u32, &'a RawProcess>,
    root_pid: u32,
) -> Vec<&'a RawProcess> {
    let mut children: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for process in index.values() {
        children
            .entry(process.parent_pid)
            .or_default()
            .push(process.pid);
    }

    let mut collected = Vec::new();
    let mut seen: BTreeSet<u32> = BTreeSet::new();
    let mut queue: VecDeque<u32> = VecDeque::new();
    queue.push_back(root_pid);
    while let Some(pid) = queue.pop_front() {
        if !seen.insert(pid) {
            continue;
        }
        if let Some(process) = index.get(&pid) {
            collected.push(*process);
        }
        if let Some(kids) = children.get(&pid) {
            for kid in kids {
                queue.push_back(*kid);
            }
        }
    }
    collected
}

/// 判定成员相对根收容组的归属。
fn containment_of(root_group: Option<u32>, group: Option<u32>) -> GroupContainment {
    match (root_group, group) {
        (Some(root_group), Some(group)) if root_group == group => GroupContainment::Contained,
        (Some(_), Some(_)) => GroupContainment::Escaped,
        _ => GroupContainment::Undetermined,
    }
}

// ===========================================================================
// 取消台账（幂等）
// ===========================================================================

/// 取消台账的一条记录。
#[derive(Clone)]
struct CancelRecord {
    root_pid: u32,
    start_time: Option<u64>,
    terminated: Vec<u32>,
}

/// 取消台账（会话内）。
static CANCEL_LEDGER: LazyLock<Mutex<Vec<CancelRecord>>> = LazyLock::new(|| Mutex::new(Vec::new()));

/// 查台账里该 pid 的记录（并校验 pid 是否已被复用）。
///
/// * 记录存在且 pid 未被复用 ⇒ 返回记录（调用方回放幂等结果），**记录保留**
///   ⇒ 第 N 次重复取消依旧是幂等回放（零副作用）。
/// * pid 已被系统复用（启动时刻不同）⇒ 记录作废并删除，返回 `None`（按新树处理）。
fn ledger_peek(root_pid: u32, current_start_time: Option<u64>) -> Option<CancelRecord> {
    let mut ledger = CANCEL_LEDGER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(position) = ledger.iter().position(|record| record.root_pid == root_pid) else {
        return None;
    };
    let record = ledger[position].clone();
    let same_tree = match (record.start_time, current_start_time) {
        (Some(recorded), Some(current)) => recorded == current,
        // 平台不提供启动时刻 ⇒ 无法区分复用 ⇒ 保守按「同一棵树」处理（不重复动它）。
        _ => true,
    };
    if same_tree {
        Some(record)
    } else {
        ledger.remove(position);
        None
    }
}

/// 记入取消台账。
fn ledger_record(root_pid: u32, start_time: Option<u64>, terminated: Vec<u32>) {
    let mut ledger = CANCEL_LEDGER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    ledger.retain(|record| record.root_pid != root_pid);
    ledger.push(CancelRecord {
        root_pid,
        start_time,
        terminated,
    });
    if ledger.len() > LEDGER_CAPACITY {
        ledger.remove(0);
    }
}

// ===========================================================================
// 平台后端
//
// 平台机制名（收容组 / 进程组 / 作业容器等具体叫法）只落在下面各后端里，
// 公共契约面（`process/mod.rs` 与本文件的类型区）只写平台无关的能力语义。
// ===========================================================================

#[cfg(target_os = "linux")]
mod backend {
    //! Linux 进程树后端：数据来源 = `/proc`（真实现，**0 新外部依赖**）。
    #![allow(unsafe_code)]

    use std::fs;

    use super::{ProcessTreeError, ProcessTreeStatus, ProcessTreeSupport, RawProcess};

    /// `/proc/<pid>/stat` 里 `comm` 之后的字段序号：`starttime` 是第 22 个字段，
    /// 而 `comm` 之后的头一个字段（状态）是第 3 个 ⇒ 偏移 19。
    const STAT_START_TIME_INDEX: usize = 19;

    pub(super) fn support() -> ProcessTreeSupport {
        ProcessTreeSupport::Enumerable
    }

    pub(super) fn enumerate() -> Result<Vec<RawProcess>, ProcessTreeError> {
        let dir = fs::read_dir("/proc").map_err(|error| ProcessTreeError::Io {
            message: format!("cannot list /proc: {error}"),
        })?;
        let mut processes = Vec::new();
        for entry in dir.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            let Ok(pid) = name.parse::<u32>() else {
                continue;
            };
            if let Some(process) = read_process(pid) {
                processes.push(process);
            }
        }
        Ok(processes)
    }

    pub(super) fn status_of(pid: u32) -> ProcessTreeStatus {
        read_stat(pid)
            .map(|stat| stat.status)
            .unwrap_or(ProcessTreeStatus::Unknown)
    }

    /// 本进程所在的进程组 id（用于「绝不误杀本进程所在组」的自保判定）。
    pub(super) fn own_group() -> Option<u32> {
        let group = unsafe { libc::getpgid(0) };
        if group < 0 {
            None
        } else {
            Some(group as u32)
        }
    }

    /// 终止单个进程。
    ///
    /// `Ok(true)` = 信号已送达；`Ok(false)` = 目标已不存在；`Err` = 失败（例如权限不足）。
    pub(super) fn terminate(pid: u32) -> Result<bool, ProcessTreeError> {
        if pid <= 1 {
            return Err(ProcessTreeError::Refused {
                pid,
                reason: "refuse to signal a protected pid".into(),
            });
        }
        let result = unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
        if result == 0 {
            return Ok(true);
        }
        let error = std::io::Error::last_os_error();
        match error.raw_os_error() {
            Some(libc::ESRCH) => Ok(false),
            _ => Err(ProcessTreeError::Io {
                message: format!("terminate pid {pid} failed: {error}"),
            }),
        }
    }

    /// 终止整个进程组（负 pid 形式）。
    pub(super) fn terminate_group(group_id: u32) -> Result<bool, ProcessTreeError> {
        if group_id <= 1 {
            return Err(ProcessTreeError::Refused {
                pid: group_id,
                reason: "refuse to signal a protected process group".into(),
            });
        }
        let result = unsafe { libc::kill(-(group_id as libc::pid_t), libc::SIGKILL) };
        if result == 0 {
            return Ok(true);
        }
        let error = std::io::Error::last_os_error();
        match error.raw_os_error() {
            Some(libc::ESRCH) => Ok(false),
            _ => Err(ProcessTreeError::Io {
                message: format!("terminate process group {group_id} failed: {error}"),
            }),
        }
    }

    /// 读取一个进程的原始记录。
    fn read_process(pid: u32) -> Option<RawProcess> {
        let stat = read_stat(pid)?;
        let command_line = read_cmdline(pid).unwrap_or_else(|| stat.comm.clone());
        Some(RawProcess {
            pid,
            parent_pid: stat.ppid,
            command_line,
            status: Some(stat.status),
            group_id: Some(stat.pgrp),
            start_time: Some(stat.start_time),
        })
    }

    /// `/proc/<pid>/stat` 的解析结果。
    struct ProcStat {
        comm: String,
        status: ProcessTreeStatus,
        ppid: u32,
        pgrp: u32,
        start_time: u64,
    }

    /// 解析 `/proc/<pid>/stat`。
    ///
    /// 格式：`pid (comm) state ppid pgrp session ...`；`comm` 自身可能含空格与括号，
    /// 因此用「最后一个右括号」切分。
    fn read_stat(pid: u32) -> Option<ProcStat> {
        let raw = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        let open = raw.find('(')?;
        let close = raw.rfind(')')?;
        let comm = raw.get(open + 1..close)?.to_string();
        let rest = raw.get(close + 1..)?;
        let fields: Vec<&str> = rest.split_whitespace().collect();

        let status = match fields.first().copied()? {
            "R" => ProcessTreeStatus::Running,
            "S" => ProcessTreeStatus::Sleeping,
            "D" => ProcessTreeStatus::Blocked,
            "T" | "t" => ProcessTreeStatus::Stopped,
            "Z" => ProcessTreeStatus::Zombie,
            "X" | "x" => ProcessTreeStatus::Exited,
            _ => ProcessTreeStatus::Unknown,
        };
        let ppid = fields.get(1)?.parse().ok()?;
        let pgrp = fields.get(2)?.parse().ok()?;
        let start_time = fields.get(STAT_START_TIME_INDEX)?.parse().ok()?;
        Some(ProcStat {
            comm,
            status,
            ppid,
            pgrp,
            start_time,
        })
    }

    /// 读取 `/proc/<pid>/cmdline`（NUL 分隔的命令行）。
    fn read_cmdline(pid: u32) -> Option<String> {
        let raw = fs::read(format!("/proc/{pid}/cmdline")).ok()?;
        let parts: Vec<String> = raw
            .split(|byte| *byte == 0)
            .filter(|part| !part.is_empty())
            .map(|part| String::from_utf8_lossy(part).into_owned())
            .collect();
        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" "))
        }
    }
}

#[cfg(windows)]
mod backend {
    //! Windows 进程树后端：数据来源 = 系统进程枚举（真实现，**0 新外部依赖**）。
    #![allow(unsafe_code)]
    //!
    //! ⚠️ **收容组归属不可判定**：执行侧的收容容器句柄不对外暴露 ⇒ 本后端拿不到
    //! 「某个进程是否仍在该容器内」的判据 ⇒ 组归属一律记 `Undetermined`（**不谎报**）。

    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, TerminateProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        PROCESS_TERMINATE,
    };

    use super::{ProcessTreeError, ProcessTreeStatus, ProcessTreeSupport, RawProcess};

    /// 进程仍在运行时 `GetExitCodeProcess` 返回的退出码。
    const STILL_ACTIVE: u32 = 259;

    pub(super) fn support() -> ProcessTreeSupport {
        ProcessTreeSupport::Enumerable
    }

    pub(super) fn enumerate() -> Result<Vec<RawProcess>, ProcessTreeError> {
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot == INVALID_HANDLE_VALUE {
                return Err(ProcessTreeError::Io {
                    message: "cannot open a process enumeration snapshot".into(),
                });
            }
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            let mut processes = Vec::new();
            if Process32FirstW(snapshot, &mut entry) != 0 {
                loop {
                    processes.push(RawProcess {
                        pid: entry.th32ProcessID,
                        parent_pid: entry.th32ParentProcessID,
                        command_line: image_name(&entry),
                        status: None,
                        group_id: None,
                        start_time: None,
                    });
                    if Process32NextW(snapshot, &mut entry) == 0 {
                        break;
                    }
                }
            }
            CloseHandle(snapshot);
            Ok(processes)
        }
    }

    pub(super) fn status_of(pid: u32) -> ProcessTreeStatus {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle.is_null() {
                return ProcessTreeStatus::Unknown;
            }
            let mut code: u32 = 0;
            let queried = GetExitCodeProcess(handle, &mut code);
            CloseHandle(handle);
            if queried == 0 {
                return ProcessTreeStatus::Unknown;
            }
            if code == STILL_ACTIVE {
                ProcessTreeStatus::Running
            } else {
                ProcessTreeStatus::Exited
            }
        }
    }

    /// Windows 侧没有可外查的「进程组」概念 ⇒ 无收容组自保判据。
    pub(super) fn own_group() -> Option<u32> {
        None
    }

    pub(super) fn terminate(pid: u32) -> Result<bool, ProcessTreeError> {
        if pid <= 4 {
            return Err(ProcessTreeError::Refused {
                pid,
                reason: "refuse to signal a protected pid".into(),
            });
        }
        unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
            if handle.is_null() {
                // 打不开 ⇒ 目标已不存在或无权限 ⇒ 记为「未能终止」，由调用方归入失败集。
                return Ok(false);
            }
            let terminated = TerminateProcess(handle, 1);
            CloseHandle(handle);
            Ok(terminated != 0)
        }
    }

    /// Windows 侧不按组终止（逐 pid 终止已覆盖树内成员）。
    pub(super) fn terminate_group(_group_id: u32) -> Result<bool, ProcessTreeError> {
        Ok(false)
    }

    /// 从枚举项里取可执行文件名。
    fn image_name(entry: &PROCESSENTRY32W) -> String {
        let name: Vec<u16> = entry
            .szExeFile
            .iter()
            .copied()
            .take_while(|unit| *unit != 0)
            .collect();
        String::from_utf16_lossy(&name)
    }
}

#[cfg(target_os = "macos")]
mod backend {
    //! macOS 进程树后端：**显式未实现**（0 装诚实）。
    //!
    //! 执行侧的 macOS 后端（进程组收容）是真实存在的，但**快照枚举 + 按 pid 幂等取消**
    //! 这一面尚未实现 ⇒ 本后端一律显式报错，绝不返回空快照冒充成功。

    use super::{ProcessTreeError, ProcessTreeStatus, ProcessTreeSupport, RawProcess};

    pub(super) fn support() -> ProcessTreeSupport {
        ProcessTreeSupport::Unsupported
    }

    pub(super) fn enumerate() -> Result<Vec<RawProcess>, ProcessTreeError> {
        Err(ProcessTreeError::Unsupported {
            platform: super::super::PlatformKind::MacOs,
        })
    }

    pub(super) fn status_of(_pid: u32) -> ProcessTreeStatus {
        ProcessTreeStatus::Unknown
    }

    pub(super) fn own_group() -> Option<u32> {
        None
    }

    pub(super) fn terminate(_pid: u32) -> Result<bool, ProcessTreeError> {
        Err(ProcessTreeError::Unsupported {
            platform: super::super::PlatformKind::MacOs,
        })
    }

    pub(super) fn terminate_group(_group_id: u32) -> Result<bool, ProcessTreeError> {
        Err(ProcessTreeError::Unsupported {
            platform: super::super::PlatformKind::MacOs,
        })
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod backend {
    //! 其他平台的进程树后端：**显式未实现**（0 装诚实）。

    use super::{ProcessTreeError, ProcessTreeStatus, ProcessTreeSupport, RawProcess};

    pub(super) fn support() -> ProcessTreeSupport {
        ProcessTreeSupport::Unsupported
    }

    pub(super) fn enumerate() -> Result<Vec<RawProcess>, ProcessTreeError> {
        Err(ProcessTreeError::Unsupported {
            platform: super::super::PlatformKind::Other,
        })
    }

    pub(super) fn status_of(_pid: u32) -> ProcessTreeStatus {
        ProcessTreeStatus::Unknown
    }

    pub(super) fn own_group() -> Option<u32> {
        None
    }

    pub(super) fn terminate(_pid: u32) -> Result<bool, ProcessTreeError> {
        Err(ProcessTreeError::Unsupported {
            platform: super::super::PlatformKind::Other,
        })
    }

    pub(super) fn terminate_group(_group_id: u32) -> Result<bool, ProcessTreeError> {
        Err(ProcessTreeError::Unsupported {
            platform: super::super::PlatformKind::Other,
        })
    }
}
