//! types — 6 类插件枚举 + 公共结构 (AuditEntry 等)

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// 6 类插件 (按 *执行语义* 分类)
///
/// 注意: 与 VCP tool/resource/prompt/sampling/elicitation/root 是 *不同* 维度.
/// VCP 是"做什么" (what), 这里是"怎么执行" (how).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginKind {
    /// 同步执行 (阻塞调用, 立即返回结果)
    Sync,
    /// 异步执行 (返回 Future)
    Async,
    /// 启动期一次性加载, 不可热替换
    Static,
    /// 长驻 service, 启动后持续运行
    Service,
    /// 消息中间件 (preprocess / transform)
    MessagePreprocessor,
    /// 同步入口 + 异步后端 (混合)
    Hybrid,
}

impl PluginKind {
    /// 6 类全部 (常量顺序)
    pub const ALL: &'static [PluginKind] = &[
        PluginKind::Sync,
        PluginKind::Async,
        PluginKind::Static,
        PluginKind::Service,
        PluginKind::MessagePreprocessor,
        PluginKind::Hybrid,
    ];

    /// 字符串表示
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginKind::Sync => "sync",
            PluginKind::Async => "async",
            PluginKind::Static => "static",
            PluginKind::Service => "service",
            PluginKind::MessagePreprocessor => "message_preprocessor",
            PluginKind::Hybrid => "hybrid",
        }
    }

    /// 字符串反解 (失败返回 None)
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "sync" => Some(PluginKind::Sync),
            "async" => Some(PluginKind::Async),
            "static" => Some(PluginKind::Static),
            "service" => Some(PluginKind::Service),
            "message_preprocessor" | "messagePreprocessor" | "preprocessor" => {
                Some(PluginKind::MessagePreprocessor)
            }
            "hybrid" => Some(PluginKind::Hybrid),
            _ => None,
        }
    }
}

impl fmt::Display for PluginKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 一条审计日志 (调用前后写入)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditEntry {
    /// 唯一 trace id
    pub trace_id: String,
    /// 插件名
    pub plugin: String,
    /// 插件 kind
    pub kind: PluginKind,
    /// 输入字节数
    pub input_bytes: usize,
    /// 输出字节数 (失败时为 0)
    pub output_bytes: usize,
    /// 耗时 (微秒)
    pub elapsed_us: u128,
    /// 是否成功
    pub success: bool,
    /// 错误信息 (成功时 None)
    pub error: Option<String>,
    /// 时间戳 (ISO 8601 RFC3339)
    pub timestamp: String,
}

impl AuditEntry {
    /// 构造成功审计
    pub fn success(
        plugin: &str,
        kind: PluginKind,
        input_bytes: usize,
        output_bytes: usize,
        elapsed_us: u128,
    ) -> Self {
        Self {
            trace_id: Uuid::new_v4().to_string(),
            plugin: plugin.to_string(),
            kind,
            input_bytes,
            output_bytes,
            elapsed_us,
            success: true,
            error: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// 构造失败审计
    pub fn failure(
        plugin: &str,
        kind: PluginKind,
        input_bytes: usize,
        elapsed_us: u128,
        err: impl Into<String>,
    ) -> Self {
        Self {
            trace_id: Uuid::new_v4().to_string(),
            plugin: plugin.to_string(),
            kind,
            input_bytes,
            output_bytes: 0,
            elapsed_us,
            success: false,
            error: Some(err.into()),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}
