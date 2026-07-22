# R6-RES-05 self_mod_safety 预研

**backend · 2026-07-22 · 只调研, 不写代码**

## 1. vs self_reproduction

| 维度 | self_reproduction (R6-PHL-01) | self_mod_safety (R6-PHL-02) |
|---|---|---|
| 拓扑 | 同型重生 (copy of copy) | 变体修改 (variant≠parent) |
| 失败后果 | 系统停→新实例缺失 | 变异→违反不变量 |
| 核心方法 | clone/lineage/spawn | snapshot/checkpoint/rollback/verify/dry_run |

哲学锚: 主 17:58 / 22:33 / 23:28. self_mod_safety ≠ self_reproduction: 前者变体安全, 后者复制安全, 不可作子任务.

## 2. 真实借鉴 (8 个)

| # | 借鉴源 | 学到 | 落地 |
|---|---|---|---|
| 1 | `dgm/DGM_outer.py` (Sakana Darwin Gödel) | archive=[]+parent_commit lineage+update_archive (L174)+prevrun_dir (L15) | V1102 |
| 2 | `letta/agents/letta_agent_v3.py:758` (_checkpoint_messages) | "run this only when messages are 'safe'" — commit-only-after-verify | V1101 |
| 3 | `letta/services/summarizer/compact.py:135` (trigger_threshold) | "verify context stays below this after compaction" — post-condition | V1103 |
| 4 | `anthropic-cookbook/.../sre_mcp_server.py:1431` | kubectl rollout undo + decision_tree "Rollback the deployment" (L1420) | V1101 |
| 5 | `openai-cookbook/articles/gpt-oss-safeguard-guide.md` | "bring-your-own-policy" T&S classifier (策略外置) | V1105 |
| 6 | `AgentMemory-master/.../web.py:123` | try: l3.upsert() except: store.delete(mem_id) + 注释 "P1-1 fix: rollback L4" | V1104 |
| 7 | `VCPToolBox-main/.../es-*.js` (themes) | snapshot: {...l, colorOverrides:{...l.colorOverrides}} — deep-clone | V1102 |
| 8 | `VCPChat-main/.../desktopMetrics.js:13-16` | capabilityKeys: ['cpu','memory','disk','network'] — object-capability | V1105 |


## 3. 5 契约方法草案 (R6-PHL-02 落地)

- snapshot(target) → SnapshotRef — deep-clone (借鉴 7)
- checkpoint(snap, *, verify_ok: bool) → CheckpointId — 仅 verify 通过才持久化 (借鉴 2)
- rollback(to) → RollbackReceipt — last-known-good (借鉴 4)
- verify(candidate, *, invariants) → VerifyReport — post-condition (借鉴 3, 5)
- dry_run(mutation) → DryRunReport — 模拟不提交 (kubectl --dry-run, 借鉴 4)

verify 必查: V3 philosophy_guard · V1074 ASI score 不降 · V1081 诚实边界 · HQB 4 维 (SC/NR/EV/CDT) 不降.

## 4. V 模块 (R6-PHL-02 承载)

- V1101_self_mod_safety — 5 契约主类 + ASIBridge
- V1102_checkpoint_store — archive-list 持久化 (借鉴 1, 7)
- V1103_invariant_checker — verify 引擎 post-condition (借鉴 3, 5)
- V1104_dry_run_engine — mutation 模拟 (借鉴 4)
- V1105_capability_policy — object-capability policy-as-data (借鉴 5, 8)
- V1106_lineage_tracker — parent_commit 血缘 (借鉴 1)

## 5. 守门 + 下一步

任何自改 → philosophy_guard.passes() → ASI score 不降 → HQB 4 维不降 → commit. 失败一律 rollback.

R6-ROADMAP-01 路线图出后正式接 R6-PHL-02. 不写代码, 不 commit, 不填空壳, 不动 V1074/V1081/philosophy.py. 