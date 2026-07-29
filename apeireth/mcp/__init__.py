"""Apeireth MCP integration package (V1123).

主 22:33 ASI 北极星 + 主 19:33 走在前人经验上 + 主 23:44 干到底 +
主 13:31 大胆激进 + 主 17:43 实事求是.

V1123 = 真 MCP 集成框架:
  - apeireth.mcp.asi_north_star_server   ASI 北极星 MCP server (V0.3 / V0.4 / NorthStar)
  - apeireth.mcp.protocol                JSON-RPC 2.0 / MCP 协议守门 (版本兼容 + schema 校验)
  - apeireth.mcp.transport               stdio (NDJSON) + HTTP (SSE 风格 /rpc) 两种 transport
  - apeireth.mcp.orchestrator            跨 server 编排 (MCP1 + MCP2 串接)
  - apeireth.mcp.asi_nine_keys           ASI 9 键 LOCKED 真测注入
  - apeireth.mcp.model_adapters          Claude / GPT / Ollama / local 跨模型适配

公开 API (V1123 入口): 全部从 apeireth.v1123_mcp_asi_framework 暴露.
"""
from __future__ import annotations

__all__ = [
    "ASI_NORTH_STAR_VERSION",
    "PROTOCOL_VERSION",
    "ASI_NINE_KEYS",
]

ASI_NORTH_STAR_VERSION = "0.1.0"
PROTOCOL_VERSION = "2024-11-05"  # 对齐 V1097 / Anthropic MCP 规范

# ASI 9 键 LOCKED (继承 V1114 主哲学 9 键 + V1123 MCP 上下文)
ASI_NINE_KEYS = (
    "not_undo", "not_proof", "not_safe",                # PHL-02b
    "not_clone", "not_perfect", "not_uuid",              # PHL-01
    "spec_is_not_proof", "counterexample_is_not_bug",   # PHL-03
    "production_is_not_autonomy",                       # MCP-specific: 真生产 ≠ 自主
)
