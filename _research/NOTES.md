# Research scratch notes (subagent)

## AGT (microsoft/agent-governance-toolkit) — 6,428 stars, default branch main, pushed 2026-10-13
Layout: policy-engine/ (Rust ACS host + core), agent-governance-python/ (agent-os 1138 files, agent-mesh 560, agent-sre 275, agent-hypervisor 104, agent-sandbox 42, agent-compliance 67, agt-policies 48), agent-governance-typescript/dotnet/golang/rust, docs/adr (33 ADRs).
Policy engine upstream = responsibleai/agent-control-spec (ACS); agent-hooks defines interception contract; AGT keeps AgentControl host APIs + framework adapters.

## Serena (oraios/serena) — 30,256 stars, default branch main, pushed 2026-10-14
Layout: src/serena (agent toolkit), src/solidlsp (LSP abstraction, 80+ language servers), test/ 783 files. v2 REPL-based agent interface.
