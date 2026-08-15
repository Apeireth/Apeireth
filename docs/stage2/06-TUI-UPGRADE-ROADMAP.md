# TUI 升级路线图 (2026-08-04 暂告段落)

> **目的**: 沉淀 R25 (round17-25) 改瘦 + 后续 4 步走的完整路线图.
> 主人 2026-08-04 23:55 拍板: TUI 暂告段落, 先去升级后端, 回来再推.
> **作者**: chuling (via mavis)
> **状态**: 暂存, 等后端升级完成后回来推 Step 2/3

---

## 🎯 战略锚定 (主人的核心洞察)

> "我们最后要做的前端应该是 Tauri, 但由于现在手头的 ai 团队没有适合干尤其是审美设计的, 所以 web 和桌面都搁置, 先做好 tui 来为桌面做准备."

**翻译**:
- TUI 不是临时品, 是 Tauri 桌面的"集成测试床"
- TUI 应该做"瘦客户端" (HTTP to apeireth-api), 不直接调后端 lib
- 缺审美设计时主人宁愿 TUI 也不上 web/桌面
- Tauri 来了直接抄 TUI 的 HTTP 集成模式

---

## ✅ Step 1 (今天完成, 2026-08-04 23:00-23:50)

### 1.1 — 创建 HTTP LLM 客户端模块
**文件**: `crates/apeireth-tui/src/http_llm.rs` (493 行, Sub-agent 交付)

**2 个公开 API**:
```rust
pub fn call_llm_http_sync(input: &str, system: &str) -> Result<HttpLlmReply, String>;
pub async fn call_llm_http_stream(
    input: &str,
    system: &str,
    sender: &std::sync::mpsc::Sender<String>,
) -> Result<String, String>;
```

**4 个 httpmock 单元测试全过**:
- `http_stream_pushes_chunks_to_sender_and_returns_full_text`
- `http_sync_parses_non_stream_response`
- `http_returns_err_on_500`
- `http_returns_err_on_connection_refused`

**核心约束**: 不 import `apeireth_api::*` (瘦客户端硬约束达成)

### 1.2 — 改 backend.rs 整合
- 砍 `call_llm_sync` 旧 lib 实现 (40 行) → 调 `http_llm::call_llm_http_sync`
- 砍 `call_llm_stream_sync` 旧 lib 实现 (45 行) → 调 `http_llm::call_llm_http_stream`
- 砍 `MINIMAXI_BASE_URL` / `MINIMAXI_MODEL` / `APIKEY_PATH` 3 个常量
- 砍 `apeireth_api::*` 5 个 import (OpenAiCompatibleConfig / Provider / LlmRequest / ChatMessage / ChatRole)
- 保留 `LlmError` (process_stream_to_reply 仍用它签名)

### 1.3 — 测试 + 端到端
- `cargo build -p apeireth-tui`: 0 error, 0.34s
- `cargo test -p apeireth-tui`: **86 passed / 1 pre-existing 失败** (perception DB 污染, 与本 fix 无关)
- **端到端冒烟**: 起 `apeireth-api` server, 模拟 TUI 调 `/v1/chat/completions` → 返真实 LLM reply "Hi. Ready when you are — tell me what you're working on" (266 tokens)

### 1.4 — 意外惊喜 🎁
- **minimaxi M3 model 实际支持 `<think>` CoT 标签** (之前以为不支持)
- LLM reply 里出现 `<think>The user says "hi"...</think>` 块
- TUI 的 `split_think` + `Ctrl+O` 展开应该能正常显示思考过程了
- 之前以为 think 功能挂了, 实际是 LLM 调用路径不对, 走 HTTP 端点就活过来了

### 1.5 — 顺手修的 bug
- 中文输入 panic (UTF-8 char boundary 错误) — `commit a7b6e52d`
- history 截断潜在同样 panic — 同 commit 一起修
- system prompt 改基地主管 + 用户母语 (W2.7 砍假装留下的空壳) — `commit f7e2e435`
- 6 个新回归测试守护这些修复

### 1.6 — 文档更新
- `apeireth-debug/02-HANDOVER.md` — 镜像 doc 同步 R17 状态
- `FINISH-CONSTRUCTION.md` — Manual-Rev-H → Rev-J-R17, 41 crates / 2271 tests / 1.0.0 可见化

### 1.7 — Commit 链
- `a7b6e52d` TUI 中文输入 panic 修
- `f7e2e435` system prompt 重定
- `0049b511` TUI 改瘦 Step 1.5 (HTTP 客户端 + 整合)

---

## 📋 Step 2 (后续, 后端升级时一起做) — apeireth-api 加 JSON 端点

**当前缺**: 6 大类强能力 (工具/器官/ASI/Memory/Sovereignty/Agent) 都没 JSON HTTP 端点

### 2.1 — Tools API
```
GET  /v1/tools/list           → [{name, kind, axes, schema}, ...]  (5 tools: web_search, file_ops, git_ops, code_exec, ?)
POST /v1/tools/invoke        {name, args} → {result, error}
```
**底层**: 复用 `apeireth_tool_registry::ToolRegistry` + `apeireth_tools::register_all`
**前端意义**: 未来前端能"工具选择器"显示可调用工具 + 用户点击调

### 2.2 — Memory API
```
GET  /v1/memory/episodes?session=X&limit=N    → [{ts, role, content}, ...]
POST /v1/memory/append        {role, content, ts} → {ok, episode_id}
GET  /v1/memory/identity     → IdentityCard JSON
POST /v1/memory/identity/update {key, value}
```
**底层**: `apeireth_memory::SqliteMemoryStore` + 6 stream
**前端意义**: history 页直接 HTTP 拉, 不直连 SQLite

### 2.3 — Organs API
```
GET  /v1/organs              → [{name, health, status, ...}, ...]  (9 organs)
GET  /v1/organs/{name}      → 单个 organ 详情
POST /v1/organs/{name}/invoke {action, args} → 触发器官 (慎, 待 supervisor)
```
**底层**: `apeireth_cognition::run_cycle` + 9 organ snapshot
**前端意义**: 9 器官拟人化显示, 状态心跳环

### 2.4 — ASI API
```
GET  /v1/asi/score?dim=X     → V0.5 score (单维度)
GET  /v1/asi/all             → V0.5 24 维 + V1136 9 子测度 (全量)
POST /v1/asi/calibrate       {dry_run, every, scope} → ML 校准
```
**底层**: `apeireth_asi::DimensionRegistry` + CalibrationLoop
**前端意义**: status bar / dashboard 显示 ASI 实时分数

### 2.5 — Sovereignty API
```
GET  /v1/sovereignty/status  → 5-Self 状态 (主人在场/时间锁/...等)
POST /v1/sovereignty/attack  → 模拟攻击 (test 5 大机制)
POST /v1/sovereignty/rearm   → 解除自禁用
```
**底层**: `apeireth_sovereignty::self_disable::SelfDisableGuard`
**前端意义**: 5-Self 控制台

### 2.6 — Agent API
```
GET  /v1/agent/aliases       → [{alias, target, hot_reload}, ...]
POST /v1/agent/alias         {alias, target} → 注册
GET  /v1/agent/cache         → LRU 命中率
```
**底层**: `apeireth_agent::Agent` (alias + LRU + notify)
**前端意义**: 5 trait 工具系统的 agent 包装层

**预计工作量**: 2-3 天 (6 个端点, 复用现有 lib, 每个 ~150 行)

---

## 📋 Step 3 (后续, Step 2 完成后) — TUI 消费这些端点

### 3.1 — 改 TUI 状态来源
- TUI 当前有些状态从本地 lib 读 (R19 token, cycle count, snapshot_all_organs 等)
- 改: 通过 HTTP 拉 `apeireth-api` 端点
- 优势: TUI 跟未来 Tauri 共用 1 套 API 拿状态

### 3.2 — 加新 UI 元素
- history 页: 调 `/v1/memory/episodes` (现在直连 SQLite)
- bridge 页: 9 器官状态调 `/v1/organs` (现在直连 cognition/perception/...)
- 状态栏: ASI 分数调 `/v1/asi/all` (现在直连 registry)
- 新加 tools 按钮: 调 `/v1/tools/invoke`

### 3.3 — Tools 流 (R26 战役 0?)
- LLM 回复里出现 `<tool_call>{"name": "web_search", "args": {...}}</tool_call>` 格式
- TUI 检测 tool_call → 调 `/v1/tools/invoke` → 把 result 塞回对话
- 这是 OpenAI function calling 协议的标准做法, minimaxi M3 应该支持

**预计工作量**: 2 天

---

## 📋 Step 4 (终极, 等设计团队到位) — Tauri 集成

**当前架构 (Step 3 后)**:
```
[TUI 进程]                    [apeireth-api 进程]      [LLM]
  http_llm (reqwest)            ←→  4 协议端点              ←→  minimaxi
  http_tools (待加)              ←→  工具端点 (待加)         ←→  本地文件
  http_memory (待加)             ←→  memory 端点 (待加)
  http_organs (待加)             ←→  organs 端点 (待加)
```

**Tauri 集成 (Step 4)**:
```
[Tauri WebView 进程]            [apeireth-api 进程]
  React/Vue UI                  ←→  同样的 HTTP 端点          ←→  LLM
  fetch/axios 替代 reqwest
  React state 替代 ratatui

  1 套 API (apeireth-api 端点) ← 2 个 consumer (TUI 测试 + Tauri 生产)
```

**Tauri 团队的工作**:
- 抄 TUI 的 HTTP 调用模式 (URL + body + SSE 解析)
- 把 ratatui 5 nav 换成 React/Vue 组件
- 加设计 (Tauri 这边美, 主人不管)
- 不需要改后端任何东西 (1 套 API 共享)

**预计工作量**: 等设计团队到位后 1-2 周

---

## 🗂️ Step 0.5 — TUI 暂告段落期间维护清单

> 主人 2026-08-04 23:55 拍板: TUI 暂告段落, 先去升级后端. 回来时检查这些项不退化.

| 检查项 | 命令 | 期望 |
|--------|------|------|
| TUI 编译干净 | `cargo build -p apeireth-tui` | 0 error, 0.34s |
| TUI 测试不退化 | `cargo test -p apeireth-tui` | ≥ 86 passed (perception 测试可失败) |
| TUI splash 是 v1.0.0 | 跑 binary 看 eprintln | "apeireth-tui v1.0.0 (R17 战役 0-4 收官)" |
| TUI 走 HTTP 端点 | env `APEIRETH_API_URL=http://localhost:8080` 起 server 后跑 TUI | 输 hi 收到 LLM reply |
| TUI think display 活 | 输 hi 后 Ctrl+O 展开 thinking | 看到 M3 的 CoT 块 |
| 中文输入不 panic | 输"晚上好" | 不崩, 字符显示正确 |
| history 截断不 panic | 输 50 字中文 + Enter | history 显示不崩 |

---

## 📁 关键路径速查

```
主仓:    .openclaw\workspace\promethean\Apeireth-rust\
TUI 客户端: target\debug\apeireth-tui.exe (11.1 MB)
HTTP 客户端: crates\apeireth-tui\src\http_llm.rs (493 行)
TUI backend: crates\apeireth-tui\src\backend.rs (call_llm_* 改 HTTP)
Server 入口: cargo run -p apeireth-api --example serve
Server 代码: crates\apeireth-api\src\server.rs:75 (build_router)
doc: apeireth-debug/02-HANDOVER.md (R17 状态)
doc: FINISH-CONSTRUCTION.md (Manual-Rev-J-R17)
doc: reports\r17-战役4-5-1.0-release-2026-08-04.md (1.0 release 收官)
doc: reports\r17-战役0-4-*.md (R17 战役报告 17 份)
本路线图: docs\v2-strategy\04-TUI-UPGRADE-ROADMAP.md (本文件)
```

---

## 🎯 关键 commit 索引 (改瘦相关)

| commit | 描述 |
|--------|------|
| `a7b6e52d` | TUI 中文输入 panic 修 (UTF-8 char boundary) |
| `f7e2e435` | system prompt 重定 (基地主管 + 用户母语) |
| `0049b511` | **TUI 改瘦 Step 1.5** (HTTP 客户端 + 整合) |

---

## 🧠 为什么 TUI 是"集成测试床"而不是"过渡品"

| 维度 | 过渡品视角 | 集成测试床视角 (主人战略) |
|------|------------|----------------------|
| Tauri 来了 | TUI 删掉 | TUI 保留, 作为"参考实现" |
| API 设计 | 临时凑合 | 1 套 API 2 consumer (TUI + Tauri) |
| 测试 | 手测 | 端到端实测 (每次 TUI 跑都验证 API) |
| 长期 | 抛弃 | 自动化 / CLI / 调试工具 |

**TUI 的真实价值**:
1. 验证后端 HTTP API 是否好用
2. 验证用户流 (5 nav + chat + tools + organs)
3. Tauri 团队的"参考实现" (抄代码)
4. 永久 CLI 工具 (curl + 脚本调 TUI 同款 API)
5. 调试后端 (直连 apeireth-api 调问题)

---

**作者**: chuling (via mavis)
**最后更新**: 2026-08-05 00:00 (Asia/Shanghai)
**状态**: ⏸️ 暂存, 等主人后端升级完成后回来推 Step 2/3
