# Apeireth Mesh RC2 Status

[Document-Meta]
Document: docs/design/mesh-rc2-status.md
Class: STATUS
Status: 🟢 活跃
Last-Modified: 2026-11-11
Author: Xephylos

> 冻结时点快照（2026-10-11）。权威现状以 [`mesh-architecture.md`](mesh-architecture.md)
> 为准；本文只是一页纸摘要。证据记录见
> [`mesh-nat-e2e-report.md`](mesh-nat-e2e-report.md)。
> **2026-11-10 更正**：原文写的「分支 `replay/mesh-on-rc2`，未 push」已不成立 ——
> 该线后来以 `feature/mesh-rc2-integration` 推到上游（PR #31），并在 #94（debt-round8）
> 合入 main 之后 rebase 成新 PR。**冻结时点的结论（下面各节）未变**：
> 打洞仍只在 Windows loopback 验过、真实 NAT 仍未测、自动传输编排仍未实现。

## Transport

- **Direct QUIC: verified.** Pinned mutual TLS (certificate key IS the identity
  key), mesh handshake, duplicate-connection convergence — loopback integration
  and hardening suites.
- **Punching: experimental.** `connect_punched()` is wired into the production
  connection path and verified on Windows loopback only. **Not verified under
  any real NAT**; the Linux namespace harness exists but has not been run.
  Best-effort direct connectivity, no compatibility guarantee.
- **Relay: implemented, not orchestrated.** The encrypted relay path
  (rendezvous presence + E2E circuits) exists and is covered by the WAN
  integration suites, but no flow automatically falls back from a failed
  direct attempt to the relay.

## Security

- Identity: verified (Ed25519 node identity, restricted atomic key file).
- Trust: verified (explicit known peers, no TOFU, live revocation, durable
  pinning).
- Authorization boundary: preserved. Inbound calls run the executing node's
  canonical governance (`ActionSource::Remote`); the mesh never mints an
  approval; punched links pass the same pinned handshake as any other link.

## Testing

- Mesh regression: passed (mesh / mesh-protocol / mesh-quic / mesh-rendezvous
  suites, 233 tests, 0 failures; re-run at freeze, 2026-10-11).
- Workspace regression: 4935 passed, 2 failed, 21 ignored — both failures are
  the pre-existing baseline pair (`apeireth-core` release-version assertion
  `rc.1` vs workspace `rc.2`; `apeireth-tools-canonical` `apply_patch` CAS
  test), unchanged and unrelated to the mesh.
- Loopback E2E (production punch path): passed — presence → punch →
  authenticated QUIC → convergence → remote `tool.mesh.echo` with result
  content checked and exactly one execution.
- Real NAT: not tested.

## Known limitations

- No automatic transport fallback (direct → punch → relay is a documented
  future state machine, not current behaviour).
- No real NAT verification (no Linux harness run, no router, no mobile
  network).
- No production gateway discovery integration: `connect_punched`'s first
  caller is the test harness; production dialing remains the static-endpoint
  `maintain_connection` path plus inbound `serve`.
- Multi-peer caveat: a punch's accept half can pick up a connection from a
  different (already pinned) peer; the handshake then fails closed (an
  availability question, never a trust one).
- The two baseline test failures above remain open and are not mesh-related.
- The live mDNS multicast probe (`crates/engine/mesh/tests/mdns_smoke.rs`) cannot
  pass on a macOS runner as an unsigned process: Apple's Local Network Privacy
  (TN3179) blocks LAN multicast sends and only root processes are exempt. CI
  runs that probe on macOS through `sudo -E`; the record-to-hint mapping is
  covered by deterministic unit tests on every platform.

## Frozen classification

- Stable / verified: node identity, trust model, pairing, authenticated QUIC
  transport, mesh handshake, remote runtime invocation, presence exchange,
  fail-closed validation, deterministic connection convergence.
- Experimental: NAT traversal (candidate exchange, presence validation, punch
  coordination, `connect_punched`, shared dual-role socket, QUIC
  connect/accept convergence) — loopback verified only.
- Not implemented: automatic transport orchestration, the direct → punch →
  relay state machine, production gateway discovery integration.

## 还差什么（open gaps，2026-11-11 复核）

- ❌ **真实 NAT 未实测**：打洞只在 Windows loopback 验过；Linux namespace harness
  （`scripts/test-mesh-nat.sh`）已写但未执行，真实 NAT 设备未测。
- ❌ **直连 → 打洞 → 中继的自动编排未实现**：当前无自动回退
  （`PeerConnector::direct_then_relay` 机制在，生产装配不在）。
- ❌ **生产网关发现未接线**：`connect_punched` 的首个调用方仍是测试夹具；
  生产拨号仍是 `maintain_connection` 静态 endpoint + 入站 `serve`。
- ❌ **远端准入不经本地 scope 校验、远端审批路径失败关闭**：本阶段的有意取舍。

## 已决的架构债（本批次收口）

- ✅ **Mesh crate 登记进架构契约**（2026-11-11）：`docs/adr/0024-mesh-line-contract-registration.md`
  把 `crates/foundation/mesh-protocol` 登记为 foundation、`crates/engine/mesh`、`crates/engine/mesh-quic`、
  `crates/services/mesh-rendezvous` 登记为 engine，并让 `[kernel].allowed_production_deps` 与 `[budgets]`
  上限对齐实测 ⇒ `python3 scripts/arch_guard.py --strict` 归零。
- ✅ **合并后树重测**（2026-11-11，upstream/main@`7abfa282` 已合入本分支）：契约上限按**合并后树**
  实测更新为 **35 crate / 132 生产边**（门面 crate `apeireth` 7 条边 + mesh 线 14 条边在合并树上相加），
  `[env]` 能力开关实测 152 ≤ 预算 154 ⇒ **预算未放宽**；明细见 ADR-0024 §2.5。

## 守卫命令（机器判对错）

- `cargo test -p apeireth-mesh -p apeireth-mesh-protocol -p apeireth-mesh-quic -p apeireth-mesh-rendezvous -j 2` —— mesh 栈回归
- `cargo test -p apeireth-runtime --test transparent_tool_routing --test transparent_provider_routing -j 2` —— 远端工具 / provider 路由
- `python3 scripts/check_dead_code.py` —— 死代码豁免只减不增
- `python3 scripts/audit_unwired.py --check` —— 孤儿 API 殓轮
- `python3 scripts/docs_truth.py --verify --block-stale` —— 状态声明自洽
