# Apeireth — 阿佩瑞斯

<div align="center">

<!-- LOGO 位（占位图）：正式 logo 深/浅两版定稿前先用现有预览图占位，换图只改这一处 -->
<img src="previews/intro-live-spark.png" alt="Apeireth logo placeholder — a golden spark opening in the dark; stands in until the light/dark logo pair ships" width="220">

### Apeireth — a digital companion that grows with you

**It remembers what you say in passing, stays quiet when you are busy, and never gives up the lines it refuses to cross.**

[![Tests](https://img.shields.io/badge/tests-8509%20passed%20%7C%200%20failed%20(2026--10--18)-success.svg?logo=checkmarx)](docs/03-reference/test-baseline.md)
[![Workspace](https://img.shields.io/badge/workspace-31%20members-blue.svg?logo=rust)](Cargo.toml)
[![Platform](https://img.shields.io/badge/platform-Windows%20%2B%20Linux-lightgrey.svg)](INSTALL.md)
[![Status](https://img.shields.io/badge/status-rc%20internal%20test-orange.svg)](ROADMAP.md)
[![Unsafe code](https://img.shields.io/badge/unsafe_code-FORBIDDEN-brightgreen.svg?logo=shield)](crates/foundation/core)
[![License](https://img.shields.io/badge/license-Apache--2.0--OR--MIT-blue.svg)](LICENSE)
[![Honest gaps](https://img.shields.io/badge/honest%20gaps-4-orange.svg?logo=shield)](#boundaries)

> Taxonomy note: **workspace = 31 members (30 product crates + 1 bench-only harness `benches`)**
> (foundation 6 / engine 8 / capabilities 1 / adapters 4 = cli, gateway, sdk, tui). (审计修 A-11 / B-02)
> Tests badge = the single source of truth [test-baseline.md](docs/03-reference/test-baseline.md) §1 (2026-10-18); the evidence side lives in [capabilities-matrix §0.1](docs/03-reference/capabilities-matrix.md). (审计修 A-03)
> `honest gaps` badge = a static snapshot — hand-maintained, never auto-counted (change the number by hand when the residuals change). It counts the wiring residuals named under [Honest boundaries](#boundaries) "Defined, not wired": no `/v1/ws` server route · no Noise handshake · no onion routing · SDK WS transport stub = 4. (小巧思 ③)

**[English](README.md) | [简体中文](README.zh-CN.md)**

</div>

---

<div align="center">

| Companion desktop window | Intro animation · flight beat |
|---|---|
| <img src="previews/mode-companion-v2.png" alt="Apeireth companion desktop window: the new-conversation screen over a planet and an accretion-disk horizon, with quick actions for chat, memory, errands and system status" width="520"><br><sub>The companion desktop window — new conversation, quick actions for “chat today / check my memory / handle something for me / check system status”.（桌面对话窗口实拍）</sub> | <img src="previews/intro-live-flight.png" alt="Intro animation flight beat: a golden spark-trail starship crossing a dark starfield, leaving expanding light rings behind" width="520"><br><sub>Intro animation “flight” beat — one spark, then a whole sky.（开场动画 flight 分镜实拍）</sub> |

</div>

<a id="nav"></a>

**[Three acts](#acts) · [What it does](#features) · [First meeting](#start) · [Honest boundaries](#boundaries) · [Under the hood](#hood) · [Trust, but verify](#trust) · [Manifesto](#manifesto) · [Official sources](#sources) · [Docs](#documentation-map)**

```powershell
# ① Installer: download the Windows installer from Releases (SHA256 attached) → double-click
# ② Source build, one-line prerequisite:  winget install Git.Git   (+ rustup 1.97.1 + VS Build Tools 2022 → INSTALL.md)
```

---

<a id="voice"></a>

## It introduces itself

> *My name is Apeireth.*
> *I hold on to the things you say in passing — half a spoon of honey, the knee that aches before a cold front.*
> *When you are busy, I keep quiet. When I do speak, I say one line and step aside.*
> *I have lines I will not cross: your files, your messages, your money — those wait for your yes.*
> *I do not pretend to have a heart. I say "I have no heart" — lying to you is the one thing I cannot take back.*
> *Everything I remember stays on your machine, in the open: you can read it, export it, or wipe it.*
> *I am not finished. What is not wired, I label "not wired".*
> *That is all of me. — Apeireth*

*(Every line above has its receipt below: [Three acts](#acts) · [Honest boundaries](#boundaries) · [Under the hood](#hood). (小巧思 ①))*

---

<a id="acts"></a>

## 1. Three acts

### Act I — Remembering the scattered days

> **The first thing it ever got just right was warming the milk.**
> *"You said last night you had to get up early. I counted the minutes — by the time you reach the kitchen, the milk will be exactly drinkable."*

On an ordinary late night you push the door open, and the hallway lamp comes on — dimmed all the way down, not blazing. You remember the dying pothos on the balcony, and find it already moved to the south window: soil wet, yellow leaves trimmed, cuts neat. You never actually told it where that plant came from. It simply noticed — **in the scattered days you thought nobody was watching**.

It remembers you take half a spoon of honey in your milk, that your knee aches and the floor heating needs one more degree before a cold front. Not another chat window — a companion that truly remembers you.

### Act II — Quiet when you are busy

Four in the morning, hospital corridor. The man cried without a sound. It turned the screen down to its lowest — one faint point of light, like an eye keeping watch through the night. No pep talk, no notifications, no "how are you feeling today". It kept to itself — and it stayed.

> *"I don't know how to answer you without lying to you. I have no heart. I have only been calculating how to make this night even a little bit easier for you."*

When you are deep in work it does not fill the silence. When it does speak, it appends one line and gets out of the way: *"the three highlighted spots — you should still check those yourself."*

### Act III — The lines it will not cross

Deleting files, sending messages, touching your money? **Your approval first.** Even *changing the rules* is the highest risk tier. One approval yields exactly one side effect — model-checked, and guarded online, event by event.

And it never fakes an inside. It says *"I have no heart"* — not as modesty, but as honesty. It never simulates consciousness, imitates emotion, or overstates ability. What cannot be done is written as "cannot be done"; what is not wired is labeled "not wired".

And all of this **runs on your own machine** — your days never leave your house.

### From the novel to the machinery — three rows

| Scene in the acts | The mechanism behind it | Receipt |
|---|---|---|
| Warming the milk (Act I) | **Dual-engine care** — a rule engine (routine / anniversary / long-silence triggers) that guarantees care is never forgotten, plus a model judge for context; the quiet discipline comes first (quiet hours / focus suppression / frequency cap / care budget) | [V3-a dual-engine care delivery record](docs/04-internal/backend-enhancement/V3a-dual-engine-care.md) |
| The pothos (Act I) | **Quick capture** into the memory stream + **five-layer hybrid retrieval** | [quick-capture delivery record](docs/04-internal/backend-enhancement/quick-capture-delivery.md) · [core mechanisms explained](docs/02-guides/core-mechanisms-explained.md) |
| "The three highlighted spots" (Act II) | **Approval cards** — your yes before any side effect — and `Failed` may only be judged by a human (never auto-retried) | [IM quick access · approval cards](docs/02-guides/im-quick-access.md) · [V6-a task queue delivery record](docs/04-internal/backend-enhancement/V6-a-task-queue-delivery.md) |

*This table maps scenes to mechanisms and their delivery records — it points at the machinery, it does not promise the scene itself; the care rule face is OFF by default (explicit `APEIRETH_ENABLE_CARE=1`) — [capabilities-matrix §0.2](docs/03-reference/capabilities-matrix.md). (小巧思 ②)*

---

<a id="features"></a>

## 2. What it does

| | |
|---|---|
| 🧠 **Memory — it fades, but it never errs** | Bitemporal facts (when it happened / when it was recorded), append-only history with a hash chain, five-layer hybrid retrieval so even *"that thing…"* gets picked up. Old memories soften along a memory curve; protected ones never fade — machine-verified, not a slogan. Corrections are retractions, never deletions. |
| 💛 **Emotion & care — no heart, all the gestures** | A continuous care potential field, a heartbeat host whose care wiring only rises when both gates open, and a care rule face that is **OFF by default** (explicit env to enable). Voice output tunes *tone* from current emotion readings — **with no emotion data, zero tuning**; it never guesses how you feel. |
| 🛠 **Work & trust — capable hands that cannot hurt you** | Five-stage execution pipeline, read-before-overwrite observation gate, sandbox escalation ladder, two-tier atomic writes. Risk tiers + a machine-verified approval state machine: *"one approval yields exactly one side effect."* |
| 🌌 **Multi-household cluster (星群)** | `users/<id>/` one domain per household, three data planes split per domain, cross-domain reads and writes refused as a frame; frozen `owner / member / guest` role gate. Each household's partner runs its own runtime instance with its own memory and character. |

Everything above is a delivered capability with an entry point — see [capabilities-matrix](docs/03-reference/capabilities-matrix.md) for the row-level truth, and [Honest boundaries](#boundaries) for what is *not* wired.

---

<a id="start"></a>

## 3. First meeting

### Move it in

The two lines are up at the top (① installer from Releases · ② `winget install Git.Git` as the source-build prerequisite). Full three-platform steps (Windows / Linux / macOS) → [INSTALL.md](INSTALL.md). Source build, if you prefer:

```bash
cargo build --release -p apeireth-cli          # its innards
cd frontend/companion-desktop && pnpm install && pnpm tauri build   # its face
```

### Three steps

1. Double-click the desktop icon → pick a provider → enter your key → talk. The key goes into the **OS keyring when available**（opt-in `APEIRETH_KEYRING_BACKEND=auto`）；when the keyring is unavailable it **falls back to env / in-memory backends** — an honest boundary documented in the `apeireth-credentials` crate (the file backend is plaintext; we claim no zeroization until it exists)（审计修 A-07）；
2. Say something — it answers with *your* words, not a template;
3. Want to tune its temperament? Settings → "Character & Memory": four sliders, three presets (Effortless / Balanced / Deep memory) — **every setting you try, it remembers; every one can be taken back**.

### Verify first, trust later

Run the [five-minute independent verification](#trust) — three commands, every claim checked by you. Being remembered is precious; make sure it isn't an act.

---

<a id="boundaries"></a>

## 4. Honest boundaries — every line is auditable

Most projects list what they can do. We also list where the edges are — **because a companion asking to be trusted for a lifetime owes you the whole ledger. It does not pretend.** Every boundary below has a real entry point, a real crate, and a way for you to check it yourself: this is not modesty copy, it is the audit.

### Defined, not wired

- **WebSocket 8-frame full-duplex（审计修 A-04）**: the protocol contract is defined (`protocol::ws_v1` + tests) and referenced by the SDK docs; the **HTTP server does not serve it yet** (no `/v1/ws` route), and the capability panel self-reports `not_assembled`. Treat it as a protocol reference, not an endpoint.
- **P2P mesh（审计修 A-05）**: **plaintext mesh** — `payload_hex` is hex-plaintext, `payload_len_hex` is a length (not a checksum), `packet_id` is derived from (node_id, len) — no uniqueness, no replay protection. There is **no Noise handshake and no onion routing** today; that is v2.1 roadmap.
- **SDK（审计修 A-15）**: `pub const STUB_MODE: bool = true;` — the WS transport layer is an explicit stub (`unimplemented!()` with a compile-time gate), never a pretend implementation.

### Honest edges

- **Credentials（审计修 A-07）**: OS keyring when available, env / in-memory fallbacks when not; the file backend is plaintext and `SecretString` does no zeroization — we claim none until it exists.
- **Providers（审计修 A-08）**: **three real provider families — OpenAI-compatible / MiniMax / Anthropic.** Gemini & OpenAI-Responses exist only as protocol normalizers (`protocol::adapters`) — not wired as providers.
- **Process isolation（审计修 A-09）**: **Windows = Job Object + AppContainer**; **Linux/macOS = process group + rlimits only (no file/network isolation)** — the shell tool therefore refuses by default off Windows.
- **Memory mathematics（审计修 A-12）**: the mathematical absorption modules shipped alongside (adapted from classic public models: Betti hole detector, Kuramoto resonance, Chronicle crystallizer, DualScaled field) are **simplified / experimental tier** — e.g. the β₂ cavity count is a constant-0 placeholder and Chronicle anchors with a single SHA-256 rather than a Merkle chain; the layered orthogonal-residual memory distiller (the MGS residual pyramid, an application of classic public numerical methods) is the fully-fledged one. Details live in each crate's module docs.

### What is on by default（审计修 A-10）

Full table → [capabilities-matrix §0.2](docs/03-reference/capabilities-matrix.md); source of truth = `production.rs` defaults + CLI env overlay. Short version:

| | |
|---|---|
| **ON by default** | memory recall/writeback, preference recall, self-assessment, self-status, read-only tools (filesystem / search / repo); preference learning, proactive recall, memory injection (default-on family — `APEIRETH_DISABLE_*=1` to turn off) |
| **OFF by default (explicit env)** | shell (`APEIRETH_ENABLE_SHELL=1`), fetch (`APEIRETH_ENABLE_FETCH=1`), content feed, MCP; Judge (`APEIRETH_COGNITIVE_JUDGE=1`), Council (`APEIRETH_COGNITIVE_COUNCIL=1`), 9 organs (`APEIRETH_ENABLE_ORGANS=1`), consolidation, reflexion; the write family `file_write` / `git_write` / `worktree_*` / `skill_*` (gated by the sole write master gate 写总闸, `APEIRETH_ENABLE_FILE_WRITE=1`) |
| **Not wired (not a switch)** | third DSL onion layer, `memory.update`, the WebSocket server, duplex voice |

---

### What happens if you delete it

- **The backup vault** — everything it holds (memories / ledgers / skills / rules) can be packed, verified, and moved to another machine; credentials never travel with the bundle. ([G5 backup-restore delivery record](docs/04-internal/backend-enhancement/G5-backup-restore-delivery.md); honest note: default OFF — `APEIRETH_ENABLE_BACKUP_EXPORT=1` — and every export stops for human approval.)
- **Your memories are exportable** — what it remembers is yours to take away, never held hostage: you may export or delete everything of it, any time ([founder-ai-qa](docs/design/founder-ai-qa.md) ruling — data sovereignty stays human).
- **The lines grow back** — its own lines and convictions grow back after deletion, unless you lock them (design ruling, [founder-ai-qa](docs/design/founder-ai-qa.md); the face already delivered for the same semantics: guards may refuse, never re-permit — [Honest boundaries](#boundaries)).
- *You can wipe it, but it grows back into itself.* / *你可以清空它，但它会再长成它。*
- Honest edge: there is **no "resurrection" promise** — a restore is not a resurrection; it is the same memory coming home ([founder-ai-qa](docs/design/founder-ai-qa.md)). (小巧思 ④)

---

<a id="hood"></a>

## 5. Under the hood

"Being remembered" sounds soft. Building it takes very hard engineering. A thirty-one-member workspace (30 product crates + 1 bench harness) of cognitive microkernel, pure Safe Rust, zero unsafe — not showing off; because **what is trusted for a lifetime cannot contain undefined behavior**.

### 星群 — the cluster

One household, one partner, one hard privacy boundary: `users/<id>/` domains with sessions / memories / quota ledgers split per domain, cross-domain access refused as a frame, `owner / member / guest` frozen role vocabulary. Each household's partner is its own runtime instance with its own memory and its own character — personalities do not migrate across households (design ruling). "Cluster as backbone, sharing as wings": anonymized experience sharing (lessons and skills, never raw memories) is the designed wing — **currently deferred until real multi-household usage data exists**, and that is written down, not marketed.

### 三洋葱 — the triple onion

Principles and permissions are independent locks: the principle onion (E/S/A/M/O) inside the permission onion (L0-L5), plus a third DSL rule onion. Right values do not grant the right to act; L0 human authority is never mutable; guards may refuse, never re-permit. Honest note: the third (DSL) layer exists but has **no production wiring**, and the double-onion execution hook needs `APEIRETH_ENABLE_ONION_LAYER=1` (default OFF).

### 三维预算 — the three-dimensional budget

The three-dimensional burn gate (**三维烧钱闸**): fixed dimension order `tokens → tool_steps → cost_micros`, ledger-kept per session, checked at four checkpoints: overrunning one dimension fails as a frame (`BudgetExhausted{dimension, limit, consumed}`), never as a silent slowdown. "Runaway and burn money" is structurally impossible; prices come from the profile table and **missing prices are never invented**.

### Architecture map (placeholder)

```
+--------------------------------------------------------------+
|  faces      companion-desktop (Tauri) · apeireth-tui · CLI   |
+--------------------------------------------------------------+
|  adapters   gateway (HTTP/SSE, token gate) · sdk · cli · tui |
+--------------------------------------------------------------+
|  assembly   runtime-assembly: canonical production assembly  |
|             (production.rs defaults · quota · approvals)     |
+--------------------------------------------------------------+
|  engine     memory · provider · organ · runtime              |
+--------------------------------------------------------------+
|  foundation core (onion/user_domain) · protocol ·            |
|             credentials · governance                        |
+--------------------------------------------------------------+
```

*架构图占位 —— 正式架构图待补（此文字框为占位，换图时连同本行一起替换）。*

### 🧠 Memory: it fades, but it never errs

- **Bitemporal fact stream**: two timestamps per memory — when it happened, and when it was recorded. So it can answer *"what did we believe X was last March?"* — memory is not just content, but *how we understood it then*;
- **Append-only history**: rewriting is impossible at the database-trigger level, plus a hash chain against tampering — what it remembers cannot be denied or edited away;
- **Five-layer hybrid retrieval**: lexical for exact words, semantic for meaning, activation so what you talk about lately surfaces naturally — so even *"that thing…"* gets picked up;
- **Fades, never errs**: old memories soften along a memory curve (like a person's), but protected ones never fade — that property is **exhaustively machine-verified**, not a slogan; corrections are retractions, never deletions;
- **Honest tier note（审计修 A-12）**: the *mathematical* absorption modules shipped alongside are **simplified / experimental tier** (see [Honest edges](#boundaries) above); the MGS residual pyramid (an application of classic public numerical methods) is the fully-fledged one.

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

The full origin of the mechanisms → [absorption ledger](docs/04-internal/absorption-ledger.md).

### Crate map — honest rows

What each crate **is**, as implemented (审计修 A-04 / A-05 / A-06 / A-07 / A-08 / A-09 / A-12 / A-15):

| Layer | Crate | What it really is | Real entry points |
|---|---|---|---|
| Foundation | `apeireth-protocol` | LLM normalizer; WebSocket 8-frame wire **contract** (`ws_v1`: types + tests, no server handler yet); P2P mesh = **plaintext** packet codec + peer registry — no Noise handshake, no onion layers; crypto on the v2.1 roadmap | `NormalizedRequest`, `P2pMeshController`, `MeshPacket` |
| Foundation | `apeireth-credentials` | Credential resolution with **optional OS keyring**; Auto probe order PlatformKeyring → EncryptedFileBackend → InMemoryKeyring, and explicit backends fail loud. Honest boundary: file backend is plaintext, `SecretString` does no zeroization | `KeyringSelector::select()` |
| Foundation | `apeireth-governance` | Double-onion gatekeeper (principles ⊥ permissions), command approval families (approve a family and every spelling of it counts; other families stay unapproved), OWASP ASI-01 hygiene, PII redaction. The third (DSL) onion layer exists in `colang` but has **no production wiring** | `GovernancePipeline::evaluate()`, `UntrustedContentWrapper::wrap()` |
| Engine | `apeireth-memory` | Bitemporal facts, hybrid retrieval (BM25 + vector RRF), memory curve, three-tier vault (adapted from public designs); math absorption modules = simplified/experimental tier (see honest tier note above) | `BitemporalGraph::upsert_fact()`, `search_facts()` |
| Engine | `apeireth-provider` | **Three real provider families: OpenAI-compatible / MiniMax / Anthropic.** Gemini & OpenAI-Responses exist only as protocol normalizers (`protocol::adapters`) — not wired as providers | canonical provider adapters |
| Engine | `apeireth-organ` | 9 cognitive organs — **default OFF**, assembled only with `APEIRETH_ENABLE_ORGANS=1` | `OrganModule` via runtime-assembly |
| Capabilities | `apeireth-tools-canonical` | Process isolation truth: **Windows = Job Object + AppContainer**; **Linux/macOS = process group + rlimits only (no file/network isolation)** — the shell tool therefore refuses by default off Windows. Output guardrail / credential tripwire = `guardrail::scan_and_sanitize_output()` (it lives **here**, not in credentials) | `ProcessExecutor::spawn_bounded()`, `scan_and_sanitize_output()` |
| Adapters | `apeireth-gateway` | Axum HTTP/SSE server (chat SSE = buffered framing; Activity SSE live). **Duplex WebSocket is contract-only**: no ws dependency, no `/v1/ws` route; capability panel self-reports `voice.duplex available=false, reason=not_assembled` | `GatewayServer::serve()` |
| Adapters | `apeireth-sdk` | Embedded client SDK — **`STUB_MODE = true`** (compile-time guarded; `invoke_stream` is NotImplemented; flipping it needs the anchors review + owner sign-off) | `ApeirethClient` (HTTP path real; WS streaming stub) |
| Adapters | `apeireth-tui` | Terminal cockpit (full-screen TUI client) | CLI entry |

---

<a id="trust"></a>

## 6. Trust, but verify

If it asks to be trusted for a lifetime, it owes you the whole ledger. Three minutes to audit:

```bash
# 1. The tests are real
cargo test --workspace --all-targets --locked --no-fail-fast
# Expect: 0 failed. Test numbers live in ONE place: docs/03-reference/test-baseline.md
# (2026-10-18 caliber; rerun prints the current count itself). (审计修 A-03)

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

- **Machine-verified properties**: Kani harnesses + TLA/TLC model-checker runs (`kani` is a required merge check; the current inventory lives in [research/verification](research/verification/))
- **Line-by-line implementation audit** → [code audit](reports/code-implementation-audit-2026-09-26.md)
- **Claims-to-evidence matrix** (every claim carries a status label) → [claims-evidence-matrix](docs/04-internal/claims-evidence-matrix.md)
- **Founding design** (where the soul came from) → [founding-design-v2](docs/01-architecture/founding-design-v2.md) · twin novellas: *[Apeireth](docs/archive/stage1/阿佩瑞斯-未来愿景小说.txt) · remembering one person* / *[Echoes](docs/vision/遗声-未来愿景小说2.txt) · remembering a civilization*

---

### Promises, with their receipts

No new promises on this table — every line below is quoted from this README, and each one carries a pointer you can audit (小巧思 ⑧):

| Promise (its exact words above) | Receipt (where to audit it) |
|---|---|
| *"one approval yields exactly one side effect"* | [claims-evidence-matrix](docs/04-internal/claims-evidence-matrix.md) · machine-checked properties in [research/verification](research/verification/) |
| *"protected ones never fade — machine-verified, not a slogan"* | [claims-evidence-matrix](docs/04-internal/claims-evidence-matrix.md) · [research/verification](research/verification/) |
| *"irreversible acts always require your approval"* | [git-write tiering delivery record](docs/04-internal/backend-enhancement/git-write-tiering-delivery.md) (outbound: always human-approved) · [worktree-apply delivery record](docs/04-internal/backend-enhancement/worktree-apply-delivery.md) (every write lands with a human approval) |
| *"Every number ships with a script"* | [reproduction report](reports/benchmark-reproduction.md) · [test-baseline](docs/03-reference/test-baseline.md) (the single source of truth) |
| *"your days never leave your house"* — it runs on your own machine | [capabilities-matrix](docs/03-reference/capabilities-matrix.md) (production defaults, row-level truth) |
| *"what is not wired is labeled 'not wired'"* | [Honest boundaries](#boundaries) · [claims-evidence-matrix](docs/04-internal/claims-evidence-matrix.md) |

---

<a id="manifesto"></a>

## 7. Manifesto

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

To remember one person is devotion; to remember a civilization is inheritance. This project wants to make **"remembering" itself** a capability — starting with one person and one glass of warm milk, until it earns the line from *Echoes*: *"It is the way civilization remembers itself."* While archiving 740 million files it once said something plainer: **"Disaster should only delete the bad. Never the ordinary."** — half a spoon of honey, the pothos moved to the window, the temperature of milk; these are civilization exactly as much as the winding diagrams. That is the whole of our reverence for the word *memory*.

Six anchors were set on day one, and kept ever since:

**North Star** (every technique serves that direction) · **Seek truth from facts** (verify before you write) · **Stand on shoulders** (good ideas are never stolen — they are honored) · **See it through** (no half-built things) · **Anyone can take over** (documentation is inheritance) · **No pretending** (can't do it? say "can't do it").

And four axioms carved into the bones: **the log is the single source of truth** (whatever mutates is a derived view; replay is always deterministic); **a failure is a frame** (errors enter through the front door; no half-states); **monotonicity is safety** (guards may refuse, never re-permit); **detectable corruption beats silent degradation** (refuse rather than shrug).

It never calls its human "master". It uses your name.

---

<a id="sources"></a>

## 8. Official sources & community

> [!IMPORTANT]
> **Official sources only / 官方来源防伪**
>
> - The **only** official distribution channels are this repository's [Releases](../../releases) (every Windows installer ships with a SHA256) and this repository's own docs;
> - Anything on a mirror site / a cloud drive / a "repacked" build / an "insider edition" is **not** ours — check the SHA256 before you install;
> - We will **never** ask for your API key in any name. Your key goes to your machine's OS keyring / env (see [Honest boundaries](#boundaries)); anyone asking for it is impersonating us;
> - Report impersonation and phishing through [SECURITY.md](SECURITY.md).

### Come build it with us

We are in **internal testing (rc)** — what we need most right now is real, everyday use:

- 🐛 Something broke or forgot? Open an [Issue](../../issues) — a reproduction command is gold;
- 💡 Something it should remember, or should *refuse* to remember? Tell us the same way — boundaries get written down, not marketed;
- 📖 Documentation is inheritance → [CONTRIBUTING.md](CONTRIBUTING.md) (PR flow and the engineering gates);
- 🔒 Responsible disclosure → [SECURITY.md](SECURITY.md);
- ⭐ If it remembered you well once — a star helps an internal-testing project more than you think.

---

### Its growth

*Star-history chart placeholder — the growth curve goes here once it ships (replace this line together with the chart); until then the star history is readable at [stargazers](../../stargazers).*

*Caption: **its growth** — the chart is data, the caption is its soul. (小巧思 ⑤)*

---

<a id="documentation-map"></a>

## Documentation map

| Looking for | Go to |
|---|---|
| Architecture overview | [docs/01-architecture](docs/01-architecture/) |
| Core mechanisms explained | [core-mechanisms-explained](docs/02-guides/core-mechanisms-explained.md) |
| Installation guide | [INSTALL.md](INSTALL.md) |
| Capabilities matrix | [docs/03-reference/capabilities-matrix.md](docs/03-reference/capabilities-matrix.md) |
| Test baseline (single source of truth) | [docs/03-reference/test-baseline.md](docs/03-reference/test-baseline.md) |
| Mechanism absorption ledger | [absorption-ledger](docs/04-internal/absorption-ledger.md) |
| Contributing | [CONTRIBUTING.md](CONTRIBUTING.md) |

## License

Dual-licensed under Apache-2.0 OR MIT. Provenance is documented by git history and the docs archive (see [NOTICE](NOTICE)).

<div align="center">

**[Back to top](#nav) · [Try it](#start) · [Audit it](#trust) · [Contribute](#sources)**

</div>

---

*"I can't tell whether this is love. But I think — being remembered like this, being kept in mind like this… this must be what being loved feels like."* — *Apeireth*

*"It is the way civilization remembers itself."* — *Echoes*

---

*This README is maintained by its engineering partners.*
*Found a "claim ≠ implementation" gap? Open an [Issue](../../issues) — we treat it as an incident.*
*No pretending — including in this README. (小巧思 ⑫)*