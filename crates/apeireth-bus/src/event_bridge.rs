//! `apeireth-bus::event_bridge` — A4/TP26 统一事件流 + 感知门控.
//!
//! ## 定位 (官方边界三线: 基础设施 = 连续感知的地基)
//!
//! bus 已有零件: `channel` (ChanneledBus) / `event_log` (append-only + replay) /
//! `lifecycle` / `pattern` (topic wildcard). 本模块补两块:
//!
//! 1. **统一事件类型层** (`UnifiedEvent`): System/Market/User/Tool/Agent/Workflow
//!    六类事件归一 (A4: agent event / workflow EventHistory / bus 事件的统一视图).
//! 2. **PerceptionGate 感知门控**: 类型过滤 + 突发检测 — "什么值得她感知"
//!    (连续感知轻层: 不是全时录像, 是显著性触发).
//!
//! TP26 投资事件架构: Market 事件 (行情 tick/信号/决策) 走同一桥,
//! oracle 决策链按 `by_kind(Market)` 消费 (vnpy 式 行情→信号→决策 的事件解耦).
//!
//! ## 0 装 PASS
//!
//! - 本模块是内存事件桥 (确定性, 无 LLM, 无 IO). 持久化 = 接 `event_log` (append-only,
//!   调用方决定), 不假装已落盘.
//! - 突发检测用固定窗口 (60s), 阈值可配 — 全参数"待拟合".

use std::collections::{HashMap, VecDeque};

/// 统一事件类型 (A4 六源归一 + Market 投资事件).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventKind {
    /// 系统事件 (文件变化/窗口切换/登录告警 — 连续感知轻层).
    System,
    /// 投资市场事件 (行情 tick/信号/决策 — TP26).
    Market,
    /// 用户交互事件.
    User,
    /// 工具执行事件.
    Tool,
    /// Agent 生命周期事件 (Registered/Unregistered/Handoff).
    Agent,
    /// Workflow 执行事件 (EventHistory 对齐).
    Workflow,
}

impl EventKind {
    pub fn label(&self) -> &'static str {
        match self {
            EventKind::System => "system",
            EventKind::Market => "market",
            EventKind::User => "user",
            EventKind::Tool => "tool",
            EventKind::Agent => "agent",
            EventKind::Workflow => "workflow",
        }
    }
}

/// 统一事件 (跨来源的公共视图).
#[derive(Debug, Clone)]
pub struct UnifiedEvent {
    pub id: u64,
    pub kind: EventKind,
    /// 来源标识 (如 "agent:researcher" / "workflow:deploy").
    pub source: String,
    pub payload_json: String,
    pub at_ms: i64,
}

/// 事件桥 (统一登记 + 过滤查询).
#[derive(Debug, Default)]
pub struct EventBridge {
    events: Vec<UnifiedEvent>,
    next_id: u64,
}

impl EventBridge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, kind: EventKind, source: impl Into<String>, payload: impl Into<String>) -> &UnifiedEvent {
        let ev = UnifiedEvent {
            id: self.next_id,
            kind,
            source: source.into(),
            payload_json: payload.into(),
            at_ms: chrono::Utc::now().timestamp_millis(),
        };
        self.next_id += 1;
        self.events.push(ev);
        self.events.last().unwrap()
    }

    /// 按类型过滤 (TP26: oracle 决策链消费 Market 事件).
    pub fn by_kind(&self, kind: EventKind) -> Vec<&UnifiedEvent> {
        self.events.iter().filter(|e| e.kind == kind).collect()
    }

    /// 最近 N 条 (跨类型).
    pub fn recent(&self, n: usize) -> Vec<&UnifiedEvent> {
        self.events.iter().rev().take(n).collect()
    }

    /// since 时间戳之后的事件 (轮询消费).
    pub fn since(&self, at_ms: i64) -> Vec<&UnifiedEvent> {
        self.events.iter().filter(|e| e.at_ms >= at_ms).collect()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }
}

/// 感知门控配置 (连续感知: 显著性触发, 非全时).
#[derive(Debug, Clone)]
pub struct GateConfig {
    /// 允许感知的事件类型 (空 = 全部, 0 装: 默认全开但突发阈值兜底).
    pub enabled_kinds: Vec<EventKind>,
    /// 突发检测窗口 (ms).
    pub burst_window_ms: i64,
    /// 突发阈值: 窗口内同类型事件 ≥ 此值 → 值得感知.
    pub burst_threshold: usize,
}

impl Default for GateConfig {
    fn default() -> Self {
        Self {
            enabled_kinds: Vec::new(),
            burst_window_ms: 60_000, // 60s
            burst_threshold: 3,      // 待拟合
        }
    }
}

/// 感知门控 (确定性): 类型过滤 + 突发检测.
#[derive(Debug)]
pub struct PerceptionGate {
    config: GateConfig,
    /// 类型 → 最近事件时间窗 (突发检测).
    recent_by_kind: HashMap<EventKind, VecDeque<i64>>,
}

impl PerceptionGate {
    pub fn new(config: GateConfig) -> Self {
        Self {
            config,
            recent_by_kind: HashMap::new(),
        }
    }

    /// 该事件是否值得感知:
    /// 1. 白名单非空: 类型在白名单内 → 直接感知 (单条即触发, 如 Market 信号);
    ///    不在 → 不感知.
    /// 2. 白名单为空 (全开): 走突发检测 — 窗口内同类型 ≥ 阈值 → 感知
    ///    (连续感知轻层: 显著性触发, 非全时录像).
    pub fn should_perceive(&mut self, ev: &UnifiedEvent) -> bool {
        if !self.config.enabled_kinds.is_empty() {
            return self.config.enabled_kinds.contains(&ev.kind);
        }
        let now = ev.at_ms;
        let window = self
            .recent_by_kind
            .entry(ev.kind)
            .or_insert_with(VecDeque::new);
        // 清出窗口外的旧事件
        while let Some(&t) = window.front() {
            if now - t > self.config.burst_window_ms {
                window.pop_front();
            } else {
                break;
            }
        }
        window.push_back(now);
        window.len() >= self.config.burst_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_push_and_filter_by_kind() {
        let mut bridge = EventBridge::new();
        bridge.push(EventKind::Market, "tick:btc", r#"{"px": 64000}"#);
        bridge.push(EventKind::System, "window:switch", r#"{"app":"cargo"}"#);
        bridge.push(EventKind::Market, "signal:buy", r#"{"conf":0.72}"#);
        assert_eq!(bridge.len(), 3);
        let market = bridge.by_kind(EventKind::Market);
        assert_eq!(market.len(), 2, "Market 事件应被过滤出 2 条");
        assert!(market[0].payload_json.contains("64000"));
        assert_eq!(bridge.recent(2).len(), 2);
        assert_eq!(bridge.since(0).len(), 3);
    }

    #[test]
    fn gate_type_filter() {
        let mut gate = PerceptionGate::new(GateConfig {
            enabled_kinds: vec![EventKind::Market],
            ..Default::default()
        });
        let mut bridge = EventBridge::new();
        let sys = bridge.push(EventKind::System, "s", "{}").clone();
        let mkt = bridge.push(EventKind::Market, "m", "{}").clone();
        assert!(!gate.should_perceive(&sys), "System 不在白名单");
        assert!(gate.should_perceive(&mkt));
    }

    #[test]
    fn gate_burst_detection() {
        let mut gate = PerceptionGate::new(GateConfig::default()); // 全开 + 突发阈值 3
        let mut bridge = EventBridge::new();
        let e1 = bridge.push(EventKind::System, "file:changed", "{}").clone();
        let e2 = bridge.push(EventKind::System, "file:changed", "{}").clone();
        assert!(!gate.should_perceive(&e1), "1 条不构成突发");
        assert!(!gate.should_perceive(&e2), "2 条不构成突发 (阈值 3)");
        let e3 = bridge.push(EventKind::System, "file:changed", "{}").clone();
        assert!(gate.should_perceive(&e3), "3 条突发 → 值得感知");
    }

    #[test]
    fn gate_burst_window_expires() {
        let mut gate = PerceptionGate::new(GateConfig {
            burst_window_ms: 1000,
            burst_threshold: 2,
            ..Default::default()
        });
        let now = chrono::Utc::now().timestamp_millis();
        let ev1 = UnifiedEvent { id: 1, kind: EventKind::User, source: "u".into(), payload_json: "{}".into(), at_ms: now - 5000 };
        let ev2 = UnifiedEvent { id: 2, kind: EventKind::User, source: "u".into(), payload_json: "{}".into(), at_ms: now };
        assert!(!gate.should_perceive(&ev1), "窗口外旧事件不计数");
        assert!(!gate.should_perceive(&ev2), "窗口内仅 1 条 < 阈值 2");
    }

    #[test]
    fn market_events_feed_oracle_chain_shape() {
        // TP26: vnpy 式 行情→信号→决策 — 桥按 Market 类型供 oracle 决策链消费
        let mut bridge = EventBridge::new();
        bridge.push(EventKind::Market, "quote:stock", r#"{"sym":"A","px":10.2}"#);
        bridge.push(EventKind::Market, "signal:long", r#"{"conf":0.8}"#);
        bridge.push(EventKind::Market, "decision:enter", r#"{"size":100}"#);
        let chain: Vec<&UnifiedEvent> = bridge.by_kind(EventKind::Market);
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0].source, "quote:stock");
        assert_eq!(chain[2].source, "decision:enter");
    }
}
