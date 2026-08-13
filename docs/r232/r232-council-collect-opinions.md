# R232 — apeireth-council collect_opinions

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R232
> **日期**: 2026-08-13
> **状态**: 1 commit, 6 测试 +6, 0 errors / 0 warnings

---

## 0. 主人指示

"全做全做全补弱" + "继续全做完"

## 1. 设计

apeireth-council::Council::deliberate 走完整路径: 召集 advisors + 按住评估 +
synthesis + sovereignty hook. 但缺 per-advisor 显式 API — 想看每个 advisor 怎么投
(不经 synthesis) 没入口. R232 加 collect_opinions.

### 1.1 collect_opinions

```rust
pub fn collect_opinions(&mut self, query: CouncilQuery) 
    -> Vec<(AdvisorOpinion, f64)> {
    let mut ctx = DeliberationContext::new(query.started_at_ms);
    let mut result = Vec::new();
    for advisor in &self.advisors {
        match advisor.deliberate(&query, &mut ctx) {
            Ok(outcome) => {
                let opinion = outcome.opinion
                    .with_weight(self.weights.for_domain(advisor.domain()));
                result.push((opinion, self.weights.for_domain(advisor.domain())));
            }
            Err(err) => {
                eprintln!("advisor {} error: {}", advisor.id(), err);
            }
        }
    }
    result
}
```

**特性**:
- 返回 `Vec<(AdvisorOpinion, f64)>` — 每个 advisor 一项, 含各自权重
- 错误 advisor 跳过 (eprintln), 不阻断
- **不** 触发 synthesis / hold / sovereignty hook (留给 deliberate)
- 跟 deliberate 走同一 dispatch 路径 (复用 advisor.deliberate)

### 1.2 不触碰

- `deliberate` / `deliberate_persona` 完整路径 0 改
- synthesis / hold 评估逻辑 0 改
- sovereignty hook 调用 0 改

## 2. 测试 (6 cases)

| 测试 | 验证 |
|---|---|
| collect_opinions_empty_council | 无 advisor → 空 Vec |
| collect_opinions_single_advisor | 1 advisor → 1 opinion |
| collect_opinions_7_advisors | ALL 7 AdvisorDomain → 7 opinions, order preserved |
| collect_opinions_includes_weight | Safety=1.00, History=0.55 (默认权重正确) |
| collect_opinions_custom_weights | set_weights + with_domain 覆盖默认 |
| collect_opinions_does_not_synthesize | 全 reject 也不触发 hold |

## 3. 工程指标

- **0 errors** workspace
- **0 warnings**
- **0 触碰** 3 不可变脊柱
- **0 引入** 新外部 dep
- **0 删除** 任何代码
- **workspace.version** 1.2.0 0 改
- **测试**: 0 → 6 (+6, collect_opinions_tests 模块)

## 4. 战区意义

apeireth-council 补 per-advisor visibility, 适用:
- **调试** — 看每个 advisor 怎么投, 不经 synthesis
- **日志** — per-advisor reasoning 完整记录
- **审计** — 谁赞成谁反对 + 各自权重
- **决策追溯** — weight 加权前 vs 后对照
- **A/B 测试** — 同一 query 在不同 advisor 集下收集 opinions 比较

## 5. 下一步候选

- **R233** council streaming deliberation (callback API)
- **R234** consciousness temporal emotion decay per-event
- **R235** tool-codesearch ast-grep in-process
- **R236+** protocol Arrow / DataFusion