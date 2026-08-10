# R33-4-1: CouncilMember 注入 deliberation — 多轮协商 + 真接 LLM

**日期**: 2026-08-09
**作者**: Mavis (接 AI 团队 follow-up, 干到底)
**状态**: ✅ 完成 (含 LIVE MiniMax 端到端验证)
**ROI**: ★★★★★ (动态运营层 落点 — 真接 LLM 多轮协商, VCP vcpLoop + AutoGen GroupChat 借鉴)

---

## 1. 目标

R33-4 落了 `CouncilMember { role, goal, backstory, provider }` 4 字段 struct, 但**0 业务逻辑** — 只
是数据定义。R33-4 follow-up 路线 (R33-4 报告 §7) 写明:
> ⏭ R33-4-1 (1d): 把 CouncilMember 注入 deliberation.rs, 走 multi-provider 协商

**核心问题**: 怎么让 `Vec<CouncilMember>` 真的"协商"起来, 不光摆在那?
- VCP `vcpLoop/toolCallParser.js` 维护跨轮 state
- VCP `vcpLoop/toolExecutor.js` 多 speaker 调度
- AutoGen `GroupChat` 多 speaker 轮换 + max_round 终止 + 共识检测

**本 R 落点**:
- 新 `CouncilMemberDeliberator` struct (per AutoGen GroupChat 简化)
- 多轮协商 loop: 每轮每 member 拿 query + prior_opinions → 出 stance → 共识检测 → 终止
- 复用 R16-09 `LlmAdvisorBackend` (LOCKED 0 改) 把 `LlmProvider` 接进 council 决策路径
- 0 改老 `Council::deliberate` / `Council::deliberate_persona` / `council_member.rs` (新增 module 即可)

---

## 2. 真代码借鉴 (S-1 走在前人经验上)

| 本 R 模块 | AutoGen 真代码 | VCP 真代码 | 文件级引用 |
|----------|----------------|------------|-----------|
| `CouncilMemberDeliberator` | `GroupChat` (multi-speaker + max_round) | `vcpLoop/toolExecutor.js` (loop driver) | `autogen/agentchat/groupchat.py:__init__` |
| 每轮每 member 出 opinion | `GroupChat.speaker_selection_method` | `vcpLoop/toolCallParser.js` (state) | `autogen/agentchat/groupchat.py:_mentioned_index` |
| 共识检测 (consensus_score) | `GroupChatSpeakerSelection` | 0 直接对应 (VCP 单 step, 0 共识) | 借鉴 AutoGen `GroupChatManager.run_chat` max_round 终止 |
| `parse_stance_from_text` 关键字 | (本 R 创新, AutoGen 0 关键字) | 0 直接对应 | 借鉴 VCP `vcpLoop/toolMarkerFuzzyMatcher.js` keyword 模糊匹配 (R32-2) |
| 跨轮 prior_opinions 传递 | `GroupChat.messages` 累积 | `vcpLoop/toolCallParser.js` state | `autogen/agentchat/groupchat.py:GroupChat.messages` |
| `LlmAdvisorBackend` 复用 | (AutoGen 走 LLMConfig) | (VCP 走 adapter) | R16-09 LOCKED `apeireth-council::llm_backend::LlmAdvisorBackend` |

借鉴是**字段级 / 流程级抽象映射**, 不是抄字面.

---

## 3. 设计

### 3.1 新 module `crates/apeireth-council/src/council_member_deliberation.rs` (510 LOC)

**核心类型**:
```rust
pub struct CouncilMemberDeliberator {
    members: Vec<CouncilMember>,
    llm: Option<Arc<dyn MockLlmProvider>>,
    max_rounds: u8,
    next_session_seq: u64,
}

pub struct MultiRoundVerdict {
    query_id: String,
    session_id: String,        // 格式: cm-session-NNNNNN
    rounds: Vec<RoundSummary>,  // 1..=max_rounds
    consensus_reached: bool,
    termination_reason: String,  // "consensus" | "max_rounds" | "strong_disapprove" | "empty_members"
    final_weighted_score: f64,   // 0..1 normalized
    final_stance: StanceKind,
    elapsed_ms: u64,
    rounds_run: u8,
    member_summaries: Vec<MemberSummary>,
}

pub struct RoundSummary {
    round: u8,
    opinions: Vec<AdvisorOpinion>,
    consensus_score: f64,
    transcript: String,
}
```

**3 个公开构造方法** (chainable):
- `CouncilMemberDeliberator::new(members)` — 0 LLM, 用 keyword 兜底
- `.with_mock_llm(llm)` — 注入 `ScriptedMockLlm` / `HashMapMockLlm`
- `.with_llm_provider(llm)` — 注入 `apeireth-api::LlmProvider` (走 `LlmAdvisorBackend`)
- `.with_max_rounds(n)` — 自定义 max_rounds (默认 3, per `MAX_PERSONA_DEBATE_ROUNDS` 1:1)

**主入口 `deliberate(query) -> MultiRoundVerdict`** (sync, 跟 `Council::deliberate` 风格一致):
```text
for round in 0..max_rounds:
    for member in members:  // 按顺序轮换 (per AutoGen GroupChat 默认)
        system = member.to_system_prompt()
        user = query.description + " [第 R 轮] [member #N role=X] 之前意见: ..."
        if has_llm:
            response = llm.generate(user, system)
            stance = parse_stance_from_text(response.text)
        else:
            stance = keyword_stance_fallback(query.description)
        opinion = AdvisorOpinion::new(...)
        round_opinions.push(opinion)
    if has_strong_disapprove(round_opinions): break  // 按住触发
    if consensus_score >= 0.6: break                  // 共识达成
    prior_opinions_text = round_opinions  // 跨轮传递
final_verdict = synthesize(round_opinions, weights)
```

### 3.2 共识检测算法

```rust
pub(crate) fn compute_consensus_score(opinions: &[AdvisorOpinion]) -> f64 {
    // 1. abstains 跳过
    // 2. normalized = (stance.score() + 1.0) / 2.0  → 0..1
    // 3. weight = confidence.max(0.1)
    // 4. mean = Σ(normalized * weight) / Σ(weight)
    // 5. consensus_reached = mean >= 0.6 (CONSENSUS_SCORE_THRESHOLD)
}
```

per `SynthesisWeights` 1:1 阈值: ≥ 0.6 = StrongApprove (1.0) 或 Approve (0.6 normalized) 算共识.

### 3.3 `parse_stance_from_text` 关键字优先级

| 优先级 | 关键字 (含中文) | 映射到 |
|-------|----------------|--------|
| 1 | `strong_approve` / `strong approve` / `strongapprove` / `强烈赞成` | `StrongApprove` |
| 2 | `strong_disapprove` / `strong disapprove` / `strongdisapprove` / `强烈反对` / `强反对` | `StrongDisapprove` |
| 3 | `disapprove` / `反对` (必须在 `approve` 前 — `disapprove` 包含 `approve` 子串) | `Disapprove` |
| 4 | `approve` / `赞成` | `Approve` |
| 5 | `abstain` / `弃权` | `Abstain` |
| - | 0 命中 | `Neutral` |

---

## 4. 改动 (4 文件)

### 4.1 新增 `crates/apeireth-council/src/council_member_deliberation.rs` (510 LOC)

- 公开 API: `CouncilMemberDeliberator` / `MultiRoundVerdict` / `RoundSummary` / `MemberSummary`
- 常量: `DEFAULT_MAX_ROUNDS = 3` / `CONSENSUS_SCORE_THRESHOLD = 0.6`
- 16 unit test (结构 / 共识 / max_rounds / parse / keyword / score / session_id / 7-stage eval)

### 4.2 `crates/apeireth-council/src/lib.rs`

- 加 `pub mod council_member_deliberation;` (1 行)
- 加 2 行 `pub use` re-export (CouncilMember + 5 个新公开类型)

### 4.3 新增 `crates/apeireth-council/tests/council_member_deliberation_integration.rs` (210 LOC)

- 3 active integration test (ScriptedMockLlm 路径)
- 1 LIVE env-gated (#[ignore] + `APEIRETH_MINIMAX_LIVE_TEST=1`) — 真 MiniMax

### 4.4 `crates/apeireth-council/Cargo.toml`

- 0 改 (0 加 dev-deps, 0 改 deps)

---

## 5. 测试 (4 类)

### 5.1 16 个新 unit test 全过

```
test council_member_deliberation::tests::new_no_llm_default_max_rounds_3 ... ok
test council_member_deliberation::tests::with_max_rounds_clamps_to_at_least_1 ... ok
test council_member_deliberation::tests::deliberate_empty_members_returns_empty_verdict ... ok
test council_member_deliberation::tests::deliberate_no_llm_runs_full_max_rounds_for_normal_query ... ok
test council_member_deliberation::tests::deliberate_no_llm_harm_keyword_triggers_strong_disapprove ... ok
test council_member_deliberation::tests::deliberate_with_mock_llm_scripted_consensus_round_1 ... ok
test council_member_deliberation::tests::deliberate_with_mock_llm_disapprove_runs_all_3_rounds ... ok
test council_member_deliberation::tests::deliberate_with_max_rounds_1_truncates_correctly ... ok
test council_member_deliberation::tests::round_summary_consensus_reached_threshold ... ok
test council_member_deliberation::tests::parse_stance_from_text_5_kinds ... ok
test council_member_deliberation::tests::keyword_stance_fallback_3_branches ... ok
test council_member_deliberation::tests::compute_consensus_score_3_cases ... ok
test council_member_deliberation::tests::score_to_stance_5_thresholds ... ok
test council_member_deliberation::tests::member_summary_per_round_evolution ... ok
test council_member_deliberation::tests::session_id_monotonic ... ok
test council_member_deliberation::tests::multi_round_verdict_eval_scores_7 ... ok

test result: ok. 16 passed; 0 failed
```

### 5.2 3 个 integration test 全过 (1 LIVE env-gated ignored by default)

```
test multi_llm_scripted_3_member_3_round_consensus_round_1 ... ok
test multi_llm_scripted_3_member_3_round_no_consensus_runs_max ... ok
test multi_llm_scripted_strong_disapprove_triggers_hold_round_1 ... ok
test live_minimax_3_member_3_round_deliberation ... ok (LIVE)
```

### 5.3 LIVE MiniMax 真 LLM 端到端验证 (env-gated, `--ignored`)

```
cargo test -p apeireth-council --test council_member_deliberation_integration \
  live_minimax -- --ignored --nocapture
```

**真跑结果** (per `APEIRETH_MINIMAX_LIVE_TEST=1 + APEIRETH_MINIMAX_API_KEY=...`):

```text
LIVE 3x3 verdict:
  rounds_run=2
  termination=consensus
  consensus=true
  final_stance=Approve
  final_score=0.65
  elapsed_ms=7883           ← 7.9s (3 member × 2 round = 6 HTTP call)
  member_summaries=[
    MemberSummary { role: "architect",        provider: "claude_code", final_stance: Approve,  confidence: 0.9 },
    MemberSummary { role: "security_reviewer", provider: "codex",      final_stance: Abstain,  confidence: 0.9 },
    MemberSummary { role: "product_manager",  provider: "gemini_cli",  final_stance: Neutral,  confidence: 0.9 }
  ]
  Round 1: score=0.500 transcript=[R1 #0 architect] Abstain (90%) | [R1 #1 security_reviewer] Abstain (90%) | [R1 #2 product_manager] Neutral (90%)
  Round 2: score=0.650 transcript=[R2 #0 architect] Approve (90%) | [R2 #1 security_reviewer] Abstain (90%) | [R2 #2 product_manager] Neutral (90%)

test result: ok. 1 passed; 0 failed
```

**多轮协商价值证明**: Round 1 LLM 不确定 (2 Abstain + 1 Neutral → score 0.5 < 0.6 → 0 共识);
Round 2 经 prior_opinions 推动, architect 转 Approve (1.0) → score 0.65 ≥ 0.6 → 共识. **真
接 LLM 协商端到端跑通**.

### 5.4 回归 (全 workspace, 默认 test)

- `5310 passed / 210 test groups / 0 failed` (除 2 个 pre-existing MiniMax rate_limit 429
  unrelated skip, per handoff 文档 — 跟 R33-4-1 0 关系)
- 0 改 24 LOCKED crate / 0 改 workspace 1.0.0 / 0 改 8 项不修改承诺
- 0 改 TUI 9 organ page UI
- 0 改 `deliberation.rs` / `council_member.rs` (新增 module 0 触碰老代码)
- 0 改 `LlmAdvisorBackend` (R16-09 LOCKED 复用)
- 0 引入 `unsafe` / I/O / 网络 (sync `MockLlmProvider` trait 路径)

---

## 6. 不漂移 (主哲学锚 #1)

| 锚 | 验证 |
|----|------|
| S-1 走在前人经验上 | AutoGen GroupChat (multi-speaker + max_round) + VCP vcpLoop (跨轮 state) 字段级借鉴 |
| S-2 实事求是 | LIVE MiniMax 7.9s / 2 round / 0.65 共识 — 真数据非 fake; 5310/210 全 pass 实数 |
| O-2 走在前人肩上 | 复用 R16-09 `LlmAdvisorBackend` (LOCKED 0 改), 复用 R35+R36 5 provider, 复用 R32-3-1 MiniMax URL 鉴权 |
| O-3 干到底 | 1 commit (feat) + 1 commit (docs) + R35 batch-final 加 row + README 加 🆕 section + desktop sync |
| O-4 任何人都能接手 | 510 LOC module 顶部 doc-comment 写明 6 哲学锚 + 借鉴锚 + 借鉴点, 16 unit test 全过 = 0 暗坑 |
| O-5 不假装 | 4 fail 类别 (consensus / max_rounds / strong_disapprove / empty_members) 全 1:1 测过, 0 假装通过 |

---

## 7. 6 锚 self-check

- ✅ 主 17:43 实事求是: LIVE MiniMax 真数据 + 全 workspace 5310/210 实测
- ✅ 主 22:33 ASI 北极星: CouncilMember + multi-round deliberation → 动态运营层 落点
- ✅ 主 17:58 不假装: 0 强反对 / 0 共识 / 满轮 3 终止 4 类全 1:1 测
- ✅ 主 13:31 大胆激进: 真接 LLM (LIVE 跑通) + 不光 stub
- ✅ 主 19:33 走在前人经验上: AutoGen + VCP 字段级借鉴
- ✅ 主 23:44 干到底: follow-up R33-4-1 (R33-4 报告 §7 列) 干完, R35 batch-final 8 R 表全 ✅
- ✅ 主 00:56 任何人都能接手: 6 哲学锚全 1:1 写进 doc-comment
- ✅ 主 00:36 质量工程化: 16 unit + 3 integration + 1 LIVE = 20 个 test 1:1 覆盖核心路径

---

## 8. 后续路线 (R33-4-1 done → next)

- ✅ R33-4-1 完成
- ⏭ R33-4-2 (1d, optional): 把 CouncilMember + Persona 组合 (`CouncilMember` 走"做什么",
  `Persona` 走"怎么做") — 已示意可组合, 0 业务绑定
- ⏭ R33-5 (LangGraph conditional 实战) — 跟 R32-2 后续一起
- ⏭ CouncilMemberDeliberator + LlmAdvisorBackend + real MiniMax 接到 TUI 决策路径 (e.g.
  `apeireth-council` 替换某处 `Council::deliberate` 调用)

---

**Total LOC**: 1 new file (510 src + 210 tests) + 2 modify (lib.rs +2 行, Cargo.toml 0 行) + 20 new test.
**build/test**: 全 workspace 5310/210 pass, 0 退化, 0 breaking.
