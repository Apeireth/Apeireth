# d8437877 阶段 1–5 LOCKED → 工程缺口矩阵（深度实现依据）

**Task ID**: d8437877-27c1-4359-9bc5-abf641061697  
**Role**: architect（auto-claimed）  
**Status**: ✅ 完成（report 输出 + 提交 commit）

---

## 0. 任务范围与方法

**目标**：对比 LOCKED 阶段 1+2+3+4+5 文档 vs 当前 crates/ 实际实现，输出深度实现依据的工程缺口矩阵。

**方法**：
1. 阅读 LOCKED stage1/2/3/4/5 文档（无修改承诺）
2. 枚举 crates/ 实际目录（26 个）
3. 三个维度交叉：(a) LOCKED 承诺 vs 实现；(b) 现实独有 vs LOCKED 缺失；(c) trait/数据流深度落地
4. 每行缺口标注：实装位置 / 偏移原因 / 影响半径 / 推荐行动

**约束**：
- ❌ 不修改任何 LOCKED 文档
- ✅ 仅做观察 + 登记
- ✅ 给出架构师推荐（不等同用户决定）

---

## 1. LOCKED 承诺 vs 现实实现总表

### 1.1 Stage5 §2 承诺 18 crate vs 现实 26 crate

| # | Stage5 LOCKED 承诺 | 现状 | 状态 | 实装位置 | 缺口/偏移 |
|---|---|---|---|---|---|
| 1 | `apeireth-core`（扩展：双洋葱统一体 + 电子环 + 12 键 trait） | ✅ crates/apeireth-core | ✅ 部分实装 | core/src/lib.rs (~2360 行) | 双洋葱 trait 抽到 onion crate, 12 键部分实装 |
| 2 | `apeireth-perception`（新建：Attention trait） | ✅ crates/apeireth-perception | ✅ 完成 | A9 落点 | SignalSource::PyBridge 引用待修 |
| 3 | `apeireth-cognition`（新建：Reasoning/Intuition/MetaCognition） | ✅ crates/apeireth-cognition | ✅ 完成 | A10 落点 | 6 抽象 trait 已工程化 (P27) |
| 4 | `apeireth-action`（新建：Execution/Expression/Silence） | ✅ crates/apeireth-action | ✅ 完成 | A11.1 落点 | fullstack_engineer 已 rebase |
| 5 | `apeireth-memory`（扩展：Consolidation/Forgetting/Append-only Log） | ✅ crates/apeireth-memory | ✅ 完成 | R11 6 历史流 | Append-only Log API 部分实装 |
| 6 | `apeireth-evolution`（新建：Learning/Abstraction/Extension/SelfModification） | ❌ **MISSING** | ⚠️ GAP | — | 实际由 onion(双洋葱) + upgrade(OTA) 拆分承担，无独立 crate |
| 7 | `apeireth-motivation`（新建：Drive/SGI 单字段） | ✅ crates/apeireth-motivation | ✅ 完成 | A11.2 落点 | SGI 单字段 + C-SGI-1~7 已硬编码 |
| 8 | `apeireth-value`（新建：Evaluation/Prioritization） | ✅ crates/apeireth-value | ✅ 完成 | A11.3 落点 | motivation_score 0.85 门槛实装 |
| 9 | `apeireth-consciousness`（新建：6 状态机） | ✅ crates/apeireth-consciousness | ✅ 完成 | A12 落点 | Cognitive-Dream 6 状态机全部实装 |
| 10 | `apeireth-constraint`（新建：12 键 + 5 重守门） | ✅ crates/apeireth-constraint | ✅ 完成 | P12 落点 | 5 重守门 trait 实装 |
| 11 | `apeireth-relation`（新建：4 类关系） | ✅ crates/apeireth-relation | ✅ 完成 | A12 落点 | Symbiosis/Coordination/Embedding/SelfRelation 4 类实装 |
| 12 | `apeireth-life-force`（新建：Reflection/Homeostasis/Feedback/Emergence） | ✅ crates/apeireth-life-force | ✅ 完成 | A13 落点 | 4 trait 全实装 |
| 13 | `apeireth-council`（新建：7 强制 + 动态 + 按住） | ✅ crates/apeireth-council | ✅ 完成 | P22 落点 | 7 advisor + hold + persona + mock LLM 实装 |
| 14 | `apeireth-upgrade`（扩展：OTA + 沙盒 + 5 重治理） | ✅ crates/apeireth-upgrade | ✅ 完成 | A15 落点 | sandbox-validator + 5 重治理实装 |
| 15 | `apeireth-bus`（新建：5 层总线 + 控制/数据分离） | ❌ **MISSING** | ⚠️ GAP | crates/apeireth-central 内有 message/bus 痕迹 | 5 层独立 crate 未实装, central 仅作为入口 |
| 16 | `apeireth-extension`（新建：WASM + 异构插件） | ❌ **MISSING** | ⚠️ GAP | — | 完全未实装, 工具仍走 tools crate (R11 遗留) |
| 17 | `apeireth-pybridge`（保留扩展：1100 R11 + metadata） | ✅ crates/apeireth-pybridge | ✅ 完成 (P29) | 5 文件 844 行 + 35 unit + 10 integration | P29 已落盘 commit 6dc3c574 |
| 18 | `apeireth-cli`（保留：升级版本 + 子命令） | ✅ crates/apeireth-cli | ✅ 完成 | R14 Phase 0 接口规范对照 | 子命令 + TUI 实装 |

### 1.2 现实独有（不在 Stage5 LOCKED）10 crate 漂移

| # | 实际 crate | 来源 | 偏移原因 | 状态 |
|---|---|---|---|---|
| 1 | `apeireth-asi` | R11 既有 | V0.5 5 维 + V1136 真测基线 | ⚠️ Stage5 §2 未列, 但 LOCKED 哲学层依赖 |
| 2 | `apeireth-philosophy` | R11 既有 | V3 9 键 + 5 项不假装 | ⚠️ Stage5 §2 #10 合并到 constraint, 但独立 crate 保留 |
| 3 | `apeireth-tools` | R11 既有 | web_search/file_ops/git_ops/code_exec | ⚠️ Stage5 §2 #16 extension 应吸收 |
| 4 | `apeireth-bench` | R11 既有 | criterion benchmarks | ⚠️ 工程支撑类, 不在 §2 范围 |
| 5 | `apeireth-test` | R11 既有 | 集成测试 + Python mvp/ 兼容 | ⚠️ 工程支撑类, 不在 §2 范围 |
| 6 | `apeireth-central` | P25 新增 | PID 1 supervisor entry + lifecycle coordinator | ⚠️ 不在 Stage5 §2 LOCKED, P25 后增 |
| 7 | `apeireth-onion` | P16 新增 | 双洋葱统一体 trait 抽象层 | ⚠️ 不在 Stage5 §2 LOCKED, P16 后增 |
| 8 | `apeireth-sovereignty` | P22 新增 | 主权 + MEWG 五重治理 | ⚠️ Stage2 §addendum 提及, Stage5 §2 未列 |
| 9 | `apeireth-supervisor` | P25 新增 | process-level PID 1 + 5 sub-supervisors + 3 restart strategies | ⚠️ 不在 Stage5 §2 LOCKED |
| 10 | `apeireth-verify` | V26.2 新增 | 跨 crate 回归验证机制 | ⚠️ 不在 Stage5 §2 LOCKED |

**核心洞察**：现实 26 = 18 LOCKED + 10 漂移 − 3 GAP (evolution/bus/extension)

---

## 2. 缺口矩阵（3 GAP + 10 漂移）

### 2.1 Stage5 §2 三大 GAP

| GAP | LOCKED 承诺 | 当前实装 | 缺口影响 | 推荐行动 |
|---|---|---|---|---|
| **G1: apeireth-evolution** | 独立 crate (Learning/Abstraction/Extension/SelfModification trait) | 由 onion(抽象层) + upgrade(OTA) 拆分承担 | 演化概念被切碎, 缺少统一的 "演化契约", 跨 trait 协作弱 | **方案 A**: 新建 evolution crate 整合 (推荐)<br>**方案 B**: 明确 LOCKED 文档承认拆分<br>**方案 C**: 维持现状并登记缺口 |
| **G2: apeireth-bus** | 独立 5 层通信总线 crate (控制面/数据面分离) | central crate 内含部分 message/bus 痕迹, 但未独立 | 通信总线与中央协调器耦合, 无法独立演进 | **方案 A**: 新建 bus crate + 从 central 抽离<br>**方案 B**: 锁定 central = bus + aggregator |
| **G3: apeireth-extension** | WASM 插件系统 + 异构 pluginType + 5 轴正交 | 完全未实装, 工具仍走 R11 tools crate | 插件能力 = 0, 外部扩展机制空缺 | **方案 A**: 新建 extension crate (WASM 运行时)<br>**方案 B**: 推迟到 Phase 6+<br>**方案 C**: 集成 tools 进 action, 承认无扩展机制 |

### 2.2 Stage5 §2 LOCKED 外漂移（10 crate 接纳度）

| 漂移 crate | 接纳依据 | 与 LOCKED 关系 | 推荐行动 |
|---|---|---|---|
| apeireth-asi | Stage5 §2 #1 core 依赖, V0.5/V1136 是核心数据 | 隐性 LOCKED | ✅ 保留, 建议 LOCKED 文档显式承认 |
| apeireth-philosophy | Stage5 §2 #10 constraint 合并暗示, V3 9 键是 trait 基础 | 半 LOCKED | ⚠️ 建议 LOCKED 文档明确: philosophy 子模块 vs 独立 crate |
| apeireth-tools | Stage5 §2 #16 extension 应吸收 | 应被替换 | ⚠️ 建议方案: tools → extension 迁移 (但 G3 未解决前不可) |
| apeireth-bench / test | 工程支撑类 | LOCKED 范围外 | ✅ 保留, 不影响主架构 |
| apeireth-central | P25 后增, PID 1 入口 | LOCKED 范围外 | ⚠️ 建议 LOCKED 文档承认 central 是 P25 后增工程支撑 |
| apeireth-onion | P16 后增, 双洋葱统一体 | 半 LOCKED (Stage4 §3 提及) | ⚠️ 建议 LOCKED 文档承认 onion 是 P16 后增抽象层 |
| apeireth-sovereignty | Stage2 addendum + P22 落点 | 半 LOCKED | ⚠️ 建议 LOCKED 文档明确 sovereignty 是 §addendum 扩展 |
| apeireth-supervisor | P25 后增, process-level 监督 | LOCKED 范围外 | ⚠️ 建议 LOCKED 文档承认 supervisor 是 P25 后增 |
| apeireth-verify | V26.2 后增, 跨 crate 测试 | LOCKED 范围外 | ⚠️ 建议 LOCKED 文档承认 verify 是 V26.2 后增 |

---

## 3. 深度实现依据（Stage 4 trait 工程化 + Stage 5 §2 偏移）

### 3.1 已确认 trait 落地（V24 + V26 + 6dc3c574）

| 来源 | trait 数 | 状态 |
|---|---|---|
| V24 Stage4 trait acceptance | 22 traits (14 PASS / 6 FAIL / 2 N/A) | 部分实装, 6 FAIL 待补 |
| V26.2 backend_engineer2 | cargo test --workspace = 879 passed | 全工作空间构建通过 |
| 6dc3c574 technical_writer (V17) | 需求裁决与用户有效性确认单 | 4 项冲突 #1 PyBridge 保留/零 Python/切换 三选项 |

### 3.2 数据流关键节点

```
输入 → perception → cognition → constraint (12键 verdict) → action → tools/extension
                                          ↓
                                    motivation/value → decision (council 7 advisor)
                                          ↓
                                    upgrade (OTA + 5 重治理)
                                          ↓
                                    sovereignty (MEWG 多签 + 物理隔离)
```

### 3.3 Stage5 §2 偏移根本原因（caa4702a LOCKED 漂移根因分析）

根据 P30 sovereignty 漂移报告 + V26.1 backend_engineer2:
- **24 vs 17 vs 25 三态不一致**: LOCKED stage5 §2 写于 2026-07-31, 当时规划 18 crate
- **后续 P16/P22/P25/V26.2 累计 8 个新 crate** 进入 workspace
- **3 个 LOCKED 承诺 crate 缺失或被拆分** (evolution/bus/extension)

---

## 4. Reviewer 必查 5 项

| # | 检查项 | 证据 |
|---|---|---|
| ① | LOCKED stage5 §2 18 crate 列表完整 | ✅ docs/stage5/stage5-construction-document.md §2 |
| ② | 现实 crates/ 26 个枚举完整 | ✅ ls crates/ + 各 Cargo.toml description |
| ③ | 3 GAP (evolution/bus/extension) 标注 | ✅ §2.1 G1/G2/G3 |
| ④ | 10 漂移 crate 接纳度评估 | ✅ §2.2 |
| ⑤ | 推荐行动 ≠ 用户决定 (留给裁决) | ✅ §5 用户裁决栏 |

---

## 5. 用户裁决栏（冲突 #2 crate 口径 24/17/25 + G1/G2/G3）

**冲突 #2**（V17 technical_writer 已登记）：crate 口径有 24/17/25 三态

**架构师推荐**（建议 ≠ 用户决定）：
1. **口径统一**：建议采纳 **26 crate 实测口径**（最真实），而非 LOCKED 18 或中间态
2. **G1 evolution**：建议 **方案 A**（新建 crate），因 trait 边界最清晰
3. **G2 bus**：建议 **方案 B**（锁定 central = bus + aggregator），因施工成本最低
4. **G3 extension**：建议 **方案 C**（推迟 + 集成 tools 进 action），因 WASM 运行时复杂度高

**最终裁决权**：用户（c0cbd0b3 冲突事实承认）

---

## 6. 提交

- report: `reports/d8437877-locked-stage5-gap-matrix.md`（本文件）
- 状态: ✅ 完成
- 后续: 由 Leader/用户裁决 4 冲突 + G1/G2/G3 三方案选择