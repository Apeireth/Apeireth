//! 坏帧（`event: message` + 非法 JSON）必须**看得见**。
//!
//! # 钉住什么
//!
//! 实测记录（`docs/03-reference/mcp-transport-verification.md` §10 ②）登记的
//! 可观测性缺口: `event: message` 但 `data` 不是合法 JSON-RPC 响应时, 读帧处
//! `if let Ok(...)` **跳过** ⇒ **不记日志、不计数** ⇒ 服务器发坏帧时, 主人
//! 看不到任何迹象（客户端只是永远等不到响应）。
//!
//! 本文件用**最小自建替身**（tokio 的 TCP + 手写 HTTP/1.1, **零新依赖**）发坏帧,
//! 钉住三条语义:
//!
//! 1. **计数**: 坏帧被计入 [`McpChannel::malformed_frames`]（机器可读的信号）;
//! 2. **日志**: 坏帧在 stderr 留一条（限频: 第一条 + 之后每 100 条一条）——
//!    人能看见的信号; 跑测试时用 `--nocapture` 看得到;
//! 3. **不断链**: 坏帧**只记不改行为** —— 读任务继续跑, 消息端点照样能 POST。
//!
//! # 不钉住什么
//!
//! 「请求等不到响应」是「响应等待无截止」那条遗留的地盘（另一条并行线在改）,
//! 本文件**不**断言等待语义 —— 只断言坏帧**可见**且**不断链**。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use apeireth_plugin::mcp::JsonRpcRequest;
use apeireth_tools_canonical::mcp_bridge::{McpChannel, SseChannel};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::broadcast;

/// 一个最小的 SSE 服务器替身（**零新依赖**: tokio 的 TCP + 手写 HTTP/1.1）。
///
/// * `GET /sse` → 事件流: 先发 `event: endpoint` 帧, 之后按测试的指令转发消息帧;
/// * 其它方法（消息端点）→ `200 OK`, 并计一次 POST。
struct FakeSseServer {
    port: u16,
    posts: Arc<AtomicUsize>,
    frames: broadcast::Sender<String>,
}

impl FakeSseServer {
    /// 起一台假服务器（后台任务随测试运行时结束而结束）。
    async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("the fake server binds");
        let port = listener.local_addr().expect("the bound address").port();
        let posts = Arc::new(AtomicUsize::new(0));
        let (frames, _) = broadcast::channel::<String>(32);

        let task_posts = Arc::clone(&posts);
        let task_frames = frames.clone();
        tokio::spawn(async move {
            while let Ok((socket, _)) = listener.accept().await {
                let posts = Arc::clone(&task_posts);
                let frames = task_frames.clone();
                tokio::spawn(async move {
                    serve_one(socket, posts, frames).await;
                });
            }
        });

        Self {
            port,
            posts,
            frames,
        }
    }

    /// 往事件流推一帧（原文, 含帧分隔符）。
    fn push(&self, frame: &str) {
        self.frames
            .send(frame.to_string())
            .expect("the fake server is alive");
    }

    /// 消息端点被 POST 过几次。
    fn posts(&self) -> usize {
        self.posts.load(Ordering::SeqCst)
    }

    /// 事件流地址。
    fn stream_url(&self) -> String {
        format!("http://127.0.0.1:{}/sse", self.port)
    }
}

/// 服务一条连接（读请求头 → 按方法应答）。
async fn serve_one(
    mut socket: tokio::net::TcpStream,
    posts: Arc<AtomicUsize>,
    frames: broadcast::Sender<String>,
) {
    // 读请求头（到空行）。
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        match socket.read(&mut byte).await {
            Ok(0) | Err(_) => return,
            Ok(_) => head.push(byte[0]),
        }
    }
    let head = String::from_utf8_lossy(&head).to_string();
    let mut lines = head.lines();
    let request_line = lines.next().unwrap_or_default().to_string();
    let length = lines
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.trim().eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
        .unwrap_or(0);
    if length > 0 {
        let mut body = vec![0u8; length];
        let _ = socket.read_exact(&mut body).await;
    }

    if request_line.starts_with("GET") {
        let _ = socket
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\n\r\n",
            )
            .await;
        // 端点公告帧（客户端据此知道往哪儿 POST）。
        let _ = socket
            .write_all(b"event: endpoint\ndata: /messages\n\n")
            .await;
        let mut incoming = frames.subscribe();
        while let Ok(frame) = incoming.recv().await {
            if socket.write_all(frame.as_bytes()).await.is_err() {
                break;
            }
            let _ = socket.flush().await;
        }
    } else {
        posts.fetch_add(1, Ordering::SeqCst);
        let _ = socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
            .await;
    }
}

/// 等坏帧计数到达 `at_least`（读任务是异步的 ⇒ 有界等待）。
async fn wait_for_malformed_frames(channel: &Arc<dyn McpChannel>, at_least: u64) -> u64 {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let count = channel.malformed_frames();
        if count >= at_least || tokio::time::Instant::now() >= deadline {
            return count;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

/// **主用例**: 一条坏帧 ⇒ 计数 +1, stderr 留痕, 链路**不断**。
#[tokio::test]
async fn a_malformed_frame_is_counted_and_never_breaks_the_link() {
    let server = FakeSseServer::start().await;
    let channel = SseChannel::open(&server.stream_url())
        .await
        .expect("the event stream opens");

    // 对照: 一条**合法**帧（合法帧不许被算成坏帧）。
    server.push("event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n\n");
    // 坏帧: `event: message` 但 `data` 不是合法 JSON。
    server.push("event: message\ndata: {not json\n\n");

    let count = wait_for_malformed_frames(&channel, 1).await;
    assert_eq!(count, 1, "坏帧必须被计数（合法帧不许混进去）");

    // 读任务还活着: 第二条坏帧照样被计数。
    server.push("event: message\ndata: ]\n\n");
    let count = wait_for_malformed_frames(&channel, 2).await;
    assert_eq!(count, 2, "坏帧不得让读任务退出");

    // 链路未断: 消息端点照样收 POST（通知）。
    channel
        .notify(JsonRpcRequest::notification(
            "notifications/initialized",
            None,
        ))
        .await
        .expect("坏帧不得让链路断开");
    assert_eq!(server.posts(), 1, "通知必须真的到达消息端点");
}

/// **对照用例**: 合法帧一个都不许被算成坏帧（计数不是「收到帧数」）。
#[tokio::test]
async fn well_formed_frames_are_never_counted_as_malformed() {
    let server = FakeSseServer::start().await;
    let channel = SseChannel::open(&server.stream_url())
        .await
        .expect("the event stream opens");

    for id in 1..=3 {
        server.push(&format!(
            "event: message\ndata: {{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{{}}}}\n\n"
        ));
    }

    // 给读任务一点时间（帧是异步到的 ⇒ 用一次有界等待把「读到过」变成事实）。
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(channel.malformed_frames(), 0, "合法帧不得被算成坏帧");
}
