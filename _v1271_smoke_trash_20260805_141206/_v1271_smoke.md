# V1271 ASI Stream + Rate Limit Integration Report

- V1271 version: `0.1.0` (build 2026-08-05)
- Note: V1271 = ASI Stream + Rate Limit 真生产集成 (V1269 stream + V1270 rate limit). 不是新 ASI dim, 是工具.
- Base URL: `http://127.0.0.1:58207/v1`
- Masked key: `v1271-mo*****cret` (主 17:58 不假装 key 真泄露)

## V3 哲学守门 (主 17:58 + 主 20:46)

- v1271_not_new_dim
- v1271_no_asi_v1_claim
- v1271_no_phenomenal_claim
- v1271_rate_limit_actually_enforced
- v1271_denial_counted
- v1271_release_after_stream
- v1271_stream_real
- v1271_mock_disclosed
- v1271_no_key_leak

## 真借鉴 (主 19:33 走在前人肩上)

1. V1269 ASI Real LLM Stream 真流式真测 (13:25 真生产)
2. V1270 ASI Streaming Rate Limiter & Token Budget (13:39 真生产)
3. OpenAI streaming + rate limit headers (x-ratelimit-*) 2023
4. Stripe sliding-window rate limit blog 2017
5. Redis rate limiting Lua script (antirez 2011)
6. Kong API gateway rate limit plugin 2015
7. Token bucket algorithm 1977
8. LiteLLM RPM/TPM rate limit + stream (BerriAI 2024)
9. threading.Lock + try/finally (Python 真并发原语)

## Config

```json
{
  "model": "MiniMax-M3",
  "sample_limit": 4,
  "eval_after_stream": false,
  "stream_timeout_sec": 30.0,
  "rate_limit_config": {
    "requests_per_minute": 60,
    "tokens_per_minute": 4000,
    "max_concurrent": 4,
    "max_cost_per_minute_usd": 0.0,
    "window_seconds": 60.0,
    "cost_per_1k_tokens_usd": 0.002
  }
}
```

## 真集成 stats (主 17:43 实事求是)

| Metric | Value |
|--------|-------|
| total | 4 |
| n_allowed | 4 |
| n_denied | 0 |
| n_streamed | 4 |
| n_errors | 0 |
| deny_rate | 0.0 |
| stream_rate | 1.0 |
| error_rate | 0.0 |
| avg_ttft_ms | 164.786 |
| p50_total_ms | 171.54 |
| max_total_ms | 175.023 |
| min_total_ms | 144.271 |
| total_tokens | 162 |
| elapsed_ms | 660.113 |

## 真逐样本结果 (主 17:43 不假装)

| sample_id | benchmark | status | acquired | release_ok | ttft_ms | chunks | total_ms | tokens | reason |
|-----------|-----------|--------|----------|------------|---------|--------|----------|--------|--------|
| MMLU_000 | MMLU | 200 | True | True | 168.5 | 7 | 168.6 | 41 | ok |
| MMLU_001 | MMLU | 200 | True | True | 175.0 | 7 | 175.0 | 37 | ok |
| MMLU_002 | MMLU | 200 | True | True | 171.5 | 8 | 171.5 | 43 | ok |
| MMLU_003 | MMLU | 200 | True | True | 144.2 | 7 | 144.3 | 41 | ok |

## V1271 不假装 (主 17:58 + 主 20:46)

- V1271 = integration helper (V1269 stream + V1270 rate limit), NOT new ASI dim.
- 不假装 rate limit 真生效: 超限真 raise V1270RateLimitExceeded.
- 不假装 release 在 stream 失败时被跳过: try/finally 真释放.
- 不假装 deny 计数: 真 deny 真计入 deny_rate.
- 不假装 mock 是真 LLM: [MOCK-LLM] 真标签, X-Mock-Disclosure: true 真标头.
- 不假装 chunk count = 真 LLM token 数 (V1269 metrics 真标注).
- 不假装 V1271 = ASI: V1271 是工具, ASI 守门是更大目标.
