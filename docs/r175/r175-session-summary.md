# R175 session summary — R170-R174 终极目标进度盘点

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R175 (summary + next-step planning)
> **日期**: 2026-08-13
> **主人授权**: 全按你的建议来 + 时间和 token 充裕, 干到底

---

## 0. R170-R174 5 commit 总览

| Commit | R | 主题 | 净影响 |
|---|---|---|---|
| `0ea01d21` | R170 | sovereignty Hyperlight micro-VM 调研 + ToolIsolation 设计 | +1 doc |
| (同) | R171 | relation SurrealDB 多模型调研 + GraphBackend 设计 | +1 doc |
| (同) | R172 | apeireth-voice MiniMax LIVE TTS 真接 (122KB MP3 EN + 118KB ZH) | +1 module + 1 demo + 11 tests |
| `4a1226c3` | R173 | "放最后" 5 模块接口完整性盘点 | +1 doc |
| `b77b2d85` | R174 | apeireth-tool-fetch HTTP 真接 (Tier 1.5 唯一缺项补完) | +1 demo + LIVE 验证 example.com/iana.org |

**净影响**:
- 2 个 LIVE 真接能力新增 (TTS R172 + HTTP R174)
- 2 份调研设计文档 (Hyperlight + SurrealDB)
- 1 份接口完整性盘点 (R173)
- 1 份 LIVE 证据文档 (R174)
- 11 个新增单测 (R172 minimax_live 模块)
- 总测试 5618 → 5629+ (R172+R174 后)
- cargo check workspace: 0 errors, 0 actionable warnings

---

## 1. 终极目标进度盘点 (R175 重新盘点)

### 1.1 5 战区现状

| 战区 | 状态 | 关键 LIVE 证据 |
|---|---|---|
| 终端 Coding Agent (TUI) | 🟡 骨架可跑, 未接真后端 | R155 TUI × runtime 桥已建 |
| LLM 网关 | ✅ 真接 MiniMax-M3 | R168 LIVE 8 段 markdown |
| Multi-Agent | ✅ council 7 advisors + group_chat | R164-R169 测试全过 |
| 长期记忆 | ✅ memory + memory-extensions (lightmemo) | R146 |
| 工具协议 | ✅ Tier 1 5/5 + Tier 1.5 fetch + 协议桥 | R140 + R174 |

### 1.2 Tier 1/1.5/2 现状 (per vcp-plugin-gap-analysis-2026-08-12.md)

| Tier | 项 | 状态 |
|---|---|---|
| 1.1 | tool-filesystem | ✅ R140 |
| 1.2 | tool-shell | ✅ R140 |
| 1.3 | tool-browser | ✅ R140 |
| 1.4 | tool-codesearch | ✅ R140 |
| 1.5 | tool-fetch | ✅ R174 |
| 1.6 | tool-image-gen | 接口 ready (放最后) |
| 1.7 | tool-image-process | 接口 ready (放最后) |
| 1.8 | tool-search | ✅ (并入 tool-fetch) |
| 1.9 | protocol-bridge | ✅ R146 |
| 2.1 | memory-lightmemo | ✅ R146 |
| 2.2 | relation SurrealDB-style | 🟡 in-memory, SurrealDB 真接待 R177+ |
| 2.3 | sovereignty Hyperlight | 🟡 设计已 R170, 真接待 R176+ |
| 2.4 | voice TTS | ✅ R172 |
| 2.5 | voice STT | 接口 ready (放最后) |
| 2.6 | voice 声纹/唤醒词 | 接口 ready (放最后) |

### 1.3 核心安全层

| 项 | 状态 |
|---|---|
| 3 不可变脊柱 (Self-Disable / L0 HA / 13-key verdict) | ✅ 0 触碰 |
| 24 LOCKED 形式撤销 | ✅ R148 |
| workspace.version 1.2.0 | ✅ 0 改 |
| V0.5 / V1136 / 9键 原始 | ✅ 0 改 |

---

## 2. 还缺什么 (按 ROI 排序)

| 优先级 | 项 | 工作量 | ROI |
|---|---|---|---|
| 🥇 高 | **TUI 接入真后端** | 1-2 days | 巨大 (UX 闭环) |
| 🥇 高 | **apeireth-relation SurrealDB 后端真接** (R171+1~6) | 5.5 days | 高 (持久化) |
| 🥇 高 | **apeireth-sovereignty Hyperlight isolation** (R170+1~5) | 4 days | 高 (安全升级) |
| 🥈 中 | **GitHub 每个模块的优秀项目调研 + 升级借鉴** | 持续 | 中 |
| 🥈 中 | 上下文协议互通 (OpenAI/Anthropic/Gemini 全兼容) | 3 days | 中 |
| 🥉 低 | VCP 官网完整调研 (主人之前指示分批看) | 1 day | 低 (已 R140+R146 吸收) |
| 🥉 低 | 占卜/酒馆/论坛 冻结模块复活 (主人指示不放最后) | — | 不做 |
| 最后 | STT / 声纹 / 唤醒词 / 生图 / 图处理 真接 | per R173 doc | 最后阶段 |

---

## 3. R175+ 路线 (5 commit 计划)

### 3.1 近期 (R175-R180)

| R | 主题 | 工作量 |
|---|---|---|
| R175 | session summary (本档) | 0.5h |
| R176 | TUI 接入真后端 (RuntimeBridge 升级) | 1-2 days |
| R177 | apeireth-relation SurrealDB 真接 (R171+1~6 实施) | 5.5 days |
| R178 | GitHub per-module 调研 + 升级 (apeireth-voice/council/tool/...) | 持续 |
| R179 | apeireth-sovereignty Hyperlight 真接 (R170+1~5 实施) | 4 days |
| R180 | protocol-bridge 上下文协议全兼容 (OpenAI/Anthropic/Gemini) | 3 days |

### 3.2 终极阶段 (R180+)

| R | 主题 |
|---|---|
| R200+ | STT / 声纹 / 唤醒词 / 生图 / 图处理 真接 |
| R210+ | VCP 全部插件兼容层最终验收 |
| R220+ | TUI 完整接入 + UX 优化 |
| R230+ | 终极目标 = 全做全补弱 + 一体化优美 — 验收 |

---

## 4. 0 触碰声明 (R170-R174 全程)

- 3 不可变脊柱: 0 触碰 (R170/R171 设计明确隔离, R172/R174 LIVE 验证都不进入 sovereignty)
- workspace.version 1.2.0: 0 改
- 24 LOCKED crate 入口签名: 0 改
- V0.5 / V1136 / 9键 原始: 0 改
- STUB_MODE compile-time hardcode: 0 改
- 5 战区骨架: 0 改

---

## 5. 主人起床后请审视

| 文件 | 路径 |
|---|---|
| R170 调研 | `docs/r170/r170-hyperlight-research.md` |
| R171 调研 | `docs/r171/r171-surrealdb-research.md` |
| R172 LIVE | `docs/r172/r172-minimax-live-voice.md` + 2 真 MP3 文件 |
| R173 审计 | `docs/r173/r173-deferred-interfaces-audit.md` |
| R174 LIVE | `docs/r174/r174-http-fetch-live.md` |
| R175 总结 (本档) | `docs/r175/r175-session-summary.md` |

**关键问题**:
1. TUI 接入真后端 (R176) 是现在最佳路径, 还是应该先做 R177 SurrealDB / R179 Hyperlight 真接?
2. GitHub per-module 调研 (R178) — 主人想针对哪些模块优先?
3. 上下文协议全兼容 (R180) 是否需要 — 取决于是否要走 Anthropic / Gemini 兼容?

按 R175+ 路线, 楚零将自主继续 R176 (TUI接入) → R177 (SurrealDB) → R179 (Hyperlight) → R180 (协议全兼容)。
