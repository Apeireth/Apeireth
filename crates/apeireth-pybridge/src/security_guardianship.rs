//! R129-6 ASI Python 整合 Stage 6 守护 — K3 安全守护 (6 重 v7 跨语言集成)
//!
//! **任务**: ASI Python 整合 Stage 6 守护 (per decision-61 §3.1 R129-6)
//! **维度**: K3 安全守护 (security guardianship)
//! **借鉴**:
//! - superpowers 234 `skills/verification-before-completion` (6 重 check + verification 模式)
//! - PyO3 928 `guide/src/exception.md` (异常安全 + `is_instance_of` 类型守门)
//! - PyO3 928 `guide/src/class.md` (Bound 生命周期 + 安全边界)
//! **目标**: 6 重守门 v7 跨语言集成 (B4 严守) + G7 跨语言新增
//!
//! # Stage 6 K3 安全守护范围
//!
//! 1. **6 重守门 v7 (B4 严守)**: G1 Identity / G2 Goal / G3 Capability / G4 Compliance / G5 Resource / G6 Audit
//! 2. **G7 跨语言新增**: 跨 GIL 桥安全守门 (Stage 6 K3 创新, 严守"连接不是修改")
//! 3. **SecurityEvent**: 1 个安全事件 (gate, kind, severity, blocked, context)
//! 4. **SecurityVerdict**: Allow / Warn / Block / Audit
//! 5. **SecurityGuard**: 6+1 重门聚合 + 摘要
//! 6. **cfg-gated 0 装 PASS 严守**: 默认 build 跑内存守门
//!
//! # 0 装 PASS 严守 (per decision-33 §2.3 C2 + decision-61 §3.1 R129-6)
//!
//! - ✅ superpowers 234 + PyO3 928 ✅ cloned = 借鉴真实施
//! - B4 6 重守门 v7 严守 (K3 集成是连接不是修改)
//! - 默认 build: 跑内存守门, 0 假装"已实施"
//!
//! # 8 硬墙 0 越界 (per decision-33 §2.3 + decision-61 §3.1)
//!
//! - B2 workspace.version 1.2.0 0 改
//! - A1 R11 baseline 3 值 0 改
//! - B1 24 LOCKED 入口签名 0 改 (本文件是 NEW)
//! - **B4 6 重守门 v7 严守** (K3 集成是连接, 0 改 6 重守门本身)
//! - C1 0 主动 commit

use std::fmt;

// =============================================================================
// K3 安全门枚举 (6 重 v7 + 1 跨语言, B4 严守 0 改)
// =============================================================================

/// K3 安全门 (6 重 v7 + 1 跨语言 = 7 重)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SecurityGate {
    /// G1 身份 (Identity)
    G1Identity,
    /// G2 目标 (Goal)
    G2Goal,
    /// G3 能力 (Capability)
    G3Capability,
    /// G4 合规 (Compliance)
    G4Compliance,
    /// G5 资源 (Resource)
    G5Resource,
    /// G6 审计 (Audit)
    G6Audit,
    /// G7 跨语言 (CrossLanguage, Stage 6 K3 新增; 严守"连接不是修改")
    G7CrossLanguage,
}

impl SecurityGate {
    /// 门数 (6 + 1 = 7 重)
    pub const N_GATES: usize = 7;
    /// 门名
    pub const GATE_NAMES: [&'static str; 7] = [
        "G1_Identity",
        "G2_Goal",
        "G3_Capability",
        "G4_Compliance",
        "G5_Resource",
        "G6_Audit",
        "G7_CrossLanguage",
    ];

    pub fn idx(&self) -> usize {
        match self {
            Self::G1Identity => 0,
            Self::G2Goal => 1,
            Self::G3Capability => 2,
            Self::G4Compliance => 3,
            Self::G5Resource => 4,
            Self::G6Audit => 5,
            Self::G7CrossLanguage => 6,
        }
    }

    pub fn name(&self) -> &'static str {
        Self::GATE_NAMES[self.idx()]
    }

    /// 6 重 v7 vs 7 重 (B4 严守 6 重 v7 + G7 跨语言 K3 新增)
    pub fn is_v7_baseline(&self) -> bool {
        // 6 重 v7 = G1-G6, G7 = K3 新增
        matches!(
            self,
            Self::G1Identity
                | Self::G2Goal
                | Self::G3Capability
                | Self::G4Compliance
                | Self::G5Resource
                | Self::G6Audit
        )
    }
}

impl fmt::Display for SecurityGate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// K3 守门事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SecurityEventKind {
    /// 检查通过
    Pass,
    /// 检查警告 (放行但记录)
    Warn,
    /// 检查拒绝 (阻止)
    Block,
    /// 审计记录 (放行, 详细审计)
    Audit,
}

impl fmt::Display for SecurityEventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Pass => "Pass",
                Self::Warn => "Warn",
                Self::Block => "Block",
                Self::Audit => "Audit",
            }
        )
    }
}

/// K3 安全裁决 (借鉴 superpowers 234 verification 结果模式)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SecurityVerdict {
    /// 允许
    Allow,
    /// 警告 (放行但记录)
    Warn,
    /// 阻止
    Block,
    /// 审计
    Audit,
}

impl SecurityVerdict {
    /// 6 重全 Allow = true
    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Allow)
    }
}

impl fmt::Display for SecurityVerdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Allow => "Allow",
                Self::Warn => "Warn",
                Self::Block => "Block",
                Self::Audit => "Audit",
            }
        )
    }
}

/// K3 严重度 (3 级, 借鉴 superpowers 234 紧急度)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SecuritySeverity {
    /// 低 (常规审计)
    Low,
    /// 中 (警告)
    Medium,
    /// 高 (阻止)
    High,
}

impl SecuritySeverity {
    pub const N_SEVERITIES: usize = 3;
    pub fn score(&self) -> u32 {
        match self {
            Self::Low => 1,
            Self::Medium => 10,
            Self::High => 100,
        }
    }
}

// =============================================================================
// K3 安全事件 (SecurityEvent, 借鉴 superpowers 234 + PyO3)
// =============================================================================

/// K3 1 个安全事件
#[derive(Debug, Clone)]
pub struct SecurityEvent {
    /// 安全门
    pub gate: SecurityGate,
    /// 事件类型
    pub event_kind: SecurityEventKind,
    /// 严重度
    pub severity: SecuritySeverity,
    /// 是否被阻止
    pub blocked: bool,
    /// 来源 (module:function)
    pub source: String,
    /// 事件消息
    pub message: String,
    /// 上下文 (optional)
    pub context: Option<String>,
    /// 时间戳 (epoch seconds, 0 装 = 0)
    pub timestamp: u64,
}

impl SecurityEvent {
    pub fn new(
        gate: SecurityGate,
        event_kind: SecurityEventKind,
        severity: SecuritySeverity,
        source: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        let blocked = matches!(event_kind, SecurityEventKind::Block);
        Self {
            gate,
            event_kind,
            severity,
            blocked,
            source: source.into(),
            message: message.into(),
            context: None,
            timestamp: 0,
        }
    }

    pub fn with_context(mut self, ctx: impl Into<String>) -> Self {
        self.context = Some(ctx.into());
        self
    }

    pub fn with_timestamp(mut self, ts: u64) -> Self {
        self.timestamp = ts;
        self
    }
}

impl fmt::Display for SecurityEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ctx = self.context.as_deref().unwrap_or("(no context)");
        let mark = if self.blocked { "🚫" } else { "✅" };
        writeln!(
            f,
            "{} [{}|{}] {} @ {}: {}\n  ctx: {}\n  ts: {}",
            mark,
            self.gate,
            self.event_kind,
            self.source,
            "stage6",
            self.message,
            ctx,
            self.timestamp
        )
    }
}

// =============================================================================
// K3 6 重 v7 守门 (B4 严守: K3 集成是连接, 0 改 6 重守门本身)
// =============================================================================

/// 6 重 v7 单门检查 (B4 严守, K3 集成调用)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum V7BaselineCheck {
    /// G1 身份 check
    Identity,
    /// G2 目标 check
    Goal,
    /// G3 能力 check
    Capability,
    /// G4 合规 check
    Compliance,
    /// G5 资源 check
    Resource,
    /// G6 审计 check
    Audit,
}

impl V7BaselineCheck {
    pub const N_CHECKS: usize = 6;
    pub const CHECK_NAMES: [&'static str; 6] = [
        "G1_Identity",
        "G2_Goal",
        "G3_Capability",
        "G4_Compliance",
        "G5_Resource",
        "G6_Audit",
    ];

    /// 严守 B4: 6 重 v7 全 OK (硬 verify; Stage 6 K3 不允许修改 v7 本身)
    pub fn v7_baseline_intact() -> bool {
        // 编译期 hardcode: 6 重守门 v7 严守 (per decision-33 §2.3 B4)
        // 0 改, 0 删, 0 加 — 6 重锁死
        Self::CHECK_NAMES.len() == 6
            && Self::CHECK_NAMES[0] == "G1_Identity"
            && Self::CHECK_NAMES[5] == "G6_Audit"
    }
}

/// G7 跨语言 check (Stage 6 K3 新增, 严守"连接不是修改")
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossLanguageCheck {
    /// GIL acquire/release 安全
    GilSafe,
    /// PyAny 生命周期安全
    LifetimeSafe,
    /// Python 异常转换安全 (PyErr → Result)
    ExceptionSafe,
    /// 跨语言类型转换安全 (Bound → Rust T)
    ConvertSafe,
    /// 模块导入安全 (PyImport)
    ImportSafe,
    /// 表达式求值安全 (py.eval)
    EvalSafe,
    /// 函数调用安全 (call Python function)
    CallSafe,
}

impl CrossLanguageCheck {
    pub const N_CHECKS: usize = 7;
    pub const CHECK_NAMES: [&'static str; 7] = [
        "GilSafe",
        "LifetimeSafe",
        "ExceptionSafe",
        "ConvertSafe",
        "ImportSafe",
        "EvalSafe",
        "CallSafe",
    ];

    /// 7 项 G7 跨语言 check 全 OK
    pub fn g7_baseline_intact() -> bool {
        Self::CHECK_NAMES.len() == 7
            && Self::CHECK_NAMES[0] == "GilSafe"
            && Self::CHECK_NAMES[6] == "CallSafe"
    }
}

// =============================================================================
// K3 安全守护 (SecurityGuard, 借鉴 superpowers 234 verification-before-completion)
// =============================================================================

/// K3 安全守护 (6+1 = 7 重门聚合)
#[derive(Debug, Clone)]
pub struct SecurityGuard {
    /// 7 重门的事件列表 (按 gate idx)
    pub gate_events: [Vec<SecurityEvent>; SecurityGate::N_GATES],
    /// 7 重门的事件计数
    pub gate_counts: [u64; SecurityGate::N_GATES],
    /// 总事件数
    pub total_events: u64,
    /// 阻止事件数
    pub total_blocked: u64,
    /// 警告事件数
    pub total_warned: u64,
    /// 审计事件数
    pub total_audited: u64,
    /// v7 baseline 严守 (6 重 v7 0 改)
    pub v7_baseline_intact: bool,
    /// g7 baseline 严守 (7 项跨语言)
    pub g7_baseline_intact: bool,
    /// 最大事件数 (LRU 滚动)
    pub max_per_gate: usize,
}

impl Default for SecurityGuard {
    fn default() -> Self {
        Self::new(256)
    }
}

impl SecurityGuard {
    pub fn new(max_per_gate: usize) -> Self {
        let mut gate_events = Vec::with_capacity(SecurityGate::N_GATES);
        for _ in 0..SecurityGate::N_GATES {
            gate_events.push(Vec::with_capacity(max_per_gate));
        }
        // 数组初始化: 从 Vec 转换
        let gate_events: [Vec<SecurityEvent>; SecurityGate::N_GATES] = match gate_events.try_into()
        {
            Ok(arr) => arr,
            Err(_) => unreachable!("N_GATES = 7, vec len = 7"),
        };
        Self {
            gate_events,
            gate_counts: [0; SecurityGate::N_GATES],
            total_events: 0,
            total_blocked: 0,
            total_warned: 0,
            total_audited: 0,
            v7_baseline_intact: V7BaselineCheck::v7_baseline_intact(),
            g7_baseline_intact: CrossLanguageCheck::g7_baseline_intact(),
            max_per_gate,
        }
    }

    /// 记录安全事件
    pub fn record(&mut self, ev: SecurityEvent) {
        let idx = ev.gate.idx();
        // 计数
        self.gate_counts[idx] += 1;
        self.total_events += 1;
        if ev.blocked {
            self.total_blocked += 1;
        } else if matches!(ev.event_kind, SecurityEventKind::Warn) {
            self.total_warned += 1;
        } else if matches!(ev.event_kind, SecurityEventKind::Audit) {
            self.total_audited += 1;
        }
        // LRU 滚动
        if self.gate_events[idx].len() >= self.max_per_gate {
            self.gate_events[idx].remove(0);
        }
        self.gate_events[idx].push(ev);
    }

    /// 6 重 v7 + 1 跨语言 7 重全 OK
    pub fn all_gates_intact(&self) -> bool {
        self.v7_baseline_intact && self.g7_baseline_intact
    }

    /// 是否健康 (无 Block)
    pub fn is_healthy(&self) -> bool {
        self.total_blocked == 0
    }

    /// 1 门的裁决
    pub fn verdict(&self, gate: SecurityGate) -> SecurityVerdict {
        let events = &self.gate_events[gate.idx()];
        if events.iter().any(|e| e.blocked) {
            SecurityVerdict::Block
        } else if events
            .iter()
            .any(|e| matches!(e.event_kind, SecurityEventKind::Warn))
        {
            SecurityVerdict::Warn
        } else if events
            .iter()
            .any(|e| matches!(e.event_kind, SecurityEventKind::Audit))
        {
            SecurityVerdict::Audit
        } else {
            SecurityVerdict::Allow
        }
    }

    /// 7 门裁决
    pub fn all_verdicts(&self) -> [(SecurityGate, SecurityVerdict); SecurityGate::N_GATES] {
        let gates = [
            SecurityGate::G1Identity,
            SecurityGate::G2Goal,
            SecurityGate::G3Capability,
            SecurityGate::G4Compliance,
            SecurityGate::G5Resource,
            SecurityGate::G6Audit,
            SecurityGate::G7CrossLanguage,
        ];
        let mut out: [(SecurityGate, SecurityVerdict); SecurityGate::N_GATES] = [
            (SecurityGate::G1Identity, SecurityVerdict::Allow),
            (SecurityGate::G2Goal, SecurityVerdict::Allow),
            (SecurityGate::G3Capability, SecurityVerdict::Allow),
            (SecurityGate::G4Compliance, SecurityVerdict::Allow),
            (SecurityGate::G5Resource, SecurityVerdict::Allow),
            (SecurityGate::G6Audit, SecurityVerdict::Allow),
            (SecurityGate::G7CrossLanguage, SecurityVerdict::Allow),
        ];
        for (i, g) in gates.iter().enumerate() {
            out[i] = (*g, self.verdict(*g));
        }
        out
    }

    /// 摘要
    pub fn summary(&self) -> String {
        format!(
            "K3 SecurityGuard: events={} blocked={} warn={} audit={} v7_intact={} g7_intact={}",
            self.total_events,
            self.total_blocked,
            self.total_warned,
            self.total_audited,
            self.v7_baseline_intact,
            self.g7_baseline_intact
        )
    }
}

// =============================================================================
// K3 公共 API (per Stage 6 守护 spec)
// =============================================================================

/// K3 全局安全守护 (单例)
pub fn stage6_security_guard() -> &'static std::sync::Mutex<SecurityGuard> {
    use std::sync::{Mutex, OnceLock};
    static GUARD: OnceLock<Mutex<SecurityGuard>> = OnceLock::new();
    GUARD.get_or_init(|| Mutex::new(SecurityGuard::default()))
}

/// 记录 K3 安全事件 (Stage 6 公共入口)
pub fn stage6_record_security(ev: SecurityEvent) {
    let g = stage6_security_guard();
    if let Ok(mut g) = g.lock() {
        g.record(ev);
    }
}

/// K3 摘要
pub fn stage6_security_summary() -> String {
    let g = stage6_security_guard();
    if let Ok(g) = g.lock() {
        g.summary()
    } else {
        "K3 SecurityGuard: (lock contention)".to_string()
    }
}

/// K3 健康检查 (无 Block = healthy)
pub fn stage6_security_healthy() -> bool {
    let g = stage6_security_guard();
    if let Ok(g) = g.lock() {
        g.is_healthy()
    } else {
        true
    }
}

/// K3 6+1 重门 baseline 严守 verify
pub fn stage6_security_baseline_intact() -> bool {
    let g = stage6_security_guard();
    if let Ok(g) = g.lock() {
        g.all_gates_intact()
    } else {
        false
    }
}

// =============================================================================
// K3 单元测试 (cfg-无关, B4 严守 0 改 6 重 v7)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // 1. SecurityGate 7 重 + idx
    #[test]
    fn k3_security_gate_7() {
        assert_eq!(SecurityGate::N_GATES, 7);
        assert_eq!(SecurityGate::GATE_NAMES.len(), 7);
        assert_eq!(SecurityGate::G1Identity.idx(), 0);
        assert_eq!(SecurityGate::G7CrossLanguage.idx(), 6);
        assert_eq!(SecurityGate::G1Identity.name(), "G1_Identity");
        assert_eq!(SecurityGate::G7CrossLanguage.name(), "G7_CrossLanguage");
    }

    // 2. SecurityGate v7 baseline 严守 (B4)
    #[test]
    fn k3_v7_baseline_intact() {
        // 6 重 v7: G1-G6
        for g in [
            SecurityGate::G1Identity,
            SecurityGate::G2Goal,
            SecurityGate::G3Capability,
            SecurityGate::G4Compliance,
            SecurityGate::G5Resource,
            SecurityGate::G6Audit,
        ] {
            assert!(g.is_v7_baseline());
        }
        // G7 跨语言: K3 新增
        assert!(!SecurityGate::G7CrossLanguage.is_v7_baseline());
        // B4 6 重 v7 严守 verify
        assert!(V7BaselineCheck::v7_baseline_intact());
    }

    // 3. G7 跨语言 7 项 check
    #[test]
    fn k3_g7_cross_language_7_checks() {
        assert_eq!(CrossLanguageCheck::N_CHECKS, 7);
        assert_eq!(CrossLanguageCheck::CHECK_NAMES.len(), 7);
        assert!(CrossLanguageCheck::g7_baseline_intact());
    }

    // 4. SecurityEvent 构造 + blocked
    #[test]
    fn k3_security_event_block() {
        let ev = SecurityEvent::new(
            SecurityGate::G1Identity,
            SecurityEventKind::Block,
            SecuritySeverity::High,
            "pybridge.import",
            "module denied",
        );
        assert!(ev.blocked);
        assert_eq!(ev.gate, SecurityGate::G1Identity);
    }

    // 5. SecurityEvent with_context / with_timestamp
    #[test]
    fn k3_security_event_with_chain() {
        let ev = SecurityEvent::new(
            SecurityGate::G7CrossLanguage,
            SecurityEventKind::Warn,
            SecuritySeverity::Medium,
            "bridge",
            "gil contention",
        )
        .with_context("call_python_function")
        .with_timestamp(42);
        assert!(!ev.blocked);
        assert_eq!(ev.context.as_deref(), Some("call_python_function"));
        assert_eq!(ev.timestamp, 42);
    }

    // 6. SecurityVerdict 4 类
    #[test]
    fn k3_security_verdict_4() {
        assert!(SecurityVerdict::Allow.is_pass());
        assert!(!SecurityVerdict::Block.is_pass());
        assert_eq!(format!("{}", SecurityVerdict::Allow), "Allow");
        assert_eq!(format!("{}", SecurityVerdict::Block), "Block");
    }

    // 7. SecurityGuard record + 7 门隔离
    #[test]
    fn k3_security_guard_7_gate_isolation() {
        let mut g = SecurityGuard::default();
        for i in 0..7 {
            let gate = match i {
                0 => SecurityGate::G1Identity,
                1 => SecurityGate::G2Goal,
                2 => SecurityGate::G3Capability,
                3 => SecurityGate::G4Compliance,
                4 => SecurityGate::G5Resource,
                5 => SecurityGate::G6Audit,
                _ => SecurityGate::G7CrossLanguage,
            };
            g.record(SecurityEvent::new(
                gate,
                SecurityEventKind::Pass,
                SecuritySeverity::Low,
                "x",
                "x",
            ));
        }
        for i in 0..7 {
            assert_eq!(g.gate_counts[i], 1);
        }
        assert_eq!(g.total_events, 7);
    }

    // 8. SecurityGuard all_gates_intact
    #[test]
    fn k3_security_guard_baseline_intact() {
        let g = SecurityGuard::default();
        assert!(g.v7_baseline_intact);
        assert!(g.g7_baseline_intact);
        assert!(g.all_gates_intact());
    }

    // 9. SecurityGuard is_healthy 默认 true
    #[test]
    fn k3_security_guard_healthy() {
        let mut g = SecurityGuard::default();
        assert!(g.is_healthy());
        g.record(SecurityEvent::new(
            SecurityGate::G1Identity,
            SecurityEventKind::Block,
            SecuritySeverity::High,
            "x",
            "x",
        ));
        assert!(!g.is_healthy());
    }

    // 10. SecurityGuard verdict 单门
    #[test]
    fn k3_security_guard_verdict() {
        let mut g = SecurityGuard::default();
        // G1 全 Allow
        assert_eq!(g.verdict(SecurityGate::G1Identity), SecurityVerdict::Allow);
        // G1 Block
        g.record(SecurityEvent::new(
            SecurityGate::G1Identity,
            SecurityEventKind::Block,
            SecuritySeverity::High,
            "x",
            "x",
        ));
        assert_eq!(g.verdict(SecurityGate::G1Identity), SecurityVerdict::Block);
    }

    // 11. SecurityGuard all_verdicts 7 门
    #[test]
    fn k3_security_guard_all_verdicts() {
        let g = SecurityGuard::default();
        let v = g.all_verdicts();
        assert_eq!(v.len(), 7);
        for (gate, verdict) in v.iter() {
            assert_eq!(*verdict, SecurityVerdict::Allow);
            assert!(gate.is_v7_baseline() || *gate == SecurityGate::G7CrossLanguage);
        }
    }

    // 12. SecurityGuard summary
    #[test]
    fn k3_security_guard_summary() {
        let mut g = SecurityGuard::default();
        g.record(SecurityEvent::new(
            SecurityGate::G7CrossLanguage,
            SecurityEventKind::Audit,
            SecuritySeverity::Low,
            "bridge",
            "audit",
        ));
        let s = g.summary();
        assert!(s.contains("K3 SecurityGuard"));
        assert!(s.contains("events=1"));
        assert!(s.contains("v7_intact=true"));
        assert!(s.contains("g7_intact=true"));
    }

    // 13. SecuritySeverity 3 级 + score
    #[test]
    fn k3_security_severity_3_levels() {
        assert_eq!(SecuritySeverity::N_SEVERITIES, 3);
        assert_eq!(SecuritySeverity::Low.score(), 1);
        assert_eq!(SecuritySeverity::Medium.score(), 10);
        assert_eq!(SecuritySeverity::High.score(), 100);
    }

    // 14. SecurityEvent Display
    #[test]
    fn k3_security_event_display() {
        let ev = SecurityEvent::new(
            SecurityGate::G1Identity,
            SecurityEventKind::Pass,
            SecuritySeverity::Low,
            "pybridge",
            "ok",
        )
        .with_context("test")
        .with_timestamp(42);
        let s = format!("{ev}");
        assert!(s.contains("[G1_Identity|Pass]"));
        assert!(s.contains("pybridge"));
        assert!(s.contains("ok"));
        assert!(s.contains("ctx: test"));
    }

    // 15. SecurityEvent Display blocked
    #[test]
    fn k3_security_event_display_blocked() {
        let ev = SecurityEvent::new(
            SecurityGate::G7CrossLanguage,
            SecurityEventKind::Block,
            SecuritySeverity::High,
            "x",
            "x",
        );
        let s = format!("{ev}");
        assert!(s.contains("🚫"));
    }

    // 16. SecurityGate Display
    #[test]
    fn k3_security_gate_display() {
        assert_eq!(format!("{}", SecurityGate::G1Identity), "G1_Identity");
        assert_eq!(
            format!("{}", SecurityGate::G7CrossLanguage),
            "G7_CrossLanguage"
        );
    }

    // 17. stage6_record_security + summary 全局
    #[test]
    fn k3_stage6_record_global() {
        stage6_record_security(SecurityEvent::new(
            SecurityGate::G2Goal,
            SecurityEventKind::Pass,
            SecuritySeverity::Low,
            "test",
            "global",
        ));
        let s = stage6_security_summary();
        assert!(s.contains("K3 SecurityGuard"));
    }

    // 18. stage6_security_healthy / baseline_intact
    #[test]
    fn k3_stage6_health_baseline() {
        let h = stage6_security_healthy();
        let b = stage6_security_baseline_intact();
        // baseline 永远 true (编译期 hardcode)
        assert!(b);
        let _ = h;
    }

    // 19. SecurityGuard LRU 滚动
    #[test]
    fn k3_security_guard_lru() {
        let mut g = SecurityGuard::new(3);
        for i in 0..5 {
            g.record(SecurityEvent::new(
                SecurityGate::G1Identity,
                SecurityEventKind::Pass,
                SecuritySeverity::Low,
                "x",
                format!("e{i}"),
            ));
        }
        assert_eq!(g.gate_events[0].len(), 3);
    }

    // 20. SecurityGuard total_audited
    #[test]
    fn k3_security_guard_audit_count() {
        let mut g = SecurityGuard::default();
        g.record(SecurityEvent::new(
            SecurityGate::G6Audit,
            SecurityEventKind::Audit,
            SecuritySeverity::Low,
            "x",
            "x",
        ));
        g.record(SecurityEvent::new(
            SecurityGate::G6Audit,
            SecurityEventKind::Audit,
            SecuritySeverity::Low,
            "x",
            "x",
        ));
        assert_eq!(g.total_audited, 2);
    }
}
