# R255 -- Runtime pluggable worker registry + LlmWorker dispatch

## 背景
- R149 已经写了 `LlmWorker` (真接 MiniMax Chat Completions API), 但只暴露了
  `chat()` 和 `AsyncWorker::execute()` 这两个 API.
- Runtime 的 `dispatch_async_task` 写死了 `SimulatedWorker::new(tool_name)`,
  没有任何 hook 让你用真 LLM. 整个 Runtime 编排链路 (bus + arbitration +
  search + group_chat + emotion) 0 接受外部 worker.

## 改动
1. Runtime 加 `worker_registry: Arc<Mutex<HashMap<String, Arc<dyn AsyncWorker>>>>`.
2. `register_worker(name, worker)` 公开 API: 把 worker 注入 registry.
3. `dispatch_async_task_with_worker(worker, params_json)` 公开 API: 用外部 worker
   跑一次任务, 走完整任务生命周期 (Pending -> Running -> Completed) + bus emit.
4. `dispatch_llm_task(prompt, system, model, base_url, api_key)` 公开 API:
   构造 LlmWorker, 然后走 #3 的链路. 让 Runtime 编排层与真 LLM 第一次接上.
5. `dispatch_async_task` 旧路径: 先查 registry, miss 才 fallback SimulatedWorker.
   0 触碰既有 435 tests.

## 5 tests (r255_01..05)
- r255_01: register_worker 写入 registry
- r255_02: 未注册时 fallback SimulatedWorker (output 含 "ok-simulated")
- r255_03: 注册后走自定义 worker (tool = "custom_tool")
- r255_04: dispatch_llm_task 返回 TaskId (fake key, 不等 HTTP)
- r255_05: with_config 同时给 model + base_url 路径

## 性质
- 0 引入新 dep
- 0 触碰 3 不可变脊柱
- 100% backward compat (既有 dispatch_async_task 行为不变)
- Runtime tests: 48 -> 53 (+5)

## 后续
- R256: TUI 接入新 runtime (driver + metrics)
- R257: 真接入 MiniMax API key 跑端到端 (用 .openclaw/apikey.txt)
