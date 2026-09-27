# Apeireth — 阿佩瑞斯

> **Apeireth — envisioning the future of AGI.** Until then, a desktop companion that truly remembers you.

<div align="center">

[![Rust Version](https://img.shields.io/badge/rustc-1.97.1%2B-blue.svg?logo=rust)](https://www.rust-lang.org)
[![Pure Safe Rust](https://img.shields.io/badge/unsafe_code-FORBIDDEN-brightgreen.svg?logo=shield)](crates/foundation/core)
[![Tests](https://img.shields.io/badge/tests-4638%20passed%20%7C%200%20failed-success.svg?logo=checkmarx)](docs/04-internal/absorption-ledger.md)
[![Clippy](https://img.shields.io/badge/clippy-0%20warnings-brightgreen.svg?logo=rust)](crates)
[![Kani](https://img.shields.io/badge/proof-Kani%20%2B%20TLA%2B%20required%20check-blueviolet.svg)](research/verification)
[![License](https://img.shields.io/badge/license-Apache--2.0--OR--MIT-blue.svg)](LICENSE)

**[English](README.md) | [简体中文](README.zh-CN.md)**

</div>

---

## 1. What it is (30 seconds)

**Apeireth is an AI companion that lives on your desktop.** It is not another chat window — it **remembers** you (the thing you mentioned in passing months ago), **keeps itself in check** (destructive actions require your approval), **grows** (its character develops from experience), and all of this **runs on your own machine**.

Four things it genuinely does:

| | Scene | How it pulls it off |
|---|---|---|
| 🧠 | **Remembers you**: "Did you ever find that jar for your mother's pickled radish?" | Bitemporal memory + lexical/semantic hybrid retrieval + a human-like forgetting curve |
| 🔒 | **Keeps itself in check**: wants to delete files, send messages, touch configs? Approval gate first | R0–R4 risk matrix + a machine-verified approval state machine |
| 🌱 | **Grows**: the longer you talk, the more the tone, preferences and rapport become *yours* | Character engine (learning on by default, transparent and revertible) |
| 🛡 | **Earns trust**: history is immutable, failures are never silent, every number ships with a script | Append-only logs + full regression suite + five-minute reproducible verification |

**Honest boundaries** (the unglamorous part first): it never pretends to have a soul — it is a well-engineered machine that plays the companion role sincerely; the model is external (DeepSeek / OpenAI-compatible / local), and all our work lives in **the ring of memory and trust machinery around the model**.

---

## 2. Quick start

### Install

Download the Windows installer from [Releases](../../releases) (NSIS, SHA256 attached), or build from source:

```bash
# Source build (Rust 1.97.1+)
cargo build --release -p apeireth-cli          # backend
cd frontend/companion-desktop && pnpm install && pnpm tauri build   # desktop shell
```

### First run

1. Double-click the desktop icon → first-run wizard (pick a provider → enter your key → chat);
2. The key goes into the **OS keychain** (never stored in plaintext; restored automatically on restart);
3. Want to tune the personality? Settings → "Character & Memory": four sliders + three presets (Effortless / Balanced / Deep memory).

### Verify before you trust?

Run the [five-minute independent verification](#5-verification-dont-take-our-word-for-it) — three commands check every core claim.

---

## 3. Why it is different (for the mildly technical)

A traditional AI assistant = a large model + a prompt layer. Apeireth = a large model + **a whole operating-system-grade ring of memory and trust machinery**. Eighteen crates of cognitive microkernel (pure Safe Rust, zero unsafe), four pillars:

### 🧠 Memory system
- **Bitemporal fact stream**: every memory carries both "when it happened" and "when we recorded it" — you can ask *"what did we believe X was last March?"*;
- **Append-only**: history cannot be rewritten (enforced at the database trigger level), with a Merkle hash chain against tampering;
- **Five-layer hybrid retrieval**: lexical + semantic + fusion + activation + three-tier progressive disclosure (not just "vector search with extra steps");
- **Fades but never errs**: forgetting decays along a memory curve, protected memories never fade (TLA+ exhaustively verified), corrections are retractions — never deletions.

### ⚖️ Governance
- **R0–R4 risk matrix**: irreversible actions always pass human approval; changing the rules is itself the highest risk tier;
- **Three onions**: principles, permissions and behavior gate independently — right values ≠ license to act;
- **Approval state machine**: three properties ("an approval produces exactly one side effect", …) verified by TLA+ model checking **and guarded online at runtime** (an invariant registry checked per event).

### 🗂 Scheduling & context
- **Multidimensional quota** (tokens / steps / cost / depth): a runaway, money-burning agent is structurally impossible;
- **Compaction checkpoints**: overflowing conversations are no longer hard-truncated — summary replacement + archived originals + deterministic replay, and cut points never split a tool-call pair;
- **Overflow spill with retrieval**: oversized content keeps head/tail previews + full text on disk + a retrieval guide ("long, never lost");
- **Overflow self-healing**: context overruns shrink the budget and re-assemble with a progress guard.

### 🛠 Tools & execution
- **Five-stage execution pipeline**: policy waterfall → monotonic guards (a refusal cannot be flipped) → timeout/retry → a correction channel → output normalization;
- **Observation gate**: you may not overwrite a file you have not read (two CAS keys);
- **Sandbox escalation ladder**: privilege requests need a justification, approval covers one call only, refusals guide you in place;
- **Unified atomic writes**: integrity/durable tiers + cross-process locks; broken configs are **refused**, never silently defaulted.

> Want the full origin story of all 23+ mechanisms? See the [absorption ledger](docs/04-internal/absorption-ledger.md).

---

## 4. Manifesto (the soul lives here)

**Apeireth — envisioning the future of AGI.**

We do not know when artificial general intelligence will arrive, but we know it should not grow into "a bigger chat box". It should **remember a lifetime**, **hold the line**, and **survive scrutiny**. So we laid the foundations first:

- **The log is the single source of truth** — every mutable state is a derived view; replay is always deterministic;
- **A failure is a frame** — errors travel through formal channels with closed vocabularies; no half-states;
- **Monotonicity is safety** — guards may only refuse, never re-permit; ordering cannot be undone;
- **Detectable corruption beats silent degradation** — bad data is refused or skipped-and-counted, never quietly defaulted.

And the old vow that runs through everything: **no pretending.** No pretending to have a soul, no pretending the tests passed, no pretending the numbers are real — what cannot be done is written as "cannot be done", what is not wired is labeled "not wired".

---

## 5. Verification (don't take our word for it)

```bash
# 1. The tests are real: full workspace regression
cargo test --workspace --locked
# Expect: every suite green, 0 failed (current baseline: 4638 passed / 147 suites)

# 2. Engineering red lines: pure Safe Rust + zero warnings
cargo clippy --workspace --all-targets --locked -- -D warnings

# 3. The LLM wiring is real (any supported provider key works)
cargo run -p apeireth-cli -- chat "Hello — do you remember me?"
```

### Performance numbers (every row ships with a script — no script, no number)

One command reruns everything: `pwsh -NoProfile -File scripts/run-benchmarks.ps1` (methodology and both raw runs in the [reproduction report](reports/benchmark-reproduction.md)).

| Metric | Target | Measured P50 | Status |
|---|---|---|---|
| Hybrid memory search (10K nodes) | < 10 ms | 3.65 ms | ✅ |
| Cognitive quota dispatch | < 50 µs | 0.81 µs | ✅ |
| Session event folding (1K events) | — | 289 µs | ✅ record row |
| OS sandbox process spawn | < 15 ms | 12.8 ms | ✅ |
| Microkernel cold start (true cold) | < 10 ms | 22.0 ms | ❌ 2.2× off (honestly labeled) |
| Idle memory footprint | < 35 MB | 17.5 MB | ✅ |

> A miss is a miss — the remaining cold-start gap is filesystem creation cost under the true-cold protocol, and we do not massage the measurement window to make it look better.

### More auditable evidence

- **Machine-verified properties**: 25 Kani harnesses + TLA/TLC model-checker runs (`kani` is a required merge check): [research/verification](research/verification/)
- **Line-by-line implementation audit**: [code audit](reports/code-implementation-audit-2026-09-26.md)
- **Claims-to-evidence matrix** (every public claim carries a status label): [claims-evidence-matrix](docs/04-internal/claims-evidence-matrix.md)
- **Founding design**: [founding-design-v2](docs/01-architecture/founding-design-v2.md)

---

## Documentation map

| Looking for | Go to |
|---|---|
| Architecture overview | [docs/01-architecture](docs/01-architecture/) |
| Capabilities matrix | [docs/03-reference/capabilities-matrix.md](docs/03-reference/capabilities-matrix.md) |
| Core mechanisms explained (for guides/owners) | [core-mechanisms-explained](docs/02-guides/core-mechanisms-explained.md) |
| Installation guide | [INSTALL.md](INSTALL.md) |
| Mechanism absorption ledger | [absorption-ledger](docs/04-internal/absorption-ledger.md) |
| Contributing | [CONTRIBUTING.md](CONTRIBUTING.md) |

## License

Dual-licensed under Apache-2.0 OR MIT. Provenance is documented by git history and the docs archive (see [NOTICE](NOTICE)).
