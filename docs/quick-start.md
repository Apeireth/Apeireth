# Apeireth 快速开始（quick-start, 2026-08-16）

> **给谁看**: 第一次把 Apeireth 跑起来的人。
> **目标**: 10 分钟内从源码到一个真在运行的基地（companion）。
> **0 假装**: 本文所有命令/env 均摘自真实代码与 `docs/maintenance-guide.md` §四 env 清单；跑不通的如实标注。
> 相关文档：机制详解见 [user-manual.md](user-manual.md)；三档装配见 [capability-packs.md](capability-packs.md)；社区插件开发见 [plugin-authoring-guide.md](plugin-authoring-guide.md)。

---

## 1. 前置条件

| 项 | 要求 | 依据 |
|---|---|---|
| Rust | **1.80+**（workspace `rust-version`），edition 2021 | `Cargo.toml:224-226` |
| 平台 | Windows + PowerShell 为首选环境（中文日志 GBK 乱码时用 `-Encoding Default` 读）；git-bash 亦可 | team-work-doc §2.2 |
| LLM API key | MiniMax（默认端点 `https://api.minimaxi.com`，模型 `MiniMax-M3`）：设 `APEIRETH_API_KEY`，或放 `apikey-ultra.txt` | `production_daemon.rs:35-36` + `load_key()` |
| 磁盘 | 记忆库 SQLite 默认在 `%APPDATA%\apeireth\memory.sqlite`（可用 env 改路径） | `companion_daemon.rs` env 头 |

## 2. 获取与构建

```powershell
git clone <repo-url> Apeireth-rust
cd Apeireth-rust
cargo build            # 首次构建较慢（workspace 较大），之后增量
```

> **测试提示**（团队纪律，per team-work-doc §2.2）：单 crate 测试用 `cargo test -p <crate> -j 4`（降并行防页文件耗尽）；`cargo test --workspace` 全量由集成守门员跑，个人不要跑。

## 3. 最小运行：companion_serve（主入口）

伙伴端点——**OpenAI 兼容 API**，任何兼容前端接上就拥有 Apeireth 全部能力（来源：`examples/companion_serve.rs` 模块头 v4）。

```powershell
$env:APEIRETH_API_KEY = "<你的 MiniMax key>"
cargo run -p apeireth-companion --example companion_serve
```

默认监听 **8090**（`APEIRETH_PORT`/`PORT` 可改）。关键 env（完整清单见 maintenance-guide §四「companion_serve 环境变量」）：

| env | 作用 | 默认 |
|---|---|---|
| `APEIRETH_API_KEY` | MiniMax key（必） | — |
| `APEIRETH_MASTER_TOKEN` | 主人批准用 | — |
| `APEIRETH_PORT` / `PORT` | 监听端口 | 8090 |
| `APEIRETH_DEEP_RECALL=1` | 推理召回 | 关 |
| `APEIRETH_MAX_TOKENS` | 输出上限 | 8192 |
| `APEIRETH_EXTRACT_INTERVAL_SECONDS` | 记忆提炼节流 | 600 |
| `APEIRETH_DREAM_QUIET_SECONDS` | 做梦安静期 | 6h |
| `APEIRETH_REFLECT_PERIOD_HOURS` | 反思周期 | 24h |
| `APEIRETH_GRANT` | 启动即授权（格式 `"工具:小时"`） | — |
| `APEIRETH_LARK_APP_ID/SECRET/RECEIVE_ID` | 飞书离线送达（可选） | — |
| `APEIRETH_TELEGRAM_BOT_TOKEN/CHAT_ID` | Telegram 离线送达（可选） | — |
| `APEIRETH_SEED_MEMORY` | 种子记忆（演示） | — |

验证：向 `POST /v1/chat/completions` 发一条消息，收到 OpenAI 兼容响应即成功（前端接入细节见 `docs/frontend-guide.md`）。

## 4. 常驻陪伴：companion_daemon

主动问候 + 真 SQLite 记忆 + 通道送达的常驻进程（来源：`examples/companion_daemon.rs` env 头，原文）：

```powershell
$env:APEIRETH_API_KEY = "<key>"
$env:APEIRETH_SINK = "console"   # 或 lark（需 APEIRETH_LARK_* 凭据）
cargo run -p apeireth-companion --example companion_daemon
```

| env | 作用 | 默认 |
|---|---|---|
| `APEIRETH_TICK_SECS` | 心跳间隔秒 | 60 |
| `APEIRETH_MAX_TICKS` | 跑 N 轮后退出 | 无限 |
| `APEIRETH_MEMORY_PATH` | 记忆库 SQLite 路径 | `%APPDATA%\apeireth\memory.sqlite` |
| `APEIRETH_SUBJECT` | 记忆检索 continuity/session id | 回落 `APEIRETH_CONTINUITY_ID` 或 `companion-main` |
| `APEIRETH_MIN_LLM_INTERVAL_SECS` | 两次主动（LLM 渲染）最短间隔 | 60 |
| `APEIRETH_SINK` | 送达通道：`console` / `lark` | console |
| `APEIRETH_DREAM=1` | 开启做梦（6h 无互动 → 合并 + LLM 摘要） | 关 |
| `APEIRETH_SEED_DEMO=1` | 预填 7 天 demo 作息种子（诚实标注为演示数据） | 关 |

stdin 交互：任意输入 = 一次用户交互；`r` = 回应上次主动；`quit` = 退出。

**统一锚点**：`APEIRETH_CONTINUITY_ID`（默认 `companion-main`）——记忆/日志/目标/反思共用（maintenance-guide §四）。

## 5. 全机制验收：production_daemon

一个进程串起全部已做机制（宪法评审真 LLM + 执行体隔离 + spill + 会话日志 + 断点续传 + Goal + 做梦/反思 + 每日摘要）。**需要 `apikey-ultra.txt` 或 `APEIRETH_API_KEY`**：

```powershell
cargo run -p apeireth-companion --example production_daemon
```

（来源：`examples/production_daemon.rs:1-9` 模块头。）

其他常用入口（摘自 maintenance-guide §四示例清单）：

| 示例 | 用途 | 命令模式 |
|---|---|---|
| `release_acceptance` | AI 自己长能力端到端（提案→评审→激活→干活） | `cargo run -p apeireth-companion --example release_acceptance` |
| `multi_turn_agent` | 多轮 function calling + 断点续传（`--crash-after` / `--resume`） | `cargo run -p apeireth-companion --example multi_turn_agent` |
| `virtual_time_simulation` | 时间机制模拟验收（23 项，虚拟时钟，无需真等待） | `cargo run -p apeireth-companion --example virtual_time_simulation` |

## 6. 常见坑（诚实标注）

1. **中文日志乱码**（Windows PowerShell GBK）：用 `-Encoding Default` 读日志文件；日志已用 ASCII 前缀（`[llm]`/`[daemon]`/`[extract]`）方便 grep（team-work-doc §2.2）。
2. **测试内存/页文件**：一律 `-j 4`，别开满并行。
3. **GitHub 直连被墙**：依赖不靠 GitHub 下载；确需克隆时用 gh_accel 插件（`docs/ref-gh-accel.md`）。
4. **没有 key 能跑什么**：`virtual_time_simulation`（虚拟时钟，无 LLM）；其余示例需真 MiniMax key。

## 7. 下一步

- 想了解基地各机制（记忆生命周期/审批/送达/安全）→ [user-manual.md](user-manual.md)
- 想按需求裁剪装配（base/能力包/套件三档）→ [capability-packs.md](capability-packs.md)
- 想给社区写插件 → [plugin-authoring-guide.md](plugin-authoring-guide.md)
