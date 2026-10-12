# MESH-NAT-E2E-1 — 最终报告

> **状态分类：experimental NAT traversal（best-effort direct connectivity），未在真实 NAT 下验证。**
> 本文是 2026-10-11 的时点记录；当前权威状态以 `mesh-architecture.md`
> 顶部的实验性横幅与 "Experimental: NAT traversal (capability boundary)"
> 章节为准。
>
> 任务：以真实 NAT 穿透验证为核心的系统集成与工程验收。
> 日期：2026-10-11 · 分支 `replay/mesh-on-rc2` · 基线 `938921f43`（rc2）
> 配套审计：`mesh-nat-e2e-audit.md`（Gap Matrix 与本报告的差距已按此推进）。

## A. Implementation Report

### 原先真正缺失的调用链

审计结论（详见审计文档）：机制全部存在，但**从 VerifiedPresence 到已认证 QUIC 的链条在生产路径上不可行走**——

1. `PunchCoordinator` / `punch()` 只有测试消费，`node.rs` 零引用；
2. `PresenceStore` 无生产持有者，没有任何组件把 rendezvous lookup 结果灌入；
3. `dial_plan()`、`PreparedMeshEndpoint`/`QuicPeerEndpoint` 同样只有测试用；
4. 生产拨号（`maintain_connection` / CLI pair）每次新建 dial-only socket，observe 的 socket 与拨号 socket 不是同一个；
5. punch 失败与 relay fallback 之间没有可区分的信号。

### 修改的模块与理由

| 模块 | 修改 | 理由 |
|---|---|---|
| `mesh/src/node.rs` | `MeshNode` 新增 `punch: Arc<PunchCoordinator>` 与 `presence: Mutex<PresenceStore>` 两个字段及访问器；新增 `note_presence` / `peer_presence` / `punch_coordinator`；新增生产入口 `connect_punched()`；`revoke_peer` 级联清理 presence 与 punch；`sweep_heartbeats` 修剪过期 presence  lease 及其绑定尝试 | 闭合 G1/G2：让正式连接流程持有并驱动协调状态，presence 替换/吊销/过期能失效派生状态 |
| `mesh/src/error.rs` | 新增 `MeshError::PunchFailed { peer, reason }` | 闭合 G3：直连尝试耗尽必须是可区分的类型化信号（→ relay fallback），不是笼统 transport 错误 |
| `mesh-quic/examples/nat_probe.rs` | 新增 `punch` 模式：observe → 签名 srflx candidate → presence 文件交换（原子写 + fail-closed 验证）→ `connect_punched` → 远端 `tool.mesh.echo` 调用并校验结果内容 | harness 穿透半边的驱动二进制，走生产入口，无测试专用捷径 |
| `scripts/test-mesh-nat.sh` | TEST A（观察，原有）+ TEST B（simultaneous open）+ TEST C（收敛，LINK_COUNT=1）+ TEST D（canonical 远端调用，结果内容校验、执行次数=1）+ TEST E（敌对入向过滤下的有界失败） | 穿透半边落地；失败有界；脚本标注未执行 |
| `docs/design/mesh-architecture.md` | "Not covered" 中 NAT 条目按事实改写 | 实事求是：机制已在生产路径上跑通，但没有任何 NAT 被穿透过 |
| `mesh-quic/tests/punched_connect.rs` | 新增 8 个 loopback 集成测试（含主链路 + 5 个 fail-closed/有界性测试） | Windows 可执行的路径证据 |

### 直接复用的已有实现

`NodeIdentity`/`TrustStore`/`TrustPolicy`、`VerifiedPresence`、`PresenceStore`、`PunchCoordinator`、`punch()`、`dial_plan`、`PreparedMeshEndpoint`→`QuicPeerEndpoint`（单 socket 双角色）、`QuicPeerEndpoint` 的 pinned mutual TLS、`MeshNode::connect`/`accept` 握手、`activate_peer` 收敛（`new_connection_survives`）、`ManagerExecutor`  canonical 端口、`PeerConnector`/`RelayMeshTransport`（relay 侧未改动）。

### 协议 / 认证 / socket 所有权

- **协议零变更**：没有新增线上字段，没有新消息类型。punch 的"发包"即 QUIC Initial，不需要与 QUIC 数据报区分。
- **认证零放宽**：punched link 与任何连接走同一 pinned 握手；未 pin 的对端在 TLS 层即被拒；trust gate 在第一个数据包之前。
- **socket 所有权**：由类型保证——`connect_punched<E: MeshTransport + MeshListener>` 的泛型约束使"两个互不协调的 socket"不可表达；调用方把同一个 `QuicPeerEndpoint` 同时作 dialer/listener；quinn 在 handoff 后拥有全部数据报；`PreparedMeshEndpoint` 消费即交接的设计原样保留。

### 新增测试与修复的 bug

- 8 个新集成测试全部通过（见 B）。
- 集成时发现并修复的真实缺陷：`punch()` 产出的 inbound 半边若直接送 `connect()` 会以拨号方身份在一条"自己是接受方"的链路上发 Hello——现已按 `kept_direction` 分派 `connect`/`accept`。
- 已知限制（非安全，可用性）：punch 的 accept 半边可能接到**另一个已 pin 对端**的连接，收敛规则若保留它会导致握手失败（fail closed，调用方重试或回退 relay）。harness 拓扑无此场景；生产多对端场景见 Remaining Gaps D4。

## B. Verification Matrix

| Test | Environment | Actual path | Result | Evidence |
|---|---|---|---|---|
| punch 主链路（presence→punch→认证→收敛→远端调用） | Windows loopback，真实 QUIC socket，两个 MeshNode + ManagerExecutor | `connect_punched` 生产入口；结果内容逐字节校验；执行侧计数=1 | **PASS** | `punched_connect.rs::two_punched_peers_authenticate_and_run_one_remote_call` |
| 未准入对端在发包前被拒 | 同上 | trust gate 先于任何数据包；协调状态零残留 | PASS | `an_unadmitted_peer_is_refused_before_any_punch` |
| 他者 presence 不能拨目标 peer | 同上 | presence.node_id ≠ peer → PunchFailed | PASS | `a_presence_describing_another_node_cannot_dial_the_target` |
| 验证后过期的 lease 被拒 | 同上 | expires_at ≤ now → PunchFailed | PASS | `a_presence_that_expired_between_verifying_and_dialing_is_refused` |
| 重放 lease 被拒（不缩短已存 lease） | 同上 | PresenceStore::StaleLease | PASS | `a_replayed_lease_is_refused_rather_than_shortening_the_stored_one` |
| 新 incarnation 失效旧 punch 绑定 | 同上 | Replaced → invalidate_incarnation | PASS | `a_replaced_incarnation_invalidates_the_punch_bound_to_the_old_one` |
| 吊销对端清 presence + punch | 同上 | revoke_peer 级联 | PASS | `revoking_a_peer_drops_its_presence_and_its_punch_attempts` |
| 无应答对端 → 有界失败、无残留 | 同上 | PunchFailed < 5s（窗口 1s）；attempt 不滞留 | PASS | `a_punch_with_no_answering_peer_fails_inside_its_window_and_leaves_nothing` |
| harness 二进制冒烟（observer + 双 peer punch + E2E） | Windows loopback，两进程 | `nat_probe punch`：PUNCH_RESULT=ok、LINK_COUNT=1、PEER_STATE=Active、E2E_RENDER 内容正确、B 侧 EXEC_COUNT=1，双进程 exit 0 | **PASS（工具链验证，非 NAT）** | 本机运行日志 |
| mesh 四件套回归（mesh/protocol/quic/rendezvous） | Windows | 全部 suite | **PASS** 0 失败 | 本机运行 |
| 工作区全量回归 | Windows，`CARGO_TARGET_DIR=E:\apeireth-mesh-verify-target; cargo test --workspace --no-fail-fast -j 4` | 178 个 result 行 | **4935 passed / 2 failed / 21 ignored**；2 失败均为基线旧账（core `test_release_version_is_2_0_0_rc_1`、tools-canonical `apply_with_gate_...`），失败现象与基线一致，**无新增失败** | `workspace-regression.log` |
| **TEST A 观察** | **Linux namespace + nft masquerade** | **未执行** | **NOT RUN** | 本机 Windows，无可用 Linux（WSL 仅 docker-desktop：无 nft/cargo，BusyBox ip） |
| **TEST B/C/D 穿透 + 收敛 + 远端调用** | 同上 | **未执行** | **NOT RUN** | 同上；复现命令：`sudo scripts/test-mesh-nat.sh` |
| **TEST E 有界失败** | 同上 | **未执行** | **NOT RUN** | 同上 |
| 真实 NAT 设备 | — | 未执行 | NOT RUN | 需要真实网络环境 |

**未执行即未通过。** Linux harness 已实现、bash 语法校验通过、行尾为 LF、可执行位已修正，但本环境不具备 root/CAP_NET_ADMIN 与 Linux，**不得**将 loopback 结果外推为 NAT 穿透证据。

## C. Git Report

- 原始 HEAD：`938921f43af5f01b7f8864620d56049fdc188025`（`replay/mesh-on-rc2`）
- 当前 HEAD：`b4e27ef79347ecf01618c9e12400058e6d5d1320`
- 本地备份：`H:\项目\CrossPlatform\Apeireth\apeireth-mesh-nat-e2e-backups\baseline-938921f43-allrefs.bundle`（134.4 MB，41 refs 含 HEAD）。`git bundle verify` 通过；clone 恢复测试通过（恢复出的 HEAD 与基线一致），测试副本已删除。
- 新增提交（3 个，均在 `replay/mesh-on-rc2`，未 push、未 rebase、未 reset、未动 origin/main）：
  1. `ada5473b4` mesh: drive the punch path from the production connection flow
  2. `723307afd` mesh(nat): run the traversal half of the namespace harness
  3. `b4e27ef79` chore(mesh): make the NAT harness script executable
- 工作树状态：干净（`git status --porcelain` 为空）。
- 分支相对 origin/main：ahead 37（基线时 ahead 34），behind 0。远端当前不可达（schannel 凭证错误），未验证远端是否已有同名分支。
- 另注：`main` 分支上有一个未提交历史遗留 stash（`wip: v6 UI shell before sync origin/main`），与本任务无关，未触碰。

## D. Remaining Gaps（按阻塞程度排序）

1. **Linux NAT E2E 未执行（阻塞"L2"认定）。** 原因：本机 Windows；WSL 仅有 docker-desktop 发行版（无 nft/iptables/cargo，BusyBox ip），不具备条件且不应改造用户的 Docker 环境。证据：本报告 B 表。是否阻塞使用：阻塞"声称支持 NAT 穿透"，不阻塞代码合入。下一步最小解：`sudo scripts/test-mesh-nat.sh`（脚本已就绪，含 TEST A–E 与明确断言）。
2. **真实 NAT 未测（阻塞"L3"）。** nft masquerade 是 endpoint-independent mapping；address/port-dependent mapping/filtering 未模拟，不得冒充。下一步：在家庭路由器/移动网络后的两台机器间运行同一 harness（observer 需公网可达或改为双方互观）。
3. **无生产宿主装配 presence→punch 调用方。** `connect_punched` 的第一个调用方是 harness；gateway/CLI 尚无"发现 presence 即拨号"的装配（生产拨号仍只有 `maintain_connection` 静态 endpoint 与 CLI pair）。证据：全仓 grep，`connect_punched` 调用方仅测试与 example。阻塞程度：中（机制就绪，缺接线）。下一步：在 gateway mesh 装配处加 presence 源（rendezvous lookup → `note_presence`）与 `connect_punched` 调用，PunchFailed → relay。
4. **多对端节点上 punch accept 半边可能接到非目标对端。** 若收敛规则保留了这条"陌生人"链，握手失败（fail closed，可用性损失）。harness 无双对端拓扑未暴露。下一步最小解：`punch()` 的 inbound 半边加"对端身份预检"（accept 后先做 Hello 前的 lightweight 校验，或将非目标连接交还 serve 循环）。
5. **relay fallback 未与 punch 组成单一生产函数。** 决策点（PunchFailed 信号、`PeerConnector::direct_then_relay`、relay 路径本身）均已实现且分别有测试；但没有一个生产函数走完整状态机，harness 也未跑 relay（拓扑无 relay server）。证据：wan_integration 覆盖 relay-only 路径。下一步：随 D3 的装配一并完成（punch 失败 → RelayMeshTransport.dial → `node.connect`）。
6. **两个基线失败测试未修**（core rc.1 断言、tools-canonical apply_patch）——基线旧账，与本任务无关，按任务要求仅确认"仍为相同测试、现象一致"。

## E. Final Verdict

**NAT traversal implemented but not verified.**

- 实现：从 `VerifiedPresence` 到已认证 QUIC 再到 canonical runtime 远端工具调用的完整链路已在**生产连接路径**上打通并被 loopback 集成测试证明（含内容级结果校验与单次执行保证）；失败有界、fail-closed 面覆盖。
- 验证：Linux namespace NAT harness（TEST A–E）已实现、可复现、附准确命令，**未执行**；本环境（Windows，无可用 Linux）不能提供 L2 证据，本报告也不将 loopback 结果当作 NAT 证据。
- 距离 L2 只差一次 `sudo scripts/test-mesh-nat.sh` 的真实运行；距离 L3 需要真实 NAT 设备环境。
