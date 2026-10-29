# ADR-0021: Consolidate the independent reactive library slice

Status: accepted for the bounded extract authorized by the owner's D4 mandate on 2026-10-29; broader UI architecture remains WIP in PR #78.

The original UI topic contains a separate workspace, renderer, DOM backend, demo, screenshots and historical evidence. Root Kernel CI does not verify that workspace. The D4 reconciliation attempt against protected main is recorded separately; this extract imports only `apeireth-ui-core` and its unchanged tests.

The crate remains a separate workspace at `crates-ui/`, with no external dependencies and no path edge to or from the root workspace. Its package version is copied from the existing root value; this does not change release authority or the root version. Root member counts, dependency budgets, LOCKED keys and ratchets stay intact. The architecture contract registers the path and component trait; an independent workflow runs check, clippy, format, native tests and compile-fail documentation tests. Root checks cannot substitute for these checks.

Scope is **IMPLEMENTED**: signals, derived values, effects and reactive scheduling. Its `Runtime` schedules reactive UI effects; it does not execute AgentTask or provide another Kernel backend. It has no production frontend consumer, DOM/GPU/browser integration or hardware validation in this slice. The thread-local convenience runtime remains prototype/test support; callers must own a runtime and dispose state deliberately. File size in this independent library is outside the root workspace scanner, as in the source topic; no root size budget is increased.

Rejected alternatives: wholesale merge of the conflicting UI mega-PR would combine unrelated rendering and historical evidence; pretending root gates validate UI would omit its tests; introducing renderer/GPU dependencies now would expand the scope. The remaining UI work stays open with the original published tip preserved. Any later host integration needs its own bounded PR, checks and readiness evidence.
