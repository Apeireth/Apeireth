# Apeireth API Reference

> 对齐 `companion_serve` 实际路由（2026-08-18 master）。Base URL: `http://127.0.0.1:8090`（`PORT` 可改）。

## OpenAI 兼容端点

### `GET /v1/models`

模型列表（OpenAI 兼容）。认证：任意非空 `Authorization: Bearer <any>`。

### `POST /v1/chat/completions`

伙伴主链路对话端点（OpenAI Chat Completions 格式）。认证：任意非空 Bearer。

```json
{
  "model": "MiniMax-M3",
  "messages": [{"role": "user", "content": "你好"}]
}
```

响应含 `x_apeireth` 扩展头，报告本轮的注入特征：

```json
"x_apeireth": {
  "continuity": "companion-main",
  "features": ["memory_injection", "today_summary", "tool_bridge", "daemon_resident",
               "memory_extractor", "l0_identity", "l1_essential_story"],
  "tool_rounds": 1,
  "tools_executed": []
}
```

行为：每轮注入 L0 Identity + L1 Essential 常驻块 + 记忆排名 + 图谱 + 偏好 + 今日 + 成长（预算截断）；工具调用走 `<<<[TOOL_REQUEST]>>>` 解析 → 审批 → 执行；`max_tokens` 可由 `APEIRETH_MAX_TOKENS` 覆盖。

## 伙伴专属端点

### `GET /health`

健康检查 + 能力清单：

```json
{
  "status": "ok",
  "service": "apeireth-companion-serve-v4",
  "version": "1.2.0",
  "features": ["persistent_memory", "daemon_resident", "dream_llm_summarizer",
               "utterance_llm", "constitution_llm_judicator", "memory_injection",
               "today_summary", "tool_bridge_all", "openai_compat", "companion_app",
               "l0_identity", "l1_essential_story"]
}
```

### `GET /v1/apeireth/approval-requests`

待批授权队列（工具执行需主人批准时进入此队列）：

```json
{
  "count": 2,
  "requests": [{"id": "apreq-...", "tool": "FileOperator",
                "args_preview": "{\"op\":\"read\",\"path\":\"Cargo.toml\"}",
                "reason": "需要主人批准 (权限洋葱)", "created_at": 1786985883}],
  "note": "主人批准后, 对话里让本座重试即可"
}
```

### `POST /v1/apeireth/grant`

显式扩权：`{"tool": "FileOperator", "hours": 24}`（或启动时 `APEIRETH_GRANT=FileOperator:24`）。

### `GET /v1/apeireth/events` / `POST /v1/apeireth/test-event`

SSE 主动推送事件流 / 测试事件。

## 面板

| 端点 | 说明 |
|---|---|
| `GET /panel` | Web 面板 v2（会话/记忆/图谱/授权/审计，只读真接口）|
| `GET /panel/:asset` | 面板静态资源 |

## 工具协议（`<<<[TOOL_REQUEST]>>>` marker）

LLM 输出中的工具调用格式（与 `apeireth-tool-runtime::ToolCallParser` 对齐）：

```
<<<[TOOL_REQUEST]>>>
tool_name:<<<FileOperator>>>
op:<<<read>>>
path:<<<Cargo.toml>>>
<<<[END_TOOL_REQUEST]>>>
```

流程：解析 → ApprovalManager 5 规则检查 → 需审批则入队 → 执行（schema 校验 + guardrail）→ 记录 + observer 沉淀。

## 认证与安全

- 伙伴端点：任意非空 Bearer（本地伙伴场景；生产前建议接真实令牌，`APEIRETH_MASTER_TOKEN` 为预留口）
- 出站：所有 HTTP 请求过 `egress` 默认拒绝白名单 + SHA-256 审计链（接入方显式启用）
- 审批：高危工具默认需主人批准（权限洋葱）
