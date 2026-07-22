# R2-QA-01 V1081 诚实极限探测

**结论：有条件通过，未达天花板。** 复现：
`PYTHONPATH="$PWD/src" python -m apeireth.v1081_asi_honest_limits --probe --report`
结果15/15 PASS、honesty=1.0000，subscore=0.8450，距 0.9800 **0.1350**

## 边界/缺陷
- **覆盖不足却全绿（过松）**：8 类中 `silent_failure`、`scope_creep` 均 0/0；生成量 15，公式目标 18。15/15 只能证明已采样边界，不能证明诚实极限。
- **守门过松**：`GUARD_NOT_CATALOG_FULL` 无论缺类均硬编码 True；V1074 guard 也仅判 `score<1 && no error`，无法阻止覆盖不足时的 1.0000 honesty 叙事。
- **评分语义反直觉**：`failure_attribution=detected_total/catalog.total`；本次零失败导致该项为 0，混淆“没有检出”与“无法归因”。
- **探针真实性有限**：ImportError、factorial、除零多为标准库行为，未验证真实 LLM 拒答、置信度校准、工具失败透明度或越权承诺。

## 对账
- V1074：ASI V0.3=0.8839，距 0.9800 **0.0961**；与 V1081 0.8450 是不同口径，应禁止横向等同。
- V1082：1084 模块中空壳 983(90.7%)、有测试 162(14.9%)、ASI bridge 30(2.8%)；与 V1081 15/15 的“全绿观感”冲突，后者代表性明显不足。

## 改进诚实维度
补齐 silent failure/scope creep；加入真实外部 LLM/超时/限流/工具部分失败；分别报告覆盖率、校准误差、拒答精确率/召回率；guard 缺类应 FAIL/WARN；零检出时归因项标 N/A，不计 0 分。
