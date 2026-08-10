# Agent R121r-1 Readmap — 5 任务具体形状 + 决策 (2026-08-10)

**时间**: 2026-08-10 10:15-10:45 (~30 min, 超 5 min)
**作者**: 团队成员 R121r (Mavis 派, 工程化战区, 主人 #10 授权自主决策)
**战区**: 工程化 (R121 续)
**状态**: R121r-1 完成, 5 任务形状清楚, 3 任务已由 V2-续 / B 实际完成, R121r 重点在 verify + 增量

---

## §0. TL;DR — 5 任务实际状态

读了 4 份关键 final (B / V2-mini / V2-续 / V2mini decision) + 6 个关键 src (hand.rs / cache.rs / retry.rs / protocol_handlers.rs / server.rs / redis_backend.rs) + workspace Cargo.toml + 24 LOCKED 索引 + baseline 跑 1 次 (1 failed `apeireth_supervision_harness_2026_08_06`) + 跑 1 次 (0 failed) — 发现:

| 任务 | V2-续 / B 已做 | R121r 需做 |
|---|---|---|
| 1: 修 1 failed | V2-续 加 tui lib.rs + 9 organ 0 触碰 | 加 `serial_test = "3"` + `#[serial]` 标签给 race-prone test (0 改 hand.rs logic) |
| 2: 流式 SSE cache 边界 | V2-续 已写 `stream_forward` (4 协议统一, server.rs 4 handler 已调), `dispatch_cached_with_status` 已 `if input.stream` bypass | R121r 增量: 加 5+ unit test 覆盖 4 协议流式 bypass + `dispatch_streaming_cached` 显式别名 (API 清晰) |
| 3: Redis cache backend stub | V2-续 已加 `redis = "0.27"` + `redis_backend::RedisCache` (真接 lazy connect) + `build_cache_redis()` + 5 test (含 2 #[ignore] 真连) | R121r 增量: 加 5+ unit test 覆盖 Redis stub (mock + trait contract) + 1 真接 example |
| 4: cache eviction + retry jitter | B 已写 `JitterMode 4 档` + `jittered_sleep` + 8 test (BackoffPolicy) + 10 test (should_retry) + 5 test (RetryStats) + 5 test (integration) | R121r 增量: 接入 `JitterMode` 到 `dispatch_with_retry` + 加 5+ test + 5 cache policy eviction test |
| 5: 新方向 (自选) | (没做) | **R121r 决策: 选 (a) dependabot PR auto-merge workflow** — 工程化战区延伸, 0 改任何 src, 0 跟 4 任务冲突 |

---

## §1. 任务 1: 修 cargo test --workspace 1 failed

### 1.1 实际 failed 测试 (R121r baseline 跑 1)

跑 `cargo test --workspace` 一次, 1 failed:
- `apeireth_tools::apeireth_supervision_harness_2026_08_06::test_100_rounds_minimax_stress` (单跑时 14/14 pass, 含 100 rounds stress 233s)

**这跟 spec 描述的 "hand.rs:332 record_tool_success_increments_today_and_ok" 不同** — spec 描述的 test 单跑稳过 (R121r 验证 1/1 pass), 但 100 rounds stress 跑 workspace 偶发挂 (1/2 fail).

### 1.2 修法选择 — 方案 1 (推荐, per spec)

**方案 1 (R121r 选)**: 加 `serial_test = "3"` 到 `crates/apeireth-tui/Cargo.toml` `[dev-dependencies]` + 给 `nav_settings_test.rs` race test 加 `#[serial]` 标签.

**理由**:
- spec 明确推荐方案 1
- 0 改 hand.rs (9 器官 logic 严守)
- 0 改 workspace Cargo.toml (workspace.version 严守)
- 仅 +1 dev-dep, 0 workspace dep 改

**风险**:
- 实际 race 可能在 `apeireth_supervision_harness_2026_08_06` (100 rounds stress 偶发), `serial_test` 不能跨 test binary 序列化 (不同 process)
- 如果方案 1 跑 3x workspace 仍 fail, R121r 退到方案 3 (Mutex<()>) 或诚实标 "race 偶发不可复现, 标 `[ignore]`"

### 1.3 验收

- ✅ `cargo test --workspace` 0 failed (deterministically 3 次重跑)
- ✅ 0 改 workspace.version (1.1.0)
- ✅ 0 触碰 9 器官 logic (hand.rs 只读不改, 0 加 #[serial] 到 mod tests)
- ✅ 0 改其他 11 agent 公共 API 签名

---

## §2. 任务 2: 流式 SSE cache 边界 (B 留 #1)

### 2.1 V2-续 已做 (现状)

- `crates/apeireth-api/src/protocol_handlers.rs:1215-1265` 新 `stream_forward(pipeline, kind, raw_body, model)` — 4 协议统一流式 SSE 字节转发, 0 走 cache
- `crates/apeireth-api/src/protocol_handlers.rs:844-873` `dispatch_cached_with_status` — `if input.stream { return dispatch_inner }` 守门
- `crates/apeireth-api/src/server.rs:230-322` 4 handler (`chat_completions` / `responses` / `messages` / `gemini_generate`) — 都 `if req.stream` 早返回, 调 `stream_forward`
- `mod stream_forward_tests` (line 1643+) — 已 8+ test 覆盖 4 协议 endpoint URL + Gemini stream serde

### 2.2 R121r 增量

按 spec "4 协议 (OpenAI Chat / OpenAI Responses / Anthropic / Gemini) 流式走 `dispatch_streaming_cached` 分支 (新增), 0 cache, 走 KeyPathSpan trace":

1. **新增 `dispatch_streaming_cached`** (protocol_handlers.rs): 显式别名, 文档化流式入口, 调 `stream_forward` + `KeyPathSpan` wrap (R120 B4 留的, 0 重写)
2. **5+ unit test 覆盖 4 协议流式** (在 `mod stream_forward_tests` 加):
   - `streaming_cached_skips_cache_for_4_protocols` — 验证 cache.miss 计数 0 增加 (流式 bypass)
   - `streaming_cached_openai_chat_url_correct` — endpoint URL 1:1
   - `streaming_cached_anthropic_quirk_url_correct` — minimaxi Anthropic 端点
   - `streaming_cached_gemini_model_substituted` — `{model}` 占位符
   - `streaming_cached_key_path_span_writes_to_trace` — KeyPathSpan wrap 调用
3. **0 改 4 handler 签名** (server.rs 0 触碰) — 仅在 protocol_handlers.rs 加新 fn

### 2.3 验收

- ✅ 5+ unit test 覆盖 4 协议流式
- ✅ 0 改 server.rs (4 handler 0 触碰)
- ✅ 0 触碰 24 LOCKED
- ✅ 0 改 workspace.version

---

## §3. 任务 3: Redis cache backend stub (B 留 #2)

### 3.1 V2-续 已做 (现状)

- `crates/apeireth-cache/Cargo.toml:44` 加 `redis = { version = "0.27", features = ["tokio-comp", "aio"] }`
- `crates/apeireth-cache/src/redis_backend.rs` — `RedisCache` 真接 (lazy connect, K=String, V=Vec<u8>, 5 Cache trait 方法)
- `crates/apeireth-cache/src/lib.rs:491-496` `build_cache_redis()` 显式构造
- `crates/apeireth-cache/src/lib.rs:474-480` `build_cache<K, V>` 改 Redis 分支返 Err (cross-K, V cast issue, 引导用 `build_cache_redis()`)

### 3.2 R121r 增量

按 spec "加 Redis backend stub (用 redis = "0.27", 但只写 trait + 1 mock impl + 1 真接 example, 0 真接真 Redis 端点)":

1. **5+ unit test 覆盖 Redis stub** (在 `redis_backend.rs::tests` 加):
   - `redis_cache_construction_lazy_connect_succeeds` — `new(url)` 0 立即连, 仅 Client::open
   - `redis_cache_url_accessor_returns_input` — url 字段
   - `redis_cache_key_must_be_string` — K = String 编译期约束 (0 接受任意 K)
   - `redis_cache_zero_ttl_returns_invalid_ttl` — TTL = 0 边界
   - `redis_cache_value_must_be_convertible_to_bytes` — V: Into<Vec<u8>> + From<Vec<u8>>
2. **1 真接 example** (新建 `examples/redis_cache_demo.rs`): 演示如何用 `build_cache_redis()` 跟真 Redis 端点, 含 `#[ignore]` 或 `tokio::test` + APEIRETH_REDIS_URL env var
3. **0 改 MemoryCache / LRU / TTL** (其他 4 backend 0 触碰)

### 3.3 验收

- ✅ 5+ unit test 覆盖 Redis stub
- ✅ 1 真接 example 文件
- ✅ 0 改 MemoryCache / LRU / TTL
- ✅ 0 触碰 24 LOCKED

---

## §4. 任务 4: cache eviction + retry jitter (B 留 #4 + #5)

### 4.1 V2-续 已做 (现状)

- `crates/apeireth-api/src/retry.rs:108-225` 完整 `JitterMode 4 档` + `jittered_sleep()` + `fastrand_u64()` (0 引新 dep)
- `crates/apeireth-api/src/retry.rs:153-197` `jittered_sleep(base, mode, prev, cap)` — 4 模式实现
- 已 28 test (8 BackoffPolicy + 10 should_retry + 5 RetryStats + 5 integration)

### 4.2 R121r 增量

按 spec "cache 5 policy eviction loop + retry jitter (AWS SDK pattern, ±25% jitter)":

**Retry jitter 接入 (主)**:
1. **加 `BackoffPolicy::with_jitter(mode)` 构造器** (retry.rs): 0 改 enum 本身, 加 1 method
2. **加 `BackoffPolicy.jitter: Option<JitterMode>` 字段** — 0 改 enum 公共 API (向后兼容: 加 Default::default() = None)
3. **改 `dispatch_with_retry` (protocol_handlers.rs:889-935)**: 退避循环用 `jittered_sleep(wait, policy.jitter, prev, cap)` 替代 `tokio::time::sleep(wait)` — 1:1 替换, 0 行为漂移 (None 模式返 base)
4. **5+ unit test**:
   - `jittered_sleep_none_returns_base_exactly` (3 mode = base)
   - `jittered_sleep_full_in_range_0_to_base` (100 sample 全在 [0, base])
   - `jittered_sleep_equal_in_range_base_half_to_base` (100 sample 全在 [base/2, base])
   - `jittered_sleep_decorrelated_in_range_base_to_prev_3` (100 sample)
   - `dispatch_with_retry_with_jitter_full_uses_random_sleep` (mock + 3 retry 全 0<wait<base)

**Cache 5 policy eviction (次, R121r 简化)**:
- 现状: `apeireth-cache::MemoryCache` 5 policy 已 hardcode (Lru/Lfu/Fifo/Arc/TinyLfu), 0 真接 eviction loop (仍返 CapacityExceeded)
- R121r 增量: 加 `MemoryCache::evict_one()` + 5+ test 验证 5 policy 各 evict 1 个 item

### 4.3 验收

- ✅ 5+ unit test 覆盖 jitter (retry 端)
- ✅ 5+ unit test 覆盖 5 policy eviction (cache 端)
- ✅ 0 改 BackoffPolicy 公共 API 签名 (向后兼容)
- ✅ 0 改 dispatch_with_retry 签名
- ✅ 0 触碰 24 LOCKED

---

## §5. 任务 5: 选 1 个新方向 — 决策: dependabot PR auto-merge workflow

### 5.1 决策

**R121r 选 (a) dependabot PR auto-merge workflow** (D-1 留的 R26 TODO, per spec).

### 5.2 理由

- 0 触碰 src (纯 `.github/workflows/` 工程化)
- 0 跟 4 任务冲突 (0 改 cache / retry / protocol_handlers / server / tui lib)
- 主人 0 范围扩散严守 (工程化战区延伸 OK)
- 现实价值: 92+ crate workspace 依赖每周会触发几个 dependabot PR, auto-merge 节省 Mavis / 主人人工
- 风险低: 1 个新 yml 文件, 0 触碰现有 workflow

### 5.3 实施

- 新建 `.github/workflows/dependabot-auto-merge.yml`:
  - 监听 `pull_request` event from `dependabot[bot]`
  - 仅在 PR title 含 "deps" / "bump" 时 auto-merge
  - 用 `gh pr merge --auto --squash` (cargo 0 跑)
  - `cargo check --workspace` 跑 OK 才 auto-merge (守门 0 漂移 1.1)
- 5+ yml syntax 验证 (gh actionlint) — 0 假装 "已 merge", 仅 yml 0 错误
- 0 commit, 0 触碰 main

### 5.4 验收

- ✅ 1 个新 yml 文件, gh actionlint 0 错
- ✅ 0 触碰任何 .rs / Cargo.toml
- ✅ 0 触碰 24 LOCKED
- ✅ 0 触碰 9 器官 logic
- ✅ 0 主动 commit

---

## §6. 衔接 11 agent 0 冲突核验

| Agent | 公共 API | R121r 触碰? | 0 冲突? |
|---|---|---|---|
| A (vector + memory) | `SqliteVecBackend`, `SemanticIndex`, `UserProfile`, `EmbedFn` | 0 | ✅ |
| A-2 (.github) | 3 ISSUE_TEMPLATE + 1 PR template | 0 (新 yml 不冲突) | ✅ |
| A-3 (PersistentSemanticIndex) | `PersistentSemanticIndex`, `open_persistent_semantic_index` | 0 | ✅ |
| B (cache + retry + routing) | `ResponseCache`, `BackoffPolicy`, `RetryStats`, `KeyPathSpan::start`, `parse_protocol_kind` | 0 改 (B 已写 BackoffPolicy, R121r 加 `BackoffPolicy::with_jitter` 构造器 + 加 `jitter` 字段向后兼容) | ✅ |
| B-2 (bench) | `swe_bench` / `agent_bench` / `self_disable_bench` / `latency_bench` | 0 | ✅ |
| C (9 product crate tests) | 9 tests files | 0 (C 1 failed 已 V2-续 修) | ✅ |
| D-1 (CI workflow) | `rustfmt.yml` + `rust.yml` + `rust-ci.yml` 注释 | 0 (R121r 加新 yml, 不改现有) | ✅ |
| D-2 (tool-registry classifier) | `Classifier` trait, 9 Category, 3 impl | 0 | ✅ |
| D-3 (council 4 模式) | `CollaborationMode`, 4 模式 + `RoleConstitution` + `TraceReport` | 0 | ✅ |
| V2-续 / V2-mini (workspace_e2e + tui lib + W3C) | 5 任务 V2-续 已做完 | 0 改 V2-续 改的 5 文件 | ✅ |
| Mavis (修 1 compile error) | apeireth-cli AppState.response_cache | 0 | ✅ |

---

## §7. 阶段总览 (2h45m 阶段, 实际从 10:15 算)

| 阶段 | 时间 | 任务 | 状态 |
|---|---|---|---|
| R121r-1 | 10:15-10:45 (0.5h) | readmap (本文件) | ✅ 10:45 |
| R121r-2 | 10:45-11:15 (0.5h) | 任务 1 (修 1 failed) | 待 |
| R121r-3 | 11:15-11:45 (0.5h) | 任务 2 (流式 SSE) | 待 |
| R121r-4 | 11:45-12:15 (0.5h) | 任务 3 (Redis stub) | 待 |
| R121r-5 | 12:15-12:45 (0.5h) | 任务 4 (eviction + jitter) | 待 |
| R121r-6 | 12:45-12:55 (0.17h) | 任务 5 (新方向 yml) | 待 |
| R121r-7 | 12:55-13:00 (0.08h) | verify + final report + decision log | 待 (主人 13:00 验收) |

**R121r-1 完. R121r-2 立即开干.**
