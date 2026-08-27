# R0T Repository Topology Audit

> **现状 (2026-08-27)**：本文是 v1 时代（master 线/86-crate）或 reconstruct_v2 过程中的历史快照，正文保留原样。当前基线：默认分支 `main`、13-crate 工作区（`crates/foundation|engine|capabilities|adapters`，见根 `ARCHITECTURE.md` 与 `docs/01-architecture/architecture.md`）、tag `v2.0.0-alpha.1` @ `d6910cf7`；旧 86-crate 代码整体在 `legacy/`（workspace exclude）；v2 下一步见根 `ROADMAP.md` §4。补充：文中 `apeireth-companion → crates/modules/companion` 规划未执行，当前无 `crates/modules/` 产品 crate。

| Package | Old path | Classification | New path | Reason |
| --- | --- | --- | --- | --- |
| apeireth-acp | crates/apeireth-acp | legacy/donor | legacy/donor/apeireth-acp | Historical donor or legacy implementation; not canonical product owner |
| apeireth-action | crates/apeireth-action | legacy/donor | legacy/donor/apeireth-action | Historical donor or legacy implementation; not canonical product owner |
| apeireth-agent | crates/apeireth-agent | legacy/donor | legacy/donor/apeireth-agent | Historical donor or legacy implementation; not canonical product owner |
| apeireth-api | crates/apeireth-api | legacy/donor | legacy/donor/apeireth-api | Historical donor or legacy implementation; not canonical product owner |
| apeireth-arbitration | crates/apeireth-arbitration | legacy/donor | legacy/donor/apeireth-arbitration | Historical donor or legacy implementation; not canonical product owner |
| apeireth-asi | crates/apeireth-asi | legacy/donor | legacy/donor/apeireth-asi | Historical donor or legacy implementation; not canonical product owner |
| apeireth-bench | crates/apeireth-bench | legacy/donor | legacy/donor/apeireth-bench | Historical donor or legacy implementation; not canonical product owner |
| apeireth-blueprint-impl | crates/apeireth-blueprint-impl | legacy/donor | legacy/donor/apeireth-blueprint-impl | Historical donor or legacy implementation; not canonical product owner |
| apeireth-bus | crates/apeireth-bus | legacy/donor | legacy/donor/apeireth-bus | Historical donor or legacy implementation; not canonical product owner |
| apeireth-central | crates/apeireth-central | legacy/donor | legacy/donor/apeireth-central | Historical donor or legacy implementation; not canonical product owner |
| apeireth-cli | crates/apeireth-cli | adapters | crates/adapters/cli | Canonical owner per ARCHITECTURE.md / M1-M3 freeze |
| apeireth-cognition | crates/apeireth-cognition | legacy/donor | legacy/donor/apeireth-cognition | Historical donor or legacy implementation; not canonical product owner |
| apeireth-companion | crates/apeireth-companion | modules | crates/modules/companion | Canonical owner per ARCHITECTURE.md / M1-M3 freeze |
| apeireth-config | crates/apeireth-config | legacy/donor | legacy/donor/apeireth-config | Historical donor or legacy implementation; not canonical product owner |
| apeireth-consciousness | crates/apeireth-consciousness | legacy/donor | legacy/donor/apeireth-consciousness | Historical donor or legacy implementation; not canonical product owner |
| apeireth-constraint | crates/apeireth-constraint | legacy/donor | legacy/donor/apeireth-constraint | Historical donor or legacy implementation; not canonical product owner |
| apeireth-context-fold | crates/apeireth-context-fold | legacy/donor | legacy/donor/apeireth-context-fold | Historical donor or legacy implementation; not canonical product owner |
| apeireth-core | crates/apeireth-core | foundation | crates/foundation/core | Canonical owner per ARCHITECTURE.md / M1-M3 freeze |
| apeireth-council | crates/apeireth-council | legacy/donor | legacy/donor/apeireth-council | Historical donor or legacy implementation; not canonical product owner |
| apeireth-credentials | crates/apeireth-credentials | foundation | crates/foundation/credentials | Canonical owner per ARCHITECTURE.md / M1-M3 freeze |
| apeireth-cron | crates/apeireth-cron | legacy/donor | legacy/donor/apeireth-cron | Historical donor or legacy implementation; not canonical product owner |
| apeireth-environment | crates/apeireth-environment | legacy/donor | legacy/donor/apeireth-environment | Historical donor or legacy implementation; not canonical product owner |
| apeireth-eval | crates/apeireth-eval | legacy/donor | legacy/donor/apeireth-eval | Historical donor or legacy implementation; not canonical product owner |
| apeireth-evolution | crates/apeireth-evolution | legacy/donor | legacy/donor/apeireth-evolution | Historical donor or legacy implementation; not canonical product owner |
| apeireth-experience | crates/apeireth-experience | legacy/donor | legacy/donor/apeireth-experience | Historical donor or legacy implementation; not canonical product owner |
| apeireth-extension | crates/apeireth-extension | legacy/donor | legacy/donor/apeireth-extension | Historical donor or legacy implementation; not canonical product owner |
| apeireth-gateway | crates/apeireth-gateway | adapters | crates/adapters/gateway | Canonical owner per ARCHITECTURE.md / M1-M3 freeze |
| apeireth-governance | crates/apeireth-governance | foundation | crates/foundation/governance | Canonical owner per ARCHITECTURE.md / M1-M3 freeze |
| apeireth-graph | crates/apeireth-graph | legacy/donor | legacy/donor/apeireth-graph | Historical donor or legacy implementation; not canonical product owner |
| apeireth-graph-primitive | crates/apeireth-graph-primitive | legacy/donor | legacy/donor/apeireth-graph-primitive | Historical donor or legacy implementation; not canonical product owner |
| apeireth-guard | crates/apeireth-guard | legacy/donor | legacy/donor/apeireth-guard | Historical donor or legacy implementation; not canonical product owner |
| apeireth-host | crates/apeireth-host | legacy/donor | legacy/donor/apeireth-host | Historical donor or legacy implementation; not canonical product owner |
| apeireth-http-client | crates/apeireth-http-client | legacy/donor | legacy/donor/apeireth-http-client | Historical donor or legacy implementation; not canonical product owner |
| apeireth-i18n | crates/apeireth-i18n | legacy/donor | legacy/donor/apeireth-i18n | Historical donor or legacy implementation; not canonical product owner |
| apeireth-integration-e2e | crates/apeireth-integration-e2e | legacy/donor | legacy/donor/apeireth-integration-e2e | Historical donor or legacy implementation; not canonical product owner |
| apeireth-lark | crates/apeireth-lark | legacy/donor | legacy/donor/apeireth-lark | Historical donor or legacy implementation; not canonical product owner |
| apeireth-library-governance | crates/apeireth-library-governance | legacy/donor | legacy/donor/apeireth-library-governance | Historical donor or legacy implementation; not canonical product owner |
| apeireth-life-force | crates/apeireth-life-force | legacy/donor | legacy/donor/apeireth-life-force | Historical donor or legacy implementation; not canonical product owner |
| apeireth-livekit | crates/apeireth-livekit | legacy/donor | legacy/donor/apeireth-livekit | Historical donor or legacy implementation; not canonical product owner |
| apeireth-llm-iface | crates/apeireth-llm-iface | legacy/donor | legacy/donor/apeireth-llm-iface | Historical donor or legacy implementation; not canonical product owner |
| apeireth-mcp | crates/apeireth-mcp | legacy/donor | legacy/donor/apeireth-mcp | Historical donor or legacy implementation; not canonical product owner |
| apeireth-memory | crates/apeireth-memory | engine | crates/engine/memory | Canonical owner per ARCHITECTURE.md / M1-M3 freeze |
| apeireth-memory-extensions | crates/apeireth-memory/extensions | legacy/donor | legacy/donor/apeireth-memory-extensions | Historical donor or legacy implementation; not canonical product owner |
| apeireth-motivation | crates/apeireth-motivation | legacy/donor | legacy/donor/apeireth-motivation | Historical donor or legacy implementation; not canonical product owner |
| apeireth-naming-v05 | crates/apeireth-naming-v05 | legacy/donor | legacy/donor/apeireth-naming-v05 | Historical donor or legacy implementation; not canonical product owner |
| apeireth-onion | crates/apeireth-onion | legacy/donor | legacy/donor/apeireth-onion | Historical donor or legacy implementation; not canonical product owner |
| apeireth-perception | crates/apeireth-perception | legacy/donor | legacy/donor/apeireth-perception | Historical donor or legacy implementation; not canonical product owner |
| apeireth-pipeline | crates/apeireth-pipeline | legacy/donor | legacy/donor/apeireth-pipeline | Historical donor or legacy implementation; not canonical product owner |
| apeireth-pipeline-g5 | crates/apeireth-pipeline-g5 | legacy/donor | legacy/donor/apeireth-pipeline-g5 | Historical donor or legacy implementation; not canonical product owner |
| apeireth-plugin | crates/apeireth-plugin | foundation | crates/foundation/plugin | Canonical owner per ARCHITECTURE.md / M1-M3 freeze |
| apeireth-protocol | crates/apeireth-protocol | foundation | crates/foundation/protocol | Canonical owner per ARCHITECTURE.md / M1-M3 freeze |
| apeireth-provider | crates/apeireth-provider | engine | crates/engine/provider | Canonical owner per ARCHITECTURE.md / M1-M3 freeze |
| apeireth-pybridge | crates/apeireth-pybridge | legacy/donor | legacy/donor/apeireth-pybridge | Historical donor or legacy implementation; not canonical product owner |
| apeireth-rate-limiter | crates/apeireth-rate-limiter | legacy/donor | legacy/donor/apeireth-rate-limiter | Historical donor or legacy implementation; not canonical product owner |
| apeireth-repo-tools | crates/apeireth-repo-tools | legacy/donor | legacy/donor/apeireth-repo-tools | Historical donor or legacy implementation; not canonical product owner |
| apeireth-runtime | crates/apeireth-runtime | engine | crates/engine/runtime | Canonical owner per ARCHITECTURE.md / M1-M3 freeze |
| apeireth-sdk | crates/apeireth-sdk | adapters | crates/adapters/sdk | Canonical owner per ARCHITECTURE.md / M1-M3 freeze |
| apeireth-skills | crates/apeireth-skills | legacy/donor | legacy/donor/apeireth-skills | Historical donor or legacy implementation; not canonical product owner |
| apeireth-sovereignty | crates/apeireth-sovereignty | legacy/donor | legacy/donor/apeireth-sovereignty | Historical donor or legacy implementation; not canonical product owner |
| apeireth-state | crates/apeireth-state | legacy/donor | legacy/donor/apeireth-state | Historical donor or legacy implementation; not canonical product owner |
| apeireth-stock | crates/apeireth-stock | legacy/donor | legacy/donor/apeireth-stock | Historical donor or legacy implementation; not canonical product owner |
| apeireth-storage | crates/apeireth-storage | engine | crates/engine/storage | Canonical owner per ARCHITECTURE.md / M1-M3 freeze |
| apeireth-supervisor | crates/apeireth-supervisor | legacy/donor | legacy/donor/apeireth-supervisor | Historical donor or legacy implementation; not canonical product owner |
| apeireth-team-lead | crates/apeireth-team-lead | legacy/donor | legacy/donor/apeireth-team-lead | Historical donor or legacy implementation; not canonical product owner |
| apeireth-telemetry | crates/apeireth-telemetry | legacy/donor | legacy/donor/apeireth-telemetry | Historical donor or legacy implementation; not canonical product owner |
| apeireth-test | crates/apeireth-test | legacy/donor | legacy/donor/apeireth-test | Historical donor or legacy implementation; not canonical product owner |
| apeireth-tool-approval | crates/apeireth-tool-approval | legacy/donor | legacy/donor/apeireth-tool-approval | Historical donor or legacy implementation; not canonical product owner |
| apeireth-tool-browser | crates/apeireth-tool-browser | legacy/donor | legacy/donor/apeireth-tool-browser | Historical donor or legacy implementation; not canonical product owner |
| apeireth-tool-codesearch | crates/apeireth-tool-codesearch | legacy/donor | legacy/donor/apeireth-tool-codesearch | Historical donor or legacy implementation; not canonical product owner |
| apeireth-tool-fetch | crates/apeireth-tool-fetch | legacy/donor | legacy/donor/apeireth-tool-fetch | Historical donor or legacy implementation; not canonical product owner |
| apeireth-tool-filesystem | crates/apeireth-tool-filesystem | legacy/donor | legacy/donor/apeireth-tool-filesystem | Historical donor or legacy implementation; not canonical product owner |
| apeireth-tool-image-gen | crates/apeireth-tool-image-gen | legacy/donor | legacy/donor/apeireth-tool-image-gen | Historical donor or legacy implementation; not canonical product owner |
| apeireth-tool-image-process | crates/apeireth-tool-image-process | legacy/donor | legacy/donor/apeireth-tool-image-process | Historical donor or legacy implementation; not canonical product owner |
| apeireth-tool-registry | crates/apeireth-tool-registry | legacy/donor | legacy/donor/apeireth-tool-registry | Historical donor or legacy implementation; not canonical product owner |
| apeireth-tool-runtime | crates/apeireth-tool-runtime | legacy/donor | legacy/donor/apeireth-tool-runtime | Historical donor or legacy implementation; not canonical product owner |
| apeireth-tool-search | crates/apeireth-tool-search | legacy/donor | legacy/donor/apeireth-tool-search | Historical donor or legacy implementation; not canonical product owner |
| apeireth-tool-shell | crates/apeireth-tool-shell | legacy/donor | legacy/donor/apeireth-tool-shell | Historical donor or legacy implementation; not canonical product owner |
| apeireth-tools | crates/apeireth-tools | legacy/donor | legacy/donor/apeireth-tools | Historical donor or legacy implementation; not canonical product owner |
| apeireth-tools-canonical | crates/apeireth-tools-canonical | capabilities | crates/capabilities/tools | Canonical owner per ARCHITECTURE.md / M1-M3 freeze |
| apeireth-tui | crates/apeireth-tui | legacy/donor | legacy/donor/apeireth-tui | Historical donor or legacy implementation; not canonical product owner |
| apeireth-tui-e2e | crates/apeireth-tui-e2e | legacy/donor | legacy/donor/apeireth-tui-e2e | Historical donor or legacy implementation; not canonical product owner |
| apeireth-upgrade | crates/apeireth-upgrade | legacy/donor | legacy/donor/apeireth-upgrade | Historical donor or legacy implementation; not canonical product owner |
| apeireth-value | crates/apeireth-value | legacy/donor | legacy/donor/apeireth-value | Historical donor or legacy implementation; not canonical product owner |
| apeireth-vector | crates/apeireth-vector | legacy/donor | legacy/donor/apeireth-vector | Historical donor or legacy implementation; not canonical product owner |
| apeireth-verify | crates/apeireth-verify | legacy/donor | legacy/donor/apeireth-verify | Historical donor or legacy implementation; not canonical product owner |
| apeireth-voice | crates/apeireth-voice | legacy/donor | legacy/donor/apeireth-voice | Historical donor or legacy implementation; not canonical product owner |
| apeireth-web | crates/apeireth-web | legacy/donor | legacy/donor/apeireth-web | Historical donor or legacy implementation; not canonical product owner |
| apeireth-wiki | crates/apeireth-wiki | legacy/donor | legacy/donor/apeireth-wiki | Historical donor or legacy implementation; not canonical product owner |
| apeireth-workflow | crates/apeireth-workflow | legacy/donor | legacy/donor/apeireth-workflow | Historical donor or legacy implementation; not canonical product owner |
| release-tools | crates/release-tools | legacy/donor | legacy/donor/release-tools | Historical donor or legacy implementation; not canonical product owner |
