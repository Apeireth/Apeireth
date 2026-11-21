# Apeireth Canonical Gateway — HTTP API 契约 v1

> 本文档是桌面端(及其他客户端)与 Apeireth canonical gateway 之间的**稳定契约**。
> UI/UX 团队据此开发界面;后端团队据此实现并保证不破坏。任何端点变更必须先改本文档。
>
> 状态:2026-09-04，当前 canonical runtime/assembly 实现。标注 `[已实现]` 的端点当前可用；能力是否真正可用以 `/v1/apeireth/capabilities` 为准。
> 响应形状与 `apeireth-ui/src/lib/types.ts` 对齐；`frontend/companion-desktop` 是 Tauri 薄壳。

## C2增补：受管文本成果与版本（2026-11-13）

`/v1/artifacts`族为默认域single_operator本地工作区提供权威成果版本服务，复用既有认证、角色与域闸；active用户表或非默认域未装配时失败关闭。精确路由、strict DTO、store身份、CAS/持久回执、容量、导出ETag及错误语义以[共同契约](design/text-artifact-versions.md)为准，实测状态见[进度](status/text-artifact-versions-2026-11-13.md)。

该族分页为每页1～32、正文最大256KiB，与下文旧面板的分页默认值分开。首批不接受任意文件路径，不编辑原导入Document，不把work_id当成服务端项目授权。agent的artifact_read/artifact_write与HTTP使用同一服务；agent写经原canonical治理与人工审批，人工保存不伪造工具审批。

2026-11-18 增补只读候选回合：`POST /v1/artifacts/:artifact_id/candidates` 只服务「按此版续做」的候选生成。请求为 strict JSON `{store_id,base_revision,base_sha256,instruction}`；服务端先只读加载明确已保存 revision，要求基准等于当前 head 且 SHA-256 相符，否则在调用 provider 前拒绝。instruction 上限 64KiB，正文+指令+8KiB 封装开销上限 512KiB。候选路线的拒绝沿用网关统一错误帧：HTTP 409/413/422，`error.code` 为 `invalid_request`，具体原因在 `error.details.artifact_error`，取值为 `stale_base_revision`、`base_digest_mismatch`、`candidate_envelope_too_large`、`candidate_turn_not_completed`、`candidate_invalid_output`、`candidate_tool_call_rejected`。

候选回合使用独立内部分会话与显式 request id；provider 不声明工具，模型若仍返回 tool calls 在派发/审批/transcript 写前回 422（`details.artifact_error=candidate_tool_call_rejected`），且不执行 behavior module/hook。成功 200 回执给出基准与生成后 head、`base_is_current`、`request_id`/`trace_id`、候选正文与 SHA、`tool_calls:0`、`artifact_writes:0`；候选不落库、不自动提交，head 在生成期间变化时只回 `base_is_current:false`。四级口径：后端路由与运行时拒绝为 PRODUCTION WIRED，实现:`crates/adapters/gateway/src/artifacts.rs`、`crates/engine/runtime/src/canonical/execute.rs`；私有前端尚未接线，未做 live E2E，不声明 HARDWARE VALIDATED。候选回合仍写内部分 Session/消息/事件用于审计，不承诺零会话写。

## C3增补：受管桌面模型连接（2026-11-19）

`APEIRETH_DESKTOP_MANAGED=1` 的 Windows 受管桌面入口新增 `GET/POST /v1/admin/model-connection`，用于读取/保存下次启动的单一 OpenAI-compatible 模型连接。GET 返回 `schema_version`、`provider`、`base_url`、`model`、`credential_state`（`missing`/`available`/`unavailable`，仅表示凭据状态）、`storage`、`endpoint_bound`、`restart_required`；POST 只收 strict `{base_url,model,api_key?}`，返回 `{schema_version,ok,restart_required}`，客户端须再 GET 核对。凭据只进 Windows Credential Manager；配置不热换 runtime；受管模式拒绝 `/v1/admin/config` 热替换（409 `invalid_request`），只装配该协议族 provider，并要求独立强制本机令牌、无额外用户/客户端档案的单一 default owner 操作者。

错误映射：400 `invalid_request`、403 `single_operator_required`、503 `profile_unavailable`/`credential_unavailable`、409 `endpoint_change_needs_key`。成果与候选服务对严格受管单操作者放行；其他用户表/客户端/非默认域仍 503 `scope_unavailable`。四级口径：PRODUCTION WIRED（代码在主干、Windows 隔离验收），非 HARDWARE VALIDATED；实现 `crates/adapters/gateway/src/model_connection.rs`、`crates/adapters/cli/src/desktop_model_connection.rs`，证据见[进度](status/desktop-managed-model-connection-2026-11-19.md)。

## 受管桌面能力配置

默认域的认证受管桌面操作者使用 `GET /v1/admin/desktop-capabilities` 读取
`{schema_version:1, revision, configured, effective, restart_required}`。
`configured` 和 `effective` 都包含三个布尔值：

| 字段 | 初始值 | 启用范围 |
|---|---|---|
| `project_file_tools` | `true` | 项目工作区文件工具。工作区修订、目录范围和逐次写审批仍由执行链核验。 |
| `preference_learning` | `true` | 从明确表述学习偏好。关闭后停止该模块学习，原有偏好记录和召回继续保留。 |
| `skill_search` | `false` | 普通对话可查找本机技能库条目。项目对话的文件工具范围由工作区合同决定。 |

`POST /v1/admin/desktop-capabilities` 接受严格的 `{expected_revision, configured}`，
两层对象都拒绝未知字段，三个配置值须显式提供。服务持久保存配置后返回同一读面结构。
修订不符返回 `409 capability_revision_conflict`。配置未变化时保留原修订。
`effective` 来自本进程实际装入的工具与行为模块，表示装配状态，逐次工具调用仍走原治理链。
`restart_required` 表示保存值与本次装配不同，正常重开后重新核对。现有任务继续沿用当前运行时。
配置文件 `desktop-capabilities.json` 仅由受管桌面后端加载，使用版本校验与原子写入。

## §1 通用约定

- Base URL:`http://127.0.0.1:{port}`(port 由桌面壳为 sidecar 动态分配;手动运行时默认 8080)
- 编码:UTF-8 JSON;时间戳统一 **epoch 毫秒**(`timestamp` 字段)
- 认证:**全 API 面令牌门**(W2-② 起,chat/事件流/面板/会话/审批/admin 族,唯一豁免 `/health` 探活)。三态口径(env `APEIRETH_GATEWAY_TOKEN` + 客户端令牌表 `APEIRETH_GATEWAY_TOKENS`/`client-tokens.json`,任一在场=强制态):强制态=`Authorization: Bearer <token>` 必须命中令牌族任一枚(恒定时间比较,缺失 401 `auth_missing_key` / 伪造 401 `auth_invalid_key`,拒绝入审计 `gateway.auth_rejected`,过门入审计 `gateway.auth_client client=<客户端名>`);两面皆空=放行但每次记警告帧 `gateway_auth_unprotected` + 审计 `gateway.auth_unprotected`(兼容路径,保守过渡);显式 `none`=操作者明示不设防,放行不告警但审计留痕(与客户端令牌表互斥,配置矛盾拒开)。桌面壳启动侧车时自铸 32 字节随机令牌注入(生产恒为强制态);客户端令牌表每客户端一枚(name→token,过门审计带客户端名)。非回环绑定(`APEIRETH_LISTEN`/`--bind`)强制令牌:令牌族两面皆空时拒绝启动(fail-closed)
- 授权(W3-② 起,角色门):用户表 `users.json`(StoredDoc 拒开语义)在场=**角色面激活**,过门客户端名沿令牌→用户→角色链(`client-tokens.json` 条目 `user_id` + 用户表 `client_names`)解析为问责身份,按治理层 role 权限矩阵过闸——写面(chat/会话写/记忆写,非读方法)需 `Write`、admin 族与面板族**管理面**(`/v1/panel/grants/revoke`、`/v1/panel/users`、`/v1/panel/domains`)需 `AdminControl`(owner 档)、`/v1/approvals/resolve` 需 `ApprovalL2` 且**跨用户审批拒**(owner 可批任意会话,其余角色只可批本人绑定会话的审批);读面(GET)全员可读。未绑定用户的受闸请求 fail-closed 拒(403 `permission_denied`);未配置用户表=兼容路径(W2-② 行为逐字不变,不新增拒绝面)。角色只来自令牌绑定的解析链,请求无法自称角色(自称客户端名另有 name↔token 绑定校验)。每次裁决落审计 `gateway.user_action user=<id> role=<role> action=<能力> endpoint=<路径> decision=allow|deny`。V2-c 起该审计行另带域标注 `domain=<id>` 记号(授权裁决=行为域,管理动作=目标域);`/v1/approvals/resolve` 在角色门之外先过**归属域校验**(V2-c,见下行)——他域会话的审批对 owner 亦同帧 404,跨用户归属闸只在同域内判定
- 数据域(W3-① 起,域门):请求沿令牌→user_id 链解析**用户域**(未绑定=默认域,单户兼容;未开域的绑定用户留默认域;域配置坏=403 `domain_refused` 拒开,真因入审计 `gateway.domain_refused`)。`/v1/sessions/*` **按域路由**(每域自己的会话库+配额账本;他域/不存在的会话 id **同帧** 404 `session_not_found`,存在性不泄露);W3-③ 起 `/v1/chat`、`/v1/chat/completions`(走该域**伙伴运行时实例**:模型/记忆/配额/写闸全按域;域文档 persona/model 覆盖缺省回落全局,`session_id` 落本域账本)与 `/v1/apeireth/events`(`?domain=` 自称域与域头同语义;按域过滤——只投**本域在册会话**的事件帧,他域/不存在同帧读作不存在)亦按域开放,非默认域无伙伴运行时实例=403 `domain_refused`(fail-closed,不用默认域代跑);**V2-c 起面板/审批/管理面逐面开放**:面板读面按域过滤返回——`/v1/panel/sessions`(本域账本直读,与 `/v1/sessions/*` 同一写闸同一账本)、`/v1/panel/memory/episodes`、`/v1/panel/graph`(行会话归属 ∈ 本域在册)、`/v1/panel/audit`(行归属记号 `session=<id>`/`domain=<id>`)、`/v1/panel/grants`、`/v1/panel/tasks`(行无域归属记号 = 域视图零行,计数按可见行重算,fail-closed),**面板行可见 ⇔ 事件帧投递**(与事件面共用同一在册探测,面板订阅与事件域永远一致);`/v1/approvals`、`/v1/approvals/resolve` 过**归属域校验**(他域会话的审批读/解与不存在**同帧** 404 `session_not_found`,owner 亦无跨域通路);users/domains 管理面(`/v1/panel/users|domains`、`/v1/admin/users*|domains*`)**保持全局**(owner 全局视角,不按域过滤)但管理动作审计带 `domain=<目标域>` 标注;面板族写面(`/v1/panel/grants/revoke`、`/v1/panel/tasks/*/human-judge-retry`)与其余面对非默认域 **403 `domain_forbidden`** fail-closed(面未按用户域开放,不泄露默认域数据)。可选自称域头 `x-apeireth-domain` 与本域不符=403 `domain_forbidden` **恒定帧**(目标域存在与否同帧)
- 错误模型:
  ```json
  { "error": { "code": "invalid_request | not_found | unsupported | runtime_error | auth_failed", "message": "人类可读信息" } }
  ```
  HTTP 语义:400 `invalid_request` / 401 `auth_missing_key` / `auth_invalid_key` / 403 `permission_denied`(W3-② 角色门)/ 403 `domain_forbidden` / `domain_refused`(W3-① 域门)/ `denied` / 404 `not_found` / 409 `conflict` /
  422 provider/runtime 语义失败 / 500 `internal_error` / 501 `unsupported` /
  503 `unavailable` / 202 审批挂起
  工作区面(A2 批, 见 §7g)的专用码:404 `workspace_not_found` / 409 `workspace_conflict` /
  403 `workspace_revoked` —— **新增码 = 合同变更**, 前端需同步;其余面的既有码语义不变。
  联系人面(B3a 批, 见 §7h)的专用码:404 `contact_not_found` / 409 `contact_conflict` /
  403 `contact_archived` / 409 `contact_binding_conflict` —— 同为**新增码 = 合同变更**。
  会话绑定面(B3b 批)的专用码:409 `agent_binding_rejected`(回合闸拒绝: 会话绑定的联系人/工作区
  已不可用)/ 409 `session_binding_conflict`(会话修订号冲突, 详情带 `expected_revision`/
  `actual_revision`) —— 同为**新增码 = 合同变更**。
  操作幂等面(C2-1 批)的专用码:409 `operation_conflict`(同一操作键 + 不同载荷)/
  409 `operation_in_progress`(同一操作键在飞) —— C2-1 起由 `/v1/chat` 的幂等键产生,
  语义与作用域见 §3 的「幂等」小节。
  同为**新增码 = 合同变更**, 前端需同步(封闭集共 24 个码, 顺序见
  `crates/adapters/gateway/src/error_frame.rs` 的 `ErrorCode` 枚举)。
  停止面(C2-2 批)**不新增码**: 404 `session_not_found`(无该回合) / 400 `invalid_request`
  (id 非法) / 409 `invalid_request`(该回合待批), 语义见 §11。
- 分页:列表端点统一 `limit`(默认 50,最大 500)+ 按时间倒序;需要更多分页时再引入 cursor

## §2 已实现端点〔四级口径：PRODUCTION WIRED〕

| 端点 | 方法 | 说明 |
|---|---|---|
| `/health` | GET | `{"status":"ok","execution_owner":"apeireth-runtime::canonical"}`(**无认证探活豁免**:探测刚需,令牌门不校验/不告警/不审计) |
| `/v1/models` | GET | OpenAI 风格模型列表 `{object:"list",data:[{id,object:"model",created,owned_by}]}` |
| `/v1/providers` | GET | 来自 live Provider Router 的 provider 状态与模型描述 |
| `/v1/runtime/snapshot` | GET | 不含凭据、私有记忆、原始 prompt/CoT 的 Runtime 诊断快照 |
| `/v1/apeireth/capabilities` | GET | 来自 GatewayServices、Runtime Registry 与 Provider Router 的动态能力清单 |
| `/v1/modules` | GET | 来自 BehaviorRegistry 的行为模块清单；`/v1/organs` 为兼容别名 |
| `/v1/chat` | POST | 原生对话(见 §3;C2-1 起支持 `x-apeireth-operation` 幂等键) |
| `/v1/chat/completions` | POST | OpenAI 兼容对话(支持 `stream:true` SSE;C2-2 起 chunk `id` = 运行时 request_id;C2-3 起流式回合在生产路径写入 C1 权威流) |
| `/v1/sessions/{session_id}/requests/{request_id}/stream` | GET | C1 规范流 SSE(见 §11;C2-3 起真实流式回合产生流数据;`Last-Event-ID`/`after_sequence` 续发, 超窗发 `resync_required` 后关闭) |
| `/v1/sessions/{session_id}/requests/{request_id}/stop` | POST | 停止在飞回合(见 §11;C2-2 起;幂等回放摘要, 不撤销已发生副作用) |
| `/v1/sessions/{session_id}/agent-binding` | GET | 会话绑定读面(见 §7i;B3b-1 起;未绑定 ⇒ 200 `bound=false`, 不是 404) |
| `/v1/sessions/{session_id}/agent-binding` | POST | 绑定/更新/显式解绑(见 §7i;B3b-1 起;必须带 `expected_revision`;`contact_id` 必带, 显式 `null` = 解绑) |
| `/v1/sessions/{session_id}/contact-session` | POST | 按登记创建空的联系人会话，或核对同一会话的原绑定（§7i） |
| `/v1/sessions/{session_id}/turns` | GET | 会话回合只读分页(见 §7j;`after` 绝对游标, `limit` 默认 8/最大 16, 单页 4 MiB 上限) |
| `/v1/sessions/{session_id}/turn-result` | GET | 按持久身份精确重读单个回合(见 §7j;`input_sha256` + 可选 `request`/`trace_id`; 歧义 409) |
| `/v1/approvals?session={id}` | GET | 该会话的待批审批(见 §6) |
| `/v1/approvals/resolve` | POST | 批准/拒绝审批(见 §6) |
| `/v1/panel/metrics` | GET | 指标聚合快照(见 §8b;会话+全局两层,空态显式空) |
| `/v1/panel/skills` | GET | 技能库元数据只读面(见 §7d;只回元数据不回卡体,坏卡列拒不砖库) |
| `/v1/panel/tasks` | GET | 任务面板只读面(见 §7e;一次快照读 = 任务行 + 时间线 + 六态计数) |
| `/v1/panel/tasks/{task_id}/human-judge-retry` | POST | 人判重试(见 §7e;failed 只能人判,唯一动作面) |
| `/v1/panel/care` | GET | 关怀规则只读面(见 §7f;care_list 帧同形 + 心跳两列) |
| `/v1/panel/traits` | GET | 性格成长账只读面(见 §7f;四滑杆现值 + 锁定态 + 最近移动,与 `tool.trait_report` 同账同源) |
| `/v1/panel/workspaces` | GET | 工作区列表(见 §7g;有界列表, 只回身份 + 安全展示名, 不回绝对根路径) |
| `/v1/workspaces/{workspace_id}` | GET | 单个工作区(见 §7g;不存在 = 404 `workspace_not_found`) |
| `/v1/workspaces` | POST | 建工作区(见 §7g;受管根由宿主决定, 请求体不含根路径) |
| `/v1/workspaces/{workspace_id}/select` | POST | 选择工作区(见 §7g;必须带 `expected_revision`) |
| `/v1/workspaces/{workspace_id}/close` | POST | 关闭工作区 = 吊销(见 §7g;必须带 `expected_revision`) |
| `/v1/agents/contacts` | GET | 联系人列表(见 §7h;默认不含已归档, `?include_archived=true` 才含) |
| `/v1/agents/contacts` | POST | 建联系人(见 §7h;身份由后端铸造, 请求体不得带 `contact_id`/`agent_id`) |
| `/v1/agents/contacts/{contact_id}` | GET | 单个联系人(见 §7h;不存在/不可见 = 404 `contact_not_found`) |
| `/v1/agents/contacts/{contact_id}` | PATCH | 改联系人(见 §7h;必须带 `expected_revision`) |
| `/v1/agents/contacts/{contact_id}/archive` | POST | 归档联系人 = 软删除墓碑(见 §7h;终态不可复活) |
| `/v1/panel/users` | GET | 用户管理列表(见 §7c;owner 专属) |
| `/v1/admin/users` | POST | 建户(见 §7c;owner 专属) |
| `/v1/admin/users/{user_id}` | PATCH / DELETE | 改角色 / 删户(见 §7c;owner 专属) |
| `/v1/panel/domains` | GET | 域列表+计数(见 §7c;owner 专属) |
| `/v1/admin/domains` | POST | 开域(见 §7c;owner 专属) |

## §3 对话 `/v1/chat`

请求:
```json
{ "session": "uuid 可选,缺省新建", "input": "用户输入", "model": "可选模型覆盖", "system": "可选系统指令(仅新会话生效)" }
```
响应(200 完成 / 202 审批挂起):
```json
{
  "session": "…", "request": "…", "trace_id": "…", "text": "助手最终文本",
  "served_by": "provider.minimax", "rounds": 1,
  "usage": { "input_tokens": 12, "output_tokens": 34 },
  "trace": { "entries": [ { "at": 123, "event": { … } } ] },
  "events": [ { "event": "tool_started", "tool_name": "tool.repo", "tool_call_id": "…", "round": 1 } ]
}
```
`/v1/chat/completions` 请求/响应遵循 OpenAI `chat.completions` 形状(`messages[{role,content}]`、`model`、`stream`)。

`/v1/chat/completions` 额外接受可选 `session_id`(UUID):缺省时后端新铸;流式新会话建议客户端预生成 UUID 并显式传入,使客户端在发起请求前就知道会话身份。首帧 SSE chunk 的 `id` 是运行时 request id,拿到后即可用 `GET /v1/sessions/{session_id}/requests/{request_id}/stream` 订阅 C1 权威流;响应或流式终帧的 `apeireth.session_id` 回显实际会话。

**流式契约**(`stream:true`,2026-09-10 起 token 级真流式,实测 210 帧 ~23ms 逐帧到达):
- 帧序列:`role` chunk → provider 逐 content delta 各一 chunk → 终帧(带 `usage` 与 apeireth 元数据)。
- 挂起审批以 `finish_reason:"approval_required"` 显式终帧终止流;provider 错误经统一错误帧契约上浮
  (`{"error":{"message","code","solution"}}`,见 ledger #16)。
- 增量链路:provider SSE → runtime sink(`execute_outcome_streaming`)→ gateway 逐 chunk 直通,非整段后分帧。
- C1 权威流(C2-3):网关在流式入口显式铸定 request id(同一 id 作为首帧起的 SSE chunk `id` 回给客户端)的回合,同一份 delta 同时写入运行时 C1 注册表
  (实时增量 + 结束时唯一终帧),可由 `GET /v1/sessions/{sid}/requests/{rid}/stream` 独立订阅/续发;
  终帧状态映射与边界见 §11。非流式回合或入口未铸定 request id 的回合不注册 C1 流(GET 仍 404)。

**幂等(C2-1)**:`/v1/chat` 接受请求头 `x-apeireth-operation`(单值, 头名大小写不敏感,
值去首尾空白后按 `validate_stable_id` 同规则校验:非空、≤128 字节、首字符为 ASCII 小写字母、
只含 `a-z0-9.-_`、以 `.` 分段且每段非空)。缺头即此前行为(每次都是新回合);头非法返回
400 `invalid_request`(文案含 operation key),且不落会话、不绑主、不入域。

- **作用域**:`(session_id, operation)`。请求必须显式给 `session`;未给 session 的请求不去重
  (每次新建会话)。幂等表在 Runtime 进程内, 重启即失, 多进程不共享。
- **载荷摘要**:SHA-256, 编码固定三段 `(input, model, system)`, 每段 = 1 字节存在位 +
  u32 小端长度 + UTF-8 字节;`model`/`system` 取解析后的实际值(与执行所用一致).
- **同键裁决**:同摘要 + 已完成 ⇒ 回放首次的终态响应(HTTP 状态与响应体同首次逐字节,
  不再执行、不再追加用户消息);同摘要 + 在飞 ⇒ 409 `operation_in_progress`;摘要不同
  (任意态)⇒ 409 `operation_conflict`;上次执行中断、结果不确定 ⇒ 同摘要也按 409
  `operation_conflict` 返回, 文案说明「上次执行中断、结果不确定, 请换新操作键重试」。
- **终态口径**:200 完成与 202 审批挂起都算终态并存储(202 回放 = 同一审批视图);
  RuntimeError 不算终态, 对应条目转为不确定态。
- **回放边界**:回放的是终态快照, 不重放流式 delta(C2-1 只接 `/v1/chat`;
  `/v1/chat/completions` 的 `stream:true` 不在本批范围)。表容量 1024 条, 淘汰只挑
  已完成/不确定态的最老条目, 在飞条目绝不淘汰(可瞬时超过 1024)。

**断连读回**：`POST /v1/chat` 经校验并取得执行容量后，由网关的独立任务保有该次请求，客户端断开只结束 HTTP 等待。每个路由器最多保有 128 个独立任务。等待容量的请求仍从属于 HTTP 连接。网关进程继续运行时，原回合沿用会话写闸、工具治理和幂等裁决。用原会话 ID、输入摘要、request 与 trace 读取持久结果，见 §7j。网关进程退出前尚未完成的回合仍可能是 `unconfirmed`。

**桌面流式交付**：`POST /v1/chat` 使用原 JSON 请求、访问令牌及可选 `x-apeireth-operation`。请求显式加 `Accept: application/x-apeireth-turn+ndjson`，校验和取得执行容量后返回 HTTP 200 与同一媒体类型。没有该协商标记时按原 JSON 合同返回。早于独立任务的拒绝仍用原 HTTP 状态与 JSON 错误帧，以下每帧均为一行 UTF-8 JSON。

| `kind` | 其他字段 | 含义 |
|---|---|---|
| `started` | `session`, `request` | 仅新执行回合发出。会话写闸和指定联系人核对通过后，进入运行时并使用这一请求身份。 |
| `delta` | `sequence`, `text` | 从 1 递增的实时增量，用于临时预览。队列约束或超长增量可造成跳号，客户端停止拼接并等待终帧。 |
| `final` | `status`, `body` | 本次原有 JSON 回执的状态和内容。包括已完成、待审批及失败。回放已完成操作或拒绝冲突只发终帧。 |
| `stopped` | `session`, `request` | 受理的停止操作产生真实停止摘要后，原执行任务收束时发出。 |

协商只改变交付形式。联系人、输入校验、会话写闸、安全上下文、工具治理、审批和操作去重沿用同一原生入口。`started` 在运行时登记停止身份前可能先到，调用客户端用本回合的请求身份执行 §11 的停止操作。模型可能在收到停止请求后完成当前输出，停止阻断后续工具动作，已发生的副作用保留。关闭流不会停止已受理的任务，使用 §7j 只读恢复。此模式只交付实时增量，兼容入口继续维护自身的 C1 重放流。流式审批仍走既有收件箱核对和显式决定。流结束而缺终帧时结果待核对。单行回执正文上限 4 MiB，超限时返回显式回执错误。

兼容模型端点若对流式请求返回成功的 `application/json`，使用同一次响应的完整答案及用量，完整文本一次呈现，额外模型请求数为零。

## §4 会话 `[P0]`

`GET /v1/panel/sessions?limit=50` →
```json
{ "sessions": [
  { "id": "…", "title": "…", "created_at": 123, "updated_at": 123, "message_count": 4, "revision": 0 }
] }
```

- `DELETE /v1/sessions/{session_id}` → 硬删除该会话的持久化记录,成功返回 `{"deleted": true}`;
  会话不存在返回 404 `session_not_found`,id 格式非法返回 400 `invalid_request`。
- **同会话并发写语义 (W2-③)**:同一会话的回合提交 / 审批续跑 / 设置补丁 / 删除走
  **同一把会话写闸**,先到先服务**排队**串行 —— 第二回合排队等第一个跑完(不发
  busy 帧),账本写永不交叠;排队事实(排位/等待)入审计行 `gateway.session_client`
  (`session=<id> client=<名> action=turn|approval.resolve|settings.patch|session.delete
  queued=<0|1> position=<n> wait_ms=<ms>`)。
- **name↔token 绑定 (W2-③ 令牌隔离)**:请求可带自称名头 `x-apeireth-client`;自称名
  必须与令牌绑定的客户端名一致,否则 401 `auth_invalid_key`(无绑定的自称名同样
  拒绝,fail-closed)。审计署名键**只来自令牌绑定**,请求自称名永不改写署名。
- **面板按域 (V2-c)**:非默认域读**本域账本直读面**(与 `/v1/sessions/*` 同一写闸
  同一账本),行与计数只含本域在册会话(A 域看不到 B 域计数);默认域/域面未接线
  = 全量返回(零回归)。

## §5 记忆

| capability | 端点 | 方法 |
|---|---|---|
| `memory.read` | `/v1/panel/memory/episodes?limit=&q=&session=` | GET |
| `memory.write` | `/v1/memory/append` | POST |
| `memory.forget` | `/v1/apeireth/memory/episodes/{id}/forget` | POST |
| `memory.protect` | `/v1/apeireth/memory/episodes/{id}/protect` | POST |
| `memory.unprotect` | `/v1/apeireth/memory/episodes/{id}/unprotect` | POST |
| `memory.graph.read` | `/v1/panel/graph` | GET |

episodes 响应:
```json
{ "episodes": [
  { "id": "…", "timestamp": 123, "role": "assistant", "content": "…", "session_id": "…",
    "category": "工作记忆", "importance": 0.7, "protected": false, "status": "active" }
] }
```
> rc 诚实边界:后端记忆 schema 不存 `category`/`importance` → 两字段**缺省**(可省略);
> `protected`/`status` 为网关级旗标(真实可用);`session` 必须是 UUID 格式;
> `timestamp` 已从底层 epoch 秒转换为契约的 epoch 毫秒。
append 请求 `{ "session": "UUID", "content": "…", "role": "可选 user|assistant(默认 user)" }` → 201 + 新 episode。
会话不存在时自动在会话账本创建(保证全局列表可达)。
forget/protect/unprotect 请求 `{ "expected_rev": 0, "reason": "可选" }` → 完整治理结果:
`{ "ok": true, "rev": 1, "id": "…", "status": "forgotten|active", "protected": false,
"revision": 1, "content": "…" }`。修订冲突返回 409 `conflict`；历史
`memory-flags.jsonl` 只在启动时幂等迁移到 `episode_governance`，迁移后不再参与判断。
graph 响应 `{ "nodes": [ { "id": "…", "label": "…", "kind": "session|episode" } ], "edges": [ { "from": "…", "to": "…", "weight": 1.0, "label": "可选" } ] }`。
(v1 语义:session → episode 包含边,源自真实存储数据)

面板按域(V2-c):episodes 行按会话归属过滤(行 `session_id` ∈ 本域在册才进域视图),
graph 会话节点在册才留、情节节点随本域会话的包含边留下;行无归属记号不进域视图
(fail-closed);默认域/域面未接线 = 全量(零回归)。

## §6 工具与审批

`GET /v1/tools/list` `[P0]` →
```json
{ "tools": [
  { "name": "tool.repo", "description": "…", "args_schema": { … } | null,
    "source": "builtin", "permission": "granted|prompt|none", "available": true }
] }
```

审批(已实现,四级口径: PRODUCTION WIRED——已进 gateway 路由表;形状确认):
- `GET /v1/approvals?session={id}` → `{ "session": "…", "approvals": [ { "approval_id": "…", "session": "…", "tool_name": "…", "capability_id": "…", "governance_hook": "…", "governance_reason": "…", "request": "…", "trace_id": "…", "created_at": 123, "expires_at": 123 } ] }`
- `POST /v1/approvals/resolve` 请求 `{ "session": "…", "approval": "…", "decision": "approve|reject", "reason": "可选" }` → 与 `/v1/chat` 响应同形状(解析后继续原回合)
- 跨用户审批(W3-②,角色面激活时):resolve 除 `ApprovalL2` 能力外还过**归属闸**——owner 可批任意会话的审批,其余角色只可批本人绑定会话的审批(会话归属=首个具名写者);跨用户/未归属/未绑定一律 403 `permission_denied`(审计 `gateway.user_action action=approval_ownership decision=deny`)
- 归属域校验(V2-c,先于上述归属闸):目标会话不在**本域账本在册**时,收件箱读与 resolve 一律与不存在**同帧** 404 `session_not_found`(跨域归属拒,存在性不泄露;owner 亦无跨域通路);本域过闸后 resolve 续跑走**本域伙伴运行时实例**(无伙伴实例 = 403 `domain_refused` 拒开不代跑)

## §7 授权管理 ✅

- `GET /v1/panel/grants` → `{ "grants": [ { "permission": "execute_tool:tool.repo", "capability": "tool.repo", "granted_at": null } ] }`
  (确定性顺序;`granted_at` 缺省——canonical policy 不记时间戳)
- `POST /v1/panel/grants/revoke` 请求 `{ "capability": "tool.repo" }` → `{ "ok": true }`(会话级热撤销,
  作用于运行时同一 policy 实例,进程重启后恢复默认策略;`capability` 支持工具名与
  `memory.read`/`memory.write`/`identity.modify`/`admin.override` 语义名)

### §7b 器官与行为模块清单 ✅

- `GET /v1/organs` → `{ "organs": [ { "id": "…", "name": "…", "enabled": true, "description": "…" } ] }`
  来源是当前 assembly 的 BehaviorRegistry；不再返回固定的 9 个静态未启用器官。
- `GET /v1/modules` → `{ "modules": [ ... ] }`，与 `/v1/organs` 共享当前行为注册状态。

### §7c 用户与域管理 ✅(2026-10-13, V2-b 域面开放与管理端点)

用户四口与域两口共用**同一个 UserTable 服务**(`apeireth_gateway::UserAdminService`,
数据目录 `users.json` 为唯一事实):CLI `apeireth user add|list|role|remove` 与 HTTP
**双入口同源**——引导口径 / 末 owner 保护 / 发令牌序一处实现,两入口互改互见。
全部端点在 W2-② 令牌门 + W3-② 角色门 `AdminControl` 档(**owner 专属**,含列表读口:
用户与域的管理数据非 owner 不可见)之内;服务未接线 = 501 `unsupported`(不假装有数据源)。

**端点合同表**:

| 端点 | 方法 | 请求体 | 成功响应 |
|---|---|---|---|
| `/v1/panel/users` | GET | — | 200 `{ "users": [UserAdminRow], "count": n }`(文件顺序) |
| `/v1/admin/users` | POST | `{ "user_id", "role", "client_names"?: [], "domain"?: "…" }` | 201 `{ "user": UserAdminRow }` |
| `/v1/admin/users/{user_id}` | PATCH | `{ "role" }`(只开改角色一门) | 200 `{ "user": UserAdminRow }` |
| `/v1/admin/users/{user_id}` | DELETE | — | 200 `{ "deleted": true, "user_id": "…" }` |
| `/v1/panel/domains` | GET | — | 200 `{ "domains": [DomainAdminRow], "count": n }`(id 序) |
| `/v1/admin/domains` | POST | `{ "user_id", "display_name", "role"?: "owner" }` | 201 `{ "domain": DomainAdminRow }` |

```json
{ "user_id": "user-owner", "role": "owner", "domain": "home",
  "client_names": ["desktop-shell"], "client_count": 1 }
{ "user_id": "alice", "display_name": "Alice", "created_at": 1728000000000,
  "role": "owner", "user_count": 2 }
```

- **UserAdminRow**:`user_id`(目录安全词面)/ `role`(冻结词表 `owner|member|guest`)/
  `domain`(域引用,可缺省=与 user_id 同名的域)/ `client_names`(绑定客户端名)/
  `client_count`(`= client_names.len()`,面板"客户端数"列)。
- **DomainAdminRow**:`user_id` / `display_name` / `created_at`(**epoch 毫秒**)/
  `role` / `user_count`(用户表中域引用落到该域的用户数,`domain` 缺省回落 user_id
  同名域;用户管理面未接线时**显式 `null`**,不编 0)。`count` = 本响应行数。

**错误模型**(与 §1 同一帧形):400 `invalid_request`(user_id/域 id 词面越界——复用
W3-① 目录安全规则 `[a-z0-9_-]`、首字符小写字母或数字,路径穿越 `../`/分隔符/大小写
混淆全拒;未知角色或角色大小写变体——冻结词表只认三字面,不就近映射)/ 404 `not_found`
(用户不在表)/ 409 `conflict`(重复 user_id、客户端名多属、末 owner 保护、令牌↔用户
绑定不一致)/ 500 `runtime_error`(坏 `users.json`/令牌表**拒开**——不回退空表)/
403 `permission_denied`(角色门:member/guest/未绑定)/ 401(令牌门)/ 501 `unsupported`。

**不变量**(与 CLI 面同一把闸):

1. **引导口径**:用户表为空时建户可入(表内首用户**必须 `owner`**,否则拒)——表非空
   后一切管理动作都要 owner;
2. **末 owner 保护**:最后一个 owner 不可删、不可降级(409);
3. **发令牌序**:令牌条目声明 `user_id` 的**先建用户再写绑定**;删用户前**先摘令牌
   绑定**(否则 409,问责链不得断);
4. **user_id 路径安全**:id 词面复用 W3-① `DomainUserId` 校验(user_id 缺省即域引用
   `users/<user_id>/` 目录名,不安全 id 一律拒);
5. **审计**:每次管理动作落一行
   `gateway.user_action user=<id> role=<role> action=user_add|user_role_change|user_remove|domain_create endpoint=<路径> decision=allow|deny`
   (角色门自身另有 `action=admin_control` 行;令牌值零泄漏)。V2-c 起行内另带域标注
   `domain=<目标域>` 记号(管理动作=目标域;失败行按意图目标/查表标注,未核实不标注;
   授权裁决行=行为域)——标注行随域视图审计面归属,管理面本身仍保持全局。

> 装载快照边界(诚实声明):管理动作**落盘即为事实**(每操作重读,双入口互见);
> 令牌门/角色门的**运行期**判定按 W2-②/W3-② 装载期快照语义——新建用户的绑定随
> 下次启动生效(与 CLI 改 `client-tokens.json` 的既有口径一致)。

### §7d 技能库只读面 ✅(2026-10-16, 技能书架读面微批)

`GET /v1/panel/skills` —— 技能库元数据列表(与 `skill_search` 同源只读: `<data_dir>/skills/*.json`,
每卡一 StoredDoc, 拒开语义原样沿用——坏 JSON / 串档 / 版本不符一律拒开并报结构化原因)。

- **只回元数据, 不回卡体**: 卡体是 L2 封套面(对话回合内按触发词披露的只读背景材料),
  读面只给七字段元数据; 响应**无 `body` 字段**。
- **坏卡如实列拒不砖库**: 打不开的卡进 `refused` 侧车(文件名 + 结构化原因), 好卡照常
  上屏——不砖化整个技能库, 也绝不「读坏用默认」。
- **一次快照读**(W2-③ 面板一致性): 响应内各字段同出一扫, 不拼两次读。
- 过 W2-② 全 API 面令牌门; 读面(GET)角色门全员可读(非管理面); W3-① 域门: 技能库是
  默认域数据, 非默认域请求按「面未按用户域开放」403 `domain_forbidden` 恒定帧(fail-closed)。
- 服务未接线 = 501 `unsupported`(不假装有数据源)。

响应(200):
```json
{
  "skills": [
    { "id": "skill-1f2e3d4c5b6a7988", "name": "发布前检查单", "summary": "一句话摘要",
      "triggers": ["发布", "上线检查"], "source": { "kind": "book", "title": "来源标题" },
      "created_at": 1728000000000, "version": 1 }
  ],
  "count": 1,
  "refused": [ { "file": "broken.json", "reason": "拒开原因 (结构化错误的显示文本)" } ]
}
```

- **SkillMetaDto**:`id`(稳定身份 `skill-{FNV-1a 64 hex}`, 卡名确定性派生)/ `name`(卡名,
  技能库内唯一键)/ `summary`(一句话摘要)/ `triggers`(已归一触发词)/
  `source{kind,title}`(来源, `kind` 冻结词表 `document|book|article`)/ `created_at`
  (**epoch 毫秒**, 版本演进不变)/ `version`(StoredDoc 信封格式版本, 拒开语义版本戳, 当前恒 1)。
- `count` = `skills` 行数(拒开数另计); `refused[] = {file, reason}` —— `file` 只回技能库内
  文件名(不回磁盘绝对路径), `reason` 为拒开结构化原因。
- 错误模型(与 §1 同一帧形):401(令牌门)/ 501 `unsupported`(服务未接线)/
  403 `domain_forbidden`(W3-① 非默认域面闸)。

实现:`crates/adapters/gateway/src/skill_shelf.rs`(DTO + `SkillShelfQuery` bounded-context
端口)、组合根适配 `crates/adapters/cli/src/gateway_panels.rs`(`CliSkillShelf` 读
`SkillLibrary::scan`)。前端 DTO 契约见 [frontend-data-contract.md](02-guides/frontend-data-contract.md) §10.8。

### §7e 任务面板面 ✅(2026-10-17, 任务面板进驾驶舱批)

`GET /v1/panel/tasks` —— 任务面板只读面(持久任务队列的面板投影, 与 `tool.task_board`
工具面同队同账);`POST /v1/panel/tasks/{task_id}/human-judge-retry` —— **人判重试**
(failed 只能人判, 失败绝不自动重试)。

- **面板边界(防越权)**: 本面只有两口——读一次快照 + 人判重试;任务的增/删/改派/
  完成走 `tool.task_board` 工具面(写面 = 人工审批档), **无对应路由**。
- **G 组核心(failed 只能人判)**: 人判重试是唯一动作面, 队列语义
  `human_judge_retry` 的 HTTP 投影(`Failed` → `Pending` + `retry_judged` 审计,
  回队待派**不自动唤醒**, 再派走派发面);非 `failed` 态 409 `conflict` 拒绝即帧。
- **一次快照读**(W2-③): `tasks[]` 行(含各自 `timeline` 事件账投影)与 `counts`
  六态计数同一刻读出, 不拼两次读。
- **署名只来自解析链**: 人判者 = 角色面绑定用户 > 令牌门客户端名 > `unattributed`;
  请求无法自称(与角色门同口径)。
- 过 W2-② 全 API 面令牌门(读与写同门);角色门: GET 全员可读, POST 需
  `UserCapability::Write`(guest 零写面)。
- 服务未接线(任务簿旋钮关 = 注册即闸) = 501 `unsupported`(不假装有数据源)。

读面响应(200):
```json
{
  "tasks": [
    { "id": "task-1", "title": "校对发布清单", "state": "failed", "attempts": 2,
      "assignee": "worker-a", "created_at_ms": 1728000000000, "updated_at_ms": 1728000000500,
      "payload": { "step": 3 },
      "timeline": [ { "kind": "failure_judged", "at_ms": 1728000000500,
                      "detail": "{\"judge\":\"owner\",\"reason\":\"验收不过\"}" } ] }
  ],
  "counts": { "pending": 1, "assigned": 0, "running": 0, "done": 0,
              "failed": 1, "blocked": 0, "total": 2 }
}
```

- **TaskPanelDto**:`id`(`task-<写派号>`, 面板上屏取短哈希)/ `title`/ `state`
  (六态冻结词表 `pending|assigned|running|done|failed|blocked`)/ `attempts`
  (派发执行次数)/ `assignee`(可空)/ `created_at_ms` / `updated_at_ms`(**epoch 毫秒**)/
  `payload`(不透明载荷, 面板给摘要)/ `timeline[]`(该任务的事件账投影:
  `kind` 冻结词表 + `at_ms` + 可选 `detail` 紧凑 JSON)。
- `counts` = 六态计数 + `total`(队列 `counts` 同口径真值)。

人判重试(POST, 请求体 `{ "reason": "…" }`, `reason` 可空——空则记显式缺省理由):
```json
{ "task": { "id": "task-1", "state": "pending", "…": "重试后的任务行 (同 TaskPanelDto)" } }
```

- 错误模型(与 §1 同一帧形):401(令牌门)/ 403 `permission_denied`(角色门 /
  人判面结构性拒绝)/ 404 `not_found`(任务不存在)/ 409 `conflict`(非 failed 态)/
  501 `unsupported`(服务未接线)。

实现:`crates/adapters/gateway/src/task_panel.rs`(DTO + `TaskBoardPanel`
bounded-context 端口)、组合根适配 `crates/adapters/cli/src/gateway_panels.rs`
(`CliTaskBoard` 附到持久任务队列, 附从句柄不重复崩溃恢复)。

### §7f 关怀/性格面板只读面 ✅(2026-10-17, 面板读面微批)

两口同批:`GET /v1/panel/care`(关怀)+ `GET /v1/panel/traits`(性格)。照技能库只读面
先例(§7d:形状 + 端口 + 组合根适配);过 W2-② 全 API 面令牌门;读面(GET)角色门全员
可读(非管理面);W3-① 域门:两面都是默认域数据(`care_rules.json` / `tuning-log.jsonl` /
`trait-locks.json` 在数据根),非默认域请求按「面未按用户域开放」403 `domain_forbidden`
恒定帧(fail-closed,同 §7d)。服务未接线 = 501 `unsupported`(不假装有数据源);读失败
(StoredDoc 拒开/坏档)= 500 `runtime_error` 报真因(错误即帧,不静默降级成空表)。
两面**零写面**:改规则走 `tool.care_schedule`(care_add/care_remove = 写文件同族,每调用
停人工审批),锁定/解锁走 CLI `apeireth trait lock|unlock`(显式动作)。

**`GET /v1/panel/care`** —— 关怀规则只读面(与 `tool.care_schedule` 同一 `care_rules.json`
同一拒开口径)。响应(200)= `care_list` 帧同形三节 + 关怀节真值:

```json
{
  "status": {
    "rules": 2,
    "next_trigger_at_ms": 1728000007200,
    "last_care_at_ms": 1728000000000,
    "last_care_note": "morning: spoke (rule_fallback): 早安问候",
    "heartbeat_next_poll_at_ms": 1728000000500,
    "heartbeat_last_hit_at_ms": 1728000000000
  },
  "rules": [
    { "id": "morning",
      "trigger": { "kind": "daily_moment", "minutes_of_day": 450 },
      "trigger_kind": "daily_moment", "material": "早安问候", "enabled": true,
      "created_at_ms": 1727000000000,
      "last_fired_at_ms": 1728000000000, "next_trigger_at_ms": 1728000007200 }
  ],
  "discipline": {
    "quiet_start_minutes": 1380, "quiet_end_minutes": 480,
    "frequency_max": 1, "frequency_window_hours": 4, "daily_max": 5,
    "focus_gap_minutes": 3, "catchup_minutes": 120
  },
  "last_care": { "rule_id": "morning", "at_ms": 1728000000000,
                 "judge": "rule_fallback", "spoke": true, "note": "早安问候" }
}
```

- **`rules` 行 = `care_list` 帧 `rule_json` 同形**(`id` / `trigger` / `trigger_kind` /
  `material` / `enabled` / `created_at_ms` 逐字同名同义;`trigger` = organ `CareTrigger`
  同一 serde 形状,内联 `kind` 标签,四类闭合词表)+ 两列真值补列:`last_fired_at_ms`
  (每规则发起锚,`null` = 尚无发起记录)、`next_trigger_at_ms`
  (`CareTrigger::next_occurrence_ms` 同口径真值)。
- **`discipline` / `last_care` = `care_list` 帧同形**(打扰纪律七字段 / `record_json`
  五字段;`quiet_*_minutes: null` = 显式关窗;`last_care: null` = 尚无关怀)。
- **`status` = `tool.self_status` 关怀节真值**(规则数 / 下次触发 / 最近关怀含判定文案,
  与自述关怀节**同一表体投影**)+ **心跳两列**:`heartbeat_next_poll_at_ms` /
  `heartbeat_last_hit_at_ms`(`null` = 心跳生产者未接线/未运行,不冒充在跑)。
- 一次快照读(W2-③):表体一次装载,各节同出一刻。

**`GET /v1/panel/traits`** —— 性格成长账只读面(与 `tool.trait_report` / 自述调参节
**同账同源**:同一本 `tuning-log.jsonl` + 同一份 `trait-locks.json`,同一套判据)。
响应(200):

```json
{
  "values": { "memory_fade": 1.25, "curiosity_strength": 1.0,
              "tone_saturation": 1.0, "consolidation_cadence": 1.0 },
  "locked": { "memory_fade": true, "curiosity_strength": false,
              "tone_saturation": false, "consolidation_cadence": false },
  "recent_moves": [
    { "param": "memory_fade", "label": "遗忘衰减强度", "at_epoch_ms": 1728000000000,
      "delta": 0.25, "reason": "上调一档" }
  ],
  "records": [
    { "seq": 1, "param": "memory_fade", "previous": 1.0, "next": 1.25,
      "reason": "上调一档", "at_epoch_ms": 1728000000000 }
  ],
  "skipped_records": 0
}
```

- `values` = 四滑杆现值(生效值:进程内自动调整 > env 旋钮 > 基线,`tool.self_status`
  调参节同源);`locked` = 四杆锁定态(`trait-locks.json` 真值;缺档 = 全解锁默认态)。
- `recent_moves` = 最近移动(新 → 旧,≤4 条;`trait_report::recent_moves` 同判据:
  零移动留痕与未来时刻不计移动)。
- `records` = 成长计既有账账面行(含零移动留痕行;周报 7 天窗同账投影的数据面);
  `skipped_records` = 账面坏行跳过计数(0 装诚实,不静默吞)。
- 锁定档坏档 = 500 报真因(绝不读成全解锁放行);一次快照读(W2-③)。

实现:`crates/adapters/gateway/src/care_panel.rs` / `trait_panel.rs`(DTO +
`CarePanelQuery` / `TraitPanelQuery` bounded-context 端口)、组合根适配
`crates/adapters/cli/src/gateway_panels.rs`(`CliCarePanel` 读 `care_rules.json` +
心跳真值口;`CliTraitPanel` 读成长计账 + 锁定档)。前端 DTO 契约见
[frontend-data-contract.md](02-guides/frontend-data-contract.md) §10.10 / §10.11。

### §7g 工作区面 ✅(2026-11-17, 后端接线合同 A2 批)

`GET /v1/panel/workspaces` —— 工作区列表(面板读面);`GET /v1/workspaces/{workspace_id}` —— 单个;
`POST /v1/workspaces` —— 建;`POST /v1/workspaces/{workspace_id}/select` —— 选择;
`POST /v1/workspaces/{workspace_id}/close` —— 关闭(= 吊销)。

- 建请求体接受 `display_name`、`root_kind`、`permission_scope`、`expires_at` 和可选的
  `root_provisioning`。省略 `root_provisioning` 时仍使用宿主原有的受管根
  （CLI 为进程当前目录）。提交 `root` 等未知字段返回 400 `invalid_request`
  (请求体 `deny_unknown_fields`)。`root_kind` 词表 = `managed|temporary|preset`
  (与前端 `governance_tab.rs` 的 span kind **不同名**, 后端口径以本行为准)。
- `root_provisioning: "isolated_v1"` 仅向经令牌认证的受管桌面单操作者开放。
  宿主在数据根的 `managed-workspace-roots/<workspace_id>/` 下新建实体目录，然后持久化登记。
  `display_name` 仅作展示，同名记录各有自己的目录。创建响应和随后读回的 `source` 为
  `desktop_managed`，`root_display` 为目录叶子名。该模式要求 `root_kind: "managed"`，
  `permission_scope` 接受 `read`（默认）或 `approval_write`（写入仍需批准）。
  关闭工作区会吊销登记并保留文件。旧版网关按未知字段返回 400，客户端应如实报告创建失败。
- **已有目录临时授权**：配套 Windows 原生桌面用系统选择器确认后，可从受管进程调用
  `GET/POST /v1/admin/desktop-folder-grants`。探测和发放均要求受管回环单操作者的
  Bearer 凭据及原生宿主从继承管道交给后端的独立凭据。浏览器来源和通用建工作区端点
  不能提交绝对路径。发放时比对原生选择和后端打开的目录句柄身份，返回标准
  `WorkspaceDto`，其中 `source=desktop_temporary`、`root_kind=temporary`、
  `permission_scope=read`、`expires_at` 为 epoch 毫秒。列表优先展示仍有效的临时授权。
  租约最长 30 分钟并随当前运行时退出失效。每次使用核对权限、到期时间、修订和目录身份，
  `/v1/workspace-chat` 对应会话只暴露根目录里的 UTF-8 文本读取。按原修订关闭后保留目录和文件。
  重启或到期后读回记录为 `unavailable`，显式撤销记录为 `revoked`。
- **读面不泄露根路径**: `root_display` 只含**叶子名**(无父目录/盘符/分隔符),
  响应体里没有绝对根路径;宿主若交回不安全的展示名, 网关 500 `internal`
  且**不回显该值**(宁拒不宽)。
- **乐观并发(选/关必须带修订号)**: `expected_revision` 缺失 = 400 `invalid_request`;
  与现值不符 = **409 `workspace_conflict`**(详情带 `expected_revision` /
  `actual_revision`),且**工作区逐字段不变**(网关层先比对再下发写动作, 领域层
  再原子复核一次)。
- **吊销 ≠ 冲突**: 已吊销的工作区再选/再关 = **403 `workspace_revoked`**
  (状态不可用, 不是并发冲突——分类写死在此)。
- 客户端提交请求作用域，由宿主确认后生效。沿用宿主原有受管根的创建请求在 CLI
  侧仍按 `write` 截断。`can_write` 表示登记允许写入，具体操作仍须经过管辖该操作的授权。
- **存在性不泄露**: 不存在 / 他人 / 他域 = 同帧 404 `workspace_not_found`;
  非默认域请求按面闸 403 `domain_forbidden`(恒定帧)。
- **可检索审计**: 建/选/关/被拒(冲突或吊销)都落审计行 `gateway.workspace_action`
  (`workspace=<id> action=workspace.create|workspace.select|workspace.close|workspace.deny
  endpoint=<路径> decision=allow|deny client=<署名> [reason=<词>]`),经
  `GET /v1/panel/audit` 可查;**详情里没有绝对根路径**。
- 过全 API 面令牌门(读与写同门);角色门: GET 全员可读, POST 需
  `UserCapability::Write`。
- 服务未接线 = 501 `unsupported`(不假装有数据源)。

列表读面(200):
```json
{
  "workspaces": [
    { "workspace_id": "6f1b0f6c-4f0a-4f6f-9a3a-1b2c3d4e5f60",
      "display_name": "校对发布清单", "root_kind": "managed", "root_display": "release",
      "permission_scope": "write", "revision": 7, "status": "ready",
      "can_select": true, "can_create_temporary": true, "can_write": true,
      "expires_at": 1800000000000, "source": "cli", "updated_at_ms": 1728000000000 }
  ],
  "count": 1
}
```

- **WorkspaceDto**:`workspace_id`(UUID)/ `display_name`/ `root_kind`
  (`managed|temporary|preset`)/ `root_display`(**安全展示名 = 叶子名**)/ `permission_scope`
  (`read|write|approval_write`)/ `revision`(根/作用域/撤销变化时递增)/ `status`
  (`ready|creating|revoked|unavailable`)/ `can_select` / `can_create_temporary` /
  `can_write` / `expires_at`(可空, **epoch 毫秒**)/ `source` / `updated_at_ms`
  (**epoch 毫秒**)。
- 列表 = `{"workspaces":[…],"count":N}`;单体 = `{"workspace":{…}}`。
- 建(POST, 受管桌面请求体示例
  `{ "display_name": "校对发布清单", "root_provisioning": "isolated_v1", "permission_scope": "approval_write" }`)
  → **201** `{"workspace":{…}}`。
- 选/关(POST, 请求体 `{ "expected_revision": 7 }`)→ **200** `{"workspace":{…}}`};
  关闭后 `status` = `revoked`、`can_select` = false、`revision` +1。

- 错误模型(与 §1 同一帧形):400 `invalid_request`(缺 `expected_revision` /
  未知字段 / `limit` 越界 / 独立目录创建请求无效)/ 401(令牌门)/ 403 `permission_denied`(角色或受管单操作者门)/
  403 `workspace_revoked` / 403 `domain_forbidden`(非默认域)/ 404 `workspace_not_found`/
  409 `workspace_conflict` / 500 `internal`(宿主目录或登记失败，或交回不安全展示名)/ 501
  `unsupported`(服务未接线)。

实现:`crates/adapters/gateway/src/workspace_panel.rs`(DTO + `WorkspacePanel`
bounded-context 端口 + 乐观并发比对 + 审计)、领域层
`crates/engine/runtime/src/canonical/workspace.rs`（`Workspace`、`WorkspaceRegistry` 与注册表）、组合根适配
`crates/adapters/cli/src/contract_panels.rs`（`CliWorkspacePanel` 持有持久化注册表及宿主受管根）与
`crates/adapters/cli/src/lib.rs`(网关装配点注入 `services.workspaces`)。
A2 批**不承诺持久化**(注册表为进程内实现, 持久化属 A3 批)。

### §7h 联系人面 ✅(2026-11-17, 后端接线合同 B3a 批)

`GET /v1/agents/contacts` —— 联系人列表(默认**不含**已归档);`POST /v1/agents/contacts` —— 建;
`GET /v1/agents/contacts/{contact_id}` —— 单个;`PATCH /v1/agents/contacts/{contact_id}` —— 改;
`POST /v1/agents/contacts/{contact_id}/archive` —— 归档(软删除墓碑, 终态)。

- **一张读面**: 只 `/v1/agents/contacts` 一族, **不另开** `/v1/panel/contacts` 别名。
- **身份由后端铸造**: 请求体带 `contact_id`/`agent_id`/`persona_revision`/`revision`
  ⇒ **400**(请求体 `deny_unknown_fields`)。
- **乐观并发**: 写面必须带 `expected_revision`;缺 ⇒ **400** 且**不调端口**;不符 ⇒
  **409 `contact_conflict`**(详情带 `expected_revision` / `actual_revision`)且
  **联系人逐字段不变**。
- **归档是终态, 不是冲突**: 已归档再写 ⇒ **403 `contact_archived`**;默认列表不含已归档
  (需 `?include_archived=true`);归档不可复活。
- **工作区绑定**: 写面可带 `workspace_id` + `revision`。服务端校验工作区存在、可选且修订号精确匹配。临时目录授权不能登记到联系人。
  校验失败返回 **409 `contact_binding_conflict`**，`details.reason` 为
  `workspace_not_found`、`workspace_revoked`、`workspace_revision_mismatch` 或 `workspace_unavailable`。
  绑定只保存 id 和修订。修改绑定后，历史会话保留原记录，继续交谈须新开会话。
- **存在性不泄露**: 不存在 / 不可见 = 同帧 404 `contact_not_found`(消息恒定
  「联系人不存在或不可见」)。
- **可检索审计**: 建/改/归档/被拒都落审计行 `gateway.contact_action`
  (`contact.create|contact.update|contact.archive|contact.deny`), allow/deny 都留痕,
  明细**不含路径与人设正文**。
- **能力清单**: `contacts.read`(list/get)与 `contacts.write`(create/update/archive)
  **分开登记**。

列表读面(200):
```json
{
  "contacts": [
    { "contact_id": "3b7a1f92-0c44-4a55-9f2e-8d1c6b0a5e31",
      "agent_id": "9f3d2c18-4a6b-4e77-8c21-5d0e7a9b2c48",
      "display_name": "校对发布清单", "subtitle": "发布线值班",
      "avatar_ref": null, "status": "ready", "persona_revision": 2,
      "memory_scope": "contact",
      "workspace_binding": { "workspace_id": "6f1b0f6c-4f0a-4f6f-9a3a-1b2c3d4e5f60",
                             "revision": 7 },
      "capabilities": { "read": true, "write": true, "approval_required": false },
      "persona": "发布线值班代理", "revision": 3, "updated_at_ms": 1728000000000 }
  ],
  "count": 1
}
```

- **ContactDto**:`contact_id` / `agent_id` / `display_name` / `subtitle` / `avatar_ref`
  (可空)/ `status`(`ready|draft|archived|unavailable`)/ `persona_revision` /
  `memory_scope`(`subject|contact|session|none`)/ `workspace_binding`
  (`{workspace_id,revision}` 或 null)/ `capabilities`
  (`{read,write,approval_required}`)/ `persona` / `revision` / `updated_at_ms`
  (**epoch 毫秒**)。
- 列表 = `{"contacts":[…],"count":N}`;单体 = `{"contact":{…}}`。
- 建(POST, 请求体
  `{ "display_name": "校对发布清单", "subtitle": "发布线值班", "persona": "…",
     "memory_scope": "contact", "workspace_binding": { "workspace_id": "…", "revision": 7 } }`,
  可省字段同上)→ **201** `{"contact":{…}}`。
- 改(PATCH, 请求体必须带 `expected_revision`)→ **200** `{"contact":{…}}`。
  `persona`、`capabilities`、`workspace_binding` 至少提供一项，所有指定字段与新联系人修订一并保存。
  省略 `workspace_binding` 保留当前绑定，显式 `null` 解除绑定。
  未修改的旧版联系人记录仍可读取。修改绑定的记录采用第 2 版格式，
  若需要使用早期版本运行时，先备份当前数据并恢复升级前数据。
- 归档(POST)→ **200** `{"contact":{…}}`;之后 `status` = `archived`。

- 错误模型(与 §1 同一帧形):400 `invalid_request`(缺 `expected_revision` /
  未知字段 / 自带身份)/ 401(令牌门)/ 403 `permission_denied`(角色门)/
  403 `contact_archived` / 403 `domain_forbidden`(非默认域)/ 404 `contact_not_found`/
  409 `contact_conflict` / 409 `contact_binding_conflict` / 500 `internal_error`/
  501 `unsupported`(服务未接线)。

实现:`crates/adapters/gateway/src/contact_panel.rs`(DTO + `ContactPanel`
bounded-context 端口 + 乐观并发比对 + 审计 + 能力入册)、领域层
`crates/foundation/core/src/agent_contact.rs`(B1 批: `Contact` /
`ContactRegistry` / `FileContactRegistry`)、组合根适配
`crates/adapters/cli/src/contact_panel.rs`(`CliContactPanel` 持真落盘注册表 +
宿主绑定档)与 `crates/adapters/cli/src/lib.rs`(网关装配点注入 `services.contacts`)。

**已接线（B3b-1/B3b-2/B3b-3，PRODUCTION WIRED）**: 会话绑定面见 §7i；已绑定会话的回合入口先过闸（未绑定放行，
绑定失效/注册表缺位 fail-closed 409 `agent_binding_rejected`，零写入），`memory_scope` 已按绑定
翻译成运行时作用域（persona/contact/session；缺装配面或失效 ⇒ 已绑定会话显式零写入，不回退
Session），命中 persona 在**空会话**的 system 种子槽注入（只改 system 文本，不碰权限/工具面/
workspace scope）。**边界**: 非空会话不追溯改 system；绑定变更写审计 `gateway.session_binding_action`
但不写 SessionEvent；注册表为进程内/单进程一致，多进程会分叉；无 live E2E。

### §7i 会话绑定面 ✅(2026-11-17, 后端接线合同 B3b-1 批)

`GET /v1/sessions/{session_id}/agent-binding` —— 读这张会话当前的 agent 绑定；
`POST /v1/sessions/{session_id}/agent-binding` —— 绑定 / 更新 / 解绑。只此一张面，不另开别名。

- **读面只回显已存事实**: 200 = `{session_id, bound, binding, session_revision}`；
  未绑定 ⇒ `bound=false` 且 `binding=null`（不是 404：会话在，只是没绑定）。
  `session_revision` 是会话当前修订号，也是下次写面要带的 CAS 值。
- **AgentBindingDto（9 字段）**: `contact_id` / `agent_id` / `persona_revision` /
  `workspace_id` / `workspace_revision` / `context_key`（可空）/ `incarnation`（可空）/
  `revision`（绑定自身修订号，不是会话修订号）/ `updated_at_ms`（epoch 毫秒）。
- **写面三态**: `contact_id` **字段缺失** ⇒ 400（不猜意图）；**显式 `null`** ⇒ 解绑
  （当前未绑定 ⇒ 400 `invalid_request`「无可解绑」）；**有值** ⇒ 绑定或更新。绑定时
  `agent_id` / `persona_revision` / `workspace_id` / `workspace_revision` 必填。
- **CAS 骑会话修订号**: 写面必须带 `expected_revision`（缺 ⇒ 400 且不碰会话）；不符 ⇒
  409 `session_binding_conflict`，`details{expected_revision, actual_revision}`。
  注意该值也会被普通回合 append 推进 ⇒ 客户端写前必须重读。
- **回显字段只透传**: `context_key` / `incarnation` 面既不生成也不改写；更新时省略即沿用
  已存值，给了不同值由领域层判冲突。
- **绑定要过注册表**: 联系人/工作区必须存在、未归档/未吊销，且 `persona_revision` /
  `workspace_revision` 精确匹配；失败按既有码回：
  404 `contact_not_found` / 403 `contact_archived` / 409 `contact_conflict`（人设修订号不符）
  / 404 `workspace_not_found` / 409 `workspace_revoked` / 409 `workspace_conflict`
  （工作区修订号不符）。注册表未装配 ⇒ 501 `unsupported`。
- **回合入口闸（B3b-1）**: 未绑定 ⇒ 放行（零回归）；已绑定但 registry 未装配、联系人已归档、
  或 `agent_id`/`persona_revision`/`workspace_id`/`workspace_revision` 任一不符 ⇒
  409 `agent_binding_rejected`，**零写入**（消息未追加、revision 未推进、sessions.save 未执行），
  且**不回落默认 Agent/默认域**。闸与 HTTP 面共用同一份 registry 实例（组合根按根目录缓存），
  否则会出现「面刚写的绑定，闸读不到」。
- **审计**: 建/改/解绑/拒绝都落 `gateway.session_binding_action`
  （`session_binding.create|update|unbind|deny`），allow 与 deny 都留痕，明细不含路径与人设正文。
- **能力**: `session_agent_binding.read` / `.bind` / `.unbind` 分开登记；过全 API 面令牌门。
- **边界**: 绑定变更**不写 SessionEvent**（只写审计）；registry 为进程内单例，多进程部署会分叉；
  初始面实现只覆盖单进程一致；无 live E2E（四级口径 PRODUCTION WIRED，非 HARDWARE VALIDATED）。

#### 桌面联系人空会话立案

`POST /v1/sessions/{session_id}/contact-session` 接受 `contact_id`、`context_key`、`incarnation` 和可选 `require_existing`（默认 `false`）。仅向经令牌认证的单操作者提供，包括通过模型连接入口所用严格单操作者核验的受管桌面。其他活动用户表和非默认域返回 403。运行时从已登记联系人解析 agent、人设、工作区和修订号，在同一会话写闸内新建并持久化空会话与绑定。未登记工作区返回 409 `contact_binding_conflict`，`details.reason=workspace_required`。动作本身没有消息或模型调用。

成功返回 200 `{schema:1, session_id, contact_id, context_key, incarnation}`。同一 `context_key`、联系人及绑定修订号再次打开仅核对原会话。回包中的 `incarnation` 是本次宿主请求的回显，持久绑定继续保存创建时的值。已有普通会话、其他联系人或过期绑定返回 409 `session_binding_conflict`。`require_existing=true` 时缺失会话也返回 409，`details.reason=contact_session_missing`。

原生 `POST /v1/chat` 接受可选 `expected_contact_id`。指定时必须提交显式会话，由同一写闸校验既有绑定，拒绝缺失、解绑或身份不同的会话。该模式仅提供文字对话，运行时关闭工具。联系人能力展示字段和工作区登记均不改变已有权限映射。兼容聊天请求沿用三段操作摘要，联系人操作摘要附带身份字段。

#### 桌面项目工作区会话

认证的受管桌面单操作者在默认域调用 `POST /v1/workspace-chat`。请求包含显式 `session` UUID、`input`、`workspace_binding` 对象，可带 `model` 和 `system`。`workspace_binding` 包含 `workspace_id` UUID 及 `revision`。通用 `/v1/chat` 保留旧客户端合同。桌面用专用路径发限域请求，旧版网关会拒绝该路径。

后端在空白项目会话上保存工作区绑定。后续回合必须带同一 ID 和修订号，改变绑定或从普通回合入口访问会被拒绝。工作区需要由 `isolated_v1` 创建并处于可用状态。`409 workspace_conflict` 表示原绑定无法确认。已完成的限域操作重放也要核对原会话和当前修订号。`x-apeireth-operation` 的摘要包含工作区身份和修订号，JSON 与原生 NDJSON 交付沿用 §3。

`read` 工作区的模型只收到 `project_files_read`。`approval_write` 工作区在会话权限允许时还可收到 `project_files_write`。支持根目录中按名称访问的 UTF-8 文本文件，最多 128 KiB。读取返回正文与内容摘要，缺少文件时返回 `revision: "missing"`。写入要求同一内容修订与当次人工批准，创建使用无覆盖发布，替换使用原子发布。凭据文件、链接和绝对路径不属于该文件面。

工作区撤销与文件操作共用登记锁。撤销后不能再读写或通过旧审批执行。已发给模型的回合可能返回，已完成文件保持原样。正常重启可重新核对原工作区和仍有效的待批回合。

### §7j 会话回合只读恢复面 ✅(2026-11-19, 补充交接 P0)

`GET /v1/sessions/{session_id}/turns` 与
`GET /v1/sessions/{session_id}/turn-result`：只从该会话已持久化的日志恢复既有回合，
不执行回合、不调用模型/工具、不写消息、不推进 revision、不产生审批。

**turn-result 选择器**

- `input_sha256` 必填，必须是 64 位小写十六进制；`request` / `trace_id` 可选，必须是规范 UUID。
  未知 query 字段 ⇒ 400 `invalid_request`。
- 选择只匹配**本会话自己的**回合：必须同时有相邻的原始用户输入与可验证的完成标记；
  继承前缀（fork/复制会话）里的回合不冒充子会话执行。没有 latest-turn 回落。
- 同一摘要命中多个回合且未给 `request` ⇒ 409 `invalid_request`（歧义）；
  给了 `request` 但该回合的输入摘要不同 ⇒ 409；补充 `request` 后必须恰好命中一个回合；
  `trace_id` 只用于交叉核对，不参与选择，不一致同样 409。查不到会话/回合 ⇒ 404 `session_not_found`。
- 200 帧：`{schema:1, source:"session_log", session, input_sha256, state, request, trace_id, text, usage, recorded_at?}`。
  `state=completed` 时 `text` 是该回合的最终助手文本原文（含原空白与 Unicode），`recorded_at` 为完成时刻毫秒；
  `state=unconfirmed` 时没有可验证完成，`text=null` 且省略 `recorded_at`。
  `usage` 恒为 `null`：旧日志没有可复核的原始用量，服务端不从文本反推。

**turns 分页**

- `after` 是绝对日志游标（默认 0，不得超过当前日志长度）；`limit` 默认 8，允许 1–16。
  未知 query 字段或越界值 ⇒ 400 `invalid_request`。
- 200 帧：`{schema:1, source:"session_log", session, revision, turns, next_cursor, has_more}`；
  每个 `turns[]` 含 `cursor`（该回合 `TurnStarted` 的日志下标，写入后不变）、`input_sha256`、`input`、
  `request`、`trace_id`、`state`、`text`、`usage`、`recorded_at?`。
- `completed` 给最终助手原文；`unconfirmed` 给 `text=null`，同一 cursor 在后续页可刷新为 completed。
- 单页序列化上限 4 MiB，按整回合计入：放不下就停在本页并把 `has_more=true`，**不截断正文、不跳过回合**；
  单个回合自身超过上限 ⇒ 413 `invalid_request`。
- 日志镜像/提交标记不自洽、回合缺相邻输入或完成窗口含他回合事实 ⇒ 409 `invalid_request`（不可验证），
  不猜测、不降级成 legacy 相邻消息。

**认证、归属与保护**

- 两条 GET 都过全 API 面令牌门；用户表启用时必须绑定用户（未绑定 403 `permission_denied`）；
  非 owner 只能读自己名下会话，跨用户与不存在统一 404（不泄漏存在性）。
- 域视图按域管理器读取；读取是会话快照，**不取回合写 permit**，不排队等待正在执行的回合。
- 响应（含 401/403/404 拒绝帧）统一加 `Cache-Control: no-store` 与 `X-Content-Type-Options: nosniff`。
- 错误码：400 `invalid_request`（选择器/分页/不可验证）、403 `permission_denied`、
  404 `session_not_found`、409 `invalid_request`（歧义/身份冲突）、413 `invalid_request`（超出页预算）、
  500 `internal`（会话记录不可读）。

**边界**：只读恢复不重发模型、不重放工具、不补审批、不返回原始 CoT；
`usage` 与 provider 原始回包不可从日志重建，宁可为 `null` 也不伪造；
可行范围取决于会话日志本身，进程内会话存储重启即失，持久会话存储可跨重启读取。

## §8 Trace / Audit / 事件总线

- `GET /v1/panel/traces?limit=20` `[P0]` → `{ "traces": [ { "trace_id": "…", "span_count": 3, "root_span": { … }, "started_at": 123 } ] }`
- `GET /v1/panel/traces/{trace_id}` `[P0]` →
  ```json
  { "trace_id": "…", "spans": [ { "span_id": "…", "parent_span_id": "…|null", "kind": "turn|provider|tool|governance", "actor": "…", "status": "ok|error", "summary": "…", "started_at": 123, "ended_at": 123, "session_id": "…" } ] }
  ```
- `GET /v1/panel/audit?limit=100` `[P1]` → `{ "events": [ { "ts": 123, "event": "chat.turn.completed", "service": "runtime", "detail": "…" } ] }`
  由 RuntimeEventSink 的审计端口归档；网关不再从响应结果反推一套 turn 事实。
  面板按域(V2-c):审计行按归属记号过滤(`session=<id>` 走本域在册探测、
  `domain=<id>` 与本域比对),无归属记号行不进域视图(fail-closed);在册探测与
  事件面同一把(面板订阅与事件域一致)。
- `GET /v1/apeireth/events` ✅ — Runtime Event Spine 的 SSE 投影:
  `backend_ready`(启动,仅广播一次)/ `turn_started` / `turn_delta` / `turn_completed` /
  `approval_required` / `approval_resolved` / `presence_state`(见下)。
  帧格式:`event: <name>` + `data: <json>`;15s keep-alive;容量 256,慢订阅者被断连(不无限缓存)。
  > 诚实边界:`turn_delta` 是**生命周期镜像**——canonical 运行时在事件总线编码前整轮已收口,
  > 所以它携带最终全文作为单条增量,总线上观测不到 token 级增量。
  > **逐字渲染请走 `POST /v1/chat/completions` 的 `stream:true` SSE**(token 级真流式,见 §3)。
  > 事件在进程内广播,不跨重启持久。

  **会话订阅 (W2-③)**:`GET /v1/apeireth/events?session=<id>` = 逐客户端独立的会话
  订阅 —— 该连接只收这个会话的事件帧(同一会话的事件扇出到它的全部订阅端,多端
  同会话各收各的),连接快照帧也只读该会话的最近一帧;会话 id 非法返回 400
  `invalid_request`。订阅动作落审计行 `gateway.session_client`
  (`action=events.subscribe`)。不带 `session` 参数 = 全量订阅(既有语义不变)。

### §8a presence_state 事件 ✅(2026-09-22,契约来源 `docs/design/00-PHILOSOPHY.md` §10 草案 v0)

`presence_state` 是「一份契约 × 四个投影」的地基事件:后端把对"他当前状态"的粗估值
送上同一条 SSE 总线,四个前端投影(T0 余烬点 / T1 光晕 / T2 光环 / 桌宠)显影同一份状态。
帧:`event: presence_state`,`data` 形状:

```json
{
  "type": "presence_state",
  "at": 1726992000000,
  "pad": { "p": 0.2, "a": -0.1, "d": 0.3 },
  "dominant": "serene",
  "intensity": 0.38,
  "stance": "attentive_presence",
  "breath": { "period_secs": 4.0, "amplitude": 0.6 },
  "significance": "turn",
  "source": { "kind": "heuristic_v0", "confidence": 0.5 }
}
```

- `at`:epoch 毫秒(与全契约时间戳一致)。
- `pad`:pleasure/arousal/dominance 三通道粗估值,各 ∈ [-1, 1];`dominant` 是从 PAD 推导的
  离散标签(`engaged`/`serene`/`strained`/`subdued`/`neutral`),`intensity` ∈ [0, 1] 为整体强度。
- `stance`:四姿态,线上值为小写下划线串,枚举复用 `ember_hud_driver.rs` 的 `EmberCognitiveStance`:
  `deep_coding_focus` / `attentive_presence` / `dreaming_consolidation` / `empathetic_care`。
  > v0 诚实边界:`empathetic_care` 在总线上没有诚实的触发信号,heuristic_v0 **永不产出**它;
  > 它留在枚举里是为未来情感引擎预留的契约空间,不是谎报现役能力。
- `breath`:呼吸节律参数,`period_secs` 固定 4.0(生理呼吸节律),`amplitude` ∈ [0, 1] 随强度变化。
- `significance` 显影三档:`heartbeat`(只动余烬点)/ `turn`(动光环/姿态)/
  `ritual`(仪式级,月频罕见,允许全屏金脉冲)。**显影必须稀缺才有效**;
  v0 没有诚实的 ritual 触发器,**永不产出 `ritual`**。
- `source.kind` 恒为 `heuristic_v0`,`confidence` 反映启发式的不确定度(当前固定 0.5):
  v0 不承诺精度,只承诺语义方向真实;未来器官/情感引擎落地后换 `source.kind` 即可平滑升级,
  契约形状不变。

**生产时机(heuristic_v0)**:回合结束(轮次时长/工具调用数/审批频率 → PAD 粗估值;
回合失败负向微调)、长时无交互(心跳拍上 PAD 向 baseline 衰减,30 分钟无交互 stance 落入
`dreaming_consolidation`)、记忆召回命中(正向微调——仅当召回以**总线上可见的能力调用**
形态发生时,即 capability id 含 `recall`/`memory` 的 dispatched 工具;canonical 装配里
`MemoryRecallModule` 的 prompt-overlay 召回路径目前**不上** RuntimeEvent 总线,v0 观测不到它,
此处不谎报)。

**频率纪律**:回合级事件(回合结束一拍)+ 低频心跳(恰好每 60s 一拍),无高频推送。
initiative(他从 `dreaming_consolidation` 苏醒主动开口)是独立事件类型、不在本帧内;
其"单用户日常 ≤ 少量次/天"的限量纪律由后端 presence 模块的预算器强制执行(默认 ≤3 次/天,
高信号才开口,宁少勿多),v0 尚无 initiative 生产者,预算器先行落地防唠叨。

### §8b 指标面板 `GET /v1/panel/metrics` ✅(2026-10-12, G-P1 指标数据源)

回合收束指标的**进程内滚动窗口**聚合快照(数据面全量口径见
`docs/04-internal/backend-enhancement/G-observability.md` 的字段表)。过全 API 面令牌门。

```json
{
  "schema_version": 1,
  "window_turns": 50,
  "retention": "in_process_rolling",
  "global": {
    "turns": 3, "succeeded": 2, "failed": 1, "pending_approval": 0,
    "latency_ms": { "count": 3, "mean": 12.0, "p50": 10, "p95": 30, "sum": 36 },
    "input_tokens": { "count": 2, "mean": 100.0, "p50": 100, "p95": 120, "sum": 200 },
    "output_tokens": { "count": 2, "mean": 20.0, "p50": 20, "p95": 24, "sum": 40 },
    "rounds": { "count": 3, "mean": 1.0, "p50": 1, "p95": 2, "sum": 3 },
    "tool_calls": { "count": 2, "mean": 1.0, "p50": 1, "p95": 2, "sum": 2 },
    "cache_hit_tokens": null, "cache_hit_rate": null, "cost_micros": null
  },
  "sessions": [ { "session_id": "…", "aggregate": { …同 global 形… } } ]
}
```

- 分布列 = 均值/最近秩 P50/P95/计数/累计和,只统计有值的收束拍;
- **空态 = 显式空 + "—"**:无数据时 `sessions: []`、计数 `0`、分布数值列 `null`、
  缓存/计价列 `null`——消费端渲染 "—",未知不用 0 冒充;
- `cache_hit_tokens`/`cache_hit_rate` 当前恒 `null`(usage 计量链尚无缓存列,见
  G-observability 缺口登记),`cost_micros` 为可选计价列(换算归花费线)。

## §9 能力清单 `[P0]`

`GET /v1/apeireth/capabilities` → 动态 CapabilityManifest(替代前端静态 release contract):
```json
{ "schema_version": 1, "runtime": { "service": "apeireth-gateway-2.0", "version": "2.0.0-rc.1" },
  "capabilities": [
    { "name": "memory", "capabilities": [ { "id": "memory.read", "supported": true, "read": true, "write": false, "operations": ["list","search"], "available": true } ] }
  ] }
```
每个条目同时提供 `supported`、`available` 与不可用时的 `reason`。当前核心 ID 包括:
`health`、`models.list`、`providers.list`、`runtime.snapshot.read`、`chat.completions`、
`sessions.read`、`memory.read/write/forget/protect/unprotect`、`memory.graph.read`、
`memory.update`（`supported=false`, `available=false`, `reason=not_implemented`）、
`tools.list`、`skills.read`（技能库元数据只读面，§7d）、
`tasks.read` / `tasks.human_judge_retry`（任务面板面: 只读投影 + 人判重试，§7e）、
`care.read`（关怀规则只读面，§7f）、`traits.read`（性格成长账只读面，§7f）、
`workspaces.read` / `workspaces.write`（工作区面：列表/单读 + 建/选/关，§7g；
`supported=available` 取值随工作区服务接线状态，见 §7g）、
`contacts.read` / `contacts.write`（联系人面：list/get + create/update/archive，§7h；
`supported=available` 取值随联系人服务接线状态，见 §7h）、
`permissions.approval.read`、`permissions.approval.resolve`、
`approvals.read` / `approvals.resolve`（compatibility aliases）、`permissions.grants.read`、
`permissions.revoke`、`organs.list`、`modules.list`、`trace.read`、`audit.read`、`activity.sse`、
`users.read` / `users.manage`、`domains.read` / `domains.manage`（V2-b 用户/域管理面，
`supported=available` 取值随管理服务接线状态，见 §7c）。

## §10 当前实现状态

| 阶段 | 内容 | 状态 |
|---|---|---|
| P0 | GatewayServices bounded-context ports、live capability manifest、Runtime snapshot、sessions/tools/traces | ✅ 已完成(2026-09-04) |
| P1 | Governed memory query/write/forget/protect/unprotect/graph 与真实 revision | ✅ 已完成(2026-09-04) |
| P2 | Live Behavior/Capability/Provider/Model projection 与 approvals/grants | ✅ 已完成(2026-09-04) |
| P3 | Runtime Event Spine → trace/audit/SSE，CLI direct turn 可观测 | ✅ 已完成(2026-09-04) |
| A2 | 工作区 HTTP 契约面：列表/单读/建/选/关 + 乐观并发(`expected_revision`)+ 审计(`gateway.workspace_action`)+ 能力入册 + 错误码 `workspace_not_found`/`workspace_conflict`/`workspace_revoked`(见 §7g) | ✅ 已完成(2026-11-17, 后端接线合同 A2 批)；**不承诺持久化**(注册表进程内, 持久化属 A3 批) |
| A3 | 工作区持久化：落盘注册表 + 跨重启恢复 + 坏记录 fail-closed + 已吊销不可复活 | ✅ 已完成(2026-11-17, 后端接线合同 A3 批)；本地文件系统介质, 非 DB/WAL |
| B3a | 联系人 HTTP 契约面：list/create/get/update/archive + 乐观并发(`expected_revision`)+ 归档终态(403 `contact_archived`)+ 工作区绑定校验(409 `contact_binding_conflict`)+ 审计(`gateway.contact_action`)+ 能力入册 + 4 个新错误码(见 §7h) | ✅ 已完成(2026-11-17, 后端接线合同 B3a 批)；**回合入口未接线**(B3b), 新建联系人当前不影响真实回合 |
| C2-1 | 对话操作幂等去重:`/v1/chat` 头 `x-apeireth-operation` + SHA-256 三段载荷摘要 + 进程内 1024 条表 + 两错误码 409 `operation_conflict`/`operation_in_progress`(见 §3 幂等小节) | ✅ 已完成(2026-11-18, 后端接线合同 C2-1 批);HTTP 面 PRODUCTION WIRED;**幂等表进程内, 重启即失, 多进程不共享**;`/v1/chat/completions` 未接线, 回放不重放流式 delta |
| C2-2 | 回合级停止:`POST /v1/sessions/{sid}/requests/{rid}/stop` + 运行时回合级取消注册表 + module_host 取消令牌注入 + 流式 SSE chunk id 与运行时 request_id 对齐(见 §11) | ✅ 已完成(2026-11-18, 后端接线合同 C2-2 批);HTTP 面 + 回合级注入 PRODUCTION WIRED;⚠️ 未做真实回合 E2E;**注册表进程内, 重启即失**;`stopped` 终帧发布归 C2-3 |
| C2-3 | 审批终帧 + C1 生产装配:流式入口显式铸定 request id 的回合实时写 C1 权威流, 结束时恰发一帧终帧(completed/approval/failed/stopped), 映射见 §11 | ✅ 已完成(2026-11-18, 后端接线合同 C2-3 批);流式回合生产路径 PRODUCTION WIRED(5 条 E2E 钉住四类终帧 + 非流式不注册);**C1 缓冲仍进程内, 重启即失**;审批恢复不续流(见 §11) |

实现位置:`crates/adapters/gateway/src/`(canonical_entry.rs + panels.rs + events.rs)，
数据源为 Runtime/Assembly 注入的 bounded-context ports；CLI 组合根位于
`crates/adapters/cli/src/gateway_panels.rs`，旧 `PanelData` 仅作为兼容适配层。
操作幂等(C2-1)的实现位置:表与守卫在
`crates/engine/runtime/src/canonical/operation_idempotency.rs`,头解析/载荷摘要在
`crates/adapters/gateway/src/operation_header.rs`,入口接线在 `canonical_entry.rs` 的
`execute_chat_attributed_operation`。
回合级停止(C2-2)的实现位置:注册表在
`crates/engine/runtime/src/canonical/turn_cancellation.rs`,回合注册在 `execute.rs`,
工具取消注入在 `module_host.rs`,端点与路由在
`crates/adapters/gateway/src/canonical_stream.rs`,流式 id 对齐在
`canonical_entry.rs` 的 `openai_chat_streaming`。
C1 生产装配(C2-3)的实现位置:纯终帧映射在
`crates/engine/runtime/src/canonical/stream_wiring.rs`,回合注册/delta 转发/终帧发布在
`crates/engine/runtime/src/canonical/execute.rs` 的 `execute_outcome_with_sink`,
停止摘要查询在 `turn_cancellation.rs`;端到端证据在
`crates/engine/runtime/tests/c1_stream_wiring.rs`。

## §11 C2 停止语义与终帧(C2-2/C2-3 批)

`POST /v1/sessions/{session_id}/requests/{request_id}/stop` 停止一个在飞回合。

- **寻址**:`request_id` 就是运行时回合 id;流式 `/v1/chat/completions` 的 SSE chunk `id`
  与该 id 同源(C2-2 起客户端从首帧即可见),之后可直接用该 id 调 stop。
- **200**(首次与重复 stop 的 body 逐字段相同):

```json
{"session_id":"...","request_id":"...","state":"stopped","effects_may_be_incomplete":true}
```

- **幂等**:同键重复 stop 回放第一次冻结的摘要(最近 512 条,进程内);回合结束后仍回放。
- **404 `session_not_found`**:没有该键的运行中回合,也没有停止摘要。
- **400 `invalid_request`**:`session_id` / `request_id` 不是合法 UUID(文案含原值)。
- **409 `invalid_request`**:该 request 正处待批;先批准/拒绝或等过期,stop 不越过审批裁决。
- **边界**:stop 不取会话写闸、不执行回合、不撤销已发生的副作用;只取消回合级取消令牌 ⇒
  阻断后续工具派发与提示工具。C2-3 起由运行时发布 `stopped` 终帧(停止事实优先于回合
  最终返回值);停止摘要与 C1 缓冲都是进程内内存,进程重启即失,多进程不共享。

### C1 终帧映射(C2-3)

流式入口在调用 provider 前显式铸定 request id 并注册一条 C1 流:provider delta
经 runtime sink 实时 `publish_delta`,回合结束时发布**恰好一帧**终帧。网关
`GET /v1/sessions/{sid}/requests/{rid}/stream` 只投影运行时缓冲,不持有第二份。

| 回合事实 | 终帧 `state` | 终帧字段 |
|---|---|---|
| 存在该回合的停止摘要 | `stopped` | `text`/`usage`/`error` 皆 `null`(只表达停止事实) |
| `Ok(TurnOutcome::Completed(response))` | `completed` | `text` = 最终助手文本;`usage` = `NormalizedUsage` 的 prompt/completion 如实映射, total 由两者相加;`error` 为 `null` |
| `Ok(TurnOutcome::PendingApproval(view))` | `approval` | `approvals` 恰一条:`approval_id` = `view.approval_id`,`action_binding_digest` 空串 ⇒ `null`,`expires_at_ms` = `view.expires_at.epoch_millis()`;`text` = `view.generated_text`(非空才给, 否则 `null`);`usage` 为 `null` |
| `Err(RuntimeError)` | `failed` | `error` = `RuntimeError::to_string()`;`text`/`usage` 为 `null` |

优先级:停止摘要存在时先判停止,即使回合随后正常完成、停在审批或失败,终帧仍为
`stopped`。终帧由注册表回填 `event`/`session_id`/`request_id`/`sequence`;终帧序号严格
大于此前所有 delta,发布后流进入终态(重复发布被拒)。

- **恰好一次**:每个已注册流只发一帧终帧;终帧发布失败被忽略,不改变回合结果、不 panic。
- **注册前置**:同一 `(session_id, request_id)` 已有流时,注册失败 ⇒ 回合不执行(fail-closed)。
- **不注册**:非流式回合,或流式但入口未铸定 request id ⇒ 不注册 C1 流,GET 仍 404
  `session_not_found`(异步生成的新 id 不补注册)。
- **边界**:C1 缓冲是进程内环形窗口(256 帧),重启即失、多进程不共享;审批恢复后不续流,
  客户端须重新发起回合;终帧只是控制面事实,不重放历史 delta。
