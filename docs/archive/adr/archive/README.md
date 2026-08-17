# ADR Archive — 历史 ADR 归档

> **性质**: 历史 ADR 归档, 保留可追溯
> **最后更新**: 2026-08-05 22:13 (R20 阶段 6 1.0 release 收口, 重排 0001-0012 时同步归档)
> **格式**: 21 条历史 ADR 分 2 批 (R14 历史 12 条 + R20 重排前 9 条)

---

## 1. 归档原因

2026-08-05 22:13 主人拍板"只干 TUI, 1.0 release 收口", R20 阶段 6 1.0 release 收口期对 12 条 R20 1-6 阶段 ADR 重排编号 (新 0001-0012):

- **原 R14 历史 12 条** (双洋葱统一体 / CLI ↔ Session API 绑定 / 等) — 移到 `archive/r14/`, 让出 0001-0012 编号给 R20 1.0 release batch
- **原 R20 重排前 9 条** (0013-apeireth-rust-1.0 / 0016-d-02 / 等) — 移到 `archive/r20-pre-renumber/`, 新 0001-0012 是其重排+加 6 哲学锚穿透 + 8 项承诺细化版

**保留原因**:
- 决策可追溯 (接手者读 archive 即可知 R14 → R20 演进)
- 重排前内容不丢 (新 0001-0012 引用 archive 旧 0013-0024)
- git 历史保留 (未 commit, 团队 review 可用 git mv 恢复)

---

## 2. R14 历史 (12 条) — `archive/r14/`

| 原 ADR # | 标题 | 状态 | 链接 |
|---|---|---|---|
| 0001 | 双洋葱统一体 | 🟢 Accepted (R14) | [0001-double-onion-unity.md](r14/0001-double-onion-unity.md) |
| 0002 | CLI ↔ Session API 绑定 | 🟢 Accepted (R14) | [0002-cli-session-api-binding.md](r14/0002-cli-session-api-binding.md) |
| 0003 | Trait 互锁 22 枚举 | 🟢 Accepted (R14) | [0003-trait-interlock-22-enum.md](r14/0003-trait-interlock-22-enum.md) |
| 0004 | 权限洋葱版本化 | 🟢 Accepted (R14) | [0004-permission-onion-versioning.md](r14/0004-permission-onion-versioning.md) |
| 0005 | 风险等级 M1-M12 阈值 | 🟢 Accepted (R14) | [0005-risk-grade-m1-m12-thresholds.md](r14/0005-risk-grade-m1-m12-thresholds.md) |
| 0006 | 集成 rebase-skip 策略 | 🟢 Accepted (R14) | [0006-integration-rebase-skip-policy.md](r14/0006-integration-rebase-skip-policy.md) |
| 0007 | 兼容组件层 | 🟢 Accepted (R14) | [0007-compat-components-layer.md](r14/0007-compat-components-layer.md) |
| 0008 | Feature gating pybridge | 🟢 Accepted (R14) | [0008-feature-gating-pybridge.md](r14/0008-feature-gating-pybridge.md) |
| 0009 | 集成 rebase-skip 政策 v2 | 🟢 Accepted (R14) | [0009-integration-rebase-skip-policy.md](r14/0009-integration-rebase-skip-policy.md) |
| 0010 | MCP from SpectrAI AgentMCPServer | 🟢 Accepted (R14) | [0010-mcp-from-spectrai-agentmcpserver.md](r14/0010-mcp-from-spectrai-agentmcpserver.md) |
| 0011 | team-lead supervisor prompt 翻译 | 🟢 Accepted (R14) | [0011-apeireth-team-lead-supervisor-prompt-translation.md](r14/0011-apeireth-team-lead-supervisor-prompt-translation.md) |
| 0012 | team-lead council 协作 | 🟢 Accepted (R14) | [0012-team-lead-council-collaboration.md](r14/0012-team-lead-council-collaboration.md) |

> R14 阶段 5 阶段演进 (灵感 → 想法 → 图纸 → 落实 → 施工) 关键决策
> 2026-08-05 22:13 归档, R20 阶段 6 1.0 release 收口重排 0001-0012 时移入

---

## 3. R20 重排前 batch (9 条) — `archive/r20-pre-renumber/`

| 原 ADR # | 标题 | 新 ADR # | 状态 | 链接 |
|---|---|---|---|---|
| 0013 | Apeireth-rust 1.0 release 收官 | 0001 | 🟢 Accepted | [0013-apeireth-rust-1.0.md](r20-pre-renumber/0013-apeireth-rust-1.0.md) |
| 0016 | D-02 6 工具各 1 URL 子路径 | 0007 | 🟢 Accepted | [0016-d-02-v1-tools-subpath.md](r20-pre-renumber/0016-d-02-v1-tools-subpath.md) |
| 0017 | D-01 工具 endpoint 真接 | 0006 | 🟢 Accepted | [0017-d-01-tool-endpoint-real.md](r20-pre-renumber/0017-d-01-tool-endpoint-real.md) |
| 0019 | D-06 8 包齐发 | 0008 | 🟢 Accepted | [0019-d-06-8-package-distribution.md](r20-pre-renumber/0019-d-06-8-package-distribution.md) |
| 0020 | D-07 一次性 SQLite → PostgreSQL 迁移 | 0009 | 🟢 Accepted | [0020-d-07-sqlite-to-postgres.md](r20-pre-renumber/0020-d-07-sqlite-to-postgres.md) |
| 0021 | 6 哲学锚 | 0010 | 🟢 Accepted | [0021-6-philosophy-anchors.md](r20-pre-renumber/0021-6-philosophy-anchors.md) |
| 0022 | TUI 瘦客户端 | 0011 | 🟢 Accepted | [0022-tui-as-thin-client.md](r20-pre-renumber/0022-tui-as-thin-client.md) |
| 0023 | SpectrAI 0.9.21 1:1 翻译 | 0012 | 🟢 Accepted | [0023-spectrAI-reverse-engineering.md](r20-pre-renumber/0023-spectrAI-reverse-engineering.md) |
| 0024 | 1.0 release 12 项 checklist | 0005 | 🟢 Accepted | [0024-1.0-release-checklist.md](r20-pre-renumber/0024-1.0-release-checklist.md) |

> 9 条重排前 ADR 是新 0001/0005-0012 的前身, 内容相同, 编号不同
> 2026-08-05 22:13 归档, R20 阶段 6 1.0 release 收口重排 0001-0012 时移入

---

## 4. 归档 vs LOCKED 关系

| 类别 | 归档 (本目录) | LOCKED (per `docs/stage4/8-locked-unified-2026-08-05.md`) |
|---|---|---|
| **性质** | 历史决策 (R14 / R20 重排前) | 7 LOCKED 文档 + 24 LOCKED crate (per §3 LOCKED 清单) |
| **可改** | ✅ 可恢复 (git mv) | ❌ 不可改 (7 LOCKED 文档 1 字不动) |
| **8 项承诺约束** | 历史决策严守 8 项承诺 | LOCKED 严守 8 项承诺 |
| **接手者读** | 读 archive 知 R14 → R20 演进 | 读 LOCKED 知当前严守约束 |

---

## 5. 重排记录 (R20 阶段 6 1.0 release 收口)

| 序 | 动作 | 原因 |
|---|---|---|
| 1 | R14 历史 12 条 移到 `archive/r14/` | 让出 0001-0012 编号给 R20 1.0 release batch |
| 2 | R20 重排前 9 条 (0013, 0016-0017, 0019-0024) 移到 `archive/r20-pre-renumber/` | 新 0001-0012 是其重排+加 6 哲学锚穿透 + 8 项承诺细化版, 旧版保留可追溯 |
| 3 | 新 0001-0012 在 `docs/adr/` 落地 | 1.0 release 收口 12 ADR 一目了然 |
| 4 | 0014/0015/0018 (D-03 / D-04 / Rust SDK) 保留文件名, ADR 号 0013-0015 | 这 3 个不在重排 batch, 文件名沿用, 编号衔接 |

---

## 6. 引用

- 主索引: [`docs/adr/README.md`](../README.md)
- 6 哲学锚: [`docs/adr/0010-6-philosophy-anchors.md`](../0010-6-philosophy-anchors.md)
- 8 项不修改承诺审计: [`docs/adr/0004-8-promise-audit.md`](../0004-8-promise-audit.md)
- 1.0 release 收口: [`docs/adr/0001-apeireth-rust-1.0.md`](../0001-apeireth-rust-1.0.md)
- 锁文件清单: [`docs/stage4/8-locked-unified-2026-08-05.md`](../../stage4/8-locked-unified-2026-08-05.md)
