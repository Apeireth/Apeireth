[Document-Meta]
Document: docs/roadmap/r20-product-finalize-2026-08-05.md
Version: Manual-Rev-A
R-Cycle: R20 (收产品)
Commit: <commit 时回填>
Last-Modified: 2026-08-05
Status: 🔍 草拟 (待 Mavis 拍板 + 主人 2026-08-04 12:30 方向复核)

---

# R20 收产品路线图 — Apeireth OS → AI 成长平台

> **性质**: R20 P1 收产品 = 把 Apeireth OS v2.0.0-alpha (R19 工程化收尾完成) 变成"可分发 / 可部署 / 可用"的 AI 成长平台.
> **依据**: 主人 2026-08-04 12:30 定的方向 (ROADMAP.md §下一阶段建议 R20 行) + 2026-08-05 细化"整合产品+部署+API"三轴
> **承接**: R17 战略 0-4 (1.0 release 后端) → R18 6 类非 LLM API → R19 工程化收尾 (9/9 业界标准 + 5 新 crate + 116 集成测试 + 2416 tests)
> **不修改承诺**: 阶段 1+2+3+4+5 + v2/v4/v4.1 + 12 键 + 6 锚 + workspace v1.0.0 + Document-Meta + R11 baseline 三值 全保留 (见 §7)

---

## §1 战略背景 (为什么)

### 1.1 主人 2026-08-04 12:30 定的方向

R20 优先级 = 🟡 P1 (ROADMAP.md §下一阶段建议表第 3 行).

主人原始表述: "Apeireth OS 长程 AI 成长平台对外, 含计费 + 订阅 + API 配额".

**2026-08-05 细化** (Mavis 跟 Mavis 拍板时, task 描述明确):
- R20 = **收产品** (不是直接商业化, 商业化是 R21+)
- 三轴: **产品形态 + 部署形态 + API 形态**
- 不动: 后端 41 crate 工程基线 + R-Measure baseline + 9 业界标准

**为什么 R20 是"收产品"不是"商业化"**:
- R19 工程化收尾刚完工 (v2.0.0-alpha, 2416 tests, 9/9 业界标准), 后端已达 1.0 stable
- R20 之前: 后端是源码 + Cargo workspace, **没有分发形态** (Docker/系统包/一键安装)
- R20 之前: API 是内部 crate 边界, **没有公开 HTTP/WebSocket API** 给外部 SDK
- R20 之前: 用户接触面只有 TUI, **没有 Web/桌面入口** (Tauri 团队独立做)
- 商业化 (计费/订阅/配额) = R21, 必须在 R20 收产品**之后**才有意义 (卖 API 配额必须先有可消费的 API)

### 1.2 跟 R17 / R18 / R19 的关系

| 周期 | 焦点 | R20 关系 |
|------|------|---------|
| **R17 战役 0-4** (2026-08-04 完成) | 后端 1.0 release: 砍 NewAPI + 4 协议归一化 + 5 类工具 + 砍前端 + TUI 真流式 + 30 crate supervisor | R20 = 把 R17 成果**包装成可消费产品** |
| **R18 6 类非 LLM API** (P0 战役 2-5) | web_search / file_ops / git_ops / code_exec / calendar + message API | R20 阶段 3 = 把 R18 API **公开 HTTP/WebSocket**, 内部 trait → 公开端点 |
| **R19 工程化收尾** (2026-08-05 完成, v2.0.0-alpha) | workspace.lints + cargo-deny + rustfmt + clippy + SECURITY + dependabot + CI matrix + 116 集成测试 + miri + coverage + rustdoc | R20 = 复用 R19 工程基线, **新增** 部署 (Docker/包) + 公开 API 表面 |
| **R20 收产品** (本路线图) | 三轴: 产品 / 部署 / API | **当前任务** |
| **R21 商业化** (下一周期) | 计费 + 订阅 + API 配额 | R20 必须先完工, R21 才有可消费对象 |

### 1.3 比喻

> R19 v2.0.0-alpha = 大型基地 (41 crate 已部署, HTTP API 表面稳定, 9/9 业界标准达标)
> R20 = 给基地**装电梯 + 装大门 + 铺公路**: 电梯 (产品形态 TUI/Web/SDK), 大门 (公开 API + OpenAPI 规范), 公路 (Docker/系统包/一键安装)
> R21 = 开始**卖门票** (计费/订阅/配额), 但要先有路和门

---

## §2 R20 三大目标

### 目标 1: 产品形态 — 从"工程化研发项目" → "可分发的 AI 成长平台"

| 当前 (R19) | 目标 (R20) |
|-----------|----------|
| 仅 TUI 9 命令 (R19 Step 1 完成, 9 器官已接真后端) | TUI 9 命令深化 (Step 2/3) + Web 文档站 + 落地页 |
| 内部 41 crate 库, 用户必须 clone + cargo build | 一行命令安装: `curl install.apeireth.io \| sh` |
| 没有"用户"概念, 只有"开发者" | 区分: **终端用户** (TUI/Web) / **集成开发者** (SDK) / **平台运营者** (部署) |

### 目标 2: 部署形态 — 从"源码 + Cargo workspace" → "docker image / 系统包 / 一键安装"

| 当前 (R19) | 目标 (R20) |
|-----------|----------|
| 18 Dockerfile + docker-compose + 4 k8s YAML (部署基建在, 但**只给开发者**用) | **多架构 Docker image** (linux/amd64 + linux/arm64) **推到 GHCR/Docker Hub**, 一行 `docker run apeireth/api` |
| 无离线包 | **离线安装包** (含 apeireth-pybridge 1100 模块 + 依赖) |
| 无系统包 | **Linux deb/rpm** + **macOS Homebrew formula** + **Windows MSI/scoop** |
| 无 install 脚本 | **install.sh / install.ps1** 一键安装 (检测 OS/arch/依赖, 引导用户配置 API key) |

### 目标 3: API 形态 — 从"内部 41 crate 库" → "公开 HTTP/WebSocket API + 多语言 SDK"

| 当前 (R19) | 目标 (R20) |
|-----------|----------|
| `apeireth-api` (4 协议归一化, 内部端点) | `apeireth-api` + **REST wrapper** (公开 HTTP/JSON 端点) |
| `apeireth-protocol` (LLM 4 协议抽象) | `apeireth-protocol` + **WebSocket 双向流** (公开) |
| `apeireth-pybridge` (1100 模块, 但**仅 cdylib**) | `apeireth-pybridge` + **Python SDK** 公开 API (PyPI 包) |
| `apeireth-sdk` (空 Cargo.toml, T13 CONCERN BLOCK) | `apeireth-sdk` + **TypeScript SDK** (npm) + **Rust SDK** (crates.io) |
| 无 OpenAPI 规范 | **OpenAPI 3.1** 规范 + Swagger UI / Redoc |
| 无 API 配额/限流 | **基础 rate limit** + API key 认证 (R21 计费的前置) |

---

## §3 R20 子目标 (5 维度)

> 阶段编号详见 docs/stage4/r19-r20-stage-unified-2026-08-05.md §3 (本章"阶段 1-5" = 套 B R20 收产品 5 阶段)

### 3.1 产品形态 (3 子项)

| 子项 | 状态 | 备注 |
|------|------|------|
| **TUI 9 命令深化** | 🟡 部分 (R19 Step 1 ✅, Step 2/3 ⏸️) | 见 `docs/v2-strategy/06-TUI-UPGRADE-ROADMAP.md` Step 2 (Tools/Memory/Organs/ASI/Sovereignty/Agent 6 类 API 端点) + Step 3 (TUI 消费这些端点). R20 阶段 1 接续 R19 暂存 |
| **Tauri 2 .exe 桌面** | ⛔ 不在 R20 范围 | 主人 2026-08-04 19:53 决定砍前端, 交给**另一团队** (Tauri-roadmap-2026-08-05.md T-001~T-013 13 项沉淀) |
| **Web 界面** | 🟢 可选, R20 阶段 5 评估 | 优先级低于 TUI, 评估期: Svelte/Next.js + ts-rs 类型同步. 阶段 5 决策点 (1-2 周出 PoC) |
| **移动端** | ⛔ 暂不做 | R22+ 议题, R20 不分配资源 |

### 3.2 部署形态 (5 子项)

| 子项 | 关键 crate | 备注 |
|------|----------|------|
| **Docker image** (linux/amd64 + linux/arm64) | `apeireth-tui` + `apeireth-api` + `apeireth-council` + `apeireth-team-lead` | multi-arch build (buildx), 推 GHCR `ghcr.io/apeireth/api:tag` + Docker Hub. **关键约束**: apeireth-pybridge cdylib 编译, 已知 issue (R18-2 解决) |
| **Linux deb / rpm** | `apeireth-supervisor` (systemd 集成) | deb: cargo-deb; rpm: cargo-rpm. 装后 `systemctl start apeireth` |
| **macOS Homebrew formula** | `apeireth-cli` (brew tap) | tap: `apeireth/tap`, formula: `apeireth.rb`. 装后 `brew services start apeireth` |
| **Windows MSI / scoop** | `apeireth-cli` (WiX / scoop manifest) | MSI: cargo-wix; scoop: `scoop bucket add apeireth`. 装后 apeireth 服务注册 |
| **离线安装包** | 全部依赖 (apeireth-pybridge 1100 Python 模块) | tarball (Linux/macOS) + zip (Windows). 大小预估: 1.5-2GB (Python wheel 缓存) |

### 3.3 API 形态 (5 子项)

> **🔥 修正 (2026-08-05 13:50)**: 之前 R20 §3.3 + §5.1 把 `apeireth-sdk` 现状误读为"空 Cargo.toml / T13 CONCERN BLOCK" — **sub-agent 报告错了**. 实情:
>
> **apeireth-sdk 现状: 已有 11 文件 ~14000 LOC** (`Cargo.toml` + `src/lib.rs` + `src/version.rs` + `src/wire.rs` + `src/error.rs` + `src/abi.rs` + `src-py/apeireth_sdk/` + `tests/smoke.rs`), 是 **low-level FFI 测试入口** (C-ABI 边界 + 版本协商 + WireFormat 反序列化), 8 个 smoke test 全过.
>
> **不**是用户面向 SDK. R20 阶段 4 任务: 在现有 11 文件基础上加 `src/client.rs` + `src/http.rs` + `src/ws.rs` + 3 SDK 入口 (Python/TS/Rust).
>
> **来源**: `docs/stage4/apeireth-sdk-gap-analysis-2026-08-05.md` (R20 阶段 0 必读). 详细 5 步实施方案见该文档 §4.

| 子项 | 关键 crate | 备注 |
|------|----------|------|
| **HTTP REST API** | `apeireth-api` (已有 4 协议端点 + 6 V2 端点) | **新增 REST wrapper**: 4 协议端点 → `/v1/openai/chat` `/v1/anthropic/messages` `/v1/gemini/generate`; 6 V2 端点 → `/v1/chat` `/v1/memory` `/v1/organs` `/v1/asi` `/v1/sovereignty` `/v1/agent` |
| **WebSocket 双向流** | `apeireth-protocol` (ProviderEvent 流) | **新增 WS 端点** `/v1/stream`, 双向: client 发 message → server 推 ProviderEvent 流. 替代 SpectrAI AgentBridge WebSocket (主人决策砍, 改 in-process + 公开 WS) |
| **Python SDK** | `apeireth-sdk` 现有 `src-py/apeireth_sdk/` (ctypes 加载 cdylib) | **复用现有 Python wrapper**, 加 `ApeirethClient` class (HTTP via httpx + async). 暴露 11 方法 (chat / memory_* / organs_* / team_* / asi_* / sovereignty_* / agent_* / stream). 文档: `docs/sdk/python/`. 验收: `from apeireth_sdk import ApeirethClient; c = ApeirethClient(); c.chat("hi")` 跑通 |
| **TypeScript SDK** | `apeireth-sdk` + 新建 `crates/apeireth-jsbridge/` (napi-rs 桥) | **全新**: `npm install @apeireth/sdk`. napi-rs 桥接 `apeireth-sdk` C-ABI → 暴露 TS `Apeireth` class. OpenAPI 类型自动生成 (openapi-typescript). 验收: `import { Apeireth } from "@apeireth/sdk"; const c = new Apeireth(); c.chat("hi")` 跑通 |
| **Rust SDK** | `apeireth-sdk` 现有 4 抽象 (SdkVersion/Envelope/SdkErrorCode/C-ABI) + 新增 client/http/ws | **复用现有 4 抽象** + 新增 `ApeirethClient` struct (11 方法, 跟 Python/TS 签名对齐). `cargo add apeireth-sdk`. 验收: `let c = ApeirethClient::new(); c.chat("hi").await?` 跑通 |

**3 SDK 共享** (R20 阶段 4 一起写):

- **共享抽象**: `apeireth-sdk::client::ApeirethClient` (R20 阶段 4 一起写, 不分阶段). 3 SDK 用同一方法签名 (snake_case / camelCase 映射, 错误码跨语言统一)
- **复用现有**: `SdkVersion` + `Envelope` + `WireKind` + `SdkErrorCode` + `#[no_mangle] extern "C"` 4 抽象 跨 SDK 复用, 不重写
- **共享 Config**: `base_url` / `api_key` / `timeout` / `max_retries` (builder 模式)

**端到端验证** (R20 阶段 4 Step 5):

- 3 SDK 同输入 → 同输出, 5 个测试用例:
  1. `chat("hi")` 返非空字符串
  2. `memory_read(id)` 返 Memory 对象 (字段一致)
  3. `organs_status()` 返 9 器官状态数组
  4. `stream()` 推 ≥ 1 个 ProviderEvent
  5. 错误码 (`unauthorized` / `rate_limited` / `protocol_error`) 3 SDK 都返对应 `SdkErrorCode`
- 起 `apeireth-api` 本地 server → 3 SDK 各跑一遍 → 输出对比

### 3.4 集成: SpectrAI 团队功能 (4 子项)

> **衔接**: `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md` (R19+ 集成蓝图, A 方案已拍板 2026-08-05)

| 子项 | 关键 crate | 备注 |
|------|----------|------|
| **`apeireth-team-lead` 公开 API** | `apeireth-team-lead` (新命名, 避免跟 `apeireth-supervisor` 冲突) | R19+ 集成蓝图 §5 命名空间冲突表决策: 新命名. **R20 阶段 1**: 暴露 REST `/v1/team/spawn` `/v1/team/agent/:id` `/v1/team/agent/:id/idle` |
| **14 工具公开化** | `apeireth-mcp::team` (新模块) | R19+ 集成蓝图 §2.1 #1: agent 模块 11 文件 5610 LOC, **R20 阶段 1**: 14 工具 (`spawn_agent` / `send_to_agent` / `wait_agent_idle` / `cancel_agent` / `list_agents` / `get_agent_status` / `subscribe_events` / `create_team` / `list_teams` / `delete_team` / `assign_role` / `broadcast` / `wait_team_idle` / `kill_team`) 公开 HTTP 端点 |
| **mid-task bug 3 处修法** | `apeireth-session` (新 crate) | R19+ 集成蓝图 §4.4 必改 3 处: ① sendMessage throw→Result ② sendToAgent 加 child session 状态检查 ③ child 状态用 `tokio::sync::broadcast` 事件驱动. **R20 阶段 1 一起改**, 不重复 SpectrAI bug |
| **5 Provider 适配** | `apeireth-api` (用户决策, 4 协议端点) | Claude/Codex/Gemini/iFlow/OpenCode 5 Provider 走 `apeireth-api` 4 协议 HTTP + base_url + auth_token. toolMapping 进 `apeireth-protocol::tool_mapping` |

### 3.5 文档 + 营销 (4 子项)

| 子项 | 形式 | 关键内容 |
|------|------|---------|
| **用户文档** | `docs.apeireth.io` (Docusaurus / mkdocs) | Quick Start (5 分钟跑通 TUI) / Tutorial (9 命令 walkthrough) / Reference (命令字典) / FAQ |
| **开发者文档** | `dev.apeireth.io` | Architecture (41 crate 总览) / API (OpenAPI 3.1 自动渲染) / SDK (Python/TS/Rust 入门) / Extension (WASM 插件) / Self-host (Docker/k8s) |
| **营销页面** | `apeireth.io` | Landing (价值主张 + demo gif) / Features (9 器官可视化) / Pricing (R21 商业化前放 Roadmap) / Changelog (自动从 CHANGELOG.md 渲染) / Blog |
| **社区基础设施** | Discord + GitHub Discussions + Twitter | Discord (实时问答) / Discussions (功能投票) / Twitter (发版公告). R20 阶段 5 选 1 个主社区 (Discord 推荐) |

---

## §4 5 阶段实施路径

**总时长**: 7-10 周 (5 阶段 × 1-2 周)
**R20 周期**: 2026-08-05 ~ 2026-10-15 (估算)
**守门**: 每阶段结束 = 1 份 `reports/r20-stage<N>-complete-<date>.md` + 3 值 baseline 守住

### 阶段 1: 产品基础 (R20 P1.1, **1-2 周**)

| 任务 | 优先级 | 依赖 | owner 建议 | 验证 |
|------|-------|------|----------|------|
| TUI 9 命令深化 (Step 2: 6 类 API 端点) | 🔴 P0 | R19 Step 1 ✅ | backend_engineer | `curl localhost:8080/v1/organs` 返 JSON |
| TUI 消费端点 (Step 3) | 🔴 P0 | 阶段 1.1 | frontend_engineer | TUI 启动接 HTTP, 9 器官状态全显 |
| `apeireth-team-lead` 公开 API (REST 3 端点) | 🔴 P0 | 命名空间决策 (R19+ 集成蓝图) | backend_engineer2 | `curl -X POST /v1/team/spawn` 返 agent_id |
| 14 工具公开化 (R19+ 集成蓝图 §2.1 #1) | 🔴 P0 | `apeireth-team-lead` 基础 | fullstack_engineer | OpenAPI 规范含 14 tool |
| mid-task bug 3 处修法 (集成蓝图 §4.4) | 🔴 P0 | 新 `apeireth-session` crate (1500-2000 LOC) | backend_engineer | 3 处单测 PASS + 集成测试 PASS |
| **R-Measure 守门** | 🔴 P0 | — | verifier | V1141 ≥ 0.8682 / V1131 ≥ 0.8532 / V1136 ≥ 0.9063 |

**owner**: backend_engineer (主) + frontend_engineer (TUI 改瘦) + backend_engineer2 (team-lead)

### 阶段 2: 部署基础 (R20 P1.2, **2 周**)

| 任务 | 优先级 | 依赖 | owner 建议 | 验证 |
|------|-------|------|----------|------|
| Docker image 多架构 (buildx) | 🔴 P0 | 阶段 1 (API 稳定) | devops_engineer | `docker run --platform linux/arm64 apeireth/api:tag` 启动 |
| 推 GHCR + Docker Hub | 🔴 P0 | 阶段 2.1 | devops_engineer | `docker pull ghcr.io/apeireth/api:latest` 成功 |
| 离线包 (tarball + zip) | 🟡 P1 | 阶段 2.1 | devops_engineer | 离线环境 `tar -xf apeireth-offline.tar.gz && ./install.sh` 成功 |
| install.sh (Linux/macOS) | 🔴 P0 | — | devops_engineer | `curl install.apeireth.io \| sh` 跑通 (含 OS/arch 检测 + 依赖引导 + API key 配置) |
| install.ps1 (Windows) | 🟡 P1 | — | devops_engineer | PowerShell 7+ `iwr install.apeireth.io \| iex` 跑通 |
| Linux deb/rpm 包 | 🟡 P1 | 阶段 2.4 | devops_engineer2 | `apt install apeireth` / `dnf install apeireth` 成功 |
| macOS Homebrew formula | 🟡 P1 | 阶段 2.4 | devops_engineer2 | `brew install apeireth/tap/apeireth` 成功 |
| Windows MSI / scoop | 🟢 P2 | 阶段 2.4 | devops_engineer2 | `scoop install apeireth` 成功 (MSI 选 1) |
| **R-Measure 守门** | 🔴 P0 | — | verifier | 3 值守住 (部署不影响 ASI) |

**owner**: devops_engineer (主, 镜像+离线+脚本) + devops_engineer2 (系统包)

### 阶段 3: API 公开 (R20 P1.3, **2 周**)

| 任务 | 优先级 | 依赖 | owner 建议 | 验证 |
|------|-------|------|----------|------|
| HTTP REST wrapper (4 协议 + 6 V2 = 10 端点) | 🔴 P0 | 阶段 1.2 | backend_engineer | 10 端点集成测试 PASS |
| WebSocket 双向流 (`/v1/stream`) | 🔴 P0 | 阶段 1.5 (mid-task bug 修完) | backend_engineer2 | 客户端订阅, 收到 ProviderEvent 流 |
| OpenAPI 3.1 规范 (`openapi.yaml`) | 🔴 P0 | 阶段 3.1 | technical_writer | swagger-cli 校验通过, Redoc 渲染无 error |
| API 认证 (API key, 基础 rate limit) | 🟡 P1 | 阶段 3.1 | security_reviewer | 错误 API key 返回 401, 限流返回 429 |
| **R-Measure 守门** | 🔴 P0 | — | verifier | 3 值守住 |

**owner**: backend_engineer (REST) + backend_engineer2 (WS) + technical_writer (OpenAPI 规范)

### 阶段 4: SDK 完善 (R20 P1.4, **1-2 周**)

| 任务 | 优先级 | 依赖 | owner 建议 | 验证 |
|------|-------|------|----------|------|
| `apeireth-sdk` Cargo.toml + src 补全 (T13 CONCERN BLOCK) | 🔴 P0 | — | fullstack_engineer | `cargo build -p apeireth-sdk` 0 error |
| Python SDK (`pip install apeireth`) | 🔴 P0 | 阶段 3.1 (REST) | fullstack_engineer | `from apeireth import ApeirethClient; c = ApeirethClient(); c.chat("hi")` 跑通 |
| TypeScript SDK (`npm install @apeireth/sdk`) | 🟡 P1 | 阶段 3.1 | fullstack_engineer | `import { Apeireth } from "@apeireth/sdk"; const c = new Apeireth(); c.chat("hi")` 跑通 |
| Rust SDK (复用 `apeireth-sdk` crate) | 🟡 P1 | 阶段 3.1 | fullstack_engineer | `let c = ApeirethClient::new(); c.chat("hi").await?` 跑通 |
| SDK 文档 + 3 语言示例 (each ≥ 5 个) | 🟡 P1 | 阶段 4.1-4.4 | technical_writer | `docs/sdk/{python,typescript,rust}/` 各 5 示例 |
| **R-Measure 守门** | 🔴 P0 | — | verifier | 3 值守住 |

**owner**: fullstack_engineer (主) + technical_writer (文档+示例)

### 阶段 5: 文档 + 营销 (R20 P1.5, **1-2 周**)

| 任务 | 优先级 | 依赖 | owner 建议 | 验证 |
|------|-------|------|----------|------|
| 用户文档站 (`docs.apeireth.io`, Docusaurus) | 🟡 P1 | 阶段 4 (SDK 稳) | technical_writer | 5 分钟 Quick Start 走通 |
| 开发者文档站 (`dev.apeireth.io`) | 🟡 P1 | 阶段 3.3 (OpenAPI) | technical_writer | OpenAPI 自动渲染 + Architecture 总览 |
| Landing page (`apeireth.io`) | 🟡 P1 | — | frontend_engineer | 价值主张 + demo gif + CTA |
| 社区基础设施 (Discord 主选) | 🟢 P2 | — | community_manager | Discord 邀请链接 + 频道分类 + bot 接入 |
| **R-Measure 守门** | 🔴 P0 | — | verifier | 3 值守住 |

**owner**: technical_writer (文档) + frontend_engineer (landing) + community_manager (社区)

---

## §5 跟现有 41 crate 的集成

> **来源**: CHANGELOG.md v2.0.0-alpha + ROADMAP.md R19 工程化收尾, 当前 workspace = 42 members (25 完整实装 + 2 skeleton placeholder + 1 DEPRECATED + 5 v2 新 crate (mcp/formal/vector/graph/sdk) 部分实装).

### 5.1 关键 crate (R20 主用)

| crate | R19 状态 | R20 阶段 | 关键改动 |
|-------|---------|---------|---------|
| **`apeireth-api`** | 4 协议 + 6 V2 端点 (已实装) | 阶段 3 | 加 REST wrapper + API key 认证 + rate limit |
| **`apeireth-protocol`** | 4 协议 LLM 抽象 (已实装) | 阶段 3 | 暴露 WebSocket ProviderEvent 流 (公开) |
| **`apeireth-team-lead`** | ⚠️ 命名空间决策, 新建 (R19+) | 阶段 1 | 公开 REST 3 端点 + 14 工具 |
| **`apeireth-session`** | ❌ 缺失 (R19+) | 阶段 1 | 新建 1500-2000 LOC, mid-task bug 3 处一起改 |
| **`apeireth-tui`** | R19 Step 1 ✅, Step 2/3 ⏸️ | 阶段 1 | 续 Step 2/3 (9 命令深化) |
| **`apeireth-mcp`** | skeleton (已实装) | 阶段 1 | `apeireth-mcp::team` 14 工具公开化 |
| **`apeireth-sdk`** | ❌ 缺 Cargo.toml/src (T13 CONCERN BLOCK) | 阶段 4 | 补全 + TS/Python/Rust 3 SDK |
| **`apeireth-pybridge`** | cdylib 1100 模块 (已实装, 已知 pyo3+rlib 冲突) | 阶段 4 | Python SDK 入口, R18-2 解决编译问题 |

### 5.2 部署 crate (R20 阶段 2 重点)

| crate | 角色 |
|-------|------|
| **`apeireth-supervisor`** | PID 1 进程级 supervisor, systemd 集成 (Linux), launchd (macOS), Windows Service (Windows) |
| **`apeireth-bus`** | 5 层通信总线, 阶段 2.4 install 脚本用 L4 WebSocket 检测依赖服务 |
| **`docker/`** | 18 Dockerfile + docker-compose + 4 k8s YAML, 阶段 2.1-2.2 多架构 buildx 改造 |

### 5.3 文档 crate (R20 阶段 5 重点)

| 路径 | 角色 |
|------|------|
| **`GLOSSARY.md`** | 术语表, 用户文档 Quick Start 引用 |
| **`README.md`** | 项目入口, 加 R20 阶段状态 + 安装命令 |
| **`docs/`** | 4 个 docs 站根 (用户/开发者/landing/community) |
| **`CHANGELOG.md`** | 自动发版, R20 阶段 5 配 RSS |

---

## §6 R-Measure baseline 守门

> **依据**: APEIRETH-CONVENTIONS.md §11 R-Measure baseline 3 值, R20 加新功能**必须守住**.

| 指标 | 值 | 含义 | R20 守门 |
|------|---|------|---------|
| **V1141-R11** | 0.8682 | IC-001 fresh 测量 (**17 维 V0.5 R11 baseline 投影**, 当前实装 24 维 LOCKED per `crates/apeireth-asi/`, 投影见 r-measure-verification-design §2.1) | ≥ 0.8682 (不能掉) |
| **V1131-R11** | 0.8532 | dashboard v05_total (17 维 V0.5 历史 baseline 综合) | ≥ 0.8532 (不能掉) |
| **V1136-R11** | 0.9063 | 真测引擎 7 子测度 (R11 baseline 投影源, 当前实装 9 子测度 LOCKED per round10-12) | ≥ 0.9063 (不能掉) |

**每阶段结束 = 跑 R-Measure 三值** (verifier 角色), 报告写入 `reports/r20-stage<N>-measure-<date>.md`.

**R-Measure verify 必跑范围** (2026-08-05 13:50 更新, 14:30 17→24 维纠正):

- **V1141 ≥ 0.8682** (R-Measure verify 必跑, 任何 42 crate 改动 + 3 SDK 端到端验证, 含新增 apeireth-web)
  - R-Measure 验证脚本设计见 `docs/stage4/r-measure-verification-design-2026-08-05.md` (🔥 2026-08-05 14:30 17→24 维纠正: 当前实装 24 维, 17 维是 R11 baseline 投影源, 17→24 维权重等主人从 v1077 抽)
  - 3 SDK 端到端测试也算 (因为他们消费 `apeireth-api`, 影响集成质量)
  - 验证触发: 42 crate 任何 src 改动 → CI 必跑 R-Measure verify, 不通过 merge 阻塞
  - 阶段报告必含: 改动清单 + R-Measure 三值 + 3 SDK 端到端 PASS 证据

**风险点**:
- 阶段 1 (mid-task bug 修法): 状态机改动, 可能影响 V1131 dashboard 5 Self 总值 → 守门要紧
- 阶段 3 (API 公开): 公开端点不影响 ASI 计算, 但加 rate limit 不能引入 sleep/wait 拖慢 dashboard
- 阶段 4 (SDK): SDK 是薄包装, 不应影响 baseline; 但 ts-rs 自动生成类型若改 core struct, 要回归

---

## §7 不修改承诺 (8 项)

> **依据**: APEIRETH-CONVENTIONS.md §10 7 项 LOCKED + ADR-0011 + CHANGELOG v2.0.0-alpha Limitations.

| # | 不修改项 | 原因 |
|---|---------|------|
| 1 | 阶段 1+2+3 LOCKED 文档 | 主人明确沉淀 |
| 2 | v2 / v4 / v4.1 LOCKED | 哲学层纲领 |
| 3 | 阶段 4 主文档 LOCKED (6ca80776) | 落实架构定稿 |
| 4 | 阶段 5 施工文档 LOCKED (631 行) | 施工蓝图定稿 |
| 5 | v6 基础架构 | 4 重守门 + 权限发放 + E 层修改路径 |
| 6 | R11 baseline 三值 | V1141=0.8682 / V1131=0.8532 / V1136=0.9063 |
| 7 | **APEIRETH-CONVENTIONS.md** / **VERSIONING.md** / **GLOSSARY.md** | 12 子规范系统, R20 期间**只加 R20 元信息, 不改内容** |
| 8 | **workspace version 1.0.0** (Cargo.toml) | semver 严格, R20 是产品功能**不变 major** (1.x.x 系列递增) |
| + | **START-CONSTRUCTION.md** | 开工手册, 不动 |
| + | **apeireth-legacy/** | 物理归档, 仅增不删 100% 守住 (R17 收尾确认) |

**R20 新增允许**: `docs/roadmap/` (本文件) + `docs/sdk/` (4 阶段新建) + `docs/api/openapi.yaml` (3 阶段新建) + `reports/r20-*` (5 阶段各 1 份) + `crates/apeireth-sdk/` (4 阶段补全) + `crates/apeireth-session/` (1 阶段新建) + `crates/apeireth-team-lead/` (1 阶段新建).

---

## §8 风险清单 (8 项)

| # | 风险 | 严重度 | 缓解 |
|---|------|-------|------|
| **R-001** | R20 工作量 vs 1-2 周/阶段 (总 7-10 周). 实际 5 阶段串行依赖, 任何一阶段延期 → 全盘推 | 🟡 中 | 阶段 1-2 可并行 (产品+部署), 阶段 3-4 严格串行. 主人周会看进度 |
| **R-002** | Tauri 2 团队进度不在 R20 范围. R20 阶段 1 TUI 深化若发现需要等 Tauri 团队反馈 → 阻塞 | 🟡 中 | R20 阶段 1-2 决策点: 跟 Tauri 团队**周会同步**, 不假设他们交付 |
| **R-003** | Docker image 多架构构建复杂度. buildx + apeireth-pybridge cdylib 跨平台编译 (linux/arm64 on x86_64 host) | 🔴 高 | 阶段 2.1 PoC 先做 1 架构 (linux/amd64) 跑通, 再加 arm64. CI 用 QEMU emulation |
| **R-004** | 离线包大小. apeireth-pybridge 1100 Python 模块 + 5 协议 LLM 抽象依赖, 预估 1.5-2GB | 🟡 中 | 阶段 2.3 评估: 拆 core (500MB) + extras (1GB) 2 包, 用户按需下载 |
| **R-005** | OpenAPI 规范跟 4 协议 LLM 抽象的同步. 协议字段变了 OpenAPI 没更新 → SDK 用户调用失败 | 🔴 高 | 阶段 3.3: CI 加 `openapi-spec-validator` + `swagger-cli validate`, 不通过 merge 阻塞. 4 协议字段从 `apeireth-protocol` 自动生成 OpenAPI schema |
| **R-006** | SDK 多语言维护成本. Python/TS/Rust 3 SDK × 10+ 端点 = 30+ 方法签名同步 | 🔴 高 | 阶段 4.5: SDK 方法签名从 OpenAPI 自动生成 (openapi-generator), 3 语言共用同一 schema. 手写只写高层 wrapper |
| **R-007** | API 公开后安全攻击面. 内部 41 crate 库边界 → 公开 HTTP 端点, 未授权访问/注入 | 🟡 中 | 阶段 3.4: API key 认证 + rate limit + security_reviewer 阶段 3-4 渗透测试. 阶段 5 文档站加 SECURITY.md 链接 |
| **R-008** | R20 商业化前提 (计费/订阅/配额) 误启动. 主人 2026-08-04 12:30 提到"含计费+订阅+API 配额", R20 范围 vs R21 范围混淆 | 🟡 中 | **本路线图明确 R20 = 收产品, R21 = 商业化**. Mavis 拍板时跟主人确认 |
| **R-009** | `apeireth-sdk` 现有 ~14000 LOC low-level FFI 是测试入口 (C-ABI + 版本协商 + WireFormat). R20 阶段 4 加 `src/client.rs` + `http.rs` + `ws.rs` + 3 SDK 共享 `ApeirethClient` 抽象, 要保证**向后兼容**, C-ABI 边界不能破坏 | 🔴 高 | ① `apeireth-sdk/src/abi.rs` 3 个 `#[no_mangle] extern "C"` stub (`apeireth_sdk_init` / `apeireth_sdk_last_error` / `_ensure_error_linked`) 不能改, R20 加的 `client/http/ws` 走**内部 API**, 不暴露新 C-ABI ② 验证: 3 SDK 端到端测试 (5 个用例, R20 §3.3) + R-Measure verify 守门 (R20 §6) ③ `apeireth-sdk` package version workspace 1.0.0, `src/version.rs` SDK_VERSION 升 0.1.0 → 1.0.0 跟 R20 阶段 3 OpenAPI 同周期 |

---

## §9 哲学 anchor 6 全穿透

> **依据**: APEIRETH-CONVENTIONS.md §9 主哲学 6 锚穿透系统, R20 路线图每节必须穿透.

| 锚 | 来源 | R20 落地 |
|----|------|---------|
| **S-1** (主 22:33) | 北极星导向 — 服务 ASI 北极星 | R20 收产品 = ASI 完整性的工程化. 9 器官可见化 + API 公开 = 用户能看到 AI 成长过程 (R20 §1.1 比喻) |
| **S-2** (主 17:43) | 实事求是 — 基于现状不重写 | R20 复用 R19 工程基线 (9/9 业界标准 + 116 集成测试) + R19+ 集成蓝图 (A 方案已拍板). **不重写 41 crate**, 只加 REST wrapper + SDK 入口 |
| **O-5** (主 17:58) | 不假装 — 12 键编译时拒绝 | R20 守门: R-Measure 3 值 ≥ baseline (R20 §6). 阶段报告**不假装 "全部完工"**, 标 🔴/🟡/🟢 真实状态 |
| **O-2** (主 19:33) | 走在前人经验上 — 借鉴 | R20 借鉴: Docusaurus (用户文档) / openapi-generator (SDK 自动生成) / cargo-deb/rpm/wix (系统包) / Homebrew (macOS) / scoop (Windows) — 全部业界共识, 不自造 |
| **O-3** (主 23:44) | 干到底 — 决策立刻沉淀 | R20 路线图**本文件** = 决策沉淀. 5 阶段每结束 = 1 份 `reports/r20-stage<N>-complete-<date>.md`. 不停留在"讨论" |
| **O-4** (主 00:56) | 任何人都能接手 — 4 件套齐全 | R20 文档: 5 阶段 × 任务表 + 验证 + owner 建议 + 风险 (本文件 §4/§8) + R-Measure 守门 (§6) + 不修改承诺 (§7) + 关联文档 (§10). 接手者能查 |

**穿透检查清单** (APEIRETH-CONVENTIONS §9):
- [x] S-1: R20 服务 ASI 北极星 (产品化让用户看见成长)
- [x] S-2: R20 不重写, 复用 R19 工程基线
- [x] O-5: R20 守门 R-Measure 3 值, 阶段报告真实标
- [x] O-2: R20 借鉴业界工具链 (Docusaurus/openapi-generator/cargo-deb 等)
- [x] O-3: R20 决策立刻沉淀 (本文件 + 5 阶段报告)
- [x] O-4: R20 4 件套齐全 (任务/验证/owner/风险 + 守门 + 承诺 + 关联)

---

## §10 关联文档

| 文档 | 角色 | 路径 |
|------|------|------|
| **ROADMAP.md** | 顶层路线图, R20 = 🟡 P1 决策 | `.openclaw\workspace\promethean\Apeireth-rust\ROADMAP.md` §下一阶段建议第 3 行 |
| **CHANGELOG.md** | v2.0.0-alpha 状态 (22 任务 10 DONE + 5 PARTIAL + 6 BLOCKED + 1 TODO) | `.openclaw\workspace\promethean\Apeireth-rust\CHANGELOG.md` |
| **APEIRETH-VERSIONING.md** | 版本号系统 (7 子系统 + Document-Meta 格式) | `.openclaw\workspace\promethean\Apeireth-rust\APEIRETH-VERSIONING.md` |
| **APEIRETH-CONVENTIONS.md** | 12 子规范系统 (含 §9 6 锚 + §10 7 不修改承诺 + §11 R-Measure 3 值) | `.openclaw\workspace\promethean\Apeireth-rust\APEIRETH-CONVENTIONS.md` |
| **R19+ SpectrAI 集成蓝图** | A 方案已拍板 2026-08-05, R20 阶段 1 衔接 | `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md` |
| **R19 TUI 升级路线图** | Step 1 ✅ (改瘦) + Step 2/3 ⏸️ (R20 阶段 1 续) | `docs/v2-strategy/06-TUI-UPGRADE-ROADMAP.md` |
| **R19+ Tauri 资产沉淀** | T-001~T-013 13 项 (R20 不在范围, 另一团队) | `docs/stage4/tauri-assets-from-spectrAI-2026-08-05.md` |
| **apeireth-sdk 缺失分析** | `apeireth-sdk` 现状 (11 文件 ~14000 LOC) + 5 步补全方案 (R20 §3.3 🔥 修正来源) | `docs/stage4/apeireth-sdk-gap-analysis-2026-08-05.md` |
| **Tauri 团队对接 SOP** | 双团队边界 + 5 步 SOP + 10 项资产传递 (Tauri 团队 lead 必读) | `docs/stage4/tauri-team-collab-sop-2026-08-05.md` |
| **全局架构图** | 1 张总图 + 13 张子图 Mermaid (5 层: 用户入口 / 接入 / 42 crate / 4 协议 / 基础层) | `docs/stage4/global-architecture-map-2026-08-05.md` |
| **ADR-0010~0012** | R19+ 集成 ADR 系列 (A 方案决策) | `docs/adr/0010-0012-*.md` |
| **R19 v2.0.0-alpha 总报告** | 22 任务矩阵 + B1-B9 风险 + R18+ 11 项清单 | `reports/v2-final-summary-2026-08-05.md` |
| **R19 决策简报** | 主人签收 R19 5 决策 | `reports/v2-decision-brief-2026-08-05.md` |

---

## §11 待 Mavis 拍板的事 (3 项)

> **本路线图为草拟, Mavis 跟主人 2026-08-04 12:30 方向复核时, 3 项需拍板**:

### 11.1 R20 vs R21 边界 (🔴 关键)

主人 2026-08-04 12:30 提到"含计费+订阅+API 配额" — **本路线图默认 R20 = 收产品 (不含商业化), R21 = 商业化**.

**拍板**:
- (A) ✅ 本路线图默认: R20 收产品 / R21 商业化 (推荐, 风险 R-008)
- (B) R20 直接含基础计费 (Stripe 接入 + API key 配额管理), 商业化推到 R22

### 11.2 Tauri 团队同步节奏 (🟡 中)

R20 阶段 1-2 期间, 主人希望 TUI 深化决策是否要等 Tauri 团队反馈?

**拍板**:
- (A) ✅ TUI 独立做 (R20 阶段 1-2 不假设 Tauri 进度, 每周跟 Tauri 团队 1 次同步)
- (B) TUI 等 Tauri 团队设计语言定稿再深化 (R20 阶段 1 延 2-3 周)

### 11.3 SDK 三语言优先级 (🟡 中)

阶段 4 1-2 周, Python/TS/Rust 3 SDK 串行做还是并行?

**拍板**:
- (A) ✅ 串行: Python 先 (apeireth-pybridge 已有基础) → TS → Rust (推荐, 风险低)
- (B) 并行: 3 SDK 同时开工, 加 1 周 (总 2-3 周) + 1 个 lead 协调

### 11.4 apeireth-sdk 升级方案 (🟡 中, 2026-08-05 13:50 新增)

`apeireth-sdk` 现有 11 文件 ~14000 LOC (low-level FFI), R20 阶段 4 一起做还是分阶段?

**拍板**:
- (A) ✅ 一起做 (推荐): R20 阶段 4 = `src/client.rs` + `http.rs` + `ws.rs` + 3 SDK 共享 `ApeirethClient` 抽象. 3 SDK 一起出, 1 周完成
- (B) 分阶段: 阶段 4a (client/http/ws) → 阶段 4b (3 SDK 端到端). 总 1.5-2 周, 但阶段 4b 验证 cross-language 一致性更稳

### 11.5 SDK_VERSION 升 0.1.0 → 1.0.0 (🟡 中, 2026-08-05 13:50 新增)

`src/version.rs` 当前 `SDK_VERSION = 0.1.0` (协议层 wire-format 版本), workspace `Cargo.toml` 是 1.0.0 (crate 自身版本). 两者不同.

**拍板**:
- (A) ✅ R20 阶段 4 一起升 0.1.0 → 1.0.0 (推荐, 跟 R20 阶段 3 OpenAPI 规范同周期)
- (B) 保持 0.1.0 (协议层未稳定, 不动)

### 11.6 apeireth-tauri-stub 命名 (🟢 低, 2026-08-05 13:50 新增)

`apeireth-tauri-stub` 当前 DEPRECATED (R17 stub, `publish=false`), workspace 还在. R20 阶段 4 之前要不要从 workspace 移除?

**拍板**:
- (A) ✅ 留 (推荐): R21+ 评估, 不动 workspace 成员 (避免影响 R20 阶段 1-5 编译)
- (B) R20 阶段 1 从 workspace 移除 + `apeireth-legacy/` 物理归档

---

## §12 时间线总览

```
2026-08-05 (R20 启动, v2.0.0-alpha 发版)
  ↓
2026-08-06 ~ 08-19 (Week 1-2): 阶段 1 产品基础 (TUI 深化 + team-lead + mid-task bug)
  ↓
2026-08-20 ~ 09-02 (Week 3-4): 阶段 2 部署基础 (Docker 多架构 + 系统包 + 离线包)
  ↓
2026-08-20 ~ 09-02 (Week 3-4, 并行): 阶段 3 API 公开 (REST wrapper + WS + OpenAPI)
  ↓
2026-09-03 ~ 09-16 (Week 5-6): 阶段 4 SDK 完善 (Python/TS/Rust + 文档示例)
  ↓
2026-09-17 ~ 09-30 (Week 7-8): 阶段 5 文档+营销 (文档站 + landing + 社区)
  ↓
🎯 7-8 周 = R20 收产品完工 (2026-09-30 目标)
  ↓
2026-10-01 ~ R21 商业化 (计费 + 订阅 + API 配额) ← 下一周期
```

---

_本路线图 v1 草拟 (按主人 2026-08-04 12:30 R20 P1 方向 + 2026-08-05 收产品细化)._
_3 项拍板 (R20 vs R21 边界 / Tauri 同步 / SDK 优先级) 待 Mavis 跟主人 2026-08-05 复核._
_主哲学 6 锚穿透. 任何接手者能查. 阶段报告 5 份 (R20 §4) 跟 R19 系列对齐._
_下一步: 拍板后 → 5 阶段 sub-task 拆解 (R20 task list) → 派 4 个 owner (backend/devops/fullstack/technical_writer) 开工._

---

## 拍板记录

- **2026-08-05 13:34** — 主人拍板 A 方案 `apeireth-team-lead` (ADR-0011, R19+ 集成蓝图 §5 命名空间决策)
- **2026-08-05 13:50** — sub-agent 报告: `apeireth-sdk` 现有 11 文件 ~14000 LOC (low-level FFI 测试入口), R20 §3.3 + §5.1 误读为"空 Cargo.toml / T13 CONCERN BLOCK" 已回写, 加 🔥 修正标签. 来源: `docs/stage4/apeireth-sdk-gap-analysis-2026-08-05.md`
- **2026-08-05 14:00** — sub-agent 报告: 13 张 Mermaid 全局架构图完成 (`docs/stage4/global-architecture-map-2026-08-05.md`), R20 §10 关联文档已引用. R20 §6 R-Measure verify 守门 + §8 R-009 (apeireth-sdk 向后兼容) + §11.4-11.6 (3 个新拍板项) 同步加完
