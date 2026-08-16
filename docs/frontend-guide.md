# 前端接入指南 — 用现成 Chat 前端连 Apeireth（2026-08-16）

> 主人不会写前端 + 不急着自研 UI → **路线 A：接现成开源 Chat 前端**，我们的
> `apeireth-api` 提供 **OpenAI 兼容端点**（`POST /v1/chat/completions`），主流前端原生支持。
> 产品形态：**先验证伙伴感**（对话可用 → 记忆可视 → 主动送达），界面从简。

## 一、后端：起 Apeireth API（OpenAI 兼容）

```powershell
# 真 LLM 后端（MiniMax-M3）—— 注意: chat 端点总是走真 pipeline, 必须真 key
$env:APEIRETH_API_KEY = (Get-Content apikey-ultra.txt -Raw).Trim()
cargo run -p apeireth-api --example serve          # 默认 :8080
```

> **实测发现（2026-08-16）**：`APEIRETH_LLM_BACKEND=scripted` 只作用于 `/council/advise`
> （LlmProvider 路径）；`/v1/chat/completions` 永远走 5 步 Pipeline 真接 MiniMax ——
> 无 key 时返回 MiniMax 401（链路通但未授权）。前端对接必须配真 key。

验证端点活着（2026-08-16 真机已验证 ✅）：

```powershell
curl http://127.0.0.1:8080/health          # → {"status":"ok","protocols":[...4 协议...]}
curl http://127.0.0.1:8080/v1/models       # → {"object":"list","data":[{"id":"MiniMax-M3",...}]} (2026-08-16 新增)
curl -X POST http://127.0.0.1:8080/v1/chat/completions `
  -H "Authorization: Bearer $env:APEIRETH_API_KEY" `
  -H "Content-Type: application/json" `
  -d '{"model":"MiniMax-M3","messages":[{"role":"user","content":"用一句话介绍你自己"}]}'
# → MiniMax-M3 真回话 ✅ (max_tokens 给小了会被 <think> 吃掉, 建议 ≥200)
```

端点清单：`/health` · `/health/deps` · `/v1/models`（OpenAI 兼容模型列表）
· `/v1/chat/completions`（OpenAI Chat）· `/v1/responses`（OpenAI Responses）
· `/v1/messages`（Anthropic）· `/v1beta/models/{model}:generateContent`（Gemini, schema bug 已知不修）
· V2 端点（/tools/list /memory/episodes /organs /asi /sovereignty 等）。

## 二、前端选型（2026-08-16 调研）

| 前端 | 形态 | 优点 | 缺点 | 适配度 |
|---|---|---|---|---|
| **LobeChat** | 桌面版 (Windows) / Docker / Web | UI 现代, 中文生态, 自定义 OpenAI 兼容 provider, 知识库 | 桌面版需下载安装包 | ⭐⭐⭐ 首选（主人零部署） |
| **NextChat** (ChatGPT-Next-Web) | Docker / Vercel / 静态页 | 最轻量, 配置 1 分钟 | UI 朴素 | ⭐⭐⭐ 次选（最简） |
| **Open WebUI** | Docker（较重） | 功能最全 (RAG/多用户/工具) | 个人场景过重, 部署门槛高 | ⭐⭐ 以后再说 |
| 自研面板 (Vite+React) | — | 完全定制 (每日摘要/记忆侧栏/涌现日志) | 要写前端 | 路线 C, 产品验证后 |

## 三、对接步骤（以 LobeChat 为例）

1. 起 Apeireth serve（§一, 默认 :8080）
2. LobeChat（桌面版或 Docker）→ 设置 → 语言模型 → 添加自定义服务商：
   - **API 地址**：`http://127.0.0.1:8080/v1`
   - **API Key**：任意非空串（本地 serve 校验存在即可；真 key 由 serve 持有）
   - **模型名**：`MiniMax-M3`
3. 开始对话。

> 注意：OpenAI 兼容前端会拉 `GET /v1/models` 列模型 —— 若前端要求该端点而 serve 未提供，
> 在 Apeireth 侧补一个静态 models 列表端点即可（见 §五 待办）。

## 四、"伙伴感"验证清单（产品形态：先验证伙伴感）

按最小前提（「打开就感觉他在、他记得我」）逐项验证：

- [ ] **P0-1 对话可用**：前端 ↔ serve ↔ MiniMax 真回话（§三）
- [ ] **P0-2 记忆感**：serve 接入 memory（recall 注入上下文）→ 问"我上次说的高数错点是什么"他能答
- [ ] **P0-3 今日感**：对话里能问"我今天/昨天干了什么"（daily_summary 数据源已有）
- [ ] **P1-1 主动性**：Lark/飞书实发早安 + 每日摘要（通道 B, 需凭据）
- [ ] **P1-2 记忆可视**：前端侧栏展示"他记得你什么"（路线 C 定制面板）
- [ ] **P1-3 工具透明**：AI 调工具时前端可见（serve 侧透传 tool_calls, 前端原生支持）

## 五、后端待办（serve 升级为"伙伴端点"）

现状：`serve` 是 **stateless chat**（无记忆/无工具/无涌现）。
伙伴感要求 serve 升级为 **companion_serve**：CompanionDaemon + ToolBridge 接进 HTTP，
OpenAI 兼容 + 记忆注入 + 工具桥 + 每日摘要端点。这是路线 C 的前置，也是 P0-2/P0-3 的后端。

```text
✅ 已完成 (2026-08-16):  /v1/models 静态端点 (OpenAI 兼容前端必需)
P0-2/P0-3 需要:  memory injection (已有 build_memory_injection)
                 recall_memory/save_memory (已有)
                 daily_summary (已有数据源)
                 → 缺: 把这些接进 serve 的 OpenAI 兼容路径 (companion_serve)
```
