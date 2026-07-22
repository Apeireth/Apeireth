# R3-PHL-01 philosophy_guard 加固

## 变更
- `apeireth/philosophy.py` 0.2→0.3，`+181/-24`：显式违规正则取代 `red_line.split("/")`；修复 BadV2 漏检；空/伪输入、证据、类别 fail-closed；`attribution_score` 独立判定（<.90 FAIL；.9790–.9800 WARN；>.9800 FAIL），上游 PASS 无放行权；结果新增 `status/warnings`。旧两参数 API 保持。
- 新增 `tests/test_v1085_philosophy_guard_hardening.py`：126 行、20 实例；未动 V1074/V1081/PASS 测试。

## 单测简表
| name | 验证点 | expected/actual |
|---|---|---|
| absolute_claims | Phenomenal peak / system conscious | FAIL/FAIL×2 |
| hardcoded_true | V1074 全 True + score=.49 | FAIL/FAIL |
| attribution | .95 与 .85 有别 | PASS/FAIL = PASS/FAIL |
| near_ceiling | .9799 距 .9800 .0001 | WARN/WARN |
| snapshot_independent | .40<1 旧规则 True | FAIL/FAIL |
| invalid_inputs | 空/None/object/dict | FAIL/FAIL×4 |
| missing_category | 缺 silent_failure | FAIL/FAIL |
| BadV2 / honest_negation | 缩减中央 AI / 诚实未达 | FAIL/PASS = FAIL/PASS |

## 结果 / 对比
- 新测试：`20 passed in 0.24s`；相关 V1074/V1081：`21 passed`；4 个旧脚本 exit 0，合法 PASS 保留、BadV2 由误 PASS→FAIL。
- 指定全量：隔离宿主 `ANTHROPIC_API_KEY` 后 `4785 passed, 2 warnings in 348.73s`。未隔离首跑仅 V1058 空 key 环境用例失败（与本 diff 无关）。
- 之前交接 PASS 依赖字面/常量，BadV2 与空输入可绿；现在需求漏洞类别 **6/6**、20/20 实例覆盖，且 V1074 代码未改。

## V1086+
让 V1074/V1081 适配结构化 guard、移除常量 True；增加中英否定/绝对声明 fuzz 语料。本轮不越界改独立路径。
