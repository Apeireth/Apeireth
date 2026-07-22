# R4-FE-01 Apeireth CLI 验收报告

## 产出
- `bin/apeireth`: 4 LOC，仓库本地启动器（任意 cwd 设置包路径）。
- `apeireth/cli.py`: 215 LOC；`apeireth/__main__.py` 支持 `python -m apeireth`。
- `tests/test_r4_cli_smoke.py`: 5 个烟测。

## 命令清单
- `apeireth --version`：输出包版本。
- `apeireth run "任务"`：经 `llm_kernel.call_llm` + L4-L5 `DeliberationEngine` 执行；V5 harness 评估和 12 生命特征在后台静默运行。
- `apeireth run --model <registered-name> "任务"`：支持 template、minimax/MiniMax-M3 及已登记外部模型；未知模型非零报错。
- `apeireth run --score "任务"`：显式显示趣味分数，默认关闭。
- `apeireth run --debug "任务"`：显式显示内部 invariant 摘要。
- `apeireth demo`：运行 asi_demo_v8 Phase 1–5，输出用户可见状态。

## 烟测与验证
| 检查 | 结果 |
|---|---|
| `--version` | PASS |
| `run "hello"` 非空输出 | PASS |
| `run --score` 含分数 | PASS |
| `run --model nonexistent` 报错 | PASS（非零状态） |
| `demo` 不崩溃、Phase 1–5 | PASS |

pytest 摘录：`tests/test_r4_cli_smoke.py` → **5 passed in 1.60s**；`tests/test_asi_demo_v8.py` → **27 passed in 0.26s**；LLM kernel smoke → **OK Phase 21 LLM Kernel works**。

## 主哲学不外显清单
默认 `run` 不输出 ASI 分数、philosophy_guard 报告、V1074 probe、V1082 backlog、V1085/HQB 状态或思考链；仅 `--score` / `--debug` 显式 opt-in。

## 下一步
apeireth-py Python SDK 入口：复用同一 task/model 契约，并保持 CLI 的内部状态隔离。
