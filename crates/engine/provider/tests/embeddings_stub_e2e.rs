//! **embeddings 传输链路的真 HTTP 端到端**（本地 stub · 零新依赖）。
//!
//! # 为什么有这个测试（2026-10-30）
//!
//! `docs/04-internal/live-verification-ledger.md` **挂账 #8** =
//! 「语义向量阶段 live E2E（真 embedding 端点）」——原因写着
//! 「无可用 embeddings 端点：DeepSeek 不提供 `/embeddings`，本地端点未部署」。
//!
//! ⇒ 那条挂账**真的需要外部端点吗**？**不需要**：
//! 我们要验的是**我们自己的传输链路**（HTTP 方法 / 路径 / 请求体 / 鉴权头 / 响应解析 /
//! 错误分类 / 端点不可达），**不是**厂商的实现。
//! ⇒ ⇒ 用**本地 stub 端点**跑**真 HTTP**（真 `reqwest`、真 socket、真 JSON）
//! ⇒ ⇒ 传输链路**当场可证**，**不需要 owner 提供任何端点或密钥**。
//!
//! # 口径（**如实**，不夸大）
//!
//! * ✅ **已证**：我们的 `POST /embeddings` 传输、请求体形状、bearer 头、响应解析、
//!   错误分类、不可达降级 —— **全是真 HTTP**，不是 mock 函数调用；
//! * ❌ **仍未证**：**厂商端点的真实行为**（MiniMax / OpenAI / DeepSeek 的实际响应形状与配额）
//!   ⇒ 那属于**厂商风险**，不属于**本仓代码风险**
//!   ⇒ 挂账 #8 应当按此口径**拆分**（本仓侧已证 / 厂商侧未测）。
//!
//! # 零新依赖
//!
//! 本仓有严格「**0 新依赖**」纪律 ⇒ ⇒ **不引** hyper / axum / wiremock
//! ⇒ 用 `tokio::net::TcpListener`（dev-dependency **已带** `net` feature）手写最小 HTTP 端点。
//!
//! # 怎么跑
//!
//! ```sh
//! cargo test -p apeireth-provider --test embeddings_stub_e2e --locked
//! ```

use std::sync::{Arc, Mutex};

use apeireth_memory::scope::{EmbeddingError, EmbeddingProvider};
use apeireth_provider::embeddings::OpenAiCompatibleEmbeddingProvider;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// 抓到的一次请求（供断言）。
#[derive(Debug, Clone, Default)]
struct CapturedRequest {
    /// 请求行（`POST /v1/embeddings HTTP/1.1`）。
    request_line: String,
    /// 全部请求头（小写键）。
    headers: String,
    /// 请求体。
    body: String,
}

impl CapturedRequest {
    fn header(&self, name: &str) -> Option<String> {
        let needle = format!("{name}:");
        self.headers
            .lines()
            .find(|line| line.trim().to_lowercase().starts_with(&needle))
            .map(|line| line.split_once(':').map(|(_, v)| v.trim().to_string()))
            .flatten()
    }
}

/// 起一个**最小 HTTP 端点**：接受一次连接、读请求、按 `status` + `body` 应答。
///
/// 返回 `(base_url, 捕获到的请求)`。
async fn spawn_stub(
    status_line: &'static str,
    body: &'static str,
) -> (String, Arc<Mutex<Option<CapturedRequest>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let captured: Arc<Mutex<Option<CapturedRequest>>> = Arc::new(Mutex::new(None));

    let sink = Arc::clone(&captured);
    tokio::spawn(async move {
        let Ok((mut socket, _)) = listener.accept().await else {
            return;
        };
        // 读请求头（到 `\r\n\r\n` 为止）。
        let mut raw = Vec::new();
        let mut chunk = [0_u8; 1024];
        loop {
            let Ok(read) = socket.read(&mut chunk).await else {
                return;
            };
            if read == 0 {
                break;
            }
            raw.extend_from_slice(&chunk[..read]);
            if raw.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let text = String::from_utf8_lossy(&raw).to_string();
        let (head, body_text) = text
            .split_once("\r\n\r\n")
            .map(|(head, body)| (head.to_string(), body.to_string()))
            .unwrap_or((text.clone(), String::new()));
        let mut lines = head.lines();
        let request_line = lines.next().unwrap_or_default().to_string();
        let headers = lines.collect::<Vec<_>>().join("\n");

        let mut request = CapturedRequest {
            request_line,
            headers,
            body: body_text,
        };
        // `Content-Length` 声明的长度可能还没收全 ⇒ 再读一轮。
        if let Some(len) = request
            .header("content-length")
            .and_then(|value| value.parse::<usize>().ok())
        {
            while request.body.len() < len {
                let Ok(read) = socket.read(&mut chunk).await else {
                    break;
                };
                if read == 0 {
                    break;
                }
                request
                    .body
                    .push_str(&String::from_utf8_lossy(&chunk[..read]));
            }
        }
        if let Ok(mut guard) = sink.lock() {
            *guard = Some(request);
        }

        let response = format!(
            "{status_line}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = socket.write_all(response.as_bytes()).await;
        let _ = socket.flush().await;
    });

    (format!("http://{addr}/v1"), captured)
}

#[tokio::test]
async fn 真http往返_解析出向量且请求形状正确() {
    let (base_url, captured) = spawn_stub(
        "HTTP/1.1 200 OK",
        r#"{"data":[{"embedding":[0.25,-0.5,1.0],"index":0}],"model":"stub-embed"}"#,
    )
    .await;

    let provider =
        OpenAiCompatibleEmbeddingProvider::new(&base_url, "stub-embed", Some("k-123".into()))
            .expect("provider");
    let vector = provider.embed("你好，世界").await.expect("embed");

    assert_eq!(vector, vec![0.25, -0.5, 1.0], "向量应被正确解析");

    let request = captured.lock().expect("lock").clone().expect("抓到请求");
    assert!(
        request.request_line.starts_with("POST "),
        "应是 POST，实际：{}",
        request.request_line
    );
    assert!(
        request.request_line.contains("/embeddings"),
        "路径应含 /embeddings，实际：{}",
        request.request_line
    );
    assert_eq!(
        request.header("authorization").as_deref(),
        Some("Bearer k-123"),
        "应带 bearer 头"
    );
    assert!(
        request.body.contains("你好，世界"),
        "请求体应含输入文本，实际：{}",
        request.body
    );
    assert!(
        request.body.contains("stub-embed"),
        "请求体应含模型名，实际：{}",
        request.body
    );
}

#[tokio::test]
async fn 真http_401_归类为不可用() {
    let (base_url, _captured) = spawn_stub(
        "HTTP/1.1 401 Unauthorized",
        r#"{"error":"invalid api key"}"#,
    )
    .await;
    let provider = OpenAiCompatibleEmbeddingProvider::new(&base_url, "m", Some("bad".into()))
        .expect("provider");
    let error = provider.embed("x").await.expect_err("应失败");
    assert!(
        matches!(error, EmbeddingError::Unavailable(_)),
        "401 应归类为 Unavailable，实际：{error:?}"
    );
}

#[tokio::test]
async fn 真http_坏json_归类为非法向量() {
    let (base_url, _captured) = spawn_stub("HTTP/1.1 200 OK", "this is not json").await;
    let provider = OpenAiCompatibleEmbeddingProvider::new(&base_url, "m", None).expect("provider");
    let error = provider.embed("x").await.expect_err("应失败");
    assert!(
        matches!(error, EmbeddingError::InvalidVector(_)),
        "坏 JSON 应归类为 InvalidVector，实际：{error:?}"
    );
}

#[tokio::test]
async fn 真http_响应缺向量_归类为非法向量() {
    let (base_url, _captured) = spawn_stub("HTTP/1.1 200 OK", r#"{"data":[]}"#).await;
    let provider = OpenAiCompatibleEmbeddingProvider::new(&base_url, "m", None).expect("provider");
    let error = provider.embed("x").await.expect_err("应失败");
    assert!(
        matches!(error, EmbeddingError::InvalidVector(_)),
        "缺向量应归类为 InvalidVector，实际：{error:?}"
    );
}

#[tokio::test]
async fn 真http_端点不可达_归类为不可用() {
    // 先占一个端口再立刻释放 ⇒ 几乎必然连不上（**真连接失败**，不是 mock）。
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    drop(listener);
    let provider = OpenAiCompatibleEmbeddingProvider::new(format!("http://{addr}/v1"), "m", None)
        .expect("provider");
    let error = provider.embed("x").await.expect_err("应失败");
    assert!(
        matches!(error, EmbeddingError::Unavailable(_)),
        "不可达应归类为 Unavailable，实际：{error:?}"
    );
}
