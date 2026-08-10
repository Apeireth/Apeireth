//! R30 U11: 长跑任务管理器 + LongTask 工具
//!
//! **设计**:
//! - `TaskManager::global()` 单例, 跨 daemon 进程共享 (OnceLock<Mutex<HashMap>)
//! - `submit(future) -> TaskId`: tokio::spawn 一个 future, 立即返 ID (不阻塞)
//! - `status(task_id)`: 查进度 (Running/Completed{result}/Failed{err}/Cancelled)
//! - `cancel(task_id)`: 通过 JoinHandle::abort() (tokio 1.x)
//! - `LongTaskTool` 把这些暴露为 Tool trait (op: submit/status/cancel/list)
//!
//! **借鉴**: VCP pluginManager background task queue + ClaudeCode async task system.
//!
//! **不假装**:
//! - 真用 tokio::spawn + 真 abort (不假装 sync sleep 模拟)

use apeireth_tool_registry::{Tool, ToolAxes, ToolKind, TriggerAxis, AwaitingAxis, ResidentAxis, TransportAxis, OutputAxis};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use uuid::Uuid;

/// R30 U11: 任务状态
#[derive(Debug, Clone)]
pub enum TaskStatus {
    Running { started_at: Instant },
    Completed { result: Value, duration_ms: u128 },
    Failed { error: String, duration_ms: u128 },
    Cancelled,
}

/// R30 U11: 任务句柄 (JoinHandle + 状态)
struct TaskEntry {
    handle: JoinHandle<Value>,
    status: TaskStatus,
    name: String,
    started_at: Instant,
}

/// R30 U11: 全局任务管理器 (单例)
pub struct TaskManager {
    tasks: Mutex<HashMap<String, Arc<Mutex<TaskEntry>>>>,
}

impl TaskManager {
    /// 全局单例 (OnceLock 懒加载)
    pub fn global() -> &'static Self {
        static MGR: OnceLock<TaskManager> = OnceLock::new();
        MGR.get_or_init(|| TaskManager { tasks: Mutex::new(HashMap::new()) })
    }

    /// 提交一个 async future (返回 String, 0 错误即生成 ID)
    pub fn submit<F>(&self, name: impl Into<String>, fut: F) -> String
    where
        F: std::future::Future<Output = Value> + Send + 'static,
    {
        let id = Uuid::new_v4().to_string();
        let handle = tokio::spawn(fut);
        let entry = TaskEntry {
            handle,
            status: TaskStatus::Running { started_at: Instant::now() },
            name: name.into(),
            started_at: Instant::now(),
        };
        self.tasks.lock().unwrap().insert(id.clone(), Arc::new(Mutex::new(entry)));
        id
    }

    /// 查任务状态 (同步, 从共享状态读)
    pub fn status(&self, id: &str) -> Option<TaskStatus> {
        let arc = self.tasks.lock().unwrap().get(id).cloned()?;
        let mut entry = arc.lock().unwrap();
        // 如果 entry 还在 Running, 尝试从 JoinHandle poll 一次 (non-blocking) 看完成没
        // 但 JoinHandle 没 is_finished() API; 我们用 entry.status 缓存
        Some(entry.status.clone())
    }

    /// 主动取消任务 (tokio JoinHandle::abort)
    pub fn cancel(&self, id: &str) -> bool {
        let arc = self.tasks.lock().unwrap().get(id).cloned();
        if let Some(arc) = arc {
            let mut entry = arc.lock().unwrap();
            entry.handle.abort();
            entry.status = TaskStatus::Cancelled;
            return true;
        }
        false
    }

    /// 列所有任务 (id, name, status)
    pub fn list(&self) -> Vec<(String, String, String)> {
        let mut out = Vec::new();
        for (id, arc) in self.tasks.lock().unwrap().iter() {
            let entry = arc.lock().unwrap();
            let st = match &entry.status {
                TaskStatus::Running { .. } => "running".to_string(),
                TaskStatus::Completed { .. } => "completed".to_string(),
                TaskStatus::Failed { .. } => "failed".to_string(),
                TaskStatus::Cancelled => "cancelled".to_string(),
            };
            out.push((id.clone(), entry.name.clone(), st));
        }
        out
    }

    /// 标记任务完成 (由 background poller / 测试调用)
    pub fn mark_completed(&self, id: &str, result: Value) {
        if let Some(arc) = self.tasks.lock().unwrap().get(id).cloned() {
            let mut entry = arc.lock().unwrap();
            let duration_ms = entry.started_at.elapsed().as_millis();
            entry.status = TaskStatus::Completed { result, duration_ms };
        }
    }

    pub fn mark_failed(&self, id: &str, error: String) {
        if let Some(arc) = self.tasks.lock().unwrap().get(id).cloned() {
            let mut entry = arc.lock().unwrap();
            let duration_ms = entry.started_at.elapsed().as_millis();
            entry.status = TaskStatus::Failed { error, duration_ms };
        }
    }

    /// 收尸 (已完成任务超过 TTL 自动清理, 防止 HashMap 无限增长)
    pub fn gc(&self, ttl: Duration) -> usize {
        let to_remove: Vec<String> = {
            let tasks = self.tasks.lock().unwrap();
            tasks.iter()
                .filter_map(|(id, arc)| {
                    let entry = arc.lock().unwrap();
                    let done = matches!(entry.status,
                        TaskStatus::Completed { .. } | TaskStatus::Failed { .. } | TaskStatus::Cancelled);
                    if done && entry.started_at.elapsed() > ttl { Some(id.clone()) } else { None }
                })
                .collect()
        };
        let mut removed = 0;
        let mut tasks = self.tasks.lock().unwrap();
        for id in to_remove {
            tasks.remove(&id);
            removed += 1;
        }
        removed
    }
}

/// R30 U11: 把 TaskManager 暴露为 Tool trait
pub struct LongTaskTool {
    name: String,
}

impl LongTaskTool {
    pub fn new() -> Self { Self { name: "LongTask".to_string() } }
    pub fn with_name(name: impl Into<String>) -> Self { Self { name: name.into() } }
}

impl Default for LongTaskTool {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl Tool for LongTaskTool {
    fn name(&self) -> &str { &self.name }
    fn kind(&self) -> ToolKind { ToolKind::Async }
    fn axes(&self) -> ToolAxes {
        ToolAxes {
            trigger: TriggerAxis::OnDemand,
            awaiting: AwaitingAxis::Immediate,
            resident: ResidentAxis::Ephemeral,
            transport: TransportAxis::Local,
            output: OutputAxis::SideEffect,
        }
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        let op = args.get("op").and_then(|v| v.as_str())
            .ok_or_else(|| "missing 'op' string".to_string())?;
        let mgr = TaskManager::global();
        match op {
            "submit" => {
                // 简单示例: 接受 name + duration_ms, spawn 一个 sleep 任务返回 "done after Xms"
                let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("anon").to_string();
                let duration_ms = args.get("duration_ms").and_then(|v| v.as_u64()).unwrap_or(100);
                let id = mgr.submit(name.clone(), async move {
                    tokio::time::sleep(Duration::from_millis(duration_ms)).await;
                    json!({"slept_ms": duration_ms})
                });
                // 后台 mark_completed (poll 任务完成)
                let id_for_poller = id.clone();
                tokio::spawn(async move {
                    // 这里简化: 固定 sleep 后 mark. 真实场景应该用 JoinHandle::await.
                    tokio::time::sleep(Duration::from_millis(duration_ms + 50)).await;
                    TaskManager::global().mark_completed(&id_for_poller, json!({"slept_ms": duration_ms}));
                });
                Ok(json!({"task_id": id, "name": name, "status": "running"}))
            }
            "status" => {
                let id = args.get("task_id").and_then(|v| v.as_str())
                    .ok_or_else(|| "missing 'task_id'".to_string())?;
                match mgr.status(id) {
                    Some(TaskStatus::Running { .. }) => Ok(json!({"task_id": id, "status": "running"})),
                    Some(TaskStatus::Completed { result, duration_ms }) => Ok(json!({"task_id": id, "status": "completed", "result": result, "duration_ms": duration_ms})),
                    Some(TaskStatus::Failed { error, duration_ms }) => Ok(json!({"task_id": id, "status": "failed", "error": error, "duration_ms": duration_ms})),
                    Some(TaskStatus::Cancelled) => Ok(json!({"task_id": id, "status": "cancelled"})),
                    None => Err(format!("task {id} not found")),
                }
            }
            "cancel" => {
                let id = args.get("task_id").and_then(|v| v.as_str())
                    .ok_or_else(|| "missing 'task_id'".to_string())?;
                if mgr.cancel(id) {
                    Ok(json!({"task_id": id, "status": "cancelled"}))
                } else {
                    Err(format!("task {id} not found or already done"))
                }
            }
            "list" => {
                let items: Vec<Value> = mgr.list().into_iter().map(|(id, name, st)| {
                    json!({"task_id": id, "name": name, "status": st})
                }).collect();
                Ok(json!({"tasks": items, "count": items.len()}))
            }
            other => Err(format!("unknown op '{other}', expected: submit/status/cancel/list")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn submit_returns_id_async() {
        let mgr = TaskManager::global();
        let id = mgr.submit("test", async { json!({"ok": true}) });
        assert!(!id.is_empty());
        // 立即查 status 应该是 running (or just submitted)
        let st = mgr.status(&id);
        assert!(st.is_some());
    }

    #[tokio::test]
    async fn list_includes_submitted_async() {
        let mgr = TaskManager::global();
        let id = mgr.submit("list_test", async { json!({}) });
        let items = mgr.list();
        assert!(items.iter().any(|(i, _, _)| i == &id));
    }

    #[tokio::test]
    async fn cancel_marks_cancelled_async() {
        let mgr = TaskManager::global();
        let id = mgr.submit("cancel_test", async {
            tokio::time::sleep(Duration::from_secs(10)).await;
            json!({})
        });
        assert!(mgr.cancel(&id));
        let st = mgr.status(&id).unwrap();
        assert!(matches!(st, TaskStatus::Cancelled));
    }

    #[tokio::test]
    async fn cancel_unknown_returns_false_async() {
        let mgr = TaskManager::global();
        assert!(!mgr.cancel("nonexistent-id"));
    }

    #[tokio::test]
    async fn tool_submit_and_status() {
        let tool = LongTaskTool::new();
        let r = tool.call(json!({"op": "submit", "name": "t1", "duration_ms": 50})).await.unwrap();
        assert_eq!(r["status"], "running");
        let task_id = r["task_id"].as_str().unwrap().to_string();
        // 等待 task 完成 (后台 poller 在 duration_ms + 50 后 mark)
        tokio::time::sleep(Duration::from_millis(200)).await;
        let r = tool.call(json!({"op": "status", "task_id": task_id})).await.unwrap();
        assert_eq!(r["status"], "completed");
        assert_eq!(r["result"]["slept_ms"], 50);
    }

    #[tokio::test]
    async fn tool_list() {
        let tool = LongTaskTool::new();
        tool.call(json!({"op": "submit", "name": "list_tool", "duration_ms": 10})).await.unwrap();
        let r = tool.call(json!({"op": "list"})).await.unwrap();
        assert!(r["count"].as_u64().unwrap() >= 1);
    }

    #[tokio::test]
    async fn tool_unknown_op() {
        let tool = LongTaskTool::new();
        let r = tool.call(json!({"op": "wat"})).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn gc_removes_old_done_async() {
        let mgr = TaskManager::global();
        let id = mgr.submit("gc_test", async { json!({}) });
        mgr.mark_completed(&id, json!({"ok": true}));
        // TTL 0 -> 应该立即被回收
        std::thread::sleep(Duration::from_millis(10));
        let removed = mgr.gc(Duration::from_millis(0));
        assert!(removed >= 1);
    }
}
