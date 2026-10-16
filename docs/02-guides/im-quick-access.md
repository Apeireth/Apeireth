# IM 快捷接入指南（消息桥 + 审批卡片）

本文说明如何把桌面伙伴接进 IM：三个渠道（`im-feishu` / `im-wecom` / `im-qq`，
中性渠道 id）如何配置、双向消息桥如何工作、以及核心卖点——**把工具审批卡片
转发到手机，点按钮完成治理闭合**。

实现位置：

| 层 | 位置 | 职责 |
|---|---|---|
| wire 面 | `crates/adapters/sdk/src/im/` | 渠道 kind、入站信封解码、出站信封组装、审批卡片渲染、分段/截断、出站传输（含断线重连）、共享秘密脱敏 |
| 配置面（件一） | `crates/adapters/gateway/src/im_channels.rs` | `APEIRETH_IM_CHANNELS` / `im-channels.json`、启动装配、脱敏启动日志 |
| 消息桥（件二） | `crates/adapters/gateway/src/im_bridge.rs` | 会话映射持久、文本 → 会话回合 → 分段回流、HTTP 入站端点 |
| 审批闭合（件三） | `crates/adapters/gateway/src/im_approval.rs` | 审批卡片、按钮回调、四态闭合、审计配对原子、超时语义 |

SDK 侧 wire 面只**消费**既有 IM 系适配族（`crates/adapters/sdk/src/lark/`）：
`im-feishu` 渠道的入站信封解析复用其 webhook 信封解析（事件订阅回调的三种
信封形状 + 重放窗口），文本/卡片内容形状复用其消息内容类型，消息预算锚
`MAX_MESSAGE_TEXT_BYTES`。

## 件一：快捷接入配置面

### 1. 写渠道配置

主入口是环境变量 `APEIRETH_IM_CHANNELS`，值为 JSON 数组：

```json
[
  {
    "id": "primary",
    "kind": "im-feishu",
    "webhook_or_endpoint": "https://open.example.test/hook/xxxx",
    "secret": "shared-signature-secret",
    "enabled": true
  },
  {
    "id": "mobile",
    "kind": "im-wecom",
    "webhook_or_endpoint": "https://open.example.test/hook/yyyy",
    "enabled": true
  }
]
```

- `id`：渠道 id（≤ 64 字节，不重复），命名启动日志与会话映射；
- `kind`：`im-feishu` / `im-wecom` / `im-qq`（闭合三值，未知值 fail-closed）；
- `webhook_or_endpoint`：出站 webhook / endpoint，必须是绝对 http(s) URL；
- `secret`：可选的入站校验共享秘密（**0 明文进日志**）；
- `enabled`：显式给出，不设默认。

次入口是数据目录里的 `im-channels.json`，走存储文档信封
（`{"name":"im-channels","version":1,"compatible_versions":[1],"body":{"channels":[...]}}`），
`StoredDoc` 单文档拒开语义加载：坏 JSON / 串档 / 版本不可读一律**拒绝打开**，
不静默回退默认值：

```rust
use apeireth_core::stored_doc;
use apeireth_gateway::{doc_compat, im_channels_path, ImChannelConfig, ImChannelSpec};

let body = ImChannelConfig::new(vec![/* ... */]);
stored_doc::save_single(&im_channels_path(&data_dir), &doc_compat(), body, stored_doc::DEFAULT_DOC_MODE)?;
```

优先级：`APEIRETH_IM_CHANNELS`（若设置则必须合法）> 数据目录文件 > 空配置。
任一入口解析/校验失败都是启动错误（fail-closed）。

### 2. 启动装配、断线重连、脱敏启动日志

```rust
use apeireth_gateway::{assemble_im_channels, ImChannelConfig};
use apeireth_sdk::im::ImReconnectPolicy;

let config = ImChannelConfig::load(Some(&data_dir))?;
let assembly = assemble_im_channels(&config, ImReconnectPolicy::default())?;
for line in &assembly.startup_log {
    tracing::info!("{line}");   // 秘密只出现 configured / none
}
```

- **装配**：只保留 `enabled: true` 的渠道为传输目标（`ImChannelTarget`）；
  有坏渠道即整体失败（fail-closed，不跳过）；
- **断线重连**：出站传输共用一份 `ImReconnectPolicy`（指数退避 + 次数上限），
  只有可重试错误（网络失败 / 429 / 5xx）才重连，永久拒绝不重试；
- **脱敏**：`redacted_startup_log` 一行一渠道，`secret=configured|none`，
  端点里的秘密类 query 参数值替换为 `[redacted]`。

## 件二：双向消息桥

### 入站 → 会话回合 → 回复回流

1. IM 平台把回调体 POST 到 `POST /v1/im/channels/{channel_id}/events`
   （路由由 `apeireth_gateway::im_router` 提供；未配置渠道时不挂载）；
2. 渠道配置了 `secret` 时，请求头 `X-Im-Signature` 必须等于
   `sha256=<hex(sha256(secret || "\n" || body))>`（恒定时间比较，失败只报
   `signature mismatch`）；
3. 信封按 kind 解码成归一化事件：文本消息 / 卡片按钮 / 握手挑战；
4. 文本消息 → **会话回合**（走既有对话链：生产实现
   `CanonicalChainHandler` 直连 `execute_chat` / canonical 审批路径）；
5. 回复按**显式分段策略**流回 IM：段边界 先空行、再换行、再字符硬切；
   每段 ≤ 该 kind 的消息预算；段数超预算时尾部丢弃并计
   `dropped_chars`，末段追加截断标记（`ImSegmentPolicy`）。

### 会话 id 映射（同源 + 持久）

IM 会话与桌面会话是**同一个** `SessionId`：首次见到的 `(channel_id,
conversation_id)` 铸一个桌面会话并写进数据目录 `im-sessions.json`
（存储文档拒开语义；`ImSessionMapStore`）。同一 IM 会话的后续消息、按钮回调
都落到同一会话历史；桌面侧的待审批也能通过映射反查该转发到哪个 IM 会话。

## 件三：审批卡片（杀手锏）

### 卡片长什么样

工具审批事件 → IM 审批卡片（`ImApprovalCard`）：

- **命令文本**（`command_text`，一行看懂要执行什么）；
- **风险级**（治理词表 `info/low/medium/high/critical/nuclear`；确定性推导：
  治理文本里的词表标签取最高位，无标签或低于 `high` 时按 `high` 基线显示；
  这是展示推导，不是第二治理判定）；
- **批准 / 拒绝按钮**（按钮回调载荷只带闭合身份：`approval_ref` / `pair_id` /
  `round` / `subject` + 本地超时戳 `expires_at_ms`；展示面 ≠ 授权面）。

### 点按钮 → 治理闭合（四态 + 审计配对原子）

按钮回调经桥进入 `ImApprovalCloser::close`：

| 按钮/情形 | `ApprovalOutcome`（四态词表） | 执行 |
|---|---|---|
| 批准 | `allowed_once` | 恰好一次（冻结操作，不换参数） |
| 拒绝 | `rejected` | 否（fail-closed） |
| 取消 | `cancelled` | 否（fail-closed） |
| 超时 / 中断 / 查无 / 失败 | `unavailable` | 否（fail-closed） |

- **四态语义即既有 `approval_closure` 词表**（`ApprovalOutcome`），
  0 第二套词表；只有 `allowed_once` 授权，且只授权它自己那一个操作一次；
- **审计配对原子**：每个闭合把 `approval.asked` ↔ `approval.decision` 一对
  记录经 `commit_approval_audit_pair` 一次提交（双写落地或整对回滚）；
  落盘审计 `FileApprovalAudit`（jsonl）可重开复核 `detect_unclosed_approval_pairs`；
- **超时语义与本地一致**：按钮载荷带本地审批的 `expires_at_ms`，到点即
  `unavailable` 且不进解析器；本地 canonical 分辨率 `expired` 同样映射
  `unavailable`；
- **一次性**：同一 `pair_id` 只闭合一次，重复点击不二次执行、不二次记账；
  canonical 侧的 `AlreadyResolved` 也直接短路；
- **未配置 IM 时本地审批零回归**：无渠道则装配 inert、路由不挂载，本地
  `execute_chat` / `resolve_approval` 原路径不受影响。

## 测试证据（`crates/adapters/gateway/tests/`，全部 mock IM 端点，0 真实网络）

| 测试 | 证据 |
|---|---|
| `im_channel_config.rs::env_channels_assemble_with_a_redacted_startup_log` | env 加载 + 装配 + 启动日志脱敏 |
| `im_channel_config.rs::broken_channel_configuration_fails_closed` | 坏配置（未知 kind / 相对端点 / 重复 id / 缺字段）拒开 |
| `im_channel_config.rs::stored_channel_file_is_loaded_and_broken_file_refuses_to_open` | `im-channels.json` 好档照读、坏档拒开 |
| `im_channel_config.rs::shared_secret_never_reaches_startup_logs` | 秘密 0 明文进日志 |
| `im_channel_config.rs::outbound_delivery_reconnects_after_transient_failures` | 断线重连预算（瞬时失败重试送达 / 预算耗尽） |
| `im_channel_config.rs::no_configured_channels_assembles_to_an_inert_surface` | 未配置 = inert |
| `im_channel_config.rs::env_entry_wins_over_the_data_directory` | env 主入口优先于数据目录 |
| `im_message_bridge.rs::inbound_text_turns_into_a_reply_on_the_mock_endpoint` | 文本双向（入站 → 回合 → 回复回流） |
| `im_message_bridge.rs::long_replies_segment_and_truncate_with_an_explicit_marker` | 分段/截断策略显式 |
| `im_message_bridge.rs::im_conversation_maps_to_one_desktop_session_and_persists` | 会话 id 映射同源 + 持久 |
| `im_message_bridge.rs::inbound_signature_is_verified_without_leaking_the_secret` | 签名校验 + 错误 0 明文秘密 |
| `im_message_bridge.rs::the_real_conversation_chain_serves_the_bridge` | 走既有对话链（真实 canonical 回合） |
| `im_message_bridge.rs::unknown_channels_and_unmapped_conversations_fail_closed` | 未知渠道/未映射会话 fail-closed |
| `im_message_bridge.rs::failed_inbound_never_posts_to_the_endpoint` | 入站失败 0 出站副作用 |
| `im_approval_cards.rs::approval_card_carries_command_text_risk_level_and_two_buttons` | 卡片生成（命令文本/风险级/双按钮） |
| `im_approval_cards.rs::approve_button_closes_allowed_once_and_runs_the_frozen_tool_once` | 按钮回调 → `allowed_once` + 恰好一次 + 审计配对 |
| `im_approval_cards.rs::reject_button_fails_closed_with_rejected_outcome` | 拒绝 fail-closed |
| `im_approval_cards.rs::closure_vocabulary_maps_every_button_onto_four_states` | 四态闭合全可达 |
| `im_approval_cards.rs::expired_cards_close_unavailable_without_execution` | 卡片超时语义 |
| `im_approval_cards.rs::local_expiry_maps_to_the_same_unavailable_closure` | 本地超时 ↔ IM 侧同一四态 |
| `im_approval_cards.rs::unconfigured_im_channels_leave_the_local_approval_flow_untouched` | 未配置 IM 本地审批零回归 |
| `im_approval_cards.rs::paired_audit_records_persist_and_reopen_as_complete_pairs` | 审计配对原子 + 落盘复核 |
| `im_approval_cards.rs::the_closer_never_commits_a_second_pair_for_one_round` | 一次性闭合 |

SDK wire 面另有 27 个单测（`crates/adapters/sdk/src/im/`：kind 词表 / 入站解码 /
卡片渲染 / 分段截断 / 重连预算 / 秘密脱敏 / 签名）。

## 新符号速查

- SDK：`ImChannelKind`、`ImChannelTarget`、`ImSecret`、`ImInboundEvent`、
  `ImInboundMessage`、`ImCardAction`、`ImButtonPayload`、`ImApprovalCard`、
  `ImSegmentPolicy`、`ImSegmentation`、`ImSender`、`ImHttpSender`、
  `ImReconnectPolicy`、`ImError`、`ImErrorClass`；
- 网关：`ImChannelSpec`、`ImChannelConfig`、`ImChannelAssembly`、
  `assemble_im_channels`、`ImSessionMapStore`、`ImTurnHandler`、
  `CanonicalChainHandler`、`ImBridge`、`im_router`、`ImApprovalNotice`、
  `ImApprovalCloser`、`ImApprovalClosure`、`ImApprovalResolver`、
  `ImApprovalAuditCommit`、`MemoryApprovalAudit`、`FileApprovalAudit`。
