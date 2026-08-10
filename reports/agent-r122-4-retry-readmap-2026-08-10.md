# Agent R122-4-retry Readmap — R121-retry 续 4 TODO (2026-08-10)

**时间**: 2026-08-10 14:17-14:25 (8 min)
**作者**: 团队成员 R122-4-retry (Mavis 派, 工程化战区, 主人 #10 授权自主决策)
**状态**: ✅ Readmap 完成, 4 TODO 现状查清, 立即开干

---

## §0. TL;DR

R122-4 因 Connection error 失败重启. 4 TODO 现状查清:

1. **TODO 1** gemini stream 硬编码 — `protocol_handlers.rs:754` 仍是 `stream: false` 硬编码, 改 1 行即可
2. **TODO 2** dispatch_with_retry 接 jittered_sleep — `retry.rs:53-61` BackoffPolicy 是 enum, 加 variant `WithJitter` 100% 向后兼容
3. **TODO 3** MemoryCache::put evictor — R121 续 V2-4 已接 `evictor.pick_victim()` (lib.rs:327-347), **只缺** `evict_one()` public method + 5+ 集成 test
4. **TODO 4** hand.rs race — 跑 5+5 = 10 个 run, 写根因报告, 0 改 hand.rs

---

## §1. 现状摸底 (4 TODO 涉及代码)

### TODO 1: gemini stream 硬编码
- 位置: `crates/apeireth-api/src/protocol_handlers.rs:754`
- 当前: `stream: false,` (硬编码)
- `GeminiRequest.stream: bool` 字段已存在 (R121 续 V2-2 加的, `#[serde(default)]` 缺省 false)
- 改 1 行: `stream: req.stream,` 即可
- 现有 7 个 stream_forward_tests (R121r-3) 验证 4 协议流式 + serde 行为, 0 改 test code

### TODO 2: BackoffPolicy + dispatch_with_retry jitter
- 位置: `crates/apeireth-api/src/retry.rs:53-61` (BackoffPolicy enum) + `crates/apeireth-api/src/protocol_handlers.rs:889-935` (dispatch_with_retry)
- 当前: `BackoffPolicy` 是 4 variant enum (Aggressive/Default/Patient/Custom), 0 加 jitter 字段
- `JitterMode` 4 模式 (None/Full/Equal/Decorrelated) + `jittered_sleep` 函数已存在 (R121 续 V2-4 加的)
- 决策: **加 enum variant `WithJitter(Box<BackoffPolicy>, JitterMode)`**, 100% 向后兼容
  - `to_durations()` 加 1 行 match: `WithJitter(p, _) => p.to_durations()`
  - 加 method `with_jitter(self, mode: JitterMode) -> Self` 包装
  - 加 method `jitter(&self) -> JitterMode` 查询
  - `tier_count()` 加 1 行 match 同理
- `dispatch_with_retry` 改 `tokio::time::sleep(wait).await` → 用 `jittered_sleep(wait, policy.jitter(), prev, cap)` 计算再 sleep, 跟踪 `prev = Some(jittered_wait)`
- 加 5+ test 覆盖 4 jitter mode + `with_jitter` 链

### TODO 3: MemoryCache::put evictor + evict_one()
- 位置: `crates/apeireth-cache/src/lib.rs:217-285` (MemoryCache struct + new) + `lib.rs:319-358` (put impl)
- 当前:
  - R121 续 V2-4 **已** 把 `evictor.pick_victim()` 接入 `MemoryCache::put` (line 327-347)
  - `MemoryCache::evict_one() -> Option<K>` **未** 存在 (grep 0 命中)
  - 现有 10 个 lib.rs test 全是构造 + 守门级别, **缺** 5 policy 集成 test
  - R121r-5 6 test 是 Evictor trait 单元层 (5 policy .policy() 标签 1:1), **缺** MemoryCache 集成层
- 决策: **加 `MemoryCache::evict_one()` public method + 5+ 集成 test**
  - `evict_one()` 从 put 内部抽出, 公共 method
  - 5+ test: `put` 到 max_size, 再 `put` 1 个, 验证 cache.len() == max_size, 验证 evicted 的 key 是 expected
  - 5 个 policy 各 1 test (LRU/LFU/FIFO/ARC/TinyLFU)

### TODO 4: hand.rs race 调查
- 位置: 跑测试, 0 改 hand.rs (严守硬约束 #4)
- 跑 5 次 `cargo test -p apeireth-tui --test nav_settings_test` (single-package, 5 consecutive runs)
- 跑 5 次 `cargo test --workspace` (5 consecutive runs)
- 记录 fail 概率 + 根因 (如可复现)
- 输出 `reports/agent-r122-4-retry-race-investigation-2026-08-10.md`

---

## §2. 0 冲突核验 (跟 R122-1-retry / R122-2 / R122-3)

| Agent | 改 / 写 | 我 | 0 冲突? |
|---|---|---|---|
| R122-1-retry | `replay_cache.rs` (新建) + `protocol_handlers.rs` cache hit path (dispatch_inner 内部) | `protocol_handlers.rs` line 754 (gemini) + line 889-935 (dispatch_with_retry) | ✅ 0 冲突 (不同位置) |
| R122-2 | `apeireth-pipeline/src/role_divider.rs` (新建) | 0 触碰 | ✅ |
| R122-3 | `tiktoken-rs` 替换 `token_pieces()` | 0 触碰 | ✅ |

**严守**: 我改 `protocol_handlers.rs` 不碰 `dispatch_inner` (R122-1 改的位置), 我改 `dispatch_with_retry` 0 改签名.

---

## §3. 决策预判 (4 TODO 拍板)

| TODO | 改 / 保留 | 理由摘要 |
|---|---|---|
| 1 | **改** | 1 行, 0 漂移 (req.stream default false), 跟其他 3 协议 1:1 |
| 2 | **改** (加 WithJitter variant) | 100% 向后兼容, 0 改 4 既有 variant |
| 3 | **改** (加 evict_one + 5+ 集成 test) | R121 续已接 put, 只缺 public method + MemoryCache 集成 test |
| 4 | **调查** (0 改 hand.rs) | 跑 5+5 个 run, 写报告 |

---

## §4. 时间预算

- 14:17 启动
- 14:25 readmap 完 (8 min)
- 14:30 TODO 1 完 (5 min, 1 行改 + 跑 1 个 test)
- 14:45 TODO 2 完 (15 min, enum variant + 2 method + 1 dispatch 改 + 5+ test)
- 15:00 TODO 3 完 (15 min, evict_one + 5+ 集成 test)
- 15:10 TODO 4 完 (10 min, 跑 5+5 个 test, 写报告)
- 15:15 verify + final (5 min)

---

## §5. 硬约束复述 (8 墙)

1. ✅ 0 改 workspace.version (1.1.0) — 加 `WithJitter` enum variant + `evict_one()` method + `with_jitter` 构造器 + `jitter()` method 全部向后兼容, 不算改公共 API 签名
2. ✅ 0 改 R11 baseline 3 值 — 0 触碰 R11
3. ✅ 0 触碰 24 LOCKED — 0 触碰
4. ✅ 0 触碰 9 器官 logic — TODO 4 调查 0 改 hand.rs
5. ✅ 0 改 11 agent 公共 API 签名 — TODO 2 选 B) enum variant (100% 向后兼容)
6. ✅ 0 主动 commit
7. ✅ 0 装 (O-5)
8. ✅ 0 范围扩散 — 严守 4 TODO 范围, 0 改其他

---

**R122-4-retry readmap 完. 立即开干 TODO 1.**
