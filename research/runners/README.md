# runners/

实验运行器入口。**已实现**：`apeireth-research-runner`(独立 cargo 项目,零依赖)。

## 使用

```bash
cargo run --release --manifest-path research/runners/Cargo.toml -- --seed 42 --turns 500 --budgets 2000,4000,8000,16000,32000
```

## 功能(2026-09-04)

- 合成保留/检索基准(局部性相关查询,确定性 PRNG,同 seed 可复现)
- 4 策略 × N 预算档的**效用-成本曲线**(utility vs avg prompt tokens)
- **bootstrap 95% CI**(1000 次重采样)的策略间效用差
- JSONL 研究日志(schema 对齐 `logs/README.md`;生成日志不入库,同 seed 可再生)

## 真实数据集即插即用

实现 `BenchmarkSource` trait(`docs()`/`turns()`)并替换 `SyntheticSource`,
运行器骨架与统计/日志逻辑不变。目标数据集:LoCoMo、LongMemEval、GAIA、τ-bench(接入时在 `datasets/` 记 MANIFEST:来源/license/规模/污染检查)。

## 原则

- 运行器不修改产品代码;不进主 workspace 默认构建(根 Cargo.toml 已 exclude `research`)
- 每个运行器输出 JSONL 到 `research/logs/`(seed + config_hash 可复现)
- 合成数据结论只算"方法学验证",不算"生产证据"(0 装口径)
