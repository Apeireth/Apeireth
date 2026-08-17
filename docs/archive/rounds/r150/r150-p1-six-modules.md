# R150 P1 终极补弱 6/7 完成 (2026-08-13)

> **定位**: R150 = P1 7 项中的 6/7 (跳过 P1 #7 pipeline Temporal-style Activity,
> 重构风险大, 留 R151+). 全部 0 触碰 3 不可变脊柱 (Self-Disable / L0 HA / 13 键 verdict cache).
>
> **本次覆盖**: vector / state / cron / council / eval / test 6 个模块升级, 全部 0 引外部大依赖.

---

## 0. 总览

| 子模块 | 目标 | 状态 | 新增行数 | 新增测试 |
|---|---|---|---|---|
| `#6` `apeireth-vector::qdrant_compat` | Qdrant HTTP REST 协议兼容层 | ✅ | 581 | +11 |
| `#8` `apeireth-state::statechart` | XState-style statechart 引擎 | ✅ | 537 | +13 |
| `#9` `apeireth-cron::scheduler` | tokio cron 引擎 (0 引外部) | ✅ | 381 | +13 |
| `#10` `apeireth-council::session_capture` | council session 自动捕获 (claude-mem 模式) | ✅ | 431 | +17 |
| `#11` `apeireth-eval::swe_bench` | SWE-bench 风格 task runner | ✅ | 415 | +13 |
| `#12` `apeireth-test::property_tests` | proptest property-based testing | ✅ | 237 | +9 proptest |
| `#7` pipeline Temporal-style Activity | 重构 pipeline 为 workflow+activity | ⏸️ 跳过 | — | — |

**总计**: 6/7, 2582 lines new code, +76 tests (含 9 proptest blocks × 256 cases).

---

## #6 — `apeireth-vector::qdrant_compat`

**借鉴**: qdrant/qdrant (22K+ stars, Rust 向量 DB 标杆) + qdrant/rust-client 官方 SDK
- 0 引官方 rust-client (那是 gRPC 依赖, 我们仅 REST 协议)
- 0 触碰 VectorStore sync trait: QdrantClient 是独立 async 协议层

**新增文件**: `crates/apeireth-vector/src/qdrant_compat.rs` (581 lines)

**8 公共结构** (1:1 镜像 Qdrant REST API v1.7+):
- `CreateCollectionRequest` / `VectorParams` — `PUT /collections/{name}`
- `PointStruct` / `UpsertPointsRequest` — `PUT /collections/{name}/points`
- `SearchRequest` — `POST /collections/{name}/points/search`
- `ScoredPoint` / `CollectionInfo` / `CollectionConfig` — 响应

**4 距离度量** (1:1 跟 Qdrant server Distance enum):
- `QdrantDistance::Cosine` / `Euclid` / `Dot` / `Manhattan` (PascalCase 序列化)

**QdrantClient** (async HTTP, Clone):
- `new(url, collection)` / `with_distance()` / `with_http_client()`
- `ensure_dimension(dim)` — 创建/更新 collection (idempotent)
- `upsert(id, vector, payload)` / `upsert_batch(points)` — 写入
- `search(vector, k)` — top-k 检索
- `delete(id)` — 按 ID 删
- `collection_info()` — 集合信息 (含 points_count)
- `to_search_hits(scored)` — 转 VectorStore::SearchHit (跟 sqlite backend 统一调用层)

**QdrantError** (7 variant, thiserror):
- HttpClient / Server{status,body} / Deserialization / DimensionMismatch / Uuid / CollectionNotFound / DimensionNotSet

**apeireth-http-client 扩展**: 加 `put()` / `put_json()` / `delete()` 方法 (Qdrant 用 PUT/DELETE).

**11 unit tests**: create_collection_request_serializes / upsert_points_request_serializes / search_request_serializes / scored_point_deserializes / distance_metric_pascal_case_serialization / client_new_strips_trailing_slash / with_distance_overrides / to_search_hits_round_trip / error_display_messages / from_http_client_error_conversion / r150_qdrant_compat_deliverables

---

## #8 — `apeireth-state::statechart`

**借鉴**: statelyco/xstate (28K stars, statechart 行业标准)
- 0 引官方 xstate crate (那是 JS 移植, 类型 API 复杂)
- 自实现 XState 子集 (atomic/compound/final + transition + guard + action + on_entry/on_exit)
- Arc<dyn Fn> 让 Transition 派生 Clone

**新增文件**: `crates/apeireth-state/src/statechart.rs` (537 lines)

**核心类型**:
- `StateKind` enum: Atomic / Compound / Final
- `Transition { event, target, guard, action }` (Clone)
- `StateNode { id, kind, initial, transitions, on_entry, on_exit }` (Clone)
- `MachineContext { data: HashMap<String, ContextValue> }` — 业务数据
- `ContextValue` enum: Bool / Int / Str (POD 友好, Kani 友好)
- `Machine { states, initial, current, context, event_count, transition_count }`

**TransitionResult** enum:
- Transitioned { from, to } / NoTransition { reason } / Done { final_state } / UnhandledEvent { event }

**Builder helpers**:
- `atomic_state(id)` / `final_state(id)` / `compound_state(id, initial)`
- `with_transition(state, event, target)` / `with_guarded_transition(state, event, target, guard)`

**13 unit tests**: machine_initial_state / sends_event_transitions / cycles_through_states / unhandled_event / guard_rejects_transition / action_invoked_on_transition / on_entry_on_exit_invoked / final_state_terminates / compound_state_has_initial / reset_clears_state_and_context / context_value_introspection / atomic_and_final_state_helpers / r150_statechart_deliverables

---

## #9 — `apeireth-cron::scheduler`

**借鉴**: questdb/tokio-cron-scheduler (800 stars, Rust cron 调度标杆)
- 0 引外部 cron crate (workspace ponytail ceiling)
- 自实现: per-tick evaluation + job handle + shutdown channel + epoch→cron 转换

**新增文件**: `crates/apeireth-cron/src/scheduler.rs` (381 lines)

**CronEngine**:
- `new()` / `default()`
- `add(id, expr, callback)` — 注册 cron job (id 重复返 DuplicateJob)
- `remove(id)` — 移除 job
- `list_jobs()` — 返 Vec<CronJobInfo> (按 id 排序)
- `start()` → JoinHandle (tokio spawn, 每 60s tick)
- `shutdown()` — notify_one 退出 loop
- `is_running()` / `job_count()`

**CronJob** + **CronJobInfo**:
- `id` / `expr: CronExpr` (既有) / `callback: Arc<dyn Fn() + Send + Sync>`
- `last_fired_at: i64` / `fire_count: u64`
- `CronJobInfo` 仅序列化的摘要 (不暴露 callback)

**epoch_to_cron_fields** (粗略 UTC 转换, 0 引 chrono/time):
- 输入 epoch secs → (minute, hour, dom, month, dow)
- 验证所有字段在 cron 合法范围内

**SchedulerError** (4 variant):
- AlreadyRunning / NotRunning / DuplicateJob / UnknownJob / CronParse

**tokio dev-dep**: `tokio = { workspace = true, features = ["rt", "rt-multi-thread", "time", "macros", "sync"] }`
serde_json dev-dep 加 (CronJobInfo serialization test)

**13 unit tests**: engine_new_is_empty_and_not_running / add_and_list_jobs / duplicate_id_rejected / remove_job / remove_unknown_rejected / invalid_cron_rejected_on_add / list_jobs_sorted_by_id / epoch_to_cron_fields_in_range / cron_job_info_serialization / start_requires_not_running / callback_is_invokable / default_engine / r150_cron_scheduler_deliverables

---

## #10 — `apeireth-council::session_capture`

**借鉴**: claude-mem (24K stars, session 捕获模式)
- 0 引外部 LLM/dep (纯 std + serde)
- session 生命周期内自动捕获 council deliberation messages

**新增文件**: `crates/apeireth-council/src/session_capture.rs` (431 lines)

**CouncilSession**:
- `session_id` / `started_at_ms` / `ended_at_ms` (0 = 未结束)
- `messages: Vec<SessionMessage>`
- `duration_ms()` / `is_ended()` / `message_count()` / `messages_by_advisor()`

**SessionMessage**:
- `seq` (session 内单调, 全局 seq counter) / `advisor_id` / `role` / `content` / `timestamp_ms`
- `metadata: HashMap<String, String>` (e.g. persona, confidence)

**SessionCapture** engine:
- `new()` / `default()`
- `start_session(id)` — 若 current 存在则自动 end + 归档到 history
- `end_session()` — 返结束的 session (clone)
- `record_message(advisor_id, role, content, metadata)` — append 到 current (若无 no-op)
- `find_session(id)` — current + history 搜索
- `search_history(query, k)` — 跨 session 关键词匹配 (case-insensitive)
- `advisor_message_counts()` — 全 session 聚合
- `history_len()` / `total_sessions()` / `has_active_session()` / `current_session_id()`

**17 unit tests**: new_capture_has_no_active_session / start_session_creates_active / start_session_auto_archives_previous / end_session_moves_current_to_history / end_without_active_returns_none / record_message_without_active_is_noop / record_message_assigns_monotonic_seq / record_message_stores_advisor_role_content / find_session_searches_current_and_history / search_history_keyword_matching / search_history_top_k_limit / search_history_case_insensitive / search_history_empty_query_no_match / advisor_message_counts_aggregates / session_duration_when_active / session_messages_by_advisor / r150_session_capture_deliverables

---

## #11 — `apeireth-eval::swe_bench`

**借鉴**: SWE-bench Verified 1.0 (3K+ stars, OpenAI + Princeton)
- 每个 task = (issue 描述, expected patch, verification tests)
- 跑 task 集合, 输出 pass rate + per-category breakdown

**新增文件**: `crates/apeireth-eval/src/swe_bench.rs` (415 lines)

**SweTask**:
- `id` / `category` / `prompt` / `expected_patch` / `verification`
- `difficulty: u8` (1-5, default 3)

**SweTaskResult**:
- `task_id` / `category` / `passed` / `score: f64` (0.0-1.0) / `observed_patch` / `error`

**SweTaskExecutor** trait (Send):
- `execute(&self, task: &SweTask) -> SweTaskResult`

**3 Executor 实现**:
- `IdentityExecutor` — 总是返 expected_patch (用于 baseline 比对)
- `FailingExecutor` — 总是失败 (用于 baseline)
- `StringEqExecutor` — 简单字符串相等 (用于 unit test)

**TaskRunner + TaskSummary**:
- `run(tasks, executor)` 跑任务集合
- `summarize(results)` 聚合 (pass_rate, mean_score, per_category breakdown)
- `total` / `passed` / `failed` / `pass_rate` / `mean_score` / `per_category: Vec<CategoryBreakdown>`
- `to_eval_scores()` — 转 Vec<EvalScore> (跟既有 mean/weighted_mean 互转)

**13 unit tests**: identity_executor_all_pass / failing_executor_all_fail / mixed_categories_per_category_breakdown / empty_tasks_pass_rate_is_one / string_eq_executor_match / string_eq_executor_no_match / task_summary_to_eval_scores / swe_task_serialization_round_trip / swe_task_result_serialization / category_breakdown_serialization / default_difficulty_helper / summarize_from_results / r150_swe_bench_deliverables

---

## #12 — `apeireth-test::property_tests`

**借鉴**: alt-proptest/proptest (1.7K stars, Rust property-based testing 标杆)
- 0 触碰既有 13 unit test (#[test] 风格)
- 仅加 `proptest!` 块 (dev-dep only)

**新增文件**: `crates/apeireth-test/src/property_tests.rs` (237 lines)

**9 proptest 块** (per proptest 默认 256 cases each):
1. `prop_summary_total_equals_sum` — 任意 (passed, failed, skipped) → total = sum
2. `prop_summary_pass_rate_in_unit_interval` — 任意 inputs → pass_rate ∈ [0.0, 1.0]
3. `prop_retry_delay_monotonic` — 任意 (base, attempt1, attempt2) → delay 单调
4. `prop_retry_delay_saturates` — attempt ≥ 32 → delay = u32::MAX
5. `prop_budget_allows_iff_total_within` — budget.allows ⇔ total ≤ max
6. `prop_test_case_validate_monotonic` — retry_count 越大越不 valid
7. `prop_should_retry_iff_attempt_within` — should_retry ⇔ attempt < max_retries
8. `prop_summarize_consistency` — CaseResult 列表 → summarize counts 一致
9. `prop_flaky_cases_iff_retry_positive` — flaky_cases ⇔ retry_count > 0

**proptest dev-dep**: `proptest = { workspace = true }` (workspace.dependencies 已有 1.5)

**lib.rs 改动**: 加 `#[cfg(test)] mod property_tests;` 一行

---

## 1. 跳过 #7 (pipeline Temporal-style Activity)

**原因** (per R150 #7 决策):
- apeireth-pipeline 是 LLM chat 5 步管线 (per R17), 不是 workflow engine
- Temporal 范畴是 "long-running deterministic workflow + side-effect activity", 跟 chat pipeline 范畴不直接对应
- 真实施需要重构 pipeline 为 Activity trait + EventHistory, 风险大 (现有 30+ 测试要全改)
- 留 R151+ 续做: 单独建 `apeireth-workflow` crate, 包装 Temporal.io 概念, 不破坏现有 pipeline

**借鉴调研留档**: docs/research/r149-github-survey.md §7 (Temporal Workflow+Activity pattern)

---

## 2. 验证结果

```bash
# 各模块单测 (全部 0 failures)
cargo test -p apeireth-vector --lib      -> 29/29  (+11 qdrant_compat)
cargo test -p apeireth-state --lib        -> 82/82  (+13 statechart, 69 existing filtered)
cargo test -p apeireth-cron --lib         -> 25/25  (+13 scheduler)
cargo test -p apeireth-council --lib session_capture  -> 17/17
cargo test -p apeireth-eval --lib         -> 74/74  (+13 swe_bench)
cargo test -p apeireth-test --lib         -> 22/22  (+9 proptest, 13 existing)

# workspace 整体编译
cargo check --workspace                   -> 0 errors
```

---

## 3. 借鉴 ID 完整列表 (R150)

| ID | 来源 | 用处 |
|---|---|---|
| `R150-VECTOR-BORROW-qdrant-http-rest-api-2026-08` | qdrant/qdrant + rust-client | `apeireth-vector::qdrant_compat` |
| `R150-STATE-BORROW-statelyco/xstate-28k-stars-2026-08` | statelyco/xstate | `apeireth-state::statechart` |
| `R150-CRON-BORROW-tokio-cron-scheduler-800-2026-08` | questdb/tokio-cron-scheduler | `apeireth-cron::scheduler` |
| `R150-COUNCIL-BORROW-claude-mem-24k-stars-2026-08` | claude-mem | `apeireth-council::session_capture` |
| `R150-EVAL-BORROW-SWE-bench-3k-stars-2026-08` | SWE-bench Verified | `apeireth-eval::swe_bench` |
| `R150-TEST-BORROW-proptest-1.7k-stars-2026-08` | alt-proptest/proptest | `apeireth-test::property_tests` |

---

## 4. 文档交叉引用

- `docs/r150/r150-p1-six-modules.md` (本文件)
- `docs/research/r149-github-survey.md` §12 (R150 候选清单)
- `crates/apeireth-vector/README.md` (更新 qdrant_compat)
- `crates/apeireth-state/README.md` (更新 statechart)
- `crates/apeireth-cron/README.md` (更新 scheduler)
- `crates/apeireth-council/README.md` (更新 session_capture)
- `crates/apeireth-eval/README.md` (更新 swe_bench)
- `crates/apeireth-test/README.md` (更新 property_tests)
- `crates/apeireth-http-client/README.md` (更新 put_json + delete)

---

## 5. 终极目标进度

- R149 P0 5/5: tool-fetch / skills / runtime / graph / formal ✅
- R150 P1 6/7: vector / state / cron / council / eval / test ✅ (跳 #7 pipeline)
- R151+ P2 候选 (per r149-github-survey.md):
  - sovereignty Hyperlight micro-VM
  - relation SurrealDB 后端
  - voice GPT-Realtime-2
  - workflow Temporal (R150 跳过的 #7)
  - TUI 接入新 runtime
