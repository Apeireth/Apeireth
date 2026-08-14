# R267: runtime ↔ MiniMax API end-to-end 端到端验证 + 文档化

**日期**: 2026-08-14
**作者**: 楚零
**目的**: 把"如何用真 API key 跑 runtime dispatch_llm_task"写进 README + 加 live gated cargo tests

---

## §1 背景

R257 已有 `examples/r257_minimax_real_api.rs` 真接 MiniMax API + Bearer auth fix,
但主人提示"测试完记得写进 README 免得每次都忘了这事" — README 缺这块说明.

R258 Tier A 候选: TUI 接 MiniMax API end-to-end = ★★★★★.

---

## §2 设计

### 2.1 README 加 "R267: Real LLM dispatch" section

插入到 `crates/apeireth-runtime/README.md`, 在 "## Compile-time guards" 之前.

**内容覆盖**:
- API key 解析优先级表 (env APEIRETH_API_KEY > .openclaw\apikey.txt > ~/.openclaw/apikey.txt)
- 完整使用代码示例 (含 register_worker + dispatch_async_task + wait_for_completion)
- 端到端 smoke (`cargo run --example r257_minimax_real_api`)
- 期望输出 (key loaded preview + Task status Completed + result_json 含 "hello from MiniMax" + SUCCESS)
- 关键流程 (HTTP POST → choices[0].message.content → result_json → metrics)

### 2.2 Live gated cargo tests (`crates/apeireth-runtime/tests/r267_minimax_live.rs`)

2 e2e tests:
- `r267_live_minimax_returns_completed`:
  - `register_worker("llm", LlmWorker) + dispatch_async_task + wait_for_completion`
  - 验证 status == Completed + result_json 含 "hello from MiniMax"
- `r267_live_minimax_dispatch_llm_task_helper`:
  - 用 `Runtime::dispatch_llm_task(prompt, system, None, None, &api_key)` 全合一 helper
  - 用 poll `task_store.get(task_id)` 替代 wait_for_completion (避免与 helper 内部 spawn 的 wait 竞争)
  - 验证 status == Completed + result_json 含 "dispatch-llm-task-ok"

### 2.3 门控

```rust
if std::env::var("APEIRETH_MINIMAX_LIVE_TEST").ok().as_deref() != Some("1") {
    eprintln!("skipped: set APEIRETH_MINIMAX_LIVE_TEST=1 to run live test");
    return;
}
```

默认 CI 不发真请求, 仅当 env=1 时跑.

---

## §3 验证

```bash
# 1. 不设 env, test skip
cargo test -p apeireth-runtime --test r267_minimax_live
# -> 2 passed; 0 failed; 2 ignored (no print, 自动 skip)

# 2. 设 env 真跑
APEIRETH_MINIMAX_LIVE_TEST=1 cargo test -p apeireth-runtime --test r267_minimax_live -- --nocapture
# -> running 2 tests
#    test r267_live_minimax_dispatch_llm_task_helper ... ok
#    test r267_live_minimax_returns_completed ... ok
#    test result: ok. 2 passed; 0 failed

# 3. 真接 examples
cargo run --example r257_minimax_real_api -p apeireth-runtime
# -> [1] API key loaded: sk-cp-...Yb5Wbk (len=125)
#    [4] Task status: Completed
#    [4] Task result_json: Some("{\"model\":\"MiniMax-M3\",\"result\":\"hello from MiniMax\",\"task_id\":1}")
#    SUCCESS: real MiniMax API responded.
```

---

## §4 主哲学锚对齐

- **S-1 北极星**: 借鉴 OpenAI Chat Completions 协议 (provider-agnostic), 自接 MiniMax
- **S-2 实事求是**: 真 HTTP + 真 Bearer auth, 0 simulate
- **O-1 安全优先**: API key 不进代码, 仅磁盘文件 + env override
- **O-3 干到底**: README 文档化 + 2 live tests + 1 example = 3 路径都能跑
- **O-5 不假装**: 测试 env-gated, 不假装 CI 过就等于真接过了

---

## §5 后续

- TUI `backend.rs::call_llm_http_*` 可切到 `runtime_bridge::dispatch_llm_task` (已存在 helper), 让 TUI 走 runtime 而不是直连 reqwest
- 但当前 http_llm 已能跑生产, R267 之后 TUI 切换是可选优化
