# V2-c 域面逐面开放（面板/审批/事件/管理面按域路由补全）交付记录

> `Status: ✅ 已交付（后端批 · 零前端触碰 · 接棒收尾）`
> 一事：非默认域的**面板/审批/管理面**逐面开放——面板读面按域过滤返回、审批面归属域
> 校验（W3-② 跨用户审批闸升级为跨域归属）、事件面与面板订阅共用同一在册探测、
> 管理面保持全局而审计带域标注。
> 战场：`crates/adapters/gateway`（panels.rs / task_panel.rs / canonical_entry.rs /
> domain_context.rs / events.rs / client_tokens.rs / user_admin.rs / domain_admin.rs）。
> 纪律：[CONSTRUCTION-DISCIPLINE.md](CONSTRUCTION-DISCIPLINE.md)；
> 前置卷：[W3-3-domain-partner-instances.md](W3-3-domain-partner-instances.md)（残余表 §7）
> 与 [W3-1-multi-user-data-isolation.md](W3-1-multi-user-data-isolation.md)（W3-① 同帧契约）。
> 诚实条款：本批只开**只读**面板面 + 审批归属域 + 管理面审计标注；面板族写面与其余面
> 仍按域闸 fail-closed（见 §1 矩阵"仍拒"行）；域开通即时生效 / 归档面逐域分库 /
> 任务与授权行的域归属标注为如实残余（§6）。本批为**接棒批**：前任施工中道猝死于
> `record_user_action` 签名演进（增 `domain` 标注参）的调用点补位断点（11 处编译断点），
> 本批补齐调用点并钉死标注语义（§4），其余面板/审批面按域代码由前任落盘、本批验证全绿。

## 1. 逐面开放矩阵（钉死）

| 面 | 端点 | 非默认域 | 过滤依据 / 语义 | 行为测试钉 |
|---|---|---|---|---|
| 会话面板 | `/v1/panel/sessions` | **开** | 本域账本直读（与 `/v1/sessions/*` 同一写闸同一账本，一次快照读）——他域行物理不在本域库，计数天然隔离 | `panel_faces_filter_every_row_by_domain_and_counts_never_leak_the_other_domain` |
| 记忆面板 | `/v1/panel/memory/episodes`、`/v1/panel/graph` | **开** | 行会话归属 ∈ 本域在册（[`DomainScope::owns`]）；graph 会话节点在册才留、情节节点随本域包含边留下 | 同上 |
| 审计面板 | `/v1/panel/audit` | **开** | 行归属记号 `session=<id>`（在册探测）/ `domain=<id>`（与本域比对）；无归属记号 = 不进域视图（fail-closed） | 同上 + `panel_rows_without_attribution_markers_stay_out_of_domain_views` |
| 授权面板 | `/v1/panel/grants` | **开** | 授权行无域归属记号 → 域视图零行（fail-closed，宁拒不泄；行归属标注见 §6 残余） | `panel_faces_filter_…` |
| 任务面板 | `/v1/panel/tasks` | **开** | 任务行无域归属记号 → 零行；**计数按可见行重算**（共享快照的计数不得泄进域视图——A 域看不进 B 域计数） | `panel_faces_filter_…` |
| 审批面 | `/v1/approvals`、`/v1/approvals/resolve` | **开** | 归属域校验（V2-c）：目标会话不在本域账本在册 = 与不存在**同帧** 404 `session_not_found`（owner 亦无跨域通路）；本域过闸后续跑走本域伙伴实例（无实例 = 403 `domain_refused` 不代跑） | `approval_faces_treat_cross_domain_sessions_as_missing` |
| 管理面 | `/v1/panel/users\|domains`、`/v1/admin/users*\|domains*` | **开** | **保持全局**（owner 全局视角，不按域过滤）；管理动作审计带 `domain=<目标域>` 标注（§4） | `admin_faces_stay_global_and_audit_rows_carry_the_target_domain_mark` |
| 事件面 | `/v1/apeireth/events` | 开（W3-③） | `?domain=` 自称 + 按域过滤；**与面板共用同一在册探测**（订阅一致，§3） | `panel_visibility_and_event_delivery_share_one_domain_membership_probe`、`event_subscriptions_cannot_name_another_domains_session` |
| chat 面 | `/v1/chat`、`/v1/chat/completions` | 开（W3-③） | 走该域伙伴运行时实例（W3-3 卷 §4） | W3-3 套件（`domain_partner_chat.rs`） |
| 其余面 | `/v1/models`、`/v1/panel/traces*`、`/v1/panel/skills`、`/v1/panel/grants/revoke`、`/v1/panel/tasks/*/human-judge-retry`、记忆写面、`/v1/admin/config\|shutdown` … | **仍拒** | 403 `domain_forbidden` 恒定帧（面未按用户域开放，不泄露默认域数据；写面随批，不随读面一起放开） | `panel_write_mates_stay_closed_while_the_read_faces_open`、`user_domain_isolation.rs::non_default_domains_are_confined_to_the_domain_routed_surface` |

**同帧语义保持（W3-① 契约钉死，零让步）**：

| 形态 | 判定 | 帧 | 测试钉 |
|---|---|---|---|
| 跨域资源（审批/事件 `?session=` 点名他域会话） | 读作不存在 | 404 `session_not_found`，与真不存在**逐字节同帧** | `approval_faces_…`、`event_subscriptions_…` |
| 自称域头/`?domain=` ≠ 本域 | 拒 | 403 `domain_forbidden` **恒定帧**（目标域存在与否同帧） | `the_domain_claim_keeps_one_constant_403_frame_on_the_new_faces` |
| 面未按域开放 | 拒 | 403 `domain_forbidden` 恒定帧 | `panel_write_mates_…` |
| 本域数据面未开/域配置坏 | 拒 | 403 `domain_refused`，真因入审计（不给空视图假装本域无数据） | `a_domain_without_an_open_session_plane_is_refused_not_given_an_empty_view` |

**面闸演进理由**（演进断言随行注明，宪章 §一.4）：W3-① 残余「非默认域仅开
`/v1/sessions/*`」由 W3-③ 开 chat/事件两面、本批开面板/审批/管理面逐面兑现——
`user_domain_isolation.rs::non_default_domains_are_confined_to_the_domain_routed_surface`
的 `/v1/panel/sessions` 403 收敛断言移除（行为本批有意变更），同名例保留
`/v1/models`、`/v1/panel/traces`、`/v1/panel/skills` 的收敛断言，并补
`panel_write_mates_stay_closed_while_the_read_faces_open` 钉「只读口开、写 mate 仍关」。

## 2. 面板按域过滤（五面 + 计数铁律）

- **过滤依据口径唯一**：行的域归属 = 行内会话 id（会话行 `id` / 记忆行 `session_id` /
  图节点 `session:<id>` 投影约定）或行归属记号（审计行 `session=`/`domain=`）；
  两者皆无 = 无归属记号行 → **不进域视图**（fail-closed，宁拒不泄）；
- **计数铁律**：计数一律按**可见行**重算（任务面板六态计数 + 合计），共享快照的
  计数不得泄进域视图（A 域看不到 B 域计数）；
- **会话面板特例**：本域账本直读（非过滤），他域行物理不在本域库；
- 默认域 / 域面未接线 = 全量返回（兼容路径逐字零变化，`default_domain_reads_stay_unfiltered_on_every_panel_face`）。

## 3. 事件面一致性（面板订阅与事件域一致）

面板过滤与事件过滤**收敛到共享的 [`DomainScope`] 与同一在册探测**
（`SessionManager::contains`，轻量存在性探测）：

- 同一会话的面板行可见 ⇔ 其事件帧投递到该域订阅（`panel_visibility_and_event_delivery_share_one_domain_membership_probe`
  双向断言）；
- `?domain=` 自称与 `x-apeireth-domain` 域头**同一语义**（口径唯一，W3-③ 钉）；
- `?session=` 点名本域不在册会话 = 404 `session_not_found` 族（与会话面同语义，
  他域存在/真不存在两态逐字节同帧）；
- 无会话归属的进程级帧（生命周期等）照常投递；在册探测失败按不在册处理
  （fail-closed，宁拒不泄）。

## 4. 管理面：全局保持 + 审计域标注

- users/domains 管理面**不按域过滤**（owner 全局视角）——非默认域 owner 读到的
  与默认域同一张全表；
- `gateway.user_action` 审计行新增 `domain=<id>` 归属记号（`record_user_action`
  第 7 参，取值有界截断）：
  - **管理动作 = 目标域**（`user_add`/`user_role_change`/`user_remove` = 该用户域引用
    缺省同名域；`domain_create` = 被开的域）；成功行取自服务返回行（已过词面闸），
    失败行按意图目标/查表标注，**未核实不标注**（fail-closed）；
  - **授权裁决 = 行为域**（角色门裁决行 / `approval_ownership` 拒行 = 行为者所在域）；
  - 标注行随**域视图审计面**归属（`domain=<本域>` 才进该域视图），管理面本身保持全局；
  - 词面未过 `DomainUserId` 目录安全闸的取值不进审计行（外部可控输入不无界进审计行）。

## 5. 测试证据（本批新增 13 例，全部为行为钉）

| # | 验收项 | 测试（文件） | 例数 |
|---|---|---|---|
| 1 | 面板六面按域过滤 + A 域看不到 B 域计数（含任务计数重算） | `gateway/tests/domain_faces_open.rs`：`panel_faces_filter_every_row_by_domain_and_counts_never_leak_the_other_domain` | 1 |
| 2 | 无归属记号行 fail-closed 不进域视图 | `domain_faces_open.rs`：`panel_rows_without_attribution_markers_stay_out_of_domain_views` | 1 |
| 3 | **面板订阅与事件域一致**（可见 ⇔ 投递，双向断言） | `domain_faces_open.rs`：`panel_visibility_and_event_delivery_share_one_domain_membership_probe` | 1 |
| 4 | 事件 `?session=` 跨域点名拒（404 逐字节同帧） | `domain_faces_open.rs`：`event_subscriptions_cannot_name_another_domains_session` | 1 |
| 5 | **审批跨域归属拒**（收件箱 + resolve；owner 亦无跨域通路；本域不误伤 → `domain_refused` 不代跑） | `domain_faces_open.rs`：`approval_faces_treat_cross_domain_sessions_as_missing` | 1 |
| 6 | **管理面全局 + 审计域标注**（全表不按域过滤；`user_add`/`domain_create` 行带目标域标注并进目标域视图） | `domain_faces_open.rs`：`admin_faces_stay_global_and_audit_rows_carry_the_target_domain_mark` | 1 |
| 7 | **默认域零回归**（五面全量读数） | `domain_faces_open.rs`：`default_domain_reads_stay_unfiltered_on_every_panel_face` | 1 |
| 8 | 自称域 403 恒定帧（存在与否同帧）+ 自称本域放行 | `domain_faces_open.rs`：`the_domain_claim_keeps_one_constant_403_frame_on_the_new_faces` | 1 |
| 9 | 只读口开 / 写 mate 与其余面仍拒（面闸表） | `domain_faces_open.rs`：`panel_write_mates_stay_closed_while_the_read_faces_open` | 1 |
| 10 | 数据面未开 = 403 `domain_refused` 拒开不给空视图 + 他域不牵连 | `domain_faces_open.rs`：`a_domain_without_an_open_session_plane_is_refused_not_given_an_empty_view` | 1 |
| 11 | 面闸开放面/关闭面词表 | `gateway/src/domain_context.rs`：`the_domain_surface_gate_opens_read_faces_but_keeps_write_faces_closed` | 1 |
| 12 | 审计行归属记号解析（session/域标注/无归属三分） | `domain_context.rs`：`audit_row_attribution_reads_the_session_and_domain_tokens` | 1 |
| 13 | 在册探测共享语义（fail-closed 三分） | `domain_context.rs`：`the_shared_scope_probes_domain_ledger_membership` | 1 |

面闸演进断言（随行注明理由）：`gateway/tests/user_domain_isolation.rs`
`non_default_domains_are_confined_to_the_domain_routed_surface` 的 `/v1/panel/sessions`
403 断言移除（V2-c 有意变更），换 `/v1/panel/traces`、`/v1/panel/skills` 收敛断言并补
演进注记。既有套件零回归口径：W3-① 全部、W3-② 角色门、W3-③ `domain_partner_chat.rs`
7 例、`multi_client_session.rs`、`events.rs`、`gateway_auth_face.rs`、`panel_*` 面等
既有面全量复测见 §6 门禁实测（整树 0 失败）。

## 6. 门禁证据与变更面

（全部 `-j 2`；取证脚本同宪章 §四。）

| 门禁 | 命令 | 结果 |
|---|---|---|
| 全量测试 | `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` | ✅ 退出码 0（整树 0 失败，含既有全量套件与 doc-tests；本批实测 2026-10-16） |
| Clippy | `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | ✅ 退出码 0，零警告 |
| 格式 | `cargo fmt --all -- --check` | ✅ 本批战场（`crates/adapters/gateway` 全包）0 diff；整树余 2 处 diff 均在**并行在途批**文件（`crates/capabilities/tools/src/self_status.rs`、`crates/engine/memory/src/{context_compiler,coordinator,retrieval_tuning}.rs`——本批禁区，零触碰），标红报备待归口线自检收口 |
| 中性表述 | `scripts/check-neutral-terms.ps1` | ✅ 零命中（43 changed files 全净，含并行在途批文件） |
| 密钥扫描 | `scripts/secret-scan.ps1` | ✅ 无泄漏（scan-staged 模式 18 pattern 过） |
| 本批套件复测 | `cargo test -p apeireth-gateway --test domain_faces_open` / `--test user_domain_isolation` / `--lib domain_context` | ✅ 10 / 7 / 3 例全绿（本批新增 13 例；`user_domain_isolation` 含面闸演进后的收敛断言） |

变更面（本批）：`crates/adapters/gateway/src/{domain_context.rs（行为域标注取值口 + 面闸表）,
panels.rs / task_panel.rs / events.rs / canonical_entry.rs（按域过滤/归属域校验/共享范围,
前任落盘）, client_tokens.rs（record_user_action 域标注参）, user_admin.rs / domain_admin.rs
（管理动作目标域标注接线）}`、测试 13 例（含 1 个新文件 `gateway/tests/domain_faces_open.rs`）。
文档：本文 + `docs/gateway-api-contract.md` §1 域行/§4/§5/§6/§7c/§8（契约随行）。

**残余（如实，不装完成）**：

| 残余 | 现状 | 归口 |
|---|---|---|
| 面板族写面按域（grants/revoke、tasks 人判重试、记忆写面） | 仍 403 `domain_forbidden` 面闸（只读口先开，写面随批） | 星群后续批 |
| 任务行/授权行的域归属标注 | 行无归属记号 → 域视图零行（fail-closed 已钉）；标注随任务队列/授权投影批补，补齐即按记号判定 | 任务/授权投影批 |
| 域开通/伙伴实例即时生效 | 仍启动期组装（W3-① 惰性开库残余不变） | 后续（惰性开库/实例） |
| 归档面（traces/审计/凭据库）逐域分库 | 仍共享数据根（W3-① 残余不变） | 后续批 |
| 域覆盖管理 UI | 「暂无接口」（`DomainRegistry::set_runtime_overrides` API 已备；D10 口径不变） | 管理面批 |
