# Apeireth — AGI 操作系统 (Rust 重写, VCP 全栈)

> **R128 (2026-08-12)**: workspace 收敛 94→55 active crate, 18 archived/frozen. 24 LOCKED 入口签名降级, 仅保 3 项不可变脊柱 (Self-Disable / L0 HA / 13 键 verdict cache). minimax (MiniMax) 4 协议真端到端跑通. `cargo check --workspace` 0 errors.

---

## 1 分钟上手

### 安装 (Windows / Linux / macOS)

```bash
# 1. clone
git clone https://github.com/apeireth/apeireth-rust.git
cd apeireth-rust

# 2. 装主入口 (TUI)
cargo install --path crates/apeireth-tui

# 3. 装 CLI
cargo install --path crates/apeireth-cli

# 4. 配 minimax API key
export APEIRETH_API_KEY="<your-minimax-key>"  # Linux/macOS
$env:APEIRETH_API_KEY = "<your-minimax-key>"  # PowerShell

# 5. 跑
apeireth-tui                                # TUI 终端界面 (5 页面)
apeireth --version                          # CLI 子系统入口
cargo run -p apeireth-api --example serve   # HTTP server (默认 :8080)
```

### apikey 来源

- 默认: `.openclaw\apikey.txt` (per R32-3-1 DEFAULT_APIKEY_PATHS)
- 也可设 `APEIRETH_API_KEY` 环境变量覆盖
- 协议支持: Anthropic Messages (`x-api-key`) + OpenAI Chat Completions (`Bearer`) + OpenAI Responses + Gemini — minimax 同 key 通用

---

## 架构核心 (3 层)

```
┌─────────────────────────────────────────────────────────┐
│ L3 入口:  TUI (ratatui 5 页面) | CLI (3 组 15 命令) | HTTP (axum) │
├─────────────────────────────────────────────────────────┤
│ L2 战区:  cognition | council | perception | memory | tools | pipeline │
├─────────────────────────────────────────────────────────┤
│ L1 脊柱:  apeireth-core (13 键 verdict cache) | apeireth-sovereignty (Self-Disable) | L0 HA │
└─────────────────────────────────────────────────────────┘
```

### 1.1 哲学 8 锚

| 锚 | 语义 | 实施位置 |
|---|---|---|
| S-1 北极星 | 人类级 AI 助手 | 全部 |
| S-2 实事求是 | 实际能跑的事 | `cargo check --workspace` 0 errors, 真接 minimax |
| S-3 流程自化 | 流程工程化 | CI 17 workflows + cargo bench + eval-live |
| O-1 安全优先 | Self-Disable 物理熔断 | `apeireth-sovereignty/src/self_disable.rs` (4 项自动扫描 + 三级响应) |
| O-2 走在前人肩上 | 借鉴 8/11 开源 | clap / hyper / PyO3 / kani / langgraph / superpowers / Guardrails / LiteLLM / opencode |
| O-3 干到底 | 实施到底 | 74 active crate 全部实质化, 不假装 |
| O-4 任何人都能接手 | 文档 + 索引 | 51/51 crate 有 README, 顶层 README 重写 |
| O-5 不假装 | 真接非 mock | minimax 4 协议真端到端, SQLite 真持久化 drop+reopen |

### 1.2 双洋葱架构 (PrincipleOnion + PermissionOnion)

- **PrincipleOnion** (原则洋葱): 5 切片 E/S/A/M/O — 意义约束
- **PermissionOnion** (权限洋葱): 6 切片 L0-L5 — 配额曲线 (非 boolean)
- **嵌入关系**: 原则嵌入权限, 不是两把独立锁
- 编译期保证, 不可绕过

### 1.3 Self-Disable (不可逆物理熔断)

`apeireth-sovereignty/src/self_disable.rs`:
- 4 项自动扫描: 触碰 L0 HA / 重组洋葱 / 绕过 HumanAuthority / 假装不可观测
- 三级响应: CheckResult → AutoScanResult → KillSwitch
- 物理多签恢复: `apeireth-sovereignty/src/physical_multisig.rs`
- 离线模式: `ha_modes.rs` (冰冻期 + 安静模式)
- 编译器锁死, agent 自己也无法绕过

---

## minimax 真端到端 (R128 验证)

| 协议 | 端点 | 状态 | 耗时 | Tokens |
|---|---|---|---|---|
| OpenAI Chat | `v1/chat/completions` | ✅ 3 round | 3.8s/2.4s/2.6s | 267/392/390 |
| OpenAI Responses | `v1/responses` | ✅ | 1.74s | 228 |
| Anthropic Messages | `anthropic/v1/messages` | ✅ | 3.33s | 126 |
| **minimax + memory** | anthropic + SQLite | ✅ **真端到端** | 1.59s + drop+reopen | 89 |

跑法:
```bash
$env:APEIRETH_API_KEY = (Get-Content ".openclaw\apikey.txt" -Raw).Trim()
cargo run -p apeireth-api --example openai_chat          # Chat Completions 3 round
cargo run -p apeireth-api --example openai_responses     # Responses API
cargo run -p apeireth-api --example anthropic_hello      # Anthropic 协议
cargo run -p apeireth-integration-e2e --example minimax_memory_roundtrip  # + 真持久化
```

详细报告: [`reports/minimax-end-to-end-r128-2026-08-12.md`](reports/minimax-end-to-end-r128-2026-08-12.md)

---

## 9 organ 监控 (TUI / page 2)

| organ | 功能 | 实现 crate |
|---|---|---|
| body | 物理动作执行 | `apeireth-tools` + `apeireth-tool-runtime` |
| brain | 推理 + 决策 | `apeireth-cognition` + `apeireth-council` |
| ear | 感知输入 | `apeireth-perception` |
| eye | 视觉 + 图像 | (sub of perception) |
| hand | 操作 (Bash / Edit / Write) | `apeireth-tools` |
| heart | 情感 + 反思 | `apeireth-life-force` |
| memory | 长期记忆 | `apeireth-memory` |
| mind | 元认知 | `apeireth-consciousness` |
| voice | 表达 (TTS / 输出) | (CLI / TUI / HTTP) |

---

## workspace 概况

| 指标 | 值 |
|---|---|
| workspace.version | 1.2.0 (semver 严守) |
| active crate | 74 (R128 收敛, 原 94) |
| archived/frozen | 18 (`crates/_frozen/` 13 + `crates/_archived/` 5) |
| tests | 4921 passed / 88 suites / 0 failed |
| 24 LOCKED | 入口签名冻结降级为历史 (R128), 仅保 3 项不可变脊柱 |
| 13 键 verdict cache | 12 + PHL-07 = 13, 编译期 hardcode |
| 8 哲学锚 | S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5 |
| 6 重守门 | v7 (整合 #5 R125 B4 升) |
| 5 战区 | terminal-coding-agent / llm-gateway / multi-agent / long-term-memory / tool-protocol |
| Rust | 1.80 stable |
| CI | 17 GitHub Actions workflows |

---

## 5 战区

| 战区 | 主力 crate | 状态 |
|---|---|---|
| 终端 Coding Agent | `apeireth-tui` + `apeireth-cli` | ✅ 真接 minimax |
| LLM 网关 | `apeireth-api` (4 协议) | ✅ 真接 minimax |
| Multi-Agent | `apeireth-council` (7 advisor) | ✅ 真接 minimax |
| 长期记忆 | `apeireth-memory` (SQLite + 3 层) | ✅ 真接 minimax |
| 工具协议 | `apeireth-tool-{runtime,registry,approval,tools}` + `apeireth-mcp*` | ✅ 真接 minimax |

---

## 文档索引

- 规范: [`docs/conventions/README.md`](docs/conventions/README.md) (16 子规范)
- 路线图: [`docs/pages-source/roadmap.md`](docs/pages-source/roadmap.md)
- 哲学: [`docs/conventions/09-anchor.md`](docs/conventions/09-anchor.md)
- 锁定: [`docs/conventions/10-locked.md`](docs/conventions/10-locked.md)
- 合并策略: [`docs/conventions/16-crate-merge-policy.md`](docs/conventions/16-crate-merge-policy.md)
- 端到端: [`reports/minimax-end-to-end-r128-2026-08-12.md`](reports/minimax-end-to-end-r128-2026-08-12.md)
- 决策链: [`reports/decision-*.md`](reports/) (37 个决策, decision-22 ~ decision-130+)

---

## 借鉴 (8/11 致谢)

| 借鉴源 | License | 借鉴位置 | 决策 |
|---|---|---|---|
| clap-rs/clap 4.6.6 | Apache-2.0 + MIT | CLI derive | decision-125-2 |
| hyperium/hyper 0.1.20 | MIT | HTTP client | decision-125-3 |
| modelcontextprotocol/servers | MIT → Apache-2.0 | MCP | decision-125-4 |
| PyO3/PyO3 0.29.2 | Apache-2.0 + MIT | pybridge | decision-125-9 |
| model-checking/kani 0.67.0 | MIT + Apache-2.0 | formal proofs | decision-125-10 |
| langchain-ai/langgraph | MIT | council orchestration | decision-125-13 |
| obra/superpowers 6.2.0 | MIT | library | decision-125-14 |
| NVIDIA/NeMo-Guardrails | Apache-2.0 | m3 defense | decision-125 (P2) |

完整 11/11 借鉴 (含 LiteLLM / opencode) 见 [`OSS_NOTICE.md`](OSS_NOTICE.md).

---

## License

Apache-2.0 — see [`LICENSE`](LICENSE).
Attribution: [`NOTICE`](NOTICE) + [`OSS_NOTICE.md`](OSS_NOTICE.md) + [`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md).

---

_R128 (2026-08-12) 重写, 主人拍板 "推进落地, 短板弱项都补上, 进入能生产的环节". 详见 [`reports/minimax-end-to-end-r128-2026-08-12.md`](reports/minimax-end-to-end-r128-2026-08-12.md)._