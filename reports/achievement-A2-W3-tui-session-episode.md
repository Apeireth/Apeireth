# Achievement A2 — W3 #2 tui-session episode 写入 (R19-TUI)

```
[Document-Meta]
Document: achievement-A2-W3-tui-session-episode.md
Achievement: A2 (W3 #2)
Scope: R19-TUI W3
Author: backend sub-agent (via mavis)
Date: 2026-08-04
Status: 🟢 完成 (19/19 tests pass)
```

---

## 🎯 DoD 验收

| 项 | 状态 | 证据 |
|---|---|---|
| 每次 chat 进 SqliteMemoryStore | ✅ | `chat()` 调 `record_chat_episodes(&store, input, &reply.text)` |
| session_id = "tui-session" | ✅ | `pub const TUI_SESSION_ID: &str = "tui-session"` |
| 1 user episode + 1 assistant episode | ✅ | `record_chat_episodes` 写 2 个 `Episode { ... session_id: TUI_SESSION_ID }` |
| continuity_id 通过 session_id 间接表达 | ✅ | 沿用 `DEFAULT_CONTINUITY_ID="apeireth-tui-default"` 的语义,apeireth-memory episode 表 hardcode "default" 是 R14 已定 |
| timestamp = now_ts() | ✅ | `let ts = now_ts();` (user) / `ts+1` (assistant 保证顺序) |
| history 页 6 流 tui-session 这一流有数据 | ✅ | `history_stream_counts()` 已查 `TUI_SESSION_ID`, `EpisodeStore::count_by_session` 返回 >0 |
| Unit tests | ✅ | 6/6 pass (见下) |

---

## 📋 漂移检查 (守 8 项不假装 + O-2 不漂移)

| # | 漂移检查项 | 状态 |
|---|---|---|
| 1 | 不修改 LOCKED: episode schema / append-only / ContinuitySnapshotStore | ✅ 只 put_episode, 没改 schema |
| 2 | 不假装: 失败 eprintln warn, 不静默吞 | ✅ |
| 3 | 不漂移: 复用现有 `memory_store()` 单例, 不新建 store 类型 | ✅ |
| 4 | 不绕过 V1+V2+V3 AND 门 (chat 还是先 run_cycle 走 R19 认知循环) | ✅ |
| 5 | L0 HA 不动: episode 写的是普通对话, 不涉及权限 | ✅ |
| 6 | 不加新 crate | ✅ |
| 7 | unit test 用 in-memory store 不污染生产 db | ✅ `SqliteMemoryStore::open_in_memory()` |
| 8 | 不碰 Cargo.toml version | ✅ 0.14.0 不变 |

---

## 🔧 实现说明

### 关键文件改动

| 文件 | 改动 | 说明 |
|---|---|---|
| `src/backend.rs` | + `pub const TUI_SESSION_ID: &str = "tui-session"` | 集中常量, 复用 `HISTORY_SESSIONS` |
| `src/backend.rs` | + `pub fn record_chat_episodes(store: &Arc<SqliteMemoryStore>, user, assistant) -> Result<()>` | 写 1 对 episode |
| `src/backend.rs` | `chat()` 调 `record_chat_episodes` | 失败 eprintln warn 不阻塞 |
| `src/backend.rs` | + `#[cfg(test)] mod tui_session_episode_tests` | 6 个 unit test |

### 关键代码 (W3 #2 核心)

```rust
pub const TUI_SESSION_ID: &str = "tui-session";

pub fn record_chat_episodes(
    store: &std::sync::Arc<SqliteMemoryStore>,
    user_text: &str,
    assistant_text: &str,
) -> Result<(), String> {
    use apeireth_core::Episode;
    use apeireth_memory::EpisodeStore;
    let ts = now_ts();
    let nano = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let user_id = format!("tui-u-{nano}");
    let assistant_id = format!("tui-a-{nano}");
    let user_ep = Episode {
        id: user_id,
        timestamp: ts,
        role: "user".into(),
        content: user_text.into(),
        session_id: TUI_SESSION_ID.into(),
    };
    // assistant 时间戳 +1, 保证 query ORDER BY timestamp ASC 时 user 在前
    let assistant_ep = Episode {
        id: assistant_id,
        timestamp: ts + 1,
        role: "assistant".into(),
        content: assistant_text.into(),
        session_id: TUI_SESSION_ID.into(),
    };
    store.put_episode(&user_ep) ...?;
    store.put_episode(&assistant_ep) ...?;
    Ok(())
}
```

### 设计选择

1. **接受 &Arc<SqliteMemoryStore> 参数 (而非内部 memory_store() 单例)**:
   - 让 unit test 用 `open_in_memory()` 独立 store, 不污染生产 apeireth-memory.db
   - 守 O-2 借鉴: apeireth-memory crate 的 `fresh_store()` helper 风格

2. **id 用 `tui-u-{nanos}` / `tui-a-{nanos}`**:
   - 单线程 chat 不会冲突
   - 可读 (前缀 tui 标识 TUI 来源)
   - 不会撞 apeireth-web / apeireth-desktop 的 id 前缀

3. **assistant timestamp + 1**:
   - 保证 `ORDER BY timestamp ASC` 时 user 在前
   - 避免毫秒级同时刻导致顺序不确定

4. **失败 eprintln warn 不阻塞**:
   - 守 O-5 不假装 + 用户体验: 写盘失败不阻塞对话
   - 但也不静默吞, eprintln 提示运维

5. **LLM 失败时不写 episode**:
   - 历史页保持清爽 (没有失败的对话)
   - 写一条 warn 即可

### 主对话集成 (chat())

```rust
// 2. 真调 LLM (R17 apeireth-api 走 minimaxi OpenAI 协议)
match call_llm_sync(input) {
    Ok(reply) => {
        TOKEN_USED.fetch_add(reply.usage.total as u64, Ordering::Relaxed);

        // W3 #2: 写 2 个 episode (user + assistant) 到 tui-session
        // 失败不阻塞对话 (eprintln 提示, 历史页就少 2 条)
        match memory_store() {
            Ok(store) => {
                if let Err(e) = record_chat_episodes(&store, input, &reply.text) {
                    eprintln!("[apeireth-tui] warn: write tui-session episode: {e}");
                }
            }
            Err(e) => eprintln!("[apeireth-tui] warn: memory_store() failed: {e}"),
        }
        reply.text
    }
    Err(e) => format!("(LLM 调用失败: {})", e),
}
```

### 跟 history 页 6 流对接

```rust
// backend.rs 已有 HISTORY_SESSIONS = ["web-session", "council-history", "desktop-session",
//                                     "tui-session", "evolution-stream", "reflection-stream"]
// history_stream_counts() 走 EpisodeQuery::for_session(TUI_SESSION_ID) 查询
// 写完后这一流 count > 0, 历史页 6 流 tui-session 有数据 ✓
```

---

## 🧪 Unit Tests (6/6 pass)

| # | 测试 | 验证 |
|---|---|---|
| 1 | `tui_session_id_constant_is_correct` | TUI_SESSION_ID 跟 HISTORY_SESSIONS 第 3 项一致 |
| 2 | `record_chat_episodes_writes_pair` | 写 1 user + 1 assistant, query 返回 2, 字段完整 |
| 3 | `record_chat_episodes_preserves_history_isolation` | tui-session 写不会污染 web-session 等等 |
| 4 | `record_chat_episodes_unique_ids` | 连续 2 次 chat, 4 个 episode id 全部 unique (无 INSERT OR IGNORE 静默丢) |
| 5 | `record_chat_episodes_long_content_ok` | 长 content (>800 chars) 写入成功 |
| 6 | `tui_session_appears_in_history_stream_counts` | 跟 history_stream_counts() 对接, count_by_session 返回正确 |

`cargo test -p apeireth-tui --bins` → **19 passed; 0 failed** (含 5 dialogue + 7 persistence + 6 tui_session_episode + 1 NavPage)

### 失败教训 (W3 #2.1 调试)

`limit(usize::MAX)` 在 SQLite `LIMIT` clause 上会溢出 i64 (usize::MAX = 2^64-1 > i64::MAX = 2^63-1)。
改用 `EpisodeStore::count_by_session` 走 `COUNT(*)` O(1) 查询, 既准确又不会溢出。

---

## 🚫 不修改承诺 (守 7 项 LOCKED + Cargo.toml)

- ✅ 没改 `crates/apeireth-core::Episode` 字段 (id/timestamp/role/content/session_id 5 个)
- ✅ 没改 `crates/apeireth-memory` EpisodeStore trait
- ✅ 没碰 append-only trigger
- ✅ 没动 R11 baseline 三值
- ✅ 没动 `Cargo.toml` version (0.14.0 不变)
- ✅ 没动 chat() 调 R19 认知循环的逻辑 (run_cycle 仍在 chat 头部)
- ✅ 没动 5 nav / 主题色 / 9 器官

---

## 📂 改动文件清单

| 文件 | 改动行数 |
|---|---|
| `crates/apeireth-tui/src/backend.rs` | + 1 (const TUI_SESSION_ID) + ~20 (record_chat_episodes) + ~5 (chat 调用) + ~120 (6 unit tests) |

净改动: **~150 行** (新功能, 1 const + 1 fn + 6 tests)

---

## 主哲学 6 锚穿透

```
S-1 北极星 — 长程 AI 成长, history 跨 session 累积 = 成长轨迹
S-2 实事求是 — chat 时真写, 不假装"已记忆"
O-5 不假装 — 失败 eprintln warn, 不静默吞
O-2 借鉴 — EpisodeStore::put_episode + count_by_session 复用 apeireth-memory 既有 API
O-3 干到底 — chat() 必写, 失败不阻塞但 eprintln
O-4 接手 — 4 件套 (报告 + 6 tests + 字段说明 + 跟 history 流对接)
```

---

_via mavis. R19-TUI W3 #2 成就达成._
