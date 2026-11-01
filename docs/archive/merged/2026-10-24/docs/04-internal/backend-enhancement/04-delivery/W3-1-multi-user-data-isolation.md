# W3-① 多户数据隔离（星群·数据层）交付记录

> `Status: ✅ 已交付（2026-10-13，后端批 · 零前端触碰）`
> 一事：用户域模型 + 单户兼容采用（零迁移）+ 三面（会话/记忆/配额）按域分库 +
> 跨域拒绝即帧（404/403 语义）+ 域上下文传递（请求→身份→user_id 域解析→按域路由）+
> 自述域节。
> 范围口径：星群架构的地基（星群为骨、共享为翼，[F-server-form.md](../F-server-form.md) 架构定案）；
> 纪律：[CONSTRUCTION-DISCIPLINE.md](../CONSTRUCTION-DISCIPLINE.md)；战场：`crates/foundation/core/src/user_domain.rs`（新增）+
> `crates/engine/runtime/src/canonical/domain_plane.rs`（新增）+ `crates/engine/runtime-assembly/src/{domain_plane.rs（新增）,canonical/self_status_source.rs,canonical/production.rs}` +
> `crates/capabilities/tools/src/self_status.rs` + `crates/adapters/gateway/src/{domain_context.rs（新增）,session_settings.rs}` +
> `crates/adapters/cli/src/lib.rs`（禁区零触碰，见 §7）。
> 接缝（W3-② 并行线，已兑现其报备）：`UserDomain.role` 由**域面建立**，治理层经
> `UserIdentity::from_wide(user_id, role, client_names)` **消费**（同一冻结词表
> `owner`/`member`/`guest`，接缝测试钉死）；`users.json` 的 `UserRecord.domain`
> 引用由本面**判域**（W3-② 只携引用）；自述域节接线兑现 W3-2 交付记录 §6 的报备注记。
> 诚实条款：本批只落**数据层**（域模型/隔离原语/域上下文/自述域节）；每户独立伙伴
> 实例（星群：逐域运行时与记忆人格）与逐域开放其余 API 面归星群后续批，见 §7 残余表。

## 1. 域模型（钉死）

```text
<数据根>/                              ← 默认用户域的数据面（原位，见 §2 采用语义）
<数据根>/users/<user_id>/domain.json   ← 每域一份域文档（StoredDoc 信封 `user-domain` v1）
<数据根>/users/<user_id>/data/         ← 非默认域的数据面（分库隔离硬）
    sessions.sqlite3                      会话库（会话 JSON + 三维配额账本）
    cognitive.sqlite3                     记忆库（episodes + 六流 + 侧库）
```

- **`UserDomain { user_id, display_name, created_at, role }`**（`crates/foundation/core/src/user_domain.rs`）：
  - `user_id`（`DomainUserId`）：1..=64 字节、字符集 `[a-z0-9_-]`、首字符小写字母或数字
    —— 路径拼接安全（`..`/分隔符/大小写混淆/控制字符天然被拒）；`default` 保留给采用域；
  - `role`（`DomainRole`）：**冻结词表** `owner` / `member` / `guest`（与治理层 `UserRole`
    同一词表，口径唯一；未知词面在域文档处即拒开，不默认降级）；
  - `created_at`：epoch 毫秒（非负）；`display_name`：1..=128 字符、无控制字符。
- **`DomainRegistry`**：域文档严格打开（`stored_doc` 单文档语义：畸形/串档/版本不符/
  `user_id` 与目录不符/字段越界一律**拒开**，不静默回退默认）、域创建（`users/<id>/data/`
  + 域文档，失败清自己刚建的空目录）、域 id 枚举（`users/` 下非域条目 = 坏配置拒开）。
- **域级授权**（`UserDomain::authorize`）：跨域点名 → `Forbidden`（目标存在与否同帧）；
  域内 `Data` 放行、`Manage` 需 `owner`（角色不足 → `Forbidden`，只说规则不指目标）。

## 2. 迁移语义（钉死：**标记为 default 域原位**，绝不丢数据）

升级到多户形态时，既有单户数据面（数据根原位的 `sessions.sqlite3` / `cognitive.sqlite3` /
记忆库 / 凭据库）**不移动、不复制、不改名、不删除**；首次启动（首次生产组装：CLI chat /
gateway serve）只写一个域标记 `users/default/domain.json`，把现有数据**原位标记**为默认
用户域（`user_id = "default"`，`role = owner`）。

为什么选「原位标记」而不是「归入 `users/default/data/`」（两案取一，理由钉死）：

1. **零迁移 = 零丢失风险**：不搬数据就没有搬丢的可能；「现有数据入 default 域零丢失」
   按构造成立，并有逐字节对抗断言（打开前后存量文件哈希不变）；
2. **升级平滑**：`APEIRETH_SESSION_DB` / `APEIRETH_COGNITIVE_DB` 覆写指向的既有文件
   逐字有效（默认域数据面 = 既有路径语义不变），旧版本二进制也可回读；
3. 幂等可重入：标记已在场则不动（`created_at` 永不被覆写）。

坏标记（畸形/串档/版本不符）→ 注册表**整体拒开**（fail-closed），数据原样不动——
「坏域配置拒开」，绝不静默降级成再采用。

## 3. 隔离原语与越权语义（404/403 钉死）

三面**按域分库**（每域自己的库文件 + 每域恰一把会话写闸；默认域与运行时**同一把**
—— W2-③「同会话账本写串行」承诺跨域面不拆）；配额账本随会话库分域（账本在会话 JSON 内）。

| 越权形态 | 判定 | 响应（钉死） | 钉法 |
|---|---|---|---|
| 资源级跨域（他域会话/记忆/配额 id）读 | 本域句柄查不到 | **404** `session_not_found` 族，与真不存在**同帧** | `cross_domain_session_reads_are_indistinguishable_from_missing`（同 id 两态逐字节同帧） |
| 资源级跨域写（改/删他域会话、写他域记忆） | 写落本域库，他域零改动 | 404 同帧 + 他域账本逐字节不变（对抗断言） | `cross_domain_session_writes_are_refused_and_leave_the_other_domain_untouched` |
| 域级点名（自称域头 `x-apeireth-domain` ≠ 本域） | 拒 | **403** `domain_forbidden` **恒定帧**（目标域存在与否同帧） | `a_claimed_domain_refusal_never_leaks_target_existence` |
| 角色不足（域管理类操作，非 owner） | 拒 | 403 `domain_forbidden`（只说规则不指目标） | `cross_domain_addressing_is_forbidden_regardless_of_target_existence`（域级恒定帧同款） |
| 面未按用户域开放（非默认域） | 拒 | 403 `domain_forbidden` 恒定帧（不出默认域数据） | `non_default_domains_are_confined_to_the_domain_routed_surface` |
| 本域配置坏/拒开 | 拒 | 403 `domain_refused`，真因入审计（`gateway.domain_refused`；本域信息，不涉他域） | `a_bound_user_whose_domain_config_is_broken_is_refused_with_domain_refused` |

**隔离矩阵（三面 × 层）**：

| 面 ↓ / 层 → | 数据层（域平面句柄） | 网关 HTTP 面 | 自述面 |
|---|---|---|---|
| 会话（`sessions.sqlite3`，含三维配额账本） | 每域分库 + 每域写闸；跨域读作不存在/写落本域 | `/v1/sessions/*` **按域路由**（跨域 id 与不存在同帧）；其余面对非默认域 403 面闸 | `domain.counts.sessions` |
| 记忆（`cognitive.sqlite3`） | 每域分库（账本 + 情节后端）；跨域读作不存在、写落本域 | 非默认域 403 面闸（面板记忆面本批不按域路由） | `domain.counts.memories/protected` |
| 配额（会话内 `CognitiveQuota`） | 随会话库分域；同 session id 两域账本互不串 | 随会话面路由 | `budget.quota`（随域会话库探测） |

## 4. 域上下文传递（请求 → ClientIdentity → user_id 域解析 → 按域路由）

```text
HTTP 请求
   │  Authorization: Bearer <令牌> (+ 可选自称名 x-apeireth-client / x-apeireth-domain)
   ▼
令牌门 require_gateway_token (W2-② 认人) → ClientIdentity
   ▼
角色门 require_user_capability (W3-② 判权; 角色面未激活 = 兼容路径)
   ▼
域门 require_domain_context (W3-① 判域, domain_context.rs)
   ├─ user_id 域解析: 令牌条目 user_id 绑定 (W3-② 链第一步)
   │    → 用户表 client_names 回落 → 无用户 = 默认域 (单户兼容)
   ├─ 域引用: 用户表 UserRecord.domain (W3-② 携引用, 本面判域) → 缺省 = user_id 同名域
   ├─ 域解析 (DomainSessionPlane::resolve): 未开域的绑定用户留默认域 (兼容延伸);
   │    域配置坏 → 403 domain_refused (坏域配置拒开, 不静默回落)
   ├─ 自称域头校验 (x-apeireth-domain ≠ 本域 → 403 恒定帧)
   └─ 面闸 (非默认域只开 /v1/sessions/*, 其余面 403 fail-closed)
   ▼
DomainContext 随请求扩展下行 → 会话面按域取写闸 (每域恰一把)
   ├─ GET/PATCH /v1/sessions/{id}/settings、DELETE /v1/sessions/{id}
   │    → 本域写闸 (默认域 = 运行时同一把); 本域查不到 → session_not_found (与不存在同帧)
   └─ 其余面 (本批): 默认域 = 既有语义逐字不变; 非默认域 403 面闸
```

## 5. 自述域节（`domain`，形状冻结合同 +1 节）

`self_status` 增 `domain` 节（合同演进，附理由：多户数据隔离的自述域面；`output_schema`
节表同步加 `domain`，节表总数以 `SelfStatusTool::output_schema` 为准，随并行批继续演进）：

```json
"domain": {
  "user_id": "default", "display_name": "Default", "role": "owner",
  "created_at": 1728000000000,
  "counts": {"sessions": 3, "memories": 12, "protected": 1, "reason": null}
}
```

- 「当前域」= 本自述来源组装的用户域的数据面（星群形态下每户伙伴实例自述自己的域），
  如实报告，不跨域代答；域身份每次现读域文档（篡改即拒）；
- 域内计数真值：会话数读本域会话库（与配额账本同库），记忆/受保护数读本域记忆库账本；
  单维缺位/失败 = 该维显式 `null` + `reason`（0 装诚实）；未接探测口 = 整节 `null` + 原因。

## 6. 测试证据（新增 35 例，全部为行为钉）

| 验收项 | 测试（文件） | 例数 |
|---|---|---|
| 域创建（数据面目录 + 域文档 + 保留 id/坏 id 拒） | `core/src/user_domain.rs`：`creating_a_domain_provisions_its_own_data_plane_and_marker`、`the_default_domain_id_is_reserved_and_bad_ids_are_refused`；`runtime-assembly/tests/user_domain_isolation.rs`：`a_created_domain_gets_its_own_three_store_files_under_its_data_dir` | 3 |
| 兼容迁移零丢失（现有数据入 default 域，对抗断言） | `user_domain.rs`：`first_launch_adopts_the_legacy_data_root_in_place_with_zero_data_loss`（逐字节哈希 + 只增域标记 + 三重对抗断言）、`adoption_is_idempotent_and_never_rewrites_the_marker`、`adoption_still_marks_the_default_domain_on_a_fresh_root`；`user_domain_isolation.rs`：`the_legacy_data_root_is_adopted_in_place_and_served_unchanged`（平面层：存量会话原样可读） | 4 |
| 三面跨域读拒/写拒（会话/记忆/配额） | `user_domain_isolation.rs`：`cross_domain_reads_and_writes_are_refused_on_all_three_planes`（读拒×3 + 显式点名 Forbidden + 写拒他域逐字节零改动） | 1 |
| 越权不泄露（404/403 语义） | `user_domain.rs`：`cross_domain_addressing_is_forbidden_regardless_of_target_existence`、`the_error_frame_semantics_are_pinned_to_404_and_403`、`loading_a_missing_domain_reads_as_not_found`；`user_domain_isolation.rs`：`cross_domain_refusals_hide_target_existence_behind_one_frame`；`gateway/tests/user_domain_isolation.rs`：`cross_domain_session_reads_are_indistinguishable_from_missing`（同 id 两态逐字节同帧）、`a_claimed_domain_refusal_never_leaks_target_existence`（403 恒定帧） | 6 |
| 跨域写拒（HTTP 面） | `gateway/tests/user_domain_isolation.rs`：`cross_domain_session_writes_are_refused_and_leave_the_other_domain_untouched` | 1 |
| 域上下文路由（多域并发互不串） | `user_domain_isolation.rs`：`concurrent_domain_traffic_never_cross_talks_between_domains`（3 域 × 8 会话×记忆并发）；`runtime/src/canonical/domain_plane.rs`：`resolution_without_a_bound_user_lands_in_the_default_domain`、`a_bound_user_resolves_by_provisioning_state_and_bad_configs_refuse` | 3 |
| 配额账本分域独立 | `user_domain_isolation.rs`：`quota_ledgers_stay_independent_per_domain_for_the_same_session_id`（同 session id 两域账本互不串） | 1 |
| 自述域节 | `user_domain_isolation.rs`：`the_self_status_domain_section_reports_the_current_domain_truthfully`；`tools/src/self_status.rs`：`a_partial_domain_counts_probe_stays_honest_per_dimension`（+ `every_section_arrives_with_real_values` 等 3 例合同测试演进：八节恒在场） | 2+3 |
| 坏域配置拒开 | `user_domain.rs`：`a_corrupt_default_marker_refuses_to_open_and_leaves_data_untouched`、`a_domain_marker_that_mismatches_its_directory_is_refused`、`domain_configs_with_bad_fields_are_refused_without_default_fallback`、`a_users_entry_that_is_not_a_domain_id_refuses_the_listing`；`user_domain_isolation.rs`：`a_bad_domain_config_refuses_only_its_own_plane_and_touches_nothing`；`gateway/tests/user_domain_isolation.rs`：`a_bound_user_whose_domain_config_is_broken_is_refused_with_domain_refused` | 6 |
| 单户路径零回归（既有套件 + 专测） | `user_domain_isolation.rs`：`a_single_user_composition_stays_on_the_legacy_paths_without_any_domain_setup`；`gateway/tests/user_domain_isolation.rs`：`an_unbound_client_stays_on_the_default_domain_legacy_surface`；既有 186 套件全绿（§7） | 2 |
| 坏绑定/非法域 id 拒开 | `user_domain_isolation.rs`：`an_invalid_bound_user_id_is_refused_even_when_everything_else_is_fine`；`domain_plane.rs`：`a_domain_without_an_open_session_plane_is_refused_rather_than_redirected` | 2 |
| 角色冻结词表 + 宽接口接缝（W3-② 消费面） | `user_domain.rs`：`the_role_vocabulary_is_frozen_to_the_three_shared_terms`；`gateway/tests/user_domain_isolation.rs`：`the_domain_role_feeds_the_governance_identity_wide_interface` | 2 |

既有套件零回归：`gateway_auth_face.rs`、`gateway_token_gate.rs`、`multi_client_session.rs`、
`user_authorization.rs`（W3-② 角色门 9 例原样）、`session_settings.rs` 单测（GET/PATCH/DELETE
行为不变，仅写闸多一层按域路由；未接域面 = 逐字旧行为）、`self_status_runtime_e2e.rs` /
`self_status_e2e.rs`（自述合同演进：新增 `domain` 节，既有字段零改动）等全绿。

## 7. 门禁证据与变更面

（全部 `-j 2`；实测时间 2026-10-14，同工作区另有并行在途批持续写 `crates/capabilities/tools/**`，
见表注。）

| 门禁 | 命令 | 结果 |
|---|---|---|
| 全量测试 | `cargo test --workspace --locked -j 2` | ✅ 退出码 0（21:00 整树实测：本批 35 新例 + W3-② 28 例 + 全部既有套件 0 失败；此后并行在途批（J1 内容流/技能面、对象级判定面）继续写 `tools/`，其在途编译错误属各自批收尾） |
| 本批套件复测 | `cargo test -p …`（core/runtime/runtime-assembly/gateway/tools） | ✅ `user_domain`（core）13 例 / `domain_plane`（runtime）4 例 / `user_domain_isolation`（runtime-assembly）10 例 / `user_domain_isolation`（gateway）7 例 / `self_status`（tools）24 例全绿 |
| Clippy | `cargo clippy -p apeireth-core -p apeireth-runtime -p apeireth-runtime-assembly -p apeireth-gateway -p apeireth-cli --all-targets --locked -j 2 -- -D warnings` | ✅ 零警告（21:15 实测；整库 clippy 在并行批在途文件收尾后由其批门禁同覆盖） |
| 格式 | `cargo fmt --all -- --check` | ✅ 本批改动文件全净；并行在途批文件存在其收尾中的机械差异（随其批 fmt 收口） |
| 中性表述 | `scripts/check-neutral-terms.ps1` | ✅ 零命中（73 changed files 全净） |
| 密钥扫描 | `scripts/secret-scan.ps1` | ✅ 无泄漏 |

变更面（本批）：`crates/foundation/core/src/{user_domain.rs（新增）,lib.rs}`、
`crates/engine/runtime/src/canonical/{domain_plane.rs（新增）,runtime.rs,mod.rs}`（`Runtime`
会话写闸改 `Arc` 形态 + `sessions_arc()` 注入缝：默认域写闸与域平面共用**同一实例**，
行为零变化）、`crates/engine/runtime-assembly/src/{domain_plane.rs（新增）,lib.rs,canonical.rs,canonical/{self_status_source.rs,production.rs}}`、
`crates/capabilities/tools/src/{self_status.rs,lib.rs}`、
`crates/adapters/gateway/src/{domain_context.rs（新增）,lib.rs,panels.rs,canonical_entry.rs,error_frame.rs,error_codes.rs,session_settings.rs}`、
`crates/adapters/cli/src/lib.rs`、测试 3 个新文件（35 例）。
跨范围最小改动（宪章 §二.4 报备）：`error_frame.rs`/`error_codes.rs` 增 `domain_forbidden` /
`domain_refused` 两码（契约随行）；`canonical_entry.rs` `truncate_label` 提 `pub(crate)`
（审计标签截断复用）；`session_settings.rs` 测试装配字面量补 `domains: None` 字段；
并行批接缝互保（同 W3-② 对本批的报备口径）：`self_status_source.rs` 曾为 J1 内容流节落
`None` 占位（J1 真接线随后无缝覆盖）、`cli/lib.rs` 装配字面量补 `self_status_content_feed: None`
占位（J1 探测接线归口）、`tools/src/lib.rs` 自述导出表补本批 `DomainCounts`/`DomainStatus`。
文档：本文 + [F-server-form.md](../F-server-form.md) 进度注 + `docs/gateway-api-contract.md` 错误码表。

**残余（如实，不装完成）**：

| 残余 | 现状 | 归口 |
|---|---|---|
| 每户独立伙伴实例（星群：逐域运行时/记忆人格/性格） | 未做；回合面 `/v1/chat` 对非默认域 403 面闸 fail-closed（宁拒不泄） | 星群后续批（W3-③ 起） |
| 面板/事件/审批/管理面按域路由 | 未做；非默认域 403 面闸（`DOMAIN_ROUTED_PATH_PREFIXES` 只开 `/v1/sessions/*`），逐面开放随批 | 星群后续批 |
| 域管理 CLI（域开通/改显示名/删域） | 暂无接口（`DomainRegistry::create` API 已备；`apeireth user add --domain D` 携域引用） | 后续管理面批 |
| 域开通即时生效 | 平面启动时开库：新开域随下次启动生效 | 后续（惰性开库） |
| 归档面（traces.jsonl / daemon-audit.jsonl / 凭据库 / keyring）逐域分库 | 仍共享数据根（默认域原位）；三面（会话/记忆/配额）已分域 | 后续批 |
| 授权角色权威 | `users.json` role = 授权判定权威（W3-② 契约，`apeireth user role` 管理）；`domain.json` role = 域模型记录（域面建立，自述报告 + `from_wide` 消费面）；两处同一冻结词表，开通流程应保持一致 | 口径已在 §1/§3 钉死 |
