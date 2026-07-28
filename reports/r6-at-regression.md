# R6-AT-01b 全量回归报告

## 结论：FAIL（回归被阻断）

命令：`python -m pytest tests/ -q --ignore=tests/test_v121_v150.py --ignore=tests/test_v251_v500.py --ignore=tests/test_v501_v1000.py`

- 结果：**3485 passed / 2 failed / 1 skipped / 3037 errors**
- 总耗时：**1087.92s (18:07)**；pytest 完成汇总，无进程 crash
- R3 baseline：4764 passed / 1 failed
- passed 差值：**-1279**，但不可作为覆盖变化结论：运行约 53% 后发生捕获流污染，后续 3037 项均级联 ERROR
- 新增 failed：表面 +1；实际存在一个可稳定复现的测试基础设施回归

## 新回归证据

`tests/test_v1077.py::TestFullMeasurementAggregator::test_aggregate_basic` 在退出上下文时触发：

`ValueError: I/O operation on closed file`

该测试关闭 pytest capture 的临时流，随后 `tempfile.py:500` 在 setup/teardown 重复抛出同一异常，造成 3037 个级联 ERROR。隔离进程复跑该用例仍为 **1 failed / 1 error**，故不能验收“0 新增 regression”。

## 已知环境失败

`tests/test_v1058.py::TestLLMEndpointClient::test_find_api_key_empty`：期望 `""`，实际读取宿主环境中的 `sk-...` API key；与 R1/R3 已知 env-dependent 失败一致（报告不记录完整密钥）。

## 强制关键测试

独立复跑：

- `test_v1085_philosophy_guard_hardening.py`：20 passed
- `test_r4_cli_smoke.py`：5 passed
- `test_r4_serve_smoke.py`：8 passed
- 合计：**33 passed in 6.72s**

原始日志：`reports/r6-at-pytest.tmp.log`。
