# TP31 / W1 世界模型第一层: 文本模拟器 — 验收报告

**任务**: `198e8449-78d3-4300-bc45-0965a14c4c34` (TP31, W1, 发布前置 P0)
**角色**: 后端工程师
**分支**: `task/tp12-schema-guardrail-rework-final`
**提交**:
- `3cc77b87` feat(companion): TP31 世界模型第一层 文本模拟器 (W1, 发布前置 P0)
- `a051ec45` docs(backlog): W1 文本模拟器 TP31 完成登记 (提交 3cc77b87)

---

## 1. 交付物清单

| 项 | 路径 / 类型 | 状态 |
|---|---|---|
| `world_model` 模块 | `crates/apeireth-companion/src/world_model.rs` | 新增 (412 行, 含 doc + 4 测试) |
| 模块挂载 | `crates/apeireth-companion/src/lib.rs` | `pub mod world_model;` + re-export 6 类型 |
| 台账登记 | `docs/backlog.md` 第 101/288 行 (W1) | ✅ + 提交 hash 3cc77b87 已追加 |

## 2. 核心结构 (建议起点, 已落地)

| 类型 | 职责 |
|---|---|
| `TimelineContext` | LLM 调用上下文 (起点/假设/累积叙事/上一步状态/tick) |
| `TimelineStep { tick, narrative, state_snapshot }` | 推演链一步 |
| `CounterfactualChain { hypothesis, steps, terminal_forecast, calibration_brier, rejected, reject_reason }` | 一条完整反事实链 |
| `TimelineLlm` trait (async_trait) | LLM 抽象 (`expand_step` + `terminal_probability` 默认 0.5) |
| `MockTimelineLlm` | 测试用 mock (脚本耗尽 = 链自然结束) |
| `TextualSimulator` | 编排器 (run + calibrate) |

## 3. 验收测试结果

### 3.1 `cargo test -p apeireth-companion --lib world_model`

**lib 绿** ✅ (4/4 全绿, 测试通过率 100%):

```
running 4 tests
test world_model::tests::textual_simulator_generates_chain ... ok
test world_model::tests::textual_simulator_rejects_high_brier ... ok
test world_model::tests::textual_simulator_calibrates_with_brier ... ok
test world_model::tests::textual_simulator_does_not_persist_to_memory ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 511 filtered out
```

| 验收点 | 测试函数 | 断言 |
|---|---|---|
| ① 推演链生成 | `textual_simulator_generates_chain` | mock 3 步 → `chain.step_count()==3`; `terminal_forecast.is_some()`; 不拒绝 |
| ② Brier 终点校准数值 | `textual_simulator_calibrates_with_brier` | p=0.7/true → 0.09; p=0.7/false → 0.49 (1e-9 容差) |
| ③ 校准差拒绝 | `textual_simulator_rejects_high_brier` | p=0.9/false → 0.81 > 0.3 → `rejected=true` + `reject_reason` 含 "Brier"+"0.3" |
| ④ 0 装 PASS 边界 | `textual_simulator_does_not_persist_to_memory` | in-memory store run+calibrate 前后 episode 数=0 |

### 3.2 `cargo test -p apeireth-companion --lib` (全 lib 回归)

**lib 绿** ✅ (515/515 全绿, 含 4 个新测试):

```
test result: ok. 515 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### 3.3 `cargo check --workspace --all-targets`

**all-targets 绿** ✅ (0 错, examples/bins 全部编译; 仅历史 warnings):

```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.42s
warning: the following packages contain code that will be rejected by a future version of Rust: nom v1.2.4, proc-macro-error2 v2.0.1
```

(注: examples `multi_turn_agent.rs` / `production_daemon.rs` 的 unused warnings 系历史遗留, 与本任务无关)

## 4. 0 装 PASS 边界 (诚实登记)

### 4.1 真 LLM 未接, trait 口已备
- `TimelineLlm` trait 完整定义在 `world_model.rs` 模块顶部 doc 显式标注「真 LLM 未接, trait 口已备」
- 测试用 `MockTimelineLlm { scripts, terminal_p }` 走通全链
- 真实接入路径明确 (按 `TimelineContext` 调 LLM, 拆 (narrative, state_snapshot) 即可)

### 4.2 推演结果永远不入库 (防幻觉固化)
- `world_model.rs` 模块不导入 `SqliteMemoryStore`, 不调 `put_episode`
- `grep -n "put_episode|memory_extractor|experience::" world_model.rs` 仅在 doc/注释中提及 (lines 11-12, 126), 0 处实际调用
- 验收测试 `textual_simulator_does_not_persist_to_memory` 锁死此边界 (in-memory store 0 写入)
- `calibrate` 方法亦不入库 (诚实标注在 doc): oracle 历史积累应走 `ForecastRegistry::resolve` (`forecast-` 前缀), 那是 oracle 历史, 不是普通记忆

### 4.3 Brier 拒绝阈值
- 默认值: **0.3**
- 可配置: `TextualSimulator::with_threshold(t)` (builder 风格, 无样板)
- 双层拒绝:
  - **终点 Brier** (本次 forecast vs actual outcome) 超阈值 → `rejected=true`
  - **历史 mean_brier** (`CalibratedResolver.status()` 注入) 超阈值 → 整链预拒绝 (LLM 历史偏倚 → 本次也别信)

## 5. 复用 / 不重复造零件

| 既有零件 | 复用方式 |
|---|---|
| `oracle::WorldState` | 推演起点/状态快照载体, 不重定义 |
| `oracle::Entity` | 实体容器, 复用 |
| `oracle::Forecast` | 终点预测断言 (含 `brier` 字段), 直接复用不重算 Brier |
| `oracle::CalibratedResolver` | 可选注入, 历史 mean_brier 校准整链 |
| `oracle::UncertaintyResolver` trait | (本任务未直接用, 但 `TextualSimulator` 接口形态与之同构) |

未新增同名/同语义结构。`TimelineStep`/`CounterfactualChain`/`TimelineContext`/`TextualSimulator`/`MockTimelineLlm` 全部为新文件 `world_model.rs` 新增, 与既有命名空间无冲突。

## 6. 纪律清单核对

| 纪律 | 状态 |
|---|---|
| 真 LLM mock, trait 口标"未接" | ✅ 见 §4.1 |
| 不注入记忆 (grep 确认) | ✅ 见 §4.2 |
| all-targets 编译 | ✅ 见 §3.3 |
| 锁纪律 (std Mutex 不可重入) | ✅ 未引入新锁, 全部结构 sync 字段, 无嵌套取锁路径 |
| 报告路径 = taskId + 角色 | ✅ `reports/198e8449-78d3-4300-bc45-0965a14c4c34-backend_engineer-report.md` |
| 台账完成即划 ✅ | ✅ docs/backlog.md 第 101/288 行 W1 末尾追加 ✅ + 提交 hash 3cc77b87 |
| 不接任务包以外的活 | ✅ 仅做 TP31 范围内事, 未碰 N20/W6/W7/A4/A5 |

## 7. 已知遗留 / 不在本任务范围

- **真 LLM 未接**: trait 形态已定, 真实 LLM 接入 (提示词模板 + 拆 narrative/state) 待 W2/W3 立项时统一设计, 非本任务阻塞项
- **Oracle 历史校准**: 当前实现已支持 `with_calibrator()`, 但默认未启用 (无 oracle 历史 = `mean_brier=0`, 不触发误拒绝). 生产环境接入需 `ForecastRegistry` 配合, 已挂接点全部就绪
- **TimelineStep.state_snapshot 的语义**: 当前 mock 手工构造, 真 LLM 接入后需约定 LLM 输出 schema (narrative 字符串 + 状态变化 JSON), 待 W2/W3 立项时设计

## 8. 结论

**TP31 / W1 文本模拟器完成, 验收标准全数达成**:

- ✅ 4 个验收测试点全绿 (lib 绿)
- ✅ 全 companion lib 回归 515/515 全绿
- ✅ `cargo check --workspace --all-targets` 0 错
- ✅ 0 装 PASS 边界锁死 (mock LLM / 不入库 / Brier 默认 0.3)
- ✅ 复用既有 oracle 零件, 不重复造沙盘
- ✅ 台账 backlog.md W1 行已 ✅ + 提交 hash 登记

**提交 hash**: `3cc77b87` (实现) + `a051ec45` (台账登记)
**报告**: `reports/198e8449-78d3-4300-bc45-0965a14c4c34-backend_engineer-report.md`



---

## 9. Rebase 后状态 (集成冲突第 1/3 轮重派)

集成冲突发生于合并到 `team/e8de47ae-.../integration` (HEAD 74efcf57) 时。本地 rebase 处理摘要:

### 9.1 冲突文件与解决策略
| 文件 | 冲突类型 | 解决策略 | 备注 |
|---|---|---|---|
| `reports/8b9d492b-tp13-hygiene-devops_engineer-report.md` | add/add + content | HEAD-side (extended sections) | 来自 TP13 任务, HEAD 已含评审补交版 |
| `docs/backlog.md` | content (W3 状态 + orphan `=======`) | HEAD-side + 清 orphan | W3 在 HEAD 标 ✅, 符合"完成即划 ✅"纪律 |
| `docs/next-team-handbook.md` | add/add (世界模型前置批节 + 优先级列表) | HEAD-side (含 TP31+TP32 优先级批) | HEAD 已添加世界模型批节 |

所有冲突文件均按 HEAD-side 解析 — 因 HEAD 均为 rebase 后的更新版本 (含我自身提交之前已存在的扩展节)。

### 9.2 提交 hash 变更 (rebase 后)
| 原 hash | 新 hash | 说明 |
|---|---|---|
| `a72f5636` | `3cc77b87` | feat(companion): 文本模拟器实现 |
| `e0103042` | `a051ec45` | docs(backlog): W1 登记 |
| `8f2e4027` | `b65158c0` | docs(report): 验收报告 |

### 9.3 最终交付 (集成 rebase 后)

**分支**: `task/tp12-schema-guardrail-rework-final`
**提交链 (7 commits on top of integration HEAD 74efcf57)**:
- 3cc77b87 feat(companion): TP31 世界模型第一层 文本模拟器  (核心)
- a051ec45 docs(backlog): W1 文本模拟器 TP31 完成登记
- b65158c0 docs(report): TP31 验收报告
- 8e69068e docs(report): 更新提交 hash 引用 (rebase 后)
- 69a012a2 docs(report): 行号修正 (101/289 → 101/288)
- efb0f336 chore: 清理 _workspace 残留 (集成前置)
- 8f09b8f5 docs: 台账 W3-W7 主人设计补充批 (集成前置)

### 9.4 集成 rebase 后验证

- ✅ `cargo test -p apeireth-companion --lib world_model` → 4/4 全绿
- ✅ `cargo test -p apeireth-companion --lib` → 515/515 全绿
- ✅ `cargo check --workspace --all-targets` → 0 错
- ✅ world_model.rs (17436 字节) 完整保留
- ✅ lib.rs re-export 完整保留 (`pub mod world_model;` + 6 类型 pub use)
- ✅ backlog.md W1 两处 (行 101/288) ✅ + 提交 hash 3cc77b87 完整保留
- ✅ 报告路径唯一存在

### 9.5 Rebase 纪律核对

| 纪律 | 状态 |
|---|---|
| 不丢自己工作 | ✅ 3 个 TP31 commit 全部保留 (3cc77b87/a051ec45/b65158c0) |
| 不丢集成已有进展 | ✅ rebase on top of integration HEAD 74efcf57 |
| 冲突解决有据 | ✅ HEAD-side 解析依据: HEAD 是含扩展节的更新版本 |
| 提交 hash 引用同步 | ✅ backlog.md + report.md 全部 hash 引用更新为 rebase 后实际值 |
| 0 装 PASS 边界仍锁死 | ✅ rebase 后 4 个测试点全绿 (含不落库边界) |


— 后端工程师 / TP31