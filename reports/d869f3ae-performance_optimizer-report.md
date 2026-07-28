# R8 Track C V1093 性能优化证据

**任务:** `d869f3ae-f444-4fa9-805c-fd70d697`  
**范围:** `apeireth/v1093_dgm_archive.py` 的多轮演化循环

## 瓶颈证据

V1093 的候选轮次只修改隔离的 JSON harness state，但每轮都会重复执行相同的：

1. `python -m py_compile apeireth/v1093_dgm_archive.py`
2. `python -m pytest tests/test_v1004.py -q`

4 轮隔离基线（替身 V1074 snapshot/V0.4，子进程为真实命令）：

| 指标 | 基线 | 优化后 | 变化 |
|---|---:|---:|---:|
| 总耗时 | 8388.89 ms | 2282.26 ms | -72.8% / 3.68x |
| 静态验证子进程 | 6 | 2 | -66.7% |
| 完成演化轮次 | 3 | 3 | 不变 |
| archive JSON | 可解析 | 可解析 | 不变 |

## 最小优化

把与 JSON 候选无关的固定源码编译和固定测试移到 experiment 级别，只执行一次；每轮仍保留：

- 真实 `StatusSnapshotBuilder.build()`；
- HQB 四维与 composite 判定；
- compile/test/guard 验证证据；
- candidate、run record、archive artifact；
- keep/partial/revert 与连续三次回退规则。

没有引入缓存框架或新依赖。

## 验证

```text
python -m pytest tests/test_v1093.py -q
6 passed in 1.59s
```

新增回归检查锁定：4 轮 experiment 只启动 2 个静态验证子进程，且每条 evolution record 都含成功的 compile/test 结果。

## 边界与升级条件

当前优化成立的前提是候选仅修改 JSON state。若未来候选获准修改 Python 源码，应升级为“源码/测试输入哈希变化时重新验证”，不能继续无条件复用 experiment 级结果。

现有 `r8-trackc-perf-raw.json` 的 `mean_speedup=49.481x` 未作为收益依据：其中多个方案实测变慢，且均值被近零耗时的 O(1) 微基准放大。本报告只认真实 V1093 路径的同口径前后对照。
