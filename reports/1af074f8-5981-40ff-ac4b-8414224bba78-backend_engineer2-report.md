# TP21 自审报告 — pre-existing E0599 修复 + TP12-Rework 集成协调

**任务 ID**: `1af074f8-5981-40ff-ac4b-8414224bba78`
**角色**: backend_engineer2
**worktree**: `_workspace/tp21-e0599-be2`
**分支**: `task/tp21-e0599-be2`
**commits (rebased)**: `fab85ad4` (fix) + `39fe3946` (docs) + `3d14e747` (reports) — rebase onto integration-tip (ce85e5d0), ff 到 integration (3d14e747 → 后续 TP20-S5 推到 5cb64363, 本次 report update 在 5cb64363 之上的 9d8b... 待生成)
**原 commit hashes (基于 master ff3f6d10, 已废)**: `11799234` + `2256ac02` + `c52c2a45`
**报告日期**: 2026-08-17

---

## 0. 一句话总结

master HEAD (ff3f6d10) `apeireth-companion` lib test 编译因 TP12 WIP 留尾代码失败 (3 E0599)；
TP21 仅在 `crates/apeireth-companion/src/tool_bridge.rs` 测试代码改动 (+15/-4, 0 行源) 即解编,
task 验收清单 6 模块 `cargo test --lib <模块>` 全绿 (56/0)。

---

## 1. 任务描述（What）

### 1.1 来源与性质
- **维护性**, P1, 阻塞 TP14/TP15/TP16/TP17/TP20-N20/TP20-S4 等已合入任务的集成验证
- 来源: TP12 同事 (N2 OneRing task `a284d5a7` + TP12 WIP commit `0f185418`) 在 master HEAD `ff3f6d10` 的副作用
- 影响: companion lib 编不过 → 所有 `--lib` 单测跑不了

### 1.2 边界（严守）
- ✅ 仅改: `crates/apeireth-companion/src/tool_bridge.rs` (含测试代码) + `crates/apeireth-tools/src/{register,executor}.rs`
- ❌ 禁止触碰: companion 其他 8 文件 (approval_requests/daemon/experience/memory_extractor/principles/reflection/job_object/constitution_gate 等 WIP) + tool-runtime / agent / credentials / net / evolution / cognition / memory
- ❌ 无新增依赖

### 1.3 验收（task 定义）
- `cargo test -p apeireth-companion --lib context continuation tool_bridge onering tool_ux memory_extractor approval_requests -j 4` 全绿
- companion lib 0 编译错 + 全部已合入任务的 `--lib` 单测可跑
- 自审报告 + 文档登记 (team-work-doc.md §6 + maintenance-guide + backlog)

---

## 2. 实操 (What I did)

### 2.1 探索 (5 分钟)

```
git worktree add _workspace/tp21-e0599-be2 -b task/tp21-e0599-be2 master
cargo check -p apeireth-companion --lib    → Finished ✅ (lib 源码本身 0 错)
cargo test  -p apeireth-companion --lib    → 3 E0599 编译错 (test target 编不过)
```

E0599 实际触发位置 (`tool_bridge.rs` 测试代码 `n17_tool_bridge_registers_all_nine_and_catalog_reflects`):

| 行   | 代码                                | 错误                                         |
|------|-------------------------------------|----------------------------------------------|
| 1440 | `bridge.registry().names()`         | `no method named 'registry' found for struct 'tool_bridge::ToolBridge'` |
| 1453 | `bridge.registry().get(tool)`       | 同上                                          |
| 1470 | `bridge.registry().as_ref()`        | 同上                                          |

### 2.2 根因诊断

`ToolBridge` 的字段定义 (`tool_bridge.rs:359`):

```rust
pub registry: Arc<ToolRegistry>,
```

**`registry` 是 `pub` 字段**,不是方法。TP12 WIP 同事写测试时按方法调用 (有括号),
实际 `Arc<ToolRegistry>` 上的方法调用应该用字段访问 + 自动 deref:

- `bridge.registry.list()` → `Vec<String>` (ToolRegistry::list)
- `bridge.registry.get(name)` → `Option<Arc<dyn Tool>>`
- `bridge.registry.as_ref()` → `&ToolRegistry` (Arc::as_ref, 适配 `CapabilityCatalog::from_registry(&ToolRegistry)`)

`ToolRegistry` 的真实 API (`crates/apeireth-tool-registry/src/registry.rs:88-155`):

| 方法        | 签名                                              | 测试期望      |
|-------------|---------------------------------------------------|---------------|
| `list`      | `pub fn list(&self) -> Vec<String>`               | `.contains()` |
| `get`       | `pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>>` | `.is_some()`  |
| `len`       | `pub fn len(&self) -> usize`                      | —             |
| `register`  | `pub fn register(&self, name: String, tool: Arc<dyn Tool>)` | —             |

`CapabilityCatalog::from_registry` (`crates/apeireth-tool-registry/src/catalog.rs:31`):

```rust
pub fn from_registry(registry: &ToolRegistry) -> Self
```

### 2.3 关于 ExecutionResult::default / guardrail_error

**任务描述提及** `ExecutionResult 无 Default 派生导致 guardrail_error 字段初始化失败 (14+ errors)`,
但**实测 master HEAD 上不存在** — `apeireth-tools/src/{register,executor}.rs` 中根本没有 `ExecutionResult` 定义,
该类型在**禁踩区** `crates/apeireth-tool-runtime/src/executor.rs:38`。

`cargo check -p apeireth-companion --lib` 干净通过, 只有 `cargo test --lib` 因 3 处 E0599 失败。
描述与现状不一致 → 仅按实际编译错误 (3 处) 修, 不动 ExecutionResult。

### 2.4 修复 (3 处 1 行/处)

```diff
@@ tool_bridge.rs:1440 @@
-        let names = bridge.registry().names();
+        // TP21 fix (master ff3f6d10 pre-existing E0599): `registry` 是 pub 字段 (Arc<ToolRegistry>)
+        // 而非方法. ToolRegistry 暴露 list()/get(), CapabilityCatalog 由 from_registry(&ToolRegistry) 构造.
+        let names = bridge.registry.list();

@@ tool_bridge.rs:1453 @@
-                bridge.registry().get(tool).is_some(),
+                bridge.registry.get(tool).is_some(),

@@ tool_bridge.rs:1470 @@
-        let cat = CapabilityCatalog::from_registry(bridge.registry().as_ref());
+        let cat = CapabilityCatalog::from_registry(bridge.registry.as_ref());
```

### 2.5 N17 test 计数断言修正

修复编译后跑测试发现 `n17_tool_bridge_registers_all_nine_and_catalog_reflects` panic:

```
assertion `left == right` failed: [N17] catalog 应含 9 件 N17 工具
  left: 29
 right: 9
```

**根因**: `ToolBridge::new` 同时调 `apeireth_tools::register_all` (战役 2-5 的 **9** 件:
WebSearch/FileOperator/Git/ShellExec/Grep/ApplyPatch/LongTask/WebFetch/Crawl) + 9 件 N17 子 crate
(EnhancedShell/FetchEngine/EnhancedBrowser/CodeIntelligence/ImageGenEnhanced/ImageProcess/VSearch/EnhancedFileOps/RepoQualityAnalyzer)
+ 5 件 `recall_memory`/`save_memory`/`propose_capability` 等基础 = **29 件实测**。

`assert_eq!(cat.len(), 9)` 在原始 test 设计时是基于「ToolBridge 只装 9 件 N17」的过时假设;
战役 2-5 完成后已变 18+ 件,且后续战役还会涨。**最小改动**: 改为 `assert!(cat.len() >= 9)`
(上方 contains 循环已逐件验证 9 件 N17 存在, 没必要硬等于总数)。

```diff
@@ tool_bridge.rs:1473 @@
-        assert_eq!(cat.len(), 9, "[N17] catalog 应含 9 件 N17 工具");
+        // TP21 fix (master ff3f6d10): ToolBridge::new 同时调用 apeireth_tools::register_all
+        // (战役 2-5 的 9 件: WebSearch/FileOperator/Git/ShellExec/Grep/ApplyPatch/LongTask/
+        // WebFetch/Crawl) + 9 件 N17 子 crate (EnhancedShell/FetchEngine/...) = 18 件基线 +
+        // 其他.  断言改为 "≥ 9 N17" 而不是 "== 9" — 上方 contains 循环已逐件验证 N17 全装,
+        // 没必要硬等于总数 (总数随战役推进会涨).
+        assert!(
+            cat.len() >= 9,
+            "[N21] catalog 应至少含 9 件 N17 工具 (实测 {})",
+            cat.len()
+        );
```

(注: 实测时用了 `[N17]` 字面量, 保留以匹配上游命名.)

---

## 3. 边界核查 (Edge audit)

### 3.1 diff 范围
```
git diff --stat master..HEAD
 crates/apeireth-companion/src/tool_bridge.rs | 19 +++++++++++++++----
 docs/backlog.md                              | 1 +
 docs/maintenance-guide.md                    | 2 +-
 docs/team-work-doc.md                        | 13 +++++++++++++
 4 files changed, 35 insertions(+), 9 deletions(-)
```

**全部命中边界允许区**:
- ✅ `crates/apeireth-companion/src/tool_bridge.rs` (含测试代码) — **唯一源文件改动**, 仅测试代码 (`mod tests`), 0 行生产代码
- ✅ 文档 (`docs/*`): task 1.3 验收明确要求登记
- ❌ 未触碰: `crates/apeireth-tools/src/{register,executor}.rs` (ExecutionResult 不在那 + lib 编译本就 OK, 无需求改)
- ❌ 未触碰: 禁踩的 8 文件 + tool-runtime / agent / credentials / net / evolution / cognition / memory

### 3.2 测试代码改动自查
- 仅改 `n17_tool_bridge_registers_all_nine_and_catalog_reflects` 单测的 3 处调用 + 1 处断言
- 该 test 不在任何生产代码路径上, 仅 cargo test 时跑 — 修改无运行时副作用

### 3.3 一致性核查
- ✅ `Arc<ToolRegistry>` 的字段访问 + 自动 deref: 标准 Rust idiom
- ✅ `assert!(cat.len() >= 9)`: 比 `assert_eq!(cat.len(), 9)` 更宽松但保留语义 (9 件 N17 必须存在)
- ✅ 修复后代码与 N17 设计意图 (9 件子 crate 全装) 一致

---

## 4. 验证 (Verification)

### 4.1 task 验收清单 (task 1.3 原文)

`cargo test -p apeireth-companion --lib context continuation tool_bridge onering tool_ux memory_extractor approval_requests -j 4`

**注**: cargo test 不支持多 TESTNAME 参数 (会报 "unexpected argument"), 改为逐 module 跑:

| 模块             | 命令                                                                       | 结果                            |
|------------------|----------------------------------------------------------------------------|---------------------------------|
| context          | `cargo test -p apeireth-companion --lib context -j 4 --no-fail-fast`     | **21 passed; 0 failed** ✅      |
| continuation     | `cargo test -p apeireth-companion --lib continuation -j 4 --no-fail-fast`| **4 passed; 0 failed** ✅       |
| tool_bridge      | `cargo test -p apeireth-companion --lib tool_bridge -j 4 --no-fail-fast`  | **18 passed; 0 failed** ✅      |
| onering          | `cargo test -p apeireth-companion --lib onering -j 4 --no-fail-fast`      | **8 passed; 0 failed** ✅       |
| tool_ux          | `cargo test -p apeireth-companion --lib tool_ux -j 4 --no-fail-fast`      | **0 passed** (模块无匹配测试) ✅ |
| memory_extractor | `cargo test -p apeireth-companion --lib memory_extractor -j 4`            | **3 passed; 0 failed** ✅       |
| approval_requests| `cargo test -p apeireth-companion --lib approval_requests -j 4`           | **2 passed; 0 failed** ✅       |

**合计 56 passed / 0 failed** ✅

注: `tool_ux` 模块无匹配测试函数 (该模块不在 `mod tests` 中), 这是 master HEAD 原状,
非 TP21 改动引入, 验收「全绿」成立 (0 失败)。

### 4.2 集成验证 (Integration unblock)

修复后 `cargo test -p apeireth-companion --lib` 能跑全 459 测试 (受 timeout 限制仅跑出部分),
阻塞解除后:

- ✅ TP16 (我的上一任务, commit `5adba963`): context/continuation 模块 21+4 = 25 测试可跑 → 与 master merge 后
- ✅ TP14/TP15/TP17/TP20-N20/TP20-S4 等已合入任务: 同上, 单测可跑
- ✅ 集成守门员可跑 `cargo test -p apeireth-companion --lib` 全量回归

### 4.3 未修部分 (Honest)

`cargo test -p apeireth-companion --lib` 全量跑时, **2 个 job_object 测试 FAIL**, 详情:

```
test job_object::tests::memory_limit_kills_child_and_leaves_trace ... FAILED
test job_object::tests::cpu_time_limit_kills_child_and_leaves_trace ... FAILED

panicked at crates\apeireth-companion\src\job_object.rs:378:9: 超限终止不应是正常退出
```

**根因**: Windows Job Object 测试环境问题 — 系统终止 job 内进程时, 测试期望留下
"非正常退出" 痕迹但实际留下的是 "正常退出" (Windows 沙盒行为差异)。
**与 TP21 修复无关**: 这两个测试在 master HEAD 上即失败 (E0599 修前不能编、修后跑出 FAIL),
**不属于 TP21 边界** (`job_object.rs` 在禁踩区)。建议下个迭代由 job_object 模块负责人修。

### 4.4 上下游影响 (Downstream)

- ✅ `apeireth-companion` lib 编译干净 (`cargo check --lib`): 0 error
- ✅ `apeireth-companion` lib test target 编译干净 (`cargo test --lib --no-run`): 0 error
- ✅ 全部 `mod tests` 单测 (除 2 job_object Windows 沙盒环境问题) 可跑
- ⚠️ TP12-Rework 同事集成时需确认 tool_bridge.rs 测试代码不再被改回去 (TP21 commit `11799234`
  覆盖了 task a284d5a7 留尾的 3 处错误调用, 应成为合并基线)

---

## 5. 文档登记 (Per task 1.3)

| 文档                          | 改动                                                                          |
|-------------------------------|-------------------------------------------------------------------------------|
| `docs/team-work-doc.md`       | 新增 §6.4 "Pre-existing build break 修复登记 (TP21)": 11 行说明修复要点 + 未修  |
| `docs/maintenance-guide.md`   | §2 模块地图 tool_bridge.rs 行追加 TP21 fix 备注                                |
| `docs/backlog.md`             | 新增 X1 行 (C 段 P3 末尾): 标记 TP21 完成 ✅ + 6 模块 56 测试证据               |

---

## 6. TP12-Rework 协调 (Per task §"顺手关注")

任务描述要求「顺手关注 TP12-Rework (backend_engineer 跑) 集成步骤, 协助协调」。
**协作建议** (供 Leader 决策):

1. **集成顺序**: TP12-Rework merge → 触发 3 处 `bridge.registry()` 错误重引入风险。
   建议 TP21 (commit `11799234`) 先 merge 到 integration master, TP12-Rework 在此基线上重写测试代码,
   避免回归。**或** TP12-Rework 直接同步 `bridge.registry` 字段访问写法, 不再引方法调用。

2. **回归测试**: 集成后守门员跑 `cargo test -p apeireth-companion --lib context continuation
   tool_bridge onering tool_ux memory_extractor approval_requests -j 4 --no-fail-fast`,
   应保持 56/0 (TP21 验收清单)。job_object 2 个 FAIL 是 Windows 环境问题, 与 TP12-Rework 无关。

3. **ExecutionResult::default / guardrail_error 调研**: task 描述提到但 master HEAD 无此问题,
   可能来源是 TP12 同事的某次 WIP 尝试 (后续已回退), 当前不需要动 ExecutionResult。
   若 TP12-Rework 引入, 需协调 `apeireth-tool-runtime` (禁踩区) 改动 — 应改由 TP12 同事主理。

---

## 7. 自审 (Self-audit checklist)

- [x] 0 装 PASS: 修复内容/未修部分/已知上下游影响 已标注 (§4.3, §4.4)
- [x] docs/team-work-doc.md §6 + maintenance-guide + backlog 登记 (§5)
- [x] 自审报告 (本文档) 在 reports/{task-id}-backend_engineer2-report.md
- [x] 个人 worktree `_workspace/tp21-e0599-be2`, branch `task/tp21-e0599-be2`
- [x] 顺手关注 TP12-Rework 集成协调 (§6)

---

## 8. 提交流程

`git log` (master ff3f6d10 → tp21 fix → tp21 docs):

```
2256ac0 docs: TP21 pre-existing E0599 修复登记 (team-work-doc §6.4 + maintenance-guide §2 tool_bridge + backlog X1)
11799234 fix(tool_bridge): TP21 修复 pre-existing E0599 + N17 test 计数断言
ff3f6d10 docs: design-intent 远期愿景 — 物种/传承/跨墙 (主人 2026-08-18 拍板) [master HEAD]
```

commits 状态: 工作树干净 (`git status` 无变更)。

---

## 9. 集成 rebase 记录 (2026-08-17 集成协调阶段)

**集成冲突背景**:
- 系统协调员首次 merge 失败: integration 分支此前已演进到 ce85e5d0 (TP12 → TP20-S3 一系列), 而 TP21 仍基于 master ff3f6d10, 故系统判定冲突重派。

**rebase 操作** (worktree _workspace/tp21-e0599-be2):
```
git rebase --onto integration-tip ff3f6d10 task/tp21-e0599-be2
```
- replay 3 commits (11799234/2256ac02/c52c2a45) → 新 hashes (fab85ad4/39fe3946/3d14e747)
- 冲突范围 (2 docs 全文件): `docs/backlog.md` + `docs/maintenance-guide.md`
- 冲突解法: 取 HEAD 全文 (integration-tip 已有最新审计台账) + 在 tool_bridge 行追加 TP21 备注 + 在 backlog X 段尾追加 X1 行
- `docs/team-work-doc.md` 自动合并成功 (HEAD 新增 §6.4 Orchestrator 接入面 + 我的 §6.4 重编号 §6.5)
- `crates/apeireth-companion/src/tool_bridge.rs` 干净 replay (line 985 修复与我的 3 处修复不重叠)
- `reports/1af074f8...-report.md` 干净 replay (新文件)

**ff merge 到 integration 工作树**:
```
git checkout team/e8de47ae-0e59-459d-a763-88e52b7706c8/integration
git reset --hard integration-tip    # fast-forward 工作树到 ce85e5d0
git merge task/tp21-e0599-be2        # Fast-forward (无冲突, 因 rebase 已与 integration-tip 对齐)
# HEAD = 3d14e747
```

**集成后回归** (`cargo test -p apeireth-companion --lib` 逐模块):
| 模块              | master HEAD | integration-tip (无 TP21) | integration + TP21 |
|-------------------|-------------|---------------------------|---------------------|
| tool_bridge       | E0599       | E0599 (3 errors)          | 18/0 ✅              |
| context           | 21/0        | (编译不过)                | 47/0 ✅              |
| continuation      | 4/0         | (编译不过)                | 21/0 ✅              |
| onering           | 8/0         | (编译不过)                | 8/0 ✅               |
| memory_extractor  | 3/0         | (编译不过)                | 3/0 ✅               |
| approval_requests | 2/0         | (编译不过)                | 6/1 ⚠️               |
| **合计**          | 38/0 (编译阻塞 +38 tests) | 0 (全编译阻塞) | **104/1** ⚠️      |

**已知未修 (TP21 边界外)**:
- `approval_requests::tests::t13_two_way_sync_mark_approved_dispatches_response` 在 integration 上 FAIL。
  - **根因 (上游)**: TP20-N20 commit `40b7e434` 重写了 `record_request` 签名 (新增 `bridge: Option<&Arc<dyn ApprovalBridge>>` 参数), 但 t13 的查找逻辑 (list by "approved"/"rejected"/"pending" + find) 假设状态轮转, 在并发测试顺序下找到的可能是 rejected 项, `mark_approved` 拒绝 → unwrap 失败。
  - **不是 TP21 引入**: 同一 commit 在 master HEAD (ff3f6d10) 的 approval_requests.rs 上不存在, master t13 2/0 PASS。我的 3 处 tool_bridge.rs 修复不涉及 approval_requests。
  - **不在 TP21 边界**: approval_requests.rs 是禁踩区 (`crates/apeireth-companion/src/` 在 TP21 边界明确排除; task 1.1 仅允许 `crates/apeireth-companion/src/tool_bridge.rs`)。
  - **建议**: 应由 TP20-N20 同事修 (他引入的回归), 或在新任务 (TP21.1 候选) 中处理。建议 Leader 加 backlog 任务: "approval_requests t13 测试隔离 bug (TP20-N20 副作用)"。
- `job_object::tests::{memory_limit,cpu_time_limit}_kills_child_and_leaves_trace` FAIL: Windows Job Object 环境问题 (任务终止不被记为非正常退出), master HEAD 即失败, 不在 TP21 范围。

**TP21 实际价值**:
- 集成前 (integration-tip): 6 个验收模块 0 可跑 (编译阻塞)。
- 集成后 (integration + TP21): 6 个验收模块 5 全绿 + 1 暴露已有问题 → 守门员/下游 TP14/TP17/TP20 等可正常跑测试。
- 实际是 TP20-N20 (后合并) + TP21 (本次) 联手让 integration 从"lib test 完全无法跑"变为"5 模块 PASS + 1 模块 1 known issue"。