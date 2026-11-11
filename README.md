# Apeireth — 阿佩瑞斯

<div align="center">

<img src="previews/apeireth-mark.svg" alt="Apeireth double-diamond mark" width="72">

### A digital companion that grows with you

**A free, open-source personal AI assistant that connects conversation, memory, and action.**

[![Status](https://img.shields.io/badge/status-pre--release-orange.svg)](ROADMAP.md)
[![License](https://img.shields.io/badge/license-Apache--2.0--OR--MIT-blue.svg)](LICENSE)

**[English](README.md) | [简体中文](README.zh-CN.md)**

</div>

<a id="nav"></a>

**[Capabilities](#features) · [Getting started](#start) · [Capabilities and privacy](#boundaries) · [Community](#sources) · [Engineering and verification](#hood) · [Our vision](#acts)**

> [!IMPORTANT]
> **Apeireth is in development and pre-release; the new desktop frontend is still being built.** This repository currently provides the backend, command-line tools, and integration interfaces. A backend capability does not mean the new desktop interface already offers that experience.
>
> [Releases](https://github.com/Apeireth/Apeireth/releases) contains earlier pre-release installers. Each belongs to its own tag and release date; it does not represent the current `main` branch or the new frontend. Check the release notes, platform requirements, and checksum files.

Apeireth aims to let a conversation continue as a traceable memory, then become an action within clearly defined permissions. You can inspect how it works, review what it stores, and help improve it. The software is free and open source; cloud model providers may charge separately for their services.

<a id="features"></a>

## What the backend can do today

| What you want to do | Current capability | Before you start |
|---|---|---|
| Continue a conversation | Model adapters, context management, and a tool-calling loop; HTTP/SSE chat supports incremental output | Configure a model service first; adapters without streaming support return output after completion |
| Keep information and preferences | Memory writes, hybrid retrieval, preference recall, and memory records you can inspect and correct | Both memory and model inference can be wrong; verify important information |
| Organize and process files | Local file reading, search, repository tools, and document operations such as Markdown → DOCX export | Document tools and write capabilities require configuration; workspace boundaries and governance policies still apply |
| Inspect actions | Risk assessment, approval workflows, execution records, and auditing | Capability switches, authorization, and host configuration determine what can run; not every tool is enabled by default |
| Extend your workflow | Optional backend capabilities such as the MCP bridge and task board | Disabled by default; follow the capability documentation to enable them. This does not imply a corresponding new graphical interface is available |

For detailed status and evidence, see the [capability matrix](docs/03-reference/capabilities-matrix.md) and [verifiable capability claims](docs/status/STATUS.md). For API integration, see the [gateway integration guide](docs/02-guides/v2-gateway-frontend-integration-spec.md). These sources distinguish code implementation, production wiring, default enablement, and validation on real hardware.

<a id="start"></a>

## Getting started

### If you want to use Apeireth

- Check the [releases page](https://github.com/Apeireth/Apeireth/releases) for assets that suit your platform and the project's release stage. The new frontend is unfinished; downloading a package does not currently provide every experience shown in the website preview.
- Ask installation or configuration questions in [Q&A](https://github.com/Apeireth/Apeireth/discussions/categories/q-a). You do not need to write code to participate.
- Before using a cloud model, read [Data and privacy](docs/02-guides/data-and-privacy.md) and your provider's pricing and data policies.

### If you want to develop or integrate

Install Git, the repository's pinned Rust toolchain, and your platform's build tools. See the [source build guide](INSTALL.md) and [rust-toolchain.toml](rust-toolchain.toml). Start with the command-line help:

```bash
git clone https://github.com/Apeireth/Apeireth.git
cd Apeireth
cargo run --locked -p apeireth-cli --bin apeireth -- --help
```

This command compiles the CLI and displays its help. The help path does not start a model task or require an API key. It is not a complete product installation or an end-to-end test. To start a conversation, configure a model service and credentials using the [configuration instructions](INSTALL.md). Keep keys out of Issues, screenshots, and commits.

To contribute code, documentation, or assets, read [CONTRIBUTING.md](CONTRIBUTING.md). The root [Cargo.toml](Cargo.toml) is the source of truth for the version and workspace members.

<a id="boundaries"></a>

## Capabilities and privacy boundaries

- **Running locally does not mean data stays on your computer.** When you use a cloud model, relevant context is sent to your configured provider. The model-context egress gate has production wiring but is disabled by default (`APEIRETH_ENABLE_EGRESS_GATE=1` enables it). It cannot promise that a provider will delete data already received. See [Data and privacy](docs/02-guides/data-and-privacy.md).
- **Memory is not a promise of perfect recall.** Retrieval, distillation, and model responses have limits. Evidence and history help you check results; they do not guarantee permanent, complete, or correct memory.
- **Governance and isolation have defined limits.** Approvals, tool allowlists, and platform isolation constrain actions; they do not guarantee risk-free software. See the [security model](docs/01-architecture/security.md) and [capability matrix](docs/03-reference/capabilities-matrix.md) for platform and tool boundaries.
- **Budgets require configuration.** Session token, tool-step, and cost quotas have no limits by default; you must configure them explicitly. Cost controls also depend on model pricing information and do not replace your provider's billing and account limits.
- **Default and optional capabilities are separate.** Core features such as memory and preference recall use different switches from optional shell, fetch, file-write, and MCP capabilities. Consult [capability matrix §0.2](docs/03-reference/capabilities-matrix.md) for configuration and defaults; do not enable every switch just to try the software.
- **Model adapters do not cover every protocol.** The current provider families are OpenAI-compatible, MiniMax, and Anthropic. The Gemini and OpenAI Responses protocol normalizers are not directly usable providers.
- **Some integrations remain unfinished.** The full-duplex WebSocket server and the third DSL onion rule layer do not yet have production wiring. The SDK supports HTTP tool calls, while WebSocket streaming and quota queries still return not-implemented errors. HTTP/SSE is a separate path. A local Mesh worker is not a public-network mesh product with encryption and peer discovery.

For specific evidence, switches, and limitations, consult the [capability claims](docs/status/claims.yaml) and current source. Design documents, historical delivery records, unit tests, and formal model proofs do not by themselves establish that the complete current product has passed validation on real hardware.

<a id="sources"></a>

## Community and official channels

Your first question is as welcome as your first contribution. See the [support guide](SUPPORT.md) to find the right place, and follow the [Code of Conduct](CODE_OF_CONDUCT.md).

| What you want to do | Where to go |
|---|---|
| Ask about usage or configuration | [Q&A](https://github.com/Apeireth/Apeireth/discussions/categories/q-a) |
| Discuss ideas or share experiences | [Ideas](https://github.com/Apeireth/Apeireth/discussions/categories/ideas) · [Show and tell](https://github.com/Apeireth/Apeireth/discussions/categories/show-and-tell) |
| Report a bug or request a specific feature | [Issue forms](https://github.com/Apeireth/Apeireth/issues/new/choose) |
| Improve code, documentation, or assets | [Contributing guide](CONTRIBUTING.md) |
| Report a product security vulnerability | [GitHub private vulnerability reporting](https://github.com/Apeireth/Apeireth/security/advisories/new) · [Security policy](SECURITY.md) |
| Report misconduct or appeal a decision | [Reporting channels in the Code of Conduct](CODE_OF_CONDUCT.md#报告与申诉渠道); the project's private contact channel is not yet active and remains to be confirmed |

A GitHub account is required to participate in discussions and submit feedback. Remove keys, private conversations, and identifying information before posting publicly. Do not put vulnerability details or sensitive misconduct reports in public Issues. `apeireth-security@apeireth.org` is not active; do not send reports to it.

Use [Apeireth/Apeireth Releases](https://github.com/Apeireth/Apeireth/releases) for release assets, and consult the verification files supplied with the relevant version. Project maintainers will not ask you to hand over API keys in public feedback or private support conversations. Website design previews and historical screenshots are not distribution channels.

<a id="hood"></a>

## Engineering and verification

One backend core handles conversation, memory, governance, and tool execution, with external access through the CLI and gateway. Most of it is written in Rust. Platform isolation uses operating-system interfaces and restricted `unsafe`/FFI implementations, so we do not present “zero unsafe across the repository” as a security guarantee.

| Area | Reading |
|---|---|
| Structure and design principles | [Architecture overview](docs/01-architecture/architecture.md) · [Philosophy and constraints](docs/01-architecture/philosophy.md) |
| Runtime, memory, models, and governance | [Core mechanisms explained](docs/02-guides/core-mechanisms-explained.md) · [Crate index](docs/03-reference/crates.md) |
| Wiring, defaults, and evidence | [Capability matrix](docs/03-reference/capabilities-matrix.md) · [Status claims](docs/status/STATUS.md) |
| Upcoming work | [ROADMAP.md](ROADMAP.md) · [CHANGELOG.md](CHANGELOG.md) |

<a id="trust"></a>

### Evidence you can inspect

[![Tests](https://img.shields.io/badge/tests-8846%20passed%20%7C%201%20failed%20(2026--10--31)-critical.svg?logo=checkmarx)](docs/03-reference/test-baseline.md)

This badge represents the **recorded full-suite test baseline from 2026-10-31**, including its failure. It is not a fresh run against the current commit, nor evidence that the desktop frontend or real model tasks have passed end-to-end testing.

- [Test baseline and reproduction](docs/03-reference/test-baseline.md): commands, toolchain, failure details, and historical records.
- [Validation by change scope](docs/02-guides/fast-feedback-gate.md): checks for everyday development. Full-suite closure uses `pwsh scripts/gate_full.ps1 -FailIfBusy`; no fixed completion time is promised.
- [Formal verification materials](research/verification/): specific models, properties, and assumptions. A proof of one property does not establish that every operation is safe.
- [Historical performance reproduction](reports/benchmark-reproduction.md): methods and results from those runs, not a performance guarantee for your device or the current commit.

One successful chat does not prove long-term memory is correct, and a screenshot does not prove a backend action ran. Review the inputs, execution records, outputs, and verification scope of the capability in question.

<a id="voice"></a>
<a id="acts"></a>

## Why a digital companion?

We want Apeireth to do more than answer the latest message. Important context should be able to carry forward: remembering what you allow it to retain, explaining what an action will do, and producing results whose origins you can inspect.

<details>
<summary>Three scenes from our vision: memory, quiet, and boundaries</summary>

The following is a creative and design vision, **not a list of current features or a live product demonstration**. Warming milk, caring for plants, and sensing a heartbeat are not promises of capabilities already delivered.

### Act I: Remembering the scattered days

Half a spoon of honey, a pothos you once mentioned, a small story left unfinished. We hope memory can reduce repetition in later conversations and make important fragments of context easier to find again.

### Act II: Quiet when you are busy

Not every moment needs a response. The companion we imagine knows when to offer help and respects quiet, concentration, and refusal. It does not need to pretend to have a human inner life.

### Act III: Keeping the decision with you

An ability to act should come with clear permissions, reasons, and costs. Before a suggestion becomes an action, its boundaries should be clear. Failures and uncertainty should be reported honestly.

Vision stories: [*Apeireth* — remembering one person](docs/archive/stage1/阿佩瑞斯-未来愿景小说.txt) · [*Echoes* — remembering a civilization](docs/vision/遗声-未来愿景小说2.txt).

</details>

<a id="manifesto"></a>

**Our direction:** make memory inspectable and correctable, and keep action under explicit governance. The long-term ambition guides the work; each delivery must describe what is actually there. See the [founding design](docs/01-architecture/founding-design-v2.md) and [project philosophy](docs/01-architecture/philosophy.md) for the full design.

The earlier [desktop screenshot](previews/mode-companion-v2.png) and [intro animation frame](previews/intro-live-flight.png) remain as design history. They do not represent the new frontend under development. The double-diamond mark matches the website's identity.

<a id="documentation-map"></a>

## Documentation map

[Get help](SUPPORT.md) · [Build environment](INSTALL.md) · [Capability matrix](docs/03-reference/capabilities-matrix.md) · [Data and privacy](docs/02-guides/data-and-privacy.md) · [Contributing](CONTRIBUTING.md) · [All documentation](docs/README.md)

## License

Dual-licensed under Apache-2.0 OR MIT. See [LICENSE](LICENSE) for the license and [NOTICE](NOTICE) for sources and attribution.

If the documentation differs from actual behavior, please let us know through the [issue forms](https://github.com/Apeireth/Apeireth/issues/new/choose).

**[Back to top](#nav)**
