# 后端接线合同：工作区、联系人 / Agent 人设 / 会话绑定、C1 真流式

[Document-Meta]
Document: docs/design/backend-contracts-20261117.md
Class: SPEC
Version: 0.1
Last-Modified: 2026-11-17
Status: 🟢 活跃
Author: lead

## 目标与范围

接住私有桌面前端交接的三块后端合同（来源：`C:\Apeireth\wt\ui-desktop-private\docs\intake\2026-11-17-backend-contract-handoff-workspace-contact-streaming.md`，
294 行，**合同草案**而非实现要求）：

- **A Workspace Contract**：工作区成为一等受管身份（UUID + revision + 作用域），不再是「本机工作区」这串无身份字符串。
- **B Custom Contact / Agent Persona / Session Contract**：联系人有独立可审计的 Agent identity；人设、记忆作用域、会话绑定由后端确认。
- **C C1 True Streaming Bridge**：逐 token 流成为**带身份**的规范流，回执仍是唯一完成事实。

前置基线：`upstream/main = 5c373955c`（PR #126 已合并）。

本设计稿只定**目标形态与验收**；实现按 §批次推进，每批走「真实 RED → GREEN → 反向验伪 → 门禁」。
前端当前保留的边界（只有运行时确认的本机工作区状态、只有默认直聊、渐进呈现只是 DOM 视觉过渡）**保持**，
本批不解除其中任何一条，直到对应合同落地并通过验收。

## 批次与验收

| 批次 | 交付目标 | 验收要求 | 当前进度 |
|---|---|---|---|
| A1 | 工作区领域类型 + 受管根 + revision CAS | 工作区有 UUID 身份；根目录、权限作用域、撤销状态任一变化都递增 revision；同名工作区仍按 UUID 区分；GET 读面不暴露完整根路径与凭据；旧客户端缺 workspace 字段**不默认提升为 write** | ✅ 已完成（提交 `91888e27c`，四级口径 **IMPLEMENTED（领域层）**；未持久化、无 HTTP 面、未接生产路径） |
| A2 | 工作区 HTTP 面 + 审计 + 与会话/项目/审批的绑定 | `GET /v1/panel/workspaces` 返回有界列表并区分「空 / 未装配 / 不可用」；创建/选择/关闭都落审计；`expected_revision` 冲突返回可分类 409；撤销或 revision 变化后旧写意图、审批、stream 不再生效 | ✅ 已完成（提交 `3854c01a8`，四级口径 **IMPLEMENTED（HTTP 面 + 端口 + 审计 + 能力入册 + 错误码 + 生产组合根接线）**；⚠️ **未持久化**（A3）、未接多根/临时根/预设根、未做自动过期回收、**workspace→session/task/审批/流的失效联动未做**、多用户归属未建模） |
| A3 | 工作区持久化（跨重启恢复） | 工作区身份、revision、状态与受管根的登记事实落库；重启后 `GET /v1/panel/workspaces` 仍能列出且 revision 单调；恢复时按 `workspace_id + revision` 精确匹配，revision 落后者被拒 | ❌ 未开始 |
| B1 | 联系人 / Agent identity + 人设（字节上限 + 控制字符 + 版本号） | `contact_id`/`agent_id`/`persona_revision` 由后端生成；人设有字节上限与控制字符校验；persona 只影响允许的注入槽，**不得**改变工具权限、审批决定、workspace scope 或系统策略；同名不同 ID 不碰撞 | ✅ 已完成（提交 `5bfb959d7`，四级口径 **IMPLEMENTED（领域层：ContactId/ContactAgentId/PersonaText/Contact + InMemory/FileContactRegistry）**；⚠️ 无 HTTP 面、无 `workspace_binding`、无记忆作用域映射、无会话绑定、无审计、未接生产路径） |
| B2 | 记忆作用域映射 + 会话 agent 绑定 + 归档语义 | `memory_scope` 受管枚举并与主体/联系人/工作区隔离（联系人 A 读不到 B 的记忆）；绑定请求携带 contact_id+agent_id+persona_revision+workspace_id+workspace_revision+expected_context_key+incarnation，任一不匹配即拒绝**且不重放消息**；归档为软删除，旧会话不静默转移到默认 Agent；全部动作有可检索审计身份 | ✅ 已完成（提交 `49213dc78`，四级口径 **IMPLEMENTED（领域层：`canonical::agent_binding` + `Session.agent_binding`；21 条契约测试 + 5 组反向验伪）**；⚠️ **未 PRODUCTION WIRED**（`admit_turn` 无调用方，`domain_plane` 默认域回落仍是未绑定会话的现役路径）、HTTP 面归 B3、canonical `MemoryScope` 翻译归 B3、审计未做） |
| B3 | 联系人 HTTP 面 + `workspace_binding` + 会话绑定面 | `GET/POST /v1/agents/contacts`、`GET/PATCH /v1/agents/contacts/{id}`、`POST /v1/agents/contacts/{id}/archive`、`GET/POST /v1/sessions/{id}/agent-binding`；`workspace_binding{workspace_id,revision}` 随联系人与会话绑定携带（B1 领域层已就位、绑定字段留到本批）；绑定任一字段不匹配即拒绝**且不重放消息**；归档为软删除、不可复活 | ❌ 未开始（B1 已完成，见批次表） |
| C1 | 规范流帧 + 后端流面（身份、sequence、重放、resync） | 帧 schema 固定（`apeireth.c1.delta.v1` / `apeireth.c1.final.v1`）；每帧带 operation/session/incarnation/context_key/sequence；sequence 单调有界；`Last-Event-ID`/`after_sequence` 有限重放，超窗返回 `resync_required`；`[DONE]` **只表示通道关闭**；断线后 GET 回执能恢复真实状态且不重发模型 | ❌ 未开始 |
| C2 | 单次 canonical dispatch + stop 幂等 + 审批终帧 | 一个 operation 只产生一次 canonical dispatch（重连/刷新/abort 不重复调模型）；stop 单独、幂等、可审计且不撤销已发生的外部副作用；approval 终帧只带**已确认**的 approval_id/action_binding_digest/expires_at；provider 非流式回退、上游错误、超时、取消、慢客户端、大正文都有有界错误契约 | ❌ 未开始 |

## 目标形态

### A. 工作区

领域类型（建议落点 `crates/engine/runtime/src/canonical/workspace.rs`，身份 `WorkspaceId` 进 `crates/foundation/core/src/kernel/ids.rs` 的 `generated_id!` 族）：

| 字段 | 含义 | 约束 |
|---|---|---|
| `workspace_id` | 一切绑定的身份 | UUID；**不能**用 display name 或路径代替 |
| `display_name` | 展示名 | 有界字符串 |
| `root_kind` | `managed` \| `temporary` \| `preset` | 受管枚举 |
| `root_display` | 安全展示名 | **不得**回传完整根路径、凭据、环境变量 |
| `permission_scope` | `read` \| `write` \| `approval_write` | 后端**已确认**的作用域；客户端提交的是 requested scope，不能直接成为 effective scope |
| `revision` | 单调版本 | 根目录、作用域、撤销状态任一变化都递增 |
| `status` | `ready` \| `creating` \| `revoked` \| `unavailable` | 受管枚举 |
| `can_select` / `can_create_temporary` / `can_write` | 能力声明 | `can_write` 只表示合同允许写，**不表示**某次操作已获人工批准 |
| `expires_at` / `source` / `updated_at_ms` | 生命周期 | epoch 毫秒 |

HTTP 面（沿用本仓既有命名约定；前端草案允许「路径按现有网关命名调整，语义保持一致」）：

```text
GET  /v1/panel/workspaces              # 有界列表 + 当前默认项 + 能力声明（空/未装配/不可用要区分）
POST /v1/workspaces                    # 创建临时工作区；客户端不得提交任意绝对路径
POST /v1/workspaces/{id}/select        # 带 expected_revision 的乐观并发
POST /v1/workspaces/{id}/close         # 临时工作区软关闭
GET  /v1/workspaces/{id}
```

### B. 联系人 / 人设 / 绑定

资源面：`GET/POST /v1/agents/contacts`、`GET/PATCH /v1/agents/contacts/{id}`、
`POST /v1/agents/contacts/{id}/archive`、`POST/GET /v1/sessions/{id}/agent-binding`。

DTO 字段按前端草案（`contact_id`/`agent_id`/`display_name`/`subtitle`/`avatar_ref`/`status`/`persona_revision`/
`memory_scope`/`workspace_binding{workspace_id,revision}`/`capabilities{read,write,approval_required}`）。
`status` 取值 `ready|draft|archived|unavailable`（**注意**：与运行时 `AgentStatus`
`Ready|Running|Paused|…` 是两套枚举，投影时显式映射，不复用）。

### C. 规范流

帧 schema 归属 `crates/foundation/protocol/src/canonical_stream.rs`（wire 类型：gateway / CLI / TUI / SDK 共用），
事件名 `apeireth.c1.delta.v1` / `apeireth.c1.final.v1`。字段按前端草案；
`incarnation` 允许 `uuid-or-bounded-label`，`context_key` 为 hex64。

**后端原生流面**（裁决 1 / 2 的直接落点）：

```text
GET  /v1/sessions/{session_id}/requests/{request_id}/stream   # ?after_sequence=N 或 Last-Event-ID 头
```

- 身份键＝`session_id + request_id`（+ `trace_id`）；宿主的 `operation` 只作**不透明有界标签**回显。
- 帧：
  - `apeireth.c1.delta.v1`＝`{session, request, trace, operation?, sequence, delta, state:"running"}`；
    `delta` 只是 **provisional** 展示，**不是**完成事实。
  - `apeireth.c1.final.v1`＝`{session, request, trace, operation?, state:"completed"|"approval"|"failed"|"unknown",
    text?, usage?, approvals?, error?}`；approval 终帧只带**已确认**的
    `approval_id` / `action_binding_digest` / `expires_at`。
  - 帧里的 `incarnation` / `context_key` 是**回显位**：调用方给了才带，后端**不生成**（裁决 8）。
  - `[DONE]` 只表示**通道关闭**；与既有 OpenAI 兼容 SSE 的 `[DONE]` 语义**并存不混**（那是别人的合同，不改）。
- 运行时落点：**规范流注册表**（`crates/engine/runtime/src/canonical/canonical_stream.rs`）按
  `(session_id, request_id)` 持有 **sequence 计数 + 有界环形缓冲（delta 帧）+ 终帧 + 订阅广播**；
  `/v1/chat` 的流式处理器把 delta **发布进注册表**，规范流端点从注册表**订阅 / 重放**
  ⇒ 同一回合可被**多个订阅者**观察，晚连者能按 `after_sequence` 补齐窗口内增量。
- 重放与超窗：`after_sequence` / `Last-Event-ID` ⇒ 从该序号之后**有限重放**；
  序号早于窗口 ⇒ 先发一帧 `resync_required`（带 `resync_from_sequence`），再从窗口最早可用序号续发
  ⇒ 客户端据此**重新取回执**，而不是假装连续。
- 断线回退：断线后 GET 回执（既有权威面）能恢复真实状态，且**不重发模型**（去重见 C2）。

## 裁决（本批已定，避免实现期反复）

1. **`/bridge/*` 是私有前端仓宿主自己的本地 HTTP 面，不是后端路由。**
   证据：`shell/desktop/src/host.rs:462`（`route.starts_with("/bridge/conversation/")` 由宿主处理）、
   `shell/desktop/src/host.rs:753`（`strip_prefix("/bridge/gateway")` ⇒ 转发给后端并只放行白名单 GET）、
   `shell/s2-server.mjs:192` / `:265-282`、`shell/desktop/src/host/c1.rs:5`。
   ⇒ ⇒ 后端**不实现** `/bridge/*` 路径；后端提供**后端原生**的规范流面，宿主把
   `/bridge/conversation/operations/{op}/stream` 映射过去。收据仍在宿主进程内存里（宿主自己的职责）。
2. **后端原生流面的身份键**用后端已有的 `session_id` + `request_id`（+ `trace_id`）；
   宿主的 `operation` 作为**不透明有界标签**原样回显，后端**不解释**它（不能由后端铸出一个无法校验的 operation id）。
3. **A 的 HTTP 路径沿用前端草案**：既有面板族就是 `/v1/panel/*` 读面（`crates/adapters/gateway/src/panels.rs:737-795`），
   写面用 `/v1/workspaces*`（与 `/v1/memory/append`、`/v1/panel/grants/revoke` 同族）。
4. **B 的 `status` 与运行时 `AgentStatus` 不合并**：两个枚举语义不同（合同态 vs 运行态），合并会让「归档」变成运行态。
5. **`memory_scope` 在 DTO 层映射，不改 `MemoryScope`**：`subject→User`、`contact→Persona`、`session→Session`、
   `none→不写记忆`（注意 `MemoryScope::Global` 的 `is_visible_in` 语义与 `none` **不等价**，不能拿 Global 当 none）。
   改 `MemoryScope` 会波及写入与隔离守卫，超出本批范围。
6. **persona 与权限结构上解耦，但必须显式声明**：现状 persona 只进 system 文本（`domain_plane.rs:146-152`），
   不参与任何 `Permission`/`tool_scope` 判定 ⇒ 合同「persona 不得覆盖权限」天然成立，但**没有校验** ⇒ 本批补显式不变量与断言。
7. **`root_kind` 命名冲突**：前端 `crates-ui/.../governance_tab.rs:867` 的 `root_kind` 是轨迹 span kind（示例 `"turn"`），
   与本合同的 `managed|temporary|preset` 语义不同 ⇒ **不得复用同一名字承载两义**；前端侧改名由前端线处理，
   后端用本合同的受管枚举并在文档里写明差异。
8. **`incarnation` 与 `context_key` 的语义由前端/宿主定义，后端只透传并做 exact-match 校验**；
   后端**不生成**它们（生成方与它们的作用域不在后端 ⇒ 后端造出来只会是假身份）。
9. **归档用墓碑 + revision + 不可复活**（照 `crates/engine/memory/src/persona_store_sqlite.rs:208-228` 与
   `migrations.rs:146-167` 的既有范式）；HTTP 侧现状皆硬删（`session_settings.rs:229-258`、`user_admin.rs:434`），
   归档语义必须**新建**而不是改写删除语义。
10. **旧会话不静默转移到默认 Agent**：现状 `domain_plane.rs:96` 有默认域回落 ⇒ 联系人归档场景必须 **fail-closed**，
    不照抄回落语义。
11. **创建工作区时，客户端不得提交任意绝对路径**（A2 批实施时追加的裁决）：`POST /v1/workspaces` 的请求体只有
    `display_name` / `root_kind` / `permission_scope` / `expires_at`；**受管根由宿主决定**。
    理由：A1 的立论是「绝对根是宿主实现细节，公开面只暴露身份 + 安全展示名」；若客户端能传路径，
    该不变量当场破（且「GET 读面不暴露完整根路径」这条合同验收项将形同虚设）。
12. **吊销 ⇒ 403 `workspace_revoked`，不是 409 冲突**（A2 批实施时追加的裁决）：吊销是「状态不可用」，
    不是并发冲突；客户端据码自愈的方式不同（409 ⇒ 重读修订号重试；403 ⇒ 停止使用该工作区）。
    同理「不存在」⇒ 404 且**消息恒定**「工作区不存在或不可见」（不区分「不存在」与「存在但不属于你」）。
13. **`workspace_binding` 与联系人 HTTP 面归 B3**（A2/B1 交付时追加的裁决）：B1 只做联系人**领域层**
    （身份 + 人设 + 墓碑归档 + 文件持久化），`workspace_binding{workspace_id,revision}` 是**跨资源绑定**，
    与会话绑定面一起做才不会造出「只有一半的绑定」；B2 做记忆作用域映射与会话 agent 绑定，B3 做 HTTP 面与绑定。
14. **`ContactOut` 在合同字段之上增补**（B1 交付时追加的裁决）：`persona` 正文 / `revision` / `updated_at_ms`
    是后端生成的事实（客户端不可写），对外暴露只读不构成越权 ⇒ 保留并写进契约文档。
15. **规范流的后端原生路径定为 `GET /v1/sessions/{session_id}/requests/{request_id}/stream`**（C 批实施时追加的裁决）：
    不实现 `/bridge/*`（裁决 1），宿主把 `/bridge/conversation/operations/{op}/stream` 映射过来。
    理由：身份键已经是 session + request（裁决 2），路径沿用既有 `/v1/sessions/...` 族
    （`crates/adapters/gateway/src/session_settings.rs:229-257`）⇒ 前端只需记两个 id，
    不必在后端造一个无法校验的 operation 身份。
16. **规范流的权威缓冲落在运行时注册表，不落在网关**（C 批实施时追加的裁决）：
    网关 SSE 处理器只做**投影**（把既有 OpenAI 兼容 chunk 转成规范帧），权威增量留在运行时的
    有界环形缓冲里 ⇒ 晚连 / 重连的订阅者能拿到**同一回合**的增量；
    否则「重放」退化成事后文本镜像（`turn_delta` 的老问题，`canonical_entry.rs:1798-1805`）。
17. **`resync_required` 是显式事实帧，不是错误**（C 批实施时追加的裁决）：
    窗口外重放 ⇒ 发 `resync_required`（带 `resync_from_sequence`）后从窗口最早序号续发，
    客户端应改取回执。理由：沿用仓内「显式省略事实」范式
    （`crates/adapters/gateway/tests/events.rs:226-266` 的 `frames_omitted`）
    ⇒ 把「省略」写成事实，客户端才知道该回退；把省略伪装成连续＝假连续，最坏。
18. **B2 补记 · 绑定的拒绝次序＝「存在 → 终态（archived / revoked）→ 版本精确匹配」**（B2 交付时追加）：
    与 B1 的 `checked_contact`（NotFound→Archived→RevisionConflict）、A3 的 `check_mutable`
    （NotFound→Revoked→RevisionConflict）同族；**故意不同于** A3 的 `restore_exact`
    （NotFound→RevisionConflict→Revoked）——后者是「按精确版本取回」的查询语义。
    理由：已归档 / 已吊销是**终态**，不得报成「版本过期，刷新后重试」（那种话术会引出永远不可能成功的重试）。
19. **B2 补记 · 作用域映射产物是 runtime 自有的镜像类型**（B2 交付时追加）：
    `MemoryScopeTarget` 只含 `user` / `persona` / `session` 三个变体，serde 形状与 canonical `MemoryScope` 一致；
    canonical `MemoryScope` 的翻译在 B3 装配层完成。
    本批 **0 触碰 `MemoryScope` 本体**，`apeireth-runtime` **0 依赖 `apeireth-memory`**（`Cargo.lock` 0 行 diff）。
    镜像类型**故意没有 `Global` 变体** ⇒ 「`none` ⇒ `Global`」在类型层不可表达（裁决 5 的结构保险）。
20. **B2 补记 · 归档 fail-closed 的落点是回合入口闸，不是 HTTP 状态码**（B2 交付时追加）：
    「拒绝且不重放」是回合语义（消息不追加、revision 不推进、不回落默认 Agent / 默认域），HTTP 状态码表达不了；
    故校验放在领域层（`admit_bound_turn` / `admit_turn`），B3 的网关只做状态码映射。
    绑定**只存 id + revision、不存 contact/workspace 快照**，每回合现查现校验 ⇒ 快照不会陈旧到放行失效绑定。
    附带边界：`Session.agent_binding` 沿用 `Session` 既有风格是 pub 字段 ⇒ 调用方理论上可写入未经校验的绑定，
    但回合闸会现查现校验 ⇒ 非法绑定必然被拒 ⇒ **保证落在闸上，不落在字段可见性上**。

## 未决（需要前端/宿主侧确认，后端按最保守口径实现）

- `expected_context_key` 的定义域与生成方（后端只校验 exact-match，不生成）。
- `incarnation` 在本合同语境下的生命周期（现仅 mesh 存在层有同名概念，语义不同）。
- 「成果版本」对象在后端尚无对应物（A.4 要求绑定到成果版本）⇒ 本批先绑 project/session/task/approval，成果版本待其对象确定。
- `workspace_binding` 是否等同于文件系统工作区（`capabilities/tools/src/worktree_apply.rs` 一族）尚未裁决 ⇒ 本批按「工作区即受管根」实现。
- 联系人 `capabilities{read,write,approval_required}` 到 `foundation/governance/src/permission.rs` 的映射粒度未定 ⇒ 本批只做**展示投影**，不作为授权依据。

## 不变量

1. **身份不可用展示名代替**。一切绑定（project/session/task/审批卡/成果版本）用 `workspace_id + revision`。
2. **客户端请求不是授权**。`requested_permission_scope` ≠ `permission_scope`；`can_write` ≠ 已获批准。
3. **不静默降级**。旧客户端缺 workspace 字段 ⇒ 明确的兼容/拒绝结果，**不默认提升为 write**；
   绑定任一字段不匹配 ⇒ 拒绝**且不重放消息**。
4. **回执是唯一完成事实**。流里的 delta 只是 provisional 展示；最终持久化事实只来自 final receipt。
5. **一个 operation 只一次 canonical dispatch**。重连、刷新、浏览器 abort 都不重复调模型。
6. **`[DONE]` 只表示通道关闭**。现有 `/v1/chat/completions` 的 `[DONE]` 语义（完成/待批/错误都跟 `[DONE]`）与本合同冲突 ⇒
   规范流**另起帧族**，不改既有 OpenAI 兼容 SSE 的行为（兼容面是别人的合同）。
7. **persona 不得改变权限**。人设文本只进允许的注入槽；工具权限、审批决定、workspace scope、系统策略不受其影响。
8. **跨工作区 / 跨联系人隔离**：同名不构成同一身份；A 不得读或推进 B 的会话、记忆、审批、成果。
9. **不泄露**：GET 读面不暴露完整根路径、凭据、环境变量或其它主体的工作区。
10. **零债**：本批新增能力自带测试与守卫；不改既有门禁阈值；LOCKED 项 0 触碰；不引新外部依赖。

## 不做

- 不实现 `/bridge/*` 路径（宿主自有面，见裁决 1）；不改私有前端仓。
- 不改 `MemoryScope` 枚举本体（只做 DTO 层映射）。
- 不改既有 OpenAI 兼容 SSE 的帧格式与 `[DONE]` 语义。
- 不把客户端提交的路径/作用域直接当受管事实。
- 不在本批引入真实模型调用（真模型调用另需本次临时凭据与预算，且只在软件入口与控制夹具通过后进行）。

## 进度与验收记录

### 2026-11-17 现状核查（只读调研，三份）

- **Workspace**：本仓**无** Workspace 领域类型（`workspace_id`/`root_kind`/`permission_scope`/`can_select` 全仓 0 命中）；
  根目录现状＝**进程单根**（`crates/adapters/cli/src/lib.rs:2060` `current_dir()`），包含性校验在
  `crates/capabilities/tools/src/shell.rs:244-268`、`filesystem.rs:124-185`、`shell/execution.rs:27-60`（审批冻结后复核）。
  CAS 范式可照抄：`crates/engine/memory/src/session_lifecycle.rs:115-129`（`Conflict{id,expected,actual}`）、
  `persona_store_sqlite.rs:212-421`；HTTP 已有 `expected_rev` 先例（`panels.rs:1276-1334`、`panel_memory_update.rs:23-56`），
  冲突经 `panel_errors.rs:33` 归 409。缺口 14 项（无领域类型/无多根/无路由/无冲突码/无权限枚举/无 Project 资源/
  无成果版本资源/无 operation id/无安全展示名规则/审计无对象维度/审批无工作区绑定/无持久化/能力清单未登记/工作线未登记）。
- **Contact / Persona**：本仓**无** contact 实体、**无** agent registry、**无** `/v1/agents*` 路由、
  **无** workspace/context_key 概念、**无** 合同版 `memory_scope`。现役人格注入**只有一个槽**
  （`crates/engine/runtime/src/canonical/domain_plane.rs:124-136` + `:146-152`，网关三处调用点）。
  记忆作用域本体 `crates/engine/memory/src/scope.rs:19-30`；隔离守卫 `engine/memory/src/backend/candidate.rs:65-107`。
  缺口 17 项。
- **C1 流式**：后端**已有真·逐 token 流**
  （`crates/engine/provider/src/canonical_openai_compatible.rs:404 complete_streaming` →
  `crates/engine/runtime/src/canonical/execute.rs:737 execute_outcome_streaming` →
  `crates/adapters/gateway/src/canonical_entry.rs:2031 openai_chat_streaming`），
  缺的是**帧身份**与**权威收口**；现有帧是 OpenAI 兼容 chunk（无 session/request/trace/operation），
  `[DONE]` 语义与合同冲突（`canonical_entry.rs:2108-2161`）。缺 stop/cancel HTTP 面
  （`barge_in.rs:42-53` 有打断器但未接路由、无幂等、无审计）、缺 sequence/replay/resync、
  审批终帧字段已现成（`runtime/canonical/approval.rs:323-365`）但无终帧载体。缺口 14 项。
- **宿主边界澄清**（本轮新增）：`/bridge/*` 属私有前端仓宿主面，详见裁决 1。

### 2026-11-17 A1 批次交付（提交 `91888e27c`）

- 落点：`crates/foundation/core/src/kernel/ids.rs` 新增 `WorkspaceId`（`generated_id!` 族，uuid v4）；
  新增 `crates/engine/runtime/src/canonical/workspace.rs`（819 行）；`canonical/mod.rs` 挂 `pub mod workspace;`；
  新增 `crates/engine/runtime/tests/workspace_contract.rs`（11 条断言）。
- 关键不变量（已由测试钉死）：同名不同 UUID；revision 只在「换根 / 换权限域 / 吊销」时 +1（**select 不动**）；
  过期 revision ⇒ `RevisionConflict{expected,actual}` 且工作区**逐字段不变**（含 `updated_at_ms`）；
  `safe_root_display` 只取叶子名（双分隔符、剥盘符/控制字符、≤64 字节、盘根⇒占位串）；
  能力字段（`can_select` / `can_create_temporary` / `can_write`）**派生而非接收**（`WorkspaceIn` 不收它们）；
  线格式不携带绝对根；`root` 私有且无公开 getter，Debug 脱敏为 `<redacted>`。
- 实测（2026-11-17，本机 Windows，`CARGO_TARGET_DIR=C:\Apeireth\cargo-target`，`-j 2`）：
  `cargo test -p apeireth-runtime --test workspace_contract --locked -j 2` ⇒ **11 passed; 0 failed**（EXIT 0）；
  `cargo clippy -p apeireth-runtime -p apeireth-core --all-targets --locked -j 2 -- -D warnings` ⇒ EXIT 0；
  `cargo fmt -p apeireth-runtime -p apeireth-core --check` ⇒ 0 diff（EXIT 0）。
- 反向验伪两组（红/绿各一条输出已贴进提交正文）：① 摘掉 `apply_permission_scope` 的 `self.revision += 1;`
  ⇒ RED「作用域变化必须恰好 +1 left: 0 right: 1」（9 passed / 2 failed）⇒ 装回 GREEN 11 passed / 0 failed；
  ② 让 `safe_root_display` 返回完整路径 ⇒ RED「只能取叶子目录名 left: "C:\Users\Someone\secret-parent\work"」
  （8 passed / 3 failed）⇒ 装回 GREEN 11 passed / 0 failed。
- ⚠️ **诚实边界（四级口径：IMPLEMENTED，仅领域层）**：注册表**进程内未持久化**（进程退出即失）；
  **无 HTTP 面**（A2 批）；**未接任何生产路径**（不在任何现有组装根里接线）；运行时仍是**进程单根**
  （未做多根解析）；`expires_at` 只透传、**无自动过期回收**；`WorkspaceId` 尚未进 kernel 再导出名单
  （当前用全路径 `apeireth_core::kernel::ids::WorkspaceId`）。

### 2026-11-17 A2 批次交付（提交 `3854c01a8`）

- 落点：新增 `crates/adapters/gateway/src/workspace_panel.rs`（718 行）与
  `crates/adapters/gateway/tests/panel_workspaces_face.rs`（13 测试）、
  `crates/adapters/cli/tests/workspace_real_chain.rs`（5 测试）；改
  `crates/adapters/gateway/src/panels.rs`（`GatewayServices.workspaces` 端口 + 5 条路由 + 能力清单
  `workspaces.read|workspaces.write`）、`crates/adapters/gateway/src/error_frame.rs`（3 个新码 + solution）、
  `crates/adapters/gateway/src/lib.rs`、`crates/adapters/cli/src/gateway_panels.rs`（`CliWorkspacePanel` +
  `bind_managed_workspace`）、`crates/adapters/cli/src/lib.rs:3264-3274`（组合根**真实注入**）、
  `docs/gateway-api-contract.md`（+84 行：§1 错误码 / §2 端点表 / 新增 §7g 工作区面 / §9 能力清单 / §10 状态表）。
- 端点面：`GET /v1/panel/workspaces`（包裹 `{"workspaces":[…],"count":n}`，有界）、`GET /v1/workspaces/{id}`、
  `POST /v1/workspaces`（201）、`POST /v1/workspaces/{id}/select`、`POST /v1/workspaces/{id}/close`。
- 关键不变量（已由测试钉死）：**端口只说 DTO** ⇒ 网关层**没有任何可承载绝对根路径的字段**；
  读面再叠一道展示名安全闸（叶子名、≤64 字节、无 `/ \ :` 与控制字符），不合格 ⇒ 500 且**不回显该值**；
  缺 `expected_revision` ⇒ 400 且**不调端口**；revision 不符 ⇒ 409 且**不调端口** ⇒ 工作区逐字段不变；
  吊销 ⇒ 403 `workspace_revoked`（裁决 12）；不存在 ⇒ 404 且消息恒定「工作区不存在或不可见」；
  创建请求体 `deny_unknown_fields` ⇒ 客户端塞 `root` 恒 400（受管根由宿主决定，裁决 11）；
  审计动作词 `workspace.create|select|close|deny`、事件 `gateway.workspace_action`，**审计详情无绝对路径**。
- 实测（2026-11-17，本机 Windows，`CARGO_TARGET_DIR=C:\Apeireth\cargo-target\backend-contracts`，`-j 2`）：
  `cargo test -p apeireth-gateway --test panel_workspaces_face --locked -j 2` ⇒ **13 passed; 0 failed**（EXIT 0）；
  `cargo test -p apeireth-cli --test workspace_real_chain --locked -j 2` ⇒ **5 passed; 0 failed**（EXIT 0）；
  `cargo clippy -p apeireth-gateway -p apeireth-cli --all-targets --locked -j 2 -- -D warnings` ⇒ EXIT 0；
  `cargo fmt -p apeireth-gateway -p apeireth-cli --check` ⇒ 0 diff（EXIT 0）；
  fast gate `python scripts/verify_changed.py` ⇒ `changed rust files: 8; affected crates: 3` +
  `check OK 57.8s` + `clippy OK 2.2s` + `test(nextest) OK 263.6s` ⇒ **FAST GATE PASSED**。
- 反向验伪三组（红/绿各一条已贴进提交正文）：① 摘掉写面修订号比对 ⇒ RED
  「stale_expected_revision_is_a_409_conflict_that_changes_nothing … left: 200 right: 409 / 11 passed; 2 failed」
  ⇒ 装回 GREEN 13 passed；② 摘掉展示名安全闸 ⇒ RED「body 里出现 `C:\\Users\\someone\\secret` … left: 200 right: 500 /
  12 passed; 1 failed」⇒ 装回 GREEN 13 passed；③ 把冲突帧从 409 降成 400 ⇒ RED「left: 400 right: 409 / 11 passed; 2 failed」
  ⇒ 装回 GREEN 13 passed。
- ⚠️ **诚实边界（四级口径：IMPLEMENTED，HTTP 面已接线）**：**未持久化**（A3 批）；**未接多根/临时根/预设根**
  （非受管根 ⇒ 400）；**未做自动过期回收**（`expires_at` 只透传）；**workspace→session/task/审批/流的失效联动未做**；
  多用户归属未建模（404 恒定消息是为归属建模预留的合同位）；作用域「宿主确认」当前＝**上限截断到 Write**；
  审计端口缺席时回落 `observations.buffer_audit`；`domain_context.rs` 未改（域门对未登记路径 fail-closed 403）。

### 2026-11-17 A3 批次交付（提交 `e5cb9bfb6`）

- 落点：`crates/engine/runtime/src/canonical/workspace.rs`（819 → 1446 行）、新增
  `crates/engine/runtime/tests/workspace_persistence.rs`（11 测试）与
  `crates/adapters/cli/tests/workspace_persistence_chain.rs`（3 测试）；改
  `crates/adapters/cli/src/gateway_panels.rs`（注册表改 `Arc<dyn WorkspaceRegistry>`、新增
  `new_persistent(managed_root, data_dir)`、`WorkspaceError::Storage` ⇒ 恒定文案「工作区存储故障」）与
  `crates/adapters/cli/src/lib.rs:3268-3277`（组合根改 `new_persistent(current_dir(), default_panel_data_dir())?`
  ⇒ **开不出注册表就启动失败**（fail-closed），不降级成进程内注册表假装成功）。
- 持久化介质：**本地文件系统 + 既有 `StoredDoc` 信封**（`name="workspace"`、`version=1`），落
  `<data_dir>/workspaces/<workspace_id>/workspace.json` + `selection.json` + 各自 `.lock`；
  **0 新增外部依赖**，与 B1 的 `FileContactRegistry` 同范式（物理隔离、零债）。
- 关键不变量（已由测试钉死）：**绝对根只落盘不外泄**（`root_display` 与能力字段是**派生**值，恢复时重算，
  读面仍只出叶子名）；**revision 随记录落盘**（重启不重置，跨工作区不可比）；**恢复按 `workspace_id + revision`
  精确匹配**，陈旧 revision ⇒ `RevisionConflict` 且状态零变化（顺序固定 NotFound → RevisionConflict → Revoked）；
  **已吊销的工作区不可复活**（`restore_exact` 与 `find_by_root` 都不返回它）；**坏记录 ⇒ fail-closed 拒开**
  （开不出注册表 ⇒ 组合根启动失败）；**落盘失败 ⇒ 内存零变化**（读盘上权威记录 → 校验 → 改副本 → 写盘 → 提交内存）；
  宿主身份按**受管根绝对路径**匹配 ⇒ `bind_managed_workspace` 幂等（同一根复用同一身份）。
- 反向验伪三组（红/绿各一条已贴进提交正文）：① 摘 revision 比对 ⇒ RED「陈旧 revision 必须被拒 …
  Workspace { workspace_id: WorkspaceId(3868fe54-…), revision: 1, status: Ready, … }」+
  「陈旧 revision 必须是 RevisionConflict, 实际是 Revoked」⇒ `9 passed; 2 failed` ⇒ 装回 GREEN 11 passed；
  ② 摘落盘里的受管根 ⇒ RED「落盘文件必须带着受管根 —— 否则重启后无从解析工作目录」+
  「重启后逐字段一致 … left: root_display: "workspace-root" right: ".tmp95NWgp"」⇒ `6 passed; 5 failed`
  ⇒ 装回 GREEN 11 passed；③ 摘恢复时的状态回填 ⇒ RED「left: Ready right: Revoked」+
  「已吊销的工作区不得被交回宿主去复用」⇒ `9 passed; 2 failed` ⇒ 装回 GREEN 11 passed。
- ⚠️ **诚实边界（四级口径：IMPLEMENTED，持久化层已就位；HTTP 面仍是 A2 的形态）**：落盘介质是**本地文件系统**
  （无 DB / 无 WAL）；原子性只到**单文件替换级**（Windows 上不做父目录 fsync）；锁粒度是**单工作区文件锁**
  + selection 独立锁（**跨工作区无全局序**）；revision 是**每工作区自己的**计数器（跨工作区不可比）；
  `restore_exact` **尚无 HTTP 端点、无生产调用点**；多根 / 临时根 / 预设根装配未做；自动过期回收未做；
  **workspace→session/task/审批/流的失效联动未做**；多用户归属未建模；落盘文件含绝对根 ⇒
  **数据根目录的读权限＝绝对根的读权限**（未做 0600 级收紧）；宿主身份按受管根绝对路径匹配 ⇒
  **换 CWD 重启会新铸身份**（跨路径稳定性未裁决）；`WorkspaceError` 新增公开变体 `Storage`
  （仓外穷举匹配算 minor 破坏，仓内唯一匹配点已同步）；A2 合同面文件 `crates/adapters/gateway/src/workspace_panel.rs`
  **0 改动**（持久化不污染合同语义）。

### 2026-11-17 B2 批次交付（提交 `49213dc78`）

- 落点：新建 `crates/engine/runtime/src/canonical/agent_binding.rs`（761 行）与
  `crates/engine/runtime/tests/agent_binding_contract.rs`（21 测试）；改
  `crates/engine/runtime/src/canonical/session.rs`（`Session.agent_binding` 字段＋`agent_binding()` /
  `set_agent_binding()`＋fork 继承＋`SessionRepr` 迁移字段）与 `canonical/mod.rs`（`pub mod agent_binding`）。
- 三个职责：① 记忆作用域映射（合同的受管枚举 → 写入落点，`none` ⇒ **不写**）；
  ② 绑定模型（contact + workspace 的一次精确配对，外加宿主自有的 `context_key` / `incarnation` 两个回显字段）；
  ③ **fail-closed 闸**（`admit_bound_turn` / `admit_turn`）。
- 关键不变量（已由测试钉死）：**`none` 在类型层不可表达为 `Global`**（镜像类型没有 `Global` 变体）；
  **同主体的不同联系人落在不同人设作用域**（`persona_id` 与 `user_id` 同时匹配才算可见）；
  **归档/吊销立即生效**（只存 id + revision，每回合现查现校验 ⇒ 快照不会陈旧到放行失效绑定）；
  **拒绝时不追加消息、不推进 revision**（`admit_turn` 在校验成功后才 `append`）；
  **绑定随 fork 继承**（不得静默掉回未绑定 / 默认域路径）；**绑定 revision 是 CAS**；
  **后端不生成 `context_key` / `incarnation`**（只透传并 exact-match）。
- 反向验伪五组（红/绿各一条已贴进提交正文）：① 摘 fail-closed 归档闸 ⇒ `18 passed; 3 failed` ⇒ GREEN 21 passed；
  ② 把 `append` 挪到 admit 之前 ⇒ RED「被拒的回合不得追加消息（不得重放）/ left: 1 / right: 0」⇒ `20 passed; 1 failed`
  ⇒ GREEN 21 passed；③ `none` 分支改产出 `User` 作用域 ⇒ RED「left: Some(User { user_id: "subject-1" }) / right: None」
  ⇒ `19 passed; 2 failed` ⇒ GREEN 21 passed；④ `persona_id` 改成 `subject_id` ⇒ RED「left: Persona { persona_id: "subject-1" … }」
  ⇒ `19 passed; 2 failed` ⇒ GREEN 21 passed；⑤ 摘 fork 继承 ⇒ RED「left: None / right: Some(ContactId(…))」
  ⇒ `20 passed; 1 failed` ⇒ GREEN 21 passed。
  ⚠️ 自检陷阱：还原文件若保留旧 mtime ⇒ cargo 判定源码未变 ⇒ 不重编 ⇒ 跑出的仍是上一次的红（必须刷新 mtime）。
- ⚠️ **诚实边界（四级口径：IMPLEMENTED 领域层，未 PRODUCTION WIRED）**：
  **无任何调用方调用 `admit_turn`** ⇒ `canonical::domain_plane` 的默认域回落仍是**未绑定会话**的现役路径；
  已绑定会话在生产回合里尚不会走 fail-closed 闸 ⇒ **回合入口接线属 B3b**；
  **HTTP 面未做**（归 B3a）；**canonical `MemoryScope` 的翻译未做**（归 B3 装配层）；
  **审计未做**（动作身份待 B3a 的 HTTP 面）；`MemoryScopeTarget` 是 runtime 自有镜像类型（serde 形状与 canonical 一致）；
  `apeireth-runtime` **0 依赖** `apeireth-memory`（`Cargo.lock` 0 行 diff）；
  `Session.agent_binding` 是 pub 字段（沿用 `Session` 既有风格）⇒ 保证来自回合闸的现查现校验，不来自字段可见性。

### 还差什么

见[进度页](../status/backend-contracts-20261117.md)。
