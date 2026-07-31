# R11 Automation — 真实 provider / 离线 deterministic 双轨测试

> 自动化测试工程师 · R11 任务
> 报告日期: 2026-07-30
> 任务 ID: e3a8d0e0-77f3-4e31-b1d2-acf55e654614
> 主哲学 anchor: 主 17:43 实事求是 + 主 17:58 不假装 + 主 19:33 走在前人经验上 + 主 22:33 ASI 北极星 + 主 23:44 干到底 + 主 00:56 任何人都能接手

## 1. 任务摘要

补一组稳定、可维护、可在 CI 中运行的双轨测试，覆盖：

- **Live track**: 真实 OpenAI-compatible HTTP 路径（MiniMax-M3 / OpenAI gpt-4o-mini 同协议），含 SSL/HTTP 错误、provider down、partial 响应、provider version mismatch。
- **Offline track**: 完全无网络的 deterministic 路径（`OfflineMockEngine` / `force_mock`），保证 0 availability 仍可重现。
- **Dashboard**: V1136 → Dashboard 渲染对失败与版本不一致的真实透传，不掩盖、不升级。

附带修复的最小缺陷（详见 §5）。

## 2. 新增测试文件

| 文件 | 行数 | 角色 | 跳过条件 |
|------|------|------|---------|
| `tests/test_r11_automation.py` | 14 + 1 opt-in skip | R11 双轨 | `test_opt_in_live_provider` 仅当 `R11_LIVE_PROVIDER` / `R11_LIVE_BASE_URL` / `R11_LIVE_MODEL` / `R11_LIVE_CREDENTIAL` 同时存在才执行；否则 `pytest.skip`（CI 默认安全）。 |

测试按主 17:43 实事求是分四组：

1. `TestLiveCompatibleWirePath` — local real HTTP（`BaseHTTPRequestHandler` + `ThreadingHTTPServer`），参数化 MiniMax / OpenAI 路径。
2. `TestProviderDownAndOfflineBoundary` — provider down 走真 socket connect failure + offline deterministic 路径。
3. `TestPartialAndVersionBoundaries` — partial choices / provider version mismatch 在 live + fallback 双侧。
4. `TestDashboardRenderingBoundaries` — `V1136Result` 真实失败状态被 dashboard 反映，缓存对失败变化敏感。
5. `TestOptInLiveProvider` — 显式 env 注入的真实 MiniMax / OpenAI 用例，默认 skip。

## 3. 运行结果

| 套件 | 通过 | 失败 | 跳过 | 备注 |
|------|------|------|------|------|
| `tests/test_r11_automation.py` | 14 | 0 | 1 | opt-in live |
| `tests/test_v1084_asi_real_llm_inference.py` | 59 | 0 | 0 | 含本轮修复的 2 个旧断言 |
| `tests/test_v1136_asi_v05_3dim_real_measurement.py` | 32 | 0 | 0 | baseline |
| `tests/test_v1136_dashboard_render.py` | 34 | 0 | 0 | baseline + 本轮修复的 dimensions 字段一致性 |
| `tests/test_v1128_real_model_adapter.py` | 58 | 0 | 1 | 既有的 ollama 环境 skip |
| **合计 (R11 涉及 5 个套件)** | **197** | **0** | **2** | 47.1s |

命令:

```bash
python -m pytest -q \
  tests/test_v1084_asi_real_llm_inference.py \
  tests/test_v1136_asi_v05_3dim_real_measurement.py \
  tests/test_v1136_dashboard_render.py \
  tests/test_v1128_real_model_adapter.py \
  tests/test_r11_automation.py
# 197 passed, 2 skipped in 55.53s
```

## 4. 测试覆盖矩阵

| 场景 | Live 路径 | Offline 路径 | Dashboard 渲染 |
|------|-----------|--------------|----------------|
| **OK 完整响应** | `test_local_real_http_path_is_not_offline` × 2 provider | `test_force_mock_never_calls_live_client_and_is_reproducible` | `test_dashboard_reports_real_score_failures_and_dimensions` |
| **SSL 错误** | `test_ssl_error_is_an_explicit_transport_error` | n/a (offline 旁路) | n/a |
| **HTTP 5xx/4xx** | `test_http_503_is_an_explicit_provider_error` | n/a | n/a |
| **Provider down** | `test_provider_down_fallback_preserves_transport_evidence`<br>`test_provider_down_without_fallback_is_not_success` | n/a | n/a |
| **Partial response** | `test_http_200_without_choices_is_partial_not_ok`<br>`test_partial_response_can_fallback_but_keeps_reason` | n/a | n/a |
| **Version mismatch** | `test_provider_api_version_mismatch_is_explicit`<br>`test_version_mismatch_fallback_is_still_marked_mock` | n/a | n/a |
| **Deterministic 重复性** | n/a | `test_offline_engine_stays_deterministic_without_wall_clock_assertion` | n/a |
| **Cache 不伪造失败** | n/a | n/a | `test_dashboard_cache_does_not_hide_changed_failure_state` |
| **真实 live provider** | `test_opt_in_live_provider` (skip 默认) | n/a | n/a |

## 5. 修复的最小缺陷

> 主 17:43 实事求是: 真实存在的 bug, 不为 KPI 掩盖。

### 5.1 `apeireth/v1084_asi_real_llm_inference.py`

1. **HTTPError/SSL/OSError 统一为显式 transport_error** — 之前 `URLError` 与 `HTTPError` 在 `except` 中合并，且 `_status` 字段缺失，导致 `error` 信息不带状态标签，下游审计难以区分"重试可恢复"与"协议错误"。修复后:
   - `HTTPError` → `status="http_error"`
   - `ssl.SSLError` / `URLError` / `socket.timeout` / `TimeoutError` / `OSError` → `status="transport_error"`
   - 错误信息带状态前缀，如 `transport_error: URLError: ...`
2. **新增 `LLMEndpointConfig.expected_api_version` 与 `LLMHTTPClient._validate_response`** — 校验 choices / message / content 必填项，失败返回 `partial`；如 provider 响应携带 `api_version`/`version` 与配置期望不一致，返回 `version_mismatch`。
3. **重试循环改为"transport 错误才重试"** — partial / version_mismatch 不再触发重试，避免无效请求轰炸 provider。
4. **mock fallback 保留原始 status** — `provider_error` 不再被覆盖为 "HTTP failed"，而是真实透传 `status` 与 `_error`，确保审计链可追溯。
5. **仅 `ok`/`mock` 暴露 completion text** — partial / version_mismatch / transport_error 时 `text=""`，避免把不完整响应冒充成可用输出。

### 5.2 `apeireth/v1136_dashboard_render.py`

1. **`dimensions` 修正为 18** — 渲染表实际 18 行（3 V1136 真测 + 15 复用 V1130 维度）；之前返回 `len(DASHBOARD_DIMENSIONS)+3 = 21` 与渲染不一致。
2. **failures 计数取 `max(failed, len(failures))`** — 保证 `failures` 列表内容不会因 `failed` 字段为 0 而在 dashboard 上显示为零。

### 5.3 `tests/test_v1084_asi_real_llm_inference.py`

更新两条旧断言以对齐新契约：

- `test_http_failure_falls_back_to_mock`: 改判 `transport_error in error` 与 `mock fallback used`。
- `test_http_failure_no_fallback`: 改判 `status == "transport_error"`。

## 6. CI 集成要点

- 默认零网络：`tests/test_r11_automation.py` 中所有用例都基于本地 `ThreadingHTTPServer` 或纯 deterministic 路径；不假设任何外网端点。
- 真实 live provider 触发条件：环境变量四件套（`R11_LIVE_PROVIDER` ∈ {`minimax`,`openai`}、`R11_LIVE_BASE_URL`、`R11_LIVE_MODEL`、`R11_LIVE_CREDENTIAL`）同时存在且非空。CI 可在 nightly 或受控环境注入，普通 PR 不受影响。
- 错误绝不伪装：fail-fast 错误（如 HTTP 503、SSL 错误、partial 响应、version mismatch）一律暴露为 `error` 字段，原 `status` 不被降级为 `ok`。
- 离线 fallback 必带 `mock fallback used` 字串，便于审计检索。

## 7. 自检与稳定性

- `force_mock` 路径在 `assertionerror` 防止 `http.call` 触发：测试断言 HTTP 客户端未被触碰。
- `OfflineMockEngine(latency_ms=0.0)` + 同 prompt 多次调用：id 与 content 完全一致。
- 失败注入用 `dataclasses.replace` 改 V1136Result，避开真测的耗时与稳定性问题。
- 全部测试共用 module-scoped `v1136_result` fixture（已在 `test_v1136_dashboard_render.py` 验证），避免重复跑真测。
- 进程级隔离：每个 `_StubHTTPServer` 用 daemon thread 启动，fixture teardown `shutdown + server_close + thread.join(timeout=2)`。

## 8. 验收清单

- [x] 真实 provider 路径在本地 OpenAI-compatible HTTP 端点真跑
- [x] 离线 deterministic 路径完全无网且可重复
- [x] SSL 错误被显式归类
- [x] HTTP 503 等 4xx/5xx 被显式归类
- [x] provider down 走 fallback 保留证据，无 fallback 不冒充成功
- [x] partial response (缺 choices / content) 不被计为 ok
- [x] provider version mismatch 显式状态，fallback 仍标 mock
- [x] dashboard 渲染透传失败与版本不一致，缓存不掩盖
- [x] 修复的最小缺陷已通过 R11 新测试与既有 5 个相关套件
- [x] 全量 5 套件 197 passed / 2 skipped / 0 failed

## 9. 关键文件清单

| 路径 | 用途 |
|------|------|
| `apeireth/v1084_asi_real_llm_inference.py` | Real LLM Inference Adapter (Live + Offline 双轨) |
| `apeireth/v1136_dashboard_render.py` | V1136 → Dashboard 渲染 (p50/p95/p99 + cache) |
| `apeireth/v1136_asi_v05_3dim_real_measurement.py` | V0.5 3-Dim 真测引擎 (主 17:43 实事求是) |
| `apeireth/v1128_real_model_adapter_w2.py` | W2 真实 model adapter (有本地 OpenAI stub 范式) |
| `tests/test_r11_automation.py` | R11 新增 14 + 1 opt-in 用例 |
| `tests/test_v1084_asi_real_llm_inference.py` | baseline + 本轮 2 条断言升级 |
| `tests/test_v1136_asi_v05_3dim_real_measurement.py` | V1136 真测基线 |
| `tests/test_v1136_dashboard_render.py` | Dashboard 渲染基线 |
| `tests/test_v1128_real_model_adapter.py` | W2 真实 provider 路径基线 |

## 10. 后续可加项 (建议而非本轮必做)

- 把 `_StubHTTPServer` 抽到 `tests/_helpers/stub_http.py` 给更多套件复用。
- 增加 streaming SSE 的 live 路径覆盖（v1084 当前为简化版，未支持 stream）。
- 把 `R11_LIVE_*` 接入 CI nightly job，并配 `R11_LIVE_CREDENTIAL` 加密 secret。

> 报告完。

## 11. 恢复续跑加固（全文读取后，2026-07-30）

> 本节为增量证据；§3 的 197 passed / 2 skipped 保留为初次交付历史，本节结果为当前终态。

### 11.1 Omnibus 全文读取与测试约束

已从第 1 行连续读到第 6002 行，完整读取 `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md`，没有用关键词或摘要替代正文。与 R11 自动化直接相关的约束为：

- provider/model 与 harness 能力必须分界，deterministic fallback 不得冒充 live provider；
- HTTP/SSL/provider down/partial/version mismatch 必须保留可审计失败语义；
- dashboard 必须展示真实失败与 V1136 分数，不能因缓存或占位数据隐藏失败；
- 历史累计测试数和 snapshot 会随采样时点漂移，CI 应断言契约与内部一致性，不硬编码旧全局计数；
- mock 单测可能掩盖真实安装/协议缺陷，因此默认 CI 同时保留真实本地 socket/HTTP wire path。

### 11.2 最小加固

1. provider-down 用例不再硬编码 `127.0.0.1:1`；fixture 让 OS 分配临时端口并保持 socket 已绑定但不监听，既走真实 connection-refused 路径，又避免 CI runner 端口碰撞。
2. 新增真实 HTTP 503 `max_retries=2` 用例，断言只收到 1 次请求；修复 `LLMHTTPClient.call`，只有 `transport_error` 才重试，HTTP/JSON/partial/version 等确定性协议错误直接返回。
3. HTTP 200 但 assistant content 为空或纯空白时标记 `partial`，不再返回 `ok`。
4. HTTP 200 顶层 JSON 为数组等非对象时转换为可审计 `partial`，不再泄出 `AttributeError`；错误只记录响应类型，不复制异常 provider payload。
5. dashboard 用例从 emoji/完整标题的脆弱字符串断言改为语义行断言（provider 行含 `failed`、guard 行布尔值为 `False`）。

### 11.3 续跑结果

```text
python -m pytest tests/test_r11_automation.py -q
17 passed, 1 skipped in 7.65s

python -m pytest -q \
  tests/test_v1084_asi_real_llm_inference.py \
  tests/test_v1136_asi_v05_3dim_real_measurement.py \
  tests/test_v1136_dashboard_render.py \
  tests/test_v1128_real_model_adapter.py \
  tests/test_r11_automation.py
200 passed, 2 skipped in 49.20s
```

跳过项仍为显式 opt-in live provider 与既有 Ollama 环境用例；默认 CI 无外网依赖、0 failed。
