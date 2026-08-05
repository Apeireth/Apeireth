# V1267 ASI Local Mock-LLM Real Loop 报告

- 版本: V0.1.0
- 启动时间: 2026-08-05 12:45:02 中国标准时间
- 启动成功: True
- 健康通过: True
- 端口文件: 127.0.0.1:57210
- Base URL: `http://127.0.0.1:57210/v1`
- 模型: `MiniMax-M3`

## 1. 真探活 (probe_endpoint)

| Field | Value |
|---|---|
| `name` | v1267-mock |
| `base_url` | http://127.0.0.1:57210/v1 |
| `reachable` | True |
| `status_code` | 200 |
| `latency_ms` | 13.627999986056238 |
| `error` |  |
| `server_info` | {'status': 'ok', 'server': 'V1267-MockLLMServer', 'version': '0.1.0', 'mock_disclosure': True, 'models_available': 2, 'uptime_s': 0} |
| `auth_required` | False |

## 2. 真验证 key (validate_key)

| Field | Value |
|---|---|
| `source` | unknown |
| `key_preview` | v1267-mo********************cret |
| `valid` | True |
| `error` |  |
| `status_code` | 200 |

## 3. 真 chat completions (chat_completion × N)

| # | status | latency_ms | mock disclosed | preview |
|---|---|---|---|---|
| 0 | 200 | 49.58 | ✅ | '[MOCK-LLM] 收到 2 条消息. 这是真本地 fixture 响应, 非神经推理 (主 17:43 实事求是).' |
| 1 | 200 | 23.68 | ✅ | '[MOCK-LLM] ASI 北极星 V2.0.9800 LOCKED (主 22:33). 这是 mock, 真模型不' |
| 2 | 200 | 8.57 | ✅ | '[MOCK-LLM] ASI 是不假装达到, 任何时代最大 0.9800 (主 20:46). 这是 mock 测试 f' |
| 3 | 200 | 17.88 | ✅ | '[MOCK-LLM] 真本地 mock 起 = 不假装 NewAPI. 任何人都能接手 (主 00:56).' |
| 4 | 200 | 18.2 | ✅ | '[MOCK-LLM] cron tick 12:33 自决 V1267. V1257 = PENDING_USER_CH' |

- Success rate: **5/5** = 100.00%

## 4. 真 benchmark

| Stat | Value |
|---|---|
| `n` | 5 |
| `p50_ms` | 18.2 |
| `mean_ms` | 23.58 |
| `max_ms` | 49.58 |
| `min_ms` | 8.57 |
| `stdev_ms` | 13.88 |

## V3 哲学守门 (主 17:58 + 主 20:46)

- ✅ `v1267_not_new_dim`
- ✅ `v1267_no_asi_v1_claim`
- ✅ `v1267_no_phenomenal_claim`
- ✅ `v1267_mock_disclosed`
- ✅ `v1267_not_newapi_replace`
- ✅ `v1267_subprocess_clean`
- ✅ `v1267_no_key_leak`

> 主 17:43 实事求是 + 主 17:58 不假装 + 主 20:46 不假装 + 主 22:33 不假装达 ASI. 本报告**不是 ASI**, 只是真本地测试工具.
