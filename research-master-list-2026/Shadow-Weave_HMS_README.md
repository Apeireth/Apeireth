<div align="center">

<img src="docs/assets/branding/hms-hero.png" alt="Holographic Memory System" width="94%">

### Structured Memory Intelligence for Reliable Long-Horizon Reasoning

<table>
  <tr>
    <td valign="middle"><strong>ShadowWeave Team</strong></td>
    <td width="74" align="center" valign="middle">
      <img src="docs/assets/branding/shadowweave-mark.png" alt="ShadowWeave" width="62">
    </td>
  </tr>
</table>

<a href="https://arxiv.org/"><img src="https://img.shields.io/badge/arXiv-coming_soon-B31B1B?style=flat-square&logo=arxiv&logoColor=white" alt="arXiv: coming soon"></a>
<img src="https://img.shields.io/badge/status-active-145DA0?style=flat-square" alt="Project status: active">

[English](README.md) · [中文](README.zh-CN.md)

</div>

---

## Abstract

The **Holographic Memory System (HMS)** is a reproducible long-term memory QA
framework for studying whether structured answer-time evidence organization can
improve a language model's reasoning over retrieved memories.

The project focuses on the LongMemEval setting, where a question may require
evidence from multiple sessions, timestamps, extracted memory facts, and raw
source snippets.

## One-Command Automatic Memory

HMS can wrap an existing OpenAI client so each model call automatically:

```text
user input -> Recall relevant memories -> inject context -> call the LLM
           -> Retain the completed user/assistant exchange
```

Configure the model Base URL, API key, and model in `.env`, then run:

```bash
bash scripts/run_memory_demo.sh
```

The script starts PostgreSQL and HMS locally, waits for the memory API, installs
the local SDK adapter in an isolated `uv` environment, and runs a two-turn demo.
The first turn stores a preference and project; the second turn recalls both
without manually calling `retain()` or `recall()`.

For this demo, one `OPENAI_API_KEY` / `OPENAI_BASE_URL` / `OPENAI_MODEL` set is
enough. The script reuses it for HMS reasoning and retain extraction when the
role-specific values remain unset. Set the embedding model separately when your
provider does not support `text-embedding-3-small`.

The application-side integration is one wrapper call:

```python
from openai import OpenAI
from hms_litellm import wrap_openai

client = wrap_openai(
    OpenAI(),
    hms_api_url="http://127.0.0.1:18080",
    api_key="YOUR_HMS_API_KEY",
    bank_id="user-alice",
)

response = client.responses.create(
    model="gpt-4o-mini",
    input="What do you remember about my current project?",
)
```

`wrap_openai()` supports both `client.responses.create(...)` and
`client.chat.completions.create(...)`, including streaming. Use a stable,
per-user `bank_id`; optionally set `session_id` to accumulate one conversation
as a tracked HMS document.

## Experiment Design

The reproducible evaluation follows one complete pipeline:

```text
Dataset conversations
  -> Retain: extract and store structured memories
  -> Recall: retrieve evidence for each question
  -> Organize: build an answer-time evidence structure
  -> Answer: generate a grounded response
  -> Judge: compare the response with the gold answer
```

The core idea is to avoid giving the answer model a loose list of retrieved
facts. Instead, the system builds an intermediate evidence structure that makes
time, source, event state, and numeric signals explicit before generation.

This setup is useful for studying questions such as:

- whether the model can connect evidence across sessions
- whether the model can distinguish old and current user states
- whether the model can ground relative dates to concrete memories
- whether the model can avoid duplicate counting
- whether missing numeric sides are handled conservatively

## Visual Demo

The project includes a database-free demo for external readers. It shows how
raw retrieved sessions are converted into an organized evidence ledger before
answer generation.

![Memory evidence organization demo](docs/assets/memory_pipeline_demo.svg)

Open the standalone demo page:

```text
docs/memory_pipeline_demo.html
```

This page can be viewed directly in a browser and does not require model keys,
database access, or benchmark artifacts.

## Dynamic Case Replay

The repository also includes a concrete benchmark-style case replay. It shows a
single multi-session question and animates how scattered session snippets move
through retrieval, evidence ledger construction, deduplication, and grounded
answer generation.

![Dynamic benchmark case replay](docs/assets/benchmark_case_replay.svg)

Open the auto-playing replay page:

```text
docs/benchmark_case_replay.html
```

The replay page auto-advances through the raw session snippets, recall candidates,
ledger rows, duplicate-control rule, answer packet, and final grounded response
for the same case.

## Pipelines

Two pipeline modes are exposed through the benchmark script.

### Ledger Pipeline

The ledger pipeline keeps memory retrieval unchanged and adds a structured
evidence ledger before answer generation.

For high-risk question types, the ledger records:

- event time
- mention time
- source session or document
- fact type
- compact evidence text
- numeric, date, and update signals
- raw source snippets for grounding

Use this mode when you want to reproduce the main evidence-organization
experiment.

### Self-Evolution Pipeline

The self-evolution pipeline keeps the ledger pipeline and adds a lightweight
answer-time controller. The controller is driven by diagnosed failure patterns:

- count and total deduplication
- relative-date lookup grounding
- amount and difference calibration
- current versus previous state arbitration

This mode is intended for studying whether targeted control instructions can
improve or change memory reasoning behavior after retrieval.

## Repository Layout

```text
.
├── .aaaSCRIPT/
│   └── run_benchmark.sh
├── core/
│   ├── dataplane/
│   ├── daemon/
│   └── local-suite/
├── deploy/
├── docs/
│   ├── assets/
│   │   ├── branding/
│   │   │   ├── hms-banner.png
│   │   │   ├── hms-hero.png
│   │   │   ├── shadowweave-mark.png
│   │   │   └── shadowweave_v6.png
│   │   ├── benchmark_case_replay.svg
│   │   └── memory_pipeline_demo.svg
│   ├── benchmark_case_replay.html
│   └── memory_pipeline_demo.html
├── interface/
├── lab/
│   └── evaluation/
│       └── benchmarks/
│           ├── common/
│           │   └── benchmark_runner.py
│           └── longmemeval/
│               └── longmemeval_benchmark.py
├── tooling/
├── .env.example
├── README.md
└── README.zh-CN.md
```

Important files:

- `.aaaSCRIPT/run_benchmark.sh`: unified experiment script
- `scripts/run_memory_demo.sh`: one-command automatic retain/recall demo
- `examples/automatic_memory/openai_responses.py`: two-turn OpenAI Responses API example
- `docs/assets/branding/hms-banner.png`: project identity banner
- `docs/assets/branding/hms-hero.png`: compact README project header
- `docs/assets/branding/shadowweave-mark.png`: compact ShadowWeave team mark
- `docs/assets/branding/shadowweave_v6.png`: ShadowWeave team identity artwork
- `docs/benchmark_case_replay.html`: auto-playing single-case process replay
- `docs/assets/benchmark_case_replay.svg`: README-embedded animated case replay
- `docs/memory_pipeline_demo.html`: static before/after visualization
- `docs/assets/memory_pipeline_demo.svg`: README-embedded visual summary
- `lab/evaluation/benchmarks/longmemeval/longmemeval_benchmark.py`: LongMemEval pipeline implementation
- `lab/evaluation/benchmarks/common/benchmark_runner.py`: shared evaluation runner
- `.env.example`: local configuration template

## Environment Setup

Create a local environment file:

```bash
cp .env.example .env
```

Open `.env` and replace the `*_change_me` values. The main model settings are:

| Pipeline role | Base URL | API key | Model |
| --- | --- | --- | --- |
| Core HMS / recall organization | `HMS_API_LLM_BASE_URL` | `HMS_API_LLM_API_KEY` | `HMS_API_LLM_MODEL` |
| Retain / fact extraction | `HMS_API_RETAIN_LLM_BASE_URL` | `HMS_API_RETAIN_LLM_API_KEY` | `HMS_API_RETAIN_LLM_MODEL` |
| Answer generation | `HMS_API_ANSWER_LLM_BASE_URL` | `HMS_API_ANSWER_LLM_API_KEY` | `HMS_API_ANSWER_LLM_MODEL` |
| LLM judge | `HMS_API_JUDGE_LLM_BASE_URL` | `HMS_API_JUDGE_LLM_API_KEY` | `HMS_API_JUDGE_LLM_MODEL` |
| Embeddings | `HMS_API_EMBEDDINGS_OPENAI_BASE_URL` | `HMS_API_EMBEDDINGS_OPENAI_API_KEY` | `HMS_API_EMBEDDINGS_OPENAI_MODEL` |

All roles may point to the same OpenAI-compatible service. In that case, use
the same Base URL and API key in each section while choosing models appropriate
for each role. Also configure:

- `HMS_API_DATABASE_URL`: a reachable PostgreSQL database with `pgvector`
- `HMS_DATASET_PATH`: the local LongMemEval dataset JSON path
- `HMS_PIPELINE`: `ledger` or `self_evolution`

The framework loads configuration from `.env`. Do not hard-code credentials in
the source code, and never commit the populated `.env` file.

## Reproduction Logic

The benchmark script now defaults to the full clean reproduction path:

```text
Retain -> Recall -> Answer -> Judge
```

For every benchmark item, HMS first retains the conversation sessions, recalls
relevant memories for each question, generates a grounded answer, and finally
uses the configured judge model to score that answer against the gold answer.

Recommended first run:

```text
1. Copy .env.example to .env
2. Fill database, Base URL, API key, model, and dataset settings
3. Start with one or two benchmark instances
4. Verify Retain, Recall, Answer, and Judge all complete
5. Increase concurrency and benchmark size
6. Inspect results under .aaaRESULT/ and logs under .aaaLOG/
```

Use `HMS_RETRIEVAL_ONLY=1` only for a later iteration when the same memories are
already present in the database and you intentionally want to rerun only
Recall → Answer → Judge.

## Minimal End-to-End Run

```bash
cp .env.example .env
# Edit .env before continuing.

export HMS_BENCHMARK=longmemeval
export HMS_PIPELINE=ledger
export HMS_RETRIEVAL_ONLY=0
export HMS_MAX_INSTANCES=2

bash .aaaSCRIPT/run_benchmark.sh \
  --parallel 1 \
  --max-concurrent-questions 1 \
  --eval-semaphore-size 1
```

The script prints the active mode at startup. For a clean reproduction it must
print `HMS reproduction mode: Retain -> Recall -> Judge`.

## Run the Ledger Pipeline

```bash
export HMS_RETRIEVAL_ONLY=0
export HMS_PIPELINE=ledger
export HMS_MAX_INSTANCES=500
export HMS_SESSION_EXPANSION_WEIGHT=0.5

bash .aaaSCRIPT/run_benchmark.sh \
  --parallel 8 \
  --max-concurrent-questions 8 \
  --eval-semaphore-size 8 \
  --quiet
```

## Run the Self-Evolution Pipeline

```bash
export HMS_RETRIEVAL_ONLY=0
export HMS_PIPELINE=self_evolution
export HMS_MAX_INSTANCES=500
export HMS_SESSION_EXPANSION_WEIGHT=0.5

bash .aaaSCRIPT/run_benchmark.sh \
  --parallel 8 \
  --max-concurrent-questions 8 \
  --eval-semaphore-size 8 \
  --quiet
```

## Common Runtime Options

Useful environment variables:

- `HMS_PIPELINE`: `ledger` or `self_evolution`
- `HMS_RETRIEVAL_ONLY`: defaults to `0`; set to `1` only to reuse retained memories
- `HMS_MAX_INSTANCES`: limit the number of evaluated questions
- `HMS_MAX_QUESTIONS`: limit questions after filtering
- `HMS_DATASET_PATH`: provide a local LongMemEval dataset path
- `HMS_SESSION_EXPANSION_WEIGHT`: override session expansion weight
- `HMS_PYTHON_BIN`: use a specific Python interpreter

Useful command-line options:

- `--parallel`: number of instances processed concurrently
- `--max-concurrent-questions`: maximum concurrent question-level tasks
- `--eval-semaphore-size`: evaluator concurrency limit
- `--category`: run a specific LongMemEval category
- `--question-id`: run one or more question IDs
- `--skip-ingestion`: skip ingestion and use existing database memories
- `--quiet`: reduce console output

## Runtime Artifacts

When the experiment runs, local runtime artifacts are written under ignored
directories:

```text
.aaaLOG/
.aaaRESULT/
```

These directories are for local reproduction and should not be committed.
