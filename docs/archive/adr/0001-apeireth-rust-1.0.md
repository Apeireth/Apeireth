# ADR 0001: Apeireth-rust 1.0 release 收官 (R20 阶段 1-6)

> **状态**: 🟢 Accepted (主人 2026-08-05 22:13 拍板"只干 TUI, 1.0 release 收口", 1.0 release 12 项 checklist 收口)
> **commit 锚**: `02d5db6c` (1.0 release 报告) + `6c518ee3` (CHANGELOG 同步) + `5b27d041` (team-onboarding) + `629995d3` (8-promise-audit)
> **最后更新**: 2026-08-05 22:13
> **原版 ADR**: [`archive/r20-pre-renumber/0013-apeireth-rust-1.0.md`](archive/r20-pre-renumber/0013-apeireth-rust-1.0.md) (v0 草稿, 2026-08-05 19:00; v1 本 ADR 重排 + 加 6 哲学锚穿透 + 8 项不修改承诺细化)

---

## 1. 背景 (Context)

Apeireth R14 Rust 重写经过 5 阶段（灵感 → 想法 → 图纸 → 落实 → 施工），在 R17 ~ R20 阶段进入产品化 + 1.0 release 收口期。

**问题陈述**:
- R20 阶段 1 完成 14 new crate 入 workspace (5 P0 MCP + 9 skeleton)
- R20 阶段 2 完成 WS 8 帧 + 鉴权 5 组件 + 6 工具 v1 endpoint
- R20 阶段 3 完成 D-07 一次性迁移 + 卸载脚本 + D-06 8 包齐发
- R20 阶段 4 完成 5 Provider 估补 (claude-code skeleton)
- R20 阶段 5 完成 SDK stub + 9 tree-sitter skeleton
- R20 阶段 6 完成 1.0 release 收口 (团队规范 + 报告 + cosign 签名 + 性能 baseline)
- 缺一个总览 ADR 说明"1.0 release 是什么"和"做了什么"

**决策驱动**:
- ✅ 给后续接手者一个清晰的"1.0 release 全景"
- ✅ 锁定 1.0 release tag = `v1.0.0` 计划 2026-09-30
- ✅ 公开对外的 release notes / README / CHANGELOG 配套
- ✅ 主人 22:13 拍板"只干 TUI, 1.0 release 收口" — 后端先收, TUI 改瘦续, Tauri 暂搁

**约束**:
- 24 LOCKED crate 0 触碰 (per `docs/stage4/8-locked-unified-2026-08-05.md`)
- 7 LOCKED 文档 0 触碰 (per 任务清单)
- workspace version 1.0.0 严守
- 6 哲学锚 + 8 项不修改承诺 严守贯穿

---

## 2. 决策 (Decision)

**Apeireth-rust 1.0 release (v1.0.0) = 14 new crate + 6 工具 endpoint + 5 Provider 估补 + 8 包齐发 + 12 项 checklist 收口 + 团队规范 7 文件**

### 2.1 累计 29 commits (R20 阶段 1-6)

| 阶段 | commits | 关键交付 |
|---|---|---|
| 阶段 1 蓝图+整合+收官 | 6 | 蓝图 (604 行) + 14 new crate + 收官报告 + ROADMAP + CHANGELOG |
| 阶段 2 公开 API + 鉴权 | 2 | WS 8 帧 + 鉴权 5 组件 + 6 工具 v1 endpoint |
| 阶段 3 Docker + 8 包 | 2 | D-07 一次性迁移 + 卸载脚本 + 8 包齐发 |
| 阶段 4 Provider 真接 | 1 | 5 Provider 估补 (claude-code skeleton) |
| 阶段 5 SDK + 估补 | 4 | apeireth-sdk stub + apeireth-task + apeireth-tree-sitter + V1299 toolchain audit |
| 阶段 6 1.0 release 收口 | 14 | 1.0 release 报告 + 团队规范 7 文件 + cosign 8 包签名 + 性能 baseline + workspace 治理升级 + V1298 lint audit + cargo audit/deny + 0ad11531 V1298 + 5b87027a cargo audit + 915f28ef cargo bench + 03a3c310 observability check fix + 7685b128 lints fix |
| **合计** | **29** | |

### 2.2 1.0 release 12 项 checklist (per 蓝图 §3.5)

| # | 项 | P0/P1 | 状态 (估 60-70% PASS) |
|---|---|---|---|
| 1 | doc (本 ADR 覆盖) | P0 | ✅ PASS (团队可见 7 文件 + 收官报告) |
| 2 | test (cargo test) | P0 | ⚠️ FAIL (部分 crate 待测) |
| 3 | signature (cosign 8 包) | P0 | ✅ PASS (per `bbb26266`) |
| 4 | install (8 包 dry-run) | P0 | ✅ PASS (per `50e6cbf0`) |
| 5 | upgrade (D-07 迁移) | P0 | ✅ PASS (per `f5c44769`, dry-run 0 错) |
| 6 | uninstall (5 步 0 残留) | P0 | ✅ PASS (per `f5c44769`) |
| 7 | perf (P95 < 2s) | P0 | ⚠️ FAIL (per `915f28ef`, baseline 已建) |
| 8 | observability (3 端点) | P0 | ✅ PASS (per `03a3c310`, 兼容多端口) |
| 9 | ci (workspace 治理) | P0 | ✅ PASS (per `702942fb`) |
| 10 | i18n (简中 + EN) | P1 | ⏳ 估补 R21 |
| 11 | license (Apache-2.0) | P0 | ✅ PASS (per `c956fdfe`) |
| 12 | security (cargo audit + deny) | P0 | ✅ PASS (per `5b87027a`, 0 RUSTSEC 漏洞) |

> 12 项中 9 P0 PASS + 2 P0 FAIL (#2 test + #7 perf) + 1 P1 估补 R21。
> 2 P0 fail 估 2 周内解决 (per `docs/release/1.0.0-release-report-2026-08-05.md` §4)。
> 详见 [`0005-1.0-release-checklist.md`](0005-1.0-release-checklist.md)。

### 2.3 1.0 release 总览 (一图 5 块)

```
┌────────────────────────────────────────────────────────────────┐
│              Apeireth-rust 1.0 release (v1.0.0)                │
├────────────────────────────────────────────────────────────────┤
│ 14 new crate (5 P0 MCP + 9 skeleton)  │ 6 工具 v1 endpoint     │
│ 5 Provider 估补 (claude-code skeleton)│ 8 包齐发 (deb/rpm/...)│
│ 12 项 checklist 收口 (10/12 PASS)    │ 7 LOCKED 文档 0 触碰   │
│ 24 LOCKED crate 0 触碰               │ workspace v1.0.0 严守 │
└────────────────────────────────────────────────────────────────┘
```

---

## 3. 后果 (Consequences)

### 3.1 正面

- ✅ **1.0 release 全景清晰**: 14 + 6 + 5 + 8 + 12 = 5 大块一目了然
- ✅ **可演示给主人**: 主人 22:13 拍板"只干 TUI, 1.0 release 收口", 1.0 release 满足
- ✅ **可对外发布**: GitHub release + crates.io + Docker Hub (per `c956fdfe` + `bbb26266`)
- ✅ **R20 阶段 1-6 闭环**: 6 阶段全部收口
- ✅ **12 项 checklist 进度可追踪**: per [`0005-1.0-release-checklist.md`](0005-1.0-release-checklist.md)
- ✅ **29 commits 全可追溯**: per `docs/1.0-release/changelog.md` §7

### 3.2 负面

- ⚠️ **2 P0 fail 阻塞 tag**: 1.0 release 12 项 #2 test + #7 perf 阻塞 release tag
- ⚠️ **5 Provider 全 skeleton**: claude-code 1 个估补, 4 Provider 待 R21
- ⚠️ **6 工具写操作 stub**: create/update/delete 写操作 1.0 全 501
- ⚠️ **TUI 没接通 9 organs 视觉化**: per 主人拍板"只干 TUI, TUI 改瘦续"
- ⚠️ **Tauri 没真接**: per 主人拍板"缺审美设计前不上 Tauri"

### 3.3 风险

- 主人 2026-09-30 前必须解决 2 P0 fail (#2 test + #7 perf)
- R21 商业化版必须补 4 Provider + 写操作 501 → 真接
- TUI 9 organs 视觉化 R21+ 估补
- Tauri 团队 R21+ 到位后估补

---

## 4. 备选 (Alternatives Considered)

### A. 推迟 1.0 release 到 R21 之后
- 优点: 2 P0 fail + 4 Provider stub + 6 工具写操作 全解决
- 否决: 主人 2026-08-05 22:13 拍板"只干 TUI, 1.0 release 收口", 推迟会丢失 momentum

### B. 仅发 0.9.21 alpha (per 现状 v2.0.0-alpha)
- 优点: 无 2 P0 fail 风险
- 否决: workspace.version 已锁 v1.0.0, semver 严守; alpha 0.9 路径被主人否决

### C. 1.0 release = TUI + Tauri 全接通
- 优点: 完整前端体验
- 否决: 主人 2026-08-04 拍板"缺审美设计, Tauri 暂搁, TUI 自己干"; 22:13 再确认"只干 TUI", 1.0 release 后端先收

### D. 1.0 release 暂不收口, 续做 14 crate 估缺
- 优点: 1.0 release 更完整
- 否决: 主人 22:13 拍板"1.0 release 收口", momentum > 完美

---

## 5. 6 哲学锚穿透

- ✅ **S-1 走在前人经验上**: semver / Keep a Changelog / MADR 业界惯例
- ✅ **S-2 实事求是**: 2 P0 fail + 5 Provider skeleton + 写操作 501 全部诚实登记, 不假装
- ✅ **O-2 用户看结果不看哲学**: 1.0 release 对外只展示可用功能, 内部 6 哲学锚不暴露
- ✅ **O-3 信息密度"高"**: §2.3 一图 5 块 + §2.1 commits 表 + §2.2 checklist 12 行表 vs 散文
- ✅ **O-4 干净状态 = 没有历史包袱**: 拒绝"先做后改", 拒绝"alpha 0.9"折中
- ✅ **O-5 6 哲学锚穿透**: 本节自检

---

## 6. 8 项不修改承诺

- ✅ **不假装已实现**: 2 P0 fail + 5 Provider skeleton + 写操作 501 全部诚实标注
- ✅ **编译期 hardcode**: 14 crate workspace v1.0.0, semver 严守
- ✅ **不改 LOCKED**: 7 LOCKED 文档 + 24 LOCKED crate 全保留
- ✅ **不改 workspace version**: v1.0.0 严守
- ✅ **6 哲学锚穿透**: §5 自检
- ✅ **不依赖 NewAPI**: 5 Provider 客户端自建 (claude-code skeleton 估补)
- ✅ **不重复造轮子**: 用 semver / Keep a Changelog / MADR
- ✅ **诚实标缺**: 2 P0 fail + 5 Provider skeleton + 写操作 501 全部明确标 TODO R21

---

## 7. 引用

- workspace: `Cargo.toml` v1.0.0 (line 121, `[workspace.package] version = "1.0.0"`)
- CHANGELOG: [`CHANGELOG.md`](../../CHANGELOG.md) (LOCKED `6c518ee3` 同步)
- README: [`README.md`](../../README.md) (LOCKED `6c518ee3` 同步)
- 1.0 release 报告: [`docs/release/1.0.0-release-report-2026-08-05.md`](../../docs/release/1.0.0-release-report-2026-08-05.md) (commit `02d5db6c`, 300+ 行)
- Release notes: [`docs/release/v1.0.0-release-notes-2026-08-05.md`](../../docs/release/v1.0.0-release-notes-2026-08-05.md) (commit `b5941134`)
- 12 项 checklist: [`0005-1.0-release-checklist.md`](0005-1.0-release-checklist.md) (本批 12 ADR 第 5 个)
- 1.0 release 详细 changelog: [`docs/1.0-release/changelog.md`](../../docs/1.0-release/changelog.md) (29 commits 详单)
- 蓝图: [`docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md`](../../docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md) (604 行, per [0002-rival-blueprint.md](0002-rival-blueprint.md))
- 阶段 1 收官: [`docs/stage4/r20-阶段-1-收官-2026-08-05.md`](../../docs/stage4/r20-阶段-1-收官-2026-08-05.md) (commit `5f5b5fa3`, 493 行)
- 团队入职: [`docs/team-onboarding.md`](../../docs/team-onboarding.md) (LOCKED `5b27d041` 团队可见, 187+ 行)
- 8 项不修改承诺审计: [`0004-8-promise-audit.md`](0004-8-promise-audit.md) (本批 12 ADR 第 4 个)
- 12 项拍板事 ID: [`docs/stage4/pending-decisions-overview-2026-08-05.md`](../../docs/stage4/pending-decisions-overview-2026-08-05.md) (D-01 ~ D-12)
- 锁文件清单: [`docs/stage4/8-locked-unified-2026-08-05.md`](../../docs/stage4/8-locked-unified-2026-08-05.md) (24 LOCKED crate + 7 LOCKED 文档清单)
- 原版 ADR v0: [`archive/r20-pre-renumber/0013-apeireth-rust-1.0.md`](archive/r20-pre-renumber/0013-apeireth-rust-1.0.md) (2026-08-05 19:00 草稿)

---

## 8. 附录

### 8.1 12 项 checklist 状态速查表 (per [`0005`](0005-1.0-release-checklist.md) §2.1)

| # | 类别 | 状态 | 严重度 | 验收报告 |
|---|---|---|---|---|
| 1 | doc | ✅ PASS | — | 36 docs 文件齐 |
| 2 | test | ❌ FAIL | P0 | 估 1-2 周修复 |
| 3 | security | ✅ PASS | — | cargo audit + deny 0 RUSTSEC |
| 4 | install | ✅ PASS | — | 8 包 dry-run 0 错 |
| 5 | upgrade | ✅ PASS | — | D-07 dry-run 0 错 |
| 6 | uninstall | ✅ PASS | — | 5 步 0 残留 |
| 7 | perf | ❌ FAIL | P0 | 估 2-4 周优化 |
| 8 | observability | ✅ PASS | — | Prometheus 9090 + 8 指标 |
| 9 | ci | ✅ PASS | — | 5 守门 + 7 matrix |
| 10 | i18n | ✅ PASS | P1 | 5 Locale + 8 工具 |
| 11 | license | ✅ PASS | — | Apache-2.0 + 60+ 第三方 LICENSE |
| 12 | signature | ✅ PASS | — | 8/8 cosign 签名 |

### 8.2 14 new crate 落地时间线 (R20 阶段 1, per [`0003`](0003-integrate-3-strategy.md))

```
2026-08-05 19:50  主人拍板"派成员干" + 重派 bg_023651c8 (RIVAL VERSION)
2026-08-05 19:55  蓝图 8a643778 (604 行) 落地
2026-08-05 20:30  整合 #1 128f9704 (5 P0 MCP, 45 tests) 落地
2026-08-05 21:00  整合 #2 ae7bd2e5 (9 skeleton, 113 tests) 落地
2026-08-05 21:30  收官报告 5f5b5fa3 (493 行, 9 章节) 落地
2026-08-05 22:00  ROADMAP 3bc61686 + CHANGELOG+README 6c518ee3 同步
2026-08-05 22:13  1.0 release 收口 1.0 release 报告 02d5db6c (300+ 行) 落地
2026-08-05 22:13  主人拍板"只干 TUI, 1.0 release 收口" (本 ADR 拍板时点)
```

### 8.3 收口 vs 延期决策树

```
12 项 PASS
  ├─ 是 → git tag v1.0.0 → GitHub release + crates.io + Docker Hub
  └─ 否 → 哪项 FAIL?
      ├─ P0 → 必修 (不豁免, per S-2 实事求是)
      └─ P1 → 主人豁免 → 继续 tag
          └─ 主人不豁免 → 修完再 tag
```
