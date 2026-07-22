# R3-BE-02 — V1085+V1086 HQB 真生产化骨架 (A 方向)

- 时间: 2026-07-22 21:25 / 执行: 后端工程师 / R2-REQ-01 A 方向

## 1. 文件清单 + LOC

| 文件 | LOC | 限制 | 状态 |
|---|---|---|---|
| `apeireth/v1085_hqb_core.py` | 139 | ≤150 | ✅ |
| `apeireth/v1086_hqb_persistence.py` | 148 | ≤150 | ✅ |
| `tests/test_v1085_hqb_smoke.py` | 111 | — | ✅ |
| `tests/test_v1086_hqb_smoke.py` | 149 | — | ✅ |

位置 `apeireth/` 根 (与 v36/v160 HQB 同位置, 项目无 `src/` 目录)。

## 2. 接口架构

**V1085 HQB core** — `HonestDecisionModule.evaluate(hqb_score, context) → HonestDecision`
- 不重建 HQB 4 维, 复用 `apeireth.v36_hqb_benchmark.HQBScore` (V36 已 247 行真生产)
- 阈值 accept≥0.70 / reject<0.40 / veto≥0.95 (主 17:58 不假装)
- 4 verdict: ACCEPT / REVIEW / REJECT / VETO

**V1086 HQB persistence** — `HQBPersistence` 类
- `record()` → `artifacts/v1086/guard_log.jsonl` (独立目录)
- `read_baseline_asi_v03()` 只读 V1074 snapshot (不写)
- `asi_delta(current) = current - baseline` (主 17:43: delta ≠ ASI)

## 3. 烟测 19/19 全过

| 模块 | 测试数 | 关键覆盖 |
|---|---|---|
| V1085 Thresholds | 2 | 默认合法 + 非法拒绝 |
| V1085 Verdicts | 4 | accept/review/reject + **veto (1.0 触发哲学守门)** |
| V1085 Stats | 3 | 空/计数/to_dict |
| V1086 Baseline | 3 | missing=0 / present=parse / malformed=0 不崩 |
| V1086 Record | 2 | 单条/多条 JSONL 真写盘 |
| V1086 AsiDelta | 3 | +Δ / -Δ / 0 |
| V1086 Isolation | 2 | 不污染 V1074 snapshot (字节级未变) |

关键: `test_evaluate_veto_perfect_score` 输入 score=1.0 → verdict=VETO, reason 含 "philosophy guard"。

pytest 摘要: `19 passed in 0.31s`。

## 4. 与上次回归对比

| 项 | R1 architect | R3-BE-02 | Δ |
|---|---|---|---|
| passed | 4745 | 4764 | **+19** |
| failed | 1 (env) | 1 (env 同) | 0 |
| warnings | 2 | 2 | 0 |
| 时长 | 357.72s | 341.30s | -16s |

**0 新增 regression**。唯一失败 `test_v1058::test_find_api_key_empty` 与 R1 同一根因 (env-dependent, 非本任务引入)。

## 5. 边界遵守 (主 07-19 4 层安全门)

- ❌ 未动 V1074 / V1081 / philosophy_guard / 真生产 artifacts 控制台
- ❌ 未重写 philosophy / v36_hqb / v160_hqb (V36/V160 是真生产, 仅 import 复用)
- ✅ V1086 只读 V1074 snapshot, 写独立 `artifacts/v1086/` 目录
- ✅ V1085 veto 阈值 0.95 = 主 17:58 哲学守门

## 6. 下一轮 V1087+ 推荐

- **V1087 HQB Runner CLI**: `python -m apeireth.v1085_hqb_core --report`, 接 V1074 runner
- **V1088 Change Manifest 集成**: V1085 verdict → HARNESS §3 gate field
- **V1089 跨小模型验证 (HARNESS §2.4)**: evaluate() 跨 Qwen/Hermes/Llama 一致性, 让 ASI `engineering` + `self_improving_core` 从 0 起跳
- **填 v1000_yaml_serializer** (R2 backlog pri=1.000) — 仍是最具体可见增量

---

结论: V1085+V1086 HQB 骨架启动完成, 19/19 烟测过, 0 新增 regression。