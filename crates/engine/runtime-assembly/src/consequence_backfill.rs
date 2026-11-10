//! 后果追踪 · 自动回填面（ADR-0014 §5 第 ③ 步）。
//!
//! 上位：`docs/04-internal/backend-enhancement/03-designs/accountability-loop-design-2026-10-23.md`
//! §2.3 与 owner 裁定（2026-10-25，D-6 选项 A 的第一步）：
//! **定义「行动 ↔ 影响」的归属语义，并只自动回填 `failed`**；`confirmed` 仍走人工/外部。
//!
//! # 为什么需要这一面
//!
//! 问责闭环第 ③ 步此前**只有手动入口**（`apeireth egress outcome <ACTION_ID> <confirmed|failed>`）：
//! 行动跑完后**没有任何东西**回填结果，账面永远停在 `Outcome::Unknown`。
//!
//! # 归属语义（1:1）
//!
//! 一次工具调用失败 ⇒ 把它**对应的那条**已声明影响回填 `failed`；
//! **找不到对应声明 ⇒ 不动**（不编造归属）。
//!
//! # 查证结论：为什么归属必须**显式登记**（无法自动推导）
//!
//! 影响声明的 `action_id` 与工具调用的 `ToolCall.id` **不同源、不可推导**
//! （2026-10-25 查证，证据见下）：
//!
//! 1. 影响声明的 `action_id` 由出网闸生成：`act-{墙钟毫秒}`
//!    （[`crate::egress_gate`] 的 `declare_impact`）—— 纯墙钟派生，与任何工具调用无关；
//! 2. 工具调用带的是 `ToolCall.id`：**provider 赋的不透明 id**
//!    （Anthropic `toolu_*` / OpenAI `call_*`），由模型侧生成；
//! 3. 出网闸在 **provider 请求**上判定 —— 影响声明在**工具调用存在之前**就落账，
//!    且它声明的边界是 `provider`（内容出境），与工具动作的边界**不是同一件事**；
//! 4. `ImpactStatement` 里**没有任何字段**指向工具调用，仓库里也**不存在**
//!    `action_id ↔ tool_call_id` 的映射账。
//!
//! ⇒ 两者之间**没有既有映射**。1:1 归属**只能由同时知道两边的调用方显式登记**
//! （[`ActionToolBindingLedger::bind`]）；**登记不到 ⇒ 不动**。
//!
//! # 三条纪律
//!
//! 1. **只自动填 `failed`**：`confirmed` 的语义是「影响已按声明发生」，需要**外部证据**
//!    （对方是否真收到）。本地工具调用成功 ≠ 影响已确认 —— 自动填 `confirmed`
//!    等于替外部背书（owner 红线）。工具调用**成功**时本模块**什么都不做**。
//! 2. **不编造归属**：没有登记 ⇒ 不动；登记冲突（破坏 1:1）⇒ 拒。
//! 3. **幂等**：同一失败重复回填**不产生重复记录**；已 `confirmed` 的事实**不被降级**。
//!
//! # 真值边界（0 装诚实）
//!
//! - 本模块**只**回填 `failed`，来源记为 `locally_observed`（本地可验证的事实）；
//! - 归属账是**显式登记**的，不是推断出来的；没有登记的工具失败**不产生任何账面变化**；
//! - 装配面（[`ConsequenceBackfillModule`]）**默认关**：与问责面共用同一个开关
//!   （[`crate::egress_gate::EGRESS_GATE_ENV`]）—— 开闸才装；关时**零注册**、
//!   零行为变化。开着时它也是**惰性**的：绑定账为空时每次观察都是 no-op
//!   （不写盘、不改回合走向）。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use apeireth_protocol::canonical::ToolResult;
use serde::{Deserialize, Serialize};

use crate::accountability::Outcome;
use crate::canonical::module::{
    BehaviorModule, HookPoint, ModuleContext, ModuleError, ModuleManifest, ModuleOutcome,
};
use crate::consequence::{ConsequenceTracker, OutcomeSource};

/// 归属绑定账的文件名（数据根下；append-only JSONL）。
pub const ACTION_TOOL_BINDINGS_FILE: &str = "action-tool-bindings.jsonl";

/// 后果账的文件名（数据根下；与 CLI `egress outcome` 同一本账）。
pub const CONSEQUENCE_LEDGER_FILE: &str = "consequence-ledger.jsonl";

/// 自动回填模块的稳定 id（装配根按 id 去重校验）。
pub const CONSEQUENCE_BACKFILL_MODULE_ID: &str = "accountability.backfill";

/// 一条「行动 ↔ 工具调用」的 1:1 归属登记（**零真值**：只有两个 id）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionToolBinding {
    /// 已声明影响的行动 id（`ImpactStatement::action_id`）。
    pub action_id: String,
    /// 工具调用 id（`ToolCall::id` / `ToolResult::tool_call_id`）。
    pub tool_call_id: String,
    /// 登记时刻（epoch 毫秒）。
    pub at_ms: i64,
}

/// 登记结果（**说清是哪一种**，不吞原因）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindOutcome {
    /// 登记成功（新增一行）。
    Recorded,
    /// 同一对**已经登记过** —— 幂等，不重复落行。
    AlreadyRecorded,
    /// **冲突**：一侧已绑到别的对象（1:1 被破坏）—— 拒，不改写既有登记。
    Conflict,
    /// 落盘失败（或没有持久化路径）—— 拒。
    Failed,
}

impl BindOutcome {
    /// 是否算「登记成功」（含幂等命中）。
    pub fn is_bound(&self) -> bool {
        matches!(self, Self::Recorded | Self::AlreadyRecorded)
    }
}

/// 归属绑定账（append-only JSONL）—— **显式登记入口**。
///
/// 1:1 由本账**强制**：一个工具调用只能对应一条已声明影响，一个已声明影响
/// 只能对应一次工具调用。破坏 1:1 的登记**被拒**（不覆盖、不改写既有登记）。
#[derive(Debug, Clone)]
pub struct ActionToolBindingLedger {
    path: Option<PathBuf>,
}

impl ActionToolBindingLedger {
    /// 新建（`path` 为 `None` = 不持久 ⇒ 不登记）。
    pub fn new(path: Option<PathBuf>) -> Self {
        Self { path }
    }

    /// 账本路径（`None` = 不持久）。
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// **显式登记**一条 1:1 归属（`action_id` ↔ `tool_call_id`）。
    ///
    /// 这是「行动 ↔ 影响」归属的**唯一**入口：系统**不推断**归属
    /// （两个 id 不同源，见模块头查证结论）。
    pub fn bind(&self, action_id: &str, tool_call_id: &str) -> BindOutcome {
        if action_id.trim().is_empty() || tool_call_id.trim().is_empty() {
            return BindOutcome::Failed;
        }
        let Some(path) = &self.path else {
            // 不持久 = 登记无处可查 = 不登记（不留「记在内存里」的影子归属）。
            return BindOutcome::Failed;
        };
        let existing = read_bindings(path);
        // 1:1 —— 同侧已绑到别的对象 ⇒ 拒（不编造、不覆盖）。
        if existing
            .iter()
            .any(|binding| binding.tool_call_id == tool_call_id && binding.action_id != action_id)
        {
            return BindOutcome::Conflict;
        }
        if existing
            .iter()
            .any(|binding| binding.action_id == action_id && binding.tool_call_id != tool_call_id)
        {
            return BindOutcome::Conflict;
        }
        // 幂等：同一对已登记 ⇒ 不重复落行。
        if existing
            .iter()
            .any(|binding| binding.action_id == action_id && binding.tool_call_id == tool_call_id)
        {
            return BindOutcome::AlreadyRecorded;
        }
        let binding = ActionToolBinding {
            action_id: action_id.to_string(),
            tool_call_id: tool_call_id.to_string(),
            at_ms: now_ms(),
        };
        match append_binding(path, &binding) {
            Ok(()) => BindOutcome::Recorded,
            Err(_) => BindOutcome::Failed,
        }
    }

    /// 反查：这次工具调用对应哪条已声明影响（`None` = 没登记 ⇒ **不动**）。
    pub fn action_for(&self, tool_call_id: &str) -> Option<String> {
        let path = self.path.as_ref()?;
        read_bindings(path)
            .into_iter()
            .rev()
            .find(|binding| binding.tool_call_id == tool_call_id)
            .map(|binding| binding.action_id)
    }

    /// 反查：这条已声明影响对应哪次工具调用。
    pub fn tool_call_for(&self, action_id: &str) -> Option<String> {
        let path = self.path.as_ref()?;
        read_bindings(path)
            .into_iter()
            .rev()
            .find(|binding| binding.action_id == action_id)
            .map(|binding| binding.tool_call_id)
    }

    /// 全部登记（时间序）。
    pub fn bindings(&self) -> Vec<ActionToolBinding> {
        match &self.path {
            Some(path) => read_bindings(path),
            None => Vec::new(),
        }
    }
}

/// 自动回填器：把「工具调用失败」回填成对应行动的 `failed` 后果。
///
/// 归属来自**显式登记**的绑定账；后果写入**后果账**（与人工回填同一本账）。
#[derive(Debug, Clone)]
pub struct OutcomeBackfiller {
    tracker: Arc<tokio::sync::Mutex<ConsequenceTracker>>,
    bindings: Arc<ActionToolBindingLedger>,
}

impl OutcomeBackfiller {
    /// 新建（注入后果状态机与绑定账）。
    pub fn new(
        tracker: Arc<tokio::sync::Mutex<ConsequenceTracker>>,
        bindings: Arc<ActionToolBindingLedger>,
    ) -> Self {
        Self { tracker, bindings }
    }

    /// 从**数据根**装配（后果账 + 绑定账，同根同文件）。
    pub fn from_data_root(root: &Path) -> Self {
        Self::new(
            Arc::new(tokio::sync::Mutex::new(ConsequenceTracker::with_ledger(
                root.join(CONSEQUENCE_LEDGER_FILE),
            ))),
            Arc::new(ActionToolBindingLedger::new(Some(
                root.join(ACTION_TOOL_BINDINGS_FILE),
            ))),
        )
    }

    /// 绑定账（只读引用）。
    pub fn binding_ledger(&self) -> &Arc<ActionToolBindingLedger> {
        &self.bindings
    }

    /// **显式登记**归属（与 [`ActionToolBindingLedger::bind`] 同语义）。
    pub fn bind(&self, action_id: &str, tool_call_id: &str) -> BindOutcome {
        self.bindings.bind(action_id, tool_call_id)
    }

    /// 回填一次工具结果（系统时钟）。
    ///
    /// 返回被回填的行动 id（`None` = 本次**什么都没做**）。
    pub fn backfill_from_tool_result(&self, result: &ToolResult) -> Option<String> {
        self.backfill_from_tool_result_at(result, now_ms())
    }

    /// 回填一次工具结果（显式时刻，便于脱时钟断言）。
    ///
    /// 判据（逐条可测）：
    /// - 工具**成功** ⇒ **不动**（绝不自动填 `confirmed`：那是替外部背书）；
    /// - **没登记**对应声明 ⇒ **不动**（不编造归属）；
    /// - 已是 `failed` ⇒ **不动**（幂等：不产生重复记录）；
    /// - 已是 `confirmed` ⇒ **不动**（已确认的事实不被降级）；
    /// - 其余 ⇒ 回填 `failed`（来源：本地可验证）。
    pub fn backfill_from_tool_result_at(&self, result: &ToolResult, at_ms: i64) -> Option<String> {
        // 成功 ⇒ 什么都不做（`confirmed` 只走人工/外部）。
        if result.is_ok() {
            return None;
        }
        // 没登记 ⇒ 不动（不编造归属）。
        let action_id = self.bindings.action_for(&result.tool_call_id)?;
        // 拿不到锁 = 并发忙 ⇒ 如实不动（不猜、不排队）。
        let mut guard = self.tracker.try_lock().ok()?;
        // 多进程共享真源：先读回账本现状，再判断该不该记。
        guard.reload();
        match guard.current_outcome(&action_id) {
            // 幂等：已经记过失败 ⇒ 不重复记账。
            Outcome::Failed => return Some(action_id),
            // 已确认的事实不能被降级。
            Outcome::Confirmed => return None,
            Outcome::Unknown | Outcome::Pending => {}
        }
        if guard.record(
            &action_id,
            Outcome::Failed,
            OutcomeSource::LocallyObserved,
            at_ms,
        ) {
            Some(action_id)
        } else {
            None
        }
    }
}

/// 自动回填模块：装配根注册的**观测面**（不改回合走向）。
///
/// 只在 `AfterToolResult` 观察；**其它钩子一律不看**（不编造信号）。
/// 观察失败（拿不到锁 / 落盘失败）**不产生错误**——本模块**永不**让回合失败。
#[derive(Debug, Clone)]
pub struct ConsequenceBackfillModule {
    backfiller: Arc<OutcomeBackfiller>,
    manifest: ModuleManifest,
}

impl ConsequenceBackfillModule {
    /// 新建（注入回填器）。
    pub fn new(backfiller: Arc<OutcomeBackfiller>) -> Self {
        Self {
            backfiller,
            manifest: ModuleManifest::new(CONSEQUENCE_BACKFILL_MODULE_ID, "Consequence Backfill"),
        }
    }

    /// 从**数据根**装配（后果账 + 绑定账）。
    pub fn from_data_root(root: &Path) -> Self {
        Self::new(Arc::new(OutcomeBackfiller::from_data_root(root)))
    }

    /// 观察一次工具结果（幂等；`None` = 本次什么都没做）。
    pub fn observe_tool_result(&self, result: &ToolResult) -> Option<String> {
        self.backfiller.backfill_from_tool_result(result)
    }

    /// 回填器（只读引用）—— 调用方用它**显式登记**归属。
    pub fn backfiller(&self) -> &Arc<OutcomeBackfiller> {
        &self.backfiller
    }
}

#[async_trait::async_trait]
impl BehaviorModule for ConsequenceBackfillModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if hook == HookPoint::AfterToolResult {
            if let Some(result) = ctx.tool_result {
                // 观测面：失败也不让回合失败（回填是记账，不是守门）。
                let _ = self.observe_tool_result(result);
            }
        }
        Ok(ModuleOutcome::continue_())
    }
}

/// 数据根（`APEIRETH_DATA_DIR` → `USERPROFILE` → `HOME` → `.`）。
///
/// 与 CLI 的 `default_panel_data_dir()` **同一路径约定**（CLI 侧那份是面板归档
/// 的数据根）：CLI 写的账与运行时读账必须落在**同一处**，否则回填看不见。
pub fn default_data_root() -> PathBuf {
    crate::paths::default_data_root()
}

/// 墙钟毫秒（epoch）。
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

/// 追加一行绑定（带换行）。父目录不存在时先建。
fn append_binding(path: &Path, binding: &ActionToolBinding) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    use std::io::Write;
    let line = serde_json::to_string(binding)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

/// 读回绑定（坏行读作不存在）。
fn read_bindings(path: &Path) -> Vec<ActionToolBinding> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| serde_json::from_str::<ActionToolBinding>(line).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_protocol::canonical::ToolOutcome;

    fn temp_root(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "apeireth-backfill-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&root).expect("mkdir");
        root
    }

    fn failed(call_id: &str) -> ToolResult {
        ToolResult {
            tool_call_id: call_id.to_string(),
            name: Some("shell.exec".to_string()),
            outcome: ToolOutcome::Error {
                message: "boom".to_string(),
                retryable: false,
            },
        }
    }

    fn succeeded(call_id: &str) -> ToolResult {
        ToolResult::ok(call_id, serde_json::json!({"ok": true}))
    }

    fn backfiller(root: &Path) -> OutcomeBackfiller {
        OutcomeBackfiller::from_data_root(root)
    }

    fn tracker_at(root: &Path) -> ConsequenceTracker {
        ConsequenceTracker::with_ledger(root.join(CONSEQUENCE_LEDGER_FILE))
    }

    /// 工具失败 ⇒ 对应影响的 outcome 从 `Unknown` 变 `failed`。
    #[test]
    fn tool_failure_backfills_the_mapped_action_as_failed() {
        let root = temp_root("failure");
        let backfiller = backfiller(&root);
        assert_eq!(
            backfiller.bind("act-1", "call_1"),
            BindOutcome::Recorded,
            "显式登记归属"
        );

        let action_id = backfiller.backfill_from_tool_result(&failed("call_1"));
        assert_eq!(action_id.as_deref(), Some("act-1"));
        assert_eq!(
            tracker_at(&root).current_outcome("act-1"),
            Outcome::Failed,
            "工具失败 ⇒ 对应影响回填 failed"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// 工具成功 ⇒ **不变**（仍 `Unknown`，绝不自动填 confirmed）。
    #[test]
    fn tool_success_leaves_the_outcome_unknown() {
        let root = temp_root("success");
        let backfiller = backfiller(&root);
        backfiller.bind("act-1", "call_1");

        assert_eq!(
            backfiller.backfill_from_tool_result(&succeeded("call_1")),
            None
        );
        let tracker = tracker_at(&root);
        assert_eq!(
            tracker.current_outcome("act-1"),
            Outcome::Unknown,
            "工具成功 ≠ 影响已确认 —— 绝不自动填 confirmed"
        );
        assert_eq!(tracker.counts().confirmed, 0);
        assert_eq!(tracker.counts().failed, 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// 没有登记对应声明 ⇒ **不动**。
    #[test]
    fn unmapped_tool_result_changes_nothing() {
        let root = temp_root("unmapped");
        let backfiller = backfiller(&root);
        backfiller.bind("act-1", "call_1");

        assert_eq!(
            backfiller.backfill_from_tool_result(&failed("call_unknown")),
            None,
            "找不到对应声明 ⇒ 不编造归属"
        );
        let tracker = tracker_at(&root);
        assert_eq!(tracker.current_outcome("act-1"), Outcome::Unknown);
        assert_eq!(
            tracker.counts().tracked_actions,
            0,
            "没回填 ⇒ 账上没有任何行动"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// 幂等：同一失败重复回填 ⇒ 只有一条记录，计数不膨胀。
    #[test]
    fn repeated_failure_is_idempotent() {
        let root = temp_root("idempotent");
        let backfiller = backfiller(&root);
        backfiller.bind("act-1", "call_1");

        assert!(backfiller
            .backfill_from_tool_result(&failed("call_1"))
            .is_some());
        assert!(
            backfiller
                .backfill_from_tool_result(&failed("call_1"))
                .is_some(),
            "重复失败仍算「这条行动已回填」"
        );
        let tracker = tracker_at(&root);
        assert_eq!(tracker.current_outcome("act-1"), Outcome::Failed);
        assert_eq!(
            tracker.events_for("act-1").len(),
            1,
            "重复回填不得产生重复记录"
        );
        let counts = tracker.counts();
        assert_eq!(counts.failed, 1);
        assert_eq!(counts.tracked_actions, 1, "计数不得膨胀");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// 已 `confirmed` 的事实不被工具失败降级。
    #[test]
    fn confirmed_outcome_is_never_downgraded_by_a_failure() {
        let root = temp_root("confirmed");
        let backfiller = backfiller(&root);
        backfiller.bind("act-1", "call_1");

        let mut tracker = ConsequenceTracker::with_ledger(root.join(CONSEQUENCE_LEDGER_FILE));
        assert!(tracker.record("act-1", Outcome::Confirmed, OutcomeSource::UserConfirmed, 1));

        assert_eq!(
            backfiller.backfill_from_tool_result(&failed("call_1")),
            None,
            "已确认的事实不能被降级成 failed"
        );
        assert_eq!(tracker.current_outcome("act-1"), Outcome::Confirmed);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// 1:1 由绑定账强制：同侧绑到别的对象 ⇒ 拒。
    #[test]
    fn binding_is_one_to_one() {
        let root = temp_root("one-to-one");
        let backfiller = backfiller(&root);
        assert_eq!(backfiller.bind("act-1", "call_1"), BindOutcome::Recorded);
        assert_eq!(
            backfiller.bind("act-2", "call_1"),
            BindOutcome::Conflict,
            "一次工具调用只能对应一条已声明影响"
        );
        assert_eq!(
            backfiller.bind("act-1", "call_2"),
            BindOutcome::Conflict,
            "一条已声明影响只能对应一次工具调用"
        );
        assert_eq!(
            backfiller.bind("act-1", "call_1"),
            BindOutcome::AlreadyRecorded,
            "同一对重复登记 = 幂等"
        );
        assert_eq!(
            backfiller.binding_ledger().bindings().len(),
            1,
            "冲突不得写入新行"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// 绑定账落盘 ⇒ 重启后仍可查（跨进程共享真源）。
    #[test]
    fn bindings_survive_a_restart() {
        let root = temp_root("restart");
        {
            let backfiller = backfiller(&root);
            backfiller.bind("act-1", "call_1");
        }
        // 新实例（新进程）：绑定仍可查 ⇒ 仍能自动回填。
        let backfiller = backfiller(&root);
        assert_eq!(
            backfiller.binding_ledger().action_for("call_1").as_deref(),
            Some("act-1")
        );
        assert!(backfiller
            .backfill_from_tool_result(&failed("call_1"))
            .is_some());
        assert_eq!(tracker_at(&root).current_outcome("act-1"), Outcome::Failed);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// 无持久化路径 ⇒ 不登记（不留「只在内存里」的影子归属）。
    #[test]
    fn in_memory_ledger_refuses_to_bind() {
        let ledger = ActionToolBindingLedger::new(None);
        assert_eq!(ledger.bind("act-1", "call_1"), BindOutcome::Failed);
        assert!(ledger.action_for("call_1").is_none());
    }

    /// 绑定账里的坏行读作不存在（与其它账本同纪律）。
    #[test]
    fn bad_lines_are_read_as_absent() {
        let root = temp_root("bad");
        let path = root.join(ACTION_TOOL_BINDINGS_FILE);
        std::fs::write(&path, "{\"action_id\":\"a\"}\nNOT_JSON\n").expect("write");
        let ledger = ActionToolBindingLedger::new(Some(path));
        assert!(ledger.bindings().is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }
}
