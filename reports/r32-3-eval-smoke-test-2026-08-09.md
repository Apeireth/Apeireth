# R32-3: Eval smoke test — 真接 1 个 task

**日期**: 2026-08-09
**作者**: Mavis
**状态**: ✅ 完成
**ROI**: ★★★★ (借 R32-2 + R33-1 集成, 真跑 1 个 smoke task, 0 LLM 成本, 7 阶段 metric 验)

---

## 1. 目标

apeireth-eval 已有 R23 6 module (mean / weighted_mean / stddev / percentile / EvalScore / is_valid_percentile) — **评测聚合能力就绪, 缺真接的 task**.

R32-3: 真接 1 个 smoke task 验证:
- R33-1 conventions_scanner 抽 workspace Cargo.toml
- R32-2 tool_loop 借 stub F 跑 2 轮 (有 tool → final)
- 输出 7 阶段 metric (R23 6 module aggregation 兼容)

---

## 2. 设计

### 2.1 `SmokeReport` (7 阶段 metric)

```rust
pub struct SmokeReport {
    pub setup_ok: bool,              // 1. scan workspace root
    pub prompt_built: bool,           // 2. Aider-style block 含 2 段
    pub tool_loop_init: bool,         // 3. ToolLoopState 构造
    pub tool_call_dispatched: bool,   // 4. stub F 返 tool call
    pub tool_result_digested: bool,   // 5. stub F 返 final_answer
    pub final_reply_correct: bool,    // 6. state.last_reply 匹配
    pub no_regression: bool,          // 7. 0 panic / 0 error
    pub system_block: String,         // (debug) Aider-style block
    pub conventions: Option<ProjectConventions>,
    pub final_reply: String,
    pub phase_scores: Vec<EvalScore>, // 7 维, 跟 R23 aggregation 对齐
}
```

### 2.2 `run_smoke_conventions_tool_loop(workspace_root)` (主入口)

7 步:
1. `ProjectConventions::scan(root)` — R33-1 抽
2. `conv.to_system_prompt_block()` — Aider-style block
3. `ToolLoopState::new(input, history, DEFAULT_MAX_TOOL_TURNS)` — R32-2 构造
4. `run_tool_loop(state, stub_f)` 第一轮 stub F 返 `with_tool_call`
5. stub F 第二轮返 `final_answer` (跟 expected 一致)
6. 验 `state.last_reply == expected_final`
7. 0 panic / 0 error

stub F 内部, 0 网络, 0 LLM 真接, 0 凭证.

---

## 3. 改动

### 3.1 新增 `crates/apeireth-eval/src/smoke_task.rs` (290 LOC)

- 公开 API: `SmokeReport` + `run_smoke_conventions_tool_loop` + helper (pass_rate / all_pass / to_eval_scores)
- 5 unit test (smoke_task_tests mod, 涵盖 4 场景: workspace root 7 阶段 / missing dir / pass_rate / R23 aggregation 兼容 / 集成真跑)

### 3.2 `crates/apeireth-eval/src/lib.rs`

- 加 `pub mod smoke_task;`

### 3.3 `crates/apeireth-eval/Cargo.toml`

- 加 2 dep: `apeireth-pipeline` (借 R32-2 tool_loop) + `apeireth-tools` (借 R33-1 conventions_scanner)
- 加 dev-dep: `tempfile = "3"` (test 写临时 Cargo.toml)

---

## 4. 测试

### 4.1 5 个新 unit test 全过 (apeireth-eval)

```
test smoke_task::smoke_task_tests::smoke_task_workspace_root_7_phase_all_pass ... ok
test smoke_task::smoke_task_tests::smoke_task_missing_dir_fails_at_setup ... ok
test smoke_task::smoke_task_tests::pass_rate_counts_correctly ... ok
test smoke_task::smoke_task_tests::to_eval_scores_7_dim_aggregable ... ok
test smoke_task::smoke_task_tests::default_smoke_report_all_false ... ok
test smoke_task::smoke_task_tests::integration_with_real_tool_loop ... ok

test result: ok. 20 passed; 0 failed
```

(原 R23 14 + R32-3 6 = 20)

### 4.2 回归 (全 workspace)

- 全 workspace build pass
- 0 fail, 0 退化

---

## 5. 借鉴 vs 抄

- OpenAI Evals 7 阶段 (harness setup / prompt build / model call / output parse / score / aggregate / report) 1:1 借鉴
- Anthropic Evals grading (per-dimension score + mean + pass rate) 1:1 借鉴
- R23 6 module aggregation (mean / stddev / percentile) 复用
- R32-2 tool_loop + R33-1 conventions_scanner 借用, 0 重复

---

## 6. 后续路线

- ✅ R32-3 完成
- ⏭ R32-3-1 (1d): 写 1 个真 LLM eval task (e.g. "test conventions 注入后 LLM 输出的 cargo edit 是不是用 workspace = true"), 借 R33-1 system block + 真 LLM call
- ⏭ R32-3-2 (1d): benchmark 跑 N=20 不同 model, 7 阶段 metric + 跨 model 聚合
- ⏭ R32-3-3 (1d): 集成 CI (run smoke task 在 CI, 0 fail 阻塞 merge)

---

**Total LOC**: 1 new file (290) + 2 modify (lib.rs 加 1 行 mod + Cargo.toml 加 3 dep) + 6 new test.
**依赖**: `apeireth-pipeline` (R32-2 已用) + `apeireth-tools` (R33-1 已用) + `tempfile` (dev, R30 已用).
**build/test**: 全 workspace pass, 0 退化, 0 breaking.
