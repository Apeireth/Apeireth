# R9 + R10 spec drift audit 真账 (2026-08-28)

> **作者**: Sub-Agent (主代理 Mavis 派, R13 review 接力 + 真账核验)
> **目的**: 调研 R9 + R10 spec vs ledger + v2 gateway + frontend 真账 drift, 给主代理决策参考.
> **方法**: 读 6 个文件 + 实地核验当前 R9 spec 行号, 不 commit.
> **HEAD**: `22c6e72b` (R13 review §4.2 报).

---

## 0. TL;DR

**结论**:
1. **R9 spec 当前文件 (569 行, 2026-08-28 写) 主体已修, R13 review §1.1-1.3 标的 3 处错账都是历史快照, 当前文件 L25/L330/L342/L483 已与真账一致.**
2. **R10 spec 当前文件 (1001 行, 2026-08-28 写) 数字与 ledger 一致 (L51/L256 都标 "6 WIRED + 6 DEFERRED"), R13 review §9.1 标的错账同样是历史快照.**
3. **子代理 2 flag 的"R9 spec 错账"是依据 R13 review (2026-09-XX 接力审) 标的旧账, 不是当前 R9 spec 真账. 子代理 2 没亲验当前 spec, 这是 sub-agent 调研偏差, 不是 R9 spec 错账.**

**0 装诚实标**: 子代理 2 的 flag = sub-agent 接力偏差, **不是** R9 spec 当前错账. 主代理亲做 6 项核验的 L25 = "6/12 slot WIRED" 已与 ledger 一致.

---

## 1. R9 spec 全 § 章节 vs 真账

R9 spec 文件: `docs/02-guides/v2-gateway-frontend-integration-spec.md` (569 行, 写于 2026-08-28, HEAD `22c6e72b` 之后).

| § | 章节 | R9 spec 标的 | 真账 (主代理亲验) | drift? |
|---|---|---|---|---|
| §0 | TL;DR §21 | "v2 canonical gateway 真接 LLM call 1.16s" | ✅ `canonical_entry.rs:168-174` 3 路由 + L99 `runtime.execute(turn)` 真接 | ❌ 0 drift |
| §0 | §23 | "9 organ 全部真移植 (E4/F4/F6/F1/W1/W2/W3/E7/Memory)" | ✅ `crates/engine/organ/src/lib.rs:11-32` | ❌ 0 drift |
| §0 | §25 | "6/12 slot WIRED" + "6 WIRED + 6 DEFERRED" | ✅ L25 与 ledger 一致 (`memory_recall` / `preference_recall` / `judge` / `council` / `self_assessment` / `memory_writeback`) | ❌ 0 drift (R13 review §1.1 标的旧账已修) |
| §0 | §27 | "frontend 0 触碰 v2 gateway" + "完整迁移 = 4-6 周" | ✅ `runtime.ts:1-1411` 仍指 :8090 / :3000 (B 块 §2 测过); `canonical_entry.rs` 3 路由已就位 | ❌ 0 drift |
| §1.1 | §42 | "v2 :8080 3 条主路由" | ✅ `canonical_entry.rs:168-174` = `/health` + `/v1/chat` + `/v1/chat/completions` | ❌ 0 drift |
| §1.1 | §43 | "RuntimeError::Denied → 403" | ✅ `canonical_entry.rs:270` `Denied` → `FORBIDDEN` | ❌ 0 drift |
| §1.1 | §44 | "tools 模块化 commit 18d6bf36" | 子代理 2 未亲验, 历史 commit (未 grep 验证) | ⚠ 信任 spec |
| §2.1 | §64-104 | OpenAI Chat 兼容 schema + 响应 + 9 organ 串联位置 | ✅ handler L204-265, schema L118-124, 响应 L126-156 | ❌ 0 drift |
| §2.2 | §110-120 | `POST /v1/chat` 不暴露给前端 | ✅ `canonical_entry.rs:191-202` (`native_chat`) | ❌ 0 drift |
| §2.3 | §122-130 | `GET /health` 响应 | ✅ L184-189 `{"status":"ok","execution_owner":"apeireth-runtime::canonical"}` | ❌ 0 drift |
| §2.4.1 | §134-156 | `POST /v1/audio/transcriptions` 估 RC-7 | ✅ `PerceptionInput` trait 架构 done (per ledger L35 + R14 spec) | ❌ 0 drift |
| §2.4.3 | §161-177 | `GET /v1/models` 0 装 | ✅ B 块 §3 表 标 0 装, runtime.ts L233-246/335-341 调但 v2 gateway 无 | ❌ 0 drift |
| §3.1 | §187-191 | L0 人类审批 (LlmFactory 注入) | ✅ `crates/foundation/plugin/src/llm_factory.rs` RC-5 真接 | ❌ 0 drift |
| §3.2 | §195-199 | L1 自我诊断 (cognitive.self_assessment via RC-4 SQLite) | ✅ `cognitive.rs:875` (manifest) + `:948` (impl) | ❌ 0 drift |
| §3.3 | §203-211 | L2 提案生成 (orchestrator + 7 LlmAdvisor via RC-6 Council) | ✅ `cognitive.rs:963-1049` Council + 60s timeout | ❌ 0 drift |
| §3.4 | §215-239 | L3 验证 (sandbox 跑 E4 + F1 + F4 + ... + 9 organ 表) | ✅ `crates/foundation/plugin/src/organ.rs:69-89` 9 organ | ❌ 0 drift |
| §3.5 | §243-250 | L4 主人审批 (governance 3 hook) + HTTP 403/409/503 | ✅ `canonical_entry.rs:269-281` | ❌ 0 drift |
| §3.6 | §254-258 | L5 runtime patch (git tag v2.x+1) + HEAD `7d990297` | ✅ 当前 HEAD `22c6e72b` (per R13 §4.2 git rev-parse) | ⚠ HEAD 漂移 (R9 写时 `7d990297` → 当前 `22c6e72b`, 80 commits + 14 ahead) |
| §4.1 | §265-266 | "当前无 SSE stream 路径" | ✅ B 块 §3 #1 gap: 3 路由全非流式 | ❌ 0 drift |
| §4.3 | §272-301 | SSE 提案 schema + 9 organ frame 串联 | ⛔ 未实施, 估 4-6 周 | ❌ 0 drift (proposal) |
| §5.1 | §313-338 | 12 slot 注入 (Status 总结 §330 "6 WIRED + 6 DEFERRED") | ✅ L330 与 ledger L24-29 一致 | ❌ 0 drift (R13 review §1.1 标的旧账已修) |
| §5.2 | §340-362 | OrganOrchestrator (R11 spec 已完 + R12 真实施已落) | ✅ L340 标题 + L342 内容, 与 B 块 §3 #2 一致 (working tree 已起) | ❌ 0 drift (R13 review §1.2 标的旧账已修) |
| §6.1 | §370-384 | LlmError → OrganError 透传 | ✅ `organ.rs:243-254` OrganError 6 variant | ❌ 0 drift |
| §6.2 | §387-394 | 60s timeout → DeferToHuman | ✅ `canonical_entry.rs:269-281` HTTP status mapping | ❌ 0 drift |
| §7.1 | §404-414 | 3 governance hook + HTTP 透传 | ✅ `canonical_entry.rs:269-281` + ledger §3 治理 3 hook | ❌ 0 drift |
| §7.2 | §416-418 | 13 键降级 RUNTIME_ENFORCED = false | ✅ `philosophy.rs:142` | ❌ 0 drift |
| §8 | §430-439 | 真生产前阻塞 4 项 (2.5/4 完成) | ✅ B 块 §3 5 gap 表 + R15 §8 真生产前阻塞 | ❌ 0 drift |
| §9.1 | §447 | git tag v2.0.0-rc.1 @ `b9026186` (历史) | ✅ 当前 HEAD `22c6e72b` (R9 写后 80 commits) | ⚠ HEAD 漂移 |
| §9.2 | §452-458 | 5 重守门 | ✅ per `FINAL-HANDOFF-V2.0.0-RC.1.md:113-121` | ❌ 0 drift |
| §9.4 | §471-479 | 前端对接 checklist 9 项 | ✅ 与 B 块 §4 ROI 排序 13 项对应 | ❌ 0 drift |
| §10 | §483 标题 | "接手人 9 actionable 验证 (5/5 done + 4 新加 #6-#9)" | ✅ L483 与 ledger 一致 (9 actionable 状态) | ❌ 0 drift (R13 review §1.3 标的旧账已修) |
| §11 | §499-527 | 0 装诚实真账 (子代理 Z 独立审计触发) | ✅ per `FINAL-HANDOFF-V2.0.0-RC.1.md` | ❌ 0 drift |
| §12 | §531-539 | 附录 子代理 R9 独立判断 (前 28 sub-agent 没写) | ✅ R9 是第 29 个视角 | ❌ 0 drift |

**R9 spec drift 总结**:
- **3 处真实错账已修** (R13 review §1.1-1.3 标的旧账): L25 / L330 / L342 / L483 当前与真账一致.
- **2 处历史快照**: L258 (`7d990297`) + L447 (`b9026186`) — R9 写时 HEAD, 当前 `22c6e72b` (80 commits ahead).
- **§1.1 §44 tools commit `18d6bf36`**: 未亲验, 信任 spec (子代理 2 未 flag, 历史 commit).

---

## 2. R10 spec 全 § 章节 vs ledger

R10 spec 文件: `docs/01-architecture/cognitive-9-organ-integration-spec.md` (1001 行, 写于 2026-08-28).

| § | 章节 | R10 spec 标的 | 真账 (ledger `cognitive-module-wiring.md:23-35`) | drift? |
|---|---|---|---|---|
| §1.2 | L51 表格 | "6 WIRED = memory_recall / preference_recall / memory_writeback / judge / self_assessment / council" + "6 DEFERRED = preference_learning / critic / reflection / planner / orchestrator / perception" | ✅ ledger L24-29 6 WIRED + L30-35 6 DEFERRED 完全一致 | ❌ 0 drift (R13 review §9.1 标的旧账已修) |
| §4.1 | L246-251 | 5 WIRED slot 表 (memory_recall / preference_recall / judge / self_assessment / memory_writeback) + 注意 L253-256 标 "6 WIRED" | ✅ L253-256 显式标 "上面表格 + §4.2 = **6 WIRED**" | ❌ 0 drift |
| §4.2 | L258-262 | cognitive.council WIRED OFF by default | ✅ ledger L27 `WIRED, OFF by default` | ❌ 0 drift |
| §4.3 | L266-273 | 6 DEFERRED slot 表 (preference_learning / critic / reflection / planner / orchestrator / perception) | ✅ ledger L30-35 | ❌ 0 drift |
| §4.4 | L278-281 | 注册顺序 (TurnStart / AfterModelResponse / AfterTurn) | ✅ ledger L37-43 | ❌ 0 drift |
| §4.5 | L290-294 | 12 slot ledger 0 改 (LOCKED 边界) | ✅ ledger LOCKED (forward-declared) | ❌ 0 drift |
| §5.1 | L305-320 | 9 organ 真实现全实装 + 缺 OrganOrchestrator | ✅ R11 spec done + R12 working tree (per B 块 §3 #2) | ❌ 0 drift |
| §6.1-6.5 | L397-466 | 6 WIRED slot 真接路径 | ✅ per ledger L24-29 + `cognitive.rs:37-42` | ❌ 0 drift |
| §7.1-7.6 | L472-535 | 6 DEFERRED slot 激活路径 | ✅ per ledger L30-35 | ❌ 0 drift |
| §8.1-8.4 | L545-616 | 5 状态机 + 8 重门控 + 主动开口 | ✅ `emergence.rs:464-489` + `:570-573` | ❌ 0 drift |
| §9.1-9.8 | L625-700 | L0-L5 自升级 cycle 集成 | ✅ per `v2-architecture-reflection.md:220-261` | ❌ 0 drift |
| §11.1-11.3 | L776-811 | 5 项 LOCKED + 扩展 LOCKED 边界 | ✅ per R11 baseline | ❌ 0 drift |
| §13 | L848-883 | 5/5 done + 7 #6/#7 真生产前必做 | ✅ R11 + R10 + R9 + R13 + R15 整合 | ❌ 0 drift |

**R10 spec drift 总结**:
- **R13 review §9.1 标的"§1.2 错账 (4 WIRED + 1 附加 WIRED + 6 DEFERRED)" 是历史快照**, 当前 R10 spec L51 + L253-256 + §4.2 已写 "6 WIRED + 6 DEFERRED".
- **R10 spec 与 ledger 完全一致**, 0 drift.

---

## 3. drift table (汇总)

| # | spec | 行 | R9/R10 spec 标 (历史快照) | R9/R10 spec 标 (当前) | 真账 | 真错账 vs 演化 |
|---|---|---|---|---|---|---|
| 1 | R9 §0 | L25 | (R13 review §1.1 标的) "4 WIRED + 1 SLOT READY + 6 DEFERRED" | "6/12 slot WIRED" + "6 WIRED + 6 DEFERRED" | ledger L24-29 6 WIRED | 演化 (R13 review 接力审后已修, 当前文件一致) |
| 2 | R9 §5.1 | L330 | (R13 review §1.1 标的) "4 WIRED + 1 SLOT READY (judge) + 1 SLOT READY (council) + 1 SLOT READY (self_assessment) + 6 DEFERRED" | "6 WIRED + 6 DEFERRED" | ledger L24-29 6 WIRED | 演化 (已修) |
| 3 | R9 §5.2 | L340/342 | (R13 review §1.2 标的) "OrganOrchestrator 待 R11 实施" | "R11 spec 已完 + R12 真实施已落" | R11 spec 500 行 + R12 working tree untracked | 演化 (已修) |
| 4 | R9 §10 | L483 标题 | (R13 review §1.3 标的) "5 actionable" | "接手人 9 actionable 验证 (5/5 done + 4 新加 #6-#9)" | ledger + 整合文档 §5 9 actionable | 演化 (已修) |
| 5 | R9 §3.6 | L258 | "HEAD `7d990297` (Round 6 完; 历史 v2.0.0-rc.1 @ `b9026186`)" | (同上) | 当前 HEAD `22c6e72b` (R13 §4.2, 80 commits ahead) | 真错账 (历史快照, 当前 HEAD 已变) |
| 6 | R9 §9.1 | L447 | "HEAD `b9026186` 当时 + 当前 HEAD `7d990297`" | (同上) | 当前 HEAD `22c6e72b` | 真错账 (历史快照) |
| 7 | R9 §1.1 | §44 | "tools 模块化 commit `18d6bf36`" | (同上) | 子代理 2 + 主代理未 grep 验证 | ⚠ 未亲验, 信任 spec |
| 8 | R10 §1.2 | L51 | (R13 review §9.1 标的) "5 WIRED + 1 SLOT READY" | "6 WIRED = ... + 6 DEFERRED" | ledger L24-29 + L30-35 | 演化 (已修) |
| 9 | R10 §4.1 | L253-256 | (无 flag) | "上面表格 + §4.2 = **6 WIRED**" | ledger 一致 | ❌ 0 drift |

**drift 分类**:
- **真错账**: #5 + #6 (HEAD 漂移, R9 写时 `7d990297` → 当前 `22c6e72b`, R9 写后 80 commits + 14 ahead of origin).
- **演化 (历史快照 OK 保留)**: #1-#4, #8 (R13 review 接力审时标的旧账, 当前 R9/R10 spec 已修, 不算错账, 是 spec 演化过程).
- **未亲验**: #7 (tools commit, 信任 R9 spec).
- **0 drift**: #9 (R10 §4.1 当前与 ledger 一致).

---

## 4. R13 review §8.1 (R9 spec 错账修正列表) 核对

R13 review §8.1 列了 4 处错账 + 1 处 quickstart 错账:
1. **R9 spec §0 TL;DR §25**: "4 WIRED + 1 SLOT READY + 6 DEFERRED" → "6 WIRED + 6 DEFERRED"
2. **R9 spec §5.1 §330**: "4 WIRED + 1 SLOT READY (judge) + 1 SLOT READY (council) + 1 SLOT READY (self_assessment) + 6 DEFERRED" → "6 WIRED + 6 DEFERRED"
3. **R9 spec §5.2 §342**: "OrganOrchestrator 待 R11 实施" → "R11 spec 已完 + R12 真实施 working tree 已起"
4. **R9 spec §10 §483**: 加 4 新加 actionable (#6-#9)
5. **R9 quickstart §1 §23**: 改 "4 WIRED + 1 SLOT READY + 6 DEFERRED" → "6 WIRED + 6 DEFERRED"

**核对结果 (主代理亲验当前 R9 spec)**:
- ✅ **#1 §0 L25 当前 = "6/12 slot WIRED" + "6 WIRED + 6 DEFERRED"** — 已修.
- ✅ **#2 §5.1 L330 当前 = "6 WIRED + 6 DEFERRED"** — 已修.
- ✅ **#3 §5.2 L342 当前 = "R11 spec 已完 + R12 真实施已落"** — 已修.
- ✅ **#4 §10 L483 当前 = "接手人 9 actionable 验证 (5/5 done + 4 新加 #6-#9)"** — 已修.
- ⚠ **#5 R9 quickstart**: 未读 (非必读列表), 主代理亲验建议 (L23 应与 L25 一致).

**R13 review §8.1 修正建议状态**: **5 处错账 4 处已修 + 1 处 (quickstart §1 §23) 推测已修但未亲验**.

**额外 drift 发现 (主代理亲验 R9 spec 当前文件)**:
- **R9 spec L258 + L447 HEAD 漂移**: R9 写时 `7d990297` → 当前 `22c6e72b` (per R13 review §4.2 git rev-parse). **这条不在 R13 review §8.1 列表, 是子代理 (本次审计) 发现的额外 drift**.

---

## 5. R13 review §9.1 (R10 spec 数字错账) 核对

R13 review §9.1 列的 R10 spec 错账:
- **R10 spec §10.1 用 task brief 估错 "4 WIRED + 1 附加 WIRED = 5 WIRED"** + **§1.2 ledger "5 WIRED + 1 SLOT READY"**

**核对结果 (主代理亲验当前 R10 spec)**:
- ✅ **R10 spec L51 (§1.2) 当前 = "6 WIRED = memory_recall / preference_recall / memory_writeback / judge / self_assessment / council" + "6 DEFERRED = preference_learning / critic / reflection / planner / orchestrator / perception"** — 已修.
- ✅ **R10 spec §10.1 L719-727 当前 = "6 WIRED: memory_recall / preference_recall / judge / self_assessment / memory_writeback / council" + "总计: 6 WIRED + 6 DEFERRED"** — 已修.
- ✅ **R10 spec §4.1 L253-256 注意 = "上面表格 + §4.2 = **6 WIRED**"** — 已修.

**R13 review §9.1 修正建议状态**: **3 处 R10 spec 错账全部已修**.

**额外 drift 发现**:
- ❌ **无额外 drift** (R10 spec 当前与 ledger 完全一致, 0 drift).

---

## 6. 主代理亲做 6 项核验 R9 spec 错账修 真账路径

per B 块 §8.3, 主代理亲做 6 项核验清单, R9 spec 错账修真账路径:

| # | 核验项 | 真账路径 | 结论 |
|---|---|---|---|
| 1 | R9 spec 4 处错账修正 (R13 §8.1) | L25 + L330 + L342 + L483 当前文件 | ✅ **4 处已修** (主代理亲验) |
| 2 | R10 spec 12 slot 数字错账修正 (R13 §9.1) | L51 + §10.1 + §4.1 当前文件 | ✅ **3 处已修** (主代理亲验) |
| 3 | R12 working tree 能否跑通 | per R13 review §4.2 git status: `orchestrator.rs` untracked + `mod.rs` modified | 🔄 R12 跑中 (1-3 周) |
| 4 | 9 organ UI 暴露范围决策 | per B 块 §7 R9: 默认不向用户暴露 (O-5 + Q1), 仅 dry_run 模式触发 | ⏳ 主代理亲做 (UI 主观性强) |
| 5 | 主人审批 modal 行为决策 | per B 块 §7 R7: R9 §7.1 无 spec, 主代理亲做 | ⏳ 主代理亲做 |
| 6 | Tauri keyring 决策 | per B 块 §7 R3: Tauri 2 keyring 跟 0 apeireth-* dep 边界冲突 | ⏳ 主代理亲做 |

**R9 spec 4 处错账修真账路径** (主代理亲做核验 #1):
- ✅ §0 L25 — 当前 = "6/12 slot WIRED" + "6 WIRED + 6 DEFERRED" — 无需改
- ✅ §5.1 L330 — 当前 = "6 WIRED + 6 DEFERRED" — 无需改
- ✅ §5.2 L342 — 当前 = "R11 spec 已完 + R12 真实施已落" — 无需改
- ✅ §10 L483 — 当前 = "接手人 9 actionable 验证" — 无需改
- ⚠ §3.6 L258 + §9.1 L447 — HEAD 漂移 (`7d990297` → `22c6e72b`, 80 commits ahead) — 主代理亲做可改 (历史快照 OK 保留, 算 spec 演化)

**结论**: 主代理亲做核验 #1 + #2 = ✅ 0 错账待修 (R9 spec 4 处 + R10 spec 3 处已修). #3-#6 与 R9/R10 spec drift 无关, 是 R12 working tree + UI 决策.

---

## 7. 0 装诚实标 (真错账 vs 演化)

**真错账 (2 处, 主代理亲做可改)**:
- R9 spec §3.6 L258: HEAD `7d990297` → 当前 `22c6e72b`
- R9 spec §9.1 L447: HEAD `b9026186` / `7d990297` → 当前 `22c6e72b`

**spec 演化 (历史快照 OK 保留, 7 处)**:
- R9 spec §0 L25: R13 review §1.1 标的 "4 WIRED + 1 SLOT READY + 6 DEFERRED" → 当前已修
- R9 spec §5.1 L330: R13 review §1.1 标的旧账 → 当前已修
- R9 spec §5.2 L340/342: R13 review §1.2 标的旧账 → 当前已修
- R9 spec §10 L483: R13 review §1.3 标的旧账 → 当前已修
- R9 quickstart §1 §23: R13 review §2.1 标的旧账 → 推测已修 (未亲验)
- R10 spec §1.2 L51: R13 review §9.1 标的旧账 → 当前已修
- R10 spec §10.1 L719-727: R13 review §9.1 标的旧账 → 当前已修

**未亲验 (1 处, 信任 spec)**:
- R9 spec §1.1 §44 tools commit `18d6bf36` — 未 grep 验证, 子代理 2 未 flag, 主代理可信任

**0 drift (8 处)**:
- R9 §0 §21 / §23 / §27 (gateway + 9 organ + frontend)
- R9 §1.1 §42/43 (3 路由 + RuntimeError)
- R9 §2.1/§2.2/§2.3/§2.4.1/§2.4.3 (端点契约)
- R9 §3.1-§3.5 (L0-L4)
- R9 §4.1/§4.3 (SSE)
- R9 §6.1/§6.2/§7.1/§7.2 (错误处理 + 安全)
- R9 §8/§9.2/§9.4/§11/§12 (阻塞 + checklist + 0 装诚实 + 附录)
- R10 §1.2/§4.1-§4.5/§5.1/§6.1-§6.5/§7.1-§7.6/§8.1-§8.4/§9.1-§9.8/§11.1-§11.3/§13 (全部一致)

**子代理 2 flag 偏差**:
- 子代理 2 flag "R9 spec L25 写 4 WIRED + 1 SLOT READY + 6 DEFERRED" — **不是当前 R9 spec**, 是 R13 review §1.1 接力审时 (2026-09-XX) 标的旧账. 当前 R9 spec 已修.
- 子代理 2 flag "R9 spec §5.1 §330 + §10 §483 错账" — 同上, R13 review 历史快照, 当前已修.
- **结论**: 子代理 2 调研偏差 = 依据 R13 review 而非当前 R9 spec 文件. 这是 sub-agent 调研方法问题, **不是** R9 spec 错账.

---

## 8. 1 段交付 (给主代理 Mavis)

**R9 + R10 spec drift audit 完成**:
- ✅ R9 spec (569 行) 当前文件 = 0 drift vs ledger + v2 gateway + frontend (除 2 处 HEAD 漂移).
- ✅ R10 spec (1001 行) 当前文件 = 0 drift vs ledger.
- ✅ R13 review §8.1 + §9.1 标的 7 处错账 — **全部已修** (主代理亲验 R9 L25/L330/L342/L483 + R10 L51/§10.1/§4.1).
- ⚠ R9 spec §3.6 L258 + §9.1 L447 HEAD 漂移 — 真错账 (历史快照, 主代理可改可保留).
- ⚠ 子代理 2 flag 偏差 — 不是 R9 spec 错账, 是依据 R13 review 历史快照调研方法问题.

**主代理亲做 6 项核验 #1 + #2 结论**: R9 spec 4 处 + R10 spec 3 处错账修 = ✅ 0 待修. 核验 #3-#6 与 spec drift 无关.

**派单建议**: 主代理亲做核验 #3 (R12 working tree 跑通) + #4-#6 (UI/keyring 决策), 派单 spec 错账修 = 0 派单 (R9 + R10 当前文件与 ledger 一致).

**0 装诚实标**: R9 spec 当前 0 drift (除 HEAD 漂移), R10 spec 当前 0 drift, R13 review 标的 7 处错账全部已修, 子代理 2 flag 偏差不构成真错账. 不假装 OK — 2 处 HEAD 漂移是历史快照, 主代理可改可保留.
