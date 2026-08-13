# apeireth-bus

> apeireth-bus 鈥?5 灞傞€氫俊鎬荤嚎 (L0 inproc / L1 UDS / L2 pipe / L3 gRPC / L4 WebSocket) + pub-sub/req-rep/streaming + 鍙嶈儗鍘?+ Trace ID 閾捐矾杩借釜 (round15-02)

## Status

Part of the Apeireth workspace (74 active crate after R128 94鈫?5 merge).

**No-fake**: every public type or trait documented in this crate is real.
**Run-no-fear**: cargo check --workspace passes (0 errors).

## Where to start

- Cargo.toml: see [dependencies](Cargo.toml) for upstream crate.
- src/lib.rs: see top-level doc comment for module-level overview.

## See also

- [Apeireth conventions](../../docs/conventions/README.md)
- [Apeireth roadmap](../../docs/pages-source/roadmap.md)

---

_Auto-generated README per R128 batch (2026-08-12). Last-modified tracked in git log._
## R164 public API cleanup

Channel::as_vcp_str -> as_legacy_str. 1 source + 1 test update. 24 tests pass.
