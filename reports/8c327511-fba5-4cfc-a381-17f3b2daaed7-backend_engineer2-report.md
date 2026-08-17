# TP22 Observer 捕获强化（E1+W5）验收报告

- 任务 ID: `8c327511-fba5-4cfc-a381-17f3b2daaed7`
- 角色: backend_engineer2
- 日期: 2026-08-18
- 范围: TP22（E1 + W5 工具结果→经验直通管道）

---

## 1. 交付清单

| # | 文件 | 类型 | 说明 |
|---|---|---|---|
| 1 | `crates/apeireth-companion/src/observer_capture.rs` | 新模块 | ExperienceCandidate / Outcome / ExperienceSource 类型 + ExperienceQueue 队列 + ObserverCaptureHook |
| 2 | `crates/apeireth-companion/src/lib.rs` | 修改 | `pub mod observer_capture` + 8 项 `pub use` 导出 |
| 3 | `crates/apeireth-companion/src/tool_bridge.rs` | 修改 | `with_observer_capture()` builder + `post_hooks_len()` 测试口（向后兼容, 旧 API 不破） |
| 4 | `docs/backlog.md` | 修改 | W5 行翻 ✅ + TP22 已完成项登记 (line 36 + 224) |

---

## 2. 设计要点（按任务边界）

### 2.1 Hook 触发点
- `ObserverCaptureHook` 实现现有 `PostExecuteHook` trait（tool_bridge.rs:185）, 与已有 `WrapHook`/`BlockHook` 同源
- 挂入位置: `ToolBridge::with_observer_capture()` → 自动实例化 hook → 推到 `post_hooks` 链尾, 确保拿到最终态 ExecutionResult
- 触发时机: 与所有 post-hook 一致 — `execute_if_allowed` 末尾、`record_execution` 审计之前

### 2.2 即时沉淀（不直接 put_episode）
- `ExperienceCandidate { tool, args_hash, outcome, ts_ms, source }` 入候选队列（`ExperienceQueue::pending: Vec<...>`）
- **不调用 `store.put_episode()`** 把候选升级为正式记忆
- 正式沉淀路径由 reflection / 对账周期通过 `drain_pending()` 消费（与 E1 反思期闭环衔接, 留接口给后续任务）
- 候选本身仍写到 sqlite (`id` 前缀 `expc-`, content=JSON), 但语义 = 候选池快照, 非正式记忆

### 2.3 去重（24h 窗口）
- in-memory LRU: `HashMap<(tool, args_hash), ts_ms>` + `VecDeque` 淘汰顺序, 默认容量 1024
- SQLite 兜底: LRU 漏掉（被淘汰或首次启动）→ `recent_episodes` 查询窗口内 sqlite 记录, O(N) 但 N≤4096 一次性扫描可接受
- 跨重启: `rehydrate()` 启动时复活窗口内的 LRU 索引 + 候选池, 验证: `sqlite_persistence_survives_rehydrate` 测试

### 2.4 args_hash 稳定化
- `sha256(canonical_json(args))` 取前 16 hex 字符（64 bit 安全边际, 碰撞概率 2^-64）
- `serde_json::to_string(&Value)` 在 `serde_json` 默认实现下, `Map` 键按字母序序列化 → 同 args 同 hash
- 验证: `args_hash_is_stable_and_distinguishes_args` 测试

### 2.5 Outcome 摘要截断
- 成功路径: `output` JSON 序列化 → 取前 200 字 + `…`
- 失败路径: `error` 字段 → 取前 200 字 + `…`
- 防止超大输出污染候选队列（与 task boundary 要点"产物摘要"对齐）

### 2.6 向后兼容
- `with_post_hook()` / `with_judicator()` / `with_spill()` / `with_goals()` / `with_isolation()` / `with_sandbox_config()` 旧 API 全部不动
- `ToolBridge::new(store)` 默认仍无 observer hook, 行为与改前一致
- 已有 22 个 tool_bridge 测试 + 8 个 experience 测试 + 3 个 memory_extractor 测试全部继续绿

---

## 3. 验收测试矩阵

| 验收项 | 测试名 | 结果 |
|---|---|---|
| Hook 触发 — 成功路径 | `hook_fires_on_success_path` | ✅ |
| Hook 触发 — 失败路径 | `hook_fires_on_failure_path` | ✅ |
| 沉淀入队 | `hook_fires_on_success_path` + `drain_pending_clears_queue` | ✅ |
| LRU 去重 | `dedup_within_window_suppresses_duplicate` | ✅ |
| 24h 窗口过期 | `dedup_allows_after_window_expires` | ✅ |
| 不同 args 不去重 | `different_args_hash_not_deduped` | ✅ |
| SQLite 持久化跨重启 | `sqlite_persistence_survives_rehydrate` | ✅ |
| 与现有 tool_bridge API 兼容 | `tool_bridge_backward_compat_post_hook_chain_still_works` + 22 个旧 tool_bridge 测试 | ✅ |
| Outcome 摘要截断 | `outcome_from_result_truncates_long_strings` | ✅ |
| args_hash 稳定 + 区分 | `args_hash_is_stable_and_distinguishes_args` | ✅ |
| LRU 容量淘汰 | `lru_cap_evicts_oldest` | ✅ |
| drain 清空 | `drain_pending_clears_queue` | ✅ |

11 个新测试 + 22 个 tool_bridge 旧测试 = **33 个 tool_bridge 模块测试全绿**。

---

## 4. 命令验证

```bash
$ cargo test -p apeireth-companion --lib observer_capture
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 522 filtered out

$ cargo test -p apeireth-companion --lib -- observer_capture tool_bridge
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 500 filtered out

$ cargo check -p apeireth-companion --lib
(error count = 0, 仅 apeireth-tool-shell 的预存 missing_docs warnings)
```

---

## 5. 边界声明（0 假装）

| 项 | 当前实现 | 升级路径 |
|---|---|---|
| 候选 → 正式记忆 promote | 仅 `drain_pending()` 暴露, 未实现 LLM 提炼 promote 链路 | E1 reflection 周期接续 TP22 队列 → 复用 `memory_extractor::apply_reconcile` |
| `ExperienceSource` 枚举 | 仅 `ToolExecution` 一个变体 | 后续 Dialog / Reflection / Dream 源可扩展, `serde rename_all = snake_case` 已对齐 |
| LRU 容量 1024 | hardcode, 不配 env | 若需可调, 走 `ExperienceQueueConfig { lru_cap, window_ms }` builder 即可 |
| SQLite 兜底查询 | `recent_episodes("me", 4096)` 一次性扫 | 候选量超 4K 时需加专用索引 (`expc_idx` on (tool, args_hash, ts)); 当前规模无需 |
| hook 顺序 | `with_observer_capture` 推到 post_hooks 链尾 | 若上游 hook 改 success 字段, observer 看到的是改后值, 符合"最终态"语义 |

---

## 6. 未触碰禁踩区（确认）

按 docs/next-team-handbook §1 + 团队 LOCKED 列表确认未触碰:
- `companion/approval_requests.rs` (WIP 锁)
- `companion/daemon.rs`
- `companion/experience.rs` (只读借用 `SqliteMemoryStore`, 未改)
- `companion/memory_extractor.rs` (本任务产出可被其未来 promote 复用, 未改)
- `companion/principles.rs`
- `companion/reflection.rs` (E1 反思期接续留口)
- `apeireth-team-lead` / `apeireth-tool-runtime` / `apeireth-agent` / `apeireth-credentials` / `apeireth-supervisor`

---

## 7. 与 W5 直通管道对齐

任务边界明确"工具执行结果 hook → 经验库"。本任务交付的是**第 1 段: 候选入队**:
- ✅ 工具执行完 → 候选入队（不等反思周期）
- ⏳ reflection 周期 → 候选 promote 到 `experience.rs::ExperienceStore`（TP22 后续 / E1 反思期接续）
- ⏳ `save_experience` 工具暴露给 LLM 主动查询 / 验证（已存在, 与本任务衔接）

候选池作为**短期记忆**与 `experience::ExperienceStore`（**长期记忆**）分层, 短期→长期由反思周期调度, 不在 TP22 范围。

---

## 8. 提交状态

- 工作树状态: 文件已写入, 待 git commit（rebase 后, 需 integration worktree 统一收编）
- 任务框架提示: 已自动重派 1 个冲突任务（团队冲突解决机制触发 rebase）, 本次提交已 redo
- 待办移交:
  - E1 反思期 promote 链路（候选 → `experience.rs::ExperienceStore.save()`）
  - 候选池 dump 调试 hook（serve 端）
  - LRU 容量 env 化（若运维反馈需要）
