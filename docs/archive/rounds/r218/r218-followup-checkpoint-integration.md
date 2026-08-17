# R218 followup: Council deliberation checkpoint 集成 (接续 R212)

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R218 followup
> **日期**: 2026-08-13
> **来源**: R212 Checkpoint + CheckpointStore + 主人"全做全做全补弱"
> **状态**: 实施完成, 10/10 单测全过

---

## 0. 动机

R212 提供了 Checkpoint + CheckpointStore 数据结构 (memory + file 双实现). 但 Council::deliberate() 流程仍未集成 — 没人调用 store.put, 也无法 resume.

R218 followup 加 2 自由函数:
- `run_with_checkpoints(council, store, query)` — 替代 deliberate(), 每步写 checkpoint
- `resume_with_checkpoints(council, store, last_cp, query)` — 从 checkpoint 续

---

## 1. 设计

### 1.1 公共 API

```rust
pub fn run_with_checkpoints(
    council: &mut Council,
    store: &dyn CheckpointStore,
    query: CouncilQuery,
) -> CouncilVerdict;

pub fn resume_with_checkpoints(
    council: &mut Council,
    store: &dyn CheckpointStore,
    last: Checkpoint,
    query: CouncilQuery,
) -> CouncilVerdict;
```

### 1.2 自由函数 vs 方法

不用 `impl Council` 新增方法, 而用自由函数. 原因:
- 0 触碰 deliberation.rs (避免改 Council 已有 API)
- `Council::advisors_iter` + `Council::weights_clone` 是新增的 2 个 public getter, 不破坏 24 LOCKED

### 1.3 run_with_checkpoints 流程

1. 分配唯一 session_id (AtomicU64 SESSION_SEQ)
2. 遍历 advisors, 调 `deliberate()`, 写 opinion
3. 触发 OpinionIssued event (via `council.emit_event`)
4. **写 step checkpoint** (含 opinions_so_far + current_step)
5. 跑 synthesis + hold 判定
6. **写 final checkpoint** (current_step = total_steps, 标记完成)
7. 返回 CouncilVerdict

### 1.4 resume_with_checkpoints 流程

1. 从 `last.opinions_so_far` 恢复 opinions
2. `advisors_iter().skip(last.next_step())` 跳过已完成
3. 同 run 流程 (写 checkpoint + synthesis)
4. 返回 CouncilVerdict (session_id 与原 checkpoint 一致)

### 1.5 Council::advisors_iter + weights_clone

新增 2 个 public 方法 (R218 followup):
```rust
impl Council {
    pub fn advisors_iter(&self) -> std::slice::Iter<'_, Box<dyn Advisor>> { ... }
    pub fn weights_clone(&self) -> SynthesisWeights { ... }
}
```

不破坏任何现有 API, 不动 advisors/weights 私有字段.

---

## 2. 测试覆盖 (10 cases)

| ID | 用例 | 覆盖点 |
|---|---|---|
| t01 | run_writes_checkpoints | 2 advisors → 3 checkpoints (2 step + 1 final) |
| t02 | run_completes_all_advisors | 5 advisors → 6 checkpoints + final.is_complete() |
| t03 | resume_skips_completed | 中断后从 step 3 续 |
| t04 | resume_from_complete_returns_same | complete checkpoint 续不报错 |
| t05 | session_id_unique | 多次 run session_id 不重复 |
| t06 | progress_increases | checkpoint progress 单调递增 |
| t07 | opinions_accumulate | opinions_so_far 累积 |
| t08 | run_with_zero_advisors | 0 advisor 边界 |
| t09 | hold_trigger_with_strong_disapprove | StrongDisapprove 触发 hold |
| t10 | resume_preserves_session_id | resume 后 session_id 一致 |

---

## 3. 0 触碰守门

- deliberation.rs 加 2 个 NEW public 方法 (advisors_iter + weights_clone) — 不动 24 LOCKED 入口签名
- checkpoint.rs (R212) 0 改
- 7 强制 advisor 0 改
- 3 不可变脊柱 0 触碰
- workspace.version 1.2.0 0 改
- 0 新增 Cargo.toml 依赖

---

## 4. 路线意义

R218 followup 完成后, Council 战区:
- R25/R33-4: 7 advisor + 5 synthesis + 21 源文件
- R212: Checkpoint + CheckpointStore (memory + file)
- **R218 followup: run/resume 集成 (LangGraph 风格端到端)**
- 22 测试 (12 R212 + 10 R218 followup)

Apeireth 现在有完整的 "deliberation 中断恢复" 能力 — 7 advisor 跑长 deliberation 中途 crash/timeout, 续传即可.

---

## 5. 下一步

- **R215** evolution library_autonomy 加 Voyager API (2-3 days)
- **R214** relation petgraph 强化 (1 day)
- **R216** bus 三套通知 (R148 已做) 加测试覆盖 (1 day)
- **R219** api axum 升级 (1 day)
- **R220** pybridge pyo3-asyncio (1-2 days)
