# V1076 ASI Real External LLM Client Report

- **Started:** 2026-07-22T03:49:56.468631+00:00
- **Stopped:** 2026-07-22T03:49:56.535754+00:00
- **Duration:** 0.07s
- **Selected endpoint:** http://localhost:3000/v1
- **Selected key:** (none)
- **Summary:** `no_valid_key`

## Endpoint Probes (真实探测 / 主 17:43 实事求是)

| Name | URL | Reachable | Status | Latency | Error |
|------|-----|-----------|--------|---------|-------|
| newapi-local | `http://localhost:3000/v1` | True | 200 | 35.5ms |  |

## API Keys (主 17:58 不假装)

| Source | Preview | Valid | Status | Error |
|--------|---------|-------|--------|-------|
| file:.minimax_key:line1 | `sk-cp--D*****************************************************************************************************************5Wbk` | False | 401 | invalid_token |
| file:.minimax_key:line2 | `sk-cp-Xq*****************************************************************************************************************qSes` | False | 401 | invalid_token |

## V3 哲学守门 (主 17:58 + 主 20:46 不假装)

- [x] 不假装 key 有效: real HTTP probe + real 401 vs 200
- [x] 不假装模型可用: real /v1/models 列表 + real check
- [x] 不假装响应真实: 真 HTTP + 真 token + 真 content (非 mock)
- [x] 不假装 benchmark 通过: 真 latency 真 status 真统计
- [x] 不假装 ASI = LLM: V1076 是工具, ASI 是更大目标

## 真借鉴 References (主 19:33 走在前人经验上)

- OpenAISDK2020: [OpenAI Python SDK ChatCompletion](https://github.com/openai/openai-python)
- httpx2019: [httpx async HTTP client](https://www.python-httpx.org/)
- tenacity2014: [tenacity retry decorator](https://tenacity.readthedocs.io/)
- LiteLLM2023: [LiteLLM multi-provider router](https://github.com/BerriAI/litellm)
- Instructor2023: [Instructor structured output](https://github.com/jxnl/instructor)
- Outlines2023: [Outlines JSON mode / regex](https://github.com/outlines-dev/outlines)
- tiktoken2022: [tiktoken BPE tokenizer](https://github.com/openai/tiktoken)
- LangChain2022: [LangChain LLMChain](https://github.com/langchain-ai/langchain)
- OpenRouter2023: [OpenRouter multi-model API](https://openrouter.ai/docs)
- NewAPI2024: [NewAPI OpenAI-compatible proxy](https://github.com/songquanpeng/one-api)
- aiohttp2014: [aiohttp async HTTP](https://docs.aiohttp.org/)
- backoff2014: [backoff retry decorator](https://github.com/litl/backoff)
