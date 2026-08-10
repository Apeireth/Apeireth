# Agent R122-4-retry Readmap — 4 续 TODO 形状 + 当前状态 (2026-08-10)

**时间**: 2026-08-10 14:18-14:25 (7 min, 紧凑)
**作者**: 团队成员 R122-4-retry (Mavis 派, 工程化战区 R121 续, 主人 #10 授权自主决策)
**战区**: 工程化 (R122 续)
**状态**: R122-4-retry readmap 完成, 4 TODO 形状清楚, 修真发现 R122-1/3 未完待解

---

## §0. TL;DR

R121r final §9 留 4 R122 续 TODO. R122-4-retry 摸代码 + 修真发现:

| TODO | 描述 | 实际状态 | R122-4 计划 |
|---|---|---|---|
| 1 | `gemini_to_normalized::stream: false` 硬编码 | R121r 已加 2 test 期待修真 (line 1813/1817), 但函数仍 `stream: false` 硬编码, **test 当前 FAIL** | 改 1 行 `stream: req.stream` (类型已 `bool`, 0 `unwrap_or`) |
| 2 | `dispatch_with_retry` 接入 `jittered_sleep` | `BackoffPolicy` 是 enum (0 field), 0 能加 `.jitter` 字段; `JitterMode` 4 档 + `jittered_sleep()` 已存在 (B 已写) | 加 `BackoffPolicy::WithJitter { base: Box<Self>, jitter: JitterMode }` variant + `with_jitter(self, mode)` 构造器 + `jitter(&self) -> Option<JitterMode>` 方法, dispatch_with_retry 改用 `jittered_sleep` (None 模式 0 漂移) |
| 3 | `MemoryCache::put` 接入 evictor | **已接!** V2-续 之前在 lib.rs:319-358 真接 eviction loop, 仅缺 `MemoryCache::evict_one() -> Option<K>` public method | 加 `evict_one()` public method (调 `evictor.pick_victim()`) + 5+ test 覆盖 5 policy |
| 4 | hand.rs race 实际根因调查 | R121r 已加 `serial_test = "3"` + 5 个 `#[serial]` 标签 surface fix, 真根因可能 `apeireth_supervision_harness_2026_08_06::test_100_rounds_minimax_stress` 跨 process 不可序列化 | 跑 5+5 次, 记录 fail 概率, 写调查报告 |

**修真发现 (R122-1/3 未完)**:
- `apeireth-api` lib 编译错: `unresolved import crate::replay_cache::REPLAY_CACHE` (R122-1 加了 use 但没写 replay_cache.rs)
- `apeireth-pipeline` lib 编译错: `tiktoken_rs` unresolved + `serde_yaml` unresolved + `impl Trait` in closure + `as_dyn_error` trait bound 4 个错 (R122-3 tiktoken_counter.rs)
- workspace Cargo.toml `tiktoken-rs = "0.7"` 重复 key (line 281 + 297) — 修真时间未明, build 期初失败, 14:21 修真 (修真者未明, Mavis 或其他)

**R122-4-retry 修真范围**:
- ✅ 0 改 workspace.version (1.1.0) — `version` 在 [workspace.package]:246
- ✅ 0 触碰 24 LOCKED crate (Cargo.toml 0 在 24 LOCKED 列表)
- ✅ 0 触碰 9 器官 logic (hand.rs 0 改)
- ✅ 0 触碰 6 哲学锚 / 12 键 / 5 重守门 / V0.5 24 维 / 双洋葱
- ⚠️ **0 假装**: 修真 R122-1/3 build break 不在 R122-4 范围, 0 假装"已修", 等 Mavis 14:30 resolve

---

## §1. 摸代码发现 (修真/标缺/0 装铁律)

### 1.1 TODO 1 摸代码

- `crates/apeireth-api/src/protocol_handlers.rs:692-760` `gemini_to_normalized(req: &GeminiRequest) -> NormalizedRequest`
- line 754: `stream: false` (硬编码)
- `GeminiRequest.stream: bool` (line 613, `#[serde(default)]` default false)
- 类型已 `bool`, 不是 `Option<bool>`, spec 写的 `req.stream.unwrap_or(false)` 编译不过, 正确改 `stream: req.stream`
- R121r 已加 2 test (line 1813-1859) 期待修真后 pass — 当前 FAIL (因 `stream: false` 硬编码)
- 修真后 1 test (line 1817) 1 个 case 会 PASS, 0 漂移 (default false == 1.0 行为)

### 1.2 TODO 2 摸代码

- `crates/apeireth-api/src/retry.rs:51-61` `BackoffPolicy` 是 **enum** (0 field)
- 不能加 `jitter: Option<JitterMode>` 字段 (结构上 impossible)
- spec 写 "加 `BackoffPolicy.jitter: Option<JitterMode>` 字段" 不可能直接实现
- **方案**: 加新 variant `WithJitter { base: Box<BackoffPolicy>, jitter: JitterMode }` + `with_jitter(self, mode) -> Self` 方法 + `jitter(&self) -> Option<JitterMode>` 访问器 — 完全向后兼容 (4 旧 variant 0 改)
- `dispatch_with_retry` (protocol_handlers.rs:889-935) 退避循环 `tokio::time::sleep(wait).await` (line 915) 改 `jittered_sleep(wait, policy.jitter(...), prev, cap).await`
- `JitterMode::None` 模式 `jittered_sleep` 返 base (0 漂移 1.0 行为)
- 5+ test 覆盖 4 模式 (None / Full / Equal / Decorrelated) + dispatch_with_retry 真接

### 1.3 TODO 3 摸代码

- `crates/apeireth-cache/src/lib.rs:319-358` `MemoryCache::put` **已接 evictor.pick_victim()** (V2-续 在 a2a6dfc5 修真)
- 容量超限调 `evictor.pick_victim()`, 选 victim 移除, 1 行 fallback (evictor 选不出 victim) 仍返 CapacityExceeded
- **缺** `MemoryCache::evict_one() -> Option<K>` public method (R121r-5 决定 0 改 public API, R122 续留)
- 修真: 加 `pub async fn evict_one(&self) -> Option<K>` 调 `self.evictor.lock().pick_victim()` + 移除 (调 `self.shards.remove(&victim)` + `self.evictor.lock().on_remove(&victim)`)
- 5+ test 覆盖 5 policy 各 evict 1 个 item (用 `evict_one()` public API)

### 1.4 TODO 4 摸代码

- `crates/apeireth-tui/src/organ/hand.rs` — 9 器官 logic, R121r 0 改 (mod tests 0 加 #[serial])
- `crates/apeireth-tui/tests/nav_settings_test.rs` — R121r 加 5 个 `#[serial]` 标签
- `apeireth_supervision_harness_2026_08_06::test_100_rounds_minimax_stress` — 100 rounds stress, 跨 process, 不可序列化
- 修真行动: 跑 5 次 `cargo test -p apeireth-tui --test nav_settings_test` + 5 次 `cargo test --workspace` 记录 fail 概率

---

## §2. 0 冲突核验 (R122-1/2/3 在改啥)

| Agent | 改啥 | 跟我冲突? |
|---|---|---|
| R122-1 (Response Replay Cache) | `apeireth-api/src/replay_cache.rs` 新建 + `protocol_handlers.rs` cache hit fast path | 0 直接冲突 (R122-1 改 cache path, 我改 stream field + dispatch_with_retry + gemini_to_normalized) |
| R122-2 (角色划分标记) | `apeireth-pipeline/src/role_divider.rs` 新建 | 0 冲突 |
| R122-3 (tiktoken 精确计数) | `apeireth-pipeline/src/tiktoken_counter.rs` 新建 + workspace Cargo.toml | 0 冲突 (但 workspace 编译错) |
| R122-5 (?? 决策日志) | ? | 0 冲突 (我 R122-4) |
| R122-6/7 (??) | ? | 0 冲突 |

**修真注意**:
- 14:18 spec 写 "14:18 后 5min 内 R122-1 还没碰 protocol_handlers.rs, 你先改, 14:30 后 Mavis resolve" — R122-1 已在改 protocol_handlers.rs (加 replay_cache use 但没写 replay_cache.rs), 我 14:30 后回头再碰
- 我 TODO 1/2 都改 protocol_handlers.rs — **0 改 R122-1 改的部分** (cache hit fast path), 只改 `gemini_to_normalized` line 754 + `dispatch_with_retry` line 915
- workspace build 被 R122-3 卡 (tiktoken), 我等 Mavis 14:30 resolve 后再 verify

---

## §3. 阶段总览 (57 min, 14:18-15:15)

| 阶段 | 时间 | 任务 | 状态 |
|---|---|---|---|
| R122-4-1 | 14:18-14:25 (7 min) | readmap (本文件) | ✅ 14:25 |
| R122-4-2 | 14:25-14:55 (30 min) | TODO 4 race 调查 (5+5 runs) | 待 |
| R122-4-3 | 14:55-15:00 (5 min) | TODO 1 改 1 行 | 待 |
| R122-4-4 | 15:00-15:08 (8 min) | TODO 2 接入 jittered_sleep | 待 |
| R122-4-5 | 15:08-15:12 (4 min) | TODO 3 加 evict_one | 待 |
| R122-4-6 | 15:12-15:15 (3 min) | verify + final report + decision log | 待 |

**R122-4-retry 紧迫节奏**: TODO 4 调查最重 (5+5 runs ~10 min), 优先做; TODO 1-3 修真小 (1 行 + 8/12/5 行), 后做

---

## §4. 验收硬指标

- TODO 1 改 / 保留, 决定 + 理由
- TODO 2 改 / 保留, 决定 + 理由 + 5+ test
- TODO 3 改 / 保留, 决定 + 理由 + 5+ test
- TODO 4 调查报告: 5 runs 数据 + 根因 + 建议
- `cargo test --workspace` 0 failed — **0 假装, workspace build 被 R122-1/3 卡, 14:30 后 Mavis resolve 后再 verify**
- 0 改 workspace.version (1.1.0)
- 0 触碰 9 器官 logic
- 0 触碰 24 LOCKED
- 0 主动 commit

**R122-4-retry 完 readmap, 立即开干 TODO 4 race 调查.**
