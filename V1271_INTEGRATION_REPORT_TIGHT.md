# V1271 ASI Stream + Rate Limit Integration Report

- V1271 version: `0.1.0` (build 2026-08-05)
- Note: V1271 = ASI Stream + Rate Limit 真生产集成 (V1269 stream + V1270 rate limit). 不是新 ASI dim, 是工具.
- Base URL: `http://127.0.0.1:58320/v1`
- Masked key: `v1271-ti*************9876` (主 17:58 不假装 key 真泄露)

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
    "requests_per_minute": 3,
    "tokens_per_minute": 600,
    "max_concurrent": 1,
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
| n_allowed | 3 |
| n_denied | 19 |
| n_streamed | 3 |
| n_errors | 0 |
| deny_rate | 0.8636 |
| stream_rate | 0.1364 |
| error_rate | 0.0 |
| avg_ttft_ms | 171.745 |
| p50_total_ms | 172.889 |
| max_total_ms | 176.354 |
| min_total_ms | 166.135 |
| total_tokens | 121 |
| elapsed_ms | 515.953 |

## 真逐样本结果 (主 17:43 不假装)

| sample_id | benchmark | status | acquired | release_ok | ttft_ms | chunks | total_ms | tokens | reason |
|-----------|-----------|--------|----------|------------|---------|--------|----------|--------|--------|
| MMLU_000 | MMLU | 200 | True | True | 166.1 | 7 | 166.1 | 41 | ok |
| MMLU_001 | MMLU | 200 | True | True | 176.3 | 7 | 176.4 | 37 | ok |
| MMLU_002 | MMLU | 200 | True | True | 172.8 | 8 | 172.9 | 43 | ok |
| MMLU_003 | MMLU | 429 | False | False | 0.0 | 0 | 0.0 | 0 | denied:rpm_exceeded(4>3) |
| MMLU_004 | MMLU | 429 | False | False | 0.0 | 0 | 0.0 | 0 | denied:rpm_exceeded(4>3) |
| MMLU_005 | MMLU | 429 | False | False | 0.0 | 0 | 0.0 | 0 | denied:rpm_exceeded(4>3) |
| MMLU_006 | MMLU | 429 | False | False | 0.0 | 0 | 0.0 | 0 | denied:rpm_exceeded(4>3) |
| MMLU_007 | MMLU | 429 | False | False | 0.0 | 0 | 0.0 | 0 | denied:rpm_exceeded(4>3) |
| MMLU_008 | MMLU | 429 | False | False | 0.0 | 0 | 0.0 | 0 | denied:rpm_exceeded(4>3) |
| MMLU_009 | MMLU | 429 | False | False | 0.0 | 0 | 0.0 | 0 | denied:rpm_exceeded(4>3) |
| GSM8K_000 | GSM8K | 429 | False | False | 0.0 | 0 | 0.0 | 0 | denied:rpm_exceeded(4>3) |
| GSM8K_001 | GSM8K | 429 | False | False | 0.0 | 0 | 0.0 | 0 | denied:rpm_exceeded(4>3) |
| GSM8K_002 | GSM8K | 429 | False | False | 0.0 | 0 | 0.0 | 0 | denied:rpm_exceeded(4>3) |
| GSM8K_003 | GSM8K | 429 | False | False | 0.0 | 0 | 0.0 | 0 | denied:rpm_exceeded(4>3) |
| GSM8K_004 | GSM8K | 429 | False | False | 0.0 | 0 | 0.0 | 0 | denied:rpm_exceeded(4>3) |
| HumanEval_000 | HumanEval | 429 | False | False | 0.0 | 0 | 0.0 | 0 | denied:rpm_exceeded(4>3) |
| HumanEval_001 | HumanEval | 429 | False | False | 0.0 | 0 | 0.0 | 0 | denied:rpm_exceeded(4>3) |
| HumanEval_002 | HumanEval | 429 | False | False | 0.0 | 0 | 0.0 | 0 | denied:rpm_exceeded(4>3) |
| HellaSwag_000 | HellaSwag | 429 | False | False | 0.0 | 0 | 0.0 | 0 | denied:rpm_exceeded(4>3) |
| HellaSwag_001 | HellaSwag | 429 | False | False | 0.0 | 0 | 0.0 | 0 | denied:rpm_exceeded(4>3) |
| HellaSwag_002 | HellaSwag | 429 | False | False | 0.0 | 0 | 0.0 | 0 | denied:rpm_exceeded(4>3) |
| HellaSwag_003 | HellaSwag | 429 | False | False | 0.0 | 0 | 0.0 | 0 | denied:rpm_exceeded(4>3) |

## V1271 不假装 (主 17:58 + 主 20:46)

- V1271 = integration helper (V1269 stream + V1270 rate limit), NOT new ASI dim.
- 不假装 rate limit 真生效: 超限真 raise V1270RateLimitExceeded.
- 不假装 release 在 stream 失败时被跳过: try/finally 真释放.
- 不假装 deny 计数: 真 deny 真计入 deny_rate.
- 不假装 mock 是真 LLM: [MOCK-LLM] 真标签, X-Mock-Disclosure: true 真标头.
- 不假装 chunk count = 真 LLM token 数 (V1269 metrics 真标注).
- 不假装 V1271 = ASI: V1271 是工具, ASI 守门是更大目标.
