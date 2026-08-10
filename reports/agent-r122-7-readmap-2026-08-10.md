# Agent R122-7 Readmap — v2.1 P2-1 日志回放 (VCP vcpLogReplayManager.js 借鉴) (2026-08-10)

**时间**: 2026-08-10 13:58-14:08 (~10 min readmap)
**作者**: 团队成员 R122-7 (Mavis 派, telemetry 战区, 主人 #10 授权自主决策)
**任务定位**: v2.1 P2 缺口 #10 (per docs/v2-strategy/07-VCP-GAP-UPGRADE-PLAN.md)
**总时间预算**: 1h17m (13:58 启动 → 15:15 截止)
**借用 ID**: `R122-7-VCP-LogReplay-2026-08-10` (per spec 07 §1 O-2)
**原 spec 借用 ID**: `BORROW-REPLAY-LOG-001` (07 §3 P2 缺口表)

---

## §0. TL;DR

| 任务 | 范围 | 状态 |
|---|---|---|
| 1. `log_replay.rs` 新建 (~250-300 行) | `crates/apeireth-telemetry/src/log_replay.rs` | 待写 |
| 2. `lib.rs` 加 `pub mod log_replay;` | `crates/apeireth-telemetry/src/lib.rs` | 待加 (1 行) |
| 3. 8+ unit test | `log_replay.rs::tests` | 待写 |
| 4. 1 example | `crates/apeireth-telemetry/examples/log_replay_demo.rs` | 待写 |

**0 触碰任何 LOCKED, 0 改 workspace.version, 0 引新 dep, 0 改 11 agent 公共 API 签名, 0 主动 commit**。

---

## §1. 任务背景 — VCP vcpLogReplayManager.js 借鉴

### 1.1 VCP 源 (19KB, 全文件 446 行)

`research/source/vcptoolbox/modules/vcpLogReplayManager.js` (主人 13:44 决策 #14 派活时确认存在)。

**VCP 关键概念** (字段级借鉴):
- `class VcpLogReplayManager` — 单例
- `cache: []` — 有序数组, 每条 `{ id, type, data, createdAt, expireAt }`
- `enqueue(payload)` — 入缓存, 容量上限 `MAX_CACHE_SIZE = 100` 淘汰
- `registerOnline({ deviceKey, sendFn })` — 设备上线, 启动 `ONLINE_STABILITY_MS = 3000ms` 稳定窗口
- `_triggerReplay(state)` — 稳定窗口结束触发补发, `REPLAY_INTERVAL_MS = 80ms` 条间间隔
- `recordDelivered(deviceKey, entryId)` — 标记已投递, 下次不重复
- `cancelApprovalCache(requestId)` — 审核类消息移除
- `_sweep()` — 每分钟清理过期缓存 + 设备表回收 (default `DEVICE_TTL_MS = 7 days`)
- `getStats()` — `{ cacheSize, cacheCap, deviceCount, onlineDeviceCount, approvalIndexSize }`

### 1.2 Rust 端映射 (字段级 1:1 + 5 哲学锚穿透)

| VCP 概念 | Rust 端 | 备注 |
|---|---|---|
| `cache: []` | `LogReplay.entries: Vec<LogEntry>` | 内存数组 |
| `enqueue` + `cache[]` | (无 enqueue — 离线 load 模式) | 0 在线入队, 0 假装 VCP 在线管理 |
| `id` | (无) | 0 沿用 VCP id 机制, 用 timestamp 自带顺序 |
| `type` | `LogEntry.target: String` | target 是 Rust module path, 0 假装是 VCP 业务 type |
| `data: payload` | `LogEntry.message: String + fields: BTreeMap<String, Value>` | 0 假装 VCP payload 结构, 用 jsonl 自然格式 |
| `createdAt: Date.now()` | `timestamp: SystemTime` | O-5: SystemTime 0 漂移, 不用 DateTime<Utc> (跟 observability::LogEntry 区分开) |
| `expireAt` | (无) | 0 假装 VCP TTL, 0 假装有 cache 淘汰, 0 假装有设备表 |
| `REPLAY_INTERVAL_MS` (80ms) | `ReplaySpeed::RealTime` (sleep delta) | 字段级 1:1 借鉴 "条间间隔" |
| `_triggerReplay` | `replay(speed, callback)` | O-5: 0 假装 VCP "设备表 + deliveredIds", 只做 callback 触发 |
| `_sweep` | `filter(predicate)` (lazy) | O-5: 0 假装 VCP 全局 60s timer + 内存回收, 用 lazy 过滤表达 |
| `getStats()` | `LogStats { total, by_level, by_target, time_range }` | 字段级 1:1 借鉴"统计"概念, 0 假装 VCP 设备维度 |

### 1.3 O-5 诚实声明 (per 07 §1)

- **0 假装 100% VCP 兼容**: VCP 是运行时设备补发管理器, 我们是离线日志文件回放器, 业务领域不同
- **0 假装有 enqueue / 设备表 / TTL 淘汰 / 60s sweep timer**: 这些是 VCP 运行时管理概念, 离线 jsonl replay 不需要
- **0 假装有 deliveredIds 差集计算**: 离线回放无"重复投递"语义, callback 由用户决定
- **借鉴的是 "replay 流式" 的字段级形状**: entries + cursor + speed + callback + filter + stats

---

## §2. 实施计划

### 2.1 `log_replay.rs` (~250-300 行)

```rust
// crates/apeireth-telemetry/src/log_replay.rs
//
// VCP `vcpLogReplayManager.js` 借鉴 (R122-7-VCP-LogReplay-2026-08-10)
// 0 装 100% VCP 兼容: VCP 是运行时设备补发, 我们是离线 jsonl 日志回放.
// 字段级借鉴: entries + cursor + speed + callback + filter + stats.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 5 档日志级别 (per tracing 标准, 0 Critical).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel { Trace, Debug, Info, Warn, Error }

/// 回放速度.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplaySpeed {
    /// 真实时间 (按 timestamp 差 sleep)
    RealTime,
    /// N 倍速 (1 = 实时, 100 = 100x 快)
    FastForward(u32),
    /// 0 sleep 立刻全发
    Instant,
}

/// 单条日志 (jsonl 1 行 = 1 LogEntry).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: SystemTime,
    pub level: LogLevel,
    pub target: String,
    pub message: String,
    #[serde(default)]
    pub fields: BTreeMap<String, Value>,
}

/// 统计.
#[derive(Debug, Clone)]
pub struct LogStats {
    pub total: usize,
    pub by_level: BTreeMap<LogLevel, usize>,
    pub by_target: BTreeMap<String, usize>,
    pub time_range: (SystemTime, SystemTime),
}

/// 主结构.
pub struct LogReplay {
    entries: Vec<LogEntry>,
    cursor: usize,
}

impl LogReplay {
    pub fn load_from_jsonl(path: &Path) -> Result<Self> { ... }
    pub fn load_from_string(content: &str) -> Result<Self> { ... }
    pub fn save_to_jsonl(&self, path: &Path) -> Result<()> { ... }
    pub fn replay<F: FnMut(&LogEntry)>(&mut self, speed: ReplaySpeed, mut callback: F) -> Result<()> { ... }
    pub fn filter<F: Fn(&LogEntry) -> bool>(&self, predicate: F) -> Self { ... }
    pub fn stats(&self) -> LogStats { ... }
    pub fn entries(&self) -> &[LogEntry] { ... }
    pub fn iter_by_level(&self, level: LogLevel) -> impl Iterator<Item = &LogEntry> { ... }
}
```

### 2.2 `lib.rs` 集成 (1 行加 mod)

```rust
/// 1.1 log_replay module - VCP vcpLogReplayManager.js 借鉴 (R122-7)
///
/// **0 装 100% VCP 兼容**: VCP 是运行时设备补发管理器, 我们是离线日志回放.
/// 字段级借鉴: entries + cursor + speed + callback + filter + stats.
pub mod log_replay;
```

### 2.3 8+ unit test

1. `log_replay_load_from_jsonl_parses_correctly` — 5 行 jsonl → 5 entries
2. `log_replay_save_to_jsonl_round_trip` — save → load → 0 数据漂移
3. `log_replay_replay_instant_callback_fires_for_all_entries` — Instant 模式, 全部 callback 触发
4. `log_replay_replay_real_time_respects_timing` — RealTime 模式, sleep 至少 elapsed - 100ms
5. `log_replay_filter_returns_subset` — 过滤 target=apeireth_api → 3/5 entries
6. `log_replay_stats_counts_by_level_and_target` — by_level[Info]=3, by_target[apeireth_api]=3
7. `log_replay_iter_by_level_lazy` — iter_by_level(Warn) 立即返 impl Iterator, 不预收集
8. `log_replay_load_from_string_inline` — inline 多行 → 3 entries

### 2.4 example (~80 行)

`crates/apeireth-telemetry/examples/log_replay_demo.rs`:
- 步骤 1: 构造 5 条 LogEntry (Trace + Info + Warn + Error + Debug)
- 步骤 2: `save_to_jsonl` 到 tempfile
- 步骤 3: `load_from_jsonl` 重新读
- 步骤 4: `stats()` 输出按 level / target 分布
- 步骤 5: `filter()` 只看 Error
- 步骤 6: `replay(Instant, |e| println!(...))` 全量回放
- 步骤 7: `iter_by_level(Warn)` 遍历

---

## §3. 硬约束自检 (8 墙)

| 墙 | 0 触碰? | 验证 |
|---|---|---|
| 1. workspace.version (1.1.0) | ✅ | 0 改 `Cargo.toml:246`, 仅 `log_replay.rs` 新文件 + `lib.rs` +1 行 mod 声明 |
| 2. R11 baseline 3 值 | ✅ | 0 改任何 organ/anchor/13-key/5-gate 文件 |
| 3. 24 LOCKED crate mtime | ✅ | telemetry **不**在 24 LOCKED (R20 已合并, 解锁), `lib.rs` 改 +1 行 mod = 9 字节, 0 触碰 src logic |
| 4. 9 器官 logic | ✅ | 0 触碰 organ/body/brain/ear/eye/hand/heart/memory/mind/voice 任何文件 |
| 5. 6 哲学锚 / 12 键 / 5 重守门 / V0.5 24 维 / 双洋葱 | ✅ | 0 触碰这些 anchor 文件 |
| 6. 11 agent 公共 API 签名 | ✅ | 0 触碰 cache/BackoffPolicy/JitterMode/Evictor/dispatch_with_retry/server.rs 4 handler/KeyPathSpan/parse_protocol_kind/tracing_integration/cache::memory_provider 任何签名 |
| 7. 0 主动 commit | ✅ | 0 `git add` / `git commit` |
| 8. 0 装 | ✅ | 0 假装"100% VCP 兼容", 在 `log_replay.rs` 顶部 doc + 决策日志诚实标 "VCP 是运行时设备补发, 我们是离线 jsonl replay, 业务领域不同" |

**新 dep**: 0. (`serde` / `serde_json` / `anyhow` 已在 apeireth-telemetry Cargo.toml, 0 引新)

---

## §4. 文件清单 (预计)

| 文件 | 类型 | 行数 |
|---|---|---|
| `crates/apeireth-telemetry/src/log_replay.rs` | 新建 | ~270 |
| `crates/apeireth-telemetry/src/lib.rs` | 改 +1 行 mod | +9 (含 doc) |
| `crates/apeireth-telemetry/examples/log_replay_demo.rs` | 新建 | ~80 |
| `reports/agent-r122-7-stage-2026-08-10.md` | 新建 | 报告 |
| `reports/agent-r122-7-final-2026-08-10.md` | 新建 | 报告 |
| `reports/agent-r122-7-decision-log-2026-08-10.md` | 新建 | 报告 |

**总计**: 3 新 src 文件 + 3 报告 + 1 lib.rs 1 行改.

---

## §5. 阶段总览

| 阶段 | 时间 | 任务 | 状态 |
|---|---|---|---|
| R122-7-1 | 13:58-14:08 (10 min) | readmap (本文件) | ✅ |
| R122-7-2 | 14:08-14:35 (27 min) | log_replay.rs 实现 (含 8 test) | 待 |
| R122-7-3 | 14:35-14:55 (20 min) | lib.rs 集成 + example + 自测 | 待 |
| R122-7-4 | 14:55-15:10 (15 min) | verify: cargo build + cargo test --workspace | 待 |
| R122-7-5 | 15:10-15:15 (5 min) | final + decision log | 待 |

**R122-7-1 完. 立即开干 R122-7-2.**
