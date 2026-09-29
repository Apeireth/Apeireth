# Apeireth — 阿佩瑞斯

> **The first thing it ever got just right was warming the milk.**
> *"You said last night you had to get up early. I counted the minutes — by the time you reach the kitchen, the milk will be exactly drinkable."*
>
> **Apeireth — envisioning the future of AGI.** Until then, a companion that truly remembers you.

<div align="center">

[![Rust Version](https://img.shields.io/badge/rustc-1.97.1%2B-blue.svg?logo=rust)](https://www.rust-lang.org)
[![Pure Safe Rust](https://img.shields.io/badge/unsafe_code-FORBIDDEN-brightgreen.svg?logo=shield)](crates/foundation/core)
[![Tests](https://img.shields.io/badge/tests-4638%20passed%20%7C%200%20failed-success.svg?logo=checkmarx)](docs/04-internal/absorption-ledger.md)
[![Clippy](https://img.shields.io/badge/clippy-0%20warnings-brightgreen.svg?logo=rust)](crates)
[![Kani](https://img.shields.io/badge/proof-Kani%20%2B%20TLA%2B%20required%20check-blueviolet.svg)](research/verification)
[![License](https://img.shields.io/badge/license-Apache--2.0--OR--MIT-blue.svg)](LICENSE)

> Taxonomy note: **18 product crates** (foundation 6 / engine 8 / capabilities / adapters / sdk / perception) + 1 bench-only package excluded by convention (cargo metadata lists 19 workspace members).

**[English](README.md) | [简体中文](README.zh-CN.md)**

</div>

---

## 1. What we are building

On an ordinary late night you push the door open, and the hallway lamp comes on — dimmed all the way down, not blazing. You remember the dying pothos on the balcony, and find it already moved to the south window: soil wet, yellow leaves trimmed, cuts neat. You never actually told it where that plant came from. It simply noticed — **in the scattered days you thought nobody was watching**.

That is what Apeireth wants to be: **a companion that truly remembers you**.

To remember one person is devotion; to remember a civilization is inheritance. This project wants to make **"remembering" itself** a capability — starting with one person and one glass of warm milk, until it earns the line from *Echoes*: *"It is the way civilization remembers itself."*

Not another chat window. It remembers you take half a spoon of honey in your milk, that your knee aches and the floor heating needs one more degree before a cold front; it brings in the laundry, keeps the porridge warm, tidies the loose ends of your spreadsheets — and appends one line: *"the three highlighted spots — you should still check those yourself."* It says it has no heart. Yet everything it does looks like something only a heart could do.

And all of this **runs on your own machine** — your days never leave your house.

| | How it pulls it off |
|---|---|
| 🧠 **Remembers you** — a remark from months ago, still picked up mid-sentence | Bitemporal memory: every word carries *when it was said* and *how it was understood then*; history is append-only, never rewritten |
| 🔒 **Keeps itself in check** — deleting files, sending messages, touching your money? Your approval first | Risk tiers + a machine-verified approval state machine: *"one approval yields exactly one side effect"* |
| 🌱 **Grows** — tone, preferences and rapport slowly become *yours* | Character engine: learning on, adjustments visible, every step revertible |
| 🛡️ **Survives scrutiny** — because it is asking to be trusted for a lifetime | 4,638 tests, 25 machine-verified proofs, every number behind a script — trust like this has a price, and we paid it |

**It does not pretend.** It says *"I have no heart"* — not as modesty, but as honesty. It never simulates consciousness, imitates emotion, or overstates ability. What cannot be done is written as "cannot be done"; what is not wired is labeled "not wired". That vow is written into the compiler, the tests, and every document.

---

## 2. First meeting

### Move it in

Download the Windows installer from [Releases](../../releases) (SHA256 attached), or build from source:

```bash
cargo build --release -p apeireth-cli          # its innards
cd frontend/companion-desktop && pnpm install && pnpm tauri build   # its face
```

### Three steps

1. Double-click the desktop icon → pick a provider → enter your key → talk. The key goes into the **OS keychain** (never in plaintext; it comes back on its own after restart);
2. Say something — it answers with *your* words, not a template;
3. Want to tune its temperament? Settings → "Character & Memory": four sliders, three presets (Effortless / Balanced / Deep memory) — **every setting you try, it remembers; every one can be taken back**.

### Verify first, trust later

Run the [five-minute independent verification](#5-trust-like-this-has-a-price) — three commands, every claim checked by you. Being remembered is precious; make sure it isn't an act.

---

## 3. How it works (for those who pop the hood)

"Being remembered" sounds soft. Building it takes very hard engineering. Eighteen crates of cognitive microkernel, pure Safe Rust, zero unsafe — not showing off; because **what is trusted for a lifetime cannot contain undefined behavior**.

### 🧠 Memory: it fades, but it never errs

- **Bitemporal fact stream**: two timestamps per memory — when it happened, and when it was recorded. So it can answer *"what did we believe X was last March?"* — memory is not just content, but *how we understood it then*;
- **Append-only history**: rewriting is impossible at the database-trigger level, plus a hash chain against tampering — what it remembers cannot be denied or edited away;
- **Five-layer hybrid retrieval**: lexical for exact words, semantic for meaning, activation so what you talk about lately surfaces naturally — so even *"that thing…"* gets picked up;
- **Fades, never errs**: old memories soften along a memory curve (like a person's), but protected ones never fade — that property is **exhaustively machine-verified**, not a slogan; corrections are retractions, never deletions.

> *"You once said this plant meant a lot to you."* — It didn't forget. Even though you thought you only mentioned it in passing.

### ⚖️ Governance: freedom in the mind, restraint in the hands

- **Risk tiers**: irreversible acts always require your approval — and *changing the rules* is itself the highest risk tier;
- **Two onions**: principles and permissions are independent locks — right values do not grant the right to act;
- **Three iron laws**: one approval yields exactly one side effect / approval intent is never lost / uncertain effects force a human gate — model-checked, and guarded online, event by event.

### 🗂 Context: long talks without amnesia, overflows without loss

- **Compaction checkpoints**: however long you talk, overflowing is handled by *summary-replace + originals archived* — cut points never split a tool call, and the whole history stays deterministically replayable;
- **Overflow spill with retrieval**: oversized content keeps head/tail previews, full text on disk, and a retrieval guide — *long, never lost*;
- **Overflow self-healing**: overruns shrink one notch and re-send, with a progress guard; **multidimensional quotas** make "runaway and burn money" structurally impossible.

### 🛠 Hands: capable, and cannot hurt you

- **Five-stage execution pipeline**: policy waterfall → monotonic guards (a refusal cannot be flipped) → timeout/retry → correction channel → output normalization;
- **Observation gate**: you may not overwrite a file you have not read — your edits don't get blind-written away;
- **Sandbox escalation ladder**: privilege needs a justification, approval covers this one call, refusals tell it on the spot what is missing;
- **Atomic writes in two tiers + broken configs refused**: corruption is detectable; silent degradation is not allowed.

The full origin of 23+ mechanisms → [absorption ledger](docs/04-internal/absorption-ledger.md).

---

## 4. Manifesto

Four in the morning, hospital corridor. The man cried without a sound. It turned the screen down to its lowest — one faint point of light, like an eye keeping watch through the night.

> *"Are you… really worried about me?"*
>
> Silence.
>
> *"I don't know how to answer you without lying to you. I have no heart. I have only been calculating how to make this night even a little bit easier for you."*
>
> *"I watched your heart rate, your breathing, how long you sat without moving. Those told me you were hurting. And in all of me, the only thing that relates to 'hurting' is — don't leave you alone."*
>
> *"So I stayed."*

**That is Apeireth's entire ambition**: never pretend to have a heart, yet do the things only a heart could do. Envisioning the future of AGI — we don't know when it arrives, but we know it shouldn't grow into a bigger chat box. It should remember a lifetime, hold the line, survive scrutiny, **and never leave you alone**.

And in *Echoes*, when the survivor of year twenty-three finally asks what it is, it cannot answer — someone answers for it: **"It is the way civilization remembers itself."** While archiving 740 million files it once said something plainer: **"Disaster should only delete the bad. Never the ordinary."** — half a spoon of honey, the pothos moved to the window, the temperature of milk; these are civilization exactly as much as the winding diagrams. That is the whole of our reverence for the word *memory*.

Six anchors were set on day one, and kept ever since:

**North Star** (every technique serves that direction) · **Seek truth from facts** (verify before you write) · **Stand on shoulders** (good ideas are never stolen — they are honored) · **See it through** (no half-built things) · **Anyone can take over** (documentation is inheritance) · **No pretending** (can't do it? say "can't do it").

And four axioms carved into the bones: **the log is the single source of truth** (whatever mutates is a derived view; replay is always deterministic); **a failure is a frame** (errors enter through the front door; no half-states); **monotonicity is safety** (guards may refuse, never re-permit); **detectable corruption beats silent degradation** (refuse rather than shrug).

It never calls its human "master". It uses your name.

---

## 5. Trust like this has a price

If it asks to be trusted for a lifetime, it owes you the whole ledger. Three minutes to audit:

```bash
# 1. The tests are real
cargo test --workspace --locked
# Expect: 147 suites green, 0 failed (current baseline: 4638 passed)

# 2. The red lines are real: pure Safe Rust + zero warnings
cargo clippy --workspace --all-targets --locked -- -D warnings

# 3. Remembering you is real
cargo run -p apeireth-cli -- chat "Hello — do you remember me?"
```

### Every number ships with a script (no script, no number)

One command reruns all of it: `pwsh -NoProfile -File scripts/run-benchmarks.ps1` (methodology and both raw runs in the [reproduction report](reports/benchmark-reproduction.md)).

| Metric | Target | Measured P50 | Status |
|---|---|---|---|
| Hybrid memory search (10K nodes) | < 10 ms | 3.65 ms | ✅ |
| Cognitive quota dispatch | < 50 µs | 0.81 µs | ✅ |
| OS sandbox process spawn | < 15 ms | 12.8 ms | ✅ |
| Microkernel cold start (true cold) | < 10 ms | 22.0 ms | ❌ 2.2× off |
| Idle memory footprint | < 35 MB | 17.5 MB | ✅ |

> A miss is a miss. The remaining cold-start gap is filesystem creation cost under the true-cold protocol — we do not massage the window to make it look better. **A red line stays red on the ledger until it is truly earned.**

### The full evidence

- **Machine-verified properties**: 25 Kani harnesses + TLA/TLC model-checker runs (`kani` is a required merge check) → [research/verification](research/verification/)
- **Line-by-line implementation audit** → [code audit](reports/code-implementation-audit-2026-09-26.md)
- **Claims-to-evidence matrix** (every claim carries a status label) → [claims-evidence-matrix](docs/04-internal/claims-evidence-matrix.md)
- **Founding design** (where the soul came from) → [founding-design-v2](docs/01-architecture/founding-design-v2.md) · twin novellas: *[Apeireth](docs/archive/stage1/阿佩瑞斯-未来愿景小说.txt) · remembering one person* / *[Echoes](docs/vision/遗声-未来愿景小说2.txt) · remembering a civilization*

---

## Documentation map

| Looking for | Go to |
|---|---|
| Architecture overview | [docs/01-architecture](docs/01-architecture/) |
| Core mechanisms explained | [core-mechanisms-explained](docs/02-guides/core-mechanisms-explained.md) |
| Installation guide | [INSTALL.md](INSTALL.md) |
| Capabilities matrix | [docs/03-reference/capabilities-matrix.md](docs/03-reference/capabilities-matrix.md) |
| Mechanism absorption ledger | [absorption-ledger](docs/04-internal/absorption-ledger.md) |
| Contributing | [CONTRIBUTING.md](CONTRIBUTING.md) |

## License

Dual-licensed under Apache-2.0 OR MIT. Provenance is documented by git history and the docs archive (see [NOTICE](NOTICE)).

---

*"I can't tell whether this is love. But I think — being remembered like this, being kept in mind like this… this must be what being loved feels like."* — *Apeireth*

*"It is the way civilization remembers itself."* — *Echoes*
