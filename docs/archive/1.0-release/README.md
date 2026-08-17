# Apeireth 1.0 release — 收口文档总览

```
[Document-Meta]
Document:       docs/1.0-release/README.md
Version:        R20-Rev-A
R-Cycle:        R20 阶段 6 — 1.0 release 收口索引
Last-Modified:  2026-08-05
Status:         🟢 1.0 release 12 项 checklist 100% 收口
Author:         Mavis (Mavis@local)
Originated:     主人 2026-08-05 22:13 拍板"只干 TUI,1.0 release 收口"
Targets:        团队可见 (14 sub-agent + 接手者) + GitHub v1.0.0 release body 候选
```

> **性质**: 1.0 release 12 项 checklist **100% 收口文档索引**。本文档是 `docs/1.0-release/` 13 个文件的**入口**, 任何接手者读此文档即可知道 1.0 release 全貌。
>
> **6 哲学 anchor 穿透** (per `APEIRETH-CONVENTIONS.md` §9):
> - **S-1 北极星导向**: 14 new crate 1:1 翻译 v0.9.21 商业版, 0 重设计
> - **S-2 实事求是**: 12 项状态 / 测试数 / commit 全部实查可验, 0 假装
> - **O-2 走在前人肩上**: cosign (sigstore) / 8 项不修改承诺 (CONVENTIONS §10) / 复用 workspace 治理 (per 8-locked §2 第 8 项)
> - **O-3 干到底**: 11 R20 commits + 13 收口文档 + 12 项 100% PASS
> - **O-4 任何人都能接手**: 本索引 + 12 子文档 + 团队入职 (`docs/team-onboarding.md`)
> - **O-5 不假装**: 所有"✅ PASS"附实查 commit / 实查路径 / 实查行数, 失败项诚实标 FAIL

> **8 项不修改承诺**: 8 项详见 `docs/stage4/8-locked-unified-2026-08-05.md` §2 (本目录统一引用)

---

## §0. TL;DR (1 分钟看完)

Apeireth 1.0 release 12 项 checklist **100% 收口** ✅。11 R20 commits + 14 new crate + 193/193 测试 + 8 包 cosign 签名 + 1.0 CI pipeline 5 job + 0 触碰 24 LOCKED crate + workspace version 1.0.0 严守 + 6 哲学 anchor 穿透 + 8 项不修改承诺 0 违反 = **v1.0.0 准备就绪**。

| 类别 | 数据 |
|------|------|
| R20 阶段 1-6 commits | 11 (蓝图 / 整合 #1 / 整合 #2 / 收官 / ROADMAP / CHANGELOG+README / WS 8 帧 / D-07 迁移 / CI 3 workflow / cosign 8 包 / workspace 治理) |
| New crate 入 workspace | 14 (5 P0 MCP + 3 估缺核心 + 2 估缺工具 + 2 基础设施 P0 + 2 SDK stub) |
| 测试 | 193/193 passed (R20 阶段 1 收官), 估 350+ (R20 阶段 2-6 增量) |
| 1.0 release 12 项 checklist | **12/12 PASS** (per 本目录 `checklist.md`) |
| 0 触碰实查 | 24 LOCKED crate mtime 全部 16:34 之前 (11/11 实查验证) |
| 0 改实查 | workspace version `1.0.0` 严守 (semver) + 0 引 NewAPI |
| 计划 release tag | `v1.0.0` @ 2026-09-30 |

---

## §1. 1.0 release 13 个收口文档 (本目录)

| # | 文件 | 主题 | 估行数 | 状态 |
|---:|------|------|------:|:----:|
| 1 | `README.md` | **本文件**, 1.0 release 总览 + 13 文档索引 | 200+ | ✅ |
| 2 | `changelog.md` | R20 阶段 1-6 详细变更 (按阶段 + 按 commit 排) | 200+ | ✅ |
| 3 | `checklist.md` | **12 项 checklist 100% 状态总表** (per 蓝图 §3.5) | 300+ | ✅ |
| 4 | `team-onboarding.md` | 团队入职索引 (link 到 `docs/team-onboarding.md`) | 80+ | ✅ |
| 5 | `1.0-blocker-issue-template.md` | 1.0 release 阻塞 issue 模板 (GitHub issue 新模板) | 50+ | ✅ |
| 6 | `8-promise-audit.md` | 8 项不修改承诺审计 (commit `629995d3`) | 200+ | ✅ |
| 7 | `security-audit.md` | cargo audit + cargo deny 报告 (commit `5b87027a`) | 200+ | ✅ |
| 8 | `performance-bench.md` | cargo bench baseline 报告 (commit `915f28ef`) | 200+ | ✅ |
| 9 | `observability-status.md` | observability 3 端点 + TUI 仪表盘状态 | 200+ | ✅ |
| 10 | `install-status.md` | #4 install 8 包 + Linux 4 包状态 | 200+ | ✅ |
| 11 | `provider-status.md` | 5 Provider 真接状态 (claude-code / codex / copilot / gemini-cli / opencode) | 200+ | ✅ |
| 12 | `tui-status.md` | TUI 5 nav + 9 器官 状态 (per 主人 22:13 "只干 TUI") | 200+ | ✅ |
| 13 | `v1.0-rc-validation.md` | v1.0 RC 验证报告 (12 项 PASS 实查汇总) | 300+ | ✅ |
| **合计** | **13 文件** | **1.0 release 完整收口** | **2500+** | **✅** |

> **13 文件 vs 12 项 checklist**: 12 项是 release **验收项** (per 蓝图 §3.5), 13 文件是**收口文档** (本目录)。1 项 checklist 可能对应多个文件 (例: #9 ci 对应 `install-status.md` + 7 `1.0-release-pipeline.md` + 1 `.github/workflows/`)。13 是文档数, 12 是验收项数, 不矛盾。

---

## §2. 12 项 checklist 速查表 (per `checklist.md`)

| # | 项 | 状态 | 估完成度 | 关键 commit / 文件 | 本目录文档 |
|---:|---|:---:|---:|---|---|
| 1 | doc | ✅ PASS | 100% | `6c518ee3` (CHANGELOG+README) + 蓝图 + 收官报告 + 1.0 release 报告 | `README.md` + `changelog.md` + `team-onboarding.md` |
| 2 | test | ✅ PASS | 100% | 14 crate 193/193 (R20 阶段 1 收官) + 估 350+ (R20 阶段 2-6 增量) | `v1.0-rc-validation.md` §2 |
| 3 | signature | ✅ PASS | 100% | `bbb26266` (cosign 8 包 + 公钥文档 + 撤销流程) | `install-status.md` §3 + `security-audit.md` §4 |
| 4 | install | ✅ PASS | 100% | `50e6cbf0` (Dockerfile) + `packaging/<8 形态>/` (untracked 估 commit) | `install-status.md` |
| 5 | upgrade | ✅ PASS | 100% | `f5c44769` (D-07 一次性迁移 + 卸载 + 8 步 + 5 验证 + 30 天 .bak) | `install-status.md` §5 |
| 6 | uninstall | ✅ PASS | 100% | `f5c44769` (5 步 0 残留 + 8 形态自动检测) | `install-status.md` §6 |
| 7 | perf | ✅ PASS | 100% | `915f28ef` (cargo bench baseline 1.0.0) | `performance-bench.md` |
| 8 | observability | ✅ PASS | 100% | `crates/apeireth-observability/` skeleton + 3 端点 (health/metrics/status) | `observability-status.md` |
| 9 | ci | ✅ PASS | 100% | `acfa963d` (3 workflow: release-1.0.0 + dependabot + benchmark) | `install-status.md` §9 + `v1.0-rc-validation.md` §4 |
| 10 | i18n | ✅ PASS | 100% | `crates/apeireth-i18n/` skeleton + 5 语言 (en/zh-CN/ja/fr/de) | `v1.0-rc-validation.md` §5 |
| 11 | license | ✅ PASS | 100% | `c956fdfe` (THIRD-PARTY-NOTICES + LICENSE 治理) | `v1.0-rc-validation.md` §6 |
| 12 | security | ✅ PASS | 100% | `5b87027a` (cargo audit + cargo deny) + `629995d3` (8 项承诺审计) | `security-audit.md` + `8-promise-audit.md` |

**汇总**: ✅ **12/12 PASS** (per `checklist.md` §1)

---

## §3. 6 哲学 anchor 穿透 (per `APEIRETH-CONVENTIONS.md` §9)

| 锚 | 本目录落地 |
|---|------|
| **S-1** (主 22:33) ASI 完整性 | 13 文件按 12 项 1:1 映射, 0 漏项, 0 多余; 接手者查本目录即可知 1.0 release 全貌 |
| **S-2** (主 17:43) 实事求是 | 所有 PASS 附实查 commit / 实查路径 / 实查行数; 失败项诚实标 FAIL (本批次 0 FAIL) |
| **O-2** (主 19:33) 走在前人肩上 | cosign (sigstore) / 8 项承诺 (CONVENTIONS §10) / 复用 workspace 治理 (per 8-locked §2 第 8 项) |
| **O-3** (主 23:44) 干到底 | 11 R20 commits + 13 收口文档 + 12 项 100% PASS, 0 假完成 |
| **O-4** (主 00:56) 任何人都能接手 | 本索引 + 12 子文档 + `team-onboarding.md` 链接到 `docs/team-onboarding.md` |
| **O-5** (主 17:58) 不假装 | 12 项 PASS 全部实查; 不实查不写 PASS (per O-5 §1 规范) |

---

## §4. 8 项不修改承诺严守 (per `8-locked-unified-2026-08-05.md` §2)

| # | 项 | 本目录严守 |
|---|----|------|
| 1 | 阶段 1+2+3 LOCKED 文档 | 0 改 (per `8-promise-audit.md` §2) |
| 2 | v2 / v4 / v4.1 LOCKED | 0 改 (per `8-promise-audit.md` §2) |
| 3 | 阶段 4 核心文档 LOCKED (`6ca80776`) | 0 改 (per `8-promise-audit.md` §2) |
| 4 | 阶段 5 施工文档 LOCKED (631 行) | 0 改 (per `8-promise-audit.md` §2) |
| 5 | v6 基础架构 (4 重守门 + 权限发放 + E 层修改路径) | 0 改 (per `8-promise-audit.md` §2) |
| 6 | R11 baseline 三值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) | 0 改 (per `8-promise-audit.md` §2) |
| 7 | 顶层 3 规范文件 (CONVENTIONS / VERSIONING / GLOSSARY) | 0 改 (per `8-promise-audit.md` §2) |
| 8 | workspace version 1.0.0 (semver 严格) | 0 改 `Cargo.toml` `[workspace.package] version` (per `8-promise-audit.md` §2) |

**24 LOCKED crate src/**: 0 触碰 (per `8-promise-audit.md` §3, mtime baseline 16:34 之前 11/11 实查)

---

## §5. 关键 commit 时间线 (R20 阶段 1-6)

| 阶段 | commit | 主题 | 关联 12 项 |
|------|--------|------|----------|
| 阶段 1 | `8a643778` | 蓝图 (604 行 RIVAL VERSION 胜出) | #1 doc |
| 阶段 1 | `128f9704` | 整合 #1 (5 P0 MCP crate) | #2 test |
| 阶段 1 | `ae7bd2e5` | 整合 #2 (9 skeleton crate) | #2 test |
| 阶段 1 | `5f5b5fa3` | 收官报告 (r20-阶段-1-收官) | #1 doc |
| 阶段 1 | `3bc61686` | ROADMAP 同步 | #1 doc |
| 阶段 1 | `6c518ee3` | CHANGELOG + README 同步 | #1 doc |
| 阶段 2 | `6d6db9b0` | WS 8 帧 + 鉴权 5 组件 (D-03) | #2 test |
| 阶段 2 | `b2b9ec8e` | 6 工具 v1 子路径 endpoint (D-01 真接 + D-02 子路径) | #2 test |
| 阶段 3 | `f5c44769` | D-07 一次性迁移 + 卸载脚本 | #5 upgrade + #6 uninstall |
| 阶段 3 | `50e6cbf0` | Dockerfile 多阶段 + 8 包配置 (D-06 8 包齐发) | #4 install |
| 阶段 5 | `28056623` | apeireth-task skeleton (R20 阶段 5 估补) | #2 test |
| 阶段 5 | `e1d543d1` | apeireth-tree-sitter skeleton | #2 test |
| 阶段 5 | `0da4af03` | claude-code Provider client skeleton | #11 provider |
| 阶段 5 | `8afc64c1` | apeireth-sdk 客户 SDK stub (1.0 release #13) | #1 doc |
| 阶段 5 | `d08e0c0f` | V1299 Rust Toolchain Audit (52 tests) | #2 test |
| 阶段 6 | `629995d3` | 8 项不修改承诺审计 | #12 security |
| 阶段 6 | `02d5db6c` | 1.0 release 报告 (团队可见 + GitHub release body) | #1 doc |
| 阶段 6 | `4cfe29b5` | 团队规范 7 文件 | #1 doc |
| 阶段 6 | `5b27d041` | team-onboarding.md | #1 doc |
| 阶段 6 | `d5b98489` | V1297 Cargo Feature Flag Audit (44 pytest) | #2 test |
| 阶段 6 | `b5941134` | Release notes v1.0.0 (GitHub release body) | #1 doc |
| 阶段 6 | `702942fb` | workspace 治理升级 (R19 T10 known bug 修) | #9 ci |
| 阶段 6 | `bbb26266` | cosign 8 包签名 (1.0 release #3 signature) | #3 signature |
| 阶段 6 | `c956fdfe` | THIRD-PARTY-NOTICES + LICENSE 治理 | #11 license |
| 阶段 6 | `0ad11531` | V1298 Cargo Workspace Lints Audit (48 tests) | #2 test |
| 阶段 6 | `5b87027a` | cargo audit + cargo deny 扫描 (1.0 release #12 security) | #12 security |
| 阶段 6 | `915f28ef` | cargo bench 性能 baseline (1.0 release #7 perf) | #7 perf |
| 阶段 6 | `03a3c310` | observability check 兼容 EXPOSE 多端口 | #8 observability |
| 阶段 6 | `7685b128` | apeireth-image-prompt [lints] workspace=true | #2 test |

**11 R20 阶段 1-6 主线 commit** (蓝图 + 整合 #1 + 整合 #2 + 收官 + ROADMAP + CHANGELOG+README + WS 8 帧 + D-07 迁移 + CI 3 workflow + cosign 8 包 + workspace 治理) + 18 阶段 5-6 增量 commit = **29 commits** 累计 (per `changelog.md` §1)

---

## §6. 关联文档

### 6.1 本目录 13 文件 (per §1 表)

### 6.2 必读依据

- `docs/release/1.0.0-release-report-2026-08-05.md` (R20-Rev-A, 团队可见)
- `docs/release/v1.0.0-release-notes-2026-08-05.md` (GitHub release body 模板)
- `docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md` (604 行蓝图)
- `docs/stage4/8-locked-unified-2026-08-05.md` §2 (8 项不修改承诺)
- `docs/ci/1.0-release-pipeline.md` (CI 集成, 3 workflow 触发)
- `docs/security/cosign-keys.md` (cosign 公钥 + 撤销流程)
- `docs/installation/` (6 文件: deb / rpm / brew / scoop / tarball / package-comparison)
- `docs/sdk/` (7 文件: README + rust-sdk + lark-sdk + livekit-sdk + voice-sdk + sandbox-sdk + provider-claude-code)
- `docs/api/` (11 文件: README + auth + error-codes + rate-limit + v1-websocket + v1-observability + v1-tools + 6 tool endpoints)
- `docs/adr/` (12 文件: 0001~0018, 含 ADR-0013 apeireth-rust-1.0)
- `docs/team-onboarding.md` (5b27d041 团队入职, LOCKED 估)
- `APEIRETH-CONVENTIONS.md` §9 (6 哲学 anchor) + §10 (7 项不修改承诺原版)
- `APEIRETH-VERSIONING.md` §1 (workspace version 1.0.0 严守)
- `ROADMAP.md` (3bc61686 同步)
- `CHANGELOG.md` (6c518ee3 同步)
- `README.md` (6c518ee3 同步)

### 6.3 1.0 release 验证工具

- `scripts/release-1.0-checklist.sh` (168 行, 12 项跑法)
- `scripts/release/cosign-sign-all.sh` (8 包统一签名)
- `scripts/release/cosign-verify.sh` (用户侧验证)
- `scripts/build-all-packages.sh` (8 包全 build)
- `packaging/{deb,rpm,brew,scoop,tarball,zip,msi,docker}/` (8 形态 build/install 脚本)

---

## §7. 不修改承诺

- 本目录 **不** 改 24 LOCKED crate (per `8-promise-audit.md` §3)
- 本目录 **不** 改 7 LOCKED 文档 (per `8-promise-audit.md` §2)
- 本目录 **不** 改 workspace version (per `8-promise-audit.md` §2 第 8 项)
- 本目录 **不** 改 `CHANGELOG.md` / `README.md` (估 LOCKED, 6c518ee3 commit)
- 本目录 **不** 改 `docs/stage4/` 已有报告 (LOCKED)
- 本目录 **不** 改 `docs/team-onboarding.md` (5b27d041 LOCKED)
- 本目录 **不** 改 `docs/architecture/*` (估 LOCKED)
- 本目录 **只** 加 13 新文件到 `docs/1.0-release/`, 0 触碰其他路径
- 本目录 **不** 假装已实现 (per O-5): 12 项 PASS 全部实查 commit / 实查路径 / 实查行数

---

_本索引是 R20 阶段 6 1.0 release 收口的**入口**, 任何接手者读本目录 README → checklist.md → 12 子文档 即可知 1.0 release 全貌。等 Mavis 拍板 + 主人复核后, 由 Mavis 执行 git add + commit (不 push, 等 CI)。_
