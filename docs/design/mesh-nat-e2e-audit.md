# Mesh NAT E2E — 定向审计与 Gap Matrix

> 任务 MESH-NAT-E2E-1 第一阶段产物。审计对象：`replay/mesh-on-rc2` @ `938921f43`（基线，bundle 备份于
> `apeireth-mesh-nat-e2e-backups/baseline-938921f43-allrefs.bundle`）。
> 方法：沿任务给定的链路逐步读代码（非假设），每格回答"存在 / 接入正式路径 / 验证 / 缺失"。

## Gap Matrix

| Stage | Existing | Integrated | Verified | Missing |
|---|---|---|---|---|
| Local node identity | ✅ `NodeIdentity`（Ed25519，受限文件、原子创建） | ✅ CLI pair / TrustStore | ✅ identity 测试 | — |
| TrustStore / peer admission | ✅ `TrustStore` / `TrustPolicy` / `admit_peer` | ✅ | ✅ | — |
| Rendezvous presence | ✅ `PresenceRecord` 签名、`RelayEndpoint` register/lookup | ✅（wan_integration） | ✅ | — |
| VerifiedPresence | ✅ 签名验证一次、类型携带 | ✅ | ✅ | — |
| PresenceStore | ✅ 类型 + 三迁移 + 两拒绝（`0274acca1`） | ❌ **无生产持有者**：没有任何组件把 rendezvous lookup 结果灌入它 | 仅单元测试 | MeshNode 不持有；无生产调用方 |
| Endpoint candidate selection | ✅ `dial_plan()`（只收 `VerifiedPresence`） | ❌ 正式 connect 不调用 | 仅 `candidate_dialing.rs` | — |
| PunchCoordinator | ✅ 完整协调状态（canonical pair、incarnation 绑定、有界） | ❌ **正式连接流程不驱动**：`node.rs` 无任何引用；`request()` 只有测试调用 | 单元 + loopback（`automatic_punch.rs`） | 无 `MeshNode` 入口；无生产调用方 |
| Simultaneous UDP punch | ✅ `punch()` driver（dial+accept 同窗、共享收敛规则） | ❌ 同上 | 仅 loopback | — |
| QUIC socket 复用 | ✅ `PreparedMeshEndpoint`→`QuicPeerEndpoint`（单 socket 双角色，消费即交接） | ⚠️ 生产 dial 用 `QuicMeshTransport::new`（**重新 bind、仅拨号**） | `bound_endpoint`/`peer_endpoint` 测试 | 生产 connect 从不用 prepared endpoint；observe 的 socket 与拨号 socket 不是同一个 |
| QUIC handshake | ✅ pinned mutual TLS | ✅ | ✅ | — |
| Mutual authentication | ✅ TLS 密钥 == 身份密钥 + pin | ✅ | ✅ | — |
| Connection convergence | ✅ `keeps_connection` / `new_connection_survives` | ✅ `activate_peer` 复用 | ✅ quic_loopback dup 测试 | — |
| MeshNode activation | ✅ `activate_peer`（含 placeholder 不驱逐 live） | ✅ | ✅ | — |
| Canonical runtime | ✅ `ManagerExecutor` + `PluginManagerView` 端口 | ✅ | ✅ `dual_runtime_mesh` | — |
| Remote tool invocation | ✅ `invoke_tool` / `start_call` | ✅ | ✅ | — |
| Relay fallback | ✅ `RelayMeshTransport` + `PeerConnector::direct_then_relay` | ❌ 无生产调用方组装 direct+relay；relay 与 presence/punch 不连通 | wan 集成测试（relay-only） | punch 失败 → relay 的回退链不存在 |

## 关键结论（逐条回答审计提问）

1. **已接入生产路径的**：身份、信任、rendezvous presence、QUIC 握手、双向认证、连接收敛、MeshNode 激活、canonical runtime、远端调用。
2. **仍是孤立类型/测试实现的**：`PresenceStore`、`dial_plan`、`PunchCoordinator`、`punch()`、`PreparedMeshEndpoint`/`QuicPeerEndpoint`、`PeerConnector`——全部只有测试消费。
3. **PunchCoordinator 是否由正式连接流程驱动：否。** `punch.rs` 与 `node.rs` 无引用关系；生产拨号只有两条：`maintain_connection`（单 endpoint 字符串）与 CLI `pair`（直连）。
4. **两端如何协调 punch 时间与目的地址：目前没有跨机协调通道。** `wait_for_session` 只在本机 coordinator 上等待；`automatic_punch.rs` 两个 peer 共享同一个 coordinator（同进程）。跨机的正确语义只能是"双方各自独立 request + 各自双角色端点同时拨/等"，窗口与候选来自各自持有的对方 verified presence。
5. **QUIC 如何复用 observe/punch 的 socket：类型上已保证（`PreparedMeshEndpoint` 单 Owner，`into_quic_peer_endpoint` 消费 self），但生产路径压根不走这条**——`QuicMeshTransport::new` 每次新建 bind。
6. **QUIC 与 punch 数据包如何区分：不需要区分**——punch 的"发包"就是 QUIC Initial（quinn 拥有 socket 后所有数据报都归它）；`punch()` 的 accept 半边即 `QuicPeerEndpoint::accept`。这是现有设计的正确解法，审计确认无需新协议字段。
7. **并发收敛是否可能误关闭唯一可用连接：不会。** `activate_peer` 中 dead incumbent 才让位；live incumbent 只有 initiator 更小才被替换；punch 自身两半收敛时"只有一半成功则无条件保留"。
8. **direct 失败后是否存在 relay fallback：机制在（`RelayMeshTransport`），装配不在。** 没有任何生产调用方把 punch/relay 编成一条链。
9. **连接激活后 canonical runtime 能否正常发起请求：能**（`dual_runtime_mesh` / `transparent_tool_routing` 已证明），但那条路没有一段是经过 punch 建立的。

## 因此本任务要闭合的缺口（实现顺序）

- G1：`MeshNode` 持有 `PunchCoordinator` + `PresenceStore`，并提供生产入口 `connect_punched()`（presence → request → punch → 既有 `connect()` 握手 → 既有收敛）。
- G2：`PresenceStore` 的 `Replaced` / 吊销 → `PunchCoordinator::invalidate_incarnation/peer`（过期 presence 不污染后续连接）。
- G3：punch 失败返回可区分的错误（`MeshError::PunchFailed`），使调用方能进入 relay fallback；relay 侧不新建信任路径。
- G4：harness 穿透半边（`nat_probe` punch 模式 + `test-mesh-nat.sh` TEST B/C/D），两端真实 UDP、真实 NAT、真实 Mesh 调用。
- G5：loopback 全链路集成测试（Windows 可执行）证明 G1–G3 在正式路径上成立。

## 环境事实

- 本机 Windows，无 root/CAP_NET_ADMIN：**Linux namespace NAT 测试不可在本机执行**。harness 可实现、可审查、附可运行命令，但不得声称已运行。
