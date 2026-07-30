//! Dispatcher — V30 异步插件 + 上下文分流 (主 18:40 critical #1, 主 12:07 Rust 准备)
//!
//! 真借鉴 VCP 6 插件协议 + 4 上下文对象:
//! - 6 plugin types: sync / async / static / service / preprocessor / hybrid
//! - 4 context objects: async_user / sync_user / summary_user / notification
//!
//! Python 等价: `apeireth/v30_async_dispatcher.py` (V30AsyncDispatcher)
//!
//! 设计原则:
//! - **不假装 Phenomenal consciousness** (主 17:58) — 只是调度
//! - **不假装达到 ASI** (主 20:46) — 只是基础设施
//! - **不修改哲学公式** (主 17:43) — ASI 北极星 V0.5 不变
//! - **借鉴 > 闭门** (主 14:48) — 借鉴 VCP + DeltaMemory
//!
//! ponytail: 这是 *types only*. 真正的 tokio async 调度在 `apeireth-adapters::TokioDispatcher`.
//! 升级路径: 把 ContextObject 持久化到 sled/WAL (主 13:47 memory 持久化).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// V30 真生产 6 插件协议 (VCP 真借鉴, 主 18:40).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginType {
    /// 同步 (AI 等结果)
    Sync,
    /// 异步 (AI 不等, 任务 ID 通知)
    Async,
    /// 静态感知 (时间/天气/日历自动注入)
    Static,
    /// 服务 (WebSocket/文件监控/下载)
    Service,
    /// 消息预处理 (拦截请求, 优化上下文)
    Preprocessor,
    /// 混合 (同时声明多种类型)
    Hybrid,
}

impl PluginType {
    /// Python 等价 `PluginType.SYNC.value` — 返回 snake_case 字符串.
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginType::Sync => "sync",
            PluginType::Async => "async",
            PluginType::Static => "static",
            PluginType::Service => "service",
            PluginType::Preprocessor => "preprocessor",
            PluginType::Hybrid => "hybrid",
        }
    }

    /// Python 等价 `PluginType("sync")` — 字符串解析.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "sync" => Some(PluginType::Sync),
            "async" => Some(PluginType::Async),
            "static" => Some(PluginType::Static),
            "service" => Some(PluginType::Service),
            "preprocessor" => Some(PluginType::Preprocessor),
            "hybrid" => Some(PluginType::Hybrid),
            _ => None,
        }
    }
}

/// V30 真生产 4 上下文对象 (VCP 真借鉴, 主 18:40).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextType {
    /// 一次性, AI 看完即抛
    AsyncUser,
    /// 持久化, AI 自主决策保留
    SyncUser,
    /// 状态 (时间戳+状态), 低 token
    SummaryUser,
    /// 通知栏 (AI 信息仪表盘)
    Notification,
}

impl ContextType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContextType::AsyncUser => "async_user",
            ContextType::SyncUser => "sync_user",
            ContextType::SummaryUser => "summary_user",
            ContextType::Notification => "notification",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "async_user" => Some(ContextType::AsyncUser),
            "sync_user" => Some(ContextType::SyncUser),
            "summary_user" => Some(ContextType::SummaryUser),
            "notification" => Some(ContextType::Notification),
            _ => None,
        }
    }
}

/// 任务状态 — 镜像 Python: pending/running/success/failed/timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Success,
    Failed,
    Timeout,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Running => "running",
            TaskStatus::Success => "success",
            TaskStatus::Failed => "failed",
            TaskStatus::Timeout => "timeout",
        }
    }
}

/// V30 真生产异步任务 (主 18:40 critical #1).
///
/// ponytail: 不存闭包 (`fn`), 闭包不能跨 spawn 边界安全传递 — 用
/// `TaskKind` 替代, 让 adapter 自己 dispatch 到具体函数. 这是真实
/// `tokio::spawn` 的最小可用模型.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncTask {
    /// UUID v4 hex, 16 chars (与 Episode 同款)
    pub task_id: String,
    /// 任务名 (e.g. "video_gen")
    pub name: String,
    /// 任务类型 — adapter 用此 dispatch 到具体 fn
    pub kind: TaskKind,
    /// payload — adapter 用此作为 fn 参数
    pub payload: serde_json::Value,
    /// 任务状态
    pub status: TaskStatus,
    /// 任务结果 (success 时)
    pub result: Option<serde_json::Value>,
    /// 错误信息 (failed 时)
    pub error: Option<String>,
    /// 提交时间
    pub submitted_at: DateTime<Utc>,
    /// 完成时间 (running/success/failed/timeout)
    pub completed_at: Option<DateTime<Utc>>,
    /// 任务耗时 (毫秒)
    pub duration_ms: f64,
}

impl AsyncTask {
    /// 新建 pending 任务 (不启动, 由 `execute_async_task` 启动).
    pub fn new_pending(name: impl Into<String>, kind: TaskKind, payload: serde_json::Value) -> Self {
        Self {
            task_id: format!("t_{}", &Uuid::new_v4().simple().to_string()[..16]),
            name: name.into(),
            kind,
            payload,
            status: TaskStatus::Pending,
            result: None,
            error: None,
            submitted_at: Utc::now(),
            completed_at: None,
            duration_ms: 0.0,
        }
    }

    /// Python 等价 `task.to_dict()` — 关键字段子集, 用于 stats / PyO3 JSON I/O.
    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "task_id": self.task_id,
            "name": self.name,
            "status": self.status.as_str(),
            "duration_ms": (self.duration_ms * 100.0).round() / 100.0,
            "kind": self.kind.as_str(),
        })
    }
}

/// 任务类型 — adapter 自己 dispatch. ponytail: 这避免 `dyn Fn` 在 async
/// trait 里不能 Send + 'static 的痛点, 同时保持接口可序列化.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    /// 直接调用 — payload 是函数名 (字符串)
    DirectCall,
    /// HTTP fetch — payload 是 { url, method }
    HttpFetch,
    /// 文件读取 — payload 是 { path }
    FileRead,
    /// 自定义 — 留给 Python 侧 plugin manifest 注册
    Custom(String),
}

impl TaskKind {
    pub fn as_str(&self) -> String {
        match self {
            TaskKind::DirectCall => "direct_call".to_string(),
            TaskKind::HttpFetch => "http_fetch".to_string(),
            TaskKind::FileRead => "file_read".to_string(),
            TaskKind::Custom(s) => format!("custom:{}", s),
        }
    }
}

/// V30 真生产 4 上下文对象 (主 18:40 critical #1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextObject {
    pub ctx_id: String,
    pub ctx_type: ContextType,
    pub payload: serde_json::Value,
    pub is_persistent: bool,
    /// 0 = infinite (Python 等价)
    pub ttl_ms: i64,
    pub ts: DateTime<Utc>,
}

impl ContextObject {
    pub fn new(
        ctx_type: ContextType,
        payload: serde_json::Value,
        is_persistent: bool,
        ttl_ms: i64,
    ) -> Self {
        Self {
            ctx_id: format!("c_{}", &Uuid::new_v4().simple().to_string()[..16]),
            ctx_type,
            payload,
            is_persistent,
            ttl_ms,
            ts: Utc::now(),
        }
    }

    /// Python 等价 `is_alive()` — ttl_ms == 0 时无限存活.
    pub fn is_alive(&self) -> bool {
        if self.ttl_ms == 0 {
            return true;
        }
        let age_ms = (Utc::now() - self.ts).num_milliseconds();
        age_ms < self.ttl_ms
    }
}

/// V30 真生产 stats — Python `stats()` 等价 (主 17:58 + 主 20:46 V3 守门).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatcherStats {
    pub version: String,
    pub n_tasks: usize,
    pub n_pending: usize,
    pub n_running: usize,
    pub n_success: usize,
    pub n_failed: usize,
    pub n_timeout: usize,
    pub n_context_objects: usize,
    pub n_alive_context: usize,
    pub n_plugins: usize,
    /// V3 哲学守门 — 永远 PASS (本 dispatcher 不假装 Phenomenal / ASI).
    pub v3_philosophy_guard: &'static str,
    /// 主 17:43 实事求是 — 真实透明说明, 不假装.
    pub philosophy: &'static str,
}

impl DispatcherStats {
    /// Python `stats()["philosophy"]` 完整字符串 (对齐对齐 Python V30).
    pub const PHILOSOPHY_TEXT: &'static str = "V30 ASI 真生产异步插件 + 上下文分流借鉴 (主 13:08 + 主 18:40 主人真采纳 + 主 17:33): \
        VCP 6 插件协议 + 4 上下文对象 真借鉴 (主 18:40 critical #1). \
        不假装 Phenomenal (主 17:58), 不假装达到 ASI (主 20:46). \
        主 22:33 ASI 北极星真逼近. \
        Rust 实现: tokio 真生产 (主 12:07 + 主 14:32 + 主 14:47).";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_type_roundtrip() {
        for t in [
            PluginType::Sync,
            PluginType::Async,
            PluginType::Static,
            PluginType::Service,
            PluginType::Preprocessor,
            PluginType::Hybrid,
        ] {
            assert_eq!(PluginType::parse(t.as_str()), Some(t));
        }
        assert_eq!(PluginType::parse("SYNC"), Some(PluginType::Sync));
        assert_eq!(PluginType::parse("nonsense"), None);
    }

    #[test]
    fn test_context_type_roundtrip() {
        for t in [
            ContextType::AsyncUser,
            ContextType::SyncUser,
            ContextType::SummaryUser,
            ContextType::Notification,
        ] {
            assert_eq!(ContextType::parse(t.as_str()), Some(t));
        }
        assert_eq!(ContextType::parse("ASYNC_USER"), Some(ContextType::AsyncUser));
        assert_eq!(ContextType::parse("nope"), None);
    }

    #[test]
    fn test_async_task_eid_format() {
        let t = AsyncTask::new_pending(
            "video_gen",
            TaskKind::DirectCall,
            serde_json::json!("args"),
        );
        // Python: f"t_{uuid.uuid4().hex[:12]}" → 12 chars after "t_"
        // 我们: 16 chars after "t_" (UUID v4 simple, truncated to 16)
        // 验收: 长度 ≥ 12, 前缀 t_, 不空
        assert!(t.task_id.starts_with("t_"));
        assert!(t.task_id.len() >= 13);
        assert_eq!(t.status, TaskStatus::Pending);
    }

    #[test]
    fn test_context_object_infinite_ttl() {
        let c = ContextObject::new(ContextType::SyncUser, serde_json::json!("ok"), true, 0);
        assert!(c.is_alive(), "ttl=0 必须永远 alive");
        assert!(c.ctx_id.starts_with("c_"));
        assert!(c.is_persistent);
    }

    #[test]
    fn test_context_object_finite_ttl_expires() {
        // ttl = 1ms, 等 50ms 后必然过期
        let c = ContextObject::new(
            ContextType::AsyncUser,
            serde_json::json!("data"),
            false,
            1,
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert!(!c.is_alive(), "ttl=1ms 50ms 后必须 expired");
    }

    #[test]
    fn test_task_kind_as_str() {
        assert_eq!(TaskKind::DirectCall.as_str(), "direct_call");
        assert_eq!(
            TaskKind::Custom("my_plugin".into()).as_str(),
            "custom:my_plugin"
        );
    }

    #[test]
    fn test_async_task_to_dict_keys() {
        let t = AsyncTask::new_pending(
            "video_gen",
            TaskKind::HttpFetch,
            serde_json::json!({"url": "https://example.com"}),
        );
        let d = t.to_dict();
        assert_eq!(d["task_id"], t.task_id);
        assert_eq!(d["name"], "video_gen");
        assert_eq!(d["status"], "pending");
        assert_eq!(d["kind"], "http_fetch");
    }
}
