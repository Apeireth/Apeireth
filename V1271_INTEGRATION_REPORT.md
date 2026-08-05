# V1271 ASI Stream + Rate Limit Integration Report

- V1271 version: `0.1.0` (build 2026-08-05)
- Note: V1271 = ASI Stream + Rate Limit 真生产集成 (V1269 stream + V1270 rate limit). 不是新 ASI dim, 是工具.
- Base URL: `http://127.0.0.1:58293/v1`
- Masked key: `v1271-te***************3456` (主 17:58 不假装 key 真泄露)

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
  "sample_limit": 22,
  "eval_after_stream": false,
  "stream_timeout_sec": 30.0,
  "rate_limit_config": {
    "requests_per_minute": 60,
    "tokens_per_minute": 8000,
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
| total | 22 |
| n_allowed | 22 |
| n_denied | 0 |
| n_streamed | 22 |
| n_errors | 0 |
| deny_rate | 0.0 |
| stream_rate | 1.0 |
| error_rate | 0.0 |
| avg_ttft_ms | 164.486 |
| p50_total_ms | 167.17 |
| max_total_ms | 220.916 |
| min_total_ms | 121.671 |
| total_tokens | 932 |
| elapsed_ms | 3623.312 |

## 真逐样本结果 (主 17:43 不假装)

| sample_id | benchmark | status | acquired | release_ok | ttft_ms | chunks | total_ms | tokens | reason |
|-----------|-----------|--------|----------|------------|---------|--------|----------|--------|--------|
| MMLU_000 | MMLU | 200 | True | True | 195.0 | 7 | 195.1 | 41 | ok |
| MMLU_001 | MMLU | 200 | True | True | 151.1 | 7 | 151.2 | 37 | ok |
| MMLU_002 | MMLU | 200 | True | True | 177.8 | 8 | 177.9 | 43 | ok |
| MMLU_003 | MMLU | 200 | True | True | 152.2 | 7 | 152.3 | 41 | ok |
| MMLU_004 | MMLU | 200 | True | True | 168.9 | 8 | 169.0 | 46 | ok |
| MMLU_005 | MMLU | 200 | True | True | 146.2 | 7 | 146.3 | 45 | ok |
| MMLU_006 | MMLU | 200 | True | True | 191.8 | 8 | 191.9 | 42 | ok |
| MMLU_007 | MMLU | 200 | True | True | 161.4 | 7 | 161.5 | 41 | ok |
| MMLU_008 | MMLU | 200 | True | True | 175.0 | 10 | 175.1 | 55 | ok |
| MMLU_009 | MMLU | 200 | True | True | 170.8 | 7 | 170.9 | 44 | ok |
| GSM8K_000 | GSM8K | 200 | True | True | 181.4 | 11 | 181.4 | 55 | ok |
| GSM8K_001 | GSM8K | 200 | True | True | 176.6 | 11 | 176.7 | 52 | ok |
| GSM8K_002 | GSM8K | 200 | True | True | 205.3 | 12 | 205.4 | 55 | ok |
| GSM8K_003 | GSM8K | 200 | True | True | 137.1 | 7 | 137.1 | 37 | ok |
| GSM8K_004 | GSM8K | 200 | True | True | 220.9 | 11 | 220.9 | 53 | ok |
| HumanEval_000 | HumanEval | 200 | True | True | 132.7 | 6 | 132.8 | 35 | ok |
| HumanEval_001 | HumanEval | 200 | True | True | 121.6 | 6 | 121.7 | 35 | ok |
| HumanEval_002 | HumanEval | 200 | True | True | 158.3 | 6 | 158.3 | 35 | ok |
| HellaSwag_000 | HellaSwag | 200 | True | True | 167.1 | 6 | 167.2 | 35 | ok |
| HellaSwag_001 | HellaSwag | 200 | True | True | 160.4 | 6 | 160.4 | 35 | ok |
| HellaSwag_002 | HellaSwag | 200 | True | True | 128.0 | 6 | 128.1 | 35 | ok |
| HellaSwag_003 | HellaSwag | 200 | True | True | 139.1 | 6 | 139.1 | 35 | ok |

## V1271 不假装 (主 17:58 + 主 20:46)

- V1271 = integration helper (V1269 stream + V1270 rate limit), NOT new ASI dim.
- 不假装 rate limit 真生效: 超限真 raise V1270RateLimitExceeded.
- 不假装 release 在 stream 失败时被跳过: try/finally 真释放.
- 不假装 deny 计数: 真 deny 真计入 deny_rate.
- 不假装 mock 是真 LLM: [MOCK-LLM] 真标签, X-Mock-Disclosure: true 真标头.
- 不假装 chunk count = 真 LLM token 数 (V1269 metrics 真标注).
- 不假装 V1271 = ASI: V1271 是工具, ASI 守门是更大目标.
