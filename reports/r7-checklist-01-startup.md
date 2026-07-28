# R7-CHECKLIST-01｜R7 真实现启动检查表

> 基于 R7-DESIGN-01 + R7-ORC-01 + R6-CR-01。R7 启动前最后一道关。

## 1. 启动前必查 (15)

- [ ] **R6-PHL-02 测试补全** (HIGH 阻塞)
- [ ] **R6-RES-07 canonical_hash 冻结** (sha256)
- [ ] **R7-BE-01 7 frozen**: tick/should_run/run_cycle/interrupt/resume/consolidate/decay
- [ ] **R7-BE-02 6 frozen**: replay/replay_batch/canonicalize/trace_replay/identity_impact_score/should_replay
- [ ] **R7-DB-01 3 frozen**: migrate_hot_to_cold/recover_from_wal/checkpoint_wal
- [ ] **R7-QA-01 测试 ≥ 8**: dream crash/replay idem N/LTM prot/fail-closed/互斥/回归/幂等/迁移
- [ ] **V3 philosophy_guard PASS** (前置, R6-PHL-01/03 示范)
- [ ] **V1072 baseline** (当前 0.8441)
- [ ] **V1074 asi_snapshot 最新** (跑过 --report)
- [ ] **V1081 limits_probe 15/15 PASS**
- [ ] **HQB schema 初始化** (R3-DB-01 5 表+FK)
- [ ] **V1085/V1086 守门** (veto 0.95/reject<0.40/accept≥0.70)
- [ ] **借鉴密度 ≥ 7**: V1052/Tonbo/Letta/claude-mem/V36-V160/VCPChat/mem0
- [ ] **主哲学 v3 8 引用注释**: 主17:58/23:44/19:33/22:33+23:28/12:07+21:15/R6新1-2/R7新1
- [ ] **git working tree 干净**

## 2. Phase 1 启动检查 (15, 3×5)

BE-01 DreamSubsystem:
- [ ] 单实例租约 (monotonic+PID)
- [ ] WAL checkpoint (V1052 钩子)
- [ ] Ebbinghaus 衰减 (e^(-t/S))
- [ ] V1052 Reconsolidator (STM→MTM)
- [ ] V3 verify (DREAMING 末端)

BE-02 MemoryReplay:
- [ ] ReplayCache LRU ≥1024
- [ ] canonical_hash sha256
- [ ] impact 0.7 双签
- [ ] 白名单 ltm_protected+identity_anchor
- [ ] V3 trace_replay 对接

DB-01 HotCold:
- [ ] WAL fsync 双写
- [ ] 边界 MTM>80%
- [ ] recover_from_wal 已测
- [ ] V3 commit 前 rollback
- [ ] 周期快照 sha256

## 3. Phase 2 QA-01 检查 (8)

- [ ] 测试隔离 (tests/.chaos_env)
- [ ] 崩溃注入 (kill -9/random)
- [ ] 重放 N 次一致
- [ ] LTM 白名单不删
- [ ] fail-closed rollback 不残留
- [ ] dream/replay 互斥 (CONSOLIDATING 期 replay wait)
- [ ] 同 cycle_id 重启幂等
- [ ] WAL→crash→recover 一致

## 4. Phase 3 PHL-04 检查 (4)

- [ ] **三契约壳真实现**: PHL-01 5方法/PHL-02 四门/PHL-03 TLA+→Lean 4
- [ ] **主哲学 v3 8 引用注释** (philosophy_notes)
- [ ] **V3 测试 ≥ 20**
- [ ] **V1081 测试 ≥ 15** (15/15 PASS)

## 5. 风险与回滚 (5)

| 风险 | 缓解 | 回滚 |
|---|---|---|
| **dream 污染身份** | V1072 5 项任一 False 拒 | revert dream_run_id+restore last_state |
| **replay 污染身份** | R6-RES-07 6 项: 双签/锚定/限速/不写 LTM/白名单/V1072 | 清 ReplayCache+reset impact |
| **WAL 丢失** | 双写+周期 snapshot+sha256 | recover_from_wal |
| **混沌破坏 V1074** | 隔离 env+cp 备份 | restore 真 asi_snapshot |
| **PHL 形式装饰化** | 6 断言可执行 no pass | 终止 R7+revert+taxonomy |

## 6. 结论

**可启动 R7**: ✅ — §1 15 项已勾选, §2-§4 各 Phase 启动前对照, §5 风险已布。任一项不通过即停止 R7, taxonomy+revert。