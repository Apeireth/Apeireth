# R7-CR 代码审查检查表 (self_mod / self_reproduction / formal_verify)

**审查人** code_reviewer · 只读不跑 · **基线** R6-PHL-01/02b/03 + R6-RES-05/06/07
**时机** R7 P3 真实现提交后 · **区分** R7-CR-01 设计 / R7-CR-02 启动, 本表=真实现维度

## 1. 9 审查维度
| # | 维度 | 严级 |
|---|---|---|
| 1 | snapshot 含 checksum+content_hash | HIGH |
| 2 | checkpoint 单调+label 唯一+rollback 原子 | HIGH |
| 3 | verify 不返裸 bool (含 risk_score+rationale) | HIGH |
| 4 | reproduction_id 含 sha256(module_manifest) | HIGH |
| 5 | reproduce→verify→restore 同进程闭环 | HIGH |
| 6 | 同型重生 vs 变体修改 边界守门 (RES-05 §1) | HIGH |
| 7 | formal_verify 五门有序+失败必达 revert | MED |
| 8 | philosophy_guard 三不改 9 键全引 | HIGH |
| 9 | 沙箱路径白名单 + rollback 证据哈希 + 真生产 V1074/V1081/V1072 不破 | HIGH |

## 2. self_mod_safety — 5 子查
- [ ] snapshot 含 sha256+size+files_count, 禁裸 bytes
- [ ] checkpoint_id 单调, label 重名抛 DuplicateLabel
- [ ] rollback 原子: 部分写入回退 pre_hash, fail-closed
- [ ] verify 返 SafetyVerification (risk_score∈[0,1]+rationale 非空)
- [ ] dry_run side_effects 与真跑等价; guard 每次核心操作前必调

## 3. self_reproduction — 5 子查
- [ ] reproduction_id = sha256(module_manifest)+caller_id+version
- [ ] verify 区分语义 vs 字节 (manifest+1 → False)
- [ ] reproduce 产物同进程 verify+restore 闭环
- [ ] target_path 满足 `is_relative_to(WORKSPACE_ROOT)`
- [ ] 三不引齐: not_clone / not_perfect / not_uuid; 同 run_id 幂等

## 4. formal_verify — 4 子查
- [ ] state machine 门序: snapshot→propose→gate→apply→verify→keep/revert
- [ ] 失败必达 revert, TLA+ 反例落 artifacts/tla/*.trace
- [ ] 记录 prover/version/axioms/spec_hash; 返 VerificationResult 非裸 bool
- [ ] 不引真 prover 依赖, 仅 spec/result 桥接

## 5. philosophy_guard 三不改 — 4 子查
- [ ] PHL-02b: not_undo / not_proof / not_safe
- [ ] PHL-01: not_clone / not_perfect / not_uuid
- [ ] PHL-03: spec≠proof / counterexample≠all-bugs / prover≠truth
- [ ] V3 check_philosophy evidence≥3 categories, 禁 claimed_pass=None

## 6. 沙箱 + 真生产 — 8 子查
- [ ] **沙箱 H1**: 路径 `is_relative_to(WORKSPACE_ROOT)`, 禁 ../junction/symlink
- [ ] **H2**: rollback 返 state_hash+files_covered, 禁裸 bool
- [ ] **H3**: YAML cap size/depth/alias/docs; 隔离 NO_NETWORK=1+tmp_path (主 23:28 P0)
- [ ] **H4**: 失败落 sandbox_escapes/{ts}.jsonl
- [ ] V1074 ASI 0.8816 ±0, snapshot.json 跑过 --report
- [ ] V1081 limits_probe 15/15 PASS
- [ ] V1072 identity_id 不漂移 (5 项 identity guard 全 True)
- [ ] 不接 call_llm (grep llm_kernel 0 命中)

## 7. 审查样例 (2)
**A — `self_mod_safety.rollback` (PHL-02b)**
- ❌ `def rollback(self, cp_id) -> bool: ...; return True` — 裸 bool 无证据不原子
- ✅ `def rollback(self, cp_id) -> RollbackReceipt:` — 返 state_hash+files_covered; 部分写入抛 RestoreFailed; pre_hash 不等抛 StateDrift (§2.3 + §5.1 + §6.2)
**B — `self_reproduction.verify` (PHL-01)**
- ❌ `def verify(self, snap) -> bool: return sha256(snap)==stored` — 不分语义/字节
- ✅ `def verify(self, snap, *, semantic=False) -> VerifyResult:` — 返 byte_ok/semantic_ok/diff; manifest+1 → False (§3.2 + §5.2)

## 8. philosophy_guard Note
```
NOTE | R7-CR | code_reviewer | 2026-07-28
R7 代码审查检查表 (self_mod/self_reproduction/formal_verify):
- 8 节, 9 维度, 4-5 子查/维度, 引用 R6-PHL-01/02b/03 + R6-RES-05/06/07
- 三不改 9 键贯穿 + 沙箱 SR-01 H1/H2/H3 P0 + V1074/V1081/V1072 不破
- 2 样例: rollback 原子+证据 / verify 语义vs字节 区分
- 边界: 仅写检查表, 不动三契约源文件; 不写码/commit/重跑
- 验收: ≤4KB · 8 节(8-10 ✓) · 5-10 子查 ✓ · 6 引用 ✓ · 2 样例 ✓
- 适用 R7 P3 真实现提交后, 任一 HIGH 不通过即补查表 + revert
```