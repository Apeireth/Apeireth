# Apeireth Mesh — Phase 0 设计（分析 A–F）

> 状态：设计定稿，随 `feature/module-plugin-unification` 实现。
> 范围：最小、正确、可扩展的网络运行时骨架。**不是**把 Apeireth 做成分布式系统。

---

## A. current runtime ownership map（当前所有权地图）

以工作树当前代码为准（含未提交的 Module/Plugin 双轨重构，40 文件 +2836/−2199）：

```text
Runtime (engine/runtime/canonical/runtime.rs)
  ├── modules: ModuleManager                 ← 唯一可变入口（register/start/shutdown）
  │     ├── ModuleRegistry                   ← Arc<dyn Module> 的唯一持有者
  │     │     BTreeMap<ModuleId, ModuleEntry{ seq, module: Arc<dyn Module>, state: Lifecycle }>
  │     └── CapabilityRegistry               ← 派生索引，绝不持有实现
  │           BTreeMap<CapabilityId, ModuleId>
  ├── providers: Arc<ProviderRouter>         ← start_all 之后从 active_providers() 构建
  ├── sessions: SessionManager
  ├── governance: Arc<dyn GovernanceHook>    ← 唯一决策点（Action → Decision）
  ├── clock / config / session_locks / event_sink / context_projector
```

关键事实：

- `ModuleRegistry` 是 `Arc<dyn Module>` 的**唯一**所有者（registry.rs 文档明确 "the only copy"）。
- `CapabilityRegistry` 是 `CapabilityId → ModuleId` 的**派生索引**（"index, not a copy"）。
- `ToolView / ProviderView / HookIndex` 全部为按需派生视图（`tool_entries()`、`active_tools()`、
  `dispatchable_observers()`、`hook_index()`），无第二份事实。
- `ModuleOrigin`（System/Installed/Development）是**纯 provenance**，manifest.rs 明言
  "A future `location` (local vs remote node) is a separate axis and is deliberately not
  modelled here" —— 为 `ModuleLocation` 预留了正确位置。
- 两个已确认 foot-gun（与前一轮报告一致）：
  1. `CapabilityOwnership`（Static/Dynamic）目前是 `Module::capability_ownership()` 方法
     （module.rs:664），不是 manifest 声明；
  2. `McpModule::register_tool` 可绕过 manager 直接填内部袋（tool_modules.rs）。

## B. capability lookup/dispatch path（查找与派发路径）

```text
模型产生 ToolCall(name, arguments)
  → execute.rs::dispatch_one_tool
      → ModuleManager::tool_by_name(name)            // 模型名 → 实现
          → CapabilityRegistry::owner(id)            // 身份索引
          → ModuleRegistry::entry(owner)             // 所有权 + Lifecycle
          → module.exports().tools 里按 CapabilityId 找
          → Lifecycle::Active 检查（不可派发 → NotActive）
      → Governance(Action::CapabilityDispatch{capability, arguments})   // 治理
          → Allow / Deny（回 ToolResult 错误） / RequireApproval（冻结挂起）
      → ToolCapability::invoke(call) → ToolResult    // 实现执行
  → ToolResult 进 transcript → 下一轮
```

Provider 路径：`ModuleManager::provider(id)` → `ProviderRouter`（fallback/health 在 router，
vendor 语义在 `ProviderCapability`）。

## C. where a remote capability can be inserted without creating a second truth
（远端 capability 的唯一合法插入点）

**不能**插入的位置：

- `ModuleRegistry` —— 放入远端 `Arc<dyn Module>`/`Arc<dyn ToolCapability>` 会宣称本地拥有远端实现
  （违反 D 项期望答案 "no"）；
- `CapabilityRegistry` —— 直接登记远端 id 会把远端身份冒充为本地 owner。

**唯一插入点**：canonical 索引**旁边**的一层 **routing / projection 层**，只回答
"`CapabilityId` + 显式 route scope → 在哪里执行"：

```text
CapabilityRoute { owner: ModuleId, location: ModuleLocation }
ModuleLocation  = Local | Remote(NodeId)

Local 路线 = 从 canonical 索引派生（查询即得，不落第二份存储）
Remote 路线 = 对端 CapabilityAdvert 的只读投影（(NodeId, CapabilityId) → 元数据，零实现对象）
```

执行侧：

- Local → 现有 `ModuleManager` 派发路径（不变）；
- Remote → Mesh RPC → 远端 Node 的 **Governance → ModuleManager → Module**（canonical 路径）。

因此：

```text
logical owner  : ModuleId                          （不变，节点内唯一）
physical owner : (NodeId, ModuleId) = LocatedModuleId（网络层 owner 身份）
location       : 仅是 routing metadata，不是 identity
```

origin（System/Installed/Development/MCP…）与 location（Local/Remote）严格分轴。
advert 是**远端状态的 snapshot/projection**，永远不复制 manifest 全量数据形成第二真相。

冲突规则（禁止静默覆盖）：local 与 remote 活在**不同命名槽**——
`(RouteScope, CapabilityId)`，scope = `Local | Remote(NodeId)`。
远端 advert 发现同名 `tool.echo` 永远不触碰本地条目；调用方**显式**选择 route，
无 first-wins、无隐式优先级。

## D. crate dependency constraints（依赖约束）

现状（不变量测试 `canonical_architecture_invariants.rs` 守护）：

```text
apeireth-core        ← 零内部依赖（ids/lifecycle/clock）
apeireth-protocol    ← core（canonical 词汇表，纯翻译层，无 IO）
apeireth-governance  ← core（唯一决策点）
apeireth-plugin      ← core + protocol（Module/Capability ABI + 注册表）
apeireth-runtime     ← core + protocol + plugin + governance + orchestration
runtime-assembly / capabilities / adapters(gateway, cli, sdk)  ← 向上装配
```

Phase 0 新增（严格遵守 "foundation 不知道 socket"）：

```text
apeireth-mesh-protocol (foundation/mesh-protocol)
    ← core + protocol + serde/serde_json/thiserror     [无 tokio、无 socket]

apeireth-mesh (engine/mesh)
    ← mesh-protocol + core + protocol + plugin + governance + tokio/async-trait
    [MeshTransport trait 在此；具体 transport 实现也在此（in-memory，后续 TCP/QUIC）]

apeireth-runtime  +=  apeireth-mesh                    [engine 内向下依赖，无环]
    （canonical/mesh.rs：ModuleManagerView for Runtime，端口接线，~30 行）
```

禁止：`foundation/* → engine/*`、`apeireth-plugin → apeireth-mesh`、
`apeireth-mesh → apeireth-runtime`（集成经 `ModuleManagerView` / `MeshCapabilityExecutor` 端口）。
transport trait 不泄漏进 ModuleManager；ModuleManager 不知道网络。

## E. proposed Mesh Phase 0 architecture（架构）

```text
Machine A                                             Machine B
┌────────────────────────────┐                ┌────────────────────────────┐
│ Runtime A                  │                │ Runtime B                  │
│  ModuleManager A           │                │  ModuleManager B ◄──┐      │
│  GovernanceHook A ◄──┐     │                │  GovernanceHook B ◄─┼──┐   │
└──────────┬───────────┼─────┘                └──────────┬──────────┼──┼───┘
           │ ModuleManagerView                            │ ModuleManagerView
┌──────────▼───────────┼─────┐                ┌──────────▼──────────┼──┼───┐
│ MeshNode A           │     │                │ MeshNode B          │  │   │
│  ManagerExecutor ────┘     │                │  ManagerExecutor ───┘  │   │
│  CapabilityRouter          │                │  CapabilityRouter      │   │
│   local  routes (derived)  │                │   local routes         │   │
│   remote projections ──────┼── CapabilityAdvert ───►│   remote projections    │   │
│  Peer A-B (Active…)        │◄────── MeshConnection (in-memory / 以后 TCP) ──►│   │
│  TrustPolicy (known peers) │   Hello/HelloAck/Invoke/Result/Heartbeat/Cancel
└────────────────────────────┘                └────────────────────────────┘

远端调用闭环：
A invoke_tool(Remote(B), tool.mesh.echo, ToolCall)
  → A Governance（request-side，ActionSource::Local）
  → MeshEnvelope{ trace_id, message_id, InvokeCapability{capability, call, session, round, deadline} }
  → MeshConnection → B MeshNode
  → B Governance（execution-side，ActionSource::Remote{node: A}）
  → ModuleManager B → Module builtin.mesh-test → ToolCapability::invoke
  → CapabilityResult{ ToolResult }（in_reply_to = message_id，trace_id 原样）
  → A  oneshot 完成 → ToolResult
```

### 新 canonical 概念与最终类型名

| 概念 | 最终类型 | 位置 |
| --- | --- | --- |
| 节点身份 | `NodeId`（stable id，可 generate/持久化，与 display name 分离） | `apeireth-core::kernel` |
| 位置轴 | `ModuleLocation { Local, Remote(NodeId) }` | `apeireth-mesh-protocol` |
| 网络层 owner | `LocatedModuleId { node, module }` | `apeireth-mesh-protocol` |
| 路线 | `CapabilityRoute { owner: ModuleId, location: ModuleLocation }` | `apeireth-mesh` |
| 显式选择 | `RouteSelection { Local, Remote(NodeId) }` | `apeireth-mesh` |
| 远端投影条目 | `RemoteCapability`（元数据，零实现对象） | `apeireth-mesh` |
| 传输边界 | `trait MeshTransport` / `trait MeshConnection` | `apeireth-mesh` |
| 对端 | `Peer` + `PeerState { Connecting, Active, Unreachable, Closed }` | `apeireth-mesh` |
| 信任 | `TrustPolicy` / `KnownPeer { node, endpoint, trust: PeerTrust }` | `apeireth-mesh` |
| 执行端口 | `MeshCapabilityExecutor` + `ManagerExecutor<V: ModuleManagerView>` | `apeireth-mesh` |
| 线上信封 | `MeshEnvelope { protocol_version, message_id, in_reply_to, trace_id, source_node, target_node, payload }` | `apeireth-mesh-protocol` |

### wire payload（Phase 0 最小集）

```text
Hello / HelloAck                （node_id, protocol_version, runtime_version; 版本不兼容 fail closed）
CapabilityAdvert                （ModuleId + CapabilityId + kind + availability 的只读投影快照）
InvokeCapability / CapabilityResult / CapabilityError   （包 canonical ToolCall/ToolResult）
Heartbeat / Cancel              （Cancel{target: message_id}，best-effort）
```

不进 wire：memory sync、agent migration、distributed scheduler、session handoff、credentials、
内部模块状态、raw private metadata。

### 语义要点

- **Handshake**：`Hello`/`HelloAck` 验证 protocol version（必须相等，无 silent downgrade）、
  node identity（双向核对 known-peer + expected id）、runtime version（携带并校验非空）。
  握手后 `envelope.source_node` 必须等于握手身份，否则拒绝（防身份混淆）。
- **Trust**：`Discovered ≠ Trusted ≠ AuthorizedForEverything`。Phase 0 只有 explicit known
  peers（`TrustPolicy`）；未知/Untrusted peer 握手即拒（fail closed）。Trusted 只意味着可连接、
  可发请求；每次 invoke 仍过双边 canonical Governance。不做 discovery。
- **Governance 双边**：A request-side（`ActionSource::Local`）+ B execution-side
  （`ActionSource::Remote{node}`）；schema 扩充 `GovernanceRequest.source`，**不**建
  MeshPolicyEngine。`RequireApproval` 在远端 fail closed（不产生隐藏人工审批）。
- **Trace**：`envelope.trace_id` 全程携带（无 span 概念，保持现有最小模型）；
  `message_id`（`RequestId`）+ `in_reply_to` 做请求关联（也是 Cancel 的句柄）。
- **Cancel/Deadline**：`Cancel{target}` 置位对端 `CancelFlag`（best-effort）；
  `deadline_epoch_ms` 跨网络传播，执行前检查。
- **Heartbeat**：任何入站消息都刷新 `last_seen`；`sweep(now)` 超时 → `PeerState::Unreachable`
  → 远端 route 不可 dispatch（不删 known-peer 身份、不假装 capability 永久消失）。
  Reconnect → 重新握手 + advert → projection 恢复。
- **Advert 语义**：来自 `ModuleManager` canonical indexed view（`tool_entries()` +
  `active_providers()`）——内部袋里未经 manager 放行的工具不可见（绕过路径封死）。
  每次 advert 是**全量快照替换**（对端 projection 随之增删）。
- **本地优先是显式选择而非静默规则**：`RouteSelection` 必须显式给 scope。

### 本轮顺带的低风险修正（foot-gun #1）

`CapabilityOwnership` 迁入 `ModuleManifest` 声明（serde default = Static），
`Module::capability_ownership()` 保留为读 manifest 的派生视图，manager 以 manifest 为唯一事实。
Mesh advert 不复制第三份声明。

## F. files expected to change/add（文件清单）

新增：

```text
crates/foundation/mesh-protocol/Cargo.toml
crates/foundation/mesh-protocol/src/lib.rs          （crate 文档 + re-export）
crates/foundation/mesh-protocol/src/identity.rs     （NodeDescriptor/ModuleLocation/LocatedModuleId）
crates/foundation/mesh-protocol/src/envelope.rs     （MeshEnvelope/MeshPayload*/codec/握手校验）
crates/engine/mesh/Cargo.toml
crates/engine/mesh/src/lib.rs
crates/engine/mesh/src/error.rs                     （MeshError/MeshExecError）
crates/engine/mesh/src/transport.rs                 （MeshTransport/MeshConnection/InMemory*）
crates/engine/mesh/src/trust.rs                     （TrustPolicy/KnownPeer/PeerTrust）
crates/engine/mesh/src/route.rs                     （CapabilityRoute/RouteSelection/RemoteCapability/RouteTable）
crates/engine/mesh/src/executor.rs                  （MeshCapabilityExecutor/ManagerExecutor/ModuleManagerView）
crates/engine/mesh/src/peer.rs                      （Peer/PeerState）
crates/engine/mesh/src/node.rs                      （MeshNode：握手/advert/invoke/cancel/heartbeat）
crates/engine/mesh/tests/mesh_semantics.rs          （22 项语义测试中的 mesh 侧）
crates/engine/runtime/tests/dual_runtime_mesh.rs    （§24 双 Runtime 集成验收）
crates/engine/runtime/src/canonical/mesh.rs         （ModuleManagerView for Runtime 端口接线）
docs/design/mesh-phase-0.md                         （本文档）
```

修改：

```text
Cargo.toml                                        （workspace members +2）
crates/foundation/core/src/kernel/ids.rs          （NodeId）
crates/foundation/core/src/kernel/mod.rs          （导出 NodeId）
crates/foundation/governance/src/lib.rs           （ActionSource + GovernanceRequest.source）
crates/foundation/plugin/src/manifest.rs          （capability_ownership 声明字段）
crates/foundation/plugin/src/module.rs            （方法默认读 manifest）
crates/foundation/plugin/src/manager.rs           （以 manifest 为唯一事实；测试同步）
crates/engine/runtime-assembly/src/canonical/tool_modules.rs（McpModule 走 manifest 声明）
crates/engine/runtime/src/canonical/mod.rs        （+ pub mod mesh）
crates/engine/runtime/Cargo.toml                  （+ apeireth-mesh）
```

明确不改：`ProviderRouter`/`NormalizedRequest`/`NormalizedResponse`、execute.rs 派发主路径、
`p2p_mesh.rs`（v1 遗留愿景稿，与本轮无关）、adapters/*。
