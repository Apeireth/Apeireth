//! 外部工具（协议桥）**周期刷新**的验收用例。
//!
//! # 钉住什么
//!
//! 启动期接线只做**一次**发现：连接自愈重连会把重发现的工具列表交回**目录**，
//! 但**注册表面**（`report.registered` / `report.removed`）只在 `refresh()` 时
//! 产出 ⇒ **运行时能力表**只在启动期登记过一次：
//!
//! * 重连后服务器**新增**的工具进不了模型工具目录（模型看不见它）;
//! * 重连后服务器**删除**的工具残留在能力表里（模型仍看得见 `mcp:<server>:<tool>`，
//!   调用只得到 retryable 错误）。
//!
//! 本文件用**仓内进程内 JSON-RPC 测试替身**（`InProcessMcpServer`，无 socket、
//! 无子进程、无真实时间等待）钉住周期刷新的四条语义：
//!
//! 1. 一次刷新后新发现的工具**进**能力表（`tool_declarations()` 可见）;
//! 2. 一次刷新后消失的工具**出**能力表（撤销即帧）;
//! 3. 刷新失败（服务器不可达）⇒ **不误撤**已注册的工具;
//! 4. 重复注册同一能力 ⇒ **幂等**（不报错、不重复）。
//!
//! 另加两条把「重连 ⇒ 能力表重登」这条主缺口钉死的用例：重连回流只进目录，
//! 下一轮刷新才把它带进能力表。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use apeireth_core::kernel::CapabilityId;
use apeireth_governance::AllowAll;
use apeireth_plugin::mcp::ReconnectPolicy;
use apeireth_runtime::canonical::{CapabilityProvider, Runtime};
use apeireth_runtime_assembly::canonical::external_tool_bridge::{
    apply_external_tool_report, refresh_external_tools, RuntimeExternalToolRegistry,
};
use apeireth_tools_canonical::mcp_bridge::{
    InProcessMcpServer, McpBridgeOptions, McpChannel, McpChannelFactory, McpServerConfig,
    McpServerConnection, McpServerSpec, McpToolBridge, MockToolSpec,
};
use apeireth_tools_canonical::McpError;
use async_trait::async_trait;

const SERVER: &str = "demo";

/// 测试用的桥选项: **退避归零**（重连不许睡真实时间 —— 用例必须确定性）。
fn test_options() -> McpBridgeOptions {
    McpBridgeOptions::default()
        .with_call_timeout(5_000, 10_000)
        .with_reconnect(ReconnectPolicy {
            initial_backoff: Duration::ZERO,
            ..ReconnectPolicy::default()
        })
}

fn demo_spec() -> McpServerSpec {
    McpServerSpec::stdio(SERVER, "in-process-test-server")
}

/// 一个会在「开关打开」时拒绝开链的工厂: 用来把一台服务器打成**不可达**。
///
/// 不可达必须同时满足两件事: 既有链路读不到工具（下一请求传输失败），
/// 且重连也开不出新链路（工厂拒绝）—— 只做一件的话, 桥会自愈重连成功,
/// 服务器就仍然「可达」。
struct FlakyFactory {
    inner: Arc<dyn McpChannelFactory>,
    refuse: Arc<AtomicBool>,
}

impl FlakyFactory {
    fn new(inner: Arc<dyn McpChannelFactory>) -> Self {
        Self {
            inner,
            refuse: Arc::new(AtomicBool::new(false)),
        }
    }

    fn refuse(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.refuse)
    }
}

#[async_trait]
impl McpChannelFactory for FlakyFactory {
    async fn open(&self) -> Result<Arc<dyn McpChannel>, McpError> {
        if self.refuse.load(Ordering::SeqCst) {
            return Err(McpError::Transport(
                "test server refuses to open".to_string(),
            ));
        }
        self.inner.open().await
    }
}

/// 一台服务器（含它的工厂开关）。
struct TestServer {
    server: Arc<InProcessMcpServer>,
    factory: Arc<FlakyFactory>,
}

impl TestServer {
    fn new(name: &str) -> Self {
        let server = InProcessMcpServer::new(name);
        let factory = Arc::new(FlakyFactory::new(server.factory()));
        Self { server, factory }
    }

    fn spec(&self) -> McpServerSpec {
        McpServerSpec::stdio(SERVER, "in-process-test-server")
    }
}

/// 造一台服务器上的桥（进程内替身, 无网络）。
fn bridge_over(server: &TestServer) -> McpToolBridge {
    let spec = server.spec();
    let options = test_options();
    let connection = Arc::new(McpServerConnection::new(
        spec.clone(),
        Arc::clone(&server.factory) as Arc<dyn McpChannelFactory>,
        &options,
    ));
    let config = McpServerConfig::new(vec![spec]);
    McpToolBridge::from_connections(config, options, vec![connection]).expect("bridge assembles")
}

/// 造一个空运行时（治理放行, 只测能力表, 不测派发）。
async fn runtime() -> Runtime {
    Runtime::builder()
        .with_governance(Arc::new(AllowAll))
        .build()
        .await
        .expect("runtime builds")
}

/// 模型面看得见的外部工具名（按模型面目录读）。
fn visible_external_tools(runtime: &Runtime) -> Vec<String> {
    runtime
        .tool_declarations()
        .into_iter()
        .map(|declaration| declaration.name)
        .filter(|name| name.starts_with("mcp:"))
        .collect()
}

/// 能力表里**外部工具族**的件数（幂等断言用: 不得重复登记）。
fn external_capability_count(runtime: &Runtime) -> usize {
    runtime
        .capability_registry()
        .capabilities()
        .iter()
        .filter(|capability| capability.id().as_str().starts_with("tool.mcp."))
        .count()
}

/// **用例 1**: 一次刷新后新发现的工具**进**能力表（模型目录立刻带上它）。
#[tokio::test]
async fn a_refresh_registers_newly_discovered_tools_into_the_capability_table() {
    let server = TestServer::new(SERVER);
    server
        .server
        .set_tools(vec![MockToolSpec::new("echo", "echo says hi")]);
    let bridge = bridge_over(&server);
    let runtime = runtime().await;

    // 第一轮: 发现 + 落表 ⇒ 模型目录带上它。
    let outcome = refresh_external_tools(&runtime, &bridge, None).await;
    assert!(outcome.is_healthy(), "{outcome:?}");
    assert_eq!(outcome.registered, 1);
    assert_eq!(visible_external_tools(&runtime), ["mcp:demo:echo"]);

    // 服务器多了一件工具 ⇒ 下一轮刷新把它带进能力表。
    server.server.set_tools(vec![
        MockToolSpec::new("echo", "echo says hi"),
        MockToolSpec::new("extra", "extra says hi"),
    ]);
    let outcome = refresh_external_tools(&runtime, &bridge, None).await;
    assert!(outcome.is_healthy(), "{outcome:?}");
    assert_eq!(outcome.registered, 1);
    let mut visible = visible_external_tools(&runtime);
    visible.sort();
    assert_eq!(visible, ["mcp:demo:echo", "mcp:demo:extra"]);
}

/// **启动期接线与周期刷新共用一张能力表**: 启动期登记过的那件不会被重复登记,
/// 周期刷新只带进新增的那件。
#[tokio::test]
async fn the_startup_wiring_and_the_periodic_refresh_share_one_capability_table() {
    use apeireth_governance::PermissionPolicy;
    use apeireth_runtime_assembly::canonical::wire_external_tools;

    let server = TestServer::new(SERVER);
    server
        .server
        .set_tools(vec![MockToolSpec::new("echo", "echo says hi")]);
    let bridge = bridge_over(&server);
    let runtime = runtime().await;
    let policy = std::sync::Mutex::new(PermissionPolicy::new());

    // 启动期接线: 发现 → 撤销 → 登记 → 授权。
    let wiring = wire_external_tools(&runtime, &bridge, None, &policy)
        .await
        .expect("startup wiring");
    assert_eq!(wiring.registered, 1);
    assert_eq!(visible_external_tools(&runtime), ["mcp:demo:echo"]);

    // 服务器多了一件工具 ⇒ 周期刷新只登记新增的那一件, 旧的不重复登记。
    server.server.set_tools(vec![
        MockToolSpec::new("echo", "echo says hi"),
        MockToolSpec::new("extra", "extra says hi"),
    ]);
    let outcome = refresh_external_tools(&runtime, &bridge, None).await;
    assert!(outcome.is_healthy(), "{outcome:?}");
    assert_eq!(outcome.registered, 1);
    assert_eq!(external_capability_count(&runtime), 2, "不得重复登记");
    let mut visible = visible_external_tools(&runtime);
    visible.sort();
    assert_eq!(visible, ["mcp:demo:echo", "mcp:demo:extra"]);
}

/// **用例 2**: 一次刷新后消失的工具**出**能力表（撤销即帧）。
#[tokio::test]
async fn a_refresh_withdraws_tools_that_the_server_dropped() {
    let server = TestServer::new(SERVER);
    server.server.set_tools(vec![
        MockToolSpec::new("echo", "echo says hi"),
        MockToolSpec::new("extra", "extra says hi"),
    ]);
    let bridge = bridge_over(&server);
    let runtime = runtime().await;

    let outcome = refresh_external_tools(&runtime, &bridge, None).await;
    assert!(outcome.is_healthy(), "{outcome:?}");
    assert_eq!(outcome.registered, 2);
    let mut visible = visible_external_tools(&runtime);
    visible.sort();
    assert_eq!(visible, ["mcp:demo:echo", "mcp:demo:extra"]);

    // 服务器撤掉一件工具 ⇒ 下一轮刷新把它从能力表里撤掉。
    server
        .server
        .set_tools(vec![MockToolSpec::new("echo", "echo says hi")]);
    let outcome = refresh_external_tools(&runtime, &bridge, None).await;
    assert!(outcome.is_healthy(), "{outcome:?}");
    assert_eq!(outcome.unregistered, 1);
    assert_eq!(visible_external_tools(&runtime), ["mcp:demo:echo"]);
}

/// **用例 3**: 刷新失败（服务器不可达）⇒ **不误撤**已注册的工具。
///
/// 不可达不是「工具没了」的证据: 撤掉它们是误伤（模型会突然看不见一件还能
/// 用的工具）。其余服务器的增量照常落地。
#[tokio::test]
async fn a_failing_server_does_not_withdraw_already_registered_tools() {
    let server = TestServer::new(SERVER);
    server
        .server
        .set_tools(vec![MockToolSpec::new("echo", "echo says hi")]);
    let bridge = bridge_over(&server);
    let runtime = runtime().await;

    let outcome = refresh_external_tools(&runtime, &bridge, None).await;
    assert!(outcome.is_healthy(), "{outcome:?}");
    assert_eq!(visible_external_tools(&runtime), ["mcp:demo:echo"]);

    // 把这台服务器打成不可达: 既有链路的下一请求传输失败, 且重连开不出新链路。
    server.server.drop_next_requests(1);
    server.factory.refuse().store(true, Ordering::SeqCst);

    let outcome = refresh_external_tools(&runtime, &bridge, None).await;
    assert!(!outcome.is_healthy(), "失败必须可观测: {outcome:?}");
    assert_eq!(outcome.unregistered, 0, "不可达 ⇒ 不许撤工具");
    assert_eq!(
        visible_external_tools(&runtime),
        ["mcp:demo:echo"],
        "不可达的服务器的工具必须留在能力表里"
    );
}

/// **用例 4**: 重复注册同一能力 ⇒ **幂等**（不报错、不重复）。
#[tokio::test]
async fn registering_the_same_capability_twice_is_idempotent() {
    let server = TestServer::new(SERVER);
    server
        .server
        .set_tools(vec![MockToolSpec::new("echo", "echo says hi")]);
    let bridge = bridge_over(&server);
    let runtime = runtime().await;

    let report = bridge.refresh().await.expect("discovery");
    let registry = RuntimeExternalToolRegistry::new(&runtime);
    registry
        .register(report.registered[0].clone())
        .expect("first registration");
    // 同一件能力再登一次: 撞 id ⇒ 幂等成功, 不是错。
    registry
        .register(report.registered[0].clone())
        .expect("second registration must be idempotent, not an error");

    assert_eq!(external_capability_count(&runtime), 1, "不得重复登记");
    assert_eq!(visible_external_tools(&runtime), ["mcp:demo:echo"]);

    // 同一份报告再落一次（周期刷新与启动期接线撞车）: 同样幂等。
    let outcome = apply_external_tool_report(&runtime, &report, None);
    assert!(outcome.failures.is_empty(), "{outcome:?}");
    assert_eq!(external_capability_count(&runtime), 1);
}

/// **主缺口用例**: 连接自愈重连把重发现的工具列表交回**目录**,
/// 但**注册表面**要等下一轮刷新才跟上 ⇒ 重连后新增的工具进得了能力表。
#[tokio::test]
async fn a_reconnect_adopted_tool_reaches_the_capability_table_on_the_next_refresh() {
    let server = TestServer::new(SERVER);
    server
        .server
        .set_tools(vec![MockToolSpec::new("echo", "echo says hi")]);
    let bridge = bridge_over(&server);
    let runtime = runtime().await;

    let outcome = refresh_external_tools(&runtime, &bridge, None).await;
    assert!(outcome.is_healthy(), "{outcome:?}");
    assert_eq!(visible_external_tools(&runtime), ["mcp:demo:echo"]);

    // 服务器在重连后多了一件工具。
    server.server.set_tools(vec![
        MockToolSpec::new("echo", "echo says hi"),
        MockToolSpec::new("extra", "extra says hi"),
    ]);

    // 触发一次自愈重连: 既有链路下一请求传输失败 ⇒ 桥重连并重发现工具列表,
    // 重发现的列表交回**目录**（目录面当场跟上）。
    server.server.drop_next_requests(1);
    let connection = bridge.connections()[0].clone();
    connection
        .call_tool("echo", serde_json::json!({}))
        .await
        .expect("the call recovers through a reconnect");
    assert!(
        bridge.catalog().get("mcp:demo:extra").is_some(),
        "重连后目录面当场跟上"
    );

    // **注册面还没跟上**: 能力表里仍然只有旧的那一件 ⇒ 模型看不见新工具。
    assert_eq!(
        visible_external_tools(&runtime),
        ["mcp:demo:echo"],
        "重连回流只进目录, 不进能力表"
    );

    // 下一轮周期刷新把目录面的增量带进能力表 ⇒ 模型看得见新工具。
    let outcome = refresh_external_tools(&runtime, &bridge, None).await;
    assert!(outcome.is_healthy(), "{outcome:?}");
    assert_eq!(outcome.registered, 1);
    let mut visible = visible_external_tools(&runtime);
    visible.sort();
    assert_eq!(visible, ["mcp:demo:echo", "mcp:demo:extra"]);
}

/// **重连后被撤掉的工具**同样要在下一轮刷新里**出**能力表
/// （否则模型仍看得见一件服务器已经撤掉的工具, 调用只得到 retryable 错误）。
#[tokio::test]
async fn a_reconnect_that_drops_a_tool_withdraws_it_on_the_next_refresh() {
    let server = TestServer::new(SERVER);
    server.server.set_tools(vec![
        MockToolSpec::new("echo", "echo says hi"),
        MockToolSpec::new("extra", "extra says hi"),
    ]);
    let bridge = bridge_over(&server);
    let runtime = runtime().await;

    let outcome = refresh_external_tools(&runtime, &bridge, None).await;
    assert!(outcome.is_healthy(), "{outcome:?}");
    assert_eq!(external_capability_count(&runtime), 2);

    // 重连后服务器撤掉了 `extra`。
    server
        .server
        .set_tools(vec![MockToolSpec::new("echo", "echo says hi")]);
    server.server.drop_next_requests(1);
    let connection = bridge.connections()[0].clone();
    connection
        .call_tool("echo", serde_json::json!({}))
        .await
        .expect("the call recovers through a reconnect");
    assert!(
        bridge.catalog().get("mcp:demo:extra").is_none(),
        "重连后目录面当场跟上（工具没了）"
    );
    assert_eq!(external_capability_count(&runtime), 2, "注册面还没跟上");

    let outcome = refresh_external_tools(&runtime, &bridge, None).await;
    assert!(outcome.is_healthy(), "{outcome:?}");
    assert_eq!(outcome.unregistered, 1);
    assert_eq!(visible_external_tools(&runtime), ["mcp:demo:echo"]);
}

/// **周期承载者**: 后台任务按间隔跑刷新, 每一轮的结果都交给观察回调
/// （生产面据此记日志 ⇒ 可观测; 测试面据此等一轮 ⇒ 不靠真实时间）。
#[tokio::test]
async fn the_periodic_carrier_runs_a_round_and_reports_it() {
    use apeireth_runtime_assembly::canonical::external_tool_bridge::spawn_external_tool_refresh;

    let server = TestServer::new(SERVER);
    server
        .server
        .set_tools(vec![MockToolSpec::new("echo", "echo says hi")]);
    let bridge = Arc::new(bridge_over(&server));
    let runtime = Arc::new(runtime().await);

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let observer: apeireth_runtime_assembly::canonical::external_tool_bridge::ExternalToolRefreshObserver =
        Arc::new(move |outcome| {
            let _ = sender.send(outcome.clone());
        });
    let task = spawn_external_tool_refresh(
        Arc::clone(&runtime),
        Arc::clone(&bridge),
        None,
        std::time::Duration::from_millis(50),
        observer,
    );

    // 第一轮在一个周期之后触发 ⇒ 承载任务必须先 park 一次（真实时间轴，周期为毫秒级）。
    let outcome = receiver.recv().await.expect("the carrier reports a round");
    assert!(outcome.is_healthy(), "{outcome:?}");
    assert_eq!(outcome.registered, 1);
    assert_eq!(visible_external_tools(&runtime), ["mcp:demo:echo"]);

    task.abort();
}

/// **能力 id 口径**: 外部工具的动态能力 id 落在族根之下（登记与撤销都以它为准）。
#[test]
fn the_external_capability_identity_is_the_family_scoped_one() {
    let id = CapabilityId::new("tool.mcp.demo.echo").expect("valid capability id");
    assert!(apeireth_runtime_assembly::canonical::is_external_tool_capability(&id));
}

/// **模块袋同源**: 周期刷新落表时, 模块袋（归属视图）跟着进跟着出,
/// 且**幂等**（同一件工具重复登记不是错）。
#[tokio::test]
async fn the_module_bag_follows_the_same_delta_idempotently() {
    use apeireth_runtime_assembly::canonical::McpModule;

    let server = TestServer::new(SERVER);
    server
        .server
        .set_tools(vec![MockToolSpec::new("echo", "echo says hi")]);
    let bridge = bridge_over(&server);
    let runtime = runtime().await;
    let bag = McpModule::new();

    let outcome = refresh_external_tools(&runtime, &bridge, Some(&bag)).await;
    assert!(outcome.is_healthy(), "{outcome:?}");
    assert_eq!(outcome.registered, 1);
    assert_eq!(bag.capabilities().len(), 1, "模块袋跟着进");

    // 同一轮再落一次: 模块袋与能力表都**不得**重复。
    let report = bridge.refresh().await.expect("discovery");
    let outcome = apply_external_tool_report(&runtime, &report, Some(&bag));
    assert!(outcome.failures.is_empty(), "{outcome:?}");
    assert_eq!(bag.capabilities().len(), 1, "模块袋不得重复登记");

    // 服务器撤掉工具 ⇒ 模块袋跟着出。
    server.server.set_tools(Vec::new());
    let outcome = refresh_external_tools(&runtime, &bridge, Some(&bag)).await;
    assert!(outcome.is_healthy(), "{outcome:?}");
    assert_eq!(outcome.unregistered, 1);
    assert!(bag.capabilities().is_empty(), "模块袋跟着出");
}
