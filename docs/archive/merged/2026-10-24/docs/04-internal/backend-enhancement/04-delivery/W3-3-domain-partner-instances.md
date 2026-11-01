# W3-③ 星群正身（逐域伙伴实例）交付记录

> `Status: ✅ 已交付（后端批 · 零前端触碰）`
> 一事：每户伙伴运行时实例（`DomainRuntimePlane`）+ 域级模型/性格人设覆盖（`users/<id>/domain.json`
> 兼容加字段）+ 域 chat 面开放（`/v1/chat`、`/v1/chat/completions` 走该域实例）+ 域事件订阅
> （`?domain=` 自称 + 按域过滤）。
> 范围口径：星群架构定案（**星群为骨、共享为翼**，[F-server-form.md](../F-server-form.md)）的运行时实例层；
> 纪律：[CONSTRUCTION-DISCIPLINE.md](../CONSTRUCTION-DISCIPLINE.md)；战场：`crates/engine/runtime/src/canonical/{domain_plane.rs,session.rs,mod.rs}`（运行时实例面）+
> `crates/engine/runtime-assembly/src/{domain_plane.rs,sqlite_session.rs}` + `crates/foundation/core/src/user_domain.rs`（域文档字段）+
> `crates/adapters/gateway/src/{domain_context.rs,canonical_entry.rs,events.rs,panels.rs}`（域路由/chat 开放/事件过滤）+
> `crates/adapters/cli/src/lib.rs`（组合根接线）。
> 诚实条款：本批落**运行时实例面 + chat/事件两面按域开放**；面板/审批/管理面仍按域闸（残余表 §7）；
> 域开通即时生效（惰性开库）与归档面逐域分库维持 W3-① 残余口径不扩。

## 1. 域实例架构（钉死）

```text
HTTP 请求
   │  Authorization: Bearer <令牌>
   ▼
令牌门 (W3-② 认人) → 角色门 (W3-② 判权) → 域门 (W3-① 判域, domain_context.rs)
   │   面闸 (W3-③): 非默认域开放 /v1/sessions/* + /v1/chat(/completions) + /v1/apeireth/events
   │                 其余面 403 domain_forbidden fail-closed
   ▼
DomainContext (域句柄) 随请求扩展下行
   │
   ├─ chat 面 (canonical_entry.rs)
   │    ├─ DomainRuntimePlane::runtime_for(域) ──▶ 该域伙伴 Runtime 实例
   │    │     默认域 = 组合根注入的现 Runtime (零变化)
   │    │     未注册实例 → 403 domain_refused (fail-closed, 不用默认域代跑)
   │    ├─ DomainRuntimePlane::context_for(域) ──▶ 模型/人设覆盖 (域文档现读, 坏域配置拒开)
   │    │     生效模型 = 请求显式 > 域 model > admin 热配 > 运行时默认 (缺省回落)
   │    │     会话种子 = 域 persona_text + 请求 system (人设缺省 = 请求 system 逐字回落)
   │    └─ 回合执行: 该实例的会话写闸 / 会话账本(配额) / 记忆库 全按域
   │         ├─ users/<id>/data/sessions.sqlite3  (会话 + 三维配额账本, 每域恰一把写闸)
   │         └─ users/<id>/data/cognitive.sqlite3 (该域记忆库, 每户独立记忆域)
   │
   └─ 事件面 (events.rs)
        ├─ ?domain= 自称域 (与 x-apeireth-domain 同语义, 不符 = 403 恒定帧)
        └─ 按域过滤: 只投递本域在册会话 (SessionManager::contains) 的事件帧
             他域/不存在同帧读作不存在; 无会话归属的进程级帧照常
```

**每户伙伴实例的组装**（`crates/adapters/cli/src/lib.rs`，组合根）：

```text
DomainDataPlane (W3-① 三面分库)
   └─ 每开一域: SessionManager(该域 sessions.sqlite3, 与数据面同一把写闸)
              + 认知模块组 (该域 cognitive.sqlite3: 记忆召回/写回/偏好/经历/自评…)
              → Runtime (该域伙伴实例; provider/治理/旋钮与默认域同款全局口径)
DomainRuntimePlane { 会话平面 + 默认域现 Runtime + BTreeMap<域, 伙伴实例> }
```

一户组装失败 = 该域不入实例面（chat 403 `domain_refused` fail-closed，审计
`gateway.domain_runtime_skipped` 留痕），其余域照常——一户坏配置不砖化整面（W3-① 同语义）。

## 2. 域文档扩展（StoredDoc 拒开 + 兼容加字段，钉死）

`users/<id>/domain.json`（信封 `user-domain` v1）body 增两个**可选**字段：

| 字段 | 形态 | 校验（越界 = 坏域配置拒开，不落盘） | 缺省语义 |
|---|---|---|---|
| `persona_text` | `Option<String>` | 1..=16_384 字符 | `None` = 回落全局（不注入人设种子） |
| `model` | `Option<String>` | 1..=128 字符、无首尾空白、无控制字符 | `None` = 回落全局默认模型 |

- **兼容加字段**：既有域文档（无此两字段）逐字可读（`serde(default)`），版本号不涨；
- **拒开不松**：`deny_unknown_fields` 保留——未知字段/类型错/越界值一律拒开（不回退默认）；
- **写入口**：`DomainRegistry::set_runtime_overrides(user_id, persona_text, model)`——严格打开
  域文档（坏域配置拒开）、只改两字段、`created_at`/角色/信封原样（不重建信封、不刷时间戳）、
  越界值拒开且文件零改动。人类管理面维持「暂无接口」口径（W3-① 残余表 D10 不变）。

## 3. 逐域运行时实例面（`DomainRuntimePlane`，engine/runtime）

| 面 | 语义 | 钉法 |
|---|---|---|
| 模型 | 域 `model` 覆盖 → 缺省回落全局（请求显式 > 域 > admin 热配 > 运行时默认） | `per_domain_runtime_isolation_holds_on_all_four_faces`（对抗改写）/ `one_domains_model_config_never_changes_another_domains_reply` |
| 记忆 | 该域伙伴实例的召回/写回只落该域 `cognitive.sqlite3`（分库之下他域记忆读作不存在） | `per_domain_runtime_isolation_holds_on_all_four_faces`（面②） |
| 配额 | 三维配额账本随该域会话账本（同 session id 两域互不串） | 同上（面③）+ W3-① 既有 `quota_ledgers_stay_independent_per_domain_for_the_same_session_id` |
| 写闸 | 每域恰一把 `SessionManager`（与域数据面**同一实例**）；本域持证不阻塞他域 | 同上（面④） |
| 实例 | 默认域 = 注入的现 `Runtime`（`Arc` 同一实例，零变化）；非默认域自己的实例；未注册 = 拒开不代跑 | `the_default_domain_serves_the_injected_runtime_with_global_fallbacks` / `a_domain_without_a_partner_instance_is_refused_not_redirected` |
| 上下文 | 域文档现读（persona/model），坏域配置 → `Refused`（本域真因，不动他域） | `one_domains_overrides_never_change_another_domains_context`（含一户坏配置不砖化） |

## 4. 域 chat 面开放（面闸演进，附理由）

- `DOMAIN_ROUTED_PATH_PREFIXES`：`/v1/sessions/` → + `/v1/chat`、`/v1/chat/completions`、
  `/v1/apeireth/events`（**演进理由**：W3-① 残余「非默认域 `/v1/chat` 等面 403 面闸」由本批
  星群实例兑现，逐面开放）；其余面（面板/审批/admin）仍 403 `domain_forbidden` fail-closed；
- `/v1/chat` 与 `/v1/chat/completions` 走同一域路由（同一 `DomainChatExecution` 解析）；
- **fail-closed 硬底线**：非默认域无伙伴实例（或域配置坏）→ 403 `domain_refused`，**绝不**拿
  默认域运行时代跑（代跑会把默认域会话/记忆/配额面泄给该域）；
- 默认域/域面未接线：请求模型/系统回落链与 W2/W3 既有语义逐字一致（零变化）；
- 会话立案：域面接线时回合开跑前把会话落本域账本（400 请求不立案），事件过滤自
  `turn_started` 起即有域归属。

## 5. 域事件订阅（`GET /v1/apeireth/events`）

- `?domain=<id>`：自称域查询面，与头 `x-apeireth-domain` **同一语义**（口径唯一）——不符 =
  403 `domain_forbidden` 恒定帧（目标域存在与否同帧）；
- 按域过滤（域面接线时）：只投递**本域在册会话**（`SessionManager::contains`，轻量存在性探测，
  不载入会话 JSON）的事件帧；他域/不存在的会话 id 同帧读作不存在（W3-① 同语义）；
  `?session=` 点名本域不在册会话 = 404 `session_not_found` 族（与会话面同语义）；
  无会话归属的进程级帧（生命周期等）照常；在册探测失败按不在册处理（fail-closed，宁拒不泄）；
- 域面未接线 = 全量订阅（兼容路径零变化）。

## 6. 测试证据（新增 15 例，全部为行为钉）

| 验收项 | 测试（文件） | 例数 |
|---|---|---|
| 域文档 persona/model 兼容加字段（旧文档可读/新字段往返/信封原样） | `core/src/user_domain.rs`：`domain_documents_accept_optional_runtime_overrides_and_keep_old_docs_readable` | 1 |
| 坏覆盖值拒开（空/超长/控制字符/类型错/未知字段，拒开不落盘） | `user_domain.rs`：`bad_runtime_override_values_refuse_to_open_without_touching_the_file` | 1 |
| 覆盖写入口（持久化/身份不改/清空回落/NotFound） | `user_domain.rs`：`runtime_overrides_clear_back_to_global_fallback_without_rewriting_identity` | 1 |
| 默认域 = 现运行时零变化（同实例 + 全局回落逐字） | `runtime/src/canonical/domain_plane.rs`：`the_default_domain_serves_the_injected_runtime_with_global_fallbacks` | 1 |
| 逐域伙伴实例 + 上下文（实例各归各/覆盖各归各） | `domain_plane.rs`：`each_domain_gets_its_own_partner_instance_and_runtime_context` | 1 |
| 跨域配置互不影响（对抗：连改 A 不动 B/默认域；A 坏配置不砖化 B） | `domain_plane.rs`：`one_domains_overrides_never_change_another_domains_context` | 1 |
| 未注册伙伴实例拒开不代跑 | `domain_plane.rs`：`a_domain_without_a_partner_instance_is_refused_not_redirected` | 1 |
| **逐域运行时隔离四面**（模型/记忆/配额/写闸，含对抗 + 跨域互不锁） | `runtime-assembly/tests/user_domain_isolation.rs`：`per_domain_runtime_isolation_holds_on_all_four_faces` | 1 |
| **域 chat 解禁全链**（域模型服务 + 会话落本域账本 + 配额记本域 + 他域查无） | `gateway/tests/domain_partner_chat.rs`：`non_default_domains_serve_chat_on_their_own_partner_runtime` | 1 |
| **跨域配置对抗**（改 A 的模型配置不改 B 的回复） | `domain_partner_chat.rs`：`one_domains_model_config_never_changes_another_domains_reply` | 1 |
| **域 persona 覆盖/回落**（人设在前 + 请求 system 保留；缺省逐字回落） | `domain_partner_chat.rs`：`domain_persona_seeds_the_turn_and_falls_back_verbatim_when_absent` | 1 |
| **域事件过滤**（本域帧投递/他域帧不投递） | `domain_partner_chat.rs`：`domain_event_subscriptions_only_receive_their_own_domains_frames` | 1 |
| `?domain=` 自称与域头同语义（恒定帧 + 自称本域放行） | `domain_partner_chat.rs`：`the_domain_claim_query_is_checked_like_the_domain_header` | 1 |
| **默认域零回归**（无覆盖 = 全局行为；显式模型仍最优先） | `domain_partner_chat.rs`：`the_default_domain_chat_surface_stays_on_the_injected_runtime` | 1 |
| **坏域配置拒开**（chat 403 `domain_refused` + 他域照常） | `domain_partner_chat.rs`：`a_broken_domain_config_refuses_the_chat_surface_without_touching_other_domains` | 1 |

面闸演进（附理由，§4）：`gateway/tests/user_domain_isolation.rs::non_default_domains_are_confined_to_the_domain_routed_surface`
的 `/v1/chat` 403 收敛断言移除（行为本批有意变更），并补一条同名面 fail-closed 断言：
无伙伴实例面的非默认域 chat = 403 `domain_refused`（不代跑）。

既有套件零回归：W3-① 全部 35 例、W3-② 角色门、`multi_client_session.rs`（`?session=` 语义
未接域面时逐字不变）、`events.rs`/`gateway_auth_face.rs`/`canonical_entry_e2e.rs` 等既有面
（未接域面 = 兼容路径零变化），见 §7 门禁实测。

## 7. 门禁证据与变更面

（全部 `-j 2`；取证脚本同宪章 §四。）

| 门禁 | 命令 | 结果 |
|---|---|---|
| 全量测试 | `cargo test --workspace --locked -j 2` | ✅ 退出码 0（整树 0 失败，含既有全量套件与 doc-tests；本批取证脚本一次复跑即全绿——中途一次红为本批自写断言缺 `content-type` 头（415 vs 403），当场修复复绿） |
| Clippy | `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | ✅ 退出码 0，零警告 |
| 格式 | `cargo fmt --all -- --check` | ✅ 干净 |
| 中性表述 | `scripts/check-neutral-terms.ps1` | ✅ 零命中（42 changed files 全净，含并行在途批文件） |
| 密钥扫描 | `scripts/secret-scan.ps1` | ✅ 无泄漏 |
| 本批套件复测 | `cargo test -p apeireth-core --lib user_domain` / `-p apeireth-runtime --lib domain_plane` / `-p apeireth-runtime-assembly --test user_domain_isolation` / `-p apeireth-gateway --test domain_partner_chat` / `-p apeireth-gateway --test user_domain_isolation` | ✅ 16 / 8 / 11 / 7 / 7 例全绿（本批新增 15 例） |

变更面（本批）：`crates/foundation/core/src/user_domain.rs`（域文档字段 + 覆盖写入口）、
`crates/engine/runtime/src/canonical/{domain_plane.rs（DomainRuntimePlane/DomainRuntimeContext）,session.rs（contains 探测/store 句柄）,mod.rs}`、
`crates/engine/runtime-assembly/src/{domain_plane.rs（记忆库路径）,sqlite_session.rs（轻量在册探测）}`、
`crates/adapters/gateway/src/{domain_context.rs（面闸演进 + 自称域校验抽取）,canonical_entry.rs（域回合目标解析/会话立案/域路由）,events.rs（?domain= + 按域过滤）,panels.rs（实例面注入 + 事件汇接线）,session_settings.rs（装配字面量补字段）}`、
`crates/adapters/cli/src/lib.rs`（逐域伙伴运行时组装 + 组合根接线）、测试 15 例
（含 1 个新文件 `gateway/tests/domain_partner_chat.rs`）。文档：本文 +
`docs/gateway-api-contract.md` §1 数据域行（契约随行）。
跨范围报备（宪章 §二.4）：本批 `DefaultDomainStores` 结构演进（增 `memory_db`）由并行在途批
在 `cli/lib.rs` 装配处按其报备口径补位（`cognitive_db_path()` 原位语义，正确）；门禁取证中途
整库 clippy 一度被并行在途文件 `gateway/tests/user_admin_endpoints.rs` 的 unused-variable 卡住
（非本批文件、非本批引入），其后由该批自行修好——本批全程零触碰该文件。

**残余（如实，不装完成）**：

| 残余 | 现状 | 归口 |
|---|---|---|
| 面板/审批/管理面按域路由 | 仍 403 `domain_forbidden` 面闸（逐面开放随批；chat/事件两面本批已开）——**✅ 已收（V2-c 批，2026-10-16）**：面板读面/审批面/管理面逐面开放，见 [V2-c-domain-faces-progressive-open.md](../V2-c-domain-faces-progressive-open.md) | 星群后续批 |
| 域级审批归属 | 审批仍按 W3-② 会话→用户归属判定；逐域审批面（域内审批归域伙伴）未做——**✅ 已收（V2-c 批，2026-10-16）**：归属域校验上线（跨域同帧 404），见 [V2-c-domain-faces-progressive-open.md](../V2-c-domain-faces-progressive-open.md) | 星群后续批 |
| 域开通/伙伴实例即时生效 | 平面与实例在启动期组装：新开域随下次启动生效（W3-① 惰性开库残余不变） | 后续（惰性开库/实例） |
| 归档面（traces/审计/凭据库）逐域分库 | 仍共享数据根（W3-① 残余不变） | 后续批 |
| 域覆盖管理 UI | 「暂无接口」（`DomainRegistry::set_runtime_overrides` API 已备；D10 口径不变） | 管理面批 |
