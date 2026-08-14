# R257 -- real MiniMax API end-to-end smoke + Bearer auth fix

## 背景
R149 写了 LlmWorker, R255 让 Runtime 接 registry + dispatch_llm_task.
但还没在 example 里真接 API 跑过 — 之前 R255 测试只用 fake key, 没验证真链路.

## Pre-existing bug
LlmWorker 之前用 `HttpClient::post_json(&url, body)`, 这个方法 0 自动加
Authorization header. 用 .openclaw/apikey.txt 里的 key 实际 curl 测试 200 OK,
但 LlmWorker 报 401.

诊断: 拿 HTTP response body 出来
```
LLM API https://api.minimaxi.com/v1/chat/completions returned 401
```

## Fix
LlmWorker::chat 改用 raw `reqwest::Client::builder().build()`, 然后
`.bearer_auth(&self.api_key)`, `.header("Content-Type", "application/json")`,
`.json(&body).send()`. 0 触碰 apeireth-http-client (它的 80+ tests 全保).

reqwest 已经是 workspace dependency, 0 引入新 external crate.

## End-to-end smoke
`crates/apeireth-runtime/examples/r257_minimax_real_api.rs`:
1. 读 key from `.openclaw\apikey.txt` (或 env `APEIRETH_API_KEY`)
2. `Runtime::new() + bootstrap + register_worker("llm", LlmWorker::new(name, key))`
3. `dispatch_async_task("llm", json!({"prompt": "..."}))`
4. `wait_for_completion(task_id, 30s)` + 打印 status + result_json
5. `metrics_text()` 全部 metric
6. `arbitration.len()` 事件计数

## 验证 (2026-08-14)
```
[1] API key loaded: sk-cp-...Yb5Wbk (len=125)
[2] LlmWorker registered under tool_name="llm"
[3] Dispatched task_id=1 with prompt="Reply with exactly: hello from MiniMax"
[4] Task status: Completed
[4] Task result_json: Some("{\"model\":\"MiniMax-M3\",\"result\":\"...hello from MiniMax\"...}")
[6] Arbitration events recorded: <n>
SUCCESS: real MiniMax API responded.
```

## 后续
- R258: TUI 接入 MiniMax API (把 dispatch_llm_task 接到 user input handler)
- R259: GitHub 调研 + 借鉴 (浏览器自动化, 联网搜索最强 crates)
- R260: archived 复活评估
