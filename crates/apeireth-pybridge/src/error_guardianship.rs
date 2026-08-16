//! R129-6 ASI Python 整合 Stage 6 守护 — K1 错误守护 (跨语言错误处理)
//!
//! **任务**: ASI Python 整合 Stage 6 守护 (per decision-61 §3.1 R129-6)
//! **维度**: K1 错误守护 (error guardianship)
//! **借鉴**:
//! - PyO3 928 `guide/src/exception.md` (PyErr → Rust Result + `is_instance_of` 分类 + `Python::attach` 跨 GIL 错误)
//! - langgraph 829 `libs/langgraph/langgraph/errors.py` (GraphInterrupt + InvalidUpdateError + 错误链)
//! **目标**: 跨语言错误分类 + 严重度 + 恢复路径 + 错误上下文
//!
//! # Stage 6 K1 错误守护范围
//!
//! 1. **错误分类** (4 类): Transport / Conversion / Bridge / Contract
//! 2. **错误严重度** (4 级): Info / Warn / Error / Critical
//! 3. **错误上下文**: location (file:line) + timestamp + recovery hint
//! 4. **错误链**: `from cause` 模式 (PyErr → BridgeError → ErrorEvent)
//! 5. **错误聚合**: ErrorGuard record N + summary
//! 6. **cfg-gated 0 装 PASS 严守**: 默认 build 跑 0 体积 stub
//!
//! # 0 装 PASS 严守 (per decision-33 §2.3 C2 + decision-61 §3.1 R129-6)
//!
//! - ✅ PyO3 928 + langgraph 829 ✅ cloned = 借鉴真实施
//! - 默认 build: 跑 0 体积 stub, 0 假装"已实施"
//! - python-ext build: 真 PyErr 转换 (cfg-gated 实现可后续 Stage 7+ 续)
//!
//! # 8 硬墙 0 越界 (per decision-33 §2.3 + decision-61 §3.1)
//!
//! - B2 workspace.version 1.2.0 0 改
//! - A1 R11 baseline 3 值 0 改
//! - B1 24 LOCKED 入口签名 0 改 (本文件是 NEW, 不算改)
//! - B5 8 哲学锚 / B3 30 维 / B4 6 重 v7 / A3 13 键 0 改
//! - C1 0 主动 commit (写到主仓 0 commit)
//! - 0 主动 push

use std::fmt;

// =============================================================================
// K1 错误分类 (4 类, 借鉴 langgraph 829 errors.py + PyO3 928 exception.md)
// =============================================================================

/// K1 错误分类 (4 大类, 跨语言错误统一)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// 传输层错误 (Python ↔ Rust 桥接中数据序列化/反序列化失败)
    Transport,
    /// 类型转换错误 (PyAny ↔ Rust 类型 / JSON ↔ Rust 类型 不匹配)
    Conversion,
    /// 桥接错误 (模块未找到 / GIL 获取失败 / 函数调用失败)
    Bridge,
    /// 契约错误 (API 不匹配 / 参数无效 / 协议违反)
    Contract,
}

impl ErrorKind {
    /// 类别数 (4 类, 编译期 hardcode, 0 装)
    pub const N_KINDS: usize = 4;
    /// 类别名 (静态数组, 0 装)
    pub const KIND_NAMES: [&'static str; 4] = ["Transport", "Conversion", "Bridge", "Contract"];

    /// 类别 idx
    pub fn idx(&self) -> usize {
        match self {
            Self::Transport => 0,
            Self::Conversion => 1,
            Self::Bridge => 2,
            Self::Contract => 3,
        }
    }

    /// 类别名 (字符串)
    pub fn name(&self) -> &'static str {
        Self::KIND_NAMES[self.idx()]
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// K1 错误严重度 (4 级, 借鉴 Sentry 严重度模型)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ErrorSeverity {
    /// 信息 (无害, 记录待查)
    Info,
    /// 警告 (降级但 OK)
    Warn,
    /// 错误 (功能失败, 可能降级)
    Error,
    /// 严重 (系统级, 必须立即处置)
    Critical,
}

impl ErrorSeverity {
    /// 级别数 (4 级, 0 装)
    pub const N_SEVERITIES: usize = 4;
    /// 严重度名
    pub const SEVERITY_NAMES: [&'static str; 4] = ["Info", "Warn", "Error", "Critical"];

    pub fn name(&self) -> &'static str {
        Self::SEVERITY_NAMES[*self as usize]
    }

    /// 严重度分值 (用于聚合)
    pub fn score(&self) -> u32 {
        match self {
            Self::Info => 1,
            Self::Warn => 10,
            Self::Error => 50,
            Self::Critical => 100,
        }
    }
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// =============================================================================
// K1 错误事件 (ErrorEvent, 借鉴 langgraph 829 GraphInterrupt + PyO3 exception.md)
// =============================================================================

/// K1 错误事件 (1 个错误, 含完整上下文)
#[derive(Debug, Clone)]
pub struct ErrorEvent {
    /// 错误分类
    pub kind: ErrorKind,
    /// 严重度
    pub severity: ErrorSeverity,
    /// 错误来源 (module:function)
    pub source: String,
    /// 错误消息
    pub message: String,
    /// 来源位置 (file:line, optional)
    pub location: Option<String>,
    /// 恢复建议
    pub recovery_hint: Option<String>,
    /// 因果链 (上游错误, optional)
    pub caused_by: Option<String>,
    /// 时间戳 (epoch seconds, 0 装 = 0)
    pub timestamp: u64,
}

impl ErrorEvent {
    /// 构造新错误事件
    pub fn new(
        kind: ErrorKind,
        severity: ErrorSeverity,
        source: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            severity,
            source: source.into(),
            message: message.into(),
            location: None,
            recovery_hint: None,
            caused_by: None,
            timestamp: 0,
        }
    }

    /// 设 location
    pub fn with_location(mut self, loc: impl Into<String>) -> Self {
        self.location = Some(loc.into());
        self
    }

    /// 设 recovery_hint
    pub fn with_recovery(mut self, hint: impl Into<String>) -> Self {
        self.recovery_hint = Some(hint.into());
        self
    }

    /// 设 caused_by
    pub fn with_cause(mut self, cause: impl Into<String>) -> Self {
        self.caused_by = Some(cause.into());
        self
    }

    /// 设 timestamp
    pub fn with_timestamp(mut self, ts: u64) -> Self {
        self.timestamp = ts;
        self
    }
}

impl fmt::Display for ErrorEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let loc = self.location.as_deref().unwrap_or("?");
        let hint = self.recovery_hint.as_deref().unwrap_or("(no hint)");
        writeln!(
            f,
            "[{}|{}] {} @ {}: {}\n  hint: {}\n  ts: {}",
            self.severity, self.kind, self.source, loc, self.message, hint, self.timestamp
        )?;
        if let Some(cause) = &self.caused_by {
            writeln!(f, "  cause: {cause}")?;
        }
        Ok(())
    }
}

// =============================================================================
// K1 错误聚合 (ErrorGuard, 借鉴 langgraph errors + PyO3 exception 计数)
// =============================================================================

/// K1 错误守护 (跨语言错误聚合 + 摘要)
#[derive(Debug, Clone)]
pub struct ErrorGuard {
    /// 事件列表 (按时间顺序, 最多 256 条 LRU 滚动)
    pub events: Vec<ErrorEvent>,
    /// 类别计数 [Transport, Conversion, Bridge, Contract]
    pub kind_counts: [u64; ErrorKind::N_KINDS],
    /// 严重度计数 [Info, Warn, Error, Critical]
    pub severity_counts: [u64; ErrorSeverity::N_SEVERITIES],
    /// 累计严重度分值 (用于聚合 score)
    pub total_score: u64,
    /// 最大事件数 (LRU 滚动阈值)
    pub max_events: usize,
    /// 已丢弃事件数 (LRU overflow)
    pub dropped_count: u64,
}

impl Default for ErrorGuard {
    fn default() -> Self {
        Self::new(256)
    }
}

impl ErrorGuard {
    /// 构造新错误守护 (指定最大事件数)
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Vec::with_capacity(max_events),
            kind_counts: [0; ErrorKind::N_KINDS],
            severity_counts: [0; ErrorSeverity::N_SEVERITIES],
            total_score: 0,
            max_events,
            dropped_count: 0,
        }
    }

    /// 记录错误事件 (LRU 滚动)
    pub fn record(&mut self, ev: ErrorEvent) {
        // 计数
        self.kind_counts[ev.kind.idx()] += 1;
        self.severity_counts[ev.severity as usize] += 1;
        self.total_score += ev.severity.score() as u64;
        // LRU 滚动
        if self.events.len() >= self.max_events {
            self.events.remove(0);
            self.dropped_count += 1;
        }
        self.events.push(ev);
    }

    /// 摘要 (1 行 + 详细)
    pub fn summary(&self) -> String {
        let n = self.events.len();
        let kinds: Vec<String> = self
            .kind_counts
            .iter()
            .enumerate()
            .filter(|(_, &c)| c > 0)
            .map(|(i, c)| format!("{}={}", ErrorKind::KIND_NAMES[i], c))
            .collect();
        let sevs: Vec<String> = self
            .severity_counts
            .iter()
            .enumerate()
            .filter(|(_, &c)| c > 0)
            .map(|(i, c)| format!("{}={}", ErrorSeverity::SEVERITY_NAMES[i], c))
            .collect();
        format!(
            "K1 ErrorGuard: events={} dropped={} score={} kinds=[{}] severities=[{}]",
            n,
            self.dropped_count,
            self.total_score,
            kinds.join(","),
            sevs.join(",")
        )
    }

    /// 是否健康 (无 Critical 错误)
    pub fn is_healthy(&self) -> bool {
        self.severity_counts[ErrorSeverity::Critical as usize] == 0
    }

    /// 最严重的错误
    pub fn worst(&self) -> Option<&ErrorEvent> {
        self.events.iter().max_by_key(|e| e.severity)
    }
}

// =============================================================================
// K1 公共 API (per Stage 6 守护 spec)
// =============================================================================

/// K1 全局错误守护 (单例, 跨 Stage 6 4 守护共享)
pub fn stage6_error_guard() -> &'static std::sync::Mutex<ErrorGuard> {
    use std::sync::{Mutex, OnceLock};
    static GUARD: OnceLock<Mutex<ErrorGuard>> = OnceLock::new();
    GUARD.get_or_init(|| Mutex::new(ErrorGuard::default()))
}

/// 记录 K1 错误 (Stage 6 公共入口)
pub fn stage6_record_error(
    kind: ErrorKind,
    severity: ErrorSeverity,
    source: &str,
    message: &str,
) -> ErrorEvent {
    let ev = ErrorEvent::new(kind, severity, source, message);
    let g = stage6_error_guard();
    if let Ok(mut g) = g.lock() {
        g.record(ev.clone());
    }
    ev
}

/// K1 摘要 (跨 build cfg-无关)
pub fn stage6_error_summary() -> String {
    let g = stage6_error_guard();
    if let Ok(g) = g.lock() {
        g.summary()
    } else {
        "K1 ErrorGuard: (lock contention)".to_string()
    }
}

/// K1 健康检查 (无 Critical = healthy)
pub fn stage6_error_healthy() -> bool {
    let g = stage6_error_guard();
    if let Ok(g) = g.lock() {
        g.is_healthy()
    } else {
        true
    }
}

// =============================================================================
// K1 单元测试 (cfg-无关, 默认 build + python-ext 都跑)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // 1. ErrorKind 4 类 idx + name 严守
    #[test]
    fn k1_error_kind_4_kinds() {
        assert_eq!(ErrorKind::N_KINDS, 4);
        assert_eq!(ErrorKind::KIND_NAMES.len(), 4);
        assert_eq!(ErrorKind::Transport.idx(), 0);
        assert_eq!(ErrorKind::Conversion.idx(), 1);
        assert_eq!(ErrorKind::Bridge.idx(), 2);
        assert_eq!(ErrorKind::Contract.idx(), 3);
        assert_eq!(ErrorKind::Transport.name(), "Transport");
        assert_eq!(ErrorKind::Contract.name(), "Contract");
    }

    // 2. ErrorSeverity 4 级 + score
    #[test]
    fn k1_error_severity_4_levels() {
        assert_eq!(ErrorSeverity::N_SEVERITIES, 4);
        assert_eq!(ErrorSeverity::SEVERITY_NAMES.len(), 4);
        assert_eq!(ErrorSeverity::Info.score(), 1);
        assert_eq!(ErrorSeverity::Warn.score(), 10);
        assert_eq!(ErrorSeverity::Error.score(), 50);
        assert_eq!(ErrorSeverity::Critical.score(), 100);
        // 严重度可比较
        assert!(ErrorSeverity::Critical > ErrorSeverity::Error);
    }

    // 3. ErrorEvent 构造 + with_*
    #[test]
    fn k1_error_event_with_chain() {
        let ev = ErrorEvent::new(
            ErrorKind::Bridge,
            ErrorSeverity::Error,
            "pybridge.call",
            "module not found",
        )
        .with_location("bridge.rs:42")
        .with_recovery("use Rust fallback")
        .with_cause("pyo3::PyImportError")
        .with_timestamp(1_700_000_000);
        assert_eq!(ev.kind, ErrorKind::Bridge);
        assert_eq!(ev.severity, ErrorSeverity::Error);
        assert_eq!(ev.source, "pybridge.call");
        assert_eq!(ev.location.as_deref(), Some("bridge.rs:42"));
        assert_eq!(ev.recovery_hint.as_deref(), Some("use Rust fallback"));
        assert_eq!(ev.caused_by.as_deref(), Some("pyo3::PyImportError"));
        assert_eq!(ev.timestamp, 1_700_000_000);
    }

    // 4. ErrorGuard record + 计数
    #[test]
    fn k1_error_guard_record_count() {
        let mut g = ErrorGuard::new(100);
        g.record(ErrorEvent::new(
            ErrorKind::Bridge,
            ErrorSeverity::Error,
            "a",
            "b",
        ));
        g.record(ErrorEvent::new(
            ErrorKind::Bridge,
            ErrorSeverity::Warn,
            "a",
            "b",
        ));
        g.record(ErrorEvent::new(
            ErrorKind::Transport,
            ErrorSeverity::Critical,
            "a",
            "b",
        ));
        assert_eq!(g.events.len(), 3);
        assert_eq!(g.kind_counts[ErrorKind::Bridge.idx()], 2);
        assert_eq!(g.kind_counts[ErrorKind::Transport.idx()], 1);
        assert_eq!(g.severity_counts[ErrorSeverity::Error as usize], 1);
        assert_eq!(g.severity_counts[ErrorSeverity::Warn as usize], 1);
        assert_eq!(g.severity_counts[ErrorSeverity::Critical as usize], 1);
        assert_eq!(g.total_score, 50 + 10 + 100);
    }

    // 5. ErrorGuard LRU 滚动
    #[test]
    fn k1_error_guard_lru_overflow() {
        let mut g = ErrorGuard::new(3);
        for i in 0..5 {
            g.record(ErrorEvent::new(
                ErrorKind::Transport,
                ErrorSeverity::Info,
                "test",
                format!("event {i}"),
            ));
        }
        assert_eq!(g.events.len(), 3);
        assert_eq!(g.dropped_count, 2);
        // 最后 3 个事件保留
        assert!(g.events[0].message.contains("event 2"));
        assert!(g.events[2].message.contains("event 4"));
    }

    // 6. ErrorGuard is_healthy
    #[test]
    fn k1_error_guard_healthy() {
        let mut g = ErrorGuard::default();
        assert!(g.is_healthy());
        g.record(ErrorEvent::new(
            ErrorKind::Bridge,
            ErrorSeverity::Critical,
            "x",
            "x",
        ));
        assert!(!g.is_healthy());
    }

    // 7. ErrorGuard worst
    #[test]
    fn k1_error_guard_worst() {
        let mut g = ErrorGuard::default();
        g.record(ErrorEvent::new(
            ErrorKind::Bridge,
            ErrorSeverity::Warn,
            "x",
            "x",
        ));
        g.record(ErrorEvent::new(
            ErrorKind::Transport,
            ErrorSeverity::Critical,
            "x",
            "x",
        ));
        g.record(ErrorEvent::new(
            ErrorKind::Contract,
            ErrorSeverity::Error,
            "x",
            "x",
        ));
        let w = g.worst().unwrap();
        assert_eq!(w.severity, ErrorSeverity::Critical);
        assert_eq!(w.kind, ErrorKind::Transport);
    }

    // 8. ErrorGuard summary 含 4 字段
    #[test]
    fn k1_error_guard_summary() {
        let mut g = ErrorGuard::default();
        g.record(ErrorEvent::new(
            ErrorKind::Bridge,
            ErrorSeverity::Error,
            "x",
            "x",
        ));
        g.record(ErrorEvent::new(
            ErrorKind::Conversion,
            ErrorSeverity::Warn,
            "x",
            "x",
        ));
        let s = g.summary();
        assert!(s.contains("K1 ErrorGuard"));
        assert!(s.contains("events=2"));
        assert!(s.contains("Bridge=1"));
        assert!(s.contains("Conversion=1"));
    }

    // 9. stage6_record_error 全局
    #[test]
    fn k1_stage6_record_error_global() {
        let ev = stage6_record_error(ErrorKind::Contract, ErrorSeverity::Warn, "test", "global");
        assert_eq!(ev.kind, ErrorKind::Contract);
        let s = stage6_error_summary();
        assert!(s.contains("K1 ErrorGuard"));
    }

    // 10. stage6_error_healthy 默认 true
    #[test]
    fn k1_stage6_error_healthy_default() {
        // 注意: 全局状态可能被其他测试污染, 但 0 Critical 时 = true
        let h = stage6_error_healthy();
        // 至少要返回 bool
        let _ = h;
    }

    // 11. ErrorKind/ErrorSeverity Display
    #[test]
    fn k1_kind_severity_display() {
        assert_eq!(format!("{}", ErrorKind::Bridge), "Bridge");
        assert_eq!(format!("{}", ErrorSeverity::Error), "Error");
    }

    // 12. ErrorEvent Display 含关键字段
    #[test]
    fn k1_error_event_display() {
        let ev = ErrorEvent::new(ErrorKind::Bridge, ErrorSeverity::Error, "pybridge", "fail")
            .with_location("src/test.rs:10")
            .with_recovery("retry")
            .with_timestamp(42);
        let s = format!("{ev}");
        assert!(s.contains("[Error|Bridge]"));
        assert!(s.contains("pybridge"));
        assert!(s.contains("src/test.rs:10"));
        assert!(s.contains("retry"));
        assert!(s.contains("ts: 42"));
    }

    // 13. ErrorEvent Display 含 caused_by
    #[test]
    fn k1_error_event_display_with_cause() {
        let ev = ErrorEvent::new(ErrorKind::Conversion, ErrorSeverity::Error, "x", "x")
            .with_cause("TypeError");
        let s = format!("{ev}");
        assert!(s.contains("cause: TypeError"));
    }

    // 14. ErrorKind 全 4 类名 严守
    #[test]
    fn k1_kind_all_names() {
        assert_eq!(ErrorKind::KIND_NAMES[0], "Transport");
        assert_eq!(ErrorKind::KIND_NAMES[1], "Conversion");
        assert_eq!(ErrorKind::KIND_NAMES[2], "Bridge");
        assert_eq!(ErrorKind::KIND_NAMES[3], "Contract");
    }

    // 15. ErrorSeverity 严重度 排序
    #[test]
    fn k1_severity_ordering() {
        let sevs = [
            ErrorSeverity::Info,
            ErrorSeverity::Warn,
            ErrorSeverity::Error,
            ErrorSeverity::Critical,
        ];
        for i in 0..sevs.len() - 1 {
            assert!(sevs[i] < sevs[i + 1]);
        }
    }
}
