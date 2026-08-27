# Apeireth 维护指南（v2 工程重构线, 2026-08-27）

> **现状 (2026-08-27)**：本文**重写**为 v2 维护指南（13-crate 工作区）。取代 v1 `apeireth-companion` 维护手册（现 `legacy/`）。当前基线：默认分支 `main`、13-crate 工作区、tag `v2.0.0-alpha.1` @ `d6910cf7`；v2 下一步见根 [ROADMAP.md](../../ROADMAP.md) §4。

```
[Document-Meta]
Document:        docs/04-internal/maintenance-guide.md
Version:         Manual-Rev-N (v2 重写)
Last-Modified:   2026-08-27
Status:          🟢 活跃 (v2 13-crate)
```

> 给谁看：维护代码的人（人或 AI）。先读 [ARCHITECTURE.md](../../ARCHITECTURE.md) 顶层，再读 [architecture.md](../01-architecture/architecture.md) 详细归属，再来这份。
> 读法：§1 概念词典（澄清易混词）→ §2 13-crate 模块地图 → §3 维护流程 → §4 进程封装 → §5 v1 时代的"现在不这样做" → §6 不漂移承诺。

---

## 1. 概念词典（v2 13-crate 语境）

### 1.1 能力栈（v2 一句话定义）

```
能力 (capability)  = 任何 runtime 可调用的统一抽象（tool / provider / memory / 等）
├─ Tool          = 模型可调用的副作用工具（filesystem/search/repo/shell/fetch）
├─ Provider      = LLM 供应商适配器（minimax/anthropic/openai-compatible）
├─ Memory        = 持久化记忆后端（M1B 移植中）
载体: Plugin        = 能力的载体（静态 in-process，提供 1+ capability）
   └─ PluginManager = 唯一事实源（两个 registry：PluginRegistry + CapabilityRegistry）
治理: GovernanceHook = runtime 调能力前的决策闸（Allow/Deny/RequireApproval）
审计: Trace        = 结构化执行追踪（runtime.execute 返回的 TurnResponse.trace）
```

v1 时代的三层交付模型（模块/套件/插件）已**弃用**——v2 只有 plugin 一层；套件/模块边界由 `PluginManifest::depend_on()` + `CapabilityKind` 表达。

### 1.2 关键易混词对照（v2 vs v1）

| v2 术语 | 含义 | v1 旧物（已废弃，R用） |
| | | |
| **Plugin** | 提供 1+ capability 的静态单元（trait 4 方法）| v1 5 方法（id/version/description/on_load/on_unload） |
| **PluginManager** | 唯一注册点，2 个 registry 索引 | v1 ToolRegistry/CapabilityCatalog/PackRegistry 四处 |
| **ToolCapability** | 模型可调用工具 trait（3 方法） | v1 `Tool` + `ToolKind` 6 类 + `ToolAxes` 5 轴（全部废弃） |
| **ProviderCapability** | LLM 供应商适配器 trait | v1 `apeireth-llm-iface::LlmProvider`（legacy/） |
| **GovernanceHook** | runtime 调能力前必经的决策闸 | v1 ToolBridge 8 闸散落多处 |
| **Decision** | Allow / Deny{reason} / RequireApproval{reason}（**Deny ≠ RequireApproval**，runtime 必须区分）| v1 `ApprovalDecision` 三类（Allow/Approve/Deny） |
| **TurnResponse.trace** | 结构化 trace，R**含原始 CoT**（`crates/engine/runtime/src/canonical/execute.rs` 设计明确禁止） | v1 公开 `ChatTurnOutput.reasoning_cot`（已废弃） |
| **CredentialResolver** | 凭据解析契约（logical name → 物理位置）| v1 `api_key: String` 字段（已废弃） |
| **Secret<T>** | 自动 redact 的凭据包装（debug 不打印） | v1 直接 `String`（debug 泄露） |
| **PluginContext** | plugin 启动时拿到（clock + credentials + trace）| v1 无对等物（plugin 直接读环境） |
| **`apeireth_core::kernel`** | 新规范命名空间（IDs/time/lifecycle/event/metadata/errors） | v1 `apeireth_core::` 直暴露（与新 kernel 冲突） |
| **BuiltinToolsPlugin** | 唯一 canonical 工具集合（filesystem/search/repo 默认 + shell/fetch opt-in） | v1 `apeireth-tools` + 9 个 `apeireth-tool-*`（全部 legacy/） |

### 1.3 工程哲学（不漂移）

8 哲学锚（per [philosophy.md](../01-architecture/philosophy.md)）必穿透：S-1 北极星 / S-2 实事求是 / S-3 质量工程化 / O-1 安全优先 / O-2 走在前前 / O-3 干到底 / O-4 接手 / O-5 不假装。

3 不漂移（per [ROADMAP.md](../../ROADMAP.md) §5 + conventions/03-adr.md）：
- **0 触碰** 3 项不可变脊柱（Self-Disable 判定 / L0 HA 物理隔离 / 13 键 verdict cache 语义）
- **0 改** workspace.version（当前 1.2.0；产品轴 vs workspace 轴分离）
- **0 改** R11 baseline 3 值（0.8682/0.8532/0.906）

---

## 2. 13-crate 模块地图（v2 当前真实）

来源：`[ARCHITECTURE.md](../../ARCHITECTURE.md)` + `[architecture.md](../01-architecture/architecture.md)` + 实测依赖图。

### 2.1 Foundation（5 crate, 稳定契约层）

| crate | 职责 | 公共面要点 | 关键源文件 |
| | | |
| `apeireth-core` | 域原语（kernel: ids/time/lifecycle/event/metadata/errors） + 旧"哲学"模块保留（onion/gate/lifecycle/memory/philosophy 在 crate 根 re-export，P2 决策见 ROADMAP）| `kernel::CapabilityId / PluginId / RequestId / SessionId / TraceId / Clock / Metadata` | `src/kernel/{ids,time,lifecycle,event,metadata,error}.rs` |
| `apeireth-protocol` | vendor-wire 归一化（OpenAI Chat/Responses/Anthropic/Gemini）→ `NormalizedRequest/Response/Tool` | `NormalizedTool::function(name, desc, schema)` 工具声明 | `src/canonical/` |
| `apeireth-plugin` | plugin + capability 单一契约（两个 registry 唯一事实源）| `Plugin`/`ToolCapability`/`ProviderCapability`/`CredentialResolver` | `src/{plugin,tool,provider,manifest,registry,manager,capability,credentials}.rs` |
| `apeireth-governance` | 决策 trait + hook 实现（policy 库） | `AllowAll` /DenyCapabilities /MaxRounds /GovernancePipeline /PermissionGovernanceHook /PromptInjectionHook /CredentialDisclosureHook /AuditHashChain | `src/{lib,permission,input_security,audit}.rs` |
| `apeireth-credentials` | **🔴 孤儿**：keyring + encrypted file backend；生产用 provider/credentials::EnvCredentialResolver | `Secret<T>` /`CredentialsStore` /`KeyringSelector`（待接线） | 整 crate 未被依赖（grep 验证） |

### 2.2 Engine（4 crate, 运行时执行）

| crate | 职责 | 公共面要点 |
| | | |
| `apeireth-runtime` | **canonical agent loop 单一入口**（execute.rs 1189 行）+ governance + provider 选择 + tool dispatch + approval 生命周期 + trace | `Runtime::builder().with_governance(...).with_plugin(...).build()` / `Runtime::execute(req)` / `Runtime::execute_outcome(req)` |
| `apeireth-provider` | 3 canonical provider（MiniMax/Anthropic/OpenAI-compatible）+ EnvCredentialResolver | `MinimaxProviderPlugin::from_env()` 等；环境变量：APEIRETH_API_KEY / APEIRETH_ANTHROPIC_KEY / OPENAI_API_KEY |
| `apeireth-storage` | SQLite pool + WAL + `PRAGMA user_version` 迁移 | `SqlitePool::open` + `Migrations` |
| `apeireth-memory` | 域原语复用 + vector/graph/检索契约 primitive（M1B 部分落地） | `Episode` / `Session` / `IdentityCard`（**走 `apeireth_core::Episode` 等 legacy re-export**——P2 drain 排期见 ROADMAP）|

### 2.3 Capabilities（1 crate, 唯一 ProcessExecutor 边界）

| crate | 职责 |
| | |
| `apeireth-tools-canonical`（包名带 -canonical 后缀，与 legacy `apeireth-tools` 区分）| 3 只读工具默认 + shell/fetch opt-in + egress + **唯一 ProcessExecutor**（Windows Job Object + CREATE_SUSPENDED 完整，Linux/macOS 进程组部分）+ **敏感路径保护**（upstream `ac5cbf5a`：filesystem/search 自动屏蔽 `.env` / `.ssh` / `.aws` / `.gnupg` / `.secret` 等，src/sensitive_path.rs）|

`BuiltinToolsPlugin::new(workspace_root)` 注册：`tool.filesystem` / `tool.search` / `tool.repo`（默认）。`BuiltinToolsOptions { shell: Some(...), fetch: Some(...) }` 显式开启 shell/fetch。

### 2.4 Adapters（3 crate, 外部入口面）

| crate | 职责 |
 | | | |
| `apeireth-gateway` | canonical HTTP `:8080`（`GET /health` / `POST /v1/chat` / `POST /v1/chat/completions`）| 全部委托 `Runtime::execute`，R自创第二个 runtime |
| `apeireth-cli` | `apeireth session / chat / gateway serve` 三命令入口 | 走 `build_canonical_runtime_from_env`，**已挂 GovernancePipeline = `PermissionGovernanceHook + CredentialDisclosureHook + PromptInjectionHook`**（upstream `873d2857`）；MaxRounds 结构性，AuditHashChain 按部署需要挂 |
| `apeireth-sdk` | **stub 模式**：6 工具白名单 + 鉴权 5 组件 + WS 8 帧协议类型已就位；真实 HTTP/WS 走 `unimplemented!()` 守门；R21 真接 | 真实用户 = R21 后才出现 |

### 2.5 依赖 DAG（实测，13 crate）

```
foundation/
  core           (leaf)
  credentials    (leaf, 孤儿)
  protocol       -> core
  governance     -> core
  plugin         -> core, protocol
engine/
  storage        (leaf)
  memory         -> core, storage
  provider       -> core, plugin, protocol
  runtime        -> core, protocol, plugin, governance, storage
capabilities/
  tools          -> core, plugin, protocol
adapters/
  gateway        -> core, protocol, runtime
  cli            -> core, gateway, plugin, provider, runtime, tools, protocol(SDK 路径走 protocol)
  sdk            -> protocol
```

**无环**。**无违规边**（foundation ← engine ← capabilities ← adapters 单向）。验证脚本：根 [ARCHITECTURE.md](../../ARCHITECTURE.md) "Effective package edges" 表 + `cargo metadata --no-deps` 解析（13 edge counts 对得上）。

---

## 3. 维护流程（v2 日常操作）

### 3.1 本地反馈环（CI 1:1 复刻）

来源：根 [Makefile](../../Makefile)。

```bash
make check       # cargo check --workspace --all-targets
make test        # cargo test --workspace --all-targets --locked (~1476 tests, v2 main = 9080cc93)
make fmt         # cargo fmt --all
make ci          # make ci-build + ci-test + ci-release (一键)
```

约束：本地 `make ci` exit 0 → push 后 GitHub Actions 必绿（同源）。任 1 个 exit != 0 → 不要推。

### 3.2 提交纪律

来源：[commit.md](../../docs/archive/conventions/06-commit.md) + [team-work-doc.md](team-work-doc.md) §2.3。

- scope: `docs:` / `R17-<topic>` / `Manual-Rev-X` / `Design-X.Y` / `Fix-N` / `crate:<name>` / `ci` / `sec` / `perf` / `round<N>-<NN>`
- subject ≤ 72 字符
- body 中文"为什么 + 做了什么 + 测试结果"
- 不提交调试输出（eprintln DEBUG 删净）
- 共享文件（`lib.rs`/`Cargo.toml`/`ARCHITECTURE.md`/`ROADMAP.md`）改动先通知集成守门员

### 3.3 添加新 capability 的 checklist

来源：[plugin-authoring-guide.md](plugin-authoring-guide.md)（v2 插件契约）+ team-work-doc §2.1 + conventions/02-path.md。

1. `crates/foundation/plugin/` 或**新 crate**（推荐：把 plugin 放进一个新 crate 而非塞 `crates/foundation/plugin` 的源码树）
2. 模块头 `//!` 写职责 + 0 装 PASS（什么没做）
3. `Cargo.toml` description 填清楚（**必填，根 v2 文档对账发现过 description 缺失/不完整**）
4. 实现 `Plugin` trait（4 方法：manifest/initialize/shutdown/tools/providers）+ `ToolCapability` 或 `ProviderCapability`
5. `Cargo.toml` license 继承 `license.workspace = true`（**不要硬硬写**，per [commit 440a9b7](../archive/conventions/06-commit.md) style）
6. 单测覆盖（正常/失败/非法输入，0 装 PASS）+ 集成测试接 `PluginManager::register`
7. 验证 manifest 与 `tools()/providers()` 1:1 对应（manager 自动验证）
8. 不读 `std::env::*` 取 secret、不读 `Utc::now()` 取时间——全走 `PluginContext`
9. 更新 [ARCHITECTURE.md](../../ARCHITECTURE.md) ownership 表（如新增 crate）
10. 提交：`git status` 核对只含自己文件 + commit msg 中文"## + ## + ##"三段
11. PR + 自审报告 + 集成守门员审查

### 3.4 修改现有 capability 的 checklist

1. `git log -- <file>` 看责任链（per `07-hash.md` 修正链）
2. 对照 [ARCHITECTURE.md](../../ARCHITECTURE.md) "Ownership boundaries" 表确认职责
3. **不要碰** deprecated / frozen 标记（v1 字段在源码里仍存但有 `#[deprecated]` 注释）
4. 改公共 API → `grep` 全部构造点（[team-work-doc.md](team-work-doc.md) §2.4 红线 4）
5. `make ci` 绿 + `cargo doc` 无新 warning

---

## 4. 进程封装（ProcessExecutor 唯一边界）

来源：`crates/capabilities/tools/src/process/` + [architecture.md](../01-architecture/architecture.md) Process ownership 表。

### 4.1 三 OS 矩阵（实测）

| 行为 | Windows | Linux | macOS |
| | | |
| 结构化 spawn / cwd / env / timeout / bounded stdout+stderr | ✅ enforced | ✅ enforced | ✅ enforced |
| 进程树隔离 | **✅ Job Object（kill-on-Job-close 完整）** | 🟡 process group 部分 | 🟡 process group 部分 |
| 预执行隔离 | ✅ CREATE_SUSPENDED → JobObject → Resume | 🟡 pre_exec setup | 🟡 pre_exec setup |

源文件：
- Windows：`crates/capabilities/tools/src/process/windows.rs`（860 行，最完整）
- Linux：`crates/capabilities/tools/src/process/linux.rs`
- macOS：`crates/capabilities/tools/src/process/macos.rs`
- 平台抽象：`crates/capabilities/tools/src/process/platform.rs`

### 4.2 修改 ProcessExecutor 的红线

- **R**改 `platform.rs` 的 trait 公共面（其他 OS 实现会破）
- **R**改 Job Object 的 `JOB_OBJECT_LIMIT_*` 标志语义（v1 教训："memory limit denies allocation, not kills"——这是 Windows 语义，0 装 PASS 必须保留）
- 改动必须跨 3 OS 都测（CI M2B-X 三 OS 隔离 workflow 跑）

### 4.3 ProcessSupervisor（v1 计划，v2 **P5 排期**）

`ProcessSupervisor` + 进程树快照模型 + 跨进程血缘追踪，**当前不在 13-crate 工作区**。见 [ROADMAP.md](../../ROADMAP.md) §4 P5。R**发明"我们用 supervisor 做 X"——明确不在 scope。

---

## 5. v1 时代的"现在不这样做"（反模式清单）

来源：[ARCHITECTURE.md](../../ARCHITECTURE.md) "Deferred work" + 本批文档对账发现。

| 反模式 | v1 长这样 | v2 该这样 | 原因 |
| | | |
| 凭据硬编码 `api_key: String` 字段 | `struct ApiConfig { api_key: String }` | `ctx.credentials.resolve("provider.<vendor>>")` → `Secret<T>` | secret R落 struct + 自动 redact + 测试可换 |
| `Utc::now()` | 工具直接 `Utc::now()` | `ctx.clock.now_ms()` | virtual clock 让测试可重放 |
| `std::env::var("OPENAI_API_KEY")` | provider 启动时读环境 | `EnvCredentialResolver` 默认 mapping | 单一事实源 + env 命名有规范 |
| `pub use legacy::*` 在 crate 根 | `apeireth_core::pub use memory::*` 把 `Session`/`Lifecycle` 都重导出 | canonical 走 `apeireth_core::kernel::*`，legacy 路径标 `(legacy v1)` | 同名类型 kernel vs legacy 冲突 → 命名混乱 |
| 新加 capability 走第二 registry | `MyToolRegistry` + `MyCapabilityCatalog` + `MyPluginManager` | `Arc::new(MyPlugin)` → `PluginManager::register` | 收敛点 = 唯一事实源，v1 的"多 registry"是重构要治的病 |
| 工具 schema 各 provider 自己定义 | `OpenAIToolSchema::new(...)` + `AnthropicToolSchema::new(...)` | `NormalizedTool::function(name, desc, schema_json)` → provider 自动转 | 协议层抽象掉 vendor 差异 |
| 工具名含 `file/patch/task/shell/exec` | `ToolName("file_delete")`（ | 选能描述清楚用途的名字（如 `tool.filesystem`）；含触发词的工具真的就是高风险 | governance 第 5 闸 LLM 评审 |
| `<<<[TOOL_REQUEST]>>>` marker 解析 | provider 输出 marker → 后端 parse | 用 provider 原生 `tool_calls` 数组 | protocol 已抽象掉差异 |
| 写 plugin = 新建整套 registry | `MyDomainPlugin { my_registry, my_pack_registry, ... }` | 实现 `Plugin` trait 的 4 方法 | plugin manager 统一管 lifecycle |
| `on_unload` 撤销 v1 授权 pack | `bridge.packs.revoke_by_name(...)` | R**做（v2 governance 接管授权——P0 接线） | 责任分离 |
| 让 plugin 自己做拒绝逻辑 | `if user_is_bad { return Err(...) }` | `CapabilityDescriptor.with_metadata("risk", ...)` + governance 闸 | policy 不该散在 plugin |
| `crates/apeireth-*` 顶层 crate 名 | `crates/apeireth-companion/` | `crates/foundation|engine|capabilities|adapters/<name>/` + `apeireth-*` package | 顶层目录按责任分组（foundation/engine/capabilities/adapters），package 名仍 `apeireth-*` |
| 测试用 `cargo test --workspace` 全量 | 团队协作无脑跑全量 | `cargo test -p <your_crate> -j 4`（per [team-work-doc.md](team-work-doc.md) §2.2） | Windows 页文件易耗尽 |

---

## 7. CI 门禁清单（13 个 workflow，1:1 复刻到本地）

来源：`.github/workflows/*.yml` + 根 [Makefile](../../Makefile) `make ci`。

| workflow | 触发 | 本地复刻命令 |
| | | |
| `rust.yml` | push main + PR | `cargo nextest run --workspace --profile ci --locked` + 13 键测试契约守门 |
| `rust-lint.yml` | push main + PR | `cargo clippy --workspace --all-targets --locked -- -D warnings` + fmt |
| `miri.yml` | push main | `cargo +nightly miri test -p apeireth-core` 等 |
| `rustdoc.yml` | push main | `cargo +nightly doc -Dwarnings` |
| `cargo-audit.yml` | push main + 周 cron | `cargo audit` |
| `cargo-deny.yml` | push main | `cargo deny check` |
| `coverage.yml` | push main | `cargo tarpaulin` |
| `m2b-xv-isolation.yml` | push main | M2B 三 OS 进程隔离（CI runner，3 OS 跑） |
| `m2c-xv-shell-validation.yml` | push main | M2C shell 验证（3 OS） |
| `m3a-canonical-fetch.yml` | push main | M3A 受控 fetch（3 OS） |
| `protocol-handlers.yml` | push main | protocol 集成测试 |
| `ci-fix.yml` | push main | CI 防御（hygiene） |
| `companion-desktop-ci.yml` | push companion-desktop/** + PR touch 它 | 前端独立 CI |

本地 `make ci`（build + nextest + release build）=CI 三个核心 job 一致；其它 workflow 的 OS-specific 步骤仅 CI runner 上跑。

### 8. 不漂移承诺

来源：conventions/10-locked.md（已 R119 形式撤销，原意保留）+ ROADMAP §5。

**0 触碰**：
- 3 项不可变脊柱（Self-Disable 判定 / L0 HA 物理隔离 / 13 键 verdict cache 语义）——见 governance + core/src/philosophy.rs
- workspace.version 1.2.0（产品轴 vs workspace 轴分离，根 [ROADMAP.md](../../ROADMAP.md) §0）

**0 改**：
- R11 baseline 3 值（0.8682/0.8532/0.906，per [11-baseline.md](../../docs/archive/conventions/11-baseline.md)）——仅守，代码不在当前工作区，约束在 git 历史
- 8 哲学锚穿透（per [09-anchor.md](../../docs/archive/conventions/09-anchor.md)）

**R 假装**：
- v2 路径接 governance hook 之外的任何机制（per ROADMAP P0 = 接线 hook 是排期，不是完成）— **上游 `873d2857` 已落实 P0 的 3 个核心 hook**（Permission + 凭据泄漏 + 注入检测），其余按部署需要
- `BuiltinToolsPlugin` 默认开 shell/fetch（opt-in 守门，**不要**默认启用）
- SDK 真接 HTTP/WS（stub 模式是诚实的，R21 才真接）
- `apeireth-credentials` keyring backend 已接线（**孤儿**——EnvCredentialResolver 是生产默认）

---

## 9. 一句话

**v2 13-crate 工作区 = 单一事实源（PluginManager）+ 单一执行入口（Runtime::execute）+ 单一进程边界（ProcessExecutor）+ 单一协议抽象（NormalizedRequest/Response）+ 单一工具 trait（ToolCapability）+ 单一插件 trait（Plugin 4 方法）+ 单一凭据契约（CredentialResolver）+ 单一决策 trait（GovernanceHook + Allow/Deny/RequireApproval 三态）。**其它任何"创新点"都先回到这 8 个"单一"过一遍。

---

_本指南 v2 重写 (2026-08-27)：取代 v1 `apeireth-companion` 维护手册（`crates/apeireth-companion` 模块地图 → 现 13-crate foundation|engine|capabilities|adapters 分组）；v2 维护的 = PluginManager 唯一注册点 + 8 哲学锚穿透 + 3 不漂移承诺。v2 下一步（governance 接线 / core drain / 记忆移植）见根 ROADMAP §4。_