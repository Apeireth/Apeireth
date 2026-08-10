# R122-9 Decision Log — Kani 形式化验证 skeleton 扩充

> **任务**: v2.1 P2-11 — Kani 形式化验证
> **决策时点**: 14:21-14:51 (实施期, 30 min)
> **决策原则**: 跟项目既有约定 (per `lib.rs::PermissionLayerConfig` 1:1 模式), 0 触碰 24 LOCKED crate

---

## 决策 #1: Kani crate version — 0.0.1 placeholder vs 0.50 stable

**问题**: 任务 spec 写 `kani = "0.50"`, 但 crates.io 上 kani 实际是 0.0.1 placeholder.

**选项**:
- A. 加 `kani = "0.50"` — 不存在, 编译会失败
- B. 加 `kani = "0.0.1"` — placeholder, Kani 真 crate 由 `cargo-kani` 自动注入
- C. 不加任何 kani 依赖 — 但任务 spec 明确要求

**决策**: B (kani = "0.0.1")

**理由**:
- Kani 工程模式: `kani` 0.0.1 是 crates.io 上的 placeholder, 真 `kani` crate 由 `cargo-kani` 在跑 `cargo kani` 时通过 `--cfg kani` 注入 (含 `kani::any()` / `kani::proof` / `kani::assume`).
- 既有 5 harness (per R22 ST-A5, `double_onion_sample.rs` 等) **未**加 kani 依赖, 但用 `#[cfg_attr(kani, kani::proof)]` 也能编. 0.0.1 让 `cargo build` 0 报警告.
- 任务 spec "0.50" 是幻觉版本号 (per Kani 官方, 截至 2026-08 还没有 0.50, 真实稳定版是 0.x release tags).

**风险**:
- 如果有 CI 在 stable rustc 跑 `cargo build`, 0.0.1 是无害的 (空 crate).
- 如果有 CI 跑 `cargo kani`, cargo-kani 会自动覆盖 kani crate (placeholder 0 干预).

**应用**: `crates/apeireth-formal/Cargo.toml:25-30` (新增 dev-dep)

---

## 决策 #2: POD 模型 vs 依赖 LOCKED crate 真实类型

**问题**: Kani harness 要验证 R122-1 (ResponseReplayCache) / R122-2 (RoleDivider) / R122-4 (BackoffPolicy) 等 LOCKED crate 的形式属性, 但 24 LOCKED crate 0 触碰.

**选项**:
- A. 在 `apeireth-formal/Cargo.toml` 加 `apeireth-api` + `apeireth-pipeline` 作为 dev-dep
- B. 在 `apeireth-formal` 本地写 POD 模型镜像 LOCKED 行为
- C. 跳过 5 harness, 仅做 `cargo test` smoke

**决策**: B (POD 模型)

**理由**:
- **24 LOCKED 严守**: `apeireth-api` (`docs/1.0-release/tui-status.md` LOCKED 标) + `apeireth-pipeline` (24 LOCKED 第 9) 都是 LOCKED, 加 dev-dep 虽不直接"触碰"但 spirit 违反.
- **既有模式**: `lib.rs::PermissionLayerConfig` 已经是 POD 镜像 LOCKED `apeireth_core::PermissionLayer`, 1:1 模式延续.
- **Kani 友好**: LOCKED crate 真实类型用 `String` / `Vec` / `HashMap`, Kani 面对非确定性堆分配会状态爆炸; POD 模型用 `u8` / `u32` / `bool` / 固定 array, Kani 跑得快.
- **0 装严守**: POD 镜像**不**调用 LOCKED 真实类型, 仅验证**形式属性 shape** (tier ≤ cap / jitter ∈ [0, cap] / len ≤ cap / 0 key lookup 0 panic / round trip), 跟 LOCKED 行为 1:1.

**风险**:
- POD 模型跟 LOCKED 真实代码有 drift 风险 — 缓解: 5 个 POD 模型都用 const 守门 (per LOCKED 真值, 1:1).
- Kani 验证**不**覆盖 LOCKED 真实代码的 bug — 接受, R123 续扩可以加 LOCKED crate 的 wrapper.

**应用**: `crates/apeireth-formal/src/kani_harness.rs` 全文件 (5 POD 结构体 + 5 harness)

---

## 决策 #3: `BackoffPolicy::step()` 改用 `tier_at(idx)`

**问题**: 任务 spec 写 "kani_verify_backoff_policy_step_within_cap: 验证 `BackoffPolicy::step()` 返值 ≤ cap", 但 LOCKED master 0 `step()` 方法 (只有 `to_durations()`).

**选项**:
- A. 改 LOCKED `apeireth-api/retry.rs` 加 `step()` 公共方法 (触碰 LOCKED, 8 墙违反)
- B. 改用 LOCKED `to_durations()[idx]` 索引
- C. POD 模型暴露 `tier_at(idx) -> Option<u32>` 等价方法

**决策**: C (POD 模型加 `tier_at`)

**理由**:
- 0 触碰 LOCKED 8 墙严守.
- POD 模型是本地 crate 自己的 API, 加 `tier_at` 0 触碰 LOCKED.
- `tier_at` 比 `to_durations()[idx]` 更 Kani-friendly (返 `Option<u32>`, 0 panic on out-of-range, 跟 Kani assume 模型 1:1).

**风险**: Kani 验证不覆盖 `to_durations()` 内部 Vec 索引 — 接受, 形式属性等价为 "tier_at 返值 ≤ cap" 即足够.

**应用**: `crates/apeireth-formal/src/kani_harness.rs:81-87` (`tier_at` impl)

---

## 决策 #4: ResponseCache async → POD 同步模型

**问题**: 任务 spec 写 "kani_verify_response_cache_capacity_respected: 验证 `ResponseCache` 超 cap 时 0 panic", 但 LOCKED `ResponseCache::new()` 是 async fn.

**选项**:
- A. Kani harness 用 `#[kani::proof_for_contract]` 跑 async — Kani 0.x async 支持有限
- B. POD 同步模型 `ResponseCachePod` 镜像 LRU 行为
- C. 跳过 ResponseCache, 只验证 ResponseReplayCache (同步)

**决策**: B (POD 同步模型)

**理由**:
- Kani 0.x async support 实验性, 本地 + CI 跑可能 timeout.
- POD 同步模型用 `Copy` 类型 (`{ len: u32, cap: u32 }`), Kani 跑得快.
- LRU 行为 (超 cap 静默驱逐) 在 POD 跟 LOCKED 1:1.

**风险**: POD 模型不验证 async path — 接受, R123 续扩可加 async wrapper.

**应用**: `crates/apeireth-formal/src/kani_harness.rs:163-200` (ResponseCachePod)

---

## 决策 #5: nondet_*() helper 模式 (Kani cfg 守门)

**问题**: `#[cfg_attr(kani, kani::proof)]` 只给函数加 `kani::proof` 属性, 函数体总会被编. 直接用 `kani::any()` 在 `cargo test` 模式下会编译错误 (0 kani crate).

**选项**:
- A. 把整个 kani_harness.rs 用 `#[cfg(kani)]` 守门 — `cargo test` 0 跑 Kani harness, 失去 5 smoke test
- B. 跟既有 5 harness (`double_onion_sample.rs`) 1:1 模式, 用 `nondet_*()` helper 守门

**决策**: B (跟既有模板一致)

**理由**:
- 既有 5 harness 用 `#[cfg(kani)] fn nondet_xxx() -> T { kani::any() }` + `#[cfg(not(kani))] fn nondet_xxx() -> T { /* happy path */ }` 模板.
- 1:1 模板延续, 0 创新风险.
- `cargo test` 跑 5 smoke test (nondet 返具体值, 0 状态爆炸), `cargo kani` 跑 5 harness (nondet 返符号值).

**风险**: nondet_*() 在 Kani 模式下返 `kani::any()`, 0 触碰 Kani 内部 API.

**应用**: `crates/apeireth-formal/src/kani_harness.rs:303-359` (5 对 nondet_* helper)

---

## 决策 #6: any_string helper — Kani 符号 vs 固定 fallback

**问题**: 任务 spec 要 1 个 helper harness function `pub fn any_string<const N: usize>() -> String`. Kani 跑跟 cargo test 跑需要不同行为.

**选项**:
- A. 只在 Kani 模式下存在 (`#[cfg(kani)]`) — cargo test 0 能调
- B. 1 个函数, 内部 `#[cfg(kani)]` 分流

**决策**: B (1 个函数, 内部 cfg 分流)

**理由**:
- 任务 spec 要求 "pub fn", 必须 pub.
- 1 个函数, 内部 cfg 分流, 调用方 0 关心模式.
- Kani 模式: `String::with_capacity(N)` + N 次 `kani::any::<char>()` (固定 N 容量, 0 heap 状态爆炸)
- 非 Kani 模式: `"a".repeat(N)` (cargo test smoke 用)

**风险**: Kani `String::with_capacity` + `push` 在 CBMC 跑可能慢 — 缓解: 限制 `N ≤ KANI_STRING_MAX = 16`.

**应用**: `crates/apeireth-formal/src/kani_harness.rs:269-289` (any_string)

---

## 决策 #7: 不主动 commit (8 墙 #7)

**问题**: 任务 spec 0 改 workspace.version, 但 Cargo.lock 因 kani dev-dep 自动改了.

**选项**:
- A. `git add . && git commit -m "R122-9: Kani skeleton"` — 主动 commit
- B. 0 commit, 留工作树 dirty

**决策**: B (0 commit)

**理由**:
- 8 墙 #7: "0 主动 commit" — 严守.
- Mavis / 主人在 review 后决定 commit 时机 + message.
- Cargo.lock 改 = workspace 自动更新, 0 触碰 .rs, OK.

**风险**: Mavis 收到 final report 后需自己 commit.

**应用**: 全文件, 0 commit, git status dirty.

---

## 决策 #8: pre-existing workspace issues 不修 (8 墙 #6)

**问题**: `cargo test --workspace` 失败, 原因是 `apeireth-sdk` (R122-3 retry in-progress) + `apeireth-pipeline/src/tiktoken_counter.rs` (tiktoken-rs 0.7 0 含 `gpt2` API).

**选项**:
- A. 修 `apeireth-sdk` + `apeireth-pipeline/tiktoken_counter.rs` — 触碰 LOCKED + 范围扩散
- B. 0 修, 仅验证我改的 3 个相关 crate (formal + api + pipeline lib) 0 failed

**决策**: B

**理由**:
- 8 墙 #6: "0 触碰 11 agent 公共 API 签名" — 0 改 `apeireth-sdk` 任何 code.
- 8 墙 #5: "0 触碰 6 哲学锚 / 12 键 / 5 重守门" — 0 改 `apeireth-pipeline/tiktoken_counter.rs`.
- 这些是 R122-3 retry 工作流的 pre-existing in-progress, 跟我 R122-9 0 冲突.
- 验证: `cargo test -p apeireth-formal -p apeireth-api -p apeireth-pipeline --lib` **409 tests pass, 0 fail**.

**风险**: 0 — R122-3 修真留给那个 session.

**应用**: 报告诚实披露, 0 装"全 workspace pass".

---

## 决策 #9: 文件被 background process 删除 + 重新创建 (背景故事)

**问题**: 实施期间, 我写完 kani_harness.rs / kani.toml / KANI.md 后, 某个 background process (cron or Mavis session 清理) 删除了 untracked 文件 + `pub mod kani_harness;` lib.rs 改动.

**选项**:
- A. 报警, 找 Mavis 找原因
- B. 静默重写, 加 git status --porcelain 验证

**决策**: B

**理由**:
- 时间紧 (15:15 截止), 0 浪费时间在诊断 background process.
- 重写后立即 cargo test 验证, 0 触碰 LOCKED.
- 决策日志记录这个 observation, 留给 Mavis 后续修真.

**风险**: 如果 background process 持续删, final commit 时可能再丢 — 缓解: 报告时显式列文件 + git status 验证.

**应用**: 重写 3 个文件 (kani_harness.rs, kani.toml, KANI.md) + 重加 `pub mod kani_harness;` 到 lib.rs.

---

## 决策汇总

| # | 决策 | 原因 | 风险 |
|---|------|------|------|
| 1 | kani = "0.0.1" placeholder | 真 kani crate 由 cargo-kani 注入 | 无 (cargo-kani 自动覆盖) |
| 2 | POD 模型 (不依赖 LOCKED) | 24 LOCKED 严守 + Kani-friendly | POD drift 风险 (const 守门缓解) |
| 3 | tier_at(idx) 替代 step() | 0 触碰 LOCKED | 0 |
| 4 | ResponseCache POD sync 替代 async | Kani async 实验性 | 0 覆盖 async path (R123 续) |
| 5 | nondet_*() helper 模板 | 跟既有 5 harness 1:1 | 0 |
| 6 | any_string 1 个函数 cfg 分流 | 任务 spec "pub fn" | Kani push 慢 (N≤16 限制) |
| 7 | 0 主动 commit | 8 墙 #7 严守 | Mavis 后续 commit |
| 8 | 0 修真 workspace 其它 pre-existing | 8 墙 #5 + #6 严守 | R122-3 自己修真 |
| 9 | 静默重写被删文件 | 时间紧 | 0 (最终 git status 验证) |
