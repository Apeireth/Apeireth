# R2-AUTOTEST-01 全量回归对账

**ID**: 8c6b32e2 | **时长**: 426.33s (7m06s) | **2026-07-22**

## 计数

PASSED **4745** / FAILED **1** / SKIPPED **0** / **总数 4746**

## 与"3896+ 全过"对账

- PASSED 4745 vs 基线 3896+ → **更多** ✅（多 ~849 已落测试）
- FAIL 1 vs 基线 0 → ⚠️ **存在回归**（基线称"全过"零失败，本次新增 1 个）

## 回归失败（1 条）

`tests/test_v1058.py::TestLLMEndpointClient::test_find_api_key_empty` — 期望 `api_key == ""`，实际拿到环境泄露的 key (`sk-cp-kug0t7...AbaEHOb8YRsUg`)。建议：测前清空 `*API*KEY*` 环境变量，或 client 构造时显式 `api_key=None`。

## 非 `--ignore` 列表内的 SKIPPED

无（SKIPPED = 0）。

## 备注

2 条非阻断 PytestCollectionWarning（`TestVerifier`/`TestMapping` 有 `__init__` 无法收集为 test class）。已按指令排除 `test_v121_v150 / test_v251_v500 / test_v501_v1000`。
