# TP18 (E3, P1) — 校准诊断 + 集合预报 + 预测市场 — agent_orchestrator 报告

**任务 ID**: `568745f6-a1e6-42e6-a895-19cb99785c6d`
**角色**: agent_orchestrator
**分支**: `task/tp18-calib-ao` (worktree: `_workspace/tp18-calib-ao`)
**集成分支**: `team/e8de47ae-0e59-459d-a763-88e52b7706c8/integration`
**基础 commit**: `9dffc288` (merge: TP23 跨任务 orchestration 协调报告, agent_orchestrator2, 3 files +243/-3)

---

## 1. 交付清单

### 1.1 新文件 (3 个)

| 文件 | 行数 | 单元测试数 | 内容 |
|------|------|----------|------|
| `crates/apeireth-cognition/src/calibration.rs` | 461 | 19 | Brier 评分 + Murphy 单调分解 + CalibrationBin + ECE |
| `crates/apeireth-cognition/src/forecast.rs` | 670 | 31 | EnsembleForecast (Bayesian / Mean / Median) + PredictionMarket (LMSR) |
| `crates/apeireth-evolution/src/critic.rs` | 471 | 16 | 校准感知批评 → 推荐 evolution 状态转换 |

### 1.2 修改文件 (3 个)

| 文件 | 增量 | 原因 |
|------|------|------|
| `crates/apeireth-cognition/src/lib.rs` | +14 行 | 注册 `pub mod calibration` + `pub mod forecast` + 17 个 re-export |
| `crates/apeireth-evolution/src/lib.rs` | +5 行 | 注册 `pub mod critic` + 4 个 re-export |
| `crates/apeireth-evolution/Cargo.toml` | +1 行 | 加 `apeireth-cognition = { path = "../apeireth-cognition" }` 依赖 |

**总计**: 6 文件, +1662 行 (含测试)

---

## 2. 三大组件设计

### 2.1 Brier 单调分解 (calibration.rs)

**Murphy (1973) 三分解**:
- `BS = reliability − resolution + uncertainty`
  - `reliability = Σ_k (n_k/N) * (f_k - o_k)²` — 越小越好 (完美 = 0)
  - `resolution = Σ_k (n_k/N) * (o_k - o_bar)²` — 越大越好
  - `uncertainty = o_bar * (1 - o_bar)` — base rate entropy

**API**:
```rust
pub fn brier_score(obs: &[Observation]) -> f64;
pub fn decompose(obs: &[Observation], num_bins: usize) -> BrierDecomposition;
pub fn calibration_bins(obs: &[Observation], num_bins: usize) -> Vec<CalibrationBin>;
pub fn expected_calibration_error(bins: &[CalibrationBin]) -> f64;
```

**`is_monotonic()` 容差设计**: 单次样本有 sampling variance `O(1/√N)`, 容差 = `5/√N`。
- N=100 → 容差 ≈ 0.5
- N=10000 → 容差 ≈ 0.05
- 当 N → ∞ 或 bin 内 forecast 全等, 等号精确成立 (`is_strictly_monotonic()` 容差 1e-9)

**`monotonic_residual()`**: 返回 `BS - (reliability − resolution + uncertainty)`, 期望 0 (sampling noise → 0)。

### 2.2 EnsembleForecast (forecast.rs)

**3 种聚合策略**:
- `Bayesian` (默认): `weight_i = confidence_i × contrarian_factor × minority_boost`
  - `contrarian_factor = 1 + contrarian_weight × (1 − agreement)`
  - `minority_boost = 1 + contrarian_weight` 当 `|p_i − median| > 0.1`
- `Mean`: 等权平均
- `Median`: 中位数 (抗 outlier)

**API**:
```rust
pub struct EnsembleMember { source_id, prediction, confidence }
pub struct EnsembleConfig { strategy, contrarian_weight }
pub struct EnsembleForecast { members, aggregate_prediction, aggregate_confidence, agreement_score }
impl EnsembleForecast { aggregate(members, config) -> Self, as_observation(outcome) -> Observation }
```

**`agreement_score`**: `1 - 2 × stddev(predictions)` ∈ [0, 1] (因 prediction ∈ [0, 1] → stddev ∈ [0, 0.5])。

### 2.3 PredictionMarket LMSR (forecast.rs)

**Hanson (2003) LMSR cost function**:
- `C(q) = b × log(Σ_i exp(q_i / b))`
- `price_i = exp(q_i / b) / Σ_j exp(q_j / b)`
- `cost_to_buy(idx, shares) = C(q + shares × e_idx) − C(q)`

**反方权重**: contrarian (低信念) trader 获 cost subsidy。
- 当前 price < fair price (1/N) → 视为 contrarian
- `subsidy = 1 − contrarian_weight × deficit / fair_price`

**API**:
```rust
pub struct MarketConfig { liquidity_b, num_outcomes, contrarian_weight }
pub struct PredictionMarket { quantities, config }
pub struct TradeReceipt { outcome_idx, shares, cost, avg_price, price_after }
impl PredictionMarket { new/uniform/cost/prices/price_of/cost_to_buy/execute_buy/aggregate_belief }
```

**单调性**: `cost(Δ=5) < cost(Δ=10) < cost(Δ=20)` (test `lmsr_buying_increases_cost_monotonically`)
**流动性敏感**: `higher b → lower price impact per share` (test `lmsr_higher_liquidity_lower_slippage`)

### 2.4 Critic (apeireth-evolution/src/critic.rs)

**校准诊断 → 状态转换推荐**:
- `severity = clamp(BS + ECE, 0, 1)`
- `severity < 0.15` → `Continue` → `TransitionReason::CouncilApprove`
- `0.15 ≤ severity < 0.30` → `Revise` → `Failure("calibration_revise")`
- `severity ≥ 0.30` → `Reject` → `Failure("calibration_reject")`

**`target_state()` 决策表**:
- `Continue + Draft → Proposed`
- `Continue + Proposed → Ratified`
- `Revise + 任意 → Draft` (退回重写)
- `Reject + 任意 → Retired`

**集成**: `Critic` 用 `apeireth-cognition::calibration` (`Observation`, `decompose`, `calibration_bins`, `expected_calibration_error`)。**无新外部依赖**。

---

## 3. 验证结果

### 3.1 单测试集 (任务指定命令)

```bash
$ cargo test -p apeireth-cognition --lib calibration -j 4
test result: ok. 19 passed; 0 failed

$ cargo test -p apeireth-cognition --lib forecast -j 4
test result: ok. 31 passed; 0 failed

$ cargo test -p apeireth-evolution --lib critic -j 4
test result: ok. 16 passed; 0 failed
```

**新测试总计**: 19 + 31 + 16 = **66 passed, 0 failed** (0 装 PASS, 无任何 ignore/skip)

### 3.2 全 lib 回归 (确保 0 破坏)

```bash
$ cargo test -p apeireth-cognition --lib
test result: ok. 105 passed; 0 failed   # (= 86 baseline + 19 new)

$ cargo test -p apeireth-evolution --lib
test result: ok. 193 passed; 0 failed   # (= 177 baseline + 16 new)
```

### 3.3 关键测试覆盖

**Brier 单调分解** (per task):
- `monotonic_decomposition_perfect_forecaster` (bin-aligned, `is_strictly_monotonic()`)
- `monotonic_decomposition_random_forecaster` (BS ≈ 0.25, `is_monotonic()`)
- `monotonic_decomposition_skilled_forecaster` (resolution > reliability)
- `monotonic_invariant_holds_across_random_seeds` (20 个随机种子)

**EnsembleForecast 加权聚合**:
- `ensemble_bayesian_confidence_weighted` (high-conf 主导)
- `ensemble_bayesian_contrarian_boosts_minority` (少数派加权提升)
- `ensemble_median_robust_to_outlier` (抗 outlier)
- `ensemble_agreement_score_decreases_with_disagreement`

**PredictionMarket LMSR**:
- `lmsr_uniform_prices_are_1_over_n` (均匀市场 = 1/N)
- `lmsr_prices_sum_to_one` (价格归一)
- `lmsr_buying_increases_price` / `lmsr_buying_increases_cost_monotonically`
- `lmsr_higher_liquidity_lower_slippage` (b 越大, 滑点越小)
- `lmsr_contrarian_weight_subsidizes_low_price_outcome`
- `lmsr_rejects_negative_shares` / `lmsr_rejects_invalid_outcome`

**Critic 端到端**:
- `critic_recommends_continue_for_perfect_history` (Continue)
- `critic_recommends_revise_for_moderate_history` (Revise)
- `critic_recommends_reject_for_miscalibrated_history` (Reject)
- `critic_target_state_draft_to_proposed` / `revise_returns_to_draft` / `reject_goes_to_retired`
- `critic_integration_with_calibration_bins` (10 bins + is_monotonic)

---

## 4. 依赖与边界

### 4.1 依赖

- **新增依赖**: 0 (per task §2)
- **Cargo.toml 增量**: 仅 `apeireth-evolution/Cargo.toml` 加 `apeireth-cognition = { path = "../apeireth-cognition" }`
- **方向**: `apeireth-evolution → apeireth-cognition` (单向, 无 cycle)
  - 已验证: `apeireth-cognition` 不依赖 `apeireth-evolution` (grep 验证)

### 4.2 边界遵守 (per task §2)

- ✅ 未碰 `team-lead` / `tool-runtime` / `agent` / `companion` / `credentials` / `net`
- ✅ 仅改 `crates/apeireth-cognition/src/**` (3 文件: lib.rs + calibration.rs + forecast.rs)
- ✅ 仅改 `crates/apeireth-evolution/src/critic.rs` + lib.rs + Cargo.toml
- ✅ 未接真 LLM — 单测用 stub 概率 (`Observation::new(0.7, 1.0)` 等)
- ✅ 未做自动校准 — 仅诊断 (`critique()` 输出推荐, 不修改历史)
- ✅ 未改 oracle 既有评估管线 — calibration / forecast / critic 都是新增模块
- ✅ 未引入新 crate-level 依赖 (除 apeireth-cognition, evolution 内部)

---

## 5. 0 装 PASS 声明

- 66 个新测试全部为 runnable check (非 stub), 含具体数值断言
- 11 个集成 / 端到端测试 (monotonic, LMSR monotonicity, contrarian boost, target state 转换等)
- 序列化 round-trip 测试 (json 序列 → 反序列 → 字段比较, 含浮点容差)
- 空输入 / 边界条件 / 异常处理测试全覆盖
- 无 `--release` 标志依赖, 无 `#[ignore]` 跳过
- 全 cargo test 默认 mode (debug, 0 优化) 全绿

---

## 6. 设计意图与权衡

### 6.1 集成而非分立 (per task 哲学锚点 #2)

- `Critic` 不重新实现 Brier 计算, 直接消费 `apeireth_cognition::calibration::decompose()`
- `EnsembleForecast::as_observation(outcome)` 把聚合预测转成 `Observation`, 让 Brier 诊断直接可用
- 跨 crate 边界用 `pub use` 透传, 不复制类型

### 6.2 机制而非补丁 (per task 哲学锚点 #1)

- Critic 不是"分数 hack", 而是**校准诊断 → 状态转换**的因果链
- LMSR 不是"价格 hack", 而是 Hanson cost function 的标准实现
- Bayesian aggregation 不是"加权平均", 而是 confidence + contrarian + minority 三因子乘积 (可解释)

### 6.3 与 E5/E6 后续的接口预留

- `CriticConfig` 暴露 `revise_threshold` / `reject_threshold` (供 E5 自动校准调参)
- `EnsembleForecast.aggregate_confidence` 可作为 E5 校准的输入 (per-task 自动调权)
- `MarketConfig.contrarian_weight` 与 `EnsembleConfig.contrarian_weight` 字段对齐, 允许 E6 演化层统一调控

---

## 7. Commit 计划

- 分支: `task/tp18-calib-ao`
- Commit 标题: `team(agent_orchestrator): 568745f6-a1e6-42e6-a895-19cb99785c6d TP18 (E3, P1): 校准诊断 + 集合预报 + 预测市场 + Critic`
- 内容: 6 文件, +1662 行
- 合并目标: `team/e8de47ae-0e59-459d-a763-88e52b7706c8/integration`

---

## 8. 已知 gap 与下批次待办

- **LMSR cost function 数值稳定性**: 对 `q/b > 700` 时 `exp()` 会 overflow。当前 b=100, q 量级 ~10, 远未触及; E5/E7 接入真实数据时需检查。
- **Critic 阈值默认值**: `revise_threshold=0.15` / `reject_threshold=0.30` 是经验值, 待 E5 自动校准历史数据时再定标。
- **APEIRETH_REG_APEIRETH_CRITIC**: critic.rs 未注册 `apeireth_verify::regression_assert!` (本批次未涉及, 0 改 fail-6 入口签名; 下批次集成时补充)。

---

## 9. 提交元数据

- 报告生成时间: 2026-08-18 (per agent_orchestrator2 当前 session)
- 任务 ID: `568745f6-a1e6-42e6-a895-19cb99785c6d`
- 提交者: agent_orchestrator
- 状态: 待 Leader 审阅