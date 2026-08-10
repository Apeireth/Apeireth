# ADR 0025: Rival Blueprint — 7 竞品对比 + Apeireth 差异化定位

> **状态**: 🟢 Accepted (R20 阶段 6 主人 2026-08-05 拍板, per `docs/competitive-analysis-2026-08-05.md`)
> **commit 锚**: `docs/competitive-analysis-2026-08-05.md` (Codex CLI 2026-08-05 17:40 审) + 1.0 release 战略层
> **最后更新**: 2026-08-05

---

## 1. 背景 (Context)

Apeireth 1.0 release 需明确"我跟 7 个同类项目比, 我在哪, 我打谁, 我不打谁"。

**问题**:
- 7 个同类项目 (3 联网 + 4 本地源码): AutoGen (60K stars), CrewAI (56K), Letta (24K), hermes-agent-rs (18 crate), memoryos-rust (9 crate), vcptoolbox (Node.js), honcho (Python)
- Apeireth 是 Rust 阵营 crate 最多 (42 → 67 in 1.0), 但功能密度不一定最高
- 主人 2026-08-05 拍板: "1.0 release 1 用户 1 年 500K 行" 场景, 不是"追 stars"

**约束**:
- 不盲目追 stars (Python 生态优势不可比, per `competitive-analysis-2026-08-05.md` §0)
- Rust 阵营最完整 (per 1.0 release 战略)
- 差异化定位 = 长程 AI 成长平台 + 双洋葱 + Self-Disable (Apeireth 独有)

---

## 2. 决策 (Decision)

**7 竞品对比 + Apeireth 差异化定位 = 长程 AI 成长平台 + 双洋葱 + Self-Disable + 6 哲学锚**

### 2.1 7 竞品全景 (per `docs/competitive-analysis-2026-08-05.md` §0)

| 项目 | 语言 | Stars / Crate | 核心定位 | 关键模块 |
|---|---|---|---|---|
| **microsoft/autogen** | Python | **60,242** | programming framework for agentic AI | 事件驱动 + 群聊 |
| **crewAIInc/crewAI** | Python | 56,645 | orchestrating role-playing autonomous agents | role-playing + task delegation |
| **letta-ai/letta** | Python | 24,099 | stateful agents with advanced memory | 持久状态 + memory |
| **hermes-agent-rs** | Rust | 18 crate | 通用 Agent + environments + skills + telemetry | 洋葱架构 + 横向能力 |
| **memoryos-rust** | Rust | 9 crate | 六边形架构 (ports & adapters) memory OS | 6 边架构 + memory |
| **vcptoolbox** | Node.js | 30+ 文件 | plugin-based AI middleware | modules + Plugin.js + Agent |
| **honcho** | Python | 14 子模块 | memory infrastructure for stateful agents | cache / deriver / dialectic / dreamer |
| **Apeireth-rust** | **Rust** | **42 → 67** | **长程 AI 成长平台 + 双洋葱 + Self-Disable** | 哲学器官 7 + 工具 6 + 战区 6 + 9 估缺 |

### 2.2 Apeireth 6 大优势 (per `competitive-analysis-2026-08-05.md` §7.1)

1. **Self-DisableGuard** — 主权可降级 + 多签恢复 (独有, vcptoolbox 缺)
2. **OTA 7 阶段状态机** — 系统级可升级 (独有, AutoGen/CrewAI 缺)
3. **DB triggers BEFORE UPDATE/DELETE → ABORT** — 物理不可篡改记忆 (per `apeireth-memory` SQLite triggers)
4. **编译期硬编码 token 预算 + 13 类敏感键** — 编译期钉死安全策略 (per `apeireth-supervisor` 编译期 enum)
5. **67 crate workspace + 7 哲学器官 + 6 工具** — Rust 阵营最完整 (per R20 阶段 1 续)
6. **6 哲学锚 + 8 项不修改承诺** — 业界唯一 (per ADR 0021)

### 2.3 Apeireth 6 大短板 (per `competitive-analysis-2026-08-05.md` §7.2)

| 短板 | 业界有 | 1.0 release 状态 | R21+ 估补 |
|---|---|---|---|
| ❌ **没有 `adapters` 抽象层** | memoryos-rust 6 边架构 ports & adapters | 1.0 release 缺 | 估补 `apeireth-adapters` crate (1 owner × 1 周) |
| ❌ **没有 `telemetry` 单独 crate** | hermes 有 `hermes-telemetry` + `otlp.rs` (OpenTelemetry) | 1.0 release 仅 `apeireth-tracing` (R20 阶段 6 估补) | R21 估补完整 OpenTelemetry exporter |
| ⚠️ **crates 密度过高** | hermes 18 crate 已能覆盖相似功能域 | 67 crate, 部分估缺, 1.0 release 实装 50+ | R21 估补 9 skeleton + 14 估补 |
| ⚠️ **Web 端缺** | 7 竞品全有 web 端 | 主人 2026-08-04 拍板 "web 搁置, 缺审美设计" | Tauri R21+ 估补 |
| ⚠️ **IDE 集成缺** | 5/7 竞品有 VSCode/Cursor 扩展 | 1.0 release 缺 | R21+ 估补 |
| ⚠️ **真实生态** | AutoGen/CrewAI 60K+ stars, 大量用户 | Apeireth 早期, 社区薄 | 1.0 release 公开 + R21 营销 |

### 2.4 Apeireth 差异化定位 (1.0 release 战略)

**Apeireth = 长程 AI 成长平台 (per 主人 2026-08-04 拍板)**

- **"长程"**: AI 跟用户一同成长, 不"用完即弃"; per O-2 用户看结果不看哲学
- **"AI 成长平台"**: 不只"AI 工具", 是"AI 持续成长 + 用户参与成长"
- **"双洋葱"**: 哲学层 (Principle) + 权限层 (Permission), 业界唯一 (per ADR 0001)
- **"Self-Disable"**: 主权可降级 + 多签恢复 + 物理不可篡改, 业界唯一

**打谁 (per 主人拍板)**:
- 主力打 hermes-agent-rs / memoryos-rust (Rust 阵营, 同类型用户)
- 次力打 letta (stateful agents, 持久化场景)
- 不打: AutoGen / CrewAI (Python 生态优势不可比, 60K stars)
- 不打: vcptoolbox (Node.js, 不同 runtime)

**不打谁**:
- AutoGen / CrewAI: Python 生态优势, 不必硬拼
- vcptoolbox: 不同语言, 不同用户群
- 普通 LLM 客户端 (Claude Desktop / ChatGPT): 不同场景

### 2.5 借鉴 (S-1 走在前人经验上)

| 借鉴 | 来源 | 1.0 release 实施 |
|---|---|---|
| 六边形架构 (ports & adapters) | memoryos-rust | 1.0 release 缺, R21 估补 `apeireth-adapters` |
| OpenTelemetry 集成 | hermes-agent-rs | R20 阶段 6 估补 `apeireth-tracing` (4 Span + 4 Exporter, 0 OpenTelemetry 全套) |
| 6 边 memory 抽象 | memoryos-rust + honcho deriver | `apeireth-memory` 5 表 + `apeireth-vector` tantivy 索引 |
| 7 advisor 协同 | vcptoolbox Agent + Council 模式 | `apeireth-council` (R10 round10-07) 24 trait 互锁 |
| 物理不可篡改 | 区块链 + DB triggers | `apeireth-memory` SQLite triggers BEFORE UPDATE/DELETE → ABORT |

### 2.6 不借鉴 (per S-1 严守 + 不依赖 NewAPI)

| 不借鉴 | 来源 | 否决理由 |
|---|---|---|
| NewAPI-style 独立代理服务 | NewAPI 项目 | 主人 2026-08-04 拍板 "不依赖 NewAPI", 5 Provider 自建 |
| Python 生态移植 | AutoGen / CrewAI | Rust idiom 优先, 不抄 TS/Python 代码 |
| Closed benchmark runner | LangChain benchmark | 主人拍板 "不绑 LangChain/LlamaIndex benchmark runner (闭门)" |
| Lock-in 商业版 | OpenAI Anthropic 商业版 SDK | 5 Provider 自建, 0 商业版 SDK |

---

## 3. 后果 (Consequences)

### 3.1 正面

- ✅ **差异化清晰**: 长程 AI 成长平台 + 双洋葱 + Self-Disable, 业界唯一
- ✅ **不盲目追 stars**: 主人 2026-08-05 拍板 "1 用户 1 年 500K 行", 不打 Python 阵营
- ✅ **借鉴业界**: 6 边架构 / OpenTelemetry / 物理不可篡改, 抄业界成熟方案
- ✅ **不打错位**: 不打 AutoGen / CrewAI / 普通 LLM 客户端, 集中 Rust 阵营
- ✅ **可对标**: 67 crate vs hermes 18 / memoryos 9, Rust 阵营最完整

### 3.2 负面

- ⚠️ **6 大短板**: adapters / telemetry / crates 密度 / web / IDE / 真实生态, R21+ 估补
- ⚠️ **Web 端缺**: 主人 2026-08-04 拍板 "缺审美设计前 Tauri 不上", 用户仅 TUI
- ⚠️ **真实生态薄**: 1.0 release 公开 + R21 营销, 估 6-12 月有 1K stars
- ⚠️ **不补全 Web**: 短期损失 50% 潜在用户 (但主人拍板优先 Rust 阵营, 1.0 release OK)

### 3.3 风险

- 1.0 release 后 6-12 月用户增长不达预期 (估 1K → 5K stars), R21 估补营销 + Web
- hermes-agent-rs 6 边架构已成熟, Apeireth R21 估补 adapters 时可能追不上 (mitigation: 抄 ports & adapters 模式, 1 owner × 1 周估补)
- vcptoolbox 7 advisor 模式被 Apeireth 借鉴, vcptoolbox 可能反向借鉴 Apeireth 双洋葱 (良性竞争, 长期 OK)

---

## 4. 备选 (Alternatives Considered)

### A. 不做竞品对比, 闷头做
- 优点: 省时
- 否决: 主人 2026-08-05 拍板 "1.0 release 1 用户 1 年 500K 行" 场景, 需明确打谁不打谁; 不对比 = 战略模糊

### B. 全面对标 AutoGen (追 stars)
- 优点: 用户群大
- 否决: AutoGen Python 生态优势不可比, Rust 阵营没必要追 Python 用户

### C. 7 竞品对比 + 差异化定位 (本决策)
- 优点: 战略清晰, 不追错位
- 拍板: R20 阶段 6 主人拍

### D. 全面对标 hermes-agent-rs (同 Rust, 同细分)
- 优点: 同语言, 可比
- 否决: hermes 18 crate 已成熟, 全面对标 = 拷贝; Apeireth 走差异化 (长程 + 双洋葱 + Self-Disable)

### E. 不对标, 走"独立路线"
- 优点: 自由
- 否决: 独立路线 = 0 借鉴 = 0 用户 (per S-1 走在前人经验上)

---

## 5. 6 哲学锚穿透

- ✅ **S-1 走在前人经验上**: 6 边架构 / OpenTelemetry / 物理不可篡改 抄业界成熟方案
- ✅ **S-2 实事求是**: 1.0 release 估 1 用户 1 年 500K 行, 不追 Python 阵营 60K stars
- ✅ **O-2 用户看结果不看哲学**: 用户只看 Apeireth 跟 hermes 比有什么优势, 不看战略层
- ✅ **O-3 信息密度"高"**: §2.1 全景 + §2.2 优势 + §2.3 短板 + §2.4 定位 4 表说清
- ✅ **O-4 干净状态 = 没有历史包袱**: 拒绝"全面对标 AutoGen" / 拒绝"独立路线"
- ✅ **O-5 6 哲学锚穿透**: 本节自检

---

## 6. 8 项不修改承诺

- ✅ **不假装已实现**: 6 大优势 ✅ + 6 大短板 诚实标
- ✅ **编译期 hardcode**: 6 哲学锚 + 13 类敏感键 编译期 enum (per `apeireth-supervisor`)
- ✅ **不改 LOCKED**: 7 LOCKED 文档 + 24 LOCKED crate 0 触碰
- ✅ **不改 workspace version**: v1.0.0 严守
- ✅ **6 哲学锚穿透**: §5 自检
- ✅ **不依赖 NewAPI**: 5 Provider 自建, 0 引 NewAPI-style 独立代理
- ✅ **不重复造轮子**: 借鉴 hermes / memoryos / honcho / vcptoolbox 业界成熟模式
- ✅ **诚实标缺**: 6 大短板 R21+ 估补, 不假装已实现

---

## 7. 引用

- 竞品对比: [`docs/competitive-analysis-2026-08-05.md`](../competitive-analysis-2026-08-05.md) (Codex CLI 2026-08-05 17:40 审)
- V2 战区: `docs/v2-strategy/02-VCP-DEEP-COMPARISON.md` + `01-INDUSTRY-LANDSCAPE.md`
- 双洋葱: [`docs/adr/0001-double-onion-unity.md`](0001-double-onion-unity.md)
- 7 哲学器官: [`docs/adr/0011-apeireth-team-lead-supervisor-prompt-translation.md`](0011-apeireth-team-lead-supervisor-prompt-translation.md)
- 6 哲学锚: [`docs/adr/0021-6-philosophy-anchors.md`](0021-6-philosophy-anchors.md)
- 1.0 release 战略: [`docs/adr/0013-apeireth-rust-1.0.md`](0013-apeireth-rust-1.0.md)
- 主人 2026-08-04 拍板 "长程 AI 成长平台": self-stance log
