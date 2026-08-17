# R203 unified_query MCP 集成 (接续 R201+R202)

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R203
> **日期**: 2026-08-13
> **来源**: R201 ast_grep MCP + R202 unified facade
> **状态**: 实施完成, 67/67 单测全过

## 0. 目标

R202 写了 unified facade (6 维 code intelligence). R203 把它暴露为 MCP tool, 让 LLM agent 用一个统一入口查询.

## 1. 实施

### 1.1 McpTool enum 扩展

加 McpTool::UnifiedQuery 变体 (第 12 个 tool). 同步:
- name(): unified_query
- all() 数组 +1
- MCP_TOOL_COUNT: 11 -> 12
- tools_list_returns_11 -> tools_list_returns_12

### 1.2 unified_query arm in handle()

参数: kind (text/file/symbol/graph/index/ast, 默认 text), pattern, path, lang (可选).
构造 UnifiedQuery + UnifiedCodeIntelligence, 调 query(), 渲染为 [N] kind: detail 列表, 限 100 条.

### 1.3 新测试

unified_query_handles_graceful: 验证 graceful handling, nonexistent path 不崩溃.

## 2. 0 触碰声明

- 3 不可变脊柱: 0 触碰
- workspace.version 1.2.0: 0 改
- 现有 11 个 tool 行为: 0 改
- unified 子模块 (R202): 0 改

## 3. 测试 (67/67 pass)

新增 1 测试, 更新 4 测试 (count + tools_list_returns_N 函数名). 总 +1: 66 -> 67.
