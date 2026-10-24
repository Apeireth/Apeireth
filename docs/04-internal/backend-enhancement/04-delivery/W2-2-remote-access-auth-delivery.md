# W2-② 远程访问与认证面（后端批）交付记录

> `Status: ✅ 已交付（2026-10-11，后端批 · 零前端触碰）`
> 一事：令牌门从「两端点」扩到**全 API 面** + 远程绑定 fail-closed + 每客户端令牌体系 + 敌人③对抗收口。
> 范围口径：F 轴 P2（多客户端凭证面：每客户端令牌，为 W3 多用户留桩）+ P3（远程访问认证主体）+
> G4 缺口 + G10 残余 (c)；纪律：[CONSTRUCTION-DISCIPLINE.md](../CONSTRUCTION-DISCIPLINE.md)；
> 威胁模型销账随行：[E-threat-model.md](../E-threat-model.md) §2/§4/§5/§6；服务端形态回填：[F-server-form.md](../02-axes/F-server-form.md)。
> 诚实条款：本批只交付后端面；桌面 WebView 的令牌覆盖缺口如实记 §5 残余，不假称桌面全端点已带令牌。

## 1. 认证面矩阵表（全网关化后的逐族口径）

三态语义沿用，「设/未设」按**整个令牌族**判定：`APEIRETH_GATEWAY_TOKEN`（env 单令牌面）或客户端令牌表
（`APEIRETH_GATEWAY_TOKENS` / `client-tokens.json`）任一在场 = **Enforced**；两面皆空 = 未设（放行 +
警告帧 + 审计，兼容路径）；`APEIRETH_GATEWAY_TOKEN=none` = 逃生阀（明示不设防，留审计；与令牌表互斥拒开）。

| 端点族 | 端点 | 未设（兼容路径） | Enforced（令牌族在场） | `none`（逃生阀） | 备注 |
|---|---|---|---|---|---|
| 健康探活 | `GET /health` | 无认证放行 | **无认证放行（豁免）** | 无认证放行 | 探测刚需（服务管理器/侧车监督/服务脚本探针）；不校验、不告警、不审计 |
| chat | `POST /v1/chat`、`POST /v1/chat/completions` | 放行 + 警告帧 + 审计 | Bearer 必须命中令牌族，否则 401 | 放行 + 留审计 | 入站 `Authorization` 只做门鉴权，凭据解析在后端凭据面 |
| 事件流 | `GET /v1/apeireth/events`、`GET /v1/apeireth/presence` | 同上 | 同上 | 同上 | SSE 连接请求本身过门一次 |
| 面板族 | `/v1/panel/*`、`/v1/tools/list`、`/v1/apeireth/capabilities`、`/v1/organs`、`/v1/modules`、`/v1/safety/guard/*`、`/v1/workbench/turn`、`/v1/memory/append|forget|protect|unprotect`、`/v1/panel/grants*` | 同上 | 同上 | 同上 | 含记忆写面（append/forget/protect） |
| 会话族 | `GET|PATCH /v1/sessions/{id}/settings`、`DELETE /v1/sessions/{id}` | 同上 | 同上 | 同上 | |
| 审批族 | `GET /v1/approvals`、`POST /v1/approvals/resolve` | 同上 | 同上 | 同上 | W1-③ 原敏感端点，口径不变 |
| admin 族 | `GET|POST /v1/admin/config`、`POST /v1/admin/shutdown` | 同上 | 同上 | 同上 | W1-③ 原敏感端点，口径不变 |
| 模型/运行时 | `GET /v1/models`、`/v1/providers`、`/v1/runtime/snapshot`（+`/v1/apeireth/…`） | 同上 | 同上 | 同上 | |
| 未匹配路径 | 任意未知路径（fallback） | 警告帧 + 审计后 404 | **401**（不借 404/401 差别泄露路由存在性） | 留审计后 404 | 令牌门是整面中间件，含 fallback |

错误口径不变：缺 Bearer = 401 `auth_missing_key`，伪造 Bearer = 401 `auth_invalid_key`，401 帧不回显任何令牌值。
恒定时间比较防时序侧信道；令牌值不进日志/帧/审计/Debug。

## 2. 接线图

```text
配置面                                          运行时面
──────────────────────────────                 ─────────────────────────────────────────────
APEIRETH_GATEWAY_TOKEN (env 单令牌面)──┐
APEIRETH_GATEWAY_TOKENS (JSON env) ────┼──► 令牌族 = GatewayTokenGate(三态) + ClientTokenTable
client-tokens.json (StoredDoc 拒开) ───┘        │  Enforced: 族内任一枚过门, 审计 client=<名>
                                                │  未设:    放行+警告帧+审计 (兼容路径)
APEIRETH_GATEWAY_TOKENS 坏配置 ──► 拒开          │  none:    逃生阀, 留审计 (与令牌表互斥拒开)
APEIRETH_LISTEN / --bind ──► 绑定解析            │            (env 面单令牌的客户端名 = `env`)
  (旗标 > env > 127.0.0.1)                      │
        │                                       ▼
        ▼                               HTTP: CORS(来源白名单, 外层) ──► 令牌门(整面中间件)
非回环绑定闸 enforce_remote_bind               │   ├─ /health (唯一豁免, UNAUTHENTICATED_PROBE_PATHS)
  回环: 放行                                    │   └─ chat/事件流/面板/会话/审批/admin 路由 + fallback
  非回环+令牌族: 放行+审计 mode=enforced        ▼
  非回环+none: 放行+审计 mode=declared_none   审计面: gateway.auth_client (过门, 带客户端名)
  非回环+无令牌: **拒启动** (fail-closed 即帧)       gateway.auth_unprotected (未设/逃生阀放行痕)
                                                  gateway.auth_rejected (401 拒绝痕)
APEIRETH_TLS_CERT / APEIRETH_TLS_KEY               gateway.remote_bind (远程绑定清场事实)
  = 接口桩: **未接线**                             gateway.tls_unwired (TLS 桩被设置时的显式警告)
  设值只触发显式警告 (不装死)
```

## 3. 配置面契约

| 旋钮 | 形状 | 语义 |
|---|---|---|
| `APEIRETH_GATEWAY_TOKEN` | 裸令牌 / `none` | 令牌族第一面（三态开关：有值=Enforced、未设=兼容路径、`none`=逃生阀）；客户端名固定 `env` |
| `APEIRETH_GATEWAY_TOKENS` | JSON `[{"name","token"}]` 或 `{"clients":[…]}` | 客户端令牌表主入口；坏配置**拒开** |
| `client-tokens.json`（数据目录） | StoredDoc 单文档（身份 `client-tokens`，版本 1） | 次入口；坏文件/身份不符/版本不可读一律**拒开**（StoredDoc 拒开语义），0 静默回退 |
| `APEIRETH_LISTEN` | host（可含方括号 IPv6） | 绑定地址；`--bind` 显式旗标 > env > `127.0.0.1`（缺省仅本机） |
| `APEIRETH_TLS_CERT` / `APEIRETH_TLS_KEY` | 路径 | **接口桩**：识别 + 显式警告（"仍未接线，流量是明文"），不启用 TLS、不硬造 |

客户端令牌表校验（全 fail-closed）：name 非空且 ≤64 字节且唯一；token 非空白、不得取哨兵 `none`、全表唯一
（重复令牌值 = 审计归谁歧义，拒开）；文件面文档身份 `client-tokens`（`CLIENT_TOKENS_DOC_NAME`），
格式版本 1，仅当前版本可读。W3 多用户留桩：本批先落「谁在调用」的可问责身份（审计署名），权限分级后批接。

## 4. 测试证据（新增 31 例，全部为行为钉）

| 验收项 | 测试（文件） | 例数 |
|---|---|---|
| 全端点 401 矩阵（chat/事件流/面板/审批/admin 各一）+ fallback | `gateway_auth_face.rs`：`enforced_gate_refuses_unauthenticated_chat_face` / `_events_stream` / `_panel_face` / `_approval_face` / `_admin_face` / `enforced_gate_also_covers_unmatched_paths` | 6 |
| 健康端点无认证 + 豁免面钉死 | `gateway_auth_face.rs`：`health_probe_stays_unauthenticated_in_every_gate_mode`（含豁免表 = 仅 `/health` 的断言） | 1 |
| 现行为兼容路径（本地回环桌面零回归） | `gateway_auth_face.rs`：`unset_gate_keeps_the_no_token_compat_path_working_with_warnings`、`the_desktop_minted_token_path_keeps_reaching_every_handler` | 2 |
| 客户端令牌表（多枚 / 审计带名 / 坏配置拒开） | `gateway_auth_face.rs`：`every_named_client_token_passes_and_the_audit_names_the_client`、`a_configured_token_table_enforces_the_gate_even_without_the_env_token`；`client_tokens.rs` 单测：`env_shape_parses_in_both_array_and_object_form`、`broken_configurations_fail_closed`、`stored_doc_file_is_loaded_and_broken_file_refuses_to_open`、`table_authenticates_each_named_client_and_redacts_values`、`escape_valve_and_token_table_are_mutually_exclusive` | 7 |
| 非回环绑定 fail-closed（无令牌拒启 / 带令牌启 / none 逃生留痕） | `remote_access.rs` 单测：`non_loopback_without_any_token_refuses_to_start`、`non_loopback_with_tokens_is_cleared_and_audit_traced`、`non_loopback_escape_valve_runs_but_leaves_the_audit_trace`、`loopback_binds_never_need_tokens`、`listen_bind_precedence_is_flag_then_env_then_loopback_default`、`loopback_literals_are_the_only_thing_treated_as_local`；进程级 `cli/tests/remote_access_e2e.rs`：`non_loopback_bind_without_any_token_refuses_to_start`、`the_listen_env_knob_goes_through_the_same_remote_bind_gate`、`non_loopback_bind_with_a_token_serves_behind_the_gate_and_keeps_the_audit`、`non_loopback_escape_valve_runs_but_leaves_the_audit_trace`、`broken_client_token_configuration_refuses_to_open` | 11 |
| 伪来源 / 伪 Bearer 对抗（敌人③） | `gateway_auth_face.rs`：`forged_origin_and_forged_bearer_are_refused_together`（伪来源头×伪 Bearer×chat/面板 全拒 + 本地来源 CORS 零回归）；逃生阀扩面 `the_escape_valve_traces_every_pass_across_the_whole_face` | 2 |
| TUI 全端点带 Bearer | `tui/src/gateway_http.rs`：`every_request_family_carries_the_bearer_token`（探活/模型/面板/会话设置/chat 五族） | 1 |
| TLS 接口桩（占位旋钮被识别并喊破） | `remote_access.rs` 单测：`tls_placeholder_knobs_are_reported_as_unwired` | 1 |

既有套件零回归：`gateway_token_gate.rs`（W1-③ 全部断言原样保留）、`resident_service.rs`（停机端点令牌门）、
`resident_gateway_e2e.rs`（真进程双开拒绝/优雅停机收尾）、`admin_config.rs` / `admin_hot_key_decoy.rs`（真热更）、
`service_management.rs`（服务脚本/单元静态面）、`supervisor_lifecycle.rs`（桌面监督：health 开 + minted token 开门）
全绿。唯一演进注记：`events.rs` 的 SSE 快照测试改在强制态+令牌下钉（未设口径下门警告帧会盖住快照断言对象，
快照语义钉法原样保留，注记在测试内）。

验证脚本 `scripts/verify-provider-path.ps1`：全部网关调用改带 `$gateBearer`（chat 族不再发凭据库密钥当入站
鉴权头——那是门的面不是凭据的面），并新增 `auth-matrix` 自检段（强制态下无令牌调 chat/面板必须 401，
未设口径显式注明兼容路径）。

## 5. 威胁模型销账记录（详表随行 [E-threat-model.md](../E-threat-model.md)）

| 账目 | 处置 | 证据 |
|---|---|---|
| G4 远程访问认证（缺口行） | **销账（本批）**：认证面全网关化 + 非回环绑定强制令牌（fail-closed）+ 每客户端令牌体系 | §4 全部 401 矩阵/绑定闸测试 |
| G10 残余 (c)「chat/事件流/面板族无认证面」 | **销账（本批）**：门扩到全 API 面，仅 `/health` 豁免 | `gateway_auth_face.rs` 401 矩阵 |
| G10 残余 (a) 逃生阀 | 仍在（如实）：`none` 明示不设防，审计留痕；非回环下同样放行但留痕 | 逃生阀测试 ×2（单测 + 进程级） |
| G10 残余 (b) 未设默认放行 | 仍在（如实，口径沿用）：兼容路径逐字不变，警告帧+审计可见可查 | `unset_gate_keeps_the_no_token_compat_path_working_with_warnings` |
| G10 残余 (d) web 构建取不到令牌 | web 构建取不到令牌仍在（设计内，web 构建可经 `VITE_APEIRETH_GATEWAY_TOKEN` 注入）；桌面 WebView 令牌覆盖缺口**已销账（前端认证面收口批）**：全 fetch 统一走 `gatewayAuthHeaders()`（三源令牌：IPC > env 注入 > 无，无令牌不发鉴权头），SSE 带令牌换 fetch 流式读取（EventSource 无法带鉴权头），无令牌保留 EventSource 原路径 | `runtime.ts` `gatewayAuthHeaders` 全调用面（32 项行为表）；`tauri-bridge.ts` 令牌面；`tests/gateway-auth-face.mjs` + `tests/sse-fetch-stream.mjs` |
| TLS 面 | **未接线（接口桩）**，如实：设 `APEIRETH_TLS_CERT/KEY` 只得显式警告，不得 TLS | `tls_placeholder_knobs_are_reported_as_unwired` + 启动警告 + `gateway.tls_unwired` 审计 |

## 6. 门禁证据（2026-10-11 实测，全部 `-j 2`）

| 门禁 | 命令 | 结果 |
|---|---|---|
| 全量测试 | `cargo test --workspace --locked -j 2` | ✅ 退出码 0；**180 套件 / 5092 例通过 / 0 失败 / 22 忽略**（既有套件零失败零跳过回归；180 = 任务书 177 套件口径 + 本批新增 `gateway_auth_face`、`remote_access_e2e` 2 套件 + 并行在途线新增面 1 套件。其中 `onboarding::tuning_state_evolution` 在一次全量并行跑中抖动失败一次，隔离复跑 3/3 绿 + 全量复跑绿，按宪章 §四「负载 flake 隔离复跑定性」定性为负载抖动，非本批引入） |
| Clippy | `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | ✅ 退出码 0，零警告 |
| 格式 | `cargo fmt --all -- --check` | ✅ 干净 |
| 中性表述 | `scripts/check-neutral-terms.ps1` | ✅ 零命中（124 个变更文件新增行全中性） |
| 密钥扫描 | `scripts/secret-scan.ps1`（scan-staged + scan-all） | ✅ 无泄漏 |

变更面（本批）：`crates/adapters/gateway/src/{canonical_entry,panels,session_settings,lib}.rs`、
新增 `client_tokens.rs` / `remote_access.rs`、`tests/gateway_auth_face.rs`（新增）、`tests/events.rs`（演进注记 1 处）、
`crates/adapters/cli/src/{lib,main}.rs`、`tests/remote_access_e2e.rs`（新增）、`crates/adapters/tui/src/{main,gateway_http}.rs`、
`scripts/verify-provider-path.ps1`、`packaging/service/{apeireth-service.ps1,linux/apeireth-gateway.service,README.md}`、
文档 `E-threat-model.md` / `F-server-form.md` / `server-deployment.md` / `gateway-api-contract.md` / 本文。
未触碰禁区（frontend/**、CHANGELOG.md、.github/workflows/**、HANDOFF-NOTES.md、docs/design/**、矿脉线文件）；未 commit。
