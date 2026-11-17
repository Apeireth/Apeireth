# 后端接线合同进度页（工作区 / 联系人·人设·会话绑定 / C1 真流式）

[Document-Meta]
Document: docs/status/backend-contracts-20261117.md
Class: STATUS
Status: 🟢 活跃
Last-Modified: 2026-11-17
Author: lead

> **用途**：本工作线「做到哪了」的**唯一权威**（`AGENTS.md` §0.2：引用状态只引文件）。
> 目标形态见[设计稿](../design/backend-contracts-20261117.md)。

## 一、批次状态（对照设计稿 §批次与验收）

| 批次 | 目标 | 四级口径 | 状态 | 证据 |
|---|---|---|---|---|
| A1 | 工作区领域类型 + 受管根 + revision CAS | **IMPLEMENTED**（仅领域层） | ✅ 已完成（提交 `91888e27c`） | `cargo test -p apeireth-runtime --test workspace_contract --locked -j 2` ⇒ 11 passed / 0 failed（2026-11-17）；clippy 0 警告；fmt 0 diff；反向验伪两组（摘 revision 递增 / 摘安全展示名）均 RED→GREEN |
| A2 | 工作区 HTTP 面 + 审计 + 会话/项目/审批绑定 | **IMPLEMENTED**（HTTP 面已接线；⚠️ 未持久化） | ✅ 已完成（提交 `3854c01a8`） | `cargo test -p apeireth-gateway --test panel_workspaces_face --locked -j 2` ⇒ 13 passed / 0 failed；`cargo test -p apeireth-cli --test workspace_real_chain --locked -j 2` ⇒ 5 passed / 0 failed（均 2026-11-17）；clippy 0 警告；fmt 0 diff；fast gate FAST GATE PASSED；反向验伪三组（摘修订号比对 / 摘展示名安全闸 / 409→400）均 RED→GREEN |
| B1 | 联系人 / Agent identity + 人设（字节上限 / 控制字符 / 版本号） | **IMPLEMENTED**（仅领域层） | ✅ 已完成（提交 `5bfb959d7`） | `cargo test -p apeireth-core --test agent_contact --locked -j 2` ⇒ 16 passed / 0 failed；`cargo test -p apeireth-runtime --test contact_persona_authority --locked -j 2` ⇒ 6 passed / 0 failed（均 2026-11-17）；clippy 0 警告；fmt 0 diff；反向验伪五组（persona_revision 不递增 / 归档可复活 / 上限被绕过 / 入面采信客户端 capabilities / 人设被当授权依据）均 RED→GREEN |
| B2 | 记忆作用域映射 + 会话 agent 绑定 + 归档语义 | — | ❌ 未开始 | 无 |
| B3 | 联系人 HTTP 面 + `workspace_binding` + 会话绑定面 | — | ❌ 未开始 | 无 |
| C1 | 规范流帧 + 后端流面（身份 / sequence / 重放 / resync） | — | ❌ 未开始 | 无 |
| C2 | 单次 canonical dispatch + stop 幂等 + 审批终帧 | — | ❌ 未开始 | 无 |

**四级口径说明**：A1 达 **IMPLEMENTED**（**仅领域层**：未持久化、无 HTTP 面、未接生产路径 ⇒ **不是** PRODUCTION WIRED）；
A2 达 **IMPLEMENTED**（HTTP 面 + 端口 + 乐观并发 + 审计 + 能力入册 + 错误码 + **生产组合根真实注入**
⇒ `apeireth gateway serve` 可用；但**未持久化**（A3）、失效联动未做 ⇒ 仍**不是** PRODUCTION WIRED）；
B1 达 **IMPLEMENTED**（**仅领域层**：无 HTTP 面、无绑定、无审计 ⇒ **不是** PRODUCTION WIRED）；
B2 / B3 / C1 / C2 **尚无任何一项**达到 IMPLEMENTED；批次完成前不得在文档或回复里写「已实现 / 已接线」。
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

1. **A1 已完成，但只到领域层**：持久化在 A3；工作区与 session/task/审批的**绑定**尚未开工；
   多根解析（运行时仍是进程单根）与自动过期回收也未做。
2. **A2 已完成，但未持久化**：HTTP 面、409/403 分类码、审计事件、能力入册、组合根真实注入均已落地；
   **仍缺**：跨重启恢复（A3）、多根/临时根/预设根的受管根装配、自动过期回收、
   **workspace→session/task/审批/流的失效联动**、多用户归属建模。
3. **B1 已完成，但只到领域层**：缺 `/v1/agents/contacts*` HTTP 面（B3）、`workspace_binding`（B3）、
   记忆作用域映射与会话绑定（B2）、归档审计事件（B3）。
4. **B2 未开工**：无合同版 `memory_scope` 映射、无会话 agent 绑定面、无「绑定不匹配即拒绝且不重放」载体、无 HTTP 级归档语义。
5. **B3 未开工**：无联系人 HTTP 面、无 `workspace_binding` 承载、无会话绑定面。
6. **C1 未开工**：无 `apeireth.c1.delta.v1` / `apeireth.c1.final.v1` 帧 schema、无后端规范流面、无 sequence / 重放 / `resync_required`。
7. **C2 未开工**：无 operation 级单次 dispatch 去重、无 stop/cancel 幂等端点、无审批终帧载体。
8. ~~能力清单未登记~~ ⇒ **已做**（A2：`workspaces.read|workspaces.write` 登记进 `crates/adapters/gateway/src/panels.rs`）；
   联系人能力清单待 B3。
9. ~~错误码未登记~~ ⇒ **已做**（A2：`workspace_not_found` / `workspace_conflict` / `workspace_revoked`
   已进 `crates/adapters/gateway/src/error_frame.rs` 并各带 solution）；联系人专属码待 B3。
10. **未决 5 条**（设计稿 §未决）：`expected_context_key` 定义域、`incarnation` 生命周期、成果版本对象、`workspace_binding` 语义、联系人 `capabilities` 到 `Permission` 的映射粒度。
11. **跨线协调未完成**：前端草案的 `root_kind` 与 `crates-ui/.../governance_tab.rs:867` 的轨迹 span kind 同名不同义，须与前端线对齐命名。

## 四、守卫（机器判对错）

```text
cargo test -p apeireth-runtime --locked -j 2
cargo test -p apeireth-gateway --locked -j 2
cargo test -p apeireth-protocol --locked -j 2
cargo clippy -p apeireth-gateway -p apeireth-runtime -p apeireth-protocol --all-targets --locked -j 2 -- -D warnings
python3 scripts/docs_truth.py --verify --check
```

各批次落地后，**每个批次必须补一条专属守卫**（该批的集成测试名），并在本页 §一 的「证据」列写明命令与实测输出。

## 五、口径纪律

- 本页写「已实现 / 已接线」必须同时给出**四级口径**与**实测命令 + 输出**；没有输出就写「未实测」。
- 批次未完成时进度页写「❌ 未开始 / 未实测」是**守卫友好**的形态（可判定），不要写成散文。
- 引用现状只引文件（本页 / 设计稿 / 源码 file:line / 门禁输出），**不引对话摘要**。
