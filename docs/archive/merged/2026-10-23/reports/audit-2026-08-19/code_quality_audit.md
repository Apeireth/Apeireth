# Apeireth-rust v1.0.0 Code Quality Audit

**审计员**: code_quality_auditor sub-agent
**日期**: 2026-08-19
**项目**: C:\Users\31683\Apeireth-rust (v1.0.0, tag 993e9107, HEAD 9bf36b1e)
**活跃 crate 数**: 84 (实测, baseline 说 85; `release-tools/` 是工具子目录, 不算 active crate; `_archived/` / `_frozen/` 不计入)

---

## 🔴 高优先级修复候选 (人审后动手)

| # | 严重度 | 类别 | 位置 | 摘要 | 修复建议 |
|---|---|---|---|---|---|
| 1 | **P0 真编译错误** | Part A | `crates/apeireth-tui/src/organ/.r125-12-13-keys-stub.rs:330` | `const _ = ();` 缺类型 annotation, rustc E0762, 文件以 `.` 开头但被 rustfmt `--check` 扫到, CI 会挂 | 改 `const _: () = ();` (一行修复), 或 `.gitignore` 整个文件后从 rustfmt 范围剔除 |
| 2 | **P1 fmt drift** | Part A | `crates/apeireth-api/src/protocol_handlers_v2.rs` (~50 行 drift) | rustfmt 反复拆行 `registry.register(...)` 调用 (一行能容下时被拆成 4 行) | 跑 `cargo fmt -p apeireth-api` 自动修复, 然后 verify no semantic change |
| 3 | **P1 fmt drift** | Part A | `crates/apeireth-core/src/eight_anchors.rs` (~12 行 drift) | 数组元素合并/拆分不一致 (R126 哲学锚 hardcode) — 改 LOCKED enum 风险**中等**, 但只是格式 | 同 #2; 注意 `ALL_EIGHT_ANCHOR_CODES` 在编译期 hardcode 中, 跑 fmt 后跑测试 verify value 不变 |
| 4 | **P1 fmt drift** | Part A | `crates/apeireth-cron/src/lib.rs:70-91` | `MONTH_ALIASES` / `DOW_ALIASES` 紧凑一行被 rustfmt 拆成多行 | `cargo fmt -p apeireth-cron` |
| 5 | **P1 fmt drift** | Part A | `crates/apeireth-supervisor/src/journal_entry.rs` (~10 处) | 多行合并/拆分 (supervisor LOCKED 边界内) | `cargo fmt -p apeireth-supervisor` + verify journal API tests 不变 |
| 6 | **P1 fmt drift** | Part A | `crates/apeireth-tui/tests/pending_companion_wip/test_main_chain_e2e_full.rs` (integration-e2e crate) | 测试文件 drift | `cargo fmt -p apeireth-integration-e2e` |
| 7 | **P2 flaky 风险** | Part D | `crates/apeireth-rate-limiter/src/fixed_window.rs:130 boundary_spike_demonstrates_known_issue` | 50ms 窗口 + 60ms sleep, 与已修 flaky 同模式 (boundary 紧贴) | 改 200ms 窗口 + 250ms sleep, 沿用 `f69154e5` 同款语义断言 |
| 8 | **P2 flaky 风险** | Part D | `crates/apeireth-rate-limiter/src/sliding_window.rs:227 log_mode_drops_expired_entries` | 50ms 窗口 + 60ms sleep, 同上模式 | 同上 |
| 9 | **P2 flaky 风险** | Part D | `crates/apeireth-rate-limiter/src/leaky_bucket.rs:197 drips_over_time_makes_room` | 30ms sleep, refill 速度边界紧 | 改 100ms sleep + 语义断言 (avail > 0) |
| 10 | **P3 clippy 噪声** | Part B | 全仓 409 个 `allow_attributes_without_reason` | 集中在 `apeireth-tui` (34), `apeireth-memory` (31), `apeireth-api` (31) | 配合 #2 一起加 `reason = "..."` 后缀; 或 workspace-level 降到 `allow` |

---

## Part A — `cargo fmt --check`

### A.1 跑法

| 命令 | 结果 |
|---|---|
| `cargo fmt --all -- --check` (顶层) | **失败**: `os error 206` ("文件名或扩展名太长") — rustfmt 1.9-stable 在 Windows 上把所有 .rs 文件拼成单命令行, 84 crate × 1702 .rs 文件累计超 64K, 触发 Windows API 限制 |
| `cargo fmt --package <crate> -- --check` (单 crate) | ✅ 跑通, 但不在 sub-agent scope 内全跑 |
| **fallback**: `rustfmt --edition 2021 --check <files...>` per crate | ✅ 跑通, 跑完所有 84 crate |

### A.2 Drift 分布 (6/84 crate = 7.1%)

```
crate                          files    exit    stderrlines    漂移类型
apeireth-api                       75      1          210       拆行 (registry.register)
apeireth-core                      17      1          236       数组元素合并/拆分 (philosophy anchors)
apeireth-cron                       4      1          238       MONTH_ALIASES / DOW_ALIASES 拆行
apeireth-integration-e2e           14      1          253       test_main_chain_e2e_full.rs drift
apeireth-supervisor                14      1          201       journal_entry.rs + filter_child + append
apeireth-tui                       83      1           10       **真编译错误** (见 #1)
```

### A.3 关键发现 — 1 个 P0 真编译错误

**`crates/apeireth-tui/src/organ/.r125-12-13-keys-stub.rs:330`**

```rust
const _ = ();  // 占位, 0 装准备 0 编译     ← E0762: missing type for const item
```

rustc 1.97-stable **不允许** `const _ = ();`, 必须 `const _: () = ();`。

但是! 这个文件以 `.` 开头 (`.r125-12-13-keys-stub.rs`), rustc **不会编译**它 (rust module 系统忽略 dotfile), 所以 `cargo build` / `cargo check` 通过。但 `rustfmt --check` 会扫所有 .rs 文件 = CI `cargo fmt --check` 会挂。

### A.4 部分 crate 的 drift 都是"风格不符" — 无功能影响

例 apeireth-api/protocol_handlers_v2.rs:312:
```rust
- registry.register(ProtocolKind::OpenAiChat, OpenAiChatHandler::new(Arc::clone(&pipeline)));
  registry.register(
+     ProtocolKind::OpenAiChat,
+     OpenAiChatHandler::new(Arc::clone(&pipeline)),
+ );
```
仅一行拆成 4 行。无功能差异。

### A.5 **未跑**项: 无 — 所有 84 crate per-crate rustfmt 全部跑通

---

## Part B — `cargo clippy --workspace --all-targets`

### B.1 跑法

```
$ cargo clippy --workspace --all-targets --message-format=short 2>&1 | Out-File clippy.log
ExitCode: 0 (成功 — workspace 无 error)
耗时: 37.43s (release dev profile, 编译所有 crate + tests + examples)
总日志: 876 行
```

### B.2 警告总数与分类

| Lint | 数量 | 严重度 |
|---|---|---|
| **`allow_attributes_without_reason`** | **409** | 低 (噪音, workspace lint 自己 allow 即可) |
| `unused_import` / `unused_imports` | 3 | 低 (死代码) |
| `dead_code` (`make_persist_config` never used) | 1 | 中 (可能遗留 API, 需确认是否要删) |
| `cast_lossless` (`u16 → i32` / `u8 → i32`) | 2 | 低 (风格, 1 行 fix) |
| 未来 Rust 不兼容 (`nom v1.2.4`, `proc-macro-error2 v2.0.1`) | 2 transitive | **P3** (升级) |
| **合计 unique warning** | **415** | |

### B.3 按 crate 聚类 (前 15)

| Crate | Warnings | 主要文件 |
|---|---|---|
| `apeireth-tui` | **34** | `backend.rs` (4), `app.rs` (2) — 大量 `#[allow(...)]` 无 reason |
| `apeireth-memory` | **31** | 多模块 |
| `apeireth-api` | **31** | `llm/router.rs` (3), `llm/providers/anthropic_compat.rs` (3) |
| `apeireth-council` | **17** | `multi_model_backend.rs` (2) |
| `apeireth-tool-codesearch` | **15** | `pure_pattern.rs` (2) |
| `apeireth-tool-fetch` | **15** | rate_limit.rs (在别处) |
| `apeireth-provider` | **13** | `opencode.rs` (2) |
| `apeireth-tool-shell` | **10** | `sandbox.rs` (2) |
| `apeireth-graph-primitive` | **9** | |
| `apeireth-tool-image-gen` | **9** | |
| `apeireth-tool-image-process` | **9** | |
| `apeireth-constraint` | **9** | `lib.rs` (5) |
| `apeireth-telemetry` | **8** | |
| `apeireth-tool-browser` | **8** | |
| `apeireth-companion` | **8** | |

绝大多数 (98.6%) 是同一个 lint — `allow_attributes_without_reason`, 这是项目锁级策略 lint 自己在 workspace.lints.clippy 里启用了, 但 409 个 `#[allow(...)]` 没填 reason 字段。

### B.4 真正需要关注的警告

1. **`apeireth-memory/src/lib.rs`**: 1 个 unused import (`std::time::Duration`) — 死代码
2. **`apeireth-api/src/llm/providers/`**: 1 个 unused import (`ProviderRegistry`), 1 个 unused (`ProviderConfig` + `ProviderScope`)
3. **`apeireth-memory/extensions`**: 1 个 `make_persist_config` never used — 待确认是否要保留
4. **2 个 cast_lossless** (`u16→i32`, `u8→i32`): 风格, 一行 fix

### B.5 状态: ✅ 跑完

---

## Part C — 测试覆盖率粗扫

### C.1 方法

对 84 active crate, 每个跑:
1. `Test-Path crates/<name>/tests` — tests/ 目录存在
2. `Get-ChildItem -Recurse crates/<name>/tests/*.rs | Measure-Object` — 测试文件数
3. `Select-String -Path crates/<name>/src/lib.rs -Pattern '#\[test\]' -AllMatches` — inline test 数

数据存于 `_research_mem/sub_agent_reports/2026-08-19/test_coverage.csv`

### C.2 全局摘要

| 桶 | Crate 数 |
|---|---|
| 总 active | 84 |
| **0 测试** (无 tests/ 目录 + 0 inline test) | **10** |
| **低测试** (< 3 tests, 但 > 0) | **13** |
| 健康 (≥ 3 tests) | 61 |

测试覆盖率: **72.6%** crate 健康; **27.4%** (23 crate) 测试不足。

### C.3 0 测试的 10 个 crate (P2 测试候选)

| Crate | 公开 API 概览 (从 lib.rs) | 测试 stub 思路 |
|---|---|---|
| **apeireth-context-fold** | `fold`, `unfold`, `FoldStrategy` (4 variant), `FoldResult`, `FoldMarker` (5 marker), `TokenAccumulator::new/feed/snapshot`, `cosine`, `fold_segments`, `unfold_semantic`, `Embedder` trait | (高价值) **3 个 stub**: ①`FoldStrategy::Truncate` 对超长字符串截断 — 验证 chars 边界不切 unicode; ②`FoldMarker::round_trip_lossless` — `fold(text)` → `unfold(fold(text)) == text` 字节级守恒; ③`TokenAccumulator` 跨多 `feed()` 累加 ≈ tiktoken chars/4 校验 |
| **apeireth-host** | `keyring` (SecureKeyring, 3 后端), `machine_id::derive_id/detect/get_machine_id/hash_machine_id/MachineId/MachineIdError` | (低价值, OS-dependent) **2 个 stub**: ①`hash_machine_id(id)` 确定性 (同 input → 同 hash); ②`MachineId::from_hex` 接受合法 hex, 拒绝非 hex。**不要**测真实 keyring (CI 无 Windows credential store 干扰) |
| **apeireth-llm-iface** | `LlmError` (6 variant), `LlmProvider` trait, `ChatMessage::user/system/assistant`, `LlmRequest::new`, `LlmResponse::ok/error`, `TokenUsage::total_tokens`, `ProviderCapabilities`, `ProviderHealth`, `ProviderMetadata`, `is_valid_provider` | (高价值, 抽象 trait) **3 个 stub**: ①`LlmRequest::new("m", vec![user("hi")])` 字段正确性; ②`LlmResponse::error("oops", "anthropic").is_success() == false` + `total_tokens() == 0`; ③`LlmError::is_retryable()` (按 variant 分流) — 重试路由决定 |
| **apeireth-repo-tools** | `scan::Scanner`, `analyzer::Complexity`, `analyzer::SecurityAudit`, `register::ToolRegistry::register` | (中价值) **2 个 stub**: ①`Scanner::scan_file(tempfile)` 在 UTF-8 + 非 UTF-8 上不 panic; ②`ToolRegistry::register` 注册 2 个同名 tool 时返回 `Err` 不静默成功 |
| **apeireth-tool-browser** | 工具壳 — 未细读 (10 文件, 0 测试) | (低价值, 浏览器自动化需要 mock) — skip 推荐, 等真接 browser driver 后补 |
| **apeireth-tool-codesearch** | grep-style 工具 — `search_files`/`pure_pattern`/LRU cache | (中价值) **2 个 stub**: ①`pure_pattern::compile("foo.*bar")` 拒绝非法 regex 返回 `Err`; ②LRU cache: `insert(10) → get(missing)` 返回 `None` |
| **apeireth-tool-fetch** | Tier 1.5 unified fetch engine (R149, absorb VCP 7 plugins) | (中价值, HTTP) — 跳过 (需 wiremock, 与 #2 同型 — 测试已加 `test_fetch_real_wiremock`) |
| **apeireth-tool-filesystem** | 文件系统工具壳 | (低价值, fs-err wrapper) — 跳过, 集成测试覆盖 |
| **apeireth-tool-image-gen** | 图片生成工具壳 (生成网络调用) | (低价值) — 跳过, 需 mock image provider |
| **apeireth-tool-image-process** | 图片处理工具壳 | (低价值) — 跳过 |

### C.4 低测试 (1-2 tests) 的 13 个 crate (P3 候选)

| Crate | Tests | 公开 API | 价值评估 + 推荐 |
|---|---|---|---|
| apeireth-evolution | 1 | `EvolutionError`, `current_time_ms`, `EvolutionStateMachine` (6 状态), `FailPolicy` (6 失败路径), `EvolutionEngine`, `PodaCycle` (4 阶段) | **高价值** — 6 状态机 + fail-6 是 LOCKED 边界逻辑, 应有边界测试; **2 stub**: ①`EvolutionStateMachine::transition(Idle→Proposed→Ratified→Active)` happy path; ②非法 transition (Ratified→Idle) 返回 `IllegalTransition` 错误 |
| apeireth-experience | 1 (但 inline 测了 wiki_kg_association_integration) | `WikiEntry`, `KnowledgeGraph`, `AssociationNetwork`, 4 个 `*_to_context_block` | 已有 1 个集成测; **skip**, 加更多会超 LOCKED 边界 |
| apeireth-http-client | 1 (wiremock 测试) | `HttpClient::get/post` | (中价值) — 已有 wiremock 测试, 增 boundary (timeout / 4xx / 5xx) |
| apeireth-pipeline-g5 | 2 | `Pipeline<I,O>`, 5 阶段 (Dispatch/Normalize/Policy/Reliability/Throttle), `CircuitBreaker` | **高价值** — 通用 5 阶段 pipeline 框架是 R20 阶段 6 估补; **2 stub**: ①`CircuitBreaker::new(2)` 第 3 次失败触发 open; ②`Pipeline::execute(normal_input)` 5 阶段顺序执行 |
| apeireth-provider | 1 (但 6 provider 入口编译期 assert 已 inline) | 6 provider (`claude_code/codex/copilot/gemini_cli/opencode/minimax`), `ALL_PROVIDERS` | 已有 1 编译期测试; **低价值** — provider 实现是 stub, 真实 SDK 在 R21+ 真接, 等那时候补 |
| apeireth-rate-limiter | 1 (但模块内部 token_bucket/leaky_bucket/sliding_window/fixed_window 各有 4-6 test, inline 总数远超 30) | `token_bucket_in_memory`/`leaky_bucket_in_memory`/`fixed_window_in_memory`/`sliding_window_in_memory` 4 个便捷构造器 | 实际是 false negative — inline 测多, **skip** |
| apeireth-state | 1 | `OnceLockState`/`MutexState`/`RwLockState` 3 模式 + `OrganStateRegistry` 9 器官 | **高价值** — 借鉴 Golutra #6 1:1 翻译; **2 stub**: ①`MutexState::with_lock` 在 RAII 释放后状态可重读; ②`OnceLockState::init_or_get` 多次调用返回同一 Arc 实例 |
| apeireth-supervisor | 2 | `Journal::new/append/filter_child`, `HostCallResult`, `Heartbeat` | 已有 2 测试, **skip** |
| apeireth-team-lead | 2 | team lead orchestration | 已有 2 测试, **skip** |
| apeireth-telemetry | 2 (inline) | `log_replay`, 6 维 trace | 已有 2 inline, **skip** |
| apeireth-tool-shell | 1 (r264_sandbox_e2e) | sandbox shell 工具 | 已有 1 e2e, **skip** |
| apeireth-vector | 1 (但 tests/store.rs 已含 sqlite-vec + semantic test) | sqlite-vec 0.1.x 集成 | false negative — inline 多; **skip** |
| apeireth-web | 2 | web HTTP server (templating) | 已有 2 tests, **skip** |

### C.5 高价值优先测试候选 (Top 5)

按"公开 API 价值 × 测试缺口"排序, 推荐人审后优先补这 5 个:

1. **`apeireth-context-fold`** — `fold/unfold` 字节级守恒 + TokenAccumulator 累加 (R144 借鉴 VCP ContextFoldingV2)
2. **`apeireth-pipeline-g5`** — `CircuitBreaker::open` 触发 + `Pipeline::execute` 5 阶段 (R20 阶段 6 通用 pipeline)
3. **`apeireth-evolution`** — 6 状态机 happy path + 非法 transition (R125-7 PODA cycle)
4. **`apeireth-state`** — 3 模式 RAII 守恒 (R21 借鉴 Golutra #6)
5. **`apeireth-llm-iface`** — `LlmError::is_retryable()` 分流逻辑 (R179 P0-3 抽象接口)

---

## Part D — CI Flaky Test 风险静态评估

### D.1 最近 flaky 相关 commit

`git log --oneline --all | grep -iE 'flaky|flake|macOS|race|intermittent|timing'`

```
f69154e5 fix(rate-limiter): available_tokens_reflects_refill 改语义断言 (macOS flaky)
4406207f fix(pipeline): retry_suppression 边界测试加大时序余量 (macOS flaky)
8ffb6b06 ci: benchmark-tracking only bench criterion crates; miri step timeout 20min
```

(无更早的 flaky fix; 最近 200 commit 只这 2 个, 都在 2026-08-18 同日)

### D.2 已修 flaky 的 diff 规律 (核心 pattern)

两个 fix 都同形:

**Fix #1** (4406207f, retry_suppression.rs):
```diff
- let s = RetrySuppression::new(Duration::from_millis(200));   // 原 200ms 窗口
- std::thread::sleep(Duration::from_millis(150));              // 紧贴 150ms
- std::thread::sleep(Duration::from_millis(80));               // 总 230ms > 200ms
+ let s = RetrySuppression::new(Duration::from_millis(500));   // 加大到 500ms
+ std::thread::sleep(Duration::from_millis(300));              // 余量 200ms
+ std::thread::sleep(Duration::from_millis(300));              // 总 600ms > 500ms
```

**Fix #2** (f69154e5, token_bucket.rs):
```diff
- std::thread::sleep(Duration::from_millis(20));
- assert!(avail >= 60.0 && avail <= 80.0);  // 紧区间
+ std::thread::sleep(Duration::from_millis(50));
+ assert!(avail > 50.0, "refill 应生效");    // 语义断言 (大于)
+ assert!(avail <= 100.0, "不应超容量");     // 语义断言 (小于等于)
```

**Pattern**:
1. **sleep 紧贴边界** (< 50ms 余量) → macOS nextest 高并发调度延迟让 sleep 漂移
2. **精确数值断言** (`[60, 80]` 紧区间) → 改**语义断言** (`> 50` / `<= 100`)
3. 都是 token bucket / retry suppression 这类 timing 算法的**窗口边界测试**

### D.3 全仓 timing-dependent 测试扫描

`grep 'thread::sleep|std::thread::sleep|tokio::time::sleep'` 在 `crates/` 找到 **160** 处 sleep 调用 (含 examples / benches / integration test)。其中**最可能 flaky**的 (按"边界紧度 + 时序敏感度"评估):

### D.4 Top 5 高风险 flaky 候选

| # | 文件:测试名 | 时序参数 | 风险评估 |
|---|---|---|---|
| 1 | `crates/apeireth-rate-limiter/src/fixed_window.rs:130 boundary_spike_demonstrates_known_issue` | 50ms 窗口 + 60ms sleep | **极高** — 与已修 flaky 同模式 (50ms+60ms = 10ms 余量, macOS 下必挂) |
| 2 | `crates/apeireth-rate-limiter/src/sliding_window.rs:215 log_mode_drops_expired_entries` | 50ms 窗口 + 60ms sleep | **极高** — 同上 (固定窗口 10ms 余量) |
| 3 | `crates/apeireth-rate-limiter/src/leaky_bucket.rs:191 drips_over_time_makes_room` | 30ms sleep, refill 期望值精确 (3 leaked) | **高** — 注释说 "30ms * 100/s = 3 leaked", 但 refill 不是原子, macOS 漂移可能给 2 或 4 |
| 4 | `crates/apeireth-rate-limiter/src/storage.rs:338 + 357` (in test) | 20ms / 10ms sleep + 容量断言 | **中** — 20ms 余量在 macOS 高并发下风险 |
| 5 | `crates/apeireth-runtime/tests/r267_minimax_live.rs:120` (live test, marked #[ignore]?) | 300ms sleep | **中** — live test, 通常 `#[ignore]`, 但跑 CI 时会挂 |

### D.5 其他次要候选

| 文件:行 | 时序 | 备注 |
|---|---|---|
| `crates/apeireth-tool-codesearch/src/lru_cache.rs:355` | 20ms | LRU expiry 边界 |
| `crates/apeireth-tool-codesearch/src/cache.rs:290` | 70ms | TTL 测试 |
| `crates/apeireth-tool-fetch/src/rate_limit.rs:184` | 60ms | 同 rate-limiter 类型 |
| `crates/apeireth-api/src/auth.rs:851` | 50ms | Auth timeout 边界 |
| `crates/apeireth-supervisor/src/heartbeat.rs:559 + 576` | 50ms + 180ms | Heartbeat 心跳检测 |
| `crates/apeireth-tui/tests/cognition_live.rs:152` | 80ms | TUI live test |
| `crates/apeireth-tool-shell/tests/r264_sandbox_e2e.rs:66` | 200ms | Sandbox async wait |

### D.6 环境依赖类 (env-dependent, 非 timing)

`grep -iE 'env::var|/etc/|home_dir|hostname'` 等 — 未深扫, 但已知的 wiremock 测试在 `apeireth-voice/test_voice_real_wiremock.rs` + `apeireth-lark/test_lark_real_wiremock.rs` 都有, 这些不是 flaky (mock 锁定)。

### D.7 **最该被 watch 的 test**

**`crates/apeireth-rate-limiter/src/fixed_window.rs:130` `boundary_spike_demonstrates_known_issue`** — 评分 = **9/10**:

- ✅ 与已修 flaky 完全同形 pattern (窗口 50ms + sleep 60ms)
- ✅ 注释甚至明说 "边界突刺" (boundary spike) — 设计上承认时序脆弱
- ✅ 在 `rate-limiter` crate — 项目已知这是 macOS nextest flaky 热点 (近 2 个 fix 都集中在该 crate)
- ✅ CI 跑率最高 (rate-limiter 是 integration chain 第二环)

**推荐修复 (人审后动手)**:
```diff
- let mut fw = FixedWindow::new(Duration::from_millis(50), 2, FixedWindowReset::OnWindowEnd).unwrap();
+ let mut fw = FixedWindow::new(Duration::from_millis(500), 2, FixedWindowReset::OnWindowEnd).unwrap();
  assert!(fw.try_acquire());
  assert!(fw.try_acquire());
- std::thread::sleep(Duration::from_millis(60)); // 窗口过期
+ std::thread::sleep(Duration::from_millis(550)); // 窗口过期 (余量 50ms)
```

### D.8 静态评估局限说明

**未真跑 N 次**: 本报告为静态分析, 未实际 `cargo test --test-threads=64 --repeat=10` 跑 timing 边界测试。建议人审后, 在 macOS runner 上跑 `cargo test -p apeireth-rate-limiter -- --test-threads=32` × 10 次做实证。

---

## 附录 — 硬约束遵守情况

| 约束 | 状态 |
|---|---|
| Part A/B 只读 | ✅ rustfmt --check / cargo clippy 都不修改任何文件 |
| Part C/D 只读 + 简单命令 | ✅ Test-Path / Get-ChildItem / git log 全只读 |
| 不修改 src/ / Cargo.toml / tests/ | ✅ 未做任何修改 |
| 不 commit | ✅ 未做任何 commit |
| cargo 命令超时或跑不动就标注 | ✅ Part A cargo fmt --all 失败 (os error 206), 用 fallback per-crate 跑通 |

## 附录 — 数据文件位置

| 文件 | 内容 |
|---|---|
| `_research_mem/sub_agent_reports/2026-08-19/test_coverage.csv` | 84 crate 测试覆盖矩阵 |
| `_research_mem/sub_agent_reports/2026-08-19/clippy.log` | cargo clippy 全量输出 (876 行) |
| `C:\Users\31683\Apeireth-rust\fmt_per_crate.log` | 84 crate rustfmt per-crate 结果 |
| `C:\Users\31683\Apeireth-rust\fmt_apeireth-{api,core,cron,integration-e2e,supervisor,tui}.log` | 6 个 drift crate 的 rustfmt diff 详情 |
| `C:\Users\31683\Apeireth-rust\fix_pipeline_retry_suppression.diff` | flaky fix #1 (4406207f) 完整 diff |
| `C:\Users\31683\Apeireth-rust\fix_rate_limiter_token_bucket.diff` | flaky fix #2 (f69154e5) 完整 diff |