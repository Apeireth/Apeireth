[Document-Meta]
Document: docs/stage4/tauri-team-collab-sop-2026-08-05.md
Version: Manual-Rev-A
R-Cycle: R20 阶段 4
Commit: <commit 时回填>
Last-Modified: 2026-08-05
Status: 🔍 草拟 (待 Mavis 拍板 + Tauri 团队 lead 复核)

---

# Tauri 团队对接 SOP (R20 阶段 4)

> **写给两边看**: Apeireth 团队 (本工作树) + Tauri 团队 (独立工作树) 共同遵守.
> **目的**: Tauri 2 .exe 桌面由**另一团队**做, 不在我们工作树. 本文档给双方的对接 SOP, 避免"隐形资产"和"信息孤岛".
> **依据**: 主人 2026-08-04 19:53 砍前端决策 + R17 战略 3 标记 + 13 项 Tauri 资产沉淀 (`tauri-assets-from-spectrAI-2026-08-05.md`).
> **Tauri 团队 lead 必读**: §2.3 共享边界 + §3 协同流程 + §5 资产传递清单 (你负责的列).

---

## §1 战略背景 (为什么)

### 1.1 为什么分两个团队

主人 2026-08-04 19:53 决定: **Apeireth 团队干 TUI + 后端, AI 团队干 Tauri 设计**. 原因:

- 主人自己干 dev (TUI/后端) — 主人擅长 Rust + 后端架构
- AI 团队干设计 (Tauri UI) — AI 擅长审美 + 跨平台 UI
- 缺审美设计时, 主人宁愿 TUI 也不上没设计感的 web/桌面 (user memory #8)

**Tauri 团队 = 另一团队**, 跟我们**不在同一工作树**:
- Apeireth 仓库: `.openclaw\workspace\promethean\Apeireth-rust\`
- Tauri 仓库: (待 Tauri 团队决定, 可能是 `github.com/apeireth/apeireth-tauri` 或私有仓库)
- 沟通: 不靠 "我去改你的代码", 靠**文档透明 + API 契约 + 资产沉淀**

### 1.2 我们这边不直接做 Tauri UI

但需要"透明传递资产":
- HTTP API 契约 (我们定, Tauri 消费)
- WebSocket 协议 (我们定, Tauri 消费)
- OpenAPI 3.1 规范 (technical_writer 定, 双方共用)
- 13 项 Tauri 资产 (我们沉淀, 见 `tauri-assets-from-spectrAI-2026-08-05.md`)

**原则**:
- ❌ 不假设 Tauri 团队会读 Apeireth 源码
- ❌ 不假设 Tauri 团队会改 Apeireth 仓库
- ✅ 一切"跨团队资产"必须在文档里写明
- ✅ 文档跟代码同步 (CI 校验)

### 1.3 13 项 Tauri 资产沉淀

per `docs/stage4/tauri-assets-from-spectrAI-2026-08-05.md`:
- T-001 Tauri command 映射 (11 类 IPC, 3797 LOC)
- T-002 Electron BrowserWindow 多窗口 (~1400 LOC)
- T-003 系统托盘 + 应用更新 (460 LOC)
- T-004 macOS/Linux PATH 引导 (107 LOC)
- T-005 结构化日志读取器 (477 LOC)
- T-006 单实例锁 (~20 LOC)
- T-007 主题覆盖层 (~50 LOC)
- T-008 启动 Stage 0-7 (107 LOC)
- T-009 文件管理 IPC (~250 LOC)
- T-010 工作区 IPC (~200 LOC)
- T-011 Git 操作 IPC (946 LOC)
- T-012 Confirmation 解析器 (543 LOC)
- T-013 文件变化追踪 UI (511 LOC)

**总计**: 13 项, ~6272 LOC, 全部 SpectrAI v0.9.21 实战检验过.

---

## §2 双团队边界 (谁负责什么)

### 2.1 Apeireth 团队负责 (本工作树)

| 资产 | 路径 | 状态 |
|------|------|------|
| **apeireth-api** (HTTP API, 4 协议 + 6 V2 端点) | `crates/apeireth-api/` | R19 已实装, R20 阶段 3 加 REST wrapper |
| **apeireth-protocol** (4 adapter) | `crates/apeireth-protocol/` | R19 已实装, R20 阶段 3 加 WS |
| **apeireth-team-lead** (团队 leader, R19+ 新命名) | `crates/apeireth-team-lead/` | R20 阶段 1 新建 |
| **apeireth-mcp::team** (14 工具) | `crates/apeireth-mcp/src/team.rs` | R20 阶段 1 公开化 |
| **apeireth-sdk** (3 SDK 入口) | `crates/apeireth-sdk/` | R20 阶段 4 补全 (见 `apeireth-sdk-gap-analysis-2026-08-05.md`) |
| **文档** (用户/开发者) | `docs/`, `README.md`, `CHANGELOG.md`, `GLOSSARY.md` | R20 阶段 5 |
| **OpenAPI 3.1 规范** | `docs/api/openapi.yaml` | R20 阶段 3 新建 |
| **13 项 Tauri 资产** | `docs/stage4/tauri-assets-from-spectrAI-2026-08-05.md` | R20 阶段 4 已沉淀 |
| **Docker image / 系统包** | `docker/`, `crates/apeireth-supervisor/` | R20 阶段 2 |
| **CI/CD pipeline** | `.github/workflows/`, `Cargo.toml` lints | R19 已实装 |

**Apeireth 团队不负责**:
- ❌ Tauri 2 .exe 桌面 app (Tauri 团队)
- ❌ Tauri command 翻译 (Tauri 团队)
- ❌ Tauri 主题/UI 设计 (Tauri 团队)

### 2.2 Tauri 团队负责 (另一工作树)

| 资产 | 角色 | 依赖 |
|------|------|------|
| **Tauri 2 .exe 桌面 app** | 主二进制 (Windows/macOS/Linux) | 调 `apeireth-api` HTTP + WS |
| **Tauri command 映射** (11 类 IPC) | `#[tauri::command]` 函数, 调 `apeireth-api` | `OpenAPI 3.1` 规范 (Apeireth 出) |
| **Tauri 多窗口管理** | `WebviewWindow` API, 聊天 + 设置 + 通知 | Tauri 官方 plugin |
| **Tauri 系统托盘** | `tauri-plugin-tray`, 4 项菜单 | 13 项 Tauri 资产 T-003 |
| **Tauri auto-update** | `tauri-plugin-updater` | 13 项 Tauri 资产 T-003 + 部署服务器 (devops_engineer) |
| **Tauri theme switcher** | 3 主题 (light/dark/auto) | 13 项 Tauri 资产 T-007 |
| **Tauri 单实例锁** | `tauri-plugin-single-instance` | 13 项 Tauri 资产 T-006 |
| **Tauri 文件操作** | `tauri-plugin-fs` | 13 项 Tauri 资产 T-009/T-010 |
| **Tauri 文件变化追踪** | `notify` crate + Tauri event | 13 项 Tauri 资产 T-013 |
| **Tauri 前端 UI 框架** | Svelte/Next.js/React (Tauri 团队选) | 13 项 Tauri 资产 T-005/T-008 |
| **Tauri 自动构建** (CI) | GitHub Actions / GitLab CI 跨平台 build | Tauri 官方工具链 |

**Tauri 团队不负责**:
- ❌ 后端 41 crate 任何代码 (Apeireth 团队)
- ❌ HTTP API 端点设计 (Apeireth 团队, Tauri 消费)
- ❌ OpenAPI 规范内容 (Apeireth 团队, Tauri 消费)
- ❌ apeireth-sdk 任何代码 (Apeireth 团队, Tauri 消费)

### 2.3 共享边界 (需协同, 双方都看)

> **Tauri 团队 lead 必读**: 本节是双方协同的接触面, 出问题先查这里.

| 边界 | Apeireth 出 | Tauri 消费 | 文档位置 | 同步方式 |
|------|------------|----------|---------|---------|
| **HTTP API 契约** | 4 协议 + 6 V2 = 10 REST 端点 | 11 类 IPC 翻译 | `docs/api/openapi.yaml` (R20 阶段 3) | OpenAPI 规范 + 契约测试 |
| **WebSocket 协议** | `/v1/stream` 双向流 | 流式消费 UI | `docs/api/websocket.md` (R20 阶段 3) | 协议文档 + 集成测试 |
| **OpenAPI 3.1 规范** | technical_writer 定 | 工具消费 (ts-rs / openapi-typescript) | `docs/api/openapi.yaml` (R20 阶段 3) | swagger-cli 校验 + Redoc 渲染 |
| **13 项 Tauri 资产** | Apeireth 团队沉淀 | Tauri 团队按表复用 | `tauri-assets-from-spectrAI-2026-08-05.md` (R20 阶段 4) | 已就绪, Tauri 阶段查表 |
| **11 类 IPC 映射** | Apeireth 团队 (OpenAPI 决定) | Tauri 翻译成 `#[tauri::command]` | R20 阶段 1 (跟 `docs/adr/0010-mcp-from-spectrai-agentmcpserver.md` 同周期) | OpenAPI 工具链 |
| **3 SDK 公开 API** | Rust/Python/TS SDK | Tauri 选 1 个 SDK 调 (推荐 Rust SDK) | `docs/sdk/{rust,python,typescript}/` (R20 阶段 4) | ts-rs + openapi-typescript 自动生成类型 |
| **14 工具 MCP** | `apeireth-mcp::team` | Tauri 暴露为 14 个 `#[tauri::command]` | `docs/api/team-tools.md` (R20 阶段 1) | OpenAPI 规范 + SDK |
| **部署产物** | Docker image / 系统包 | Tauri .exe 嵌 `apeireth-api` server 或指向远程 | R20 阶段 2 | 部署文档 |

---

## §3 协同流程 (5 步 SOP)

> **5 步可执行, 不是哲学**. 每步都有 owner + 验收 + 时长.

### Step 1: 资产沉淀 (已完成, 2026-08-05)

**Apeireth 团队 owner**: technical_writer
**验收**: 13 项 Tauri 资产表填全, 每项 8 列 (资产 / 原始 / 文件 / LOC / 何时 / 怎么用 / 风险 / 累计)
**文档**: `docs/stage4/tauri-assets-from-spectrAI-2026-08-05.md` ✅ (2026-08-05 完工)

**Tauri 团队 lead 行动**:
- 接手时第一件事: 通读 §2 13 行表, 确认"我要 X → T-XXX → SpectrAI 哪个文件"
- 必读: §1.4 战略原则 + §3 5 步走 SOP

### Step 2: API 契约发布 (R20 阶段 3, 2 周)

**Apeireth 团队 owner**: technical_writer (主) + backend_engineer (REST) + backend_engineer2 (WS)
**验收**:
- `docs/api/openapi.yaml` 通过 swagger-cli validate
- `docs/api/websocket.md` 写清 `/v1/stream` 双向流协议
- Redoc 渲染无 error
- 10 REST 端点集成测试全过 (CI)

**Tauri 团队 lead 行动**:
- 每周一 10:00 看 `git pull` Apeireth 仓库的 `docs/api/` 目录
- 关注 GitHub Issues 标签 `api-contract` (R20 阶段 3 开始用)

### Step 3: Tauri 团队订阅 (持续, R20 阶段 3 起)

**Tauri 团队 owner**: Tauri 团队 lead
**机制**:
- Tauri 仓库订阅 Apeireth 仓库 (GitHub Watch)
- 每周一 10:00 `git pull` Apeireth 仓库的 `docs/stage4/` 目录
- 重点看 `tauri-assets-from-spectrAI-2026-08-05.md` 增改行 (T-XXX ✅/❌ 标记)
- 重点看 `docs/roadmap/r20-product-finalize-2026-08-05.md` 阶段 1-5 状态

**Apeireth 团队承诺**:
- 改 `docs/stage4/tauri-assets-*` → 1 个工作日内 commit + 通知 Tauri 团队 lead
- 改 OpenAPI 规范 → 1 个工作日内 commit + 通知 Tauri 团队 lead
- 改 R20 路线图 → commit 即可 (Tauri 团队订阅自取)

### Step 4: 同步会议 (每 2 周 1 次)

**机制**:
- 频率: 每 2 周 1 次 (周一下午 14:00, 主人周会后 30 分钟)
- 参加: Apeireth 团队 lead + Tauri 团队 lead + Mavis (记录)
- 时长: 30 分钟 (15 分钟 Apeireth 进展 + 15 分钟 Tauri 进展)
- 输出: `reports/tauri-collab-sync-<date>.md` (每次会议 1 份)

**议程固定模板**:
1. Apeireth 这 2 周做了什么 (链接到 commits)
2. Tauri 这 2 周做了什么 (链接到 commits)
3. 双方阻塞的事 (issue + 谁来解决)
4. 下 2 周计划 (双方对齐)
5. 13 项 Tauri 资产 ✅/❌ 状态更新

**第一次会议**: 2026-08-19 (R20 阶段 3 中期).

### Step 5: Issue 跟踪 (GitHub Issues 标签)

**机制**:
- Apeireth 仓库: GitHub Issues 标签 `tauri-collab`
- Tauri 仓库: GitHub Issues 标签 `apeireth-collab` (镜像)
- 跨仓库问题: 双方各开 1 issue, 用 `Refs: xxx` 互引
- SLA: 1 个工作日内双方 lead 确认 + 分配

**Issue 模板** (跨仓库协同):
```markdown
## 类型
[ ] API 契约变更
[ ] 资产新增
[ ] 部署相关
[ ] Bug 报告
[ ] 文档不清晰

## 描述
<问题>

## 影响范围
- Apeireth: <版本/crate>
- Tauri: <版本/feature>

## 提议方案
<建议>

## 验收
<done 定义>
```

### 5 步 SOP 总览

```
Step 1 (完成)  ─→  Step 2 (R20 阶段 3)  ─→  Step 3 (持续)  ─→  Step 4 (每 2 周)  ─→  Step 5 (持续)
资产沉淀          API 契约发布            Tauri 订阅          同步会议            Issue 跟踪
(13 项表)        (openapi.yaml)         (git pull)          (周一 14:00)        (GitHub 标签)
```

---

## §4 文档透明原则

### 4.1 3 条铁律

1. **任何 Tauri 团队需要的资产, 先写到文档** (不藏在代码里)
2. **文档同步**: commit 时自动更新 (CI 校验 commit 改了 src 也要改对应文档)
3. **13 项 Tauri 资产全部维护在 `tauri-assets-from-spectrAI-2026-08-05.md`** (不要分散)

### 4.2 CI 校验机制 (R20 阶段 1 加)

```yaml
# .github/workflows/docs-sync-check.yml (R20 阶段 1 新建)
name: Docs Sync Check
on: [pull_request]
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Check src/ changes require docs/ updates
        run: |
          if git diff --name-only HEAD~1 | grep -E '^crates/.*/src/.*\.rs$' >/dev/null; then
            if ! git diff --name-only HEAD~1 | grep -E '^docs/' >/dev/null; then
              echo "❌ src/ changed but no docs/ update"
              exit 1
            fi
          fi
      - name: OpenAPI schema validate
        run: |
          if [ -f docs/api/openapi.yaml ]; then
            npx swagger-cli validate docs/api/openapi.yaml
          fi
```

**触发**:
- PR 改了 `crates/apeireth-api/src/*.rs` 但没改 `docs/api/` → 阻塞
- PR 改了 `crates/apeireth-sdk/src/*.rs` 但没改 `docs/sdk/` → 阻塞
- PR 改了 `crates/apeireth-mcp/src/team.rs` 但没改 `docs/api/team-tools.md` → 阻塞

### 4.3 资产不分散原则

- ❌ 13 项 Tauri 资产不要拆到 13 个文件 — 维护成本爆炸
- ✅ 全部在 `tauri-assets-from-spectrAI-2026-08-05.md` 一张表
- ✅ 新增 T-XXX 行 (按 8 列格式填)
- ✅ 用过的项标 ✅, 弃用标 ❌ + 原因

---

## §5 资产传递清单 (10 项)

> **这是给 Tauri 团队 lead 看的**. 4 列: 资产 / 路径 / 谁提供 / 谁用 / 何时. 双方各看自己负责的列.
>
> **Tauri 团队 lead 必读**: 找"我要做什么 → Apeireth 已经提供了什么 → 我不用重写".

| # | 资产 | 路径 | 谁提供 | 谁用 | 何时 |
|---|------|------|--------|------|------|
| **1** | **HTTP API 契约** (10 REST 端点) | `docs/api/openapi.yaml` (R20 阶段 3 新建) | Apeireth (backend_engineer) | Tauri (11 类 IPC 翻译) | R20 阶段 3 (2 周内) |
| **2** | **WebSocket 协议** (`/v1/stream` 双向流) | `docs/api/websocket.md` (R20 阶段 3 新建) | Apeireth (backend_engineer2) | Tauri (流式 UI 消费) | R20 阶段 3 |
| **3** | **14 工具 MCP 描述** (spawn_agent / send_to_agent / wait_agent_idle / ...) | `apeireth-mcp::team` 文档 + `docs/api/team-tools.md` | Apeireth (fullstack_engineer) | Tauri (暴露为 14 个 `#[tauri::command]`) | R19+ 阶段 1 (R20 阶段 1 公开化) |
| **4** | **team-lead 公开 API** (`/v1/team/spawn` / `/v1/team/agent/:id` / `/v1/team/agent/:id/idle`) | `crates/apeireth-team-lead/` + `docs/api/team-lead.md` | Apeireth (backend_engineer2) | Tauri (调 3 端点) | R19+ 阶段 3 (R20 阶段 1) |
| **5** | **13 项 Tauri 资产** (T-001 ~ T-013, ~6272 LOC) | `docs/stage4/tauri-assets-from-spectrAI-2026-08-05.md` | Apeireth (technical_writer) | Tauri (按表 1:1 翻译) | 已就绪 (2026-08-05) |
| **6** | **Tauri command 映射** (11 类 IPC → `#[tauri::command]`) | 11 类文档 + OpenAPI schema | Apeireth (OpenAPI 决定命名) | Tauri (1:1 翻译成 `#[tauri::command]`) | R20 阶段 1 (跟 OpenAPI 同步) |
| **7** | **多窗口管理** (聊天 + 设置 + 通知) | Tauri 团队自有 | Tauri | Tauri | Tauri 阶段 (R20 阶段 4 之后) |
| **8** | **系统托盘** (4 项菜单: 打开/退出/更新/关于) | Tauri 团队自有 (`tauri-plugin-tray`) | Tauri | Tauri | Tauri 阶段 |
| **9** | **auto-update** (`tauri-plugin-updater` + 部署服务器) | Tauri 团队自有 + devops_engineer 配部署服务器 | Tauri (UI) + devops_engineer (server) | Tauri | Tauri 阶段 |
| **10** | **theme switcher** (3 主题: light/dark/auto) | Tauri 团队自有 | Tauri | Tauri | Tauri 阶段 |

**关键观察**:
- 资产 1-6 由 **Apeireth 团队出**, Tauri 团队**消费** (Tauri 不用重写)
- 资产 7-10 由 **Tauri 团队自有** (Tauri 官方 plugin, 我们不参与)
- 资产 9 的"部署服务器"由 devops_engineer 配 (R20 阶段 2 一起)

**Tauri 团队 lead 行动 (按优先级)**:
1. 接手时**先读** `tauri-assets-from-spectrAI-2026-08-05.md` (13 项表) — 知道 SpectrAI 已实现什么
2. 等 OpenAPI 规范发布 (R20 阶段 3) — 知道 HTTP 端点怎么调
3. 复用 `apeireth-sdk` (R20 阶段 4 完工) — 直接用 Rust SDK 调 backend, 不用裸 reqwest
4. 11 类 IPC 命名空间跟 OpenAPI 一致 (避免 `agent_spawn` vs `spawn_agent` 这种冲突)

---

## §6 失败模式 + 防御

| 失败模式 | 触发场景 | 防御机制 |
|---------|---------|---------|
| **资产藏在代码里** | Tauri 团队接手时不知道 apeireth-team-lead 已实装 team spawn | ① 13 项资产全在文档表里 ② CI 校验 src 改 → docs 必改 ③ 阶段报告必引文档 |
| **双方 API 不同步** | Apeireth 改 `/v1/chat` 但 OpenAPI 没更新 → Tauri 调失败 | ① OpenAPI 规范是 single source ② `swagger-cli validate` 阻塞 PR ③ 契约测试 (Apeireth 服务跑 + Tauri 客户端跑同一组 test case) |
| **Tauri 团队用错资产** | Tauri 团队不知道 T-005 (OutputReader) 已沉淀, 自己重写 | ① 资产表每项 8 列 (何时/怎么用/风险) ② 同步会议 1 次/2 周 ③ GitHub Issues `tauri-collab` 标签 |
| **协同靠人 (Slack 喊一句)** | 关键决策只在 Slack 说, 没沉淀到文档 | ① 决策必有 issue ② issue 必有 ADR 或 docs/stage4/ 链接 ③ 会议纪要必写 `reports/tauri-collab-sync-<date>.md` |
| **Tauri 团队用了 v1 弃用 API** | SpectrAI V1 PTY 路径已弃用, Tauri 团队不知道 | ① 资产表明确标 V1 弃用 ② §3 蓝图引用 "⚪ 沉淀" 行 ③ Tauri 团队必读 `tauri-assets-*.md` §1.3 沉淀范围 |
| **R20 阶段 3 延期** | OpenAPI 规范没按时发, Tauri 团队等 | ① 阶段报告 `reports/r20-stage3-*.md` 必含 ② 延期 ≥ 1 周触发主人周会议题 ③ Tauri 团队先用 `apeireth-api` 直接 curl 调试 |
| **apeireth-sdk 公开 API 改 → 3 SDK 不同步** | 改了 Rust SDK 方法签名, Python/TS SDK 没跟 | ① OpenAPI 规范 single source ② ts-rs 自动生成 (Rust → Rust 类型) ③ openapi-typescript 自动生成 (openapi.yaml → TS 类型) ④ Pydantic 手动跟 (慢但稳) |
| **跨平台编译失败** (apeireth-pybridge cdylib) | macOS arm64 编译冲突 (R18-2 已知) | ① Python SDK 复用现有 `src-py/` (已生成) ② CI 3 平台 build 跑通再 merge ③ apeireth-pybridge 跨平台 issue 列表 (`reports/pybridge-issues-*.md`) |

---

## §7 实施时间表 (R20 阶段 4, 1 周)

| 阶段 | 时长 | 任务 | Owner | 验证 |
|------|------|------|-------|------|
| 1 | 0.5 天 | 等 code_reviewer 完工后补 apeireth-sdk 骨架 (Step 1) | rust-coder | `cargo build -p apeireth-sdk` 0 error |
| 2 | 1 天 | apeireth-sdk 公开 API 设计 (11 方法签名 + config + error) (Step 2) | backend_engineer + architect | `docs/sdk/api-design-2026-08-05.md` 通过 architect 拍板 |
| 3 | 1 天 | Python SDK 入口 (`pip install apeireth`, 11 方法) (Step 3) | fullstack_engineer | `from apeireth import ApeirethClient; c.chat("hi")` 跑通 |
| 4 | 1 天 | TypeScript SDK 入口 (`npm install @apeireth/sdk`) (Step 4) | fullstack_engineer | `import { Apeireth } from "@apeireth/sdk"; c.chat("hi")` 跑通 |
| 5 | 1 天 | Rust SDK 入口 (复用 apeireth-sdk crate) + 端到端验证 (Step 5) | backend_engineer | `cargo add apeireth-sdk` + 3 SDK 各跑 1 smoke |
| **总计** | **4.5 天** | (1 周) | — | 3 SDK 公开 + 11 方法端到端 PASS |

**关键里程碑**:
- **Day 1 结束**: `cargo build -p apeireth-sdk` 0 error
- **Day 2 结束**: 11 方法签名定稿 (后续 SDK 全部跟)
- **Day 3 结束**: Python SDK 跑通
- **Day 4 结束**: TypeScript SDK 跑通
- **Day 5 结束**: Rust SDK 跑通 + 3 SDK 端到端 PASS

**同步会议 (跟 Tauri 团队)**:
- **Day 1 (周一 14:00)**: apeireth-sdk 骨架完工, 通知 Tauri 团队 lead
- **Day 2 (周二 14:00)**: 11 方法签名定稿, 通知 Tauri 团队 lead 复核
- **Day 5 (周五 14:00)**: 3 SDK 完工, 通知 Tauri 团队 lead 端到端验证结果

---

## §8 风险清单 (8 项)

| # | 风险 | 严重度 | 缓解 |
|---|------|-------|------|
| **R-001** | apeireth-pybridge cdylib 编译冲突 (R18-2 已知, `pyo3 + rlib` 互斥) | 🟡 中 | Python SDK 复用 `src-py/` 现有 (已生成), 不重做 cdylib; CI 3 平台 build 验证 |
| **R-002** | 3 SDK 维护成本 (改 apeireth-sdk 公开 API → 3 SDK 同步) | 🟡 中 | OpenAPI 规范做 single source + ts-rs/openapi-typescript 自动生成 |
| **R-003** | 跨语言 ABI 不一致 (Rust/Python/TS 数据结构映射) | 🔴 高 | 跨语言 contract 集成测试 (3 SDK 同输入 → 同输出) |
| **R-004** | Tauri 团队节奏不可控 (不在我们工作树, 延期无直接控制) | 🔴 高 | 5 步 SOP + 每 2 周会议 + GitHub Issue 跟踪; 主人周会议题 |
| **R-005** | OpenAPI 规范跟代码不同步 (改了 `apeireth-api/src/*.rs` 但 openapi.yaml 没改) | 🔴 高 | CI `swagger-cli validate` 阻塞 PR + 契约测试 (consumer-driven) |
| **R-006** | 13 项 Tauri 资产有 1 项理解错, Tauri 团队翻译时重写 | 🟡 中 | 资产表每项 8 列 (何时/怎么用/风险) + 同步会议 1 次/2 周 + 首次使用前 owner 必签 |
| **R-007** | Tauri 团队 fork 我们的 OpenAPI 规范, 自己改, 不回流 | 🟡 中 | OpenAPI 规范是 Apeireth 仓库 `docs/api/openapi.yaml`, Tauri 仓库只读不写; 改必须走 PR |
| **R-008** | 主人周会忘记把 Tauri 团队进度纳入议程 | 🟢 低 | Mavis 在主人周会议程模板加"Tauri 团队进展"项 (固定) |

---

## §9 不修改承诺 (11 项)

跟 `r20-product-finalize-2026-08-05.md` §7 + ADR-0011 一致:

1. 阶段 1+2+3 LOCKED 文档
2. v2 / v4 / v4.1 LOCKED
3. 阶段 4 主文档 LOCKED (`6ca80776`)
4. 阶段 5 施工文档 LOCKED (631 行)
5. v6 基础架构 (4 重守门 + 权限发放 + E 层修改路径)
6. R11 baseline 三值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063)
7. APEIRETH-CONVENTIONS / VERSIONING / GLOSSARY (顶层 3 文件)
8. START-CONSTRUCTION.md
9. workspace version 1.0.0 (Cargo.toml, semver 严格)
10. apeireth-legacy/ (物理归档, 仅增不删)
11. 现有 ADR 0001~0009

---

## §10 哲学 anchor (6 锚穿透)

按 APEIRETH-CONVENTIONS §9:

- [x] **S-1 (22:33) 6 anchor ASI 完整性** — Tauri 是 ASI 完整性的**桌面入口** (主人 2026-08-04 19:53 决策, user memory #8)
- [x] **S-2 (17:43) 6 anchor 实验室** — Tauri 团队用 Apeireth 41 crate 实验室产出, 13 项资产是已实战检验的"参考实现"
- [x] **O-5 (17:58) 6 anchor 12 急救** — 协同靠文档不靠人 (5 步 SOP + 文档透明原则)
- [x] **O-2 (19:33) 6 anchor 4 分类** — 双方边界 4 分类 (Apeireth 出 / Tauri 出 / 共享 / 各看自己)
- [x] **O-3 (23:44) 6 anchor 决策清单** — 5 步 SOP + 5 步实施 (可执行, 不是哲学)
- [x] **O-4 (00:56) 6 anchor 12 统一** — 跟 12 子规范统一 (Document-Meta + 锚穿透 + 不修改承诺)

---

## §11 关联文档

- **R20 路线图**: `docs/roadmap/r20-product-finalize-2026-08-05.md` (R20 战略 + 5 阶段)
- **13 项 Tauri 资产**: `docs/stage4/tauri-assets-from-spectrAI-2026-08-05.md` (Tauri 团队 lead 必读)
- **集成蓝图**: `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md` (R19+ 战略, §5.2 集成映射)
- **apeireth-sdk 缺失分析**: `docs/stage4/apeireth-sdk-gap-analysis-2026-08-05.md` (本周期, 3 SDK 5 步实施方案)
- **ADR 0010~0012**: `docs/adr/0010-mcp-from-spectrai-agentmcpserver.md` (apeireth-mcp 来源) + 0011 (不修改承诺) + 0012 (R20 范围)
- **HTTP API 契约**: `docs/api/openapi.yaml` (R20 阶段 3 新建)
- **WebSocket 协议**: `docs/api/websocket.md` (R20 阶段 3 新建)
- **R20 阶段 1 报告**: `reports/r20-stage1-complete-<date>.md` (阶段 1 完工后)
- **同步会议纪要**: `reports/tauri-collab-sync-<date>.md` (每 2 周 1 份)

---

_Tauri 团队对接 SOP (technical_writer)._
_5 步可执行 SOP + 10 项资产传递清单 + 8 项风险防御 + 6 锚穿透._
_两边都看, 但尤其给 Tauri 团队 lead 看 §2.3 共享边界 + §3 5 步 + §5 资产表._
