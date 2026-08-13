# R245 -- bus MessagePriority tag + stats bucket

## Problem
Bus publish is uniform: every message takes same path, no quality-of-service hint.
External observability cannot tell -- of N publishes -- how many were high-importance
(Self-Disable / error escalation) vs low (telemetry / heartbeat).

## Solution: per-message priority tag + 3 counter buckets

### New struct
\\\ust
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessagePriority {
    High,
    Normal, // default
    Low,
}
\\\

### BusMessage additions
- new field \priority: MessagePriority\ (default Normal, serde-friendly)
- builder: \with_priority(p: MessagePriority) -> Self\
- \map()\ carries priority over

### BusStats additions
- \high_priority: AtomicU64\
- \
ormal_priority: AtomicU64\
- \low_priority: AtomicU64\
- snapshot includes all 3

### Behavior
\L0Bus::publish()\ increments the priority counter on each call.

## Tests (3 new tests pass)
- r245_01: default priority = Normal
- r245_02: with_priority builder transitions correctly
- r245_03: 1H + 2N + 3L publishes -> bucket counts match

## Files
- \crates/apeireth-bus/src/lib.rs\ (+MessagePriority +3 fields +builder +3 tests)
- \crates/apeireth-bus/src/l0.rs\ (+1 publish-time counter inc block)

cumulative: ~6336 tests pass.