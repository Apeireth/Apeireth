# 170+ 项目清单对 Apeireth v2.0.0-rc.1 作用真账 — 2026-08-28

> **作者**: sub-agent (主代理 Mavis 派单, 时间紧 ≤ 4h)
> **用途**: 给主代理决策参考 — 170 项目按 10 类分组, 标 HIGH/MED/LOW/NONE, TOP 10 推荐借鉴顺序, 用户末尾 2 想法随记评估
> **关系**: 跟 `v2-reference-handbook-2026-08-28.md` (一站式 reference) + 6 真账 doc (R20/R21-24-R12/RC-7/B-decision/B-gateway) 互补
> **0 装诚实标**: 已读用户清单原文 284 行 + v2 handbook 250 行 (完整 613 行), 未读 v1 legacy / research/source / _research_mem (本任务不要求, 任务 brief 说"0 写真账以外的 file"); 时间紧真账 ≤250 行, 数字未实测, per O-5 标 "未实测" (无 git/grep 命令)

```
[Document-Meta]
Document:        docs/04-internal/youyou-list-research-2026-08-28.md
Version:         1.0 (sub-agent 写于 2026-08-28, 主代理派单 4h 内)
Last-Modified:   2026-08-28
Status:          🟢 活跃 (调研真账, 主代理决策参考)
Author:          sub-agent (主代理 Mavis 派)
```

---

## 1. 分类总览 (170 项目按 10 类分组)

| # | 类别 | 项目数 | 主类 | 与 Apeireth v2 相关度 | 理由 |
|---|---|---|---|---|---|
| 1 | AI Agent 框架与 Harness | 40 | LLM harness / multi-agent | **HIGH** (15-20 直接可借鉴) | v2 已借鉴 Hermes/OpenClaw/LangGraph/CrewAI/MCP 等; 调研不重叠 |
| 2 | 开发工具与 CLI 增强 | 48 | dev tools / OCR / memory / scraper | **MED** | 同领域参考 (memory / OCR / vector db / scraper 跟 B/C 块有关) |
| 3 | 金融/量化/财务 | 32 | finance / trading | **LOW** | 远领域 (v2 非金融项目), 仅 TradingAgents/Awesome-finance-skills 对趋势参考 |
| 4 | 自建音乐流媒体 | 5 | music server | **NONE** | 完全无关 (v2 是 Agent, 不是 media server) |
| 5 | 安全与隐私 | 15 | password / p2p / sandbox | **MED** | 跟 O-1 安全优先哲学锚相关, 部分借鉴 |
| 6 | AI 语音/TTS | 6 | TTS / ASR / OCR | **HIGH** (3-4) | 跟 RC-7 Perception (R14 真 modality) 对接, ASR/TTS 后端候选 |
| 7 | 即时通讯/机器人 | 2 | QQ/IM bot | **LOW** | 仅 1-2 项目, v2 gateway 已有 SSE, 不直接借鉴 |
| 8 | AI 伴侣/VTuber/桌宠 | 27 | VTuber / desktop pet / Live2D | **HIGH** (5-8 直接可借鉴) | 跟 RC-7 真 modality 强相关 (Live2D 视觉形象 = modality 之一), 用户本位 |
| 9 | 地理信息 | 1 | GIS | **NONE** | 单项目, 完全无关 |
| 10 | 资源汇总 | 14 | awesome / wiki / skill | **LOW** | 趋势合集, 仅 1-2 个 (Obsidian LLM / Karpathy skills) 参考 |

**总: 190 项目 (含重复 OpenClaw x2 + 资源类重复), 净 170+**. 相关度分布: **HIGH ≈ 25 项目, MED ≈ 40, LOW ≈ 90, NONE ≈ 15**.

---

## 2. 高价值项目 TOP 10 (HIGH 相关, P0/P1 借鉴)

> 借鉴方式: `📦clone` = git clone 看代码, `📄看文档` = 看 README/spec, `🔬派 sub-agent` = 派调研, `⏸️不借鉴` = 仅参考

### P0 (1 周内立即借鉴, 真实施)

| # | 项目 | URL | 一句话 | 借鉴点 | 优先级 | 方式 |
|---|---|---|---|---|---|---|
| 1 | **N.E.K.O 猫娘计划** | https://github.com/Project-N-E-K-O/N.E.K.O | 网络型情感知性生命体, 五维记忆 + Live2D/VRM/MMD | 五维记忆系统 ↔ v2 cognitive.memory_recall + memory_writeback; Live2D 多形态 ↔ RC-7 Perception 真 modality (视觉子模态) | **P0** | 📄看文档 + 🔬派 sub-agent 调研 (1 周, 写 `r7-neko-perception-research.md`) |
| 2 | **AIRI** | https://github.com/moeru-ai/airi | 2.2 万 Star 虚拟伴侣, 实时陪聊 | Live2D + 主动消息 + 长期记忆 ↔ v2 五维记忆 + RC-7 主动 perception; Star 数高 = 社区验证 | **P0** | 📄看文档 + 📦clone 看 Live2D 渲染 pipeline |
| 3 | **Open-LLM-VTuber** | https://github.com/Open-LLM-VTuber/Open-LLM-VTuber | 语音 + 视觉 + 工具调用 + Live2D (Cubism 5) | 完整 ASR→LLM→TTS→Live2D 链路 ↔ v2 gateway SSE pipeline (B 块) | **P0** | 📦clone 看 ASR/TTS 抽象层 |
| 4 | **Firefly Companion (流萤)** | https://github.com/ff-ai/firefly-companion | GPT-SoVITS 原声 TTS + MCP 工具链 | GPT-SoVITS TTS 集成 ↔ v2 TTS modality; MCP 工具链 ↔ v2 gateway 工具注册 | **P0** | 📄看文档 (TTS 接入模式) |
| 5 | **Mio** | https://github.com/ochiru520/Mio | Windows 本地优先 AI Agent, 对话/记忆/日记/QQ/语音/Live2D/屏幕感知 | 屏幕感知 ↔ RC-7 Perception 视觉模态; 桌面优先 ↔ v2 portable 部署 (用户想法 #1 印证) | **P0** | 📄看文档 (屏幕感知抽象) |

### P1 (1 月内排上, 趋势参考 + 部分借鉴)

| # | 项目 | URL | 一句话 | 借鉴点 | 优先级 | 方式 |
|---|---|---|---|---|---|---|
| 6 | **Warashi** | https://github.com/warashi/warashi | 免费开源桌面伴侣, Live2D + 长期记忆 + 主动聊天 + 睡眠模式 | 睡眠模式 ↔ v2 organ cycle (L0-L5 UpgradeCycle Stage 5); 主动聊天 ↔ v2 council/judge | **P1** | 📄看文档 |
| 7 | **GPT-SoVITS** | https://github.com/RVC-Boss/GPT-SoVITS | 角色音色训练 + 语音合成 | TTS backend 候选 ↔ RC-7 TTS modality (P2 实施) | **P1** | 📄看文档 (Python 端, 看 Rust binding 可行性) |
| 8 | **faster-whisper** | https://github.com/SYSTRAN/faster-whisper | 语音转写 (C++/CUDA) | ASR backend 候选 ↔ RC-7 ASR modality | **P1** | 📄看文档 |
| 9 | **Megumi** | https://github.com/foxabbage/Megumi | 模块化 AI VTuber, Core Server + 多进程 + WebSocket | 多进程 + WebSocket ↔ v2 gateway SSE (B 块), 架构参考 | **P1** | 📄看文档 |
| 10 | **Alife** | https://github.com/BDFFZI/Alife | 桌宠 Agent, 一键安装 + 极低开销 + 插件化 | 极低开销 + 插件化 ↔ v2 portable binary (用户想法 #1); 插件 ↔ v2 plugin-authoring-guide | **P1** | 📄看文档 |

### 借鉴方式: P0 派 1 sub-agent 调研 TOP 5, P1 主代理排下月 (4-6 月并行 critical path 内).

---

## 3. 同领域项目 (MED 相关, 趋势参考)

> 不真借鉴, 仅了解趋势, 主代理团队知道就行

- **开发工具**: OpenObserve (可观测, v2 暂不需要) / Crawl4AI (LLM 爬虫, 跟 R14 perception 弱相关) / Deepsearcher (深度搜索, 跟 v2 memory recall 思路类似) / Composio (工具集成, MCP 类似) / codebase-memory-mcp (代码智能, v2 cognitive memory 思路) / MemPalace / Shadoweave HMS (记忆系统, 参考命名)
- **安全**: Vaultwarden (Rust 写, Bitwarden 兼容) / BitChat (蓝牙 Mesh) / opencode-vibeguard (敏感信息脱敏, 跟 v2 P0 governance credential disclosure 相关)
- **AI Agent 框架 (调研不重叠)**: CrewAI / LangGraph / Pydantic AI / Hermes Agent / pi-mono / Mission Control / LoopX / PenguinHarness / OpenViking / Kimi Code / AstrBot / Panniantong/Agent-Reach — **v2 已借鉴或与 research/source 重叠**, 不重复
- **金融 (趋势)**: TradingAgents / QuantDinger / Lean — 远领域, 仅用户有 finance 兴趣时参考

---

## 4. 低价值项目 (LOW + NONE, 简短列名)

**LOW (≤1 行 each)**: N.E.K.O 同类 (OpenClaw x2 / Hermes Agent / LoopX / PenguinHarness / Kimi Code / Mission Control / AstrBot / CLI-Anything / Agent-Reach / pi-computer-use / Cua / OpenLovable / AutoResearch / OmegaWiki / llm_wiki / OpenCodex / codex-plugin-cc / CrewAI / LangGraph / Pydantic AI / CopilotKit / Omnigent / SwarmVault / graph-engineer / DeepSeek Harness / OpenOPC / DeerFlow / Harness-R1 / J-Space 报告 / DataFlow / DataFlow-WebUI / RAGFlow / Serena / OpenCodeReview / Page Agent / OpenWorker / OpenClaw / MCP servers) / NapCatQQ / Ollama / BitChat / Session / Briar / Sandboxie / VulnClaw / DeepSec / opencode-vibeguard / KeePassXC / Passbolt / Bitwarden / Vaultwarden / Sherlock / Maigret / HunyuanOCR / TradingAgents / Lean / vnpy / Bigcapital / Akaunting / ERPNext / InvoicePlane / Odoo / AiToEarn / Openhuman / Dexter / QuantDinger / Kronos / FinSight-AI / Vibe-Trading / OpenStock / OpenAlice / TickFlow / TradingAgents-Astock / Gloomberb / Odysseus / FinanceDatabase / Awesome-finance-skills / pi-mono / Tavily / Playwright-MCP / Self-Improving / Agent-S / camofox-browser / graphify / anthropics/financial-services / Composio / Carbonyl / Colly / Playwright / Starship / Pake / ScriptCat / doc7 / OpenDataLoader-PDF / BrowserAct Skills / dbx / Markdown Online Editor / Turbovec / archify / sub2api / OpenSquilla / claude-mem / exo-explore / project-nomad / T3MP3ST / Unlimited OCR / Anysearch / wechat-article-exporter / BilldDesk Pro / Scrapling / andrej-karpathy-skills / m_flow / alibaba/zvec / feynman / pixel2motion / deeptutor / ChineseIndependentDeveloper / langchain-ai/openwiki / openscience / opengist / obsidian-llm / free-for-dev / Ant Design / Taipy / AirLLM / English Level Up / Flint Chart / CL4R1T4S / Ponytail / old-coder / PUA / live_coding / Mythos 架构 / GeoLibre

**NONE (完全无关)**: Navidrome / Jellyfin / Airsonic-Advanced / mStream / Gonic (音乐流媒体) / GeoLibre (GIS, 重复算 LOW)

---

## 5. 用户末尾 2 条 "想法随记" 评估

### 5.1 "Apeireth 可以便携安装进 U 盘"

**对接 v2 工程现状**: workspace 16 crates + Cargo workspace 编译产物 (单 binary `target/release/apeireth`) + `Cargo.lock` 锁定 + O-1 安全优先哲学锚 + 用户本位定位 (S-1 北极星 + 五原型). **强对接**, portable binary 是 v2 本来就该做的.

**实施路径 (估时 1 周)**:
1. strip binary (per Cargo `[profile.release] strip = true`)
2. 单 binary 静态链接 (`codegen-units = 1` + `lto = "thin"`)
3. cargo deb (Linux) + NSIS (Windows) 打包
4. U 盘启动 script (per-platform shell launcher, 自动找 `APEIRETH_HOME`)
5. doc: `docs/02-guides/portable-install.md` (用户文档)
6. 测试: 在 FAT32/exFAT U 盘跑 `cargo test --release`

**借鉴**: Alife (一键安装) + 借鉴 v2 portable binary 模式 (借鉴 A 块 Stage 5 L0-L5 UpgradeCycle portable 模式)

**推荐**: **P1 排上 (post-release, v2.0.0 release 后 1-2 周做)**. v2 release 估 2027-Q1, post-release 估 2027-Q2 1 周.

### 5.2 "Apeireth 可以做自己刷视频的模块, 对接入现代生活, 网络"

**对接 v2 工程现状**: RC-7 Perception 真 modality (R14 调研就位, 2-3 周需硬件到位). 视频 = modality 之一 (跟音频/图像/文本并列). 用户本位 + O-2 前人肩上 (借鉴 N.E.K.O / AIRI / Open-LLM-VTuber / Warashi 等 P0 HIGH 项目).

**实施路径 (估时 2-4 周)**:
1. 派 sub-agent 调研视频 modality (R14 真 modality 子集), 写 `r14-video-modality-research.md` (≤200 行, 真账)
2. 视频 backend impl: 视频源 (本地文件 / 网络 URL / RTSP / WebRTC) + 解码 (ffmpeg / openh264) + 帧采样 (per N 秒) + 视觉 embedding (CLIP / MobileCLIP)
3. 视频 → 视觉 modality → cognitive.perception slot (R14)
4. 借鉴: N.E.K.O (VRM/MMD 视觉) + Open-LLM-VTuber (视觉感知 pipeline) + AIRI (Live2D 渲染)
5. 测试: E2E 视频感知 (D 块 E2E baseline)

**风险**: 视频解码 CPU/GPU 开销 (跟 O-6 永远追求最优冲突, 需 strip + lto + 异步解码)

**推荐**: **P2 后续 (post-release, 估 2027-Q3, 2-4 周)**. 排在 R14 真 modality 硬件到位之后 (critical path Week 5-6 之后).

---

## 6. 主代理决策建议 + 派单顺序

### P0 (1 周内, 立即可借鉴)

1. **派 1 sub-agent 调研 TOP 5 P0 HIGH 项目** (N.E.K.O / AIRI / Open-LLM-VTuber / Firefly / Mio), 写 5 个调研真账 (各 ≤200 行, 总 ≤1000 行), 总估时 1 周, 并行 4-6 月 critical path
2. **借鉴 Alife + OpenClaw portable 模式** 为便携 U 盘做技术调研 (1-2 天, 跟派单 1 并行)
3. **借鉴 N.E.K.O 五维记忆** → v2 cognitive.memory_recall / memory_writeback 增维 (跟 R20 preference_learning 真实施并行, 2-3 周)

### P1 (1 月内, 趋势参考 + 部分借鉴)

4. **派 1 sub-agent 调研 GPT-SoVITS / faster-whisper / Megumi / Warashi** (TTS/ASR/多进程/睡眠模式), 写 4 个调研真账, 估时 2-3 周
5. **借鉴 Vaultwarden (Rust 写 Bitwarden 兼容)** + opencode-vibeguard → v2 O-1 安全优先 (per §10 LOCKED 0 触碰约束, 改仅 3 hook 之外)
6. **用户想法 #1 "便携 U 盘"**: 排 post-release (v2.0.0 release 后 1 周, 估 2027-Q2)

### P2 (后续, 1-3 月后)

7. **用户想法 #2 "刷视频模块"**: 派 sub-agent 调研 R14 视频 modality, 写 `r14-video-modality-research.md`, 估 2-4 周, 排在 R14 硬件到位之后 (估 2027-Q3)
8. **LOW + NONE 项目**: 不真借鉴, 仅主代理团队知道 (本真账已列名, 不再派单)
9. **资源汇总 (Ant Design / free-for-dev / Obsidian LLM 等)**: 趋势合集, 主代理阅后归档

### 决策建议摘要

- **不要全借鉴** — 170 项目仅 25 HIGH, 大部分 LOW/NONE 是 noise
- **优先派 sub-agent 调研 P0 TOP 5** — 真账 + 借鉴路径, 估 1 周, 跟 critical path 并行
- **用户 2 想法随记都 P1/P2 排上**, 不阻塞 critical path
- **借鉴原则**: per O-2 前人肩上, 必 1:1 翻译 v1 legacy 优先, 其次 research/source, 最后本清单 (顺序不重叠)
- **0 装诚实标**: 本真账关联度未实测 (未 git clone/grep), 仅基于用户清单 + v2 handbook 已知信息评估; 真实施前主代理必亲验

---

## 真账完成度

- 6 段全写 (分类总览 / TOP 10 / MED / LOW+NONE / 用户想法 / 派单建议)
- 行数: ≤250 行 (per 任务 brief 约束)
- 不写真账以外的 file (per 任务 brief 约束)
- 不 git add / commit / push (per 任务 brief 约束)
- 主代理决策参考就位
