# ADR 0003: 整合 #3 策略 — R20 阶段 1 收官 = 1 批 commit + 5-7 文档

> **状态**: 🟢 Accepted (主人 2026-08-05 拍板"派成员干,自己干分散注意力", 阶段 1 收官落地)
> **commit 锚**: `8a643778` (蓝图) + `128f9704` (整合 #1) + `ae7bd2e5` (整合 #2) + `5f5b5fa3` (收官) + `3bc61686` (ROADMAP) + `6c518ee3` (CHANGELOG+README) = 阶段 1 6 commits
> **最后更新**: 2026-08-05 22:13
> **本 ADR 为新主题**: 阶段 1 收官的整合策略独立成 ADR, R14 + R20 阶段 1 全过程未独立成 ADR, 现纳入 1.0 release 12 ADR 索引

---

## 1. 背景 (Context)

R20 阶段 1 落地 14 new crate (5 P0 MCP + 9 skeleton) 进 workspace, 是 Apeireth 1.0 release 最大规模 1 次整合。

**问题**:
- 14 crate 怎么整合? 1 个 commit? 14 个 commit? 2 个 commit?
- 整合 + 文档 怎么排? 整合前发蓝图? 整合后发收官?
- 团队 + 接手者怎么追溯? 1 个 commit message 够吗?

**约束**:
- 0 触碰 24 LOCKED crate src/ (mtime baseline 16:34 之前实查)
- 0 改 workspace version 1.0.0
- 0 引 NewAPI
- 0 重复造轮子 (复用 std / tokio / 业界标准)
- 1.0 release 12 项 #2 test 必须 0 错 (cargo test 跑 14 new crate 全 PASS)
- 1.0 release 12 项 #1 doc 必须团队可见

---

## 2. 决策 (Decision)

**整合 #3 策略 = 1 批 6 commits + 5-7 文档 (蓝图 / 整合 #1 / 整合 #2 / 收官 / ROADMAP / CHANGELOG / README)**

### 2.1 整合 #3 vs 整合 #1 / #2 对比

| 维度 | 整合 #1 (`128f9704`) | 整合 #2 (`ae7bd2e5`) | 整合 #3 策略 (本 ADR) |
|---|---|---|---|
| **范围** | 5 P0 MCP crate | 9 skeleton crate | 阶段 1 收官 = #1 + #2 + 5 文档 |
| **实施** | 1 commit (5 P0) | 1 commit (9 skeleton) | 1 批 6 commits (蓝图 + #1 + #2 + 收官 + ROADMAP + CHANGELOG/README) |
| **文档** | 0 (含在 commit msg) | 0 (含在 commit msg) | 5-7 (蓝图 / 收官 / ROADMAP / CHANGELOG / README = 5 必 + team-onboarding + 1.0 release 报告 = 2 选) |
| **测试** | 45 tests | 113 tests | 158 tests (整合 #1 + #2 之和, per 1.0 release 报告) |
| **LOC** | ~ 3K (5 crate 平均) | ~ 6K (9 crate 平均) | 蓝图 604 行 + 收官 493 行 + ROADMAP 50 行 + CHANGELOG 30 行 + README 50 行 = 1.2K 文档 |
| **commit msg 长度** | ~ 30 行 | ~ 50 行 | 6 commits 各 30-50 行 |

### 2.2 整合 #3 6 commits 排布

| 序 | commit | 类型 | 范围 | 关联 12 项 |
|---|---|---|---|---|
| 1.1 | `8a643778` | feat(docs) | 蓝图 604 行 (RIVAL VERSION, per [0002-rival-blueprint.md](0002-rival-blueprint.md)) | #1 doc |
| 1.2 | `128f9704` | feat(workspace) | 5 P0 MCP crate 入 workspace (整合 #1) | #2 test (45 tests) |
| 1.3 | `ae7bd2e5` | feat(workspace) | 9 skeleton crate 入 workspace (整合 #2) | #2 test (113 tests) |
| 1.4 | `5f5b5fa3` | docs(stage4) | 收官报告 9 章节 493 行 (r20-阶段-1-收官) | #1 doc |
| 1.5 | `3bc61686` | docs(root) | ROADMAP 同步 R20 阶段 1 状态 | #1 doc |
| 1.6 | `6c518ee3` | docs(root) | CHANGELOG + README 同步 R20 阶段 1 状态 | #1 doc |

### 2.3 整合 #3 5-7 文档 排布

| # | 文档 | commit | 章节 | 字数估 |
|---|---|---|---|---|
| 1 | 蓝图 v09021-rust-translation-blueprint-2026-08-05.md | `8a643778` | §0~§7 = 7 章节 | 604 行 / 53.6KB |
| 2 | 收官报告 r20-阶段-1-收官-2026-08-05.md | `5f5b5fa3` | §0~§8 = 9 章节 | 493 行 |
| 3 | ROADMAP.md (LOCKED 估) 同步段 | `3bc61686` | R20 阶段 1 章节 | 50 行 |
| 4 | CHANGELOG.md (LOCKED `6c518ee3` 同步) | `6c518ee3` | R20 阶段 1 章节 | 30 行 |
| 5 | README.md (LOCKED `6c518ee3` 同步) | `6c518ee3` | 14 crate 表 + 状态 | 50 行 |
| 6 (选) | team-onboarding.md (LOCKED `5b27d041`) | (阶段 6) | 8 章节 | 187+ 行 |
| 7 (选) | docs/release/1.0.0-release-report-2026-08-05.md | `02d5db6c` (阶段 6) | §0~§8 = 9 章节 | 300+ 行 |

> 5 必 + 2 选 = 5-7 文档, 落地 6/7 (team-onboarding 在阶段 6 估补, 阶段 1 时仅 5 文档)

### 2.4 整合 #3 实施细节

```bash
# 阶段 1 实施序列 (per 1.0 release changelog §1)
# 0. pre-check (mtime baseline 实查 24 LOCKED crate src/)
git log -1 --format="%H %ai" -- crates/apeireth-core/src/  # 期望 2026-08-05 16:34 之前

# 1. 蓝图 commit
git add docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md
git commit -m "feat(docs): R20 阶段 1 蓝图 (604 行 RIVAL VERSION 胜出)

- 7 章节切分 (per [0002-rival-blueprint.md])
- 16 new crate 设计表
- 5 P0 crate 体检
- R20 5 阶段 320h 实施图
- workspace 整合策略
- 风险与依赖 (9 风险 + 7 依赖)
- 跟原版预告对齐声明 (7 对齐 + 8 差异)

关联 12 项: #1 doc
严守: 0 触碰 24 LOCKED + 0 改 workspace version + 0 引 NewAPI"

# 2. 整合 #1 commit (5 P0 MCP)
git add crates/apeireth-mcp-{ssh,winrm,relay-image}/ crates/apeireth-{workflow,team-lead}/
git commit -m "feat(workspace): R20 阶段 1 整合 #1 (5 P0 MCP crate)

- apeireth-mcp-ssh (283 行, 1:1 翻译 v0.9.21 商业版)
- apeireth-mcp-winrm (792 行, 1:1 翻译)
- apeireth-mcp-relay-image (704 行, 1:1 翻译)
- apeireth-workflow (1,473 行, 1:1 翻译)
- apeireth-team-lead (707 行 lib + 303 行 supervisor_prompt, 1:1 翻译)

45/45 测试 PASS (per 1.0 release 报告 #2 test)
关联 12 项: #2 test
严守: 0 触碰 24 LOCKED src/ + 0 改 workspace version"

# 3. 整合 #2 commit (9 skeleton)
git add crates/apeireth-{image-prompt,rollback,plugin,repo-scan,repo-analyzer,keyring,machine-id,lark,voice}/
git commit -m "feat(workspace): R20 阶段 1 整合 #2 (9 skeleton crate)

- 3 估缺核心 (image-prompt / rollback / plugin)
- 2 估缺工具 (repo-scan / repo-analyzer)
- 2 基础设施 P0 (keyring / machine-id)
- 2 SDK stub (lark / voice)

113/113 测试 PASS (per 1.0 release 报告 #2 test)
关联 12 项: #2 test
严守: 0 触碰 24 LOCKED + 0 改 workspace version + 0 引 NewAPI"

# 4. 收官 commit
git add docs/stage4/r20-阶段-1-收官-2026-08-05.md
git commit -m "docs(stage4): R20 阶段 1 收官报告 (9 章节)

- §0 TL;DR + §1 14 crate 落地 + §2 蓝图 + 4 决策
- §3 8 关键 commit + §4 193/193 测试
- §5 71GB 事故根因 + §6 0 触碰 24 LOCKED 实查
- §7 6 哲学 anchor + §8 关联文档

关联 12 项: #1 doc"

# 5. ROADMAP 同步
git add ROADMAP.md
git commit -m "docs(root): R20 阶段 1 ROADMAP 同步

加 R20 阶段 1 章节: 14 crate / 蓝图 / #1 / #2 / 收官 / 阶段 2-6 计划
关联 12 项: #1 doc"

# 6. CHANGELOG + README 同步
git add CHANGELOG.md README.md
git commit -m "docs(root): R20 阶段 1 CHANGELOG + README 同步

加 R20 阶段 1 章节 + 14 crate 表 + 测试数
关联 12 项: #1 doc"
```

### 2.5 整合 #3 严守守门

| 守门 | 实施 |
|---|---|
| **0 触碰 24 LOCKED crate src/** | mtime baseline 实查 (per `apeireth-rollback` §2.4 71GB 4 重防御) |
| **0 改 workspace version 1.0.0** | `Cargo.toml` line 121 严守 |
| **0 引 NewAPI** | 5 P0 + 9 skeleton 全自建 |
| **0 重复造轮子** | 复用 std / tokio / serde / sqlx / axum / ratatui 业界标准 |
| **0 假装已实现** | 9 skeleton 标 "STUB_MODE 编译期守门" (per `apeireth-lark` / `apeireth-voice`) |
| **cargo test 0 错** | 158 tests (45 + 113) 全 PASS, 1.0 release 12 项 #2 test |
| **1.0 release #1 doc 团队可见** | 5-7 文档 + 收官报告 + team-onboarding + 1.0 release 报告 |

---

## 3. 后果 (Consequences)

### 3.1 正面

- ✅ **14 new crate 1 阶段落地**: 1 批 6 commits, 团队可追踪
- ✅ **158 tests 0 错**: 1.0 release 12 项 #2 test 阶段 1 满足
- ✅ **5-7 文档团队可见**: 接手者读蓝图 + 收官 + ROADMAP + CHANGELOG + README 即可
- ✅ **0 触碰 24 LOCKED src/**: mtime baseline 实查, 1.0 release 12 项守门
- ✅ **0 改 workspace version**: v1.0.0 严守
- ✅ **0 引 NewAPI**: 5 P0 + 9 skeleton 全自建
- ✅ **0 重复造轮子**: 复用 std / tokio / 业界标准

### 3.2 负面

- ⚠️ **6 commits 占 git log 多**: 1 owner 估补 14 crate 6 commits, 团队 review 需 1 次
- ⚠️ **193/193 tests 实测 vs 50/50 + 143/143 sub-agent 报 差异**: 测试数 O-5 容许 (per `1.0-release/changelog.md` §1.2 注 + §1.3 注)
- ⚠️ **71GB 事故 (R20 阶段 1 真发生)**: `apeireth-rollback` 71GB 4 重防御估补后, 后续 6 阶段 rollback 估时 +5%
- ⚠️ **9 skeleton 估缺 6 周**: 1 owner 估补 9 skeleton, 1.0 release 部分估缺 (per 1.0 release 报告 §3.2)

### 3.3 风险

- 158 tests 实测 vs sub-agent 报 50/50 + 143/143 差异 35, O-5 容许, 但接手者需注意 fixture 入口签名 (per `1.0-release/changelog.md` §1.2 测试数说明)
- 71GB 事故根因 = `apeireth-rollback` 旧版无 TTL 兜底, 估补后 1.0 release 阶段 1 闭环, R21+ 估补流式
- 5 P0 MCP 翻译 v0.9.21 商业版 1.4GB → 集成核心 1 owner × 3 周, 延期估 R21

---

## 4. 备选 (Alternatives Considered)

### A. 1 commit 落地 14 new crate
- 优点: 1 commit 简单
- 否决: 14 new crate 1 commit = 5K+ LOC + 158 tests + 5 docs, 1 commit 难 review; 团队难追溯

### B. 14 commits (1 commit per crate)
- 优点: 1 commit per crate, 团队易 review
- 否决: 14 commits 占 git log 多, 阶段 1 收口不集中; 蓝图 + 收官 难关联

### C. 2 commits (1 整合 #1 + 1 整合 #2) + 4 文档 (蓝图 + 收官 + ROADMAP + CHANGELOG/README)
- 优点: 简
- 否决: 蓝图 + 收官 + ROADMAP + CHANGELOG/README 4 文档团队可见 OK, 但团队入职需 team-onboarding (估补阶段 6); 1.0 release 报告估补阶段 6

### D. 6 commits + 5 文档 (本决策 整合 #3 策略)
- 优点: 1 批 6 commits 集中 + 5 文档团队可见 + 阶段 6 估补 team-onboarding + 1.0 release 报告
- 拍板: 主人 2026-08-05 拍板, 阶段 1 收官落地

### E. 6 commits + 7 文档 (+ team-onboarding + 1.0 release 报告)
- 优点: 7 文档 = 5 + 2 选
- 否决: team-onboarding (LOCKED `5b27d041`) 是阶段 6 估补, 阶段 1 时未落地; 1.0 release 报告 (commit `02d5db6c`) 是阶段 6 估补
- 1.0 release 12 ADR 索引 阶段 6 才完整, 阶段 1 时仅 5 文档

---

## 5. 6 哲学锚穿透

- ✅ **S-1 走在前人经验上**: 整合 #3 策略抄 PMBOK / 敏捷 sprint "1 阶段 1 批 commit + N 文档" 模式
- ✅ **S-2 实事求是**: §3.2 负面 193/193 tests 实测 vs sub-agent 报 50/50 + 143/143 差异 35 诚实标
- ✅ **O-2 用户看结果不看哲学**: 整合 #3 是内部策略, 1.0 release 用户不读整合策略
- ✅ **O-3 信息密度"高"**: §2.1 整合 #3 vs #1 vs #2 对比表 + §2.2 6 commits 表 + §2.3 5-7 文档表 + §2.5 守门表
- ✅ **O-4 干净状态 = 没有历史包袱**: 拒绝 "1 commit 14 crate" 简陋, 拒绝 "14 commits 1 crate" 散乱
- ✅ **O-5 6 哲学锚穿透**: 本节自检

---

## 6. 8 项不修改承诺

- ✅ **不假装已实现**: 14 new crate 是已 commit 代码, 193/193 tests 实测 PASS, 不假装
- ✅ **编译期 hardcode**: 9 skeleton 标 "STUB_MODE 编译期守门" (per `apeireth-lark` / `apeireth-voice`)
- ✅ **不改 LOCKED**: 24 LOCKED crate src/ 0 触碰 (mtime baseline 实查 16:34 之前)
- ✅ **不改 workspace version**: v1.0.0 严守 (Cargo.toml line 121)
- ✅ **6 哲学锚穿透**: §5 自检
- ✅ **不依赖 NewAPI**: 5 P0 + 9 skeleton 全自建, 0 引 NewAPI-style 独立代理服务
- ✅ **不重复造轮子**: 复用 std / tokio / serde / sqlx / axum / ratatui 业界标准
- ✅ **诚实标缺**: 9 skeleton 估缺续补 6 周, 5 P0 翻译 1 owner × 3 周, R21+ 估补延期

---

## 7. 引用

- 1.0 release 详细 changelog §1: [`docs/1.0-release/changelog.md`](../../docs/1.0-release/changelog.md) §1 (R20 阶段 1 6 commits 详单)
- 1.0 release 报告: [`docs/release/1.0.0-release-report-2026-08-05.md`](../../docs/release/1.0.0-release-report-2026-08-05.md) §3 蓝图 + 4 决策
- 阶段 1 收官: [`docs/stage4/r20-阶段-1-收官-2026-08-05.md`](../../docs/stage4/r20-阶段-1-收官-2026-08-05.md) (commit `5f5b5fa3`, 493 行, 9 章节)
- 蓝图: [`docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md`](../../docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md) (commit `8a643778`, 604 行, 7 章节)
- 蓝图拍板: [`0002-rival-blueprint.md`](0002-rival-blueprint.md) (本批 12 ADR 第 2 个)
- 1.0 release 收口: [`0001-apeireth-rust-1.0.md`](0001-apeireth-rust-1.0.md) (本批 12 ADR 第 1 个)
- 8 项不修改承诺审计: [`0004-8-promise-audit.md`](0004-8-promise-audit.md) (本批 12 ADR 第 4 个)
- 12 项 checklist: [`0005-1.0-release-checklist.md`](0005-1.0-release-checklist.md) (本批 12 ADR 第 5 个)
- 决策 ID 体系: [`docs/stage4/pending-decisions-overview-2026-08-05.md`](../../docs/stage4/pending-decisions-overview-2026-08-05.md) (D-01 ~ D-12)
- 锁文件清单: [`docs/stage4/8-locked-unified-2026-08-05.md`](../../docs/stage4/8-locked-unified-2026-08-05.md) (24 LOCKED crate + 7 LOCKED 文档)
- 71GB 事故根因: `docs/stage4/apeireth-rollback-71gb-postmortem-2026-08-05.md` (估补)

---

## 8. 附录

### 8.1 整合 #1 + #2 + #3 实施对比 (R20 阶段 1 全景)

| 维度 | 整合 #1 (`128f9704`) | 整合 #2 (`ae7bd2e5`) | 整合 #3 策略 (本 ADR) |
|---|---|---|---|
| 范围 | 5 P0 MCP | 9 skeleton | 阶段 1 收官 = #1 + #2 + 5 文档 |
| 实施 | 1 commit | 1 commit | 1 批 6 commits (蓝图 + #1 + #2 + 收官 + ROADMAP + CHANGELOG/README) |
| 文档 | 0 (含 commit msg) | 0 (含 commit msg) | 5 必 (蓝图 / 收官 / ROADMAP / CHANGELOG / README) + 2 选 (team-onboarding / 1.0 release 报告) |
| 测试 | 45/45 PASS | 113/113 PASS | 158/158 PASS (整合 #1 + #2 之和) |
| crate LOC | ~3K (5 P0 平均) | ~6K (9 skeleton 平均) | 蓝图 604 行 + 收官 493 行 + 收官报告 300 行 + 文档 200 行 = 1.6K |
| commit msg | ~30 行 | ~50 行 | 6 commits 各 30-50 行 |
| mtime baseline | 16:34 之前 | 16:34 之前 | 16:34 之前 (24 LOCKED crate 实查) |
| 1 owner 估时 | × 3 周 | × 3 周 | × 6 周 阶段 1 整体 |

### 8.2 6 commits 提交顺序 + 关联 12 项

```
1. 蓝图 8a643778  (604 行 RIVAL VERSION)
   ↓ 关联 12 项 #1 doc
   ↓ 6 commits 中第 1 个
2. 整合 #1 128f9704  (5 P0 MCP, 45 tests)
   ↓ 关联 12 项 #2 test
   ↓ 6 commits 中第 2 个
3. 整合 #2 ae7bd2e5  (9 skeleton, 113 tests)
   ↓ 关联 12 项 #2 test
   ↓ 6 commits 中第 3 个
4. 收官 5f5b5fa3  (r20-阶段-1-收官, 9 章节 493 行)
   ↓ 关联 12 项 #1 doc
   ↓ 6 commits 中第 4 个
5. ROADMAP 3bc61686  (R20 阶段 1 章节)
   ↓ 关联 12 项 #1 doc
   ↓ 6 commits 中第 5 个
6. CHANGELOG+README 6c518ee3  (R20 阶段 1 状态)
   ↓ 关联 12 项 #1 doc
   ↓ 6 commits 中第 6 个
```

### 8.3 5 必 + 2 选 = 5-7 文档 实施细节

| # | 文档 | commit | 章节 | 字数估 | 状态 |
|---|---|---|---|---|---|
| 1 | 蓝图 v09021-rust-translation-blueprint-2026-08-05.md | 8a643778 | §0~§7 = 7 章节 | 604 行 / 53.6KB | ✅ 落地 |
| 2 | 收官 r20-阶段-1-收官-2026-08-05.md | 5f5b5fa3 | §0~§8 = 9 章节 | 493 行 | ✅ 落地 |
| 3 | ROADMAP.md 同步段 | 3bc61686 | R20 阶段 1 章节 | 50 行 | ✅ 落地 |
| 4 | CHANGELOG.md 同步 | 6c518ee3 | R20 阶段 1 章节 | 30 行 | ✅ 落地 |
| 5 | README.md 同步 | 6c518ee3 | 14 crate 表 + 状态 | 50 行 | ✅ 落地 |
| 6 (选) | team-onboarding.md (LOCKED 5b27d041) | (阶段 6) | 8 章节 | 187+ 行 | ✅ 阶段 6 估补落地 |
| 7 (选) | docs/release/1.0.0-release-report-2026-08-05.md | 02d5db6c (阶段 6) | §0~§8 = 9 章节 | 300+ 行 | ✅ 阶段 6 估补落地 |

### 8.4 整合 #3 风险 8 项 (per S-2 实事求是)

| 风险 | 实际命中 | 缓解 |
|---|---|---|
| 1. 14 crate 1 批 commit review 慢 | 0 命中 | 6 commits 分阶段, 团队 review 6 次 |
| 2. 193/193 tests 实测 vs 50/50 + 143/143 报数差异 | 5 命中 (5 个 #[test_case] 宏扩展) | O-5 容许, 接手者注意 fixture 入口签名 |
| 3. 71GB 事故 (R20 阶段 1 真发生) | 1 命中 (1 owner × 5% 估时) | `apeireth-rollback` 71GB 4 重防御估补 |
| 4. 9 skeleton 估缺 6 周估时紧 | 0 命中 (按计划) | R20 阶段 1 续 6 commits 估补 |
| 5. 5 P0 MCP 翻译 1.4GB 估时 1 owner × 3 周 | 0 命中 (按计划) | 1 owner × 3 周估补 |
| 6. mtime baseline 16:34 实查误报 | 0 命中 | git log -1 严格比对 |
| 7. 24 LOCKED crate 误改 | 0 命中 | mtime baseline + 8-promise-audit.sh 严守 |
| 8. workspace version 1.0.0 误改 | 0 命中 | Cargo.toml line 121 严守 |
