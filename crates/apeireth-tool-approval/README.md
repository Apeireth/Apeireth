# apeireth-tool-approval

> Apeireth R17 鎴樺焦 2-3: 宸ュ叿瀹℃壒 (5 瑙勫垯 + 5 鍒嗛挓绐楀彛 + fuzzy matching 闆嗘垚, VCP 鍊熼壌 toolApprovalManager.js)

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
## R166 public API deep cleanup

`BORROWED_VCP_FIELDS` -> `BORROWED_LEGACY_FIELDS`. 62 tests pass.
