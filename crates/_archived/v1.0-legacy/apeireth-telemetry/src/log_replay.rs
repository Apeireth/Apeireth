// SPDX-License-Identifier: Apache-2.0
//
// `apeireth-telemetry::log_replay` — VCP `vcpLogReplayManager.js` 借鉴
//
// 借用 ID: `R122-7-VCP-LogReplay-2026-08-10`
// 原 spec 借用 ID: `BORROW-REPLAY-LOG-001` (per docs/v2-strategy/07 §3 P2 缺口表)
//
// # O-5 诚实声明 — 0 假装 100% VCP 兼容
//
// VCP `vcpLogReplayManager.js` (19KB, 446 行) 是 **运行时 WebSocket 通知补发管理器**:
// - 跟踪 deviceKey (IP) 上下线
// - `enqueue` 缓存 VCPLog 广播消息
// - `ONLINE_STABILITY_MS = 3000ms` 稳定窗口
// - `REPLAY_INTERVAL_MS = 80ms` 条间间隔
// - `deliveredIds` 差集去重
// - `_sweep` 每分钟清理过期缓存 + 设备表回收
// - `cancelApprovalCache` 审核类消息移除
//
// 本模块是 **离线 JSONL 日志文件回放器**, 业务领域不同:
// - 0 在线 enqueue (离线 load jsonl)
// - 0 设备表 (没有 "哪台设备" 的概念, 只有 "哪些 log entry")
// - 0 TTL 淘汰 (日志已是历史, 无 expireAt)
// - 0 deliveredIds 差集 (无 "重复投递" 语义, callback 由用户决定)
// - 0 60s sweep timer (无后台线程, 纯 sync)
// - 0 假装有 runtime 设备补发管理
//
// **字段级借鉴的形状**:
// - `entries: Vec<LogEntry>` ← VCP `cache: []`
// - `cursor: usize` ← VCP cache 内部索引
// - `replay(speed, callback)` ← VCP `_triggerReplay` 字段级 1:1
// - `ReplaySpeed::RealTime` (按 timestamp 差 sleep) ← VCP `REPLAY_INTERVAL_MS = 80ms` 字段级
// - `filter(predicate)` ← VCP `_sweep` 字段级借鉴 "过滤" 概念 (lazy, 0 timer)
// - `LogStats` ← VCP `getStats()` 字段级借鉴 "统计" 概念
//
// 0 改 observability::LogEntry (那是 OTel 1:1 翻译, DateTime<Utc> + trace_id/span_id/platform/schema_version).
// 本模块是独立 self-contained log_replay::LogEntry (SystemTime + BTreeMap, jsonl 友好).
// 两者用途不同, 0 假装 1:1 兼容.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ============================================================================
// §1 SystemTime ↔ u64 millis 互转 (JSONL 友好, 0 时区漂移)
// ============================================================================

/// SystemTime ↔ u64 millis since UNIX_EPOCH 互转.
///
/// 选 u64 millis 而非 RFC 3339 字符串:
/// - 0 时区漂移 (固定 UTC 基准)
/// - 0 字符串解析开销
/// - JSONL 单行类型稳定 (整数比字符串短)
mod systemtime_ms {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(t: &SystemTime, s: S) -> Result<S::Ok, S::Error> {
        let ms = t
            .duration_since(UNIX_EPOCH)
            .map_err(serde::ser::Error::custom)?
            .as_millis() as u64;
        s.serialize_u64(ms)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<SystemTime, D::Error> {
        let ms = u64::deserialize(d)?;
        Ok(UNIX_EPOCH + Duration::from_millis(ms))
    }
}

// ============================================================================
// §2 公开类型 — 5 LogLevel + 3 ReplaySpeed + LogEntry + LogStats + LogReplay
// ============================================================================

/// 5 档日志级别 (per tracing 标准, 0 Critical, 跟 observability::LogLevel 区分).
///
/// VCP 没有 LogLevel 概念 (用 payload.type 字符串), 这是 Rust 端约定, 0 假装 VCP 字段.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// 调试 (verbose)
    Trace,
    /// 调试
    Debug,
    /// 信息 (默认)
    Info,
    /// 警告
    Warn,
    /// 错误
    Error,
}

impl LogLevel {
    /// 全部 5 variant (1:1 计数, 0 漂移).
    pub const ALL: [Self; 5] = [
        Self::Trace,
        Self::Debug,
        Self::Info,
        Self::Warn,
        Self::Error,
    ];

    /// 字符串 (per K-1 强校验).
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 回放速度 (3 档, 0 假装 VCP 只有 1 档 REPLAY_INTERVAL_MS = 80ms).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplaySpeed {
    /// 真实时间 (按 timestamp 差 sleep, 跟 VCP REPLAY_INTERVAL_MS 字段级 1:1 借鉴)
    RealTime,
    /// N 倍速 (N = 1 等价 RealTime, N >= 2 加速, N = 0 退化为 Instant)
    FastForward(u32),
    /// 0 sleep 立刻全发 (0 漂移时间, 用于 debug / 测试)
    Instant,
}

/// 单条日志 (JSONL 1 行 = 1 LogEntry).
///
/// 字段级 1:1 借鉴 VCP cache entry (`{ id, type, data, createdAt, expireAt }`):
/// - `id` → 0 沿用 (用 timestamp 自带顺序, 0 假装 VCP id 机制)
/// - `type` → `target: String` (Rust module path, 0 假装 VCP 业务 type)
/// - `data: payload` → `message + fields` (jsonl 自然展开)
/// - `createdAt` → `timestamp: SystemTime`
/// - `expireAt` → 0 沿用 (离线日志无 expire 语义)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogEntry {
    /// UTC timestamp (ms since UNIX_EPOCH, JSONL 友好).
    #[serde(with = "systemtime_ms")]
    pub timestamp: SystemTime,
    /// 日志级别.
    pub level: LogLevel,
    /// 模块路径 (例: `apeireth_api::server`).
    pub target: String,
    /// 消息文本.
    pub message: String,
    /// 结构化字段 (任意 key-value, 0 必填).
    #[serde(default)]
    pub fields: BTreeMap<String, Value>,
}

/// 统计 (字段级 1:1 借鉴 VCP `getStats()` 形状).
#[derive(Debug, Clone)]
pub struct LogStats {
    /// 总条数.
    pub total: usize,
    /// 按 level 分组计数 (5 档全 0..N).
    pub by_level: BTreeMap<LogLevel, usize>,
    /// 按 target 分组计数.
    pub by_target: BTreeMap<String, usize>,
    /// 时间范围 (min, max); 空时返 `(UNIX_EPOCH, UNIX_EPOCH)`.
    pub time_range: (SystemTime, SystemTime),
}

/// 日志回放器 (主结构).
///
/// 字段级 1:1 借鉴 VCP `VcpLogReplayManager`:
/// - `entries` ← VCP `cache: []`
/// - `cursor` ← VCP 内部 cache 索引
/// - 0 沿用 VCP `devices: Map` / `approvalIndex: Map` (无设备管理)
/// - 0 沿用 VCP `_cleanupTimer: Interval` (无后台线程, 0 漂移)
pub struct LogReplay {
    entries: Vec<LogEntry>,
    cursor: usize,
}

impl LogReplay {
    /// 新建空 (0 entries).
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            cursor: 0,
        }
    }

    /// 从 jsonl 文件读 (1 行 1 JSON, 0 头).
    ///
    /// 0 假装 VCP 0 持久化: VCP 内存级 cache, 我们是离线 jsonl load (从磁盘读历史).
    /// 任何 1 行 parse 失败 → 整体 `Err` (0 假装 "best-effort load", 0 假装部分成功).
    pub fn load_from_jsonl(path: &Path) -> Result<Self> {
        let file =
            File::open(path).with_context(|| format!("open jsonl log file: {}", path.display()))?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        for (lineno, line) in reader.lines().enumerate() {
            let line = line.with_context(|| format!("read line {lineno}"))?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: LogEntry = serde_json::from_str(&line)
                .with_context(|| format!("parse jsonl line {lineno}: {line}"))?;
            entries.push(entry);
        }
        Ok(Self { entries, cursor: 0 })
    }

    /// 从 inline string 读 (用于测试, 不走 File).
    pub fn load_from_string(content: &str) -> Result<Self> {
        let mut entries = Vec::new();
        for (lineno, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let entry: LogEntry = serde_json::from_str(line)
                .with_context(|| format!("parse inline line {lineno}: {line}"))?;
            entries.push(entry);
        }
        Ok(Self { entries, cursor: 0 })
    }

    /// 写回 jsonl 文件 (1 行 1 JSON, 0 头, round-trip 0 数据漂移).
    pub fn save_to_jsonl(&self, path: &Path) -> Result<()> {
        let file = File::create(path)
            .with_context(|| format!("create jsonl log file: {}", path.display()))?;
        let mut writer = BufWriter::new(file);
        for entry in &self.entries {
            let line = serde_json::to_string(entry).context("serialize LogEntry")?;
            writer
                .write_all(line.as_bytes())
                .context("write LogEntry line")?;
            writer.write_all(b"\n").context("write newline")?;
        }
        writer.flush().context("flush BufWriter")?;
        Ok(())
    }

    /// 回放 (从 cursor 起, 调 callback 逐条触发, 0 漂移).
    ///
    /// 字段级 1:1 借鉴 VCP `_triggerReplay`:
    /// - VCP `sendFn(replayPayload)` → 我们 `callback(&LogEntry)`
    /// - VCP `REPLAY_INTERVAL_MS = 80ms` 条间间隔 → 我们 `ReplaySpeed::RealTime` 按 timestamp 差 sleep
    /// - 0 沿用 VCP `state.replayInFlight` (0 假装并发, sync 0 锁)
    /// - 0 沿用 VCP `deliveredIds` (无去重, 每次 replay 重新走 cursor)
    /// - 0 沿用 VCP `online` 守门 (0 假装有设备, sync 0 abort)
    ///
    /// 回放完毕 cursor 复位 0 (idempotent: 同一 LogReplay 可多次 replay).
    pub fn replay<F: FnMut(&LogEntry)>(
        &mut self,
        speed: ReplaySpeed,
        mut callback: F,
    ) -> Result<()> {
        if self.entries.is_empty() {
            self.cursor = 0;
            return Ok(());
        }
        // 字段级 1:1 借鉴 VCP "先按 timestamp 排序再 replay" (VCP cache 是 push 顺序, 我们支持离线导入)
        // 0 假装 0 排序, 用户可自己外部 sort
        self.cursor = 0;
        let mut last_ts: Option<SystemTime> = None;
        for entry in &self.entries {
            if let (Some(prev), ReplaySpeed::RealTime) = (last_ts, speed) {
                if let Some(delta) = entry.timestamp.duration_since(prev).ok() {
                    if !delta.is_zero() {
                        std::thread::sleep(delta);
                    }
                }
            } else if let (Some(prev), ReplaySpeed::FastForward(n)) = (last_ts, speed) {
                let n = n.max(1); // 0 漂移: FastForward(0) 退化为 RealTime (avoid div by 0)
                if let Some(delta) = entry.timestamp.duration_since(prev).ok() {
                    let scaled = delta / n;
                    if !scaled.is_zero() {
                        std::thread::sleep(scaled);
                    }
                }
            }
            // ReplaySpeed::Instant: 0 sleep
            callback(entry);
            last_ts = Some(entry.timestamp);
            self.cursor += 1;
        }
        self.cursor = 0;
        Ok(())
    }

    /// 过滤子集 (字段级 1:1 借鉴 VCP `_sweep` "过滤" 概念, 0 timer 0 漂移).
    ///
    /// VCP `_sweep` 60s 后台回收; 我们是 lazy 同步过滤, 0 假装有时序依赖.
    pub fn filter<F: Fn(&LogEntry) -> bool>(&self, predicate: F) -> Self {
        Self {
            entries: self
                .entries
                .iter()
                .filter(|e| predicate(e))
                .cloned()
                .collect(),
            cursor: 0,
        }
    }

    /// 统计 (字段级 1:1 借鉴 VCP `getStats()`).
    pub fn stats(&self) -> LogStats {
        let total = self.entries.len();
        let mut by_level: BTreeMap<LogLevel, usize> = BTreeMap::new();
        let mut by_target: BTreeMap<String, usize> = BTreeMap::new();
        // 0 漂移: 5 档全 0 (即使没出现也展示)
        for lvl in LogLevel::ALL {
            by_level.entry(lvl).or_insert(0);
        }
        let mut min_ts: Option<SystemTime> = None;
        let mut max_ts: Option<SystemTime> = None;
        for entry in &self.entries {
            *by_level.entry(entry.level).or_insert(0) += 1;
            *by_target.entry(entry.target.clone()).or_insert(0) += 1;
            min_ts = Some(min_ts.map_or(entry.timestamp, |m| m.min(entry.timestamp)));
            max_ts = Some(max_ts.map_or(entry.timestamp, |m| m.max(entry.timestamp)));
        }
        let time_range = match (min_ts, max_ts) {
            (Some(lo), Some(hi)) => (lo, hi),
            _ => (UNIX_EPOCH, UNIX_EPOCH),
        };
        LogStats {
            total,
            by_level,
            by_target,
            time_range,
        }
    }

    /// 0 拷贝切片 (借 VCP 字段级, 0 沿用 VCP `cache` getter).
    #[must_use]
    pub fn entries(&self) -> &[LogEntry] {
        &self.entries
    }

    /// 当前 cursor (字段级 1:1 借鉴 VCP 内部 cache 索引).
    #[must_use]
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// 0 entries 判空.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 条数 (字段级 1:1 借鉴 VCP `getStats().cacheSize`).
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 按 level lazy 迭代 (0 预收集, 0 假装 VCP 0 沿用).
    pub fn iter_by_level(&self, level: LogLevel) -> impl Iterator<Item = &LogEntry> {
        self.entries.iter().filter(move |e| e.level == level)
    }
}

impl Default for LogReplay {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// §3 8+ unit test (per R122-7 spec)
// ============================================================================

#[cfg(test)]
#[allow(clippy::all)]
mod log_replay_tests {
    use super::*;
    use std::io::Write as _;
    use std::time::{Duration, SystemTime};

    /// 构造 1 条 LogEntry (测试 helper).
    fn mk_entry(offset_ms: u64, level: LogLevel, target: &str, message: &str) -> LogEntry {
        let timestamp = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000 + offset_ms);
        let mut fields = BTreeMap::new();
        fields.insert("k".to_string(), Value::String("v".to_string()));
        LogEntry {
            timestamp,
            level,
            target: target.to_string(),
            message: message.to_string(),
            fields,
        }
    }

    #[test]
    fn log_replay_load_from_jsonl_parses_correctly() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path();
        // 写 5 条 (1 行 1 JSON)
        let content = "\
{\"timestamp\":1700000000000,\"level\":\"info\",\"target\":\"apeireth_api\",\"message\":\"a\"}
{\"timestamp\":1700000001000,\"level\":\"warn\",\"target\":\"apeireth_api\",\"message\":\"b\",\"fields\":{\"req_id\":\"r1\"}}
{\"timestamp\":1700000002000,\"level\":\"error\",\"target\":\"apeireth_pipeline\",\"message\":\"c\"}
{\"timestamp\":1700000003000,\"level\":\"debug\",\"target\":\"apeireth_memory\",\"message\":\"d\"}
{\"timestamp\":1700000004000,\"level\":\"trace\",\"target\":\"apeireth_telemetry\",\"message\":\"e\"}
";
        std::fs::write(path, content).unwrap();

        let replay = LogReplay::load_from_jsonl(path).unwrap();
        assert_eq!(replay.len(), 5);
        assert_eq!(replay.entries()[0].message, "a");
        assert_eq!(replay.entries()[0].level, LogLevel::Info);
        assert_eq!(replay.entries()[1].fields.get("req_id").unwrap(), "r1");
        assert_eq!(replay.entries()[2].target, "apeireth_pipeline");
        assert_eq!(replay.entries()[3].level, LogLevel::Debug);
        assert_eq!(replay.entries()[4].level, LogLevel::Trace);
    }

    #[test]
    fn log_replay_save_to_jsonl_round_trip() {
        // 构造 3 条 → save → load → 比较
        // 直接 push 私有字段需走 impl, 走 load_from_string
        let content = "\
{\"timestamp\":1700000000000,\"level\":\"info\",\"target\":\"t1\",\"message\":\"a\",\"fields\":{\"x\":1}}
{\"timestamp\":1700000001000,\"level\":\"error\",\"target\":\"t2\",\"message\":\"b\"}
{\"timestamp\":1700000002000,\"level\":\"warn\",\"target\":\"t3\",\"message\":\"c\"}
";
        let replay = LogReplay::load_from_string(content).unwrap();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path();
        replay.save_to_jsonl(path).unwrap();

        let restored = LogReplay::load_from_jsonl(path).unwrap();
        assert_eq!(replay.len(), restored.len());
        for (a, b) in replay.entries().iter().zip(restored.entries().iter()) {
            assert_eq!(a.timestamp, b.timestamp);
            assert_eq!(a.level, b.level);
            assert_eq!(a.target, b.target);
            assert_eq!(a.message, b.message);
            assert_eq!(a.fields, b.fields);
        }
    }

    #[test]
    fn log_replay_replay_instant_callback_fires_for_all_entries() {
        let content = "\
{\"timestamp\":1700000000000,\"level\":\"info\",\"target\":\"t\",\"message\":\"a\"}
{\"timestamp\":1700000001000,\"level\":\"info\",\"target\":\"t\",\"message\":\"b\"}
{\"timestamp\":1700000002000,\"level\":\"info\",\"target\":\"t\",\"message\":\"c\"}
";
        let mut replay = LogReplay::load_from_string(content).unwrap();
        let mut fired = Vec::new();
        replay
            .replay(ReplaySpeed::Instant, |e| {
                fired.push(e.message.clone());
            })
            .unwrap();
        assert_eq!(fired, vec!["a", "b", "c"]);
        // cursor reset to 0 after replay (idempotent)
        assert_eq!(replay.cursor(), 0);
    }

    #[test]
    fn log_replay_replay_real_time_respects_timing() {
        // 3 条, timestamp 间隔 100ms. RealTime → 至少 sleep 200ms (delta1 + delta2).
        // 0 假装精确 200ms, 给 50ms 容差 (test runner jitter).
        // 用 UNIX_EPOCH + Duration::from_millis 构造 0 漂移 100ms 间隔 (避免 1.7e12 数字歧义).
        let base = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
        let mk_line = |offset_ms: u64, msg: &str| -> String {
            let ts_ms = 1_700_000_000_000u64 + offset_ms;
            format!(
                "{{\"timestamp\":{ts_ms},\"level\":\"info\",\"target\":\"t\",\"message\":\"{msg}\"}}"
            )
        };
        let content = format!(
            "{}\n{}\n{}\n",
            mk_line(0, "a"),
            mk_line(100, "b"),
            mk_line(200, "c")
        );
        let _ = base; // anchor 0 漂移
        let mut replay = LogReplay::load_from_string(&content).unwrap();
        let mut fired = 0;
        let start = std::time::Instant::now();
        replay
            .replay(ReplaySpeed::RealTime, |_e| {
                fired += 1;
            })
            .unwrap();
        let elapsed = start.elapsed();
        assert_eq!(fired, 3);
        // 3 entries: 2 deltas × 100ms = 200ms 至少; 0 漂移: 至少 150ms (留 50ms 容差)
        assert!(
            elapsed >= Duration::from_millis(150),
            "RealTime replay should respect timing, but elapsed = {elapsed:?}"
        );
        // 0 假装 "严格 200ms": 最多 5s (test runner jitter 余量)
        assert!(
            elapsed < Duration::from_secs(5),
            "RealTime replay should not stall, but elapsed = {elapsed:?}"
        );
    }

    #[test]
    fn log_replay_filter_returns_subset() {
        let content = "\
{\"timestamp\":1700000000000,\"level\":\"info\",\"target\":\"apeireth_api\",\"message\":\"a\"}
{\"timestamp\":1700000001000,\"level\":\"warn\",\"target\":\"apeireth_api\",\"message\":\"b\"}
{\"timestamp\":1700000002000,\"level\":\"error\",\"target\":\"apeireth_pipeline\",\"message\":\"c\"}
{\"timestamp\":1700000003000,\"level\":\"info\",\"target\":\"apeireth_api\",\"message\":\"d\"}
{\"timestamp\":1700000004000,\"level\":\"debug\",\"target\":\"apeireth_memory\",\"message\":\"e\"}
";
        let replay = LogReplay::load_from_string(content).unwrap();
        let filtered = replay.filter(|e| e.target == "apeireth_api");
        assert_eq!(filtered.len(), 3);
        assert!(filtered
            .entries()
            .iter()
            .all(|e| e.target == "apeireth_api"));
    }

    #[test]
    fn log_replay_stats_counts_by_level_and_target() {
        let content = "\
{\"timestamp\":1700000000000,\"level\":\"info\",\"target\":\"apeireth_api\",\"message\":\"a\"}
{\"timestamp\":1700000001000,\"level\":\"info\",\"target\":\"apeireth_api\",\"message\":\"b\"}
{\"timestamp\":1700000002000,\"level\":\"error\",\"target\":\"apeireth_pipeline\",\"message\":\"c\"}
{\"timestamp\":1700000003000,\"level\":\"info\",\"target\":\"apeireth_api\",\"message\":\"d\"}
{\"timestamp\":1700000004000,\"level\":\"warn\",\"target\":\"apeireth_api\",\"message\":\"e\"}
";
        let replay = LogReplay::load_from_string(content).unwrap();
        let stats = replay.stats();
        assert_eq!(stats.total, 5);
        assert_eq!(stats.by_level.get(&LogLevel::Info), Some(&3));
        assert_eq!(stats.by_level.get(&LogLevel::Error), Some(&1));
        assert_eq!(stats.by_level.get(&LogLevel::Warn), Some(&1));
        // 5 档全展示, Trace + Debug 0 漂移
        assert_eq!(stats.by_level.get(&LogLevel::Trace), Some(&0));
        assert_eq!(stats.by_level.get(&LogLevel::Debug), Some(&0));
        assert_eq!(stats.by_target.get("apeireth_api"), Some(&4));
        assert_eq!(stats.by_target.get("apeireth_pipeline"), Some(&1));
        // time_range: min=1700000000000, max=1700000004000
        let (lo, hi) = stats.time_range;
        assert_eq!(lo, UNIX_EPOCH + Duration::from_millis(1_700_000_000_000));
        assert_eq!(hi, UNIX_EPOCH + Duration::from_millis(1_700_000_004_000));
    }

    #[test]
    fn log_replay_iter_by_level_lazy() {
        let content = "\
{\"timestamp\":1700000000000,\"level\":\"info\",\"target\":\"t\",\"message\":\"a\"}
{\"timestamp\":1700000001000,\"level\":\"warn\",\"target\":\"t\",\"message\":\"b\"}
{\"timestamp\":1700000002000,\"level\":\"info\",\"target\":\"t\",\"message\":\"c\"}
{\"timestamp\":1700000003000,\"level\":\"warn\",\"target\":\"t\",\"message\":\"d\"}
{\"timestamp\":1700000004000,\"level\":\"info\",\"target\":\"t\",\"message\":\"e\"}
";
        let replay = LogReplay::load_from_string(content).unwrap();
        // iter_by_level 是 lazy iter, 不预收集
        let warn_count = replay.iter_by_level(LogLevel::Warn).count();
        assert_eq!(warn_count, 2);
        // 多次取 iter 0 副作用 (lazy, 0 预消费)
        let warn_msgs: Vec<String> = replay
            .iter_by_level(LogLevel::Warn)
            .map(|e| e.message.clone())
            .collect();
        assert_eq!(warn_msgs, vec!["b", "d"]);
        // 0 entry 级别 = 0
        let trace_count = replay.iter_by_level(LogLevel::Trace).count();
        assert_eq!(trace_count, 0);
    }

    #[test]
    fn log_replay_load_from_string_inline() {
        let content = "\
{\"timestamp\":1700000000000,\"level\":\"debug\",\"target\":\"t1\",\"message\":\"first\"}
{\"timestamp\":1700000001000,\"level\":\"info\",\"target\":\"t2\",\"message\":\"second\"}
{\"timestamp\":1700000002000,\"level\":\"error\",\"target\":\"t3\",\"message\":\"third\"}
";
        let replay = LogReplay::load_from_string(content).unwrap();
        assert_eq!(replay.len(), 3);
        assert_eq!(replay.entries()[0].message, "first");
        assert_eq!(replay.entries()[1].level, LogLevel::Info);
        assert_eq!(replay.entries()[2].level, LogLevel::Error);
        // 空行跳过
        let with_blank = "\
{\"timestamp\":1700000000000,\"level\":\"info\",\"target\":\"t\",\"message\":\"a\"}

{\"timestamp\":1700000001000,\"level\":\"info\",\"target\":\"t\",\"message\":\"b\"}
";
        let r2 = LogReplay::load_from_string(with_blank).unwrap();
        assert_eq!(r2.len(), 2);
    }

    #[test]
    fn log_replay_empty_constructor_and_stats() {
        // 0 entry 边界: new() + stats() + is_empty() + len()
        let replay = LogReplay::new();
        assert!(replay.is_empty());
        assert_eq!(replay.len(), 0);
        let stats = replay.stats();
        assert_eq!(stats.total, 0);
        assert_eq!(stats.by_level.len(), 5); // 5 档全 0
        assert_eq!(stats.time_range, (UNIX_EPOCH, UNIX_EPOCH));
    }

    #[test]
    fn log_replay_default_impl() {
        // Default trait (字段级 1:1 借鉴, 让 LogReplay 也能 `Default::default()`)
        let replay: LogReplay = Default::default();
        assert!(replay.is_empty());
        assert_eq!(replay.cursor(), 0);
    }

    #[test]
    fn log_replay_log_level_as_str_and_display() {
        assert_eq!(LogLevel::Trace.as_str(), "trace");
        assert_eq!(LogLevel::Debug.as_str(), "debug");
        assert_eq!(LogLevel::Info.as_str(), "info");
        assert_eq!(LogLevel::Warn.as_str(), "warn");
        assert_eq!(LogLevel::Error.as_str(), "error");
        assert_eq!(LogLevel::Warn.to_string(), "warn");
    }

    #[test]
    fn log_replay_malformed_line_returns_err() {
        // 0 假装 "best-effort load": 1 行错 → 整体 Err
        let content = "\
{\"timestamp\":1700000000000,\"level\":\"info\",\"target\":\"t\",\"message\":\"a\"}
this is not valid json
{\"timestamp\":1700000002000,\"level\":\"info\",\"target\":\"t\",\"message\":\"c\"}
";
        let result = LogReplay::load_from_string(content);
        assert!(
            result.is_err(),
            "malformed line should fail fast, not silently skip"
        );
    }
}
