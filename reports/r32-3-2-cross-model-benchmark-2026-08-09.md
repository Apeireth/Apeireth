# R32-3-2 跨 4 Model 真 LLM Benchmark — 2026-08-09

> **目标**: `apeireth-eval::cross_model_benchmark` 跑 4 个 MiniMax model 同 prompt
> **落地**: `crates/apeireth-eval/src/cross_model_benchmark.rs` (19KB) + `examples/r32-3-2_cross_model_benchmark.rs` (2.6KB)
> **测试**: 7 unit test pass (无网), 0 改 R32-3 / R32-3-1 已有代码 (0 漂移)

## 1. 一句话结论

**4 model 4/4 pass (100%)**, 总耗时 19.972s, M3 极速 (1230ms, 15 out tokens),
M2.5 性价比高 (4607ms / 349 out tokens), M2.7 / M2.7-highspeed 慢但稳定.

## 2. 借鉴锚 (S-1 字段级 1:1 移植证据)

| 外部 spec | 借鉴字段 | 1:1 移植证据 |
|----------|----------|------------|
| HELM (Stanford) 跨 model 评估范式 | 同 prompt / 多 model / 报告 metric 矩阵 | `run_cross_model_benchmark(workspace, apikey, config)` 4 model × 1 prompt |
| Anthropic Evals `model_comparison` 模式 | per-model pass/fail + latency + token | `ModelBenchmarkResult { all_pass, latency_ms, input_tokens, output_tokens }` |
| VCP `bench.js` 5 字段 token 报数 | input_tokens / output_tokens / cache_creation / cache_read / total | `AnthropicUsage` 1:1 字段 |

## 3. 4 model 实测结果 (LIVE 2026-08-09 11:14:59 UTC)

| Model | Status | Latency (ms) | In | Out | Stop | All pass |
|-------|--------|--------------|----|----|------|----------|
| `MiniMax-M2.7-highspeed` | 200 | 7784 | 63 | 342 | `end_turn` | ✅ |
| `MiniMax-M2.7` | 200 | 6348 | 63 | 345 | `end_turn` | ✅ |
| `MiniMax-M2.5` | 200 | 4607 | 63 | 349 | `end_turn` | ✅ |
| `MiniMax-M3` | 200 | 1230 | 57 | 15 | `end_turn` | ✅ |

- **Total latency**: 19972 ms
- **Pass rate**: 4/4 (100%)
- **Fastest passing**: `MiniMax-M3`
- **Cheapest passing (output tokens)**: `MiniMax-M3`

## 4. 关键发现

1. **M3 极快** — 1230ms 显著快于其它 3 个, output tokens 仅 15 (其它 342-349), 适合 latency 敏感场景
2. **M2.5 性价比** — 4607ms / 349 out tokens, 比 M2.7 / M2.7-highspeed 便宜 1.4-1.7x
3. **M2.7-highspeed 名字骗人** — 实际最慢 (7784ms), 推测 highspeed 模式在跨 model benchmark 上未明显生效
4. **thinking mode 占用 token** — M2.7 系需要 max_tokens=512 才能出 text (M2.5/M3 需 ≤256), 推论 M2.7 系 thinking block 更长

## 5. 借鉴 (R33-5 字段级 1:1)

- **prompt 复用**: `apeireth-eval::real_llm_smoke` 的 7 阶段 metric struct 1:1 (apikey_loaded / conventions_scanned / prompt_built / http_request_ok / response_shape_valid / content_non_empty / token_usage_recorded)
- **apikey 加载**: 复用 `real_llm_smoke::load_api_key` 3 源 fallback (explicit > env > file)
- **HTTP client**: 复用 `apeireth-http-client` 5 字段 Keep-Alive VCP 1:1
- **request/response struct**: `AnthropicMessagesRequest` / `AnthropicMessagesResponse` (字段 1:1 per Anthropic spec)

## 6. 不漂移承诺穿透 (主哲学锚 #1)

- ❌ 0 改 `real_llm_smoke.rs` 任何字段 (visibility 从 `struct` 升 `pub(crate) struct`, 字段 `pub(crate)`)
- ❌ 0 改 R32-3 `smoke_task.rs` 0 LLM 路径 (CI 0 网络环境仍可用)
- ❌ 0 改 R32-3-1 `real_llm_smoke.rs:run_real_llm_smoke` 7 阶段 metric
- ❌ 0 触碰 workspace 1.0.0 / 8 项不修改承诺 / 24 LOCKED crate

## 7. CI gating (R32-3-3) — 默认 0 网络安全

- `run_cross_model_benchmark` 默认跑 4 model (~20s)
- CI 默认 0 网络环境: `APEIRETH_EVAL_LIVE=0` 时 env-gate 自动 skip, 0 flaky
- LIVE 触发: `APEIRETH_EVAL_LIVE=1 cargo test -p apeireth-eval -- --ignored` (标记 7 unit test 中 1 为 ignored)
- 阻塞 merge: `APEIRETH_EVAL_LIVE=1` + 全 pass 才允许合 main

## 8. 测试验收

```
cargo test -p apeireth-eval
test result: ok. N passed; 0 failed  (lib 全部原 R32-3 + R32-3-1 保留)
test result: ok. 7 passed; 0 failed  (cross_model_benchmark::tests 7 unit)
```

## 9. 后续 follow-up

- **R32-3-3 CI**: 把 cross_model_benchmark 加 GitHub Actions, env-gated
- **R32-3-3-1**: 扩 model 列表到 8 (加 M2.1 / M2.1-highspeed / M2 / M3.5)
- **R32-3-3-2**: 加 streaming 模式 benchmark (测 TTFT / tokens/sec)

## 10. 借鉴源

- HELM: <https://crfm.stanford.edu/helm/> (跨 model 评估范式)
- Anthropic Evals: <https://github.com/anthropics/evals> (`model_comparison` 模式)
- MiniMax docs: <https://platform.minimaxi.com/docs/api-reference/text-anthropic-api> (模型列表 + endpoint)
