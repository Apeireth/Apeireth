# V1269 ASI Real LLM Stream 真流式真测 Report

- **Started**: True
- **Base URL**: `http://127.0.0.1:60961/v1`
- **Model**: `MiniMax-M3`
- **Mock disclosed**: True

## Summary (主 17:43 实事求是)

- **n_samples**: 22
- **n_correct**: 2
- **accuracy**: 9.09%

### Stream TTFT (Time-To-First-Token)

- p50: 187.4ms
- p95: 235.2ms
- mean: 191.2ms
- min: 147.5ms
- max: 243.4ms

### Stream Total Time

- p50: 187.4ms
- p95: 235.4ms
- mean: 191.3ms

### Stream Chunks

- p50: 9.0
- mean: 9.4
- total: 207

### Non-stream Latency

- p50: 0.9ms
- p95: 12.6ms
- mean: 4.0ms

- **stream / nonstream ratio**: 47.7955

## V3 哲学守门 (主 17:58 + 主 20:46 不假装)

- [x] `v1269_not_new_dim`
- [x] `v1269_no_asi_v1_claim`
- [x] `v1269_no_phenomenal_claim`
- [x] `v1269_mock_disclosed`
- [x] `v1269_not_newapi_replace`
- [x] `v1269_subprocess_clean`
- [x] `v1269_no_key_leak`
- [x] `v1269_sse_real_parse`

## 真借鉴 References (主 19:33 走在前人肩上)

- v1267-local-mock-2026-08: V1267 ASI Local Mock-LLM Real Loop
- v1076-asi-real-llm-2026-08: V1076 ASI 真外部 LLM 客户端
- v1268-22-samples-2026-08: V1268 ASI 22 真样本真评测
- v1034-asi-real-benchmark-2026-07: V1034 ASI 真 benchmark 真跑
- openai-chat-completions-2023-03: [OpenAI Chat Completions API spec](https://platform.openai.com/docs/api-reference/chat)
- openai-streaming-sse-2023: [OpenAI streaming SSE chunk spec](https://platform.openai.com/docs/api-reference/chat-streaming)
- httpx-iter-lines-2021: [httpx iter_lines for SSE](https://www.python-httpx.org/advanced/streaming/)
- requests-iter-lines-2010: [requests iter_lines stream mode](https://requests.readthedocs.io/en/latest/user/advanced/#streaming)
- aiohttp-streamreader-2014: [aiohttp StreamReader async stream](https://docs.aiohttp.org/en/stable/streams.html)
- sse-w3c-2015: [Server-Sent Events W3C spec](https://html.spec.whatwg.org/multipage/server-sent-events.html)
- tiktoken-bpe-2022: [tiktoken BPE tokenizer](https://github.com/openai/tiktoken)
- litellm-stream-2024: [LiteLLM stream completion abstraction](https://docs.litellm.ai/docs/completion/stream)
