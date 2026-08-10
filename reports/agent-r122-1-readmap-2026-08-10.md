# R122-1 Readmap — Response Replay Cache (VCP 借鉴)

**Date**: 2026-08-10  
**Coder**: R122-1-retry (Connection error 第一波失败后重派)  
**Slot**: v2.1 P1 #1, 借鉴 VCP `chatCompletionHandler.js:73-156 ResponseReplayCache`  
**借鉴 ID**: `R122-1-VCP-ResponseReplayCache-2026-08-10`

---

## 1. 任务一句话

新建 `crates/apeireth-api/src/replay_cache.rs` (~150-250 行) 1:1 翻译 VCP `ResponseReplayCache`
(Map 存 key→entry + maxEntries LRU + cachedAt TTL), 集成到 `protocol_handlers.rs`
的 `send_and_decode_with_status` (4 协议 HTTP 出口, 0 改 dispatch 签名, 0 改 R120 dispatch_cached 行为)。

## 2. 现状摸清 (7 项关键事实)

| # | 事实 | 路径:行 | 影响 |
|---|------|---------|------|
| 1 | workspace version 1.1.0 | `Cargo.toml:246` | 0 改 ✓ |
| 2 | R11 baseline 3 值 0.8682/0.8532/0.9063 | `crates/apeireth-asi/tests/integration_r_measure.rs:42-44` | 0 触碰 ✓ |
| 3 | R120 已有 `cache.rs` 用 apeireth-cache (MemoryCache, LRU+TTL+32 分片) | `crates/apeireth-api/src/cache.rs:147-208` | 跟我新加的 `replay_cache.rs` 共存, 不冲突 (2 层 cache) |
| 4 | R120 已有 `dispatch_cached_with_status` 走 `cache.put/get` | `protocol_handlers.rs:844-873` | 0 改 ✓ |
| 5 | 4 协议 HTTP 出口统一在 `send_and_decode_with_status` | `protocol_handlers.rs:1118-1167` | **集成点** ✓ — 有 url + body, 跟 `hash_request(method, url, body)` 完美对齐 |
| 6 | sha2 + lru + once_cell 已在 apeireth-api/Cargo.toml | `Cargo.toml:51,59,61` | 0 新 dep ✓ |
| 7 | R121 续有 5+ tests 在 `mod tests` (line 1271-) | `protocol_handlers.rs:1271-1872` | 我新加 `mod replay_cache_tests` 跟它并列, 0 影响 |

## 3. 集成点选择 (决策)

**任务 spec 字面要求** "在 `dispatch_inner` 内部加 cache hit fast path + record"。
但 `dispatch_inner` (line 982-991) 是 thin wrapper, 实际 5 步管线在
`dispatch_inner_with_status` (line 940-977), 4 协议 HTTP 出口统一在
`send_and_decode_with_status` (line 1118-1167)。

**最终 hook 点**: `send_and_decode_with_status` 内部:
- HTTP send 之前: `hash_request("POST", &url, body_bytes) + REPLAY_CACHE.lookup`
- 命中 → 返 (200, Ok(cached))
- 未命中 → 走原 HTTP + decode
- 成功后 → `REPLAY_CACHE.record`

**为什么是 send_and_decode_with_status**:
1. 4 协议全走, 1 处覆盖 4 协议
2. 有 `url` + `body: &Value` 完整信息, 跟 `hash_request(method, url, body)` 签名 1:1
3. 跟 R120 `dispatch_cached` 互补不冲突: R120 在 dispatch 层 (NormalizedRequest 哈希), 我在 HTTP 层 (raw POST + URL 哈希)
4. 0 改 `dispatch` / `dispatch_cached` / `dispatch_with_retry` 任何签名 ✓

## 4. 8 墙核验 (实施前)

| 墙 | 状态 |
|----|------|
| 0 改 workspace.version (1.1.0) | ✓ (0 动 Cargo.toml) |
| 0 改 R11 baseline 3 值 | ✓ (0 动 integration_r_measure.rs) |
| 0 触碰 24 LOCKED crate mtime | ✓ (0 动其他 24 crate) |
| 0 触碰 9 器官 logic | ✓ |
| 0 触碰 6 哲学锚 / 12 键 / 5 重守门 | ✓ |
| 0 改 11 agent 公共 API 签名 | ✓ (只 `pub mod replay_cache` 加 module, 不改签名) |
| 0 主动 commit | ✓ |
| 0 装 (O-5) | ✓ (sha2/lru/once_cell 已有, 0 新 dep) |

## 5. VCP 借鉴映射 (字段级)

| VCP 字段 (`chatCompletionHandler.js:73-156`) | Rust 翻译 | 位置 |
|---|---|---|
| `this.cache = new Map()` | `Arc<RwLock<HashMap<String, ReplayEntry>>>` | `replay_cache.rs:ResponseReplayCache` |
| `this.maxEntries` | `max_entries: usize` | 同上 |
| `enabled` (默认 true) | `impl Default` (1.0 行为 0 漂移, 默认 1000/1h) | `impl Default` |
| `cache.set(key, { ...entry, cachedAt: Date.now() })` | `created_at: SystemTime` + `record` | `ReplayEntry` |
| LRU `get(key) { delete + set }` | `lookup` 内 0 漂移 (VCP JS Map 保序即 LRU, Rust HashMap 无序, LRU 由 `evict_lru` 独立管) | `lookup` |
| `while (size > maxEntries) { delete oldestKey }` | `evict_lru(max)` | `evict_lru` |
| VCP 0 TTL (Map 不支持 TTL) | `evict_expired(now)` + 1h default | `evict_expired` |
| `buildKey(clientIp, messageId)` | `hash_request(method, url, body)` (跟 spec 1:1, 不漂移) | `hash_request` |
| `replay(key, req, res)` (写 chunks) | `lookup` 返 Option<ReplayEntry> (调用方决定怎么用) | `lookup` |
| `installResponseCacheRecorder` (line 126+) | 不移植 (VCP 在 Express middleware 装, 我在 HTTP 层 hook) | 0 移植 |

## 6. 风险 & 缓解

| 风险 | 缓解 |
|------|------|
| 跟 R120 `cache.rs` 行为重复 (双层 cache) | 不重复 — R120 哈希 `NormalizedRequest`, 我哈希 `(method, url, raw body)`. 同请求 2 层都命中无害 (但增加 1 次 lookup 开销, 接受) |
| `RwLock` 写阻塞 (高并发) | VCP 用 Map 0 锁; Rust `parking_lot::RwLock` 已在 dep (line 51), 用它性能跟 std 等价但无 poison |
| 序列化 NormalizedResponse 出错 | `record` 内部 `serde_json::to_vec` 失败 → 静默跳过 (跟 R120 `put` 一致) |
| TTL 过期 0 主动清理 | `evict_expired` 暴露成 pub fn, 留给 caller 周期调 (VCP 0 TTL 同样不主动清理) |
| HashMap 无序 (LRU 实现) | `evict_lru` 按 `created_at` ASC 排序删最旧, 0 假装 LRU |
| 集成测试在 `tests/` 需要 send_and_decode_with_status pub 可见性 | 当前是 `async fn send_and_decode_with_status` (无 `pub`) — 集成测试通过 dispatch 间接测, 不需要 pub |

## 7. 实施节奏 (57 min)

| 段 | 时间 | 内容 |
|----|------|------|
| Readmap | 14:18-14:26 (8 min) | 本文件 ✓ |
| 实施 replay_cache.rs | 14:26-14:50 (24 min) | 新建 + 7 unit tests |
| 集成 protocol_handlers.rs | 14:50-15:00 (10 min) | send_and_decode_with_status hook + lib.rs mod |
| 集成测试 tests/ | 15:00-15:05 (5 min) | tests/test_replay_cache.rs |
| Verify + report | 15:05-15:15 (10 min) | cargo build + cargo test + 3 报告 |

## 8. 完成定义 (DoD)

- [x] Readmap 报告 (本文件)
- [ ] `replay_cache.rs` ~150-250 行, 字段 1:1 翻译 VCP ResponseReplayCache
- [ ] 7 unit tests in `mod replay_cache_tests` (spec 要求 5+)
- [ ] 集成到 `send_and_decode_with_status` (0 改 dispatch 签名)
- [ ] `lib.rs` 加 `pub mod replay_cache;`
- [ ] 2+ integration tests in `tests/test_replay_cache.rs`
- [ ] `cargo build -p apeireth-api` 0 error
- [ ] `cargo test -p apeireth-api --lib replay_cache_tests` 5+ passed
- [ ] `cargo test --workspace` 0 failed
- [ ] 0 触碰 24 LOCKED, 0 改 workspace.version, 0 改 R11 baseline
- [ ] Stage + Final + Decision Log 3 报告
