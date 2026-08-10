# R34 架构改进调研 (工程师角度) (2026-08-09)

**作者**: Mavis
**范围**: 主人 8-09 00:51 指令 "工程师角度, 有的地方需不需要合并"
**前置**: R30 + R31 + R32 + R33

---

## 1. 主拍 (TL;DR)

**91 个 crate 是过度拆分** (按 Cargo.toml 数过). 1.0 仅 12 个, 0.1 占位 70+, 编译慢, 维护成本高, 跟主人在 R25 锁的"精简" 战略有偏差.

按 ROI 排 5 个最值得改的 (砍 + 合并, 不动 LOCKED 边界):

| 序 | 改动 | 类型 | 估时 | ROI | 紧迫 |
|---|---|---|---|---|---|
| 1 | **91→40 瘦身 (合并占位 crate)** | 合并 | 3-5d | ★★★★★ | release 前必做 |
| 2 | **observability 4 合并** (cache/observability/metrics/tracing) | 合并 | 1d | ★★★★★ | 立即 |
| 3 | **provider 5 合并** (5 个 skeleton 1 个 enum) | 合并 | 1d | ★★★★ | 主人想真接 provider 时 |
| 4 | **ProtocolRouter + ProtocolGateway 2 层砍 1 层** | 简化 | 1d | ★★★ | 接 2+ provider 时 |
| 5 | **9 organ 部分合并** (memory+life_force, perception+consciousness) | 合并 | 3-5d | ★★ | 1.0 release 前 |

不动: 8 项不修改承诺 + R11 LOCKED enum + R17 战役 0 7 阶段 + R19 cycle/verdicts + R25 改瘦 + R26 9 器官 5 nav (organ page UI 名字).

---

## 2. 91 个 crate 现状 (按版本分)

| Version | 数 | 占比 | 评估 |
|---|---|---|---|
| **1.0.0** | 12 | 13% | 真正能用, 主人下个 release 候选 (R30 + R32 已覆盖大部分) |
| **0.1.0** | 70+ | 77% | 大部分是 "骨架/占位/未真接", R17 战役 0 留下没动 |
| 无 Cargo.toml / 子目录 | 9 | 10% | 测试 fixture / 子模块 / 文档 |

**0.1.0 里又分两类**:
- **A 类: 应该合并 (重复)** — cache / observability / metrics / tracing; 5 provider; 4 sdk; 4 mcp variant; 2 pipeline; 2 integration
- **B 类: 该独立但太散 (主路径相关)** — core / memory / cognition / asi / value / relation / motivation / constraint / consciousness / perception / pipeline / protocol / tool-* / tui / api / bench / verify / eval / workflow / team-lead / etc.

A 类砍掉, B 类保留. 91 → 40 是合理目标.

---

## 3. 改动 1 (★★★★★ 立即): observability 4 合并

**现状 (4 个 crate)**:
- `apeireth-cache` (0.1.0) — caching
- `apeireth-observability` (0.1.0) — observability 5 component health
- `apeireth-metrics` (0.1.0) — metrics counter
- `apeireth-tracing` (0.1.0) — tracing span

**重复度**:
- observability 5 component health 跟 metrics counter 都做 "看系统状态", 区别是 health 是 bool / metrics 是数字
- cache 跟 metrics 都做"读快" 的事 (一个 mem cache / 一个 counter cache)
- tracing 跟 observability 都做"看 log"

**方案**: 合并到 1 个 `apeireth-telemetry` crate, 4 module (`cache` / `metric` / `trace` / `health`).

**估时**: 1d (改 import path + 验证 0 regression)
**触发**: 任何时候 (现在)
**不动**: 0 (都没 LOCKED 边界)

---

## 4. 改动 2 (★★★★ 立即): provider 5 合并

**现状 (5 个 crate 0.1.0 skeleton)**:
- `apeireth-provider-claude-code` — Claude Code (Anthropic SDK 0.2.112 8 工具)
- `apeireth-provider-codex` — Codex (OpenAI)
- `apeireth-provider-copilot` — GitHub Copilot
- `apeireth-provider-gemini-cli` — Gemini CLI
- `apeireth-provider-opencode` — OpenCode

**重复度**:
- 5 个都是 skeleton (8 工具 + 3 ModelKind + 8 TOOL_WHITELIST + 4 K-1 校验, 字段级 1:1 翻译)
- 5 个 Cargo.toml 几乎一模一样
- 5 个 mod 几乎一模一样

**方案**: 合并到 1 个 `apeireth-provider` crate, 5 provider 用 `enum Provider { ClaudeCode, Codex, Copilot, GeminiCli, OpenCode }`, 启动时 1 个配置项选 provider.

**估时**: 1d (5 crate 合并, 字段级 1:1 保留)
**触发**: 主人想真接 1 个 provider 时
**不动**: 5 个 provider 的字段级接口 (都是 8 工具 + 3 ModelKind + 8 TOOL_WHITELIST)

---

## 5. 改动 3 (★★★ 中期): ProtocolRouter 砍 1 层

**现状 (2 层抽象)**:
- `apeireth-protocol/src/router.rs` — `ProtocolRouter` 把 4 个 `ProtocolAdapter` 按 kind dispatch
- `apeireth-protocol/src/gateway.rs` (R10 新加) — `ProtocolGateway` 把 `ProtocolBridge` (7 kind: 4 LLM + ACP + MCP + OpenClaw) 按 kind dispatch

**重复度**:
- 2 层都是 "按 kind 找实现", 逻辑一样 (HashMap 查表)
- 4 个 ProtocolAdapter 跟 ProtocolBridge trait 不一样 (1 是 sync, 1 是 async) — 不直接互替
- 但 router 是 sync dispatch, gateway 是 async dispatch, 2 个用法不一样

**方案 A (保守)**: ProtocolGateway 直接装 4 个 ProtocolAdapter (不 wrapper), 砍 router
**方案 B (干净)**: 4 个 ProtocolAdapter 改为实现 ProtocolBridge trait (加 async), router 砍

**估时**: 1d
**触发**: 接 2+ provider 真正要切换时 (R32 U7 已接 SemanticRouter, 这是 facade over 4 LLM 协议)
**不动**: 4 协议归一 字段级 1:1 (VCP protocolBridge.js 真值)

---

## 6. 改动 4 (★★★ 中期): 其他重复 crate 合并

| 重复 | 数量 | 合并方案 | 估时 |
|---|---|---|---|
| **sdk 4** (lark / livekit / sandbox / voice) | 4 → 1 `apeireth-sdk` | enum SDK { Lark, LiveKit, Sandbox, Voice } | 1d |
| **mcp 4** (mcp / mcp-relay-image / mcp-ssh / mcp-winrm) | 4 → 1 `apeireth-mcp` | enum Backend { Standard, RelayImage, Ssh, Winrm } | 1d |
| **pipeline 2** (pipeline / pipeline-g5) | 2 → 1 `apeireth-pipeline` | 留主, pipeline-g5 是 0.1 占位 merge 进来 | 0.5d |
| **integration 2** (integration-e2e / integration-r20-stage4) | 2 → 1 `apeireth-integration` | e2e 留, stage4 是 0.1 fixture merge | 0.5d |
| **i18n / lark / livekit / image-prompt / oauth / metrics / ...** | 0.1 占位 30+ | 合并到 1 个 `apeireth-stubs` | 2d |

**总估时**: 5d
**不动**: 0 (都是 0.1 占位)

---

## 7. 改动 5 (★★ 长期): 9 organ 部分合并

**现状 (9 organ)**:
- perception (5 通道)
- cognition (R19 循环)
- consciousness (梦状态机)
- memory (sqlite)
- motivation (autonomy + intrinsic + value 3 测度)
- value (5 维度)
- relation (4 kind)
- action (3 trait)
- life_force (1 health)

**重复度**:
- memory + life_force 都看 "持久" 状态 (episode count vs endurance)
- perception + consciousness 都看 "意识" 状态 (5 通道 vs 梦状态)
- motivation + value 都看 "价值" 状态 (autonomy 3 vs 5 dim)

**方案**: 9 → 6 (合并 3 对)
- `memory_life` (1 snapshot 函子 = memory.count + life_force.endurance)
- `perception_conscious` (5 通道 + 梦状态)
- `motivation_value` (3 测度 + 5 维)
- 留: cognition, relation, action

**估时**: 3-5d (要重写部分 snapshot, 不动 API 名字以保 R26 LOCKED)
**触发**: 1.0 release 前, 主人说 "9 个器官太散了" 时
**不动**: R26 9 器官 page UI 名字 (snapshot 合并, page 仍叫 memory/life_force 等)

---

## 8. 决策日志

1. **不砍掉 9 organ 一类 LOCKED 名字**: 主人 R26 锁了 9 器官 page UI 名字. 内部 snapshot 合并, 外部仍叫 9 器官, 0 触.
2. **不砍 apeireth-tui 1 个 bin**: TUI 是 1.0 核心, 91 里就它特殊.
3. **不砍 0.1 主路径 crate** (core / memory / asi / etc.): 这些是真有内容的, 不在 70+ 占位里.
4. **不砍 R11 LOCKED enum 跟 7 阶段**: 主哲学锚 #1 0 触.
5. **估时保守**: 都是 1-5d. 给主人完整 plan 选.

---

## 9. 推荐路径 (1+2 必做, 3+4 中期, 5 长期)

| 阶段 | 改动 | 估时 |
|---|---|---|
| **R35 立即** | observability 4 合并 + provider 5 合并 | 2d |
| **R36 1.0 前** | 91→40 瘦身 (合并 sdk/mcp/pipeline/integration/i18n...) | 5d |
| **R37 1.0 前** | ProtocolRouter 砍 1 层 + 9 organ 合并 | 4-6d |
| R38+ 长期 | 看 R30/R32 实战反馈再调 |

---

## 10. 不动边界 (R34 0 触)

- ✅ 8 项不修改承诺 0 触
- ✅ R11 LOCKED enum 0 触
- ✅ R17 战役 0 7 阶段 0 触
- ✅ R19 cycle / verdicts 0 触
- ✅ R25 改瘦路径 0 触
- ✅ R26 9 器官 page UI 名字 0 触
- ✅ 1.0 version 12 个 crate 的字段级接口 0 触

---

## 11. 主人接下来可选

A. **R35 (2d)**: 立即砍重复 (observability + provider)
B. **R36 (5d)**: 91→40 瘦身, release 前 1 次
C. **R37 (4-6d)**: ProtocolRouter + 9 organ 合并, 1.0 release 前
D. **别的方向** (mem0 借鉴 / 后端升级 / 别的)
