# v1.1.0 — R38 1.1 RC 9 B-stage 一气呵成

```
[Document-Meta]
Document:       docs/release/1.1.0/CHANGELOG.md
Version:        R38 + R40-R45 (1.2 source sync)
R-Cycle:        R38 (1.1.0 RC)
Last-Modified:  2026-08-10 (R119-3c 索引层下沉)
Status:         🟢 已 release
```

**主题**: R35 4→1 + R36 5→1 真合并, 9 B-stage (B1-B9) 一气呵成: telemetry 4 合 1 + provider 5 合 1 + pipeline tool_loop + MCP 3 ResourceServer + CouncilMember 跨 5 provider + CI yaml + memory 7 provider + OAuth device_code + cognition 24 dim + workspace 1.0→1.1。

**关键数字**: workspace version 1.0.0→1.1.0 (R35 → R38), 0 改 24 LOCKED crate, 0 改 R11 baseline 三值, 0 改 8 项不修改承诺。

## 9 B-stage 索引

| Stage | 主题 | 报告 |
|---|---|---|
| B1 | telemetry 4→1 真合并 | [`reports/b1-telemetry-merge-2026-08-09.md`](../../../reports/b1-telemetry-merge-2026-08-09.md) |
| B2 | pipeline tool_loop (web+tauri-stub) | reports/r38 § B2 |
| B3 | MCP 3 ResourceServer 真接 | reports/r38 § B3 |
| B4 | CouncilMember 跨 5 provider | reports/r38 § B4 |
| B5 | GitHub Actions + eval-live.yml | reports/r38 § B5 |
| B6 | memory 7 provider | reports/r38 § B6 |
| B7 | OAuth device_code (RFC 8628) | reports/r38 § B7 |
| B8 | cognition 24 维 + 1 summary + 1 decide | reports/r38 § B8 |
| B9 | workspace 1.0 → 1.1 | reports/r38 § B9 |

## 详细资料

- **1.1 release index**: [`docs/1.1-release/README.md`](../../1.1-release/README.md) (9 B-stage + API 引用 + 哲学锚)
- **R38 主报告**: [`reports/r38-batch-1.1-rc-2026-08-09.md`](../../../reports/r38-batch-1.1-rc-2026-08-09.md)
- **R35 前奏**: [`reports/r35-batch-final-2026-08-09.md`](../../../reports/r35-batch-final-2026-08-09.md)
- **R36 5 老 provider 真删**: [`reports/r36-91-40-slim-5-old-provider-crates-2026-08-09.md`](../../../reports/r36-91-40-slim-5-old-provider-crates-2026-08-09.md)
- **R40-R45 1.2 source sync**: [`reports/r40-r45-1.2-source-sync-2026-08-09.md`](../../../reports/r40-r45-1.2-source-sync-2026-08-09.md)
- **1.2 路线 (预留)**: [`docs/roadmap/v1.2-release-plan-2026-08-09.md`](../../roadmap/v1.2-release-plan-2026-08-09.md)
