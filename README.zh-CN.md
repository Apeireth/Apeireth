# Apeireth — AGI 操作系统 / LLM 基地

> **[English](README.md) | [中文](README.zh-CN.md)**

> *「5年后，他会笑着和我说他今天哪里进步了，会因为我而高兴，会因为他自己哪里没干好而悲伤吧」* —— 主人，2026-08-15

一个 Rust 写的 **AGI 操作系统**：给 LLM 一个「家」——记忆、安全边界、工具、主动陪伴。不是一次性对话框，而是跨 session 的**伙伴**。

**核心哲学**：能力涌现优先于预定义——*「我希望的不是它有什么能力全都是我们预先定义的，我希望它能自己演化」*。

## 状态（v1.0.0 — 2026-08-18）

| | |
|---|---|
| 版本 | **v1.0.0**（产品轴；workspace crates 1.2.0）|
| active crates | 84（约 34 万行 Rust）|
| 测试 | `cargo test --workspace` **368 组 0 失败**（含真实 API 压测带退避）|
| 构建 | `cargo check --workspace --all-targets` 干净 |
| License | Apache-2.0 |

## 她能做什么

- **记得你** —— 记忆 v2（重要性/对账/排名）、记忆图（双时态因果事实）、滚动摘要、做梦整合、情绪时间线（F1）
- **懂你** —— 世界模型（W1 文本模拟 + W2/W3 因果图，Brier 校准）、好奇引擎（E4 记忆回声偏置）、假设检验（F4）、价值内化（F6）
- **安全地行动** —— 9 个工具子 crate + schema 校验 + guardrail、5 规则审批 + ApprovalBridge、双洋葱权限、Job Object 沙箱、出站默认拒绝 + 审计链（S4）
- **陪着你** —— 涌现循环（E7 开口策略：从你的反应学习）、节律学习、情绪感知门控、主动送达（SSE/Lark/Telegram）
- **随处可跑** —— companion_serve（OpenAI 兼容伙伴端点）、TUI、CLI

## 快速开始

```bash
cargo build --workspace

# PowerShell:
$env:APEIRETH_API_KEY = (Get-Content C:\path\to\your-key.txt -Raw).Trim()
cargo run -p apeireth-companion --example companion_serve   # :8090, OpenAI 兼容

curl http://127.0.0.1:8090/v1/chat/completions \
  -H "Content-Type: application/json" -H "Authorization: Bearer any" \
  -d '{"model":"MiniMax-M3","messages":[{"role":"user","content":"你好"}]}'
```

完整指南：[docs/02-guides/quick-start.md](docs/02-guides/quick-start.md)

## 文档

- [docs/ 索引](docs/README.md)
- 架构：[愿景](docs/01-architecture/vision.md) · [哲学](docs/01-architecture/philosophy.md) · [架构总览](docs/01-architecture/architecture.md) · [安全模型](docs/01-architecture/security.md)
- 参考：[85 crates](docs/03-reference/crates.md)
- 发布说明：[RELEASE_NOTES.md](RELEASE_NOTES.md)

## License

Apache-2.0 — 见 [LICENSE](LICENSE)。
