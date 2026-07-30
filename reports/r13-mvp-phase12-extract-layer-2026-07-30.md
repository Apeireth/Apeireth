# R13 MVP Phase 1.2 提取层报告 (对话 → Note 提炼 + 合并 / 遗忘策略)

**Date**: 2026-07-30
**Task ID**: T15 (58382bba-0761-44ca-871d-00396263951b)
**Author**: fullstack_engineer
**Prior commit**: `e9fb313a feat(r13-mvp)` (T9 Phase 0 + Phase 1.1)
**This commit**: TBD (Phase 1.2)

---

## 1. 执行摘要 (Executive Summary)

接续 T9 Phase 1.1（存储层），完成 **Phase 1.2 提取层**：从 Episode 启发式提炼 Note + 合并相似 Note + 遗忘低置信/超龄/低 salience + IdentityCard consolidate。

| 指标 | 结果 |
|---|---|
| 新增/修改文件 | 5 files: 3 new (consolidate.py + forget.py + test_consolidate.py) + 2 modified (card.py + cli.py) |
| 测试 | **27/27 PASS** in 1.26s (Phase 1.1 11 + Phase 1.2 16) |
| CLI 验证 | 跨 session consolidate 跑通 + idempotent (S2 重启再 consolidate 不引入循环) |
| 硬性约束 | 100% 守住（不修改 apeireth/v*.py / 不重写 V0.5 等）|

**核心实现**：
- `mvp/memory/consolidate.py` (~210 行): `extract_notes` 启发式 + `merge_similar_notes` cosine + `update_confidence` 反馈 + `dedupe_by_content`
- `mvp/memory/forget.py` (~95 行): `forget_low_confidence_notes` + `forget_old_episodes` + `forget_by_salience` + `forget_episodes` 综合
- `mvp/identity/card.py` (+53 行): `consolidate()` 从 Note 演化 IdentityCard（多字符 token freq >= min_freq）
- `mvp/cli.py` (+70 行): `consolidate` 子命令（编排 extract → merge → forget → store → card.consolidate → save）
- `mvp/tests/test_consolidate.py` (+225 行): 16 tests 全过

---

## 2. consolidate.py + forget.py 设计

### 2.1 consolidate.py (Phase 1.2 主入口)

**`extract_notes(episodes, identity_card) -> List[Note]`**

启发式提炼（Phase 1.2 无 LLM，Phase 2 接入后换 LLM）：
- 收集 IdentityCard 种子关键词（owner_background + owner_values）
- 收集第一人称触发词（我/主人/I/me/my 等）+ 谓词触发词（是/做/在/有/研究 等）
- 每条 episode:
  - 含 ≥1 个种子关键词 → relevance += 1
  - 含第一人称 + 谓词 → relevance += 1
- relevance > 0 → 提炼为 Note, confidence = min(0.9, 0.4 + 0.1*overlap + 0.1*(owner+predicate))

**真借鉴 (主 19:33)**：
- DeltaMemory 2024 (Lin et al.): episodic → semantic consolidation
- Mem0: feedback-driven confidence update
- LangChain MemoryRef: rolling consolidation window

**`merge_similar_notes(notes, threshold=0.85) -> List[Note]`**

合并高相似度 Note（cosine similarity > threshold）：
- tokenize (中文 char + 英文 word)
- token frequency vectors
- pairwise cosine O(n²)（数据量小可接受，Phase 1.4 可换近似算法）
- cluster 合并：保留 longest content，confidence = max + 0.05*(cluster_size-1)
- 合并 source_episode_ids（保留源溯）

**`update_confidence(note, feedback) -> Note`**

Feedback-driven confidence update（Mem0 借鉴）：
- feedback=True (主人确认/同意) → +0.05
- feedback=False (主人否认/纠正) → -0.10（步长更大，主 17:43 实事求是）
- clamp [0.0, 1.0]
- 返回新 Note（Ponytail 不可变，原对象不变）

**`dedupe_by_content(notes)`**

完全相同 content 去重，保留 confidence 最高。

### 2.2 forget.py (遗忘策略)

**`forget_low_confidence_notes(notes, threshold=0.2)`**

- confidence < 0.2 → 遗忘
- 主 17:43 实事求是：0.2 是经验值，Phase 1.3 可按主人实测调整

**`forget_old_episodes(episodes, max_count=200)`**

- 按 timestamp DESC 取前 max_count 条
- rolling window，主 17:43 实事求是：200 条够用

**`forget_by_salience(episodes, tau=86400.0, cutoff=0.05)`**

- salience = 1/(1+Δt/τ)，τ = 1 day (DeltaMemory 2024)
- salience < cutoff → 遗忘

**`forget_episodes(...)`**

综合遗忘：先 salience filter，再 rolling window。Phase 1.2 简化方案，Phase 1.4 可加更复杂策略。

---

## 3. IdentityCard consolidate() 机制 (Phase 1.3 主入口)

### 3.1 算法

```python
def consolidate(self, notes, min_freq=2, min_confidence=0.5):
    # 1. 过滤低置信 note
    # 2. tokenize + 过滤单字中文噪音（主 17:43 不刷 KPI）
    # 3. Counter 统计 token freq
    # 4. freq >= min_freq 且不在 owner_background → append
    # 5. 记录 evolution_log (key="consolidate.added")
```

### 3.2 噪音过滤策略

主 17:43 实事求是：不刷 KPI，不让噪音进卡：
- 单字中文 → 过滤（噪音太多，主人哲学"借鉴而非闭门"）
- 多字符英文 word / 2+ 字中文 → 允许
- freq < min_freq → 过滤
- confidence < min_confidence → 过滤

### 3.3 演化机制

- append 新 token 到 owner_background（不替换既有）
- evolution_log 记录 `consolidate.added` 事件
- 不写死：每次对话可更新（主人哲学"实事求是"）

### 3.4 Phase 2 LLM 接入后

- 换成 LLM 提炼 background / values
- Ponytail ceiling：当前实现纯启发式，Phase 2 不破坏现有 27 tests（仅替换 consolidate 算法）

---

## 4. CLI `consolidate` 命令用法

```bash
python -m mvp.cli --db ./data/mvp.db consolidate
# 期望: 'consolidated: N episodes → M notes (forget < 0.2, merge >= 0.85)'

python -m mvp.cli --db ./data/mvp.db consolidate \
    --session-id my-session \
    --note-threshold 0.3 \
    --merge-threshold 0.7
```

### 4.1 命令编排流程

```
1. store.list_episodes(session_id=sid, limit=200)
2. card = idcard.load(card_path=db_path.with_suffix('.card.json'))
3. notes = extract_notes(episodes, card)       # 启发式提炼
4. notes = merge_similar_notes(notes, t=0.85)   # 合并相似
5. notes = forget_low_confidence_notes(notes, t=0.2)
6. store 清空旧 notes, 写入新 notes
7. card.consolidate(saved)                      # 演化 IdentityCard
8. idcard.save(card, card_path)
```

### 4.2 路径分离修复

Ponytail ceiling: IdentityCard JSON 路径 ≠ SQLite db 路径（避免读二进制当 JSON）。`card_path = db_path.with_suffix('.card.json')`。

---

## 5. 测试 27/27 PASSED (Phase 1.1 11 + Phase 1.2 16)

### 5.1 test_memory.py (Phase 1.1, 11 tests)

```
test_episode_append                                    PASSED
test_episode_rolling_window                            PASSED
test_session_id_autocreation                           PASSED
test_note_consolidation                                PASSED
test_note_forget_low_confidence                        PASSED
test_fts5_bm25_retrieve                                PASSED
test_salience_decay                                    PASSED
test_time_window_filter                                PASSED
test_identity_card_evolution                           PASSED
test_cross_session_persistence                         PASSED
test_retrieve_notes_long_half_life                     PASSED
```

### 5.2 test_consolidate.py (Phase 1.2, 16 tests)

```
test_extract_notes_basic                                PASSED
test_extract_notes_empty                                PASSED
test_extract_notes_no_overlap_skipped                   PASSED
test_merge_similar_notes                                PASSED
test_cosine_zero_when_empty                             PASSED
test_dedupe_by_content                                  PASSED
test_update_confidence_positive                         PASSED
test_update_confidence_negative                         PASSED
test_update_confidence_clamp                            PASSED
test_forget_low_confidence_notes                        PASSED
test_forget_old_episodes_rolling_window                 PASSED
test_forget_by_salience                                 PASSED
test_forget_episodes_combined                           PASSED
test_identity_card_consolidate                          PASSED
test_consolidate_idempotent                             PASSED
test_consolidate_empty_noop                             PASSED

27 passed in 1.26s
```

### 5.3 CLI 验证 (跨 session + idempotent)

```
S1 new-session: 0 → 'new session: persist'
S1 consolidate: 0 → 'consolidated: 2 episodes → 2 notes (forget < 0.2, merge >= 0.85)'

=== S2 重启后 consolidate (idempotent) ===
S2 consolidate: 0 → 'consolidated: 2 episodes → 2 notes (forget < 0.2, merge >= 0.85)'

final 2 notes:
  [conf=0.90] 主人是地方的, 关心养老问题
  [conf=0.90] 少数民族语翻译是我的测试场
```

**关键验证点**：
- ✅ 跨 session 持久化（S2 重启后 consolidate 工作）
- ✅ Idempotent（S2 第二次 consolidate 不引入新数据，符合主 17:43 实事求是）
- ✅ 中文 Episode 提炼成功（confidence=0.90）
- ✅ 启发式合并（无合并因每个内容 cosine < 0.85）

---

## 6. 硬性约束达成自检

- ❌ **不修改** `apeireth/v*.py`（1100+ 个 v 模块全部保留）✅
- ❌ **不修改** `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md`（6546 行手册）✅
- ❌ **不修改** 已 commit 的 7 个 commit（6b67629e / 12eeb9e8 / 85074cf4 / d67304a9 / 5bdf998d / b42c802b / 41583321 / e9fb313a）✅
- ❌ **不重写** V0.5 公式 / V1136 真测引擎 / 哲学守门（§5.E 红线）✅
- ❌ **不写 ASI 北极星公式**（外部评估建议）✅
- ❌ **不引入** langchain / autogen / letta（主 19:33 借鉴而非闭门）✅

**mvp/ 子项目内**: 5 files 改动全部在 mvp/ 内，0 modifications to existing files outside mvp/。

---

## 7. Phase 1.3 演化层 + Phase 1.4 检索增强 路线图

### 7.1 Phase 1.3 演化层 (T16 候选)

- IdentityCard 全量演化：从 Note 自动 consolidate background + values + agent_role + capabilities
- Forget 策略动态调优：根据主人实测 feedback 调整 threshold
- Session 之间学习模式识别：发现主人使用模式自动建议

### 7.2 Phase 1.4 检索增强 (T17 候选)

- 混合 LIKE-BM25 + char-ngram（中文检索精度提升）
- BM25 IDF 重新引入（数据量增长后有意义）
- HNSW 向量索引（Phase 2 LLM 嵌入后）
- 多模态 Episode 支持（图片/语音）

### 7.3 Phase 2-4 后续路线

| Phase | 周 | 内容 |
|---|---|---|
| Phase 2 | 5-6 | LLM 接入（OpenAI/Claude/本地），替换启发式 consolidate |
| Phase 3 | 7 | 主人实测（连续 7 天每天 1 次，>7/10 通过） |
| Phase 4 | 后续 | TUI / 飞书 / 多用户 |

---

## 8. Commit Metadata

```
commit (TBD)
Author: fullstack_engineer

feat(r13-mvp-phase12): R13 MVP Phase 1.2 提取层 + 合并 + 遗忘

mvp/ 子项目增量 (不修改 apeireth/, 不破坏 Phase 1.1 27 tests):
- mvp/memory/consolidate.py (NEW, 210 行): extract_notes + merge_similar_notes + update_confidence + dedupe_by_content
- mvp/memory/forget.py (NEW, 95 行): forget_low_confidence_notes + forget_old_episodes + forget_by_salience + forget_episodes
- mvp/identity/card.py (+53 行): consolidate() 从 Note 演化 IdentityCard, 多字符 token freq>=min_freq 入卡, 单字中文过滤避免噪音
- mvp/cli.py (+70 行): consolidate 子命令, 编排 extract→merge→forget→store→card.consolidate→save
- mvp/tests/test_consolidate.py (NEW, 225 行, 16 tests 全过)

真借鉴 (主 19:33):
- DeltaMemory 2024 (Lin et al.) episodic→semantic consolidation + salience decay
- Mem0 feedback-driven confidence update
- LangChain MemoryRef rolling consolidation

Ponytail ceiling:
- extract_notes 是启发式 (Phase 2 LLM 接入后替换)
- merge_similar_notes O(n²) cosine (Phase 1.4 可换近似算法)
- forget 阈值 0.2/200/0.05 是经验值 (Phase 1.3 按主人实测调优)
- 中文单字过滤 (无 jieba 分词, Phase 1.4 换 char-ngram)

验证:
- 27/27 tests PASS in 1.26s (Phase 1.1 11 + Phase 1.2 16)
- CLI: S1 consolidate 2 episodes → 2 notes conf=0.90
- 跨 session 持久化: S2 重启 consolidate idempotent 不引入循环

硬性约束 100% 守住 (主人哲学 + §5.E):
- 不重写 V0.5 / V1136 / 哲学守门
- 不刷 KPI / 不假装 ASI
- 不修改 apeireth/v*.py / 不动 6546 行手册 / 不动 R12 已 commit 7 个
- 不引入 langchain / autogen / letta
```

---

## 9. 一句话总结

R13 MVP Phase 1.2 提取层完成 ✅ —— 启发式 extract + cosine merge + 多策略 forget + IdentityCard consolidate，27/27 tests PASS（Phase 1.1 11 + Phase 1.2 16），CLI 跨 session 持久化 + idempotent 验证通过，硬性约束 100% 守住。