# R224 — apeireth-mcp JSON-RPC 2.0 §6 Batch 支持

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R224
> **日期**: 2026-08-13
> **状态**: 1 commit, 14 测试 +14, 0 errors / 0 warnings

---

## 0. 主人指示

"全做全做全补弱 + 一体化优美" + "继续推到底" + "GitHub 调研不要只看整体的同类项目了, 还要针对我们的每一个模块去看有没优秀项目"

## 1. 调研结论

apeireth-mcp 现状 (R223 末):
- 7400+ 行代码 / 22 src 文件 / 205 测试
- 3 transports: stdio / SSE / HTTP-Streamable + Memory (测试用)
- 完整 protocol (initialize / tools/list / tools/call / resources/list / resources/read / prompts + subscriptions / telemetry / multimodal)
- **缺**: JSON-RPC 2.0 §6 Batch

GitHub mcp rust-sdk (`modelcontextprotocol/rust-sdk` `rmcp` v1.5.0) 已支持 batch, 是事实标准。

## 2. 设计

### 2.1 protocol.rs — JsonRpcBatch<T>

**enum**:
- `Single(T)` — 单个 request / response (向后兼容)
- `Batch(Vec<T>)` — 数组形式

**Serialize / Deserialize**: 自实现, 用 `serde_json::Value` 二次分发
- Serialize: 直接转发 T 的 serialize (Vec 走 array, T 走 object/number/...)
- Deserialize: 先 deser 为 `Value`, `is_array()` → Batch, 否则 Single

**why 自实现**: serde `#[serde(untagged)]` 对 enum 不会保留外层 array vs object 的区分 (Vec<T> vs T 都是合法 JSON), 必须从 wire 形态手动分发.

### 2.2 looks_like_batch(line) 启发式

服务端主循环需要先快速判断: 这一行是 batch 还是 single?

**算法**: `line.trim_start().starts_with('[')`

**为什么不先 parse 一次再分发**: parse 失败会浪费 CPU, 且 §6 空 batch 合法 (parse OK) 但语义不同. trim + starts_with 是 O(n) 字符串扫描, 不分配.

### 2.3 服务端 §6 严格行为

| 输入 | 行为 |
|---|---|
| 空 batch `[]` | Invalid Request 单错误响应 (-32600) |
| 全 notification batch | **不响应** (per §6 "if batch has no response, server SHOULD NOT reply") |
| Mixed batch (有 id + notification) | 只回有 id 的 response, 顺序与请求一致 |
| Single request | 原 dispatch 单响应 |
| 单行 parse 失败 | warn + skip (不回响应, 无法关联) |

### 2.4 McpClient::send_batch

**算法**:
1. `ensure_initialized()` — 未 init 报错
2. 空 batch 客户端就拒绝 (省一次 round-trip)
3. 给无 id 的 request 自动分配 id (复用 `next_id` 计数器)
4. transport 单飞 `send + recv` — 防 request/response 交错
5. 解析响应: Batch → Vec, Single → Vec[1]

**防御**: Single 响应时 (服务端回单 error), 返回 `Vec` 长度为 1 (与协议一致)

## 3. 测试覆盖 (14 cases)

### protocol.rs (7)
- batch_request_serialize — 序列化为 `[...]`
- batch_request_deserialize — 反序列化 batch
- batch_single_fallback — 单 object 解析为 Single
- batch_response_roundtrip — 响应 roundtrip
- batch_empty_is_invalid_batch — 空 batch 合法 parse 但语义 Invalid
- looks_like_batch_heuristic — `[` vs `{` 检测
- batch_from_vec_single_collapses — 1 元素塌缩为 Single

### lib.rs (7)
- batch_end_to_end_via_memory_pipe — 端到端 batch (3 requests, 顺序保持, 结果正确)
- batch_with_error_response — mixed success/error
- batch_empty_rejected — 客户端就拒绝
- batch_requires_initialize — 未 init 报错
- handle_line_batch_with_notification_returns_no_response — notification drop
- handle_line_empty_batch_returns_error — §6 空 batch 单错误响应
- handle_line_single_request — 单请求兼容
- handle_line_empty_string_returns_none — 空行忽略

## 4. 工程指标

- **0 errors** workspace
- **0 warnings** (余 3rd-party future-incompat)
- **0 触碰** 3 不可变脊柱
- **0 引入** 新外部 dep
- **0 删除** 任何代码
- **workspace.version** 1.2.0 0 改
- **测试**: 205 → 219 (+14)

## 5. 战区意义

apeireth-mcp 的协议层从单 request 升级到 JSON-RPC 2.0 §6 完整 batch 支持, 与 `rmcp` 等官方 SDK 的 wire 兼容度提升一个量级. 这一步是 mcp 从 "能跑" 到 "wire 完整" 的关键拼图, 也是 mcp 子方向 "终极目标 = 全做全补弱" 的最后一公里之一.

## 6. 下一步候选 (per R218 路线 + 主人"继续推到底")

- **R225** protocol Arrow + DataFusion — 大项目, 2-3 days
- **R226** TUI 接入新 runtime (5 nav pages 已有, 持续)
- **R227** constraint symbolic solver (egg 调研) — 2-3 days
- **R228+** pybridge asyncio 真接 / supervisor metrics endpoint / upgrade 跨进程 IPC