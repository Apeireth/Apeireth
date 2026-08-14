# R256 -- runtime_bridge exposes metrics + LLM dispatch to TUI

## 背景
- R255 让 Runtime 接 LlmWorker (registry + dispatch_llm_task).
- runtime_bridge 是 TUI 唯一观察 Runtime 的窗口, 但 0 暴露这些新 API.
- TUI status bar 0 看见 runtime_cycle_total / cycle_latency / supervisor 心跳.

## 改动
1. `RuntimeBridge::metrics_text()` 透传 Runtime::metrics_text()
2. `RuntimeBridge::cycle_latency_summary()` 透传 (count, sum_ms, mean)
3. `RuntimeBridge::supervisor_heartbeat_count()` / `supervisor_tick_duration_count()`
4. `RuntimeBridge::dispatch_llm_task(prompt, system, model, base_url, api_key)`
5. `RuntimeBridge::register_worker(name, worker)` / `dispatch_task_with_worker(w, p)`
6. `snapshot_json()` 加 6 个指标字段

## 6 tests (r256_01..06)
- r256_01: metrics_text 返回 runtime_cycle_total
- r256_02: cycle_latency_summary 初始 0
- r256_03: supervisor_heartbeat_count + tick_duration_count 初始 0
- r256_04: dispatch_llm_task 返回 TaskId
- r256_05: register_worker 写入 registry
- r256_06: snapshot_json 包含 6 个新指标

## 验证
- runtime_bridge.rs: 16/16 tests pass (10 existing + 6 new)
- 0 编译错误 / 0 新增 warning
- 0 引入新 dep
- 0 触碰 3 不可变脊柱

## 后续
- R257: 真接入 MiniMax API key 跑端到端 (用 .openclaw/apikey.txt)
