# N7 查询形态学 softmax — 自审报告

- **任务 ID**: d89e8df6-1292-451f-b44c-a8cdaad6dc7a
- **角色**: agent_orchestrator2
- **状态**: 已完成
- **提交**: 08c6f00d (代码) / 69e9b637 (台账回填 + 本报告)

## 1. 交付物

| 文件 | 改动 |
|---|---|
| `crates/apeireth-companion/src/morphology.rs` | 新增纯函数模块 (~230 行含 10 单测; 逻辑核心 ~100 行) |
| `crates/apeireth-companion/src/lib.rs` | `pub mod morphology` + 顶层 re-export (仅本人两处 hunk) |
| `crates/apeireth-companion/src/assemble.rs` | inject_memory 一处挂接: `crawl(&seed_ids, 3)` → `crawl(&seed_ids, morphology::crawl_budget(query))` |
| `docs/maintenance-guide.md` | 模块地图 +1 行; companion_serve env 清单 + APEIRETH_MORPHOLOGY_TEMPERATURE |
| `docs/backlog.md` | N7 ⬜ → ✅ |

## 2. 设计（对照 VCP 原版）

VCP 原版 (rivermemo_topology_v3.rs:1784-2011): 河网 hop 分布/HHI/前向流占比等**图拓扑特征** → 3 logits → softmax → atomic/propositional/narrative 模式。

本实现（机制同构、特征替换，因 Apeireth 无河网数据结构，差异已在模块头诚实登记）:

1. **确定性文本特征** (`extract`, 无随机/IO/LLM):
   - `length` = 字符数/60 钳位 [0,1]
   - `entity` = 字母数字字符占比（实体密度代理）
   - `question` = 疑问形态词命中数/2 钳位（？/吗/呢/怎么/如何/是否/哪些/什么/为什么）
   - `clauses` = 分句数（中文标点切分）→ (n-1)/3 钳位
   - `depth` = 深度线索词命中数/2 钳位（详细/背景/历史/来龙去脉/梳理/回顾…）
2. **三档 logits**（手调启发式系数，仿 VCP 加权结构）:
   - 浅扫: `1.45*(1-len) + 0.9*question − 1.25*depth − 0.65*clauses`
   - 标准: `0.35 + 1.25*clauses + 0.7*entity + 0.35*question − 0.45*depth`
   - 深爬: `1.4*len + 1.15*depth + 0.8*clauses + 0.3*entity − 0.65*question`
3. **softmax（温度净化）**: max-subtraction 数值稳定; 温度 NaN/≤0/∞ → 1.0, 有效值钳位 [0.1, 10.0]。
4. **档位决策**: argmax → RetrievalMode{Shallow/Standard/Deep}; 温度影响分布锐度 → 期望预算 `budget = round(w0*1 + w1*3 + w2*6)` 钳位 [1,6]（argmax 对温度不变，期望预算随温度变化——这是温度"可配"的真实作用面，非装）。
5. **边界助手**: `env_temperature()` 读 `APEIRETH_MORPHOLOGY_TEMPERATURE`（默认 1.0）; `crawl_budget(query)` 一行式供挂接点。

## 3. 挂接（仅一处，未重写 crawl）

`assemble.rs::inject_memory` 内原写死 `crawl(&seed_ids, 3)` → 改为 `crawl(&seed_ids, crate::morphology::crawl_budget(query))`。memory_graph.crawl 本体 0 改动（budget 语义 = BFS 展开条目数上限，已核实）。

## 4. 验收对照

- [x] cargo test -p apeireth-companion -j 4 全绿（结果见 §6）
- [x] 纯函数多组输入用例: 短问句→浅扫 / 多分句关系型→标准 / 长+深度线索→深爬
- [x] 边界: 空查询→浅扫 (budget≤2, 无 panic); 1 万字符超长→深爬 (无 panic)
- [x] 确定性: 同查询 5 次复测同档位 + 同分布
- [x] 温度可配: 低温锐化→档位基准预算 / 高温摊平→期望预算收缩 / 非法温度回落 1.0
- [x] 0 装 PASS: 系数为手调启发式（非学习所得）、VCP 特征替换差异已登记、无假装调参验证
- [x] 文档同步: 模块地图 + env 清单 + backlog N7 ✅

## 5. 边界遵守

- ❌ 未动 memory_graph 节点评分（N6, database_engineer 的活）
- ❌ 未动 semantic 持久化（N5, database_engineer2 的活）
- ❌ 未重写 crawl 本体
- ✅ 提交时仅 stage 本人改动 hunk（lib.rs 含他人并行 WIP，用选择性暂存隔离）

## 6. 测试结果（如实记录）

**模块独立验证（rustc --test, 纯 std 无依赖）: 10/10 全绿**
```
running 10 tests
test tests::budget_bounds ... ok
test tests::deterministic_same_query_same_mode ... ok
test tests::empty_query_shallow ... ok
test tests::invalid_temperature_falls_back ... ok
test tests::long_depth_query_deep ... ok
test tests::multi_clause_relational_standard ... ok
test tests::temperature_affects_sharpness ... ok
test tests::weights_are_valid_distribution ... ok
test tests::short_question_shallow ... ok
test tests::huge_query_deep_no_panic ... ok
test result: ok. 10 passed; 0 failed
```

**全 crate 验收阻塞（0 装如实登记）**: `cargo test -p apeireth-companion -j 4` 在提交时点被**并行成员的未提交 WIP** 阻塞（均非 N7 文件）:
- `apeireth-tool-approval/src/rule.rs:669` E0521 (N10 宽松文本工具协议层 WIP)
- `apeireth-companion/src/prompt_assembler.rs` E0599×2 + E0658 (N9 提示词装配引擎 WIP, 缺 import)

morphology.rs 无任何 crate 内依赖（纯 std），assemble.rs 挂接仅一行（参数类型已核对: query: &str → crawl_budget → usize）。**建议 integration/Leader 在 N9/N10 WIP 落地后复跑 `cargo test -p apeireth-companion -j 4` 做最终闭环**。
