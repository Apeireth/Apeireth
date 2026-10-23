# W2-③ 多客户端会话复用（后端批）交付记录

> `Status: ✅ 已交付（2026-10-12，后端批 · 零前端触碰）`
> 一事：桌面/TUI/IM/web 多端同时在线同一个后端 —— 并发会话路由（事件扇出）、
> 同会话并发写语义（串行化回合）、会话归属面（`gateway.session_client`）、面板读快照一致性。
> 范围口径：F 轴 P2/P3 会话复用面（F-server-form.md「多客户端复用 + 会话并发安全」缺口）；
> 纪律：[CONSTRUCTION-DISCIPLINE.md]../CONSTRUCTION-DISCIPLINE.md)；战场：`crates/adapters/gateway/**` +
> `crates/engine/runtime/src/canonical/session.rs`（禁区零触碰，见 §7）。

## 1. 并发语义决策：同会话并发回合 = **排队 + 审计**（钉死，不用 busy 帧）

两客户端同时对一个会话发消息时，第二个回合**排队**等第一个跑完 —— 不发 busy 帧、
不丢用户输入、不交叠写会话账本。理由：

1. **多端同会话是协作面不是冲突面**：桌面 + TUI + IM + web 是同一个主人的多端，
   busy 拒绝会把并发压力转成用户可感知的失败，每个客户端都要自建重试循环，用户输入直接丢失；
2. **底层本来就在排队**：运行时内部已有 per-session 回合互斥，第二个回合本来就等；
   再贴一张 busy 帧是与屋内真实行为不符的假旋钮（名实不符）。显式排队 + 排队审计
   让等待**可观测**（排第几 / 等多久都入审计），而不是把排队伪装成拒绝；
3. **账本零交叠写是硬要求**，排队（先到先服务）是它最直接的实现：回合次序 = 入场次序，
   账本追加序号天然连续；busy 语义在突发下仍需排队兜底，等于两套语义并存；
4. **实现面最小**：`tokio` 互斥锁即先到先服务，零新造调度器。

钉死点（有测试为证）：第二回合 `queued=1 position=1` 入审计、持闸期间绝不入场、
放行后按入场次序落账、全程 200 无 busy 帧；同会话写者串行、会话之间并发。

两处如实边界：
- **审批暂停态不是 busy 帧**：会话挂着待审批时，新回合按既有口径 409
  `SessionApprovalPending`（回合暂停等人批的显式错误，与并发写语义无关），本批不改；
- **排队无上限**（先到先服务），排队深度可观测（`writers_in_flight`）。限流/队列上限
  是另一个显式语义，需要时装真旋钮，本批不假造。

## 2. 接线图

```text
多客户端 (桌面 / TUI / IM / web)
   │  Authorization: Bearer <令牌>  (+ 可选自称名头 x-apeireth-client)
   ▼
令牌门中间件 (三态沿用 W2-②)
   ├─ name↔token 绑定校验: 自称名 ≠ 令牌绑定名 → 401 auth_invalid_key (冒名 fail-closed)
   ├─ 过门身份 ClientIdentity 随请求扩展下行 (署名键只来自令牌绑定)
   └─ gateway.auth_client / auth_rejected 审计 (W2-② 口径不变)
   │
   ├── 回合面 POST /v1/chat[/completions] ──► execute_chat_attributed / run_turn_serialized
   │      ┌────────────────────────────────────────────────────────┐
   │      │ SessionManager::write_permit(session)  会话写闸 (session.rs) │
   │      │   同会话: 先到先服务排队 (第二回合排队, 不发 busy 帧)        │
   │      │   会话间: 并发 (车道互相独立, 4096 车道有界 M20 语义)        │
   │      │   排队事实 SessionWriteAdmission { position, waited }       │
   │      └────────────────────────────────────────────────────────┘
   │        │ 持证
   │        ▼
   │      Runtime::execute_outcome / execute_outcome_streaming (运行时既有回合互斥)
   │        → 账本写永不交叠 (消息/日志 lockstep, 追加序号连续)
   │        → gateway.session_client 审计: session= client= action=turn queued= position= wait_ms=
   │
   ├── 审批面 POST /v1/approvals/resolve ──► resolve_approval_attributed (同闸, action=approval.resolve)
   │      IM 桥 CanonicalChainHandler::resolve ──► 同闸 (im-bridge 进程内客户端)
   │
   ├── 会话写面 PATCH/DELETE /v1/sessions/{id}[/settings] ──► SessionManager::update / write_permit+delete
   │        → 读-改-写全程持证: 设置补丁与回合互相不吞并 (零丢更新)
   │        → gateway.session_client: action=settings.patch | session.delete
   │
   ├── 事件面 GET /v1/apeireth/events(?session=<id>)
   │      ├─ 全量订阅: 既有语义 (逐连接租约 + 连接即送快照帧 + 丢帧显式省略)
   │      └─ 会话订阅 (W2-③): 每客户端独立订阅 (独立接收器 + 独立登记)
   │            同会话事件扇出多端 / 会话间隔离 / 会话帧面只在订阅期留存 (有界)
   │            gateway.session_client: action=events.subscribe
   │
   └── 面板面 GET /v1/panel/*
          每请求 = 一次快照读 (会话列表/内存计数): 响应内行自洽, 多客户端读数一致
```

## 3. 语义契约（钉死面）

| 面 | 语义 | 钉法 |
|---|---|---|
| 同会话并发回合 | **排队 + 审计**（见 §1） | `multi_client_session.rs::a_second_turn_on_a_busy_session_queues_instead_of_being_rejected` |
| 会话账本写 | 同会话写者先到先服务串行，会话间并发；绝不交叠写 | `session.rs` 写闸单测 + 对抗压测 |
| 设置补丁/删除 | 与回合同闸（读-改-写全程持证），零丢更新 | `a_settings_patch_during_a_running_turn_preserves_both_writes` |
| 事件扇出 | 每客户端独立订阅；同会话事件扇出多端；会话间隔离；会话帧面仅订阅期留存（有界、不回放） | `two_clients_on_one_session_each_get_the_sessions_event_fan_out`、`the_sse_endpoint_can_scope_a_connection_to_one_session` |
| 会话归属 | `gateway.session_client`：`session= client= action=… [queued= position= wait_ms=]`；署名键只来自令牌绑定 | `the_gateway_session_client_audit_binds_the_client_name_to_the_session` |
| 令牌隔离 | 自称名头必须与令牌绑定名一致，否则 401；无绑定的自称名一律 401（fail-closed） | `a_client_cannot_claim_another_clients_audit_name`、`a_claimed_name_without_a_token_binding_is_refused` |
| 面板读 | `/v1/panel/*` 每请求一次快照读：响应内行自洽（同版本），多客户端静止窗口读数一致 | `panel_session_reads_are_one_consistent_snapshot_per_request`、`panel_memory_counts_are_read_as_one_snapshot_per_request` + `session.rs` 存储快照单测 |

## 4. 测试证据（新增 12 例集成 + 7 例单测，全部为行为钉）

| 验收项 | 测试（文件） | 例数 |
|---|---|---|
| 双客户端订阅扇出（独立订阅/同会话扇出多端/会话隔离/一端断开不动另一端/帧面有界） | `multi_client_session.rs`：`two_clients_on_one_session_each_get_the_sessions_event_fan_out` | 1 |
| 会话订阅 HTTP 面（`?session=` 过滤 + 快照只读本会话 + `events.subscribe` 归属审计） | `multi_client_session.rs`：`the_sse_endpoint_can_scope_a_connection_to_one_session`、`an_invalid_scoped_session_id_is_refused_with_an_explicit_frame` | 2 |
| 同会话并发回合串行化（**排队 + 审计**钉死：第二回合排队不拒绝、持闸期不入场、按入场次序落账、queued/position 入审计） | `multi_client_session.rs`：`a_second_turn_on_a_busy_session_queues_instead_of_being_rejected`；`session.rs` 单测：`write_permits_serialize_one_session_and_stay_concurrent_across_sessions`、`a_queued_writer_really_waits_until_the_holder_releases` | 3 |
| 会话账本零交叠写（对抗并发压测：6 回合 × 2 客户端 + 3 设置补丁同砸一会话；消息全到齐/追加序号连续/回合块相邻/revision 不回退/设置不被回合覆写） | `multi_client_session.rs`：`concurrent_turns_and_settings_writes_never_interleave_the_session_ledger`、`a_settings_patch_during_a_running_turn_preserves_both_writes`；`session.rs` 单测：`concurrent_updates_never_lose_a_write_on_one_session` | 3 |
| 客户端名审计关联（`gateway.session_client` 带 `session= client= action=`，与回合事件共享会话键） | `multi_client_session.rs`：`the_gateway_session_client_audit_binds_the_client_name_to_the_session`；`client_tokens.rs` 单测：`session_client_audit_rows_name_the_bound_client_and_the_session` | 2 |
| 令牌隔离（A 令牌自称 B → 401 拒 + 冒名者名绝不进审计；无绑定自称名 → 401 fail-closed） | `multi_client_session.rs`：`a_client_cannot_claim_another_clients_audit_name`、`a_claimed_name_without_a_token_binding_is_refused`；`client_tokens.rs` 单测：`a_claimed_name_binds_only_to_its_own_token_identity` | 3 |
| 面板快照一致（每请求一次快照读：行自洽同版本/静止窗口多客户端读数一致/计数不混版本） | `multi_client_session.rs`：`panel_session_reads_are_one_consistent_snapshot_per_request`、`panel_memory_counts_are_read_as_one_snapshot_per_request`；`session.rs` 单测：`a_stored_session_list_is_one_coherent_snapshot_under_concurrent_writes` | 3 |
| 读改写原子面（缺失会话不凭空造） | `session.rs` 单测：`update_reports_missing_sessions_without_creating_them` | 1 |
| 过门身份下行（令牌绑定署名键） | `multi_client_session.rs`：`the_gate_passes_the_token_bound_identity_down_to_handlers` | 1 |

既有套件零回归：`gateway_auth_face.rs`（W2-② 401 矩阵原样）、`gateway_token_gate.rs`、`events.rs`
（SSE 交付面钉法原样，仅新增会话订阅面）、`canonical_entry_e2e.rs` / `canonical_entry_architecture.rs`
（`execute_chat` 签名不变，入口仍直连 `Runtime::execute_outcome`）、`session_settings.rs` 单测
（PATCH/DELETE 行为不变，仅写法改走写闸）、`im_message_bridge.rs` / `im_approval_cards.rs`
（IM 链签名不变，内部多一层写闸串行）等全部保留。

## 5. W3 多用户留桩（本批再进一步）

- `ClientIdentity`（令牌绑定署名键）已随请求扩展下行到所有处理器 —— W3 按客户端授权
  只需在这个身份上挂权限判定，无需再动认证面；
- `gateway.session_client` 审计行已把「谁」与「哪个会话的什么动作」绑定 —— 问责链成形；
- 每客户端**权限**分级（主人/访客/只读）仍属 W3（E 轴），本批不装。

## 6. 门禁证据（2026-10-12 实测，全部 `-j 2`）

| 门禁 | 命令 | 结果 |
|---|---|---|
| 全量测试 | `cargo test --workspace --locked -j 2` | ✅ 退出码 0；**184 套件 / 5163 例通过 / 0 失败 / 22 忽略**（既有 180 套件零失败回归；套件数 180→184 含本批新增 `multi_client_session` 与并行线新增面） |
| Clippy | `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | ✅ 退出码 0，零警告 |
| 格式 | `cargo fmt --all -- --check` | ✅ 干净（含跨范围顺手修的 1 行机械回流，见变更面） |
| 中性表述 | `scripts/check-neutral-terms.ps1` | ✅ 零命中 |
| 密钥扫描 | `scripts/secret-scan.ps1` | ✅ 无泄漏 |

## 7. 残余（诚实条款）

| 残余 | 状态 |
|---|---|
| IM 桥回合具名审计 | IM 是进程内客户端（`im-bridge`），回合已过同一把会话写闸串行；逐频道具名审计留 W3（IM 身份是频道签名，与令牌表分属两套凭证面） |
| 绕过网关写入口的直连写者 | 直接调 `Runtime`/`SessionStore` 写面的调用方不经会话写闸；回合间互斥仍由运行时 per-session 回合锁兜底，但与网关设置写的交叠窗口如实登记（生产多客户端流量全走网关入口，均在闸内） |
| TLS | W2-② 接口桩不变，本批零触碰 |
| 每客户端权限分级 | W3（E 轴多用户） |

变更面（本批）：`crates/engine/runtime/src/canonical/session.rs`（会话写闸 + 原子读改写 + 单测）、
`crates/engine/runtime/src/canonical/mod.rs`（导出 3 个新符号，最小叠加）、
`crates/adapters/gateway/src/{events,client_tokens,canonical_entry,session_settings,im_bridge,panels,lib}.rs`、
`crates/adapters/gateway/tests/multi_client_session.rs`（新增）、
文档 `gateway-api-contract.md`（§4/§8 增补）/ `F-server-form.md` / 本文。
跨范围顺手修（报备 + 最小改动 + 说明）：`crates/foundation/governance/src/risk.rs` 仅
rustfmt 一行机械回流（`cargo fmt --all` 门禁要求全库干净；内容零改动，属验证线在途文件）。
未触碰禁区（frontend/**、CHANGELOG.md、.github/**、docs/design/**、HANDOFF-NOTES.md、
canonical/{quota,execute}.rs、crates/capabilities/tools/**、memory 两文件、src-tauri/**）；未 commit。
