[Document-Meta]
Document: docs/04-internal/01-handoffs/memory-convergence-plan-2026-11-05.md
Class: DECISION
Status: 🟡 待裁决
Last-Modified: 2026-11-05
Author: deputy

# 记忆引擎「概念收敛」方案（**只出方案，不动代码**）

> **性质**：**设计方案**，**未实施**。动 API 形状 ⇒ ⇒ **属架构裁决** ⇒ ⇒ 需 owner / 总指挥拍板。
> **口径**：`debt-round8` @ `315158e368a2`，记忆 crate **179 模块 / 81108 行**。
> **前置**：孤儿基线我区已清（**待处置只剩 1 条**）⇒ ⇒ 现在该看**结构债**，不是零引用债。

## 一句话结论

**「概念没收敛」在记忆 crate 里的本体不是「同名函数太多」，而是三件事**：

1. ⭐ **6 个测试夹具模块住在 `src/` 且 `pub mod` 导出** ⇒ ⇒ **4426 行测试代码进了产品 API 面**
   ⇒ ⇒ 4 个下游 crate 编译时都要编它们；
2. ⭐ **同一个「SQLite 存储」模式复制了 6 遍**（2962 行）⇒ ⇒ **6 个各自独立的表结构 + 连接池 + CRUD**；
3. **同一概念有「产品面 / 报告面 / 实验面」三套模块**（`golden_*` 7 个）⇒ ⇒ **没人说得清哪个是权威**。

⇒ ⇒ **收敛抓手 = 分层，不是删模块** ⇒ ⇒ 详见 §四。

---

## 二、⚠️ 先排除两个**看起来像债、其实不是**的

### ① 「96 个 API 撞名」—— ❌ **大部分不是问题**
实测 `pub fn` 重名统计：`new` 159 次 · `as_str` 18 · `is_empty` 15 · `len` 12 …
⇒ ⇒ 这些是**不同类型上的同名标准方法** ⇒ ⇒ **不是收敛问题**
⇒ ⇒ **这个指标不能用**（我一开始也差点拿它当抓手 ⇒ ⇒ 幸好核了一遍）。

### ② 「130 个模块」—— ⚠️ **模块多不等于概念散**
`scope.rs` / `consolidation.rs` / `episode.rs` 各司其职 ⇒ ⇒ **不该按数量压**
⇒ ⇒ 收敛的判据必须是「**同一能力被切成多个入口**」，不是「模块多」。

---

## 三、三件**实测出来的**真债

### 债 1 ⭐：**6 个测试夹具模块住在 `src/` 且导出**

| 模块 | 行数 | 用途 | CI 闸门用？ |
|---|---|---|---|
| `golden_set` | — | **记忆质量考卷 + `check_no_regression`** | ✅ **是**（`check_golden_baseline.py`） |
| `golden_baseline` | 336 | 考卷基线报表 | ❌ 否 |
| `golden_diag` | 557 | 召回缺口诊断（Hit/RankedOut/NotInPool） | ❌ 否 |
| `golden_injection` | 469 | 注入形态对照（第一轮） | ❌ 否 |
| `golden_injection2` | 784 | 注入形态对照（第二轮） | ❌ 否 |
| `golden_mining` | 1323 | 挖题 | ❌ 否 |
| `golden_run` | 353 | 把考卷接到真实检索 | ❌ 否 |

**代价（实测）**：
- 合计 **4426 行 / 整个 crate 81108 行 = 5.5%**；
- `pub mod` 导出 ⇒ ⇒ **进了产品 API 面** ⇒ ⇒ 下游 crate（`cli` / `runtime-assembly` / `provider`）
  编译时**都要编**这些夹具；
- ⚠️ `golden_injection` 与 `golden_injection2`（第一轮 / 第二轮）**并存**
  ⇒ ⇒ 第二轮注释自承「消偏置 + 总预算固定」⇒ ⇒ **第一轮是历史遗留**，但没人敢删。

**⭐ 但 `golden_set` 必须留 `src/`** ⇒ ⇒ 它是 **CI 闸门的载体**
（`check_no_regression` ⇒ 「每次改动在同一份卷子上不许变差」⇒ ⇒ **规矩写进 CI**）
⇒ ⇒ ⇒ **不能一刀切搬到 `tests/`** ⇒ ⇒ 否则闸门就没有权威数据源了。

### 债 2 ⭐：**同一个「SQLite 存储」模式复制了 6 遍**

| 模块 | 行数 |
|---|---|
| `persona_store_sqlite` | 822 |
| `experience_store_sqlite` | 742 |
| `preference_store_sqlite` | 458 |
| `milestone_store_sqlite` | 378 |
| `self_assessment_store_sqlite` | 337 |
| `partner_store_sqlite` | 225 |
| **合计** | **2962 行** |

**实测现状**：crate 里已有 **18 个 `pub trait *Store`**（`EpisodeStore` / `MemoryGovernanceStore` /
`PersonaMemoryStore` / `DiaryStore` / `PrincipleStore` / …）⇒ ⇒ **端口（trait）已经统一了**
⇒ ⇒ **但「SQLite 实现」这一层没统一** ⇒ ⇒ 每个 store 各自一张表、各自一个连接池、各自一套 CRUD
⇒ ⇒ ⇒ **端口统一、实现复制** ⇒ ⇒ 这是**典型的「抽象只做了一半」**。

**⭐ 证据链**：`store` 后缀模块 9 个（`cluster_store` + 6 个 `*_store_sqlite` + `preference_store` + `experience_store`）
⇒ ⇒ 而 `*_store_sqlite` 6 个**共享同一套模式** ⇒ ⇒ 抽一个
`SqliteStore<T>` 泛型基座，**单点改表结构/连接池/事务语义**。

### 债 3：**同一概念三套模块**（`golden_*` 的权威不清）
- 记忆 crate 里有 **7 个 `golden_*`**，外加 `examples/` 里 **7 个 `golden_*_report`** 报表程序
  ⇒ ⇒ **卷子（`golden_set`）/ 诊断（`golden_diag`）/ 挖题（`golden_mining`）/ 跑分（`golden_run`）/
  注入形态两轮（`golden_injection{,2}`）/ 基线（`golden_baseline`）** ⇒ ⇒
  **7 个角色，7 个模块，7 套命名** ⇒ ⇒ 新人无法判断「哪个是权威」。

---

## 四、📌 **收敛方案**：**分层，不删**

> **核心原则**：**「谁被 CI 闸门依赖 ⇒ 留在产品面；其余下沉」**
> ⇒ ⇒ **不删任何能力** ⇒ ⇒ **只改「住在哪」** ⇒ ⇒ **可回滚、风险低**。

### 第 1 层：**闸门层**（留在 `src/`）
`golden_set`（考卷 + `check_no_regression`）
⇒ ⇒ **理由**：它是 CI 闸门的权威数据源 ⇒ ⇒ 搬走 ⇒ ⇒ 闸门失源。

### 第 2 层：**产品能力层**（留在 `src/`，但**合并入口**）
`golden_diag`（召回缺口诊断）—— 它是**产品可观测能力**（Hit/RankedOut/NotInPool 分类）
⇒ ⇒ 建议：**保留，但改名归族**（见 §五命名表）。

### 第 3 层：**实验/报告层**（**下沉到 `tests/` 或 `examples/`**）
`golden_baseline` · `golden_injection` · `golden_injection2` · `golden_mining` · `golden_run`
⇒ ⇒ 它们**只被 `examples/` 与 `tests/` 引用**（实测产品侧引用 0～1 处）
⇒ ⇒ ⇒ **下沉**：从 `pub mod`（产品 API 面）移到 `tests/` 支撑模块或 `examples/` 内部
⇒ ⇒ ⇒ **收益**：**4426 行里约 3265 行**离开产品 API 面 ⇒ ⇒ 下游 3 个 crate 少编这些。

### 第 4 层：**SQLite 实现统一**（**最大的一块，但要分批**）
抽 `SqliteStore<T>` 泛型基座 ⇒ ⇒ 6 个 `*_store_sqlite` 逐个迁移
⇒ ⇒ **必须分批**（每个 store 的表结构与事务语义不同 ⇒ ⇒ **一次迁一个** ⇒ ⇒ 每批跑全量测试）。

---

## 五、📌 **给 owner / 总指挥的选项**

### 选项 A：**只做第 3 层（下沉 5 个夹具模块）**（**我建议先做这个**）
⇒ ⇒ 收益：产品 API 面少 **3265 行**（占 crate 4%）· 下游 3 个 crate 少编这些
⇒ ⇒ 风险：**低**（无行为变更 ⇒ ⇒ 引用只从 `examples`/`tests` 改到新位置）
⇒ ⇒ 代价：`golden_injection`（第一轮）**要不要一起搬** ⇒ ⇒ 搬 ⇒ 则历史遗留清掉；不搬 ⇒ 则留个纪念

### 选项 B：**A + 命名归族**（**A 做完顺手把名字理顺**）
⇒ ⇒ 给 7 个 `golden_*` 定**角色前缀**：`golden_set`（卷）· `golden_diag`（诊断）·
`golden_report::*`（报告）· `golden_experiment::*`（实验）
⇒ ⇒ 收益：**新人一眼能看出哪个是权威**
⇒ ⇒ 代价：**破坏性改名** ⇒ ⇒ 需全仓改引用（`golden_mining` 86 处 + `golden_run` 88 处 ⇒ ⇒ 机械替换）

### 选项 C：**只做第 4 层（SQLite 基座）**（**收益最大、风险也最大**）
⇒ ⇒ 收益：**2962 行复制代码**收敛成一份 ⇒ ⇒ 改表结构/连接池**只改一处**
⇒ ⇒ 风险：**中**（6 个 store 逐个迁 ⇒ ⇒ 每批跑全量测试 ⇒ ⇒ **6 批**）

### 选项 D：**什么都不做**（**等更完整的「记忆引擎 v3 设计」**）
⇒ ⇒ 理由：179 模块 / 81k 行 ⇒ ⇒ **可能本来就该有一轮重构** ⇒ ⇒ 现在做会**做两遍**
⇒ ⇒ 代价：编译成本与 API 面混浊继续存在

### 我的建议
**A 先做**（低风险、立刻见效、可回滚）⇒ ⇒ **B 跟着做**（改名是机械活）⇒ ⇒
**C 单独立项分 6 批**（**不要和 A/B 混在一批** ⇒ ⇒ 出问题时定位不到）
⇒ ⇒ **D 作为前置判断**：若你判断「记忆引擎 v3 要重新设计」⇒ ⇒ **A/B 都可以不做**（直接 v3 里一次解决）。

---

## 六、我这次**没有做**什么

| 没做 | 原因 |
|---|---|
| **没动一行代码** | 动 API 形状（`pub mod` 下沉 / 改名）⇒ ⇒ **破坏性** ⇒ ⇒ 属架构裁决 |
| **没把 `golden_set` 搬到 `tests/`** | 它是 **CI 闸门的权威数据源**（`check_no_regression`）⇒ ⇒ 搬走 ⇒ ⇒ 闸门失源 |
| **没动 `*_store_sqlite` 的任何一行** | 6 个 store 逐个迁 ⇒ ⇒ 每批都要跑全量测试 ⇒ ⇒ **单独立项** |
| **没用「模块数 / 撞名数」当收敛指标** | 实测证明那是**坏指标**（`new` 159 次是不同类型的标准方法）⇒ ⇒ **判据必须是「同一能力多个入口」** |

## 七、复现命令

```sh
cd /home/bai/Projects/apeireth-debt94-wt
find crates/engine/memory/src -name '*.rs' | wc -l            # 179
find crates/engine/memory/src -name '*.rs' -exec cat {} + | wc -l   # 81108
grep -rn "pub mod golden" crates/engine/memory/src/lib.rs    # 7 个夹具在产品 API 面
wc -l crates/engine/memory/src/golden_*.rs                   # 夹具总行数
```

## 相关文件

- 跨区孤儿清单：[`unwired-cross-zone-list-2026-11-05.md`](./unwired-cross-zone-list-2026-11-05.md)
- `DailyNote` 工具规格：[`dailynote-mcp-tool-spec-2026-11-04.md`](./dailynote-mcp-tool-spec-2026-11-04.md)
- 记忆线状态：[`memory-engine-status-2026-11-03.md`](./memory-engine-status-2026-11-03.md)
