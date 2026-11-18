# 后端接线合同进度页（工作区 / 联系人·人设·会话绑定 / C1 真流式）

[Document-Meta]
Document: docs/status/backend-contracts-20261117.md
Class: STATUS
Status: 🟢 活跃
Last-Modified: 2026-11-18
Author: lead

> **用途**：本工作线「做到哪了」的**唯一权威**（`AGENTS.md` §0.2：引用状态只引文件）。
> 目标形态见[设计稿](../design/backend-contracts-20261117.md)。

## 一、批次状态（对照设计稿 §批次与验收）

| 批次 | 目标 | 四级口径 | 状态 | 证据 |
|---|---|---|---|---|
| A1 | 工作区领域类型 + 受管根 + revision CAS | **IMPLEMENTED**（仅领域层） | ✅ 已完成（提交 `91888e27c`） | `cargo test -p apeireth-runtime --test workspace_contract --locked -j 2` ⇒ 11 passed / 0 failed（2026-11-17）；clippy 0 警告；fmt 0 diff；反向验伪两组（摘 revision 递增 / 摘安全展示名）均 RED→GREEN |
| A2 | 工作区 HTTP 面 + 审计 + 会话/项目/审批绑定 | **IMPLEMENTED**（HTTP 面已接线；⚠️ 未持久化） | ✅ 已完成（提交 `3854c01a8`） | `cargo test -p apeireth-gateway --test panel_workspaces_face --locked -j 2` ⇒ 13 passed / 0 failed；`cargo test -p apeireth-cli --test workspace_real_chain --locked -j 2` ⇒ 5 passed / 0 failed（均 2026-11-17）；clippy 0 警告；fmt 0 diff；fast gate FAST GATE PASSED；反向验伪三组（摘修订号比对 / 摘展示名安全闸 / 409→400）均 RED→GREEN |
| A3 | 工作区持久化（跨重启恢复） | **IMPLEMENTED**（持久化层已就位；HTTP 面仍 A2 形态） | ✅ 已完成（提交 `e5cb9bfb6`） | `cargo test -p apeireth-runtime --locked -j 2` ⇒ 全绿（含 `workspace_contract` 11 passed + `workspace_persistence` 11 passed）；`cargo test -p apeireth-cli --locked -j 2` ⇒ 全绿（含 `workspace_real_chain` 5 passed + `workspace_persistence_chain` 3 passed）；clippy 0 警告；fmt 0 diff；孤儿 API 棘轮 0 新增；反向验伪三组（摘 revision 比对 / 摘落盘受管根 / 摘恢复状态回填）均 RED→GREEN |
| B1 | 联系人 / Agent identity + 人设（字节上限 / 控制字符 / 版本号） | **IMPLEMENTED**（仅领域层） | ✅ 已完成（提交 `5bfb959d7`） | `cargo test -p apeireth-core --test agent_contact --locked -j 2` ⇒ 16 passed / 0 failed；`cargo test -p apeireth-runtime --test contact_persona_authority --locked -j 2` ⇒ 6 passed / 0 failed（均 2026-11-17）；clippy 0 警告；fmt 0 diff；反向验伪五组（persona_revision 不递增 / 归档可复活 / 上限被绕过 / 入面采信客户端 capabilities / 人设被当授权依据）均 RED→GREEN |
| B2 | 记忆作用域映射 + 会话 agent 绑定 + 归档语义 | **IMPLEMENTED**（仅领域层） | ✅ 已完成（提交 `49213dc78`） | `cargo test -p apeireth-runtime --test agent_binding_contract --locked -j 2` ⇒ 21 passed / 0 failed；`cargo test -p apeireth-runtime --all-targets --locked -j 2` ⇒ 全绿（均 2026-11-17）；clippy 0 警告；fmt 0 diff；反向验伪五组（fail-closed 归档闸 / 不重放 / none≠Global / contact 间隔离 / fork 继承）均 RED→GREEN |
| B3a | 联系人 HTTP 面 + `workspace_binding` 校验 | **IMPLEMENTED**（HTTP 面已接线；⚠️ 回合入口未接线） | ✅ 已完成（提交 `43291e4ea`） | `cargo test -p apeireth-gateway --test panel_contacts_face --locked -j 2` ⇒ 15 passed / 0 failed；`cargo test -p apeireth-cli --test contact_real_chain --locked -j 2` ⇒ 5 passed / 0 failed（均 2026-11-17）；clippy 0 警告；fmt 0 diff；反向验伪四组（摘修订号比对 / 摘归档过滤 / 摘 deny_unknown_fields / 摘绑定修订号校验）均 RED→GREEN |
| B3b-1 | 会话绑定面 + 回合入口闸 | **PRODUCTION WIRED**（绑定面路由已 merge ＋ 组合根装配 registry ＋ 回合入口闸已接线；⚠️ 未做 live E2E） | ✅ 已完成（提交 `b7bcbacc0`） | `cargo test -p apeireth-runtime --test turn_admission_gate --locked -j 2` ⇒ 9 passed / 0 failed；`cargo test -p apeireth-gateway --test session_binding_face --locked -j 2` ⇒ 9 passed / 0 failed（均 2026-11-17）；clippy 0 警告；fmt 0 diff；`arch_guard --strict` ⇒ VIOLATIONS 0；反向验伪七组（摘路由接线 / 摘闸接线 / 闸恒拒 / CAS 弱化 / 归档码 403→404 / 显式 null 不当解绑 / deny 不写审计）均 RED→GREEN |
| B3b-2 | 装配层作用域翻译 + 写回接线（`MemoryScopeTarget` → canonical `MemoryScope`） | **PRODUCTION WIRED**（翻译层 + 绑定会话写回链路已接生产装配；⚠️ 未做 live E2E；profile 分区路径保持；恢复补写未翻译；`memory_scope_binding` 缺位 ⇒ 已绑定会话显式零写入） | ✅ 已完成（提交 `c137f5a9d`） | `pwsh scripts/cargo.ps1 -Slot base -Line backend-contracts-b3b2 -CargoArgs "test -p apeireth-runtime-assembly --test memory_scope_wiring"` ⇒ 6 passed / 0 failed（含真实 runtime 回合）；`cargo test -p apeireth-runtime-assembly --lib memory_scope_wire` ⇒ 5 passed / 0 failed（均 2026-11-18）；clippy 0 警告；fmt 0 diff；`arch_guard --strict` ⇒ VIOLATIONS 0；`audit_unwired.py` ⇒ 本批新符号 0 orphan；反向验伪四组（翻译层 Persona→Session / none 回退 Session / 绑定路径摘掉 / 失效回退 Session）均 RED→GREEN；合并 upstream/main（提交 `553cad251`）后按官方修法 `check_crate_budget.py --gen` 重建基线（34 crate · 600371 行），`--check` ⇒ exit 0 |
| B3b-3 | persona 注入（绑定命中 persona 进 system 种子槽） | **PRODUCTION WIRED**（真实 runtime 回合的空会话 system 槽消费绑定 persona；⚠️ 未做 live E2E；仅空会话种子槽，非空会话不追溯；persona 只进 system 文本，不碰权限/工具面） | ✅ 已完成（本批两个提交：feat `7d18e4c32` + docs/claims `f60642cf7`） | `cargo test -p apeireth-runtime --test turn_persona_injection --locked -j 2` ⇒ 7 passed / 0 failed（真实 `Runtime::execute` + 捕获 provider）；`cargo test -p apeireth-runtime --lib --locked -j 2` ⇒ 195 passed / 0 failed；`--test turn_admission_gate` ⇒ 9 passed / 0 failed；`--test contact_persona_authority` ⇒ 6 passed / 0 failed（均 2026-11-18）；clippy 0 警告；fmt 0 diff；`arch_guard --strict` ⇒ VIOLATIONS 0；`audit_unwired.py --check` ⇒ 0 新增孤儿；反向验伪三组（seeded_system 分支改 None / admission persona 丢弃 / persona 改后置）均 RED→GREEN |
| C1 | 规范流帧 + 后端流面 + 生产装配（身份 / sequence / 重放 / resync） | **PRODUCTION WIRED**（流式回合生产路径；⚠️ 缓冲进程内，重启即失） | ✅ 已完成（领域层提交 `e397cc687`，PR #135；生产装配 C2-3 批） | `cargo test -p apeireth-protocol --test canonical_stream_frames --locked -j 2` ⇒ 9 passed / 0 failed；`cargo test -p apeireth-runtime --test canonical_stream_registry --locked -j 2` ⇒ 11 passed / 0 failed；`cargo test -p apeireth-gateway --test canonical_stream_face --locked -j 2` ⇒ 7 passed / 0 failed（均 2026-11-17）；C2-3 起 `cargo test -p apeireth-runtime --test c1_stream_wiring --locked -j 2` ⇒ 5 passed / 0 failed（真运行时回合四类终帧 + 非流式不注册）；clippy 0 警告；fmt 0 diff；`arch_guard --strict` ⇒ VIOLATIONS 0；反向验伪 C2-3 组（摘实时 delta / 审批映射 / 停止优先）均 RED→GREEN |
| C2-0 | 错误码目录对齐（24 码封闭集） | — | ✅ 已完成（提交 `80edf9832`） | 错误帧目录随 C2-1 批次复核（24 码不变）；无独立新端点 |
| C2-1 | operation 幂等去重（`x-apeireth-operation` + SHA-256 载荷摘要 + 进程内 1024 条表） | **PRODUCTION WIRED**（HTTP 面；⚠️ 幂等表进程内，重启即失） | ✅ 已完成（提交 `da81ac1be`） | `cargo test -p apeireth-runtime -p apeireth-gateway --locked` ⇒ 72 suites / 739 passed / 0 failed（runtime lib 188、gateway 幂等 HTTP 4）；clippy 0 警告；fmt 0 diff；`arch_guard --strict` ⇒ VIOLATIONS 0；反向验伪 2 组（Replay 改重执行 / 摘摘要比对）均 RED→GREEN |
| C2-2 | stop/cancel 端点 + 回合级取消注入 + 流式 chunk id 对齐 | **PRODUCTION WIRED**（HTTP 面＋回合级注入；⚠️ 未做真实回合 E2E） | ✅ 已完成（本批提交；C2-2 批） | `cargo test -p apeireth-protocol --test canonical_stream_frames --locked -j 2` ⇒ 9 passed / 0 failed；`cargo test -p apeireth-runtime --test turn_cancellation_face --locked -j 2` ⇒ 6 passed / 0 failed；`cargo test -p apeireth-gateway --test stop_endpoint_face --locked -j 2` ⇒ 5 passed / 0 failed；`cargo test -p apeireth-gateway --lib --locked -j 2` ⇒ 145 passed / 0 failed；clippy 0 警告；fmt 0 diff；`arch_guard --strict` ⇒ VIOLATIONS 0；反向验伪 3 组（摘 `token.cancel()` / 摘 stopped 队列查询 / 摘 `cancellation_for` 注入）均 RED→GREEN；⚠️ 注册表进程内、重启即失；`stopped` 终帧归 C2-3 |
| C2-3 | 审批终帧 + C1 生产装配 | **PRODUCTION WIRED**（流式回合生产路径；⚠️ C1 缓冲与停止摘要均为进程内，重启即失；审批恢复不续流） | ✅ 已完成（本批提交；C2-3 批） | `cargo test -p apeireth-runtime --test c1_stream_wiring --locked -j 2` ⇒ 5 passed / 0 failed（completed 多 delta / approval / failed / stopped E2E + 非流式不注册）；`cargo test -p apeireth-runtime --lib --locked` ⇒ 195 passed / 0 failed（含 `stream_wiring` 4 + `turn_cancellation::stop_summary` 3）；`cargo test -p apeireth-protocol --test canonical_stream_frames --locked -j 2` ⇒ 9 passed / 0 failed；`cargo test -p apeireth-runtime --test turn_cancellation_face --locked -j 2` ⇒ 6 passed / 0 failed；`cargo test -p apeireth-gateway --test stop_endpoint_face --locked -j 2` ⇒ 5 passed / 0 failed；clippy 0 警告；fmt 0 diff；`arch_guard --strict` ⇒ VIOLATIONS 0（`execute.rs` 5006 / `canonical_entry.rs` 2291 未触碰）；反向验伪 3 组（摘实时 delta / 审批终帧映射 / 停止优先）均 RED→GREEN；⚠️ C1 只注册流式入口已铸定 request id 的回合，非流式与隐式 id 不注册 |

**四级口径说明**：A1 达 **IMPLEMENTED**（**仅领域层**：未持久化、无 HTTP 面、未接生产路径 ⇒ **不是** PRODUCTION WIRED）；
A2 达 **IMPLEMENTED**（HTTP 面 + 端口 + 乐观并发 + 审计 + 能力入册 + 错误码 + **生产组合根真实注入**
⇒ `apeireth gateway serve` 可用；但**未持久化**（A3）、失效联动未做 ⇒ 仍**不是** PRODUCTION WIRED）；
B1 达 **IMPLEMENTED**（**仅领域层**：无 HTTP 面、无绑定、无审计 ⇒ **不是** PRODUCTION WIRED）；
B2 达 **IMPLEMENTED**（**仅领域层**：`canonical::agent_binding` + `Session.agent_binding`；
但 **`admit_turn` 无调用方** ⇒ `domain_plane` 默认域回落仍是未绑定会话的现役路径 ⇒ **不是** PRODUCTION WIRED）；
B3b-1 达 **PRODUCTION WIRED**（会话绑定面路由已 merge ＋ 组合根装配 registry ＋ **回合入口闸已接线** ⇒
`admit_turn` 从「无调用方」变成「每回合现查现校验」；⚠️ **未做 live E2E** ⇒ 不到 HARDWARE VALIDATED）；
B3b-2 达 **PRODUCTION WIRED**（装配层 `translate_scope` 把 `MemoryScopeTarget` 逐字段翻译成 canonical `MemoryScope`
＋ `MemoryWritebackModule` 写回链路已接生产装配：profile 分区与未绑定会话仍走 `Session`（逐字节不变），
已绑定会话按合同落 `User/Persona/Session`，失效 / `none` / 装配面缺位 ⇒ 整段不写（fail-closed，**绝不回退 `Session`**）；
⚠️ **未做 live E2E** ⇒ 不到 HARDWARE VALIDATED；**恢复补写（recovery-drain）未翻译**，仍按会话锚点重放；
`check_crate_budget.py --gen` 已按官方修法重建基线（34 crate · 600371 行），`--check` ⇒ exit 0）；
B3b-3 达 **PRODUCTION WIRED**（`admit_bound_binding`/`admit_turn` 返回 `BoundTurnAdmission` 携带绑定 persona，
`turn_admission::seeded_system` 复用 `DomainRuntimeContext::seeded_system` 语义，`execute.rs` 空会话 system 槽优先种 persona；
persona 只进 system 文本，**不碰** Permission/tool_scope/审批/workspace scope；⚠️ **仅在空会话种子槽生效**（非空会话不追溯），
未做 live E2E ⇒ 不到 HARDWARE VALIDATED）；
C2-1 达 **PRODUCTION WIRED**（HTTP 面：`/v1/chat` 幂等键 + 摘要 + 进程内 1024 条表；⚠️ 表**进程内、重启即失、多进程不共享**，
无头不幂等、无显式 session 不去重，回放不重放流式 delta ⇒ 不是 HARDWARE VALIDATED）。
C2-2 达 **PRODUCTION WIRED**（stop 端点 + 回合级取消注入 + 流式 chunk id 对齐；⚠️ 未做真实模型回合 E2E，
注册表进程内、重启即失；stop 只阻断后续工具派发，`stopped` 终帧已由 C2-3 在该回合结束时发布）。
C2-0 只做错误码目录对齐。
C2-3 达 **PRODUCTION WIRED**（流式回合的 provider delta 实时写 C1 权威流，结束时恰发一帧
completed/approval/failed/stopped 终帧；5 条真运行时 E2E 钉住四类终帧与非流式不注册；
⚠️ **C1 缓冲与停止摘要都是进程内，重启即失、多进程不共享**；只注册流式入口已铸定 request id 的回合，
审批恢复不续流 ⇒ 不是 HARDWARE VALIDATED）。
C1 达 **PRODUCTION WIRED**（C2-3 起生产流式回合产生真实 C1 流；帧族 + 运行时权威缓冲 + 网关投影面
齐备，已登记进 `docs/gateway-api-contract.md` 的〔PRODUCTION WIRED〕端点表；⚠️ 权威缓冲不持久化）。
`DEFAULT ENABLED` 是横切属性，不随阶梯走。

## 二、2026-11-17 开工前现状核查（只读调研，非交付）

三份只读调研已收齐，要点已并入设计稿 §进度与验收记录：

- **Workspace**：本仓**无** Workspace 领域类型（`workspace_id` / `root_kind` / `permission_scope` / `can_select` 全仓 0 命中）；
  根目录现状＝**进程单根**（`crates/adapters/cli/src/lib.rs:2060` 的 `current_dir()`）；
  缺口 14 项（无领域类型 / 无多根 / 无路由 / 无冲突码 / 无权限枚举 / 无 Project 资源 / 无成果版本资源 /
  无 operation id / 无安全展示名规则 / 审计无对象维度 / 审批无工作区绑定 / 无持久化 / 能力清单未登记 / 工作线未登记）。
- **Contact / Persona**：本仓**无** contact 实体、**无** agent registry、**无** `/v1/agents*` 路由、**无** 合同版 `memory_scope`；
  现役人格注入**只有一个槽**（`crates/engine/runtime/src/canonical/domain_plane.rs:124-136`、`:146-152`）；
  缺口 17 项。
- **C1 真流式**：后端**已有真·逐 token 流**（provider → runtime → gateway 三段齐备），
  缺的是**帧身份**与**权威收口**；现有 OpenAI 兼容 SSE 的 `[DONE]` 语义与本合同冲突；缺口 14 项。
- **宿主边界（本批关键澄清）**：`/bridge/*` 是私有前端仓宿主自己的本地 HTTP 面，**不是**后端路由
  （证据见设计稿 §裁决 1）⇒ ⇒ 后端不实现 `/bridge/*`，只提供后端原生的规范流面，宿主映射过去。

## 三、还差什么（逐条可判定）

1. **A1 已完成，但只到领域层**：工作区与 session/task/审批的**绑定**尚未开工；
   多根解析（运行时仍是进程单根）与自动过期回收也未做。持久化已在 A3 落地。
2. **A2 已完成，持久化已在 A3 落地**：HTTP 面、409/403 分类码、审计事件、能力入册、组合根真实注入均已落地；
   A3 又把登记事实落到本地文件系统（跨重启可恢复）。
   **仍缺**：多根/临时根/预设根的受管根装配、自动过期回收、
   **workspace→session/task/审批/流的失效联动**、多用户归属建模、
   `restore_exact` 的 HTTP 端点与生产调用点。
3. **B1 已完成，但只到领域层**：缺 `/v1/agents/contacts*` HTTP 面（B3a）、`workspace_binding`（B3a）、
   归档审计事件（B3a）。记忆作用域映射与会话绑定已在 B2 落地（领域层）。
4. **B2 已完成，但只到领域层**：合同版 `memory_scope` 映射、会话 agent 绑定、「绑定不匹配即拒绝且不重放」的载体
   都已落地（`canonical::agent_binding` + `Session.agent_binding`，21 条契约测试 + 5 组反向验伪）。
   **已补齐**：回合入口闸（B3b-1）、canonical `MemoryScope` 的装配层翻译/写回（B3b-2）与 persona 注入（B3b-3，仅空会话种子槽）。
   **仍缺**：HTTP 级归档语义/审计（B3a 的后续项）。
5. **B3a 已完成**（提交 `43291e4ea`）：联系人 HTTP 面 + `workspace_binding` 服务端校验 + 4 个新错误码 + 能力入册 + CLI 真落盘构造。
   **B3b-1 已完成**（提交 `b7bcbacc0`）：会话绑定面 `GET/POST /v1/sessions/{id}/agent-binding` ＋ **回合入口闸**（`admit_turn` 从「无调用方」变成「每回合现查现校验」）
   ＋ 2 个新错误码（`agent_binding_rejected` / `session_binding_conflict`）＋ 审计事件 `gateway.session_binding_action`。
   ⚠️ 四级口径 **PRODUCTION WIRED**（未做 live E2E ⇒ 不到 HARDWARE VALIDATED）；绑定变更不写 SessionEvent；多进程部署 registry 仍会分叉。
   **B3b-2 已完成**（提交 `c137f5a9d`，合并提交 `553cad251`）：装配层 `MemoryScopeTarget` → canonical `MemoryScope` 翻译
   ＋ 绑定会话写回；已绑定但装配缺位/计划拒绝/绑定失效 ⇒ 整段不写（fail-closed，绝不回退 `Session`）。
   **B3b-3 已完成**（本批两个提交：feat `7d18e4c32` + docs/claims `f60642cf7`）：绑定命中 persona 经 `BoundTurnAdmission` 进入回合，`turn_admission::seeded_system` 复用
   `DomainRuntimeContext::seeded_system` 语义，`execute.rs` 空会话 system 槽优先注入 persona（persona + 两换行 + request.system）；
   **只改 system 文本，不碰权限/能力/工具面**；7 条真实 runtime 回合测试 + 3 组反向验伪；⚠️ **仅空会话种子槽**（非空会话不追溯），
   未做 live E2E ⇒ 不到 HARDWARE VALIDATED。
6. **C1 已完成**（提交 `e397cc687`，PR #135）：帧族 ＋ 运行时权威缓冲注册表 ＋ 网关后端原生面
   `GET /v1/sessions/{sid}/requests/{rid}/stream`（`Last-Event-ID` 优先 → `?after_sequence=`，
   先订阅后重放 ＋ 交界去重；超窗 ⇒ 发 `resync_required` 事实帧后关闭通道）。
   ⚠️ 按裁决 15：**不实现 `/bridge/*`**（那是私有前端仓宿主自己的本地面）。
   **生产装配已由 C2-3 落地**：流式回合在 `execute_outcome_with_sink` 注册 C1 流、实时转发 delta、
   结束时发布唯一终帧（completed/approval/failed/stopped），`c1_stream_wiring` 5 条真运行时 E2E 钉住。
   **仍缺**：权威缓冲持久化与跨进程共享（重启即失）、审批恢复后继续同一条流（客户端须重发回合）；
   ~~operation 级去重（C2-1 已完成）~~、~~stop/cancel（C2-2 已完成）~~。
7. **C2 已完成**（C2-0 `80edf9832` / C2-1 `da81ac1be` / C2-2 `3bb73032f` / C2-3 `ccb62e678`，PR #139 已合主干 `27e3872f0`）：
   operation 幂等去重、stop 幂等端点与回合级取消注入、审批终帧与 C1 生产装配均已落地；
   ⚠️ 幂等表 / C1 缓冲 / 停止摘要均进程内（重启即失、多进程不共享）；stop 不中断模型生成、不撤销已发生副作用；
   审批恢复续流与缓冲持久化仍未做；未做 live E2E ⇒ 不是 HARDWARE VALIDATED。（裁决与批次拆分见 `brief_c2.txt`。）
8. ~~能力清单未登记~~ ⇒ **已做**（A2：`workspaces.read|workspaces.write`；B3a：`contacts.read|contacts.write`，
   均登记进 `crates/adapters/gateway/src/panels.rs`）。
9. ~~错误码未登记~~ ⇒ **已做**（A2：`workspace_not_found` / `workspace_conflict` / `workspace_revoked`；
   B3a：`contact_not_found` / `contact_conflict` / `contact_archived` / `contact_binding_conflict`；
   均进 `crates/adapters/gateway/src/error_frame.rs` 并各带 solution）。
10. **未决 5 条**（设计稿 §未决）：`expected_context_key` 定义域、`incarnation` 生命周期、成果版本对象、`workspace_binding` 语义、联系人 `capabilities` 到 `Permission` 的映射粒度。
11. **跨线协调未完成**：前端草案的 `root_kind` 与 `crates-ui/.../governance_tab.rs:867` 的轨迹 span kind 同名不同义，须与前端线对齐命名。

## 四、守卫（机器判对错）

```text
cargo test -p apeireth-runtime --locked -j 2
cargo test -p apeireth-gateway --locked -j 2
cargo test -p apeireth-protocol --locked -j 2
cargo test -p apeireth-cli --locked -j 2
cargo clippy -p apeireth-gateway -p apeireth-runtime -p apeireth-protocol -p apeireth-cli --all-targets --locked -j 2 -- -D warnings
python3 scripts/docs_truth.py --verify --check
```

各批次落地后，**每个批次必须补一条专属守卫**（该批的集成测试名），并在本页 §一 的「证据」列写明命令与实测输出。
本批已落的专属守卫：

```text
cargo test -p apeireth-runtime --test workspace_contract --locked -j 2       # A1
cargo test -p apeireth-gateway --test panel_workspaces_face --locked -j 2    # A2
cargo test -p apeireth-runtime --test workspace_persistence --locked -j 2   # A3
cargo test -p apeireth-cli --test workspace_persistence_chain --locked -j 2  # A3（真实落盘 + 真路由）
cargo test -p apeireth-core --test agent_contact --locked -j 2              # B1
cargo test -p apeireth-runtime --test contact_persona_authority --locked -j 2 # B1
cargo test -p apeireth-runtime --test agent_binding_contract --locked -j 2   # B2
cargo test -p apeireth-gateway --test panel_contacts_face --locked -j 2     # B3a
cargo test -p apeireth-cli --test contact_real_chain --locked -j 2          # B3a（真落盘 + 真路由）
cargo test -p apeireth-protocol --test canonical_stream_frames --locked -j 2 # C1（帧族 wire 契约）
cargo test -p apeireth-runtime --test canonical_stream_registry --locked -j 2 # C1（权威缓冲注册表）
cargo test -p apeireth-gateway --test canonical_stream_face --locked -j 2    # C1（后端原生 SSE 面）
cargo test -p apeireth-gateway --test operation_idempotency_http --locked -j 2 # C2-1（幂等键 + 摘要 + 重放）
cargo test -p apeireth-runtime --test turn_cancellation_face --locked -j 2   # C2-2（回合级取消注册表）
cargo test -p apeireth-gateway --test stop_endpoint_face --locked -j 2       # C2-2（stop 端点幂等/404/400）
cargo test -p apeireth-runtime --test c1_stream_wiring --locked -j 2         # C2-3（C1 生产装配 + 四类终帧）
cargo test -p apeireth-runtime --test turn_admission_gate --locked -j 2    # B3b-1（回合入口只读闸）
cargo test -p apeireth-gateway --test session_binding_face --locked -j 2   # B3b-1（会话绑定面 + 审计 + CAS）
python3 scripts/arch_guard.py --strict                                      # 尺寸棘轮（VIOLATIONS 必须为 0）
python3 scripts/audit_unwired.py --check                                   # 孤儿 API 棘轮（只减不增）
```

## 五、口径纪律

- 本页写「已实现 / 已接线」必须同时给出**四级口径**与**实测命令 + 输出**；没有输出就写「未实测」。
- 批次未完成时进度页写「❌ 未开始 / 未实测」是**守卫友好**的形态（可判定），不要写成散文。
- 引用现状只引文件（本页 / 设计稿 / 源码 file:line / 门禁输出），**不引对话摘要**。

## 六、本轮（三条工作线）过程中被门禁抓到的真实缺陷（不是演练）

三条都登记在 `docs/04-internal/ui-engine-defect-ledger.md`（台账编号只增不改）；**E-199 与 E-200 落在本分支**，另两条分别落在磁盘纪律线（PR #131）与 harness 裁决线（PR #133）：

| 编号 | 现象 | 根因 | 修在哪 |
|---|---|---|---|
| E-197 | fast gate 把契约类 `.toml` 当 Rust 改动，并把三方依赖名塞进 `cargo -p` ⇒ 撞多版本依赖报 `error: specification 'bit-set' is ambiguous` | `scripts/verify_changed.py` 的文件分类只按后缀，不按「是不是清单」 | 磁盘纪律线 PR #131（提交 `6aaff28ec`：`classify_files()` 分类 + 影响面限制在 35 个 workspace 成员内）＋新增回归守卫 `scripts/check_fast_gate_classifier.py`（11 条断言） |
| E-198 | 合并裁决把「适用却没跑」判成 BLOCKED，但**阶段选择只看有没有 Rust 改动** ⇒ docs/contract/ci-only PR 必假红 | `scripts/harness.py` 的阶段集合只由影响级别决定，与门禁 scope 适用性是两套口径 | harness 裁决线 PR #133（提交 `9e438190f`：阶段集合＝影响级别阶段 ∪ 适用且 required 的门禁所在阶段）＋回归测试 `scripts/test_harness_gate_status.py` 新增 5 例 |
| E-199 | fast gate 的 `self_status_e2e` 偶发假红（`protect: Conflict { id: "e1", expected: 0, actual: 1 }`） | 测试临时路径用 `std::process::id()` 拼名 ⇒ Windows 回收进程号 ⇒ 上一轮残留被当成本轮事实 | **本分支**（提交 `2dd369247`：本进程首次用到该路径时清一次残留；**不放宽断言**）。⚠️ 同类写法全仓 236 处，属长尾 |
| E-200 | CI 判 `docs/status/wiring.yaml drift` ⇒ 门禁假红（本机 `--verify` 却绿） | `docs_truth.py` 的**两条解析路径**（有/无 PyYAML）给出不同值：最小解析器把引号标量内部的 ` #`（正文里的 `PR #125` 链接）当注释截断 ⇒ 本机生成的字节 ≠ CI 生成的字节 ⇒ 60 条声明里 53 条标量值两边不同 | **本分支**：`docs_truth.py` 注释剥离改引号感知（`_strip_comment`）＋ `wiring_report.py` 的 `--gen`/`--verify` 前置 `claims_parse_agreement()`（有 PyYAML ⇒ 逐字段对账；无 ⇒ 对账「引号内原文」）⇒ 「两条路径不一致」成为门禁可判定的事实。**首修不完整 ⇒ CI 抓到第二根因**（最小解析器没还原 YAML 双引号转义 `\"`）⇒ 补 `_unescape_yaml_double` ＋ **字节级跨环境对账**（子进程屏蔽 yaml 后重算并比字节）＋ 本机装 PyYAML ⇒ RED 可在本地复现 |

**教训**：四条都不是「演练出来的红」，是本批真的撞上的门禁红；修法一律落在**机制**（分类口径 / 阶段口径 / 隔离口径 / 解析口径），不是落在「把断言放宽」。**共同形态**：**「同一份真源 + 同一段代码」不等于「同一份事实」** —— 判定口径只要有两条路径，就必须让它们逐字段对账。

## 七、尺寸棘轮：本批撞上的第五个门禁红（arch-strict）

- **现象**：PR #132 的合并裁决收敛到唯一红＝`arch-strict FAIL`（`scripts/arch_guard.py --strict` ⇒ `VIOLATIONS: 4`）。
- **根因**：本批（A1/A2/A3/B1/B2/B3a）的新增代码把 4 个**既有**文件推过了架构尺寸棘轮：
  `crates/adapters/cli/src/gateway_panels.rs` 1526→1805、`crates/adapters/cli/src/lib.rs` 3788→3809、
  `crates/adapters/gateway/src/panels.rs` 1623→1668、`crates/foundation/core/src/lib.rs` 2483→2487
  （行数用 `len(text.splitlines())` 计，与 `scripts/arch_guard.py:201-210 scan_sizes` 同口径；
  基线与 `upstream/main` 逐字一致 ⇒ **不是**上游欠账，**是本批的账**）。
- **机制**：`scripts/arch_guard.py:662-686` 里 `loc > old` ⇒ W1 size-grow 违规（`--strict` 退 1）；
  `loc < old` ⇒ 只记 STALE（可缩表）；`MIN_SIZE_TRACK = 1200` ⇒ **新建文件不进棘轮** ⇒ ⇒ 搬迁是合规手段。
- **合规路径（三条里只允许一条）**：
  ① 抬 `scripts/arch-baseline.txt` ⇒ **禁止**（基线只许缩：`docs/architecture/README.md:61`、`docs/architecture/line-coordination.md:21`）；
  ② 登记 size 例外 ⇒ **不可用**（`docs/architecture/exceptions.toml` 活跃额度已满＝2，`contract.toml` `active_exceptions_max = 2` ⇒ 必触发 W10）；
  ③ **行为等价搬迁，把 4 个文件压回基线以内** ⇒ 采用；**先例＝PR #126**
  （`docs/design/backend-hardening-completion.md:119-123`：超棘轮 ⇒ 不抬基线 ⇒ 压回基线以内的行为等价 refactor）。
- **当前状态**：✅ **已压回并合并**（PR #132；压回提交 `ea4acf270e9`）：
  `cli/gateway_panels.rs` 1805→1464（基线 1526）、`cli/lib.rs` 3809→3786（基线 3788）、
  `gateway/panels.rs` 1663→1489（基线 1623）、`core/lib.rs` 2487→2481（基线 2483）；
  新模块 `trait_panel.rs` 99 行 / `contract_panels.rs` 310 行 / `capabilities.rs` 193 行 ⇒ 全 < 1200 不进棘轮；
  `python scripts/arch_guard.py --strict` ⇒ **VIOLATIONS: 0**（STALE 5 条为「比基线小」的记录项）。
- **B3b-1 批的同一约束（已按同一口径处理）**：`execute.rs` 5010 顶格、`runtime.rs` 1331 顶格、
  `canonical_entry.rs` 2291 顶格、`cli/lib.rs` 3788 顶格（实测 3786）⇒ 靠**行为等价搬迁**腾行
  （`RuntimeBuilder` 整块搬进 `runtime_builder.rs`、`classify_runtime_error` 搬进 `runtime_error_map.rs`、
  `expire_active_approval_if_needed` 搬进 `turn_admission.rs`）⇒ 搬迁后 `execute.rs` 4960、`runtime.rs` 774（已 < 1200 脱离棘轮）、
  `canonical_entry.rs` 2240、`cli/lib.rs` 3788（**正好等于基线，0 余量**）；新文件 667/115/938/68/94 行全 < 1200；
  `arch_guard --strict` ⇒ **VIOLATIONS: 0**。
- **诚实边界**：压回是**结构搬迁**，不改任何端点/错误码/审计/能力清单/DTO/状态机语义；
  `scripts/crate-budget-baseline.json` 会因「每 crate 文件数变化」需按官方修法① `--gen` 重写，并在提交正文说明增长来源。
  ⚠️ 另有一条同源的棘轮：文档路径基线 `scripts/doc-path-baseline.txt` 只减不增 ⇒ C1 落地后那条「设计稿引用尚不存在的路径」必须删（PR #135 提交 `af89f76b6`）。
