# 1. 三套通知系统 — O-5 不假装

```
[Document-Meta]
Document: docs/architecture-v4-2-r145-modules/01-channel.md
Version: R145-Init
R-Cycle: R145
Last-Modified: 2026-08-12
Status: 🟢 活跃
```

## 设计

`Channel` enum: `Ai` / `Human` / `Both` 3 variant, 编译期 `CHANNEL_COUNT = 3` 守门.

## 为什么是 3 (不是 2/5/8)

VCP v1.1 官方定义 3 套:
- AI 通知栏 (AI 不可见 / 人类不可见)
- VCPLog (AI 不可见 / 人类可见)
- VCPInfo (双方可见)

3 是认知最小正交基底:
- 2 类 = 维度不够 (AI 看见 vs 人类看见, 缺"双方都看")
- 5/8 类 = 加复杂度无新维度

## 借鉴 vs 上升

| VCP | 我们 |
|---|---|
| 3 套字符串 | `Channel` enum 编译期 |
| 主题字符串手动加前缀 | `ChanneledBus::publish` 自动加 `ai:` / `human:` / `both:` |
| 多套 broadcast | 1 个 L0Bus + 3 套前缀 |
| 跨通道泄漏风险 | 编译期 `ChannelSet` 位运算守门 |

## 哲学基础

**O-5 不假装**: 透明分流是"不假装"的具体表现 — 哪些信息 AI 真的看到, 哪些是人类真的看到, 不混.
**S-2 实事求是**: 双向可见 ≠ 双向接收. VCPInfo 是双方可见, 但 AI 仍可"看不到" (如 UI 进度条是光学反馈).

我们 `Channel::Both` = 双方可见且 AI 可订阅, 区分了"可见"和"接收".

## 局限

- 单进程 inproc (L0) — 跨进程需换 L3/L4
- 主题前缀是字符串拼接, 未来可改 RingBuffer

## 内部参考

- 实现: [`crates/apeireth-bus/src/channel.rs`](../../crates/apeireth-bus/src/channel.rs)
- L0 总线: [`crates/apeireth-bus/src/l0.rs`](../../crates/apeireth-bus/src/l0.rs)
- 索引: [`README.md`](README.md)
