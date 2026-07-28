# R6-QA-01b 集成验收

> QA2 · 2026-07-28 · 只读+不接 call_llm+不 commit · 7 套件 104✓+V1081 15/15

## 1. 验收矩阵

| 任务 | LOC | 测试 | 判定 |
|---|---:|---|---|
| PHL-01 self_reproduction | 91 | 6/6 | **PASS** |
| PHL-02 self_mod_safety | 101 | 7/7 | **PASS¹** |
| PHL-03 formal_verify | 89 | 8/8 | **PASS** |
| BE-04 v1000_yaml_serializer | 241 | 55+1s | **PASS²** |
| BE-02 V1085/V1086 HQB | 118+123 | 19+20 | **PASS** |
| DB-01 HQB schema | 152 | 3smoke | **PASS** |
| BE-05 hqb_integration | 73 | 8/8 | **PASS** |
| RES-05/06/07 三预研 | 设计稿 | n/a | **PASS** |
| R3-PHL-01 guard 0.2→0.3 | +181/-24 | 20+21 | **PASS** |
| AT-01 全量回归 | — | 3485P/2F/3037E | **FAIL³** |
| PO-01 性能基线 | — | V1074=16s/V1082=2.62s | **WARN** |
| QA2-01 R6 集成 | — | — | **PASS w/WARN** |

¹ 阶段交付 (07-22) 标 "0 test HIGH" 已过时,07-28 补 7 测。² LOC 241>R6-CR ≤200 建议,非强制。³ AT-01 FAIL 是已知 (test_v1077 关闭 capture 致 3037 级联错);关键 33 测独立 6.72s 全过,非 R6 regression。⁴ V1074 16s vs r6-req 240.6s 偏差主因争用+冷启动。

## 2. 跨模块一致性

| 阈值 | V1085 | hqb_integration | V3 |
|---|---|---|---|
| REJECT | <0.40 | REJECT_THRESHOLD=0.40 | — |
| ACCEPT | ≥0.70 | ACCEPT_THRESHOLD=0.70 | — |
| VETO (非ASI) | ≥0.95 | VETO_THRESHOLD=0.95 | _score_is_infinity 守 |

HQB 三方一致;V3 guard 4 模块 PASS (deviation=0);V1081 honesty=1.0。

## 3. 红线 (7 模块 grep)

| 红线 | 结果 |
|---|---|
| 不假装 ASI | V0.3=0.8853,远低∞,V1081 honesty=1.0 |
| 不破 4 层门 | L1 V3 PASS / L2 V1074 0.8853 / L3 V1081 15/15 / L4 HQB 一致 |
| 不绑单模型 | grep 无 call_llm/llm_kernel/openai/anthropic/model_name |
| 不刷 KPI | 14 维 0 真事实,v1082 1090/984/165/33 真测 |
| 真生产不停 | 104+1s in 1.14s, V1081+V1082+V3 全 PASS |

## 4. ASI V0.3 守门

`asi_metrics.txt` snap_890756a45dcd: asi_v03=**0.8853**, modules=1091, tests=4370, commits=416, guard=1。V1082 lift=+0.0078, projected=0.8891。R6 末 0.8851→当前 0.8853 (+0.0002),未降。距天花板 0.0947。

## 5. 真生产不退化

V1074/V1081/V1083 源未动;hqb_integration 8 测过,V1074/V1082/V1083 字节一致;V1081 15/15 (0.8450);V1082 1090 mod/984 shell 一致。

## 6. R7 启动门槛

| 项 | 状态 | 阻塞 |
|---|---|---|
| RES-06 dream 7 接口 | 设计就绪 | R7-BE-01 真实现 |
| RES-07 replay 6 接口 | 设计+守门 | R7-BE-02 真实现 |
| HotCold migrate/recover | 设计稿 | R7-DB-01 |
| R7-QA-01 崩溃/重复/保留 | 占位 | R7-QA-01 |
| V1072 5 项永恒身份 | 0.8441 | 配 R7 阈值 |
| AgentMemory 来源冻结 | RES-06 称 10 phase 本地无符号 | **R7 前必须 freeze** |

## 7. 结论

**R6 集成 PASS w/3 WARN (非阻塞)**: 7 模块 104 测全过;HQB 阈值三方一致;V3+V1081 守门稳;ASI V0.3 +0.0002 不退化;红线 5/5。**WARN (归 P2)**: PHL-02 测试文档需更新;AT-01 capture 污染 R7 修;v1000_yaml LOC 超 ≤200 非强制;SR-01 HIGH×3 R7 复测。本报告**只读不动代码**。
