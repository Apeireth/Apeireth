//! `rt.mesh_local_worker` 生产接线（`mesh.json` Host 配置 → 真 UDP worker）。
//!
//! 证明的是**生产装配函数**（`desktop_host::build_desktop_main_runtime_at`）的三态
//! 语义 + 真链路：
//! ① 无 `mesh.json` ⇒ 不构造、不注册 mesh 模块 ⇒ 装配照常、**没有任何 UDP 流量**
//!    （缺省零行为变化；② 提供阳性对照，证明这份「静默」不是「机制根本不通」）；
//! ② `mesh.json` 合法 **且** System One 已绑定 ⇒ 注册 mesh 模块，后台 worker 真发
//!    UDP 包到固定本机端点（载荷 = 配置里的 `private_payload`）；
//! ③ 配置坏 / 有配置但没绑监督者 ⇒ **拒绝装配并如实报错**，不猜默认值。
//!
//! ⚠️ 反证口径（已实测）：把 `build_desktop_main_runtime_at` 里的注册臂停用
//! （`Some(_) => (builder, policy)`）⇒ ② 因 10s 内收不到包而**变红**。
#[path = "support/system_one.rs"]
mod review_support;
#[path = "support/convergence.rs"]
mod support;

use apeireth_plugin::agents::AgentBudget;
use apeireth_runtime::AgentHostPolicy;
use apeireth_runtime_assembly::canonical::desktop_host::{
    build_desktop_main_runtime_at, local_mesh_module_at, MESH_CONFIG_FILE,
};
use std::net::SocketAddr;
use std::time::Duration;
use support::{builder, Provider};

/// 生产桌面策略的最小等价物（只放开预算；消息管道由接线代码按需武装）。
fn host_policy() -> AgentHostPolicy {
    AgentHostPolicy {
        max_budget: AgentBudget {
            max_requests: u64::MAX,
            max_tokens: u64::MAX,
            max_tool_calls: u64::MAX,
            max_cost_micros: None,
            turn_timeout_ms: u64::MAX,
        },
        ..Default::default()
    }
}

/// 写一份合法 `mesh.json`（StoredDoc 信封，与设备表 / `care_rules.json` 同姿势）。
fn write_config(dir: &std::path::Path, endpoint: SocketAddr, payload: &str) {
    let doc = serde_json::json!({
        "name": "apeireth.mesh.host_config",
        "version": 1,
        "compatible_versions": [1],
        "body": {
            "peer": "peer-a",
            "endpoint": endpoint.to_string(),
            "private_payload": payload,
        }
    });
    std::fs::write(
        dir.join(MESH_CONFIG_FILE),
        serde_json::to_vec(&doc).unwrap(),
    )
    .unwrap();
}

/// 真 UDP 回显：收到一包就把原始字节回给对方（扮演「固定本机 peer」）。
struct Echo {
    address: SocketAddr,
    received: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Echo {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Echo {
    async fn start() -> Self {
        let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let address = socket.local_addr().unwrap();
        let (tx, received) = tokio::sync::mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
            let mut bytes = [0u8; 2048];
            loop {
                let (len, peer) = socket.recv_from(&mut bytes).await.unwrap();
                let _ = tx.send(bytes[..len].to_vec());
                let _ = socket.send_to(&bytes[..len], peer).await;
            }
        });
        Self {
            address,
            received,
            task,
        }
    }

    /// 等到下一包（超时 = 没有任何流量）。
    async fn next_datagram(&mut self, timeout: Duration) -> Option<Vec<u8>> {
        tokio::time::timeout(timeout, self.received.recv())
            .await
            .ok()
            .flatten()
    }
}

/// ① 无配置 ⇒ 不构造、不注册、**零 UDP 流量**；装配照常成功。
#[tokio::test]
async fn absent_host_config_registers_nothing_and_emits_no_traffic() {
    let dir = tempfile::tempdir().unwrap();
    // 注册的唯一入口是纯函数：无文件 ⇒ None ⇒ 模块根本没被构造，
    // 且与监督者绑没绑无关（缺省态不许被「顺带打开」）。
    assert!(local_mesh_module_at(dir.path(), false).unwrap().is_none());
    assert!(local_mesh_module_at(dir.path(), true).unwrap().is_none());

    // 装配照常（缺省零行为变化：入参 builder/policy 在 None 臂原样透传）。
    let mut echo = Echo::start().await;
    let runtime =
        build_desktop_main_runtime_at(builder(Provider::new(vec![])), host_policy(), dir.path())
            .await
            .expect("无 mesh.json 时装配必须照常成功");
    assert!(
        echo.next_datagram(Duration::from_secs(1)).await.is_none(),
        "无 mesh.json 时不许有任何 UDP 流量（缺省零行为变化）"
    );
    drop(runtime);
}

/// ② 合法配置 + 已绑监督者 ⇒ 真注册，后台 worker 真发 UDP（载荷 = private_payload）。
///
/// 这个用例同时是 ① 的**阳性对照**：同一套观测手段在配置存在时能收到包
/// ⇒ ⇒ ① 里的「静默」是配置缺席导致的，不是观测手段失灵。
#[tokio::test]
async fn valid_host_config_registers_the_worker_and_sends_the_private_payload() {
    let mut echo = Echo::start().await;
    let dir = tempfile::tempdir().unwrap();
    let payload = "mesh-production-host-private-payload";
    write_config(dir.path(), echo.address, payload);
    let (gate, _audit) = review_support::allow_gate();
    let runtime = build_desktop_main_runtime_at(
        builder(Provider::new(vec![])).with_system_one(gate),
        host_policy(),
        dir.path(),
    )
    .await
    .expect("合法配置必须装配成功");

    let datagram = echo
        .next_datagram(Duration::from_secs(10))
        .await
        .expect("后台 worker 必须在超时内发出 UDP 包（Protected exchange 经监督者放行）");
    assert!(
        datagram
            .windows(payload.len())
            .any(|window| window == payload.as_bytes()),
        "包体必须携带配置里的 private_payload（实际 {} 字节）",
        datagram.len()
    );
    drop(runtime);
}

/// ③-a 配置存在但坏（端点非法）⇒ 拒开并如实报错，不回落默认值。
#[tokio::test]
async fn malformed_host_config_is_refused_instead_of_defaulted() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join(MESH_CONFIG_FILE),
        serde_json::to_vec(&serde_json::json!({
            "name": "apeireth.mesh.host_config",
            "version": 1,
            "compatible_versions": [1],
            "body": {"peer": "peer-a", "endpoint": "not-an-endpoint", "private_payload": "x"}
        }))
        .unwrap(),
    )
    .unwrap();
    let error = local_mesh_module_at(dir.path(), true)
        .err()
        .expect("坏端点必须报错");
    assert!(
        error.contains("endpoint"),
        "错误必须点名坏字段，实际: {error}"
    );
    assert!(
        build_desktop_main_runtime_at(builder(Provider::new(vec![])), host_policy(), dir.path())
            .await
            .is_err(),
        "坏配置不许被当成「没配置」静默放过"
    );
}

/// ③-b 配置合法但**没绑监督者** ⇒ fail-closed（否则 worker 起来也永远被拒执行）。
#[tokio::test]
async fn host_config_without_a_bound_supervisor_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), "127.0.0.1:65000".parse().unwrap(), "payload");
    let error = local_mesh_module_at(dir.path(), false)
        .err()
        .expect("缺监督者必须报错");
    assert!(
        error.contains("APEIRETH_ENABLE_SYSTEM_ONE"),
        "错误必须给出可执行的修法，实际: {error}"
    );
    assert!(
        build_desktop_main_runtime_at(builder(Provider::new(vec![])), host_policy(), dir.path())
            .await
            .is_err(),
        "没绑监督者时不许静默注册一个永远起不来的 worker"
    );
}
