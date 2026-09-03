# R12-CoordinationContext-1 真实施 brief — v1 context.rs + context_rot.rs rot_score 融合 (2026-08-28)

> **作者**: 主代理 Mavis (写于 Round 15, 用户原话 "立刻 push, 立刻做点小的, 边做边更新文档, 注意文档工程规范, 哲学锚, 追求整体系统架构最优" 触发)
> **用途**: R12-CoordinationContext-1 派单 brief 真账 (主代理亲做 #1 spec, 不改 src, 0 装诚实标, 真实施由派 sub-agent 真实施)
> **关系**: 跟 `r12-r13-r14-implementation-brief-template-2026-08-28.md` (§3.1 R12-CoordinationContext-1 派单 brief 模板) + `round-14-v2-completion-plan-2026-08-28.md` (§2.1 #1 主代理亲做 10 项 spec) + `r11-coordination-context-gap-research-2026-08-28.md` (R11 真调研, 9 项 v2 现状) + `round-13-1-0-maturity-audit-2026-08-28.md` (8 个核心 .rs 实测) 互补

```
[Document-Meta]
Document:        docs/04-internal/r12-coordination-context-1-merge-spec-2026-08-28.md
Version:         1.0 (主代理 Mavis 写于 Round 15, v1 context.rs + context_rot.rs rot_score 融合 spec 决策冻结)
Last-Modified:   2026-08-28
Status:          🟢 活跃 (R12-CoordinationContext-1 真实施 brief, 主代理亲做 spec, 派 sub-agent 真实施)
Author:          主代理 Mavis
```

---

## 0. 用户原话触发 + 主代理自省

**用户原话** (Round 15): "立刻 push, 立刻做点小的, 边做边更新文档, 注意文档工程规范, 哲学锚, 追求整体系统架构最优"

**主代理自省** (per O-5 + S-2 实事求是 + 9 哲学锚 + 整体系统架构最优):
- ✅ 立刻 push 已做 (Round 15 commit `800bdb1a` push 成功)
- ✅ 立刻做小的 (本真账 brief, token 经济, 1 真账 doc-only)
- ✅ 边做边更新文档 (本真账写真账)
- ✅ 文档工程规范 (per 真账 brief 模板 §1, 必含 7 段)
- ✅ 哲学锚 (per Round 15 真账 brief 模板, 9 哲学锚 + 真实施真账)
- ✅ 整体系统架构最优 (per R12 真实施 critical path + v1 真账 1:1 翻译 + 物种化借签边界)

---

## 1. 真账 brief (per 真账 brief 模板 §1.1-§1.7)

### 1.1 任务 (Brief)

**任务**: v1 `context.rs` (L141-451) + `context_rot.rs` (L1-180) `rot_score` 融合 spec 决策冻结

**背景**:
- Round 11 R11-CoordinationContext sub-agent catch 我主代理真账 §1.8 标错, **1.0 真账 context.rs + context_rot.rs 同一 rot_score 两 file 各实现一遍**, 真实施前必先融合
- 1.0 真账实测 (主代理亲测 legacy/donor/apeireth-companion/src/{context.rs, context_rot.rs}):
  - **context.rs** (L141-451, ~770 行): `RotBlock` + `RotConfig` + `RotWeights` (w_duplicate 0.4 + w_stale 0.3 + w_irrelevant 0.3) + `DuplicatePair` + `RotBreakdown` — 真实施 rot_score 函数未在本文件, 文档注释 L142-143 提启发式公式但无实现
  - **context_rot.rs** (L1-180, ~526 行): `Segment` + `RotConfig` (w_repetition 0.4 + w_staleness 0.3 + w_relevance 0.3, 含 stale_half_life_turns) + `RotBreakdown` + `repetition_factor` (多行去重比 + char 6-gram 滑窗) + `query_tokens` (ASCII 小写词 + CJK char-bigram) + `rot_breakdown` (3 因子公式, 0 LLM) + `rot_score`
- 1.0 真账两个 rot_score 实现不同 (命名 + 实现细节), 真实施前主代理亲做 spec 决策冻结

**物种化借签边界** (per vision.md L29-49 + Round 13 1.0 maturity 补查):
- RotBlock / Segment 抽象是 v2 真账 cognitive memory 增维路径 (per R11-LongTermMemory 真账 6 项)
- context.rs + context_rot.rs 0 LLM 1:1 翻译 (启发式, 确定性, A/B 调权重, 0 装 PASS 标注)
- v2 真账无 rot_score 真账 (0 真实施), 借签 1.0 真账 + R20 preference_learning 1:1 翻译

**承接**:
- 主代理亲做 10 项 spec 之一 (#1, per Round 14 真实施完成计划 §2.1)
- 真账 brief 模板 (Round 15 `r12-r13-r14-implementation-brief-template-2026-08-28.md`)
- 真实施由派 sub-agent 真实施 (R12-CoordinationContext-1, 3-4 周 critical path, 估时按真账 brief)

### 1.2 必读 (Brief)

```
Apeireth v2.0 真实施必读 (主代理亲做 spec + 派 sub-agent 真实施 必读):

1. 1.0 真账实测 (主代理亲测):
   - legacy/donor/apeireth-companion/src/context.rs (L141-451, ~770 行)
   - legacy/donor/apeireth-companion/src/context_rot.rs (L1-180, ~526 行)
   - 真账 maturity (per Round 13): REAL, 1:1 可移植, 0 装 PASS 标注
   - 1.0 真账 self-flag (context.rs L142-143): "rot_score = w1·duplicate_ratio + w2·stale_ratio + w3·(1 - relevance_score)"; 默认权重 0.4 / 0.3 / 0.3; 标注"启发式, 待 A/B 调权重" (0 装)
   - 1.0 真账 self-flag (context_rot.rs L81): "多行→行级去重比; 单行→6 字滑窗去重比. 确定性"
   - 1.0 真账 self-flag (context_rot.rs L104): "ASCII 小写词 + CJK char-bigram (确定性, 无分词器依赖)"

2. 工程规范 (必含):
   - docs/04-internal/r12-r13-r14-implementation-brief-template-2026-08-28.md (派单 brief 模板)
   - docs/04-internal/v2-reference-handbook-2026-08-28.md (§3.1 brief 模板 + §4 改前必跑 + §5 commit msg 模板 + §7 工程规范 + §8.5 hook)
   - docs/04-internal/ENGINEER-MANIFESTO.md (§13 12 真实陷阱 + §10 LOCKED 5 项)

3. 物种化借签 (Round 10 5 真调研 + Round 11 6 gap 真调研):
   - docs/01-architecture/r7-mio-species-research-2026-08-28.md (Windows 本地优先 + 日记反思+写回耦合, 物种化借签边界)
   - docs/01-architecture/apeireth-true-understanding-2026-08-28.md (三面一体 + 五原型 + 物种化)
   - docs/01-architecture/vision.md (L29-49, 物种而非个体 + 五原型)
   - docs/04-internal/r11-coordination-context-gap-research-2026-08-28.md (9 项 v2 现状, 11:1 翻译 v1 donor)
   - docs/04-internal/round-13-1-0-maturity-audit-2026-08-28.md (8 个核心 .rs 实测, 修订真实施估时)

4. v2 真账 (本地 working tree, 0 git clone 必要):
   - crates/engine/memory/src/canonical/{vector.rs, graph.rs} (v2 Storage 抽象层, VectorIndex + MemoryGraph 已 1:1 翻译)
   - crates/engine/runtime/src/canonical/{orchestrator.rs, organ_kani_proofs.rs} (A 块 Stage 5 L0-L5 UpgradeCycle + organ_kani_proofs)
   - crates/engine/memory/src/{layered_memo/search.rs, dailynote/search.rs} (v2 BM25-lite 子模块, 不是 storage 主线)
   - crates/foundation/core/src/{eight_anchors.rs:58-79 (9 哲学锚 LOCKED), philosophy.rs:142 (13 键 LOCKED), onion.rs:249 (3 项不可变脊柱 LOCKED)} (LOCKED 5 项, 0 触碰)
   - Cargo.toml:44 ("1.2.0", workspace.version LOCKED, 0 改)
   - legacy/donor/apeireth-asi/tests/integration_r_measure.rs:42-44 (R11_V1141/1131/1136_BASELINE = 0.8682/0.8532/0.9063, R11 baseline LOCKED, 0 触碰)

5. 真账 brief 模板 (Round 15):
   - docs/04-internal/r12-r13-r14-implementation-brief-template-2026-08-28.md (§1 真账 brief 模板 + §3.1 R12-CoordinationContext-1 派单 brief + §4 0 装诚实标)

6. 子代理 brief (本派单项):
   - legacy/donor/apeireth-companion/src/context.rs (~770 行, 1:1 翻译 source)
   - legacy/donor/apeireth-companion/src/context_rot.rs (~526 行, 1:1 翻译 source, 含完整 rot_score 实现)
   - crates/engine/memory/src/canonical/{vector.rs, graph.rs} (v2 真账参考)
```

### 1.3 必输出 (Brief)

```
写真账 to: docs/04-internal/r12-coordination-context-1-implementation-2026-08-28.md (≤ 300 行, 必含 §1.3 7 段)

写真账必含 7 段 (per 真账 brief 模板 §1.3):

### 1. 真实施摘要 (≤ 50 行)
- 1.0 真账实测 (legacy/donor/apeireth-companion/src/{context.rs, context_rot.rs} ~770 + 526 行, REAL maturity, 1:1 可移植)
- 2.0 真账实测 (crates/engine/memory/src/canonical/{vector.rs, graph.rs} + crates/engine/runtime/src/canonical/{orchestrator.rs, organ_kani_proofs.rs}, 0 真实施 rot_score)
- 真实施 7 段: 1.0 真账 1:1 翻译 (RotBlock + Segment) + 2.0 真账对接 (v2 cognitive memory 模块) + 融合策略 + 集成测试 + 0 装诚实 + 下一步

### 2. 5 重守门 baseline 实测 (≤ 30 行)
- cargo test --workspace --locked (期望 1739+N passed / 0 failed, 含新 rot_score 测试)
- cargo clippy --workspace --all-targets --locked -- -D warnings (期望 0 warning)
- cargo check --workspace --locked (期望 0 副作用)
- git diff HEAD -- crates/foundation/core/src/{eight_anchors,philosophy,onion}.rs Cargo.toml:44 crates/foundation/core/src/cognitive.rs (期望 0 行, LOCKED 0 触碰)
- grep -r "legacy/" crates/ | wc -l (期望 < 100)

### 3. LOCKED 5 项 0 触碰 (≤ 30 行)
- 9 哲学锚本体 (eight_anchors.rs:58-79): 0 行
- 13 键 (philosophy.rs:142): 0 行
- 3 项不可变脊柱 (onion.rs:249): 0 行
- workspace.version (Cargo.toml:44): 0 改
- R11 baseline 3 值 (legacy reference): 0 触碰
- 9 哲学锚表头 (eight_anchors.rs enum): 0 减

### 4. 真账对接 + 物种化借签 (≤ 50 行)
- 1:1 翻译 v1 真账 (RotBlock + Segment + RotConfig + RotWeights + RotBreakdown + rot_score + repetition_factor + query_tokens + rot_breakdown)
- 融合策略: context.rs 提供 RotBlock + RotWeights (w_duplicate 0.4 + w_stale 0.3 + w_irrelevant 0.3), context_rot.rs 提供 Segment + 完整 rot_breakdown + rot_score + query_tokens + repetition_factor + stale_half_life_turns → 统一到 context.rs 添加 rot_score + rot_breakdown + repetition_factor + query_tokens 函数 (从 context_rot.rs 借签), context_rot.rs 保留独立 Segment (compaction 原语)
- 命名统一: RotWeights (context.rs) + RotConfig (context_rot.rs, 命名统一到 RotConfig with combined fields w_repetition + w_staleness + w_relevance)
- 物种化借签边界 (per vision.md L47 + Round 13 maturity 补查): RotBlock / Segment 抽象是 v2 真账 cognitive memory 增维路径 (per R11-LongTermMemory 真账 6 项)
- 0 装诚实: 0 装诱导 prevention (1.0 真账 self-flag "0 装 PASS", 不假装 rot_score 准确, 明示启发式 + 待 A/B 调权重)
- 0 引新外部 dep (per 真账 brief 约束, 1:1 翻译优先借签 1.0 真账)

### 5. 真账对接 + 集成测试 (≤ 30 行)
- 真实施代码 (per 1.0 真账 1:1 翻译 + 2.0 真账对接 cognitive memory 模块)
- 集成测试 (cargo test + 真账对接 + species 塑形边界)
- 物种化借签 (per R7 真账 species + R11 真账 coordination-context)

### 6. 主代理决策建议 (≤ 30 行)
- 1.0 真账可移植度: REAL (context.rs + context_rot.rs, 1:1 可移植, 0 LLM, 确定性启发式)
- 2.0 真账对接路径: 走扩展 trait 接口 (cognitive memory 模块增维)
- 真实施 critical path 估时: 3-4 周 critical path (R12 真实施 critical path 最重, per 真账 brief brief)
- 下一步 (跟其他派单对接 / 真账 brief / 真实施主代理亲测)

### 7. 0 装诚实标 (≤ 30 行, 必含)
- 1.0 真账 ~1300 行 2 .rs maturity (context.rs + context_rot.rs, REAL, 1:1 可移植)
- 0 装 PASS 标注 (1.0 真账 self-flag, 启发式待 A/B 调权重, 不假装 rot_score 准确)
- 物种化借签 (per vision.md L47 + Round 13 maturity 补查, per-user memory 塑形)
- 真实施时主代理亲测 (~2-3 天本地实测, 0 git clone 必要, 本地 working tree 已就位)
- 0 引新外部 dep
```

### 1.4 0 装诚实标 (Brief)

```
真实施时主代理亲测 (0 装诚实 doctrine):
- 1.0 .rs 0 实测部分补查 (35 项中 Round 13 亲测 8 项, 余 27 项需真实施时主代理亲测, context.rs + context_rot.rs 是余 27 项中 2 项, 本次融合是余 27 项中 1 项)
- 2.0 真账实测 (16 crates workspace 真账, 本地 working tree 已就位)
- 真实施时主代理必亲测 (~2-3 天本地实测, 0 git clone 必要, per Round 15 用户 catch 修订)

真账 brief 必含 (per O-6 永远追求最优):
- 物种化借签边界 (per vision.md + apeireth-true-understanding-2026-08-28.md)
- 0 装 PASS 标注 (1.0 真账 self-flag "0 装 PASS", 不假装 rot_score 准确)
- 真账 brief 模板 (r12-r13-r14-implementation-brief-template-2026-08-28.md)

真实施时主代理必亲验:
- 真账 brief 模板 (Round 15)
- 真账 brief 必含 §3 5 重守门 baseline 实测
- 真账 brief 必含 §4 LOCKED 5 项 0 触碰 verify
- 真账 brief 必含 §5 真账对接 + 集成测试
- 真账 brief 必含 §7 0 装诚实标
```

### 1.5 5 重守门 + LOCKED 0 触碰 (Brief)

```
真实施时主代理亲测 (必含在写真账 §2 + §3):

5 重守门 baseline 实测:
1. clippy 0 warning:
   - 命令: cargo clippy --workspace --all-targets --locked -- -D warnings
   - 期望: 0 warning, 0 error
   - 当前实测: 0 warning
2. tests 0 fail:
   - 命令: cargo test --workspace --locked
   - 期望: 1739+N passed / 0 failed / 12 ignored
   - 当前实测: 1739 passed
3. legacy compat path < 100:
   - 命令: grep -r "legacy/" crates/ | wc -l
   - 期望: < 100
   - 当前实测: 36
4. LOCKED 5 项 0 触碰:
   - 命令: git diff HEAD -- crates/foundation/core/src/{eight_anchors,philosophy,onion}.rs Cargo.toml:44 crates/foundation/core/src/cognitive.rs
   - 期望: 0 行
   - 当前实测: 0 行
5. 9 哲学锚表头 0 减:
   - 命令: grep "S-[1-3]\|O-[1-6]" crates/foundation/core/src/eight_anchors.rs | wc -l
   - 期望: 9 (S-1, S-2, S-3, O-1, O-2, O-3, O-4, O-5, O-6)
   - 当前实测: 9

LOCKED 5 项 0 触碰 verify:
- 9 哲学锚本体: crates/foundation/core/src/eight_anchors.rs:58-79 (enum)
- 13 键: crates/foundation/core/src/philosophy.rs:142 (RUNTIME_ENFORCED = false)
- 3 项不可变脊柱: crates/foundation/core/src/onion.rs:249
- workspace.version: Cargo.toml:44 ("1.2.0")
- R11 baseline 3 值: legacy/donor/apeireth-asi/tests/integration_r_measure.rs:42-44 (R11_V1141/1131/1136_BASELINE = 0.8682/0.8532/0.9063)

真实施必含 §3 + §4 5 重守门 baseline + LOCKED 0 触碰 verify (走扩展 trait 接口, 不破现有 9 organ trait + 12 cognitive slot wiring + LOCKED 5 项)
```

### 1.6 真实施流程 (Brief)

```
真实施流程 (主代理亲做 spec + 派 sub-agent 真实施分工):

Phase 1: 主代理亲做 spec (~2 周, 立即可做, 不依赖网络, 本地 working tree 已就位)
  - ✅ #4 6 真实施派单 brief 模板 (Round 15 commit `800bdb1a` 已 done)
  - 🔄 #1 v1 context.rs + context_rot.rs rot_score 融合 spec (本真账 brief, 1-2 天)
  - 🔄 #3 cognitive module consolidation_writeback_pipeline + reflection_writeback_pipeline trait spec (1-2 天)
  - 🔄 #5 education 真 CAS spec (1-2 周, 物种化核心)
  - 🔄 #6 confidence BetaBinomial trait spec (1 周, 物种化核心)
  - 🔄 #7 reflexion 3 trait 口实接线 spec (1 周, 物种化核心)
  - 🔄 #8 本地实测 27 项 1.0 .rs maturity 补查 + 2.0 真账实测 (~2-3 天, 不依赖网络, 本地 working tree 已就位)

Phase 2: 派 sub-agent 真实施 (12-14 周 critical path, 不依赖网络, 本地 working tree 已就位真账)
  - R12-CoordinationContext-1/2/3 (协调+上下文, 11-13 周最重)
  - R12-SpeciesCore-1/2 (物种化核心, 4 周)
  - R12-LongTermMemory (长期记忆塑形, 5-7 周)
  - R12-Storage (修订后 1-2 周, BM25 hybrid + causal engine 补)
  - R13-SpeciesForm + MetaCognition + ToolsSecurity (后续, 5-8 周 + 5-7 周 + 6-10 周)
  - R20 preference_learning (in-progress, 2-3 周)

Phase 3: release 流程 (Week 18-20, 1-2 周)
  - 5 重守门 baseline 实测 + ROADMAP §7 + MANIFESTO §14 + ROADMAP §12 check
  - git tag v2.0.0 (per 真账 §6 修订)
  - push v2.0.0 tag + release notes + release announcement

真实施 + release 总估时: ~4 月 (~15-18 周 critical path, 主代理亲做 + 真实施 + release 流程)
```

### 1.7 融合策略 (主代理亲做 spec 决策冻结, 本真账 brief)

**融合策略**: 1.0 真账 context.rs + context_rot.rs rot_score **两 file 各实现一遍** (per R11 catch), 主代理亲做 spec 决策冻结:

**真账实测**:
- **context.rs** (L141-451, ~770 行): `RotBlock` + `RotConfig` + `RotWeights` (w_duplicate 0.4 + w_stale 0.3 + w_irrelevant 0.3) + `DuplicatePair` + `RotBreakdown`
- **context_rot.rs** (L1-180, ~526 行): `Segment` + `RotConfig` (w_repetition 0.4 + w_staleness 0.3 + w_relevance 0.3, 含 stale_half_life_turns) + `RotBreakdown` + `repetition_factor` (多行去重比 + char 6-gram 滑窗) + `query_tokens` (ASCII 小写词 + CJK char-bigram) + `rot_breakdown` + `rot_score`

**融合 spec**:
- **方案 A (推荐)**: 在 context.rs 添加 rot_score 实现 (从 context_rot.rs 借签 repetition_factor + query_tokens + rot_breakdown 函数), context_rot.rs 保留独立 Segment (compaction 原语, 不重复)
- **命名统一**: RotWeights (w_repetition + w_staleness + w_relevance, 0.4 / 0.3 / 0.3) + RotConfig (now_ms + stale_threshold_ms + ngram_size + duplicate_threshold + trigger_threshold + weights + latest_user_message + pinned_block_ids + min_chars_per_block) + RotBreakdown (repetition + staleness + irrelevance + score) + rot_score(seg, query, cfg) + rot_breakdown(seg, query, cfg) + repetition_factor(content) + query_tokens(query)
- **方案 B**: 完全删除 context_rot.rs, 全部合并到 context.rs (改 context_rot.rs 的 Segment 借签 context.rs 的 RotBlock, 或重命名为统一 Block)
- **方案 C**: 保留两 file, 但去重 rot_score 函数 (context_rot.rs 借 context.rs 的 RotBlock + RotConfig, 但保留 Segment compaction 原语)

**主代理推荐方案 A**:
- 1.0 真账 context.rs 提供 RotBlock + RotConfig + RotWeights (启发式 3 因子公式 + 重复度 + 权重配置), context_rot.rs 提供 Segment + 完整 rot_score 实现 (repetition_factor + query_tokens + rot_breakdown + rot_score + stale_half_life)
- v2 真账对接 cognitive memory 模块增维路径 (per R11-LongTermMemory 真账 6 项)
- 0 装 PASS 标注: 1.0 真账 self-flag "0 装 PASS", 不假装 rot_score 准确, 明示启发式 + 待 A/B 调权重
- 0 引新外部 dep (per 真账 brief 约束, 1:1 翻译优先借签 1.0 真账)

**真实施派 sub-agent**:
- 派 sub-agent 真实施 (R12-CoordinationContext-1, 3-4 周 critical path, 不依赖网络, 本地 working tree 已就位真账)
- 真实施代码: context.rs 添加 rot_score + rot_breakdown + repetition_factor + query_tokens 函数 (从 context_rot.rs 借签), context_rot.rs 保留独立 Segment (compaction 原语, 不重复)
- 集成测试 (cargo test + 真账对接 + species 塑形边界)

---

## 2. 真实施 brief (per Round 15 真账 brief 模板 §1.3 + §1.4 + §1.5 + §1.6)

### 2.1 主代理亲做 spec (本真账 brief)

- ✅ 7 段写真账必含 (per 真账 brief 模板 §1.3)
- ✅ 0 装诚实标必含 (per 真账 brief 模板 §1.4)
- ✅ 5 重守门 + LOCKED 0 触碰 (per 真账 brief 模板 §1.5)
- ✅ 真实施流程 (per 真账 brief 模板 §1.6, Phase 1-3)
- ✅ 融合策略 spec (主代理亲做 #1 spec 决策冻结, 见 §1.7)

### 2.2 派 sub-agent 真实施 (R12-CoordinationContext-1, 3-4 周 critical path)

- 派单 brief 必含 7 段 (per 真账 brief 模板 §1.3)
- 真实施代码: context.rs 添加 rot_score 实现 + context_rot.rs 保留独立 Segment (不重复)
- 集成测试: cargo test + 真账对接 + species 塑形边界
- 5 重守门 baseline + LOCKED 0 触碰 verify
- 主代理亲验 commit + push

---

## 3. 0 装诚实标 (per O-5 + 9 哲学锚 + S-2 实事求是)

### 3.1 真账 brief 必含 (per 真账 brief 模板)

- ✅ 真账 brief 必含 5 重守门 baseline 实测 (per §1.5)
- ✅ 真账 brief 必含 LOCKED 5 项 0 触碰 verify (per §1.5)
- ✅ 真账 brief 必含 0 装诚实标 (per §1.4)
- ✅ 真账 brief 必含 真实施流程 (per §1.6)
- ✅ 真账 brief 必含 物种化借签边界 (per §1.1)
- ✅ 0 引新外部 dep (per §1.7 融合策略)

### 3.2 真实施时主代理必亲测 (per Round 15 用户 catch 修订)

- ✅ 真实施时主代理必亲测 (~2-3 天本地实测, 不依赖网络, 本地 working tree 已就位, 0 git clone 必要)
- ✅ 1.0 真账 maturity 补查 (Round 13 主代理亲测 8 个核心 .rs, 余 27 项需真实施时主代理亲测, context.rs + context_rot.rs 是余 27 项中 2 项, 本次融合是余 27 项中 1 项)
- ✅ 2.0 真账实测 (本地 working tree, 16 crates workspace)

---

## 4. 留 backlog

### 4.1 主代理亲做 (10 项 spec, ~2 周, 立即可做)

per Round 14 真实施完成计划 §2.1 表 1-10 + 本真账 brief #1:
- ✅ #4 6 真实施派单 brief 模板 (Round 15 commit `800bdb1a` 已 done)
- 🔄 #1 v1 context.rs + context_rot.rs rot_score 融合 spec (本真账 brief, 1-2 天)
- 🔄 #3 cognitive module spec (1-2 天)
- 🔄 #5 education 真 CAS spec (1-2 周, 物种化核心)
- 🔄 #6 confidence BetaBinomial trait spec (1 周, 物种化核心)
- 🔄 #7 reflexion 3 trait 口实接线 spec (1 周, 物种化核心)
- 🔄 #8 本地实测 27 项 1.0 .rs maturity 补查 + 2.0 真账实测 (~2-3 天, 本地 working tree)

### 4.2 派 sub-agent 真实施 (11 项, ~12-14 周 critical path)

per Round 14 真实施完成计划 §2.2 表 1-11 (派 sub-agent 真实施 11 项, 12-14 周 critical path, 不依赖网络, 本地 working tree 已就位真账).

### 4.3 release 流程 (Week 18-20, 1-2 周)

per Round 14 真实施完成计划 §2.3 (release 流程 5 重守门 + ROADMAP §7 + MANIFESTO §14 + git tag v2.0.0 + release announcement).

---

## 5. 真账 brief 结束语

per 9 哲学锚 + O-6 永远追求最优 + S-2 实事求是 + 文档工程规范 + 整体系统架构最优:

**R12-CoordinationContext-1 派单 brief 真账 (本文件) = 主代理亲做 #1 spec 决策冻结 + 派 sub-agent 真实施 v1 context.rs + context_rot.rs rot_score 融合 (1.0 真账 1:1 翻译 + 2.0 真账对接 cognitive memory 模块 + 融合策略方案 A + 物种化借签 + 0 装诚实标 + 5 重守门 baseline + LOCKED 0 触碰 verify + 真实施流程 Phase 1-3).**

**总估时**: 真实施 3-4 周 critical path + 真账 brief 写真账 ~1-2 天 + 主代理亲做 spec ~1-2 天 = R12-CoordinationContext-1 总估时 ~4-5 周 critical path.

---

_Mavis 写于 2026-08-28 Round 15, 用户原话 "立刻 push, 立刻做点小的, 边做边更新文档, 注意文档工程规范, 哲学锚, 追求整体系统架构最优" 触发, 写真账 R12-CoordinationContext-1 真实施 brief (v1 context.rs + context_rot.rs rot_score 融合 spec 决策冻结, ~300 行, 必含 7 段 per 真账 brief 模板 §1.3 + 0 装诚实标 §1.4 + 5 重守门 §1.5 + 真实施流程 §1.6 + 融合策略 §1.7). 0 装诚实标: 真实施时主代理亲测 ~2-3 天本地实测 (0 git clone 必要, per Round 15 用户 catch 修订)._
