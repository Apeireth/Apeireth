# B 块 frontend 对接 6 项主代理亲做决策 真账 (2026-08-28)

**作者**: Sub-Agent | **HEAD**: `3eb7f26b` | **关系**: 1 份决策文档, 不写代码, 不 git add/commit

---

## 0. 重大发现 (优先级重排)

**R12 working tree 已跑通** (per `git log` HEAD `3eb7f26b`):
- 5 stage OrganOrchestrator 完整化 **已 commit + push** (`c003e078` → `087ab2ac` → `50ba2e57` → `29e5ce66` → `0afa733f`, 2026-08-28)
- tests 1726 → 1739 passed (+13 new, 0 failed), 0 clippy 警告
- 0 触碰 LOCKED 5 项 + 0 引新外部 dep
- **§8.3 #3 "R12 working tree 能否跑通" = 已跑通, 不再阻塞 B 块**

`.harness-msg/{1..5}.txt` 是 5 个**已实施 commit message 草稿**, 不是 stage 计划. sub-agent 2 §7 R4 估时 1-3 周 + 真账 3-5 周 = **过时, 真账 0 周**. critical path 6-8 周 → 估 **5-7 周**.

---

## 1. R9 spec 4 处错账修正 (主代理亲做)

| # | 位置 | 错账 | 真账 | 修法 |
|---|---|---|---|---|
| 1 | R9 §0 §25 | "4 WIRED + 1 SLOT READY + 6 DEFERRED" | **6 WIRED + 6 DEFERRED** (`memory_recall`/`preference_recall`/`judge`/`council`/`self_assessment`/`memory_writeback` WIRED; judge/council "OFF by default", memory_writeback 不漏) | 改 §25 数字 |
| 2 | R9 §5.1 §330 | "4 WIRED + 1 SLOT READY (judge) + ... + 6 DEFERRED" | **6 WIRED + 6 DEFERRED** | 改 §330 数字 + 解释 "OFF by default" ≠ "SLOT READY" |
| 3 | R9 §5.2 §342 | "OrganOrchestrator 待 R11 实施" | **R11 spec done + R12 真实施 5 stage commit 已跑通** | 改 §342 标 R12 完成 + 1739 tests 真账 |
| 4 | R9 §10 §483 | "5 actionable 验证" | **9 actionable 验证** (5 done + 4 新加: #6 OrganOrchestrator + #7 6 DEFERRED + #8 frontend + #9 RC-7 真 modality) | 改 §483 标题 + 加 #6-#9 |
| 5 | R9 quickstart §1 §23 | "4 WIRED + 1 SLOT READY + 6 DEFERRED" | **6 WIRED + 6 DEFERRED** | 改 quickstart §23 |
| 6 | R10 §1.2 ledger 散落 | "5 WIRED + 1 SLOT READY" (commit `0e53a668` 散落) | **6 WIRED + 6 DEFERRED** | 全文统一 + commit message 改 |

**主代理 commit message 模板** (per Q1 C1 policy 主代理亲做, 0 派 sub-agent):

```
fix(docs): R9/R10/R9-quickstart spec 12 slot 数字 + OrganOrchestrator + 9 actionable 错账修正

- R9 §0 §25 + §5.1 §330: "4 WIRED + 1 SLOT READY + 6 DEFERRED" → "6 WIRED + 6 DEFERRED"
  (per R13 §1.1 + cognitive-module-wiring.md:23-35 真账; judge/council "OFF by default" ≠ "SLOT READY")
- R9 §5.2 §342: "OrganOrchestrator 待 R11 实施" → "R11 spec done + R12 真实施 5 stage commit
  (c003e078/087ab2ac/50ba2e57/29e5ce66/0afa733f, 1726→1739 tests, 0 clippy, 0 LOCKED 触碰)"
- R9 §10 §483: "5 actionable" → "9 actionable (5 done + #6 OrganOrchestrator + #7 6 DEFERRED + #8 frontend + #9 RC-7)"
- R9 quickstart §1 §23 + R10 §1.2: 同步改 "6 WIRED + 6 DEFERRED"
0 装诚实: 不重写 R9/R10 spec 主体 (565+1001 行保留), 0 引新外部 dep, 0 触碰 LOCKED 5 项, 不动 .harness-msg/
```

**改 3 doc** (`v2-gateway-frontend-integration-spec.md` + `v2-frontend-quickstart.md` + `cognitive-9-organ-integration-spec.md`), 1 commit, 主代理亲做不派.

---

## 2. R10 spec 12 slot 数字错账 (主代理亲做)

**12 slot 当前真账** (per `cognitive-module-wiring.md:23-35`):

| Slot | Status |
|---|---|
| `memory_recall` / `preference_recall` | WIRED (TurnStart) |
| `judge` / `council` | **WIRED, OFF by default** (AfterModelResponse, 需 `APEIRETH_COGNITIVE_JUDGE=1` / `_COUNCIL=1`) |
| `self_assessment` | **WIRED, Judge-backed** (AfterTurn) |
| `memory_writeback` | WIRED (AfterTurn, successful final turn only) |
| `preference_learning` | DEFERRED |
| `cognitive.critic` / `cognitive.reflection` | DEFERRED INTO JUDGE / SELF-ASSESSMENT |
| `cognitive.planner` / `cognitive.orchestrator` / `cognitive.perception` | NOT AN AGENT MODULE (forward-declared service/adapter) |

**6 WIRED** 都是 AgentModule ABI 真接; **6 DEFERRED** 中 3 个 (critic/reflection) 已并入 JUDGE/SELF-ASSESSMENT, 3 个 (planner/orchestrator/perception) 是 forward-declared service/adapter. **0 重复, 0 假装**.

**R10 错账位置**: §1.2 ledger 表主体**正确** (6 WIRED 已列), 但 §1.2 后续文本 + commit `0e53a668` "5 WIRED + 1 SLOT READY" 散落. **R10 §4.4 §253-256 自注已标 R13 真账**, 但 ledger 表主体 + R10 全文 + git commit message 散落仍需主代理审.

**修法**: 同 §1 commit message, R10 spec §1.2 文本 + commit `0e53a668` message 散落主代理审一遍统一改 "6 WIRED + 6 DEFERRED".

---

## 3. R12 working tree 跑通 — **已跑通, 不需续**

**5 stage .harness-msg/{1..5}.txt** ✅ 全是已实施 commit message 草稿 (Stage 1-5 = `c003e078` → `0afa733f`). 每 stage: tests +1~10, clippy 0 警告, LOCKED 0 触碰, 0 新外部 dep, O-6 三阶审查 + 候选/拒方案都标.

**5 重守门 baseline 验证**: `cargo test --workspace --locked` 1739 passed 0 FAILED; `cargo clippy --workspace --all-targets --locked -- -D warnings` 0 警告; LOCKED 5 项 0 触碰 (9 哲学锚 + 13 键 + 3 不可变脊柱 + workspace.version 1.2.0 + R11 baseline 12 slot + Cargo.lock); legacy compat path < 100 引用; 哲学锚表头 0 减.

**5 stage 工序**: **已完成 (5 单独 commit, 不是 1 batch)**.

**结论**: R12 = **已跑通, 主代理不需决策, 不需续派**. sub-agent 2 §7 R4 "最长串行依赖" = **解除**.

---

## 4. 9 organ UI 暴露范围决策 (主代理亲做)

**核心决策**: frontend 应暴露哪些 organ 给用户?

**候选**: **A** 默认不暴露 (UI 0) / **B** 暴露全部 9 (估 +1-2 周 UI, 主观性 +) / **C** subset 4 (E4+E7+Memory+W1) / **D** dry_run 模式 opt-in.

**行业惯例**: VSCode Continue/Cursor 默认折叠 reasoning chain; ChatGPT o1/o3 不暴露 CoT; Claude extended thinking opt-in toggle; AutoGen/CrewAI 暴露 agent 间 message 流但不暴露 agent 内部 state. **模式**: 默认 0 暴露, 用户 opt-in 才开 debug 视图.

**v1 companion 暴露历史**: v1 `RuntimeModal.svelte` 暴露 6 子系统状态 (api/companion/memory/tools/events/sessions) — **0 暴露 organ**. v1 `audit.html`/`approvals.html`/`memory.html`/`sessions.html`/`graph.html` 0 organ mention. v1 9 organ **0 UI 暴露**, 仅 8 重门控留痕 + 主代理审 (`organs.rs:48` `last_decision` + `emergence.rs:460-503` 8 重 gate). v1 `approvals.html` 暴露"批准请求"列表 (主人审批 modal 历史模式).

**推荐 + 理由**: **候选 A 默认不暴露 + 候选 D dry_run 模式 opt-in**. 理由:
1. **O-5 哲学锚 (不假装)**: 9 organ SSE schema (R9 §4.3) 是 proposal 未真生产, 默认 0 暴露 = 0 装诱导预防.
2. **v1 历史一致**: v1 0 暴露 organ, UI 6 子系统状态已成熟.
3. **行业惯例**: ChatGPT/Cursor/VSCode Continue 默认折叠 reasoning.
4. **E7 emergence 8 重门控严守** (R7 独立判断): frontend 不能为 UI 跳过门控, 默认不暴露 = 0 跳过风险.
5. **dry_run = escape hatch**: 高级用户/调试可主动 opt-in, 估 +2-3 天 UI (vs 候选 B +1-2 周).

**主代理决策点**: 候选 A + D 是否接受? 候选 A 0 暴露主对话可见性影响 = 用户看不到 organ 主动开口, 仅看 final answer. **确认候选 A + D = 主代理亲批**.

---

## 5. 主人审批 modal 行为决策 (主代理亲做)

**核心决策**: 主人审批 modal 何时触发 / 什么 UI / 怎么 approve / approve 后状态?

**R9 §7.1 spec 真账**: HTTP 透传 `403` (Denied) + `409` (ApprovalRequired) → 弹主人审批 UI (`canonical_entry.rs:269-281` HTTP error 映射已写). **触发时机**: spec 无明确行为, 仅说"前端 runtime.ts 处理 403 + 409 → 弹主人审批 UI". approve 机制: v1 用 `POST /v1/apeireth/grant` (master_token + 权限洋葱 PermissionPack, 到期自动失效) (`companion_serve.rs:1935-1964`). approve 后: 0 装 PASS — 不自动重试, 主人批准后由对话继续驱动 (`approval_requests.rs:9`).

**v1 主人审批真实载体** (`approval_requests.rs:1-10`): AI 被 RequireApproval 拒绝 → 产生**待批请求** (append-only, `apreq-*`) → 前端轮询展示 → 主人一键批准 → 复用 `/v1/apeireth/grant` PermissionPack 授权. 同 chain append-only: 批准 = 新 id + 同 chain + rev+1. 过期/手动忽略 = 同 chain + expired.

**候选**: **A** 409 ApprovalRequired 弹 modal (spec 路径, v1 一致, 需 runtime.ts 改 classifyHttpError) / **B** every tool call 前 modal (friction 极高, 不可用) / **C** dry_run 模式用户主动触发 preview (主人失去实时控制).

**行业惯例**: VSCode Continue modal + "auto-approve this session" toggle; Cursor terminal 类弹 modal, file edit 直接执行; Claude tool_use 透明执行不 modal; Copilot 0 modal diff 视图. **模式**: 默认 0 modal, 危险 tool (terminal/delete/exec) 弹 modal + auto-approve toggle.

**推荐 + 理由**: **候选 A (409 ApprovalRequired 弹 modal) + "session auto-approve" toggle (类似 VSCode Continue)**. 理由:
1. **R9 spec 明确路径**: HTTP 透传 409 → 弹 modal 是 spec 写的.
2. **v1 历史一致**: v1 `approval_requests` append-only + `POST /v1/apeireth/grant` 已成熟.
3. **O-5 0 装诚实**: 不假装"已自动批准", modal 让主人**真**决策.
4. **runtime.ts 改动最小**: 改 `classifyHttpError()` + 加 `ApprovalModal.svelte` (~80 行, 类似 `RuntimeModal.svelte`).
5. **escape hatch**: "session auto-approve" toggle 让主人**主动**减少 friction.
6. **403 Denied**: **不**弹 modal, 仅 toast 提示 (主人**不**能 approve denied, 拒是绝对禁止).

**approve 后状态**: 主人点 modal "批准" → frontend 调 `POST /v1/apeireth/grant` (master_token + tool + hours) → 关闭 modal + toast "已批准 X 工具 1 小时" + **不自动重试** (`approval_requests.rs:9`). 用户**主动**重发 message 触发 tool 重试.

**主代理决策点**: 候选 A + auto-approve toggle + 拒绝 modal 关闭 (仅 toast) = 主代理亲批.

---

## 6. Tauri keyring 决策 (主代理亲做)

**关键背景**: v2 `crates/adapters/cli/src/keyring_bootstrap.rs` (RC-9 真接, 191 行):
- 4 backend: **platform** (Linux Secret Service / macOS Keychain / Windows Credential Manager) / **encrypted-file** (~/.apeireth/keyring/) / **in-memory** (测试) / **auto** (probe + fallback)
- env `APEIRETH_KEYRING_BACKEND` 选 backend, fallback `EnvCredentialResolver`
- 0 装诚实: 退化时**真**退化 (`eprintln` 写退化原因)

**frontend runtime.ts 现状** (`runtime.ts:1-10`): "Security invariant: apiKey / masterToken are NEVER persisted to localStorage". `loadConfig()` (L126-169) 默认 baseUrl, model, **apiKey 空字符串** (transient in-memory). 当前 alpha 用 env `APEIRETH_API_KEY`.

**候选**: **A** `tauri-plugin-store` 官方 K/V (需新 dep + 边界冲突) / **B** `tauri-plugin-stronghold` Rust AES (复杂度 +) / **C** `tauri-plugin-keyring` 第三方调 OS keyring (边界冲突 + libsecret 依赖) / **D** 自实现 browser localStorage + AES-GCM (**安全倒退**, 违反 runtime.ts L1-10) / **E** 复用 v2 后端 keyring (RC-9), frontend transient in-memory, 启动 fetch 一次 (`apeireth-server --print-key` 子命令 or env `APEIRETH_API_KEY`).

**0 触碰 LOCKED**: A/B/C 需 `src-tauri/Cargo.toml` 加 dep (frontend 独立 workspace 不污染根, 但 +Tauri shell 复杂度). D/E 0 加 dep. **E = 0 引新外部 dep**.

**跨平台支持**: A/B/C 全平台 + 加密依赖 OS; D 全平台 (browser localStorage); **E 跨"前端+后端"全平台 (后端 RC-9 已接 OS keyring)**.

**推荐 + 理由**: **候选 E (复用 v2 后端 keyring, frontend transient in-memory)**. 理由:
1. **O-5 0 装诚实**: 候选 D 加密假 OS 不等同真 keyring, 选 E 0 装假装.
2. **0 重复造 keyring**: v2 RC-9 已 4 backend 真接, Tauri shell 不应再写.
3. **0 apeireth-* dep 边界** (per README L120-123): 候选 A/B/C 需加 dep, 候选 E 0 加, Tauri shell 保持 ~110 lines.
4. **runtime.ts 已 0 装诚实**: 当前 alpha 已 transient in-memory + NEVER localStorage, 候选 E 延续.
5. **frontend 集成成本 0**: 仅加 ~30 行 (启动 fetch + memory cache + 0 落盘).
6. **真生产路径**: v2.0.0 release 主人跑 `apeireth-server --keyring-setup` (RC-9), key 进 OS keyring, frontend 启动 fetch transient 用, 重启电脑 keyring 仍存.
7. **0 引新外部 dep**: 0 触碰 LOCKED 5 项 + 0 改根 workspace.

**主代理决策点**: 候选 E = 主代理亲批. 派 sub-agent C (Tauri shell 集成) 时, **不**装 store/stronghold/keyring plugin, 仅 runtime.ts 加 ~30 行 transient fetch.

---

## 7. 0 触碰 LOCKED 验证 (B 块 6 项决策后)

| LOCKED 项 | 触碰? | 验证 |
|---|---|---|
| 5 项 LOCKED (`10-locked.md` + `philosophy.md`) | ❌ | 6 项决策全文档级 + 0 改根 workspace Rust |
| 9 哲学锚本体 (`eight_anchors.rs:58-79`) | ❌ | R12 0 触碰 + 6 项决策不改 anchor |
| 13 键 (`philosophy.rs:142`) | ❌ | frontend 不引入 13 键降级标准 |
| workspace.version = "1.2.0" (`Cargo.toml:43`) | ❌ | frontend 独立 workspace, 0 改根 |
| R11 baseline (12 slot + Cargo.lock) | ❌ | R12 0 触碰 + 6 项决策不改 |
| R12 baseline (1739 tests + 0 clippy) | ❌ | R12 已 commit, 6 项决策不改 |

**结论**: B 块 6 项决策**完全 0 触碰 LOCKED 6 项** + R12 baseline (新加).

---

## 8. 派单建议 (主代理亲批后)

| 块 | 推荐 | 估时 |
|---|---|---|
| 主代理亲做 6 项核验 + commit (§1/§2 + §4/§5/§6 决策) | 主代理亲做 | 1-2 天 |
| gateway SSE + auth + panel 端点 (A+C+G+H+I+K) | 派 sub-agent A | 3-4 周 |
| frontend runtime.ts (B+E+F+主人审批 modal) | 派 sub-agent B | 2-3 周 |
| Tauri shell keyring 集成 (J per §6 候选 E) | 派 sub-agent C | 3-5 天 |
| R12 OrganOrchestrator | **已跑通, 不续派** | 0 |
| E2E + 5 重守门 (M) | 派 sub-agent D (或合 A 末尾) | 1-2 周 |

**总估时**: 主代理 1-2 天 + A/B/C 并行 4-5 周 + M 1-2 周 = **5-7 周 critical path**. **派单门槛**: 主代理亲批 §1-§6 → 派 A + B + C 并行 → 2027-Q1 启动, 2027-Q2 完.

---

## 9. 1 段交付 (给主代理 Mavis)

**B 块 6 项主代理亲做决策真账 = 🟢 可启动**.

**重大更新**: R12 working tree 已跑通 (5 stage commit + 1739 tests + 0 LOCKED 触碰), §8.3 #3 = 已完成, 不再阻塞 B 块.

**6 项决策**: ① R9/R10 spec 4 处错账 (主代理亲做 commit 改 3 doc): "4 WIRED + 1 SLOT READY" → "6 WIRED + 6 DEFERRED"; §5.2 §342 OrganOrchestrator 标 R12 完成; §10 §483 加 4 新加 actionable. ② R10 spec 12 slot: 同 §1 commit, 全文统一. ③ R12 working tree: 已跑通, 不续派. ④ 9 organ UI 暴露: 候选 A 默认 0 暴露 + 候选 D dry_run 模式 opt-in. ⑤ 主人审批 modal: 候选 A 409 ApprovalRequired 弹 modal + session auto-approve toggle + 拒绝 toast. ⑥ Tauri keyring: 候选 E 复用 v2 后端 keyring (RC-9), frontend transient in-memory, 0 新外部 dep.

**派单门槛**: 主代理亲批 §1-§6 → 派 A (gateway) + B (frontend) + C (Tauri shell trivial) 并行, **5-7 周 critical path**, 2027-Q1 启动, 2027-Q2 完. **0 触碰 LOCKED 6 项 + R12 baseline 0 触碰**.

**未做**: ❌ 不写代码 / ❌ 不 git add/commit (§1/§2 commit 由主代理执行) / ❌ 不真实施 5-7 周 (派 A/B/C) / ❌ 不派 R12 续跑 (已跑通).

(End of B-block 决策真账, sub-agent 写, 主代理 Mavis 待审, 不 commit)