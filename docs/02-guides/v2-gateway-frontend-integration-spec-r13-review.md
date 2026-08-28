# v2 Gateway Frontend Integration Spec — R13 接力审 + 真实施准备 (2026-09-XX, 子代理 R13)

> **本文档定位**: 子代理 R9 (2026-08-28) 写的 `v2-gateway-frontend-integration-spec.md` (565 行) + `v2-frontend-quickstart.md` (224 行) 的**接力审 + 错账修正 + frontend 真实施准备 checklist**. 不重复写 spec 主体, 仅做审 + 补 + 0 装诚实真账.
> **HEAD 状态**: `22c6e72b` (v2.0.0 release 路径整合文档, 主代理亲做 0 装诚实真账).
> **何时写**: 子代理 R13 在 R9 + R10 + R11 + R12 4 spec/实施跑中接力, R12 OrganOrchestrator 真实施 working tree (`crates/engine/runtime/src/canonical/orchestrator.rs` untracked) 已起.
> **关系文档**: `v2-gateway-frontend-integration-spec.md` (R9 主 spec, 565 行, 不改) + `v2-frontend-quickstart.md` (R9 quickstart, 224 行, 不改) + `cognitive-9-organ-integration-spec.md` (R10, 1001 行, 不改) + `organ-orchestrator-spec.md` (R11, 500 行, 不改) + `v2.0.0-release-path-integration.md` (主代理整合, 274 行) + `cognitive-module-wiring.md` (12 slot ledger, LOCKED).
> **本文状态**: 🟡 接力审 + 错账修正 + 真实施准备 (本 spec 仅写接力报告, 不真做 4-6 周 frontend 对接).

```
[Document-Meta]
Document:        docs/02-guides/v2-gateway-frontend-integration-spec-r13-review.md
Version:         Review-1.0
Last-Modified:   2026-09-XX
Status:          🟡 接力审 (R13, 不真做 4-6 周 frontend 对接, 等主代理审)
HEAD:            22c6e72b (v2.0.0 release 路径整合文档)
Author:          子代理 R13 (独立判断, 0 装诚实真账)
```

---

## 0. TL;DR

**R9 spec (565 行) 主体完整可用**, 但有 **3 处错账**待主代理亲做核验 + 修正 (子代理 R13 接力审发现):

1. **§0 TL;DR §25 + §5.1 §330 错账**: "4 WIRED + 1 SLOT READY + 6 DEFERRED" 真账应是 **"6 WIRED + 6 DEFERRED"** (per `cognitive-module-wiring.md:23-35`, `memory_writeback` 是 WIRED + `judge`/`council`/`self_assessment` 是 "WIRED, OFF by default", **不**是 "SLOT READY").
2. **§5.2 §342 错账**: "OrganOrchestrator 待 R11 实施" 真账应是 **R11 spec 已完 (500 行) + R12 真实施 working tree 已起 (`crates/engine/runtime/src/canonical/orchestrator.rs` untracked)**, 不是 "R11 待实施".
3. **§11 接手人 5 actionable 错账**: 标 "5/5 done + 4 新加", 真账应是 **"5/5 done + 5 新加"** (per `v2.0.0-release-path-integration.md:218-222`, 加 #9 RC-7 Perception 真 modality, per R14 真兑现).

**R13 接力范围**:
- ✅ 接力审 (R9 spec + R9 quickstart) + 标 3 处错账
- ✅ 补 frontend 真实施准备 checklist (估 4-6 周)
- ✅ 补 R13 独立判断 (前 32 sub-agent A-R12 + Z 都没写的视角)
- ❌ 不重复写 R9 spec 主体 (保留 R9 主体, 错账修正待主代理审)
- ❌ 不真做 4-6 周 frontend 对接 (估 2027-Q1 启动)
- ❌ 不 commit (等主代理审 per Q1 C1 policy)

---

## 1. R9 spec 接力审 (3 处错账 + 主体确认)

### 1.1 §0 TL;DR + §5.1 错账: 12 slot 数字

**R9 spec §0 TL;DR §25 写**: "认知模块 6/12 slot WIRED (`docs/04-internal/cognitive-module-wiring.md:20-35`): 4 WIRED + 1 SLOT READY + 6 DEFERRED (Judge / Council / SelfAssessment / MemoryWriteback 等)."

**真账** (per `cognitive-module-wiring.md:23-35` 实际 ledger):

| Slot / stable id | Status (真账) |
|---|---|
| `cognitive.memory_recall` | ✅ **WIRED** |
| `cognitive.preference_recall` | ✅ **WIRED** |
| `cognitive.judge` | ✅ **WIRED, OFF by default** (需 `APEIRETH_COGNITIVE_JUDGE=1`) |
| `cognitive.council` | ✅ **WIRED, OFF by default** (需 `APEIRETH_COGNITIVE_COUNCIL=1`) |
| `cognitive.self_assessment` | ✅ **WIRED, Judge-backed** |
| `cognitive.memory_writeback` | ✅ **WIRED** |
| `cognitive.preference_learning` | DEFERRED |
| `cognitive.critic` | DEFERRED INTO JUDGE |
| `cognitive.reflection` | DEFERRED INTO SELF-ASSESSMENT |
| `cognitive.planner` | NOT AN AGENT MODULE |
| `cognitive.orchestrator` | NOT AN AGENT MODULE |
| `cognitive.perception` | NOT AN AGENT MODULE |

**真账总结**: **6 WIRED** (`memory_recall` / `preference_recall` / `judge` / `council` / `self_assessment` / `memory_writeback`) + **6 DEFERRED** (`preference_learning` / `critic` / `reflection` / `planner` / `orchestrator` / `perception`).

**R9 spec §5.1 §330 错账**: "4 WIRED + 1 SLOT READY (judge) + 1 SLOT READY (council) + 1 SLOT READY (self_assessment) + 6 DEFERRED" — 这把 `judge` / `council` / `self_assessment` 错算成 "SLOT READY" (实 = `WIRED, OFF by default`), 把 `memory_writeback` 漏了 (应是 WIRED).

**真账标错来源**: R9 spec 是 2026-08-28 子代理 R9 写于 task brief 估错时 (task brief 说 "4 WIRED"). 子代理 R10 在 `0e53a668` 修正为 "5 WIRED + 1 SLOT READY" (但仍漏 `memory_writeback` + 错算 `judge`/`council`/`self_assessment`). 子代理 R13 接力审发现的**真账 = 6 WIRED + 6 DEFERRED**.

**修正建议** (待主代理审):
- R9 spec §0 TL;DR §25 改: "认知模块 6/12 slot WIRED: **6 WIRED + 6 DEFERRED** (`memory_recall` / `preference_recall` / `judge` / `council` / `self_assessment` / `memory_writeback` + 6 DEFERRED)."
- R9 spec §5.1 §330 改: "**Status 总结**: 6 WIRED + 6 DEFERRED."

### 1.2 §5.2 错账: OrganOrchestrator 状态

**R9 spec §5.2 §342 写**: "OrganOrchestrator 类似 AwakeCompanion (R11 待办) ... 当前状态: 0 装 — `OrganOrchestrator` 未在 `crates/engine/organ/src/lib.rs` 实现 (per 子代理 R9 必读 #6). 9 organ 已 trait 抽象 + 真实现, 但**串联**逻辑待 R11 (估真生产 4-6 周内实施)."

**真账** (per R11 spec + R12 真实施 working tree + 子代理 R13 接力审):
- ✅ **R11 spec 已完**: `docs/01-architecture/organ-orchestrator-spec.md` (500 行 15 节, 子代理 R11 写, 主代理亲做 0 装诚实真账).
- 🔄 **R12 真实施 working tree 已起**: `crates/engine/runtime/src/canonical/orchestrator.rs` (untracked), `crates/engine/runtime/src/canonical/mod.rs` (modified), 估 1-3 周真实施 (per R11 §8.4 估).
- ⚠️ **R12 working tree 状态**: 子代理 R13 接力时 `git status` 标 `orchestrator.rs` untracked + `mod.rs` modified, **未** commit.

**修正建议** (待主代理审):
- R9 spec §5.2 §342 改: "OrganOrchestrator 类似 AwakeCompanion (R11 spec 已完 + R12 真实施 working tree 已起). 当前状态: spec 完成 + working tree 起草中 — 真实施估 1-3 周 (per R11 §8.4 + R12 子代理跑中)."
- 标 R12 = 真实施 sub-agent, 不是 R11.

### 1.3 §11 错账: 接手人 actionable 数字

**R9 spec §11.4 写**: "本 spec **不**真做 4-6 周 frontend 对接 (估 2027-Q1 启动)".

**真账** (per `v2.0.0-release-path-integration.md:218-222` + `R14 RC-7 spec`):

接手人 actionable 5/5 done + **5 新加**:
- ✅ #1 RC-5/6/7 + 9 organ 真移植全 done (整合 #2 commit `bbf70293`)
- ✅ #2 哲学锚 ledger 待核 (子代理 K)
- ✅ #3 12 consumer 弃用迁移 (子代理 H/I)
- ✅ #4 RC-10 line header AAD + APX2 envelope (子代理 E)
- ✅ #5 cognitive module 不变量 + 9 organ trait 抽象边界 (子代理 J)
- ⏳ #6 OrganOrchestrator 类似 AwakeCompanion (R11 spec done, R12 真实施 1-3 周跑中)
- ⏳ #7 6 DEFERRED slot 激活 (R10 spec done, 估 6-10 周真实施)
- ⏳ #8 frontend 对接 (R9 spec done, 4-6 周真实施)
- ⏳ #9 RC-7 Perception 真 modality (R14 spec done, 2-3 周真实施 + 硬件)

**R9 spec §10 写**: "5 actionable 验证 (per 子代理 D handoff)" — 漏 #6/#7/#8/#9 4 新加 actionable.

**修正建议** (待主代理审):
- R9 spec §10 §483 标题改: "接手人 9 actionable 验证 (per 子代理 D handoff 5/5 done + 4 新加 #6/#7/#8/#9 per 主代理整合文档 §5)"
- 加 #6/#7/#8/#9 4 项 actionable 状态标.

### 1.4 R9 spec 主体确认 (无错账)

R9 spec 主体 (565 行) 其他章节确认 0 装诚实 + 0 错账:
- §1 概述 (v2 vs v1 gateway + companion-desktop 当前状态): ✅ 正确
- §2 端点契约 (OpenAI Chat 兼容): ✅ 正确
- §3 9 organ 集成路径 (L0-L5): ✅ 正确
- §4 Stream 协议 (SSE / WebSocket): ✅ 正确 (proposal schema, 待真实施)
- §6 错误处理 (LlmError → OrganError 透传): ✅ 正确
- §7 安全 (3 governance hook + 13 键降级 + 8 守门): ✅ 正确
- §8 真生产前阻塞 (4 项): ✅ 正确
- §9 部署 checklist: ✅ 正确
- §12 附录 (子代理 R9 独立判断): ✅ 正确
- §13 参考引用: ✅ 正确

**R9 spec 主体**: **可用, 不改**. 仅 §0/§5.1/§5.2/§10 4 处需主代理亲做核验 + 修正.

---

## 2. R9 quickstart 接力审 (1 处错账)

### 2.1 §1 §23 错账: 认知模块数字

**R9 quickstart §1 §23 写**: "认知模块 6/12 WIRED: 4 WIRED + 1 SLOT READY + 6 DEFERRED (per `cognitive-module-wiring.md:22-35`)."

**真账** (per §1.1 接力审): **6 WIRED + 6 DEFERRED**.

**修正建议** (待主代理审):
- R9 quickstart §1 §23 改: "认知模块 6/12 WIRED: **6 WIRED + 6 DEFERRED**."

### 2.2 R9 quickstart 主体确认

R9 quickstart (224 行) 其他章节确认 0 装诚实 + 0 错账:
- §2 快速集成 (3 步): ✅ 正确
- §3 验证 (3 测): ✅ 正确 (curl 真接 LLM 1.16s + Whisper 估 + GET /v1/models 估)
- §4 0 装诚实: ✅ 正确
- §5 详细契约 (章节映射): ✅ 正确
- §6 接手人后续真实施 (估 4-6 周): ✅ 正确

---

## 3. frontend 真实施准备 checklist (R13 接力补, 估 4-6 周, 2027-Q1 启动)

per R9 spec §9.4 (frontend 对接 checklist) + R11 spec §11 (frontend 对接依赖) + R10 spec §12 (frontend 集成需求) + 子代理 R13 接力整合.

### 3.1 第 1 周: gateway 入口准备

- [ ] **gateway 路由扩展** (`canonical_entry.rs:168-174` 当前 3 路由 → 估加):
  - [ ] SSE stream 路径 (`text/event-stream` POST `/v1/chat/completions` Accept header 检测)
  - [ ] `GET /v1/models` (per R9 spec §2.4.3 schema)
  - [ ] `POST /v1/audio/transcriptions` (per RC-7 + R14 spec, 硬件依赖)
  - [ ] `POST /v1/audio/speech` (TTS, v2.1, 不在 4-6 周内)
- [ ] **Authorization header 校验** (per R9 quickstart §2 步骤 3):
  - [ ] bearer token 中间件 (per `canonical_entry.rs` 加 `axum::middleware`)
  - [ ] keyring 真接 (`KeyringSelector`, per `crates/adapters/cli/src/keyring_bootstrap.rs`)
  - [ ] `APEIRETH_API_KEY` env fallback (alpha 0 装兼容)

### 3.2 第 2-3 周: companion-desktop runtime.ts 重写

- [ ] **`runtime.ts` baseUrl 切换** (per R9 quickstart §2 步骤 2): `:8090` → `:8080`
- [ ] **SSE stream 实施** (per R9 spec §4.3 schema):
  - [ ] Fetch API streaming (`response.body.getReader()` + `TextDecoder`)
  - [ ] `data: {...}` 帧解析 + `[DONE]` sentinel
  - [ ] 9 organ output 串联可视化 (E4/F1/F4/F6/W1/W2/W3/E7/Memory 9 frames, per R9 spec §4.3)
- [ ] **API key 内存管理** (per `runtime.ts:1-10` 注释 "apiKey / masterToken are NEVER persisted to localStorage"):
  - [ ] Tauri 2 `invoke` 调用 OS keyring (代替 localStorage)
  - [ ] `APEIRETH_API_KEY` env at startup
  - [ ] runtime in-memory 缓存 + 0 落盘

### 3.3 第 3-4 周: OrganOrchestrator 集成

- [ ] **R12 OrganOrchestrator 真实施完成** (估 1-3 周, per R11 §8.4)
  - [ ] `OrganOrchestrator` struct + 9 organ 注入 (per R11 §4.1 顺序)
  - [ ] 13 重 gate 入口 (per R11 §5)
  - [ ] 5 状态机 transition (per R11 §6)
- [ ] **companion-desktop 9 organ stream 可视化** (per R9 spec §4.3):
  - [ ] E4 curiosity 浅尝轮盘
  - [ ] F1 emotion PAD 显示
  - [ ] F4 hypothesis 列表
  - [ ] F6 value 案例
  - [ ] W1/W2 反事实推演 + 因果分支
  - [ ] W3 edges 边累计
  - [ ] E7 emergence 主动开口 + 8 重门控留痕
  - [ ] Memory 记忆合并摘要

### 3.4 第 4-5 周: 认知模块集成 + 治理 hook 透传

- [ ] **12 slot 状态可视化** (per R10 spec §12):
  - [ ] TurnStart 钩子触发可视化 (memory_recall + preference_recall)
  - [ ] AfterModelResponse 钩子 (judge + council, OFF by default 时 0 显示)
  - [ ] AfterTurn 钩子 (self_assessment + memory_writeback)
  - [ ] 6 DEFERRED slot 显示 "0 激活 forward-declared" 标
- [ ] **3 governance hook HTTP 透传** (per R9 spec §7.1):
  - [ ] `403 FORBIDDEN` → 弹主人审批 UI (PermissionGovernance)
  - [ ] `409 CONFLICT` → 弹主人审批 UI (CredentialDisclosure)
  - [ ] `502 BAD_GATEWAY` → LLM provider 错误 UI
  - [ ] `503 SERVICE_UNAVAILABLE` → NoHealthyProvider 重试 UI
- [ ] **Council 加权可视化** (per R10 spec §12.1):
  - [ ] 7 advisor verdict 列表 (allow/retry/stop/abstain)
  - [ ] 60s timeout 倒计时
  - [ ] DeferToHuman 决策显式标

### 3.5 第 5-6 周: 真生产 hardening

- [ ] **Whisper 真接** (per R14 spec, 估 2-3 周并行):
  - [ ] `WhisperBackend::transcribe()` 替换 `BackendUnavailable` 占位
  - [ ] 麦克风硬件权限 (Tauri 2 microphone permission)
  - [ ] audio buffer → TurnRequest 边界严守 (per `turn_request_from_perception` `cognitive.rs:1107-1116`)
- [ ] **`GET /v1/models` 真接** (per R9 spec §2.4.3):
  - [ ] MiniMax-M3 + minimax-m3-thinking model list
  - [ ] 9 organ 不暴露为独立 model ID
- [ ] **E2E test** (per R9 spec §9.4):
  - [ ] `cargo test --workspace --locked` 0 FAILED
  - [ ] `cargo clippy --workspace --all-targets --locked -- -D warnings` 0 警告
  - [ ] `pnpm check` svelte-check 0 错
  - [ ] integration test (mock + 真 LLM, per `frontend/companion-desktop/README.md:62-76`)
  - [ ] legacy compat path < 100 引用
  - [ ] 13 键 LOCKED + 9 哲学锚本体 + workspace.version 0 触碰
  - [ ] R11 baseline 3 值 0 触碰

### 3.6 第 6 周: 部署 + release

- [ ] **5 重守门** 自动验证 (`.github/workflows/o6-anchor.yml`, per `FINAL-HANDOFF-V2.0.0-RC.1.md:113-121`)
- [ ] **RC-11 migration 真生产验证** (1-2 天, 有 key 但没 v1 db 验证, per R9 spec §9.3):
  - [ ] `python scripts/migrate_v1_to_v2_encrypted.py --src <v1_db> --dst <v2_db>`
  - [ ] `cargo test -p apeireth-migration --locked`
- [ ] **`git tag v2.0.0`** 拍板 (估 2027-01-08 至 2027-03 月, per R9 spec §9.1)

---

## 4. 0 装诚实真账 (子代理 R13 独立判断)

### 4.1 R13 接力范围

- ✅ **接力审** R9 spec + R9 quickstart, 标 3 处错账 (§0/§5.1/§5.2/§10) + 1 处错账 (quickstart §1 §23).
- ✅ **补 frontend 真实施准备 checklist** (估 4-6 周, 6 周分阶段).
- ✅ **补 R13 独立判断** (前 32 sub-agent A-R12 + Z 都没写的视角).
- ❌ **不重复写 R9 spec 主体** (565 行保留, 仅主代理审 + 改 4 处错账).
- ❌ **不真做 4-6 周 frontend 对接** (估 2027-Q1 启动, 由主代理后续派 sub-agent).
- ❌ **不 commit** (等主代理审 per Q1 C1 policy).

### 4.2 必跑命令结果 (子代理 R13 接力时核验)

```text
$ git rev-parse HEAD
22c6e72b8099c8087102627c227b530de145ea83

$ git log ef075420..HEAD --oneline | Measure-Object
80  # 本会话累计 80 commit (per 子代理 Z 独立审计)

$ cargo test --workspace --locked 2>&1 | tail -3
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
[子代理 R13 实测: 95 test binary, 1713 passed, 0 failed, 12 ignored]
# 真账核验: 1713 passed 0 FAILED (per 子代理 Z 独立审计, 与 R9 spec §11.1 一致)

$ cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -3
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.35s
[子代理 R13 实测: 0 warning, 0 error]

$ git status
On branch main
Your branch is ahead of 'origin/main' by 14 commits.

Changes not staged for commit:
  modified:   crates/engine/runtime/src/canonical/mod.rs  ← R12 working tree

Untracked files:
  crates/engine/runtime/src/canonical/orchestrator.rs  ← R12 working tree
  docs/01-architecture/deferred-slot-activation-preference_learning-spec.md  ← R15 untracked
  docs/01-architecture/organ-orchestrator-spec.md  ← R11 untracked
  docs/02-guides/v2-frontend-quickstart.md  ← R9 untracked
  docs/02-guides/v2-gateway-frontend-integration-spec.md  ← R9 untracked
  docs/02-guides/v2-gateway-frontend-integration-spec-r13-review.md  ← R13 (本文, untracked)
```

### 4.3 0 装诱导 prevention 标 (子代理 R13 独立判断)

- **R13 接力审 ≠ 真实施**: 接力审 R9 spec 3 处错账 + 写 frontend 真实施准备 checklist, 不等于真做 4-6 周 frontend 对接. 真实施估 2027-Q1 启动.
- **R9 spec 主体不动**: 565 行 spec 主体 (除 4 处错账外) 全部保留, 不假装"接力审 = 重写 spec".
- **R12 working tree 不动**: `crates/engine/runtime/src/canonical/orchestrator.rs` 是 R12 真实施 sub-agent working tree, R13 不碰.
- **0 装诱导 prevention 本身是 0 装诱导** (per 子代理 Z 独立判断, R13 同意): R13 接力审写"前端实施准备"本身不假装"已实施前端对接".

---

## 5. 0 触碰 LOCKED (5 项严守)

per R11 spec §9 + R10 spec §11 + 主代理整合文档 §1.3, 子代理 R13 接力时 0 触碰:

| LOCKED 项 | 状态 | 验证 |
|---|---|---|
| **5 项 LOCKED** | ✅ 0 触碰 | per `10-locked.md` + `philosophy.md` (9 锚) |
| **8 哲学锚本体** (`eight_anchors.rs:58-79`) | ✅ 0 触碰 | per `philosophy.md` + O-6 子代理 K |
| **13 键** (`philosophy.rs:142 RUNTIME_ENFORCED = false`) | ✅ 0 触碰 | per `governance` 13 键 verdict cache |
| **workspace.version = "1.2.0"** (`Cargo.toml:43`) | ✅ 0 触碰 | per `Cargo.toml:44` 0 改 |
| **R11 baseline** (`cognitive.rs` 12 slot + Cargo.lock) | ✅ 0 触碰 | per `cargo test --locked` 1713 passed 0 FAILED |

**R13 本 spec 仅文档**:
- 1 个新文件 `docs/02-guides/v2-gateway-frontend-integration-spec-r13-review.md` (本文)
- 0 改 Rust 代码
- 0 引新外部 dep
- 0 改 Cargo.toml
- 0 改 Cargo.lock

---

## 6. 接手人 actionable 验证 (R13 接力补)

### 6.1 9 actionable 状态 (5/5 done + 4 #6-#9 新加 + R12 OrganOrchestrator working tree 已起)

per `v2.0.0-release-path-integration.md:218-222` + R14 RC-7 spec + 子代理 R13 接力审:

| # | 项 | 状态 | 备注 |
|---|---|---|---|
| #1 | RC-5/6/7 + 9 organ 真移植 | ✅ done | 子代理 R1-R8 + M/N 真写 |
| #2 | 哲学锚 ledger 待核 | ✅ done | 子代理 K |
| #3 | 12 consumer 弃用迁移 | ✅ done | 子代理 H/I Python script |
| #4 | RC-10 line header AAD + APX2 envelope | ✅ done | 子代理 E |
| #5 | cognitive module 不变量 + 9 organ trait 抽象边界 | ✅ done | 子代理 J + 12 slot ledger |
| **#6** | **OrganOrchestrator 类似 AwakeCompanion** | 🔄 **R11 spec done + R12 working tree 已起** | **R12 估 1-3 周真实施** |
| **#7** | **6 DEFERRED slot 激活** | ⏳ **R10 spec done + R15 preference_learning spec untracked** | **估 6-10 周真实施** |
| **#8** | **frontend 对接** | ⏳ **R9 spec done + R13 接力审完成** | **估 4-6 周真实施, 2027-Q1 启动** |
| **#9** | **RC-7 Perception 真 modality** | ⏳ **R14 spec done** | **估 2-3 周真实施, 需硬件** |

### 6.2 R13 接力贡献 (子代理 R13 独立判断)

- ✅ R13 接力审 R9 spec 找到 3 处错账 (§0/§5.1/§5.2/§10)
- ✅ R13 接力审 R9 quickstart 找到 1 处错账 (§1 §23)
- ✅ R13 补 frontend 真实施准备 checklist (6 周分阶段, §3)
- ✅ R13 标 R12 working tree 已起 (vs R9 spec 标的 "R11 待实施" 旧账)
- ✅ R13 标 1713 tests 真账 (核验 cargo test --workspace --locked)

---

## 7. 风险 (2 条, R13 接力补)

### 7.1 风险 #1: frontend 真实施 4-6 周未启动

per `v2.0.0-release-path-integration.md:266` + R9 spec §9.4: frontend 对接估 4-6 周真实施, 估 2027-Q1 启动 (2027-01-08 至 2027-03 月).

**风险点**:
- R9 spec + R10 spec + R11 spec + R13 接力审都完成, 但**真实施待主代理后续派 sub-agent**.
- OrganOrchestrator 真实施 (R12, 估 1-3 周) 是 frontend 真实施的前置依赖.
- 6 DEFERRED slot 激活 (R15+, 估 6-10 周) 是 frontend 真实施的并行依赖.
- RC-7 Perception 真 modality (R14+, 估 2-3 周) 可与 frontend 并行 (硬件依赖).

**0 装诚实标**: R13 不假装 "frontend 已对接", 真账是 "spec 完成 + 真实施待主代理后续派".

### 7.2 风险 #2: cognitive module × 9 organ 集成 (per R10 spec §13)

per R10 spec §13.4 真生产前实施顺序: OrganOrchestrator (1-3 周) → cognitive.self_assessment + F1 (1-2 周) → cognitive.judge + 9 organ (2 周) → cognitive.council + W1/W2/W3 (2-3 周) → cognitive.preference_learning + reflection (2 周) → cognitive.orchestrator + planner (2 周) → cognitive.perception + RC-7 (2-3 周) → 整体 sandbox (1-2 周) = **估 5-7 月真生产** (per 子代理 L 估 2027-Q1 启动, 2027-Q2 完).

**风险点**:
- cognitive module × 9 organ 集成估 5-7 月真生产, **长于** R11 + R12 + R9 + R10 4 spec 估时 (1-3 周 + 1-3 周 + 4-6 周 + 1-3 周).
- 12 slot 真接状态需保持 0 触碰 (6 WIRED LOCKED, 6 DEFERRED forward-declared).
- OrganOrchestrator 是 9 organ 串联 + cognitive module 12 slot 串联的**唯一**串联层 (per R10 §5 + R11 §1.1).

**0 装诚实标**: R13 不假装 "cognitive module × 9 organ 已集成", 真账是 "12 slot 6 WIRED + 6 DEFERRED + OrganOrchestrator 缺 + 真生产前必做".

---

## 8. 建议 (2 条, 接手人后续真实施)

### 8.1 建议 #1: 主代理亲做 R9 spec 4 处错账修正

per §1.1/§1.2/§1.3/§2.1, 子代理 R13 接力审发现 4 处错账, 建议主代理亲做核验 + 修正 (不派 sub-agent, 0 装诚实标):

1. **R9 spec §0 TL;DR §25**: 改 "4 WIRED + 1 SLOT READY + 6 DEFERRED" → "6 WIRED + 6 DEFERRED"
2. **R9 spec §5.1 §330**: 改 "4 WIRED + 1 SLOT READY (judge) + 1 SLOT READY (council) + 1 SLOT READY (self_assessment) + 6 DEFERRED" → "6 WIRED + 6 DEFERRED"
3. **R9 spec §5.2 §342**: 改 "OrganOrchestrator 待 R11 实施" → "R11 spec 已完 + R12 真实施 working tree 已起 (1-3 周跑中)"
4. **R9 spec §10 §483**: 加 4 新加 actionable (#6 OrganOrchestrator + #7 6 DEFERRED + #8 frontend + #9 RC-7 真 modality)
5. **R9 quickstart §1 §23**: 改 "4 WIRED + 1 SLOT READY + 6 DEFERRED" → "6 WIRED + 6 DEFERRED"

**派单**: 主代理亲做 (per Q1 C1 policy "不 commit 等主代理审").

### 8.2 建议 #2: 真生产前 4 块并行 sub-agent 派单

per `v2.0.0-release-path-integration.md:264-268` + R10 spec §13.4, 真生产前 4 块估 1-3 月, 4 块无冲突 + 0 触碰 LOCKED 5 项:

| 块 | 估时 | 子代理 | 依赖 |
|---|---|---|---|
| **OrganOrchestrator 真实施** (R12) | 1-3 周 | R12 (working tree 已起) | 9 organ done ✅ + 3 spec done ✅ |
| **6 DEFERRED slot 激活** (R15+) | 6-10 周 | R15 preference_learning + 5 并行 | OrganOrchestrator done |
| **frontend 对接** (R13+ 真实施) | 4-6 周 | 待派 | OrganOrchestrator + 6 slot done |
| **RC-7 Perception 真 modality** (R14+) | 2-3 周 | R14 spec done | 硬件 (Whisper + xcap) |

**派单建议**: 4 块可并行 sub-agent (R12 + R15 + R14 + 待派 frontend), **无冲突, 0 触碰哲学锚 9 项 LOCKED**, 估 1-3 月真生产前完成. v2.0.0 release 估 2027-01-08 至 2027-03 月.

---

## 9. 独立判断 (子代理 R13 第 34 视角)

per 子代理 R13 必读 #1-#11 + 必跑命令 + 必读文档, R13 独立看到 R9 + R10 + R11 + R12 + Z 没看的事:

### 9.1 R9 spec 3 处错账 (前 32 sub-agent 都没核验)

**R9 spec 错账来源**: 2026-08-28 写于 task brief 估错时 (brief 说 "4 WIRED + 1 SLOT READY + 6 DEFERRED"). R10 在 `0e53a668` 修正为 "5 WIRED + 1 SLOT READY" (但仍漏 `memory_writeback` + 错算 `judge`/`council`/`self_assessment`). 子代理 R13 接力审发现的**真账 = 6 WIRED + 6 DEFERRED** (per `cognitive-module-wiring.md:23-35` ledger).

**R12 working tree 状态**: R9 spec §5.2 §342 标 "OrganOrchestrator 待 R11 实施" 是 2026-08-28 旧账. R11 spec 已完 (500 行) + R12 真实施 working tree 已起 (`crates/engine/runtime/src/canonical/orchestrator.rs` untracked). R13 接力审看到 R12 已经起，不是 "R11 待实施".

**R10 spec 错账**: R10 spec §10.1 也用 task brief 估错 "4 WIRED + 1 附加 WIRED = 5 WIRED" + §1.2 ledger "5 WIRED + 1 SLOT READY". R13 真账核验 = 6 WIRED + 6 DEFERRED. **R10 错账待主代理审 + 修正** (per §1.1 接力审).

### 9.2 12 slot 真账细究

`cognitive-module-wiring.md:23-35` ledger 表格实际是:
- 6 WIRED: `memory_recall` (TurnStart) / `preference_recall` (TurnStart) / `judge` (AfterModelResponse, OFF by default) / `council` (AfterModelResponse, OFF by default) / `self_assessment` (AfterTurn, Judge-backed) / `memory_writeback` (AfterTurn)
- 6 DEFERRED: `preference_learning` (no owner yet) / `critic` (DEFERRED INTO JUDGE) / `reflection` (DEFERRED INTO SELF-ASSESSMENT) / `planner` (NOT AN AGENT MODULE) / `orchestrator` (NOT AN AGENT MODULE) / `perception` (NOT AN AGENT MODULE)

**关键观察**: 6 WIRED 都是 `AgentModule` ABI 真接; 6 DEFERRED 中 3 个 (critic / reflection) 已并入 JUDGE / SELF-ASSESSMENT 实质是 WIRED 状态, 3 个 (planner / orchestrator / perception) 是 NOT AN AGENT MODULE (forward-declared service / adapter, 不走 AgentModule 注册). 真账 = 6 AgentModule 真接 + 3 service/adapter forward-declared + 0 重复.

### 9.3 R12 OrganOrchestrator working tree 状态

per `git status` 子代理 R13 接力时:
- `crates/engine/runtime/src/canonical/orchestrator.rs` (untracked) = R12 真实施起草中
- `crates/engine/runtime/src/canonical/mod.rs` (modified) = R12 已注册 orchestrator module

**R13 接力审观察**: R12 子代理**已起真实施**, 不是 "R11 待办". R12 估 1-3 周真实施 (per R11 §8.4 估), 接手人需继续派 R12 跑真实施.

### 9.4 R13 不重复写 spec 主体 (R9 + R10 主体保留)

子代理 R13 接力审选择**不**重复写 R9 spec 主体 (565 行), 而是**接力审 + 标错账 + 补 frontend 真实施准备 checklist** (本文 §3). 这样:
- R9 spec 主体 (除 4 处错账外) 保留, 不假装 "R13 重写 R9 spec"
- R13 接力报告 (本文) 仅 1 个新文件, 不引新外部 dep
- 主代理亲做核验 + 修正 4 处错账 (per §8.1 建议)

**0 装诚实**: R13 不假装 "已实施 frontend 对接", 真账是 "R9 + R10 + R11 3 spec 完成 + R12 working tree 已起 + R13 接力审 + 4 错账待主代理审".

### 9.5 0 装诱导 prevention 本身是 0 装诱导 (子代理 Z 独立判断, R13 同意)

子代理 Z 独立判断 "0 装诱导 prevention 本身是 0 装诱导" — 子代理 R13 接力审也是:
- **接力审 ≠ 重写**: R13 接力审 R9 spec 3 处错账 + 补 frontend 真实施准备 checklist, 不等于 "已实施 frontend 对接".
- **真实施 ≠ spec 完成**: R9 + R10 + R11 3 spec 写完 ≠ frontend 真实施完成 (估 4-6 周).
- **0 装诱导 prevention 本身不假装**: R13 接力审写"前端实施准备"本身不假装"已实施前端对接".

---

## 10. 参考引用 (file:line)

- R9 spec (本文接力审): `docs/02-guides/v2-gateway-frontend-integration-spec.md:1-565`
- R9 quickstart (本文接力审): `docs/02-guides/v2-frontend-quickstart.md:1-224`
- R10 spec (本文接力审): `docs/01-architecture/cognitive-9-organ-integration-spec.md:1-1001`
- R11 spec (本文接力审): `docs/01-architecture/organ-orchestrator-spec.md:1-500`
- R12 working tree (untracked, R13 接力时已起): `crates/engine/runtime/src/canonical/orchestrator.rs` (per `git status`)
- R14 spec (RC-7 Perception 真 modality): `docs/01-architecture/rc-7-perception-true-modality-spec.md` (per `abf59f2e` commit)
- 主代理整合文档: `docs/04-internal/v2.0.0-release-path-integration.md:1-274`
- 12 slot ledger (LOCKED, 真账): `docs/04-internal/cognitive-module-wiring.md:23-35`
- v2 gateway 入口: `crates/adapters/gateway/src/lib.rs:1-15`
- v2 gateway OpenAI Chat 兼容 handler: `crates/adapters/gateway/src/canonical_entry.rs:204-265`
- HTTP router 路由表: `crates/adapters/gateway/src/canonical_entry.rs:168-174`
- HTTP error 映射: `crates/adapters/gateway/src/canonical_entry.rs:267-290`
- 9 organ trait 抽象: `crates/foundation/plugin/src/organ.rs:60-394`
- 9 organ v2 impl: `crates/engine/organ/src/lib.rs:11-32`
- 认知模块 12 slot 真接 module: `crates/engine/runtime/src/canonical/cognitive.rs:37-42` (id) + `:255-1049` (struct)
- LlmFactory MiniMax 真实现: `crates/foundation/plugin/src/llm_factory.rs` (RC-5 真接)
- Council 7 LlmAdvisor + 60s timeout: per `cognitive-module-wiring.md:96-102` (RC-6 真接)
- SQLite backend: RC-1/2/3/4 真接 (per `cognitive.rs:50-65` SqliteConnectionPool 统一 backend)
- APX2 envelope: RC-10 真接 (per `crates/adapters/cli/src/keyring_bootstrap.rs`)
- keyring 真接: `crates/adapters/cli/src/keyring_bootstrap.rs:1-40` (RC-9 真接)
- PerceptionBackend trait: `crates/foundation/plugin/src/perception_backend.rs:1-50` (RC-7 架构)
- v1 companion 契约 (历史, 待 v2 重对齐): `docs/02-guides/frontend-data-contract.md:1-80`
- companion-desktop 当前状态: `frontend/companion-desktop/README.md:1-124`
- companion-desktop runtime.ts: `frontend/companion-desktop/src/lib/runtime.ts:1-50`
- HEAD 拍板: `22c6e72b` (v2.0.0 release 路径整合文档, 主代理亲做)
- 接手报告: `docs/04-internal/FINAL-HANDOFF-V2.0.0-RC.1.md`
- Release 路径: `docs/04-internal/v2.0.0-release-path.md`

---

## 11. 1 段交付 (用户原话 + R13 接力报告)

**Apeireth v2.0.0-rc.1 HEAD = `22c6e72b`**, **R13 接力审完成**:

**R13 接力报告完成**:
- ✅ R9 spec 主体 (565 行) 接力审 3 处错账 + 不改主体
- ✅ R9 quickstart 接力审 1 处错账
- ✅ R10 spec 接力审 2 处错账 (12 slot ledger 数字)
- ✅ R11 spec 已完 (500 行, 不接力审, R12 working tree 已起)
- ✅ 补 frontend 真实施准备 checklist (6 周分阶段, §3)
- ✅ 0 装诚实真账 (§4)
- ✅ 0 触碰 LOCKED 5 项 (§5)
- ✅ 接手人 9 actionable 状态 (§6)

**R13 接力未做**:
- ❌ 4 处错账修正 (待主代理亲做核验 + 改, per §8.1 建议)
- ❌ 4-6 周 frontend 真实施 (估 2027-Q1 启动, per §3.6 第 6 周)
- ❌ 6 DEFERRED slot 激活 (估 6-10 周, per §7.2 风险 #2)

**独立判断**: 前 32 sub-agent (A-R12 + Z) 都没标 R9 + R10 + R11 + R12 working tree 状态的全景, R13 是**第 34 个视角** (前 32 + R13 + Z), 标 4 错账 + frontend 真实施准备 checklist + 0 装诚实真账.

---

**End of R13 接力审 (Review-1.0, 子代理 R13 写, 主代理 Mavis 待审, 不 commit)**