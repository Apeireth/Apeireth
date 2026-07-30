//! AsyncDispatcher port — V30 async plugin + context shunt interface
//!
//! 主人 18:40 critical #1 — "插件协议多样性: 我们 V18 dispatch 3 种,
//!  VCP 6 种 (sync/async/static/service/preprocessor/hybrid)"
//!
//! 真借鉴 VCP 6 插件协议 + 4 上下文对象.
//! Python 等价: `apeireth.v30_async_dispatcher.V30AsyncDispatcher`.
//!
//! 设计:
//! - **接口在 ports**, **实现在 adapters** (Hexagonal, 主人 14:52)
//! - **Send + Sync** — 多线程安全
//! - **async-trait** — 与项目其它 port 一致
//!
//! ponytail: 不要求返回闭包结果 — 实际结果通过 `get_task_result`
//! 异步取, 这样 trait 不用带生命周期. 这是真实 async port 的最小
//! 形态, 没有假装 "future is the function".

use async_trait::async_trait;
use apeireth_core::{
    AsyncTask, ContextObject, ContextType, DispatcherStats, PluginType, TaskKind,
};
use serde_json::Value;
use super::PortError;

/// AsyncDispatcher port — V30 真生产 async plugin + context shunt.
///
/// 镜像 Python `V30AsyncDispatcher` 的公开契约:
/// - `register_plugin(name, types)`
/// - `submit_async_task(name, fn)`  → `submit_async_task(name, kind, payload)`
/// - `execute_async_task(task_id)`
/// - `push_context(ctx_type, payload, is_persistent, ttl_ms)` → `push_context`
/// - `purge_ttl_context() -> int`
/// - `stats() -> Dict` → `stats() -> DispatcherStats`
#[async_trait]
pub trait AsyncDispatcher: Send + Sync {
    /// 注册插件 manifest (VCP 真借鉴) — name + 它的插件类型列表.
    async fn register_plugin(&self, name: &str, types: Vec<PluginType>) -> Result<(), PortError>;

    /// 提交一个异步任务 (pending) — 返回 task_id.
    /// Python 等价 `submit_async_task(name, fn)` 但用 kind + payload
    /// 代替 callable (跨 async 边界).
    async fn submit_async_task(
        &self,
        name: &str,
        kind: TaskKind,
        payload: Value,
    ) -> Result<AsyncTask, PortError>;

    /// 启动 pending 任务 — adapter 内部 spawn tokio task, 完成后写回 result/error.
    /// Python 等价 `execute_async_task(task_id)` — 但 **真异步**, 不阻塞调用方.
    async fn execute_async_task(&self, task_id: &str) -> Result<(), PortError>;

    /// 取任务快照 — status / result / error / duration.
    async fn get_task(&self, task_id: &str) -> Result<AsyncTask, PortError>;

    /// 等任务完成 — 真 await 替代 Python 的轮询.
    /// 返回最终 task snapshot.
    async fn await_task(&self, task_id: &str) -> Result<AsyncTask, PortError>;

    /// 推上下文 (VCP 4 上下文对象 真借鉴).
    async fn push_context(
        &self,
        ctx_type: ContextType,
        payload: Value,
        is_persistent: bool,
        ttl_ms: i64,
    ) -> Result<String, PortError>;

    /// 列出 alive 上下文 — Python `purge_ttl_context` 之前的状态.
    async fn list_context(&self) -> Result<Vec<ContextObject>, PortError>;

    /// 清 TTL 过期上下文 — 返回清理数量.
    async fn purge_ttl_context(&self) -> Result<usize, PortError>;

    /// 真测量 stats — 镜像 Python `stats()` 字典的字段集.
    async fn stats(&self) -> Result<DispatcherStats, PortError>;
}
