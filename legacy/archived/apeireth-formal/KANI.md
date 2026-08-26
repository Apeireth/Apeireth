# Kani 形式化验证 — `apeireth-formal` (R122-9 v2.1 P2-11)

> **借鉴 ID**: `R122-9-NEW-Kani-2026-08-10` (VCP 0 Kani, Apeireth 独家, 护城河之一)
> **范围**: 本 crate 的 5 个新 Kani harness + 5 个 runtime smoke test
> **架构**: 跟既有 `lib.rs::PermissionLayerConfig` 1:1 模式, **0 触碰 24 LOCKED crate**
> **官方**: <https://model-checking.github.io/kani/>

---

## 1. 什么是 Kani (1 段)

Kani 是 AWS 开源的 Rust 模型检查器 (基于 CBMC):
- **符号执行** + **有界模型检查**, 完备覆盖**所有**输入(非抽样的)
- 对**所有非确定性输入**自动探索 (`kani::any()`)
- 给出反例轨迹 (若不变量失败)
- 与 cargo 集成 (`cargo kani`)

代价: 慢 + 内存大. 单个简单 harness 通常 1-5 分钟, 复杂状态空间可能数小时.

---

## 2. 5 个 Kani harness (本 R122-9 新增)

每个 harness 都在 `src/kani_harness.rs`, 用 POD 模型镜像 LOCKED crate 的形式属性. **0 触碰** 24 LOCKED crate (per `docs/omnibus/24-locked-crates.md`).

### 2.1 `kani_verify_backoff_policy_step_within_cap`

**验证**: `BackoffPolicyPod::tier_at(idx) ≤ cap` 永真
**对应 R122-4 LOCKED**: `apeireth-api/retry.rs::BackoffPolicy::to_durations()`
**POD 模型**: `BackoffPolicyPod { tiers_ms: [u32; 8], tier_count: u8, cap_ms: u32 }`
**不变量**: 任意 6 档 + 任意 cap, `tier_at(idx)` 返值 ≤ cap
**不漂移 1.0**: 默认 `Patient()` = 1s/3s/10s/30s/2m/10m (6 档, 跟 LOCKED 1:1)

### 2.2 `kani_verify_jitter_sleep_returns_value_in_range`

**验证**: `jittered_sleep(..., Full) ∈ [0, cap]` 永真
**对应 R122-4 LOCKED**: `apeireth-api/retry.rs::jittered_sleep`
**POD 模型**: `jittered_sleep_pod(base_ms, JitterModePod, prev_ms, cap_ms) -> u32`
**不变量**: Full 模式返值 ≤ cap, None 模式 = min(base, cap), Equal 模式 ≤ base
**不漂移 1.0**: 4 档 `JitterModePod` 跟 LOCKED `JitterMode` enum 1:1 (None/Full/Equal/Decorrelated)

### 2.3 `kani_verify_response_cache_capacity_respected`

**验证**: ResponseCache POD LRU 超 cap 0 panic, `len ≤ cap` 永真
**对应 R120 LOCKED**: `apeireth-api/cache.rs::ResponseCache` (async, Kani 0 直接符号化)
**POD 模型**: `ResponseCachePod { len: u32, cap: u32 }` + `lru_insert()`
**不变量**: 任意 cap, 任意 insert 次数, 0 panic, `len ≤ cap` 永真
**不漂移 1.0**: 静默驱逐 (LRU 最旧) 跟 LOCKED fail-soft 1:1

### 2.4 `kani_verify_response_replay_lookup_consistent`

**验证**: `ResponseReplayCache.lookup` 任意 key (含 empty) 0 panic
**对应 R122-1 LOCKED**: `apeireth-api/replay_cache.rs::ResponseReplayCache::lookup`
**POD 模型**: `ResponseReplayPod { len: u32, cap: u32, hash_len: u32 }` + `invariant_lookup_no_panic(key_len)`
**不变量**: 任意 key_len (0..=N), lookup 路径 0 panic
**不漂移 1.0**: HashMap::get_mut(empty) 合法, 跟 LOCKED 行为 1:1

### 2.5 `kani_verify_role_divide_wrap_unwrap_round_trip`

**验证**: `wrap_with_role(r, c)` + `parse_typed_message` 闭环, 0 panic
**对应 R122-2 LOCKED**: `apeireth-pipeline/role_divider.rs::{wrap_with_role, parse_typed_message}`
**POD 模型**: `RoleDividePod { role: u8, content_len: u32 }` + `wrap_len()` + `invariant_round_trip()`
**不变量**: 6 role (0..=5) + 任意 content_len, wrap + parse 0 panic, round trip 还原 role
**不漂移 1.0**: 22 字符 START + 23 字符 END 跟 LOCKED 1:1, 6 variant 跟 LOCKED 1:1

### 2.6 Helper: `pub fn any_string<const N: usize>() -> String`

**用途**: Kani 符号化字符串生成器, 给 harness 调用方用
**行为**:
- Kani 模式: 返长度 N 的 nondet 字符串 (`String::with_capacity(N)` + N 次 `kani::any::<char>()`)
- 非 Kani 模式: 返 `"a".repeat(N)` (cargo test smoke 用)
**0 触碰 String 内部**: Kani 模式下 String 是固定 N 容量, 不会 heap 状态爆炸

---

## 3. 跑命令

### 3.1 跑单个 harness

```bash
# 仓库根
cd Apeireth-rust

# 5 个新 harness 逐个跑 (R122-9)
cargo kani -p apeireth-formal --harness kani_verify_backoff_policy_step_within_cap
cargo kani -p apeireth-formal --harness kani_verify_jitter_sleep_returns_value_in_range
cargo kani -p apeireth-formal --harness kani_verify_response_cache_capacity_respected
cargo kani -p apeireth-formal --harness kani_verify_response_replay_lookup_consistent
cargo kani -p apeireth-formal --harness kani_verify_role_divide_wrap_unwrap_round_trip

# 既有 5 个 (R22 ST-A5)
cargo kani -p apeireth-formal --harness double_onion_sample
cargo kani -p apeireth-formal --harness e_layer_isolation
cargo kani -p apeireth-formal --harness permission_grant_l0
cargo kani -p apeireth-formal --harness mid_task_atomicity
cargo kani -p apeireth-formal --harness seven_advisor_voting
```

### 3.2 跑 cargo test (5 smoke test, 0 装)

```bash
cargo test -p apeireth-formal --lib

# 输出: 30 passed (5 kani_harness + 1 all_5 + 1 any_string + 23 existing)
```

### 3.3 跑 cargo build (sanity)

```bash
cargo build -p apeireth-formal
```

### 3.4 可选 flag

```bash
# 看 Kani 内部 trace / CBMC args
cargo kani -p apeireth-formal --harness kani_verify_backoff_policy_step_within_cap --verbose

# 给 harness 限定 unwind bound (本 crate 5 harness 默认 100, 见 kani.toml)
cargo kani -p apeireth-formal --harness kani_verify_jitter_sleep_returns_value_in_range --unwind 50
```

---

## 4. 0 触碰 24 LOCKED crate (核验)

| R122-x | LOCKED crate | R122-9 验证方式 |
|---|---|---|
| R122-1 ResponseReplayCache | `apeireth-api/replay_cache.rs` | `ResponseReplayPod` POD 模型 (`kani_verify_response_replay_lookup_consistent`) |
| R122-2 RoleDivider | `apeireth-pipeline/role_divider.rs` | `RoleDividePod` POD 模型 (`kani_verify_role_divide_wrap_unwrap_round_trip`) |
| R122-4 BackoffPolicy + JitterMode | `apeireth-api/retry.rs` | `BackoffPolicyPod` + `jittered_sleep_pod` POD 模型 (2 个 harness) |
| R120 ResponseCache (async) | `apeireth-api/cache.rs` | `ResponseCachePod` POD 模型 (`kani_verify_response_cache_capacity_respected`) |

**POD 镜像原则** (per `lib.rs::PermissionLayerConfig` 1:1 模式):
- 0 复用 LOCKED crate 的真实类型 (String / Vec / HashMap 在 Kani 下状态爆炸)
- 仅验证**形式属性 shape** (tier ≤ cap / jitter ∈ [0, cap] / len ≤ cap / 0 key lookup 0 panic / round trip)
- 真实生产代码不调用进 harness (per `lib.rs` 禁止条款)

---

## 5. 0 假装 (per 哲学锚 #1)

| 项 | 实情 |
|---|---|
| 5 harness 0 假装"全形式化" | 仅 5 个关键不变量骨架, 真实生产级形式化 (full coverage, contract verification, model checking on real production types) 留给 R123 续扩 |
| POD 模型 0 假装"调用真实 LOCKED 代码" | 完全本地 POD 镜像, 形式属性 1:1 跟 LOCKED 行为对应, 但 0 触碰 LOCKED |
| `cargo kani` 0 装"已形式化全部" | 仅 5 harness + 既有 5 = 10 个, R123 续扩到 30+ |
| `cargo test` 0 装"smoke test = 形式化证明" | 5 smoke test 仅验证代码 path 0 panic, 形式化证明靠 `cargo kani` |

---

## 6. 已知陷阱 (Kani 工程经验)

| 陷阱 | 说明 |
|---|---|
| **String / Vec / HashMap** 入参 | Kani 面对堆类型状态爆炸, 用 POD (u8 / u32 / bool / 固定 array) |
| **浮点** (`f32` / `f64`) | Kani 支持但成本高, 用整数 + 定点更好 |
| **递归 / 任意循环** | 必须配 `--unwind N`; 否则 CBMC 会跑死 |
| **`unsafe`** | 本 crate 已 `deny(unsafe_code)`; 其它 crate 也应避免在 harness 路径用 |
| **大 N (e.g. 100 步)** | Kani 单 harness 可能跑几小时. 拆小 harness, 用 assume 收窄 |
| **`kani::any()`** 在 `#[cfg(not(kani))]` 模式下 | 必须通过 `nondet_*()` helper 间接调, 跟 `lib.rs::PermissionLayerConfig` 1:1 模板 |

---

## 7. 引用

- Kani 官方: <https://model-checking.github.io/kani/>
- CBMC 后端: <https://www.cprover.org/cbmc/>
- 论文: "Kani: A New Rust Verifier" (AWS, 2024)
- 项目文档: `docs/v2-strategy/03-EXTREME-PLAN.md §4A` (战区 5)
- 既有 5 harness: `src/invariants/double_onion_sample.rs` 等
- Kani 安装 / 跑 / 写新不变量 (per 主人前置笔记): `docs/kani-setup.md`
- 路线图来源: R2 路线图 P2-11 (Kani 形式化验证, R122-9 周期)
