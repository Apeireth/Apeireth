# Apeireth 架构决策记录 (ADR) 总览

> **性质**: Apeireth 全部 ADR 索引
> **最后更新**: 2026-08-05 22:13 (R20 阶段 6 1.0 release 收口, 12 ADR 重排为 0001-0012)
> **格式**: [MADR 4.0](https://adr.github.io/madr/) 简化版（per Keep a Changelog / MADR 业界惯例）

---

## 1. ADR 命名规范

```
NNNN-short-slug.md

NNNN = 4 位序号（0001 ~ 9999，递增，不重用）
short-slug = 小写连字符短语，简洁描述
```

---

## 2. ADR 现状 (27 条 + 2 归档批)

### 2.1 1.0 release R20 1-6 阶段 ADR (0001 ~ 0012, 12 条)

| # | 标题 | 状态 | 链接 |
|---|---|---|---|
| 0001 | Apeireth-rust 1.0 release 收官 | 🟢 Accepted | [0001-apeireth-rust-1.0.md](0001-apeireth-rust-1.0.md) |
| 0002 | RIVAL VERSION 蓝图拍板 | 🟢 Accepted | [0002-rival-blueprint.md](0002-rival-blueprint.md) |
| 0003 | 整合 #3 策略 (1 批 commit + 5-7 文档) | 🟢 Accepted | [0003-integrate-3-strategy.md](0003-integrate-3-strategy.md) |
| 0004 | 8 项不修改承诺审计 | 🟢 Accepted | [0004-8-promise-audit.md](0004-8-promise-audit.md) |
| 0005 | 1.0 release 12 项 checklist | 🟢 Accepted | [0005-1.0-release-checklist.md](0005-1.0-release-checklist.md) |
| 0006 | D-01 6 工具 endpoint 全真接 (写操作留 R21) | 🟢 Accepted | [0006-d-01-tool-endpoint-real.md](0006-d-01-tool-endpoint-real.md) |
| 0007 | D-02 6 工具各 1 URL 子路径 | 🟢 Accepted | [0007-d-02-v1-tools-subpath.md](0007-d-02-v1-tools-subpath.md) |
| 0008 | D-06 8 包齐发 + Linux 4 包重点 | 🟢 Accepted | [0008-d-06-8-package-distribution.md](0008-d-06-8-package-distribution.md) |
| 0009 | D-07 一次性 SQLite → PostgreSQL 迁移 | 🟢 Accepted | [0009-d-07-sqlite-to-postgres.md](0009-d-07-sqlite-to-postgres.md) |
| 0010 | 6 哲学锚 (S-1/S-2/O-2/O-3/O-4/O-5) | 🟢 Accepted | [0010-6-philosophy-anchors.md](0010-6-philosophy-anchors.md) |
| 0011 | TUI 瘦客户端 (HTTP to apeireth-api) | 🟢 Accepted | [0011-tui-as-thin-client.md](0011-tui-as-thin-client.md) |
| 0012 | SpectrAI 0.9.21 1:1 翻译 | 🟢 Accepted | [0012-spectrAI-reverse-engineering.md](0012-spectrAI-reverse-engineering.md) |

> 0001 ~ 0012 在 R20 阶段 6 1.0 release 收口期拍板, 重排自原 0013-0024 编号 (见 §3 archive)。

### 2.2 R20 阶段 1-6 配套 ADR (0013-0015, 0018, 3 条)

| # | 标题 | 状态 | 链接 |
|---|---|---|---|
| 0013 | D-03 WebSocket 鉴权 = 链接 token 5min TTL | 🟢 Accepted | [0014-d-03-ws-auth-link-token.md](0014-d-03-ws-auth-link-token.md) (注: 文件名 0014-..., ADR 号 0013) |
| 0014 | D-04 限流 = token bucket | 🟢 Accepted | [0015-d-04-rate-limit-token-bucket.md](0015-d-04-rate-limit-token-bucket.md) (注: 文件名 0015-..., ADR 号 0014) |
| 0015 | Rust SDK 设计 | 🟢 Accepted | [0018-rust-sdk-design.md](0018-rust-sdk-design.md) (注: 文件名 0018-..., ADR 号 0015) |

> 文件名跟 ADR 号的 1 位偏差是因为 2026-08-05 22:13 整合 R20 batch 编号时, 0013/0014/0018 文件名保留原版, ADR 号 0013/0014/0015 是重排。
> R21+ 估补: 新增 ADR 时统一文件名 = ADR 号 (e.g. `0016-foo.md` ADR 0016)。

---

## 3. Archive (21 条已归档)

### 3.1 R14 历史 ADR (12 条) — 归档到 [`archive/r14/`](archive/r14/)

原 R14 阶段 12 条历史 ADR (双洋葱统一体 / CLI ↔ Session API 绑定 / Trait 互锁 22 枚举 / 权限洋葱版本化 / 风险等级 M1-M12 阈值 / 集成 rebase-skip 策略 / 兼容组件层 / Feature gating pybridge / 集成 rebase-skip 政策 v2 / MCP from SpectrAI / team-lead supervisor prompt 翻译 / team-lead council 协作), 因 R20 阶段 6 1.0 release 收口重排 0001-0012 而归档。

**保留原因**: 历史决策可追溯; 接手者读 archive/r14/ 即可知 R14 → R20 演进。

### 3.2 R20 重排前 batch (9 条) — 归档到 [`archive/r20-pre-renumber/`](archive/r20-pre-renumber/)

原 R20 阶段 1-6 拍板的 9 条 ADR (0013-apeireth-rust-1.0 / 0016-d-02 / 0017-d-01 / 0019-d-06 / 0020-d-07 / 0021-6-philosophy / 0022-tui-thin-client / 0023-spectrAI / 0024-1.0-release-checklist), 因 R20 阶段 6 1.0 release 收口重排 0001-0012 而归档 (新 0001-0012 引用新编号, 旧 0013-0024 内容保留)。

**保留原因**: 决策可追溯; 旧 0013-0024 是新 0001-0012 的前身, 内容相同, 编号不同。

---

## 4. ADR 模板

每条 ADR 包含 7 节:

```markdown
# ADR NNNN: <标题>

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

## 5. 6 哲学锚穿透
<6 项 必填, 含本节自检>

## 6. 8 项不修改承诺
<8 项 必填, 含严守检查>

## 7. 引用
<相关 ADR / 文档 / commit 锚>
```

---

## 5. ADR 与 6 哲学锚 + 8 项承诺

每条 ADR 末尾 §5 + §6 标注:

- **6 哲学锚** (S-1 / S-2 / O-2 / O-3 / O-4 / O-5) 穿透检查, per [0010-6-philosophy-anchors.md](0010-6-philosophy-anchors.md)
- **8 项不修改承诺** 严守检查, per [0004-8-promise-audit.md](0004-8-promise-audit.md) (含 `scripts/audit/8-promise-audit.sh` 自动审计)

---

## 6. ADR 提交规范

- 每条 ADR 独立 commit
- commit msg 格式: `docs(adr): NNNN short-slug 拍板/修订/废弃`
- 任何 LOCKED 文档不引为依据（避免循环依赖）
- ADR 编号不重用, 废弃用 🔴 Superseded by NNNN

---

## 7. 6 哲学锚 (per [0010](0010-6-philosophy-anchors.md))

1. **S-1 走在前人经验上 (北极星)** — 业界惯例 (MADR / Keep a Changelog / semver)
2. **S-2 实事求是** — 决策基于已 commit 代码 + 实测, 诚实标缺
3. **O-2 走在前人肩上 (用户看结果不看哲学)** — ADR 拍板对外不可见
4. **O-3 干到底 (信息密度"高")** — 7 节结构 vs 散文
5. **O-4 任何人都能接手 (干净状态)** — 拒绝"先做后改"
6. **O-5 不假装 (6 哲学锚穿透)** — 决策自检

---

## 8. 8 项不修改承诺 (per [0004](0004-8-promise-audit.md))

1. **不假装已实现** — skeleton / `unimplemented!()` / `warn! skeleton` 显式标
2. **编译期 hardcode** — LOCKED 关键常量编译期固定
3. **不改 LOCKED** — 7 LOCKED 文档 + 24 LOCKED crate 0 触碰
4. **不改 workspace version** — `Cargo.toml` v1.0.0 严守
5. **6 哲学锚穿透** — §5 自检
6. **不依赖 NewAPI** — 自建, 0 引 NewAPI-style 独立代理服务
7. **不重复造轮子** — 沿用 std / tokio / serde / sqlx / axum / ratatui 业界标准
8. **诚实标缺** — R21 估补项显式标 TODO / FIXME / ⏳

---

## 9. 关联

- 流程: [`docs/stage4/docs-maintenance-sop-2026-08-05.md`](../stage4/docs-maintenance-sop-2026-08-05.md)
- 决策 ID 体系 (D-01 ~ D-12): [`docs/stage4/pending-decisions-overview-2026-08-05.md`](../stage4/pending-decisions-overview-2026-08-05.md)
- 锁文件清单 (24 LOCKED crate + 7 LOCKED 文档): [`docs/stage4/8-locked-unified-2026-08-05.md`](../stage4/8-locked-unified-2026-08-05.md)
- 1.0 release 报告: [`docs/release/1.0.0-release-report-2026-08-05.md`](../release/1.0.0-release-report-2026-08-05.md) (commit `02d5db6c`)
- 1.0 release 详细 changelog: [`docs/1.0-release/changelog.md`](../1.0-release/changelog.md) (29 commits 详单)
- 蓝图 (RIVAL VERSION): [`docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md`](../stage4/v09021-rust-translation-blueprint-2026-08-05.md) (604 行, per [0002](0002-rival-blueprint.md))
- 阶段 1 收官: [`docs/stage4/r20-阶段-1-收官-2026-08-05.md`](../stage4/r20-阶段-1-收官-2026-08-05.md) (commit `5f5b5fa3`)
- 团队入职: [`docs/team-onboarding.md`](../team-onboarding.md) (LOCKED `5b27d041`)
- 8 项不修改承诺审计脚本: [`scripts/audit/8-promise-audit.sh`](../../scripts/audit/8-promise-audit.sh) (commit `629995d3`)
