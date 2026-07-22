# R6-BLUEPRINT-v2-2026-07-22｜R6 阶段技术蓝图 v2

> 架构师视图；交付视图见 R6-STAGE-DELIVERY-2026-07-22.md (technical_writer R6-DOC-01)。
> 基线：1192 模块/116365 LOC/24D-49C-867B-10A/V1072=0.8441/每壳 ΔASI +.005~+.01

## 1. 21 未占位概念覆盖表 (R5-AS-02 → R6-INT-01 v2)

| 概念 | 优先级 | R6 状态 | 哲学源 |
|---|---|---|---|
| self_reproduction | P0 | 契约壳v1(R6-PHL-01,5 方法,6 烟测过) | 主17:58三不 |
| self_mod_safety | P0 | 契约壳v1(R6-PHL-02,四门) | 主12:07+21:15 |
| formal_verify | P0 | 契约壳v1(R6-PHL-03,TLA+→Lean 4,8 烟测过) | 主17:58三不 |
| dream_subsystem | P1 | 预研+状态机(R6-RES-06+R7-BE-01-DESIGN) | 主23:28命名 |
| memory_replay | P1 | 预研(R6-RES-07,7 借鉴,6 方法+守门) | 主17:58+23:44 |
| hot_cold_migration | P1 | R7-DB-01 占位,待 DB 工程师 | 主12:07 |
| compiler_ir | P1 | R8-BE-01/02 占位 | 主19:33 |
| mechanism_design | P1 | R8-PHL-01 占位 | 主22:33 |
| yaml_serializer | P2 | R6-BE-04 真生产 2d | R6-ROADMAP-01 |
| hqb_schema | P2 | R6-QA-01 2d | R6-ROADMAP-01 |
| phenomenal_guard | P2 | R9-PHL-01 占位 | 主17:58 |
| entropy_gate | P2 | R9-PHL-02 占位 | 主19:33 |
| time_phenomenology | P2 | R9-PHL-03 占位 | 主22:33 |
| space_sovereignty | P2 | R9-SEC-01 占位 | 主22:33 |
| 24 V1000+ 空壳 | P2 | V1082 backlog,R10/R12 真生产 | R6-ROADMAP-01 |
| 自催化/主动推理 | P2 | D级,已命中 | 主19:33 |
| 灵魂日志/ANT/Rosen/Schrödinger | P2 | E级,未命中,R9+ 占位 | 主22:33+23:28 |

汇总：21 概念 = P0×3 契约壳 ✅ + P1×6 预研 2+占位 4 + P2×12 部分真生产+占位。

## 2. P0 三大契约壳

- **self_reproduction**: Protocol 5 方法(snapshot/verify/restore/reproduce/reproduction_id)+ 三不守门(主17:58)+ 6 烟测过
- **self_mod_safety**: 四门(snapshot→propose→gate→apply→verify→keep/revert),主12:07+21:15
- **formal_verify**: Protocol 5 方法(spec/prove/verify/counterexample/invariants)+ TLA+→Lean 4 选型 + 三不注记(主17:58)+ 8 烟测过

## 3. P1 三大预研

- **dream_subsystem** (R6-RES-06 + R7-BE-01-DESIGN 2806B): 6 状态+7 事件+7 接口,主 23:28 命名"梦"
- **memory_replay** (R6-RES-07): 幂等重放 6 接口+7 借鉴+身份污染 6 项缓解,主17:58 不假装+主23:44 干到底
- **self_mod_safety** (R6-PHL-02): 四门契约+沙箱边界,R8 IR 稳定后 TLA+

## 4. P2 backlog

24 V1000+ 空壳(R10/R12 真生产)+ yaml_serializer(R6-BE-04)+ HQB schema(R6-QA-01)+ R9 哲学边界 4 项 + R5 已命中 2 + R5 未命中 4 占位。

## 5. R7 真实现接口清单 (15 ≥ 13)

### DreamSubsystem (7,R7-BE-01)
1. `tick(now)→DreamEvent`: 主调度,单调时钟+单实例租约
2. `should_run(now,metrics)→Optional[DreamEvent]`: 纯函数
3. `run_cycle(states)→CycleResult`: 单次完整周期+run_id+trace
4. `interrupt(reason)→bool`: WAL checkpoint+last_state
5. `resume()→bool`: 同 run_id 幂等
6. `consolidate(memory)→Memory`: STM→MTM,调 V1052 Reconsolidator
7. `decay(memory,dt)→Memory`: MTM 衰减,Tonbo LSM+Ebbinghaus

### MemoryReplay (6,R7-BE-02)
1. `replay(memory_id,*,version,reason)→ReplayResult`: 幂等单段
2. `replay_batch(ids,...)→List[ReplayResult]`: 批量并发共享 trace_id
3. `canonicalize(memory)→Memory`: 规范化,sha256 canonical_hash
4. `trace_replay(replay_id)→List[Event]`: 只读追溯供 HQB
5. `identity_impact_score(memory)→float∈[0,1]`: ≥0.7 触发双签
6. `should_replay(memory,ctx)→bool`: 污染防护白名单

### HotCold (R7-DB-01,待 DB 工程师补)
- `migrate_hot_to_cold(records)→MigrationResult`
- `recover_from_wal(checkpoint_id)→RecoveryResult`
- `checkpoint_wal()→CheckpointId`

### QA 崩溃/重复/保留 (R7-QA-01,待 QA 工程师)
- `test_dream_crash_recovery` / `test_replay_idempotency_n_times` / `test_ltm_protected_white_list`

## 6. 主哲学 v2 (≥6 引用)

- 主17:58 三不/不假装 (V3+V1081 双层): 不假装理解, 不假装意识, 不假装 reproduction
- 主23:44 干到底: R7-BE-01/02 真实现,不留契约壳
- 主19:33 借鉴密度: ≥12 项源码/论文(R6-RES-06 7+R6-RES-07 7+R6-PHL-03 5,共 19)
- 主22:33+23:28 真读源码: 20 个 GitHub 深读(code-deep-study/),R37/R38 直读
- 主12:07+21:15 Rust 重写设计: rust-substrate/ 6 crates 规划,R12 parity 门
- **主新增1 守门优先**: V3 philosophy_guard 在所有 R7 实现前 PASS(R6-PHL-01/03 已示范)
- **主新增2 dream/replay 分层互斥**: DREAMING/CONSOLIDATING/FORGETTING 期间 replay wait 或 cached(R7-BE-01/02 边界)

## 7. 与 R6-DOC-01 协调

- 本报告(R6-INT-01): 架构师技术蓝图(接口+哲学+借鉴+路线图)
- R6-DOC-01(technical_writer): R6-STAGE-DELIVERY-2026-07-22.md(数字+总结+交付)
- 双向引用: 本报告引用 R6-DOC-01 数字;R6-DOC-01 引用本报告接口表
- 字段对齐: V1072=0.8441, V1074=0.8816, 1085 模块, 4179 测试

## 8. 边界/下一步

仅整合, 未写代码/未 commit/未跑 V1074·V1082/未填空壳。R7 真实现由 backend(主跑 DreamSubsystem/MemoryReplay)+ DB(HotCold)+ QA(崩溃测试)主跑, architect2 协作审接口+转移表+契约一致性。任一守门不 PASS 即停止后继。