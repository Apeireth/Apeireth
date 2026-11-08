//! 主人日程面 (`tool.calendar`, 声明名 `calendar`)。
//!
//! **这是什么**: 主人自己的日程账 —— 查询 / 创建 / 改期 / 取消 主人自己的安排。
//!
//! **先本地、零外部集成**: 账本落在本地数据目录, 不连任何外部日历服务, 不做
//! 跨设备漫游 (owner 暂缓) ⇒ ⇒ 本面**没有**出网面、**没有**同步面、**没有**
//! 推送面; 全部读写只碰本地账本两个文件。
//!
//! # 主体与边界: 本面 ≠ `tool.care_schedule` (主人日程 ≠ 代理自身的关怀例程)
//!
//! ⚠️ **这两个面主体不同、治理档不同、账本不同, 互不可见**:
//!
//! | 维度 | 本面 `tool.calendar` | `tool.care_schedule` |
//! |---|---|---|
//! | 主体 | **主人** (owner 的日程: 约了谁、几点、去哪) | **代理自身** (关怀例程: 代理什么时候该主动关怀主人) |
//! | 动作 | `list` / `create` / `update` / `cancel` | `care_add` / `care_remove` / `care_list` |
//! | 治理档 | 写面**逐次人批** (每次调用都停人工审批, 无自动放行档) | 改规则 = 写文件 (write_intent 同族), 每次人批 |
//! | 账本 | 本地日程账 (`<data_dir>/calendar/entries.jsonl`) | 关怀规则账 (organ 侧规则存储) |
//!
//! ⇒ **本面不读、不写、不感知关怀规则账**; 关怀例程触发器不是日程条目, 日程
//! 条目也不是关怀触发器 ⇒ 两边的 id / 时刻 / 状态机**互不通用**。
//! ⇒ 本面**不**把「代理要关怀主人」写进主人日程 (那是关怀例程的职责), 也
//! **不**把「主人的安排」当关怀触发器。
//!
//! # 模块分层
//!
//! - [`store`]: 数据模型 + 字段校验 + StoredDoc 账本 (append-only, 坏行按不存在);
//! - [`tool`]: 生产工具面 (`tool.calendar`) + 单调边界 Guard + 拒绝即帧;
//! - 本模块: 身份 / 词表 / 治理钩子 / 装配 (流水线 · 开关 · 能力组装)。
//!
//! # 存储面 (StoredDoc 账本 · append-only)
//!
//! 落点 (数据目录下的**日程面目录**, 与既有族同姿势):
//!
//! ```text
//! <data_dir>/calendar/entries.jsonl   日程条目账 (逐行一个 StoredDoc 信封)
//! <data_dir>/calendar/audit.jsonl     写动作审计账 (逐行一个 StoredDoc 信封)
//! ```
//!
//! - **append-only**: 只追加行, 不重写、不删行 (与提案账 / 关怀账 / 设备审计账
//!   同一姿势); 行的追加复用仓内**唯一**的追加手续
//!   ([`crate::proposal::append_jsonl_row`]: 行体与行尾换行同在一次写 ⇒ 并发
//!   追加不挤行; 末行缺换行的撕裂半行先补换行封口 ⇒ 不会吞掉下一行)。
//! - **坏行按不存在处理**: 单行坏 JSON / 身份不符 / 版本不符 ⇒ **跳过并计数**
//!   ([`apeireth_core::stored_doc::scan_records`] 的 `skipped` 口径), **不砖化
//!   整本**; 好行照读。计数**如实上报**到结果帧的 `skipped` 字段与 `note`
//!   ⇒ 不静默吞错。
//! - **状态折算**: 条目当前态 = 账上按顺序折叠的结果 (`created` → `updated` →
//!   `cancelled`); 取消是**终态** (取消过的条目不可改期)。
//!
//! # 治理 (写面逐次人批 · 读面零审批)
//!
//! - `list` = **纯读** ⇒ 零审批、零写 (读动作连审计行都不落)。
//! - `create` / `update` / `cancel` = **写主人日程账** ⇒ **每次调用都停人工
//!   审批** (无自动放行档), 与本地审批面板 / IM 审批卡同一条审批链; 档位由
//!   [`CalendarApprovalHook`] 唯一承载 (照 `device_control` / `content_feed` /
//!   `care_schedule` 的既有范式)。
//! - 每次写动作落一行**审计行** ([`CalendarAuditRow`]: 动作 / 目标条目 id /
//!   结果 / 时刻 / 人读备注)。
//! - 词表外动作 = **fail-closed 拒绝即帧** (治理层 `deny` + 工具层永久错误帧),
//!   不静默放行、不猜动作。
//!
//! # 边界 (拒绝即帧 · 宁拒不截)
//!
//! - **起止时间非法** (结束早于开始 / 零时长 / 超长时段 / 非正时刻) ⇒ 拒绝即帧。
//! - **超长标题 / 超长备注** ⇒ 拒绝即帧 (**不截断**: 截断会造出主人没写过的
//!   安排, 比拒更危险)。
//! - **条数上限** (有效条目上限 + 账本行数上限) ⇒ 超限**拒绝**, 不截断、不静默
//!   丢条目 (防炸弹账本)。
//! - **条目 id 重复 / 改期已取消条目 / 取消不存在条目** ⇒ 拒绝即帧。
//!
//! # 接线口径 (本线只实现, 不越界接线)
//!
//! 本目录是**实现体** (IMPLEMENTED 级)。生产接线 (在 `runtime-assembly` 的
//! `production.rs` 注册 + 治理链挂 [`CalendarApprovalHook`] + 自述名册登记)
//! **不在本线写域** ⇒ **接线前本面在生产不可达**。
//! 开关口径: `APEIRETH_ENABLE_CALENDAR` (注册即闸, 只认显式 `"1"`, 默认关
//! fail-closed ⇒ 关时逐字节零变化)。
//!
//! # 模型侧声明名的口径对齐
//!
//! 声明名取 `calendar`, 与 SDK 侧**规划契约**的 `TOOL_WHITELIST` 里的
//! `calendar` (语义 = 日历 list/create/update events) **对齐口径** ⇒ 该契约名
//! 不再是悬空的占位词。⚠️ 但 SDK 侧的 `calendar_list` /
//! `/v1/tools/calendar/invoke` 仍是**客户端规划契约** (网关无此路由) ⇒ 本面
//! **不**提供该 HTTP 路由 (越出本线写域)。

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};
use serde_json::Value;

use crate::exec_pipeline::{
    AroundPolicy, OutputSchema, SchemaKind, ToolExecutionPipeline, DEFAULT_MAX_TIMEOUT_MS,
};

pub mod store;
pub mod tool;

pub use store::{
    next_calendar_entry_id, now_epoch_ms, validate_calendar_entry_id, validate_calendar_note,
    validate_calendar_span, validate_calendar_title, CalendarAuditRow, CalendarEntry,
    CalendarError, CalendarLedgerEntry, CalendarSnapshot, CalendarStatus, CalendarStore,
};
pub use tool::{CalendarBoundaryGuard, CalendarTool};

// ---------------------------------------------------------------------------
// 常量 (身份 / 落点 / 上限 / 词表)
// ---------------------------------------------------------------------------

/// 稳定能力标识 (注册表 / 治理授权 / 审批档共用)。
pub const CALENDAR_CAPABILITY_ID: &str = "tool.calendar";
/// 模型侧声明名 (短名, 每次请求都进上下文)。
pub const CALENDAR_TOOL_NAME: &str = "calendar";
/// 生产执行时限 (毫秒): 本地账本读取 + 追加写, 到期即归 `timeout.*`。
pub const CALENDAR_TIMEOUT_MS: u64 = 5_000;
/// 日程面总开关 (`APEIRETH_ENABLE_CALENDAR`): 注册即闸, 默认关 fail-closed。
pub const CALENDAR_ENABLE_ENV: &str = "APEIRETH_ENABLE_CALENDAR";

/// 日程面目录名 (数据目录下的子目录: `<data_dir>/calendar/`)。
pub const CALENDAR_DIR_NAME: &str = "calendar";
/// 日程条目账文件名 (目录内)。
pub const CALENDAR_LEDGER_FILE: &str = "entries.jsonl";
/// 日程写动作审计账文件名 (目录内)。
pub const CALENDAR_AUDIT_FILE: &str = "audit.jsonl";
/// 日程条目账 StoredDoc 的身份戳。
pub const CALENDAR_LEDGER_DOC_NAME: &str = "apeireth.calendar_ledger";
/// 日程条目账 StoredDoc 的格式版本戳。
pub const CALENDAR_LEDGER_DOC_VERSION: u32 = 1;
/// 日程审计账 StoredDoc 的身份戳。
pub const CALENDAR_AUDIT_DOC_NAME: &str = "apeireth.calendar_audit";
/// 日程审计账 StoredDoc 的格式版本戳。
pub const CALENDAR_AUDIT_DOC_VERSION: u32 = 1;

/// 标题最大字节数 (超长 ⇒ 拒绝即帧, **不截断**)。
pub const CALENDAR_TITLE_MAX_BYTES: usize = 200;
/// 备注最大字节数 (超长 ⇒ 拒绝即帧, **不截断**)。
pub const CALENDAR_NOTE_MAX_BYTES: usize = 2_000;
/// 条目 id 最大字节数 (闭合字符集, 见 [`validate_calendar_entry_id`])。
pub const CALENDAR_ENTRY_ID_MAX_BYTES: usize = 64;
/// 有效 (未取消) 条目数上限 (防炸弹账本: 超限 ⇒ 拒绝, 不截断)。
pub const CALENDAR_MAX_ENTRIES: usize = 512;
/// 账本总行数上限 (append-only 的增长闸: 超限 ⇒ 拒绝, 不截断)。
pub const CALENDAR_MAX_LEDGER_ROWS: usize = 8_192;
/// 单条日程的最大跨度 (毫秒, 366 天): 超长时段 ⇒ 拒绝即帧。
pub const CALENDAR_MAX_SPAN_MS: i64 = 366 * 24 * 3_600 * 1_000;

/// 主体边界声明 (工具描述与结果帧同文): 本面是**主人日程**, 不是代理自身的
/// 关怀例程 (`tool.care_schedule`)。两者主体不同、治理档不同、账本不同、
/// 互不可见 ⇒ 模型不得把本面的条目当关怀触发器, 也不得把关怀触发器写进
/// 主人日程。
pub const CALENDAR_SCOPE_NOTE: &str =
    "本面是主人日程 (owner 自己的安排): 查询 / 创建 / 改期 / 取消。与 tool.care_schedule \
     (代理自身的关怀例程: 代理何时该主动关怀主人) 主体不同、治理档不同、账本不同、互不可见: \
     关怀触发器不是日程条目, 日程条目也不是关怀触发器。";

// 动作: 列日程 (纯读, 零审批)。
pub const ACTION_LIST: &str = "list";
// 动作: 新建日程条目 (写账本, 逐次人批)。
pub const ACTION_CREATE: &str = "create";
// 动作: 改期 / 改标题 / 改备注 (写账本, 逐次人批)。
pub const ACTION_UPDATE: &str = "update";
// 动作: 取消日程条目 (写账本, 逐次人批)。
pub const ACTION_CANCEL: &str = "cancel";

/// 闭合动作词表 (list / create / update / cancel)。
pub const CALENDAR_ACTIONS: &[&str] = &[ACTION_LIST, ACTION_CREATE, ACTION_UPDATE, ACTION_CANCEL];

// ---------------------------------------------------------------------------
// 写面归类 + 审批理由
// ---------------------------------------------------------------------------

/// 动作的写面归类 (write_intent 同族口径; 未知动作无写面)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarWriteFace {
    /// 纯读 (列日程), 零本地写。
    ReadOnly,
    /// 写日程账 (append 一行条目 + 一行审计)。
    LedgerAppend,
}

impl CalendarWriteFace {
    /// 稳定标签 (审计 / 测试用)。
    pub const fn label(self) -> &'static str {
        match self {
            Self::ReadOnly => "read_only",
            Self::LedgerAppend => "ledger_append",
        }
    }
}

/// 动作的写面归类 (write_intent 同族口径; 未知动作无写面)。
pub fn calendar_write_face_for_action(action: &str) -> Option<CalendarWriteFace> {
    match action {
        ACTION_LIST => Some(CalendarWriteFace::ReadOnly),
        ACTION_CREATE | ACTION_UPDATE | ACTION_CANCEL => Some(CalendarWriteFace::LedgerAppend),
        _ => None,
    }
}

/// 审批档位理由 (审批卡载荷同文)。按动作如实标注写面。
pub fn calendar_approval_reason(action: Option<&str>) -> String {
    let face = match action {
        Some(ACTION_CREATE) => "create = 写主人日程账 (新建条目)",
        Some(ACTION_UPDATE) => "update = 写主人日程账 (改期 / 改标题 / 改备注)",
        Some(ACTION_CANCEL) => "cancel = 写主人日程账 (取消条目)",
        Some(ACTION_LIST) => "list = 只读日程账, 零审批",
        _ => "动作未识别, fail-closed 拒绝即帧",
    };
    format!(
        "calendar ({face}) —— 写面逐次人批: 每次调用都停人工审批, 无自动放行档; \
         主人日程的写入必须有人拍板"
    )
}

// ---------------------------------------------------------------------------
// 治理 (写面逐次人批 · 读面零审批)
// ---------------------------------------------------------------------------

/// 治理档位钩子: `tool.calendar` 的**写动作每次**停 [`Decision::RequireApproval`]
/// (无自动放行档), **读动作零审批**。
///
/// 与本地审批面板 / IM 审批卡同一条审批链 (照 `device_control` / `content_feed`
/// / `care_schedule` 的既有范式): 档位在治理层的钩子上, 其它能力一律放行
/// (本钩子只管自己的面)。
#[derive(Debug, Clone, Copy, Default)]
pub struct CalendarApprovalHook;

impl CalendarApprovalHook {
    /// 空构造 (无状态钩子)。
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl GovernanceHook for CalendarApprovalHook {
    fn name(&self) -> &str {
        "calendar_write_approval_tier"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        let Action::CapabilityDispatch {
            capability,
            arguments,
        } = &request.action
        else {
            return Decision::Allow;
        };
        if capability.as_str() != CALENDAR_CAPABILITY_ID {
            return Decision::Allow;
        }
        let action = arguments.get("action").and_then(Value::as_str);
        match action {
            // 纯读面: 零审批。
            Some(ACTION_LIST) => Decision::Allow,
            // 写面: 逐次人批 (无自动放行档)。
            Some(ACTION_CREATE) | Some(ACTION_UPDATE) | Some(ACTION_CANCEL) => {
                Decision::require_approval(calendar_approval_reason(action))
            }
            // 词表外 / 动作缺失: fail-closed 拒绝即帧 (不静默放行, 不猜动作)。
            other => Decision::deny(format!(
                "calendar 动作 {other:?} 不在闭合词表 (list / create / update / cancel): \
                 fail-closed 拒绝即帧"
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// 装配 (流水线 / 开关 / 能力)
// ---------------------------------------------------------------------------

/// 组装日程面五段流水线: 单调边界 Guard → 超时 → 输出归一合同。
pub fn calendar_pipeline() -> Arc<ToolExecutionPipeline> {
    Arc::new(
        ToolExecutionPipeline::new()
            .with_guard(Arc::new(CalendarBoundaryGuard::new()))
            .with_around(
                AroundPolicy::new().with_timeout(CALENDAR_TIMEOUT_MS, DEFAULT_MAX_TIMEOUT_MS),
            )
            .with_output_schema(CalendarTool::output_schema()),
    )
}

/// 组装主人日程工具面 (模型侧一个能力)。
pub fn calendar_capability(config: CalendarConfig) -> Arc<dyn apeireth_plugin::ToolCapability> {
    let tool: Arc<dyn apeireth_plugin::ToolCapability> =
        Arc::new(CalendarTool::new(config.store()));
    Arc::new(crate::exec_pipeline::PipelinedCapability::new(
        tool,
        calendar_pipeline(),
    ))
}

/// 日程面配置 (日程账落位)。
///
/// 组装根按 [`CalendarConfig::from_env`] 注入 (注册即闸): 缺配置 =
/// `tool.calendar` 不注册。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarConfig {
    /// 数据目录 (日程面落在 `<data_dir>/calendar/`)。
    pub data_dir: PathBuf,
}

impl CalendarConfig {
    /// 日程面落位 = 数据目录。
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    /// 本配置的日程账。
    pub fn store(&self) -> CalendarStore {
        CalendarStore::under_data_dir(&self.data_dir)
    }

    /// 开关实况: `APEIRETH_ENABLE_CALENDAR` 只认显式 `"1"` (默认关 fail-closed)。
    pub fn enabled_from_env() -> bool {
        calendar_enabled_from_env()
    }

    /// 从 env 构造: 开关开才 `Some` (默认关 → [`None`], fail-closed)。
    pub fn from_env(data_dir: impl Into<PathBuf>) -> Option<Self> {
        calendar_enabled_from_env().then(|| Self::new(data_dir))
    }
}

/// 日程面开关 (`APEIRETH_ENABLE_CALENDAR`) 实况: 只认显式 `"1"`。
///
/// 未设 / `"0"` / `"true"` / 其它任何取值 = 关 (fail-closed, 命名即承诺:
/// 没有这把钥匙就没有日程面)。
pub fn calendar_enabled_from_env() -> bool {
    std::env::var(CALENDAR_ENABLE_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_vocabulary_is_closed_and_write_faces_are_pinned() {
        assert_eq!(
            CALENDAR_ACTIONS,
            [ACTION_LIST, ACTION_CREATE, ACTION_UPDATE, ACTION_CANCEL]
        );
        assert_eq!(
            calendar_write_face_for_action(ACTION_LIST),
            Some(CalendarWriteFace::ReadOnly)
        );
        assert_eq!(
            calendar_write_face_for_action(ACTION_CREATE),
            Some(CalendarWriteFace::LedgerAppend)
        );
        assert_eq!(
            calendar_write_face_for_action(ACTION_UPDATE),
            Some(CalendarWriteFace::LedgerAppend)
        );
        assert_eq!(
            calendar_write_face_for_action(ACTION_CANCEL),
            Some(CalendarWriteFace::LedgerAppend)
        );
        // 词表外动作没有写面 (不猜)。
        assert_eq!(calendar_write_face_for_action("delete"), None);
        assert_eq!(calendar_write_face_for_action("care_add"), None);
    }

    #[test]
    fn scope_note_names_both_sides_of_the_boundary() {
        assert!(CALENDAR_SCOPE_NOTE.contains("主人日程"));
        assert!(CALENDAR_SCOPE_NOTE.contains("tool.care_schedule"));
        assert!(CALENDAR_SCOPE_NOTE.contains("互不可见"));
    }
}
