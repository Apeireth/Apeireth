# ADR 0026: 整合 3 策略 — R20 阶段 1 实战模式

> **状态**: 🟢 Accepted (R20 阶段 1 主人 2026-08-05 拍板 "整合 #1+#2+#3", per `docs/team-onboarding.md` §2)
> **commit 锚**: `8a643778` (蓝图) + `128f9704` (整合 #1) + `ae7bd2e5` (整合 #2) + `5f5b5fa3` (收官) + `3bc61686` (ROADMAP) + `6c518ee3` (CHANGELOG+README)
> **最后更新**: 2026-08-05

---

## 1. 背景 (Context)

R20 阶段 1 多 sub-agent 并行 (per 主人 2026-08-05 "卡挺久了, 我重启了你" + "多派几个人一起干"), 产出 14+ new crate 入 workspace + 24 LOCKED 0 改 + 6 文档站。

**问题**:
- 14 sub-agent 各自 commit 1-3 crate + 若干文档, 散在 git worktree
- Mavis (本仓库 main agent) 整合时, **0 触碰 24 LOCKED** 是硬性 (per `team-onboarding.md` §1 "8 项不修改承诺")
- 主人 2026-08-05 拍板 "派成员干", 但派太多 = 整合时乱, 派太少 = 进度慢

**约束**:
- 24 LOCKED crate 0 改 (per `team-onboarding.md` §1)
- workspace version 1.0.0 0 改 (per 8 项不修改承诺)
- 7 LOCKED 文档 0 改 (per `team-onboarding.md` §1)
- 整合时 **先看 sub-agent 产出, 不要重写** (per `team-onboarding.md` §2)
- 整合后 `git diff main..HEAD` 实查 0 触碰 24 LOCKED (per `team-onboarding.md` §2)

---

## 2. 决策 (Decision)

**3 整合策略 (整合 #1 + #2 + #3) — 一次性大批量入 workspace, 0 触碰 LOCKED**

### 2.1 整合 #1 = 5 P0 MCP crate 一次性入 workspace

**commit 锚**: `128f9704` (+5,731 lines)

**5 P0 crate**:
- `apeireth-mcp-ssh` (SSH 远程执行, per 1.0 release 估缺 #12)
- `apeireth-mcp-winrm` (WinRM Windows 远程, per 1.0 release 估缺 #12)
- `apeireth-mcp-relay-image` (镜像转发, per 1.0 release 估缺 #12)
- `apeireth-workflow` (工作流引擎, per 1.0 release 估缺 #4)
- `apeireth-team-lead` (team-lead supervisor, per ADR 0011)

**整合方法**:
- 1 owner (Mavis) 整合 5 crate
- 1 commit `128f9704` 一次性 5 crate
- 0 触碰 24 LOCKED (整合前 `git diff main..HEAD` 实查 0)
- 整合后 `git diff main..HEAD` 复查 0

**估时**: 1 owner × 1 天 (per 5 P0 crate 估 1.5K LOC × 5 = 7.5K LOC, 估 8h)

### 2.2 整合 #2 = 9 skeleton crate 一次性入 workspace

**commit 锚**: `ae7bd2e5` (+21,011 lines)

**9 skeleton crate** (per 主人 2026-08-05 19:50 拍板"派成员干", 20:30 "最大效率"):
- 3 估缺核心 (v0.9.21 chunks 1:1 翻译, per RIVAL §2.2):
  - `apeireth-image-prompt` (v0.9.21 imageTools.ts 1:1)
  - `apeireth-rollback` (v0.9.21 rollback.js 1:1)
  - `apeireth-plugin` (v0.9.21 plugin 体系 1:1)
- 2 估缺工具 (R20 阶段 4 主体, per RIVAL §2.3):
  - `apeireth-repo-scan` (v0.9.21 repoScan.ts 1:1)
  - `apeireth-repo-analyzer` (v0.9.21 repoAnalyzer.ts 1:1)
- 2 估缺基础设施 (P0 安全, per RIVAL §2.4):
  - `apeireth-keyring` (OS keychain 集成)
  - `apeireth-machine-id` (hardware ID)
- 2 SDK stub (R20 阶段 3 必补, per RIVAL §2.5):
  - `apeireth-lark` (Lark SDK stub, R21 真接)
  - `apeireth-voice` (TTS/STT stub, R21 真接)

**整合方法**:
- 9 sub-agent 各干 1 crate
- Mavis 整合 1 commit `ae7bd2e5` 一次性 9 crate
- 0 触碰 24 LOCKED (per `git diff main..HEAD` 复查)
- 整合时 **先看 sub-agent 产出**, 不要重写 (per `team-onboarding.md` §2)

**估时**: 9 sub-agent × 估 1-3 天/crate + Mavis 整合 1 owner × 1 天

### 2.3 整合 #3 = 全 0 改验证

**commit 锚**: `5f5b5fa3` (收官) + `3bc61686` (ROADMAP) + `6c518ee3` (CHANGELOG+README LOCKED)

**0 改验证清单** (per `team-onboarding.md` §2 整合后实查):

| 类别 | 0 改项 | 验证方法 |
|---|---|---|
| **24 LOCKED crate** | 24 个 | `git diff main..HEAD -- crates/apeireth-{core,memory,asi,...}` 应为空 |
| **7 LOCKED 文档** | 7 个 | `git diff main..HEAD -- docs/{team-onboarding,stage4/08-NEXT-UPGRADE-DIRECTIONS,architecture-v3-aircraft-carrier,architecture-v4-living-intelligence,architecture-v4-1-living-intelligence-update,APEIRETH-CONVENTIONS,APEIRETH-VERSIONING,GLOSSARY}` 应为空 |
| **workspace version** | `Cargo.toml` line 151 = 1.0.0 | `git diff main..HEAD -- Cargo.toml | grep "version ="` 应空 |
| **CHANGELOG.md** | LOCKED commit `6c518ee3` | `git log` 应见 `6c518ee3` 是 R20 阶段 1 一次性 commit |
| **README.md** | LOCKED commit `6c518ee3` | 同上 |

**整合 #3 收官 commit**:
- `5f5b5fa3` (R20 阶段 1 收官) — 总结 5 P0 + 9 skeleton 入 workspace + 24 LOCKED 0 改
- `3bc61686` (ROADMAP 更新) — R20 阶段 1 → 阶段 2 接力
- `6c518ee3` (CHANGELOG + README LOCKED) — 主人拍 "CHANGELOG + README 锁住, 后续不动"

### 2.4 Mavis 角色 (per `team-onboarding.md` §2)

> **Mavis 角色**: team lead (协调 + 整合 + 决策), 不是 worker. 主人 (chuling) 才是 dev 主.

**Mavis 在整合 3 策略中的角色**:
- **派活前**: 写清楚任务 + 集成规范 + 不重复造轮子 (per `team-onboarding.md` §2)
- **整合时**: 先看 sub-agent 产出, 不要重写 (per `team-onboarding.md` §2)
- **整合后**: `git diff main..HEAD` 实查 0 触碰 24 LOCKED (per `team-onboarding.md` §2)
- **决策时**: 整合冲突 / 估时争议 / 优先级, 主人拍板 (per `team-onboarding.md` §2)

### 2.5 整合失败模式 (rejected)

| 失败模式 | 后果 | 主人拍板 |
|---|---|---|
| **逐 crate commit** (14 commit 逐个入) | git log 难读, 估时 × 3 | 拍板"一次性入" |
| **重写 sub-agent 产出** | 估时 × 2, 团队委屈 | 拍板"先看产出" |
| **改 24 LOCKED** | 违反 8 项不修改承诺 | 拍板"0 触碰" |
| **改 workspace version** | 违反 semver 严守 | 拍板"0 改" |
| **改 7 LOCKED 文档** | 违反 8 项不修改承诺 | 拍板"0 改" |
| **多 owner 整合** | 协调成本 × N | 拍板"Mavis 单 owner 整合" |

---

## 3. 后果 (Consequences)

### 3.1 正面

- ✅ **整合快**: 1 owner × 2 天 (整合 #1 + #2) 14 crate 入 workspace
- ✅ **0 触碰 LOCKED**: 整合 #3 验证 24 LOCKED crate + 7 LOCKED 文档 + workspace version 全 0 改
- ✅ **可读 git log**: 1 commit = 1 整合事件, 不是 14 个散 commit
- ✅ **团队不委屈**: 整合时 "先看 sub-agent 产出, 不要重写" (per `team-onboarding.md` §2)
- ✅ **Mavis 角色清晰**: team lead ≠ worker, 主人是 dev 主
- ✅ **复用模式**: R20 阶段 2-6 续用 3 整合策略 (估补 / 文档 / 1.0 release)

### 3.2 负面

- ⚠️ **1 commit 风险大**: 整合 #1 / #2 各 5-9 crate, 1 commit 出问题 = 全部回滚 (mitigation: 整合前 dry-run + sub-agent 各跑 `cargo check`)
- ⚠️ **整合 #3 验证靠 git diff**: 不能编译期守门, 人工 review (mitigation: `scripts/audit/8-promise-audit.sh` 自动审, per ADR 0027)
- ⚠️ **Mavis 压力大**: 1 owner 整合 14 crate, 估 8-16h (mitigation: 派 sub-agent 协助 `git diff` 验证)

### 3.3 风险

- 整合 #1 或 #2 触碰了 24 LOCKED 之一 (e.g. workspace version 被 sub-agent 改 1.0.0 → 1.0.1), 1.0 release 阻塞; mitigation: 整合前 `git diff main..HEAD` 必查
- 整合 #3 漏检某 LOCKED 文档 (e.g. `docs/stage4/08-NEXT-UPGRADE-DIRECTIONS.md`), 1.0 release 后发现已改; mitigation: `scripts/audit/8-promise-audit.sh` 7 LOCKED 文档全列, 自动审

---

## 4. 备选 (Alternatives Considered)

### A. 14 sub-agent 各 commit 1 crate, 不整合
- 优点: sub-agent 自治
- 否决: git log 散, 24 LOCKED 0 改难保, 不符合"8 项不修改承诺"

### B. 1 owner 干 14 crate (Mavis 单干)
- 优点: 简单
- 否决: 估时 × 3, 主人 2026-08-05 "派成员干" 拍板否决

### C. 14 sub-agent 派活 + 1 owner (Mavis) 整合, 1 commit 一次性入 (整合 #1 + #2)
- 优点: 估时短, 0 触碰 LOCKED
- 拍板: R20 阶段 1 主人拍

### D. 14 sub-agent 派活 + 1 owner 整合, 14 commit 逐个入
- 优点: 风险小
- 否决: 估时 × 3, git log 散, 主人拍板"一次性入"

### E. 14 sub-agent 派活 + 1 owner 整合, 1 commit 一次性入 + 整合 #3 0 改验证 (本决策)
- 优点: 估时短 + 0 触碰 LOCKED + 可验证
- 拍板: R20 阶段 1 主人拍

### F. 14 sub-agent 派活 + 1 owner 整合, 1 commit 一次性入 + 整合 #3 0 改验证 + Mavis team lead 角色明确
- 优点: 同 E + 角色清晰
- 拍板: R20 阶段 1 主人拍

---

## 5. 6 哲学锚穿透

- ✅ **S-1 走在前人经验上**: git diff 验证 + 8-promise-audit.sh 抄业界 CI 守门
- ✅ **S-2 实事求是**: 整合 #1 + #2 估时基于已 commit 代码, 不凭想象
- ✅ **O-2 用户看结果不看哲学**: 用户只看 1.0 release 装上能不能用, 不看整合策略
- ✅ **O-3 信息密度"高"**: §2.1 + §2.2 + §2.3 3 节说清 3 整合策略
- ✅ **O-4 干净状态 = 没有历史包袱**: 拒绝"逐 crate commit" (git log 散) / 拒绝"重写 sub-agent 产出"
- ✅ **O-5 6 哲学锚穿透**: 本节自检

---

## 6. 8 项不修改承诺

- ✅ **不假装已实现**: 3 整合策略基于 R20 阶段 1 实战 commit (8a643778 / 128f9704 / ae7bd2e5 / 5f5b5fa3 / 3bc61686 / 6c518ee3)
- ✅ **编译期 hardcode**: 24 LOCKED crate 路径 + 7 LOCKED 文档路径 编译期 `scripts/audit/8-promise-audit.sh` 守门
- ✅ **不改 LOCKED**: 整合 #3 验证 24 LOCKED crate + 7 LOCKED 文档 + workspace version 全 0 改
- ✅ **不改 workspace version**: v1.0.0 严守 (整合 #3 验证清单)
- ✅ **6 哲学锚穿透**: §5 自检
- ✅ **不依赖 NewAPI**: 自建整合策略, 0 依赖 NewAPI-style 整合
- ✅ **不重复造轮子**: 沿用 git worktree + git diff + CI 守门业界标准
- ✅ **诚实标缺**: §3.2 负面 + §3.3 风险诚实标 (1 commit 风险 / 整合 #3 验证靠 git diff)

---

## 7. 引用

- 团队入职: [`docs/team-onboarding.md`](../team-onboarding.md) §1 + §2 (8 项不修改承诺 + 整合 3 模式)
- 蓝图: `8a643778` (R20 阶段 1 收官蓝图)
- 整合 #1 commit: `128f9704` (+5,731 lines, 5 P0 MCP crate)
- 整合 #2 commit: `ae7bd2e5` (+21,011 lines, 9 skeleton crate)
- 整合 #3 commit: `5f5b5fa3` (收官) + `3bc61686` (ROADMAP) + `6c518ee3` (CHANGELOG+README LOCKED)
- 8 项守门: [`docs/adr/0027-8-promise-audit.md`](0027-8-promise-audit.md) (估补)
- 6 哲学锚: [`docs/adr/0021-6-philosophy-anchors.md`](0021-6-philosophy-anchors.md)
- 主人 2026-08-05 拍板: "卡挺久了, 我重启了你" + "多派几个人一起干" + "派成员干" (per self-stance log)
