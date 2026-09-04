# Apeireth 2.0 Runtime 微内核化与 Capability 收敛报告

## Executive Summary

本轮把 `apeireth-runtime` 收敛为可独立构建的机制内核，并建立
`Runtime / Assembly → GatewayServices → Capability Manifest → Desktop` 的单一事实链。
具体 Cognitive、Organ、Tool、SQLite Session 装配移入新增的
`apeireth-runtime-assembly`；普通 Desktop 对话默认只走 canonical Gateway/Runtime。

## Before / After

旧的生产关系将 Main Loop、具体 Cognitive/Organ/Tool、SQLite Session 和部分
观测逻辑混在 `apeireth-runtime`，Gateway 再通过 `PanelData`、静态模型/器官数组
和自建 EventBus 补齐 UI。

现在的依赖方向是：

```text
foundation contracts
        ↓
apeireth-runtime                         mechanism kernel
        ↑                                  (ports + registries + events)
apeireth-runtime-assembly                 concrete production wiring
  ├─ apeireth-memory / organ / tools / storage
  └─ cognitive behaviors + tool capabilities + SQLite SessionStore
        ↓
CLI / Gateway / Desktop
```

`apeireth-runtime` 的正常生产依赖不再包含 `apeireth-organ`、
`apeireth-tools-canonical`、`apeireth-storage` 或 `rusqlite`。

## Kernel Boundary

`apeireth-runtime` 只负责：

- Canonical Main Loop、Session state machine、locking、budget/cancellation、approval；
- `BehaviorModule` / `BehaviorRegistry`；
- `CapabilityProvider` / `CapabilityRegistry`；
- Provider contract/router、GovernanceHook、SessionStore 等抽象端口；
- `RuntimeEvent`、`RuntimeEventSink`、Noop/Closure/Composite sink；
- `ExecutionTrace`、fail-closed dispatch、确定性注册顺序、Stop > Retry > Continue。

它可以在 0 Behavior、0 Capability、fake Provider、InMemory SessionStore 下完成普通
chat turn；具体产品实现不再通过 kernel 反向依赖。

## Assembly Boundary

`apeireth-runtime-assembly` 是唯一的生产装配点，负责：

- Memory/Preference/Judge/Council/SelfAssessment/Organ 等 Behavior；
- Filesystem/Search/Repo/Shell/Fetch/MCP 等 Capability Provider；
- Organ LLM bridge、canonical production composition；
- SQLite `SessionStore` 与 memory/tool/storage 的具体连接。

Tool-only 实现不再伪装成 Behavior Module；动态 MCP 注册/卸载进入同一
`CapabilityRegistry`，并立即影响工具投影。

## Behavior / Capability Split

| 类型 | 代表实现 | 注册位置 |
|---|---|---|
| Behavior | Memory Recall/Writeback、Preference Learning、Judge/Council、Self Assessment、Organ cognition、SubLoop behavior | `BehaviorRegistry` |
| Capability | Filesystem、Search、Repo、Shell、Fetch、MCP、插件工具 | `CapabilityRegistry` |
| Kernel mechanism | Main Loop、Approval、Session、Provider routing、Budget、Trace/Event | Runtime kernel |

两个注册表都拒绝重复 ID；Capability 另拒绝 model-facing name 冲突，歧义查找
fail closed。已有 side-call budget、depth limit、PromptOverlay 不进入持久 transcript
和 Stop > Retry > Continue 语义保留。

## Memory Governance

旧路径同时存在裸 `recent_episodes()`、治理 store 和
`memory-flags.jsonl` 三份状态。现在 recall/query/mutation 统一通过
`episode_governance`：

```text
append / query / forget / protect / unprotect / override
              ↓
       episode_governance
              ↓
       Runtime recall + Gateway + Desktop
```

查询会过滤 forgotten 条目并应用 `content_override`。历史
`memory-flags.jsonl` 只在启动时做幂等迁移：仅处理实际存在的 episode，不覆盖已有
新治理状态；迁移后 flags 不再参与生产判断。Mutation 返回 `id/status/protected/
revision/content`，并保留 `rev` 兼容字段。

## Runtime Event Spine

Runtime 在 turn/approval 生命周期发出结构化 `RuntimeEvent`，默认使用 Noop sink，
可组合多个消费者。Gateway 的 SSE、trace archive、audit archive 和 CLI direct turn
都从该事件源派生；Gateway 不再凭 handler 返回结果重复推断 turn 语义。

事件只包含 request/trace/session、结构化状态、计数和安全摘要，不包含 raw CoT、凭据
或工具参数。SSE 仍诚实地以最终文本作为单条 `turn_delta`，不是伪装的 token stream。

## Gateway Ports

`GatewayState` 持有 `GatewayServices`，按 bounded context 注入：

`SessionQuery`、`MemoryQuery`、`MemoryCommand`、`MemoryGovernanceCommand`、
`ToolCatalogQuery`、`TraceQuery`、`TraceCommand`、`AuditQuery`、`AuditCommand`、
`GrantQuery`、`GrantCommand`、`ModuleQuery`。

旧 `PanelData` 只保留给旧嵌入者的兼容 adapter，不再进入生产 `GatewayState` 的领域
依赖。兼容 `/v1/panel/*` URL 保留，内部走上述 ports。

## Capability Matrix

实际状态来自 `/v1/apeireth/capabilities`，而非本报告或前端静态数组。

| Capability ID | supported | available | Source | Frontend surface |
|---|---:|---:|---|---|
| `health`, `runtime.snapshot.read` | true | true | Gateway + Runtime | RuntimeModal/bootstrap |
| `models.list`, `providers.list`, `chat.completions` | true | 取决于 Provider Router | live providers/models | Settings/Conversations |
| `sessions.read` | 取决于 SessionQuery | 同 supported | SessionQuery | Conversations |
| `memory.read/write/forget/protect/unprotect/graph.read` | 取决于 governed memory ports | 同 supported | Memory ports + `episode_governance` | Memory |
| `tools.list` | 取决于 ToolCatalogQuery | 同 supported | CapabilityRegistry + plugins | Tools |
| `permissions.approval.read`, `permissions.approval.resolve` | true | true | Runtime Approval protocol | Tools |
| `approvals.read`, `approvals.resolve` | true | true | compatibility aliases (`alias_of` the permissions.approval.* ids) | Tools |
| `permissions.grants.read`, `permissions.revoke` | 取决于 Grant ports | 同 supported | Grant ports | Tools |
| `trace.read`, `audit.read`, `activity.sse` | 取决于对应 ports/Event Spine | 同 supported | Trace/Audit/Event sinks | Activity |
| `organs.list`, `modules.list` | 取决于 ModuleQuery | 同 supported | BehaviorRegistry projection | RuntimeModal |

`supported=true, available=false` 表示能力已实现但当前 Provider、凭据或装配不可用；
客户端显示后端 `reason`。未知能力和 manifest 请求失败不推测为已支持。

## Frontend Migration

活动 UI 源码位于本仓库 `frontend/companion-desktop/`（从本机 `apeireth-ui` 提取后迁回 in-tree，避免依赖 sibling checkout）：

- Conversations：以 `sessions.read` gate ledger；不可用显示原因并停止请求。
- Memory：分别 gate read/write/forget/protect/unprotect/graph；使用后端真实 revision/state。
- Tools：从 live tools catalog 展示 CapabilityRegistry；approval/grants 走真实端点。
- Activity：trace/audit/SSE 由统一 Event Spine 驱动，不展示 raw CoT 或私有记忆内容。
- RuntimeModal：读取 health、snapshot、Provider/Model、Behavior/Organ、tool count。
- Settings：Provider 配置默认仅服务于 Gateway；`debugDirect === true` 才允许开发者
  raw-provider bypass，并明确绕过 Runtime 的语义。

递归配置清理会移除 `apiKey`、`api_key`、`masterToken`、`master_token` 及 nested
secret 字段；API key 不进入 localStorage。Capability fetch 失败时只保留最小的
health/models/chat/approval-resolve fallback。

## Security Fixes

- CLI 默认绑定 `127.0.0.1`；显式非 loopback bind 会输出 warning。
- 删除默认 `CorsLayer::permissive()`，不再开放敏感 panel/memory API 的任意跨域。
- Credential API 只返回 configured/backend/service 等状态，不返回 secret。
- 普通聊天不直连 OpenAI/Anthropic；raw Provider 只在显式 developer debug path 可用。

## Dependency Audit

`cargo tree -p apeireth-runtime --edges normal --depth 2` 的实际检查结果：

- `apeireth-runtime` 仅保留 core/protocol/plugin/governance/orchestration 等机制依赖；
- 无 `apeireth-organ`、`apeireth-tools-canonical`、`apeireth-storage`、`rusqlite`；
- `sha2` 仍保留在 kernel，仅用于 approval operation fingerprint，不是 Cognitive 依赖；
- workspace 共 17 个 Rust package，assembly 是唯一的 concrete runtime wiring crate。

## Tests

以下命令均在本地实际执行：

```text
cargo check -p apeireth-runtime --all-targets --offline        PASS
cargo check -p apeireth-runtime-assembly --all-targets --offline PASS
cargo check -p apeireth-gateway --all-targets --offline        PASS
cargo check -p apeireth-cli --all-targets --offline            PASS
cargo check --workspace --all-targets --offline                 PASS
cargo test -p apeireth-runtime --all-targets --offline          PASS (45 targets)
cargo test -p apeireth-runtime-assembly --all-targets --offline PASS
cargo test -p apeireth-gateway --all-targets --offline          PASS
cargo test -p apeireth-cli --all-targets --offline              PASS
cargo fmt --package apeireth-runtime --package apeireth-runtime-assembly --package apeireth-gateway --package apeireth-cli -- --check NOT PASS (既有 gateway/runtime 测试文件仍有格式差异)
cargo clippy -p apeireth-runtime --all-targets --offline -- -D warnings PASS
cargo clippy -p apeireth-gateway --all-targets --offline -- -D warnings PASS
python scripts/check_no_legacy_deps.py                           PASS (17 members, 0 violations)
node tests/run-all.mjs                                           PASS (7/7)
node_modules/.bin/svelte-check.CMD --tsconfig ./tsconfig.json   PASS (0 errors, 5 warnings)
node_modules/.bin/vite.CMD build                                 PASS
cargo check --workspace --all-targets --offline (src-tauri)       PASS
```

`cargo test --workspace --all-targets --offline` 已实际尝试，但 broad workspace run
在 300 秒超时；该次未计为 PASS。`pnpm check` 与 `pnpm build` 被当前 pnpm 的
`esbuild@0.25.12` ignored build script 策略拦截；同一已安装依赖的直接
`svelte-check`/`vite build` 已通过。`cargo fmt --all -- --check` 仍包含本轮之外的
既有 dirty 文件格式差异，未将其记为全仓通过。`cargo clippy --workspace --all-targets
--offline -- -D warnings` 实际执行但被既有 `apeireth-memory` 的
`layered_memo` 非 CamelCase warning 阻断，未将其记为通过。

## Remaining Unsupported Capabilities

本轮没有伪造 backend 的 Workbench 写能力、Forge/PR、Video、Attachments、Voice
等能力。它们在没有真实 production service 时保持 `supported=false`。

## Known Follow-ups

- 清理本轮之外的历史 dirty 文件后，再单独恢复全仓 `fmt` 门禁。
- 在 CI 依赖策略允许后运行完整 pnpm 命令和未离线的全 workspace 回归。
- 为更多真实 assembly service 增加跨层 memory/event integration fixtures；当前核心
  kernel、assembly、gateway、CLI 与前端契约测试已通过。
