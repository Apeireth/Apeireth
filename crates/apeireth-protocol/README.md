# apeireth-protocol

> Apeireth R17 鎴樺焦 1-1: LLM 鍗忚褰掍竴鍖栧眰 (OpenAI Chat / OpenAI Responses / Anthropic Messages / Gemini), 瀛楁绾у€熼壌 VCP protocolBridge.js 鐪熶唬鐮?
keywords = [

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

MessageRole::from_vcp -> from_legacy_str; ContentPart::from_vcp -> from_legacy_value. 13 test fns + 3 caller sites updated (apeireth-api, router_demo example). 96 tests still pass.
