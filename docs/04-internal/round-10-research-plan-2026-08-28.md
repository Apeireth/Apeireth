# Round 10 调研 plan + 主代理初步评估 (2026-08-28)

> **作者**: 主代理 Mavis (写于 Round 10 启动, sub-agent 调研跑中)
> **用途**: 记录 Round 10 调研 真账 + plan, 给接手工程师 / 未来 Mavis cycle 参考
> **关系**: 跟 `v2-reference-handbook-2026-08-28.md` (Round 9 一站式 reference) + 后续 sub-agent 真账 doc 互补

```
[Document-Meta]
Document:        docs/04-internal/round-10-research-plan-2026-08-28.md
Version:         1.0 (主代理 Mavis 写于 Round 10 启动)
Last-Modified:   2026-08-28
Status:          🟡 调研中 (sub-agent 调研跑中, 真账待 sub-agent 报告后 fill)
Author:          主代理 Mavis
```

---

## 0. Round 10 派活真账

**用户原话** (Round 10 启动): "调研区域啥情况，说人话" + "测试的垃圾删了，然后调研的记录和有价值的都留下来，同时，去把 `C:\Users\31683\Desktop\Youyou\重要参考项目，产品方向前辈同行，想法随记.txt` 这里的项目都调研一下，看看对我们的项目有什么用"

**主代理行动顺序**:
1. ✅ **删 .db 垃圾** (research/08-rust-substrate-current/data/*.db, 180 KB, R8 临时数据) — gitignore L21 `*.db` 隐含 ignore, 0 commit
2. ✅ **保留 research/source/ + _research_mem/** (全 gitignore 隐含 ignore, 0 commit, 调研 + 借鉴链)
3. 🟡 **派 sub-agent 调研 170 项目** (用户清单原文 + 评估对我们 Apeireth v2 的作用 + 末尾 2 条 "想法随记")
4. 🔲 **主代理亲验 sub-agent 报告** (per §6 + Round 4 教训, catch 任何误判)
5. 🔲 **写真账** (sub-agent + 主代理, 1 commit)
6. 🔲 **push** (Round 10 收尾)

---

## 1. Apeireth v2 工程现状 (主代理亲验, sub-agent 必读)

### 1.1 HEAD + 测试 + LOCKED

- `origin/main @ 8286dac8` (Round 9 完)
- 1739 tests passed / 0 failed / 12 ignored
- 0 clippy 警告 / 0 错误
- 16 crates / ~74k 行 active (不含 legacy/)
- 9 哲学锚 LOCKED, 0 触碰 LOCKED 5 项
- A 块 (OrganOrchestrator 完整化 5 stage) done

### 1.2 12 cognitive slot 真账

| Slot | Status | 路径 |
|---|---|---|
| `cognitive.memory_recall` | WIRED | runtime cognitive adapter / `TurnStart` / `Arc<dyn MemoryBackend>` |
| `cognitive.preference_recall` | WIRED | runtime cognitive adapter / `TurnStart` / `Arc<dyn PreferenceStore>` |
| `cognitive.judge` | WIRED, OFF by default | `AfterModelResponse` |
| `cognitive.council` | WIRED, OFF by default | `AfterModelResponse` |
| `cognitive.self_assessment` | WIRED, Judge-backed | `AfterTurn` |
| `cognitive.memory_writeback` | WIRED | `AfterTurn` |
| `cognitive.preference_learning` | DEFERRED → R20 派单 (调研就位, 2-3 周真实施) | — |
| `cognitive.critic` | DEFERRED INTO JUDGE → R21 派单 | — |
| `cognitive.reflection` | DEFERRED INTO SELF-ASSESSMENT → R22 派单 | — |
| `cognitive.planner` | NOT AN AGENT MODULE → R23 派单 (LLM Adapter 新设计) | — |
| `cognitive.orchestrator` | NOT AN AGENT MODULE → R24 派单 | — |
| `cognitive.perception` | NOT AN AGENT MODULE → R14 真 modality | — |

位置: `docs/04-internal/cognitive-module-wiring.md` (110 行)

### 1.3 LOCKED 5 项

| LOCKED 项 | 位置 |
|---|---|
| 9 哲学锚本体 | `crates/foundation/core/src/eight_anchors.rs:58-79` |
| 13 键 | `crates/foundation/core/src/philosophy.rs:142` `RUNTIME_ENFORCED = false` |
| 3 项不可变脊柱 | `crates/foundation/core/src/onion.rs:249` |
| workspace.version | `Cargo.toml:44` `"1.2.0"` |
| R11 baseline 3 值 | legacy reference (`legacy/donor/apeireth-asi/tests/integration_r_measure.rs:42-44`) |

### 1.4 Apeireth v2 已有借鉴链

| 位置 | 用途 |
|---|---|
| `legacy/donor/` (~13 v1 仓库) | 12 slot cognitive module + 9 organ 1:1 翻译源 |
| `research/source/` (~36 真开源借鉴) | tokio / wasmtime / qdrant / sled / hermes-agent-rs / MetaGPT / openclaw / LangGraph / CrewAI / Claude Code 等 |
| `_research_mem/` (24 子目录) | 子代理调研真账 + Apeireth 旧 fork + wave2-wave7 shots |

---

## 2. 用户清单 170+ 项目 (主代理初步评估)

### 2.1 清单分 10 类 (per 用户原文 L4-L272)

| 类别 | 项目数 | 主类别 |
|---|---|---|
| 🤖 AI Agent 框架与 Harness | ~37 | DeepSeek Harness / LangGraph / CrewAI / Hermes Agent / Pydantic AI / CopilotKit / Cua / Serena / Computer Use 等 |
| 🛠️ 开发工具与 CLI 增强 | ~38 | Crawl4AI / Playwright / Serena / OpenObserve / Carbonyl / codebase-memory-mcp / exo-explore / LangChain OpenWiki 等 |
| 💳 金融/量化/财务 | ~28 | TradingAgents / Lean / vnpy / Bigcapital / ERPNext / Pi Mono / Composio / Dexter / Kronos 等 |
| 🤖 AI 伴侣 / VTuber / 桌宠 | ~22 | N.E.K.O / AIRI / Open-LLM-VTuber / SillyTavern / OpenClaw / Warashi / Firefly / DesktopFriends 等 |
| 🎤 AI 语音/TTS | 6 | GPT-SoVITS / Genie-TTS / faster-whisper / FireRedTTS3 / HunyuanOCR / serena |
| 🔐 安全与隐私 | ~14 | Vaultwarden / Bitwarden / Sherlock / Maigret / Sandboxie / VulnClaw / DeepSec / opencode-vibeguard 等 |
| 📋 资源汇总 | ~14 | free-for-dev / Ant Design / Taipy / AirLLM / CL4R1T4S / Ponytail / old-coder / PUA / live_coding / OpenMythos / Obsidian LLM 等 |
| 🎵 自建音乐流媒体 | 5 | Navidrome / Jellyfin / Airsonic-Advanced / mStream / Gonic |
| 💬 即时通讯/机器人 | 2 | NapCatQQ / Ollama |
| 🗺️ 地理信息 | 1 | GeoLibre |
| **总计** | **170+** | (用户 L276: "170+ 个项目") |

### 2.2 评估维度 (per O-6 总体最优 + sub-agent 写真账)

| 维度 | HIGH | MED | LOW | NONE |
|---|---|---|---|---|
| **定义** | 直接可借鉴 (代码 / 设计 / 思路) | 同领域参考 (了解趋势, 不直接借鉴) | 远领域 (知道就行) | 完全无关 |
| **Apeireth v2 路径** | AI Agent framework (16 crates, 9 organ, 12 slot, governance P0 hook 装) |
| **直接借鉴** | 代码可移植 / 借鉴边界 / 0 触碰 LOCKED | 趋势参考 / 设计思路 |
| **不直接借鉴** | 太远或风险高 | 太远或不合规 |

### 2.3 主代理预判 (待 sub-agent 真账修订)

#### 🤖 AI Agent 框架与 Harness (37 个) — **HIGH**

- **真开源 + Apeireth v2 边界内**: LangGraph / CrewAI / LangChain OpenWiki / Pydantic AI / CopilotKit / Cua / Computer Use / Serena / Hermes Agent / DeepSeek Harness / LoopX / OpenClaw (我们已有)
- **直接借鉴候选 (TOP 5)**:
  - **DeepSeek Harness** (官方 Agent Harness, 我们的 ancestor 之一) — 看 R8+ R9+ 集成怎么写
  - **LangGraph** (状态多 Agent 应用框架) — 我们的 cognitive module wiring 跟 LangGraph state machine 模式对比
  - **CrewAI** (多 Agent 编排框架) — 我们的 OrganOrchestrator 跟 CrewAI crew/task 对比
  - **Cua** (开源 Computer Use 2.0 驱动) — 我们 D 块 RC-7 Perception 工具调用借鉴
  - **Serena** (MCP 编码工具包) — 我们的 tool pipeline (per `crates/capabilities/tools/`) 借鉴
- **P0 立即** (TOP 5 clone + 调研)
- **P1 排上** (其余 32 个, MED 趋势参考)

#### 🛠️ 开发工具与 CLI (38 个) — **MED**

- **真开源 + 直接可借鉴**: Playwright MCP (跨浏览器自动化) / Crawl4AI (LLM 友好爬虫) / Serena (MCP 工具包) / Carbonyl (终端 Chromium) / codebase-memory-mcp (代码智能 MCP)
- **趋势参考**: free-for-dev / OpenObserve (可观测性) / Ant Design / Taipy / AirLLM (单卡大模型)
- **P2 后续** (按需 clone)

#### 💳 金融/量化 (28 个) — **NONE**

- **完全无关** — Apeireth v2 是 AI Agent 框架, 不是金融 / 量化平台
- **0 借鉴价值** — skip

#### 🤖 AI 伴侣 / VTuber / 桌宠 (22 个) — **LOW**

- **路径不同** — 我们是 AI Agent framework (后台 orchestrator), 它们是 Companion UI (前端 Live2D / VTuber)
- **有限借鉴**:
  - **Open-LLM-VTuber** (语音交互 + 视觉感知 + 工具调用 + Live2D 形象) — 跟 B 块 frontend (Svelte 5 + Live2D) 路径交叉
  - **Warashi** (长期记忆 + 主动聊天 + 睡眠模式) — 跟我们 cognitive.self_assessment + memory_writeback 模式有交集
  - **Mio** (Windows 本地优先 + 屏幕感知 + QQ + Live2D) — Windows 桌面部署借鉴
- **P2 后续** (B 块 frontend 真实施时考虑 UI 借鉴)

#### 🎤 AI 语音/TTS (6 个) — **MED**

- **D 块 RC-7 真 modality 借鉴候选**:
  - **faster-whisper** (语音转写, CTranslate2 + HF tokenizers) — 我们 R14 spec 借 `WhisperHttpBackend` 真接
  - **GPT-SoVITS / Genie-TTS** (TTS + 角色音色) — 我们 R14 spec TTS 借 (如果前端需要 TTS)
- **P1 排上** (R14 spec 实施时一并调研)

#### 🔐 安全与隐私 (14 个) — **MED-LOW**

- **有限借鉴**:
  - **Vaultwarden** (Rust Bitwarden 兼容服务端) — 我们 cargo keyring + governance credential 借鉴
  - **Sandboxie** (Windows 沙箱) — 我们 process isolation 借鉴
  - **opencode-vibeguard** (敏感信息脱敏插件) — 我们的 governance hook 脱敏借鉴
- **P2 后续** (需要时 clone)

#### 📋 资源汇总 (14 个) — **VARIES**

- **直接相关** (主代理 / 借鉴知识库):
  - **free-for-dev** (免费开发者服务清单) — 接手工程师参考
  - **Ant Design** (React UI 库) — 我们 frontend 选型 (虽然 B 块用 Svelte, 备选 React)
  - **Taipy** (Python 数据/AI Web 应用) — B 块 frontend 备选
  - **AirLLM** (单卡 4GB 运行 70B 大模型) — 部署参考
  - **Obsidian LLM** (Obsidian 作为 AI 第二大脑) — 长期记忆架构借鉴
  - **OpenMythos** (开源 Mythos 重建) — 知识库架构借鉴
- **AI 提示词 / Skill 合集** (CL4R1T4S / Ponytail / old-coder / PUA / andrej-karpathy-skills / live_coding / karpathy/AutoResearch) — 借鉴工作流 / Sub-agent prompt 工程
- **P1 排上** (B 块 frontend + C 块 cognitive module 实施时调研)

#### 🎵 自建音乐流媒体 (5 个) — **NONE**

- **完全无关** — Apeireth v2 不是媒体服务器
- **0 借鉴价值** — skip

#### 💬 即时通讯 (2 个) — **MED**

- **NapCatQQ** (QQ + OneBot 通信协议) — 用户可能想加 QQ interface (跟 v1 companion 类似)
- **Ollama** (本地 LLM 运行环境) — 我们 v2 已用 3 家 provider (MiniMax / Anthropic / OpenAI-compatible), Ollama 可加第 4 家 (本地 LLM)
- **P2 后续** (B 块 frontend + R10 OrganKind variant 决策时调研)

#### 🗺️ 地理信息 (1 个) — **NONE**

- **完全无关** — skip

### 2.4 主代理初评汇总

| 类别 | 项目数 | HIGH | MED | LOW | NONE |
|---|---|---|---|---|---|
| AI Agent 框架与 Harness | 37 | ~5 | ~32 | 0 | 0 |
| 开发工具与 CLI | 38 | 0 | ~5 | ~28 | ~5 |
| 金融/量化 | 28 | 0 | 0 | 0 | 28 |
| AI 伴侣 / VTuber | 22 | 0 | 0 | ~3 | ~19 |
| AI 语音/TTS | 6 | 0 | ~3 | ~3 | 0 |
| 安全与隐私 | 14 | 0 | ~3 | ~5 | ~6 |
| 资源汇总 | 14 | 0 | ~6 | ~5 | ~3 |
| 音乐流媒体 | 5 | 0 | 0 | 0 | 5 |
| 即时通讯 | 2 | 0 | ~2 | 0 | 0 |
| 地理信息 | 1 | 0 | 0 | 0 | 1 |
| **总计** | **170+** | **~5** | **~51** | **~44** | **~69** |

---

## 3. 用户末尾 2 条 "想法随记" 评估

### 3.1 "Apeireth 可以便携安装进 U 盘" (用户 L277)

**评估**: P1 排上 (post-release), 估时 1 周

**实施路径**:
- Cargo workspace portable binary (`cargo build --release --target x86_64-unknown-linux-musl` 或 windows-msvc)
- strip + UPX 压缩 (减少 binary size)
- U 盘启动 script (Linux systemd / Windows service)
- portable data dir (`./data/` 而非 `/var/lib/`)
- README + 使用说明 (跨平台 portable)

**0 触碰 LOCKED**: 0 改 nine_anchors.rs / philosophy.rs / onion.rs / Cargo.toml version / R11 baseline. 新增 `scripts/portable.sh` + `docs/portable-guide.md`.

**对接现状**:
- 16 crates Cargo workspace 兼容 portable (cargo metadata)
- v2.0 release 后用户可用 U 盘随身带
- 估时 1 周 (1 commit, 文档 + script + 测试)

### 3.2 "Apeireth 做自己刷视频的模块" (用户 L278)

**评估**: P2 后续, 估时 2-4 周

**实施路径**:
- D 块 RC-7 Perception 真 modality 拓展 (Video = 新 modality, 已有 Voice + Vision)
- 视频 backend impl (类似 WhisperHttpBackend / XcapVisionBackend):
  - **YouTube/Bilibili 视频感知** — API 集成 (官方 API + 反爬 awareness)
  - **TikTok / 抖音 视频感知** — API 集成
  - **本地视频文件感知** — ffmpeg + frame extraction
- 跟 D 块 R14 spec 借 modality 设计 (`crates/foundation/plugin/src/perception_backend.rs`)
- 9 organ 流式集成 (frontend video player 跟 cognitive module 流)

**0 触碰 LOCKED**: 0 改 nine_anchors.rs / philosophy.rs / onion.rs / Cargo.toml version / R11 baseline. 新增 modality impl + frontend UI.

**对接现状**:
- D 块 RC-7 真 modality 已规划 (估时 2-3 周, 需硬件)
- 视频 modality 拓展 = D 块扩展 (P2)
- 估时 2-4 周 (1 modality 后端 + 1 frontend 集成 + 测试)

---

## 4. 主代理决策建议 + 派单顺序 (待 sub-agent 真账修订)

### 4.1 P0 (1 周内)

- **TOP 5 HIGH 借鉴** (sub-agent 写真账确认后): DeepSeek Harness / LangGraph / CrewAI / Cua / Serena
  - 派 sub-agent 真调研 (每个 1-2h, 写真账 ≤200 行)
  - 主代理亲验 (per §6, catch 任何误判)
  - 1 commit per 项目, 写借鉴笔记到 docs/04-internal/borrowed-from-X-2026-08-28.md

### 4.2 P1 (1 月内)

- **MED 趋势参考** (32 + 5 + 3 + 6 = ~46 个): 调研真账, 不真借鉴, 知道就好
- **GPT-SoVITS / faster-whisper** (D 块 RC-7 R14 spec 实施时一并调研)
- **资源汇总** (6 个直接相关): free-for-dev / Ant Design / Taipy / AirLLM / Obsidian LLM / OpenMythos
- **用户想法 1** (便携 U 盘): post-release, P1

### 4.3 P2 (后续)

- **用户想法 2** (刷视频): D 块 RC-7 扩展, P2
- **AI 伴侣 UI 借鉴** (3 个): B 块 frontend 真实施时调研
- **NapCatQQ + Ollama**: interface 扩展 + 第 4 家 provider, P2
- **金融/量化 + 音乐流媒体 + 地理信息** (35 个): skip

---

## 5. sub-agent 调研 brief (已在跑, ebdd7a77)

### 5.1 Brief

- 任务: 逐类分析 170 项目对我们项目 (Apeireth v2) 的实际作用 + 可借鉴度
- 必读: `C:\Users\31683\Desktop\Youyou\重要参考项目，产品方向前辈同行，想法随记.txt` (284 行) + `docs/04-internal/v2-reference-handbook-2026-08-28.md` (613 行) + Round 9 调研 6 真账 doc
- 输出: `docs/04-internal/youyou-list-research-2026-08-28.md` (≤ 250 行)

### 5.2 sub-agent 调研 6 段结构

1. 分类总览 (10 类 + 相关度)
2. 高价值项目 TOP 10 (HIGH, 借鉴点 + 优先级)
3. 同领域项目 (MED, 趋势参考)
4. 低价值项目 (LOW + NONE, 列名)
5. 用户末尾 2 条 "想法随记" 评估
6. 主代理决策建议 + 派单顺序 (P0/P1/P2)

### 5.3 主代理亲验时 catch (per Round 4 教训)

- 验证 sub-agent TOP 10 是否真的 HIGH (跟主代理预判对比)
- catch "调研方法偏差" (如 R9 spec flag 误判)
- 0 触碰 LOCKED 验证 (sub-agent 不写真账以外的 file)

---

## 6. 留 backlog (per §11 风险 + 0 装诚实标)

| # | 项 | 估时 | 阻塞 |
|---|---|---|---|
| 1 | sub-agent 报告回来 → 主代理亲验 → 写真账真账 → commit → push | 1h | sub-agent 4h |
| 2 | **TOP 5 HIGH 借鉴** (DeepSeek Harness / LangGraph / CrewAI / Cua / Serena) — 派 5 sub-agent 真调研 | 1-2 周 | 0 |
| 3 | **D 块 RC-7 真 modality 真实施** (per R14 spec) | 2-3 周 | 硬件 + 真实施启动 |
| 4 | **B 块 frontend 真实施** (派 sub-agent A) | 1-2 周 | §8.2 决策冻结 |
| 5 | **C 块 preference_learning 真实施** (派 sub-agent R20) | 2-3 周 | R10 OrganKind 决策 |
| 6 | **用户想法 1** (U 盘便携) P1 | 1 周 | post-release |
| 7 | **用户想法 2** (刷视频) P2 | 2-4 周 | D 块 RC-7 之后 |
| 8 | **NapCatQQ + Ollama** (interface + 第 4 家 provider) P2 | 1-2 周 | 0 |

---

_Mavis 写于 2026-08-28 Round 10 启动, sub-agent 调研跑中, 真账待填._
