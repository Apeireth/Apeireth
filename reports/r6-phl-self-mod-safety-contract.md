# R6-PHL-02｜self_mod_safety 占位契约壳

> orchestrator 代执行 (auto_claim 错配) | 2026-07-27

## 1. 契约壳 `apeireth/self_mod_safety.py` (126 行)

`@runtime_checkable SelfModSafetyProtocol` 5 方法: `snapshot()→bytes` / `checkpoint(label)→str` / `rollback(cp_id)→bool` (状态恢复, 非撤销) / `verify(code)→bool` (heuristic) / `dry_run(mutation)→Dict`. 3 dataclass: `Checkpoint` / `SafetyVerification` / `DryRunResult` (cp_id / mutation_id+risk_score+rationale / expected_impact+side_effects). `guard_self_mod_safety()` → `check_philosophy` PASS (0 deviation).

## 2. vs self_reproduction (variant vs replica)

同型重生 vs 变体修改. 5 方法集合不同 (snapshot/verify 重叠; restore/reproduce vs checkpoint/rollback/dry_run). AST 校验 6 项禁用 import 不接 PHL-01/03.

## 3. 哲学守门

`PHILOSOPHY_NOTES = {not_undo, not_proof, not_safe}` — rollback≠undo / verify≠proof / dry_run≠safe (主 17:58).

## 4. 7 烟测 (81 行, 全过)

test_protocol_exists / test_protocol_methods (5 方法) / test_dataclasses (3 类+不变量) / test_no_real_implementation_yet / test_philosophy_guard_imports (3 哲学键) / test_module_in_apeireth / test_distinct_from_reproduction (6 项禁用 import). `tests/test_r6_self_mod_safety_contract.py` 81 行, **7 passed in 0.40s**.

## 5. R7+ 真实现验收

1. snapshot 含 checksum; 2. checkpoint_id 单调 + label 唯一; 3. rollback 原子 (失败不回半状态); 4. verify 必返 risk_score + rationale (无 raw bool); 5. dry_run side_effects 与真跑相等; 6. guard 每次核心操作前必调, 不通过即终止.

## 6. 关联

V1074 (check_philosophy) / V1081 (不假装) / philosophy.py (唯一上游) / self_reproduction.py + formal_verify.py (无 import 依赖) / V1085 HQB (不接).

## 7. 旁注

任务原文指派 backend_engineer, auto_claim 误分配给 orchestrator. 按规格写占位+测试+报告, **未 commit** (任务明令). 边界未破: PHL-01+PHL-03+V1074 = **92/92 still PASS**. R7+ 建议交接给 backend_engineer.
