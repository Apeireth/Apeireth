# R237 -- emotion decay -> bus closed loop

## Problem
R235 makes \untime.run_one_cycle()\ call \EmotionEngine::auto_decay()\ at the beginning, but the
result is silently absorbed: nothing observable downstream knows \"a decay just happened, and here's
how big it was.\"  Subscribers (search indexers, arbitration log, group chat agents, future TUI
panels) cannot react to emotional drift.

## Solution: emit \DecaySnapshot\ on auto_decay path

### 1. \peireth-consciousness::emotion\

New public type:

\\\ust
pub struct DecaySnapshot {
    pub timestamp_ms: i64,
    pub elapsed_secs: f32,    // seconds since last event; 0 = no-op
    pub pad_before: Pad,
    pub pad_after: Pad,
}
impl DecaySnapshot {
    pub fn drift(&self) -> f32;             // 3D PAD distance
    pub fn is_significant(&self, min: f32) -> bool;
}
\\\

\EmotionEngine\:
- new field \last_decay: Option<DecaySnapshot>\
- \uto_decay\ / \uto_decay_at\ write \last_decay\ AFTER capturing pad before decay
- new accessors \last_decay()\ (peek) and \	ake_decay_snapshot()\ (clone)

\\\	ext
auto_decay
  |
  v
pad_before = self.pad            <-- BEFORE decay
self.decay(elapsed_secs)
self.last_decay = Some(DecaySnapshot{...pad_before..., pad_after: self.pad})
return elapsed_secs
\\\

### 2. \peireth-runtime\

\RuntimeConfig\ new fields (all with safe defaults):
\\\ust
pub emit_decay_bus: bool,                // default true
pub decay_emit_min_elapsed_secs: f32,    // default 1.0
pub decay_emit_min_drift: f32,           // default 0.01
\\\

\un_one_cycle\ opens with:
\\\ust
let decay_snap: Option<DecaySnapshot> = {
    let mut eng = self.emotion.lock();
    eng.auto_decay();
    eng.take_decay_snapshot()
};
if let (true, Some(snap)) = (self.config.emit_decay_bus, decay_snap.as_ref()) {
    if snap.elapsed_secs >= self.config.decay_emit_min_elapsed_secs
        && snap.is_significant(self.config.decay_emit_min_drift)
    {
        let payload = serde_json::to_string(snap).unwrap_or_default();
        let event = RuntimeEvent::new(...);
        let _ = self.bus.publish_multi(ChannelSet::BOTH, "emotion.decay", ...).await;
    }
}
\\\

### 3. Why locked-emotion snapshot copy (not sink trait)

Considered adding \DecaySink\ trait on consciousness so engine.emit(...). Avoided because:
- \EmotionEngine\ stays free of bus dependency (cleaner crate DAG).
- runtime already pays the cost of locking engine (it owns the Arc<Mutex<…>>).
- consumers of \last_decay\ are likely runtime/tui/council all of which already know about \us\.

\	ake_decay_snapshot\ is deliberately non-consuming — last_decay stays Some so multiple
downstream observers all see the same snapshot.

## Tests (10 new tests, all passing)

\peireth-consciousness\: r237_01..r237_06 (6 tests, DecaySnapshot + take/peek + serde roundtrip)
\peireth-runtime\: r237_01..r237_04 (4 tests, decay publish + threshold guards + config defaults)

Total cumulative: ~6310 tests pass.

## Design notes / open questions

- Backwards compatibility: pure addition, no API change, runtime cfg defaults keep old behavior.
- Determinism: emission depends on wallclock elapsed; tests using \	okio::sleep\ for determinism
  may flake — future iterations could expose test-only emit-on-apply helper.
- Stats visibility: \us.stats()\ has no emotion-channel breakdown yet. Followup could expose
  per-topic sent counters so monitoring knows if emotion.decay is flowing.

## Files touched

- \crates/apeireth-consciousness/src/emotion.rs\ (+1 struct, +3 methods, +6 tests)
- \crates/apeireth-consciousness/src/lib.rs\ (+1 re-export)
- \crates/apeireth-runtime/src/lib.rs\ (+3 config fields, +1 publish block, +4 tests)