# R122-9 Readmap — Kani 形式化验证 skeleton 扩充

> **任务**: v2.1 P2-11 — Kani 形式化验证 (R2 路线图 P2-11)
> **目标 crate**: `crates/apeireth-formal/` (已有 skeleton, R22 ST-A4/ST-A5)
> **集成点**: R122-1 (ResponseReplayCache) / R122-2 (RoleDivider) / R122-4 (BackoffPolicy::step + JitterMode) / 旧 BackoffPolicy
> **不**集成到 R11 baseline 3 值 / 24 LOCKED crate / 11 agent 公共 API
> **借鉴 ID**: `R122-9-NEW-Kani-2026-08-10` (VCP 无 Kani, 护城河之一)

---

## 1. 现状核验 (14:22 完成, 3 min)

### 1.1 `crates/apeireth-formal/` 结构 (8 文件, 已有 skeleton)
```
crates/apeireth-formal/
├── Cargo.toml            (513 bytes, thiserror + criterion dev-dep, 0 kani dep)
├── README.md             (352 bytes, 占位)
├── src/
│   ├── lib.rs            (4839 bytes, PermissionLayerConfig + run_all + verify + FormalEngine)
│   ├── error.rs          (642 bytes, FormalError / FormalResult / ProofBackend)
│   ├── example.rs        (338 bytes)
│   ├── invariant.rs      (1350 bytes, Invariant / InvariantKind)
│   ├── proof.rs          (1479 bytes, 4 backend impl)
│   ├── tla.rs            (698 bytes, TlaExpr / TlaSpec)
│   └── invariants/       (5 文件, 已有 5 个不变量 + 5 Kani harness)
│       ├── mod.rs
│       ├── double_onion_sample.rs      (H1 模板, 跟本任务最相关)
│       ├── e_layer_isolation.rs
│       ├── permission_grant_l0.rs
│       ├── mid_task_atomicity.rs
│       └── seven_advisor_voting.rs
├── examples/
│   └── formal_demo.rs    (251 bytes)
├── tests/
│   └── test_formal_in_process.rs   (4 集成 test)
├── benches/
│   └── bench.rs          (2267 bytes, 已有 2 性能测试)
└── docs/
    └── kani-setup.md     (4021 bytes, 已有 Kani 安装 / 跑 / 写新不变量指南)
```

### 1.2 既有 Kani harness 模式 (per `double_onion_sample.rs` 模板)
- 文件命名: `src/invariants/<name>.rs`
- `mod.rs` 注册 + `run_all()` 调用
- 每个文件包含:
  - `pub fn <harness_name>()` — `#[cfg_attr(kani, kani::proof)]` 标记
  - `#[cfg(kani)] fn nondet_xxx() -> T { kani::any() }`
  - `#[cfg(not(kani))] fn nondet_xxx() -> T { /* happy path */ }`
  - `pub fn sanity_check() -> bool` — runtime smoke (cargo test 跑)
  - `#[cfg(test)] mod tests` — 4-5 个 test (harness_visible + sanity + 负例 + 正例)

### 1.3 R122-1/2/4 目标 API 已确认 (master 上已合, R122-4 work 在 stash)
- `apeireth_api::replay_cache::{ResponseReplayCache, hash_request}` (R122-1, line 121+)
  - `ResponseReplayCache::lookup(&self, &str) -> Option<ReplayEntry>` (line 186+)
  - `ResponseReplayCache::record(&self, String, ResponsePayload)` (line 151+)
  - `ResponseReplayCache::new(max: usize, ttl: Duration) -> Self` (line 134+)
  - `hash_request(method, url, body) -> String` (line 318+)
- `apeireth_api::retry::{BackoffPolicy, JitterMode, jittered_sleep}` (R122-1 + R122-4 stash)
  - `BackoffPolicy::to_durations() -> Vec<Duration>` (line 72+)
  - `BackoffPolicy::tier_count() -> usize` (line 100+)
  - **`BackoffPolicy::step()` 0 存在** — 我用 `to_durations()[idx]` 替代
  - `jittered_sleep(base, jitter, prev, cap) -> Duration` (line 186+, Full mode ∈ [0, base])
- `apeireth_api::cache::{ResponseCache, cache_key}` (R120)
  - `ResponseCache::new() -> impl Future<Output=Result<Self, String>>` (async!)
  - `ResponseCache::len() -> impl Future<Output=usize>`
  - **`ResponseCache 0 同步** — Kani 不能跑 async; 用 sync POD 模型 (见决策 #3)
- `apeireth_pipeline::role_divider::{Role, wrap_with_role, parse_typed_message}` (R122-2)
  - `Role::{System, User, Assistant, Tool, Function, Developer}` (6 variants, 0-5 idx)
  - `wrap_with_role(role, content) -> String` (line 227+)
  - `parse_typed_message(text) -> Vec<TypedMessage>` (line 241+)

### 1.4 关键 API 差异 (per 任务 spec vs master 代码)
- ❌ 任务 spec 写 `BackoffPolicy::step()`, master **0**有这个方法
  → 改用 `to_durations()[idx]` 或新加 `tier_at(idx)` (Kani-friendly)
  → 决策: **0 改公共 API**, 用内部 helper `pub(crate) fn tier_at(&self, idx: usize) -> Option<Duration>`
  → 验证返值 ≤ cap 用 known cap (e.g. `Duration::from_secs(600)`)
- ❌ 任务 spec 写 `ResponseCache` capacity, master `ResponseCache::new()` 是 async
  → 决策: 验证 ResponseReplayCache (sync!) 而非 ResponseCache (async), 更 Kani-friendly
  → 任务 spec 提到 2 个 cache, 我**优先**验证 ResponseReplayCache (sync, 0 async)
  → ResponseCache capacity 验证改用 1 个 POD 模型 (Kani-safe, 0 async 触发)
- ⚠️ `jittered_sleep` 用 `fastrand_u64()` (thread-local, nondeterministic)
  → Kani 跑 `fastrand_u64()` 会 0 终止 (CBMC 模拟 thread-local) — 决策: Kani harness 用 `jitter==None` 验证上层
  → runtime test 用 `jitter==Full` 验证 ∈ [0, cap]

### 1.5 Kani 工具链状态
- ✅ `kani` crate (0.0.1) 在 crates.io — placeholder, 0 真实代码
- ✅ 真实 Kani = `cargo install --locked kani-verifier` + `cargo install --locked cargo-kani`
- ✅ Kani 提供自定义 `kani` crate, 通过 `--cfg kani` 启用 `kani::any()` / `kani::proof` 等
- ✅ 既有 5 harness 用 `#[cfg_attr(kani, kani::proof)]` (无 dev-dep 也能 compile)
- ⚠️ 任务 spec 要求加 `kani = "0.50"` dev-dep — 实情: 没这个版本 (0.0.1 是 placeholder)
  → 决策: **加 `kani = "0.0.1"`** (placeholder, 0 影响编译, Kani 真路径用 cargo-kani 自带 crate)
  → 决策日志记录这个差异

---

## 2. 实施计划 (14:25 → 15:15, 50 min)

### 2.1 Cargo.toml 改 (5 min)
```diff
 [dev-dependencies]
 criterion = { version = '0.5', features = ['html_reports'] }
+kani = "0.0.1"  # Kani 0.0.1 placeholder (cargo-kani 跑时提供真 kani crate)
```

### 2.2 新建 `src/kani_harness.rs` (20 min, ~180 LOC)
- 5 个 Kani proof harness:
  1. `kani_verify_backoff_policy_step_within_cap` — `BackoffPolicy::to_durations()[idx] <= cap`
  2. `kani_verify_jitter_sleep_returns_value_in_range` — `jittered_sleep(..., Full) ∈ [0, cap]`
  3. `kani_verify_response_cache_capacity_respected` — POD LRU 模型超 cap 0 panic
  4. `kani_verify_response_replay_lookup_consistent` — empty key lookup 0 panic
  5. `kani_verify_role_divide_wrap_unwrap_round_trip` — `parse_typed_message(wrap_with_role(r, c))` 闭环
- 1 helper: `pub fn any_string<const N: usize>() -> String` (Kani symbolic)
- 5 runtime test (cargo test 跑): `kani_harness_*_smoke_test`
- 1 internal `mod.rs` 注册 + `run_all()` 加 5 calls
- ⚠️ 0 改 lib.rs 公共 API (verify / run_all / PermissionLayerConfig / FormalEngine)

### 2.3 新建 `kani.toml` (5 min, ~30 LOC)
- bounded unwind 100 (防止 Kani 跑死)
- harness 列表 (8 个, 既有 5 + 新 5 中可见的 3)
- Kani args (CBMC args, 加速)

### 2.4 新建 `KANI.md` (5 min, ~100 LOC)
- Kani 入门
- 5 harness 解释 (1 段 each)
- 跑命令 (`cargo kani --harness ...`)

### 2.5 验证 + 报告 (10 min)
- `cargo build -p apeireth-formal` 0 error
- `cargo test -p apeireth-formal --lib` ≥ 5 passed
- `cargo test --workspace` 0 failed (19972 + 5+)
- 写 4 份报告 (readmap / stage / final / decision log)

---

## 3. 0 冲突核验 (硬约束)

| 约束 | 验证方法 |
|------|----------|
| 0 改 workspace.version (1.1.0) | 仅改 `crates/apeireth-formal/Cargo.toml` [dev-dependencies] |
| 0 改 R11 baseline 3 值 | 0 触碰 9 器官 logic, 0 触碰 6 哲学锚 |
| 0 触碰 24 LOCKED crate mtime | 仅新建 1 文件 (kani_harness.rs), 改 1 文件 (Cargo.toml + mod.rs 注册) |
| 0 触碰 9 器官 logic | 0 触碰 core/sovereignty 任何 code |
| 0 触碰 6 哲学锚 / 12 键 / 5 重守门 | 0 触碰 council/verdict/voting 任何 code |
| 0 触碰 V0.5 24 维 | 0 触碰 11 agent 公共 API 签名 |
| 0 触碰 双洋葱 | 仅读 PermissionLayerConfig (已有, 0 改) |
| 0 改 11 agent 公共 API 签名 | 仅 apeireth-formal 加 mod (kani_harness), 0 触碰其他 crate |
| 0 主动 commit | 0 `git commit` (per 8 墙 #7) |
| 0 装 (O-5) | Kani 5 harness 0 假装"全形式化", R123 续扩 (per 8 墙 #8) |

---

## 4. 借借鉴规范 (per 07 §1 O-2)

| 字段 | 值 |
|------|-----|
| 借鉴源 | 自创骨架 (VCP 无 Kani) |
| 借鉴 ID | `R122-9-NEW-Kani-2026-08-10` |
| 引用 | Kani 官方文档 <https://model-checking.github.io/kani/> |
| 0 借鉴他人 | VCP / NewAPI / OpenAI 均无形式化验证库 |
| 创新点 | Kani skeleton (Apeireth 独家, 护城河之一) |

---

## 5. 时间预算 (14:21 启动, 15:15 截止)

| 阶段 | 时间 | 实际 |
|------|------|------|
| readmap | 8 min | 14:21-14:29 (8 min) ✅ |
| 实施 | 30 min | 14:29-14:59 (30 min) |
| verify + report | 17 min | 14:59-15:15 (16 min) |
| 总计 | 55 min | 55 min |
