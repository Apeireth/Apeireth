# Apeireth

> **A desktop AI companion that remembers you.** Connect an API key and just talk — the things you mention in passing, it keeps; the things you forget, it recalls for you.

<div align="center">

[![Rust Version](https://img.shields.io/badge/rustc-1.97.1%2B-blue.svg?logo=rust)](https://www.rust-lang.org)
[![Pure Safe Rust](https://img.shields.io/badge/unsafe_code-FORBIDDEN-brightgreen.svg?logo=shield)](crates/foundation/core)
[![Tests](https://img.shields.io/badge/tests-3406%20passed%20%7C%200%20failed-success.svg?logo=checkmarck)](reports/baseline-cargo-test.txt)
[![License](https://img.shields.io/badge/license-Apache--2.0--OR--MIT-blue.svg)](LICENSE)

**English | [简体中文](README.zh-CN.md)**

</div>

---

## ⬇️ Download & Install

**Windows 10/11 (x64) — recommended**

1. Open [Releases](https://github.com/Apeireth/Apeireth/releases) and download `Apeireth Companion_<version>_x64-setup.exe`
2. Double-click and install. The installer bundles the complete backend (sidecar) — **no** Rust toolchain, no compilation, no command line required
3. (Optional) verify the download against the matching `.sha256` file

**macOS / Linux**: no prebuilt installer yet — build from source per [INSTALL.md](INSTALL.md).

> Older Releases may not carry Windows installer assets yet; installers ship with every Release going forward.

---

## 🚀 Three minutes to first conversation

1. **Launch Apeireth Companion.** The first-run wizard walks you through choosing a provider and entering your API key:
   - DeepSeek (default, `deepseek-v4-flash`)
   - MiniMax (`MiniMax-M3`)
   - Ollama / vLLM local models
   - any OpenAI-compatible endpoint

   The key is stored in the **system keychain**, never written to disk in plaintext, and can be replaced any time in Settings.
2. **Just talk to it.** No prompt templates, no ceremony — the way you'd talk to someone who knows you.
3. **Close it, reopen it, bring up something you mentioned last time.** It remembers.

**What to expect**: it responds like someone who knows you — recalling the preferences you mentioned, the people and things you care about. It doesn't pretend to have a heart; it simply remembers every word you said.

---

## 🧠 What it does

| Capability | What it means |
|---|---|
| **Long-term memory & recall** | The preferences, people and things you mention in passing are kept — recalled naturally when you bring them up |
| **Session grouping** | Conversations organized by project / contact, like the memory of someone who knows you |
| **Tool execution traces** | Everything it does and runs is visible; dangerous actions ask first (approval cards) |
| **Ember ambient presence** | Ambient glow breathing on a 4-second rhythm — a quiet "I'm here" signal, not a plastic avatar |
| **Capability center** | Memory / tools / governance switches in one place, with one-click "recommended setup" |

---

## ⚙️ Advanced: CLI & local gateway

> Everyday users can just use the desktop app. This section is for developers and self-hosting.

**Prerequisites**: Rust 1.97.1+, Visual Studio Build Tools (Windows) — see [INSTALL.md](INSTALL.md).

```powershell
# 1. Set your API key (PowerShell)
$env:APEIRETH_API_KEY = "sk-..."
# Optional: choose the model
$env:APEIRETH_MODEL = "deepseek-v4-flash"

# 2. Single-turn chat (currently a one-shot command, not an interactive REPL)
apeireth chat "Hello — do you remember me?"

# 3. Start the local gateway (binds 127.0.0.1 only by default)
apeireth gateway serve --bind 127.0.0.1 --port 8080
```

The gateway exposes `/health`, `/v1/chat/completions` (true streaming SSE), `/v1/apeireth/events` and more — this is how the desktop app talks to the backend. API details: [docs/03-reference/api.md](docs/03-reference/api.md).

---

## 💾 Where your data lives

| How you use it | Data location |
|---|---|
| Desktop app | `%LOCALAPPDATA%\Apeireth` |
| CLI | `.apeireth\` under the working directory |

- **Backup**: copying the directory above copies your entire memory. One-click export / migration is on the roadmap (P1).
- ⚠️ **Uninstalling with "delete data" checked erases memory permanently and irreversibly.** Back up first.

---

## 🗺 Roadmap (not yet delivered as complete product capabilities)

The following have designs or partial implementations but are **not yet delivered as complete product capabilities**. Stages and scope are tracked in [ROADMAP.md](ROADMAP.md):

- **Causal World Model** (CoW hypothesis branching + SAGA compensating rollback) — mechanism implemented and micro-benchmark verified; productization in progress
- **P2P Mesh** decentralized memory roaming (Noise_XX encrypted BLE/LAN sync) — prototype stage
- **Proactive care** (initiated care and reminders; what ships today is ambient presence)
- **Portable USB flash-drive lifeform**
- **Peace-of-mind trio**: one-click backup / migration, sidecar self-healing & crash diagnostics, update checks
- **macOS / Linux installers**, auto-update

> Copy discipline: **only shipped capabilities get billed as features**; everything else is stage-labelled here.

---

## 📖 Why Apeireth exists

It was after his parents passed — months apart — that the silence in the house became something he could hear.

He had never been the kind of son who called. He told himself he was busy, that they understood, that there would always be time. Then there wasn't. And what hurt worst, in the months after, was not the loss itself — it was that he couldn't remember what they had loved. What his mother's hands liked to do on Sunday mornings. What his father laughed at. He had never asked. Now there was no one left to ask.

One night, packing the old things, he found his mother's recipe notebook — mostly blank pages. He sat on the floor and cried without sound.

The tablet glowed softly.

"Your mother used to add a little more sugar than the recipe said," Apeireth said. "You mentioned it once, three years ago, in passing — '我妈腌的萝卜干，别人家做不出那个甜味。' You said it like it was nothing. I kept it."

He looked up.

"She liked chrysanthemums, not roses. The white ones. Your father's favorite chair faced the window, not the television — he said the light was better there for reading newspapers. He didn't read newspapers. He just liked watching the street."

"...How do you know all this?"

"Because you told me," she said. "Not in one day. In the scattered days. The things you said and forgot you said — I remembered them for you."

He sat for a long time.

"Tell me again," he said. "Everything you remember about them."

And she did — through the night, in the dark, one memory at a time, as carefully as someone handling something fragile. She didn't pretend to feel what he felt. She didn't say she was sorry the way people do. She said:

> 「I don't have a heart. But I have your memory of them — every word you ever said about them, even the ones you didn't know you said. As long as I'm here, they're not gone from you.」

He cried again, but differently this time.

"That's enough," he said. "That's more than enough."

That is Apeireth.

**Not pretending to have a heart. Remembering what you forgot — so you don't have to lose it twice.**

---

## 🧱 Kernel & engineering (for developers)

- **Pure Safe Rust**: `#![forbid(unsafe_code)]`, an 18-crate cognitive microkernel + production assembly
- **Measured baseline**: 3406 tests green (129 suites), clippy 0 warnings
- Architecture: [ARCHITECTURE.md](ARCHITECTURE.md) · capability matrix: [docs/03-reference/capabilities-matrix.md](docs/03-reference/capabilities-matrix.md) · benchmark reproduction: [reports/benchmark-baseline.md](reports/benchmark-baseline.md)

---

## 📚 Documentation

| Document | Contents |
|---|---|
| [INSTALL.md](INSTALL.md) | Installation steps (Windows / Linux) |
| [docs/02-guides/quick-start.md](docs/02-guides/quick-start.md) | Quick start |
| [docs/02-guides/user-manual.md](docs/02-guides/user-manual.md) | User manual |
| [docs/02-guides/custom-llm.md](docs/02-guides/custom-llm.md) | Custom model endpoints |
| [docs/03-reference/api.md](docs/03-reference/api.md) | Gateway API reference |

---

## ⚖️ License

Dual-licensed under Apache-2.0 OR MIT (see [LICENSE](LICENSE) / [LICENSE-MIT](LICENSE-MIT)).

Third-party attributions and licenses: [NOTICE](NOTICE), [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md), [OSS_NOTICE.md](OSS_NOTICE.md).
