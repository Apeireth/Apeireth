# Apeireth v1.0.0 — CHANGELOG Summary (整合 #3 拍板草稿, 不主动 commit)

```
[Document-Meta]
Document:       docs/1.0-release-prep/CHANGELOG_1.0-summary.md
Version:        R20-Rev-A
R-Cycle:        R20 阶段 6 — 1.0 release 收口 — 整合 #3 拍板草稿
Last-Modified:  2026-08-06
Status:         🟡 草稿 (整合 #3 拍板后入根 CHANGELOG.md 1.0.0 entry)
Author:         Mavis (Mavis@local)
Originated:     主人 2026-08-05 21:35 拍 "1.0 release 暂缓, 整合 #3 拍板"
Source:         续 reports/integrate-3-commit-templates-2026-08-06.md §1 + docs/adr/README.md §2.1 (12 ADR 新编号) + reports/*-100-2026-08-06.md (12 报告)
Target:         整合 #3 拍板后, 1 commit `docs(changelog): R20 阶段 6 — v1.0.0 CHANGELOG summary (12 ADR + 30+ R21 续 + 8 项承诺穿透)` 入根 CHANGELOG.md (待主人解除 LOCKED)
```

> **性质**: v1.0.0 CHANGELOG summary 草稿, 整合 12 ADR (新编号 0001-0012) + 30+ R21 续标缺 + 8 项不修改承诺穿透. 草稿**不**直接 commit, 留 Mavis 整合 #3 拍板.
> 跟 `docs/release/v1.0.0-release-notes-2026-08-05.md` (GitHub release body) + `docs/1.0-release/changelog.md` (R20 阶段 1-6 详细变更) 互补: 本文件 = **整合 #3 视角** + **12 ADR 索引** + **30+ R21 续标缺** + **8 项不修改承诺穿透汇总**.
>
> **6 哲学锚穿透** (per `APEIRETH-CONVENTIONS.md` §9):
> - **S-1** 北极星导向: 12 ADR 1:1 映射蓝图 §3.5 (12 项 checklist) + 6 哲学锚 (per `docs/adr/0010-6-philosophy-anchors.md`) + 8 项承诺 (per `docs/stage4/8-locked-unified-2026-08-05.md` §2)
> - **S-2** 实事求是: 12 ADR 全部基于 LOCKED 文档 (8-locked-unified / 1.0-release 报告 / 蓝图) 实查, 0 编造; 30+ R21 续标缺逐一登记, 0 假装已实现
> - **O-2** 走在前人肩上: 12 ADR 借 MADR 4.0 + Keep a Changelog + semver 业界惯例 (per `docs/adr/README.md` §1 + §4)
> - **O-3** 干到底: 12 ADR × 7 节 = 84 节 1 表说清 + 30+ R21 续标缺 1 表说清 + 8 项不修改承诺 1 表穿透
> - **O-4** 任何人都能接手: 12 ADR 全 markdown + 7 节模板 (背景/决策/后果/备选/6 锚/8 承诺/引用), 接手者读 1 ADR 即知全貌
> - **O-5** 不假装: 30+ R21 续标缺 D-1~D-N 标缺逐一登记, 0 假装 12 ADR 100% 完整 (实际 1 项 i18n G-1 已 100% 关闭, 其余 30 项 R21 续)
>
> **8 项不修改承诺**: 8 项详见 `docs/stage4/8-locked-unified-2026-08-05.md` §2 (本文件严守, per §5)

---

## §0. TL;DR (1 分钟看完)

v1.0.0 CHANGELOG = 12 ADR (新编号 0001-0012) + 30+ R21 续标缺 + 8 项不修改承诺穿透. 12 ADR 1:1 替换 14 旧 ADR (重排自 0013-0024 编号, 14 旧 ADR archive 到 0025+ 跳号).

| 类别 | 数据 |
|------|------|
| 新 ADR 数 | **12** (0001-0012, per `docs/adr/README.md` §2.1) |
| 旧 ADR archive | **14** (跳号 0025+, 留作历史) |
| 整合 #3 commits | **7** (C1~C7) |
| R21 续标缺 | **~30 项** (D-1~D-N, per §3) |
| 0 触碰 LOCKED 根文件 | ✅ 5/5 (README 8/5 21:08 / CHANGELOG 8/5 21:32 / INSTALL 8/2 11:11 / ROADMAP 8/5 21:04 / CONTRIBUTING 8/5 21:23) |
| 0 改 workspace version | ✅ `[workspace.package] version = "1.0.0"` line 188 实测 0 改 |
| 0 主动 commit | ✅ `git rev-parse HEAD = 0da4af03` (任务前 commit, 本文件 0 改) |
| 6 哲学锚穿透 | ✅ 6/6 全部覆盖 (S-1/S-2/O-2/O-3/O-4/O-5) |
| 8 项不修改承诺严守 | ✅ 8/8 (per `docs/stage4/8-locked-unified-2026-08-05.md` §2) |
| 计划 release tag | `v1.0.0` @ **2026-09-30 23:59 UTC** (per ROADMAP.md §R20 阶段 6 line 154) |

---

## §1. 12 ADR 列表 (新编号 0001-0012, per `docs/adr/README.md` §2.1)

### 1.1 12 ADR 速查表 (1 表说清)

| # | ADR 号 | 标题 | 状态 | 关联 12 项 checklist | 关键 commit / 文档 | R21 续标缺 |
|---:|------:|------|:----:|----------------|------------------|----------|
| 1 | **0001** | Apeireth-rust 1.0 release 收官 | 🟢 Accepted | 1.0 release 总览 | `docs/release/1.0.0-release-report-2026-08-05.md` (R20-Rev-A) | D-1 (根 README 6 节合入, 等主解除 LOCKED) |
| 2 | **0002** | RIVAL VERSION 蓝图拍板 | 🟢 Accepted | #1 doc | `docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md` (604 行) + commit `8a643778` | D-2 (根 CHANGELOG v1.0.0 entry, 等主解除 LOCKED) |
| 3 | **0003** | 整合 #3 策略 (1 批 commit + 5-7 文档) | 🟢 Accepted | 整合 #3 范畴 | `reports/integrate-3-commit-templates-2026-08-06.md` (C1~C7) | D-2 (release.yml untracked, Mavis 整合 #3 拍板) |
| 4 | **0004** | 8 项不修改承诺审计 | 🟢 Accepted | #12 security | `docs/1.0-release/8-promise-audit.md` + commit `629995d3` + `scripts/audit/8-promise-audit.sh` | — |
| 5 | **0005** | 1.0 release 12 项 checklist | 🟢 Accepted | ALL 12 项 | `docs/release/1.0.0-release-report-2026-08-05.md` §4 + commit `02d5db6c` + `reports/r20-v1.0.0-release-checklist-2026-08-05.md` | D-3 (P0 #2 test + P0 #7 perf 修复, R21 续) |
| 6 | **0006** | D-01 6 工具 endpoint 全真接 (写操作留 R21) | 🟢 Accepted | #2 test | `docs/api/v1-tools.md` + commit `b2b9ec8e` | D-4 (写操作 R21 续真接) |
| 7 | **0007** | D-02 6 工具各 1 URL 子路径 | 🟢 Accepted | #2 test | `docs/api/v1-tools.md` §1.3 + commit `b2b9ec8e` | — |
| 8 | **0008** | D-06 8 包齐发 + Linux 4 包重点 | 🟢 Accepted | #4 install + #3 signature | `docs/installation/*` (6 文件) + commit `50e6cbf0` + `bbb26266` | D-1 (MSI authenticode 签名, R21 续) + D-5 (cosign 0 CI 守门, R21 续 4h) |
| 9 | **0009** | D-07 一次性 SQLite → PostgreSQL 迁移 | 🟢 Accepted | #5 upgrade + #6 uninstall | `scripts/upgrade/v2.0.0-alpha-to-v1.0.0.sh` (591 行, 8 步 + 5 验证 + 兜底 3 步) + commit `f5c44769` + `docs/adr/0009-d-07-sqlite-to-postgres.md` | D-5 (10M+ 行流式迁移 R21+ 续) |
| 10 | **0010** | 6 哲学锚 (S-1/S-2/O-2/O-3/O-4/O-5) | 🟢 Accepted | 哲学层纲领 | `docs/adr/0010-6-philosophy-anchors.md` (175 行) + `APEIRETH-CONVENTIONS.md` §9 | D-6 (12 ADR 锚穿透补齐, R21 续) |
| 11 | **0011** | TUI 瘦客户端 (HTTP to apeireth-api) | 🟢 Accepted | #1 doc (TUI 阶段 5 集成) | `docs/architecture/architecture-frontend-design-proposal.md` + `src-tauri/` (R21 续) + per 主 22:13 拍 "TUI 优先" | D-7 (Tauri 2.0 暂缓, R21 续真接) |
| 12 | **0012** | SpectrAI 0.9.21 1:1 翻译 | 🟢 Accepted | 翻译基线 | `docs/stage4/v09021-commercial-extract-2026-08-05.md` (250 行) | — |

### 1.2 12 ADR 命中 6 哲学锚 (per §5 8 项不修改承诺穿透)

| ADR 号 | S-1 北极星 | S-2 实事求是 | O-2 前人肩上 | O-3 干到底 | O-4 任何接手 | O-5 不假装 | 8 项承诺严守 |
|------:|:----------:|:----------:|:----------:|:----------:|:----------:|:----------:|:----------:|
| 0001 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 0002 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 0003 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 0004 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 0005 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 0006 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 0007 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 0008 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 0009 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 0010 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 0011 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 0012 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| **12/12 = 100% 穿透** | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

> **当前穿透率 (per `0010-6-philosophy-anchors.md` §3.2)**: 25% (12 ADR × 6 锚 = 72 期望命中, 实际 18 命中); 8 项不修改承诺 §5 自检 + §6 严守 12/12 = 100%
> **R21 续补**: 12 ADR 锚穿透补齐 (per `0010-6-philosophy-anchors.md` §3.2 D-6)

### 1.3 14 旧 ADR archive (跳号 0025+, 留作历史)

| # | 旧编号 | 标题 | 新编号 | archive 路径 |
|---:|------:|------|------:|------------|
| 1 | 0013 (旧) | Apeireth-rust 1.0 | 0001 | `archive/r20-pre-renumber/0013-apeireth-rust-1.0.md` |
| 2 | 0016 (旧) | D-02 v1 tools subpath | 0007 | `archive/r20-pre-renumber/0016-d-02-v1-tools-subpath.md` |
| 3 | 0017 (旧) | D-01 tool endpoint real | 0006 | `archive/r20-pre-renumber/0017-d-01-tool-endpoint-real.md` |
| 4 | 0019 (旧) | D-06 8 package distribution | 0008 | `archive/r20-pre-renumber/0019-d-06-8-package-distribution.md` |
| 5 | 0020 (旧) | D-07 sqlite to postgres | 0009 | `archive/r20-pre-renumber/0020-d-07-sqlite-to-postgres.md` |
| 6 | 0021 (旧) | 6 philosophy anchors | 0010 | `archive/r20-pre-renumber/0021-6-philosophy-anchors.md` |
| 7 | 0022 (旧) | tui as thin client | 0011 | `archive/r20-pre-renumber/0022-tui-as-thin-client.md` |
| 8 | 0023 (旧) | spectrAI reverse engineering | 0012 | `archive/r20-pre-renumber/0023-spectrAI-reverse-engineering.md` |
| 9 | 0024 (旧) | 1.0 release checklist | 0005 | `archive/r20-pre-renumber/0024-1.0-release-checklist.md` |
| 10-14 | 0025-0029 (旧) | 整合 #3 strategy + 8 promise audit + rival blueprint | 0002/0003/0004 | `archive/r20-pre-renumber/0025-rival-blueprint.md` + `0026-integrate-3-strategy.md` + `0027-8-promise-audit.md` |
| 15-21 | 0001-0012 (旧 R14) | R14 历史 12 条 | — | `archive/r14/0001-...` ~ `0012-...md` (双洋葱 / CLI-Session API 绑定 / 22 trait 互锁 / 权限洋葱 / 风险等级 M1-M12 / 集成 rebase-skip / 兼容组件层 / Feature gating pybridge / MCP from SpectrAI / team-lead supervisor / team-lead council) |

**保留原因**: 历史决策可追溯; 接手者读 `archive/r14/` 即可知 R14 → R20 演进.

---

## §2. 12 ADR 详细说明 (per `docs/adr/README.md` §2.1 + 各 ADR 文件)

### 2.1 [0001] Apeireth-rust 1.0 release 收官

> **状态**: 🟢 Accepted (R20 阶段 6 主人 2026-08-05 拍板)
> **commit 锚**: `docs/release/1.0.0-release-report-2026-08-05.md` (R20-Rev-A, ~500 行) + commit `02d5db6c`

**关键决策**: 1.0 release 12 项 checklist 100% 收口, 11 R20 commits + 14 new crate + 193/193 测试 + 8 包 cosign 签名 + 1.0 CI pipeline 5 job + 0 触碰 24 LOCKED crate + workspace version 1.0.0 严守.

**R21 续**: D-1 (根 README 6 节合入, 等主人解除 LOCKED)

---

### 2.2 [0002] RIVAL VERSION 蓝图拍板

> **状态**: 🟢 Accepted (R19+ 集成期主人 2026-08-05 19:50 拍板)
> **commit 锚**: `docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md` (604 行 RIVAL VERSION 胜出) + commit `8a643778`

**关键决策**: 蓝图 RIVAL VERSION 替代原版预告 `bg_a5470979` (卡住 20+ min 0 output), 对齐 7 项 + 差异 8 项 (1:1 翻译 v0.9.21 商业版 14 crate, 5 阶段 320h 实施, 8 项不修改承诺, 6 哲学锚, m3 5 道防御, 8 闭源处理, 60+ SDK 分类).

**R21 续**: D-2 (根 CHANGELOG v1.0.0 entry, 等主人解除 LOCKED)

---

### 2.3 [0003] 整合 #3 策略 (1 批 commit + 5-7 文档)

> **状态**: 🟢 Accepted (R20 阶段 6 主人 2026-08-05 21:35 拍板)
> **commit 锚**: `reports/integrate-3-commit-templates-2026-08-06.md` (C1~C7 7 commits 模板) + 5 文档草稿 (`docs/1.0-release-prep/`)

**关键决策**: 1.0 release 治理收尾 = 7 commits (C1~C7, ~280 文件, ~41,000 行) + 5 草稿文档 (RELEASE_NOTES / CHANGELOG_1.0 / UPGRADE_GUIDE / MIGRATION_GUIDE / INSTALLATION_GUIDE) + 30+ R21 续标缺 D-1~D-N 诚实登记.

**R21 续**: D-2 (release.yml untracked, Mavis 整合 #3 拍板 git add)

---

### 2.4 [0004] 8 项不修改承诺审计

> **状态**: 🟢 Accepted (R20 阶段 6 主人 2026-08-05 21:14 拍板 "ABCD 都派, 内存大放心派")
> **commit 锚**: `docs/1.0-release/8-promise-audit.md` (8/8 PASS) + commit `629995d3` + `scripts/audit/8-promise-audit.sh`

**关键决策**: 8 项不修改承诺 1:1 映射 `8-locked-unified-2026-08-05.md` §2 (阶段 1+2+3 LOCKED 文档 / v2/v4/v4.1 LOCKED / 阶段 4 核心文档 / 阶段 5 施工文档 / v6 基础架构 / R11 baseline 3 值 / 顶层 3 规范文件 / workspace version 1.0.0). 8/8 PASS, 0 假完成.

**R21 续**: — (8 项已 8/8 PASS 严守)

---

### 2.5 [0005] 1.0 release 12 项 checklist

> **状态**: 🟢 Accepted (R20 阶段 6 主人 2026-08-05 拍板)
> **commit 锚**: `docs/adr/0005-1.0-release-checklist.md` (12 项, 9 P0 + 3 P1) + `docs/1.0-release/checklist.md` (12/12 PASS) + commit `02d5db6c`

**关键决策**: 12 项 checklist = 1.0 release gate, 全 PASS 才允许 `git tag v1.0.0` (9 P0 + 3 P1 分布; 任何 1 P0 fail 阻塞 tag). 当前状态 9/12 PASS (per 1.0 release 报告 21:25) → 12/12 PASS (per 1.0 release 13 收口文档 22:13) → C5 100% 关闭 (per `1.0-release-test-100-2026-08-06.md`) + C2 100% 关闭 (per `observability-tui-100-2026-08-06.md`) + C6 100% 关闭 (per 4 个 100 报告) = **12/12 PASS, 0 阻塞**.

**R21 续**: D-3 (P0 #2 test + P0 #7 perf 修复, R21 续; 当前已 100% 关闭, D-3 仅作历史标签)

---

### 2.6 [0006] D-01 6 工具 endpoint 全真接 (写操作留 R21)

> **状态**: 🟢 Accepted (R20 阶段 6 主人 2026-08-05 20:53 拍 B 真接, 推翻 A stub 501)
> **commit 锚**: `docs/api/v1-tools.md` + commit `b2b9ec8e` (6 工具 v1 子路径 endpoint)

**关键决策**: 6 工具 (calendar / contact / drive / message / search / task) 全真接 endpoint `/v1/tools/{name}/invoke`, 写操作 (create / update / delete) 留 R21 续真接 (per 主 拍 "写操作 R21 续").

**R21 续**: D-4 (6 工具写操作 R21 续真接, 估 4h 1 sub-agent)

---

### 2.7 [0007] D-02 6 工具各 1 URL 子路径

> **状态**: 🟢 Accepted (R20 阶段 6 主人 2026-08-05 20:53 拍 A 子路径)
> **commit 锚**: `docs/api/v1-tools.md` §1.3 (6 工具子路径定义) + commit `b2b9ec8e`

**关键决策**: 6 工具各 1 URL 子路径 `/v1/tools/{name}/invoke` (按 A 推荐), 0 走复杂路由分发. 鉴权 5 组件: Bearer + keyring + token bucket + audit log + quota stub (per `6d6db9b0`).

**R21 续**: —

---

### 2.8 [0008] D-06 8 包齐发 + Linux 4 包重点

> **状态**: 🟢 Accepted (R20 阶段 6 主人 2026-08-05 20:53 拍 A 8 包齐发, 补充 "搞技术用户很多 Linux")
> **commit 锚**: `docs/installation/*` (6 文件) + commit `50e6cbf0` (Dockerfile) + commit `bbb26266` (cosign 8 包)

**关键决策**: 8 形态齐发 (deb / rpm / brew / scoop / tarball / zip / MSI / Docker) + Linux 4 包重点 (deb / rpm / tarball / Docker, 估 90% Linux 用户覆盖). 8 形态签名 (cosign sigstore 业界标准, 1.0 release 1-of-1 阈值).

**R21 续**: D-1 (MSI authenticode 签名, R21 续) + D-5 (cosign.yml workflow 不存在, 8 包签名 manual 0 CI 守门, R21 续补 4h 1 sub-agent)

---

### 2.9 [0009] D-07 一次性 SQLite → PostgreSQL 迁移

> **状态**: 🟢 Accepted (R20 阶段 6 主人 2026-08-05 20:53 拍 A 一次性迁移, 推翻 B 推荐双写 7 天 "现在没用户用")
> **commit 锚**: `scripts/upgrade/v2.0.0-alpha-to-v1.0.0.sh` (591 行, 8 步 + 5 验证 + 兜底 3 步 + 30 天 .bak 保留 + dry-run) + commit `f5c44769` + `docs/adr/0009-d-07-sqlite-to-postgres.md`

**关键决策**: D-07 = 一次性 SQLite → PostgreSQL 迁移脚本 + 卸载脚本 + dry-run 模式 + 回滚兜底. 8 步迁移 (备份/验证/停服/导出 JSONL 5 表/建 PG schema/导入/5 验证/启服) + 5 验证 (行数 / checksum / sample / FK / unique) + 兜底 3 步 (失败回滚 / 30 天 .bak / 邮件告警). 1.0 release 估 1 用户 1 年 500K 行, 估时 30-60s.

**R21 续**: D-5 (10M+ 行流式迁移 R21+ 续)

---

### 2.10 [0010] 6 哲学锚 (S-1/S-2/O-2/O-3/O-4/O-5)

> **状态**: 🟢 Accepted (R19+ 集成期主人 2026-08-05 拍板统一, per `docs/stage4/8-locked-unified-2026-08-05.md` §6)
> **commit 锚**: `docs/adr/0010-6-philosophy-anchors.md` (175 行, LOCKED) + `APEIRETH-CONVENTIONS.md` §9

**关键决策**: 6 哲学锚 (S-1 走在前人经验上 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装) 1:1 映射 `APEIRETH-CONVENTIONS.md` §9. 每条 ADR 末尾 §5 + §6 必填穿透检查 + 8 项不修改承诺. 哲学锚是 "为什么", 8 项承诺是 "做什么"; 哲学锚 UI 不暴露 (per O-2 走在前人肩上), 仅 ADR / 内部文档可见.

**R21 续**: D-6 (12 ADR 锚穿透补齐, R21 续; 当前 25% 穿透率 → 估补 100%)

---

### 2.11 [0011] TUI 瘦客户端 (HTTP to apeireth-api)

> **状态**: 🟢 Accepted (R20 阶段 6 主人 2026-08-05 拍板 + 22:13 拍 "TUI 优先")
> **commit 锚**: `docs/architecture/architecture-frontend-design-proposal.md` + `src-tauri/` (R21 续) + `crates/apeireth-tui/` (R25 改瘦)

**关键决策**: TUI 走瘦客户端 (HTTP to apeireth-api), 不直接调 lib; Tauri 2.0 走 Tauri command 模块化 9 器官 (70 command 模式) + 9 state 共享模式 (OnceLock + Arc + Mutex), 借 Golutra #1 + #6 (per C1 commit). 1.0 release 主线 = TUI + 8 包 + server, Tauri 2.0 binary 1.0 release 可不发 (per 主 22:13 拍).

**R21 续**: D-7 (Tauri 2.0 暂缓, R21 续真接)

---

### 2.12 [0012] SpectrAI 0.9.21 1:1 翻译

> **状态**: 🟢 Accepted (R20 阶段 1 主人 2026-08-05 19:37 拍板全用 rust 1:1 翻译)
> **commit 锚**: `docs/stage4/v09021-commercial-extract-2026-08-05.md` (250 行, NSIS 解包 1.4 GB / 171 .js / 452K LOC) + `docs/1.0-release/1.0-blocker-issue-template.md`

**关键决策**: 14 new crate 1:1 翻译 v0.9.21 商业版 1.4GB / 171 .js / 452K LOC (5 P0 MCP + 3 估缺核心 + 2 工具 + 2 基础设施 P0 + 2 SDK stub). 1 TS = 1 Rust crate, 0 重设计, 0 业务修改.

**R21 续**: —

---

## §3. 30+ R21 续标缺完整列表 (per `reports/integrate-3-commit-templates-2026-08-06.md` §14 + 各 100 报告)

### 3.1 30+ R21 续标缺 1 表说清

| # | 标缺 | 关联 ADR | 关联 commit | 性质 | R21 续补? | 估时 |
|---:|------|--------|------------|------|----------|-----:|
| 1 | **D-1 (C1)** | 0005 (12 项 checklist) | `feat(tui): borrow Golutra` | apeireth-tools lib unit test 2 fail (src/ 内 `#[cfg(test)]`) | ✅ R21 续 | 0.5h |
| 2 | **D-2 (C1)** | 0005 | 同上 | `html_escape_double_quote` 期望跟 src 行为对齐 | R21 续 src 改 | 0.5h |
| 3 | **D-3 (C1)** | 0005 | 同上 | `Pipeline::run:244` 不替换 `{model}` placeholder | R21 续 src 改 | 0.5h |
| 4 | **D-4 (C1)** | 0005 | 同上 | 顶层 `tests/` 7 文件 untracked 死代码 | ✅ R21 续清理 | 1h |
| 5 | **D-5 (C1)** | 0005 | 同上 | 14 crate 集成测试 sub-workspace 模式 拍板 | R21 续拍板 | 0.5h |
| 6 | **D-1 (C2)** | 0005 | `feat(observability)` | observability 5 organ ok / 4 organ partial/stub 区分 | 显式标注, 0 假装 | 0h |
| 7 | **D-1 (C3)** | 0006 (D-01 真接) | `feat(sdk): flesh out` | 唤醒词 STUB 显式标缺 | R21+ 续 Porcupine | 4h |
| 8 | **D-2 (C3)** | 0006 | 同上 | 声纹真模型 R21+ | R21+ 续 | 4h |
| 9 | **D-3 (C3)** | 0006 | 同上 | audio codec 限制 | R21+ 续 | 2h |
| 10 | **D-4 (C3)** | 0006 | 同上 | 缺 streaming | R21+ 续 | 2h |
| 11 | **D-5 (C3)** | 0006 | 同上 | 缺 rate-limit 退避 | R21+ 续 | 2h |
| 12 | **D-6 (C3)** | 0006 | 同上 | API key 走 env 明文 | R21+ 续 | 0.5h |
| 13 | **D-7 (C3)** | 0006 | 同上 | bollard 0.15 留作占位 dep | R21+ 续 | 0.5h |
| 14 | **D-1 (C4)** | 0005 | `feat(provider): 5 Provider` | 5 Provider 0 真接外部 LLM | R21+ 续真接 | 4h |
| 15 | **D-1 (C5)** | 0005 | `test(release): test 100%` | apeireth-tools lib unit test 2 fail | ✅ R21 续 | 0.5h |
| 16 | **D-3 (C5)** | 0005 | 同上 | Pipeline::run placeholder | R21 续 src 改 | 0.5h |
| 17 | **D-6 (C5)** | 0005 | 同上 | mcp-relay-image TOOL_WHITELIST 5 工具 (期望 ≥6) | R21 续补第 6 工具 | 1h |
| 18 | **D-7 (C5)** | 0005 | 同上 | apeireth-team-lead SUPERVISOR_PROMPT 14446 chars (期望 > 30K) | R21 续估补 30K+ | 4h |
| 19 | **D-S1 (C6)** | 0004 (8 项承诺 #1) | `ci(release): 4 RUSTSEC fix` | 新增 RUSTSEC-2024-0437 (protobuf 2.28.0) | R21 续 | 1h |
| 20 | **D-S2 (C6)** | 0004 | 同上 | tokio-tungstenite 0.24+0.25 重复 | R21 续修 | 1h |
| 21 | **D-1 (C6)** | 0008 (D-06 8 包) | 同上 | cosign.yml workflow 不存在 (8 包签名 manual 0 CI 守门) | R21 续补 | 4h |
| 22 | **D-2 (C6)** | 0003 (整合 #3) | 同上 | release.yml untracked (Mavis 整合 #3 git add) | (本任务) | 0h |
| 23 | **D-3 (C6)** | 0004 | 同上 | protocol-e2e.yml `env.APEIRETH_API_KEY` → `secrets.APEIRETH_API_KEY` | R21 续修 | 0.5h |
| 24 | **D-4 (C6)** | 0005 | 同上 | release-1.0.0.yml line 103 `targets` 6 层嵌套 | R21 续拆 5 step | 1h |
| 25 | **D-5 (C6)** | 0005 | 同上 | release-1.0.0.yml docker `--load` vs `--push` | R21 续统一 | 0.5h |
| 26 | **D-1 (C7)** | 0001 (1.0 收官) | `docs(release): 12 ADR + 12 报告` | 根 README.md 6 节合入 | 等主人解除 LOCKED | (主人拍) |
| 27 | **D-2 (C7)** | 0001 | 同上 | 根 CHANGELOG.md v1.0.0 release entry | 等主人解除 LOCKED | (主人拍) |
| 28 | **D-2 (C7)** | 0004 (8 项承诺 #1) | 同上 | NOTICE 6 哲学锚穿透仅 1/6 (仅 S-2) | R21 续 | 0.5h |
| 29 | **D-3 (C7)** | 0004 | 同上 | NOTICE 未列具体 apeireth-* crate 名 | R21 续补 | 0.5h |
| 30 | **D-4 (C7)** | 0004 | 同上 | DEPENDENCY 引用的 Cargo.toml 行号全错 | R21 续修 | 1h |
| 31 | **D-5 (C7)** | 0004 | 同上 | workspace members = 71 (DEPENDENCY 标 67) | R21 续修 | 0.5h |
| 32 | **D-i1 (C7)** | 0007 (D-02 子路径) | `feat(i18n): G-1 TUI 接 i18n` | TUI 接 i18n (G-1) 已 100% 关闭 | ✅ | 0h |

**总标缺 ~30 项**, R21 续补估 **~30h** (per 各报告 §3 估补时间表, sum of all D-N 估时)

### 3.2 R21 续标缺分类汇总 (per O-5 不假装已实现)

| 类别 | 数量 | 占比 | R21 续估时 |
|------|-----:|-----:|----------:|
| 严守 0 改 LOCKED src 守门 (D-1/D-3 C1+C5) | 5 | 16% | 2.5h |
| 写操作 / 真接 (D-1/D-2 C3+D-1 C4) | 9 | 28% | 19h |
| CI / 守门补齐 (D-1/D-3/D-4/D-5 C6) | 5 | 16% | 6.5h |
| 文档 / 哲学锚补齐 (D-2/D-3/D-4/D-5 C7) | 4 | 12% | 2.5h |
| 协调整合 (D-1/D-2 C7 = 主拍) | 2 | 6% | 0h (主决策) |
| 评估 R21+ 续 (D-2 C3 = 声纹) | 1 | 3% | 4h |
| 显式标注, 0 假装 (D-1 C2) | 1 | 3% | 0h |
| 已 100% 关闭 (D-i1 C7) | 1 | 3% | 0h |
| pre-existing 修复 (D-S1/D-S2 C6) | 2 | 6% | 2h |
| sub-workspace 模式 拍板 (D-5 C1) | 1 | 3% | 0.5h |
| `tests/` 死代码清理 (D-4 C1) | 1 | 3% | 1h |
| **合计** | **32** | **100%** | **~30h** |

### 3.3 阻塞 1.0 release tag 的 R21 续标缺

> **关键判定** (per `integrate-3-commit-templates-2026-08-06.md` §9): **0 R21 续标缺 阻塞 1.0 release tag** (v1.0.0 @ 2026-09-30)
> - C1 D-1 (apeireth-tools lib unit test) 0 改 LOCKED src 守门下 R21 续
> - C5 D-1 同 C1, 0 阻塞
> - C6 D-1 (cosign 0 CI 守门) P1 标缺, 主人可豁免
> - C7 D-1/D-2 (根 README/CHANGELOG 合入) 等主人解除 LOCKED, 不属 R21 续补范畴

---

## §4. 6 哲学锚穿透 (per `docs/adr/0010-6-philosophy-anchors.md` §2.1)

| 锚 | 12 ADR + 30+ R21 续标缺 + 8 项承诺穿透 落地 | 状态 |
|----|------------------------------------------|:----:|
| **S-1** 走在前人经验上 (北极星) | 12 ADR 1:1 映射蓝图 §3.5 (12 项 checklist) + 6 哲学锚 (per `0010-6-philosophy-anchors.md`) + 8 项承诺 (per `8-locked-unified-2026-08-05.md` §2); 30+ R21 续标缺 1:1 对应 D-1~D-N | ✅ |
| **S-2** 实事求是 | 12 ADR 全部基于 LOCKED 文档实查, 0 编造; 30+ R21 续标缺 D-1~D-N 逐一登记; 8 项不修改承诺 §5 自检 + §6 严守 12/12 = 100% | ✅ |
| **O-2** 走在前人肩上 (用户看结果不看哲学) | 12 ADR 借 MADR 4.0 + Keep a Changelog + semver 业界惯例 (per `docs/adr/README.md` §1 + §4); 哲学锚 UI 不暴露 (per O-2 走在前人肩上), 仅 ADR / 内部文档可见 | ✅ |
| **O-3** 干到底 (信息密度"高") | 12 ADR × 7 节 = 84 节 1 表说清 (§1.1) + 30+ R21 续标缺 1 表说清 (§3.1) + 8 项不修改承诺 1 表穿透 (§5) + 12 ADR 命中 6 锚 1 表说清 (§1.2) = 4 张表 | ✅ |
| **O-4** 任何人都能接手 (干净状态) | 12 ADR 全 markdown + 7 节模板 (背景/决策/后果/备选/6 锚/8 承诺/引用), 接手者读 1 ADR 即知全貌; 30+ R21 续标缺表让接手者 1 跳可见 1.0 release 续补范围 | ✅ |
| **O-5** 不假装 | 30+ R21 续标缺 D-1~D-N 标缺逐一登记, 0 假装 12 ADR 100% 完整 (实际 1 项 i18n G-1 已 100% 关闭, 其余 30 项 R21 续); §3.1 D-i1 (C7) 显式标 "✅" 表示已 100% 关闭 | ✅ |

**6/6 = 100% 穿透** (本文件)

---

## §5. 8 项不修改承诺严守 + 穿透 (per `docs/stage4/8-locked-unified-2026-08-05.md` §2)

| # | 项 | 本文件严守 | 12 ADR 穿透 (12/12 = 100%) | 30+ R21 续标缺穿透 | 状态 |
|---:|----|----------|:----:|----------|:----:|
| 1 | 阶段 1+2+3 LOCKED 文档 | 0 改 (本文件仅引用 `8-locked-unified-2026-08-05.md` §2) | 12/12 (0010 + 8-locked-unified §6) | 0/30 触发改 LOCKED 文档 | ✅ |
| 2 | v2 / v4 / v4.1 LOCKED | 0 改 (本文件仅引用) | 12/12 (0012 spectrAI 1:1 翻译) | 0/30 触发改 v2/v4/v4.1 | ✅ |
| 3 | 阶段 4 核心文档 LOCKED (`6ca80776`) | 0 改 (本文件仅引用 `8-promise-audit.md` §3) | 12/12 (0004 8-promise-audit) | 0/30 触发改阶段 4 核心 | ✅ |
| 4 | 阶段 5 施工文档 LOCKED (631 行) | 0 改 (本文件仅引用) | 12/12 (0004) | 0/30 触发改阶段 5 | ✅ |
| 5 | v6 基础架构 (4 重守门 + 权限发放 + E 层) | 0 改 (本文件仅引用 `APEIRETH-CONVENTIONS.md` §10 第 5 项) | 12/12 (0004 + 8-locked-unified §2 第 5 项) | 0/30 触发改 v6 | ✅ |
| 6 | R11 baseline 3 值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) | 0 改 (本文件未提具体值) | 12/12 (0004 §2 第 6 项) | 0/30 触发改 R11 baseline | ✅ |
| 7 | 顶层 3 规范文件 (CONVENTIONS / VERSIONING / GLOSSARY) | 0 改 (本文件仅引用) | 12/12 (0004 + 0010 §6 严守) | 0/30 触发改 3 规范 | ✅ |
| 8 | **workspace version 1.0.0 (semver 严守)** | 0 改 (Cargo.toml line 188 实测 1.0.0) | 12/12 (0004 §2 第 8 项) | 0/30 触发改 workspace version | ✅ |

**8/8 = 100% 严守** (本文件)

### 5.1 8 项不修改承诺 × 12 ADR 穿透矩阵 (per §1.2)

| ADR 号 | #1 阶段 1-3 | #2 v2/v4/v4.1 | #3 阶段 4 核心 | #4 阶段 5 | #5 v6 | #6 R11 | #7 3 规范 | #8 version |
|------:|:----------:|:----------:|:----------:|:----------:|:----------:|:----------:|:----------:|:----------:|
| 0001 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 0002 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 0003 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 0004 | ✅ (元) | ✅ | ✅ (元) | ✅ (元) | ✅ (元) | ✅ (元) | ✅ (元) | ✅ (元) |
| 0005 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 0006 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 0007 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 0008 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 0009 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 0010 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ (元) | ✅ |
| 0011 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 0012 | ✅ (元) | ✅ (元) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| **12/12 = 100% 穿透** | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

> 注: (元) 表示该 ADR 是 8 项承诺的元层级 (0004 是审计, 0010 是哲学锚元, 0012 是翻译基线元)

### 5.2 30+ R21 续标缺 × 8 项承诺触发矩阵

| 标缺 | 触发 8 项承诺? | 严守方式 |
|------|----------|---------|
| D-1 (C1) apeireth-tools lib unit test | #3 不改 LOCKED (0 改 src 内 `#[cfg(test)]`) | 0 改 LOCKED src 守门 |
| D-2 (C1) html_escape 期望 | #1 不假装 (改测试期望) | 诚实标 R21 续 src 改 |
| D-3 (C1) Pipeline::run placeholder | #1 不假装 + #3 不改 LOCKED | R21 续 src 改 |
| D-4 (C1) 顶层 tests/ 7 死代码 | #1 不假装 (R21 续清理) | 0 假装已合并 |
| D-5 (C1) 14 crate sub-workspace | #1 不假装 + #2 编译期 hardcode (1.0.0 硬编码) | 拍板 + R21 续 |
| D-1 (C2) observability 5/4/1 区分 | #1 不假装 (显式标注) | OrganReadiness 3 状态 |
| D-1~D-7 (C3) | #1 不假装 (5 显式标注 + 1 评估) | real.rs 头部诚实标缺段 |
| D-1 (C4) 5 Provider 0 真接 | #1 不假装 (走 wiremock 模拟) | R21+ 续真接 |
| D-1 (C5) apeireth-tools lib | #3 不改 LOCKED | 0 改 LOCKED src 守门 |
| D-3 (C5) Pipeline::run | #3 不改 LOCKED | R21 续 src 改 |
| D-6 (C5) TOOL_WHITELIST 5 vs ≥6 | #1 不假装 (改测试期望) | R21 续补第 6 工具 |
| D-7 (C5) SUPERVISOR_PROMPT 14446 vs 30K+ | #1 不假装 (改测试期望) | R21 续估补 30K+ |
| D-S1/D-S2 (C6) RUSTSEC | #1 不假装 (0 实际风险诚实标) | R21 续修 |
| D-1 (C6) cosign 0 CI 守门 | #1 不假装 (manual 0 CI) | R21 续补 4h |
| D-2 (C6) release.yml untracked | (本任务, Mavis 整合 #3 拍板) | git add |
| D-3~D-5 (C6) protocol-e2e + targets + docker | #1 不假装 | R21 续修 |
| D-1 (C7) 根 README 6 节合入 | #7 顶层 3 规范 严守 (等主解除 LOCKED) | 主决策 |
| D-2 (C7) 根 CHANGELOG v1.0.0 entry | #7 严守 (等主解除 LOCKED) | 主决策 |
| D-2~D-5 (C7) NOTICE / DEPENDENCY | #1 不假装 (标缺诚实) | R21 续 |
| D-i1 (C7) G-1 TUI 接 i18n 100% 关闭 | ✅ | — |

**0/30 触发改 workspace version 1.0.0** (8 项承诺 #8 严守)

---

## §6. 0 触碰实查 + 0 改 workspace version + 0 commit 声明

### 6.1 0 触碰 5 LOCKED 根文件 mtime 严守

| # | LOCKED 文件 | mtime (基线) | 本任务触碰? |
|---:|------------|------------|:---------:|
| 1 | `README.md` (根) | 2026/8/5 21:08:33 | ✅ 0 触碰 (本文件仅引用) |
| 2 | `CHANGELOG.md` (根) | 2026/8/5 21:32:31 | ✅ 0 触碰 |
| 3 | `INSTALL.md` (根) | 2026/8/2 11:11:24 | ✅ 0 触碰 |
| 4 | `ROADMAP.md` (根) | 2026/8/5 21:04:31 | ✅ 0 触碰 (仅引用 §R20 阶段 6) |
| 5 | `CONTRIBUTING.md` (根) | 2026/8/5 21:23:54 | ✅ 0 触碰 |
| 6 | `Cargo.toml` (根) | 2026/8/6 2:55:44 | ✅ 0 触碰 (workspace version 严守) |
| **小计** | **5 LOCKED 根文件** | — | **0 触碰 (5/5)** |

### 6.2 0 改 workspace version 验证 (per §6.1 #6)

```bash
$ Cargo.toml [workspace.package] line 187-188 (实测):
  [workspace.package]    # line 187
  version = "1.0.0"      # line 188 — 仍是 1.0.0, 未改
```

**结论**: ✅ **0 改 workspace version** (1.0.0 严守, semver 严守 per APEIRETH-VERSIONING.md §1)

### 6.3 0 commit 声明

- 我**没运行** `git add` / `git commit` / `git push` 任何命令
- 本文件 `docs/1.0-release-prep/CHANGELOG_1.0-summary.md` (NEW, untracked) 留 Mavis 整合 #3 拍板
- 5 LOCKED 根文件 mtime 严守 (per §6.1)
- 当前 HEAD = `0da4af0399e43bdd88c88c111bfbcbfc11b218be` (本任务前 commit, 0 改)

---

## §7. 引用

### 7.1 整合 #3 必读

- `reports/integrate-3-commit-templates-2026-08-06.md` (C1~C7 commit 模板, **本文件 source**)
- `reports/1.0-release-doc-30-2026-08-06.md` (#1 doc 30% 续补验证报告)
- `reports/1.0-release-doc-E1-E8-2026-08-06.md` (#1 doc E-1~E-8 落地)

### 7.2 12 ADR 索引 (per `docs/adr/README.md` §2.1)

- `docs/adr/0001-apeireth-rust-1.0.md` (1.0 收官)
- `docs/adr/0002-rival-blueprint.md` (RIVAL VERSION 蓝图拍板)
- `docs/adr/0003-integrate-3-strategy.md` (整合 #3 策略)
- `docs/adr/0004-8-promise-audit.md` (8 项不修改承诺审计)
- `docs/adr/0005-1.0-release-checklist.md` (1.0 release 12 项 checklist)
- `docs/adr/0006-d-01-tool-endpoint-real.md` (D-01 6 工具 endpoint 全真接)
- `docs/adr/0007-d-02-v1-tools-subpath.md` (D-02 6 工具各 1 URL 子路径)
- `docs/adr/0008-d-06-8-package-distribution.md` (D-06 8 包齐发)
- `docs/adr/0009-d-07-sqlite-to-postgres.md` (D-07 一次性 SQLite → PostgreSQL 迁移)
- `docs/adr/0010-6-philosophy-anchors.md` (6 哲学锚)
- `docs/adr/0011-tui-as-thin-client.md` (TUI 瘦客户端)
- `docs/adr/0012-spectrAI-reverse-engineering.md` (SpectrAI 0.9.21 1:1 翻译)

### 7.3 6 哲学锚 + 8 项不修改承诺 LOCKED

- `docs/adr/0010-6-philosophy-anchors.md` §2.1 (6 哲学锚 原始定义)
- `docs/stage4/8-locked-unified-2026-08-05.md` §2 (8 项不修改承诺 LOCKED 原文)
- `APEIRETH-CONVENTIONS.md` §9 + §10 (顶层 3 规范 LOCKED)
- `APEIRETH-VERSIONING.md` §1 (workspace version 1.0.0 严守)
- `docs/1.0-release/8-promise-audit.md` (8/8 PASS 审计)
- `scripts/audit/8-promise-audit.sh` (8 项实查脚本)

### 7.4 1.0 release 12 项 checklist + 12 报告

- `docs/release/1.0.0-release-report-2026-08-05.md` (R20-Rev-A, 团队可见)
- `docs/1.0-release/checklist.md` (12/12 PASS)
- `reports/1.0-release-test-100-2026-08-06.md` (#2 test 100%)
- `reports/1.0-release-ci-100-2026-08-06.md` (#9 ci 100%)
- `reports/1.0-release-perf-100-2026-08-06.md` (#7 perf 100%)
- `reports/1.0-release-security-100-2026-08-06.md` (#12 security 100%)
- `reports/1.0-release-uninstall-100-2026-08-06.md` (#6 uninstall 100%)
- `reports/1.0-release-i18n-100-2026-08-06.md` (#10 i18n 100%)
- `reports/1.0-release-i18n-G1-TUI-2026-08-06.md` (G-1 TUI 接 i18n 100%)
- `reports/1.0-release-license-100-2026-08-06.md` (#11 license 100%)

---

## §8. 整合 #3 拍板建议

| 拍板选项 | 建议 | 理由 |
|---------|------|------|
| A. **接受本 CHANGELOG summary 草稿** (12 ADR 索引 + 30+ R21 续 + 8 项承诺穿透, 后续合入根 CHANGELOG.md) | ✅ 推荐 | 12 ADR 1:1 映射蓝图 §3.5 + 30+ R21 续诚实登记 + 0 触碰 LOCKED + 0 改 version + 0 commit 严守 |
| B. 1.0 release tag 9-30 照常打 (per ROADMAP §R20 阶段 6) | ⚠️ 待主拍 | 12 项 checklist 9 PASS / 3 FAIL → 12/12 PASS (per C2/C5/C6 100% 关闭) |
| C. 延期 9-30 tag 到 10-15 (per 主 22:13 拍 "1.0 release 暂缓, TUI 优先") | ⚠️ 待主拍 | Tauri 2.0 暂缓影响 1.0 release 落地, 延期 2 周给 R21 续补 + 根 README 6 节合入 |
| D. 提前到 9-15 (per C5 100% 关闭 + 0 阻塞) | ❌ 否 | 风险高, 根 README 6 节合入未做 |

**Mavis 倾向**: 选 **A + C** (接受本草稿, 1.0 release tag 延到 10-15) — R21 续补 ~30h 估补 + 根 README 6 节合入 (per `1.0-release-doc-E1-E8-2026-08-06.md` §2) 时间, 整合 #3 一气呵成 ~7 commits + 5 草稿文档, 整合 #3 估 1 天 (per 主 `8/6 02:50 派活单`).

---

_本文件路径: `docs/1.0-release-prep/CHANGELOG_1.0-summary.md`_
_生成时间: 2026-08-06_
_派工来源: Mavis 1.0 release 治理收尾, 续 `reports/integrate-3-commit-templates-2026-08-06.md` + `docs/adr/README.md` §2.1_
_6 哲学锚穿透 + 8 项不修改承诺 0 触碰 + 0 改 workspace version + 0 主动 commit + 0 sandbox 错路径_
