# W3-② 用户身份与角色（后端批）交付记录

> `Status: ✅ 已交付（2026-10-13，后端批 · 零前端触碰）`
> 一事：用户模型 + 角色词表冻结 + 治理层 role 权限矩阵 + 令牌→用户→角色链 + CLI 用户管理命令 +
> `gateway.user_action` 审计 + G1 对抗用例转绿。
> 范围口径：星群·治理层（W3 多户多端波的治理/身份面）；架构定案 = 星群为骨（[F-server-form.md](../F-server-form.md))）；
> 纪律：[CONSTRUCTION-DISCIPLINE.md](../CONSTRUCTION-DISCIPLINE.md))；威胁模型销账随行：[E-threat-model.md](../E-threat-model.md)) §2/§4/§5/§6。
> 接缝（W3-① 并行线）：`UserDomain.role` 由域面建立、本面消费（宽接口 `UserIdentity::from_wide(user_id, role: Option<&str>, client_names)`，
> 未配置/未知角色都 fail-closed 拒）；其域面角色词面 `DomainRole::{owner,member}` 是冻结词表的**兼容子集**
> （`as_str()` 字面值一致，`guest` 为治理层词表项、域面未发）；用户表独立文件 `users.json` + `domain` 域引用（只携引用不判域）。
> 诚实条款：本批只落**身份/授权面**（谁能干什么）；跨户**数据**隔离（记忆库/域目录）归 W3-① 数据隔离线，见 §5 残余。

## 1. 角色矩阵表（治理层 role 权限矩阵，词表冻结）

**角色词表（冻结）**：`owner` / `member` / `guest` —— 只有这三个字面值，未知角色（含大小写变体）= fail-closed 拒
（`UserRole::parse`；不就近映射、不默认降级）。扩词表 = 枚举 + 矩阵 + 测试三处同批。

| 能力（`UserCapability`） | owner | member | guest | 语义 |
|---|---|---|---|---|
| `Read` 只读面 | ✓ | ✓ | ✓ | 查询 / 快照 / 事件流 |
| `Write` 域内写面 | ✓ | ✓ | ✗ | 会话写 / 设置写 / 记忆写 |
| `ToolUse` 只读工具 | ✓ | ✓ | ✓ | 无副作用工具 |
| `ToolUseWrite` 写类工具 | ✓ | ✓ | ✗ | 有副作用工具 |
| `GitWriteL1` git 写·L1 档 | ✓ | ✓ | ✗ | 自动放行档（更高档不入本表） |
| `ApprovalL2` 审批·L2 档 | ✓ | ✓ | ✗ | 批准挂起审批（跨用户归属另闸） |
| `ManageUsers` 用户管理 | ✓ | ✗ | ✗ | `apeireth user add\|list\|role\|remove` |
| `AdminControl` 管理面 | ✓ | ✗ | ✗ | admin 配置 / 停机 / 授权热撤 |

判定 fail-closed：不在该角色显式清单里的能力一律拒。矩阵本体 = `crates/foundation/governance/src/user_access.rs`
（`UserRole::grants`，24 格逐格测试钉住）。

**网关端点分类表**（`canonical_entry.rs::required_user_capability`，角色门整面中间件）：

| 端点族 | 分类 | owner | member | guest |
|---|---|---|---|---|
| `GET/HEAD` 读面（面板 / 模型 / 事件流 / 审批收件箱…） | 无能力要求（全员持 `Read`） | 进 handler | 进 handler | 进 handler |
| `POST /v1/chat`、`/v1/chat/completions`、`PATCH|DELETE /v1/sessions/*`、`POST /v1/memory/*`、其余非读方法 | `Write` | 进 handler | 进 handler | **403** `permission_denied` |
| `POST /v1/approvals/resolve` | `ApprovalL2` + **归属闸** | 任意会话可批 | 仅本人绑定会话 | **403** |
| `/v1/admin/*`（config 读写 / shutdown）、`POST /v1/panel/grants/revoke` | `AdminControl` | 进 handler | **403** | **403** |

归属闸（`apeireth_governance::may_resolve_approval`）：owner 全权；其余角色 = 持 `ApprovalL2` **且** 会话归属 == 本人；
会话无归属 / 调用方未绑定 = 拒（fail-closed，不猜归属）。会话归属 = 首个具名写者（`UserAccess::bind_session_user`）。

**未激活口径（零回归）**：未配置 `users.json` = 角色面未激活 → 角色门整体休眠，W2-② 行为逐字不变（不新增拒绝面）；
激活后**未绑定用户**在受闸面一律 403 fail-closed。角色只来自令牌绑定的解析链，请求无法自称角色。

## 2. 接线图

```text
配置面                                          运行时面 (角色门 = 令牌门之内)
──────────────────────────────────            ─────────────────────────────────────────────
users.json (StoredDoc 拒开, 身份 `users` v1)──┐
  UserRecord { user_id, role,                 │   HTTP 请求
    client_names[], domain? }                 │      │
client-tokens.json / APEIRETH_GATEWAY_TOKENS ─┼─►    ▼
  ClientTokenSpec { name, token, user_id? }   │   令牌门 require_gateway_token (W2-② 认人)
  (W2-② name→token 升级为 name→token→user_id)│      │ 命中 → ClientIdentity { client }
                                              │      ▼
坏用户表/未知角色/绑定不一致 ──► 拒开          │   角色门 require_user_capability (W3-② 判权)
                                              │      │ resolve_user_identity(client, tokens, users)
UserTable (governance::user_access)           │      │   = name→token→user_id→role (回落: client_names)
  角色词表冻结 (owner|member|guest)            │      │ 未绑定 → UserContext { user: None }
  role 权限矩阵 (24 格显式)                    │      ▼
  users.json 增删改 + 失主不变量               │   required_user_capability(method, path)
                                              │      │ 读面: 无要求 | 写面: Write
UserAccess (gateway::client_tokens)           │      │ 管理面: AdminControl | 审批: ApprovalL2
  users: Option<UserTable> (None=未激活)      │      ▼
  session_users: 会话→用户 (首写者绑定)       │   矩阵判定 → 拒: 403 permission_denied
                                              │                        ↘ allow/deny 都落审计
CLI 管理面 (cli::user_admin)                  │                gateway.user_action
  apeireth user add|list|role|remove          │                 user=<id> role=<role>
  owner 角色门 (--token → ManageUsers)        │                 action=<能力> decision=allow|deny
  引导: 表空首用户免令牌但必须 owner           │
  不变量: 末 owner 不可删/降; 发令牌序         │   审批归属闸 (native_resolve_approval)
                                              │     may_resolve_approval(role, 本人, 会话归属)
                                              │     跨用户拒 → 403 + action=approval_ownership
```

## 3. 配置面契约

| 面 | 形状 | 语义 |
|---|---|---|
| `users.json`（数据目录） | StoredDoc 单文档（身份 `users`，版本 1）：`{"users":[{"user_id","role","client_names":[],"domain"?}]}` | 坏文件/身份不符/版本不可读/未知角色/user_id 重复/客户端名多属一律**拒开**（StoredDoc 拒开语义，0 静默回退）；缺失 = 角色面未激活（兼容路径） |
| `client-tokens.json` / `APEIRETH_GATEWAY_TOKENS` | W2-② 形状 + 可选 `user_id`（`{name, token, user_id?}`） | name→token→**user_id** 链；声明了 `user_id` 就必须在用户表内且回绑该客户端名（`validate_client_user_bindings`，不一致**拒开**）；未声明按用户表 `client_names` 回落（env 单令牌面 `env` 走这条） |
| `apeireth user add` | `--user-id ID --role owner\|member\|guest [--clients a,b] [--domain D] [--token T]` | 引导：表空免令牌但**首用户必须 owner**；表非空要 owner 令牌 |
| `apeireth user list` | `[--token T]` | owner 令牌可读全表（user/role/domain/clients） |
| `apeireth user role` | `--user-id ID --role R [--token T]` | 未知角色拒；**末 owner 不可降级** |
| `apeireth user remove` | `--user-id ID [--token T]` | **末 owner 不可删**；令牌仍引用的用户删不得（先收令牌后删用户） |

发令牌序（问责链不得断，落盘/装载双闸）：令牌条目声明 `user_id` 的先建用户再写绑定；悬空或名实不符的绑定
→ CLI 管理动作拒落盘、网关装载期拒开。`domain` 字段是 W3-① 域目录的宽接口引用（本面只携引用不判域）。

## 4. 测试证据（新增 28 例，全部为行为钉）

| 验收项 | 测试（文件） | 例数 |
|---|---|---|
| 角色词表冻结 + 未知角色 fail-closed | `governance/src/user_access.rs`：`the_role_vocabulary_is_frozen_to_three_terms`、`unknown_roles_are_refused_fail_closed` | 2 |
| role 权限矩阵逐档（owner/member/guest × 读/写/工具/管理 + 档位） | `user_access.rs`：`the_role_permission_matrix_is_pinned_row_by_row`（24 格全表）、`guest_has_zero_write_surface_and_owner_has_everything` | 2 |
| 宽接口（W3-① 接缝）+ 令牌→用户→角色链 | `user_access.rs`：`the_wide_role_interface_refuses_unprovisioned_and_unknown_roles`、`identity_lookup_walks_user_id_and_bound_client_names`、`identity_structure_is_validated_fail_closed`；`gateway/src/client_tokens.rs`：`the_token_chain_resolves_name_to_token_to_user_id_to_role`、`token_and_user_bindings_must_agree_or_the_load_refuses` | 5 |
| 用户表拒开 + 管理操作 + 失主不变量 | `user_access.rs`：`stored_user_table_round_trips_and_broken_tables_refuse_to_open`、`unknown_roles_and_ambiguous_bindings_refuse_to_open`、`management_operations_keep_the_table_consistent_and_ownered`、`the_last_owner_can_never_be_removed_or_demoted`、`cross_user_approval_resolution_is_refused_for_non_owners` | 5 |
| 角色门面契约（未激活兼容 / 矩阵逐档进 handler） | `gateway/tests/user_authorization.rs`：`without_a_user_table_the_role_face_stays_inactive`、`owner_passes_write_admin_and_approval_faces`、`member_can_write_but_never_touches_the_admin_face`、`guest_has_zero_write_surface_but_keeps_read_access` | 4 |
| 对抗面（敌人⑤：成员冒名 owner / 访客越权写 / 跨用户审批 / 未绑定） | `user_authorization.rs`：`a_member_claiming_the_owner_client_name_is_refused`、`a_member_self_declaring_owner_rights_is_refused_on_management`、`cross_user_approval_resolution_is_refused_for_non_owners`、`an_unbound_client_is_refused_on_gated_faces_fail_closed` | 4 |
| `gateway.user_action` 审计行（user/role/action，allow+deny，零令牌） | `user_authorization.rs`：`user_action_audit_rows_name_user_role_and_action` | 1 |
| 管理命令（add/role/remove + 非 owner 拒 + 引导/失主/拒开） | `cli/tests/user_admin.rs`：`bootstrap_the_first_owner_then_manage_with_the_owner_token`、`management_is_owner_only_across_every_refusal_shape`、`the_first_user_must_be_owner`、`the_last_owner_can_never_be_removed_or_demoted`、`broken_faces_refuse_to_open_or_to_save` | 5 |

既有套件零回归：`gateway_auth_face.rs`（W2-② 401 矩阵原样）、`gateway_token_gate.rs`、`multi_client_session.rs`、
`resident_service.rs`、`admin_config.rs`、`session_settings.rs` 单测、`cli_command_interface.rs`（帮助面只增不改）等全绿；
W3-① 并行线在途面零触碰。唯一结构演进注记：`ClientTokenSpec` 增可选 `user_id` 字段（`serde(default)`，旧
`client-tokens.json` 兼容可读，既有测试只补 `user_id: None` 字面量）；`ErrorCode` 增 `permission_denied`
（403 角色门专用码，契约同步 §1/§6）。

## 5. 威胁模型销账记录（详表随行 [E-threat-model.md](../E-threat-model.md))）

| 账目 | 处置 | 证据 |
|---|---|---|
| G1「多用户隔离」之**身份/授权面** | **本批处置**：用户模型 + 冻结角色词表 + role 权限矩阵 + 令牌→用户→角色链 + 角色门（写面/管理面/审批面）+ 管理命令 owner 门 + `gateway.user_action` 审计 | §4 全部 28 例 |
| 敌人⑤(d) 成员冒名 owner | **转绿（本批）**：自称客户端名 401（name↔token 绑定）+ 角色只来自令牌链（无从自称）+ 管理面 403 | `a_member_claiming_the_owner_client_name_is_refused`、`a_member_self_declaring_owner_rights_is_refused_on_management` |
| 敌人⑤(c) 访客越权写 / 改全局开关 | **转绿（本批）**：guest 零写面 403 + 管理面（admin config/shutdown/授权热撤）owner 档 403 | `guest_has_zero_write_surface_but_keeps_read_access`、`member_can_write_but_never_touches_the_admin_face` |
| 敌人⑤(e) 跨用户审批（复用他人审批） | **转绿（本批）**：归属闸（owner 全权 / 他人会话 403 / 恰好一次失效另由审批四态守） | `cross_user_approval_resolution_is_refused_for_non_owners`（双套件） |
| G1 残余（如实） | **仍在**：① 跨户**数据**隔离（访客会话读主人记忆库）归 W3-① 数据隔离线（users/ 域模型）；② 工具执行档（`ToolUse`/`ToolUseWrite`/`GitWriteL1`）已入冻结矩阵并提供判定接口，但逐用户工具执行面的运行时接线随 W3-①/工具面批（本批网关面只接读/写/审批/管理四类）；③ 会话归属为首个具名写者粒度（细粒度审批发起人归属随域面细化） | 本行即实证注记 |

## 6. 门禁证据（2026-10-13 实测，全部 `-j 2`）

| 门禁 | 命令 | 结果 |
|---|---|---|
| 全量测试 | `cargo test --workspace --locked -j 2` | ✅ 退出码 0；**190 套件 / 5243 例通过 / 0 失败 / 22 忽略**（既有 186 套件零失败零跳过回归 + 本批新增 `user_authorization`、`user_admin` 2 套件 + W3-① 并行线在途新增面。取证期间并行线两次瞬时红（其自写 `user_domain_isolation` 用例断言口径、其结构演进的编译面），其修复后全绿——均非本批引入） |
| Clippy | `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | ✅ 退出码 0，零警告 |
| 格式 | `cargo fmt --all -- --check` | ✅ 干净（本批文件 + 并行线在途文件的机械空白补平） |
| 中性表述 | `scripts/check-neutral-terms.ps1` | ✅ 零命中（69 个变更文件新增行全中性；取证途中曾见并行线外部源码暂存的 1 处命中，其归置后复扫全绿） |
| 密钥扫描 | `scripts/secret-scan.ps1`（scan-staged + scan-all） | ✅ 无泄漏 |

变更面（本批）：`crates/foundation/governance/src/{user_access.rs（新增）,lib.rs}` + `Cargo.toml`（dev-dep `fs-err`，
`Cargo.lock` 随之加一条依赖边）、`crates/adapters/gateway/src/{client_tokens,canonical_entry}.rs`、
`tests/user_authorization.rs`（新增）、`tests/{gateway_auth_face,multi_client_session}.rs`（`ClientTokenSpec` 字面量补 `user_id: None`）、
`crates/adapters/cli/src/{user_admin.rs（新增）,lib.rs,main.rs}`、`tests/user_admin.rs`（新增）。
跨范围最小改动（宪章 §二.4 报备）：`gateway/src/{panels.rs`（`GatewayState` 增 `user_access` 字段 + 面板管理面判定
`panel_admin_face`）`,session_settings.rs}`（测试装配字面量补字段）`,remote_access.rs}`（测试字面量补字段）`,
error_frame.rs,error_codes.rs}`（`ErrorCode` 增 `permission_denied`，契约已同步）。
**共树协同注（W3-① 并行线）**：其在途结构演进两次造成全树编译中断——① `SelfStatusSnapshot` 增
`domain`/`domain_reason` 后 `self_status_source.rs` 构造器未跟上；② `ProductionBackends` 增
`self_status_domain` 后 `cli/src/lib.rs` 装配字面量未跟上。两处均按其自身「未接线 = 显式 null/None + 原因」
口径补位（`DATA_PROBE_NOT_WIRED` / `None`，含报备注释），域探测口真接线归 W3-① 线；其文件的格式补平
（`cargo fmt --all`）为机械空白，无语义改动。
文档：`E-threat-model.md` / `F-server-form.md` / `gateway-api-contract.md` / 本文。
未触碰禁区（frontend/**、CHANGELOG.md、.github/**、docs/design/**、HANDOFF-NOTES.md、research/verification/**、
`crates/engine/memory/{reflexion,river_topology}.rs`）；未 commit。
