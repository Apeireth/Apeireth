# Apeireth Canonical Gateway — HTTP API 契约 v1

> 本文档是桌面端(及其他客户端)与 Apeireth canonical gateway 之间的**稳定契约**。
> UI/UX 团队据此开发界面;后端团队据此实现并保证不破坏。任何端点变更必须先改本文档。
>
> 状态:2026-09-04，当前 canonical runtime/assembly 实现。标注 `[已实现]` 的端点当前可用；能力是否真正可用以 `/v1/apeireth/capabilities` 为准。
> 响应形状与 `apeireth-ui/src/lib/types.ts` 对齐；`frontend/companion-desktop` 是 Tauri 薄壳。

## §1 通用约定

- Base URL:`http://127.0.0.1:{port}`(port 由桌面壳为 sidecar 动态分配;手动运行时默认 8080)
- 编码:UTF-8 JSON;时间戳统一 **epoch 毫秒**(`timestamp` 字段)
- 认证:本机回环网关,无 bearer;`/v1/panel/*` 为内省只读面,不做外部鉴权(网关只绑 127.0.0.1)
- 错误模型:
  ```json
  { "error": { "code": "invalid_request | not_found | unsupported | runtime_error | auth_failed", "message": "人类可读信息" } }
  ```
  HTTP 语义:400 `invalid_request` / 403 `denied` / 404 `not_found` / 409 `conflict` /
  422 provider/runtime 语义失败 / 500 `internal_error` / 501 `unsupported` /
  503 `unavailable` / 202 审批挂起
- 分页:列表端点统一 `limit`(默认 50,最大 500)+ 按时间倒序;需要更多分页时再引入 cursor

## §2 已实现端点

| 端点 | 方法 | 说明 |
|---|---|---|
| `/health` | GET | `{"status":"ok","execution_owner":"apeireth-runtime::canonical"}` |
| `/v1/models` | GET | OpenAI 风格模型列表 `{object:"list",data:[{id,object:"model",created,owned_by}]}` |
| `/v1/providers` | GET | 来自 live Provider Router 的 provider 状态与模型描述 |
| `/v1/runtime/snapshot` | GET | 不含凭据、私有记忆、原始 prompt/CoT 的 Runtime 诊断快照 |
| `/v1/apeireth/capabilities` | GET | 来自 GatewayServices、Runtime Registry 与 Provider Router 的动态能力清单 |
| `/v1/modules` | GET | 来自 BehaviorRegistry 的行为模块清单；`/v1/organs` 为兼容别名 |
| `/v1/chat` | POST | 原生对话(见 §3) |
| `/v1/chat/completions` | POST | OpenAI 兼容对话(支持 `stream:true` SSE) |
| `/v1/approvals?session={id}` | GET | 该会话的待批审批(见 §6) |
| `/v1/approvals/resolve` | POST | 批准/拒绝审批(见 §6) |

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

## §4 会话 `[P0]`

`GET /v1/panel/sessions?limit=50` →
```json
{ "sessions": [
  { "id": "…", "title": "…", "created_at": 123, "updated_at": 123, "message_count": 4, "revision": 0 }
] }
```

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

## §6 工具与审批

`GET /v1/tools/list` `[P0]` →
```json
{ "tools": [
  { "name": "tool.repo", "description": "…", "args_schema": { … } | null,
    "source": "builtin", "permission": "granted|prompt|none", "available": true }
] }
```

审批(已实现,形状确认):
- `GET /v1/approvals?session={id}` → `{ "session": "…", "approvals": [ { "approval_id": "…", "session": "…", "tool_name": "…", "capability_id": "…", "governance_hook": "…", "governance_reason": "…", "request": "…", "trace_id": "…", "created_at": 123, "expires_at": 123 } ] }`
- `POST /v1/approvals/resolve` 请求 `{ "session": "…", "approval": "…", "decision": "approve|reject", "reason": "可选" }` → 与 `/v1/chat` 响应同形状(解析后继续原回合)

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

## §8 Trace / Audit / 事件总线

- `GET /v1/panel/traces?limit=20` `[P0]` → `{ "traces": [ { "trace_id": "…", "span_count": 3, "root_span": { … }, "started_at": 123 } ] }`
- `GET /v1/panel/traces/{trace_id}` `[P0]` →
  ```json
  { "trace_id": "…", "spans": [ { "span_id": "…", "parent_span_id": "…|null", "kind": "turn|provider|tool|governance", "actor": "…", "status": "ok|error", "summary": "…", "started_at": 123, "ended_at": 123, "session_id": "…" } ] }
  ```
- `GET /v1/panel/audit?limit=100` `[P1]` → `{ "events": [ { "ts": 123, "event": "chat.turn.completed", "service": "runtime", "detail": "…" } ] }`
  由 RuntimeEventSink 的审计端口归档；网关不再从响应结果反推一套 turn 事实。
- `GET /v1/apeireth/events` ✅ — Runtime Event Spine 的 SSE 投影:
  `backend_ready`(启动,仅广播一次)/ `turn_started` / `turn_delta` / `turn_completed` /
  `approval_required` / `approval_resolved`。
  帧格式:`event: <name>` + `data: <json>`;15s keep-alive;容量 256,慢订阅者被断连(不无限缓存)。
  > 当前诚实边界:`turn_delta` 携带**最终全文作为单条增量**——canonical 运行时在网关编码前
  > 已完整收口,网关边界观测不到 token 级增量。事件在进程内广播,不跨重启持久。

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
`tools.list`、`approvals.read`、`approvals.resolve`、`permissions.grants.read`、
`permissions.revoke`、`organs.list`、`modules.list`、`trace.read`、`audit.read`、`activity.sse`。

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
