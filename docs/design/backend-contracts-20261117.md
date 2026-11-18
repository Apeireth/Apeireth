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
| A3 | 工作区持久化（跨重启恢复） | 工作区身份、revision、状态与受管根的登记事实落库；重启后 `GET /v1/panel/workspaces` 仍能列出且 revision 单调；恢复时按 `workspace_id + revision` 精确匹配，revision 落后者被拒 | ✅ 已完成（提交 `e5cb9bfb6`，四级口径 **IMPLEMENTED（持久化层：本地文件系统 + `StoredDoc` 信封；跨重启可恢复；坏记录 fail-closed；已吊销不可复活；落盘失败内存零变化）**；⚠️ 介质＝本地文件系统（非 DB/WAL，原子性只到单文件替换级）、`restore_exact` 尚无 HTTP 端点、多根/临时根/预设根装配未做、自动过期回收未做、workspace→session/task/审批/流的失效联动未做、多用户归属未建模、**绝对根落盘但绝不外泄**） |
| B1 | 联系人 / Agent identity + 人设（字节上限 + 控制字符 + 版本号） | `contact_id`/`agent_id`/`persona_revision` 由后端生成；人设有字节上限与控制字符校验；persona 只影响允许的注入槽，**不得**改变工具权限、审批决定、workspace scope 或系统策略；同名不同 ID 不碰撞 | ✅ 已完成（提交 `5bfb959d7`，四级口径 **IMPLEMENTED（领域层：ContactId/ContactAgentId/PersonaText/Contact + InMemory/FileContactRegistry）**；⚠️ 无 HTTP 面、无 `workspace_binding`、无记忆作用域映射、无会话绑定、无审计、未接生产路径） |
| B2 | 记忆作用域映射 + 会话 agent 绑定 + 归档语义 | `memory_scope` 受管枚举并与主体/联系人/工作区隔离（联系人 A 读不到 B 的记忆）；绑定请求携带 contact_id+agent_id+persona_revision+workspace_id+workspace_revision+expected_context_key+incarnation，任一不匹配即拒绝**且不重放消息**；归档为软删除，旧会话不静默转移到默认 Agent；全部动作有可检索审计身份 | ✅ 已完成（提交 `49213dc78`，四级口径 **IMPLEMENTED（领域层：`canonical::agent_binding` + `Session.agent_binding`；21 条契约测试 + 5 组反向验伪）**；⚠️ **未 PRODUCTION WIRED**（`admit_turn` 无调用方，`domain_plane` 默认域回落仍是未绑定会话的现役路径）、HTTP 面归 B3、canonical `MemoryScope` 翻译归 B3、审计未做） |
| B3a | 联系人 HTTP 面（+ `workspace_binding` 校验） | `GET/POST /v1/agents/contacts`、`GET/PATCH /v1/agents/contacts/{id}`、`POST /v1/agents/contacts/{id}/archive`；身份由后端铸造；写面必须带 `expected_revision`（缺⇒400、不符⇒409 且状态逐字段不变）；归档为软删除、不可复活（**已归档是终态 ⇒ 403 `contact_archived`，不是 409 版本过期**）；`workspace_binding{workspace_id,revision}` 由服务端校验（不存在/已吊销/修订号不符 ⇒ 409 `contact_binding_conflict`）；同名不同 ID 不碰撞；审计事件与能力清单入册 | ✅ 已完成（提交 `43291e4ea`，四级口径 **IMPLEMENTED（HTTP 面 + 端口 + 审计 + 能力入册 + 4 个新错误码 + CLI 组合根接线）**；15 条网关测试 + 5 条 CLI 链路测试 + 4 组反向验伪；⚠️ **回合入口未接线 ⇒ 新建联系人当前不影响真实回合**、会话绑定面未做、`memory_scope` 只到 DTO 层未翻译、`capabilities` 只是展示投影非授权依据、绑定只存 id+revision 不存快照） |
| B3b-1 | 会话绑定面 + 回合入口闸（`admit_turn` 上线） | `GET/POST /v1/sessions/{id}/agent-binding`；回合入口先 `admit` 后 `append`（拒绝即不重放）；绑定不合法 ⇒ 回合拒绝；已归档联系人 ⇒ fail-closed，不回落默认 Agent/默认域；旧会话不静默转移到默认 Agent | ✅ 已完成（提交 `b7bcbacc0`，四级口径 **PRODUCTION WIRED**（绑定面路由已 merge ＋ 组合根装配 registry ＋ 回合入口闸接线）；⚠️ **未做 live E2E** ⇒ 不到 HARDWARE VALIDATED；作用域翻译未做（归 B3b-2）、persona 注入未做（归 B3b-3）） |
| B3b-2 | 装配层作用域翻译 + 写回接线（`MemoryScopeTarget` → canonical `MemoryScope`） | 装配层把 `MemoryScopeTarget` 翻译成 canonical `MemoryScope`（`none` ⇒ **不产生 scope**，不写记忆，**禁拿 `Global` 代替**）；写回链路按翻译结果落 scope，`writes=false` ⇒ 整段跳过 materializer | ✅ 已完成（提交 `c137f5a9d`，四级口径 **PRODUCTION WIRED**：翻译层单测 + 绑定会话写回链路已接生产装配；未绑定 / profile 路径逐字节不变；失效 fail-closed 整段不写）；⚠️ **未做 live E2E**；恢复补写未翻译；`memory_scope_binding` 缺位 ⇒ 已绑定会话显式零写入 |
| B3b-3 | persona 注入 | 绑定命中的 persona 文本注入回合（只改 system 文本，**不得**改权限/能力/工具面） | ✅ 已完成（本批两个提交：feat `7d18e4c32` + docs/claims `f60642cf7`；四级口径 **PRODUCTION WIRED**：`BoundTurnAdmission` 携带绑定 persona，`turn_admission::seeded_system` 复用 `DomainRuntimeContext::seeded_system` 语义，`execute.rs` 空会话 system 槽消费；7 条真实 runtime 回合测试 + 3 组反向验伪；⚠️ **仅空会话种子槽生效**（非空会话不追溯）；**未做 live E2E** ⇒ 不到 HARDWARE VALIDATED） |
| C1 | 规范流帧 + 后端流面（身份、sequence、重放、resync） | 帧 schema 固定（`apeireth.c1.delta.v1` / `apeireth.c1.final.v1`）；每帧带 session_id/request_id/trace_id/sequence（**不带** operation/incarnation/context_key —— 裁决 2/8：宿主 operation 是不透明标签、后端不解释，后端不生成 incarnation/context_key）；sequence 单调有界；`Last-Event-ID`/`after_sequence` 有限重放，超窗发 `resync_required` 事实帧后关闭通道；`[DONE]` **只表示通道关闭**；断线后 GET 回执能恢复真实状态且不重发模型 | ✅ 已交付（领域层 `e397cc687` + 生产装配 C2-3 `ccb62e678`，PR #135/#139；四级口径 **PRODUCTION WIRED**：流式回合注册 C1 流、实时转发 delta、结束时恰发一帧 completed/approval/failed/stopped 终帧，已登记进 `docs/gateway-api-contract.md` §2〔PRODUCTION WIRED〕；⚠️ 权威缓冲与停止摘要均为进程内，重启即失、多进程不共享；只注册显式 request id 的流式回合；未做 live E2E ⇒ 不到 HARDWARE VALIDATED；**不实现 `/bridge/*`**（裁决 15）） |
| C2 | 单次 canonical dispatch + stop 幂等 + 审批终帧 | 一个 operation 只产生一次 canonical dispatch（重连/刷新/abort 不重复调模型）；stop 单独、幂等、可审计且不撤销已发生的外部副作用；approval 终帧只带**已确认**的 approval_id/action_binding_digest/expires_at；provider 非流式回退、上游错误、超时、取消、慢客户端、大正文都有有界错误契约 | ✅ 已完成（C2-0 `80edf9832` / C2-1 `da81ac1be` / C2-2 `3bb73032f` / C2-3 `ccb62e678`；PR #139 已合主干 `27e3872f0`；四级口径 **PRODUCTION WIRED**：24 码封闭集、operation 幂等去重、stop 幂等端点、审批/停止终帧与 C1 生产装配；⚠️ 幂等表 / C1 缓冲 / 停止摘要均进程内，重启即失、多进程不共享；stop 不中断模型生成、不撤销已发生副作用；未做 live E2E ⇒ 不到 HARDWARE VALIDATED） |

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
  序号早于窗口 ⇒ 先发一帧 `resync_required` 事实帧（带 `resync_from_sequence`），
  **随后关闭该通道**（本通道不自动续发）⇒ 客户端据此**重新取回执**，而不是假装连续。
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
    窗口外重放 ⇒ 发 `resync_required` 事实帧（带 `resync_from_sequence`）**后关闭该通道**，
    客户端应改取回执。理由：沿用仓内「显式省略事实」范式
    （`crates/adapters/gateway/tests/events.rs:226-266` 的 `frames_omitted`）
    ⇒ 把「省略」写成事实，客户端才知道该回退；把省略伪装成连续＝假连续，最坏。
    **措辞裁定（2026-11-17，C1 实施时补记）**：**不在同一条通道内自动续发**。
    理由：同通道自动续发会把一次断线放大成无限循环，且把「客户端该回退到回执面」的判断
    藏进服务端；C1 实现按「先发事实帧 → 再关通道」落地（见下节 C1 交付）。
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

### 2026-11-17 B3a 批次交付（提交 `43291e4ea`）

- 落点：新建 `crates/adapters/gateway/src/contact_panel.rs`（1233 行）、`crates/adapters/cli/src/contact_panel.rs`（455 行）、
  `crates/adapters/gateway/tests/panel_contacts_face.rs`（15 测试）、`crates/adapters/cli/tests/contact_real_chain.rs`（5 测试）；
  改 `crates/adapters/gateway/src/{error_frame.rs,lib.rs,panels.rs}` 与 `crates/adapters/cli/src/{gateway_panels.rs,lib.rs}`；
  文档同步 `docs/gateway-api-contract.md`（§1 错误码 ＋ §2 端点表 ＋ 新增 §7h 联系人面 ＋ §9 能力清单 ＋ §10 状态表）。
- HTTP 面：`GET/POST /v1/agents/contacts`、`GET/PATCH /v1/agents/contacts/{id}`、`POST /v1/agents/contacts/{id}/archive`
  （**不另开** `/v1/panel/contacts` 别名）；`ContactDto` 13 键（合同字段 ＋ B1 增补 persona/revision/updated_at_ms
  ＋ `workspace_binding{workspace_id,revision}` ＋ `capabilities{read,write,approval_required}`）⇒ **不含绝对路径**。
- 关键不变量（已由测试钉死）：身份后端铸造（客户端自带 ⇒ 400）；写面必须带 `expected_revision`
  （缺 ⇒ 400 且**不调端口**；不符 ⇒ 409 `contact_conflict` ＋ `details{expected_revision,actual_revision}`，状态逐字段不变）；
  **归档是终态 ⇒ 403 `contact_archived`**（不是 409「版本过期，刷新后重试」——与 B2/A3 的拒绝次序同族，见裁决 12/18）；
  404 消息恒定「联系人不存在或不可见」；`deny_unknown_fields`（自带 `contact_id` ⇒ 400）；
  绑定只存 id+revision 不存快照（不符 ⇒ 409 `contact_binding_conflict` ＋ `details.reason`）；
  审计事件 `gateway.contact_action`，动作词 `contact.create|update|archive|deny`；
  能力清单 `contacts.read` / `contacts.write` 分开登记；CLI 组合根注入真落盘 `FileContactRegistry`
  ＋ 宿主绑定档 `<data_dir>/agents/contact-bindings.json`，开不出 ⇒ **fail-closed**。
- 反向验伪四组（红/绿各一条已贴进提交正文）：① 摘 revision 比对 ⇒ RED（left: 200 / right: 409）⇒ GREEN；
  ② 摘归档过滤 ⇒ RED（left: 1 / right: 0）⇒ GREEN；③ 摘 `deny_unknown_fields` ⇒ RED（自带 contact_id ⇒ left: 201 / right: 400）⇒ GREEN；
  ④ 摘工作区绑定修订号校验 ⇒ RED（left: 200 / right: 409）⇒ GREEN（还原后 SHA256 同哈希）。
- ⚠️ **诚实边界（四级口径：IMPLEMENTED，未 PRODUCTION WIRED）**：**回合入口未接线** ⇒ 新建联系人当前**不影响真实回合**
  （绑定闸 `admit_turn` 仍无调用方，归 B3b）；**会话绑定面未做**（`/v1/sessions/{id}/agent-binding` 归 B3b）；
  **`memory_scope` 只到 DTO 层未翻译**（装配层翻译归 B3b）；**`capabilities` 只是展示投影，非授权依据**
  （授权仍由 `Permission`/审批链决定）；绑定只存 id+revision ⇒ 改名/改人设不会让旧绑定失效（靠每回合现查现校验）；
  `ContactOut` 在合同字段之上增补 persona/revision/updated_at_ms（裁决 14）。
- **尺寸棘轮联动（本批的真实代价）**：B3a 使 4 个既有文件越过架构尺寸棘轮 ⇒ `arch-strict` 判红 ⇒ 合并裁决 BLOCKED
  ⇒ 按 PR #126 的既有口径（不抬基线、不登记例外）改为**行为等价搬迁**，把 4 个文件压回基线以内（见进度页）。

### 2026-11-17 C1 批次交付（提交 `e397cc687`，PR #135）

- 落点（独立工作树 `C:\Apeireth\wt\backend-contracts-c1-20261117`，分支 `feat/backend-contracts-c1-20261117`）：
  新建 `crates/foundation/protocol/src/canonical_stream.rs`（帧族 wire 类型）、
  `crates/engine/runtime/src/canonical/canonical_stream.rs`（运行时权威缓冲注册表）、
  `crates/adapters/gateway/src/canonical_stream.rs`（后端原生 SSE 面）；
  测试 3 个文件（protocol 9 / runtime 11 / gateway 7）；改 4 个既有文件（3 个 `pub mod` ＋ 路由注册一行 merge）。
- 帧族：`apeireth.c1.delta.v1` / `apeireth.c1.final.v1`；delta 帧带 `session_id/request_id/trace_id/sequence/delta/state`，
  final 帧带 `state`（`completed|approval|failed|unknown`）＋ `text/usage/approvals/error/resync_required`；
  **帧内无 `[DONE]`**（`[DONE]` 只表示通道关闭），**无 operation/incarnation/context_key**（裁决 2/8）。
- 权威缓冲（运行时注册表）：按 `(session_id, request_id)` 索引；sequence 从 1 单调；环形缓冲 256 帧；
  每订阅者队列 64 ＋ `try_send`（**发布方永不阻塞**，满了丢最旧并计数）；终帧只发一次（重复发布 ⇒ `FinalAlreadyPublished`）；
  终态后拒绝增量（`StreamFinalized`）；`replay(after_sequence)` 超窗返回**显式事实** `ResyncRequired{oldest_available_sequence}`。
- 后端原生面：`GET /v1/sessions/{session_id}/requests/{request_id}/stream`（text/event-stream）；
  续发锚点＝`Last-Event-ID` 头**优先** → `?after_sequence=` 次之 → 缺省 0；
  **先订阅后重放**（不漏帧）＋ 交界按序号去重；超窗 ⇒ 发一条 `resync_required` 事实帧（`state:"unknown"`）
  **再关闭通道**（裁决 17 措辞裁定，见上）。
- 反向验伪四组（红/绿各一条已贴进 PR 正文与提交正文）：① 摘终帧唯一性闸 ⇒ RED（10 passed / 1 failed）⇒ GREEN（11 passed）；
  ② 摘环形有界 ⇒ RED（left: 266 / right: 256）⇒ GREEN；③ 把超窗改成错误 ⇒ runtime RED（replay 不得失败）
  ＋ gateway RED（left: 500 / right: 200）⇒ GREEN；④ 摘 `.id(sequence)` ⇒ RED（left: None / right: Some("11")）⇒ GREEN。
- ⚠️ **诚实边界（本批交付时四级口径：IMPLEMENTED，未 PRODUCTION WIRED）**：**本批未接生产装配** ⇒ 当时端点**不产生真实流数据**
  ⇒ 当时**未登记**进 `docs/gateway-api-contract.md` 的「已实现端点〔PRODUCTION WIRED〕」一节（登记即抬高口径）。
  **后续已补齐**：2026-11-18 C2-3 `ccb62e678` 把流式回合接入注册/实时 delta/唯一终帧，并登记进该节
  ⇒ 当前四级口径为 **PRODUCTION WIRED**（见上表 C1 行；缓冲仍为进程内）；
  帧只两级；**不做** operation 级去重与 stop/cancel（归 C2）；权威缓冲是**进程内内存** ⇒ 进程重启即失
  （客户端须改取回执）；全局单例注册表 ⇒ 多实例部署下重连到另一实例得 404 而不是 `resync_required`；
  慢订阅者丢帧是静默的（未补 `frames_omitted` 类事实帧）。
- **尺寸棘轮联动**：C1 需在 `canonical_entry.rs` 注册一条路由，而该文件基线 2291 行 ⇒ 按既有口径
  （不抬基线、不登记例外）把路由定义搬进 `canonical_stream::routes()`、主路由表只加一行 `merge`
  （并把该函数的两行文档注释并成一行）⇒ 该文件**净增 0 行**，`arch_guard --strict` ⇒ VIOLATIONS 0。

### 2026-11-17 B3b-1 批次交付（提交 `b7bcbacc0`）

- 落点（独立工作树 `C:\Apeireth\wt\backend-contracts-b3b-20261117`，分支 `feat/backend-contracts-b3b-20261117`）：
  新建 `crates/engine/runtime/src/canonical/turn_admission.rs`（回合入口只读闸）、
  `crates/engine/runtime/src/canonical/runtime_builder.rs`（**行为等价搬迁**：`RuntimeBuilder` 整块搬来并加两个 registry 字段）、
  `crates/adapters/gateway/src/session_binding.rs`（绑定 HTTP 面）、
  `crates/adapters/gateway/src/runtime_error_map.rs`（**行为等价搬迁**：`classify_runtime_error`/`provider_code`）、
  `crates/adapters/cli/src/binding_registries.rs`（registry 进程级单例）；
  测试 2 个文件（runtime 9 用例 / gateway 9 用例）；改 16 个既有文件。
- 端点：`GET/POST /v1/sessions/{session_id}/agent-binding`（路由由 `panel_routes()` 一行 merge 装配）。
  GET ⇒ `{session_id, bound, binding, session_revision}`；POST 必带 `expected_revision`（缺 ⇒ 400 且不碰会话）、
  `contact_id` 字段**必带**（**显式 null ＝ 解绑**；字段缺失 ⇒ 400），绑定时 `agent_id/persona_revision/workspace_id/workspace_revision` 必填；
  `context_key`/`incarnation` 只透传不生成（省略即沿用已存值）。
- 回合入口闸（`execute.rs` 生产回合入口，先闸后写）：判定顺序＝
  ① 未绑定 ⇒ **放行**（零回归）② 已绑定但 registry 未装配 ⇒ **fail-closed 拒绝**（409 `agent_binding_rejected`）
  ③ 交领域层 `admit_bound_turn` 逐项校验（联系人不存在/已归档 ⇒ 拒且**不回落默认 Agent**；`agent_id`/`persona_revision`/
  `workspace_id`/`workspace_revision` 任一不符 ⇒ 拒）。
  拒绝 ⇒ 直接 `Err` ⇒ **零写入**（消息未追加、`revision` 未推进、`sessions.save` 未执行 ⇒ 不落盘）。
- 错误码目录（封闭集）新增 2 个：`agent_binding_rejected`（409，回合闸拒绝）、`session_binding_conflict`（409，会话 CAS 不符，带 `details{expected_revision, actual_revision}`）；
  复用既有码：`invalid_request`/`session_not_found`/`contact_not_found`/`contact_archived`(403)/`contact_conflict`/`workspace_not_found`/`workspace_revoked`/`workspace_conflict`/`unsupported`(501)。
- 审计：事件 `gateway.session_binding_action`，动作词 `session_binding.create|update|unbind|deny`；**allow 与 deny 都留痕**。
- registry 单例：`FileContactRegistry`/`FileWorkspaceRegistry` 是「开档即全读进内存 + 之后只改内存」⇒ 两个实例互不可见
  ⇒ 闸与 HTTP 面**必须共用同一实例**（`binding_registries.rs` 按根目录缓存）⇒ 否则会出现「HTTP 刚写的绑定，闸读不到」。
- 反向验伪七组（红/绿两条已贴进提交正文与仓外日志 `b3b1_falsify.log`）：摘路由接线 ⇒ gateway 9 条全红；
  摘闸接线 ⇒ runtime 7 条红＋gateway 1 条红；闸恒拒 ⇒ 放行类 2 条红；
  四个语义变异（CAS 由 `!=` 弱化成 `>`、归档码 403→404、显式 null 不当解绑、deny 不写审计）⇒ 各钉一条用例红；回退后 ⇒ 9＋9 全绿。
- 搬迁等价机器证明（去注释/去空行后代码行多重集比对）：`RuntimeBuilder` 缺失 **0** 行；
  `expire_active_approval_if_needed` 缺失 **1** 行＝签名行（可见性放宽）；`classify_runtime_error` 缺失 **1** 行＝签名行
  ⇒ 函数体逐字保留，行为改动 0。
- **尺寸棘轮联动**：`execute.rs` 5010→4960、`runtime.rs` 1331→**774**（已 <1200 脱离棘轮）、`canonical_entry.rs` 2291→2240；
  `cli/src/lib.rs` 3786→3788（**正好等于基线，0 余量**）；新增 5 个 src 文件 667/115/938/68/94 行 ⇒ 全部 <1200 不进棘轮。
  `arch_guard --strict` ⇒ VIOLATIONS 0。
- ⚠️ **诚实边界**：四级口径 **PRODUCTION WIRED**（绑定面路由已 merge、组合根已装配 registry、回合入口闸已接线），
  **未做 live E2E**（没起真实网关打真流量）⇒ **不到 HARDWARE VALIDATED**；
  **作用域翻译（B3b-2）与 persona 注入（B3b-3）未接线**（领域层已产出，本批刻意不消费）；
  `agent_id` 不符分支**结构不可达**（`contact_id`/`agent_id` 由后端铸造且不可变）⇒ 端到端只覆盖可达的 `persona_revision` 不符；
  CAS 骑在 `session.revision` 上（该值也会被普通 append 推进）⇒ 客户端写前必须重读；
  **绑定变更不写 SessionEvent**（只写审计）；多进程部署下 registry 仍会分叉（本批只保证单进程内一致）。

### 2026-11-18 B3b-2 批次交付（提交=`c137f5a9d`；分支 `feat/backend-contracts-b3b2-20261117`，起点 `9b91de8fe`）

> 任务：把合同受管作用域忠实翻译成 canonical `MemoryScope`，并让**已绑定会话**的记忆写回真的落到该作用域；
> 不改变未绑定会话与 profile 分区路径的逐字节行为。

- 落点（独立工作树 `C:\Apeireth\wt\backend-contracts-b3b2-20261117`）：
  新建 `crates/engine/runtime-assembly/src/canonical/memory_scope_wire.rs`（168 行：`translate_scope` / `writes_scoped` + 5 条单测）、
  `crates/engine/runtime-assembly/src/canonical/production_helpers.rs`（26 行，`collect_tool_module` **行为等价搬迁**，使 `production.rs` 净增为负）、
  `crates/engine/runtime-assembly/tests/memory_scope_wiring.rs`（425 行 / 6 用例）；
  改 `crates/engine/runtime-assembly/src/canonical/cognitive.rs`（`MemoryScopeWiring` + `writeback_scope` + 7 处作用域替换）、
  `.../src/canonical/production.rs`（`ProductionBackends.memory_scope_binding` + 构造接线）、`.../src/canonical.rs`（模块声明）、
  `crates/engine/runtime/src/canonical/module.rs`（`ModuleContext.agent_binding: Option<&'a AgentBinding>`，host 事实、只读、不参与授权判定）、
  `.../behavior_adapter.rs`（`BehaviorInput.agent_binding: Option<AgentBinding>`，owned；`:274` 透传）、
  `.../execute.rs`（`run_hook` 填 `session.agent_binding().cloned()`，**净增 +1 行，在 50 行余量内**）、
  `crates/adapters/cli/src/binding_registries.rs`（`memory_scope_wiring(root)` + 搬迁 `typed_recall_identity_from_env` 及 4 个常量，
  `lib.rs` 以 `pub use` 保持 `crate::typed_recall_identity_from_env` 公开路径与签名不变）、
  `crates/adapters/cli/src/lib.rs`（`CognitiveBackends` 填 `memory_scope_binding`，净增 −18 行），其余 13 个测试/装配文件补 15 处 `agent_binding: None`（生产构造点仅 `behavior_adapter.rs:274` 填真实绑定）。
- 翻译层语义（`memory_scope_wire.rs`）：`MemoryScopeTarget::User{user_id}` ⇒ `MemoryScope::User`、
  `Persona{persona_id,user_id}` ⇒ `MemoryScope::Persona`、`Session{session_id}` ⇒ `MemoryScope::Session`，字段逐个同名；
  `scope: None` ⇒ `None`；`MemoryScopeTarget` 无 `Global` 变体 ⇒ **`Global` 在类型上不可达**（不放行「拿 Global 顶替」）。
- 写回判定顺序（`MemoryWritebackModule::writeback_scope`，AfterTurn 主路径）：profile 私有分区 ⇒ `Session`（**优先级高于绑定**，逐字节不变）
  → 未绑定 ⇒ `Session`（0 回归）→ `memory_scope` 装配面缺位 ⇒ `None`（已绑定会话显式零写入，**不是回退**）
  → `bound_memory_plan` 拒绝（archived / agent / persona / workspace 任一失效）⇒ `metrics.warning()` + `None`（**fail-closed 整段不写，不回退 `Session`**）
  → `writes=false` ⇒ `None`（整段跳过 materializer）→ 否则 `translate_scope`。合同 `writes` 与作用域仍由 B2 `map_memory_scope` 决定（设计稿裁决 5/19 不变）。
- 7 处硬编码 `MemoryScope::Session` 按裁决 A 统一经同一局部 scope：BoundedMemoryInput / typed_input / ProfileTypedSink 作用域 /
  profile 分区 guard 比对 / materialize 产物写回 / legacy 原始 episodes 写回 / consolidation insight 写回；循环与落库结构未改。
- 装配：`production.rs::ProductionBackends.memory_scope_binding: Option<MemoryScopeWiring>`（**缺位＝已绑定会话写回显式跳过**）；
  CLI 组合根用 `binding_registries::contacts/workspaces(&default_panel_data_dir())` 的 Ok 值（与 B3b-1 注入 Runtime 的同一对进程级单例）
  + `subject_id` 复用 `typed_recall_identity_from_env()` 的默认 `local-user`（不新造身份、不新增 env 名），开不出 ⇒ `None`（不 panic）。
- 测试（2026-11-18 实跑）：`memory_scope_wiring` 6 passed / 0 failed（真实 runtime 回合：预存带绑定会话 → `runtime.execute` → 逐条落库记忆均为
  `Persona{contact_id,user_id}` 且同回合无 `Session` 行；另盖未绑定 ⇒ `Session`、`Session` 合同 ⇒ `Session`、`none` ⇒ 零写入、archived ⇒ 零写入、缺装配面 ⇒ 零写入）；
  翻译层单测 5 passed / 0 failed。
- 反向验伪四组（红/绿两条实跑；回退后刷新 mtime 且 SHA256 同哈希）：
  ① Persona 臂临时映射成 `Session` ⇒ 单测 **3 passed / 2 failed**（`left: Session` vs `right: Persona`），还原 ⇒ 5 passed，sha `D204F5DA…C7DC2`；
  ② `Ok(_) => None` 改成回退 `Session` ⇒ `a_bound_none_contract_writes_nothing` **0 passed / 1 failed**（实际落 2 条 Session），还原 ⇒ 1 passed，sha `12A0C2B0…7538E`；
  ③ 绑定读取恒 `None` ⇒ 真实回合用例 **0 passed / 1 failed**（实际 `Session`，期望 `Persona`），还原 ⇒ 1 passed，同 sha；
  ④ 失效分支改成回退 `Session` ⇒ `an_archived_contact_fails_closed_without_any_write` **0 passed / 1 failed**（实际落 2 条 Session），还原 ⇒ 1 passed，同 sha。
- 门禁：clippy `-D warnings` 0 警告；`fmt-check` 35 crate 全 0 diff；`arch_guard --strict` ⇒ VIOLATIONS 0（edges 132 不变、无新例外；
  `cognitive.rs` 5758→5835 行由既有例外 EXC-0002 抑制）；`audit_unwired.py` ⇒ 本批新符号 0 孤儿；
  `check_crate_budget.py --check` 首跑红（cli +23 / runtime +8 / runtime-assembly +273 行，新增功能代码所致）；按官方修法 `--gen` 重建基线（34 crate · 600371 行，合并 upstream/main 后重算）⇒ `--check` exit 0。
- 尺寸棘轮联动：`execute.rs` 4960→4961（+1，余量内）、`production.rs` 1953→1946（−7）、`cli/src/lib.rs` 3788→3770（−18）；
  新文件 168 / 26 / 425 行 ⇒ 全部 <1200 不进棘轮。
- 边界核验：0 新外部依赖、0 新 crate（workspace 35 members）、production edges 132 不变、`apeireth-runtime` 不依赖 `apeireth-memory`、
  LOCKED 5 项 0 触碰、未改 `MemoryScope` 本体 / Gateway HTTP 面 / C1 `canonical_stream` / C2 错误码面；未登记 `docs/gateway-api-contract.md`（无新端点）。
- ⚠️ **诚实边界**：四级口径 **PRODUCTION WIRED**（翻译层单测 + 写回链路已接生产装配 + 6 条真实 runtime 路径测试）；
  **未做 live E2E**（没起真实网关 / 真实记忆库打全链路流量）⇒ **不到 HARDWARE VALIDATED**；
  **恢复补写（recovery-drain）未翻译**（早退路径只加 TODO 注释，仍按收集时会话锚点重放）；
  `profile_partition` 路径逐字节保持（绑定不改变私有分区行为）；
  测试用 `InMemoryContactRegistry` / `InMemoryWorkspaceRegistry`，未覆盖真实文件注册表跨进程场景；
  `memory_scope_binding` 缺位时已绑定会话**显式零写入**（CLI 组合根开不出注册表即此形态）。

### 2026-11-18 B3b-3 批次交付（本批两个提交：feat `7d18e4c32` + docs/claims `f60642cf7`；分支 `feat/backend-contracts-b3b3-20261117`，起点 `a1300b7bd`）

> 任务：把合同绑定命中的 persona 注入真实回合的 system 槽；只改 system 文本，不得改变权限/能力/工具面。

- 落点（独立工作树 `C:\Apeireth\wt\backend-contracts-b3b3-20261117`）：
  改 `crates/engine/runtime/src/canonical/turn_admission.rs`（115→141 行：`admit_bound_binding`/`admit_turn` 返回 `BoundTurnAdmission`；
  新增 `pub(super) fn seeded_system(admission, request_system)`，逐字复用 `DomainRuntimeContext::seeded_system` 的 persona/request 顺序）、
  `crates/engine/runtime/src/canonical/execute.rs`（5007→5008 行，**净增 +1**：`:878` 接住 admission，空会话 system 槽改为
  `seeded_system(&admission, request.system.as_deref())`）、`crates/engine/runtime/tests/turn_admission_gate.rs`
  （既有断言「空会话回合 +2 条消息」改 +3）；新建 `crates/engine/runtime/tests/turn_persona_injection.rs`（657 行 / 7 用例，
  真实 `Runtime::execute` + 捕获 provider）。
- 注入语义：`Unbound` ⇒ 不产 system（零回归）；`Admitted{persona: None}` ⇒ 与未绑定逐字相同；
  `Admitted{persona: Some(p)}` 且空会话 ⇒ 无 request.system 时 system 恰为 `p`，有则 `p + "\n\n" + request.system`（persona 在前）；
  非空会话 ⇒ 不追加第二条 system（绑定只在空会话种子槽生效）。
- 权限隔离：persona 只写入 system 消息，**不改变** `session.tool_scope`、Permission、审批决定、workspace scope 或系统策略；
  归档 / 校验不符仍由 `admit_bound_binding` 返回 `AgentBindingRejected` ⇒ `Runtime::execute` 在 `session.append`/revision/sessions.save 之前 Err
  ⇒ provider 0 调用、消息数与 revision 逐字段不变。
- 测试（2026-11-18 实跑）：`turn_persona_injection` **7 passed / 0 failed**（已绑定 + persona + request.system 拼接顺序 /
  已绑定无 persona 与未绑定逐字相同 / 未绑定零回归 / 仅 persona / 归档拒绝零调用 / persona 诱导越权前后 `tool_scope` 逐字段不变 /
  非空会话不追加第二条 system）；`--lib` 195 passed；`turn_admission_gate` 9 passed；`contact_persona_authority` 6 passed。
- 反向验伪三组（红/绿两条实跑，日志 `C:\Apeireth\tmp-build-discipline\logs\b3b3-f{1,2,3}-{red,green}.log`）：
  三组还原时点的 SHA256 均为 `7B0169A5…C19C`；随后 `cargo fmt -p apeireth-runtime` 对文件做了格式重排，
  最终提交 blob 的 SHA256 为 `651DE5F0…4A1D7`（提交前已重跑全部门禁，见下）。
  ① `seeded_system` 的 Admitted persona 分支临时改 None ⇒ **4 passed / 3 failed**（用例 1/4/6 红），还原 ⇒ 7 passed；
  ② 领域层 match 改为丢弃 persona（`Ok(_) => Unbound`）⇒ **4 passed / 3 failed**（同 1/4/6 红），还原 ⇒ 7 passed；
  ③ persona 与 request.system 顺序对调（persona 后置）⇒ **6 passed / 1 failed**（仅用例 1 红），还原 ⇒ 7 passed。
- 尺寸/门禁：`execute.rs` 5007→5008（+1，arch baseline 5010 内）；`canonical_entry.rs` 2291 未动；
  `arch_guard --strict` ⇒ VIOLATIONS 0（edges 132 不变、无新例外）；`audit_unwired.py --check` ⇒ 0 新增孤儿；
  `clippy -D warnings` 0 警告；`fmt-check` 35 crate 0 diff；`check_crate_budget.py --check` 首跑红（runtime +27 行）
  ⇒ 按官方修法 `--gen` 重建基线（runtime 31154→31181，34 crate · 602012 行）⇒ `--check` exit 0。
- 边界核验：0 新外部依赖、0 新 crate（workspace 35 members）、未改 `MemoryScope` / Gateway HTTP 面 / C1 `canonical_stream` /
  C2 错误码与幂等面；LOCKED 5 项 0 触碰。
- ⚠️ **诚实边界**：四级口径 **PRODUCTION WIRED**（真实 `Runtime::execute` 回合消费绑定 persona），**未做 live E2E**
  （没起真实网关 / 模型供应商）⇒ **不到 HARDWARE VALIDATED**；**仅空会话种子槽生效**，非空会话不追溯
  （绑定变更不会改写既有对话的 system）；persona 文本校验仍只在 B1 入册时做（本批不重复校验）；
  测试用 `InMemoryContactRegistry` / `InMemoryWorkspaceRegistry`。

### 还差什么

见[进度页](../status/backend-contracts-20261117.md)。
