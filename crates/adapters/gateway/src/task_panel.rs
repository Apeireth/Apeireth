//! 任务面板数据面 (`GET /v1/panel/tasks` 只读 + `POST /v1/panel/tasks/{task_id}/human-judge-retry`
//! 人判重试) —— 驾驶舱任务面板的 HTTP 形状。
//!
//! **面板边界 (G 组核心: failed 只能人判)**: 本面只有两口 ——
//! 1. [`list_tasks`] 只读一次快照读 (任务行 + 各自时间线 + 分态计数同一刻);
//! 2. [`human_judge_retry`] 唯一动作面: 对 `failed` 任务显式人判重试
//!    (队列语义 [`TaskQueue::human_judge_retry`](orchestration) 的唯一 HTTP 投影,
//!    `Failed` → `Pending`, 不自动唤醒)。
//!
//! **增删改派不进本面** (防越权): 任务的增/改派/完成走 `tool.task_board` 工具面
//! (写面 = 人工审批档), 本面不设对应路由 —— 路由表即边界, 见
//! [`crate::panels::panel_routes`]。
//!
//! 数据面 = 组合根注入的 [`TaskBoardPanel`] 端口 (照技能库只读面先例: 形状 +
//! 端口 + 组合根适配)。未注入 (任务簿旋钮关 = 注册即闸) = `501 unsupported`
//! 如实报, 不假装有数据源。
//!
//! 响应信封 (稳定合同, 字段改名须同步契约文档):
//! `{ "tasks": [TaskPanelDto], "counts": TaskCountsDto }`。

use async_trait::async_trait;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::client_tokens::{identity_label, ClientIdentity, UserContext};
use crate::panels::GatewayState;

/// 人判重试的缺省理由 (调用方未给理由时如实记录, 不编造)。
const DEFAULT_RETRY_REASON: &str = "驾驶舱任务面板人判重试";

/// 任务时间线一行 (事件账按任务的只读投影)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskTimelineDto {
    /// 事件种类 (事件账冻结词表: `enqueued` / `failure_judged` / `retry_judged` …)。
    pub kind: String,
    /// 事件时刻 (epoch 毫秒)。
    pub at_ms: i64,
    /// 事件细节 (人判者 / 原因 / 执行者等字段的紧凑 JSON; 无附加字段 = None)。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// 一条任务的面板投影 (列表行与选中详情同源一形状)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskPanelDto {
    /// 任务 id (`task-<写派号>`; 列表上屏取其短哈希)。
    pub id: String,
    /// 任务标题。
    pub title: String,
    /// 状态 (六态冻结词表: `pending` / `assigned` / `running` / `done` / `failed` / `blocked`)。
    pub state: String,
    /// 派发执行次数 (尝试数)。
    pub attempts: u32,
    /// 当前负责人 (可空)。
    pub assignee: Option<String>,
    /// 入队时刻 (epoch 毫秒)。
    pub created_at_ms: i64,
    /// 最近状态推进时刻 (epoch 毫秒)。
    pub updated_at_ms: i64,
    /// 不透明载荷 (面板详情给摘要; 队列不解释)。
    pub payload: serde_json::Value,
    /// 时间线 (该任务的事件账投影, 时间升序)。
    pub timeline: Vec<TaskTimelineDto>,
}

/// 分态计数 (六态 + 合计; 面板分组计数的真值)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskCountsDto {
    /// `pending` 计数。
    pub pending: u64,
    /// `assigned` 计数。
    pub assigned: u64,
    /// `running` 计数。
    pub running: u64,
    /// `done` 计数。
    pub done: u64,
    /// `failed` 计数。
    pub failed: u64,
    /// `blocked` 计数。
    pub blocked: u64,
    /// 合计。
    pub total: u64,
}

/// `GET /v1/panel/tasks` 响应信封 (一次快照读: 行 + 时间线 + 计数同一刻)。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TaskBoardSnapshotDto {
    /// 任务行 (稳定排序: 入队时刻, 同刻按 id)。
    pub tasks: Vec<TaskPanelDto>,
    /// 分态计数。
    pub counts: TaskCountsDto,
}

/// 人判重试请求体 (`reason` 可空; 空则记缺省理由)。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct HumanJudgeRetryRequest {
    /// 人判重试理由 (进 `retry_judged` 审计)。
    pub reason: Option<String>,
}

/// 人判重试响应信封 (重试后的任务行, 状态为回队后的 `pending`)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HumanJudgeRetryResponse {
    /// 重试后的任务行。
    pub task: TaskPanelDto,
}

/// 人判动作的失败归因 (HTTP 状态映射的稳定词表)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskPanelError {
    /// 任务不存在 (404)。
    NotFound {
        /// 被点名的任务 id。
        task_id: String,
    },
    /// 当前状态不允许人判重试 —— 只有 `failed` 能重试 (409)。
    InvalidState {
        /// 被点名的任务 id。
        task_id: String,
        /// 实际状态 (六态词表)。
        state: String,
    },
    /// 人判面结构性拒绝 (403)。
    Refused {
        /// 拒因。
        reason: String,
    },
    /// 存储/内部错误 (500)。
    Internal {
        /// 真因。
        reason: String,
    },
}

impl TaskPanelError {
    /// 错误帧 (`{"error": {code, message, solution}}`, 与管理面同词表)。
    pub fn into_response(self) -> Response {
        match self {
            Self::NotFound { task_id } => crate::panels::admin_error(
                StatusCode::NOT_FOUND,
                "not_found",
                format!("任务 {task_id} 不存在"),
                "刷新任务面板后重试; 任务可能已被清理",
            ),
            Self::InvalidState { task_id, state } => crate::panels::admin_error(
                StatusCode::CONFLICT,
                "conflict",
                format!("任务 {task_id} 当前状态 {state} 不允许人判重试 (只有 failed 能重试)"),
                "失败不自动重试; 只有 failed 任务可显式人判重试",
            ),
            Self::Refused { reason } => crate::panels::admin_error(
                StatusCode::FORBIDDEN,
                "permission_denied",
                format!("人判面拒绝: {reason}"),
                "人判动作只能由人发起; agent 走工具面, 且工具面无人判口",
            ),
            Self::Internal { reason } => crate::panels::panel_error(reason),
        }
    }
}

/// 任务面板端口 (面板族 bounded-context 端口; 适配实现在组合根, 投影同一
/// 持久任务队列 —— 与 `tool.task_board` 工具面同队同账)。
///
/// 面板边界在此封口: 端口只有**读一次快照**与**人判重试**两个方法,
/// 增/删/改派/完成不在此面 (防越权, 走工具面)。
#[async_trait]
pub trait TaskBoardPanel: Send + Sync {
    /// 一次快照读: 任务行 (含各自时间线) + 分态计数同一刻。
    async fn task_snapshot(&self) -> Result<TaskBoardSnapshotDto, String>;

    /// 人判重试 (failed 只能人判; 队列 `human_judge_retry` 的唯一 HTTP 投影,
    /// `Failed` → `Pending`, 显式动作不自动唤醒)。
    async fn human_judge_retry(
        &self,
        task_id: &str,
        judge: &str,
        reason: &str,
    ) -> Result<TaskPanelDto, TaskPanelError>;
}

/// 人判者署名 (请求不可自称, 与角色门同一条解析链):
/// 角色面绑定用户 > 令牌门客户端署名 > 如实「未归属」。
fn judge_identity(
    user: Option<&Extension<UserContext>>,
    client: Option<&Extension<ClientIdentity>>,
) -> String {
    if let Some(Extension(context)) = user {
        if let Some(user_id) = context.user_id() {
            return user_id.to_string();
        }
    }
    identity_label(client).to_string()
}

/// `GET /v1/panel/tasks` —— 任务面板只读面 (一次快照读; 过令牌门, 读面全员可读)。
///
/// 未接线 (端口未注入) = 501 `unsupported` 如实报, 不假装有数据源。
///
/// V2-c 面板按域: 非默认域 = 按域过滤返回 (与其余面板面同一铁律 —— 行有域归属
/// 记号才进域视图)。任务投影行 (`TaskPanelDto`) **无域归属记号** (id/标题/状态/
/// 载荷均非归属字段), 故域视图为零行 + 零计数 (fail-closed, 宁拒不泄);
/// 计数一律**按可见行重算**, 共享快照的计数不得泄进域视图 (A 域看不进 B 域
/// 计数)。任务行的域归属标注残余见交付记录 (随任务队列投影批补, 补齐即按
/// 记号判定)。
pub async fn list_tasks(
    State(state): State<GatewayState>,
    domain: Option<Extension<crate::domain_context::DomainContext>>,
) -> Response {
    let scope = match crate::domain_context::request_scope(
        &state,
        domain.as_ref().map(|Extension(context)| context),
        "/v1/panel/tasks",
    ) {
        Ok(scope) => scope,
        Err(refusal) => return refusal,
    };
    let Some(tasks) = &state.services.tasks else {
        return crate::panels::unsupported("tasks.read");
    };
    match tasks.task_snapshot().await {
        Ok(snapshot) => {
            let snapshot = domain_scoped_snapshot(&scope, snapshot);
            (StatusCode::OK, Json(snapshot)).into_response()
        }
        Err(message) => crate::panels::panel_error(message),
    }
}

/// 域视图快照: 行按域归属过滤 + 分态计数按**可见行**重算 (不复述共享计数)。
fn domain_scoped_snapshot(
    scope: &crate::domain_context::DomainScope,
    snapshot: TaskBoardSnapshotDto,
) -> TaskBoardSnapshotDto {
    if !scope.is_restricted() {
        return snapshot;
    }
    // 任务投影行无域归属记号 → 无行进域视图; 计数由可见行重算 (此处恒为零)。
    let tasks: Vec<TaskPanelDto> = Vec::new();
    let counts = counts_from_rows(&tasks);
    TaskBoardSnapshotDto { tasks, counts }
}

/// 分态计数 = 可见行的分态聚合 (六态词表 + 合计), 与行同一刻同源。
fn counts_from_rows(tasks: &[TaskPanelDto]) -> TaskCountsDto {
    let mut counts = TaskCountsDto::default();
    for task in tasks {
        match task.state.as_str() {
            "pending" => counts.pending += 1,
            "assigned" => counts.assigned += 1,
            "running" => counts.running += 1,
            "done" => counts.done += 1,
            "failed" => counts.failed += 1,
            "blocked" => counts.blocked += 1,
            _ => {}
        }
    }
    counts.total = tasks.len() as u64;
    counts
}

/// `POST /v1/panel/tasks/{task_id}/human-judge-retry` —— 人判重试 (唯一动作面)。
///
/// G 组核心: `failed` 只能人判, 失败绝不自动重试; 本口是显式人判动作的 HTTP
/// 投影。**无增删改派口** (防越权): 那些是工具面的写面 (人工审批档), 不进面板。
pub async fn human_judge_retry(
    State(state): State<GatewayState>,
    Path(task_id): Path<String>,
    user: Option<Extension<UserContext>>,
    client: Option<Extension<ClientIdentity>>,
    Json(request): Json<HumanJudgeRetryRequest>,
) -> Response {
    let Some(tasks) = &state.services.tasks else {
        return crate::panels::unsupported("tasks.human_judge_retry");
    };
    let judge = judge_identity(user.as_ref(), client.as_ref());
    let reason = request.reason.as_deref().unwrap_or(DEFAULT_RETRY_REASON);
    match tasks.human_judge_retry(&task_id, &judge, reason).await {
        Ok(task) => (StatusCode::OK, Json(HumanJudgeRetryResponse { task })).into_response(),
        Err(error) => error.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use apeireth_governance::{UserIdentity, UserRole};

    use super::*;

    /// 人判署名解析链: 绑定用户赢过客户端署名; 两者皆无 = 如实未归属。
    #[test]
    fn judge_identity_comes_only_from_the_resolution_chain() {
        let bound = UserContext {
            user: Some(UserIdentity::new("user-a", UserRole::Owner, Vec::new()).expect("身份")),
        };
        let client = ClientIdentity::authenticated("tui");
        assert_eq!(
            judge_identity(Some(&Extension(bound)), Some(&Extension(client.clone()))),
            "user-a",
            "绑定用户是第一署名"
        );
        assert_eq!(
            judge_identity(None, Some(&Extension(client))),
            "tui",
            "无绑定用户时署令牌门客户端名"
        );
        assert_eq!(judge_identity(None, None), "unattributed");
    }
}
