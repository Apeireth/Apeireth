//! SSE 传输面的两处协议消费缺口：③ 无 `event:` 的帧 / ④ 服务端的 `retry:` 与 `id:`。
//!
//! # 钉住什么
//!
//! 实测记录（`docs/03-reference/mcp-transport-verification.md` §10 ③/④）登记的两条：
//!
//! **③ 无 `event:` 的帧连计数都不进**：协议库判「是不是消息」只看事件类型
//! （`event == "message"`）⇒ 一条**只有 `data:`** 的帧两个分支都不进 ⇒ 改前
//! **完全无声**（不计数、不记日志）。本文件钉住三条语义：
//!
//! 1. **可数**：它被计入 [`McpChannel::unrouted_frames`]（机器可读的信号）;
//! 2. **不断链**：它**只记不改行为** —— 帧仍被丢弃，读任务继续跑，消息端点照样收 POST;
//! 3. **不混淆**：它**不是**坏帧（[`McpChannel::malformed_frames`] 保持 0），
//!    且正常帧 / 控制帧（`retry:` / `id:`）/ 显式命名了事件类型的帧**都不进**这个计数。
//!
//! **④ 服务端的 `retry:` / `id:` 被解析但从不消费**：重连永远用本地算的退避。
//! 本文件钉住：
//!
//! 1. 传输层把 `retry:` / `id:` 帧交给观察者（[`McpFrameObserver`]）—— 不再是死字段;
//! 2. 恢复的流带 `Last-Event-ID` 请求头（`id:` 真的被用上）;
//! 3. 重连退避**真的**用了服务端给的 `retry:`（用注入的短退避做确定性断言）;
//! 4. 超上限的 `retry:` 被夹在策略上限内（服务端不能把客户端挂住一小时）。
//!
//! # 不钉住什么
//!
//! * 「裸 `data:` 帧算不算消息」是**协议行为变更**（另批）：本文件只钉住
//!   **可观测**（可数、可查）与**不断链**，不钉住「它被当成消息」。
//! * 真实第三方服务器：**未实测** —— 本文件用的是仓内自建替身（tokio 的 TCP +
//!   手写 HTTP/1.1，零新依赖）。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use apeireth_plugin::mcp::{
    JsonRpcRequest, JsonRpcResponse, ReconnectPolicy, ServerInfo, SseFrame,
};
use apeireth_tools_canonical::mcp::McpError;
use apeireth_tools_canonical::mcp_bridge::connection::bounded_server_retry;
use apeireth_tools_canonical::mcp_bridge::transport::{
    McpChannel, McpChannelFactory, McpFrameObserver, SseChannel,
};
use apeireth_tools_canonical::mcp_bridge::{
    McpBridgeOptions, McpServerConnection, McpServerSpec, McpTransportKind,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;

// ---------------------------------------------------------------- ③ 传输层

/// 一个最小的 SSE 服务器替身（**零新依赖**: tokio 的 TCP + 手写 HTTP/1.1）。
///
/// * `GET /sse` → 事件流: 先发 `event: endpoint` 帧, 之后按测试的指令转发消息帧;
/// * 其它方法（消息端点）→ `200 OK`, 并计一次 POST。
///
/// 每个连接都记下请求头 ⇒ 用例能断言恢复的流带了 `Last-Event-ID`。
struct FakeSseServer {
    port: u16,
    posts: Arc<AtomicUsize>,
    heads: Arc<Mutex<Vec<String>>>,
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
        let heads = Arc::new(Mutex::new(Vec::new()));
        let (frames, _) = broadcast::channel::<String>(32);

        let task_posts = Arc::clone(&posts);
        let task_heads = Arc::clone(&heads);
        let task_frames = frames.clone();
        tokio::spawn(async move {
            while let Ok((socket, _)) = listener.accept().await {
                let posts = Arc::clone(&task_posts);
                let heads = Arc::clone(&task_heads);
                let frames = task_frames.clone();
                tokio::spawn(async move {
                    serve_one(socket, posts, heads, frames).await;
                });
            }
        });

        Self {
            port,
            posts,
            heads,
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

    /// 已经见过的请求头。
    fn heads(&self) -> Vec<String> {
        self.heads
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// 事件流地址。
    fn stream_url(&self) -> String {
        format!("http://127.0.0.1:{}/sse", self.port)
    }
}

/// 服务一条连接（读请求头 → 按方法应答）。
async fn serve_one(
    mut socket: TcpStream,
    posts: Arc<AtomicUsize>,
    heads: Arc<Mutex<Vec<String>>>,
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
    heads
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .push(head.clone());
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

/// 等「路由不到」的计数到达 `at_least`（读任务是异步的 ⇒ 有界等待）。
async fn wait_for_unrouted_frames(channel: &Arc<dyn McpChannel>, at_least: u64) -> u64 {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let count = channel.unrouted_frames();
        if count >= at_least || tokio::time::Instant::now() >= deadline {
            return count;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

/// **主用例（③）**: 一条**只有 `data:` 没有 `event:`** 的帧 ⇒ 被计数 + 链路**不断**。
#[tokio::test]
async fn a_frame_without_an_event_type_is_counted_and_never_breaks_the_link() {
    let server = FakeSseServer::start().await;
    let channel = SseChannel::open(&server.stream_url())
        .await
        .expect("the event stream opens");

    // 对照: 一条**合法**帧（合法帧不许被算成「路由不到」）。
    server.push("event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n\n");
    // 裸 `data:` 帧（**无** `event:`）: 协议库不判它是 message ⇒ 改前静默丢弃。
    server.push("data: {\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{}}\n\n");

    let count = wait_for_unrouted_frames(&channel, 1).await;
    assert_eq!(count, 1, "无 event: 的帧必须被计数（合法帧不许混进去）");

    // 读任务还活着: 第二条裸帧照样被计数。
    server.push("data: {\"jsonrpc\":\"2.0\",\"id\":3,\"result\":{}}\n\n");
    let count = wait_for_unrouted_frames(&channel, 2).await;
    assert_eq!(count, 2, "裸帧不得让读任务退出");

    // 链路未断: 消息端点照样收 POST（通知）。
    channel
        .notify(JsonRpcRequest::notification(
            "notifications/initialized",
            None,
        ))
        .await
        .expect("裸帧不得让链路断开");
    assert_eq!(server.posts(), 1, "通知必须真的到达消息端点");

    // 它**不是**坏帧: 两个计数器各说各的（坏帧 = 进了 message 分支但 JSON 不合法）。
    assert_eq!(
        channel.malformed_frames(),
        0,
        "裸帧不许被算成坏帧（两个语义必须分得开）"
    );
}

/// **阴性对照（③）**: 正常帧 / 控制帧 / 显式类型帧**都不进**「路由不到」这个计数。
#[tokio::test]
async fn only_payload_frames_without_an_event_type_are_counted() {
    let server = FakeSseServer::start().await;
    let channel = SseChannel::open(&server.stream_url())
        .await
        .expect("the event stream opens");

    for id in 1..=3 {
        server.push(&format!(
            "event: message\ndata: {{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{{}}}}\n\n"
        ));
    }
    // 控制帧（`retry:` / `id:`，**无** data）: 协议上合法 ⇒ 不是丢掉的消息。
    server.push("retry: 250\nid: evt-9\n\n");
    // 显式命名了事件类型的帧（不是 message）: 按事件类型分派**故意**忽略。
    server.push("event: progress\ndata: {\"pct\":10}\n\n");

    // 哨兵: 一条裸帧。它被计数 ⇒ 前面的帧都已读过（流是有序的）。
    server.push("data: {\"jsonrpc\":\"2.0\",\"id\":9,\"result\":{}}\n\n");
    let count = wait_for_unrouted_frames(&channel, 1).await;
    assert_eq!(
        count, 1,
        "只有哨兵那一条裸帧算数（合法帧 / 控制帧 / 显式类型帧都不许算）"
    );
    assert_eq!(channel.malformed_frames(), 0, "它们也不是坏帧");
}

// ---------------------------------------------------------------- ④ 传输层

/// 把读到的每一帧都记下来的观察者。
#[derive(Default)]
struct RecordingObserver {
    frames: Mutex<Vec<SseFrame>>,
}

impl RecordingObserver {
    /// 已经观察到的帧。
    fn frames(&self) -> Vec<SseFrame> {
        self.frames
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

impl McpFrameObserver for RecordingObserver {
    fn observe_frame(&self, frame: &SseFrame) {
        self.frames
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(frame.clone());
    }
}

/// 等观察者看见一条 `id:` 为 `id` 的帧（流有序 ⇒ 前面的帧也已观察过）。
async fn wait_for_observed_id(observer: &Arc<RecordingObserver>, id: &str) -> Vec<SseFrame> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let frames = observer.frames();
        if frames.iter().any(|f| f.id.as_deref() == Some(id))
            || tokio::time::Instant::now() >= deadline
        {
            return frames;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

/// **用例（④ 传输层）**: 服务端的 `retry:` / `id:` 帧**真的**被交到观察者手里。
#[tokio::test]
async fn the_streams_retry_and_id_fields_reach_the_observer() {
    let server = FakeSseServer::start().await;
    let observer = Arc::new(RecordingObserver::default());
    let channel = SseChannel::open_with_observer(
        &server.stream_url(),
        Duration::from_secs(30),
        Arc::clone(&observer) as Arc<dyn McpFrameObserver>,
        None,
    )
    .await
    .expect("the event stream opens");

    // 服务端给一条流级提示帧（只有 `retry:` / `id:`，没有 data）。
    server.push("retry: 250\nid: evt-9\n\n");
    // 再给一条带 `id:` 的消息帧（哨兵: 它被观察到 ⇒ 前一帧也已观察到）。
    server
        .push("event: message\nid: evt-10\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n\n");

    let frames = wait_for_observed_id(&observer, "evt-10").await;
    assert!(
        frames
            .iter()
            .any(|f| f.retry == Some(Duration::from_millis(250))),
        "服务端的 retry: 必须被观察者看见（不再是死字段）: {frames:?}"
    );
    assert!(
        frames.iter().any(|f| f.id.as_deref() == Some("evt-9")),
        "服务端的 id: 必须被观察者看见（不再是死字段）: {frames:?}"
    );
    // 控制帧**不是**「路由不到」的帧。
    assert_eq!(
        channel.unrouted_frames(),
        0,
        "只有 retry: / id: 的控制帧不算丢掉的消息"
    );
}

/// **用例（④ 传输层）**: 恢复的流带 `Last-Event-ID` 请求头（`id:` 真的被用上）。
#[tokio::test]
async fn a_resumed_stream_asks_the_server_to_continue_from_the_last_event_id() {
    let server = FakeSseServer::start().await;
    let observer = Arc::new(RecordingObserver::default());
    SseChannel::open_with_observer(
        &server.stream_url(),
        Duration::from_secs(30),
        observer,
        Some("evt-9"),
    )
    .await
    .expect("the event stream opens");

    // 等请求头到齐（读任务是异步的 ⇒ 有界等待）。
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let heads = server.heads();
        if !heads.is_empty() || tokio::time::Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let heads = server.heads();
    assert_eq!(heads.len(), 1, "事件流必须被请求过一次");
    assert!(
        heads[0].to_lowercase().contains("last-event-id: evt-9"),
        "恢复的流必须带 Last-Event-ID 请求头: {}",
        heads[0]
    );
}

// ---------------------------------------------------------------- ④ 连接层

/// 一条按剧本行动的通道：握手与工具清单正常应答, `tools/call` 打成传输错误。
struct ScriptedChannel {
    /// 服务端一直接不上（连握手都失败）⇒ 每次恢复都以失败告终。
    dead: bool,
}

#[async_trait::async_trait]
impl McpChannel for ScriptedChannel {
    async fn request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
        if self.dead {
            return Err(McpError::Transport("the scripted link is dead".to_string()));
        }
        match request.method.as_str() {
            "initialize" => Ok(JsonRpcResponse::ok(
                request.id.clone(),
                serde_json::to_value(ServerInfo::for_server("scripted")).unwrap(),
            )),
            "tools/list" => Ok(JsonRpcResponse::ok(
                request.id.clone(),
                serde_json::json!({ "tools": [] }),
            )),
            _ => Err(McpError::Transport("the scripted link died".to_string())),
        }
    }

    async fn notify(&self, _notification: JsonRpcRequest) -> Result<(), McpError> {
        Ok(())
    }
}

/// 每次都开一条 [`ScriptedChannel`] 的工厂。
#[derive(Clone)]
struct ScriptedFactory {
    /// 服务端给的 `retry:` 提示。
    retry: Option<Duration>,
    /// 服务端给的 `id:`。
    id: Option<String>,
    /// 服务端一直接不上。
    dead: bool,
    /// 每次开链时工厂收到的 `Last-Event-ID`（恢复时才有值）。
    resumed_from: Arc<Mutex<Vec<Option<String>>>>,
}

impl ScriptedFactory {
    fn new(retry: Option<Duration>, id: Option<String>) -> Self {
        Self {
            retry,
            id,
            dead: false,
            resumed_from: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 一直接不上的服务端。
    fn dead(retry: Option<Duration>, id: Option<String>) -> Self {
        Self {
            dead: true,
            ..Self::new(retry, id)
        }
    }

    /// 每次开链收到的 `Last-Event-ID`。
    fn resumed_from(&self) -> Vec<Option<String>> {
        self.resumed_from
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

#[async_trait::async_trait]
impl McpChannelFactory for ScriptedFactory {
    async fn open(&self) -> Result<Arc<dyn McpChannel>, McpError> {
        Ok(Arc::new(ScriptedChannel { dead: self.dead }))
    }

    async fn open_with_observer(
        &self,
        observer: Arc<dyn McpFrameObserver>,
        last_event_id: Option<&str>,
    ) -> Result<Arc<dyn McpChannel>, McpError> {
        self.resumed_from
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(last_event_id.map(str::to_string));
        // 服务器在流上发了 `retry:` / `id:` ⇒ 观察者必须看见它们。
        observer.observe_frame(&SseFrame {
            id: self.id.clone(),
            retry: self.retry,
            ..SseFrame::default()
        });
        Ok(Arc::new(ScriptedChannel { dead: self.dead }))
    }
}

/// 一条 http 规格（通道由工厂给 ⇒ 地址不用真）。
fn spec() -> McpServerSpec {
    McpServerSpec::remote("scripted", McpTransportKind::Sse, "http://127.0.0.1:9/sse")
}

/// 断线预算：本地算出来的退避 **10 s**（服务端给 40 ms 时，差 250 倍 ⇒ 可判）。
fn options_with(retry_ceiling: Duration, max_attempts: u32) -> McpBridgeOptions {
    McpBridgeOptions::default().with_reconnect(ReconnectPolicy {
        initial_backoff: Duration::from_secs(10),
        max_backoff: retry_ceiling,
        max_attempts,
        ..ReconnectPolicy::default()
    })
}

/// **用例（④ 连接层）**: 服务端给的 `retry:` **真的**成了重连退避。
#[tokio::test]
async fn the_servers_retry_hint_becomes_the_reconnect_wait() {
    let factory = ScriptedFactory::new(Some(Duration::from_millis(40)), Some("evt-9".into()));
    let connection = McpServerConnection::new(
        spec(),
        Arc::new(factory.clone()),
        &options_with(Duration::from_secs(30), 3),
    );
    connection
        .connect()
        .await
        .expect("the scripted server completes the handshake");

    let started = Instant::now();
    let outcome = tokio::time::timeout(
        Duration::from_secs(5),
        connection.call_tool("lookup-item", serde_json::json!({ "key": "x" })),
    )
    .await;
    assert!(
        outcome.is_ok(),
        "重连不得等本地算出来的退避（10 s）—— 服务端给了 retry: 40ms"
    );
    let elapsed = started.elapsed();
    assert!(
        elapsed >= Duration::from_millis(40),
        "退避必须用服务端给的 retry:（不得早于它）: {elapsed:?}"
    );

    // 服务端的 `id:` 被记下来了, 恢复的流带着它。
    assert_eq!(
        connection.last_event_id().as_deref(),
        Some("evt-9"),
        "服务端的 id: 必须被记下来"
    );
    assert_eq!(
        connection.server_retry(),
        Some(Duration::from_millis(40)),
        "服务端的 retry: 必须被记下来"
    );
    assert_eq!(connection.reconnect_count(), 1, "断线恢复必须被记一次");
    let resumed = factory.resumed_from();
    assert_eq!(
        resumed.last().cloned().flatten().as_deref(),
        Some("evt-9"),
        "恢复的链必须带着服务端的 id: 续上: {resumed:?}"
    );
}

/// **用例（④ 连接层）**: 超上限的 `retry:` 被夹住（服务端不能把客户端挂住一小时）。
#[tokio::test]
async fn a_retry_hint_beyond_the_ceiling_is_clamped() {
    let factory = ScriptedFactory::new(Some(Duration::from_secs(3600)), None);
    // 策略上限 50 ms ⇒ 服务端的一小时被夹成 50 ms。
    let connection = McpServerConnection::new(
        spec(),
        Arc::new(factory.clone()),
        &options_with(Duration::from_millis(50), 3),
    );
    connection
        .connect()
        .await
        .expect("the scripted server completes the handshake");

    let started = Instant::now();
    let outcome = tokio::time::timeout(
        Duration::from_secs(5),
        connection.call_tool("lookup-item", serde_json::json!({ "key": "x" })),
    )
    .await;
    assert!(
        outcome.is_ok(),
        "服务端的一小时重连提示必须被夹在策略上限内（不得把客户端挂住）"
    );
    let elapsed = started.elapsed();
    assert!(
        elapsed >= Duration::from_millis(50),
        "夹过的退避不得早于上限: {elapsed:?}"
    );
    assert_eq!(
        connection.server_retry(),
        Some(Duration::from_millis(50)),
        "记下来的 retry: 必须已经是夹过的值"
    );
}

/// **用例（④ 连接层）**: 服务端的 `retry:` **不延长**重连预算（次数仍由策略封顶）。
#[tokio::test]
async fn the_server_retry_hint_does_not_extend_the_reconnect_budget() {
    // 服务端一直接不上 ⇒ 每次恢复都以失败告终（尝试计数不会被成功重连清零）。
    let factory = ScriptedFactory::dead(Some(Duration::from_millis(40)), None);
    let connection = McpServerConnection::new(
        spec(),
        Arc::new(factory),
        &options_with(Duration::from_secs(30), 2),
    );
    // 先开一条链: 服务端一直接不上 ⇒ 握手失败（如实报错），但链已经开过 ⇒
    // 服务端在流上给的 `retry:` 已经被观察到。
    assert!(
        connection.connect().await.is_err(),
        "一直接不上的服务端必须让握手如实失败"
    );

    // 前两次：预算之内 ⇒ 每次都恢复一次（服务端一直接不上 ⇒ 恢复以失败告终）。
    for round in 1..=2 {
        let outcome = tokio::time::timeout(
            Duration::from_secs(5),
            connection.call_tool("lookup-item", serde_json::json!({ "key": "x" })),
        )
        .await;
        assert!(
            outcome.is_ok(),
            "第 {round} 次恢复必须在预算之内完成（不得挂住）"
        );
        let inner = outcome.expect("the call must return before the timeout");
        assert!(inner.is_err(), "服务端一直接不上 ⇒ 恢复必须如实报错");
    }
    assert_eq!(connection.reconnect_count(), 2, "两次调用 ⇒ 两次恢复");

    // 第三次恢复：预算耗尽（服务端的 retry: 不能把次数预算也买通）。
    let exhausted = connection
        .call_tool("lookup-item", serde_json::json!({ "key": "x" }))
        .await
        .expect_err("预算耗尽必须报错");
    assert!(
        exhausted.to_string().contains("reconnect budget exhausted"),
        "预算耗尽必须如实报错: {exhausted}"
    );
    assert_eq!(connection.reconnect_count(), 3, "耗尽那一次也算一次恢复");
}

/// **用例（④ 纯函数）**: 上限夹取的口径（服务端可以缩短等待, 不能拉长到界外）。
#[test]
fn the_server_retry_hint_is_bounded_by_the_policy_ceiling() {
    // 无提示 ⇒ 不夹（用本地算的退避）。
    assert_eq!(
        bounded_server_retry(Duration::from_millis(250), Duration::from_secs(30)),
        Duration::from_millis(250),
        "比上限短的提示原样保留"
    );
    // 超上限 ⇒ 夹到上限。
    assert_eq!(
        bounded_server_retry(Duration::from_secs(3600), Duration::from_secs(30)),
        Duration::from_secs(30),
        "超上限的提示必须被夹住"
    );
    // 边界: 等于上限。
    assert_eq!(
        bounded_server_retry(Duration::from_secs(30), Duration::from_secs(30)),
        Duration::from_secs(30),
        "等于上限的提示原样保留"
    );
}
