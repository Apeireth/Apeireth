# Apeireth-rust 后端能力清单 (给 AI 看的)

> **目的**: 让 Claude / GPT / Gemini 等 LLM 在对话里一眼知道我们这后端能干啥、能怎么调。
> **维护**: 后端每加端点/能力时同步更新。
> **适用**: apeireth-api daemon + TUI/桌面/web 前端 (frontend-agnostic)

---

## 1. 进程 & 端点总览

| 项 | 值 |
| --- | --- |
| 进程 | `apeireth-api.exe` (Windows) / `apeireth-api` (Unix) |
| 默认端口 | `8080` |
| 默认监听 | `127.0.0.1:8080` (本机) |
| 协议 | HTTP + Server-Sent Events (SSE) |
| OpenAI 兼容度 | 100% (`/v1/chat/completions` / `/v1/models` 字段对齐) |
| 上游默认 | `https://api.minimaxi.com` (MiniMax-M3 系列) |
| 鉴权 | daemon 持 `APEIRETH_API_KEY`, 前端可无 key 直连 |
| 健康检查 | `GET /health` → `200 OK` `{ok: true, daemon: "apeireth-api", version: "..."}` |

---

## 2. 4 个 OpenAI-兼容协议端点 (LlmProvider 抽象)

`apeireth-api` 通过 `LlmProvider` trait 屏蔽 OpenAI / Anthropic / Gemini, 暴露 4 个等价端点:

| 协议 | 路径 | 用途 | 流式 |
| --- | --- | --- | --- |
| OpenAI Chat | `POST /v1/chat/completions` | OpenAI 标准 chat | ✅ SSE |
| OpenAI Responses | `POST /v1/responses` | OpenAI 新版 responses API | ✅ SSE |
| Anthropic Messages | `POST /v1/messages` | Claude API 协议 | ✅ SSE |
| Gemini generateContent | `POST /v1beta/models/{model}:generateContent` | Gemini 协议 (含 SSE: `?alt=sse`) | ✅ SSE |

请求 body 跟上游完全一致, response 也按上游协议直接转发 (不重新包装)。
**模型字段被忽略** (proxy transparent) — 想换模型改 `apeireth-api` 启动时的 `--model` / 配置, 不用前端调。

### 2.1 流式 SSE 直通 (R25 修)

`stream=true` 时, daemon **不**等上游返回完才转发, 而是按上游 chunk 实时往客户端写:
- OpenAI 协议: `data: {json}\n\n` 事件流, 末条 `data: [DONE]\n\n`
- Anthropic 协议: `event: message_start\ndata: {...}\n\nevent: content_block_delta\n...`
- Gemini 协议: `data: {...}\n\n` (无 `[DONE]`)

前端拿到 `data: {...}` 后按协议解析增量 delta, 拼到 UI。

---

## 3. R19 认知循环元端点 (Council + Verdict)

后端不只是透传 LLM, 还跑 R19 哲学层的 5 步循环:

| 端点 | 路径 | 用途 |
| --- | --- | --- |
| Council | `POST /council/advise` | 9 器官 (5 senses + 4 actors) 集体审议, 给出建议向量 |
| Verdict | `POST /verdict` | R19 dual-process 仲裁 (System 1 快路径 + System 2 慢路径), 返 transferability / verdicts / cycle# |

请求体跟 `CouncilRequest` / `DecisionRequest` 一致, 返 `CouncilAdvice` / `DomainCheckResult` (serde JSON)。

---

## 4. 配置与启动

```bash
# 启动 daemon (前台, 8080 端口)
apeireth-api

# 启动 daemon (后台, 写日志到文件)
apeireth-api --log-file apeireth-api.log &

# 改端口 / 改模型
apeireth-api --port 9090 --model MiniMax-M3-thinking

# 环境变量 (可选)
export APEIRETH_API_KEY=...      # 上游 key (必需)
export APEIRETH_API_BASE=https://api.minimaxi.com  # 上游 base
export APEIRETH_PORT=8080
```

启动后 `GET /health` 应回 200。LLM 调用前最好先探一下, 避免前端以为连上了其实 daemon 挂了。

---

## 5. 前端怎么告诉 AI 后端能干啥

把这文档路径写到前端 on-boarding / system prompt 里:

- **TUI**: `crates/apeireth-tui/src/onboarding.rs` (R21 G-1 加翻译时塞进去)
- **桌面** (tauri): `apps/apeireth-desktop/src/onboarding/`
- **web**: `apps/apeireth-web/src/onboarding/`

或者更激进 — 直接把摘要塞进 system prompt 的固定段落:

```text
## 后端: apeireth-api (本地 127.0.0.1:8080)
支持的协议: OpenAI Chat / OpenAI Responses / Anthropic Messages / Gemini generateContent (4 个等价端点).
流式: SSE 直通, 实时增量.
特殊能力: Council / Verdict (R19 认知循环, 5 步审议 + 双路径仲裁).
健康检查: GET /health.
上游: 代理 MiniMax-M3 / MiniMax-M3-thinking (OpenAI 协议兼容).
鉴权: 前端无需 API key (daemon 持有).
模型字段被忽略: 想换模型改 daemon 启动参数, 不在前端调.
```

---

## 6. 失败模式 (AI 应当知道)

| 现象 | 原因 | 前端处理 |
| --- | --- | --- |
| `GET /health` 502 / 不通 | daemon 没启动 | 启动 `apeireth-api`, status bar 显示 ✗ |
| LLM 调 401 | 上游 key 失效 / 配错 | 改 `APEIRETH_API_KEY`, 重启 daemon |
| LLM 调 429 | 上游限流 | 退避 30s 后重试 (已在 `LlmProvider::complete_with_retry`) |
| LLM 调 5xx | 上游崩 | 退避 1s/3s/10s 三次, 后报"上游不可用" |
| 流式中途断 | 网络抖 | 自动重连一次 (幂等性靠 `n` 字段), 否则报用户 |

---

## 7. 当前版本支持的具体能力

- ✅ 多 LLM 协议 (4 个) + 上游代理
- ✅ 流式 SSE 直通 (R25 修)
- ✅ R19 认知循环 (Council + Verdict)
- ✅ 9 organ system (按 spirit 蓝图 v1: consciousness/perception/cognition/motivation/life-force/memory/value/graph-primitive/companion, R23+ 鲜本实装; 3 crate 是 transparent re-export 到 perception/memory/motivation per ADR-0031)
- ✅ TUI 9 organ (R11 LOCKED 旧名: heart/brain/hand/eye/ear/memory/voice/body/mind, ASCII 艺术 + i18n key 0 可改; 桥接表见 `crates/apeireth-tui/src/organ/bridge_table.rs` + ADR-0028)
- ✅ ASI 哲学层 (5 Gap 闭环, V1324)
- ✅ Memory (S3 / MongoDB / Disk LRU provider)
- ✅ TUI (ratatui, 5 nav, char-level 选区)
- ⏳ Web 前端 (开发中)
- ⏳ 桌面 (tauri, 计划中)
