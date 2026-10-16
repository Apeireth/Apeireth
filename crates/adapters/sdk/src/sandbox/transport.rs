//! # Orchestration transport — mock 边界
//!
//! 多沙箱编排客户端协议层与**外部编排服务**之间的唯一边界。本仓库不包含
//! 编排服务本体; 服务以 mock 边界存在 (见 [`crate::sandbox::mock`]), 协议
//! 编解码 / 生命周期 / 配额 / 错误分类 / 超时全部是本层真实实现, 只有
//! "帧最终送达哪个服务" 这一件事在边界之外。
//!
//! 边界上传输的是**协议帧 JSON 字符串** (见 [`crate::sandbox::protocol`]):
//! 编解码在客户端真实发生, 边界不做任何类型捷径。
//!
//! 边界纪律 (零假装):
//! - [`OrchestrationTransport`] 的实现必须如实报告失败 ([`TransportError`]),
//!   不允许把失败伪装成成功响应;
//! - 本层不区分 mock 与真实服务: 二者走同一协议帧、同一错误闭合词表。

use async_trait::async_trait;
use thiserror::Error;

/// 传输层失败 (与协议错误分类正交: 传输失败 = 请求未获得可信响应)。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TransportError {
    /// 服务不可达 / 连接被拒。
    #[error("orchestration service unavailable: {0}")]
    Unavailable(String),
    /// 传输已关闭 (不再接受请求)。
    #[error("orchestration transport closed")]
    Closed,
}

/// 编排服务传输边界 (async trait)。
///
/// 一次 [`call`](OrchestrationTransport::call) = 一帧请求 JSON 进、一帧响应
/// JSON 出。超时由调用方经 `apeireth_core::deadline` 施加; 传输实现不做超时假装。
#[async_trait]
pub trait OrchestrationTransport: Send + Sync {
    /// 请求/响应往返 (协议帧 JSON)。
    async fn call(&self, request_frame: String) -> Result<String, TransportError>;
}
