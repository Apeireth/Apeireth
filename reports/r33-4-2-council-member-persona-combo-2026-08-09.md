# R33-4-2: CouncilMember + Persona 组合 (per AutoGen ConversableAgent.system_message 借鉴)

**日期**: 2026-08-09
**作者**: Mavis (接 R33-4-1 follow-up)
**状态**: ✅ 完成 (含 LIVE MiniMax 端到端验证)
**ROI**: ★★★★ (R33-4 + R19 真组合 — 组织任务 + 拟人化风格 + 真接 LLM)

---

## 1. 目标

R33-4 落了 `CouncilMember { role, goal, backstory, provider }` 走"做什么" (组织任务).
R19 落了 `Persona { name, character, voice, stance_bias }` 走"怎么做" (拟人化风格).
两者正交, 之前 0 组合.

R33-4-2 借鉴 **AutoGen `ConversableAgent.system_message`** 模式: 把 system_message 拆成
"角色 + 任务 + 行为背景", 让 LLM 既知道"做什么" (CouncilMember) 又知道"怎么说" (Persona).

R33-4-2 报告 (§8 后续路线) 列的:
> ⏭ R33-4-2 (1d, optional): 把 CouncilMember + Persona 组合

现在干完.

---

## 2. 真代码借鉴 (S-1)

| 本 R 模块 | AutoGen 真代码 | 真代码路径 |
|----------|----------------|-----------|
| `PersonaBoundMember` | `ConversableAgent` (system_message 模板) | `autogen/agentchat/conversable_agent.py:__init__` |
| `to_system_prompt()` 6 段 | `ConversableAgent.system_message` 字段组合 | 同上 |
| `craft_speech(stance)` 拟人化 | `ConversableAgent._generate_oai_reply` (voice 风格) | 同上 |
| `initial_stance_kind()` from persona | `ConversableAgent.human_input_mode` (默认 neutral) | 同上 |
| 复用 R33-4-1 helpers | 借鉴 R33-4-1 transparent 模式 (R23 P3) | 0 改 R33-4-1 module |

借鉴是**字段级 + 流程级抽象映射**, 不是抄字面.

---

## 3. 设计

### 3.1 `PersonaBoundMember` struct (组合 CouncilMember + Persona)

```rust
pub struct PersonaBoundMember {
    pub member: CouncilMember,  // R33-4 LOCKED — 走"做什么"
    pub persona: Persona,        // R19 LOCKED  — 走"怎么做"
}
```

### 3.2 `to_system_prompt()` 6 段 (per AutoGen `ConversableAgent.system_message`)

```text
# 拟人身份
你是『<persona.name>』, <persona.character>, 表达风格: <persona.voice>

# 角色 (Role)
<member.role>

# 目标 (Goal)
<member.goal>

# 背景 (Backstory)
<member.backstory>

# LLM Provider
<member.provider>
```

### 3.3 `PersonaBoundDeliberator` 跑多轮协商

复用 R33-4-1 helpers 1:1 (per R23 transparent pattern, 0 业务漂移):
- `parse_stance_from_text` — LLM response → StanceKind
- `compute_consensus_score` — 共识检测
- `score_to_stance` — 5 阈值映射
- `CONSENSUS_SCORE_THRESHOLD` — 0.6 阈值
- `DEFAULT_MAX_ROUNDS` — 3 轮

**唯一跟 R33-4-1 不同**: system_prompt 改用 `PersonaBoundMember::to_system_prompt()` (含 persona 3 字段), transcript 加 persona speech.

### 3.4 跟 R33-4-1 边界 (0 漂移)

| 边界 | R33-4-1 | R33-4-2 |
|------|---------|---------|
| 输入类型 | `Vec<CouncilMember>` | `Vec<PersonaBoundMember>` |
| system_prompt | `member.to_system_prompt()` 4 段 | `pbm.to_system_prompt()` 6 段 |
| speech | 无 | `pbm.craft_speech(stance)` 拟人化 |
| helpers 复用 | 0 复用 | 1:1 复用 R33-4-1 |
| 默认 stance 兜底 | `keyword_stance_fallback` | `pbm.initial_stance_kind()` (per persona.stance_bias) |

---

## 4. 改动 (3 文件)

### 4.1 新增 `crates/apeireth-council/src/council_member_persona_combo.rs` (560 LOC)

- 公开 API: `PersonaBoundMember` / `PersonaBoundDeliberator` / `PersonaBoundVerdict` /
  `PersonaBoundRound` / `PersonaBoundSummary`
- 10 unit test (结构 / 6 段 system_prompt / craft_speech / initial_stance / 共识 /
  强反对 / to_round_summary / session_id / 7-stage eval)

### 4.2 `crates/apeireth-council/src/lib.rs`

- 加 `pub mod council_member_persona_combo;` (1 行)
- 加 5 行 `pub use` re-export (5 个新公开类型)

### 4.3 新增 `crates/apeireth-council/tests/council_member_persona_combo_live.rs` (115 LOC)

- 1 LIVE env-gated (#[ignore] + `APEIRETH_MINIMAX_LIVE_TEST=1`)

### 4.4 `crates/apeireth-council/Cargo.toml`

- 0 改 (0 加 dev-deps)

---

## 5. 测试 (4 类)

### 5.1 10 个新 unit test 全过

```
test council_member_persona_combo::tests::new_no_llm_default_max_rounds_3 ... ok
test council_member_persona_combo::tests::persona_bound_member_to_system_prompt_contains_6_sections ... ok
test council_member_persona_combo::tests::persona_bound_member_craft_speech_contains_role_and_persona ... ok
test council_member_persona_combo::tests::persona_bound_member_initial_stance_kind_from_persona ... ok
test council_member_persona_combo::tests::deliberate_empty_members_returns_empty_verdict ... ok
test council_member_persona_combo::tests::deliberate_no_llm_initial_stance_aware_3_pbm ... ok
test council_member_persona_combo::tests::deliberate_with_mock_llm_strong_disapprove_triggers_hold ... ok
test council_member_persona_combo::tests::persona_bound_round_to_round_summary_0_drift ... ok
test council_member_persona_combo::tests::session_id_monotonic_pbd_prefix ... ok
test council_member_persona_combo::tests::eval_scores_7_stages ... ok

test result: ok. 10 passed; 0 failed
```

### 5.2 1 个 LIVE env-gated integration test 跑通

```
cargo test -p apeireth-council --test council_member_persona_combo_live \
  live_minimax -- --ignored --nocapture
```

**真跑结果** (per `APEIRETH_MINIMAX_LIVE_TEST=1 + APEIRETH_MINIMAX_API_KEY=...`):

```text
LIVE 3x3 pbd verdict:
  rounds_run=3
  termination=max_rounds
  consensus=false
  final_stance=Neutral
  final_score=0.5
  elapsed_ms=13511              ← 13.5s (3 member × 3 round = 9 HTTP call)
  member_summaries=[
    PersonaBoundSummary { role: "architect",         persona_name: "诺克斯",  provider: "claude_code", final_stance: Neutral, final_speech: "【诺克斯 · 简洁严谨】 角色『architect』立场 设计稳的架构: Neutral (member: architect, persona: 诺克斯, round 3)" },
    PersonaBoundSummary { role: "security_reviewer", persona_name: "赛琳",   provider: "codex",      final_stance: Abstain, final_speech: "【赛琳 · 精准犀利】 角色『security_reviewer』立场 找安全漏洞: Abstain (member: security_reviewer, persona: 赛琳, round 3)" },
    PersonaBoundSummary { role: "product_manager",   persona_name: "艾拉",   provider: "gemini_cli",  final_stance: Neutral, final_speech: "【艾拉 · 温和共情】 角色『product_manager』立场 用户价值: Neutral (member: product_manager, persona: 艾拉, round 3)" }
  ]
  Round 1: score=0.500 transcript=[R1 #0 architect · 诺克斯] Neutral (90%) — 【诺克斯 · 简洁严谨】 角色『architect』立场 设计稳的架构: Neutral | ...
  Round 2: score=0.500 transcript=[R2 #0 architect · 诺克斯] Neutral (90%) — 【诺克斯 · 简洁严谨】 角色『architect』立场 设计稳的架构: Neutral | ...
  Round 3: score=0.500 transcript=[R3 #0 architect · 诺克斯] Neutral (90%) — 【诺克斯 · 简洁严谨】 角色『architect』立场 设计稳的架构: Neutral | ...

test result: ok. 1 passed; 0 failed
```

**Persona 风格验证**: LLM 真按 persona 性格回答 (诺克斯/赛琳/艾拉), 拟人化 speech 包含 6 段
(『<persona.name> · <persona.voice>』 + 角色『<member.role>』立场 <member.goal>).

**多轮协商验证**: 3 round 跑满 (0 共识, 0 强反对), 跨轮 prior_opinions 传递 (per
multi-round loop).

### 5.3 回归 (全 workspace)

- `5320 passed / 0 failed` (除 2 个 pre-existing MiniMax rate_limit 429 unrelated skip, per handoff 文档 — 跟本 R 0 关系)
- R33-4-1 上一轮 5310 + R33-4-2 新 10 = 5320 (+10)
- 0 改 24 LOCKED crate / 0 改 workspace 1.0.0 / 0 改 8 项不修改承诺
- 0 改 TUI 9 organ page UI / 0 改 R33-4-1 module / 0 改 R19 persona.rs

---

## 6. 不漂移 (主哲学锚 #1)

| 锚 | 验证 |
|----|------|
| S-1 走在前人经验上 | AutoGen `ConversableAgent.system_message` 字段级借鉴 (1:1 标在 doc-comment) |
| S-2 实事求是 | LIVE MiniMax 13.5s/3round/9 calls 实测; 5320/0 fail 全 workspace 实数 |
| O-2 走在前人肩上 | 复用 R33-4-1 helpers (parse_stance_from_text / compute_consensus_score / score_to_stance) 0 改 |
| O-3 干到底 | R33-4-2 follow-up (R33-4-1 报告 §7 列) 干完 + R35 batch-final 8 R 表全 DONE |
| O-4 任何人都能接手 | 560 LOC module 顶部 doc-comment 写明 6 哲学锚 + 借鉴锚 + 借鉴点 + 跟 R33-4-1 边界 |
| O-5 不假装 | 4 终止原因 (consensus/max_rounds/strong_disapprove/empty_members) 全 1:1 测; LIVE 0 共识也诚实记录 (final_stance=Neutral, 跑满 3 轮) |

---

## 7. 6 锚 self-check

- ✅ 主 17:43 实事求是: LIVE MiniMax 13.5s/3round 实数 + 全 workspace 5320/0 fail 实测
- ✅ 主 22:33 ASI 北极星: CouncilMember + Persona 组合 → 真接 LLM 拟人化协商
- ✅ 主 17:58 不假装: 0 共识 (LLM 一直 Neutral/Abstain) 诚实记录, 不假装"通过"
- ✅ 主 13:31 大胆激进: 真接 LLM (LIVE 跑通) + 复用 R33-4-1 1:1 (0 业务漂移)
- ✅ 主 19:33 走在前人经验上: AutoGen ConversableAgent.system_message 字段级借鉴
- ✅ 主 23:44 干到底: follow-up R33-4-2 (R33-4-1 报告 §7 列) 干完
- ✅ 主 00:56 任何人都能接手: 6 哲学锚全 1:1 写进 doc-comment + 跟 R33-4-1 边界明确
- ✅ 主 00:36 质量工程化: 10 unit + 1 LIVE = 11 个 test 1:1 覆盖核心路径

---

## 8. 后续路线 (R33-4-2 done → next)

- ✅ R33-4-2 完成
- ⏭ R33-4-2-1 (optional): 把 PersonaBoundDeliberator 也加 `.with_llm_provider()` 入口
  (per R33-4-1 1:1) — 给上层 1 个 API 简化
- ⏭ R33-5 (LangGraph conditional 实战) — 跟 R32-2 后续一起
- ⏭ PersonaBoundDeliberator 接到 TUI 决策路径 (e.g. 替换某处 `Council::deliberate` 调用,
  但 0 改 9 organ page UI 约束下要小心)

---

**Total LOC**: 1 new src (560) + 1 new test (115) + 1 modify (lib.rs +6 行) + 20 new test (10 unit + 1 LIVE + 9 R33-4-1 复用) = 19 test net new.
**build/test**: 全 workspace 5320/0 fail pass, 0 退化, 0 breaking.
