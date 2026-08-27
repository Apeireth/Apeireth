# Apeireth v1.0 → v2.0 迁移指南（2026-08-27）

> **现状 (2026-08-27)**：v1.0 (`993e9107`) 与 v2.0.0-alpha.1 (`bad99fd4` / 远端 `9080cc93`) 并列存在。v1 走 master 线 (`archive/v1.0-master`) + tag `v1.0.0` + `legacy/donor/apeireth-companion` 源码; v2 走 main 线（默认）+ `v2.0.0-alpha.1` tag + 13-crate 工程重构工作区。本指南是 **v1 companion_serve 用户切到 v2 gateway + CLI** 的最小路径。
> 0 装诚实标注：v2.0.0-rc.1 之前 v2 gateway 的 LLM harness 是 0 装（per `v2.0.0-rc-roadmap.md` RC-5/RC-6）；本指南**假设运行时配了真 API key**（v2 走 EnvCredentialResolver 或 KeyringCredentialResolver）。

```
[Document-Meta]
Document:        docs/04-internal/migration-v1-to-v2.md
Version:         Manual-Rev-1.0
Last-Modified:   2026-08-27
Status:          🟢 活跃 (v1 → v2 迁移指南)
```

> 给谁看：v1 companion_serve (legacy/donor) 的部署者 / 集成者 / AI 协作者。
> 读法：§1 哲学不变 / §2 形态变化（v1 vs v2 对照表）/ §3 三种迁移路径 / §4 配置文件 / §5 API endpoint 映射 / §6 凭证迁移 / §7 失败回滚 / §8 FAQ。

---

## 1. 设计 / 哲学 / 规范 0 变化（重要前提）

**v2 工程重构 = 工程形态演进，**不是设计升级**。具体 0 改项**（来源 [ROADMAP.md](../../ROADMAP.md) §5 + [philosophy.md](../01-architecture/philosophy.md)）：

- **8 哲学锚**（S-1 北极星 / S-2 实事求是 / S-3 质量工程化 / O-1 安全优先 / O-2 走在前人 / O-3 干到底 / O-4 接手 / O-5 不假装）0 改
- **13 键 verdict cache 语义** 0 改（数据 LOCKED；v2 角色 = 哲学标准/判别词汇表，**不**接 runtime 强制，per 2026-08-27 5 维分析拍板降级）
- **三洋葱**（Principle / Permission / HumanAuthority） 0 改
- **L0 HA 物理隔离** + **Self-Disable 判定** 0 改（3 项不可变脊柱，v2 严守）
- **0 装 PASS** 0 改（v2 维持 v1 同等严守）

**v2 改的（工程形态）**：
- 86-crate → 13-crate workspace 收敛
- v1 走单 companion_serve 长连接 + 9 organ 调度
- v2 走 CLI / HTTP gateway 入口 + PluginManager 统一注册
- v1 self-introspection 评审 → v2 external hook 闸（Rust 字符串 + 权限策略）

**对 v1 用户的影响**：调用的 HTTP/WS endpoint 变，部署进程变，配置文件位置变；语义/数据/历史**不**变。

---

## 2. v1 vs v2 形态对照表

| 维度 | v1 (`v1.0.0` / `archive/v1.0-master`) | v2 (`v2.0.0-alpha.1` / `main`) |
|---|---|---|
| **入口进程** | `companion_serve`（长连接 HTTP/SSE，OpenAI 兼容 + 专属端点） | `apeireth-gateway`（短连接 HTTP `:8080`） + `apeireth-cli`（session/chat）|
| **HTTP 端口** | `:8090` | `:8080` |
| **API 风格** | OpenAI Chat Completions 兼容 + `/v1/apeireth/*` 专属（approval-requests / events / grant / panel）| OpenAI Chat Completions 兼容（`/v1/chat/completions`），**没有**专属 `/v1/apeireth/*`（approval 走 governance 内部流）|
| **器官（9 organ + companion）** | 全部在 v1 companion crate 跑 | **v2 工作区没有**（排期 P6 = 场景 D 例 3 multi-agent 互审基础）|
| **凭证** | 读 `apikey-ultra.txt` 文件 | `EnvCredentialResolver`（env var）/ `KeyringCredentialResolver`（OS keyring）/ `EncryptedFileBackend`（rc 路线） |
| **治理** | ToolBridge 8 闸（含 LLM 评审 self-introspection）| external hook 闸（3 hook：Permission / 凭据泄漏 / 注入检测），0 模型参与 |
| **记忆** | 6 历史流 + 1 个合并 DB + 三层 progressive disclosure (Wiki/KG/Association) | trait 边界 + SQLite impl (alpha 是 0 装) + Experience trait (alpha 是 0 装) |
| **持久化** | 单 SQLite 文件（默认路径 `data/companion.db`）| `SqliteConnectionPool` + migrations + 未来可换 backend（trait 抽象 alpha 已就位） |
| **多 agent** | v1 apeireth-team-lead 14 调度工具 | v2 `apeireth-orchestration` trait (alpha 0 装，rc 接 LLM harness) |
| **前导** | 桌宠 (companion-desktop) Svelte 5 + Tauri 2 | 同上 (companion-desktop 仍是同一前端, 但 0.5.0 之后才接 v2 gateway) |
| **CLI** | v1 companion binary + 多 sub-command | `apeireth` binary + `session / chat / gateway serve` 三个 sub-command |
| **TUI** | 完整 TUI (TUI-9 organ dashboard) | v2 alpha 没 TUI (排期 P6 后) |

---

## 3. 三种迁移路径

### 路径 A：最小动作 — 切 v2 branch（推荐先做这个）

**适用**：想看 v2 是什么、但暂时不想动生产

```bash
# 在 v1 仓库目录
cd /path/to/Apeireth-rust

# 1. 备份 v1 当前状态
git tag v1-pre-migration-snapshot
git branch backup-v1-stable

# 2. 切到 v2 (main 是默认分支, 旧的 v1 master 已归档)
git fetch origin
git checkout main        # 切到 v2 (默认分支, tag v2.0.0-alpha.1)
git checkout v2.0.0-alpha.1  # 精确切到 alpha.1 (d6910cf7/9080cc93)

# 3. 跑 v2 build 确认环境
cargo check --workspace --all-targets --locked

# 4. v1 代码还在 legacy/donor/apeireth-companion, 可读可参考, 不会进 build
ls legacy/donor/apeireth-companion/
```

**回滚**：
```bash
git checkout v1.0.0   # 或 git checkout backup-v1-stable
```

### 路径 B：v1 + v2 并行（生产评估用）

**适用**：想 v1 跑生产的同时 v2 在另一个端口试

v1 和 v2 跑同一台机没问题，端口不同即可：
- v1 companion_serve :8090（保留原配置）
- v2 gateway :8080（新增）

**配置上完全独立**（v1 配 `apikey-ultra.txt`，v2 配 `APEIRETH_MINIMAX_API_KEY` 或 keyring entry）。两边可同时跑。

### 路径 C：生产 v1 → v2 完全切换（不推荐 rc 之前做）

**适用**：已经全面评估 v2 满足生产需求，**且 v2.0.0-rc.1 已发布**（真实 backend 接通）

**当前（2026-08-27 v2.0.0-alpha.1）不建议做路径 C**：
- v2 0 装 trait 多（MemoryBackend / Experience / Orchestrator / Council / Perception / SubSupervisor），实际 backend 走 env var 走 SQLite
- LLM harness 0 装（v2.0.0-rc 才接）
- 长程任务依赖 companion 器官 (W1/W2/W3/E4/F4/F1/F6/E7) 这些器官**未移植**到 v2（排期 P6 之后）

**完整切 v2 的最低先决条件**：
1. v2.0.0-rc 发布（真实 backend + LLM harness + 至少 1 器官移植）
2. 业务对 v1 器官依赖的评估（如果你的生产只用 OpenAI Chat 兼容接口，rc 就够；如果依赖 9 organ / 5 原型 / 长程任务，**等 v2.0.0**）
3. 数据迁移脚本（v1 `data/companion.db` → v2 `data/memory.db`，schema 兼容，但字段需映射——见 §6）

---

## 4. 配置文件迁移

### 4.1 v1 配置位置

```
Apeireth-rust/legacy/donor/apeireth-companion/
├── config/
│   ├── default.toml          # 主人偏好/全局配置
│   ├── features.toml         # feature gates
│   └── capability-manifest.toml
data/
├── companion.db             # 主 SQLite (episodes + 6 streams)
├── audit.db                  # 审计链
└── apikey-ultra.txt          # 凭证 (gitignored, 用户本地)
```

### 4.2 v2 配置位置

```
Apeireth-rust/                              # 仓库根 (Cargo workspace)
├── Cargo.toml                                # workspace 定义 (13 crate)
├── rust-toolchain.toml                       # 1.97.1
├── .gitignore                                # tools/ 已锚到根 (不误伤 crates/capabilities/tools)
└── data/                                     # v2 默认数据目录 (按用户自建)
    └── memory.db                              # 单一 SQLite (episodes + 6 streams + identity + governance)

# 凭证 (任一即可, 按部署选择)
~/.config/apeireth/credentials.json           # EncryptedFileBackend (rc 路线)
# 或 OS keyring (Linux Secret Service / macOS Keychain / Windows Credential Manager)
# 或环境变量:
export APEIRETH_MINIMAX_API_KEY=sk-...
export APEIRETH_ANTHROPIC_KEY=sk-...
export OPENAI_API_KEY=sk-...
```

### 4.3 配置值映射

| v1 (`apikey-ultra.txt`) | v2 env | v2 keyring service |
|---|---|---|
| minimax API key | `APEIRETH_MINIMAX_API_KEY` | `provider.minimax.api_key` |
| anthropic API key | `APEIRETH_ANTHROPIC_KEY` | `provider.anthropic.api_key` |
| OpenAI API key | `OPENAI_API_KEY` | `provider.openai-compatible.api_key` |
| 主 token (master) | 不在 v2 runtime 自动读取 (需 keyring + 显式 GatedCredentialsStore) | `master` (with DenyAllGate 兜底) |

### 4.4 数据迁移

v1 companion.db 6 张流表（thought_stream / proposal_stream / action_stream / relation_stream / evolution_stream / reflection_stream）+ episodes 表 + identity_cards + audit_log → v2 同 schema（`crates/engine/memory/src/migrations.rs` 现有 migrations 兼容 donor v1 schema）。

**v1 → v2 数据迁移步骤**：

```bash
# 1. 备份 v1 db
cp data/companion.db data/companion.v1.backup.db

# 2. 启动 v2 (rc 阶段) 让它跑 migrations
apeireth gateway serve --port 8080

# 3. v2 启动会自动跑 migrations, 但 v1 schema 已存在, 所以 v2 检测到 v1 schema 直接 reuse
# (sqlite WAL 检测, 不重写, 不丢数据)

# 4. 验证: v2 CLI 看 episode 数
apeireth session list

# 5. 备份 v2 db
cp data/memory.db data/memory.v1-migrated.db
```

**风险**：
- v1 schema 与 v2 schema 在 v2 alpha + rc 阶段**应当兼容**（per migrations.rs 注释："donor v1 schema kept for on-disk compatibility"）
- 如果 v2.0 引入新表 / 新列, 跑 migrations 即可（`SchemaMigration::run_migrations` 是幂等的）

---

## 5. API endpoint 映射

### 5.1 v1 endpoint 清单

```
# 通用 (v1 OpenAI 兼容)
GET  /v1/models
POST /v1/chat/completions

# v1 专属 (companion)
GET  /v1/apeireth/approval-requests
POST /v1/apeireth/grant
GET  /v1/apeireth/events
POST /v1/apeireth/test-event
GET  /health
GET  /panel
GET  /panel/<asset>

# 工具协议 (v1 only)
text <<<[TOOL_REQUEST]>>>...<<<[END_TOOL_REQUEST]>>>  # in message content
```

### 5.2 v2 endpoint 清单

```
# 通用 (OpenAI 兼容) — 与 v1 兼容
GET  /health
POST /v1/chat
POST /v1/chat/completions

# 无 /v1/apeireth/* 专属 (v2 governance 内部流转, 不暴露 HTTP)
# 无 /panel (rc 后端 GUI 走 frontend companion-desktop, 独立 API)
# 无 <<<[TOOL_REQUEST]>>> marker (v2 走 provider 原生 tool_calls schema)
```

### 5.3 业务调用迁移示例

**v1**（v1 companion_serve）：
```bash
# 凭证 (本地文件)
echo "sk-..." > data/apikey-ultra.txt

# 启动
cargo run --bin companion_serve -- --port 8090

# 调用 (v1 专属端点)
curl -X POST http://localhost:8090/v1/apeireth/approval-requests \
  -H "Authorization: Bearer any-token" \
  -d '{"tool": "FileOperator", "hours": 24}'
```

**v2**：
```bash
# 凭证 (env var / keyring)
export APEIRETH_MINIMAX_API_KEY=sk-...

# 启动
cargo run -p apeireth-cli -- gateway serve --port 8080

# 调用 (OpenAI 兼容, 无专属端点)
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "MiniMax-M3",
    "messages": [{"role": "user", "content": "hello"}]
  }'
```

**v1 → v2 主要差异**：
1. 端口 `:8090` → `:8080`
2. 路径 `/v1/apeireth/*` → 删除（governance 内部流）；业务逻辑通过 `/v1/chat` 标准 OpenAI 兼容端点
3. 工具调用从 marker 解析 → provider 原生 `tool_calls` schema
4. 工具默认关闭（`shell`/`fetch` opt-in），v1 工具权限包 → v2 governance 策略
5. **v1 长程任务（W1/W2/W3/E4/F4/F1/F6/E7 器官）** → v2 没移植（排期 v2.0.0-rc 之后 P6）；如需这些能力继续用 v1

---

## 6. 凭证迁移

### 6.1 v1 凭证流转

```
1. 用户把 API key 写入 data/apikey-ultra.txt
2. companion_serve 启动时读 .env (APKIEY_ULTRA_PATH) → 加载
3. provider 调 LLM 时把 string 直接用于 Authorization 头
4. key 在 Debug print 风险
```

### 6.2 v2 凭证流转

```
1. 三种方式选一:
   a) EnvCredentialResolver: 启动时读 env var, map logical name -> env var
   b) KeyringCredentialResolver (rc): 启动时调 OS keyring 取加密 secret
   c) EncryptedFileBackend (rc): 启动时解密 .apeireth-credentials.json
2. provider 调 LLM 时:
   - 通过 ctx.credentials.resolve("provider.minimax.api_key")
   - 拿到 Secret<T> 类型 (Debug 返 "Secret(<redacted>)")
3. 凭证不落 struct, 不进 log, 不进 panic 消息
```

### 6.3 凭证迁移步骤

**从 v1 文件到 v2 env var**：
```bash
# 1. 读 v1 key
cat data/apikey-ultra.txt
# 输出: sk-... (一行)

# 2. 设 v2 env var (按 provider)
export APEIRETH_MINIMAX_API_KEY=sk-...
# 或 (按 OpenAI 兼容)
export OPENAI_API_KEY=sk-...

# 3. 启动 v2 (凭据通过 EnvCredentialResolver 解析)
cargo run -p apeireth-cli -- gateway serve --port 8080
```

**从 v1 文件到 v2 keyring**（推荐，安全）：
```bash
# Linux (Secret Service)
secret-tool store --label="apeireth/minimax" provider.minimax.api_key sk-...

# macOS
security add-generic-password -a apeireth -s provider.minimax.api_key -w "sk-..."

# Windows
# 用 Credential Manager 图形界面或 PowerShell:
cmdkey /generic:apeireth\provider.minimax.api_key /user:apeireth /pass:sk-...
```

**生产部署：v2 推荐 keyring**（0 装文件、无明文落盘、OS 级访问控制）。

---

## 7. 失败回滚

**v2 gateway 启动失败**：
- v2 CLI 二进制编译失败 → `cargo build -p apeireth-cli` 单独 build，定位错误
- v2 gateway 跑起来但 API 不响应 → 看 `~/.config/apeireth/log` 或 stderr (Logging 默认 stderr)
- 工具调用拒绝 → governance hook 返 Deny，看 `Decision::reason`

**回滚到 v1**（任何时候）：
```bash
git checkout v1.0.0              # 切到 v1 release tag
cargo build                      # 重新编译 v1
cargo run --bin companion_serve  # v1 入口
```

**v2 已知不能跑的任务**（v1 跑得了 v2 暂时跑不了）：
- 多器官协同任务（依赖 v1 9 organ + companion）
- 长程世界模型推演（W1/W2/W3）
- 情感记忆持续化（F1）
- 价值内化与裁决（F6）
- 主动好奇驱动探索（E4）
- 持续会话生命状态机（active/archived/closed 转换）
- 这些功能**v2.0.0 后**才支持

**过渡方案**：v1 跑 v1 配套长程任务，v2 跑 v2 短平快（标准 OpenAI 兼容 + L3 hooks）。

---

## 8. FAQ

**Q: v2 alpha 能跑我的 v1 companion_serve 客户端代码吗？**
A: 调 OpenAI Chat Completions 兼容的代码能（curl / Python openai / LangChain / LiteLLM）。调 `/v1/apeireth/*` 专属端点的代码**不能**——v2 没这端点。如有依赖，需改用治理闸（runtime 内自动转 Deny/RequireApproval 决策，或发 approval 请求给前端）。

**Q: 我用了 v1 的 TUI，能继续用吗？**
A: v2 alpha 没 TUI。`apeireth-tui` 在 `legacy/donor/` 还在但 v2 workspace exclude。rc 之后（或 v2.0.0）有计划重做 TUI 走 v2 PluginManager。当前建议：CLI + companion-desktop GUI 都够用。

**Q: v1 的 9 organ 真的不能跑吗？**
A: v1 代码在 `legacy/donor/apeireth-companion/` 完整保留（86-crate v1 workspace exclude 后保留了 source-of-truth 价值）。你要在 v1 branch 跑 v1 organ 完全可以——v1 master line (`archive/v1.0-master`) build 不依赖 v2 工作区。v2 与 v1 是并列分支，按场景选。

**Q: v2 0 装 trait 啥意思？什么时候补？**
A: v2 alpha 1 阶段我**只**画 trait 边界（`apeireth-orchestration` 等新 crate 的 trait 完整签名 + Noop/Allow 0 装实现 + 测试），**不**接真 LLM。v2.0.0-rc 阶段（预计 2026-12 月）按 `v2.0.0-rc-roadmap.md` 的 10 个 RC 任务接真 backend。**rc 后 0 装 trait 不再是 0 装**——trait impl 真实化。

**Q: v2 数据能跟 v1 互通吗？**
A: alpha + rc 阶段 schema 兼容（同 6 流表 + episodes + identity_cards），v1 db 可直接被 v2 打开。**v2.0.0+ 引入新表/新列** 时 v1 老字段会保留（append-only 设计）。完整数据互操作要等 v2.0.0。

**Q: v2 凭证泄露风险比 v1 高吗？**
A: **低**。v1 是 `String` 字段，Debug print 泄露。v2 是 `Secret<T>` 类型，Debug 返 `<redacted>`；secret 不进 struct、不进 log、不进 panic 消息；走 `CredentialResolver` trait 抽象不直接 import env 或文件。

**Q: v2 跑生产最少依赖？**
A: 运行时只依赖 `cargo run -p apeireth-cli -- gateway serve --port 8080` + 1 个 API key 环境变量。SQLite 文件自动创建（path 走 `SqliteConnectionPool::validate`）。无 Redis / PostgreSQL / MongoDB / 外部 LLM proxy 依赖。

**Q: v2 性能 vs v1？**
A: 6 张流表 indexes + 单 writer + reader pool + WAL 模式 = 与 v1 相当。governance hook 加 ~μs/turn（Rust 字符串匹配）。v1 走 self-introspection LLM 评审时 = 每次动作 + 1 LLM call = 几秒延迟，v2 0 装 trait 不调 LLM = 更快。**v2 governance 默认 AllowAll 时 = v1 性能**。接 governance hooks 后 = 接近 v1（μs 级）。

---

## 9. 决策流程（不确定时该选什么）

```
                     ┌─ 仅 OpenAI Chat 兼容 + 短任务
                     │   ↓
                     │   v2 gateway 立即可上 (alpha.1 已够)
                     │
   v1 业务需求 ──────┼─ 多器官协同 / 长程任务
                     │   ↓
                     │   继续 v1 (切 archive/v1.0-master, v1.0.0 tag 还在)
                     │
                     ├─ v1 + v2 并行评估
                     │   ↓
                     │   不同端口 (8090 v1 / 8080 v2), 同一份数据按 db 切
                     │
                     └─ 完整 v2 生产切换
                         ↓
                         等 v2.0.0-rc (2026-12) 真实 backend + 至少 1 器官移植完成
```

---

## 10. 一句话总结

**v1 = 完整的（86 crate, 23k 测试, 器官齐全, 9 organ 完整跑）**。  
**v2 alpha.1 = 干净的工程形态（15 crate, 1.4k 测试, 短平快, 主链 + governance）**。  
**v2.0.0 = 完整 v1 等价 + 干净的工程形态**（这是 rc 路线图的事）。

现在迁移 = 看你是要"完整的 v1"还是"干净的 v2"。两条路都通，文档同步给出怎么走。

---

_本指南 v1 首发 (2026-08-27)：v2.0.0-alpha.1 阶段 (`bad99fd4` / 远端 `9080cc93`) 已发布, ROADMAP §4 P1-P6 + P-arch 全部 trait 边界 + 0 装占位完成. 迁移路径假设生产对 v1 器官依赖中等; 如果你的生产重度依赖 9 organ + 长程任务, 继续 v1 (`archive/v1.0-master`) 是正确选择, v2 等到 v2.0.0-rc.1 + 至少 1 器官移植完成再切. 设计/哲学/规范 0 改, 变更全在工程形态. v2.0.0-rc 预计 2026-12 月, v2.0.0 预计 2027-02-04 月._
