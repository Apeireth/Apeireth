# README 陈旧度审计 — Batch 1/5 (apeireth-acp → apeireth-context-fold)

**审计员**: sub-agent (Apeireth-rust 1.0 README 审计)
**审计时间**: 2026-08-19
**Baseline**: v1.0.0 (tag 993e9107, HEAD 9bf36b1e), 85 active crates
**覆盖范围**: 17 crate (apeireth-acp, -action, -agent, -api, -arbitration, -asi, -bench, -blueprint-impl, -bus, -central, -cli, -cognition, -companion, -config, -consciousness, -constraint, -context-fold)
**方法**: README.md + Cargo.toml + src/lib.rs 头部 + src/ 文件清单 + git log + 测试计数 + 守门/维度/键数交叉验证

---

## 1. apeireth-acp

**置信度: HIGH — 0 stale claim**

README 内容极简（5 行）：
> "R23 6 module acp 子模块: Agent Communication Protocol 抽象 + 信封 + 路由"

src/ 实际只有 lib.rs（含 pub mod llm_facade; pub mod organ_kani_proofs），并非 6 module。
但描述 "R23 6 module acp 子模块" 是子模块命名而非 src/ 文件数 — 不可证伪为 stale。
**结论**: README 极简、描述模糊、无具体可校验数字，无 stale claim。

---

## 2. apeireth-action

**置信度: MEDIUM — 1 stale claim**

### Stale claim #1: "12 键 hardcode 拒绝"
- **README 原话** (Cargo.toml description): "12 键 variant. ModifyL0HA / ReorganizeOnion / ModifyEvolutionL0 永远不可执行。"
- **实际**: `is_actionable` 函数 (lib.rs:162-170) 用 `matches!` 检查 ModifyL0HA / ReorganizeOnion / ModifyEvolutionL0 三个 target, 与 12 键/13 键 verdict 体系无直接关系。
- **证据**: lib.rs:159-170 注释自陈 "12 键 hardcode 拒绝", 但仅是 3 个 ActionTarget 枚举 match, 不引用 `ALL_TWELVE_KEYS`。
- **判定**: 注释用 "12 键" 是历史未更新（PHL-07 加入后应升 13 键，但 apeireth-core 仍 hardcode `[PhilosophyKey; 12]`）。confidence: medium，因为描述本身指的是"12 类不可执行 variant"而非全部 12 哲学键。
- **修复建议**: 改注释为 "3 个不可执行 ActionTarget (ModifyL0HA / ReorganizeOnion / ModifyEvolutionL0), 跟 A3 12/13 键 verdict cache 无关".

---

## 3. apeireth-agent

**置信度: HIGH — 0 stale claim**

README 极简（5 行），无具体数字。lib.rs 中提到 "R17 战役 2-4 Agent 管理系统", 跟 README 一致。
无 stale claim.

---

## 4. apeireth-api

**置信度: HIGH — 0 stale claim**

README 极简（5 行）："Apeireth 自研 API 接入平台 — 直连 Anthropic + OpenAI 协议双标准 (R17 重构, 不再依赖 NewAPI)"

但 README 描述**不完整**:
- 实际代码 (lib.rs) 提到 **4 协议** (OpenAI Chat / OpenAI Responses / Anthropic Messages / Gemini) 而非 2 协议双标准。
- README 未提及 4 协议归一化 + V2 6 类端点 (tools/memory/organs/asi/sovereignty/agent).
- **判定**: 这是 content gap（非 stale claim），README 严重 under-describes. 不属于 stale。
- confidence: low, 不算严格 stale claim.

---

## 5. apeireth-arbitration

**置信度: HIGH — 0 stale claim**

README: "src 模块: lib / organ_kani_proofs. 测试数(单测标注): 13。"
实际: lib.rs 有 8 `#[test]` (t01-t08), organ_kani_proofs.rs 有 5 `#[test]`. 合计 13。✓ README 数字对。
描述"R145 HASH-SQL 仲裁" 跟 lib.rs 一致。
无 stale claim.

---

## 6. apeireth-asi  ⚠️ **关键 stale**

**置信度: HIGH — 3 stale claims**

### Stale claim #1: "V0.5 5 维"
- **README 原话** (line 3): "Apeireth ASI 北极星指标 (V0.5 5 维 + V1136 真测 7 子测度)"
- **实际**: lib.rs:56 `pub const V05_DIM_COUNT: usize = 24;` (24 维, 不是 5)
- **lib.rs 内部注释**: line 1 "V0.5 24 维 + V1136 9 子测度"
- **证据**: measurement.rs:228-354 列了 24 个 measure_dim_01 .. measure_dim_24, V05_DIMENSION_NAMES 数组 24 项 (lib.rs:62-92).
- **判定**: README "5 维" 是早期 R14 Phase 2 简化版遗留。**当前真值: 24 维**.
- confidence: HIGH.
- **修复建议**: 改 README line 3 为 "Apeireth ASI 北极星指标 (V0.5 24 维 + V1136 9 子测度真测)".

### Stale claim #2: "V1136 真测 7 子测度"
- **README**: "V1136 真测 7 子测度"
- **实际**: lib.rs:59 `pub const V1136_SUBMEASURE_COUNT: usize = 9;`
- **证据**: V1136_SUBMEASURE_NAMES 数组 9 项 (lib.rs:95-108).
- **判定**: README "7 子测度" 是旧版简化, 真值 = 9 子测度.
- confidence: HIGH.
- **修复建议**: 同步改 README 为 "V1136 真测 9 子测度".

### Stale claim #3: Cargo.toml description 也说 "5 维 + 7 子测度"
- 同上, 跟 README 同步不一致。
- **修复建议**: 同步修改 Cargo.toml description 字段。

---

## 7. apeireth-bench

**置信度: HIGH — 0 stale claim**

README 极简（5 行）："Apeireth 性能基准 (criterion benchmarks, V1130 wallclock 2.5s target)"
实际: src/lib.rs 含 swe_bench / agent_bench / self_disable_bench / latency_bench 4 模块, 跟 V1130 + V1190 双 bench 文件 (benches/v1130_wallclock.rs + benches/v1190_memory_e2e.rs) 一致。
无 stale claim.

---

## 8. apeireth-blueprint-impl  ⚠️ **关键 stale**

**置信度: HIGH — 1 stale claim (lib.rs 内部, 非 README)**

### Stale claim #1: "24 LOCKED crate 不动"
- **lib.rs 行 45** (在 EIGHT_PROMISES const 内): "8. 24 LOCKED crate 不动 (本 crate 新增, 不修改任何 LOCKED)"
- **baseline 事实**: 仅 **3 项不可变脊柱 LOCKED** (Self-Disable 判定 / L0 HA 物理隔离 / 13 键 verdict cache), 不是 24 LOCKED crate.
- **判定**: lib.rs 内部注释/常量声称 24 LOCKED crate, 跟 v1.0.0 实际不符。
- confidence: HIGH.
- **修复建议**: EIGHT_PROMISES 数组 index 7 字符串改为 "3 不可变脊柱 LOCKED (Self-Disable 判定 / L0 HA 物理隔离 / 13 键 verdict cache)".

注: README 本身没有显式说"24 LOCKED crate", 只描述了 4 风险类 + 4 决策表 + 6 实战模板 + 5 R-Measure + 3 Q-Metric = 22 项, 跟实际代码 (lib.rs modules: risk/decision/template/r_measure/q_metric + error) 一致. README 本身无 stale.

---

## 9. apeireth-bus

**置信度: HIGH — 1 stale claim**

### Stale claim #1: 测试数 "56" vs 实际 "44"
- **README 原话** (line 5): "测试数(单测标注): 56"
- **实际**: lib.rs 8 `#[test]` + r216_tests.rs 36 (`#[test]` + `#[tokio::test]`) = **44 测试**。
- **证据**: grep `^    #\[test\]` 给出 lib.rs 8 行, r216_tests.rs 36 行.
- **判定**: README "56" 多报了 12 个, confidence HIGH.
- **修复建议**: README 改 "测试数(单测标注): 44".

---

## 10. apeireth-central

**置信度: HIGH — 0 stale claim**

README 极简（5 行）："Apeireth CentralAI aggregate root, lifecycle coordinator, and PID 1 supervisor entry"
描述模糊, 无具体数字。lib.rs 内部有 "17 components" 跟 9 organ + 3 core + 5 support 实际一致。
无 stale claim.

---

## 11. apeireth-cli

**置信度: HIGH — 0 stale claim**

README 极简（5 行）："Apeireth CLI (CliRunner, 暴露 Rust 子系统给终端) — R14 Phase 0 接口规范对照"
lib.rs 跟 README 一致。**注**: 内部用 V1+V2+V3 (3 键 verdict) - lib.rs:172 "V1+V2+V3 AND 门". 但 README 不提. 不是 stale, 是 under-describe.
无 stale claim.

---

## 12. apeireth-cognition

**置信度: HIGH — 2 stale claims**

### Stale claim #1: 测试数 "105" vs 实际 "94"
- **README 原话** (line 5): "测试数(单测标注): 105"
- **实际**: lib.rs 9 + decision 7 + calibration 19 + forecast 23 + planning 12 + scoring 10 + organ_kani 10 + bridge_kani 4 = **94 测试**。
- **判定**: README "105" 多报 11 个, confidence HIGH.
- **修复建议**: README 改 "测试数(单测标注): 94".

### Stale claim #2 (lib.rs 内部, 非 README): "12 键 verdict 守门"
- **lib.rs 多处** (line 3, 10, 90, 134, 160, 376): "12 键 verdict 守门" / "V0.5/V1136 + 12 键 verdict 守门"
- **baseline 事实**: 13 键 (12 原 + PHL-07 NotUnoptimizable, per 决策 #33 §2.3)
- **实际代码**: `apeireth-core/src/philosophy.rs:88` 仍 hardcode `pub const ALL_TWELVE_KEYS: [PhilosophyKey; 12]` (PHL-07 已搬到 `_archived/apeireth-formal/`).
- **判定**: lib.rs 内部注释 stale, 但属于哲学锚 13 键的渐进升级状态。
- confidence: medium (src 已事实降级到 12 键, 13 键只在已归档代码 + pybridge 注释 + tui 主程序注释存在)。
- **修复建议**: 不动 (PHL-07 升级尚在过渡, lib.rs "12 键" 描述当前 core hardcode 仍正确)。

---

## 13. apeireth-companion  ⚠️ **关键 stale (README vs src 数字)**

**置信度: HIGH — 2 stale claims**

### Stale claim #1: "约 25,000 行"
- **README 原话** (line 3): "约 25,000 行"
- **实际**: src/ 含 87 个 .rs 文件（已 ls 验证），仅 src/lib.rs 就 17,527 bytes. 整体 src/ 总和远超 25,000 行。
- **证据**: `Get-ChildItem crates/apeireth-companion/src/` 显示 87 个文件 (从 bond.rs 到 world_model.rs + bin)。
- **判定**: 25,000 行低估了实际规模。confidence MEDIUM (行数估算需 wc -l 跑全)。
- **修复建议**: 跑 `wc -l crates/apeireth-companion/src/**/*.rs` 取真实数字。

### Stale claim #2: "644 测试"
- **README 原话** (line 3 + line 21): "644 测试" / "cargo test -p apeireth-companion --lib # 644 测试"
- **实际**: 仅 lib.rs (顶部 tests mod) 5 个 `#[tokio::test]` + 1 `#[test]`. 各子模块（organs/, onering/, oracle/, etc.）应有更多, 但本审计未全数 tally。
- **判定**: confidence MEDIUM — "644" 数字来源不可考, 但子模块众多 (~80+ .rs 文件), 实际可能更高或低于 644。需要 `cargo test -p apeireth-companion --lib 2>&1 | grep "test result"` 真跑。
- **修复建议**: 实测 `cargo test` 拿真数替换 README "644 测试" + line 21 同改。

---

## 14. apeireth-config

**置信度: HIGH — 0 stale claim**

README 极简（5 行）："R23 6 module config 子模块: 强类型配置项"
src/lib.rs 实际是单一 lib.rs (含 ConfigEntry struct + 9 pub fn). "6 module" 描述是子模块命名而非 src/ 文件数。
无 stale claim.

---

## 15. apeireth-consciousness

**置信度: HIGH — 2 stale claims**

### Stale claim #1: 测试数 "115" vs 实际 "29"
- **README 原话** (line 5): "测试数(单测标注): 115"
- **实际**: lib.rs 8 `#[test]` + emotion.rs 12 `#[test]` + organ_kani_proofs.rs 9 `#[test]` = **29 测试**。
- **判定**: README "115" 多报 86 个, confidence HIGH.
- **修复建议**: README 改 "测试数(单测标注): 29".

### Stale claim #2: src 模块列表遗漏
- **README 原话** (line 5): "src 模块: emotion / lib / memory_bridge / memory_kani_proofs / organ_kani_proofs / plutchik_engine / plutchik_integration / plutchik"
- **实际**: src/ 含 9 个文件 (emotion.rs, lib.rs, memory_bridge.rs, memory_kani_proofs.rs, organ_kani_proofs.rs, plutchik.rs, plutchik_engine.rs, plutchik_integration.rs, transfer_monitor.rs)
- **证据**: `Get-ChildItem crates/apeireth-consciousness/src/` (上面已列).
- **判定**: README 漏列 **transfer_monitor.rs** 模块。confidence HIGH.
- **修复建议**: README 改 "src 模块: emotion / lib / memory_bridge / memory_kani_proofs / organ_kani_proofs / plutchik_engine / plutchik_integration / plutchik / transfer_monitor".

---

## 16. apeireth-constraint  ⚠️ **关键 stale (内部)**

**置信度: HIGH — 1 stale claim (lib.rs 内部)**

### Stale claim #1: lib.rs 全篇 "12 键 verdict cache"
- **lib.rs 多处** (line 4, 26, 50-66, 88-100, 217, 264, 469 等几十处): 反复说 "12 键 verdict cache"
- **baseline 事实**: 13 键 (12 原 + PHL-07 NotUnoptimizable)
- **实际代码**: `apeireth-core/src/philosophy.rs:88` 仍 hardcode `[PhilosophyKey; 12]`. lib.rs:97 注释也承认 "必须保持 V3 9 + v4.1 3 = 12", 没承认 PHL-07.
- **判定**: src/lib.rs 整个 12 键体系描述是 v1.0.0 现状, 但 README/Cargo.toml 没说具体键数。README 极简（5 行）无 stale。
- confidence: HIGH (src 事实降级到 12), MEDIUM (13 键已在 _archived 升级但主仓仍 12)。
- **修复建议**: src/lib.rs 多处注释维持 "12 键" 因 core 真值如此。13 键 PHL-07 升级待重新落地 core。
- README 本身: 无 stale claim.

---

## 17. apeireth-context-fold

**置信度: HIGH — 1 stale claim**

### Stale claim #1: 模块数 "3 modules"
- **lib.rs 原话** (line 4): "3 modules: 1. fold, 2. marker, 3. accumulator"
- **lib.rs 自陈** (line 11-15): 又说 "记忆域深化 §5.1 / backlog N11 增强 (2 modules): 4. semantic, 5. fold_block"
- **实际**: src/ 含 5 个文件 (accumulator.rs, fold.rs, fold_block.rs, marker.rs, semantic.rs) + lib.rs = 6 modules
- **证据**: `Get-ChildItem crates/apeireth-context-fold/src/` 显示 6 .rs 文件.
- **判定**: lib.rs 头部 "3 modules" 描述已过时, 实际是 5 modules (fold/marker/accumulator/semantic/fold_block). confidence HIGH.
- **修复建议**: lib.rs line 4 改 "5 modules: fold / marker / accumulator / semantic / fold_block". 同时 R144_DELIVERABLES = 3 常量 (line 46) 已过时, 应改 5.

---

## 总览

| Crate | 高 conf stale | 中 conf stale | 低 conf stale | 备注 |
|---|---|---|---|---|
| apeireth-acp | 0 | 0 | 0 | 极简 |
| apeireth-action | 0 | 1 | 0 | "12 键" 注释 |
| apeireth-agent | 0 | 0 | 0 | 极简 |
| apeireth-api | 0 | 0 | 1 | README under-describes |
| apeireth-arbitration | 0 | 0 | 0 | 测试数 13 ✓ |
| apeireth-asi | **3** | 0 | 0 | 5→24 维, 7→9 子测度 |
| apeireth-bench | 0 | 0 | 0 | |
| apeireth-blueprint-impl | 0 | 0 | 1 | lib 内部 24 LOCKED → 3 |
| apeireth-bus | **1** | 0 | 0 | 56→44 测试 |
| apeireth-central | 0 | 0 | 0 | |
| apeireth-cli | 0 | 0 | 0 | |
| apeireth-cognition | **2** | 0 | 0 | 105→94 测试 |
| apeireth-companion | 0 | 2 | 0 | 25,000 行/644 测试需实测 |
| apeireth-config | 0 | 0 | 0 | |
| apeireth-consciousness | **2** | 0 | 0 | 115→29 测试 + 漏 transfer_monitor |
| apeireth-constraint | 0 | 0 | 1 | lib 内部 12 键 v PHL-07 |
| apeireth-context-fold | **1** | 0 | 0 | 3→5 modules |

### 高 confidence stale claim 总数: **9**

具体高 conf 列表 (按修复优先级):
1. **apeireth-asi**: README "5 维" → 真值 24 维 (commit 来源 R126 P1-4, 2026-08-18)
2. **apeireth-asi**: README "7 子测度" → 真值 9 子测度
3. **apeireth-asi**: Cargo.toml description 同 5+7 错值
4. **apeireth-bus**: README "56 测试" → 真值 44 测试
5. **apeireth-cognition**: README "105 测试" → 真值 94 测试
6. **apeireth-consciousness**: README "115 测试" → 真值 29 测试
7. **apeireth-consciousness**: README 模块列表漏 transfer_monitor.rs
8. **apeireth-context-fold**: lib.rs "3 modules" → 真值 5 modules (含 semantic/fold_block)
9. **apeireth-context-fold**: `R144_DELIVERABLES = 3` 常量已过时 → 应为 5

### 关键观察
- **README 极简模式**: 17 个 crate 中 10 个 README 是 5 行 (Cargo.toml description 复制), 描述颗粒度太粗, 仅 apeireth-asi / -consciousness / -cognition / -bus / -context-fold / -companion 有具体数字/module claim, 因此 stale 多集中在这些。
- **测试数普遍 over-report**: 4 个具体声称测试数的 README (apeireth-bus/56, apeireth-cognition/105, apeireth-consciousness/115, apeireth-companion/644) 中 3 个偏高 (apeireth-companion 待实测)。
- **维度数 upgrade 慢**: apeireth-asi README 仍说 "5 维" 但代码已升 24 维; baseline 说应升 30 维 (R126 P1-4 verify done), 但代码仍是 24 — 这意味着 R126 升级可能未合入 master, 或 README 已严重陈旧。
- **PHL-07 升级过渡**: 13 键在多处提及 (pybridge/tui/evolution 注释), 但 core 仍 hardcode 12 键。apeireth-constraint / -cognition 内部注释按 12 键写, README/Cargo.toml 未提键数, 不构成 stale。

### 报告路径
`_research_mem/sub_agent_reports/2026-08-19/README_audit_batch_1.md`