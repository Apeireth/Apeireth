> ⚠️ 2026-09-05 对账批标注：本文为历史记录，写作时数字属实于当时。当时实测基线（2026-09-05）：17 crates / 3120 passed / 0 failed / 13 ignored / workspace.version 2.0.0-rc.1（现行基线见 INSTALL.md）。

# 170+ 项目清单对 Apeireth v2.0.0-rc.1 作用真账 — 2026-08-28

> **作者**: sub-agent (主代理 Mavis 派单, 时间紧 ≤ 4h)
> **用途**: 给主代理决策参考 — 170 项目按 10 类分组, 标 HIGH/MED/LOW/NONE, TOP 10 推荐吸收顺序, 用户末尾 2 想法随记评估
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
| 1 | AI Agent 框架与 Harness | 40 | LLM harness / multi-agent | **HIGH** (15-20 直接可对齐) | v2 已对齐同类 Agent 框架/多智能体编排公开设计 (含 MCP 协议); 调研不重叠 |
| 2 | 开发工具与 CLI 增强 | 48 | dev tools / OCR / memory / scraper | **MED** | 同领域参考 (memory / OCR / vector db / scraper 跟 B/C 块有关) |
| 3 | 金融/量化/财务 | 32 | finance / trading | **LOW** | 远领域 (v2 非金融项目), 仅个别金融量化/技能合集类项目对趋势参考 |
| 4 | 自建音乐流媒体 | 5 | music server | **NONE** | 完全无关 (v2 是 Agent, 不是 media server) |
| 5 | 安全与隐私 | 15 | password / p2p / sandbox | **MED** | 跟 O-1 安全优先哲学锚相关, 部分可对齐 |
| 6 | AI 语音/TTS | 6 | TTS / ASR / OCR | **HIGH** (3-4) | 跟 RC-7 Perception (R14 真 modality) 对接, ASR/TTS 后端候选 |
| 7 | 即时通讯/机器人 | 2 | IM bot | **LOW** | 仅 1-2 项目, v2 gateway 已有 SSE, 不直接对齐 |
| 8 | AI 伴侣/VTuber/桌宠 | 27 | VTuber / desktop pet / Live2D | **HIGH** (5-8 直接可对齐) | 跟 RC-7 真 modality 强相关 (Live2D 视觉形象 = modality 之一), 用户本位 |
| 9 | 地理信息 | 1 | GIS | **NONE** | 单项目, 完全无关 |
| 10 | 资源汇总 | 14 | awesome / wiki / skill | **LOW** | 趋势合集, 仅 1-2 个 (LLM 笔记 / 技能集等) 参考 |

**总: 190 项目 (含 1 项重复计 x2 + 资源类重复), 净 170+**. 相关度分布: **HIGH ≈ 25 项目, MED ≈ 40, LOW ≈ 90, NONE ≈ 15**.

---

## 2. 高价值项目 TOP 10 (HIGH 相关, P0/P1 参考)

> 参考方式: `📦clone` = git clone 看代码, `📄看文档` = 看 README/spec, `🔬派 sub-agent` = 派调研, `⏸️不吸收` = 仅参考

### P0 (1 周内立即吸收, 真实施)

| # | 项目 | URL | 一句话 | 参考点 | 优先级 | 方式 |
|---|---|---|---|---|---|---|
| 1 | **情感知性陪伴工程（五维记忆类）** | https://github.com/Project-N-E-K-O/N.E.K.O | 网络型情感知性生命体, 五维记忆 + Live2D/VRM/MMD | 五维记忆系统 ↔ v2 cognitive.memory_recall + memory_writeback; Live2D 多形态 ↔ RC-7 Perception 真 modality (视觉子模态) | **P0** | 📄看文档 + 🔬派 sub-agent 调研 (1 周, 写 `r7-perception-research.md`) |
| 2 | **虚拟伴侣工程（实时陪聊类）** | https://github.com/moeru-ai/airi | 2.2 万 Star 虚拟伴侣, 实时陪聊 | Live2D + 主动消息 + 长期记忆 ↔ v2 五维记忆 + RC-7 主动 perception; Star 数高 = 社区验证 | **P0** | 📄看文档 + 📦clone 看 Live2D 渲染 pipeline |
| 3 | **VTuber 工程（语音+视觉+工具调用链路）** | https://github.com/Open-LLM-VTuber/Open-LLM-VTuber | 语音 + 视觉 + 工具调用 + Live2D (Cubism 5) | 完整 ASR→LLM→TTS→Live2D 链路 ↔ v2 gateway SSE pipeline (B 块) | **P0** | 📦clone 看 ASR/TTS 抽象层 |
| 4 | **语音伴侣工程（原声 TTS + MCP 工具链）** | https://github.com/ff-ai/firefly-companion | 角色音色原声 TTS + MCP 工具链 | 原声 TTS 集成 ↔ v2 TTS modality; MCP 工具链 ↔ v2 gateway 工具注册 | **P0** | 📄看文档 (TTS 接入模式) |
| 5 | **桌面本地优先 AI Agent** | https://github.com/ochiru520/Mio | Windows 本地优先 AI Agent, 对话/记忆/日记/IM/语音/Live2D/屏幕感知 | 屏幕感知 ↔ RC-7 Perception 视觉模态; 桌面优先 ↔ v2 portable 部署 (用户想法 #1 印证) | **P0** | 📄看文档 (屏幕感知抽象) |

### P1 (1 月内排上, 趋势参考 + 部分吸收)

| # | 项目 | URL | 一句话 | 参考点 | 优先级 | 方式 |
|---|---|---|---|---|---|---|
| 6 | **桌面伴侣工程（睡眠模式类）** | https://github.com/warashi/warashi | 免费开源桌面伴侣, Live2D + 长期记忆 + 主动聊天 + 睡眠模式 | 睡眠模式 ↔ v2 organ cycle (L0-L5 UpgradeCycle Stage 5); 主动聊天 ↔ v2 council/judge | **P1** | 📄看文档 |
| 7 | **语音合成训练工程（角色音色类）** | https://github.com/RVC-Boss/GPT-SoVITS | 角色音色训练 + 语音合成 | TTS backend 候选 ↔ RC-7 TTS modality (P2 实施) | **P1** | 📄看文档 (Python 端, 看 Rust binding 可行性) |
| 8 | **语音转写工程（C++/CUDA 实现）** | https://github.com/SYSTRAN/faster-whisper | 语音转写 (C++/CUDA) | ASR backend 候选 ↔ RC-7 ASR modality | **P1** | 📄看文档 |
| 9 | **模块化 VTuber 工程（多进程架构）** | https://github.com/foxabbage/Megumi | 模块化 AI VTuber, Core Server + 多进程 + WebSocket | 多进程 + WebSocket ↔ v2 gateway SSE (B 块), 架构参考 | **P1** | 📄看文档 |
| 10 | **桌宠 Agent 工程（低开销/插件化）** | https://github.com/BDFFZI/Alife | 桌宠 Agent, 一键安装 + 极低开销 + 插件化 | 极低开销 + 插件化 ↔ v2 portable binary (用户想法 #1); 插件 ↔ v2 plugin-authoring-guide | **P1** | 📄看文档 |

### 参考方式: P0 派 1 sub-agent 调研 TOP 5, P1 主代理排下月 (4-6 月并行 critical path 内).

---

## 3. 同领域项目 (MED 相关, 趋势参考)

> 不吸收内容, 仅了解趋势, 主代理团队知道就行

- **开发工具**: 同类工程 7 项 (可观测 / LLM 爬虫 / 深度搜索 / 工具集成 / 代码记忆 / 记忆系统等方向) — 结论: v2 暂不需要或思路已覆盖, 仅记忆系统命名可参考
- **安全**: 同类工程 3 项 (Rust 密码库 / 蓝牙 Mesh 聊天 / 敏感信息脱敏) — 结论: 脱敏方向跟 v2 P0 governance credential disclosure 相关, 其余仅趋势
- **AI Agent 框架 (调研不重叠)**: 同类工程十余项 (多智能体编排 / 任务控制 / CLI Agent 等方向) — **v2 已对齐或与 research/source 重叠**, 不重复
- **金融 (趋势)**: 同类工程 3 项 (量化交易 / 回测框架方向) — 远领域, 仅用户有 finance 兴趣时参考

---

## 4. 低价值项目 (LOW + NONE, 简短列名)

**LOW (≤1 行 each)**: LOW 档 ≈ 90 项同类工程 (Agent 框架/多智能体、开发工具、安全隐私、金融量化、爬虫采集、OCR、资源合集等方向, 明细已聚合为中性汇总) — 结论: 均为趋势参考, 不派单

**NONE (完全无关)**: 音乐流媒体类 5 项 + GIS 类 1 项 (GIS 重复算 LOW)

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

**参考**: 桌宠工程的"一键安装"做法 + v2 portable binary 模式 (对齐 A 块 Stage 5 L0-L5 UpgradeCycle portable 模式)

**推荐**: **P1 排上 (post-release, v2.0.0 release 后 1-2 周做)**. v2 release 估 2027-Q1, post-release 估 2027-Q2 1 周.

### 5.2 "Apeireth 可以做自己刷视频的模块, 对接入现代生活, 网络"

**对接 v2 工程现状**: RC-7 Perception 真 modality (R14 调研就位, 2-3 周需硬件到位). 视频 = modality 之一 (跟音频/图像/文本并列). 用户本位 + O-2 前人肩上 (对齐 §2 P0 HIGH 同类工程做法).

**实施路径 (估时 2-4 周)**:
1. 派 sub-agent 调研视频 modality (R14 真 modality 子集), 写 `r14-video-modality-research.md` (≤200 行, 真账)
2. 视频 backend impl: 视频源 (本地文件 / 网络 URL / RTSP / WebRTC) + 解码 (ffmpeg / openh264) + 帧采样 (per N 秒) + 视觉 embedding (CLIP / MobileCLIP)
3. 视频 → 视觉 modality → cognitive.perception slot (R14)
4. 对齐同类工程做法: VRM/MMD 视觉形象 + 视觉感知 pipeline + Live2D 渲染 (§2 P0 三项同类工程)
5. 测试: E2E 视频感知 (D 块 E2E baseline)

**风险**: 视频解码 CPU/GPU 开销 (跟 O-6 永远追求最优冲突, 需 strip + lto + 异步解码)

**推荐**: **P2 后续 (post-release, 估 2027-Q3, 2-4 周)**. 排在 R14 真 modality 硬件到位之后 (critical path Week 5-6 之后).

---

## 6. 主代理决策建议 + 派单顺序

### P0 (1 周内, 立即可吸收)

1. **派 1 sub-agent 调研 TOP 5 P0 HIGH 项目** (§2 P0 五项同类工程), 写 5 个调研真账 (各 ≤200 行, 总 ≤1000 行), 总估时 1 周, 并行 4-6 月 critical path
2. **对齐同类桌宠/Agent 工程的 portable 模式** 为便携 U 盘做技术调研 (1-2 天, 跟派单 1 并行)
3. **对齐同类工程五维记忆设计** → v2 cognitive.memory_recall / memory_writeback 增维 (跟 R20 preference_learning 真实施并行, 2-3 周)

### P1 (1 月内, 趋势参考 + 部分吸收)

4. **派 1 sub-agent 调研 §2 P1 四项同类工程** (TTS/ASR/多进程/睡眠模式), 写 4 个调研真账, 估时 2-3 周
5. **对齐同类安全工程做法 (Rust 密码库 + 敏感信息脱敏)** → v2 O-1 安全优先 (per §10 LOCKED 0 触碰约束, 改仅 3 hook 之外)
6. **用户想法 #1 "便携 U 盘"**: 排 post-release (v2.0.0 release 后 1 周, 估 2027-Q2)

### P2 (后续, 1-3 月后)

7. **用户想法 #2 "刷视频模块"**: 派 sub-agent 调研 R14 视频 modality, 写 `r14-video-modality-research.md`, 估 2-4 周, 排在 R14 硬件到位之后 (估 2027-Q3)
8. **LOW + NONE 项目**: 不吸收, 仅主代理团队知道 (本真账已列分级, 不再派单)
9. **资源汇总 (UI 组件库 / 免费资源清单 / LLM 笔记等)**: 趋势合集, 主代理阅后归档

### 决策建议摘要

- **不要全吸收** — 170 项目仅 25 HIGH, 大部分 LOW/NONE 是 noise
- **优先派 sub-agent 调研 P0 TOP 5** — 真账 + 吸收路径, 估 1 周, 跟 critical path 并行
- **用户 2 想法随记都 P1/P2 排上**, 不阻塞 critical path
- **吸收原则**: per O-2 前人肩上, 必先语义对齐 v1 legacy, 其次 research/source, 最后本清单 (顺序不重叠)
- **0 装诚实标**: 本真账关联度未实测 (未 git clone/grep), 仅基于用户清单 + v2 handbook 已知信息评估; 真实施前主代理必亲验

---

## 真账完成度

- 6 段全写 (分类总览 / TOP 10 / MED / LOW+NONE / 用户想法 / 派单建议)
- 行数: ≤250 行 (per 任务 brief 约束)
- 不写真账以外的 file (per 任务 brief 约束)
- 不 git add / commit / push (per 任务 brief 约束)
- 主代理决策参考就位
