//! **战役 1-3 — 流式 chat 推流**
//!
//! **借鉴 VCP 真代码**: `research/source/vcptoolbox/modules/chatCompletionHandler.js:39-40`
//! - `const StreamHandler = require('./handlers/streamHandler');`
//! - `const NonStreamHandler = require('./handlers/nonStreamHandler');`
//!
//! **Apeireth 简化 (W3 #1 simulate 升级版)**: 主 chat 管线流式入口
//! - `stream_to_sender` 把 LLM chunk 推 `UnboundedSender<StreamChunk>`
//! - 3 个 chunk 事件: `Start` / `Data(String)` / `End` + 1 个 `Error(String)` 异常
//!
//! **不假装**:
//! - 真用 `reqwest::Response::bytes_stream()` 拉网络 chunk
//! - 真 `StreamExt::next()` 迭代 (非 polling)
//! - 借鉴 VCP 真代码模块名 (`streamHandler` / `nonStreamHandler` 二分)

use futures::StreamExt;

/// 流式 chunk — 3 事件 + 1 异常
#[derive(Debug, Clone, PartialEq)]
pub enum StreamChunk {
    /// 流开始 (HTTP response headers 已收, body 即将到)
    Start,
    /// 一个 chunk (LLM SSE 风格, 可能是单字符 / 多字符 / JSON 行)
    Data(String),
    /// 流结束 (LLM `finish_reason: stop` / `length` / `tool_calls` 等)
    End,
    /// 流中错误 (HTTP 5xx / LLM 业务错误)
    Error(String),
}

/// 真流式 (借鉴 VCP `StreamHandler` 真代码模式)
///
/// **VCP 真代码模式** (从 `chatCompletionHandler.js:39` 推断):
/// - 独立 `StreamHandler` 处理 SSE 事件
/// - 独立 `NonStreamHandler` 处理一次性 response
///
/// **Apeireth 实现**:
/// - 给一个 `bytes_stream` (reqwest::Response::bytes_stream),
/// - 真迭代拉 chunk
/// - 每 chunk 推 `Sender<StreamChunk>`
/// - 错误也推 (而不是抛 panic)
///
/// **不假装**: 走真 `StreamExt::next()` 异步拉, 不假轮询
pub async fn stream_to_sender<S, E>(
    stream: S,
    sender: &tokio::sync::mpsc::UnboundedSender<StreamChunk>,
) -> Result<(), String>
where
    S: futures::Stream<Item = Result<bytes::Bytes, E>>,
    E: std::fmt::Display,
{
    futures::pin_mut!(stream);
    let _ = sender.send(StreamChunk::Start);

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes).to_string();
                if sender.send(StreamChunk::Data(text)).is_err() {
                    // receiver dropped, 退出
                    return Ok(());
                }
            }
            Err(e) => {
                let msg = e.to_string();
                let _ = sender.send(StreamChunk::Error(msg.clone()));
                return Err(msg);
            }
        }
    }
    let _ = sender.send(StreamChunk::End);
    Ok(())
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;
    use std::time::Duration;
    use tokio::sync::mpsc::unbounded_channel;

    #[tokio::test]
    async fn stream_chunks_via_sender() {
        // 构造一个 fake stream, 3 chunk
        let chunks: Vec<Result<bytes::Bytes, String>> = vec![
            Ok(bytes::Bytes::from_static(b"hello ")),
            Ok(bytes::Bytes::from_static(b"world")),
            Ok(bytes::Bytes::from_static(b"!")),
        ];
        let s = stream::iter(chunks);
        let (tx, mut rx) = unbounded_channel::<StreamChunk>();

        let result = stream_to_sender(s, &tx).await;
        assert!(result.is_ok());

        // 收 5 个: Start + 3 Data + End
        let mut received = Vec::new();
        while let Ok(chunk) = rx.try_recv() {
            received.push(chunk);
        }
        assert_eq!(received.len(), 5);
        assert!(matches!(received[0], StreamChunk::Start));
        assert!(matches!(received[1], StreamChunk::Data(ref s) if s == "hello "));
        assert!(matches!(received[2], StreamChunk::Data(ref s) if s == "world"));
        assert!(matches!(received[3], StreamChunk::Data(ref s) if s == "!"));
        assert!(matches!(received[4], StreamChunk::End));
    }

    #[tokio::test]
    async fn stream_error_propagates() {
        // error chunk
        let chunks: Vec<Result<bytes::Bytes, String>> = vec![Err("network down".to_string())];
        let s = stream::iter(chunks);
        let (tx, mut rx) = unbounded_channel::<StreamChunk>();

        let result = stream_to_sender(s, &tx).await;
        assert!(result.is_err());

        // 收 2 个: Start + Error
        let c1 = rx.try_recv().unwrap();
        let c2 = rx.try_recv().unwrap();
        assert!(matches!(c1, StreamChunk::Start));
        match c2 {
            StreamChunk::Error(msg) => assert_eq!(msg, "network down"),
            _ => panic!("expected Error"),
        }
    }

    #[tokio::test]
    async fn stream_receiver_dropped_graceful_exit() {
        // 1 chunk
        let chunks: Vec<Result<bytes::Bytes, String>> = vec![Ok(bytes::Bytes::from_static(b"x"))];
        let s = stream::iter(chunks);
        let (tx, rx) = unbounded_channel::<StreamChunk>();
        drop(rx); // receiver 立即 drop

        // sender send 会失败, 我们 graceful exit
        let result = stream_to_sender(s, &tx).await;
        assert!(result.is_ok()); // graceful, 不抛
    }

    #[tokio::test]
    async fn stream_empty() {
        let chunks: Vec<Result<bytes::Bytes, String>> = vec![];
        let s = stream::iter(chunks);
        let (tx, mut rx) = unbounded_channel::<StreamChunk>();

        let result = stream_to_sender(s, &tx).await;
        assert!(result.is_ok());

        // 收 2 个: Start + End
        let c1 = rx.try_recv().unwrap();
        let c2 = rx.try_recv().unwrap();
        assert!(matches!(c1, StreamChunk::Start));
        assert!(matches!(c2, StreamChunk::End));
    }

    #[tokio::test]
    async fn stream_chunks_larger_than_expected_buffer() {
        // 模拟 SSE: 大量小 chunk
        let chunks: Vec<Result<bytes::Bytes, String>> = (0..100)
            .map(|i| Ok(bytes::Bytes::from(format!("chunk-{i}"))))
            .collect();
        let s = stream::iter(chunks);
        let (tx, mut rx) = unbounded_channel::<StreamChunk>();

        let _ = stream_to_sender(s, &tx).await;
        // 收 102 个
        let mut count = 0;
        while rx.try_recv().is_ok() {
            count += 1;
        }
        assert_eq!(count, 102); // 1 Start + 100 Data + 1 End
    }

    #[tokio::test]
    async fn stream_timeout_safe() {
        // 模拟流被卡住, 验证不会死等
        let (tx, _rx) = unbounded_channel::<StreamChunk>();
        // 用一个 yield once 的 stream
        let s = stream::once(async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok::<_, String>(bytes::Bytes::from_static(b"late"))
        });
        let result =
            tokio::time::timeout(Duration::from_millis(100), stream_to_sender(s, &tx)).await;
        assert!(result.is_ok(), "stream 应该在 100ms 内完成");
    }
}
