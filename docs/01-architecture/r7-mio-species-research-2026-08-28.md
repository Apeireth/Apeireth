# Mio 对 Apeireth v2 物种化借鉴调研 (2026-08-28)

> **作者**: sub-agent R7-Mio (主代理 Mavis 派单, 时间紧 ≤ 4h)
> **用途**: 给主代理决策参考 — Mio 对 Apeireth v2 的**物种化借鉴**作用 (Windows 本地优先 + 个人 AI Agent + 日记 + 屏幕感知)
> **关系**: 跟 `apeireth-true-understanding-2026-08-28.md` (物种化真理解) + `youyou-list-research-2026-08-28.md` L50 (Mio P0) + `rc7-perception-research-2026-08-28.md` §1.2 (Vision xcap) + `v2-reference-handbook-2026-08-28.md` §1.3 (12 cognitive slot) 互补
> **0 装诚实标**: 已读用户清单 L11 + Apeireth 真理解 §1-3 + vision.md L29-49 + v2 handbook §1.3 + R20 真账; **未 git clone Mio** (brief 4h 限 + github.com 直连防火墙 [exit code: 1] 21s + web_search auth fail 双 0 装); 仅基于用户清单 L11 + 你you-list L50 + 真理解推论, **真实施前主代理必亲验**

```
[Document-Meta]
Document:        docs/01-architecture/r7-mio-species-research-2026-08-28.md
Version:         1.0 (sub-agent R7-Mio 写于 2026-08-28)
Status:          🟢 活跃 (调研真账)
Author:          sub-agent R7-Mio (主代理 Mavis 派)
```

---

## 1. Mio 项目定位

**一句话**: Windows 本地优先个人 AI Agent, 对话/记忆/日记/QQ/语音/Live2D/屏幕感知 (per 你you-list L50 + 用户清单 L11).

**10 项目维度评估**:

| # | 维度 | 评估 | 来源 |
|---|---|---|---|
| 1 | 代码规模 | **未实测** (未 git clone) | 用户清单 L11 |
| 2 | Star | **未实测** (github 不可达 + web auth fail) | per L50 |
| 3 | 活跃度 | **未实测**; commit/issue/release 0 数据 | n/a (O-5) |
| 4 | License | **未实测**; 假设 MIT/Apache-2.0 | 假设 (O-5 flag) |
| 5 | 文档质量 | L11 简练 (7 大功能 1 行); 推断**中等** | L50 |
| 6 | **物种化借鉴** | **HIGH** — Windows 本地优先 ↔ v2 portable binary (用户想法 #1) + single-user ↔ v2 物种化 per-user 塑形 | 真理解 §1.1.3 + L50 |
| 7 | frontend 借鉴 | **MED-HIGH** — Live2D ↔ companion-desktop (与 N.E.K.O/AIRI 同类, 但 Mio 非 species 视角) | L50 |
| 8 | backend 借鉴 | **HIGH** — 日记 (反思+写回) ↔ v2 self_assessment+memory_writeback 增维 (核心) + 屏幕感知 ↔ RC-7 Vision | 真理解 §3 + RC-7 §1.2 |
| 9 | 0 装 PASS | L11 "Windows 本地优先" = 工程层 0 装 (下载即用, 不 cloud 依赖); 跟 Apeireth "不假装完成" 同源 | L11 |
| 10 | 风险 | **未实测** + github 不可达 + 0 真 clone = **0 装诚实标必标** (亲验必做) | O-5 |

---

## 2. 物种化借鉴价值 (核心)

> **0 装诚实**: 之前 Round 10 你you-list L50 把 Mio 标 "P0 屏幕感知 + 桌面优先", brief 是 frontend 视角. 本调研按真理解 §1.1.3 修订为 **物种化借鉴维度** (per-user 塑形 + 本地优先 + 日记 + 屏幕感知).

### 2.1 Windows 本地优先 ↔ v2 companion-desktop (用户想法 #1 便携 U 盘印证)

**Mio**: Windows 本地优先 — 单 binary + 本地存储 + 0 cloud 依赖 (中文个人 AI Agent 标准范式).

**Apeireth** (per 真理解 §1.1 + L87-99):
- L57: "companion-desktop 是物种化的具体形态 — 同一套 backend, 不同用户不同前端皮肤"
- **用户想法 #1** "Apeireth 可以便携安装进 U 盘" — post-release v2.0.0 估 2027-Q2
- 工程: 16 crate workspace + 单 binary `target/release/apeireth` + `Cargo.lock` 锁定

**借鉴点**:
1. **Windows 本地 binary 部署** ↔ v2 portable binary (per L93 借鉴 Alife)
2. **便携 U 盘启动** (FAT32/exFAT + `APEIRETH_HOME` 环境变量) ↔ Mio 本地优先范式
3. **本地存储 = 物种化基础** — per-user 记忆/偏好完全本地, 0 cloud 共享, 跟 vision.md L47 "每个她独立塑形" 完美对接

### 2.2 个人 AI Agent ↔ v2 物种化 per-user 塑形

**Mio**: 个人 AI Agent = single-user 设计 (默认一用户养一"她").

**Apeireth vision.md L47**: "物种而非个体" + "机制/哲学/安全同源, 记忆/偏好/好奇形状被各自的共同生活塑形" + 工程兑现 per-user memory(5维)+ preference + curiosity + emotional timeline.

**物种 vs 个体**:
- **同**: Mio single-user + Apeireth 物种化 — 同源哲学 (用户专属 AI, 非 framework)
- **异**: Mio "个人" 是单设备狭义; Apeireth "物种" 是单用户广义 (本地优先 + 跨设备可选, post-v2.0.0)
- **借鉴启示**: Mio 是 v2 物种化 **最贴近的工程参考**

### 2.3 日记功能 ↔ v2 cognitive self_assessment + memory_writeback (核心!)

**Mio**: 日记 — 推断每日自动生成 + 用户可读 + 长期保存 (per "对话/记忆/日记" 组合).

**Apeireth v2 cognitive slot 现状** (per handbook §1.3 L58-62):
- `cognitive.self_assessment` — WIRED, Judge-backed, AfterTurn
- `cognitive.memory_writeback` — WIRED, AfterTurn, append-only Episodes
- `cognitive.reflection` — **DEFERRED INTO SELF-ASSESSMENT** → R22 派单 (1 周)
- `cognitive.preference_learning` — **DEFERRED → R20 派单** (调研就位, 2-3 周)
- **缺耦合**: self_assessment → memory_writeback 无显式 pipeline

**借鉴点 (核心!)**:
1. **日记 = 反思 + 写回合一** ↔ v2 `self_assessment + memory_writeback` 二合一路径完美对齐
2. **R22 reflection 真实施直接借鉴** (1 周, per L62) — 日记反思部分对标
3. **R20 preference_learning 真实施直接借鉴** (2-3 周, per L60) — 日记 → preference 提炼
4. **cognitive module 增维**: 加 `reflection_writeback_pipeline` trait, R20+R22+日记三者**并发** (3-4 周 critical path)

### 2.4 屏幕感知 ↔ v2 RC-7 perception modality

**Mio**: 屏幕感知 — 推断 GetForegroundWindow 轮询 (v1 同款, per RC-7 §1.2) 或更深 xcap 截屏.

**Apeireth RC-7 真账** (§1.2): v1 屏幕"感知" = 窗口轮询 (168 行, **不是截屏**); RC-7 Vision 真接 (xcap) = **新增能力**; xcap 仅 Windows 真接, Linux/macOS NoopVisionBackend.

**借鉴点**:
1. **0 装诱导 prevention**: Mio 若走 v1 同款 (窗口轮询), 跟 v1 等价, **不假装** "RC-7 Vision 真接"; RC-7 Vision 真接是新增能力
2. **xcap 真接路径** (1 周, 跟 RC-7 §1.2 完全重叠)
3. **主代理决策**: 派 sub-agent 真调研 Mio 屏幕感知 (推断路径) + 主代理亲验, 写 `r7-mio-screen-perception-research.md` ≤200 行

### 2.5 QQ interface ↔ v2 即时通讯扩展 (NapCatQQ, LOW)

**Mio**: QQ — 推断 NapCatQQ / go-cqhttp / OneBot 协议.

**Apeireth v2 gateway** (per L29): 即时通讯 = LOW priority, "v2 gateway 已有 SSE, 不直接借鉴"; NapCatQQ 在 LOW 段 (L79).

**借鉴点**: **LOW**, 不排 critical path; P2 后续调研, 跟 N.E.K.O/Firefly 同类合并.

### 2.6 语音 (TTS/ASR) ↔ v2 RC-7 真 modality (重叠, 不重)

**Mio**: 语音 = ASR + TTS 标配.

**Apeireth RC-7 §1.1**: Voice 真接 = Whisper HTTP (麦克风 + API key); TTS trait 已写, 缺真 backend impl.

**借鉴点**: 跟 N.E.K.O / Open-LLM-VTuber 真账已写, Mio 语音**不是 unique 借鉴点**, 重叠路径不重复.

---

## 3. 物种架构借鉴 (具体借鉴点, 思路非代码)

### 3.1 Mio Windows 本地优先 ↔ v2 portable binary

**思路**: Mio 单 binary + 本地存储 + 0 cloud = per-user 记忆/偏好完全本地 = 物种化塑形物理基础 (cloud 共享破坏"每个她独立").

**真实施 (P1, 1-2 周调研)**:
- 派 sub-agent 真调研 Mio Windows 本地架构 (单 binary + 本地 DB + 配置目录)
- 写真账 `r7-mio-portable-research.md` ≤200 行
- 主代理决策: v2 portable binary 按 Mio 范式落地 (LOCKED 5 项 0 改, 走 `APEIRETH_HOME` 环境变量)

### 3.2 日记功能 ↔ v2 cognitive self_assessment + memory_writeback 增维 (核心!)

**思路**: Mio 日记 = 反思 + 写回合一 ↔ v2 self_assessment + memory_writeback 分离实现缺耦合. 借鉴: 加 `reflection_writeback_pipeline` trait (AfterTurn → Judge → 反思 → append-only Episodes).

**真实施 (P0, 跟 R20+R22 并行, 1-2 周)**:
- 派 sub-agent 真调研 Mio 日记实现 (推断: LLM 总结 + 时序 append + 本地 SQLite/Markdown)
- 写真账 `r7-mio-diary-research.md` ≤200 行
- 主代理决策: cognitive module 加 reflection_writeback pipeline trait (LOCKED 5 项 0 触碰, 走扩展 trait 接口)
- **R22 reflection (1 周) + R20 preference_learning (2-3 周) + 日记 cognitive 增维 (1-2 周) 三者并发**

### 3.3 个人 AI Agent 塑形 ↔ v2 per-user memory/preference

**思路**: Mio single-user 范式 ↔ per-user memory + preference + emotional timeline 物理独立 (本地优先 + 单 binary).

**借鉴点**:
1. per-user memory 物理隔离 ↔ `APEIRETH_HOME/data/{user_id}/` 路径
2. per-user preference 隐式学习 ↔ v2 R20 preference_learning (2-3 周)
3. per-user emotional timeline ↔ v1 F1 emotion_memory organ + v2 memory_writeback emotion 标签

### 3.4 屏幕感知 ↔ v2 RC-7 perception modality

**思路** (跟 RC-7 真账对齐): Mio 屏幕感知 = 推断 GetForegroundWindow OR xcap 截屏. **0 装诱导 prevention**: 跟 v1 同款窗口轮询**不算** RC-7 Vision 真接, 仅 v2.x 续; 新增能力 = xcap 像素截屏 (1 周, 跟 RC-7 §1.2 重叠).

---

## 4. 前端借鉴 (物种化具体形态)

### 4.1 Live2D ↔ companion-desktop

**Mio**: 默认 Live2D (桌宠标配), 推断 Cubism 5 + WebGL (per N.E.K.O/AIRI/Open-LLM-VTuber 同类).

**Apeireth v2**: companion-desktop (Svelte 5 + Tauri 2, post-1.0.0 PR #1); v1 done 1411 行 runtime.ts (SSE/WS/panel).

**借鉴点**: Live2D 渲染 pipeline ↔ companion-desktop 渲染层; 真实施: 跟 N.E.K.O §4.1 + Open-LLM-VTuber 真账同类合并, 派 sub-agent 一次性产出.

### 4.2 Windows 本地 UI ↔ v2 portable 部署

**Mio Windows 本地 UI**: 默认 Windows 桌面 UI (electron/tauri 推断).

**Apeireth v2 物种化 frontend 部署**: vision.md L47 "不同用户不同前端皮肤" + companion-desktop 多皮肤 (默认 Live2D + 可选 VRM/MMD) + **本地优先 = portable binary 范式**.

**借鉴点**: Mio "Windows 本地优先" 工程范式 ↔ v2 portable binary (per §3.1); 借鉴价值 = **frontend 部署范式** (本地优先 vs cloud-first), 非 frontend 视觉.

---

## 5. backend 借鉴 (基础层)

### 5.1 日记 ↔ v2 cognitive module wiring 增维 (P0 核心!)

**Mio 日记真账** (推断): LLM 总结 + 时序 append + 本地 SQLite/Markdown.

**Apeireth v2 cognitive module** (per handbook §1.3):
- 6/12 WIRED (memory_recall / preference_recall / judge / council / self_assessment / memory_writeback)
- 6/12 DEFERRED (preference_learning R20 / critic R21 / reflection R22 / planner R23 / orchestrator R24 / perception R14)
- **缺耦合**: self_assessment → memory_writeback 无显式 pipeline

**借鉴点 (P0, 核心)**:
1. **日记 = "反思 + 写回" 耦合范式** ↔ cognitive module 加 `reflection_writeback_pipeline` trait (扩展 trait, LOCKED 5 项 0 触碰)
2. **R22 reflection** (1 周, L62) 直接对标 Mio 日记反思部分
3. **R20 preference_learning** (2-3 周, L60) 借鉴 Mio 日记 → preference 提炼

### 5.2 屏幕感知 ↔ v2 RC-7 真 modality (高度重叠)

**Apeireth RC-7** (§1.2): xcap 跨平台仅 Windows 真接; 估时 XcapVisionBackend 骨架 + 2 error path test = 2-3 天 (不需硬件); 真截屏需 Windows + 多显示器.

**借鉴点**: Mio 屏幕感知路径调研跟 RC-7 真实施**完全重叠**, 派 1 sub-agent 一次性产出; 估时 1 周, 跟 R15+ RC-7 真实施并行 (per RC-7 §4 "R15+ 优先做不需硬件的 1 周").

### 5.3 QQ ↔ v2 NapCatQQ (LOW, 不排 critical path)

**Apeireth v2 gateway**: 即时通讯 = LOW (per L29), NapCatQQ 在 LOW 段 (L79).

**借鉴点**: P2 后续调研, 跟 N.E.K.O/Firefly 同类合并, 主代理可选不借鉴.

---

## 6. 借鉴实施路径 (按优先级 + 估时)

| P | # | 项 | 方式 | 估时 | 输出 |
|---|---|---|---|---|---|
| **P0** | 1 | **日记功能真调研** (反思+写回耦合范式, **核心**) | 📄+🔬 | 1 周 | `r7-mio-diary-research.md` ≤200 行 |
| **P0** | 2 | 决策: cognitive module 加 reflection_writeback pipeline trait | 主代理 | 1 天 | spec |
| **P1** | 3 | 📦clone + Windows 本地优先调研 (用户想法 #1 对齐) | 📦+🔬 | 1 周 | `r7-mio-portable-research.md` ≤300 行 |
| **P1** | 4 | R20 preference_learning 真实施 (跟 Mio 日记并行) | 派 sub-agent | 2-3 周 | L60 DEFERRED→WIRED |
| **P1** | 5 | R22 reflection 真实施 (跟 Mio 日记反思部分直接对标) | 派 sub-agent | 1 周 | L62 DEFERRED→WIRED |
| **P1** | 6 | 屏幕感知调研 (跟 RC-7 真实施合并) | 🔬 | 1 周 | `r7-mio-screen-research.md` ≤200 行 |
| **P1** | 7 | Live2D 渲染调研 (跟 N.E.K.O/AIRI 合并) | 🔬 | 1-2 周 | `r7-mio-live2d-research.md` ≤200 行 |
| **P2** | 8 | portable binary 真实施 (用户想法 #1, post-release 2027-Q2) | 📦+派 | 2-3 周 | `APEIRETH_HOME` portable binary |
| **P2** | 9 | cognitive module reflection_writeback pipeline trait 真实施 | 派 | 1-2 周 | cognitive 增维 |
| **P2** | 10 | QQ 接口 (NapCatQQ, LOW, 主代理可选不借鉴) | 派 | 1-2 周 | gateway QQ route |
| **P2** | 11 | 语音 (ASR/TTS) 真实施 (跟 RC-7 重叠) | 派 | 2-3 周 | RC-7 真 backend impl |

**借鉴方式汇总**: 📦clone 2 + 📄看文档 1 + 🔬派 sub-agent 5 + 主代理亲做 1.

---

## 7. 主代理决策建议 + 0 装诚实标

### 7.1 物种化借鉴 vs frontend 借鉴 vs backend 借鉴 占比

| 维度 | 占比 | 理由 |
|---|---|---|
| **物种化借鉴** | **45%** | Windows 本地优先 ↔ portable binary (§3.1) + single-user ↔ per-user 塑形 (§3.3) + 日记反思写回 ↔ 物种化塑形物理基础 (§3.2) |
| **backend 借鉴** | **35%** | 日记反思写回 ↔ cognitive module 增维 (§5.1, **核心**) + 屏幕感知 ↔ RC-7 真 modality (§5.2) + QQ (§5.3, LOW) |
| **frontend 借鉴** | **20%** | Live2D + Windows 本地 UI (§4.1-4.2) — 物种化 frontend 具体形态 |

**vs 之前 Round 10 你you-list L50 (屏幕感知 + 桌面优先)**:
- 之前: 屏幕感知 (HIGH backend) + 桌面优先 (LOW frontend) — 主打 RC-7 vision
- 现在: 物种化架构 (per-user 塑形 + 本地优先 + 日记反思写回) — 从 "frontend 借鉴" 升级到 "物种化借鉴 (本地优先架构 + 日记反思)"
- **差异**: 借鉴维度从 "屏幕感知 modality" 升级到 "per-user 塑形本地架构 + 日记反思写回"

### 7.2 0 装诚实标

| 失守 | 详情 | 修法 |
|---|---|---|
| **0 实测** | 未 git clone Mio (github 直连防火墙 + web_search auth fail 双 0 装), 仅基于 L11 + L50 + 真理解 §1-3 + RC-7 推论 | 真实施前主代理亲验: git clone (走代理/VPN/git protocol 9418) + 看 README |
| **0 数字漂移** | Star / 代码规模 / License / 活跃度 / commit 时间 全 "未实测" | 主代理亲验时实测 |
| **0 装诱导 prevention** | 不假装 "Mio 日记具体怎么实现" + "Mio 屏幕感知是 GetForegroundWindow 还是 xcap" + "Mio QQ 协议栈是 NapCatQQ 还是 go-cqhttp" — 仅推论思路 | sub-agent 真调研时看 Mio 代码 + README + spec |
| **github 直连 0 装诚实** | github.com:443 被挡, git clone [exit code: 1] 21s timeout; **不假装** "我读了 Mio README" | 主代理亲验走代理/VPN |

### 7.3 主代理下一步 (推荐)

| # | 行动 | 估时 | 阻塞 |
|---|---|---|---|
| 1 | **派 sub-agent 真调研 Mio 日记实现** (§3.2 + §5.1, **P0 核心**) | 1 周 (派单 §6 P0 #1) | 0 |
| 2 | **主代理亲 git clone Mio + 看 README + 看 spec** (走代理/VPN, **必做**) | 1-2 天 | 0 |
| 3 | **决策冻结: cognitive module reflection_writeback pipeline trait 是否增维** (§6 P0 #2) | 1 天 (主代理亲做) | #1+#2 done |
| 4 | **跟 R20 preference_learning + R22 reflection 真实施并行** (handbook L60+L62, 关键!) | 3-4 周 (R20+R22 critical path) | R10 OrganKind 决策 |
| 5 | **写 `r7-mio-diary-research.md`** ≤200 行 | 1 周 (派单内) | #1+#2 done |
| 6 | **写 `r7-mio-portable-research.md`** ≤300 行 (用户想法 #1 对齐) | 1 周 (派单 P1 #3) | 真实施调研 |

**总估时**: P0 (1 周) + P1 (4-6 周) + P2 (4-6 周) = **2-3 月 critical path**, 跟 v2 release critical path 5-7 周并行 (R20+R22 占大头), **不冲突**.

**关键同步点**:
- R20 preference_learning (2-3 周) + R22 reflection (1 周) + 日记反思写回 cognitive module 增维 (1-2 周) **三者并发** (3-4 周 critical path)
- Windows 本地优先 portable binary (P1 调研 1 周 + P2 实施 2-3 周) 排 post-release (用户想法 #1 估 2027-Q2), 跟 v2.0.0 release 解耦

---

_R7-Mio 写于 2026-08-28, 4h 限, 物种化借鉴维度修订, 0 实测诚实标 (github 直连 + web_search 双 0 装), 主代理亲验必做. 真账就位._
