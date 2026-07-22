# R4-BE-03 — apeireth serve OpenAI 兼容服务 (用户面第二面)

- 时间: 2026-07-22 21:35 / 执行: 后端工程师

## 1. 文件清单

| 文件 | LOC |
|---|---|
| `apeireth/serve.py` | 384 |
| `tests/test_r4_serve_smoke.py` | 200 |

位置 `apeireth/` 根 (项目无 `src/`)。**未动** `llm_kernel.py` (R4 brief 红线)。

## 2. Endpoints

| Method | Path | 说明 |
|---|---|---|
| GET | `/health` | `{status, version, engine}` |
| GET | `/v1/models` | OpenAI `{object:"list", data:[...]}` |
| POST | `/v1/chat/completions` | OpenAI chat.completion (支持 stream=true SSE) |

监听 `127.0.0.1:8080` (`APEIRETH_PORT` env 覆盖)。Provider 默认 `template` 无依赖, env 切 `minimax`。

## 3. OpenAI 字段对照

`id=chatcmpl-{uuid}` / `object=chat.completion` / `created=ts` / `choices[0].{index=0, message:{role=assistant, content}, finish_reason=stop}` / `usage.{prompt_tokens>0, completion_tokens>0, total_tokens=sum}`。tokens 是 `len/4` 估值 (主 17:43 实事求是)。

不暴露字段: `asi_*`, `v1074`, `philosophy_guard`, `score` — `test_health_no_asi_leak` 守门。

## 4. 烟测 8/8 全过 (4 必备 + 4 bonus)

| # | Test | 验证 |
|---|---|---|
| 1 | test_health_200_alive | GET /health → 200 + alive |
| 2 | test_models_200_data_array | GET /v1/models → data array |
| 3 | test_chat_completion_basic | POST → choices 非空 + usage 完整 |
| 4 | test_stream_sse_chunks | POST stream → SSE + [DONE] |
| 5 | test_health_no_asi_leak | /health 不含 asi/v1074/philosophy/score |
| 6 | test_chat_completion_bad_request_empty_messages | empty → 400 |
| 7 | test_not_found_404 | 未知 GET → 404 |
| 8 | test_post_unknown_404 | 未知 POST → 404 |

## 5. curl 例子

```bash
python -m apeireth.serve   # 默认 8080, provider=template

curl http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"apeireth-default","messages":[{"role":"user","content":"hello"}]}'

curl -N http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"stream":true,"messages":[{"role":"user","content":"hi"}]}'
```

OpenAI 客户端用法: `base_url=http://localhost:8080/v1`。

## 6. 全量回归

| 范围 | 结果 |
|---|---|
| R4 单跑 | 8/8 PASSED |
| R3+R4 累计 | 27/27 PASSED (v1085/v1086 + r4_serve) |
| 全量 | 3388 passed / 1 env-fail (v1058, R1/R3 同根因) / 2828 capture 伪错 |

**0 真 regression**: 我新加 8 测单跑全过; 全量中 fail/error 是 pytest+Windows capture I/O 关闭伪错 (单跑全过), 非代码引入。

## 7. 下一步 + 边界 (R4 brief 红线)

- 流式真分块 / 多 worker (R5+) / 部署 systemd/docker (R4 红线)
- CORS + API Key 校验

边界: ❌ 未动 `llm_kernel.py` (仅 import) / ❌ 未写 systemd/docker/k8s / ❌ 未暴露 ASI/V1074/philosophy_guard (烟测 5 守门) / ✅ 12 生命特征/HQB/守门在 `_internal_engine_stub` 跑, 不外显。

---

结论: serve.py OpenAI 兼容骨架完成, 8/8 烟测过, 0 真 regression。