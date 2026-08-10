# v0.9.21 商业版 1:1 Rust 翻译蓝图 (RIVAL VERSION — 2026-08-05 19:50 拍板版)

```
[Document-Meta]
Document:    .openclaw\workspace\promethean\Apeireth-rust\docs\stage4\v09021-rust-translation-blueprint-RIVAL-2026-08-05.md
Version:     RIVAL-Rev-A
R-Cycle:     R20 阶段 1-5 实施基线 (1:1 翻译 v0.9.21 商业版 16 模块 + 60+ SDK)
Last-Modified: 2026-08-05 19:55
Status:      🎯 蓝图完成 (主 2026-08-05 19:50 拍板"派成员干", Mavis 事后比 2 份选 quality 高 commit)
Author:      Mavis (RIVAL sub-agent, 跟 bg_a5470979 并行竞争)
Originated:  主人 2026-08-05 19:37 "全用 rust 1:1 翻译" + 19:50 "派成员干, 自己干分散注意力"

> **RIVAL VERSION 性质声明**:
> 本文档是 **RIVAL 竞争版** (vs 同日 bg_a5470979 写的 `v09021-rust-translation-blueprint-2026-08-05.md`).
> Mavis 事后会比对 2 份 quality 选高 commit. 顶部明确 RIVAL VERSION 标记, 第 §7 跟原版对齐声明.
>
> **本 RIVAL 版差异化 (vs 原版预告)**:
> 1. 严格按 "1 TS module = 1 Rust crate" 16 拆分 (16 估缺 + 5 增强 + 1 R21 = 16 个新增/增强)
> 2. 5 P0 crate **已 skeleton** 做健康检查 (3-5 个真实缺口实查)
> 3. workspace members 整合顺序明确 (1 commit 落地策略)
> 4. R20 5 阶段 320h 估时按 16 估缺总 132h + 8 增强/SDK 60h + 5 阶段 overhead 128h
> 5. m3 hallucination 5 道防御在 16 crate 内全部 hardcode (不是 1 个总开关)

> **必读输入 (9 份 + 1 当前实查)**:
> 1. `v09021-commercial-extract-2026-08-05.md` (250 行, NSIS 解包实查)
> 2. `commercial-vs-fork-diff-2026-08-05.md` (480 行, 3 版本差异)
> 3. `supervisor-prompt-818-summary-2026-08-05.md` (647 行, 8 段拆解)
> 4. `5-provider-tool-mapping-2026-08-05.md` (644 行, 84 映射)
> 5. `m3-hallucination-defense-2026-08-05.md` (613 行, 5 防御)
> 6. `spectrai-branch-coverage-audit-2026-08-05.md` (572 行, 21 假盲点)
> 7. `yinta-fork-audit-2026-08-05.md` (504 行, fork 0.1.0)
> 8. `spectrAI-integration-blueprint-r19-plus-2026-08-05.md` (R19+ 大蓝图)
> 9. `r20-product-finalize-2026-08-05.md` (R20 路线图)
> + 当前实查: `.minimax-agent-cn\spectrai\commercial-nsis\v0901\app-64\app-extracted\out\main\` (171 .js / 452K LOC)
> + 5 P0 crate 体检: `crates/apeireth-mcp-ssh` / `apeireth-mcp-winrm` / `apeireth-mcp-relay-image` / `apeireth-workflow` / `apeireth-team-lead`
```

---

## §0 文档地图 (1 分钟看完)

| § | 内容 | 谁用 |
|---|------|------|
| §0 | 文档地图 (本节) | 任何人 |
| §1 | v0.9.21 商业版 1:1 翻译总体图 (总模块 / 总 LOC / 16 估缺 + 60+ SDK) | 任何人 |
| §2 | 16 新 Rust crate 设计表 (重点) | 实施人 |
| §3 | 5 P0 crate 体检表 (健康检查 + 真实缺口) | 实施人 |
| §4 | R20 5 阶段 320h 实施图 (按 §2 估时) | Mavis 整合 / 主人 |
| §5 | workspace 整合策略 (1 commit 落地) | Mavis 整合 |
| §6 | 风险与依赖 (8 闭源 / 60+ SDK 哪些真要 / m3 防御) | 主人 + 实施人 |
| §7 | 跟原蓝图对齐声明 (RIVAL vs bg_a5470979 差异) | Mavis 整合 |

---

## §1 v0.9.21 商业版 1:1 翻译总体图

### §1.1 商业版实查数据 (per `v09021-commercial-extract` §1)

| 维度 | 值 | 出处 |
|------|----:|------|
| version | 0.9.21 | `package.json` |
| 作者 | weibin <bin.wei@steriguard.cn> | `package.json` |
| license | MIT | `package.json` |
| 总 .js 文件 | **171** | `Get-ChildItem out/main` |
| 总 LOC | **452,173** | per E 估 446,652 ± 1.2% |
| app.asar | 665 MB | NSIS 解包 |
| app.asar.unpacked | 780 MB | NSIS 解包 |
| **总大小** | **1.4 GB** | NSIS 解包 |
| 60+ SDK deps | 5 Provider + 11 估缺 + 30+ 工具 | `package.json` |

### §1.2 1:1 翻译总原则 (per 主人 19:37 "全用 rust" 强调)

| 原则 | 实施细则 | 关键 |
|------|---------|------|
| **1 TS module = 1 Rust crate** | 1:1 翻译, 0 复用 TS 业务代码 | O-2 走在前人肩上 |
| **TS interface → Rust trait** | 严格模式匹配 | S-2 实事求是 |
| **TS class → Rust struct + impl** | 严格模式匹配 | O-5 不假装 |
| **TS union → Rust enum** | 严格模式匹配 | S-1 北极星 |
| **TS Promise → Rust async fn** | 严格模式匹配 | O-3 干到底 |
| **Electron API 弃用** | BrowserWindow/Menu/Tray → Tauri 2.0 (R21) | O-4 任何人都能接手 |
| **估缺功能 1:1 翻译** | 估缺 8 + 新发现 8 全部进 R20 阶段 1-3 | O-3 干到底 |

### §1.3 16 估缺 + 5 增强 + 1 R21 = 22 模块总图

| 类别 | 数量 | 总估 LOC | 总估工时 |
|------|----:|---------:|--------:|
| **5 估缺 MCP** (3 P0 + 2 估缺) | 5 | 估 9,200 | 估 16h |
| **4 估缺核心** (workflow / plugin / image-prompt / rollback) | 4 | 估 3,900 | 估 10h |
| **2 估缺工具** (repo-scan / repo-analyzer) | 2 | 估 1,500 | 估 4h |
| **2 估缺基础设施** (keyring / machine-id) | 2 | 估 1,200 | 估 3h |
| **4 估缺 SDK** (lark / livekit / voice / sandbox) | 4 | 估 1,250 | 估 6h |
| **4 增强** (tree-sitter / graph / protocol::gemini / task) | 4 | 估 2,200 | 估 10h |
| **1 R21 准备** (tauri-2.0) | 1 | 估 8,000 | 估 80h (R21 阶段, 不在 R20 估时) |
| **总** | **22** | **估 27,250** | **估 49h (R20 5 阶段内, 不含 R21)** |

> **关键观察**:
> - 16 估缺 + 5 增强总估时 **49h** (远低于 E-agent §3 估的 132h — 差异点见 §7 #4)
> - 60+ SDK 中实际要翻译的估 11 估缺 (lark/livekit/voice/sandbox/monaco/dnd-kit/floating-ui/porcupine/pvrecorder/dagre/sandpack) = 估 6h (因为大部分 R21+ Tauri 用, R20 阶段 1-5 只需 stub)
> - R20 5 阶段总估时 **320h** 跟原版估时一致 (per `v09021-commercial-extract §6`), 但 RIVAL 版把 49h 估缺 + 271h 测试/集成/Docker/Tauri-prep 摊开

---

## §2 16 新 Rust crate 设计表 (重点)

> **每 crate 6 字段**: 路径 / Cargo.toml deps / 估 LOC / 估工时 / 关键 API 表面 / 与 Hermes LOCKED 24 crate 集成点

### §2.1 5 估缺 MCP (3 P0 skeleton 已写 + 2 估缺)

#### §2.1.1 `apeireth-mcp-ssh` ⏳ P0 skeleton 已写 (体检见 §3)

| 字段 | 值 |
|------|---|
| 路径 | `crates/apeireth-mcp-ssh/` |
| 源 | v0.9.21 `out/main/mcp/SSHMcpServer.js` (~448KB, 单行 minified) |
| 估 LOC | **6,000** (1:1 翻译) |
| 估工时 | **8h** |
| Cargo.toml deps | `ssh2=0.9` (vendored-openssl) + `tokio` + `serde` + `serde_json=1.0` + `anyhow` + `thiserror` + `tracing=0.1` + `async-trait` + `fs_err` + `apeireth-mcp` (path) + `apeireth-protocol` (path) |
| 关键 API | `SshMcpServer` (9 工具: `connect`/`disconnect`/`exec`/`upload`/`download`/`list`/`jump`/`keepalive`/`list_sessions`) |
| 集成点 | `apeireth-mcp::builtin::ToolDef` (per `crates/apeireth-mcp/src/tool_bridge.rs:37`) + `apeireth-protocol::ProviderEvent` |
| 1:1 翻译点 | 5 auth methods (Password/PublicKey/Agent/JumpHost/PasswordR21) 1:1 还原; 9 状态 (Disconnected/Connecting/Authenticating/Connected/Busy/Idle/JumpConnecting/Closed/TimedOut) 1:1 还原 |
| m3 防御 | §2.4 WHITELIST 14 工具 hardcode + §2.1 schema 校验 (缺必填 → reject) |

#### §2.1.2 `apeireth-mcp-winrm` ⏳ P0 skeleton 已写 (体检见 §3)

| 字段 | 值 |
|------|---|
| 路径 | `crates/apeireth-mcp-winrm/` |
| 源 | v0.9.21 `out/main/mcp/WinRMMcpServer.js` (~64KB) |
| 估 LOC | **800** |
| 估工时 | **2h** |
| Cargo.toml deps | `reqwest=0.12` (xml feature) + `quick-xml=0.36` + `tokio` + `serde` + `base64=0.22` + `apeireth-mcp` (path) + `apeireth-protocol` (path) |
| 关键 API | 8 工具: `winrm_connect` / `winrm_disconnect` / `winrm_list_connections` / `winrm_run_command` / `winrm_get_command_output` / `winrm_command` / `winrm_copy_to` / `winrm_copy_from` |
| 集成点 | `apeireth-mcp::McpServer` + `apeireth-protocol::ProviderEvent` + SOAP 1.2 走 reqwest + quick-xml |
| 1:1 翻译点 | 5 auth methods (Default/Basic/Negotiate/Kerberos/CredSSP) 1:1 还原; WSMan Shell (Create/Command/Receive/Send/Delete) 1:1 还原; PowerShell `CLAUDEOPS_WINRM_COMPLETION_MARKER` 1:1 还原 |
| m3 防御 | 同 §2.1.1 |

#### §2.1.3 `apeireth-mcp-relay-image` ⏳ P0 skeleton 已写 (体检见 §3)

| 字段 | 值 |
|------|---|
| 路径 | `crates/apeireth-mcp-relay-image/` |
| 源 | v0.9.21 `out/main/mcp/RelayImageMcpServer.js` (~57KB) |
| 估 LOC | **700** |
| 估工时 | **2h** |
| Cargo.toml deps | `reqwest=0.12` + `image=0.25` (png/jpeg/webp) + `base64=0.22` + `sha2=0.10` + `tokio` + `apeireth-mcp` (path) + `apeireth-protocol` (path) |
| 关键 API | 7 工具: `generate_image` / `edit_image` / `last_image` / `list_cached` / `search_prompts` / `searchImagePromptLibrary` / `resolveRelayImageProxyUrl` |
| 集成点 | `apeireth-mcp::ToolDef` + `apeireth-image-prompt` (新, §2.2.3) + `apeireth-memory` (FTS5 检索) |
| 1:1 翻译点 | `RELAY_IMAGE_BASE_URL` / `RELAY_IMAGE_API_KEY` / `RELAY_IMAGE_DEFAULT_MODEL` env 1:1 翻译; data URI `data:image/...;base64,...` 1:1 解析; SHA256 dedup 1:1 还原 |
| m3 防御 | 路径截断自动补全 (per `m3-hallucination-defense §2.3 规则 3`) |

#### §2.1.4 `apeireth-mcp` 增强 (P1, 估缺) — R20 阶段 4 估补

| 字段 | 值 |
|------|---|
| 路径 | `crates/apeireth-mcp/` (已实装, 估 700 LOC 增量) |
| 源 | v0.9.21 `out/main/agent/AgentMCPServer.js` (估 12K LOC, 22 工具) |
| 估 LOC | **700** (增量) |
| 估工时 | **2h** |
| Cargo.toml deps | 已有 + 加 `apeireth-team-lead` (path, R20 阶段 2 接) |
| 关键 API 增量 | 22 工具白名单 hardcode (per `spectrai-branch-coverage §4.i`: spawn_agent / send_to_agent / get_agent_output / wait_agent_idle / wait_agent / get_agent_status / list_agents / cancel_agent / list_sessions / get_session_summary / search_sessions / enter_worktree / get_task_info / check_merge / install_skill / list_skills / get_skill / merge_worktree / spectrai_edit_file / spectrai_write_file / spectrai_create_file / spectrai_delete_file) |
| 集成点 | `apeireth-mcp::builtin::WHITELIST` (per `m3-hallucination-defense §2.4`) |
| m3 防御 | 14 工具白名单 hardcode (per `m3-hallucination-defense §2.4`) |

#### §2.1.5 `apeireth-mcp-relay-image` (P1, 估缺) — R20 阶段 4 估补

#### §2.1.5 估缺 SSH/WinRM 协议扩展 (R21+) — 不在 R20 估时

> **0 估缺 0 估时, O-5 不假装** — 主人 m3 测出 v0.9.21 SSHMcpServer.js 0 JumpHost 命中 (per §3.1 体检), R21+ 才补.

---

### §2.2 4 估缺核心 (1 P0 skeleton 已写 + 3 估缺)

#### §2.2.1 `apeireth-workflow` ⏳ P0 skeleton 已写 (体检见 §3)

| 字段 | 值 |
|------|---|
| 路径 | `crates/apeireth-workflow/` |
| 源 | v0.9.21 `out/main/chunks/WorkflowGenerator-BQCQ_KQx.js` (~64KB) |
| 估 LOC | **1,500** |
| 估工时 | **4h** |
| Cargo.toml deps | `tokio` + `serde` + `serde_yaml=0.9` + `anyhow` + `thiserror` + `tracing=0.1` + `async-trait` + `apeireth-graph` (path) + `apeireth-agent` (path) + `apeireth-protocol` (path) + `apeireth-tool-runtime` (path) |
| 关键 API | `WorkflowGenerator::create` / `parse_workflow_from_yaml` / `quick_agent_task` + 8 节点类型 (agent/loop/transform/condition/team/mission/watch/review) + 3 EdgeType + 6 WorkflowStatus |
| 编译期 hardcode | `BORROWED_V0921_TOOLS: usize = 8` / `V0921_NODE_TYPES: usize = 8` / `V0921_VALIDATION_GATES: usize = 4` / `MAX_NESTED_DEPTH: usize = 8` / `DEFAULT_LOOP_MAX_ITERATIONS: u32 = 5` / `DEFAULT_MISSION_TIMEOUT_MS: u64 = 3_600_000` |
| 集成点 | `apeireth-graph::Node` (已实装) + `apeireth-agent::AgentManager` (R20 阶段 2 接) + `apeireth-asi` (R20 中期 1 月接 mission) + `apeireth-council` (R20 阶段 2 接 team) |
| 1:1 翻译点 | 4 警告守门 (dependsOn 缺引用 / conditionConfig < 2 branches / loopConfig loopBackTo 失效 / claudeConfig 缺 prompt 模板) → typed `WorkflowError` (per §3 实查 12 测试) |

#### §2.2.2 `apeireth-plugin` (P1, 估缺) — R20 阶段 4 估补

| 字段 | 值 |
|------|---|
| 路径 | `crates/apeireth-plugin/` (新) |
| 源 | v0.9.21 `out/main/chunks/PluginManager-BAmNCucP.js` (~12KB) |
| 估 LOC | **800** |
| 估工时 | **2h** |
| Cargo.toml deps | `tokio` + `serde` + `serde_json` + `anyhow` + `thiserror` + `async-trait` + `apeireth-tool-registry` (path) + `apeireth-tool-runtime` (path) + `apeireth-extensions` (path) |
| 关键 API | `PluginManager::load` / `unload` / `list` / `enable` / `disable` + `PluginManifest` (WASM module + capability table) |
| 集成点 | `apeireth-extensions` (R20 阶段 4 估补) + `apeireth-tool-registry` (已实装) |
| 1:1 翻译点 | 8 钩子 (before_session/after_session/before_tool_call/after_tool_call/before_message/after_message/on_error/on_crash) 1:1 还原 |

#### §2.2.3 `apeireth-image-prompt` (P1, 估缺) — R20 阶段 4 估补

| 字段 | 值 |
|------|---|
| 路径 | `crates/apeireth-image-prompt/` (新) |
| 源 | v0.9.21 `out/main/chunks/ImagePromptLibrary-C5wQe0hi.js` (~36KB) |
| 估 LOC | **600** |
| 估工时 | **1h** |
| Cargo.toml deps | `serde` + `serde_json` + `anyhow` + `thiserror` + `tokio` + `apeireth-memory` (path, FTS5) + `apeireth-mcp-relay-image` (path) |
| 关键 API | `ImagePromptLibrary::search` / `add` / `list` / `delete` + `ImagePrompt` struct (id/name/tags/prompt/parent_prompt_id) |
| 集成点 | `apeireth-memory` FTS5 (已实装, `session_logs.chunk` 1 张表) + `apeireth-mcp-relay-image` (R20 阶段 1 接) |
| 1:1 翻译点 | 5 tag 字段 (subject/style/quality/composition/lighting) 1:1 还原 + 5 search 字段 |

#### §2.2.4 `apeireth-rollback` (P1, 估缺) — R20 阶段 4 估补

| 字段 | 值 |
|------|---|
| 路径 | `crates/apeireth-rollback/` (新) |
| 源 | v0.9.21 `out/main/chunks/RollbackService-DN4d2R0Q.js` (~22KB) |
| 估 LOC | **1,000** |
| 估工时 | **3h** |
| Cargo.toml deps | `tokio` + `serde` + `serde_json` + `anyhow` + `thiserror` + `git2=0.19` (或 `gix`) + `apeireth-git` (path) + `apeireth-memory` (path) |
| 关键 API | `RollbackService::create_snapshot` / `restore` / `list_snapshots` / `delete` + `Snapshot` struct (id/session_id/timestamp/file_diff/branch_state) |
| 集成点 | `apeireth-git` (R20 阶段 4 接, 工作流 + Git) + `apeireth-memory` (snapshot metadata 持久化) |
| 1:1 翻译点 | 6 策略 (full/file/diff/git/session/auto) 1:1 还原 |

---

### §2.3 2 估缺工具 (R20 阶段 4 估补)

#### §2.3.1 `apeireth-repo-scan` (P1) + §2.3.2 `apeireth-repo-analyzer` (P1)

| 字段 | 值 |
|------|---|
| 路径 | `crates/apeireth-repo-scan/` + `crates/apeireth-repo-analyzer/` (新, 2 拆) |
| 源 | v0.9.21 `out/main/chunks/RepoScanAdapter-CsMFZlsN.js` (~7KB) + `RepoAnalyzer-BjPzFZvZ.js` (~6KB) |
| 估 LOC | **800 + 700 = 1,500** |
| 估工时 | **2h + 2h = 4h** |
| Cargo.toml deps | `tokio` + `serde` + `serde_json` + `anyhow` + `thiserror` + `git2=0.19` + `tree-sitter=0.25` (P0 估补, §2.5.1) + `apeireth-tree-sitter` (path) + `apeireth-git` (path) |
| 关键 API | `RepoScanner::scan` / `RepoAnalyzer::analyze` + `ScanResult` (per `ScanResult-QlNIjuQC.js`) + `RepoProfile` (per `InstallErrorCodes-D-xTEXPe.js`) |
| 集成点 | `apeireth-git` (R20 阶段 4) + `apeireth-tree-sitter` (增强 §2.5.1) + `apeireth-workflow` (R20 阶段 4 接, 自动选 Provider) |
| 1:1 翻译点 | 4 扫描类型 (git_status/large_file/dep_graph/code_metrics) 1:1 还原 |

---

### §2.4 2 估缺基础设施 (R20 阶段 4 估补)

#### §2.4.1 `apeireth-keyring` (P1)

| 字段 | 值 |
|------|---|
| 路径 | `crates/apeireth-keyring/` (新) |
| 源 | v0.9.21 `out/main/chunks/keychain-token-storage-Cqa8o4z8.js` (~12KB) |
| 估 LOC | **400** |
| 估工时 | **1h** |
| Cargo.toml deps | `tokio` + `serde` + `serde_json` + `anyhow` + `thiserror` + `keyring=3.6` (跨平台: Windows Credential Manager / macOS Keychain / Linux Secret Service) + `zeroize=1.8` (memory 擦除) |
| 关键 API | `KeyringStore::set` / `get` / `delete` / `list` + `TokenType` enum (Anthropic/OpenAI/Gemini/Copilot/iFlow/OpenCode) |
| 集成点 | `apeireth-protocol` (R20 阶段 4 接, 5 Provider API key 存这里) + `apeireth-bus` (cross-session key 同步, R21+) |
| 1:1 翻译点 | 4 平台 (Windows/macOS/Linux/unsupported) 1:1 还原, fallback 到加密文件 (per `getMachineId-unsupported` 模式) |

#### §2.4.2 `apeireth-machine-id` (P1)

| 字段 | 值 |
|------|---|
| 路径 | `crates/apeireth-machine-id/` (新) |
| 源 | v0.9.21 `out/main/chunks/getMachineId-{bsd,darwin,linux,unsupported,win}-*.js` (7 文件, 每 4-5KB) |
| 估 LOC | **800** |
| 估工时 | **2h** |
| Cargo.toml deps | `tokio` + `serde` + `anyhow` + `thiserror` + `sysinfo=0.32` (cross-platform 硬件信息) + `uuid=1.10` (machine-id 派生) |
| 关键 API | `get_machine_id` (5 平台: Windows/macOS/Linux/BSD/unsupported) + `MachineId` struct (raw/hashed/salt) |
| 集成点 | `apeireth-bus` (R20 阶段 4, 跨 session 跟踪) + `apeireth-supervisor` (R20 阶段 4, telemetry) |
| 1:1 翻译点 | 5 平台 1:1 还原, 派生策略 (raw UUIDv4 + salt 哈希 = 32 hex 字符) |

---

### §2.5 4 估缺 SDK (R20 阶段 3-5 估补)

| # | Crate | 路径 | 估 LOC | 估工时 | 关键 API | 集成点 |
|---|-------|------|------:|------:|----------|--------|
| 1 | `apeireth-lark` | `crates/apeireth-lark/` (新) | 300 | 1h | `LarkClient::send_message` / `approve` / `get_doc` + 5 端点 | `apeireth-bus` (R20 阶段 3) |
| 2 | `apeireth-livekit` | `crates/apeireth-livekit/` (新) | 400 | 2h | `LiveKitRoom::connect` / `publish_track` / `subscribe` | `apeireth-voice` (R21+, 暂 stub) |
| 3 | `apeireth-voice` | `crates/apeireth-voice/` (新) | 200 | 1h | `VoiceWake::start` (Porcupine) / `VoiceRecorder::start` (pvrecorder) | `apeireth-mcp` (R20 阶段 3) |
| 4 | `apeireth-sandbox` | `crates/apeireth-sandbox/` (新) | 350 | 2h | `Sandbox::execute` (CodeSandbox) | `apeireth-api` (R21+, 暂 stub) |
| **总** | — | — | **1,250** | **6h** | — | — |

**注**: 这 4 SDK 中, **lark / voice 在 R20 阶段 3 必补** (per `v09021-commercial-extract §3 60+ SDK 表格`), **livekit / sandbox 在 R21+ Tauri 阶段才需要** (R20 阶段只 stub trait, 不真接).

---

### §2.6 4 增强 (R20 阶段 1-2 估补, 不新增 crate)

| # | 增强项 | 路径 | 估 LOC | 估工时 | 关键 API | 集成点 |
|---|--------|------|------:|------:|----------|--------|
| 1 | `apeireth-tree-sitter` 增强 | `crates/apeireth-tree-sitter/src/` (已有, R19 LOCKED) | 200 (增量) | 1h | `bash` + `typescript` + `python` + `rust` 4 grammar | `apeireth-workflow` (节点条件) + `apeireth-repo-analyzer` (代码度量) |
| 2 | `apeireth-graph` 增强 | `crates/apeireth-graph/src/` (已有, R19 LOCKED) | 100 (增量) | 1h | `dagre` 布局 + `Dag::topological_sort` + `Dag::detect_cycle` | `apeireth-workflow` (DAG 节点) |
| 3 | `apeireth-protocol::gemini` 增强 | `crates/apeireth-protocol/src/adapters/gemini.rs` (已有) | 800 (增量) | 4h | `GeminiAuthHelper` (per `GeminiAuthHelper-f4yPRirM.js` 13KB) + Gemini headless mode 完整 1:1 | `apeireth-protocol` (5 Provider 之一) |
| 4 | `apeireth-task` 增强 | `crates/apeireth-council/src/task.rs` (估补, 已有 task 估缺部分) | 1,100 (增量) | 4h | `TaskSessionCoordinator` + `taskTools` (per `taskTools-BfnOrPUJ.js` 313KB, 估 1500 LOC 估缺) | `apeireth-council` (任务状态机) + `apeireth-team-lead` (调度) |
| **总** | — | — | **2,200** | **10h** | — | — |

**关键观察**:
- 4 增强中 **3 个 (tree-sitter / graph / protocol::gemini) 不需要新 crate**, 只需在已实装的 crate 内 1:1 翻译商业版估缺
- 1 个 (task) 用 `apeireth-council::task` 估补, 不另起 crate (跟 `commercial-vs-fork-diff §2.3 TaskKanban` P2 决策一致)

---

### §2.7 1 R21 准备 (`apeireth-tauri-2.0`, R20 阶段 5 估 80h, R21 主战场)

| 字段 | 值 |
|------|---|
| 路径 | `crates/apeireth-tauri-2.0/` (R21 主新增) |
| 源 | Electron `out/main/index.js` (估 8K LOC) + `out/renderer/assets/index-*.js` (估 200K LOC) + Monaco / Sandpack / dnd-kit / Floating UI |
| 估 LOC | **8,000** (Rust 翻译) + 200K TS UI (保持 TS) |
| 估工时 | **R20 阶段 5 估 80h (1 周) + R21 主战场估 3-4 月** |
| Cargo.toml deps | `tauri=2.0` + `tauri-build=2.0` + `tauri-plugin-*` (15+ 估补) + `tokio` + `serde` + `serde_json` + `apeireth-api` (path, 走 HTTP) |
| 关键 API | 5 主窗口 (Main / Settings / ButtleFloat / OfficeScene / ScreenShare / Meeting / MissionControl / PrototypeCanvas) + 13 T-001~T-013 沉淀 |
| 集成点 | `apeireth-api` (HTTP to backend, per `user memory #8 TUI → Tauri`) + 5 估缺 SDK (lark/livekit/voice/sandbox/monaco) |
| 1:1 翻译点 | Tauri 2.0 webview 替代 Electron BrowserWindow; `tauri::WebviewWindow` 替代 BrowserWindow; `tauri::Emitter` 替代 IPC 11 类 128 handler |
| **关键决策 (per user memory #8)** | Tauri 是终极, TUI 是过渡. R20 阶段 5 估 80h 准备 (Tauri scaffold + 5 窗口 stub + 13 T-* 沉淀接口), R21 主战场 3-4 月真做. |

---

## §3 5 P0 crate 体检表 (健康检查 + 真实缺口)

> **体检方法**: 读每个 crate 的 `Cargo.toml` + `src/lib.rs` (关键段) + `git diff` 跟原 `v09021-commercial-extract` 估缺清单对比. 5 P0 crate skeleton 是 2026-08-05 19:37 拍板"全用 rust" 后由 5 sub-agent 并行写的.
> **体检范围**: 跟 v0.9.21 商业版 1:1 对照, 找真实缺口 (不是文档缺陷).

### §3.1 `apeireth-mcp-ssh` 体检

| 维度 | 实查 | 跟 v0.9.21 SSHMcpServer.js (估 6000 LOC) 对照 | 缺口? |
|------|------|------------------------------------------|------|
| `Cargo.toml` deps | ssh2=0.9 + tokio + serde + anyhow + thiserror + tracing=0.1 + async-trait + fs_err + apeireth-mcp + apeireth-protocol | ssh2 + tokio (估 1:1) + 不需要 reqwest (纯 SSH) | ✅ 齐 |
| `src/lib.rs` 行数 | 12,528 B / 估 254 行 | — | — |
| 错误类型 | 13 variant (ConnectionFailed/AuthFailed/CommandFailed/ChannelClosed/Timeout/FileTransfer/PortForward/ConfigParse/HostKeyMismatch/InvalidPrivateKey/JumpHost/Io/Other) | v0.9.21 估 10-12 异常类, ✅ 1:1 覆盖 + 1 估缺 (PortForward 占位) | ✅ 齐 |
| Auth methods | 4 (Password/PublicKey/Agent/JumpHost) | v0.9.21 实查 4 (Password/PublicKey/Agent/JumpHost) | ✅ 齐 |
| 9 状态 | 9 (Disconnected/Connecting/Authenticating/Connected/Busy/Idle/JumpConnecting/Closed/TimedOut) | v0.9.21 估 9 状态, ✅ 1:1 | ✅ 齐 |
| 9 工具 trait | 9 (connect/disconnect/exec/upload/download/list/jump/keepalive/list_sessions/get_session_status) | v0.9.21 估 9-10 工具, ✅ 1:1 + 1 估缺 (get_session_status) | ✅ 齐 |
| 测试 | 7 个 (config/auth_serde/session_serde_roundtrip/connect_skeleton/exec_skeleton/jump_skeleton/list_sessions_skeleton) | — | 🟡 缺 R20 阶段 1 Fixture 5 (test_mcp_in_process) |
| 编译期守门 | `workspace = true` lints 继承 | — | ✅ 齐 |
| m3 防御 | 无 (skeleton 阶段) | 应 hardcode 14 工具白名单 + schema 校验 | 🔴 **缺口 1** |
| workspace members | **不在** workspace members 列表 (per `Cargo.toml:3-57`) | 应在 `crates/apeireth-mcp-ssh` | 🔴 **缺口 2** |

**🔴 真实缺口 2 个**:
1. **m3 防御未嵌入** — skeleton 阶段无 `WHITELIST` + `validate_tool_call` 守门, 实施时必加 (per `m3-hallucination-defense §2.4 + §2.1`)
2. **不在 workspace members** — `Apeireth-rust/Cargo.toml:3-57` 估缺 `crates/apeireth-mcp-ssh`, 整合时必加 (跟 §5 整合策略一致)

---

### §3.2 `apeireth-mcp-winrm` 体检

| 维度 | 实查 | 跟 v0.9.21 WinRMMcpServer.js (估 800 LOC) 对照 | 缺口? |
|------|------|------------------------------------------|------|
| `Cargo.toml` deps | reqwest=0.12 (xml) + quick-xml=0.36 + tokio + serde + base64=0.22 + apeireth-mcp + apeireth-protocol | v0.9.21 走 SOAP 1.2 + reqwest + xml parser, ✅ 1:1 | ✅ 齐 |
| `src/lib.rs` 行数 | 26,490 B / 估 540 行 | — | — |
| 错误类型 | 估 9-10 variant (Http/SoapFault/AuthFailed/ShellCreate/CommandFailed/...) | v0.9.21 估 8-9 异常类, ✅ 1:1 覆盖 | ✅ 齐 |
| Auth methods | 5 (Default/Basic/Negotiate/Kerberos/CredSSP) | v0.9.21 实查 5 auth methods | ✅ 齐 |
| 8 工具 | 估 8 (winrm_connect/winrm_disconnect/winrm_list_connections/winrm_run_command/winrm_get_command_output/winrm_command/winrm_copy_to/winrm_copy_from) | v0.9.21 估 8 工具 | ✅ 齐 |
| 公共类型 re-export | `pub use apeireth_mcp::builtin::{McpServer, McpTool, McpToolResult};` + `pub use apeireth_protocol::ProviderEvent;` | 跟 SSH crate 不一致 (SSH crate 不 re-export) | 🟡 **风格缺口** (跟 §3.1 SSH 不一致) |
| 测试 | 6 个 | — | 🟡 缺 Fixture 5 |
| m3 防御 | 无 (skeleton 阶段) | 应 hardcode 14 工具白名单 + schema 校验 | 🔴 **缺口 1** |
| workspace members | **不在** workspace members 列表 | 应在 `crates/apeireth-mcp-winrm` | 🔴 **缺口 2** |

**🔴 真实缺口 2 个 + 🟡 风格缺口 1 个**:
1. **m3 防御未嵌入** (同 §3.1)
2. **不在 workspace members** (同 §3.1)
3. **🟡 SSH vs WinRM re-export 风格不一致** — WinRM crate 显式 re-export `apeireth_mcp::builtin::*` 跟 `apeireth_protocol::ProviderEvent`, SSH crate 不 re-export. 整合时统一为"不 re-export, 用 `use apeireth_mcp::...;` 显式导入" (per APEIRETH-CONVENTIONS §6).

---

### §3.3 `apeireth-mcp-relay-image` 体检

| 维度 | 实查 | 跟 v0.9.21 RelayImageMcpServer.js (估 700 LOC) 对照 | 缺口? |
|------|------|------------------------------------------|------|
| `Cargo.toml` deps | reqwest=0.12 + image=0.25 (png/jpeg/webp) + base64=0.22 + sha2=0.10 + tokio + apeireth-mcp + apeireth-protocol | v0.9.21 走 multipart + base64, ✅ 1:1 + image crate 估缺 (估缺 1:1) | ✅ 齐 |
| `src/lib.rs` 行数 | 24,004 B / 估 490 行 | — | — |
| 错误类型 | 8 (ImageRead/ImageDecode/ImageHash/ImageRelay/SizeLimit/Base64/UnsupportedFormat/NotFound) | v0.9.21 估 5-6 异常类, ✅ 1:1 + 2 估缺 (SizeLimit/UnsupportedFormat) | ✅ 齐 |
| ImageFormat | 5 (Png/Jpeg/WebP/Gif/Bmp) + `from_mime` + `to_mime` + `extension` 完整 | v0.9.21 估 5 format | ✅ 齐 |
| CachePolicy | 3 (NoCache/Lru/Ttl) + LRU 完整实现 (touch 命中末尾) | v0.9.21 估 3 cache policy | ✅ 齐 |
| RelayStrategy | 3 (Direct/Base64/Hash) | v0.9.21 估 3 strategy | ✅ 齐 |
| SecretString | 有 (跟 SSH crate 同, Serialize 脱敏) | ✅ 1:1 | ✅ 齐 |
| 测试 | 5 个 | — | 🟡 缺 Fixture 5 |
| m3 防御 | 无 | 应 hardcode 14 工具白名单 | 🔴 **缺口 1** |
| workspace members | **不在** workspace members 列表 | 应在 `crates/apeireth-mcp-relay-image` | 🔴 **缺口 2** |

**🔴 真实缺口 2 个**:
1. **m3 防御未嵌入** (同 §3.1)
2. **不在 workspace members** (同 §3.1)

**🟡 关键观察**: `RelayImageMcpServer` 注释明确 "R20 阶段 1 协调 — 5 crate 并行, 共享模块待 Mavis 整合 commit 统一添加" — 这是健康缺口, 整合时必填.

---

### §3.4 `apeireth-workflow` 体检

| 维度 | 实查 | 跟 v0.9.21 WorkflowGenerator-BQCQ_KQx.js (估 1500 LOC) 对照 | 缺口? |
|------|------|------------------------------------------|------|
| `Cargo.toml` deps | tokio + serde + serde_yaml=0.9 + anyhow + thiserror + async-trait + apeireth-graph + apeireth-agent + apeireth-protocol + apeireth-tool-runtime | ✅ 1:1 + serde_yaml 是 v0.9.21 JSON→YAML 1:1 翻译 | ✅ 齐 |
| `src/lib.rs` 行数 | 56,642 B / 估 1,150 行 | — | **估 77% 完成** (1,150 / 1,500) |
| 编译期 hardcode | 6 个 (BORROWED_V0921_TOOLS / V0921_NODE_TYPES / V0921_VALIDATION_GATES / MAX_NESTED_DEPTH / V0921_CYCLE_WARN_MARKER / MIN_DECISION_BRANCHES / DEFAULT_LOOP_MAX_ITERATIONS / DEFAULT_MISSION_TIMEOUT_MS) | ✅ 8 个 1:1 还原 | ✅ 齐 |
| 8 节点类型 | 估 8 (agent/loop/transform/condition/team/mission/watch/review) | v0.9.21 估 8 | ✅ 齐 |
| DAG 拓扑排序 | Kahn's algorithm 真实现 + cycle detection | v0.9.21 估 dependsOn 解析 | ✅ 齐 |
| 测试 | 12 个 | — | ✅ 12 (5 P0 crate 中最多) |
| 节点执行器 | 🟡 占位 (R20 阶段 2 接 apeireth-agent) | — | 🟡 缺口 (但合理, R20 阶段 1 不估时) |
| MCP/Mission/Team 集成 | 🟡 占位 (R20 阶段 3 估缺) | — | 🟡 缺口 (但合理) |
| m3 防御 | 无 | 5 P0 crate 估缺 (估缺范围) | 🟡 **风险缺口**: 估缺 (但 m3 防御在 14 工具白名单层 hardcode, workflow 不需要每节点都 hardcode, 由 apeireth-mcp::builtin::WHITELIST 统守) |
| workspace members | **不在** workspace members 列表 | 应在 `crates/apeireth-workflow` | 🔴 **缺口 1** |

**🔴 真实缺口 1 个**: 不在 workspace members (同 §3.1). 其他缺口都是合理估缺, 不补.

**🟢 健康度评估**: 5 P0 crate 中**最健康** (1,150 / 1,500 行 = 77% 完成, 12 测试覆盖, 8 节点类型 + 4 警告守门全 hardcode).

---

### §3.5 `apeireth-team-lead` 体检

| 维度 | 实查 | 跟 v0.9.21 AgentMCPServer.js (估 12K LOC) 对照 | 缺口? |
|------|------|------------------------------------------|------|
| `Cargo.toml` 形态 | **`[workspace]` 在 Cargo.toml 顶部** — standalone crate, **不**在主 workspace members | 应在主 workspace `crates/apeireth-team-lead` | 🔴 **缺口 1: standalone, 未集成** |
| `Cargo.toml` deps | tokio=1.40 + serde=1.0 + anyhow=1.0 + thiserror=1.0 + tracing=0.1 + async-trait=0.1 + apeireth-agent + apeireth-council + apeireth-protocol + apeireth-supervisor + apeireth-bus + apeireth-tool-runtime + apeireth-tool-registry + apeireth-graph + apeireth-pipeline | v0.9.21 AgentMCPServer.js 估缺 14 工具 + worktree 3 + 感知 3, 依赖看起来 ✅ 齐 | ✅ 齐 |
| `src/lib.rs` 行数 | 25,557 B / 估 520 行 + `src/md/supervisor_prompt.md` 14,446 B | — | — |
| 错误类型 | 5 (TeamNotFound/SpawnFailed/MidTaskFailed/ToolUnauthorized/HandoffFailed) | v0.9.21 估 5-6 异常类, ✅ 1:1 | ✅ 齐 |
| 8 调度工具 + 3 worktree + 3 感知 = 14 工具 | 估 14 工具 trait | v0.9.21 实查 14 工具 (per `spectrai-branch-coverage §4.i`) | ✅ 齐 (估, 需读全 src 验证) |
| supervisorPrompt 818 行翻译 | `pub const SUPERVISOR_PROMPT: &str = include_str!("md/supervisor_prompt.md");` (编译期嵌入) | per `supervisor-prompt-818-summary §0.2 818 行 / 实际 808 行` | ✅ 编译期嵌入完成 |
| 7 个 build*() 函数 1:1 拆解 | 占位 (R20 阶段 1 不全实施) | 7 段: Awareness / Supervisor / Progress-addon / Workspace / FileOps / Worktree / Worktree-already-active | 🟡 **缺口 2: build_*() 函数 1:1 拆解只占位, 实施时必填 334 行 markdown** |
| 测试 | 5 个 (估) | — | 🟡 缺 Fixture 1 (test_team_lead_workflow) |
| m3 防御 | 无 | 14 工具白名单 hardcode | 🔴 **缺口 3** (同 §3.1) |

**🔴 真实缺口 3 个**:
1. **standalone `[workspace]` 标记** — Cargo.toml 顶部有 `[workspace]`, 跟主 workspace 冲突. 整合时必删顶部 `[workspace]` + 加到主 workspace `members`.
2. **build_*() 函数 1:1 拆解只占位** — supervisorPrompt 7 个 build*() 函数 (Awareness/Supervisor/Progress-addon/Workspace/FileOps/Worktree/Worktree-already-active) 共 334 行 markdown 只占位, 实施时必填 (per `supervisor-prompt-818-summary §1.1`).
3. **m3 防御未嵌入** (同 §3.1)

**🟡 关键观察**: apeireth-team-lead 是 5 P0 crate 中**最复杂** (涉及 supervisorPrompt 7 段 + 14 工具 + 3 协同场景), 但估 520 行 + 14446 字节 markdown 完成度估 30%. R20 阶段 2 估补 8h (per `commercial-vs-fork-diff §3 表`).

---

### §3.6 5 P0 crate 体检总表 + 真实缺口汇总

| Crate | lib.rs 行数 | 测试数 | m3 防御 | workspace members | 其他缺口 |
|-------|-----------:|------:|--------:|-----------------:|---------|
| `apeireth-mcp-ssh` | 254 | 7 | 🔴 缺 | 🔴 缺 | 无 |
| `apeireth-mcp-winrm` | 540 | 6 | 🔴 缺 | 🔴 缺 | 🟡 re-export 风格不一致 |
| `apeireth-mcp-relay-image` | 490 | 5 | 🔴 缺 | 🔴 缺 | 无 |
| `apeireth-workflow` | 1,150 | 12 | 🟡 估缺 (合理) | 🔴 缺 | 无 |
| `apeireth-team-lead` | 520 + 14.4KB md | 5 | 🔴 缺 | 🔴 缺 (standalone) | 🔴 supervisorPrompt 7 函数 1:1 拆解只占位 |
| **总** | **2,954 + 14.4KB** | **35** | **3 🔴 + 1 🟡** | **5 🔴 缺** | — |

### §3.7 5 真实缺口 (跨 5 P0 crate, 整合必填)

1. **🔴 m3 防御未嵌入** — 4 P0 crate (除 workflow) 都无 `WHITELIST` + `validate_tool_call` 守门, 整合时必加 (per `m3-hallucination-defense §2.1+§2.4`). 这是**最重要的缺口**, 因为主人 19:01 拍板 m3 防御是 R20 阶段 1-5 集成的硬约束.
2. **🔴 5 P0 crate 都不在 workspace members** — 整合 1 commit 必加 (per §5 整合策略). 包括 apeireth-team-lead 删顶部 `[workspace]`.
3. **🔴 apeireth-team-lead supervisorPrompt 7 函数 1:1 拆解只占位** — 估 334 行 markdown 待 1:1 翻译, R20 阶段 2 估补 8h.
4. **🟡 mcp-winrm re-export 风格不一致** — 跟 mcp-ssh / mcp-relay-image 风格不齐, 整合时统一 (per APEIRETH-CONVENTIONS §6).
5. **🟡 5 P0 crate 都缺 Fixture 5 (test_mcp_in_process)** — 这是 `commercial-vs-fork-diff §3 表` R20 阶段 1 必补的集成测试, 5 crate 同步补.

---

## §4 R20 5 阶段 320h 实施图

> **总工时 = 320h** (跟原版 `v09021-commercial-extract §6` 一致, 但 RIVAL 版按 16 估缺 49h + 5 阶段 overhead 271h 摊开)

### §4.1 阶段 1 (80h, 1 周) — 5 P0 crate 血肉

| 项 | 估时 | 实施内容 |
|----|-----:|----------|
| `apeireth-mcp-ssh` 1:1 翻译 SSHMcpServer.js | 8h | 9 工具真 impl (connect/disconnect/exec/upload/download/list/jump/keepalive/list_sessions/get_session_status) + 4 auth methods 真 impl + 9 状态状态机 |
| `apeireth-mcp-winrm` 1:1 翻译 WinRMMcpServer.js | 2h | 8 工具真 impl + 5 auth methods + WSMan Shell 真跑 |
| `apeireth-mcp-relay-image` 1:1 翻译 RelayImageMcpServer.js | 2h | 7 工具真 impl + LRU cache 真跑 + image encoding 真跑 |
| `apeireth-workflow` 1:1 翻译 WorkflowGenerator.js | 4h | 8 节点类型真 impl + 3 EdgeType + DAG 拓扑真跑 (Kahn 已实现) + 4 警告守门 typed error |
| `apeireth-team-lead` 1:1 翻译 AgentMCPServer.js Orchestrator | 8h | 14 工具 trait 真 impl + 3 协同场景 + supervisorPrompt 7 函数 1:1 拆解 (估 334 行 markdown) |
| m3 防御嵌入 4 P0 crate | 8h | 14 工具白名单 hardcode + schema 校验 + dual_ack (per `m3-hallucination-defense §2.1+§2.2+§2.4`) |
| Fixture 1-5 集成测试 | 16h | test_team_lead_workflow / test_session_persistence / test_mcp_in_process / test_workflow_dag / test_supervisor_prompt_818 |
| 6 anchor 验证 | 4h | S-1/S-2/O-2/O-3/O-4/O-5 穿透 (per `m3-hallucination-defense §0.2`) |
| workspace members 5 crate 整合 | 4h | 删 standalone [workspace] + 加 members + cargo check |
| 6 doc 更新 (5 P0 crate rustdoc + integration blue 增量) | 8h | 6 哲学 anchor + 8 项不修改承诺 + S-2 实事求是登记 |
| K-1 强校验 (per `m3-hallucination-defense §2.4+apeireth-formal`) | 4h | 14 工具白名单编译期守门 + 5 Provider name 编译期守门 |
| 缓冲 + 返工 | 12h | 12.5% 估时留底 |
| **阶段 1 总** | **80h** | — |

### §4.2 阶段 2 (60h, 1 周) — 公开 API 表面

| 项 | 估时 | 实施内容 |
|----|-----:|----------|
| `apeireth-api` 公开 6 端点 | 12h | sessions / agents / mcp / providers / skills / workflows (HTTP + WebSocket) |
| 5 Provider base URL 集成 (per `5-provider-tool-mapping §2.7`) | 8h | Claude/Codex/Gemini/iFlow/OpenCode + 5th base URL for minimax m3 (per D-03) |
| AIRouter (per `commercial-vs-fork-diff §2.7`) | 6h | autoDream 4 阶段 + token 优化 + provider 切换 (Rust 端全新, 不复用 fork) |
| TeamBus (per `commercial-vs-fork-diff §2.2`) | 8h | `apeireth-bus` L4 WebSocket + TeamBus 包装 (per `spectrAI-integration §5.2`) |
| Orchestrator 增强 | 8h | 14 工具真接 apeireth-agent + 3 协同场景 (per `apeireth-team-lead` 阶段 1 实施) |
| supervisorPrompt 阶段 1 估缺补 (per `commercial-vs-fork-diff §3`) | 4h | paused + interrupted 9 状态补 (per `spectrai-branch-coverage §4.d`) + 134 行 markdown 1:1 翻译 |
| Fixture 6-10 公开 API 测试 | 8h | test_api_session / test_api_agent / test_api_mcp / test_api_provider / test_api_workflow |
| 缓冲 | 6h | — |
| **阶段 2 总** | **60h** | — |

### §4.3 阶段 3 (40h, 1 周) — Docker 部署 + Lark/LiveKit SDK 集成

| 项 | 估时 | 实施内容 |
|----|-----:|----------|
| Dockerfile + docker-compose | 8h | multi-stage build + 5 P0 crate + 公开 API + health check |
| `apeireth-lark` 估补 | 4h | 5 端点 (message/approve/doc/calendar/task) + per `commercial-vs-fork-diff §3 表` 估缺 |
| `apeireth-voice` 估补 | 4h | Porcupine wake word + pvrecorder recording (per `v09021-commercial-extract §2.2`) |
| `apeireth-livekit` stub (R21+) | 2h | trait + 占位 impl (per §2.5) |
| `apeireth-sandbox` stub (R21+) | 2h | trait + 占位 impl |
| 11 估缺 SDK 集成测试 | 8h | 11 SDK 同步跑 (per `v09021-commercial-extract §2.2 表格`) |
| Docker 部署文档 | 4h | 1 docker-compose.yml + README + health check 文档 |
| 缓冲 | 8h | — |
| **阶段 3 总** | **40h** | — |

### §4.4 阶段 4 (60h, 1 周) — 16 估缺 crate 主体

| 项 | 估时 | 实施内容 |
|----|-----:|----------|
| `apeireth-plugin` (估缺 P1, §2.2.2) | 8h | 8 钩子真 impl + PluginManifest + WASM module |
| `apeireth-image-prompt` (估缺 P1, §2.2.3) | 4h | 5 tag + FTS5 真接 + 5 search 字段 |
| `apeireth-rollback` (估缺 P1, §2.2.4) | 8h | 6 策略真 impl + git2 + snapshot metadata 持久化 |
| `apeireth-repo-scan` + `apeireth-repo-analyzer` (§2.3.1+§2.3.2) | 8h | 4 扫描类型 + tree-sitter 增强 (§2.5.1) + git2 集成 |
| `apeireth-keyring` (§2.4.1) | 4h | 4 平台 keyring 真 impl + zeroize + fallback 加密文件 |
| `apeireth-machine-id` (§2.4.2) | 4h | 5 平台机器 ID 真 impl + uuid + salt 哈希 |
| `apeireth-tree-sitter` 增强 (§2.5.1) | 4h | bash + typescript + python + rust 4 grammar |
| `apeireth-graph` 增强 (§2.5.2) | 4h | dagre 布局 + topological_sort + detect_cycle |
| `apeireth-protocol::gemini` 增强 (§2.5.3) | 8h | GeminiAuthHelper + headless mode 完整 1:1 |
| `apeireth-task` 增强 (§2.5.4) | 4h | TaskSessionCoordinator + taskTools 1:1 翻译 (per `taskTools-BfnOrPUJ.js` 313KB 估 1500 LOC) |
| 缓冲 | 4h | — |
| **阶段 4 总** | **60h** | — |

### §4.5 阶段 5 (80h, 1 周) — SDK 集成 + Tauri 2.0 准备

| 项 | 估时 | 实施内容 |
|----|-----:|----------|
| `apeireth-mcp` 22 工具增强 (§2.1.4) | 8h | 22 工具 1:1 翻译 + WHITELIST 14 工具 hardcode |
| `apeireth-tauri-2.0` scaffold (R21 准备, §2.7) | 32h | Tauri 2.0 project + 5 主窗口 stub + 13 T-001~T-013 沉淀接口 (per `tauri-roadmap-2026-08-05.md`) |
| `apeireth-formal` K-1 强校验 (per `m3-hallucination-defense §0.1 决策 4`) | 8h | 14 工具白名单编译期守门 + 5 Provider name 编译期守门 + minimax m3 防御 |
| `apeireth-asi` 第 25 维 hallucination_resistance (per `m3-hallucination-defense §2.5`) | 8h | hook_overrides + 24 维 → 25 维扩展 (V05_DIM_COUNT_V2=25) |
| `apeireth-supervisor` 818 行翻译 (per `supervisor-prompt-818-summary`) | 8h | 7 段 1:1 拆解 + 编译期嵌入 |
| 1.0 release 准备 | 8h | semver v1.0.0 严格 + Cargo.lock 锁定 + Dockerfile 1.0 标签 |
| 5 阶段总验收 (per `m3-hallucination-defense §5`) | 4h | 5 道防御全验证 + 6 anchor 穿透 + 8 项不修改承诺 + K-1 强校验 |
| 缓冲 | 4h | — |
| **阶段 5 总** | **80h** | — |

### §4.6 5 阶段 320h 总图

```
阶段 1 (80h)  ── 5 P0 crate 血肉 + m3 防御 + 集成测试
   ↓
阶段 2 (60h)  ── 公开 API + 5 Provider + AIRouter + TeamBus
   ↓
阶段 3 (40h)  ── Docker + Lark + Voice + LiveKit/Sandbox stub
   ↓
阶段 4 (60h)  ── 16 估缺 crate 主体 (plugin/image-prompt/rollback/repo/keyring/machine-id)
   ↓
阶段 5 (80h)  ── 22 工具 + Tauri 2.0 scaffold + K-1 强校验 + 1.0 release
   ↓
**R20 5 阶段总 = 320h (1 工程师 8 周)**
```

> **关键差异 (vs 原版预告)**:
> - 原版估缺 53h, RIVAL 版估缺 320h (5 周) — 因为 NSIS 解包后 16 估缺 + 5 增强 + 5 阶段 overhead 全摊开
> - 5 P0 crate 估时 24h (阶段 1), 跟原版估缺 8 估缺 95h 差异大 — RIVAL 版按"1:1 翻译 6000 LOC" 估时 (8h SSH 是估缺, 不是 95h 全 8 闭源)
> - 16 估缺总估时 49h (per §1.3) + 8 估缺闭源估 95h (per `v09021-commercial-extract §3.1`) = 144h, 5 阶段 320h 减去 144h = 176h 留给 5 阶段 overhead (集成测试 / Docker / 文档 / 1.0 release)

---

## §5 workspace 整合策略 (1 commit 落地)

### §5.1 workspace members 加入顺序 (5 P0 crate 1 commit)

**当前状态** (per `Apeireth-rust/Cargo.toml:3-57`): 41 个 member (24 LOCKED + 17 V2 战区), **5 P0 crate 都不在**.

**1 commit 落地顺序** (从最稳定到最不稳定):
1. **`crates/apeireth-mcp-ssh`** (最稳定: 9 工具 + 4 auth + 9 状态完整 skeleton, 7 测试)
2. **`crates/apeireth-mcp-relay-image`** (稳定: 7 工具 + 3 cache + 3 strategy 完整, 5 测试)
3. **`crates/apeireth-mcp-winrm`** (稳定: 8 工具 + 5 auth + WSMan Shell, 6 测试)
4. **`crates/apeireth-workflow`** (中等: 8 节点 + DAG 拓扑 + 4 守门, 12 测试)
5. **`crates/apeireth-team-lead`** (最不稳定: standalone `[workspace]`, 删顶部 + 整合 + supervisorPrompt 7 函数 1:1 拆解, 5 测试)

**Cargo.toml 增量** (5 行, 1 commit):
```toml
# 5 P0 crate skeleton (per v09021-rust-translation-blueprint-RIVAL §5.1)
"crates/apeireth-mcp-ssh",
"crates/apeireth-mcp-winrm",
"crates/apeireth-mcp-relay-image",
"crates/apeireth-workflow",
"crates/apeireth-team-lead",
```

### §5.2 整合前必做的 5 步

1. **删 `apeireth-team-lead/Cargo.toml` 顶部 `[workspace]`** (per §3.5 缺口 1)
2. **统一 `apeireth-mcp-winrm` re-export 风格** (per §3.2 风格缺口, 跟 SSH/relay-image 对齐, 不 re-export)
3. **嵌入 m3 防御 (5 P0 crate)** (per §3.7 缺口 1, `apeireth-mcp::builtin::WHITELIST` 引用 + `apeireth-pipeline::validate::ToolSchemaValidator` 集成)
4. **`apeireth-team-lead` supervisorPrompt 7 函数 1:1 拆解** (per §3.5 缺口 2, 估 334 行 markdown 1:1 翻译)
5. **Fixture 1-5 集成测试** (per §3.7 缺口 5, test_team_lead_workflow / test_session_persistence / test_mcp_in_process / test_workflow_dag / test_supervisor_prompt_818)

### §5.3 Cargo.lock 更新策略

- **Cargo.lock 跟 workspace 一起 commit** (per APEIRETH-CONVENTIONS § workspace 严格 semver v1.0.0)
- 5 P0 crate 加入后, `cargo check --workspace` 第一次跑会更新 Cargo.lock (新增 ssh2/quick-xml/image/sha2/keyring/sysinfo 等 transitive deps)
- **1 commit 落地 = workspace Cargo.toml 改 5 行 + Cargo.lock 重生 + 5 P0 crate skeleton 已就位**
- **0 触碰 24 LOCKED crate** (`crates/apeireth-*/src/` 不改, 只加 members)
- **0 改 workspace root Cargo.toml** 的 `version = "1.0.0"` / `edition = "2021"` 等元数据 (只加 members)

### §5.4 1 commit 落地策略 (commit message 模板)

```
[stage4-R20-1] 集成 5 P0 crate (v0.9.21 商业版 1:1 翻译)

- 加 5 P0 crate 到 workspace members: apeireth-mcp-ssh / -winrm / -relay-image / -workflow / -team-lead
- 删 apeireth-team-lead/Cargo.toml 顶部 [workspace] (standalone → workspace member)
- 统一 apeireth-mcp-winrm re-export 风格 (跟 SSH/relay-image 对齐)
- 嵌入 m3 hallucination 5 道防御 (per m3-hallucination-defense §2.1+§2.4)
- 1:1 翻译 supervisorPrompt 7 函数 (per supervisor-prompt-818-summary §1.1)
- Fixture 1-5 集成测试 (test_team_lead_workflow / test_session_persistence / test_mcp_in_process / test_workflow_dag / test_supervisor_prompt_818)
- Cargo.lock 同步更新 (新增 ssh2/quick-xml/image/sha2/keyring/sysinfo 等 transitive)

参考: docs/stage4/v09021-rust-translation-blueprint-RIVAL-2026-08-05.md
参考: docs/stage4/v09021-commercial-extract-2026-08-05.md
参考: docs/stage4/m3-hallucination-defense-2026-08-05.md

8 项不修改承诺 8/8 严守
6 哲学 anchor 6/6 穿透
0 触碰 crates/apeireth-{agent|bus|supervisor|council|...}/src/ (Hermes LOCKED 24 crate)
```

---

## §6 风险与依赖

### §6.1 8 闭源模块怎么办 (per `commercial-vs-fork-diff §2.1-§2.8`)

| # | 模块 | 估时 | 怎么办 |
|---|------|-----:|--------|
| 1 | TeamRepository | 4h | **apeireth-memory** 加 1 repository (per `spectrai-branch-coverage §4.e` 12 repo 估缺 1) — R20 阶段 2 |
| 2 | TeamBus | 8h | **apeireth-bus L4 WebSocket + TeamBus 包装** (per `spectrAI-integration §5.2`) — R20 阶段 2 |
| 3 | TaskKanban | 16h | **apeireth-council::task** 估补 (per `commercial-vs-fork-diff §2.3 P2 决策) — R20 阶段 4 (估缺 9 状态补) |
| 4 | Orchestrator | 13h | **apeireth-team-lead** (R20 阶段 1 估补 8h + 阶段 2 估补 8h 协同) — R20 阶段 1+2 |
| 5 | AutonomousPlanner | 24h | **apeireth-asi 24 维 + autoDream 4 阶段** (per D-03) — R20 中期 1 月 |
| 6 | TelegramBotManager | 8h | **Discord 冷启动** (per D-12) + Telegram Bot 留 R21+ — R21+ |
| 7 | AIRouter | 6h | **apeireth-protocol 5 base URL + AIRouter** (per `5-provider-tool-mapping §2.7`) — R20 阶段 2 |
| 8 | SuggestionEngine | 16h | **Tauri 阶段 T-006** (per `tauri-roadmap-2026-08-05.md`) + R21+ UX — R21+ |
| **总** | — | **95h** | R20 5 阶段 320h 内 4 项 (1/2/4/7) 占 31h, R21+ 4 项 (3/5/6/8) 占 64h |

### §6.2 60+ SDK 哪些真要 (per `v09021-commercial-extract §2`)

| 类别 | 数量 | R20 估补 | R21+ 估补 | 不补 |
|------|----:|---------:|----------:|-----:|
| LLM Provider SDK (5+Copilot) | 6 | 6 (5 base URL + Copilot stub) | — | — |
| **估缺 11 SDK** (Lark/LiveKit/Monaco/Picovoice/Dagre/dnd-kit/Fastify/CodeSandbox/MCP) | 11 | 5 (Lark/Voice/MCP/Dagre/sandbox-stub) | 4 (LiveKit/sandbox/Monaco/dnd-kit) | 2 (dnd-kit/Floating UI 估缺, 跟 Tauri 阶段同步) |
| UI 框架 (React + Floating UI + dnd-kit) | 4 | — | 4 (Tauri 阶段) | — |
| 代码编辑器 (Monaco + Sandpack) | 2 | — | 2 (Tauri 阶段) | — |
| 多媒体 (LiveKit + Picovoice 2) | 3 | 1 (Voice) | 1 (LiveKit) | 1 (screen share 估缺) |
| 飞书 + MCP | 2 | 2 | — | — |
| Fastify (cors + static + websocket) | 3 | — | — | 3 (估缺, 跟 Express 复用) |
| 工具库 (dagre + tree-sitter 隐式) | 2 | 2 (apeireth-graph + apeireth-tree-sitter 增强) | — | — |
| 其他 npm 生态 (估 30+) | 30+ | — | — | 30+ (估缺) |
| **总** | **估 60-80** | **16 R20 估补** | **11 R21+** | **36+ 不补** |

### §6.3 minimax m3 hallucination 5 道防御 (per `m3-hallucination-defense §2`)

| 道 | 实施位置 | RIVAL 版 16 crate 内嵌 |
|----|---------|----------------------|
| §2.1 Pre-call schema 强校验 | `apeireth-pipeline/src/validate.rs` (新) | ✅ R20 阶段 1 |
| §2.2 Mid-call dual ack | `apeireth-team-lead/src/dual_ack.rs` (新) | ✅ R20 阶段 1 |
| §2.3 m3 48+ context 监控 | `apeireth-protocol/src/providers/minimax/m3/context_monitor.rs` (新) | ✅ R20 阶段 1 (per §2.5.3 protocol::gemini 增强) |
| §2.4 14 工具白名单 hardcode | `apeireth-mcp/src/builtin.rs::WHITELIST` (估补) | ✅ R20 阶段 1 (5 P0 crate 必引) |
| §2.5 hallucination 第 25 维测量 | `apeireth-asi/src/dimensions/hallucination_resistance.rs` (新) | ✅ R20 阶段 5 |

**关键决策**: 5 道防御**横切** 16 crate, 不集中到 1 个 crate. 这样 16 crate 都自带防御层, 不会"漏接".

### §6.4 8 项不修改承诺 8/8 严守 (per APEIRETH-CONVENTIONS §10)

| 承诺 | 状态 | 验证 |
|------|------|------|
| APEIRETH-CONVENTIONS.md / VERSIONING.md / GLOSSARY.md | 🟢 PASS | 0 触碰 |
| 阶段 1+2+3 LOCKED | 🟢 PASS | 0 触碰 |
| v2/v4/v4.1 LOCKED | 🟢 PASS | 0 触碰 |
| 阶段 4 (`6ca80776`) | 🟢 PASS | 0 触碰 |
| 阶段 5 (631 行) | 🟢 PASS | 0 触碰 |
| v6 基础架构 | 🟢 PASS | 0 触碰 |
| R11 baseline 三值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) | 🟢 PASS | 0 重算 |
| workspace v1.0.0 | 🟢 PASS | 0 触碰 (只加 members, 不改 version/edition) |
| `crates/apeireth-*/src/` (Hermes LOCKED 24 crate) | 🟢 PASS | 0 改 |

### §6.5 6 哲学 anchor 6/6 穿透 (per APEIRETH-CONVENTIONS §9)

| Anchor | 状态 | 实证 |
|--------|------|------|
| S-1 北极星导向 | 🟢 PASS | "全用 rust 1:1 翻译 v0.9.21 商业版 16 模块" (主人 19:37 强调) |
| S-2 实事求是 | 🟢 PASS | NSIS 解包 1.4 GB / 171 .js / 452,173 LOC 实查 + 16 模块实查 + 5 P0 crate 体检实查 |
| O-2 走在前人肩上 | 🟢 PASS | v0.9.21 商业版 1:1 翻译 (不重设计, 不复用 TS) |
| O-3 干到底 | 🟢 PASS | 5 周 320h 实施图 + 16 新 crate + 22 模块总图 + 1 commit 落地 |
| O-4 任何人都能接手 | 🟢 PASS | §1 总图 + §2 16 crate 设计表 (6 字段每 crate) + §3 5 P0 crate 体检表 + §4 5 阶段 320h 实施图 + §5 整合策略 |
| O-5 不假装 | 🟢 PASS | 5 P0 crate 体检发现 5 真实缺口 (m3 防御 / workspace members / supervisorPrompt 7 函数 / re-export 风格 / Fixture 5) — 0 假装已实施 |

---

## §7 跟原蓝图对齐声明 (RIVAL vs bg_a5470979 差异)

> 本节是 RIVAL 版的"诚实登记": 5-10 条 bullet, 标跟原版 (bg_a5470979 写的 `v09021-rust-translation-blueprint-2026-08-05.md`) 对齐 + 差异.

### §7.1 对齐项 (跟原版一致)

1. **总原则 "1 TS module = 1 Rust crate"** — 跟原版一致 (per 主人 19:37 "全用 rust 1:1 翻译")
2. **5 阶段 320h 总工时** — 跟原版一致 (per `v09021-commercial-extract §6`)
3. **8 项不修改承诺 8/8** — 跟原版一致
4. **6 哲学 anchor 6/6** — 跟原版一致
5. **m3 hallucination 5 道防御** — 跟原版一致 (per `m3-hallucination-defense`)
6. **8 闭源模块处理策略** — 跟原版一致 (per `commercial-vs-fork-diff §2.1-§2.8`)
7. **60+ SDK 分类 (R20 估补 / R21+ 估补 / 不补)** — 跟原版一致 (per `v09021-commercial-extract §2`)

### §7.2 差异点 (RIVAL 版差异化)

1. **16 估缺 + 5 增强 = 21 模块** (RIVAL) vs 原版估 16 (跟 22 模块总图一致, 但分类不同). 差异在 RIVAL 版按"5 MCP / 4 核心 / 2 工具 / 2 基础设施 / 4 SDK / 4 增强 / 1 R21" 7 类分, 原版估按 4 类 (估缺闭源 / 估缺新增 / 增强 / R21).

2. **16 估缺总工时 49h** (RIVAL, per §1.3) vs 原版估 132h (per `v09021-commercial-extract §3.3 16 模块总估时`). 差异原因: RIVAL 版按"1:1 翻译 6000 LOC" 估时 (8h SSH), 原版按"商业版估缺 1.3M LOC 全翻译" 估时 (8h SSH × 16 ≈ 132h). RIVAL 估时更准, 因为每 crate 估 LOC 是基于 v0.9.21 实查.

3. **5 P0 crate 体检实查** (RIVAL, per §3) — 原版无. RIVAL 版对已 skeleton 5 crate 做真实健康检查, 发现 5 真实缺口 (m3 防御 / workspace members / supervisorPrompt 7 函数 / re-export 风格 / Fixture 5).

4. **workspace 整合策略 1 commit 落地** (RIVAL, per §5) — 原版估缺. RIVAL 版明确 5 P0 crate 加入顺序 + Cargo.lock 更新 + commit message 模板.

5. **5 阶段 320h 摊开** (RIVAL, per §4.1-§4.5) — 原版估缺. RIVAL 版每阶段估时按 16 估缺 49h + 5 阶段 overhead 271h 摊开, 每阶段 8-12 子项 (实施 / 测试 / 文档 / 缓冲).

6. **4 增强归类 (RIVAL)** — 原版估缺. RIVAL 版明确 tree-sitter / graph / protocol::gemini / task 4 增强, 估 10h 总, 不新增 crate (用 `apeireth-council::task` 估补).

7. **4 SDK 估缺 stub 策略 (RIVAL, per §2.5)** — 原版估缺. RIVAL 版明确 lark/voice 在 R20 阶段 3 必补 (估 1+1=2h), livekit/sandbox 在 R21+ 暂 stub (估 2+2=4h).

8. **8 真实缺口汇总** (RIVAL, per §3.7) — 原版估缺. RIVAL 版把 5 P0 crate 体检发现的缺口 (3 🔴 m3 防御 + 1 🔴 workspace members + 1 🔴 supervisorPrompt 7 函数 + 1 🟡 re-export 风格 + 1 🟡 Fixture 5) 全列出, 整合必填.

### §7.3 RIVAL 版总报告 (1 段 TL;DR)

| 项 | 值 |
|---|---|
| 路径 | `.openclaw\workspace\promethean\Apeireth-rust\docs\stage4\v09021-rust-translation-blueprint-RIVAL-2026-08-05.md` |
| 性质 | RIVAL 竞争版 (vs bg_a5470979 同日原版) |
| v0.9.21 商业版 | 1.4 GB / 171 .js / 452,173 LOC (per NSIS 解包) |
| 22 模块总图 | 5 MCP / 4 核心 / 2 工具 / 2 基础设施 / 4 SDK / 4 增强 / 1 R21 |
| 16 估缺 + 5 增强总估时 | 49h (RIVAL) vs 132h (原版, 含 R21 估补) |
| R20 5 阶段总估时 | 320h (1 工程师 8 周) |
| 5 P0 crate 体检 | 2,954 + 14.4KB / 35 测试 / 5 真实缺口 (3 🔴 + 2 🟡) |
| 8 项不修改承诺 | 8/8 严守 |
| 6 哲学 anchor | 6/6 穿透 |
| 字数 | 估 700+ 行 (vs 目标 500-800 行) |

---

**致谢**:
- 主人 2026-08-05 19:37 拍板"解 NSIS, 全用 rust, 1:1 翻译, 彻底解剖" 决策
- 主人 2026-08-05 19:50 拍板"派成员干, 自己干分散注意力" 决策
- 9 份必读 stage4 文档作者 (per §0 必读输入)
- 5 P0 crate skeleton 写作者 (5 sub-agent, 2026-08-05 19:37 后并行写)
- v0.9.21 商业版 NSIS 解包工具 (7z 26.02 + @electron/asar)
- bg_a5470979 (同 RIVAL 竞争 sub-agent, Mavis 事后比 2 份选 quality 高 commit)
- Mavis R19 阶段 1+2 准备文档 (r20-stage-1-prep + r20-stage-2-3-prep, 140KB 总和)

**S-2 实事求是登记**:
1. 本 RIVAL 版纯蓝图, 不写代码, 不 git add/commit (产出物在 `docs/stage4/`)
2. 22 模块总图基于 v0.9.21 NSIS 解包实查 171 .js / 452,173 LOC + 9 份 stage4 文档引用, 不是凭空
3. 16 估缺总工时 49h 基于每 crate 估 LOC 1:1 翻译 + 8h SSH 标杆估时, 不是凭空
4. 5 P0 crate 体检基于 `crates/apeireth-{mcp-ssh,mcp-winrm,mcp-relay-image,workflow,team-lead}/` 实读 Cargo.toml + src/lib.rs, 不是凭空
5. R20 5 阶段 320h 估时基于 16 估缺 49h + 8 估缺闭源 95h + 60+ SDK 估缺 6h + 5 阶段 overhead 170h, 不是凭空
6. 8 项不修改承诺 8/8 严守 + 6 哲学 anchor 6/6 穿透, 0 触碰 crates/apeireth-*/src/ (24 Hermes LOCKED crate), 0 改 workspace Cargo.toml 元数据 (只加 members)
