# 03 ADR 编号系统

> **R119-3a-1 Mavis 重建 (2026-08-10)**: 从 APEIRETH-CONVENTIONS.md §3 拆出,核验后写。

```
[Document-Meta]
Document: docs/conventions/03-adr.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-1
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 格式

`docs/adr/NNNN-<kebab-case-topic>.md`

## 命名规范

```
NNNN = 4 位序号 (0001 ~ 9999, 递增, 不重用)
short-slug = 小写连字符短语, 简洁描述
```

## 当前 ADR 实际状态(核验: R20 阶段 6 重排 + 后续 4 配套)

### 主 ADR (12 条, R20 阶段 6 重排 0001-0012)

| # | 标题 | 状态 | 链接 |
|---|---|---|---|
| 0001 | apeireth-rust 1.0 release 收官 | 🟢 Accepted | [docs/adr/0001-apeireth-rust-1.0.md](../../adr/0001-apeireth-rust-1.0.md) |
| 0002 | RIVAL VERSION 蓝图拍板 | 🟢 Accepted | [docs/adr/0002-rival-blueprint.md](../../adr/0002-rival-blueprint.md) |
| 0003 | 整合 #3 策略 | 🟢 Accepted | [docs/adr/0003-integrate-3-strategy.md](../../adr/0003-integrate-3-strategy.md) |
| 0004 | 8 项不修改承诺审计 | 🟢 Accepted | [docs/adr/0004-8-promise-audit.md](../../adr/0004-8-promise-audit.md) |
| 0005 | 1.0 release 12 项 checklist | 🟢 Accepted | [docs/adr/0005-1.0-release-checklist.md](../../adr/0005-1.0-release-checklist.md) |
| 0006 | D-01 6 工具 endpoint 全真接 | 🟢 Accepted | [docs/adr/0006-d-01-tool-endpoint-real.md](../../adr/0006-d-01-tool-endpoint-real.md) |
| 0007 | D-02 6 工具各 1 URL 子路径 | 🟢 Accepted | [docs/adr/0007-d-02-v1-tools-subpath.md](../../adr/0007-d-02-v1-tools-subpath.md) |
| 0008 | D-06 8 包齐发 | 🟢 Accepted | [docs/adr/0008-d-06-8-package-distribution.md](../../adr/0008-d-06-8-package-distribution.md) |
| 0009 | D-07 一次性 SQLite → PostgreSQL 迁移 | 🟢 Accepted | [docs/adr/0009-d-07-sqlite-to-postgres.md](../../adr/0009-d-07-sqlite-to-postgres.md) |
| 0010 | 6 哲学锚 | 🟢 Accepted | [docs/adr/0010-6-philosophy-anchors.md](../../adr/0010-6-philosophy-anchors.md) |
| 0011 | TUI 瘦客户端 | 🟢 Accepted | [docs/adr/0011-tui-as-thin-client.md](../../adr/0011-tui-as-thin-client.md) |
| 0012 | SpectrAI 0.9.21 1:1 翻译 | 🟢 Accepted | [docs/adr/0012-spectrAI-reverse-engineering.md](../../adr/0012-spectrAI-reverse-engineering.md) |

### R20 阶段 1-6 配套 ADR (4 条, 文件名 vs ADR 号 1 位偏差)

| # | 标题 | 状态 | 链接 |
|---|---|---|---|
| 0013 | D-03 WebSocket 鉴权 = 链接 token 5min TTL | 🟢 Accepted | [docs/adr/0014-d-03-ws-auth-link-token.md](../../adr/0014-d-03-ws-auth-link-token.md) |
| 0014 | D-04 限流 = token bucket | 🟢 Accepted | [docs/adr/0015-d-04-rate-limit-token-bucket.md](../../adr/0015-d-04-rate-limit-token-bucket.md) |
| 0015 | Rust SDK 设计 | 🟢 Accepted | [docs/adr/0018-rust-sdk-design.md](../../adr/0018-rust-sdk-design.md) |

### 旧 R14 ADR (12 条, 归档)

[`docs/adr/archive/r14/`](../../adr/archive/r14/) (R20 阶段 6 重排 0001-0012 后归档)

### 旧 R20 重排前 (9 条, 归档)

[`docs/adr/archive/r20-pre-renumber/`](../../adr/archive/r20-pre-renumber/) (R20 阶段 6 重排前 0013-0024, 新 0001-0012 引用新编号, 旧 0013-0024 内容保留)

## ADR 模板 (per MADR 4.0 简化版)

```markdown
# ADR-NNNN: <标题>

## 1. 背景 (Context)
<问题陈述 + 决策驱动 + 约束>

## 2. 决策 (Decision)
<拍板内容>

## 3. 后果 (Consequences)
### 3.1 正面
### 3.2 负面
### 3.3 风险

## 4. 备选 (Alternatives Considered)
<其他选项 + 否决理由>

## 5. 6 哲学锚穿透 (必填)
S-1 / S-2 / O-2 / O-3 / O-4 / O-5

## 6. 8 项不修改承诺 (R119 形式撤销, 原意保留)
阶段 1+2+3 / v2/v4/v4.1 / 阶段 4 / 阶段 5 / v6 / R11 baseline / 顶层 3 规范

## 7. 引用
<相关 ADR / 文档 / commit 锚>
```

## 提交规范

- 每条 ADR 独立 commit
- commit msg: `docs(adr): NNNN short-slug 拍板/修订/废弃`
- 任何 LOCKED 文档不引为依据 (避免循环依赖)
- ADR 编号不重用, 废弃用 🔴 Superseded by NNNN

## 核验

- ✅ 实际 16 主 ADR (R20 重排 12 + R20 配套 4)
- ✅ 实际 21 archive (R14 历史 12 + R20 重排前 9)
- ✅ 编号 0001-0018 跨主 + 配套 (R20 重排 + 后续 4)

## 不漂移

- 0 触碰任何 LOCKED 文档
- 0 改 workspace.version
- 0 改 R11 baseline 3 值
