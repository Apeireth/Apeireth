# R6-RES-07｜MemoryReplay 预研（R7-BE-02）

## 调研（24 轮 + 3 真读）
V1052 `WAL.replay()` JSONL+sha256 幂等底座 · MemoryOS `manager.rs:745-776` 三层 `retrieve_context` → scope tag · Letta `agents.py:1510-1551` query+tags+temporal+top_k · VCP `diarySyntaxParser.ts:5,33` replay mode 来源 · Mem0 `prompts.py:176-185` ADD/UPDATE/DELETE/NONE

## 动机
MemoryReplay = 事件触发的记忆调取/重放（≠ dream 周期整理，≠ search 找记录），重现经历供决策。R7-BE-02：可幂等、可审计、可冻结 schema；不写 LTM；不假装理解。

## 5 方法契约（`MemoryReplayProtocol`）
- `capture_state(scope)→StateID`：状态快照（非历史备份）
- `restore_state(state_id)→bool`：仅返成功与否，不自动恢复
- `replay_events(from_ts,to_ts)→Iterator[Event]`：半开 `(from_ts, to_ts]`；只读；单调 ts
- `diff_states(a,b)→StateDiff`：对称 up to sign；空 ≠ byte-exact
- `idempotent_apply(event)→ApplyResult`：仅白名单幂等；外显 rejected

dataclass: `StateID` / `Event` / `StateDiff` / `ApplyResult`

## 幂等原理
1. 白名单 `IDEMPOTENT_OPS = {tag_set, anchor_link, anchor_unlink, score_record, phase_emit, trace_record}`：写 LTM / 改身份 / 删主记忆不在内，调用返 `rejected`+`reason`，不抛。
2. 去重：`event_id` dedup；首次 `applied`，二次 `cached=True`，`event_hash` 不变。
3. 不可推广：守门 `IDEMPOTENT_NOT_SAFE` 禁扩到所有 op。

## R7-BE-02 路径（4 步）
1. 冻结 schema + HQB 双签；
2. 挂 V1052 WAL；
3. scope {stm,mtm,ltm,trace}；
4. 锚 V1072 identity_id + 拒白名单外。

## 协同
- **V1072**：replay 前后校验 5 项 identity guard；任一 False 即拒，记 violation 到 V0.2 永恒身份（不直接升 ASI 分）。`identity_impact_score ≥ 0.7` 触发双签。
- **R6-RES-06 Dream**：dream=周期整理调 replay 端口；replay=事件触发被 dream 用。共享 MTM/LTM 但 lock 分层互斥（dream 写、replay 读）。
- **V1074/V1081**：replay 仅影响 V0.2 identity；trace 写到 `artifacts/asi_snapshot.json` evidence；replay 是 heuristic 重放非真回忆/理解，守门 `REPLAY_NOT_UNDERSTANDING` 强制声明。

## 风险
- 一致性/漂移 → 单调时钟+半开+`content_hash` 记事件数+scope+trace 记时钟源+skew
- 状态污染 → V1072 5 项+impact≥0.7 双签+同 canonical_hash 每会话 ≤3/min+`ltm_protected`/`identity_anchor` 跳过
- 写路径滥用/身份副产物 → 白名单+reject 结构化；非白名单不抛但永不写；`apply` 失败前不写 identity

## 产出
`memory_replay_design.py`（140 ≤150）+ `tests/test_r6_memory_replay_design.py`（99 ≤100，6 项契约测试全过）；6 passed in 0.60s；不动 V1072/V1086/R6-RES-06；不接 call_llm；不破 V1074/V1081。