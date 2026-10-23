# IM 社交通道模块（未来专项）—— 决策与范围

> 日期：2026-10-23 · 状态：**暂缓（owner 裁定）；作为产品面模块另立专项**
> 相关：未接线台账 W1、`docs/02-guides/mcp-external-tools.md`、
> `crates/adapters/gateway/src/im_channels.rs` / `im_bridge.rs` / `im_approval.rs`

## 1. 为什么现在记这一页

设备受控执行（K 轴 L3/L4）的 **money 档**（购物 / 下单）设计要求：
每次执行都要一张 IM 审批卡送达回执（`im_card_id`），缺卡即拒。

代码面其实齐了：

| 资产 | 位置 | 状态 |
|---|---|---|
| 三种渠道枚举（飞书 / 企业微信 / QQ） | `crates/adapters/sdk/src/im/channel.rs` | ✅ |
| HTTP 发送器（发文本 / 发审批卡） | `crates/adapters/sdk/src/im/transport.rs`（`ImHttpSender`） | ✅ |
| 渠道配置解析（`APEIRETH_IM_CHANNELS` / `im-channels.json`） | `crates/adapters/gateway/src/im_channels.rs` | ✅ |
| 审批卡四态闭合 + 审计对 | `im_approval.rs` / `chain_audit.rs` | ✅ |
| **生产装配里构造 `ImBridge` 并把设备审批面注入** | —— | ❌ 缺这一根线 |

owner 2026-10-23 裁定：**暂不接**（没有现成的 IM 机器人，且这件事应当作为
「Apeireth 能在社交软件里和用户自由聊天」的**产品面模块**整体设计后再做，
而不是为了一个审批卡临时插一条线）。

## 2. 因此当前的如实状态

- **money 档动作当前不可用**：缺 `im_card_id` 即拒（`device_control.rs`）。
  这是**有意的 fail-closed**，不是故障 —— 动钱的通道宁可不给，也不给个能绕过的。
- 其余两档不受影响：`reversible`（灯/空调类）按审批档正常可用；
  `irreversible`（开火/开门/物理删除）逐次人批，不走 IM 卡。
- 台账 W1 已按此登记，不再以「待接线」挂着装样子。

## 3. 模块范围（未来专项的设计输入）

要做的是「Apeireth 在真实聊天场景里和人自由对话」这一整块，不只是发卡：

1. **渠道接入**：飞书 / 企业微信 / QQ 三选一先跑通（webhook 出站 + 事件入站）；
2. **身份与归属**：谁在跟哪个伙伴说话 —— 需与 ADR-0012 的主体裁定对齐
   （中央 AI 按人分区；每户伙伴独立），群聊里多用户身份如何映射是核心设计点；
3. **会话语义**：群聊 vs 私聊、多轮上下文边界、@ 触发规则、消息分段与限流；
4. **审批面复用**：设备审批卡是「聊天通道」的一个特例消费者 —— 先有通道，
   再把卡挂上去，而不是为卡造通道；
5. **隐私与出境**：聊天内容出境需过 `redaction_bridge` 的脱敏面与显式开关；
6. **安全**：入站请求验签（`secret`）、重放防护、限速、越权拒绝即帧。

## 4. 触发条件

出现下面任一情况即立项：

- owner 有了常用的 IM 机器人 / 明确要用某个 IM 作为交互入口；
- 或产品面进入「桌面之外的第二入口」阶段（发布 checklist 的后续波次）。

在此之前，设备 money 档维持「缺卡即拒」的不可用状态，文档与台账如实标注。
