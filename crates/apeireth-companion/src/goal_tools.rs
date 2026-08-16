//! `apeireth-companion::goal_tools` — 目标驱动工具 (模块 6: Goal Integration).
//!
//! 设计 (docs/companion-gaps.md 模块 6): 注入侧已有 (状态块含当前目标),
//! 本模块补**写入侧** — AI 可建目标/查状态/报进度 (complete/block/pause/resume),
//! 让「目标」从单向注入变成双向驱动 (它知道自己要干什么、干到哪、向你汇报).
//!
//! 安全: 目标状态机由 GoalService 严格守护 (revision+1, 非法迁移拒绝);
//! 工具只暴露合法操作, 不绕过状态机. 低风险 (无副作用外泄), 白名单直放.

use std::sync::{Arc, Mutex};

use apeireth_tool_registry::{Tool, ToolAxes, ToolKind};
use serde_json::{json, Value};

use crate::goal::{GoalPhase, GoalService};

/// 目标工具共享的 GoalService (与 serve 注入侧同一实例).
pub type SharedGoals = Arc<Mutex<GoalService>>;

/// 「建立目标」工具 — 模块 6 写入侧.
pub struct GoalCreateTool {
    goals: SharedGoals,
}

impl GoalCreateTool {
    pub fn new(goals: SharedGoals) -> Self {
        Self { goals }
    }
}

#[async_trait::async_trait]
impl Tool for GoalCreateTool {
    fn name(&self) -> &str {
        "goal_create"
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Sync
    }
    fn axes(&self) -> ToolAxes {
        ToolAxes::default()
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        let objective = args
            .get("objective")
            .and_then(|v| v.as_str())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "objective (目标描述) 不能为空".to_string())?;
        let max_rounds = args.get("max_rounds").and_then(|v| v.as_u64()).unwrap_or(20);
        let mut goals = self.goals.lock().unwrap();
        match goals.create(objective, max_rounds) {
            Ok(g) => Ok(json!({
                "ok": true, "id": g.id, "objective": g.objective,
                "phase": g.phase.label(), "revision": g.revision,
                "note": "目标已建立 (active); 完成后 goal_status 报进度"
            })),
            Err(crate::goal::GoalError::AlreadyExists) => {
                let cur = goals.current().map(|g| g.objective.clone()).unwrap_or_default();
                Err(format!("已有未完成目标: {cur} — 先完成 (goal_complete) 或暂停 (goal_pause) 再建新目标"))
            }
            Err(e) => Err(format!("创建目标失败: {e:?}")),
        }
    }
}

/// 「目标状态」工具 — 查询当前目标 (含进度).
pub struct GoalStatusTool {
    goals: SharedGoals,
}

impl GoalStatusTool {
    pub fn new(goals: SharedGoals) -> Self {
        Self { goals }
    }
}

#[async_trait::async_trait]
impl Tool for GoalStatusTool {
    fn name(&self) -> &str {
        "goal_status"
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Sync
    }
    fn axes(&self) -> ToolAxes {
        ToolAxes::default()
    }
    async fn call(&self, _args: Value) -> Result<Value, String> {
        let goals = self.goals.lock().unwrap();
        match goals.current() {
            Some(g) => Ok(json!({
                "goal": {
                    "id": g.id, "objective": g.objective, "phase": g.phase.label(),
                    "revision": g.revision, "rounds_started": g.rounds_started,
                    "max_rounds": g.max_goal_rounds,
                    "blocked_reason": g.blocked_reason.as_ref().map(|b| json!({"code": b.code, "message": b.message})),
                }
            })),
            None => Ok(json!({"goal": null, "note": "当前无目标; 需要时用 goal_create 建立"})),
        }
    }
}

/// 「完成目标」工具 — 报进度: 目标达成 → completed (可建新目标).
pub struct GoalCompleteTool {
    goals: SharedGoals,
}

impl GoalCompleteTool {
    pub fn new(goals: SharedGoals) -> Self {
        Self { goals }
    }
}

#[async_trait::async_trait]
impl Tool for GoalCompleteTool {
    fn name(&self) -> &str {
        "goal_complete"
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Sync
    }
    fn axes(&self) -> ToolAxes {
        ToolAxes::default()
    }
    async fn call(&self, _args: Value) -> Result<Value, String> {
        let mut goals = self.goals.lock().unwrap();
        match goals.complete() {
            Ok(g) => Ok(json!({"ok": true, "phase": "completed", "objective": g.objective, "note": "目标已完成; 可建立新目标"})),
            Err(e) => Err(format!("完成目标失败: {e:?}")),
        }
    }
}

/// 「暂停目标」工具 — 主动暂停 (active → paused).
pub struct GoalPauseTool {
    goals: SharedGoals,
}

impl GoalPauseTool {
    pub fn new(goals: SharedGoals) -> Self {
        Self { goals }
    }
}

#[async_trait::async_trait]
impl Tool for GoalPauseTool {
    fn name(&self) -> &str {
        "goal_pause"
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Sync
    }
    fn axes(&self) -> ToolAxes {
        ToolAxes::default()
    }
    async fn call(&self, _args: Value) -> Result<Value, String> {
        let mut goals = self.goals.lock().unwrap();
        match goals.pause() {
            Ok(g) => Ok(json!({"ok": true, "phase": "paused", "objective": g.objective})),
            Err(e) => Err(format!("暂停失败: {e:?}")),
        }
    }
}

/// 「报告阻塞」工具 — 目标受阻 (active → blocked + 原因; 主人知情).
pub struct GoalBlockTool {
    goals: SharedGoals,
}

impl GoalBlockTool {
    pub fn new(goals: SharedGoals) -> Self {
        Self { goals }
    }
}

#[async_trait::async_trait]
impl Tool for GoalBlockTool {
    fn name(&self) -> &str {
        "goal_block"
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Sync
    }
    fn axes(&self) -> ToolAxes {
        ToolAxes::default()
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        let code = args.get("code").and_then(|v| v.as_str()).unwrap_or("blocked").trim().to_string();
        let message = args.get("message").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let mut goals = self.goals.lock().unwrap();
        match goals.block(&code, &message) {
            Ok(g) => Ok(json!({"ok": true, "phase": "blocked", "reason": g.blocked_reason.map(|b| b.message)})),
            Err(e) => Err(format!("报告阻塞失败: {e:?}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_tool_registry::Tool as _;

    fn goals() -> SharedGoals {
        Arc::new(Mutex::new(GoalService::new(std::env::temp_dir().join(format!("apeireth-goal-test-{}", uuid::Uuid::new_v4())))))
    }

    #[tokio::test]
    async fn create_status_complete_lifecycle() {
        let g = goals();
        let create = GoalCreateTool::new(Arc::clone(&g));
        let r = create.call(json!({"objective": "辅助主人通过高数期中"})).await.unwrap();
        assert_eq!(r["phase"], json!("active"));
        // 重复建 → 拒绝 (已有未完成目标)
        assert!(create.call(json!({"objective": "另一个"})).await.is_err());
        // 状态查询
        let status = GoalStatusTool::new(Arc::clone(&g));
        let s = status.call(json!({})).await.unwrap();
        assert_eq!(s["goal"]["objective"], json!("辅助主人通过高数期中"));
        // 完成 → 可再建
        let complete = GoalCompleteTool::new(Arc::clone(&g));
        complete.call(json!({})).await.unwrap();
        let s2 = status.call(json!({})).await.unwrap();
        assert_eq!(s2["goal"]["phase"], json!("completed"));
        create.call(json!({"objective": "新目标"})).await.unwrap();
        // 空 objective 拒绝
        assert!(create.call(json!({"objective": ""})).await.is_err());
    }

    #[tokio::test]
    async fn pause_and_block() {
        let g = goals();
        GoalCreateTool::new(Arc::clone(&g)).call(json!({"objective": "x"})).await.unwrap();
        let pause = GoalPauseTool::new(Arc::clone(&g));
        let p = pause.call(json!({})).await.unwrap();
        assert_eq!(p["phase"], json!("paused"));
        let block = GoalBlockTool::new(Arc::clone(&g));
        let b = block.call(json!({"code": "needs_input", "message": "等主人提供资料"})).await.unwrap();
        assert_eq!(b["phase"], json!("blocked"));
        // 无目标时完成 → 报错
        let g2 = goals();
        let complete = GoalCompleteTool::new(g2);
        assert!(complete.call(json!({})).await.is_err());
    }
}
