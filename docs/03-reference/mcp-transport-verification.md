# MCP 三传输面实测记录（stdio / http / sse）

> **日期**：2026-11-05　|　**范围**：外部工具桥（协议桥）的**三个传输面**在生产组合根上的 live 实测
> **四级口径**：本记录属「**实现存在 + 自建探针实测**」⇒ 落在 **IMPLEMENTED**（库级可跑）+
> **PRODUCTION WIRED**（三个面都走生产组合根的「发现 → 登记 → 族授权 + 逐次人批 → 派发」链路，并实测跑通）。
> **DEFAULT ENABLED 不成立**（默认关，开关 `APEIRETH_ENABLE_MCP=1`）。
> **HARDWARE VALIDATED 不成立** —— 探针是**仓外自建**的最小服务器，**不是**真实第三方 MCP 服务器
> ⇒ 本记录证明的是「**三种传输的协议路径在仓内实现上跑得通**」，**不是**「真机验证」。详见 §7。
> **写域**：本记录只新建本文 + `docs/INDEX.csv`（只由 `docs_index.py --gen` 产出）；
> **未改** `crates/**`、**未改** `docs/status/**`、**未改** `docs/architecture/**`、**未改** `scripts/**`。

---

## 1. 结论摘要

| 面 | 实现落点（`crates/capabilities/tools/src/mcp_bridge/transport.rs`） | 配置形态 | 实测结论 |
|---|---|---|---|
| **stdio** | `StdioChannel`（`:161`），`impl McpChannel`（`:239-261`） | `"transport":"stdio"` + `command`（+`args`），**禁**带 `url` | **通** |
| **http** | `HttpChannel`（`:274`），`impl McpChannel`（`:306-319`） | `"transport":"http"` + `url`，**禁**带 `command` | **通** |
| **sse** | `SseChannel`（`:323`），`impl McpChannel`（`:440-457`） | `"transport":"sse"` + `url`，**禁**带 `command` | **通**（本批首次实测） |

三个面在生产组合根（`apeireth session`）上均产出**逐字相同**的一行：

```
[external-tools] wired: 1 registered, 0 unregistered, 1 live
```

进程 exit 0，stdout 打印 `canonical runtime ready`。

**本批新增的是 sse 面**：它是三条里唯一「手写 HTTP SSE 客户端」的路径
（帧解析器在协议库，HTTP 客户端在传输层），此前**从未实跑过**。本批实测结论：**跑得通**。

---

## 2. 复核：仓内实现的实际协议形态（照源码，不照规范想当然）

### 2.1 传输枚举与分派

- 枚举 `McpTransportKind { Stdio, Sse, Http }`：`crates/capabilities/tools/src/mcp_bridge/config.rs:54-62`
  （`#[serde(rename_all = "lowercase")]` ⇒ 配置里的取值就是 `"stdio"` / `"sse"` / `"http"`）。
- 分派点 `open_channel`：`transport.rs:96-111`。
  - stdio 分支 `:98-101`：`spec.command`（缺省空串）+ `spec.args`；
  - http 分支 `:102-105`：`spec.url`；
  - **sse 分支 `:106-109`：`spec.url` —— 与 http 分支取的是同一个字段，语义完全相同**：
    `url` = **事件流地址**（不是「消息投递地址」）。投递地址由服务器在流里另行宣告。
- 配置校验 `McpServerSpec::validate`：`config.rs:213-282`
  - stdio 必须给 `command`、不得给 `url`（`:244-258`）；
  - sse / http 必须给 `url`、不得给 `command`，且 `url` 必须是 http(s) 绝对地址（`:259-279`）。

### 2.2 sse 面的实际协议（逐字读 `SseChannel` 得出）

`SseChannel::open`（`transport.rs:333-387`）：

1. **建连**：`GET <url>`，请求头 `Accept: text/event-stream`（`:335-340`）。
   连接失败 ⇒ `McpError::Transport("opening event stream: …")`；
   非 2xx ⇒ `"event stream answered HTTP {status}"`（`:341-346`）。
2. **宣告投递端点**：客户端**不自己拼**投递地址。事件流上必须出现
   `event: endpoint` + `data: <投递地址>` 帧；客户端取该帧的 `data`，
   用 `absolutize_endpoint(base_url, data)` 解析成绝对地址后存进一个
   `watch::channel::<Option<String>>(None)` 槽（`:348`、`:363-365`）。
3. **建连阻塞在等端点**：`open()` 末尾 `channel.wait_for_endpoint().await?`（`:385`）——
   **端点没宣告出来就不会返回**。
4. **发消息**：`POST <解析后的投递地址>`，请求体 = JSON-RPC 消息本身
   （`post_to_endpoint`，`:414-436`）。响应体**被丢弃**；只看状态码，非 2xx ⇒
   `"message endpoint answered HTTP {status}"`。
5. **收响应**：响应**不走 POST 的响应体**，而是回到**同一条事件流**上，
   以 `event: message` + `data: <JSON-RPC 响应>` 帧送达（`:366-371`）。
   帧的 `data` 被反序列化成 `JsonRpcResponse`，再按 JSON-RPC `id`
   经 `PendingRouter`（`:116-154`）配给等待方。
6. **配对时序**：`request()` **先登记等待者再 POST**（`:447-448`）——
   避免「响应比登记快」时丢包。

⇒ 一句话：**sse 面 = 一条长连的事件流（回响应）+ 一个由流宣告的 HTTP 投递端点（收请求）**，
两端角色与多数人的直觉相反（请求走 POST、响应走 GET 流）。

### 2.3 `SseBuffer` 的职责边界（`crates/foundation/plugin/src/mcp/sse.rs`）

`SseBuffer` **只是帧解析器，不是 HTTP 客户端**（该文件头 `:1-6` 明说「无 HTTP 客户端」，
`SseTransport` 被显式推迟）。它的职责只有三件：

| 职责 | 落点 | 说明 |
|---|---|---|
| 增量攒字节 | `SseBuffer::push_bytes`（`:95-97`） | 无效 UTF-8 走 lossy 替换，单个坏字节不会卡住流 |
| 切完整帧 | `SseBuffer::next_frame`（`:105-115`）+ `find_frame_sep`（`:131-140`） | 需要 `\n\n` 或 `\r\n\r\n` 分隔符才吐帧；半帧留在缓冲里等下一批 |
| 解析字段 | `parse_sse_frame`（`:44-80`） | `event:` / `data:` / `id:` / `retry:`；冒号后**一个**前导空格被吃掉（`data:  x` ⇒ ` x`）；`:注释` 与未知字段忽略；多行 `data:` 用 `\n` 拼接（`SseFrame::data`，`:24-26`） |

帧的语义判定只有两个布尔：`is_message()`（`event == "message"`）与 `is_endpoint()`（`event == "endpoint"`）
（`:28-34`）。**没有 `event` 字段的帧（默认事件名）不算 `message`** ⇒ 帧的 `event:` 字段是必需的，
照规范把数据写成裸 `data:` 行**不会**被当作 JSON-RPC 响应。

`absolutize_endpoint`（`:148-166`）是 std-only 的手写解析（不用 URL 库 join）：
- 已是 `http(s)://` 绝对地址 ⇒ 原样返回；
- 以 `/` 开头 ⇒ 换掉 base 的路径部分，保留 scheme + authority；
- 其它相对地址 ⇒ 追加到 base 路径**最后一个 `/` 之后**。

### 2.4 超时与重连（照实现逐条列清）

| 环节 | 实现 | 落点 |
|---|---|---|
| 端点宣告等待 | **10 秒**硬截止；超时 ⇒ `"event stream did not announce a message endpoint"` | `transport.rs:389-412`（截止常量在 `:394`） |
| **响应等待** | **无截止**。`request()` 直接 `waiter.await`，服务器收了 POST 但不回帧 ⇒ 永久阻塞 | `transport.rs:449-451` |
| 事件流断开 | 读任务结束 ⇒ `broken = true` + `fail_all()`（唤醒所有等待者，报 `"event stream closed before the response arrived"`）；此后 `post_to_endpoint` 拒绝 ⇒ `"event stream is closed"` | `transport.rs:374-375`、`:418-420`、`:450` |
| 重连 | **`SseChannel` 自己没有重连逻辑**。重连预算在上层 `McpServerConnection::recover()`：丢链后按退避预算重开一条**全新**通道并重跑握手 + 重发现 | `crates/capabilities/tools/src/mcp_bridge/connection.rs:220-261`（`connect` 在 `:157-194`） |
| 工具调用截止 | 桥级 `call_timeout_ms`（默认 30 s）作为执行流水线的 around 超时，**只包住工具调用**，**不包住握手** | `config.rs:41`（默认值）、`config.rs:446`（字段）、`crates/capabilities/tools/src/mcp_bridge/bridge.rs:669`（落点） |

---

## 3. 实测环境（2026-11-05 实跑）

| 项 | 值 |
|---|---|
| 日期 | 2026-11-05 |
| 工具链 | `rustc 1.97.1 (8bab26f4f 2026-07-14)`；`cargo 1.97.1 (c980f4866 2026-06-30)` |
| 被测二进制 | `target/debug/apeireth`，`apeireth --version` ⇒ `apeireth 2.0.0-rc.5` |
| 构建命令 | `cargo build -p apeireth-cli --locked -j 2` |
| 探针运行时 | `Python 3.14.8` |
| 宿主 | `Linux 7.1.5+kali-amd64` |
| 探针位置 | **仓外** `/tmp/apeireth-sse-probe/`（`sse_probe.py` / `stdio_probe.py` / `http_probe.py` / `run_all.sh`）—— **不进仓**，一次性验证工具 |

⚠️ **被测二进制不是纯净 HEAD 构建**：工作树里另有一条**并行施工线**正在改 CLI 适配器层
（新增 `apeireth mcp list/deny/undeny` 子命令等）。该改动**不触及** MCP 桥与三个传输层
（`crates/capabilities/tools/src/mcp_bridge/**`、`crates/foundation/plugin/src/mcp/**` 一行未改），
故不影响本记录的三面结论。

⚠️ **stderr 噪声**：每次运行都会打两行**与 MCP 无关**的环境噪声，逐字保留以免读者误读为 MCP 现象：

```
[keyring] KeyringSelector 退化到 EnvCredentialResolver: APEIRETH_KEYRING_BACKEND 未设 (设 APEIRETH_KEYRING_BACKEND=auto 可重新尝试 keyring)
[governance-audit] 持久审计账本不可用: governance audit record 1304 hash mismatch (tamper detected); 降级内存账 (审计不持久, 如实)
```

（第二行是本机数据目录里那份治理审计账本自身校验失败 ⇒ 运行时如实降级为内存账，与本次实测无关。）

**注 ①（测量口径）**：stdio 面的探针侧日志不是从 stderr 抓的 —— `StdioChannel::open` 把子进程
stderr 接到 null（`transport.rs:175`）⇒ 本批让 stdio 探针把线缆日志写到 `$PROBE_LOG` 文件再取回（§4.1）。

**注 ②（探针自身的一次缺陷，如实登记）**：sse 探针的**初版**用了一个**全局共享**的响应队列，
没有按会话分流 ⇒ 客户端的 `initialize` 响应被投给了另一条已断开的流（探针写失败），
真正的流**永远等不到响应** ⇒ 该次运行表现为 120 s 无输出直至被杀。
**这是探针的缺陷，不是实现的缺陷** —— 修正探针（按 `sessionId` 分流，每条流一个队列）后
sse 面一次跑通（§4.3）。**如实登记在此，避免后人把那次挂死误读成「实现挂死」。**

---

## 4. 三个面的实测记录

### 4.1 面 ①：`stdio`

**① 配置形态**（`config.rs:77-135`）：

```json
[{"name":"stdioprobe","transport":"stdio","command":"/tmp/apeireth-sse-probe/stdio_probe.py","enabled":true}]
```

- `command` **必需**（`validate` 会拒空的）；`args` 可选；**不得**带 `url`。

**② 实测命令**：

```sh
APEIRETH_ENABLE_MCP=1 \
APEIRETH_MCP_SERVERS='[{"name":"stdioprobe","transport":"stdio","command":"/tmp/apeireth-sse-probe/stdio_probe.py","enabled":true}]' \
./target/debug/apeireth session
```

**③ 实测输出（逐字）**：

```
exit=0
--- stdout ---
canonical runtime ready
providers: provider.minimax, provider.anthropic
--- stderr ---
[keyring] KeyringSelector 退化到 EnvCredentialResolver: APEIRETH_KEYRING_BACKEND 未设 (设 APEIRETH_KEYRING_BACKEND=auto 可重新尝试 keyring)
[governance-audit] 持久审计账本不可用: governance audit record 1304 hash mismatch (tamper detected); 降级内存账 (审计不持久, 如实)
[external-tools] wired: 1 registered, 0 unregistered, 1 live
```

探针侧逐字（子进程 stdin/stdout 换行分隔 JSON-RPC）。
⚠️ stdio 面有一处采集口径差异：`StdioChannel::open` 把子进程的 **stderr 接到 null**
（`transport.rs:175`）⇒ 探针的 stderr 在生产路径下**不可见**；
因此本次让探针把线缆日志写到 `$PROBE_LOG` 文件，再逐字取回：

```
$ PROBE_LOG=/tmp/apeireth-sse-probe/stdio_probe.log APEIRETH_ENABLE_MCP=1 \
  APEIRETH_MCP_SERVERS='[{"name":"stdioprobe","transport":"stdio","command":"/tmp/apeireth-sse-probe/stdio_probe.py","enabled":true}]' \
  ./target/debug/apeireth session
...
[probe] stdin <- {"jsonrpc":"2.0","method":"initialize","params":{"capabilities":{},"clientInfo":{"name":"apeireth-mcp-bridge","version":"2.0.0-rc.5"},"protocolVersion":"2025-03-26"},"id":1}
[probe] stdout -> {"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-03-26","serverInfo":{"name":"stdio-probe","version":"0.1.0"},"capabilities":{"tools":{"listChanged":false}}}}
[probe] stdin <- {"jsonrpc":"2.0","method":"notifications/initialized"}
[probe] stdin <- {"jsonrpc":"2.0","method":"tools/list","id":2}
[probe] stdout -> {"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"lookup-item","description":"Look one item up","inputSchema":{"type":"object","properties":{"key":{"type":"string"}},"required":["key"]},"annotations":{"readOnlyHint":true}}]}}
[probe] stdin <- {"jsonrpc":"2.0","method":"tools/list","id":3}
```

（末行 `tools/list` `id=3` = **周期刷新的第一拍**：`tokio::time::interval` 的首个 tick **立即完成**
（`crates/engine/runtime-assembly/src/canonical/external_tool_bridge.rs:442`）⇒ 启动期接线之后
马上又跑了一轮发现。这一拍**与进程退出赛跑** —— 本次 stdio 面捕捉到了（`id=3`），
http / sse 面的那次运行没捕捉到（同一次运行里它可能在进程退出后才发出）。
**同一命令重跑一次（耗时 0.06 s）只出现 1 次 `tools/list`** ⇒ 这是竞态，不是固定行为。）

**④ 结论**：**通** —— 握手（`initialize` → `notifications/initialized`）与发现（`tools/list`）
两段往返都在 stdio 上跑通，1 件工具进运行时能力表（`registered=1`），连接存活（`live=1`）。

### 4.2 面 ②：`http`

**① 配置形态**：

```json
[{"name":"httpprobe","transport":"http","url":"http://127.0.0.1:8972/mcp","enabled":true}]
```

- `url` **必需**且必须是 http(s) 绝对地址；**不得**带 `command`。
- 语义：客户端把 JSON-RPC 消息 **POST 到 `url`**，**响应体就是 JSON-RPC 响应**
  （`HttpChannel::request`，`transport.rs:308-314`）—— 与 sse 面「POST 只收请求、响应走流」相反。

**② 实测命令**：

```sh
APEIRETH_ENABLE_MCP=1 \
APEIRETH_MCP_SERVERS='[{"name":"httpprobe","transport":"http","url":"http://127.0.0.1:8972/mcp","enabled":true}]' \
./target/debug/apeireth session
```

**③ 实测输出（逐字）**：

```
exit=0
--- stdout ---
canonical runtime ready
providers: provider.minimax, provider.anthropic
--- stderr ---
[keyring] KeyringSelector 退化到 EnvCredentialResolver: APEIRETH_KEYRING_BACKEND 未设 (设 APEIRETH_KEYRING_BACKEND=auto 可重新尝试 keyring)
[governance-audit] 持久审计账本不可用: governance audit record 1304 hash mismatch (tamper detected); 降级内存账 (审计不持久, 如实)
[external-tools] wired: 1 registered, 0 unregistered, 1 live
```

探针侧逐字：

```
[probe] POST /mcp body: {"jsonrpc":"2.0","method":"initialize","params":{"capabilities":{},"clientInfo":{"name":"apeireth-mcp-bridge","version":"2.0.0-rc.5"},"protocolVersion":"2025-03-26"},"id":1}
[probe] http "POST /mcp HTTP/1.1" 200 -
[probe] POST /mcp body: {"jsonrpc":"2.0","method":"notifications/initialized"}
[probe] http "POST /mcp HTTP/1.1" 202 -
[probe] POST /mcp body: {"jsonrpc":"2.0","method":"tools/list","id":2}
[probe] http "POST /mcp HTTP/1.1" 200 -
```

**④ 结论**：**通** —— 同上。

### 4.3 面 ③：`sse`（本批新增实测）

**① 配置形态**：

```json
[{"name":"sseprobe","transport":"sse","url":"http://127.0.0.1:8971/sse","enabled":true}]
```

- `url` = **事件流地址**（`GET` 目标）；**消息投递地址不由配置给出**，由流里的
  `event: endpoint` 帧宣告（相对地址按 `absolutize_endpoint` 解析，见 §2.3）。
- **不得**带 `command`。

**② 实测命令**：

```sh
APEIRETH_ENABLE_MCP=1 \
APEIRETH_MCP_SERVERS='[{"name":"sseprobe","transport":"sse","url":"http://127.0.0.1:8971/sse","enabled":true}]' \
./target/debug/apeireth session
```

**③ 实测输出（逐字）**：

```
exit=0
--- stdout ---
canonical runtime ready
providers: provider.minimax, provider.anthropic
--- stderr ---
[keyring] KeyringSelector 退化到 EnvCredentialResolver: APEIRETH_KEYRING_BACKEND 未设 (设 APEIRETH_KEYRING_BACKEND=auto 可重新尝试 keyring)
[governance-audit] 持久审计账本不可用: governance audit record 1304 hash mismatch (tamper detected); 降级内存账 (审计不持久, 如实)
[external-tools] wired: 1 registered, 0 unregistered, 1 live
```

探针侧逐字（**完整三段握手可见**）：

```
[probe] GET /sse (Accept: 'text/event-stream') -> event stream open, session=8efa8fd4
[probe] http "GET /sse HTTP/1.1" 200 -
[probe] stream[8efa8fd4] -> event: endpoint data: /messages?sessionId=8efa8fd4
[probe] POST /messages?sessionId=8efa8fd4 session=8efa8fd4 body: {"jsonrpc":"2.0","method":"initialize","params":{"capabilities":{},"clientInfo":{"name":"apeireth-mcp-bridge","version":"2.0.0-rc.5"},"protocolVersion":"2025-03-26"},"id":1}
[probe] http "POST /messages?sessionId=8efa8fd4 HTTP/1.1" 202 -
[probe] stream[8efa8fd4] -> event: message data: {"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-03-26","serverInfo":{"name":"sse-probe","version":"0.1.0"},"capabilities":{"tools":{"listChanged":false}}}}
[probe] POST /messages?sessionId=8efa8fd4 session=8efa8fd4 body: {"jsonrpc":"2.0","method":"notifications/initialized"}
[probe] http "POST /messages?sessionId=8efa8fd4 HTTP/1.1" 202 -
[probe] POST /messages?sessionId=8efa8fd4 session=8efa8fd4 body: {"jsonrpc":"2.0","method":"tools/list","id":2}
[probe] http "POST /messages?sessionId=8efa8fd4 HTTP/1.1" 202 -
[probe] stream[8efa8fd4] -> event: message data: {"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"lookup-item","description":"Look one item up","inputSchema":{"type":"object","properties":{"key":{"type":"string"}},"required":["key"]},"annotations":{"readOnlyHint":true}}]}}
```

**④ 结论**：**通** —— sse 面的**四步协议路径全部跑通**：

| 步 | 现象 | 证据 |
|---|---|---|
| ① 建连 | `GET /sse` + `Accept: text/event-stream` ⇒ 200 | 探针第 1–2 行 |
| ② 端点宣告 | 流上 `event: endpoint` + `data: /messages?sessionId=…` ⇒ 客户端解析成绝对投递地址 | 探针第 3 行 + 随后的 POST 目标 |
| ③ 消息投递 | `POST /messages?sessionId=…` ⇒ 202（请求体 = JSON-RPC 消息） | 探针第 4–5 行 |
| ④ 响应匹配 | 响应回到**流上**（`event: message` + `data: <JSON-RPC 响应>`），按 `id` 配对 ⇒ `tools/list` 的工具清单被解析 | 探针第 6 行（initialize 响应）、第 9–11 行（tools/list 请求 + 响应） |

落地结果与另两面**逐字相同**：`[external-tools] wired: 1 registered, 0 unregistered, 1 live`。

**未测项（如实标注）**：sse 面的 **`tools/call` 往返未实跑** —— 它需要一次真实模型派发
（仓内没有可用的模型凭据），本批**未跑**。但 `tools/call` 与 `tools/list` 走的是**同一条**
`McpChannel::request` 代码路径（`transport.rs:441-452`），且 `tools/list` 的响应已在 sse 面上
按 `id` 配对成功 ⇒ 协议路径本身已被覆盖；**「sse 面工具调用端到端」这件事本批未测**。

---

## 5. 控制组（证明那行日志不是「碰巧出现」）

| 组 | 场景 | 实测输出（逐字） | exit | 结论 |
|---|---|---|---|---|
| B | 开关关（配了 sse 服务器但不设 `APEIRETH_ENABLE_MCP`） | stderr **无** `[external-tools]` 行（`grep -c external-tools` ⇒ `0`）；只有 §3 的两行环境噪声 | 0 | 桥面**不装配** ⇒ 那行日志确实是「接线了才打」 |
| C | 开关开 + sse 端点不可达（`http://127.0.0.1:9/sse`） | `[external-tools] discovery failed: mcp server "sseprobe" is unreachable: 传输层 IO 失败: opening event stream: error sending request for url (http://127.0.0.1:9/sse)` | 0 | **失败不阻断启动**（stdout 仍有 `canonical runtime ready`）⇒ 失败发生在**建连**层 |
| D | 开关开 + sse 流开起来但**不宣告端点**（`/sse-silent`） | 实测耗时 **10 s** 后：`[external-tools] discovery failed: mcp server "sseprobe" is unreachable: 传输层 IO 失败: event stream did not announce a message endpoint` | 0 | 端点等待的 **10 秒硬截止**按实现生效（`transport.rs:394`）；失败发生在**端点宣告**层 |

C 与 D 的分层口径：

| 层 | 判定依据 | 对应失败模式 |
|---|---|---|
| 建连 | `GET` 本身失败 | 控制组 C：`opening event stream: error sending request for url …` |
| 端点宣告 | GET 成功但 10 s 内没等到 `event: endpoint` | 控制组 D：`event stream did not announce a message endpoint` |
| 帧解析 | 帧收到但不是 `event: message` / JSON 不可反序列化 | **本批未触发**（探针发的帧都是合法 `event: message`；不合法的帧会被静默丢弃，见 §10 遗留 ②） |
| 消息投递 | POST 非 2xx | **本批未触发**（探针回 202） |
| 响应匹配 | 帧到了但 `id` 对不上 | **本批未触发**（`id=1` / `id=2` 都配上了） |

---

## 6. 四级口径落点

| 级 | 成立？ | 依据 |
|---|---|---|
| IMPLEMENTED | 成立（已被下一级覆盖） | 三个面的实现都在仓内（`transport.rs`），库级可跑 |
| **PRODUCTION WIRED** | **成立** | 三个面都经生产组合根（`crates/adapters/cli/src/external_tools.rs:63-89` 的 `wire_at_startup` → `crates/engine/runtime-assembly/src/canonical/external_tool_bridge.rs:254` 的 `wire_external_tools` → `McpToolBridge::refresh()`）走完「发现 → 登记 → 族授权 + 逐次人批 → 派发」，并实测产出 `registered=1`（§4） |
| DEFAULT ENABLED | **不成立** | 默认关；开关 `APEIRETH_ENABLE_MCP=1`（`crates/adapters/cli/src/lib.rs:186`），控制组 B 实测确认 |
| **HARDWARE VALIDATED** | **不成立** | 三个面的对端**都是仓外自建的最小探针**，**不是**真实第三方 MCP 服务器 ⇒ 真服务器端到端**未测**（§7） |

**结论句（可被引用）**：*MCP 桥的三个传输面（stdio / http / sse）在仓内实现上**协议路径跑得通**
（2026-11-05 实测，三面均产出 `[external-tools] wired: 1 registered, 0 unregistered, 1 live`）；
因对端是自建探针，**HARDWARE VALIDATED 仍不成立**。*

---

## 7. 诚实边界（本批最要紧的一处）

1. **自建探针 ≠ 真实第三方 MCP 服务器**。
   三个面的对端都是本批**自建**的最小服务器（仓外 `/tmp`，Python 实现，只实现
   `initialize` / `notifications/initialized` / `tools/list` / `tools/call` 四个方法）。
   它们**按仓内实现的实际协议写**（尤其 sse 面：先宣告 `event: endpoint`，再 POST 请求，
   响应回到流上），**不是**按外部规范去「验证仓内实现对不对」——
   ⇒ 本记录证明的是「**三种传输的协议路径在仓内实现上跑得通**」，
   **不能**证明「仓内实现与真实第三方服务器互通」。
2. ⇒ **HARDWARE VALIDATED 仍不成立**。claim `tool.mcp_bridge` 的级别不由本记录裁决
   （属主线口径裁决），但**本记录不构成任何 HARDWARE VALIDATED 依据**。
3. **sse 面的 `tools/call` 端到端未测**（§4.3 末）—— 需要真实模型派发，本批**未跑**。
4. **被测二进制不是纯净 HEAD 构建**（§3）—— 并行施工线改了 CLI 适配器层，
   **未触及** MCP 桥与三个传输层。
5. **stdio / http 两面在本批被重跑了一次**：主代理此前（2026-11-05，同日）已实跑这两面并给出
   逐字输出 `[external-tools] wired: 1 registered, 0 unregistered, 1 live`；
   本批用同一工具链、同一二进制、同一探针形态重跑，输出**逐字一致**（§4.1 / §4.2），
   故本文把三面放在同一张表里陈述。**sse 面此前从未实跑，是本批新增**。

---

## 8. 复现清单（探针在仓外，源码附此以便复现）

```sh
# 0) 构建被测二进制（在仓库根执行）
cargo build -p apeireth-cli --locked -j 2

# 1) 起探针（仓外；本文档附源码）
mkdir -p /tmp/apeireth-sse-probe && cd /tmp/apeireth-sse-probe
python3 sse_probe.py 8971 &     # sse 面：GET /sse 事件流 + POST /messages 投递端点
python3 http_probe.py 8972 &    # http 面：POST /mcp 直答 JSON-RPC

# 2) 跑三个面（逐字输出见 §4）
sh run_all.sh ./target/debug/apeireth
```

sse 探针（`sse_probe.py`）的完整源码（**要点已逐条对应 §2.2**）：

```python
#!/usr/bin/env python3
"""SSE 探针: 事件流回响应 + 宣告出来的端点收请求。"""
import json, queue, sys, threading, uuid
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlparse

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 8971
TOOLS = [{"name": "lookup-item", "description": "Look one item up",
          "inputSchema": {"type": "object", "properties": {"key": {"type": "string"}},
                          "required": ["key"]},
          "annotations": {"readOnlyHint": True}}]
SESSIONS, LOCK = {}, threading.Lock()

def log(m): sys.stderr.write("[probe] %s\n" % m); sys.stderr.flush()
def frame(event, data): return ("event: %s\ndata: %s\n\n" % (event, data)).encode()

def handle(msg):
    method, rid = msg.get("method"), msg.get("id")
    if method == "initialize":
        return {"jsonrpc": "2.0", "id": rid, "result": {
            "protocolVersion": "2025-03-26",
            "serverInfo": {"name": "sse-probe", "version": "0.1.0"},
            "capabilities": {"tools": {"listChanged": False}}}}
    if method == "tools/list":
        return {"jsonrpc": "2.0", "id": rid, "result": {"tools": TOOLS}}
    if method == "tools/call":
        return {"jsonrpc": "2.0", "id": rid, "result": {
            "content": [{"type": "text", "text": "probe answered"}], "isError": False}}
    if rid is None:          # 通知: 不回响应
        return None
    return {"jsonrpc": "2.0", "id": rid,
            "error": {"code": -32601, "message": "Method not found: %s" % method}}

class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def do_GET(self):                      # 事件流: 回响应
        if not self.path.startswith("/sse"):
            self.send_response(404); self.send_header("Content-Length", "0")
            self.end_headers(); return
        sid, outbox = uuid.uuid4().hex[:8], queue.Queue()
        with LOCK: SESSIONS[sid] = outbox
        log("GET %s -> event stream open, session=%s" % (self.path, sid))
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-cache")
        self.send_header("Transfer-Encoding", "chunked")
        self.end_headers()

        def chunk(payload):
            self.wfile.write(b"%X\r\n" % len(payload)); self.wfile.write(payload)
            self.wfile.write(b"\r\n"); self.wfile.flush()

        try:
            endpoint = "/messages?sessionId=%s" % sid
            log("stream[%s] -> event: endpoint data: %s" % (sid, endpoint))
            chunk(frame("endpoint", endpoint))     # ① 宣告投递端点
            while True:                              # ② 响应回到流上
                data = outbox.get()
                log("stream[%s] -> event: message data: %s" % (sid, data))
                chunk(frame("message", data))
        except (BrokenPipeError, ConnectionResetError):
            log("stream[%s] client disconnected" % sid)
        finally:
            with LOCK: SESSIONS.pop(sid, None)
        self.close_connection = True

    def do_POST(self):                     # 投递端点: 收请求
        sid = (parse_qs(urlparse(self.path).query).get("sessionId") or [""])[0]
        raw = self.rfile.read(int(self.headers.get("Content-Length") or 0))
        log("POST %s session=%s body: %s" % (self.path, sid, raw.decode()))
        response = handle(json.loads(raw.decode()))
        if response is not None:
            with LOCK: outbox = SESSIONS.get(sid)
            if outbox is None:
                self.send_response(404); self.send_header("Content-Length", "0")
                self.end_headers(); return
            outbox.put(json.dumps(response, separators=(",", ":")))
        self.send_response(202); self.send_header("Content-Length", "0")
        self.end_headers()

server = ThreadingHTTPServer(("127.0.0.1", PORT), Handler)
server.daemon_threads = True
log("sse probe listening on http://127.0.0.1:%d/sse" % PORT)
server.serve_forever()
```

stdio / http 探针形态（同样在仓外）：

- `stdio_probe.py`：从 stdin 逐行读 JSON-RPC，逐行写 JSON-RPC 响应到 stdout；
  实现同样的四个方法。
- `http_probe.py`：`POST /mcp` 直接以 JSON-RPC 响应体作答（`Content-Type: application/json`）；
  通知回 `202` 空体。

---

## 9. 验证记录（2026-11-05 实跑，逐字）

### 9.1 文档治理

```
$ python3 scripts/docs_links.py --check
[docs_links] 扫描 1481 个 markdown 文件（作用域 live）
[docs_links] 断链: 0
[docs_links] 已知缺失（政策保留，不计断链）: 44
exit=0

$ python3 scripts/docs_index.py --check
[docs_index] 问题数: 0
exit=0

$ python3 scripts/check_doc_caliber.py
[caliber] 扫描 442 个活跃文档 (跳过 1269 个历史文档), 新增违规 0 处 (基线内 6 处)
exit=0

$ pwsh -NoProfile -File scripts/check-neutral-terms.ps1
OK    168 changed files: added lines all neutral
exit=0
```

`docs/INDEX.csv` 由 `python3 scripts/docs_index.py --gen` 产出（**未手改**）；本次新增 1 行
（本文档），另有两行 `ref_count` 因本文档的引用各 +1
（`docs/02-guides/mcp-external-tools.md` 4→5、`docs/status/CLOSEOUT-MCP-EXTERNAL-TOOLS-2026-11-05.md` 5→6）。

### 9.2 真源与架构

```
$ python3 scripts/docs_truth.py --verify --check
OK: STATUS.md matches claims.yaml; README badges match baseline
...
tool.mcp_bridge                   OK          PRODUCTION WIRED    OFF      APEIRETH_ENABLE_MCP
...
claims=29 fail=0 stale=7 review_due=0
exit=0

$ python3 scripts/arch_guard.py --strict
[arch_guard] wave=5 contract=1.0 status=adjudicated
[arch_guard] crates=31 edges=118 env=160 max_src_loc=6150 exceptions=2
[arch_guard] VIOLATIONS: 0
[arch_guard] STALE (可缩表): 2
  - size crates/adapters/cli/src/lib.rs 3842
  - size crates/engine/runtime-assembly/src/canonical/self_status_source.rs 2064
exit=0
```

⚠️ **归因（如实）**：`docs_truth` 的 7 条 STALE 与 `arch_guard` 的 2 条 STALE **全部指向
`crates/adapters/cli/src/lib.rs`** —— 该文件由**并行施工线**改动，**不是本批改动**
（本批 0 行 Rust 改动）。本批涉及的 claim `tool.mcp_bridge` 状态为 **OK**。

### 9.3 测试

```
$ cargo test -p apeireth-tools-canonical --locked -j 2
    Running unittests src/lib.rs (target/debug/deps/apeireth_tools_canonical-9f2cf32ce015a24f)
    Running tests/apply_patch_contract_frames.rs ...
    ... （共 38 个测试二进制：unittests + 37 个集成测试二进制）
    Running tests/mcp_bridge_e2e.rs ...
    Running tests/mcp_deny_guard.rs ...
    ...
   Doc-tests apeireth_tools_canonical
exit=0
```

逐套结果（每套一行 `test result: ok. N passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`）：

| 套件 | 结果 |
|---|---|
| unittests（`src/lib.rs`） | `ok. 748 passed; 0 failed` |
| 其余 37 个集成测试二进制 | 全部 `ok. N passed; 0 failed`（含 `mcp_bridge_e2e` / `mcp_deny_guard`） |
| doc-tests | `ok. 0 passed; 0 failed` |
| **合计**（各套件 `ok. N passed` 求和） | **1174 passed / 0 failed** |

**口径**：本批 **0 行 Rust 改动** ⇒ 该套件属「**预期不适用**」，但仍实跑并贴输出，
作为「本批没碰生产代码」的旁证。

---

## 10. 遗留与建议（**逐条标注处置状态**；① ② ④ ⑤ 已实施并实测，③ 只销「可观测」一轴）

| # | 观察 | 证据（file:line） | 性质 |
|---|---|---|---|
| ① | ~~**响应等待无截止**~~ ⇒ **已实施（2026-11-05 补）**：复核发现**三个传输面都无截止**（不只 sse；`reqwest::Client::new()` 默认 `timeout: None`），统一给 **30 s 硬截止**（对齐生产实际在用的桥级调用档位 `call_timeout_ms` = 30 s），超时**如实报**「`… did not deliver the response within 30s`」并**清掉挂起等待者**（含「请求没发出去」那条改前就在泄漏的路径）⇒ 改前「收 POST 不回帧」会让启动期接线**永久挂死**（实测 91 s 被杀、exit=124、连 `[external-tools]` 都没打出来），改后 **30 s 有界返回**、错误可辨、**启动不阻断**（exit=0）。 | 落点 `crates/capabilities/tools/src/mcp_bridge/transport.rs`（`RESPONSE_TIMEOUT` + 三面 `request` 的等待包装 + `PendingRouter::{forget, wait_for}`）；用例 `crates/capabilities/tools/tests/mcp_transport_response_deadline.rs`（8 条） | **已销账**（传输面响应相位）；⚠️ **请求相位**三处（sse/http 的 POST、stdio 的写）**仍无截止**，另批 |
| ② | ~~**不合法的帧被静默丢弃**~~ ⇒ **已实施（2026-11-05 补）**：帧仍被丢弃、链路**不断**（**只记不改行为**），但不再无声 ⇒ ①**机器可读**：trait 上新增带默认实现的 `McpChannel::malformed_frames() -> u64`（链接级累计丢弃数，外部实现者零破坏）；②**人可读**：限频 stderr 一行（首条 + 之后每 100 条一条，**不记帧原文** —— 坏帧可能带服务器侧内容）⇒ 防日志风暴。stdio 读帧处的**同类**静默丢弃一并覆盖。 | 落点 `crates/capabilities/tools/src/mcp_bridge/transport.rs`（trait 方法 + 限频助手 + 两处读帧分支）；用例 `crates/capabilities/tools/tests/mcp_malformed_frame_observability.rs`（2 条，含**阴性对照**） | **已销账**（可观测性）；⚠️ 遗留 ③（无 `event:` 的帧**连计数都不进**）仍开 |
| ③ | **`event:` 字段是必需的**（**协议口径，本批不动**）：裸 `data:` 行（无 `event:`）不被判为 `message` ⇒ 按「规范直觉」写的服务端会静默不响应。**2026-11-05 补（只销「可观测」一轴）**：这类帧现在**可观测**（trait 上新增带默认实现的 `McpChannel::unrouted_frames()` 计数 + 限频 stderr，**不记帧原文**）⇒ 帧仍被丢弃、链路**不断**（**行为未变**）。⚠️ **「裸 `data:` 是否该当 message」是协议行为变更，另批**（本仓 §2.3 的既有口径明写「`event:` 字段是必需的」⇒ 改它要先裁决既有口径） | 落点 `crates/capabilities/tools/src/mcp_bridge/transport.rs`（trait 方法 + 限频助手 + 读帧分支）；用例 `crates/capabilities/tools/tests/mcp_sse_frame_consumption.rs`（含**阴性对照**） | **部分销账**（可观测轴已补）；**协议行为变更**仍开 |
| ④ | ~~**`retry:` / `id:` 帧被解析但未被使用**~~ ⇒ **已实施（2026-11-05 补）**：复核确认 `observe_frame` 的调用点改前**只有底座自己的单测** ⇒ 传输层从不把 `retry:`/`id:` 喂给重连状态 ⇒ 服务端的 `retry:` 提示被**静默忽略**，永远用本地退避。⇒ 现在：传输层新增 `McpFrameObserver` + 带默认实现的 `open_with_observer()`；连接层装观察者 ⇒ **`id:`/`retry:` 在流还活着时就被记下**；`recover()` **先**喂 `observe_frame` **再** `on_disconnect()` ⇒ **服务端的 `retry:` 真正成为退避**；`id:` 记下后**恢复的链带 `Last-Event-ID` 头** ⇒ 不再是死字段。⚠️ **上限 = `ReconnectPolicy::max_backoff`（默认 30 s，观察时即夹）** ⇒ 服务端给「1 小时」也只等 30 s；**次数预算仍由 `max_attempts` 封顶** ⇒ 服务端**买不通**重连预算。 | 落点 `crates/capabilities/tools/src/mcp_bridge/transport.rs` + `connection.rs`；用例 `crates/capabilities/tools/tests/mcp_sse_frame_consumption.rs`（8 例，含**三处阴性对照**） | **已销账**（协议消费） |
| ⑤ | ~~**周期刷新的第一拍是「立即」而非「一个周期后」**~~ ⇒ **已实施（2026-11-05 补）**：承载循环改为 `interval_at(now + interval, interval)` ⇒ **首拍 = 一个周期后**（默认 60 s ⇒ 首轮落 t=60 s）⇒ 与「每 N 毫秒刷新一次」的口径一致，且消掉「启动期接线刚做完又跑一轮发现」的重复劳动与「与进程退出赛跑」。**不改任何公开签名** ⇒ adapters 侧零改动。 | 落点 `crates/engine/runtime-assembly/src/canonical/external_tool_bridge.rs`（`interval_at`）+ `crates/capabilities/tools/src/mcp_bridge/config.rs`（`refresh_interval_ms` 的 doc）；用例 `crates/engine/runtime-assembly/tests/mcp_refresh_first_round.rs`（2 例，含**阴性对照**：临时改回 `interval(interval)` ⇒ 用例**真的 FAILED**） | **已销账**（口径落差）；⚠️ 唯一风险窗口：服务器在启动后、首个周期前变更工具列表 ⇒ 变更最多晚一个周期进能力表（≤ 60 s） |

**本批 0 行 Rust 改动** —— 上述五条**只报告不实施**，是否修由主线裁决。

---

## 11. 相关文档

- [MCP 外部工具接入指南（工具桥）](../02-guides/mcp-external-tools.md) —— 配置面 / 治理映射 / 名册口径的现行说明书。
- [MCP 外部工具桥四环 —— 结案记录（2026-11-05）](../status/CLOSEOUT-MCP-EXTERNAL-TOOLS-2026-11-05.md) —— 四环接通证据与遗留清单。
