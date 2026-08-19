# `agentos-windows-recovery` 沙箱借鉴调研报告

**日期**：2026-08-19
**项目**：`Jimmyxiao2009/agentos-windows-recovery` (MIT, C#, 14 个 .cs, 58 KB)
**HEAD**：`ba30e4f` (单 commit initial release, 2026-07-16)
**作者画像**：个人小开发者，单 commit 工程，工程成熟度 ★☆☆☆☆，但**设计思路 ★★★★☆**
**读者**：Apeireth-rust 维护者（YintaTriss）
**本地路径**：`C:\Users\31683\.minimax-agent-cn\agentos-windows-recovery\`
**Apeireth 项目根**：`C:\Users\31683\Apeireth-rust\`

> 严守"借鉴上升不模仿"原则。本文只提取**设计模式**，不抄代码、不抄架构命名、不抄 LICENSE。

---

## 1. 项目定位（避免误读）

README 第一句：

> "Windows user-mode Agent transaction recovery runtime (file/registry/service/task scopes)"

定位很精确：**用户态、4 scope、事务恢复运行时**。明确写在 PLAN_GAP_MATRIX 里：

> "用户态声明 scope 的条件式恢复层，不是完整系统事务沙箱或 EDR 边界"

跟 Apeireth 的 7 层沙箱不在同一抽象层 —— 它**不解决 agent 推理安全**，只解决**写操作的副作用可回滚**。这恰好是 Apeireth 7 层沙箱里**最大的空白点**。

---

## 2. 10 大设计模式（提炼）

读 14 个 .cs + README + PLAN_GAP_MATRIX 后，提炼出 10 个可独立借鉴的设计模式：

| # | 模式 | 文件 | 行数/规模 |
|---|---|---|---|
| **D-1** | **Transaction 10 状态机** | `Models.cs:6` + `TransactionEngine.cs:39KB` | Created→Preparing→Active→Prepared→Committed / RollingBack→RolledBack / Conflicted / RecoveryRequired / Failed |
| **D-2** | **3-state snapshot + 三方冲突检测** | `FileSnapshotEngine.cs:41KB` | baseline / after / current 三方 diff；rollback 只能撤销 after-state 的差异，否则标记 Conflicted |
| **D-3** | **content-addressed blob store** | `TransactionStore.cs` | SHA-256 文件名 + dedup；`FileOptions.WriteThrough` 原子写 |
| **D-4** | **journal hash chain** | `TransactionJournal.cs` | 每条 ndjson 含 `prev_hash`，自校验断链 |
| **D-5** | **stage-then-verify fail-closed** | `FileSnapshotEngine.PrepareRestore` | rollback 前先把所有 recovery 内容复制到 staging 目录，SHA-256 全通过才执行第一个 write；中途失败 throw + 自动清 staging |
| **D-6** | **Windows Job Object 进程隔离** | `WindowsJobRunner.cs:8KB` | `JobObjectLimitKillOnJobClose` + `DieOnUnhandledException`；关 Job handle = 杀整个进程树 |
| **D-7** | **Sandboxie 适配器** | `SandboxieAdapter.cs:1.8KB` | 外部 Sandboxie 做 namespace 虚拟化，内部自己保 transaction journal（**清晰分层**） |
| **D-8** | **VSS 卷影复制（能力探测）** | `VssShadowCopyProvider.cs:6KB` | 需 elevated token；按 scope volume root 触发 `Win32_ShadowCopy.Create`；失败 fallback 到 FileSnapshotEngine |
| **D-9** | **DPAPI evidence MAC + ACL hardening** | `StoreProtection.cs` + `EvidenceIntegrity.cs` | store 根目录 ACL 锁 current user + Admin + SYSTEM；每个 transaction 自签 `evidence.mac` |
| **D-10** | **声明式 scope + 自动 prepare** | CLI `--file-scope --registry-scope --on-success` | 默认 `on_success=retain`（即 Prepared），需显式 commit/rollback；非 0 退出码 → 自动 rollback |

---

## 3. 跟 Apeireth 7 层沙箱对比

| 模式 | agentos-windows-recovery | Apeireth 对应层 | 借鉴价值 |
|---|---|---|---|
| **D-1 Transaction 状态机** | 10 状态完整 | ❌ **无**；`apeireth-action` 只有 3 trait (Execution/Expression/Silence)，PHL-02b 明确 `not_undo` | ★★★★★ |
| **D-2 三方冲突检测** | baseline+after+current diff | ❌ **无** | ★★★★★ |
| **D-3 content-addressed blob** | SHA-256 文件名 + dedup | ⚠️ `apeireth-credentials` 局部有，但**未抽象为通用 blob 层** | ★★★☆☆ |
| **D-4 journal hash chain** | ndjson + prev_hash 链 | ⚠️ `apeireth-supervisor` 有 Erlang OTP 重启日志，**非 cryptographic chain** | ★★★☆☆ |
| **D-5 stage-then-verify** | fail-closed 预检 | ❌ **无** | ★★★★☆ |
| **D-6 Job Object 进程隔离** | OS 级 | ❌ **完全无 OS 进程隔离层** | ★★★★★ |
| **D-7 Sandboxie 适配** | 第三方沙箱 | ❌ Apeireth **自建 7 层**，不接第三方 | ★☆☆☆☆（拒绝） |
| **D-8 VSS 卷影复制** | Windows-only | ❌ 跨平台 OS 不能锁 Windows API | ★★☆☆☆（仅 Windows 路径） |
| **D-9 DPAPI + ACL** | Windows-only 凭证 | ⚠️ `apeireth-credentials` 已有 chacha20poly1305 AEAD + keyring | ★★☆☆☆（已具备） |
| **D-10 声明式 scope** | CLI flags | ⚠️ `apeireth-tool-approval` 有 rule-based scope，**但非事务性** | ★★★★☆ |

### 关键空白（Apeireth 缺什么）

| 空白 | 影响 |
|---|---|
| **写操作无回滚能力** | agent 一旦写错文件/改错 config/调错 service，**永久副作用**，违反 PHL-01 (provable) |
| **无 OS 级进程隔离** | 子进程可逃逸 PII guard / tool approval / constraint 三层用户态检查 |
| **无 transactional state machine** | apeireth-action 的 Execution trait 是 fire-and-forget，**没有 Prepared 阶段让上层决策 commit/rollback** |

---

## 4. Top 3 借鉴点（按价值/成本比）

### **借鉴 1：Transaction 10 状态机 + 三方冲突检测** — 价值 ★★★★★

**哲学对齐**：PHL-02b `not_undo` 说的是 **agent 决策不可撤销**（forward-only thinking），但**写操作副作用应该是事务化的** —— 这两个不冲突，反而互补。决策 forward-only，副作用 transactional，两层分离。

**借鉴什么（思路，不是代码）**：
- 引入 10 状态机到 `apeireth-action`：`Created → Preparing → Active → Prepared → Committed / RollingBack → RolledBack / Conflicted / RecoveryRequired / Failed`
- D-2 三方冲突检测：每次 commit/rollback 前先 snapshot current，对比 after；不一致标 `Conflicted` 等待人工
- D-4 journal hash chain：把 `apeireth-supervisor` 的 Erlang 重启日志升级成 cryptographic hash chain

**不借鉴什么**：
- 不抄 `TransactionEngine.cs` 的代码（39KB C# 是 OO 风格，Apeireth 用 trait + state pattern）
- 不抄 Sandboxie 适配层（Apeireth 自建 7 层）

**决策矩阵**：

| 维度 | 评估 |
|---|---|
| 必要性 | PHL-01 provable 要求副作用可验证；无回滚 = 不可验证 |
| 可行性 | Rust 实现更简洁（~1.5KB 而非 39KB C#） |
| 风险 | 需要重新论证 PHL 文档（PHL-02b 表述要加 footnote） |
| 估时 | **2-3 周**（设计 + 实现 + 测试 + PHL 论证） |
| 路径 | `crates/apeireth-action/src/{transactional, journal, snapshot}.rs` 新增模块 |
| 不做代价 | agent 写错配置 / 删错文件无回滚，PHL-01 在写操作维度不成立 |

---

### **借鉴 2：Windows Job Object 进程隔离（Linux cgroup v2 对称）** — 价值 ★★★★★

**哲学对齐**：Apeireth 7 层沙箱**没有任何 OS 级进程隔离**。agent spawn 子进程跑 tool 时，子进程能逃逸所有用户态检查（PII guard / tool approval / constraint），这是真实攻击面。

**借鉴什么（思路，不是代码）**：
- Windows 路径：`AssignProcessToJobObject` + `JobObjectLimitKillOnJobClose` + `JobObjectLimitDieOnUnhandledException`
- Linux 路径：`cgroup v2 freezer` + `pidfd_send_signal(SIGKILL)` 对称
- macOS 路径：`sandbox_init(3)` + `sandbox-exec` profile
- 任何平台 fallback：`prctl(PR_SET_PDEATHSIG, SIGKILL)` 至少保证父死子亡

**不借鉴什么**：
- 不抄 `WindowsJobRunner.cs` 的 P/Invoke（Rust 用 `windows-sys` crate 更干净）
- 不抄 Sandboxie adapter（自建体系冲突）

**决策矩阵**：

| 维度 | 评估 |
|---|---|
| 必要性 | 7 层沙箱最大缺口；无 OS 隔离 = 用户态 7 层是 theater |
| 可行性 | Rust 跨平台 crate 模板成熟（`landlock` / `seccompiler` 已有） |
| 风险 | 误杀子进程 → 引入 grace period + supervision 兜底 |
| 估时 | **Windows 1 周 + Linux 1 周 + macOS 1 周**（3 周） |
| 路径 | `crates/apeireth-supervisor/src/{job_runner, cgroup_runner, sandbox_runner}.rs` |
| 不做代价 | 子进程逃逸 → 7 层纵深防御实质失效 |

---

### **借鉴 3：Content-Addressed Blob Store 抽象化** — 价值 ★★★☆☆

**哲学对齐**：`apeireth-credentials` 已经是 SHA-256 文件名 + dedup 的 content-addressing 设计（凭证 blob）。把同样的设计抽到通用层，给 snapshot/blob/credential 复用，是**认知一致性 + DRY**。

**借鉴什么（思路，不是代码）**：
- 抽 `apeireth-blob` crate，提供 `put(bytes) -> sha256` + `get(sha256) -> bytes` + `verify(sha256)`
- `apeireth-credentials` 现有逻辑迁过来（向后兼容）
- `apeireth-action` 的 snapshot 层（借鉴 1 的 D-2/D-3）直接依赖 `apeireth-blob`
- `apeireth-supervisor` 的 journal hash chain（D-4）也复用 `apeireth-blob` 的 hash 接口

**决策矩阵**：

| 维度 | 评估 |
|---|---|
| 必要性 | DRY + 认知一致；不是新功能 |
| 可行性 | 现成抽象，迁移成本低 |
| 风险 | `apeireth-credentials` 现有 keyring 集成要保留，不能简单替换 |
| 估时 | **1-2 周** |
| 路径 | 从 `crates/apeireth-credentials/src/{lib,store}.rs` 抽到新 `crates/apeireth-blob/src/` |
| 不做代价 | credentials / action / supervisor 三处各自实现 hash 文件名，重复 |

---

## 5. 明确**不**借鉴的项

| 项 | 不借鉴原因 |
|---|---|
| **Sandboxie adapter** (D-7) | Apeireth 自建 7 层纵深，引入第三方沙箱会破坏单一可信源 |
| **VSS 卷影复制** (D-8) | Windows-only；Apeireth 跨平台；Linux 等价是 LVM/btrfs snapshot（可选 backend，不进 core） |
| **DPAPI evidence MAC** (D-9) | `apeireth-credentials` 已有 chacha20poly1305 AEAD（更强），再加一层重复 |
| **NTFS ADS / hardlink group** | Windows 文件系统特性，不通用 |
| **reparse points 拒绝** | Windows 边界声明；跨平台无 reparse 概念 |

---

## 6. 借鉴 1 的 Rust 设计草图（仅思路）

```rust
// crates/apeireth-action/src/transactional.rs

pub enum TransactionState {
    Created, Preparing, Active, Prepared,
    Committed, RollingBack, RolledBack,
    Conflicted, RecoveryRequired, Failed,
}

pub trait TransactionalAction: Send + Sync {
    fn begin(&mut self, scope: &Scope) -> Result<TxId, ActionError>;
    fn prepare(&mut self, tx: TxId) -> Result<(), ActionError>;  // 三方冲突预检
    fn commit(&mut self, tx: TxId) -> Result<(), ActionError>;
    fn rollback(&mut self, tx: TxId) -> Result<(), ActionError>;
    fn state(&self, tx: TxId) -> TransactionState;
}

// 默认 on_success = Retain（Prepared），需显式 commit；exit != 0 → 自动 rollback
// 参考 agentos README： "Successful commands default to Prepared, so the caller
// must explicitly commit or roll back. A non-zero root-process exit triggers automatic rollback."
```

借鉴 `FileSnapshotEngine.FindRollbackConflicts` 的三方 diff 思路，对应 Apeireth 写 `ConflictDetector`：

```rust
// crates/apeireth-action/src/conflict.rs
pub fn detect_conflict(
    baseline: &Snapshot,
    after: &Snapshot,
    current: &Snapshot,
) -> Vec<Conflict> { /* 三方 diff, 只对 changed keys 比 after vs current */ }
```

借鉴 `TransactionJournal` 的 hash chain：

```rust
// crates/apeireth-supervisor/src/journal_chain.rs
pub struct JournalEntry {
    pub seq: u64,
    pub timestamp: DateTime<Utc>,
    pub event: String,
    pub data: String,
    pub prev_hash: String,  // "GENESIS" or hex(SHA256)
    pub hash: String,       // hex(SHA256(seq || ts || event || data || prev_hash))
}
```

---

## 7. 借鉴 2 的 Rust 设计草图

```rust
// crates/apeireth-supervisor/src/job_runner.rs (Windows)

#[cfg(windows)]
pub fn run_in_job(
    cmd: &str,
    args: &[&str],
    cwd: &Path,
) -> Result<i32, SupervisorError> {
    use windows_sys::Win32::System::JobObjects::*;
    // 1. CreateJobObjectW
    // 2. SetInformationJobObject with KillOnJobClose | DieOnUnhandledException
    // 3. CreateProcessW (CREATE_SUSPENDED)
    // 4. AssignProcessToJobObject
    // 5. ResumeThread
    // 6. WaitForSingleObject(handle, INFINITE)
    // 7. drop job handle → kills descendants
    // (完全镜像 WindowsJobRunner.cs)
}

#[cfg(target_os = "linux")]
pub fn run_in_cgroup(cmd: &str, args: &[&str], cwd: &Path) -> Result<i32, SupervisorError> {
    // 1. 创建 transient cgroup v2 scope
    // 2. fork+exec
    // 3. pidfd 监听
    // 4. parent exit / panic → SIGKILL to cgroup
}

#[cfg(target_os = "macos")]
pub fn run_in_sandbox(cmd: &str, args: &[&str], cwd: &Path) -> Result<i32, SupervisorError> {
    // 1. sandbox_init(SANDBOX_NAMED)
    // 2. fork+exec with profile
}
```

---

## 8. 决策矩阵汇总（按推荐度排序）

| 排名 | 借鉴项 | 价值 | 成本 | 推荐 | 时机 |
|---|---|---|---|---|---|
| 🥇 #1 | **Transaction 10 状态机 + 三方冲突检测** | ★★★★★ | 2-3 周 | **强推** | v1.1 (cl failed 修完后) |
| 🥈 #2 | **Windows Job Object + Linux cgroup + macOS sandbox** | ★★★★★ | 3 周 | **强推** | v1.1 |
| 🥉 #3 | **Content-Addressed Blob 抽象化** | ★★★☆☆ | 1-2 周 | 中等 | v1.1 (跟 #1 一起做，省依赖) |
| — | Sandboxie adapter | ★☆☆☆☆ | — | **拒绝** | — |
| — | VSS 卷影复制 | ★★☆☆☆ | — | **延后** | v2.0 (L0 HA 备份场景) |
| — | DPAPI MAC | ★★☆☆☆ | — | **拒绝** | 已有 chacha20poly1305 |

---

## 9. 跟 PHL 文档的关系

| 借鉴项 | 需要更新的 PHL |
|---|---|
| #1 Transaction 状态机 | **PHL-02b 加 footnote**：决策不可撤销 ≠ 副作用不可回滚；副作用事务化，决策 forward-only |
| #2 OS 进程隔离 | **PHL-07 (defense-in-depth) 强化**：7 层纵深补第 8 层 OS 隔离 |
| #3 Blob 抽象化 | 无 PHL 变更（纯工程） |

---

## 10. 风险与未决问题

1. **PHL-02b vs Transaction 回滚 的语义冲突**：要论证清"决策 forward-only"指的是 LLM 推理层（Council 决策、Action 选择），不是副作用执行层（文件写入、服务变更）。需用户决策。
2. **OS 进程隔离的边界**：要确定**什么粒度** —— 每 tool call 一进程 / 每 task 一进程 / 每 session 一进程？粒度越细越安全，但 fork 开销越大。
3. **三方冲突检测的 scope**：要确定 baseline / after 是什么的快照 —— 文件？注册表？环境变量？还是抽象的 `Action::SideEffect`？

---

## 11. 下一步

- [ ] **等你拍板**：借鉴 1 / 2 / 3 是否进 v1.1 backlog？
- [ ] **PHL-02b footnote** 论证（如果你认可借鉴 1）
- [ ] **OS 隔离粒度** 决策（如果你认可借鉴 2）
- [ ] **三方冲突 scope** 决策（如果你认可借鉴 1）

不动代码、不 commit、本报告存 `reports/` 等你审。