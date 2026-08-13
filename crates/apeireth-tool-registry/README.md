# apeireth-tool-registry

> Apeireth R17 鎴樺焦 2-1: 宸ュ叿娉ㄥ唽涓績 (6 绫?enum + 5 杞存浜?+ token 棰勭畻涓夊眰 + notify 鐑姞杞? VCP 鍊熼壌 搂6.2.1 #12/#13 + 搂6.2.2 #15 + agentManager.js chokidar)

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

ToolKind::as_vcp_str -> as_legacy_str; ToolKind::from_vcp_str -> from_legacy_str; Category::from_vcp_name -> from_legacy_name. 3 test fn names updated. 100 tests pass.
