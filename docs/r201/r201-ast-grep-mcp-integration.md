# R201 ast-grep MCP 集成 (接续 R193)

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R201
> **日期**: 2026-08-13
> **来源**: R193 ast_grep 子模块 + R181 调研
> **状态**: 实施完成, 56/56 单测全过

---

## 0. 目标

R193 写了 ast_grep 子模块, 56 个单测中 8 个新. R201 把 ast-grep 暴露给 MCP, 让 LLM agent 可以通过 MCP 协议调用 AST 级别搜索.

---

## 1. 实施

### 1.1 McpTool enum 扩展

加 McpTool::AstGrepSearch 变体 (第 11 个 tool). 同时:
- 
ame() 加 arm
- ll() 数组加 1
- MCP_TOOL_COUNT = 11
- 	ools_list_returns_10 → 	ools_list_returns_11
- 	ool_count_is_10 → 	ool_count_is_11 (断言 11)

### 1.2 ast_grep_search arm in handle()

加 MCP 	ools/call 处理 arm:
- 参数: pattern (必需), path (默认 "."), lang (可选)
- 调用 AstGrepSearcher::search(path, pattern, lang)
- 成功: 列出 ile:start-end text 每行
- 失败: 返回错误文本 (isError: true), 不 panic
- 找不到 binary / 无匹配: 友好降级

### 1.3 新测试

st_grep_search_handles_missing_binary:
- 验证 graceful 处理 (不 panic, 返回非空 text)
- 当 ast-grep binary 不存在时仍然返回有效响应

---

## 2. 0 触碰声明

- 3 不可变脊柱: 0 触碰
- workspace.version 1.2.0: 0 改
- McpTool 公开 enum 变体加 1 (必要, 新功能)
- 现有 10 个 tool 行为: 0 改
- ast_grep 子模块 (R193): 0 改 (本 R 仅消费其 API)

---

## 3. 测试 (56/56 pass)

新增 1 测试:
- st_grep_search_handles_missing_binary — graceful handling

更新测试:
- 	ool_count_is_10 → 	ool_count_is_11
- 	ools_list_returns_10 → 	ools_list_returns_11

总计: 55 → 56 测试 (+1 新增)

---

## 4. 不假装 (O-5)

- 依赖系统 ast-grep binary (cargo install ast-grep --locked)
- 无 binary 时 graceful 返回 \"(no matches or ast-grep unavailable)\"
- subprocess 性能: 100ms 启动 + 毫秒级搜索
- 错误: 不 panic, 返回 isError: true

---

## 5. 风险

- **R1**: 0 新依赖
- **R2**: 测试改动: 2 个测试名 + 计数更新 (R201 必要)
- **R3**: McpTool 公开 enum 加变体, 但所有现有 tool 行为不变

---

## 6. 中期路径 (R201+1 候选)

- in-process lib 集成 (ast-grep crate ~3MB)
- YAML rule 暴露为 MCP tool
- tree-sitter fallback (无 ast-grep 时用 tree-sitter)