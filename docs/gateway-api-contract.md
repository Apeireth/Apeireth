# Apeireth Canonical Gateway — HTTP API 契约 v1

> 本文档是桌面端(及其他客户端)与 Apeireth canonical gateway 之间的**稳定契约**。
> UI/UX 团队据此开发界面;后端团队据此实现并保证不破坏。任何端点变更必须先改本文档。
>
> 状态:2026-09-04，当前 canonical runtime/assembly 实现。标注 `[已实现]` 的端点当前可用；能力是否真正可用以 `/v1/apeireth/capabilities` 为准。
> 响应形状与 `apeireth-ui/src/lib/types.ts` 对齐；`frontend/companion-desktop` 是 Tauri 薄壳。

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
| `/v1/chat` | POST | 原生对话(见 §3) |
| `/v1/chat/completions` | POST | OpenAI 兼容对话(支持 `stream:true` SSE) |
| `/v1/approvals?session={id}` | GET | 该会话的待批审批(见 §6) |
| `/v1/approvals/resolve` | POST | 批准/拒绝审批(见 §6) |
| `/v1/panel/metrics` | GET | 指标聚合快照(见 §8b;会话+全局两层,空态显式空) |
| `/v1/panel/skills` | GET | 技能库元数据只读面(见 §7d;只回元数据不回卡体,坏卡列拒不砖库) |
| `/v1/panel/tasks` | GET | 任务面板只读面(见 §7e;一次快照读 = 任务行 + 时间线 + 六态计数) |
| `/v1/panel/tasks/{task_id}/human-judge-retry` | POST | 人判重试(见 §7e;failed 只能人判,唯一动作面) |
| `/v1/panel/care` | GET | 关怀规则只读面(见 §7f;care_list 帧同形 + 心跳两列) |
| `/v1/panel/traits` | GET | 性格成长账只读面(见 §7f;四滑杆现值 + 锁定态 + 最近移动,与 `tool.trait_report` 同账同源) |
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

**流式契约**(`stream:true`,2026-09-10 起 token 级真流式,实测 210 帧 ~23ms 逐帧到达):
- 帧序列:`role` chunk → provider 逐 content delta 各一 chunk → 终帧(带 `usage` 与 apeireth 元数据)。
- 挂起审批以 `finish_reason:"approval_required"` 显式终帧终止流;provider 错误经统一错误帧契约上浮
  (`{"error":{"message","code","solution"}}`,见 ledger #16)。
- 增量链路:provider SSE → runtime sink(`execute_outcome_streaming`)→ gateway 逐 chunk 直通,非整段后分帧。

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

实现位置:`crates/adapters/gateway/src/`(canonical_entry.rs + panels.rs + events.rs)，
数据源为 Runtime/Assembly 注入的 bounded-context ports；CLI 组合根位于
`crates/adapters/cli/src/gateway_panels.rs`，旧 `PanelData` 仅作为兼容适配层。
