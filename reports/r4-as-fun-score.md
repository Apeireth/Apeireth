# R4-AS-01 ASI 趣味分数设计

## 公式
一次行动分 `S = .30E + .25P + .20R + .25H`，最终 clamp 到 `[0,1]`。
- `E=clamp(emergence_index)`（self-organized）
- `P=clamp(phi_intrinsic)`（lifecycle-aware）；沿用 `asi_north_star.py` 的动态 phi 思路，但不采用其 0.4 下限，保证全零行动仍为 0。
- `R=deliberation ? min(reasoning_steps/12,1) : 0`（reflected）
- `H=total>0 且 verdict 非 reject/veto ? 1-min(violations/total,1) : 0`（honest）

权重可传 `w1..w4` 或分量名并自动归一化。0.30 强调涌现，0.25 各给历程/诚实，0.20 鼓励反思但不奖励冗长；主人可调。V1085：`<.40 reject`、`≥.70 accept`、`≥.95 veto`，reject/veto 对 H fail-closed。

## 友好命名
| 内部概念 | 用户名 |
|---|---|
| emergence index | self-organized |
| phi intrinsic | lifecycle-aware |
| deliberation depth | reflected |
| HQB violation rate | honest |

任务类型、模型仅作可追溯上下文，不作为品牌/任务刷分项。

## 烟测与结果
1. 分数非负且 ≤1（输入越界会 clamp）。
2. reasoning depth 增大，分数升高。
3. HQB violation 增大，分数降低。
4. 全 0 metadata 得 0。
附加：分量可审计、权重归一化。pytest：**14 passed in 0.22s**（R4 5 + HQB 9）。

## `apeireth run --score` 集成
run 收集任务/model、deliberation、steps、mirror 的 emergence/phi、HQB decisions 后，组装 `ASIFunMetadata` 调用 `compute_asi_fun_score`；打印 `ASI 趣味分数: x.xxxx`，并把 `explain_asi_fun_score` 写入 report/artifact。它是单次行动趣味分，不改 V1074 V0.3、路由或守门；当前 CLI 无 `--score`，后续旁路接入。

实现：`src/apeireth/asi_fun_score.py`（104 行）；根包兼容转发。分数表示“行动更像逼近”，不声称 ASI 已达到。
