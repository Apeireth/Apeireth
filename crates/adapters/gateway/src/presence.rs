//! Presence state synthesizer — the v0 heuristic behind the `presence_state`
//! event on the gateway SSE bus (`GET /v1/apeireth/events`).
//!
//! Contract: `docs/design/00-PHILOSOPHY.md` §10 (draft v0); wire format:
//! `docs/gateway-api-contract.md` §8a. One contract, four projections — this
//! module owns the single backend estimate that every projection renders.
//!
//! Honesty boundary (0 装): every frame carries `source.kind = "heuristic_v0"`
//! and a fixed low `confidence`. The heuristic promises truthful semantic
//! direction only, never precision. v0 has no honest trigger for
//! `stance = empathetic_care` or `significance = ritual`, so it never emits
//! them; both stay in the contract as reserved space for a future affect
//! engine — reserved, not faked.
//!
//! Bus reality this module is built against (canonical runtime):
//! - `RuntimeEvent::TurnStarted` is emitted live at turn start; the turn's
//!   `Trace` events are replayed to sinks at outcome time, immediately before
//!   `TurnCompleted`. Counting tools/approvals from the replayed trace at turn
//!   end is therefore exact, not sampled.
//! - A memory recall is only observable here when it ran as a *dispatched
//!   capability* whose id mentions `recall`/`memory` (e.g. an MCP recall tool).
//!   The canonical `MemoryRecallModule` prompt-overlay path does not emit
//!   `RuntimeEvent`s today, so v0 cannot see it and does not pretend to.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;

use apeireth_core::kernel::Timestamp;
use apeireth_runtime::canonical::{RuntimeEvent, RuntimeEventSink, TraceEvent};

use crate::ember_hud_driver::{EmberCognitiveStance, EmberHudDriver};
use crate::events::{EventBus, GatewayEvent};

/// Heartbeat cadence: exactly one low-frequency tick per 60 s.
/// Contract §10 频率纪律: 回合级事件 + 低频心跳(≤ 每 60s), 无高频推送.
pub const HEARTBEAT_INTERVAL_SECS: u64 = 60;

/// Initiative (he wakes from `dreaming_consolidation` and speaks first) is rare
/// by discipline — 防唠叨, 宁少勿多. Default budget: at most this many takes
/// per rolling 24 h window.
pub const INITIATIVE_DAILY_CAP: u32 = 3;

/// Rolling window of the initiative budget.
const INITIATIVE_WINDOW_MS: i64 = 24 * 60 * 60 * 1_000;

/// Idle time after which `deep_coding_focus` decays back to plain attentiveness
/// (no fresh evidence of engineering work).
const IDLE_FOCUS_DECAY_MS: i64 = 5 * 60 * 1_000;

/// Idle time after which he visibly settles into background consolidation.
const IDLE_DREAMING_MS: i64 = 30 * 60 * 1_000;

/// A turn dispatching at least this many tools reads as deep engineering work.
const DEEP_FOCUS_TOOL_THRESHOLD: u32 = 3;

/// A turn running at least this long adds one arousal notch (轮次时长是
/// 契约点名的 PAD 原料之一).
const LONG_TURN_MS: i64 = 30 * 1_000;

/// Defensive cap on in-flight turn accumulators; entries only leak when a turn
/// starts and never closes (e.g. a killed request).
const MAX_IN_FLIGHT_TURNS: usize = 256;

/// `source.kind` — always honest about the estimator vintage.
const SOURCE_KIND: &str = "heuristic_v0";

/// Fixed confidence: the v0 heuristic cannot grade its own uncertainty per
/// frame, so it says so once, honestly, at a deliberately low constant.
const SOURCE_CONFIDENCE: f32 = 0.5;

/// PAD baseline: the neutral resting point heartbeats decay toward.
const BASELINE_PAD: PresencePad = PresencePad {
    p: 0.0,
    a: 0.0,
    d: 0.0,
};

/// Per-heartbeat-tick decay factor toward [`BASELINE_PAD`].
const HEARTBEAT_DECAY: f32 = 0.8;

/// Round to 3 decimals so wire values stay clean (`0.3`, not
/// `0.30000000000000004`); grooming float noise, not fabricating precision.
fn round3(value: f32) -> f32 {
    (value * 1_000.0).round() / 1_000.0
}

fn clamp_unit(value: f32) -> f32 {
    value.clamp(-1.0, 1.0)
}

/// Pleasure / arousal / dominance estimate, each ∈ [-1, 1].
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct PresencePad {
    pub p: f32,
    pub a: f32,
    pub d: f32,
}

/// Breath rhythm parameters for the ember-point / halo projections.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct PresenceBreath {
    pub period_secs: f32,
    pub amplitude: f32,
}

/// Render-grade tier of a presence frame (contract §10 显影分级).
///
/// `Ritual` is contract space only: heuristic_v0 has no honest ritual trigger
/// and never emits it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PresenceSignificance {
    /// Low-frequency tick; only the ember point moves.
    Heartbeat,
    /// Turn-level change; halo / stance may move.
    Turn,
    /// Ceremony-grade, month-rare; reserved, never produced by heuristic_v0.
    Ritual,
}

/// Provenance of the estimate. `kind` is always `"heuristic_v0"` in this
/// module; a future affect engine swaps the kind, not the contract shape.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct PresenceSource {
    pub kind: &'static str,
    pub confidence: f32,
}

/// One `presence_state` frame — the exact wire shape of contract §10 / §8a.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PresenceState {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    /// Epoch milliseconds.
    pub at: i64,
    pub pad: PresencePad,
    /// Discrete label derived from `pad` (see [`dominant_label`]).
    pub dominant: String,
    /// Overall magnitude ∈ [0, 1].
    pub intensity: f32,
    pub stance: EmberCognitiveStance,
    pub breath: PresenceBreath,
    pub significance: PresenceSignificance,
    pub source: PresenceSource,
}

/// Discrete label derived from PAD signs; the mapping is deliberately tiny and
/// fixed so the label never claims more than the numbers carry.
fn dominant_label(pad: PresencePad) -> &'static str {
    if pad.p >= 0.1 {
        if pad.a >= 0.1 {
            "engaged"
        } else if pad.a <= -0.1 {
            "serene"
        } else {
            "neutral"
        }
    } else if pad.p <= -0.1 {
        if pad.a >= 0.1 {
            "strained"
        } else if pad.a <= -0.1 {
            "subdued"
        } else {
            "neutral"
        }
    } else {
        "neutral"
    }
}

/// Overall magnitude of a PAD vector, ∈ [0, 1].
fn pad_intensity(pad: PresencePad) -> f32 {
    ((pad.p.abs() + pad.a.abs() + pad.d.abs()) / 3.0).clamp(0.0, 1.0)
}

/// Per-day initiative limiter (contract §10 initiative 限量). The budget is
/// enforced here so that any future initiative producer is physically unable
/// to nag: after `cap` takes inside one rolling window, `try_take` refuses.
#[derive(Debug, Clone)]
pub struct InitiativeBudget {
    cap: u32,
    window_ms: i64,
    window_start_ms: i64,
    used: u32,
}

impl InitiativeBudget {
    /// Start a fresh budget window at `now_ms`.
    pub fn new(cap: u32, window_ms: i64, now_ms: i64) -> Self {
        Self {
            cap,
            window_ms,
            window_start_ms: now_ms,
            used: 0,
        }
    }

    /// Take one initiative slot, or refuse once the window's cap is spent.
    pub fn try_take(&mut self, now_ms: i64) -> bool {
        if now_ms - self.window_start_ms >= self.window_ms {
            self.window_start_ms = now_ms;
            self.used = 0;
        }
        if self.used >= self.cap {
            return false;
        }
        self.used += 1;
        true
    }

    /// Slots already spent in the current window (observability for tests and
    /// future diagnostics).
    pub fn used_in_window(&self) -> u32 {
        self.used
    }
}

/// Facts of one in-flight turn, accumulated from the trace replay.
#[derive(Debug, Default, Clone, Copy)]
struct TurnAccumulator {
    started_ms: i64,
    tool_calls: u32,
    approvals: u32,
    recall_hits: u32,
}

/// Pure presence estimator. Every method takes `now_ms` explicitly so the
/// heuristic is fully deterministic under test; the tokio-facing
/// [`PresenceService`] is the only place that reads a clock.
#[derive(Debug)]
pub struct PresenceSynthesizer {
    pad: PresencePad,
    stance: EmberCognitiveStance,
    last_activity_ms: Option<i64>,
    in_flight: HashMap<String, TurnAccumulator>,
    initiative: InitiativeBudget,
    breath_period_secs: f32,
}

impl Default for PresenceSynthesizer {
    fn default() -> Self {
        Self {
            pad: BASELINE_PAD,
            stance: EmberCognitiveStance::AttentivePresence,
            last_activity_ms: None,
            in_flight: HashMap::new(),
            initiative: InitiativeBudget::new(INITIATIVE_DAILY_CAP, INITIATIVE_WINDOW_MS, 0),
            breath_period_secs: EmberHudDriver::default().breath_period_secs,
        }
    }
}

impl PresenceSynthesizer {
    pub fn new() -> Self {
        Self::default()
    }

    /// A turn acquired its trace identity; open an accumulator for it.
    pub fn turn_started(&mut self, turn_key: &str, now_ms: i64) {
        if self.in_flight.len() >= MAX_IN_FLIGHT_TURNS {
            self.in_flight.clear();
        }
        self.in_flight.insert(
            turn_key.to_string(),
            TurnAccumulator {
                started_ms: now_ms,
                ..TurnAccumulator::default()
            },
        );
    }

    /// One capability dispatch inside an in-flight turn. A capability whose id
    /// mentions `recall`/`memory` additionally counts as a recall hit — the
    /// only recall signal observable on this bus (see module docs).
    pub fn observe_tool_dispatch(&mut self, turn_key: &str, capability: &str) {
        if let Some(turn) = self.in_flight.get_mut(turn_key) {
            turn.tool_calls += 1;
            if capability.contains("recall") || capability.contains("memory") {
                turn.recall_hits += 1;
            }
        }
    }

    /// One approval request inside an in-flight turn (from the trace replay —
    /// the top-level `ApprovalRequired` event is ignored so pause/resume does
    /// not double-count).
    pub fn observe_approval_request(&mut self, turn_key: &str) {
        if let Some(turn) = self.in_flight.get_mut(turn_key) {
            turn.approvals += 1;
        }
    }

    /// A turn committed. Map its facts to a PAD rough estimate (契约: 轮次
    /// 时长 / 工具调用数 / 审批频率 → PAD 粗估值; 召回命中正向微调):
    /// - arousal rises with rounds, tool calls, and a long turn duration;
    /// - pleasure dips per approval wait, rises per recall hit;
    /// - dominance rises mildly with agency (tool calls).
    pub fn turn_completed(&mut self, turn_key: &str, rounds: u32, now_ms: i64) -> PresenceState {
        let turn = self.in_flight.remove(turn_key).unwrap_or_default();

        let mut a = 0.2 + 0.08 * turn.tool_calls as f32 + 0.05 * rounds.saturating_sub(1) as f32;
        if now_ms - turn.started_ms >= LONG_TURN_MS {
            a += 0.1;
        }
        let pad = PresencePad {
            p: 0.1 + 0.15 * turn.recall_hits as f32 - 0.15 * turn.approvals as f32,
            a: a.min(0.8),
            d: 0.1 + 0.04 * turn.tool_calls as f32,
        };
        self.pad = PresencePad {
            p: clamp_unit(pad.p),
            a: clamp_unit(pad.a),
            d: clamp_unit(pad.d),
        };
        self.stance = if turn.tool_calls >= DEEP_FOCUS_TOOL_THRESHOLD {
            EmberCognitiveStance::DeepCodingFocus
        } else {
            EmberCognitiveStance::AttentivePresence
        };
        self.last_activity_ms = Some(now_ms);
        self.frame(now_ms, PresenceSignificance::Turn)
    }

    /// A turn failed before committing. Honest read: friction — pleasure dips,
    /// arousal rises, dominance dips. Tool/approval facts are unavailable on
    /// this path (the runtime does not replay the trace of a failed turn), so
    /// the estimate is a fixed modest nudge rather than invented detail.
    pub fn turn_failed(&mut self, turn_key: &str, now_ms: i64) -> PresenceState {
        self.in_flight.remove(turn_key);
        self.pad = PresencePad {
            p: -0.2,
            a: 0.3,
            d: -0.1,
        };
        self.stance = EmberCognitiveStance::AttentivePresence;
        self.last_activity_ms = Some(now_ms);
        self.frame(now_ms, PresenceSignificance::Turn)
    }

    /// One heartbeat tick: decay toward baseline, and let the stance settle
    /// with idle time (deep focus → attentive after 5 min quiet → dreaming
    /// consolidation after 30 min quiet).
    pub fn heartbeat(&mut self, now_ms: i64) -> PresenceState {
        self.pad = PresencePad {
            p: round3(BASELINE_PAD.p + (self.pad.p - BASELINE_PAD.p) * HEARTBEAT_DECAY),
            a: round3(BASELINE_PAD.a + (self.pad.a - BASELINE_PAD.a) * HEARTBEAT_DECAY),
            d: round3(BASELINE_PAD.d + (self.pad.d - BASELINE_PAD.d) * HEARTBEAT_DECAY),
        };
        if let Some(last) = self.last_activity_ms {
            let idle = now_ms - last;
            if idle >= IDLE_DREAMING_MS {
                self.stance = EmberCognitiveStance::DreamingConsolidation;
            } else if idle >= IDLE_FOCUS_DECAY_MS
                && self.stance == EmberCognitiveStance::DeepCodingFocus
            {
                self.stance = EmberCognitiveStance::AttentivePresence;
            }
        } else {
            // Never any turn since boot: he has been quietly consolidating.
            self.stance = EmberCognitiveStance::DreamingConsolidation;
        }
        self.frame(now_ms, PresenceSignificance::Heartbeat)
    }

    /// Gate for a future initiative producer (he wakes from dreaming and
    /// speaks first). v0 ships the discipline before the producer: the budget
    /// is real and enforced from day one.
    pub fn try_take_initiative(&mut self, now_ms: i64) -> bool {
        self.initiative.try_take(now_ms)
    }

    /// Slots already spent in the current initiative window.
    pub fn initiative_used_in_window(&self) -> u32 {
        self.initiative.used_in_window()
    }

    /// Build one wire frame from the current estimate.
    fn frame(&self, at_ms: i64, significance: PresenceSignificance) -> PresenceState {
        let intensity = round3(pad_intensity(self.pad));
        PresenceState {
            event_type: "presence_state",
            at: at_ms,
            pad: PresencePad {
                p: round3(self.pad.p),
                a: round3(self.pad.a),
                d: round3(self.pad.d),
            },
            dominant: dominant_label(self.pad).to_string(),
            intensity,
            stance: self.stance,
            breath: PresenceBreath {
                period_secs: self.breath_period_secs,
                amplitude: round3((0.2 + 0.6 * intensity).clamp(0.0, 1.0)),
            },
            significance,
            source: PresenceSource {
                kind: SOURCE_KIND,
                confidence: SOURCE_CONFIDENCE,
            },
        }
    }
}

/// Tokio-facing wrapper: owns the synthesizer, observes the runtime event
/// spine, and publishes `presence_state` frames onto the gateway SSE bus.
#[derive(Debug)]
pub struct PresenceService {
    bus: EventBus,
    synthesizer: Mutex<PresenceSynthesizer>,
}

impl PresenceService {
    /// Create the service publishing onto `bus`.
    pub fn new(bus: EventBus) -> Arc<Self> {
        Arc::new(Self {
            bus,
            synthesizer: Mutex::new(PresenceSynthesizer::new()),
        })
    }

    /// Publish one heartbeat frame. Called by the heartbeat task once per
    /// [`HEARTBEAT_INTERVAL_SECS`]; also directly callable in tests.
    pub fn emit_heartbeat(&self) {
        let now_ms = Timestamp::now().epoch_millis();
        let Ok(mut synth) = self.synthesizer.lock() else {
            return; // a poisoned estimator must never disturb the runtime
        };
        let state = synth.heartbeat(now_ms);
        drop(synth);
        self.publish(state);
    }

    fn publish(&self, state: PresenceState) {
        if let Ok(data) = serde_json::to_value(&state) {
            self.bus.publish(GatewayEvent::new("presence_state", data));
        }
    }
}

impl RuntimeEventSink for PresenceService {
    fn emit(&self, event: RuntimeEvent) {
        let Ok(mut synth) = self.synthesizer.lock() else {
            return; // a poisoned estimator must never disturb the runtime
        };
        let now_ms = Timestamp::now().epoch_millis();
        match event {
            RuntimeEvent::TurnStarted { trace, .. } => {
                synth.turn_started(&trace.to_string(), now_ms);
            }
            RuntimeEvent::Trace { trace, event, .. } => match event {
                TraceEvent::CapabilityDispatched { capability, .. } => {
                    synth.observe_tool_dispatch(&trace.to_string(), capability.as_str());
                }
                TraceEvent::ApprovalRequested { .. } => {
                    synth.observe_approval_request(&trace.to_string());
                }
                _ => {}
            },
            RuntimeEvent::TurnCompleted { trace, rounds, .. } => {
                let state = synth.turn_completed(&trace.to_string(), rounds, now_ms);
                drop(synth);
                self.publish(state);
            }
            RuntimeEvent::TurnFailed { trace, .. } => {
                let state = synth.turn_failed(&trace.to_string(), now_ms);
                drop(synth);
                self.publish(state);
            }
            // Pause/resume would double-count approvals; the turn-end trace
            // replay carries `ApprovalRequested` exactly once per request.
            RuntimeEvent::ApprovalRequired { .. } => {}
        }
    }
}

/// Spawn the 60 s heartbeat task. It holds the service weakly: once the
/// gateway state (and with it the runtime's sink fan-out) is dropped, the task
/// exits instead of leaking. Outside a tokio runtime it degrades honestly to
/// turn-level frames only — no executor, no heartbeat, no panic.
pub fn spawn_presence_heartbeat(service: &Arc<PresenceService>) {
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return;
    };
    let weak = Arc::downgrade(service);
    handle.spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await; // the first tick is immediate; skip it
        loop {
            interval.tick().await;
            let Some(service) = weak.upgrade() else { break };
            service.emit_heartbeat();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::{CapabilityId, RequestId, SessionId, TraceId};

    const T0: i64 = 1_726_992_000_000; // 契约示例的 at

    fn turn_key() -> String {
        TraceId::new().to_string()
    }

    /// Run one complete turn through the synthesizer and return its frame.
    fn run_turn(
        synth: &mut PresenceSynthesizer,
        tool_capabilities: &[&str],
        approvals: u32,
        rounds: u32,
        duration_ms: i64,
    ) -> PresenceState {
        let key = turn_key();
        synth.turn_started(&key, T0);
        for capability in tool_capabilities {
            synth.observe_tool_dispatch(&key, capability);
        }
        for _ in 0..approvals {
            synth.observe_approval_request(&key);
        }
        synth.turn_completed(&key, rounds, T0 + duration_ms)
    }

    // ① 事件 shape / serde 序列化形状

    #[test]
    fn presence_state_serializes_the_contract_shape() {
        let state = PresenceState {
            event_type: "presence_state",
            at: T0,
            pad: PresencePad {
                p: 0.2,
                a: -0.1,
                d: 0.3,
            },
            dominant: "serene".to_string(),
            intensity: 0.38,
            stance: EmberCognitiveStance::AttentivePresence,
            breath: PresenceBreath {
                period_secs: 4.0,
                amplitude: 0.6,
            },
            significance: PresenceSignificance::Turn,
            source: PresenceSource {
                kind: "heuristic_v0",
                confidence: 0.5,
            },
        };
        // 与 00-PHILOSOPHY §10 契约实例逐字节对齐(字段序即声明序)。
        let expected = r#"{"type":"presence_state","at":1726992000000,"pad":{"p":0.2,"a":-0.1,"d":0.3},"dominant":"serene","intensity":0.38,"stance":"attentive_presence","breath":{"period_secs":4.0,"amplitude":0.6},"significance":"turn","source":{"kind":"heuristic_v0","confidence":0.5}}"#;
        assert_eq!(serde_json::to_string(&state).unwrap(), expected);
    }

    #[test]
    fn presence_state_covers_every_contract_field_and_stance_value() {
        let mut synth = PresenceSynthesizer::new();
        let frame = run_turn(&mut synth, &[], 0, 1, 5_000);
        let json = serde_json::to_value(&frame).unwrap();
        // Value 的 Map 按键排序;字段顺序由上面的 to_string 形状测试锁定。
        let mut keys: Vec<&str> = json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "at",
                "breath",
                "dominant",
                "intensity",
                "pad",
                "significance",
                "source",
                "stance",
                "type"
            ]
        );
        assert_eq!(json["at"], serde_json::json!(T0 + 5_000));
        assert_eq!(json["source"]["kind"], "heuristic_v0");
        assert!((json["source"]["confidence"].as_f64().unwrap() - 0.5).abs() < 1e-6);

        for (stance, wire) in [
            (EmberCognitiveStance::DeepCodingFocus, "deep_coding_focus"),
            (
                EmberCognitiveStance::AttentivePresence,
                "attentive_presence",
            ),
            (
                EmberCognitiveStance::DreamingConsolidation,
                "dreaming_consolidation",
            ),
            (EmberCognitiveStance::EmpatheticCare, "empathetic_care"),
        ] {
            assert_eq!(
                serde_json::to_value(stance).unwrap(),
                serde_json::json!(wire)
            );
        }
        for (significance, wire) in [
            (PresenceSignificance::Heartbeat, "heartbeat"),
            (PresenceSignificance::Turn, "turn"),
            (PresenceSignificance::Ritual, "ritual"),
        ] {
            assert_eq!(
                serde_json::to_value(significance).unwrap(),
                serde_json::json!(wire)
            );
        }
    }

    // ② significance 分级判定

    #[test]
    fn significance_follows_production_timing() {
        let mut synth = PresenceSynthesizer::new();
        let turn_frame = run_turn(&mut synth, &[], 0, 1, 5_000);
        assert_eq!(turn_frame.significance, PresenceSignificance::Turn);

        let failed = synth.turn_failed(&turn_key(), T0 + 10_000);
        assert_eq!(failed.significance, PresenceSignificance::Turn);

        let heartbeat = synth.heartbeat(T0 + 70_000);
        assert_eq!(heartbeat.significance, PresenceSignificance::Heartbeat);
        // v0 没有任何诚实 ritual 触发器:完整流程里它永不出现。
        assert_ne!(turn_frame.significance, PresenceSignificance::Ritual);
        assert_ne!(heartbeat.significance, PresenceSignificance::Ritual);
    }

    #[test]
    fn tool_heavy_turn_reads_as_deep_coding_focus() {
        let mut synth = PresenceSynthesizer::new();
        let quiet = run_turn(&mut synth, &["tool.calculator"], 0, 1, 5_000);
        assert_eq!(quiet.stance, EmberCognitiveStance::AttentivePresence);

        let busy = run_turn(
            &mut synth,
            &["tool.repo", "tool.shell", "tool.repo"],
            0,
            2,
            40_000,
        );
        assert_eq!(busy.stance, EmberCognitiveStance::DeepCodingFocus);
        // 高唤起 → engaged 标签;强度高于安静回合。
        assert_eq!(busy.dominant, "engaged");
        assert!(busy.intensity > quiet.intensity);
    }

    #[test]
    fn approvals_lower_pleasure_and_recall_hits_raise_it() {
        let mut synth = PresenceSynthesizer::new();
        let plain = run_turn(&mut synth, &[], 0, 1, 5_000);
        let gated = run_turn(&mut synth, &[], 2, 2, 5_000);
        let recalled = run_turn(
            &mut synth,
            &["tool.mcp.recall_memory", "tool.calculator"],
            0,
            2,
            5_000,
        );
        assert!(gated.pad.p < plain.pad.p, "审批等待是负向微调");
        assert!(recalled.pad.p > plain.pad.p, "召回命中是正向微调");
        // 失败回合:诚实读作摩擦,p 落负。
        let failed = synth.turn_failed(&turn_key(), T0 + 60_000);
        assert!(failed.pad.p < 0.0);
    }

    #[test]
    fn idle_heartbeat_decays_pad_and_settles_into_dreaming() {
        let mut synth = PresenceSynthesizer::new();
        let busy = run_turn(
            &mut synth,
            &["tool.repo", "tool.shell", "tool.repo"],
            0,
            2,
            40_000,
        );
        assert_eq!(busy.stance, EmberCognitiveStance::DeepCodingFocus);

        // 5 分钟无交互:深焦衰减回 attentive;PAD 向 baseline 收缩。
        let after_quiet = synth.heartbeat(T0 + 40_000 + IDLE_FOCUS_DECAY_MS);
        assert_eq!(after_quiet.stance, EmberCognitiveStance::AttentivePresence);
        assert!(after_quiet.pad.a < busy.pad.a, "心跳拍上 arousal 必须衰减");

        // 30 分钟无交互:落入 dreaming_consolidation。
        let dreaming = synth.heartbeat(T0 + 40_000 + IDLE_DREAMING_MS);
        assert_eq!(dreaming.stance, EmberCognitiveStance::DreamingConsolidation);

        // 从启动就无任何回合:他一直在后台整合。
        let mut fresh = PresenceSynthesizer::new();
        let first_tick = fresh.heartbeat(T0);
        assert_eq!(
            first_tick.stance,
            EmberCognitiveStance::DreamingConsolidation
        );
    }

    #[test]
    fn dominant_label_tracks_pad_signs() {
        assert_eq!(
            dominant_label(PresencePad {
                p: 0.2,
                a: -0.1,
                d: 0.3
            }),
            "serene"
        );
        assert_eq!(
            dominant_label(PresencePad {
                p: 0.2,
                a: 0.4,
                d: 0.0
            }),
            "engaged"
        );
        assert_eq!(
            dominant_label(PresencePad {
                p: -0.2,
                a: 0.4,
                d: 0.0
            }),
            "strained"
        );
        assert_eq!(
            dominant_label(PresencePad {
                p: -0.2,
                a: -0.4,
                d: 0.0
            }),
            "subdued"
        );
        assert_eq!(
            dominant_label(PresencePad {
                p: 0.05,
                a: 0.0,
                d: 0.0
            }),
            "neutral"
        );
    }

    // ③ 频率纪律

    #[test]
    fn heartbeat_cadence_respects_the_frequency_contract() {
        // 契约: 低频心跳 ≤ 每 60s —— 即间隔不得短于 60s。
        assert!(HEARTBEAT_INTERVAL_SECS >= 60);
    }

    #[test]
    fn initiative_budget_enforces_daily_cap() {
        let mut synth = PresenceSynthesizer::new();
        // 默认限量: 单用户日常 ≤ 少量次/天,宁少勿多。
        assert!(INITIATIVE_DAILY_CAP <= 5);
        for expected in [true, true, true, false] {
            assert_eq!(synth.try_take_initiative(T0), expected);
        }
        assert_eq!(synth.initiative_used_in_window(), INITIATIVE_DAILY_CAP);
        // 窗口滚动后恢复。
        assert!(synth.try_take_initiative(T0 + INITIATIVE_WINDOW_MS));
    }

    // 总线接线

    #[tokio::test]
    async fn runtime_events_become_presence_frames_on_the_bus() {
        let bus = EventBus::new(16);
        let service = PresenceService::new(bus.clone());
        let mut receiver = bus.subscribe();
        let sink: &dyn RuntimeEventSink = service.as_ref();

        let session = SessionId::new();
        let trace = TraceId::new();
        sink.emit(RuntimeEvent::TurnStarted {
            session,
            request: RequestId::new(),
            trace,
        });
        sink.emit(RuntimeEvent::Trace {
            session,
            trace,
            at: Timestamp::from_epoch_millis(T0).unwrap(),
            event: TraceEvent::CapabilityDispatched {
                capability: CapabilityId::new("tool.mcp.recall_memory").unwrap(),
                tool_call_id: "call_1".into(),
                round: 1,
            },
        });
        sink.emit(RuntimeEvent::TurnCompleted {
            session,
            request: RequestId::new(),
            trace,
            rounds: 1,
            served_by: CapabilityId::new("provider.fake").unwrap(),
        });

        let frame = receiver.recv().await.unwrap();
        assert_eq!(frame.event, "presence_state");
        assert_eq!(frame.data["type"], "presence_state");
        assert_eq!(frame.data["significance"], "turn");
        assert_eq!(frame.data["source"]["kind"], "heuristic_v0");
        assert_eq!(frame.data["stance"], "attentive_presence");
        assert!(frame.data["at"].as_i64().unwrap() > 0);
        // 召回命中的正向微调真实到达线上帧。
        assert!(frame.data["pad"]["p"].as_f64().unwrap() > 0.1);
    }

    #[tokio::test]
    async fn heartbeat_frames_reach_bus_subscribers() {
        let bus = EventBus::new(16);
        let service = PresenceService::new(bus.clone());
        let mut receiver = bus.subscribe();

        service.emit_heartbeat();

        let frame = receiver.recv().await.unwrap();
        assert_eq!(frame.event, "presence_state");
        assert_eq!(frame.data["significance"], "heartbeat");
        assert_eq!(frame.data["stance"], "dreaming_consolidation");
    }
}
