//! 周期刷新的**首拍不是立即**。
//!
//! # 钉住什么
//!
//! 实测记录（`docs/03-reference/mcp-transport-verification.md` §10 ⑤）登记的
//! 口径落差: `tokio::time::interval` 的**首个 tick 立即完成** ⇒ 启动期接线
//! 刚跑完一次发现, 承载任务在 t=0 又跑一轮（实测捕捉到第二次 `tools/list`，
//! `id=3`）⇒ 那一拍**与进程退出赛跑**（短命的 CLI 进程跑不到它）; 且与
//! 「默认 60 000 ms 周期」的措辞有落差（实际首个 tick 在 t=0）。
//!
//! 本批把首拍改成**一个周期之后** ⇒ 口径与「每 N 毫秒刷新一次」一致。
//!
//! # 确定性
//!
//! * **首拍不是立即**: 用一个**测试期间不会到点**的周期（60 s）承载任务,
//!   让运行时 park 一次（有界 200 ms —— 让承载任务真正跑得动; **不是等周期
//!   到点**）, 然后断言**观察回调一次都没被调用** —— 立即首拍会在这里被抓住。
//! * **阳性对照**: 另起一个**短周期**（20 ms）的承载任务, 断言**到点后确实
//!   跑了一轮**（有界等待 ≤ 2 s）—— 证明上一条不是「回调根本没接线」的空断言。

use std::sync::Arc;
use std::time::Duration;

use apeireth_governance::AllowAll;
use apeireth_plugin::mcp::ReconnectPolicy;
use apeireth_runtime::canonical::Runtime;
use apeireth_runtime_assembly::canonical::external_tool_bridge::{
    spawn_external_tool_refresh, ExternalToolRefreshObserver,
};
use apeireth_tools_canonical::mcp_bridge::{
    InProcessMcpServer, McpBridgeOptions, McpServerConfig, McpServerConnection, McpServerSpec,
    McpToolBridge, MockToolSpec,
};
use tokio::sync::mpsc::error::TryRecvError;

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

/// 造一台**进程内**服务器上的桥（无网络、无子进程）。
fn bridge_over(server: &Arc<InProcessMcpServer>) -> Arc<McpToolBridge> {
    let spec = McpServerSpec::stdio(SERVER, "in-process-test-server");
    let options = test_options();
    let connection = Arc::new(McpServerConnection::new(
        spec.clone(),
        server.factory(),
        &options,
    ));
    let config = McpServerConfig::new(vec![spec]);
    Arc::new(
        McpToolBridge::from_connections(config, options, vec![connection])
            .expect("bridge assembles"),
    )
}

/// 造一个空运行时（治理放行, 只测承载时机, 不测派发）。
async fn runtime() -> Arc<Runtime> {
    Arc::new(
        Runtime::builder()
            .with_governance(Arc::new(AllowAll))
            .build()
            .await
            .expect("runtime builds"),
    )
}

/// **主用例**: 首拍**不是立即** —— 启动期接线刚跑完发现, 承载任务不得在
/// t=0 又跑一轮。
#[tokio::test]
async fn the_periodic_carrier_does_not_run_a_round_before_the_first_period_elapses() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("echo", "echo says hi")]);
    let bridge = bridge_over(&server);
    let runtime = runtime().await;

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let observer: ExternalToolRefreshObserver = Arc::new(move |outcome| {
        let _ = sender.send(outcome.clone());
    });

    // 周期 60 s: 测试期间**不会到点** ⇒ 任何一轮都只能是「立即首拍」造成的。
    let task = spawn_external_tool_refresh(
        Arc::clone(&runtime),
        Arc::clone(&bridge),
        None,
        Duration::from_secs(60),
        observer,
    );

    // 让承载任务**真正跑起来**: 刷新路径里有一个 0 时长的退避定时器 ⇒ 运行时
    // 必须 park 一次, 承载任务才跑得动（只 yield 不 park ⇒ 任务跑不动）。
    // 这里的 200 ms 是**有界 park**, 不是等周期到点（周期 60 s, 测试期间不到点）。
    tokio::time::sleep(Duration::from_millis(200)).await;

    let first = receiver.try_recv();
    assert!(
        matches!(first, Err(TryRecvError::Empty)),
        "首拍不得立即跑一轮（观察回调被调用了）: {first:?}"
    );
    assert!(!task.is_finished(), "承载任务不得已经结束");

    task.abort();
}

/// **阳性对照**: 到点之后承载任务**确实**跑一轮（证明上一条不是空断言）。
#[tokio::test]
async fn the_periodic_carrier_runs_a_round_once_the_period_elapses() {
    let server = InProcessMcpServer::new(SERVER);
    server.set_tools(vec![MockToolSpec::new("echo", "echo says hi")]);
    let bridge = bridge_over(&server);
    let runtime = runtime().await;

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let observer: ExternalToolRefreshObserver = Arc::new(move |outcome| {
        let _ = sender.send(outcome.clone());
    });

    // 短周期（20 ms）+ 有界等待（≤ 2 s）: 只等**到点后的那一轮**。
    let task = spawn_external_tool_refresh(
        Arc::clone(&runtime),
        Arc::clone(&bridge),
        None,
        Duration::from_millis(20),
        observer,
    );

    let outcome = tokio::time::timeout(Duration::from_secs(2), receiver.recv())
        .await
        .expect("the carrier reports a round within the bound")
        .expect("the observer reports a round");
    assert!(outcome.is_healthy(), "{outcome:?}");
    assert_eq!(outcome.registered, 1);

    task.abort();
}
