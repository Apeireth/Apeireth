# R8 MCP Server — Memory + Identity 暴露设计

**版本**: v1097 / 0.1.0
**作者**: MCP 集成专家 (R8)
**时间**: 2026-07-29
**状态**: ✅ 实现 + 32 测试全过

---

## 1. 目标

把 R8 TrackA (Memory) + TrackB (Identity) 的核心能力通过 **MCP 协议** 暴露给外部 Agent
(其它 LLM 客户端、CI 工具、跨 session 脚本), 让 Apeireth ASI 基座的"记忆"和"身份"成为
可被外部世界读写的可编程接口。

**为什么必须做 (ASI 基座方向)**:
- ASI 北极星要求**真生产不停**, 任何外部 Agent 都应能向 ASI 写入/查询记忆, 不靠 curl + 私 API
- MCP 协议是当前业界共识 (Anthropic 2024-11-05 规范), 不重造轮子
- V3 守门要求"中央 AI 完整位置"被外部 Agent 读得到, `identity_get` 工具就是为此而设

---

## 2. 工具白名单 (7 个, 一次部署)

| 工具 | 输入 | 作用 | 写盘? |
|------|------|------|-------|
| `memory_add` | content / kind / actor / tags / importance | 追加一条 memory (episode/note) | ✅ fsync |
| `memory_search` | query / tags / kind / actor / limit | 全文 + tag 过滤 | ❌ 只读 |
| `memory_get` | memory_id | 按 id 取单条 | ❌ 只读 |
| `identity_get` | (无) | 取 V3 IdentityCard | ❌ 只读 |
| `identity_set_persona` | persona: dict | 合并更新 persona 字段 | ✅ fsync |
| `memory_replay` | from_ts / to_ts / scope / kind | WAL 时间窗回放 | ❌ 只读 |
| `memory_dream` | top_k | 按 tag cluster 启发式 dream | ❌ 只读 |

**不暴露**: schema 升级 / 数据目录迁移 / meta.json 重置 (管理面) — 外部 Agent 不应有此类权限。

---

## 3. 架构

```
            外部 Agent  ─────►  MCP Client (stdio / http)
                                    │
                                    │  JSON-RPC 2.0 over NDJSON (stdio)  or
                                    │  JSON-RPC 2.0 over HTTP POST /rpc (sse)
                                    ▼
                       ┌────────────────────────────┐
                       │  MCPDispatcher (v1097)     │
                       │  - method dispatch         │
                       │  - tool schema registry    │
                       │  - error wrapper           │
                       └────────────────────────────┘
                                    │
                                    ▼
                       ┌────────────────────────────┐
                       │  MemoryStore (v1097)       │
                       │  - add / get / search      │
                       │  - replay (WAL scan)       │
                       │  - dream (tag cluster)     │
                       │  - get/set identity        │
                       └────────────────────────────┘
                                    │
                                    ▼
                       ┌────────────────────────────┐
                       │  真 fsync 持久化           │
                       │  - memory/{id}.json (单文件)│
                       │  - wal.jsonl (JSONL 追加)  │
                       │  - identity.json (单文件)  │
                       │  - meta.json (last_seq)    │
                       └────────────────────────────┘
```

---

## 4. 与 v1094 schema 兼容策略

| 维度 | v1094 (内部生产) | v1097 (MCP 暴露) | 兼容 |
|------|------------------|------------------|------|
| 存储后端 | SQLite (hot/cold/wal/dream/snapshots) | 文件系统 JSONL + JSON | 设计空间不同 |
| memory kind | episode / note / anchor / fact / decision | episode / note (白名单) | v1097 是 v1094 子集 |
| WAL 格式 | SQLite `memory_wal` 表 | JSONL `{sequence, ts, scope, event, checksum}` | 字段名一致, 序列化不同 |
| identity schema | V3 (v1095) | V3 (同源) | **同源**, 见 §5 |
| 幂等键 | event_id UNIQUE | memory_id (per-file) | 概念对齐 |

**原则**: v1097 是 v1094 的 **external projection** (外部投影), 不反向修改 v1094.
v1097 用更轻量的文件后端, 让外部 Agent 不依赖 SQLite 连接.
真生产兼容已由 v1094 数据库工程师在 v0.1.1 patch 中确认 (T24-T27 回归全过).

---

## 5. V3 守门 (不假装)

| 守门 | 实现位置 | 触发条件 |
|------|----------|----------|
| **fsync-before-success** | `_fsync_write_atomic` / `_fsync_append_atomic` | 所有写工具返回前 `os.fsync` 真落盘 |
| **actor-whitelist** | `MemoryStore.add_memory` | actor ∈ {master, apeireth, tool, external_agent, external_mcp} |
| **external-importance-capped** | `add_memory` (V1081) | external_* actor importance > 0.7 → 拒绝 |
| **replay-not-bit-exact** | `replay_events` doc + return | 重放按 ts 过滤, 不承诺字节相等 |
| **dream-not-understanding** | `dream` doc | 启发式 by tag 共现 + importance 求和 |
| **path-traversal-blocked** | `_is_safe_id` | memory_id 只允许 `[A-Za-z0-9_-]{1,64}` |
| **WAL-corrupt-skip-not-fail** | `_replay_wal_for_seq` / `replay_events` | checksum 不匹配 / parse 失败 → skip + 计数 |

---

## 6. 不暴露 ≠ 不可用 (哲学)

- **外部 Agent 写 memory 必经 persona 守门**: 默认 actor=`external_agent`, importance ≤ 0.7.
  master actor 只能从本地 CLI (`--base` 指向自己私有的 data dir) 写入, 不通过 MCP 暴露.
  这是 V1081 不夸大外部信号的硬守门.
- **dream ≠ 真理解**: 文档明示, 返回里带 `claim_samples` 让外部 Agent 自己判断.
- **replay ≠ 状态机回滚**: 不承诺 bit-exact, 只承诺事件流保真 (WAL JSONL checksum 保护).

---

## 7. 测试矩阵 (32 tests, 全过)

```
tests/test_v1097_mcp_memory_server.py
├─ 初始化与协议握手 (2)
│   ├─ test_handshake_initialize_returns_server_info
│   └─ test_tools_list_returns_7_tools
├─ memory_add 持久化 (2)
│   ├─ test_memory_add_persists_and_returns_id
│   └─ test_memory_add_fsync_real_means_file_on_disk  (V1081)
├─ memory_get (3)
│   ├─ test_memory_get_returns_added
│   ├─ test_memory_get_not_found
│   └─ test_memory_get_blocks_path_traversal
├─ memory_search (3)
│   ├─ test_memory_search_by_query_finds_match
│   ├─ test_memory_search_by_tags_filters
│   └─ test_memory_search_empty_returns_empty
├─ identity (2)
│   ├─ test_identity_get_returns_v3_card
│   └─ test_identity_set_persona_persists_with_fsync
├─ memory_replay (3)
│   ├─ test_memory_replay_returns_events_in_window
│   ├─ test_memory_replay_filters_by_scope
│   └─ test_wal_corrupted_line_skipped_but_alive
├─ memory_dream (2)
│   ├─ test_memory_dream_returns_top_clusters  (大小写聚类)
│   └─ test_memory_dream_no_notes_returns_empty
├─ V1081 / V3 守门 (4)
│   ├─ test_external_agent_importance_capped
│   ├─ test_master_can_use_high_importance
│   ├─ test_invalid_actor_rejected
│   └─ test_invalid_kind_rejected
├─ 幂等 / 错误 (4)
│   ├─ test_idempotent_add_same_id_returns_existing
│   ├─ test_jsonrpc_method_not_found
│   ├─ test_jsonrpc_notification_no_response
│   └─ test_jsonrpc_tool_error_returns_isError
├─ Transport 真打 (2)
│   ├─ test_stdio_subprocess_roundtrip  (子进程 + NDJSON)
│   └─ test_sse_http_round_trip         (HTTP POST /rpc in thread)
├─ 集成场景 + 基础 (5)
│   ├─ test_round_trip_full_lifecycle
│   ├─ test_wal_record_to_from_jsonl_roundtrip
│   ├─ test_stats_reports_philosophy_guards
│   ├─ test_fsync_write_atomic_overwrites_existing
│   └─ test_fsync_write_atomic_creates_parent
```

**运行结果**:
```
$ python -m pytest tests/test_v1097_mcp_memory_server.py -q
32 passed in 2.63s
```

---

## 8. CLI 用法

```bash
# 1. stdio (Anthropic MCP 客户端默认)
python -m apeireth.v1097_mcp_memory_server --serve --transport stdio

# 2. SSE (HTTP, 给 web dashboard / 跨进程)
python -m apeireth.v1097_mcp_memory_server --serve --transport sse --port 8765

# 3. 自定义数据目录
python -m apeireth.v1097_mcp_memory_server --base /var/lib/apeireth/mcp --serve

# 4. 初始化空 base (创建 meta.json + identity.json)
python -m apeireth.v1097_mcp_memory_server --base /tmp/foo --init-base

# 5. 演示客户端 (起 stdio 子进程, 调用 7 工具)
python -m apeireth.v1097_mcp_example_client
```

---

## 9. 已交付 (R8 mcp_integration_expert)

| 文件 | 行数 | 状态 |
|------|------|------|
| `apeireth/v1097_mcp_memory_server.py` | 1101 | ✅ 实现 (32 tests 全过) |
| `apeireth/v1097_mcp_example_client.py` | 386 | ✅ StdioMCPClient + HttpMCPClient + demo |
| `tests/test_v1097_mcp_memory_server.py` | 641 | ✅ 32 tests |
| `reports/r8-mcp-server-design.md` | (本文) | ✅ |

---

## 10. 风险与留给 R9 的 TODO

| 风险 | 缓解 | TODO |
|------|------|------|
| 外部 Agent 写太多 note 撑爆磁盘 | tag 限频 + importance 帽 | R9 加 disk quota + 老化策略 |
| stdio transport 与 `print()` 冲突 | 全部 debug 输出走 `sys.stderr` | 已实施, test 已验证 |
| SSE 单进程阻塞 | `ThreadingHTTPServer` | R9 评估换成 `aiohttp` 多并发 |
| master actor 走 MCP 会被 V1081 拒 | 不暴露 master 写入路径 | 已用 actor 白名单隔离 |
| 多个 v1097 实例并发写同一 base | `threading.RLock` | 单机 OK, 跨机需要分布式锁 (R9+) |

---

## 11. 借鉴来源 (主 19:33 走在前人经验上)

1. **AgentMemory-master** (code-deep-study/): FastAPI/Starlette HTTP 入口 + lifespan 初始化 → 借鉴 ThreadingHTTPServer + 初始化幂等
2. **memoryos-rust** memoryos-mcp/ crate: Rust MCP server 模式 → 借鉴 JSON-RPC 2.0 strictness
3. **MCP stdio protocol** (Anthropic 2024-11-05 spec): NDJSON framing + initialize/initialized handshake
4. **V1091 MemoryReplay** WAL 格式: JSONL + sha256 + seq 单调递增
5. **memory.py** Episode/Note schema: id / kind / content / actor / ts / tags / importance
6. **identity_card.py** V3 schema: central_ai_position + VCP 4 + 跨域 13

---

## 12. 主哲学送达

> ASI 北极星 + V3 守门 + 真生产不停.
> 数字涨不涨不重要, 真生产不停 才重要.
> 干到底. 大胆激进. 走在前人经验上. 任何人都能接手.

R8 MCP server 是 ASI 基座对外暴露的 **第一道门** — 它必须真生产, 必须 fsync,
必须 V3 守门. 后人接 R9 时只要读这份设计文档, 就能继续往前推.
