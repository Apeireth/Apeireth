R18 round-02: 12 个产品型 crate 集成测试 (P0 + P1 + P2)

# R18 第 2 阶段: 产品型 crate 集成测试 (122 个新测试)

12 个产品型 crate 集成测试编译通过 (cargo check --tests exit 0):

P0 + P1 (10 crate):
- apeireth-protocol (16): wire_format.rs - 4 协议 OpenAI Chat/Responses + Anthropic + Gemini + Router
- apeireth-api (14): endpoints.rs - axum tower::ServiceExt::oneshot in-memory
- apeireth-tools (19): e2e.rs - file_ops / code_exec / git_ops 真 shell
- apeireth-tool-registry (10): registry.rs - mock Tool impl + 异步 call
- apeireth-tool-runtime (8): parser.rs - VCP <<<[TOOL_REQUEST]>>> 真文本
- apeireth-tool-approval (16): rules.rs - 5 规则真实现 + manager 短路
- apeireth-pipeline (9): pipeline.rs - wiremock 4 协议 e2e
- apeireth-agent (15): agent.rs - Agent + AgentManager CRUD
- apeireth-mcp (9): conformance.rs - JSON-RPC 2.0 + MCP 2025-03-26
- apeireth-memory (6): sqlite.rs - 真 SQLite in-memory

P2 done (2 crate):
- apeireth-vector (13): store.rs - SqliteVecBackend CRUD + 余弦 search
- apeireth-web (13): templates.rs - html_escape + render_error_page XSS 防护

P2 跳过 (3 crate, 留 R20): tui (binary crate 改 lib 入口是侵入式, 其他 AI 已在 tui/tests/ 写 app_state.rs) /
formal (已有完整 inline) / graph (已有 tests/smoke.rs) / bench (criterion benches, 不是 unit test)

加 dev-deps: mcp (tokio + serde_json) / memory (rusqlite) / vector (tempfile + uuid)

**累计 122 个新集成测试 + 12 个 product crate tests 目录**

注: tests/ 在 gitignore, 用 `git add -f` 强制加入
