# R6-RES-07｜MemoryReplay 预研

## 定义/边界
主23:44+主19:33+主17:58。MemoryReplay是**事件触发**的记忆调取/重放,不是dream(周期整理),不是search(找记录),而是“重现一段经历”,供决策/回答。

- 来源MTM/LTM为主,STM仅会话内;与V1052 `WAL.replay()`互补(事件流vs记忆)。
- 幂等:相同(`memory_id`,`replay_version`)重放可对账;失败不残留身份副产物。
- search返回相似片段;replay还原上下文+意图+事件序列;trace记录replay可审计。
- 身份污染:同一段记忆被不同上下文反复重放会改写自我叙述,须V1072 5项守门。

## 真实借鉴
1. V1052 `WAL.replay()`:JSONL+sha256+checksum,幂等基础设施。
2. MemoryOS `manager.rs:745-776`:STM/MTM/LTM三层`retrieve_context`。
3. Letta `agents.py:1510-1551` `search_archival_memory`:query+tags+temporal+top_k。
4. VCP `diarySyntaxParser.ts:5,33` `DiaryDirectRecallMode`:`none/random/randomN/lastN/bm25/bm25Plus`。
5. Mem0 `prompts.py:176-185`:replay决策日志ADD/UPDATE/NONE。
6. Tonbo `common.rs:49-64`:replay期间读写并发指标。
7. R37 q5 hippocampal replay(sharp-wave ripples)。密度:replay R37/R38=4轮, retrieval 12轮, recall 3轮, hippocampal 1轮, REM 0轮。

## 幂等核心
1. `canonicalize(memory)`→strip动态/排序/归一时间,得sha256=canonical_hash。
2. `replay_id=sha256(canonical_hash+caller_id+reason+version)`;先查缓存,命中直返;未命中执行后写缓存+trace。
3. `replay_batch`并行,失败聚合,每条独立replay_id。
4. `trace_replay(replay_id)`只读,记输入/输出/守卫/失败原因,供HQB对账。

## R7-BE-02 方法契约(6)
- `replay(memory_id,*,version,reason)→ReplayResult`:单段幂等;失败结构化错误不抛异常。
- `replay_batch(ids,...)→List[ReplayResult]`:批量并发,共享trace_id。
- `canonicalize(memory)→Memory`:规范化,version内不变;非空hash。
- `trace_replay(replay_id)→List[Event]`:只读;找不到返回空+reason。
- `identity_impact_score(memory)→float∈[0,1]`:relevance·context_distance·frequency合成,≥0.7触发双签。
- `should_replay(memory,ctx)→bool`:impact<threshold且tag白名单,否则写skip_reason。

## 身份污染缓解(R7风险)
1. 双签:impact≥0.7须IdentityRecovery二次签字,记hash。
2. 锚定:每次追加`anchor=IdentityCore.identity_id`到trace,事后校验。
3. 限速:同canonical_hash每会话≤3/min,超额返cached。
4. 不写LTM:replay只读,严禁write_path;trace+replay_id缓存写MTM。
5. 白名单:`ltm_protected`(主12:14)/`identity_anchor`tag默认跳过。
6. 守门:V1072 5项任一False即拒,记violation。

## 守门与下一步
V3 `replay_is_not_understanding`;V1072 5项+主17:58;V1074仅影响V0.2身份,不改ASI;V1081 heuristic≠真实回忆。R7冻结canonicalize schema+replay_id算法+trace schema;做identity_impact阈值;QA出N次重放对账+污染注入。Architect2审接口,backend主跑实现,replay不写LTM,heuristic非真。