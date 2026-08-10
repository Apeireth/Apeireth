# apeireth-supervisor

Process-level supervision tree: PID 1 + 5 sub-supervisors + 3 restart strategies + actor mailbox.

## Architecture

```
                ┌─────────────────────┐
                │  PidOneSupervisor   │  (root, never restarted)
                │  plan_version: u64  │
                └──────────┬──────────┘
                           │
        ┌─────────┬────────┼────────┬─────────┐
        ▼         ▼        ▼        ▼         ▼
      Core   Cognition  Council  Upgrade   Plugin
       (3)     (4)        (7)      (3)       (4)
                          21 child specs total
```

## Strategy matrix

| Kind      | Strategy      | Count | Why                                    |
|-----------|---------------|-------|----------------------------------------|
| Core      | OneForOne     | 3     | organs are independent                 |
| Cognition | RestForOne    | 4     | meta depends on reasoning depends on intuition depends on cognition |
| Council   | OneForOne     | 7     | advisors are independent voters        |
| Upgrade   | Transient     | 3     | upgrade failures should NOT auto-restart (avoid half-applied state) |
| Plugin    | OneForOne     | 4     | plugins are independent                |

## Hard constraints

- **Pure tokio::process::Command** — no PyO3, no external scripts
- **PID 1 has NO restart_strategy** (struct field)
- **Windows**: child processes use `cmd /c exit <code>`
- **Unix**: child processes use `true` / `false`

## Files

| Path | Purpose |
|---|---|
| `src/lib.rs` | public re-exports |
| `src/pid_one.rs` | `PidOneSupervisor` (root, never restartable) |
| `src/supervisor.rs` | `SubSupervisorKind` + `default_plan()` |
| `src/strategy.rs` | `RestartStrategy` (OneForOne / RestForOne / Transient) |
| `src/child.rs` | `ChildSpec` + restart-window + snapshot_id |
| `src/actor.rs` | tokio-mpsc actor + `CounterActor` for tests |
| `examples/supervisor_demo.rs` | boots PID 1, prints plan, exercises actor |
| `tests/supervisor_integration.rs` | smoke tests |
| `tests/supervisor_q14.rs` | Q14 4-dimension acceptance tests |

## ponytail markers

Every file in this crate carries `ponytail:` comments naming the ceiling and
upgrade path. The most important ceilings:

- **Child runtime** (PID polling + signal handling) — not implemented; tests
  exercise the *spec contract*, the runtime will be a separate crate that
  consumes `ChildSpec`.
- **Snapshot rollback** — `snapshot_id` is the *target*, the actual rollback
  logic lives in `apeireth-upgrade`'s switchover engine.
- **Council evaluation trigger** — supervisor provides the slots; the
  evaluation logic lives in `apeireth-council`.