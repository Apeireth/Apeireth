# Round 10 主代理亲验修订 — 170 项目清单调研 (2026-08-28)

> **作者**: 主代理 Mavis (写于 Round 10 sub-agent 调研后, 亲验修订决策)
> **用途**: 记录主代理亲验 sub-agent `ebdd7a77` 170 项目调研真账 + 修订派单顺序 (per O-5 + Round 4 教训)
> **关系**: 跟 `youyou-list-research-2026-08-28.md` (sub-agent 156 行调研真账) + `round-10-research-plan-2026-08-28.md` (主代理预判 310 行 plan) 互补

```
[Document-Meta]
Document:        docs/04-internal/round-10-youyou-list-mainagent-verify-2026-08-28.md
Version:         1.0 (主代理 Mavis 写于 Round 10 sub-agent 报告后)
Last-Modified:   2026-08-28
Status:          🟢 活跃 (主代理亲验修订决策)
Author:          主代理 Mavis
```

---

## 0. 派活真账 (per O-5 + Round 4 教训)

**用户原话** (Round 10 启动): "调研都要落文档你知道吧" + 之前 "调研区域啥情况" + "去把 `C:\Users\31683\Desktop\Youyou\重要参考项目...` 这里的项目都调研一下，看看对我们的项目有什么用".

**主代理行动顺序**:
1. ✅ **删 .db 垃圾** (research/08-rust-substrate-current/data/*.db, 180 KB, R8 临时数据)
2. ✅ **写真账 Round 10 plan** (`round-10-research-plan-2026-08-28.md`, 310 行, 主代理预判 170 项目分类 + TOP 10 预判 + 派单顺序)
3. ✅ **派 sub-agent 真调研** (`ebdd7a77`, 真账 `youyou-list-research-2026-08-28.md`, 156 行, 6 段全)
4. ✅ **主代理亲验** (per O-5 + Round 4 教训 catch 任何误判) — **catch 1 处需要修订** (TOP 5 评估方法偏差, 详见 §1)
5. 🔲 **写真账本 doc** (主代理亲验修订 + 派单顺序修订) — 本文件
6. 🔲 **commit + push** (Round 10 收尾)

---

## 1. 主代理亲验 catch (per O-5 + Round 4 教训)

### 1.1 sub-agent TOP 5 P0 HIGH 评估 (AI 伴侣类)

| # | sub-agent TOP 5 | 类型 | sub-agent 评估理由 |
|---|---|---|---|
| 1 | N.E.K.O 猫娘计划 | AI 伴侣 | 五维记忆系统 ↔ v2 cognitive.memory_recall + memory_writeback; Live2D ↔ RC-7 Perception 视觉子模态 |
| 2 | AIRI | AI 伴侣 | Live2D + 主动消息 + 长期记忆 ↔ v2 五维记忆 + RC-7 主动 perception; 2.2 万 Star 社区验证 |
| 3 | Open-LLM-VTuber | AI 伴侣 | 语音 + 视觉 + 工具调用 + Live2D (Cubism 5) ↔ v2 gateway SSE pipeline (B 块) |
| 4 | Firefly Companion (流萤) | AI 伴侣 | GPT-SoVITS TTS + MCP 工具链 ↔ v2 TTS modality + gateway 工具注册 |
| 5 | Mio | AI 伴侣 | Windows 本地优先 + 屏幕感知 + QQ + Live2D ↔ RC-7 屏幕感知 + portable 部署 |

### 1.2 主代理预判 TOP 5 (Agent 框架类)

| # | 主代理预判 TOP 5 | 类型 | 主代理预判理由 |
|---|---|---|---|
| 1 | DeepSeek Harness | Agent 框架 | DeepSeek 官方 Agent Harness, 我们的 ancestor 之一 |
| 2 | LangGraph | Agent 框架 | 状态多 Agent 应用框架 ↔ v2 cognitive module wiring state machine |
| 3 | CrewAI | Agent 框架 | 多 Agent 编排框架 ↔ v2 OrganOrchestrator |
| 4 | Cua | Agent 框架 | 开源 Computer Use 2.0 驱动 ↔ v2 D 块 RC-7 Perception 工具调用 |
| 5 | Serena | Agent 框架 | MCP 编码工具包 ↔ v2 tool pipeline |

### 1.3 catch: TOP 5 不矛盾 — 是两个维度 (per O-2 前人肩上)

**关键发现 (主代理亲验)**:
- **sub-agent TOP 5 全是 AI 伴侣类** (N.E.K.O / AIRI / Open-LLM-VTuber / Firefly / Mio)
- **主代理预判 TOP 5 全是 Agent 框架类** (DeepSeek Harness / LangGraph / CrewAI / Cua / Serena)
- **不矛盾**, 是两个维度:
  - **Agent 框架类借鉴**: 12 slot cognitive module + 9 organ 抽象 + agent loop 模式 (直接代码借鉴边界)
  - **AI 伴侣类借鉴**: Live2D 视觉子模态 + 五维记忆系统 + ASR/TTS 抽象层 (设计借鉴, 不直接借鉴 Live2D 渲染)

**关键: Apeireth v2 是 AI Agent framework (后端 orchestrator), 不是 AI 伴侣 (前端 Live2D UI)**
- Agent 框架类借鉴价值: 直接代码 / 抽象边界 / 接口设计
- AI 伴侣类借鉴价值: Live2D 视觉形象子模态 (RC-7 真 modality 子集) + 五维记忆 (memory module 拓展) + ASR/TTS 抽象层

### 1.4 修订决策 (per O-5 + O-6 总体最优)

**采纳 sub-agent TOP 5 (AI 伴侣类) + 主代理预判 TOP 5 (Agent 框架类), 但有调研优先级差异**:

**借鉴顺序 (per O-2 前人肩上, 不重叠)**:
1. **已借鉴 (0 调研需要)** — `research/source/` 里 Agent 框架 (LangGraph / CrewAI / DeepSeek Harness / Cua / Serena / MetaGPT / openclaw / hermes-agent-rs 等 ~36 真开源)
2. **P0 派单 (1 周内)** — sub-agent TOP 5 (N.E.K.O / AIRI / Open-LLM-VTuber / Firefly / Mio) 真调研 (Live2D 视觉子模态 + 五维记忆 + ASR/TTS 抽象)
3. **P1 排上 (1 月内)** — sub-agent P1 5 (Warashi / GPT-SoVITS / faster-whisper / Megumi / Alife) 调研 + 用户想法 #1 (便携 U 盘)

**关键修订: 借鉴 AI 伴侣类不借鉴 Live2D 渲染本身** (那不是 v2 backend), 只借鉴:
- Live2D 视觉子模态 (跟 D 块 RC-7 Perception 真 modality 对接, P0 P1)
- 五维记忆系统 (跟 v2 cognitive.memory_recall / memory_writeback 增维, P0 P1)
- ASR/TTS 抽象层 (跟 B 块 gateway SSE + D 块 RC-7 ASR/TTS modality 对接, P0 P1)
- 多进程 + WebSocket 架构 (跟 v2 gateway 架构参考, P1)
- 一键安装 + 极低开销 (跟用户想法 #1 便携 U 盘对接, P1)

### 1.5 0 装诚实标 (per O-5)

- sub-agent 评估方法偏差: 用 "社区活跃度" (Star 数) 评估 HIGH, 不是用 "直接借鉴边界" (代码可移植性). 主代理亲验 catch.
- sub-agent 0 实测 (未 git clone / grep), 仅基于用户清单 + v2 handbook 已知信息评估. **真实施前主代理必亲验** (per sub-agent §6 §146 0 装诚实标)
- sub-agent TOP 5 (AI 伴侣) 跟主代理预判 TOP 5 (Agent 框架) 不重叠, 是两个维度. 借鉴顺序: research/source 已借鉴 Agent 框架 → 派单新调研 AI 伴侣类 → 不重复.

---

## 2. 主代理亲验修订后派单顺序 (vs sub-agent §6)

### 2.1 P0 (1 周内, 立即可借鉴)

#### 2.1.1 派 1 sub-agent 真调研 TOP 5 (N.E.K.O / AIRI / Open-LLM-VTuber / Firefly / Mio) — sub-agent 推 P0

| # | 项目 | 借鉴点 | 调研方式 | 估时 |
|---|---|---|---|---|
| 1 | **N.E.K.O** | 五维记忆系统 (跟 v2 cognitive.memory_recall + memory_writeback 增维) | 派 sub-agent 真调研 + 写真账 ≤200 行 | 1-2 天 |
| 2 | **AIRI** | Live2D 视觉形象 (跟 RC-7 Perception 视觉子模态) + 长期记忆 + 主动消息 (跟 council/judge) | 派 sub-agent 真调研 | 1-2 天 |
| 3 | **Open-LLM-VTuber** | ASR→LLM→TTS→Live2D 完整链路 (跟 B 块 gateway SSE + RC-7 ASR/TTS 真 modality 对接) | 派 sub-agent 真调研 | 1-2 天 |
| 4 | **Firefly Companion** | GPT-SoVITS TTS 集成 (跟 RC-7 TTS modality) + MCP 工具链 (跟 v2 tool 注册) | 派 sub-agent 真调研 | 1-2 天 |
| 5 | **Mio** | Windows 本地优先 + 屏幕感知 (跟 RC-7 视觉模态) + QQ (跟 interface 扩展) | 派 sub-agent 真调研 | 1-2 天 |

**派单 brief 模板** (per `v2-reference-handbook-2026-08-28.md` §3.1):
- 5 sub-agent 真调研, 每个写真账到独立文件 `docs/01-architecture/borrowed-from-{name}-2026-08-28.md`
- 每个 ≤200 行
- 写真账必含: 实现摘要 / 借鉴点 (代码 / 设计 / 接口) / 0 触碰 LOCKED 验证 / 借鉴路径 (clone / 看文档 / 不借鉴) / 下一步
- 跑 5 重守门 baseline (主代理亲验前不假装 PASS — per Round 9 B-A sub-agent 失守教训)
- 0 引新外部 dep (per §10 LOCKED 0 触碰约束)

#### 2.1.2 借鉴 Alife + OpenClaw portable 模式 — sub-agent 推 P1, 主代理改 P0

- **Alife** (一键安装 + 极低开销 + 插件化) ↔ 用户想法 #1 (便携 U 盘, P1)
- **OpenClaw** (跨平台 CLI + Gateway) ↔ 用户想法 #1 (便携 U 盘, P1)
- **借鉴**: sub-agent 真调研 Alife (1-2 天, 写真账 ≤200 行, 含借鉴路径)

### 2.2 P1 (1 月内, 趋势参考 + 部分借鉴)

#### 2.2.1 借鉴 N.E.K.O 五维记忆 → v2 cognitive.memory_recall / memory_writeback 增维

- 跟 R20 preference_learning 真实施并行 (2-3 周)
- N.E.K.O 五维记忆拆解 (episodic / semantic / procedural / emotional / reflective) ↔ v2 memory module 现有架构
- 派 sub-agent 真调研 N.E.K.O 五维记忆 + 写真账 ≤200 行
- 估时 1-2 周

#### 2.2.2 派 sub-agent 真调研 P1 5 (Warashi / GPT-SoVITS / faster-whisper / Megumi / Alife)

- Warashi (睡眠模式 ↔ v2 organ cycle L0-L5 UpgradeCycle Stage 5; 主动聊天 ↔ v2 council/judge)
- GPT-SoVITS (TTS backend 候选 ↔ RC-7 TTS modality)
- faster-whisper (ASR backend 候选 ↔ RC-7 ASR modality)
- Megumi (多进程 + WebSocket ↔ v2 gateway SSE)
- Alife (一键安装 + 插件化 ↔ 用户想法 #1 + v2 plugin-authoring-guide)

每个派 sub-agent 真调研, 写真账 ≤200 行.

#### 2.2.3 借鉴 Vaultwarden + opencode-vibeguard → v2 O-1 安全优先

- Vaultwarden (Rust 写 Bitwarden 兼容) ↔ v2 cargo keyring + governance credential
- opencode-vibeguard (敏感信息脱敏) ↔ v2 P0 governance credential disclosure hook
- **0 触碰 LOCKED** (改仅 3 hook 之外, per §10)

#### 2.2.4 用户想法 #1 (便携 U 盘)

- 排 post-release (v2.0.0 release 后 1 周, 估 2027-Q2)
- 估时 1 周 (post-release)
- 借鉴 Alife (一键安装) + OpenClaw (跨平台 CLI + Gateway)

### 2.3 P2 (后续, 1-3 月后)

#### 2.3.1 用户想法 #2 (刷视频模块)

- 派 sub-agent 调研 R14 视频 modality, 写真账 `docs/01-architecture/r14-video-modality-research.md`
- 估时 2-4 周
- 排在 R14 硬件到位之后 (估 2027-Q3)
- 视频 modality = R14 真 modality 子集 (跟音频/图像/文本并列)

#### 2.3.2 LOW + NONE 项目不真借鉴

- 仅主代理团队知道 (本真账已列名, 不再派单)
- 90 LOW 项目 (含金融 / 量化 / 音乐 / 地理 / 资源汇总大部分) + 15 NONE 项目 (完全无关)
- skip, 不浪费 token

---

## 3. 0 装诚实标 (per O-5 历次 flag 真账)

| 失守 | 详情 | 修法 |
|---|---|---|
| **sub-agent TOP 5 评估方法偏差** | 用 "社区活跃度" (Star 数) 评估 HIGH, 不是用 "直接借鉴边界" (代码可移植性). 真账标 AI 伴侣类 (5/5 TOP5), 但 Apeireth v2 是 AI Agent framework 不是 AI 伴侣 | 主代理亲验 catch, 修订借鉴边界 (仅借鉴 Live2D 视觉子模态 + 五维记忆 + ASR/TTS 抽象, 不借鉴 Live2D 渲染本身) |
| **sub-agent 0 实测** | 未 git clone / grep 170 项目, 仅基于用户清单 + v2 handbook 已知信息评估. 真账 §6 §146 0 装诚实标已标 "关联度未实测" | 主代理亲验 + 5 sub-agent 真调研 (P0) 时实测 |
| **本轮无新失守** | O-6 三阶审查实跑, 0 引新外部 dep, 0 写真账以外的 file, 5 重守门 baseline 维持 | ✅ |

---

## 4. 5 重守门 baseline + LOCKED 0 触碰 验证

| 守门 | 实测 |
|---|---|
| clippy 0 warning | ✅ (前 baseline) |
| tests 0 fail (1739 passed) | ✅ (前 baseline) |
| legacy compat path < 100 (36) | ✅ (前 baseline) |
| LOCKED 5 项 0 触碰 | ✅ (本轮 0 改 src / Cargo.toml / Cargo.lock) |
| 9 哲学锚 0 减 | ✅ |

---

## 5. 留 backlog (per §2 派单顺序)

| # | 项 | 估时 | 阻塞 |
|---|---|---|---|
| 1 | 派 5 sub-agent 真调研 P0 TOP 5 (N.E.K.O / AIRI / Open-LLM-VTuber / Firefly / Mio) | 1-2 周 (5 sub-agent 并行) | 0 |
| 2 | 派 1 sub-agent 真调研 Alife (portable 模式借鉴) | 1-2 天 | 0 |
| 3 | 借鉴 N.E.K.O 五维记忆 → v2 cognitive memory module 增维 (跟 R20 并行) | 1-2 周 | R20 真实施 |
| 4 | 派 1 sub-agent 真调研 P1 5 (Warashi / GPT-SoVITS / faster-whisper / Megumi / Alife) | 2-3 周 (5 sub-agent) | 0 |
| 5 | 借鉴 Vaultwarden + opencode-vibeguard → v2 O-1 安全优先 | 1-2 天 | §10 LOCKED 0 触碰约束 |
| 6 | 用户想法 #1 (便携 U 盘) P1 | 1 周 (post-release, 估 2027-Q2) | v2.0.0 release |
| 7 | 用户想法 #2 (刷视频模块) P2 | 2-4 周 (估 2027-Q3) | R14 硬件到位 |
| 8 | R20 preference_learning 真实施 (跟 P0 派单并行) | 2-3 周 | R10 OrganKind 决策 |
| 9 | B/C/D 块真实施 (B gateway SSE + C preference_learning + D RC-7 perception) | 6-8 周 critical path | 0 |

---

_Mavis 写于 2026-08-28 Round 10 sub-agent 报告后, 主代理亲验修订派单顺序, 真账就位._
