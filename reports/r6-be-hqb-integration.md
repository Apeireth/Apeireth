# R6-BE-05 HQB 真生产集成验收

## 产出

| 文件 | LOC | 作用 |
|---|---:|---|
| `apeireth/hqb_integration.py` | 96 | V1074/V1082/V1083 adapter + read-only verifier |
| `tests/test_r6_hqb_integration.py` | 66 | 8 集成烟测 |

adapter 组合 V1085 `HonestDecisionModule`、V1086 `HQBPersistence`、DB `HqbStore`；不改目标 runner/V1085/V1086，不接 `call_llm`。HQB 是 score gate，不是 AI/ASI。

## 三个真生产接入点

| 来源 | 输入 | 行为/记录 |
|---|---|---|
| V1074 | `asi_v03` | verdict + score 写 `hqb_decisions`，guard + ASI delta；同步 V1086 JSONL |
| V1082 | `audit_quality` | 审计质量门：≥.95 ACCEPT，≥.40 REVIEW，<.40 REJECT；记录同上 |
| V1083 | `decision_quality` | 路由决策质量按 HQB 阈值记录 verdict + score |

`HQBReadOnlyVerifier.get/list` 只暴露 SQLite 读 API，不提供写入口。

## 阈值与守门

通用 V1085 阈值：`<0.40 REJECT`，`0.40–<0.70 REVIEW`，`≥0.70 ACCEPT`，`≥0.95 VETO`（防止把完美分数假装为 ASI）。V1082 的 `.95 ACCEPT` 是 audit-quality 语义，明确不触发 ASI 完美声明 VETO。

`guard_hqb_integration()` 调用 `philosophy.check_philosophy`，使用 `bounded_gate/no_asi_claim/philosophy_referenced`，PASS、0 deviations。

## 烟测与未污染证据

8/8 PASS：V1074(0.85)→`task_id=v1074` ACCEPT；V1082 audit(0.96)→ACCEPT；V1083 route(0.55)→REVIEW；V1074/V1082/V1083 源文件前后字节一致；guard PASS；adapter 与 V1085/V1086 类型独立。

`pytest tests/test_r6_hqb_integration.py -q`：**8 passed**。

## R7 HQB 路线

真实循环携带 measurement/decision ID、代码/快照 hash、四维分项；依次写 `hqb_decisions`、`hqb_guard_events`、`hqb_asi_deltas`，由只读 verifier 回放。ASI delta 是归因 inventory，不是 ASI 分数；补断电/重复写、并发、回滚、fail-closed 测试。

## 关联

V1074/V1082/V1083、V1085/V1086、`hqb/schema.py`；未改 V1081/philosophy.py。
