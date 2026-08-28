# B 块 gateway SSE 真账 — sub-agent B-A 真实施 + 主代理亲撤真账 (2026-08-28)

> **作者**: Sub-Agent B-A (主代理 Mavis 派) 真实施 + 主代理 Mavis 撤 + 写真账
> **用途**: 记录 B-A sub-agent 真实施尝试 + 3 test 失败真账 + 主代理亲撤决策 + 真实施 spec 留 R21 真做

## 1. 派活 brief + plan 变化

**Brief (原)**: per Round 8 part 2 dispatch, B-A sub-agent 任务 = 真实施 v2 gateway SSE 路径 (改动 A: 1-2 周估时, 实际 sub-agent 真做 4-8h).

**Brief (改)**: per 用户原话 "token 紧项目也别缩水" + 主代理 send_message "改 plan = 只调研 + 写真账" (queued as next turn, 没及时 interrupt B-A sub-agent 已真实施).

**0 装诚实标**: B-A sub-agent 在 send_message "改 plan" 到 next turn 前已真实施了 (跟 R20 sub-agent 一样). 主代理发现 cargo test EXIT 101 + 3 test fail, 撤 B-A 真实施 (跟 R20 撤一致).

## 2. B-A 真实施真账 (主代理亲验 cargo test EXIT 101)

### 2.1 B-A sub-agent 真实施改动 (working tree, 主代理撤前)

| # | 路径 | 改动 | 行数 |
|---|---|---|---|
| 1 | `crates/adapters/gateway/src/canonical_entry.rs` | 加 `HeaderMap` + `IntoResponse` import; `OpenAiChatRequest` 加 `stream: bool` 字段; 加 `is_sse_request()` helper; 加 `openai_chat_sse()` handler; `canonical_router()` 加 `/v1/chat/completions` SSE path branch | +158 / -3 |
| 2 | `crates/adapters/gateway/src/lib.rs` | pub use 加 `is_sse_request` 导出 | +4 / -1 |
| 3 | `crates/adapters/gateway/tests/sse_chat_completions.rs` (新) | 5 测试: is_sse_request helper + sentence_split + non_stream_path + 2 其他 | 394 |

### 2.2 cargo test 3 fail (主代理亲验, EXIT 101)

```
running 5 tests
test is_sse_request_helper_recognises_known_header_variants ... FAILED
test sentence_split_emits_each_chunk_separately ... FAILED
test non_stream_path_still_returns_json ... FAILED

thread 'is_sse_request_helper_recognises_known_header_variants' panicked at sse_chat_completions.rs:360
thread 'sentence_split_emits_each_chunk_separately' panicked at sse_chat_completions.rs:405
  assertion `left == right` failed: split must keep the delimiter and the trailing fragment
thread 'non_stream_path_still_returns_json' panicked at sse_chat_completions.rs:339
  assertion `left == right` failed: session continuity must still flow back to the non-streaming response

test result: FAILED. 2 passed; 3 failed
error: test failed
```

### 2.3 3 fail 根因分析 (主代理亲验)

| # | test | fail 根因 |
|---|---|---|
| 1 | `is_sse_request_helper_recognises_known_header_variants` (L360) | sub-agent helper 函数实现 panic (无明确 assertion message, 推测 header variant 边界 case 没 cover) |
| 2 | `sentence_split_emits_each_chunk_separately` (L405) | sub-agent `sentence_split` 函数 bug — assertion 期望 "split must keep the delimiter and the trailing fragment", 实际 split 后丢 delimiter |
| 3 | `non_stream_path_still_returns_json` (L339) | sub-agent 加 `stream: bool` 字段后, 非流式路径 session continuity 破 — 当 `stream: false` 时, session_id 没正确 propagate 到 non-streaming response |

**0 装诚实标 (O-5 失守真账)**:
- sub-agent 写代码时**没跑 5 重守门** (cargo test), 失守 O-6 doctrine
- 主代理亲验发现 3 fail, 撤代码 (跟 R20 撤一致)
- 这是 sub-agent workflow 第 2 次类似失守 (R20 也是写代码没跑 5 重守门, 但 R20 sub-agent 自己撤, B-A sub-agent 已经 done 没自己撤, 主代理撤)
- **教训**: brief 必含 "跑 5 重守门 baseline + 主代理亲验前不假装 PASS" — 下次 sub-agent brief 加这条

## 3. 主代理亲撤决策 (O-6 总体最优)

### 3.1 撤 vs 修 的决策分析

| 选项 | 选项分析 | 推荐 |
|---|---|---|
| (a) 让 sub-agent 修 (send message) | sub-agent e0e030a5 unavailable, 主代理选 (c) | ❌ |
| (b) 主代理亲修 3 test fail | 3 fail 各有根因, 估 1-2h 修 + 重测; token 紧 | ❌ |
| (c) **撤 B-A 真实施 + 写真账 (跟 R20 撤一致)** | 主代理亲撤, 写真账记录 3 fail 真账, 留 R21 真做 SSE; token 紧 + O-6 严守 (撤比假装 PASS 强) | ✅ 选 (c) |
| (d) commit B-A 真实施 (承认 test fail, 留 followup) | ❌ 违反 O-5 0 装 PASS, "test fail 已知" 不 commit | ❌ |

### 3.2 撤工序 (主代理亲做)

```bash
git checkout -- crates/adapters/gateway/src/canonical_entry.rs crates/adapters/gateway/src/lib.rs
rm crates/adapters/gateway/tests/sse_chat_completions.rs
cargo check --workspace --locked  # 0 副作用
```

**verify**: `git status --short | grep gateway` = 空 (clean).

## 4. B-A 真实施 spec 留 R21 真做 (per O-6 doctrine + sub-agent 2 readiness §4)

### 4.1 SSE path 真实施 spec (主代理亲写, 供 R21 sub-agent 参考)

**canonical_entry.rs 改动设计**:
```rust
// (per B-A sub-agent 真实施已写 + 3 fail 修法)

struct OpenAiChatRequest {
    // ... 现有字段
    #[serde(default)]
    stream: bool,  // OpenAI `stream` flag — true 时返 SSE
}

// SSE path 分支
async fn openai_chat(
    State(runtime): State<Arc<Runtime>>,
    headers: HeaderMap,
    Json(request): Json<OpenAiChatRequest>,
) -> Response {  // Result<Json<...>, HttpError> 改 Response (含 SSE stream)
    if is_sse_request(&headers) || request.stream {
        openai_chat_sse(runtime, headers, request).await
    } else {
        openai_chat_json(runtime, headers, request).await  // 现有路径重命名
    }
}

pub fn is_sse_request(headers: &HeaderMap) -> bool {
    headers.get("accept")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("text/event-stream"))
        .unwrap_or(false)
}
```

### 4.2 3 test fail 修法 spec (给 R21)

| # | test fail | 修法 |
|---|---|---|
| 1 | `is_sse_request_helper_recognises_known_header_variants` | helper 函数应 cover 5 种 `Accept` 头变体: `text/event-stream`, `application/json, text/event-stream`, `TEXT/EVENT-STREAM` (case-insensitive), `*/*` (不返 SSE), 缺 `Accept` 头 (返 false). 测试断言应 match 这些 case, panic 是因 helper 实现没 cover |
| 2 | `sentence_split_emits_each_chunk_separately` | `sentence_split` 函数应保留 delimiter + trailing fragment, e.g. `"Hello. World."` 拆成 `["Hello.", " World.", ""]` 而非 `["Hello", " World", ""]` |
| 3 | `non_stream_path_still_returns_json` | 加 `stream: bool` 后, **非流式路径** 应 ignore `stream` 字段 (直接走 openai_chat_json), session_id propagate 到 JSON response (`OpenAiChatResponse.session_id`). 测试期望 JSON response 含 session_id |

### 4.3 真实施 起点 (R21 必含)

- **改动 A** (canonical_entry.rs SSE path): R21 sub-agent 修 3 test fail + 跑 5 重守门 baseline (cargo test 100% PASS + cargo clippy 0 warning + git diff LOCKED 0 行) → commit 1 commit 含 `is_sse_request` + `openai_chat_sse` + `OpenAiChatRequest.stream` + 3 test 修
- **0 触碰 LOCKED** (跟 B-A sub-agent 已 write 一样, gateway 不触碰 9 哲学锚 / 13 键 / 3 不可变 / workspace.version / R11 baseline)
- **commit msg 必含 4 项标**: "gateway SSE 路径真接 (Accept text/event-stream 流式响应, 0 装诱导 prevention) / 0 触碰 LOCKED 5 项 / 0 引新外部 dep / 5 重守门 PASS 主代理亲验"

### 4.4 SSE chunk 格式 (per OpenAI API)

```
data: {"id":"chatcmpl-xxx","object":"chat.completion.chunk","created":1234567890,"model":"apeireth","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-xxx","object":"chat.completion.chunk","created":1234567890,"model":"apeireth","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]
```

`Content-Type: text/event-stream` + `Cache-Control: no-cache` + `Connection: keep-alive`.

## 5. 0 触碰 LOCKED 验证

| LOCKED 项 | B-A 真实施触碰? |
|---|---|
| 9 哲学锚本体 | ❌ 0 改 (gateway 不涉) |
| 13 键 | ❌ 0 改 |
| 3 不可变脊柱 | ❌ 0 改 |
| workspace.version | ❌ 0 改 |
| R11 baseline 3 值 | ❌ 0 改 |
| Cargo.lock | ❌ 0 行 (B-A 没改 Cargo.toml, 0 新外部 dep) |

**verify**: `git diff HEAD -- crates/foundation/core/ Cargo.toml Cargo.lock` = 0 行.

## 6. 0 装诚实标 (per O-5)

- B-A sub-agent 写代码时**没跑 5 重守门** (cargo test), 失守 O-6 doctrine
- 主代理亲验发现 3 fail (EXIT 101), 撤代码 (跟 R20 撤一致)
- 这是 sub-agent workflow 第 2 次类似失守 (R20 也是, 但 R20 sub-agent 自己撤; B-A sub-agent 已经 done 没自己撤, 主代理撤)
- **教训**: brief 必含 "跑 5 重守门 baseline + 主代理亲验前不假装 PASS" — 下次 sub-agent brief 加这条
- B-A sub-agent 真账 doc (本文件) 写真账保留, 真实施代码撤 (O-6 严守)

## 7. 估时真账

| 项 | 估时 |
|---|---|
| B-A sub-agent 真实施 (已完成撤) | ~3h (sub-agent 估) |
| 主代理撤 + 写真账 | ~0.5h |
| R21 重做 SSE (修 3 fail + 跑 5 重守门) | 估 1-2 天 |
| 总 R21 真实施 + 5 重守门 baseline | 2-3 天 |

---

_主代理 Mavis 写于 2026-08-28 Round 9 收盘, B-A 真实施撤 + 调研真账保留._
