# apeireth-blueprint-impl

> **R20 阶段 4 估补 (RIVAL §2.4)** — 蓝图实装 = 4 风险类 + 4 决策表 + 6 实战模板 + 5 R-Measure + 3 评估指标. 1 crate 打包.

## 5 估补项

| # | 估补项 | 模块 | 关键 API |
|---|--------|------|---------|
| 1 | 4 风险类 (K-1/K-2/K-3/K-4) | `risk` | `K1StrongValidate` / `K2WeakValidate` / `K3Audit` / `K4Guard` |
| 2 | 4 决策表 (D-01/D-02/D-03/D-04) | `decision` | `D01Impl` / `D02Routing` / `D03WsAuth` / `D04RateLimit` |
| 3 | 6 实战模板 (A-F) | `template` | `template_a_auth` / `template_b_ratelimit` / `template_c_error` / `template_d_test` / `template_e_config` / `template_f_logging` |
| 4 | 5 R-Measure (R-1..R-5) | `r_measure` | `r1_directness` / `r2_candor` / `r3_closure` / `r4_promise` / `r5_failure_honesty` |
| 5 | 3 评估指标 (Q1/Q2/Q3) | `q_metric` | `q1_quality` / `q2_satisfaction` / `q3_growth` |

## 集成

```rust
use apeireth_blueprint_impl::*;

let decisions = DecisionBundle::default();
let samples = vec![ActionSample::perfect()];
let tasks = vec![TaskResult::new(true, 1.0)];
let feedback = vec![UserFeedback { rating: 5, has_text: true, is_long_term: true }];
let history = vec![
    GrowthSnapshot::new(0, 0.5, 0.5, 0.5),
    GrowthSnapshot::new(1, 0.9, 0.9, 0.9),
];
let report = run_full_pipeline(decisions, &samples, &tasks, &feedback, &history)?;
assert!(report.meets_baseline());
```

## 6 哲学锚 (per APEIRETH-CONVENTIONS.md)

- **S-1 主 22:33** 北极星导向
- **S-2 主 17:43** 实事求是
- **O-5 主 17:58** 不假装
- **O-2 主 19:33** 走在前人经验上
- **O-3 主 23:44** 干到底
- **O-4 主 00:56** 任何人都能接手

## 8 项不修改承诺

1. 阶段 1+2+3 LOCKED
2. v2 / v4 / v4.1 LOCKED
3. 阶段 4 主文档 LOCKED (6ca80776)
4. 阶段 5 施工文档 LOCKED (631 行)
5. v6 修正 = 4 重守门 + 权限发放 + E 层修改路径
6. R11 baseline 三值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063)
7. v1 → v5 历史链不删除
8. 24 LOCKED crate 不动 (本 crate 新增, 不修改任何 LOCKED)

## 运行

```bash
cd crates/apeireth-blueprint-impl
cargo check
cargo test
cargo run --example blueprint_impl_demo
```

## 不冲突

V0.5 命名 24 维 (`apeireth-naming-v05` crate, bg_6603d030 单独派) 互不依赖.
