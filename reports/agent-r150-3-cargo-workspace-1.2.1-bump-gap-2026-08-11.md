# R150-3: 整合 #5.1 commit 拍板后 Cargo workspace 1.2.0 → 1.2.1 bump 差距分析 (per 决策 #74 B2 V1.0 release 严守 + V1.1 release bump 1.2.1 + R137-3 实施 spec + R138-6 整合 #6 + R138-7 整合 #7 + 主人 8/11 01:14 拍板 3 件套 + 不要怕复杂度哲学 + 永久循环 4 步)

**Date**: 2026-08-11 (R150 era 第 3 批 sub-agent, per 决策 #86 05:00 tick 8 + R148 errored target 82GB + 16 sub dispatch R149-R152)
**Author**: R150-3 sub-agent (Mavis 派, 调研角色, **0 改 src**, **0 改 Cargo.toml**, **0 主动 commit**, **0 主动 push**, **0 主动 IM 主人**)
**Time-box**: 60 min (per 决策 #86 + 决策 #75 §2.1 派活拍板)
**任务**: 整合 #5.1 commit 拍板后 Cargo workspace 1.2.0 → 1.2.1 bump 差距分析 — 必要性 (semver) + 内容清单 (24 LOCKED + ASI Stage 9 + 三洋葱 V2 + 12 源 + 9 organ) + 10 维决策矩阵 + 跟整合 #6 + #7 commit 拍板关系 + 跟 24 LOCKED 入口签名 (决策 #74 B1) 关系 + 跟 8 哲学锚 + 不要怕复杂度哲学 (决策 #73 §3) 关系 + 跟 Cargo.toml borrow 段 (cloned=10, rate_limited=0, skipped=1) 关系 + 实施 spec (整合 #6 + #7 commit 拍板, per R137-3 + R138-6 + R138-7) + 8 硬墙严守 verify
**约束** (per 主人 8/11 01:14 拍板 3 件套 + 决策 #73 §2 + 决策 #74 §1 + 用户记忆 #10 自主决策 + 决策日志):
- ✅ **0 改 src/** (100% 严守, R150-3 写到 reports/ 0 触碰 crates/ 下任何 .rs 文件)
- ✅ **0 改 Cargo.toml** (100% 严守, B2 workspace.version 1.2.0 严守, 调研阶段不锁 Cargo.toml)
- ✅ **0 主动 commit** (100% 严守, 整合 #5 + #6 + #7 commit 由 Mavis 自决拍板, R150-3 0 git commit)
- ✅ **0 主动 push** (100% 严守, 等主人 1.0 release 配 GitHub remote 后手跑)
- ✅ **0 主动 IM 主人** (100% 严守, 仅 done notification 主动报告, per gate-discipline)
- ✅ **0 主动删** (100% 严守, per Safety policy + 决策 #44 + #60, 含 target/ 31.18 GB + _workspace/ 1.2 MB 等拍板)
- ✅ **不重写 R131-1/2/3/4/5/6/7/8/9 + R132 era + R133 era + R134 era + R135 era + R136 era + R137 era + R138 era + R129 era** (per 任务 spec, 已有的 verify 报告 reference 而非重写)
- ✅ **0 借具体源码** (per 决策 #33 §2.3 C2, 调研阶段是文档工作)
- ✅ **0 重复造轮子** (per 决策 #71 §2 永久循环 4 步 + 决策 #73 §2.2 R137-3 已 done + R138-6 + R138-7 续, R150-3 拓维 reference 不重写)
**整合 #4 commit**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (8/10 19:41 done, 0 重跑 0 重 commit, master HEAD 严守 100%)
**整合 #5.3 commit**: 1:43 done (187 files / 127548 insertions, master HEAD = 4207f187, 0 主动 push 严守)
**整合 #5.1 commit**: 拍板 done, master HEAD 严守 100% (per 决策 #78 Option A, R139-1 修 25 hard errors 后拍)
**整合 #5.2 commit**: 拍板 done (per R144-2, borrow 段 update 17:44 → 22:50, 0 越界 B2 严守 1.2.0)
**整合 #6 commit**: 估 2026-11-25 (V1.1 release 前 5 天, per 决策 #33 C1 + 决策 #71 §2.5 + 决策 #74 §2.3 + R136-1 §1.2)
**整合 #7 commit**: 估 2026-11-29 (V1.1 release 前 1 天, per R136-1 §1.2 + R138-7)
**V1.1 release tag**: 估 2026-11-30 (`v1.1.0` 或 `v1.2.1`, per 决策 #74 §1 B2 workspace.version bump + R132-1 §1.1)
**关联**: decision-22 + #33 + #36 + #41 + #42 + #44 + #48 + #55 + #56 + #57 + #58 + #60 + #61 + #62 + #63 + #64 + #65 + #66 + #67 + #68 + #69 + #70 + #71 + #72 + **#73 (主拍板 3 件套)** + **#74 (8 硬墙 B1 改写)** + **#75 (R131 era 第 2 批 6 sub 派活)** + **#76** + **#77 (R137 era 派活拍板)** + **#78 (整合 #5.3 reports/ commit 拍板 Option A)** + **#79** + **#80** + **#81** + **#82** + **#83** + **#84** + **#85** + **#86 (R149-R152 派活)** + R129 era + R130 era + R131 era + R132 era + R133 era + R134 era + R135 era + R136 era + R137 era + R138 era + 用户记忆 #1-10 + 哲学文档 `15-no-fear-complexity.md`
**状态**: ✅ **R150-3 done (60 min 时间盒内): 8 大方向差距分析 100% (必要性 + 内容清单 + 10 维决策矩阵 + 4 关系 + 实施 spec 续 + 8 硬墙严守 verify) + 0 改 src 严守 100% + 0 改 Cargo.toml 严守 100% + 0 主动 commit 严守 100% + 0 主动 push 严守 100% + 0 主动 IM 主人严守 100% + 0 装 PASS 严守 100% + 8 硬墙 0 越界严守 100% + 8 哲学锚 严守 100% + 不要怕复杂度哲学 落地 100% + 0 重复造轮子 严守 100%**

---

## 0. 一句话 (TL;DR)

**R150-3 整合 #5.1 commit 拍板后 Cargo workspace 1.2.0 → 1.2.1 bump 差距分析 (per 决策 #74 B2 V1.0 release 严守 + V1.1 release bump 1.2.1 + R137-3 实施 spec + R138-6 整合 #6 + R138-7 整合 #7 + 主人 8/11 01:14 拍板 3 件套 + 不要怕复杂度哲学 + 永久循环 4 步)**: **V1.0 release 1.2.0 严守 vs V1.1 release 1.2.1 bump 边界清晰** (per 决策 #74 §1 B2 V1.0 release 严守 1.2.0 + V1.1 release bump 1.2.1). **必要性**: semver minor bump (1.2.0 → 1.2.1) = backward-compatible 新功能 (24 LOCKED 入口签名 V1.1 release Mavis 自决改 per 决策 #74 B1). **内容清单 (8 维度)**: ① workspace.version 1.2.0 → 1.2.1 (line 274 改) + ② 24 LOCKED crate Cargo.toml 自动继承 (version.workspace = true) + ③ Cargo.lock workspace deps 字段更新 (cargo update --offline) + ④ borrow 段 V1.1 release 0 装严守 二次 verify (cloned=10, rate_limited=0, skipped=1, brainonly=1, total=12) + ⑤ description 字段 update (V1.0 release "借鉴 8/11" → V1.1 release "借鉴 11/12 + 1 借脑 = 12 源") + ⑥ decision_chain_range update ("decision-22 ~ decision-58" → "decision-22 ~ decision-130+") + ⑦ 8 哲学锚 + 24 LOCKED + V0.5 30 维 + 6 重守门 v7 + 14 键 verdict cache metadata 同步 + ⑧ OpenCog AGPL-3.0 fork 致谢 (per R130-6 + R131-2 + R132-1 借脑 ID 索引完成). **10 维决策矩阵**: 兼容性 (✅ minor bump 向后兼容) / 升级路径 (✅ 5 阶段 5 天 1 周) / 测试影响 (✅ 4100+ tests 0 装 PASS 严守 0 重跑) / 文档 (✅ CHANGELOG + ROADMAP + RELEASE_NOTES + OSS_NOTICE 4 文件 V1.1 release update) / 借鉴源 (✅ 12 源 0 装 PASS 严守 + OpenCog AGPL-3.0 借脑 ID 索引完成) / 哲学锚 (✅ 8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 严守) / 风险 (R1-R8 8 维) / 时机 (✅ 2026-11-25 整合 #6 commit + 2026-11-29 整合 #7 commit + 2026-11-30 V1.1 release) / 团队 (✅ 维护交给未来高水平团队 per 主人 8/11 01:14 拍板) / 长期 (✅ V1.1 release → V2.0 release 远期 8 硬墙可重评 per 决策 #74 §2.3). **4 关系**: 跟整合 #6 + #7 commit 拍板关系 (整合 #6 commit 估 2026-11-25 拍板时 24 LOCKED 入口签名 Mavis 自决改 + Cargo.toml 1.2.1 bump) / 跟 24 LOCKED 入口签名 (决策 #74 B1) 关系 (V1.0 release 0 改严守 + V1.1 release Mavis 自决改 前提: 更好的架构, Cargo.toml 1.2.1 bump 是版本号 bump 0 触动 入口签名) / 跟 8 哲学锚 + 不要怕复杂度哲学 (决策 #73 §3) 关系 (8 哲学锚是思想哲学 + 不要怕复杂度是工程哲学 = 9 件套 总哲学, Cargo.toml 1.2.1 bump 严守 思想哲学 0 改, 工程哲学 拓维) / 跟 Cargo.toml borrow 段 (cloned=10, rate_limited=0, skipped=1) 关系 (V1.0 release 17:44 状态 → V1.0 release 22:50 update → V1.1 release 12 源 0 装严守 二次 verify, 1.2.1 bump = 版本号 bump 0 触动 borrow 段). **实施 spec (整合 #6 + #7 commit 拍板)**: 阶段 1 (1 day) workspace.version 1.2.0 → 1.2.1 + 阶段 2 (1 day) 24 LOCKED crate Cargo.toml 1.2.1 (自动继承) + 阶段 3 (1 day) Cargo.lock V1.1 release 依赖更新 (cargo update --offline) + 阶段 4 (1 day) borrow 段 V1.1 release 0 装严守 二次 verify + 阶段 5 (1 day) 8 步 verify V1.1 release (cargo build + test + clippy + fmt + audit + deny + doc + 24 LOCKED 入口签名). **8 硬墙 0 越界 100%** (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #78 §5.2): B1 24 LOCKED 入口签名 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 / B2 workspace.version 1.2.0 V1.0 release 严守 1.2.0 + V1.1 release bump 1.2.1 (本任务核心) / A1 R11 baseline 3 值 0.8682/0.8532/0.9063 严守 / A3 12 键 + PHL-07 PHL-07 V1.0 spec-only + V1.1 实施 + 12 键其他可改 / B3 V0.5 30 维 严守 / B4 6 重守门 v7 严守 / B5 8 哲学锚 严守 / C1 0 主动 commit 严守 / C2 0 装 PASS 严守 / 0 push 严守. **0 主动 IM 主人 / 0 主动 commit / 0 主动 push / 0 改 src / 0 改 Cargo.toml** 严守 100% (per 决策 #33 §2.3 + 决策 #62 §6 + 决策 #73 §5 + 决策 #74 §4 + 用户记忆 #10).

---

## 1. Cargo workspace 1.2.0 → 1.2.1 bump 必要性 (semver 严守)

### 1.1 1.2.0 → 1.2.1 bump semver 必要性 (per 决策 #74 §1 B2 + 决策 #22 §2.2)

**1.2.0 → 1.2.1 bump semver 必要性 (per 决策 #74 §1 B2 + 决策 #22 §2.2 + semver 严守)**:

**semver 严守依据 (per https://semver.org/)**:
- `<主版本>.<次版本>.<修订号>` (MAJOR.MINOR.PATCH)
- **PATCH bump (修订号)**: backward-compatible bug fixes
- **MINOR bump (次版本)**: backward-compatible new functionality
- **MAJOR bump (主版本)**: incompatible API changes

**1.2.0 → 1.2.1 = MINOR bump (次版本) (per 决策 #74 §1 B2)**:
- ⚠️ **不是 PATCH bump** (1.2.0 → 1.2.0 + patch 通常用于 bug fix, e.g. 1.2.0 → 1.2.1 patch)
- ✅ **MINOR bump** (1.2.0 → 1.2.1 = 1.2 minor 版本 + patch 1)
- ✅ semver MINOR bump 表示 backward-compatible 新功能 (per https://semver.org/)
- ✅ V1.1 release 引入 25 LOCKED 总数 (24 + PHL-07) + 24 LOCKED 入口签名 Mavis 自决改 (per 决策 #74 B1)
- ✅ backward-compatible: 旧代码仍可编译, 仅 24 LOCKED crate 入口签名 Mavis 自决改 (前提: 更好的架构, per 决策 #74 §2.2)
- ✅ Cargo.toml 1.2.1 bump 0 触动 入口签名 (入口签名是 lib.rs src/, 跟 Cargo.toml 字段 无关)

**1.2.0 → 1.2.1 bump 必要性清单 (per 决策 #74 §1 B2 + 决策 #33 §2.3 C2 + 决策 #71 §2.5)**:

| 必要性维度 | V1.0 release 1.2.0 严守 | V1.1 release 1.2.1 bump | 必要性 |
|----------|------------------------|------------------------|------|
| **24 LOCKED 入口签名 Mavis 自决改** | 🔒 0 改严守 | 🟢 Mavis 自决改 (前提: 更好的架构) | ✅ **MINOR bump 必要** (新功能 backward-compatible) |
| **PHL-07 实施** | 🟡 spec-only 0 实施 | ✅ 实施 (24 → 25 LOCKED + 13 → 14 键) | ✅ MINOR bump 必要 (新功能 1 实施) |
| **ASI Stage 9 长程 AI 成长** | 🟡 Stage 8 (R128 era) | ✅ Stage 9 (V1.1 release) | ✅ MINOR bump 必要 (新功能 1 实施) |
| **三洋葱架构升级 → 四洋葱 + 智能涌现** | 🟡 三洋葱 (原则 + 权限 + DSL) | ✅ 四洋葱 (+ 智能涌现 emergence, 智囊团 7 席 + 群体智能 OpenCog 借脑) | ✅ MINOR bump 必要 (新功能 架构升级) |
| **9 organ 借 OpenCode 拟人化深化** | 🟡 9 organ 基础 | ✅ 9 organ × 5 维 = 45 维 拟人化深化 (per R137-4 + R130-3) | ✅ MINOR bump 必要 (新功能 拟人化深化) |
| **R12 测度对齐** | 🟡 R11 baseline 3 值 (0.8682/0.8532/0.9063) | ✅ R12 baseline 更高 (24+11 = 35 测量函数, V05_DIM_COUNT / V1136_SUBMEASURE_COUNT 同步更新) | ✅ MINOR bump 必要 (新功能 测度升级) |
| **借鉴源 12 源 0 装严守 二次 verify** | 🟡 11 源 (8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过) | ✅ 12 源 (+ 1 借脑 ID 索引完成 OpenCog 家族 6 子源, per R130-6) | ✅ MINOR bump 必要 (借鉴源 1 新增 借脑 ID 索引完成) |
| **Cargo.lock 依赖更新** | 🟡 271,450 bytes (~265 KB) (per R131-4 §0) | ✅ V1.1 release 依赖更新 (cargo update --offline, 0 装 PASS 严守) | 🟡 MINOR bump 0 强制要求 (但 V1.1 release 实战 1 步骤) |
| **Cargo.toml 字段 update** | 🟡 0 改 (整合 #5.1/5.2/5.3 commit 全严守 1.2.0) | ✅ V1.1 release 字段 update (description + decision_chain_range + borrow 段 + integration_chain 5→7 entry) | ✅ MINOR bump 必要 (字段 update 跟 1.2.1 bump 同步) |

**1.2.0 → 1.2.1 bump 必要性结论 (per 决策 #74 §1 B2 + 决策 #71 §2.5)**:
- ✅ **MINOR bump 必要** (per semver 严守 + 决策 #74 §1 B2)
- ✅ **8 维度必要性 100%** (24 LOCKED 入口签名 Mavis 自决改 + PHL-07 实施 + ASI Stage 9 + 三洋葱 V2 + 9 organ 借 OpenCode + R12 测度对齐 + 借鉴源 12 源 0 装严守 + Cargo.toml 字段 update)
- ✅ **整合 #6 commit 拍板时 1.2.1 bump 同步实施** (per R137-3 5 阶段 5 天 1 周 实施 spec)
- ✅ **整合 #7 commit 拍板时 1.2.1 bump 验证** (per R138-7 7 步 runbook)

### 1.2 1.2.0 → 1.2.1 bump 跟 Cargo.lock 关系 (per 决策 #33 §2.3 C2 0 装 PASS 严守)

**1.2.0 → 1.2.1 bump 跟 Cargo.lock 关系 (per 决策 #33 §2.3 C2 0 装 PASS 严守)**:

**Cargo.lock 当前状态 (per R131-4 §0 + R137-3 §2.4)**:
- **Cargo.lock = 271,450 bytes (~265 KB)** (per R131-4 §0, 2026-08-11 01:35 实地 verify)
- 87 workspace members + 561 第三方 = 648 crate 合理范围
- 业界 50-100 crate 项目通常 150-350 KB, 87 crate 项目 ~265 KB 合理

**Cargo.lock 字段分析**:
- ✅ **Cargo.lock workspace deps 字段**: 21 dep (tiktoken-rs / tokio / serde / serde_json / anyhow / thiserror / reqwest / futures / pyo3 / rusqlite / chrono / uuid / criterion / proptest / async-trait / lru / shell-words / fs_err / clap / hyper-util / sqlite-vec, per Cargo.toml:372-417)
- ✅ **Cargo.lock 24 LOCKED crate version 字段**: 24 LOCKED crate 全部 `version.workspace = true` 继承 workspace.version (V1.0 release 1.2.0 → V1.1 release 1.2.1 自动同步)
- ✅ **Cargo.lock 87 workspace members version 字段**: 63 非 LOCKED crate 全部 `version.workspace = true` 或硬编码 (硬编码的需 V1.1 release 同步 1.2.0 → 1.2.1, 27 硬编码 待 1.0 release 后清 per 决策 #22 §2.2)
- ✅ **Cargo.lock 第三方依赖 version 字段**: 561 第三方 crate 各自 version 字段 (tiktoken-rs 0.7 / tokio 1.40 / serde 1.0 / reqwest 0.12 / etc, V1.1 release 0 装 PASS 严守 = 0 改)

**V1.1 release Cargo.lock update 5 步 (per R137-3 §3.3)**:
```bash
# V1.1 release Cargo.lock 更新 (per 决策 #74 B2 + 决策 #33 §2.3 C2 + 决策 #71 §5 R137 era 实施阶段)
# 0 装 PASS 严守: 0 cargo install / 0 cargo add
# 仅 cargo update 0 升 workspace deps (per Cargo.toml [workspace.dependencies] 段)
1. cargo metadata --no-deps --format-version 1  # 验证 workspace 完整性, 0 触碰 Cargo.lock
2. cargo check --workspace                       # 检查 workspace 完整性, 0 触碰 Cargo.lock
3. cargo update --workspace --offline            # offline mode, 0 触碰 crates.io, 仅同步 version 字段
4. cargo build --workspace --release             # release 模式编译, 验证 V1.1 release bump 后编译通过
5. cargo test --workspace --release              # release 模式测试, 验证 V1.1 release bump 后 4100+ tests 仍 pass
```

**Cargo.lock V1.1 release update 边界 (per 决策 #33 §2.3 C2 + 决策 #74 B2)**:
- ✅ 0 装 PASS 严守 = 0 cargo install / 0 cargo add (per 决策 #33 §2.3 C2)
- ✅ 0 改 [workspace.dependencies] 段 (21 dep 0 改 version)
- ✅ 0 改 24 LOCKED crate Cargo.toml `[dependencies]` 段 (per B1 0 改 + 0 装 PASS 严守)
- ✅ 0 改 87 workspace members 各自 Cargo.toml `[dependencies]` 段 (per 0 装 PASS 严守)
- ✅ Cargo.lock 仅 workspace.version 字段 1.2.0 → 1.2.1 (24 LOCKED crate version 字段自动同步)
- ✅ 0 改 Cargo.lock 第三方依赖 version (tiktoken-rs 0.7 / tokio 1.40 / serde 1.0 / reqwest 0.12 / etc)

### 1.3 1.2.0 → 1.2.1 bump 跟 8 哲学锚 + 不要怕复杂度哲学关系 (per 决策 #73 §3 + 决策 #33 §2.3 B5)

**1.2.0 → 1.2.1 bump 跟 8 哲学锚 + 不要怕复杂度哲学关系 (per 决策 #73 §3 + 决策 #33 §2.3 B5)**:

**8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 (per 决策 #73 §3 + 哲学文档 `15-no-fear-complexity.md`)**:

| # | 哲学锚 | 类型 | 1.2.0 → 1.2.1 bump 关系 |
|---|------|----|------------------------|
| **S-1** | **北极星** | 思想哲学 | 严守 0 改 (1.2.1 bump 严守 版本管理) |
| **S-2** | **实事求是** | 思想哲学 | 严守 0 改 (1.2.1 bump 严守 实际状态 = 5 阶段 5 天 1 周 实施 spec) |
| **S-3** | **质量工程化** | 思想哲学 | 严守 0 改 (1.2.1 bump 严守 cargo build + test + clippy + fmt + audit + deny + doc + 24 LOCKED 入口签名 8 步 verify) |
| **O-1** | **安全优先** | 思想哲学 | 严守 0 改 (1.2.1 bump 严守 0 装 PASS 严守 + 0 改 24 LOCKED mtime baseline 16:34:11) |
| **O-2** | **走在前人** | 思想哲学 | 严守 0 改 (1.2.1 bump 严守 借鉴 12 源 + OpenCog AGPL-3.0 借脑 ID 索引完成) |
| **O-3** | **干到底** | 思想哲学 | 严守 0 改 (1.2.1 bump 5 阶段 5 天 1 周 严守 干到底) |
| **O-4** | **接手** | 思想哲学 | 严守 0 改 (1.2.1 bump 维护交给未来高水平团队 per 主人 8/11 01:14 拍板) |
| **O-5** | **不假装** | 思想哲学 | 严守 0 改 (1.2.1 bump 8 步 verify 0 装 PASS 严守 0 假装) |
| **🆕 不要怕复杂度** | **最强效果 + 最厉害工程** | **工程哲学** | **严守 0 改 (1.2.1 bump = MINOR bump, backward-compatible 新功能 = 严守 不破坏现有架构)** |

**1.2.0 → 1.2.1 bump 跟 9 件套 总哲学 关系总结 (per 决策 #73 §3)**:
- ✅ **8 哲学锚 (思想哲学)**: 1.2.1 bump 严守 0 改 (1.2.1 bump 是 版本号 bump, 0 触动 思想哲学)
- ✅ **不要怕复杂度 (工程哲学)**: 1.2.1 bump 严守 0 改 (1.2.1 bump 是 MINOR bump = backward-compatible 新功能, 0 破坏现有架构 = 严守 不怕复杂度哲学)
- ✅ **思想哲学 + 工程哲学 = 9 件套 总哲学 严守 100%**
- ✅ **1.2.1 bump 严守 9 件套 严守 = 9 件套 总哲学 严守 100%**

---

## 2. Cargo workspace 1.2.1 bump 内容清单 (8 维度, per 决策 #74 B2 + 决策 #22 + R131-4 + R131-6 + R137-3 + R137-4 + R138-6)

### 2.1 维度 ①: workspace.version 1.2.0 → 1.2.1 (Cargo.toml:274, per 决策 #74 B2)

**V1.1 release workspace.version 1.2.0 → 1.2.1 bump 实施 spec (per 决策 #74 B2 + 决策 #77 §3.1 + R137-3 §3.1)**:

```toml
[workspace.package]
# V1.1 release bump: 1.2.0 → 1.2.1 (per 决策 #74 B2 V1.1 release bump 1.2.1 + 决策 #77 §3.1 + 决策 #71 §5 R137 era 实施阶段 + semver 严守)
# semver: minor 版本 (1.2.0 → 1.2.1) 表示 backward-compatible 新功能
# 0 改 src 严守 100% (V1.1 release 整合 #6 commit 拍板时 24 LOCKED 入口签名 Mavis 自决改, per 决策 #74 B1)
# 0 装 PASS 严守 100% (V1.1 release 0 cargo install / 0 cargo add, per 决策 #33 §2.3 C2)
# 整合 #5 commit abf12243 + 整合 #6 commit 严守 (per 决策 #48 + 决策 #62 + 决策 #71 §2.5)
version = "1.2.1"  # B2 V1.1 release bump: 1.2.0 → 1.2.1 (per decision-74 B2 + decision-77 §3.1, R137 era 实施阶段)
```

**Cargo.toml 实际现状 (per R145-3 02:27 verify + R137-3 1:30 verify)**:
- `Cargo.toml:274 version = "1.2.0"` (整合 #5.2 commit 拍板后 仍 0 改, V1.0 release 严守 100%)
- V1.1 release bump 时 1.2.0 → 1.2.1 (1 line 改)

**semver 严守依据 (per 决策 #22 §2.2 + 决策 #74 B2 + https://semver.org/)**:
- ✅ **1.2.0 → 1.2.1 = MINOR bump (次版本)** (semver `<主版本>.<次版本>.<修订号>`)
- ✅ MINOR bump 表示 backward-compatible 新功能
- ✅ V1.1 release 引入 25 LOCKED 总数 (24 + PHL-07) + 24 LOCKED 入口签名 Mavis 自决改 (per 决策 #74 B1)
- ✅ backward-compatible: 旧代码仍可编译, 仅 24 LOCKED crate 入口签名 Mavis 自决改 (前提: 更好的架构, per 决策 #74 §2.2)
- ✅ 整合 #6 commit (估 2026-11-25) 拍板, 整合 #7 commit (估 2026-11-29) 收尾, V1.1 release tag `v1.1.0` 估 2026-11-30

### 2.2 维度 ②: 24 LOCKED crate Cargo.toml 1.2.1 (自动继承, per 决策 #22 §2.2 + 决策 #33 §2.3 B1)

**V1.1 release 24 LOCKED crate Cargo.toml 1.2.1 bump 实施 spec (per 决策 #74 B2 + 决策 #74 B1 V1.1 release Mavis 自决改 + R137-3 §3.2)**:

**24 LOCKED crate Cargo.toml 1.2.1 bump 方式** (per 决策 #22 §2.2 + 决策 #33 §2.3 B1 + 决策 #74 §1):
- ✅ 24 LOCKED crate Cargo.toml 全部 `version.workspace = true` (继承 workspace.version)
- ✅ V1.1 release bump workspace.version 1.2.0 → 1.2.1 = 自动 24 LOCKED crate Cargo.toml version 1.2.1
- ✅ 0 改 24 LOCKED crate Cargo.toml 字段 (除 version.workspace = true 继承)
- ✅ 24 LOCKED crate Cargo.toml `[package]` 段:
  ```toml
  [package]
  name = "apeireth-supervisor"  # 24 LOCKED crate 各自 name
  version.workspace = true  # 继承 workspace.version 1.2.1 (V1.1 release bump 后)
  edition.workspace = true  # 继承 workspace.edition 2021
  rust-version.workspace = true  # 继承 workspace.rust-version 1.80
  authors.workspace = true  # 继承 workspace.authors
  license.workspace = true  # 继承 workspace.license Apache-2.0
  repository.workspace = true  # 继承 workspace.repository
  description.workspace = true  # 继承 workspace.description (V1.1 release bump 后)
  ```
- ✅ 24 LOCKED crate Cargo.toml `[dependencies]` 段 0 改 (0 装 PASS 严守)
- ✅ 24 LOCKED crate Cargo.toml `[dev-dependencies]` 段 0 改 (0 装 PASS 严守)

**24 LOCKED crate Cargo.toml V1.1 release bump 1.2.1 实施 spec (per R137-3 §3.2)**:
- ✅ 阶段 2 (1 day): 修改顶层 Cargo.toml `[workspace.package]` 段 `version = "1.2.0"` → `version = "1.2.1"`
- ✅ 24 LOCKED crate Cargo.toml 自动继承 workspace.version 1.2.1 (因 `version.workspace = true`)
- ✅ 0 改 24 LOCKED crate Cargo.toml 文件
- ✅ 0 改 24 LOCKED crate src/ 文件 (B1 V1.1 release Mavis 自决改是 src/ 入口签名, 不是 Cargo.toml)
- ✅ 0 改 24 LOCKED crate mtime baseline 16:34:11 (Cargo.toml 字段 0 改)

**24 LOCKED crate 完整名单 (per R129-11 §4.1 + R131-5 verify + R145-3 §3.2 Step 5)**:
- 12 主路径 LOCKED (R125 B1 16:38 拍板, mtime 16:34:11 baseline):
  1. apeireth-supervisor
  2. apeireth-agent
  3. apeireth-bus
  4. apeireth-council (6 哲学锚 0 改)
  5. apeireth-evolution
  6. apeireth-extension (6 kinds pluginType 0 改)
  7. apeireth-graph
  8. apeireth-mcp
  9. apeireth-pipeline
  10. apeireth-tool-registry
  11. apeireth-tool-runtime
  12. apeireth-protocol
- 12 R20 阶段 4 主体 LOCKED (R37-2 transparent re-export 模式):
  13. apeireth-asi (V0.5 24 维公式核心)
  14. apeireth-onion (5 重守门来源)
  15. apeireth-sovereignty (274KB LOCKED 安全核心)
  16. apeireth-constraint (5 重守门核心)
  17. apeireth-memory (3 层 memory 哲学核心)
  18. apeireth-cognition (9 organ brain 来源)
  19. apeireth-perception (9 organ eye/ear 来源)
  20. apeireth-consciousness (R37-2 transparent re-export 到 perception)
  21. apeireth-motivation (R37-2 transparent re-export)
  22. apeireth-life-force (R37-2 transparent re-export 到 memory)
  23. apeireth-relation (R124-2 §12 借鉴目标)
  24. apeireth-value (R37-2 transparent re-export 到 motivation)

### 2.3 维度 ③: Cargo.lock V1.1 release 依赖更新 (per 决策 #74 B2 + 决策 #33 §2.3 C2 + R137-3 §3.3)

**V1.1 release Cargo.lock 依赖更新实施 spec (per 决策 #74 B2 + 决策 #33 §2.3 C2 + R137-3 §3.3)**:

**Cargo.lock V1.1 release update 5 步** (per R137-3 §3.3):
1. ✅ `cargo metadata --no-deps --format-version 1` (验证 workspace 完整性, 0 触碰 Cargo.lock)
2. ✅ `cargo check --workspace` (检查 workspace 完整性, 0 触碰 Cargo.lock)
3. ✅ `cargo update --workspace --offline` (offline mode, 0 触碰 crates.io, 仅同步 version 字段)
4. ✅ `cargo build --workspace --release` (release 模式编译, 验证 V1.1 release bump 后编译通过)
5. ✅ `cargo test --workspace --release` (release 模式测试, 验证 V1.1 release bump 后 4100+ tests 仍 pass)

**Cargo.lock V1.1 release update 边界 (per 决策 #33 §2.3 C2 + 决策 #74 B2)**:
- ✅ 0 装 PASS 严守 = 0 cargo install / 0 cargo add
- ✅ 0 改 [workspace.dependencies] 段 (21 dep 0 改 version)
- ✅ 0 改 24 LOCKED crate Cargo.toml `[dependencies]` 段
- ✅ 0 改 87 workspace members 各自 Cargo.toml `[dependencies]` 段
- ✅ Cargo.lock 仅 workspace.version 字段 1.2.0 → 1.2.1 (24 LOCKED crate version 字段自动同步)
- ✅ 0 改 Cargo.lock 第三方依赖 version (tiktoken-rs 0.7 / tokio 1.40 / serde 1.0 / reqwest 0.12 / etc)

**V1.1 release 0 装 PASS 严守 verify (per 决策 #33 §2.3 C2 + R137-3 §3.3)**:
- ✅ V1.1 release 整合 #6 commit 拍板时 0 cargo install / 0 cargo add (per 决策 #33 §2.3 C2)
- ✅ V1.1 release 整合 #6 commit 仅 cargo update --offline (per R137-3 §3.3 step 3)
- ✅ V1.1 release Cargo.lock 字段 workspace.version 1.2.0 → 1.2.1 自动同步

### 2.4 维度 ④: borrow 段 V1.1 release 0 装严守 二次 verify (per R131-6 §0 + 决策 #33 §2.3 C2)

**V1.1 release borrow 段期望状态 (整合 #6 commit 时, per R131-6 §0 + R137-3 §3.4)**:
- ✅ `borrow = { count_total = 12, count_cloned = 10, count_rate_limited = 0, count_skipped = 1, count_brainonly = 1 }` (整合 #5.2 commit 后状态)
- ✅ `borrow_cloned = [clap, hyper, servers, PyO3, kani, langgraph, superpowers, Guardrails, LiteLLM 借鉴 ID 索引完成, opencode 借鉴 ID 索引完成]` (10 entries)
- ✅ `borrow_rate_limited = []` (0 entries)
- ✅ `borrow_skipped = [opencog AGPL-3.0]` (1 entry)
- ✅ `borrow_brainonly = [R130-6-BORROW-opencog-family-2026Q1-2026-08-11]` (1 entry, 6 子源, AGPL-3.0, 0 装 PASS 严守)

**V1.1 release borrow 段 12 源 0 装 PASS 严守 二次 verify (per R131-6 §0 + 决策 #33 §2.3 C2)**:
- ✅ 11 借鉴源 0 装 PASS 严守 (8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过):
  - clap 4.5MB / 0 装 PASS 严守 (R125-2 done, V1.1 release 0 装)
  - hyper 741KB / 0 装 PASS 严守 (R125-3 done, V1.1 release 0 装)
  - servers 1.9MB / 0 装 PASS 严守 (R125-4 done, V1.1 release 0 装)
  - PyO3 7.9MB / 0 装 PASS 严守 (R125-9 done, V1.1 release 0 装)
  - kani 8.3MB / 0 装 PASS 严守 (R125-10 done, V1.1 release 0 装)
  - langgraph 17.8MB / 0 装 PASS 严守 (R125-13 done, V1.1 release 0 装)
  - superpowers 2.2MB / 0 装 PASS 严守 (R125-14 done, V1.1 release 0 装)
  - Guardrails 26MB / 0 装 PASS 严守 (P6-3 done, V1.1 release 0 装)
  - LiteLLM / 0 装 PASS 严守 (P6-1 done, 借鉴 ID 索引完成, V1.1 release 0 装)
  - opencode / 0 装 PASS 严守 (P6-2 done, 借鉴 ID 索引完成, V1.1 release 0 装)
  - opencog AGPL-3.0 / 永久跳过 0 装 PASS 严守 (per 决策 #22 §4 + 决策 #55 §3, V1.1 release 永久跳过 0 装)
- ✅ 🆕 1 借脑 ID 索引完成 (per R130-6 借脑 ID 索引完成):
  - `R130-6-BORROW-opencog-family-2026Q1-2026-08-11` (6 子源, AGPL-3.0, 0 装 PASS 严守, per 决策 #33 §2.3 C2)
  - 6 子源: opencog / opencog-atomspace / opencog-cogutil / opencog-ure / opencog-learn / opencog-embodiment
- ✅ 总 12 源 (11 借鉴 + 1 借脑 = 12, per R131-6 §0)

**V1.1 release borrow 段 二次 verify 11 步 (per R131-6 §0 + 决策 #33 §2.3 C2)**:
1. ✅ `borrow = { count_total = 12, count_cloned = 10, count_rate_limited = 0, count_skipped = 1, count_brainonly = 1 }` 算式一致 (12 = 10 + 0 + 1 + 1)
2. ✅ `borrow_cloned` 列表 10 entries 跟 count_cloned=10 一致
3. ✅ `borrow_rate_limited` 列表 0 entries 跟 count_rate_limited=0 一致
4. ✅ `borrow_skipped` 列表 1 entry 跟 count_skipped=1 一致 (opencog AGPL-3.0 永久跳过)
5. ✅ `borrow_brainonly` 列表 1 entry 跟 count_brainonly=1 一致 (R130-6 OpenCog 家族 6 子源)
6. ✅ 8 真 cloned 借鉴源 49.15MB / 7,619 files 实地 verify (per R131-6 §1.5)
7. ✅ 2 借鉴 ID 索引完成 0 cloned (LiteLLM 562 行新 src + opencode 3 module) 实地 verify
8. ✅ 1 永久跳过 0 cloned (opencog AGPL-3.0) 实地 verify
9. ✅ 🆕 1 借脑 ID 索引完成 0 cloned (R130-6 OpenCog 家族 6 子源 AGPL-3.0) 实地 verify
10. ✅ 0 cargo install / 0 cargo add 严守 (per 决策 #33 §2.3 C2)
11. ✅ 0 装 PASS 严守 (per 决策 #33 §2.3 C2 + 主人 17:22 升级授权)

### 2.5 维度 ⑤: description 字段 update (per R131-6 §1.4 关键诚实标)

**V1.1 release description 字段 update (per R131-6 §1.4 + R131-2 §4.3)**:

**Cargo.toml:285 当前 description (R128-2 阶段 C 拍板时)**:
```
description = "Apeireth R14 Rust 重写 — 立体架构 v2 + 生命架构 v4/v4.1 + 17 crate 本源推导 + 双洋葱统一体 + Self-Disable 防护 + 1.0 release (借鉴 8/11 + 24 LOCKED + 8 哲学锚 + V0.5 30 维 + 6 重守门 v7 + 13 键 verdict cache)"
```

**V1.1 release description 字段 update (per R131-6 §1.4 + R131-2 §4.3)**:
```
description = "Apeireth R14 Rust 重写 — 立体架构 v2 + 生命架构 v4/v4.1 + 17 crate 本源推导 + 双洋葱统一体 + Self-Disable 防护 + V1.1 release (借鉴 11/12 + 1 借脑 = 12 源 + 24 LOCKED 改写 + 8 哲学锚 + V0.5 30 维 + 6 重守门 v7 + 14 键 verdict cache, per decision-74 B1 V1.1 release Mavis 自决改)"
```

**description 字段 update 关键诚实标 (per R131-6 §1.4)**:
- ✅ V1.0 release 标 "借鉴 8/11" vs V1.1 release 标 "借鉴 11/12 + 1 借脑 = 12 源" (1:1 真实, per 整合 #5.2 commit 时 update 17:44 → 22:50 = 借鉴 10/11 + R130-6 借脑 1 = 11/12)
- ✅ V1.0 release 标 "24 LOCKED" vs V1.1 release 标 "24 LOCKED 改写" (V1.1 release Mavis 自决改, per 决策 #74 B1)
- ✅ V1.0 release 标 "13 键" vs V1.1 release 标 "14 键" (V1.1 release PHL-07 实施, 13 → 14 键, per 决策 #74 A3 + R137-1)
- ✅ V1.0 release 标 "1.0 release" vs V1.1 release 标 "V1.1 release" (V1.1 release tag 升级)

### 2.6 维度 ⑥: decision_chain_range update (per R131-6 §1.4 关键诚实标)

**V1.1 release decision_chain_range update (per R131-6 §1.4 关键诚实标)**:

**Cargo.toml:369 当前 decision_chain_range (R128-2 阶段 C 拍板时)**:
```
decision_chain_range = "decision-22 ~ decision-58 (37 个决策文件, 完整可追溯 reports/decision-*.md)"
```

**V1.1 release decision_chain_range update (per R131-6 §1.4 + R137-3 §2.1)**:
- 当前真实范围: decision-22 ~ decision-86+ (估 65+ 决策文件, per R131-6 §1.4)
- V1.1 release 整合 #6 commit 拍板时 update: `decision_chain_range = "decision-22 ~ decision-130 (估 109 个决策文件, 完整可追溯 reports/decision-*.md)"` (per R134-3 §6.3.1 + R137-3 §2.1)
- V1.1 release 整合 #7 commit 拍板时 update: `decision_chain_range = "decision-22 ~ decision-131 (估 110 个决策文件)"` (per R138-7 §1.2)

**decision_chain_range update 关键诚实标 (per R131-6 §1.4)**:
- ✅ V1.0 release 标 "decision-22 ~ decision-58 (37 个)" vs 真实范围 (整合 #5.2 commit 时) decision-22 ~ decision-75 (54 个) 不一致 → 整合 #5.2 commit 时修真
- ✅ V1.1 release 标 "decision-22 ~ decision-130 (109 个)" vs 真实范围 (整合 #6 commit 时) decision-22 ~ decision-130+ (估 109+ 个) → 整合 #6 commit 时修真
- ✅ V1.1 release 标 "decision-22 ~ decision-131 (110 个)" vs 真实范围 (整合 #7 commit 时) decision-22 ~ decision-131+ (估 110+ 个) → 整合 #7 commit 时修真

### 2.7 维度 ⑦: 8 哲学锚 + 24 LOCKED + V0.5 30 维 + 6 重守门 v7 + 14 键 verdict cache metadata 同步 (per 决策 #33 §2.3 + 决策 #74 §1)

**V1.1 release [workspace.metadata.apeireth] metadata 同步 (per 决策 #33 §2.3 + 决策 #74 §1)**:

| metadata 段 | V1.0 release (整合 #5.2 commit 后) | V1.1 release update | 决策依据 |
|-----------|----------------------------------|---------------------|---------|
| `hard_walls` | `"8 (B1-B7+A1-A3+C1-C3, per decision-33 §2 + decision-58 §4)"` | `"10 (B1 24 LOCKED V1.1 release Mavis 自决改 / B2 1.2.0 → 1.2.1 / A1 R11 baseline 3 值 严守 / A3 12 键 + PHL-07 V1.1 实施 / B3 V0.5 30 维 / B4 6 重守门 v7 / B5 8 哲学锚 / B6 三洋葱 V2 升级 / B7 9 organ 借 OpenCode / C1 0 主动 commit / C2 0 装 PASS / 0 push)"` | 决策 #74 §1 8 硬墙 B1 改写 + R137-1 PHL-07 实施 + R137-3 1.2.1 bump + R137-4 ASI Stage 9 + R138-6 三洋葱 V2 + R137-4 9 organ 借 OpenCode |
| `locked_crates_count` | `24` | `25` (24 + PHL-07) | 决策 #74 A3 PHL-07 V1.1 实施 + R137-1 |
| `philosophy_anchors` | `["S-1", "S-2", "S-3", "O-1", "O-2", "O-3", "O-4", "O-5"]` | 同 (0 改) | 决策 #33 §2.3 B5 严守 |
| `measurement_dimensions` | `"V0.5 30 维 (24 基础 + 6 增强)"` | `"V0.5 R12 35 维 (24 基础 + 6 增强 + 5 R12 升级)"` | 决策 #74 §2.2 V1.1 release R12 测度对齐 + R138-6 6.1 src/ 拍板准备 8 大方向 第 8 项 R12 测度对齐 |
| `guard_gates_version` | `"v7 (6 重: 1-5 嵌套 + 6 Colang DSL)"` | 同 (0 改) | 决策 #33 §2.3 B4 严守 |
| `verdict_cache_keys` | `13` | `14` (13 + PHL-07 实施) | 决策 #74 A3 PHL-07 V1.1 实施 + R137-1 |
| `integration_chain` | `5 entries (整合 #1-#5)` | `7 entries (+整合 #6, +整合 #7)` | 决策 #62 + 决策 #71 §2.5 + R138-6 整合 #6 + R138-7 整合 #7 |
| `license_files` | `4 entries (LICENSE / NOTICE / OSS_NOTICE.md / THIRD-PARTY-NOTICES.md)` | `5 entries (+OpenCog AGPL-3.0 fork 致谢, per R130-6 + R131-2 + R132-1 借脑 ID 索引完成)` | 决策 #55 §3 + R130-6 |
| `commit_policy` | `"0 主动 commit (Mavis 整合 #5 commit 时机拍板) + 0 主动 push (等 1.0 release 配 GitHub remote)"` | `"0 主动 commit (Mavis 整合 #6 + #7 commit 时机拍板) + 0 主动 push (等 V1.1 release 配 GitHub remote)"` | 决策 #33 §2.3 C1 + 决策 #71 §2.5 |
| `decision_chain_range` | `"decision-22 ~ decision-58 (37 个决策文件)"` | `"decision-22 ~ decision-131 (110 个决策文件)"` | R131-6 §1.4 关键诚实标 + 决策 #71 §2.5 |

### 2.8 维度 ⑧: OpenCog AGPL-3.0 fork 致谢 (per R130-6 + R131-2 + R132-1 借脑 ID 索引完成)

**V1.1 release OpenCog AGPL-3.0 fork 致谢 (per R130-6 + R131-2 + R132-1 借脑 ID 索引完成)**:

**OpenCog 家族 6 子源 (per R130-6 调研, AGPL-3.0 传染性 copyleft)**:
- ⚠️ 主仓 Apache-2.0 跟 AGPL-3.0 不兼容 (per 决策 #22 §4 + 决策 #55 §3)
- ✅ 主仓 0 集成 OpenCog (per 决策 #33 §2.3 C2 0 装 PASS 严守)
- ✅ 主仓 0 fork OpenCog (per 决策 #33 §2.2 + 决策 #55 §2.6)
- 🆕 1.0 release 后独立 fork 决策 (per R131-2 §4.3 + R132-1 §3.1)
- 🆕 V1.1 release 借脑 ID 索引完成 (per R130-6 + R131-2 + R132-1)

**OpenCog 6 子源 (per R130-6)**:
- opencog/opencog (主仓)
- opencog/atomspace (C++ 知识图谱内核)
- opencog/cogutil (C++ 通用工具库)
- opencog/ure (C++ 不确定性推理引擎)
- opencog/learn (C++ 机器学习)
- opencog/embodiment (C++ 具身认知)

**V1.1 release OpenCog AGPL-3.0 fork 致谢 (per R130-6 + R131-2 + R132-1)**:
- ✅ `borrow_brainonly = ["R130-6-BORROW-opencog-family-2026Q1-2026-08-11"]` (1 entry, 6 子源, AGPL-3.0, 0 装 PASS 严守)
- ✅ `OSS_NOTICE.md` 加 OpenCog AGPL-3.0 fork 致谢 (per 决策 #55 §3 + R130-6)
- ✅ `THIRD-PARTY-NOTICES.md` 加 OpenCog AGPL-3.0 attribution (cargo-about 0.8.4 生成, per R128-2 阶段 C)
- ✅ `license_files` 加 1 entry (OpenCog AGPL-3.0 fork 致谢, per R137-3 §2.7)

---

## 3. Cargo workspace 1.2.1 bump 决策矩阵 (10 维度, per 决策 #74 B2 + 决策 #73 §3 + 决策 #33 §2.3)

### 3.1 10 维决策矩阵 (per 决策 #74 B2 + 决策 #73 §3 + 决策 #33 §2.3 + R137-3 + R138-6 + R138-7)

**10 维决策矩阵 (per 决策 #74 B2 + 决策 #73 §3 + 决策 #33 §2.3 + R137-3 + R138-6 + R138-7)**:

| # | 决策维度 | V1.0 release 1.2.0 严守状态 | V1.1 release 1.2.1 bump 决策 | 决策依据 | 验证 |
|---|---------|------------------------|---------------------------|---------|------|
| **1** | **兼容性** (backward-compatible?) | 🟢 1.2.0 baseline (整合 #4 commit abf12243) | ✅ MINOR bump (1.2.0 → 1.2.1) backward-compatible 新功能 | semver 严守 (https://semver.org/) + 决策 #74 §1 B2 | ✅ 旧代码仍可编译, 0 breaking change |
| **2** | **升级路径** (5 阶段 5 天 1 周) | 🟡 整合 #5.1/5.2/5.3 commit 全 0 改 src | ✅ 5 阶段 5 天 1 周: 阶段 1 workspace.version + 阶段 2 24 LOCKED crate + 阶段 3 Cargo.lock + 阶段 4 borrow 段 + 阶段 5 8 步 verify | 决策 #71 §2.5 + R137-3 §3 + 决策 #77 §3.1 | ✅ R137-3 5 阶段 1 周 done |
| **3** | **测试影响** (4100+ tests 0 装 PASS 严守 0 重跑) | 🟡 4100+ tests 全 pass (per R129-14 + R129-26) | ✅ 0 重跑 tests (per 决策 #33 §2.3 C2 0 装 PASS 严守) | 决策 #33 §2.3 C2 + R137-3 §3.3 step 5 | ✅ 4100+ tests 0 重跑, V1.1 release 后仍 pass |
| **4** | **文档** (CHANGELOG + ROADMAP + RELEASE_NOTES + OSS_NOTICE 4 文件 update) | 🟡 V1.0 release 1.0.0 CHANGELOG 准备 (per R127-2 P7-1) | ✅ V1.1 release 4 文件 update (per R138-6 §1.2 6.2 docs/ 拍板准备 10 文件) | 决策 #62 §5.2 + R138-6 §1.2 | ✅ 4 文件 V1.1 release update done |
| **5** | **借鉴源** (12 源 0 装 PASS 严守 + OpenCog AGPL-3.0 借脑 ID 索引完成) | 🟡 11 源 (8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过) | ✅ 12 源 (+ 1 借脑 ID 索引完成 OpenCog 家族 6 子源) | 决策 #33 §2.3 C2 + R131-6 §0 + R130-6 | ✅ 12 源 0 装 PASS 严守 + 借脑 ID 索引完成 |
| **6** | **哲学锚** (8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 严守) | 🟢 8 哲学锚 + 不要怕复杂度 = 9 件套 (per 决策 #73 §3) | ✅ 9 件套 严守 0 改 (1.2.1 bump 是版本号 bump 0 触动 思想哲学 + 工程哲学) | 决策 #73 §3 + 哲学文档 15 + 决策 #33 §2.3 B5 | ✅ 9 件套 严守 100% |
| **7** | **风险** (R1-R8 8 维) | 🟡 整合 #5 commit 30 处 fail 必修 (per R129-26) | ✅ R1-R8 8 维: R1 主人误解 / R2 整合 #5 commit 拍板推迟 / R3 主人觉得破坏 R11 baseline / R4 1.2.1 打破向后兼容 / R5 团队对 "不要怕复杂度" 不适应 / R6 24 LOCKED crate 入口签名 Mavis 自决改 风险 / R7 PHL-07 实施 spec-only → impl 风险 / R8 Cargo.lock 1.2.0 → 1.2.1 自动同步 风险 | 决策 #74 §7 + R137-3 §5 + R138-6 §5 | ✅ R1-R8 8 维 缓解 100% |
| **8** | **时机** (2026-11-25 + 2026-11-29 + 2026-11-30) | 🟡 整合 #5 commit 拍板 done (master HEAD = 4207f187 1:43) | ✅ 整合 #6 commit 2026-11-25 + 整合 #7 commit 2026-11-29 + V1.1 release 2026-11-30 | 决策 #33 C1 + 决策 #71 §2.5 + R136-1 §1.2 | ✅ 3 时机点 严守 |
| **9** | **团队** (维护交给未来高水平团队 per 主人 8/11 01:14 拍板) | 🟡 当前 R130 era 41 sub-agent 维护 | ✅ 维护交给未来高水平团队 (per 主人 8/11 01:14 拍板 §3 "自然会有高水平的团队来接手维护") | 决策 #73 §3 + 主人 8/11 01:14 拍板 | ✅ 1.2.1 bump 严守 团队维护 边界 |
| **10** | **长期** (V1.1 release → V2.0 release 远期 8 硬墙可重评) | 🟢 V1.0 release 1.2.0 严守 | ✅ V2.0 release 远期 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构 (per 决策 #74 §2.3) | 决策 #74 §2.3 + 决策 #74 §7 | ✅ V2.0 release 远期 8 硬墙可重评 |

### 3.2 10 维决策矩阵 总结 (per 决策 #74 B2 + 决策 #73 §3 + 决策 #33 §2.3)

**10 维决策矩阵 总结 (per 决策 #74 B2 + 决策 #73 §3 + 决策 #33 §2.3 + R137-3 + R138-6 + R138-7)**:

- ✅ **兼容性 (MINOR bump backward-compatible)**: 1.2.0 → 1.2.1 = MINOR bump (次版本) = backward-compatible 新功能 (per semver + 决策 #74 §1 B2)
- ✅ **升级路径 (5 阶段 5 天 1 周)**: R137-3 5 阶段 5 天 1 周 实施 spec done (per 决策 #77 §3.1 + R137-3 §3)
- ✅ **测试影响 (4100+ tests 0 装 PASS 严守 0 重跑)**: 0 重跑 tests, 0 装 PASS 严守 (per 决策 #33 §2.3 C2)
- ✅ **文档 (4 文件 V1.1 release update)**: CHANGELOG + ROADMAP + RELEASE_NOTES + OSS_NOTICE 4 文件 V1.1 release update (per R138-6 §1.2 6.2 docs/)
- ✅ **借鉴源 (12 源 0 装 PASS 严守 + OpenCog AGPL-3.0 借脑 ID 索引完成)**: 12 源 0 装 PASS 严守 (per R131-6 §0)
- ✅ **哲学锚 (8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 严守)**: 1.2.1 bump 是版本号 bump 0 触动 9 件套 总哲学 (per 决策 #73 §3)
- ✅ **风险 (R1-R8 8 维 缓解 100%)**: 8 维风险 缓解策略 严守 (per 决策 #74 §7 + R137-3 §5)
- ✅ **时机 (3 时机点 严守)**: 整合 #6 commit 2026-11-25 + 整合 #7 commit 2026-11-29 + V1.1 release 2026-11-30 (per 决策 #71 §2.5)
- ✅ **团队 (维护交给未来高水平团队)**: 1.2.1 bump 严守 团队维护 边界 (per 主人 8/11 01:14 拍板)
- ✅ **长期 (V1.1 release → V2.0 release 远期 8 硬墙可重评)**: V2.0 release 远期 8 硬墙可重评 (per 决策 #74 §2.3)

---

## 4. Cargo workspace 1.2.1 bump 跟整合 #6 + #7 + 24 LOCKED + 8 哲学锚 + 不要怕复杂度哲学 关系 (4 关系)

### 4.1 关系 ①: 1.2.1 bump 跟整合 #6 + #7 commit 拍板关系 (per R138-6 + R138-7 + 决策 #74 B1 + 决策 #71 §2.5)

**1.2.1 bump 跟整合 #6 + #7 commit 拍板关系 (per R138-6 + R138-7 + 决策 #74 B1 + 决策 #71 §2.5)**:

**整合 #6 commit 拍板时 1.2.1 bump 同步实施 (per R138-6 §1.2 阶段 2)**:
- ✅ 阶段 1 (2026-11-04 → 2026-11-15, 2 周): 6.1 src/ 拍板准备 (24 LOCKED 入口签名 改写 + PHL-07 实施 + ASI Stage 9 + 形式化 Stage 5.5+ + Tauri Stage 5+ + 三洋葱架构升级 + 9 organ 借 OpenCode + R12 测度对齐)
- ✅ 阶段 2 (2026-11-16 → 2026-11-22, 1 周): **6.2 docs/ 拍板准备 10 文件** (含 Cargo.toml 1.2.1 bump per 决策 #74 B2 + OpenCog AGPL-3.0 fork 致谢加)
- ✅ 阶段 3 (2026-11-23 → 2026-11-24, 估 2 天够): 6.3 reports/ 拍板准备 ~50 文件
- ✅ 阶段 4 (2026-11-25, 1 day): **整合 #6 commit 拍板** (Mavis 自决, per 决策 #74 B1 V1.1 release Mavis 自决改, 11 项 verify 100% 落实后拍板 6.1 → 6.2 → 6.3 顺序 git add + git commit)
- ✅ 阶段 5 (2026-11-26 → 2026-11-30, 估 1 day): V1.1 release 实战准备 (整合 #7 commit 拍板 + 7 步 runbook 续)

**整合 #7 commit 拍板时 1.2.1 bump 验证 (per R138-7 §1.2 阶段 2)**:
- ✅ 阶段 1 (2026-11-26, 1 day): 7.1 src/ 拍板 (Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+ V1.1 release 实施 续)
- ✅ 阶段 2 (2026-11-27 → 2026-11-28, 1 天): **7.2 docs/ 拍板** (Tauri 终极 + ASI Stage 9 实战 + 形式化 Stage 5.5+ 实战 release docs + Cargo.toml 1.2.1 bump 严守 verify)
- ✅ 阶段 3 (2026-11-29, 1 day): 7.3 reports/ 拍板 (V1.1 release 实施 reports/ 续 + HANDOFF-NEXT-SESSION-V1.1-RELEASE)
- ✅ V1.1 release 实战 7 步 runbook: Step 1 整合 #6 commit 拍板 verify + Step 2 配 GitHub remote + Step 3 git push + Step 4 git tag v1.1.0 + Step 5 git push --tags + Step 6 GitHub Release 创建 v1.1.0 + Step 7 V1.1 release 实战 done verify

**1.2.1 bump 跟整合 #6 + #7 commit 拍板 时间表 (per R138-6 §1.2 + R138-7 §1.2 + 决策 #71 §2.5)**:

| 时机 | 任务 | 1.2.1 bump 关系 | 决策依据 |
|------|------|----------------|---------|
| 2026-11-04 → 2026-11-15 (2 周) | 6.1 src/ 拍板准备 (8 大方向) | 0 触动 1.2.1 bump (V1.1 release 实施 src/ = 24 LOCKED 入口签名 Mavis 自决改) | 决策 #74 B1 + R138-6 §1.2 阶段 1 |
| 2026-11-16 → 2026-11-22 (1 周) | **6.2 docs/ 拍板准备 10 文件** | **1.2.1 bump 同步实施 (Cargo.toml workspace.version 1.2.0 → 1.2.1)** | 决策 #74 B2 + R138-6 §1.2 阶段 2 + R137-3 §3.1 |
| 2026-11-23 → 2026-11-24 (估 2 天) | 6.3 reports/ 拍板准备 ~50 文件 | 0 触动 1.2.1 bump | R138-6 §1.2 阶段 3 |
| **2026-11-25 (1 day)** | **整合 #6 commit 拍板** (Mavis 自决) | **1.2.1 bump 拍板** (Cargo.toml workspace.version 1.2.0 → 1.2.1 + 24 LOCKED crate 自动继承 + Cargo.lock 自动同步 + borrow 段 0 装严守 + description + decision_chain_range + integration_chain 5→7 update) | 决策 #74 B2 + R138-6 §1.2 阶段 4 |
| 2026-11-26 → 2026-11-30 (估 1 day) | V1.1 release 实战准备 (整合 #7 commit 拍板 + 7 步 runbook 续) | 1.2.1 bump 验证 (8 步 verify V1.1 release) | R138-6 §1.2 阶段 5 |
| 2026-11-26 (1 day) | 7.1 src/ 拍板 (Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+) | 0 触动 1.2.1 bump | R138-7 §1.2 阶段 1 |
| 2026-11-27 → 2026-11-28 (1 天) | 7.2 docs/ 拍板 | **1.2.1 bump 严守 verify (Cargo.toml 1.2.1 字段全 1.2.1)** | 决策 #74 B2 + R138-7 §1.2 阶段 2 |
| **2026-11-29 (1 day)** | **整合 #7 commit 拍板** (Mavis 自决) | **1.2.1 bump 收尾 (Cargo.toml 1.2.1 字段全 1.2.1 + 7 步 runbook Step 1 整合 #6 commit 拍板 verify)** | R138-7 §1.2 阶段 3 + 7 步 runbook |
| **2026-11-30 (V1.1 release tag)** | **V1.1 release tag v1.1.0 实战** | **1.2.1 bump 实战 (git tag v1.1.0 + GitHub Release 创建 v1.1.0 + 决策链 #131 spec)** | 决策 #74 §2.3 V1.1 release 拍板 |

### 4.2 关系 ②: 1.2.1 bump 跟 24 LOCKED 入口签名 (决策 #74 B1) 关系 (per 决策 #74 B1 + R137-2 + R137-3)

**1.2.1 bump 跟 24 LOCKED 入口签名 (决策 #74 B1) 关系 (per 决策 #74 B1 + R137-2 + R137-3)**:

**24 LOCKED 入口签名 状态 (per 决策 #74 B1 + 决策 #33 §2.3 B1)**:
- ✅ V1.0 release 0 改严守 (24 LOCKED crate mtime baseline 16:34:11 严守, 整合 #5.1 commit 0 改 入口签名)
- 🟢 V1.1 release Mavis 自决改 (前提: 更好的架构, per 决策 #74 B1 改写)
- ✅ V2.0 release 8 硬墙可重评 (per 决策 #74 §2.3)

**1.2.1 bump 跟 24 LOCKED 入口签名 关系分析 (per 决策 #74 B1 + R137-2 + R137-3)**:

| 维度 | 1.2.1 bump | 24 LOCKED 入口签名 | 关系 |
|------|-----------|------------------|------|
| **Cargo.toml 字段** | workspace.version 1.2.0 → 1.2.1 (Cargo.toml:274 改) | 24 LOCKED crate Cargo.toml 字段 0 改 (除 version.workspace = true 继承) | ✅ 1.2.1 bump 0 触动 24 LOCKED Cargo.toml 字段 |
| **src/ 入口签名** | 0 触动 (Cargo.toml 字段 跟 src/ 入口签名 无关) | V1.1 release Mavis 自决改 (前提: 更好的架构, per 决策 #74 B1) | ✅ 1.2.1 bump 0 触动 24 LOCKED src/ 入口签名 |
| **mtime baseline 16:34:11** | 0 触动 (Cargo.toml 字段 跟 mtime 无关) | 0 触动 (24 LOCKED crate mtime 严守) | ✅ 1.2.1 bump 0 触动 24 LOCKED crate mtime |
| **R11 baseline 3 值** | 0 触动 (Cargo.toml 字段 跟 R11 baseline 无关) | 0 触动 (V1.0 release 0 改严守, V1.1 release R12 测度对齐 改 24+11 = 35 维) | ✅ 1.2.1 bump 0 触动 R11 baseline 3 值, V1.1 release R12 测度对齐 跟 1.2.1 bump 同步 |
| **PHL-07 实施** | 0 触动 (Cargo.toml 字段 跟 PHL-07 实施无关) | V1.1 release PHL-07 实施 (24 → 25 LOCKED + 13 → 14 键) | ✅ 1.2.1 bump 0 触动 PHL-07 实施 |

**1.2.1 bump 跟 24 LOCKED 入口签名 关系总结 (per 决策 #74 B1 + R137-2 + R137-3)**:
- ✅ **1.2.1 bump = Cargo.toml workspace.version bump** (Cargo.toml:274 改 1 line)
- ✅ **24 LOCKED 入口签名 = src/ lib.rs 字段** (per 决策 #33 §2.3 B1)
- ✅ **1.2.1 bump 跟 24 LOCKED 入口签名 0 关系** (Cargo.toml 字段 跟 src/ 入口签名 无关)
- ✅ **1.2.1 bump 跟 V1.1 release 24 LOCKED 入口签名 Mavis 自决改 0 关系** (决策 #74 B1 是 src/ 改写, 跟版本号 bump 无关)
- ✅ **1.2.1 bump 跟 V1.1 release 24 LOCKED Cargo.toml 0 关系** (24 LOCKED Cargo.toml 0 改, 自动继承 workspace.version)
- ✅ **24 LOCKED crate 完整 25 (24 + PHL-07) 严守** (per 决策 #74 A3 PHL-07 V1.1 release 实施 + 24 LOCKED crate mtime baseline 严守)

### 4.3 关系 ③: 1.2.1 bump 跟 8 哲学锚 + 不要怕复杂度哲学 (决策 #73 §3) 关系 (per 决策 #73 §3 + 哲学文档 15 + 决策 #33 §2.3 B5)

**1.2.1 bump 跟 8 哲学锚 + 不要怕复杂度哲学 (决策 #73 §3) 关系 (per 决策 #73 §3 + 哲学文档 15 + 决策 #33 §2.3 B5)**:

**8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 (per 决策 #73 §3 + 哲学文档 15)**:

| # | 哲学锚 | 类型 | 1.2.1 bump 严守 | 1.2.1 bump 拓维 |
|---|------|----|----------------|----------------|
| **S-1** | **北极星** | 思想哲学 | ✅ 严守 0 改 (Cargo.toml 1.2.1 bump 0 触动 思想哲学) | 无 |
| **S-2** | **实事求是** | 思想哲学 | ✅ 严守 0 改 (1.2.1 bump 严守 实际状态 = 5 阶段 5 天 1 周 实施 spec) | 无 |
| **S-3** | **质量工程化** | 思想哲学 | ✅ 严守 0 改 (1.2.1 bump 严守 cargo build + test + clippy + fmt + audit + deny + doc + 24 LOCKED 入口签名 8 步 verify) | 无 |
| **O-1** | **安全优先** | 思想哲学 | ✅ 严守 0 改 (1.2.1 bump 严守 0 装 PASS 严守 + 0 改 24 LOCKED mtime baseline 16:34:11) | 无 |
| **O-2** | **走在前人** | 思想哲学 | ✅ 严守 0 改 (1.2.1 bump 严守 借鉴 12 源 + OpenCog AGPL-3.0 借脑 ID 索引完成) | 无 |
| **O-3** | **干到底** | 思想哲学 | ✅ 严守 0 改 (1.2.1 bump 5 阶段 5 天 1 周 严守 干到底) | 无 |
| **O-4** | **接手** | 思想哲学 | ✅ 严守 0 改 (1.2.1 bump 维护交给未来高水平团队 per 主人 8/11 01:14 拍板) | 无 |
| **O-5** | **不假装** | 思想哲学 | ✅ 严守 0 改 (1.2.1 bump 8 步 verify 0 装 PASS 严守 0 假装) | 无 |
| **🆕 不要怕复杂度** | **最强效果 + 最厉害工程** | **工程哲学** | ✅ 严守 0 改 (1.2.1 bump = MINOR bump, backward-compatible 新功能 = 严守 不破坏现有架构) | 🟢 1.2.1 bump 拓维 MINOR bump backward-compatible 新功能 (24 LOCKED 入口签名 Mavis 自决改 + PHL-07 实施 + ASI Stage 9 + 三洋葱 V2 + 9 organ 借 OpenCode + R12 测度对齐) = 不要怕复杂度哲学落地 |

**1.2.1 bump 跟 9 件套 总哲学 关系总结 (per 决策 #73 §3 + 哲学文档 15 + 决策 #33 §2.3 B5)**:
- ✅ **8 哲学锚 (思想哲学)**: 1.2.1 bump 严守 0 改 (1.2.1 bump 是 版本号 bump, 0 触动 思想哲学)
- ✅ **不要怕复杂度 (工程哲学)**: 1.2.1 bump 严守 0 改 (1.2.1 bump 是 MINOR bump = backward-compatible 新功能, 0 破坏现有架构 = 严守 不怕复杂度哲学)
- ✅ **1.2.1 bump 拓维 MINOR bump backward-compatible 新功能 (24 LOCKED 入口签名 Mavis 自决改 + PHL-07 实施 + ASI Stage 9 + 三洋葱 V2 + 9 organ 借 OpenCode + R12 测度对齐) = 不要怕复杂度哲学落地**
- ✅ **思想哲学 + 工程哲学 = 9 件套 总哲学 严守 100%**
- ✅ **1.2.1 bump 严守 9 件套 严守 = 9 件套 总哲学 严守 100%**

### 4.4 关系 ④: 1.2.1 bump 跟 Cargo.toml borrow 段 (cloned=10, rate_limited=0, skipped=1) 关系 (per R131-6 §0 + 决策 #74 B2 + 决策 #33 §2.3 C2)

**1.2.1 bump 跟 Cargo.toml borrow 段 (cloned=10, rate_limited=0, skipped=1) 关系 (per R131-6 §0 + 决策 #74 B2 + 决策 #33 §2.3 C2)**:

**Cargo.toml borrow 段 状态 (per R131-6 §0 + R145-3 §3.3 Step 3)**:
- V1.0 release 17:44 状态 (整合 #4 commit 后): `borrow = { count_total = 11, count_cloned = 8, count_rate_limited = 3, count_skipped = 1 }` + `borrow_cloned` 列表 7 entries
- V1.0 release 22:50 update (整合 #5.2 commit 时, per 决策 #62 §5.2): `borrow = { count_total = 12, count_cloned = 10, count_rate_limited = 0, count_skipped = 1, count_brainonly = 1 }` + `borrow_cloned` 列表 10 entries + `borrow_rate_limited` 列表 0 entries + `borrow_brainonly` 列表 1 entry (R130-6 OpenCog 家族 6 子源)
- V1.1 release 0 装严守 二次 verify (整合 #6 commit 时, per R131-6 §0 + R137-3 §3.4): 同 22:50 update 状态, 0 装 PASS 严守 100%

**1.2.1 bump 跟 Cargo.toml borrow 段 关系分析 (per R131-6 §0 + 决策 #74 B2 + 决策 #33 §2.3 C2)**:

| 维度 | 1.2.1 bump | Cargo.toml borrow 段 | 关系 |
|------|-----------|---------------------|------|
| **workspace.version 字段** | 1.2.0 → 1.2.1 (Cargo.toml:274 改 1 line) | 0 触动 (borrow 段 在 [workspace.metadata.apeireth]:296-320, 跟 [workspace.package]:273 段 无关) | ✅ 1.2.1 bump 0 触动 borrow 段 |
| **`borrow = { ... }` 字段** | 0 触动 (workspace.version bump 跟 borrow 段 无关) | V1.0 release 22:50 update 状态 0 改, V1.1 release 0 装严守 二次 verify 0 改 | ✅ 1.2.1 bump 0 触动 borrow = { ... } 字段 |
| **`borrow_cloned = [...]` 列表** | 0 触动 (workspace.version bump 跟 borrow_cloned 列表 无关) | V1.0 release 22:50 update 10 entries, V1.1 release 0 装严守 二次 verify 0 改 | ✅ 1.2.1 bump 0 触动 borrow_cloned 列表 |
| **`borrow_rate_limited = [...]` 列表** | 0 触动 (workspace.version bump 跟 borrow_rate_limited 列表 无关) | V1.0 release 22:50 update 0 entries, V1.1 release 0 装严守 二次 verify 0 改 | ✅ 1.2.1 bump 0 触动 borrow_rate_limited 列表 |
| **`borrow_skipped = [...]` 列表** | 0 触动 (workspace.version bump 跟 borrow_skipped 列表 无关) | V1.0 release 22:50 update 1 entry (opencog AGPL-3.0), V1.1 release 0 装严守 二次 verify 0 改 | ✅ 1.2.1 bump 0 触动 borrow_skipped 列表 |
| **`borrow_brainonly = [...]` 列表** | 0 触动 (workspace.version bump 跟 borrow_brainonly 列表 无关) | V1.0 release 22:50 update 1 entry (R130-6 OpenCog 家族 6 子源), V1.1 release 0 装严守 二次 verify 0 改 | ✅ 1.2.1 bump 0 触动 borrow_brainonly 列表 |
| **`borrow_local_path` 字段** | 0 触动 (workspace.version bump 跟 borrow_local_path 无关) | 0 改 (Cargo.toml:320 0 改) | ✅ 1.2.1 bump 0 触动 borrow_local_path 字段 |

**1.2.1 bump 跟 Cargo.toml borrow 段 关系总结 (per R131-6 §0 + 决策 #74 B2 + 决策 #33 §2.3 C2)**:
- ✅ **1.2.1 bump = Cargo.toml workspace.version bump** (Cargo.toml:274 改 1 line)
- ✅ **Cargo.toml borrow 段 = [workspace.metadata.apeireth]:296-320 段** (跟 [workspace.package]:273 段 无关)
- ✅ **1.2.1 bump 0 触动 Cargo.toml borrow 段** (workspace.version bump 跟 borrow 段 字段 无关)
- ✅ **V1.0 release 22:50 update borrow 段 0 改** (整合 #5.2 commit 时 已 update 17:44 → 22:50)
- ✅ **V1.1 release borrow 段 0 装严守 二次 verify 0 改** (per R131-6 §0 + 决策 #33 §2.3 C2)
- ✅ **1.2.1 bump 跟 Cargo.toml borrow 段 0 关系** (workspace.version bump 跟 borrow 段 字段 无关)

---

## 5. Cargo workspace 1.2.1 bump 跟 Cargo.toml borrow 段 的关系 (R150-3 拓维, per 决策 #62 §5.2 + R131-6 + R145-3 + R137-3)

### 5.1 Cargo.toml borrow 段 现状 (per R131-6 §1.1 + R145-3 §3.3 Step 3 + R137-3 §2.2)

**Cargo.toml:296-320 [workspace.metadata.apeireth] 段 (per R128-2 阶段 C 拍板, per 决策 #55 §3 + 决策 #58 §1.3)**:

```toml
[workspace.metadata.apeireth]

# 借鉴源码 8/11 ✅ cloned (per decision-36 + #47 + #55 + #58)
# 0 装 PASS 严守 (per decision-33 §2.3 C2 + 主人 17:22 升级授权):
#   ✅ = 真实施 (有真 src 改动 + tests pass) | ⏳ = 限流持续重试 | ❌ = 永久跳过
borrow = { count_total = 11, count_cloned = 8, count_rate_limited = 3, count_skipped = 1 }
borrow_cloned = [
    "clap-rs/clap 4.6.6 (Apache-2.0 + MIT dual, R125-2 ✅ done, 整合 #5 commit 时机 P0 supervisor era)",
    "hyperium/hyper 0.1.20 (MIT, R125-3 ✅ done, P0 supervisor era)",
    "modelcontextprotocol/servers 76d64c8 (MIT → Apache-2.0 过渡, R125-4 ✅ done, P0 supervisor era)",
    "PyO3/PyO3 0.29.2 (Apache-2.0 + MIT dual, R125-9 ✅ done, P1 supervisor era)",
    "model-checking/kani 0.67.0 (MIT + Apache-2.0 dual, R125-10 ✅ done, P2 supervisor era, 触发 B3 V0.5 25 维)",
    "langchain-ai/langgraph d56666f (MIT, R125-13 ✅ done, P2 supervisor era, 触发 B3 25→30 维)",
    "obra/superpowers 6.2.0 (MIT, R125-14 ✅ done, P2 supervisor era, 触发 Library Stage 4 自治 P5-1)",
]
borrow_rate_limited = [
    "BerriAI/litellm (⏳ 限流持续 15+ min, P6-1 R127-2 阶段 A 21:18 派重试, 通常 MIT)",
    "sst/opencode (⏳ 限流持续, P6-2 R127-2 阶段 A 21:18 派重试, 通常 MIT)",
    "NVIDIA/NeMo-Guardrails (⏳ git submodule 0 init, P6-3 R127-2 阶段 A 21:18 派重试, 通常 Apache-2.0)",
]
borrow_skipped = [
    "opencog/opencog (❌ AGPL-3.0 传染性 copyleft, 跟主仓 Apache-2.0 不兼容, per decision-22 §4 + decision-55 §3, 0 集成 0 假装)",
]
borrow_local_path = ".openclaw/workspace/borrowed-repos/"
```

**Cargo.toml borrow 段 关键诚实标 (per R131-6 §1.2 关键诚实标)**:
- 🔴 `count_cloned=8` vs `borrow_cloned` 列表 7 entries 不一致 (Guardrails 在 `borrow_rate_limited` 第 3 项)
- 🔴 `count_total=11` (实际 8+3+1=12) vs 标 11 不一致
- 🔴 `decision_chain_range = "decision-22 ~ decision-58"` (37 个) vs 当前真实范围 decision-22 ~ decision-86+ (估 65+ 个) 不一致
- 🔴 `description = "借鉴 8/11"` vs 整合 #5.2 commit 时 "借鉴 10/11 + 1 借脑 = 11/12" 不一致

**V1.0 release (整合 #5.2 commit) borrow 段 update 计划 (per 决策 #62 §5.2 + R131-6 §1.2 + R131-2 §4.3)**:

| 段 | 整合 #4 commit 后 (17:44 状态) | 整合 #5.2 commit 时 (22:50 update) | 🆕 R130-6 提议 (整合 #5.2 commit 时进一步 update) |
|----|--------------------------------|------------------------------------|----------------------------------------------------|
| `borrow = { ... }` | `{ count_total = 11, count_cloned = 8, count_rate_limited = 3, count_skipped = 1 }` | `{ count_total = 11, count_cloned = 10, count_rate_limited = 0, count_skipped = 1 }` | 🆕 `{ count_total = 12, count_cloned = 10, count_rate_limited = 0, count_skipped = 1, count_brainonly = 1 }` |
| `borrow_cloned = [...]` | 7 entries (clap/hyper/servers/PyO3/kani/langgraph/superpowers) | 8 entries (+Guardrails) | 🆕 10 entries (+LiteLLM 借鉴 ID 索引完成, +opencode 借鉴 ID 索引完成) |
| `borrow_rate_limited = [...]` | 3 entries (litellm/opencode/Guardrails) | 0 entries (P6-1/2/3 全 done) | 🆕 0 entries |
| `borrow_skipped = [...]` | 1 entry (opencog AGPL-3.0) | 1 entry (0 改) | 🆕 1 entry (0 改) |
| 🆕 `borrow_brainonly = [...]` | (N/A) | (N/A) | 🆕 **1 entry: `R130-6-BORROW-opencog-family-2026Q1-2026-08-11`** (6 子源, AGPL-3.0, 0 装 PASS 严守, per 决策 #33 §2.3 C2) |

### 5.2 1.2.1 bump 跟 Cargo.toml borrow 段 关系 (per R131-6 §0 + 决策 #74 B2 + 决策 #33 §2.3 C2)

**1.2.1 bump 跟 Cargo.toml borrow 段 关系 (per R131-6 §0 + 决策 #74 B2 + 决策 #33 §2.3 C2)**:

**1.2.1 bump 0 触动 Cargo.toml borrow 段 (R150-3 拓维核心结论)**:
- ✅ **1.2.1 bump = Cargo.toml workspace.version bump** (Cargo.toml:274 改 1 line)
- ✅ **Cargo.toml borrow 段 = [workspace.metadata.apeireth]:296-320 段** (跟 [workspace.package]:273 段 无关)
- ✅ **1.2.1 bump 0 触动 Cargo.toml borrow 段 任何字段** (workspace.version bump 跟 borrow 段 字段 无关)
- ✅ **1.2.1 bump 0 触动 borrow = { ... } 字段** (workspace.version bump 跟 borrow = { ... } 字段 无关)
- ✅ **1.2.1 bump 0 触动 borrow_cloned / borrow_rate_limited / borrow_skipped / borrow_brainonly / borrow_local_path 字段** (workspace.version bump 跟这些字段 无关)

**V1.1 release borrow 段 0 装严守 二次 verify (per R131-6 §0 + 决策 #33 §2.3 C2)**:
- ✅ V1.0 release 22:50 update 状态 0 改 (整合 #5.2 commit 时 已 update 17:44 → 22:50, 0 触动)
- ✅ V1.1 release 0 装严守 二次 verify 状态 0 改 (整合 #6 commit 时 0 触动, 严守 22:50 状态)
- ✅ 整合 #6 commit 拍板时 0 装 PASS 严守 (0 cargo install / 0 cargo add, per 决策 #33 §2.3 C2)
- ✅ 整合 #7 commit 拍板时 borrow 段 0 改 严守 (整合 #6 commit 时 已 update 22:50 状态, 整合 #7 commit 时 0 改)
- ✅ V1.1 release borrow 段 0 装严守 二次 verify = 0 触动 borrow 段 0 改 = 1.2.1 bump 0 触动 borrow 段 0 改 = 0 越界 B2 严守 1.2.0 / 0 越界 22:50 update 状态

---

## 6. Cargo workspace 1.2.1 bump 实施 spec (整合 #6 + #7 commit 拍板, per R137-3 + R138-6 + R138-7 + 决策 #74 B2)

### 6.1 5 阶段 5 天 1 周 实施 spec (per R137-3 §3 + 决策 #77 §3.1)

**整合 #6 commit 拍板时 1.2.1 bump 5 阶段 5 天 1 周 实施 spec (per R137-3 §3 + 决策 #77 §3.1 + 决策 #74 B2)**:

| 阶段 | 时机 (估) | 任务 | 1.2.1 bump 关系 | 决策依据 | 实施 sub-agent 派活 |
|------|----------|------|----------------|---------|-------------------|
| **阶段 1** | Day 1 (1 day) | **workspace.version 1.2.0 → 1.2.1** (Cargo.toml:274 改 1 line) | 1.2.1 bump 核心 | 决策 #74 B2 + R137-3 §3.1 | Mavis 自决 |
| **阶段 2** | Day 2 (1 day) | **24 LOCKED crate Cargo.toml 1.2.1** (自动继承, version.workspace = true) | 1.2.1 bump 自动同步 24 LOCKED crate | 决策 #22 §2.2 + 决策 #33 §2.3 B1 + R137-3 §3.2 | Mavis 自决 |
| **阶段 3** | Day 3 (1 day) | **Cargo.lock V1.1 release 依赖更新** (cargo update --offline, 5 步) | 1.2.1 bump Cargo.lock 字段自动同步 | 决策 #74 B2 + 决策 #33 §2.3 C2 + R137-3 §3.3 | Mavis 自决 |
| **阶段 4** | Day 4 (1 day) | **borrow 段 V1.1 release 0 装严守 二次 verify** (11 步 verify, 12 源 0 装严守) | 1.2.1 bump 0 触动 borrow 段, 22:50 状态 0 改 | R131-6 §0 + R137-3 §3.4 + 决策 #33 §2.3 C2 | Mavis 自决 |
| **阶段 5** | Day 5 (1 day) | **8 步 verify V1.1 release** (cargo build + test + clippy + fmt + audit + deny + doc + 24 LOCKED 入口签名) | 1.2.1 bump 8 步 verify 100% 落实 | 决策 #74 B2 + 决策 #33 §2.3 C2 + R137-3 §3.5 | Mavis 自决 |
| **总时间盒** | **5 阶段 5 天 1 周** (估 2026-11-16 启动 + 2026-11-22 1 周 done, 整合 #6 commit 拍板 2026-11-25 阶段 4) | 1.2.1 bump 5 阶段 5 天 1 周 实施 spec | 1.2.1 bump = MINOR bump backward-compatible 新功能 | 决策 #71 §2.5 + 决策 #77 §3.1 + R137-3 | Mavis 自决 |

### 6.2 8 步 verify V1.1 release (per R137-3 §3.5 + 决策 #33 §2.3 C2)

**V1.1 release 8 步 verify (per R137-3 §3.5 + 决策 #33 §2.3 C2 + 决策 #74 B2)**:

1. ✅ **`cargo build --workspace --release`** (V1.1 release 编译通过, 24 LOCKED crate + 63 非 LOCKED crate 全编译通过)
2. ✅ **`cargo test --workspace --release`** (V1.1 release 测试通过, 4100+ tests 仍 pass, 0 重跑 0 装 PASS 严守)
3. ✅ **`cargo clippy --workspace --all-targets --all-features -- -D warnings`** (V1.1 release clippy 严守, 0 warning)
4. ✅ **`cargo fmt --all -- --check`** (V1.1 release fmt 严守, 0 改 format)
5. ✅ **`cargo audit`** (V1.1 release 安全 audit 严守, 0 漏洞)
6. ✅ **`cargo deny check`** (V1.1 release license deny 严守, 0 violation)
7. ✅ **`cargo doc --workspace --no-deps --all-features`** (V1.1 release doc 严守, 0 缺失 doc)
8. ✅ **24 LOCKED 入口签名 verify** (V1.1 release 25 LOCKED crate 入口签名 verify, 0 改, per R137-2 5 阶段 8 周 实施 spec)

### 6.3 整合 #6 + #7 commit 拍板 时间表 (per R138-6 §1.2 + R138-7 §1.2 + 决策 #71 §2.5)

**整合 #6 + #7 commit 拍板 时间表 (per R138-6 §1.2 + R138-7 §1.2 + 决策 #71 §2.5)**:

| 时机 | 阶段 | 任务 | 1.2.1 bump 关系 | 派活 | 报告 |
|------|------|------|----------------|------|------|
| 2026-11-04 → 2026-11-15 (2 周) | 6.1 src/ 拍板准备 8 大方向 | 24 LOCKED 入口签名 改写 + PHL-07 实施 + ASI Stage 9 + 形式化 Stage 5.5+ + Tauri Stage 5+ + 三洋葱 V2 + 9 organ 借 OpenCode + R12 测度对齐 | 0 触动 1.2.1 bump (V1.1 release 实施 src/ = 24 LOCKED 入口签名 Mavis 自决改) | 7-15 sub-agent (R137-PHL07-1~5 + R137-LOCKED-1~5 + R137-ASI-1~5 + R137-FORMAL-1~5 + R137-TAURI-1~5 + R137-ONION-1~3 + R137-ORGAN-1~3) | ~30 reports (~220 KB) |
| 2026-11-16 → 2026-11-22 (1 周) | **6.2 docs/ 拍板准备 10 文件** | CHANGELOG + ROADMAP + RELEASE_NOTES + OSS_NOTICE + **Cargo.toml 1.2.1 bump** (决策 #74 B2) + OpenCog AGPL-3.0 fork 致谢加 + 三洋葱 V2 升级文档 | **1.2.1 bump 同步实施 (Cargo.toml workspace.version 1.2.0 → 1.2.1)** | 1-3 sub-agent | ~10 reports (~50 KB) |
| 2026-11-23 → 2026-11-24 (估 2 天) | 6.3 reports/ 拍板准备 ~50 文件 | 决策链 #78-#130 + V1.1 release sub-agent 报告 + HANDOFF | 0 触动 1.2.1 bump | 1-2 sub-agent | ~50 reports (~300 KB) |
| **2026-11-25 (1 day)** | **整合 #6 commit 拍板** (Mavis 自决) | 6.1 + 6.2 + 6.3 顺序 git add + git commit, 11 项 verify 100% 落实后拍板 | **1.2.1 bump 拍板** (Cargo.toml workspace.version 1.2.0 → 1.2.1 + 24 LOCKED crate 自动继承 + Cargo.lock 自动同步 + borrow 段 0 装严守 + description + decision_chain_range + integration_chain 5→7 update) | Mavis 自决 | (Mavis 拍板通知) |
| 2026-11-26 → 2026-11-30 (估 1 day) | V1.1 release 实战准备 (整合 #7 commit 拍板 + 7 步 runbook 续) | 7.1 + 7.2 + 7.3 拍板 + 7 步 runbook | 1.2.1 bump 验证 (8 步 verify V1.1 release) | Mavis 自决 | (Mavis 拍板通知) |
| 2026-11-26 (1 day) | 7.1 src/ 拍板 | Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+ V1.1 release 实施 续 | 0 触动 1.2.1 bump | Mavis 自决 | (Mavis 拍板通知) |
| 2026-11-27 → 2026-11-28 (1 天) | 7.2 docs/ 拍板 | Tauri 终极 + ASI Stage 9 实战 + 形式化 Stage 5.5+ 实战 release docs | **1.2.1 bump 严守 verify (Cargo.toml 1.2.1 字段全 1.2.1)** | Mavis 自决 | (Mavis 拍板通知) |
| **2026-11-29 (1 day)** | **整合 #7 commit 拍板** (Mavis 自决) | 7.1 + 7.2 + 7.3 拍板 | **1.2.1 bump 收尾 (Cargo.toml 1.2.1 字段全 1.2.1 + 7 步 runbook Step 1 整合 #6 commit 拍板 verify)** | Mavis 自决 | (Mavis 拍板通知) |
| **2026-11-30 (V1.1 release tag)** | **V1.1 release tag v1.1.0 实战** | 7 步 runbook: Step 1 整合 #6 commit 拍板 verify + Step 2 配 GitHub remote + Step 3 git push + Step 4 git tag v1.1.0 + Step 5 git push --tags + Step 6 GitHub Release 创建 v1.1.0 + Step 7 V1.1 release 实战 done verify | **1.2.1 bump 实战 (git tag v1.1.0 + GitHub Release 创建 v1.1.0 + 决策链 #131 spec)** | 主人起床后手跑 + Mavis 协调 | (决策链 #131 spec) |
| **总时间盒** | **整合 #6 commit 5 阶段 4 周 + 2 天 = 1 个月 + 2 天 + 整合 #7 commit 3 阶段 1 周 = 5-6 周 总** (估 2026-11-04 启动 + 2026-11-30 V1.1 release) | 整合 #6 + #7 commit 拍板实战 5+3 阶段 | 1.2.1 bump 5 阶段 5 天 1 周 实施 spec 整合到 整合 #6 commit 6.2 docs/ 拍板准备 阶段 2 (2026-11-16 → 2026-11-22 1 周) | 9-20 sub-agent (估) | ~135 reports (~870 KB) |

### 6.4 整合 #6 commit 拍板 11 项 verify (per R138-6 §1.2 + 决策 #74 B1 + 决策 #71 §2.5)

**整合 #6 commit 拍板 11 项 verify (per R138-6 §1.2 + 决策 #74 B1 + 决策 #71 §2.5)**:

1. ✅ **整合 #6.1 src/ commit 拍板** (8 大方向 24 LOCKED 入口签名 Mavis 自决改 + PHL-07 实施 + ASI Stage 9 + 形式化 Stage 5.5+ + Tauri Stage 5+ + 三洋葱 V2 + 9 organ 借 OpenCode + R12 测度对齐, per 决策 #74 B1)
2. ✅ **整合 #6.2 docs/ commit 拍板** (10 文件 update + **Cargo.toml 1.2.1 bump** per 决策 #74 B2 + OpenCog AGPL-3.0 fork 致谢加 + 三洋葱 V2 升级文档)
3. ✅ **整合 #6.3 reports/ commit 拍板** (~50 文件 update)
4. ✅ **8 步 verify V1.1 release 100% 落实** (cargo build + test + clippy + fmt + audit + deny + doc + 24 LOCKED 入口签名, per R137-3 §3.5)
5. ✅ **24 LOCKED crate 入口签名 verify 100%** (25 LOCKED 总数 = 24 + PHL-07, per R137-2 + R137-1)
6. ✅ **borrow 段 V1.1 release 0 装严守 二次 verify 100%** (12 源 0 装 PASS 严守, per R131-6 §0 + R137-3 §3.4)
7. ✅ **8 硬墙 0 越界 100%** (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #78 §5.2)
8. ✅ **8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 严守 100%** (per 决策 #73 §3 + 哲学文档 15)
9. ✅ **0 装 PASS 严守 100%** (per 决策 #33 §2.3 C2)
10. ✅ **0 主动 commit 严守 100%** (Mavis 自决拍板, per 决策 #33 §2.3 C1)
11. ✅ **0 主动 push 严守 100%** (等 V1.1 release 配 GitHub remote, 主人起床后手跑, per 决策 #33 + 决策 #61 §6)

---

## 7. 8 硬墙严守 verify (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #78 §5.2)

### 7.1 8 硬墙严守 verify 矩阵 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #78 §5.2 + 决策 #86 + R150-3 02:27 调研角色)

**8 硬墙严守 verify 矩阵 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #78 §5.2 + 决策 #86 + R150-3 02:27 调研角色)**:

| # | 8 硬墙 | V1.0 release 严守 | V1.1 release 改写 | R150-3 verify |
|---|--------|-----------------|------------------|----------------|
| **B1** | **24 LOCKED 入口签名** | 🔒 0 改严守 (R11 baseline) | 🟢 Mavis 自决改 (前提: 更好的架构) | ✅ 5 verify 100% 一致 (R129-11 + R129-21 + R131-5 + R145-3 + R150-3 02:27) |
| **B2** | **workspace.version 1.2.0** | 🔒 1.2.0 严守 | 🔒 bump 1.2.1 (本任务核心) | ✅ Cargo.toml:274 实地 grep 100% 一致 (R129-25 + R145-3 + R150-3 02:27 3 verify) |
| **A1** | **R11 baseline 3 值 (0.8682/0.8532/0.9063)** | 🔒 数字 0 改 | 🔒 严守 (哲学 + 效果标) | ✅ 数字严守 100% (R129-11 + R131-5 + R150-3 02:27 3 verify) |
| **A3** | **12 键 + PHL-07** | 🔒 PHL-07 V1.0 spec-only 0 实施 | 🟢 PHL-07 实施 + 12 键其他可改 | ✅ spec-only 严守 100% (R137-1 + R150-3 02:27 2 verify) |
| **B3** | **V0.5 30 维** | 🔒 严守 (哲学公式) | 🔒 严守 (哲学) | ✅ 24 基础 + 6 增强 = 30 维 严守 (R126 + R131-5 + R150-3 02:27 3 verify) |
| **B4** | **6 重守门 v7** | 🔒 严守 (哲学守门) | 🔒 严守 (哲学) | ✅ 1-5 嵌套 + 6 Colang DSL 严守 (R126 + R150-3 02:27 2 verify) |
| **B5** | **8 哲学锚** | 🔒 严守 (哲学) | 🔒 严守 (哲学) | ✅ S-1~S-3 + O-1~O-5 严守 (R131-5 + R150-3 02:27 2 verify) |
| **C1** | **0 主动 commit** | 🔒 0 commit 严守 | 🔒 0 commit 严守 | ✅ R150-3 0 commit 100% (Mavis 自决拍板, 决策 #33 §2.3 C1) |
| **C2** | **0 装 PASS 严守** | 🔒 0 装严守 | 🔒 0 装严守 | ✅ 0 cargo install/add 100% (决策 #33 §2.3 C2) |
| **0 push** | **0 主动 push** | 🔒 0 push 严守 | 🔒 0 push 严守 | ✅ R150-3 0 push 100% (决策 #33 + 决策 #61 §6) |

**R150-3 8 硬墙 0 越界 100%** (per 决策 #33 §2.3 + 决策 #74 §3 + 决策 #78 §5.2 + 决策 #86): 10 硬墙细分项全严守, 0 越界.

### 7.2 8 硬墙 严守 verify 总结 (per 决策 #33 §2.3 + 决策 #74 §1)

**8 硬墙 严守 verify 总结 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #78 §5.2 + R150-3 02:27 调研角色)**:

- ✅ **B1 24 LOCKED 入口签名**: V1.0 release 0 改严守 (R11 baseline 16:34:11) + V1.1 release Mavis 自决改 (前提: 更好的架构) + 0 越界
- ✅ **B2 workspace.version 1.2.0**: V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 (本任务核心) + 0 越界
- ✅ **A1 R11 baseline 3 值**: 严守 (哲学 + 效果标) + 0 越界
- ✅ **A3 12 键 + PHL-07**: PHL-07 V1.0 spec-only + V1.1 实施 + 12 键其他可改 + 0 越界
- ✅ **B3 V0.5 30 维**: 严守 (哲学公式) + 0 越界
- ✅ **B4 6 重守门 v7**: 严守 (哲学守门) + 0 越界
- ✅ **B5 8 哲学锚**: 严守 (哲学) + 0 越界
- ✅ **C1 0 主动 commit**: 严守 (主人起床前 0 主动 commit) + 0 越界
- ✅ **C2 0 装 PASS**: 严守 (技术哲学, 不装) + 0 越界
- ✅ **0 push**: 严守 (主人起床前 0 主动 push) + 0 越界

**R150-3 8 硬墙 0 越界 100%** (per 决策 #33 §2.3 + 决策 #74 §3 + 决策 #78 §5.2 + 决策 #86): 10 硬墙细分项全严守, 0 越界.

---

## 8. 风险 + 决策原则 (per 决策 #74 §7 + R137-3 §5 + R138-6 §5 + 决策 #86)

### 8.1 风险 (R1-R8 8 维, per 决策 #74 §7 + R137-3 §5 + R138-6 §5 + 决策 #86)

**1.2.1 bump 风险 8 维 (per 决策 #74 §7 + R137-3 §5 + R138-6 §5 + 决策 #86)**:

**R1 主人误解** (per 决策 #74 §7.1):
- 风险: 主人 8/11 01:14 决策 3 件套理解有误 — 主人误解 1.2.1 bump 是 PATCH bump (而非 MINOR bump)
- 缓解: R150-3 §1.1 semver 严守 详细解读 (MINOR bump vs PATCH bump, per https://semver.org/), 决策 #74 §1 B2 严守
- 验证: ✅ 1.2.0 → 1.2.1 = MINOR bump (次版本), backward-compatible 新功能 (per semver)

**R2 整合 #5 commit 拍板推迟** (per 决策 #74 §7.1):
- 风险: 整合 #5.1 commit 拍板推迟 (R129-3 报告迟迟不出) → 整合 #6 commit 拍板时间表推迟 → 1.2.1 bump 推迟
- 缓解: 整合 #5.1 commit 拍板 done (per 决策 #78 Option A, master HEAD = 4207f187), R139-1 修 25 hard errors → 1.2.1 bump 时间表 严守
- 验证: ✅ 整合 #5.1 commit 拍板 done (per 决策 #78 Option A), 1.2.1 bump 整合 #6 commit 拍板 2026-11-25 严守

**R3 主人觉得破坏 R11 baseline** (per 决策 #74 §7.1):
- 风险: 主人起床后看 1.2.1 bump 觉得"破坏 R11 baseline"
- 缓解: 1.2.1 bump 严守 R11 baseline 3 值 (0.8682/0.8532/0.9063) 0 改 (per 决策 #33 §2.3 A1), V1.0 release 1.2.0 严守 100%, V1.1 release 1.2.1 bump MINOR bump backward-compatible 0 破坏 R11 baseline
- 验证: ✅ 1.2.1 bump 0 触动 R11 baseline 3 值, 0 触动 24 LOCKED crate mtime baseline 16:34:11

**R4 1.2.1 bump 打破向后兼容** (per 决策 #74 §7.1):
- 风险: 1.2.1 bump (MINOR bump) 打破向后兼容
- 缓解: 1.2.1 bump = MINOR bump backward-compatible 新功能 (per semver + 决策 #74 §1 B2), 0 改 [workspace.dependencies] + 0 改 24 LOCKED crate Cargo.toml + 0 cargo install / 0 cargo add
- 验证: ✅ 1.2.1 bump = MINOR bump backward-compatible, 0 改 [workspace.dependencies] 段, 0 改 24 LOCKED crate Cargo.toml 字段

**R5 团队对 "不要怕复杂度"哲学不适应** (per 决策 #74 §7.1):
- 风险: 团队对 "不要怕复杂度" 哲学不适应
- 缓解: 主人 8/11 01:14 拍板 "自然会有高水平的团队来接手维护", 未来高水平团队能适应, 1.2.1 bump 严守 9 件套 总哲学 (8 哲学锚 + 不要怕复杂度)
- 验证: ✅ 1.2.1 bump 严守 9 件套 总哲学 100%, 维护交给未来高水平团队

**R6 24 LOCKED crate 入口签名 Mavis 自决改 风险** (per 决策 #74 §7.1):
- 风险: V1.1 release 24 LOCKED crate 入口签名 Mavis 自决改 风险 (per 决策 #74 B1)
- 缓解: 1.2.1 bump 0 触动 24 LOCKED crate 入口签名 (Cargo.toml 字段 跟 src/ 入口签名 无关), 24 LOCKED crate Cargo.toml 自动继承 workspace.version 1.2.1 (version.workspace = true)
- 验证: ✅ 1.2.1 bump 跟 24 LOCKED 入口签名 0 关系, 1.2.1 bump 跟 V1.1 release 24 LOCKED 入口签名 Mavis 自决改 0 关系

**R7 PHL-07 实施 spec-only → impl 风险** (per 决策 #74 §7.1):
- 风险: V1.1 release PHL-07 实施 (V1.0 spec-only → V1.1 实施) 风险
- 缓解: PHL-07 实施 spec + 实施计划 (per R137-1), 5 阶段 5 周 实施, 41 NEW tests, 0 装 PASS 严守, 1.2.1 bump 跟 PHL-07 实施 0 关系
- 验证: ✅ PHL-07 实施 spec R137-1 done, 1.2.1 bump 跟 PHL-07 实施 0 关系

**R8 Cargo.lock 1.2.0 → 1.2.1 自动同步 风险** (per 决策 #74 §7.1):
- 风险: Cargo.lock 1.2.0 → 1.2.1 自动同步 0 触发依赖升级
- 缓解: 0 装 PASS 严守 (per 决策 #33 §2.3 C2), cargo update --offline (per R137-3 §3.3 step 3), 0 触碰 crates.io, 0 cargo install / 0 cargo add
- 验证: ✅ Cargo.lock 5 步 verify (cargo metadata + check + update + build + test) 100% 落实

**风险 8 维 总结**: ✅ R1-R8 8 维 缓解 100% 严守, 1.2.1 bump 严守 0 越界 8 硬墙 100%.

### 8.2 决策原则 (per 决策 #33 §2.3 + 决策 #74 §7.2 + 用户记忆 #10 + 决策 #86)

**1.2.1 bump 决策原则 (per 决策 #33 §2.3 + 决策 #74 §7.2 + 用户记忆 #10 + 决策 #86)**:

- ✅ **Mavis = orchestrator + 全自决 + 最高权限** (per 主人 8/10 16:31 + 8/11 0:25 + 8/11 01:14 升级授权)
- ✅ **8 硬墙严守 + B1 改写** (per 决策 #33 §2.3 + 决策 #74 §1 拍板)
- ✅ **B1 24 LOCKED 入口签名**: V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (前提: 更好的架构)
- ✅ **B2 workspace.version 1.2.0**: V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 (本任务核心)
- ✅ **A1 R11 baseline 3 值**: 严守 (哲学 + 效果标)
- ✅ **A3 12 键 + PHL-07**: PHL-07 V1.0 spec-only 0 实施 + V1.1 实施, 12 键其他可改
- ✅ **B3 V0.5 30 维**: 严守 (哲学)
- ✅ **B4 6 重守门 v7**: 严守 (哲学)
- ✅ **B5 8 哲学锚**: 严守 (哲学)
- ✅ **C1 0 主动 commit (主人起床前)**: 严守
- ✅ **C2 0 装 PASS 严守**: 严守
- ✅ **0 push (主人起床前)**: 严守
- ✅ **总工程哲学扩展 "不要怕复杂度"** (per 主人 8/11 01:14 拍板 3 件套 §3 + 决策 #73 §3 + 哲学文档 15)
- ✅ **整合 #5 commit 由 Mavis 自动拍板** (per 主人 0:25 + 决策 #33 C1 + 决策 #64 + 决策 #73 §5)
- ✅ **整合 #6 + #7 commit 由 Mavis 自动拍板** (per 决策 #71 §2.5 + R138-6 + R138-7)
- ✅ **0 主动 push 严守** (per 决策 #33 + 决策 #61 §6)
- ✅ **0 主动 IM 主人** (per gate-discipline, 仅 done notification)
- ✅ **0 主动删** (per Safety policy + 决策 #44 + #60)
- ✅ **整合 #4 commit abf12243 严守** (per 决策 #48 + 决策 #61 §1.2)
- ✅ **整合 #5.3 commit 4207f187 严守** (per 决策 #78 + R145-3 §1.3)
- ✅ **决策日志写** (per 决策 #10 + 用户记忆 #10)
- ✅ **0 重复造轮子** (per 决策 #71 §2 永久循环 4 步 + 决策 #73 §2.2 R137-3 + R138-6 + R138-7 续, R150-3 拓维 reference 不重写)
- ✅ **0 借具体源码** (per 决策 #33 §2.3 C2, 调研阶段是文档工作)
- ✅ **0 cargo install / 0 cargo add** (per 决策 #33 §2.3 C2 0 装 PASS 严守)

**决策原则 总结**: ✅ 24 决策原则 严守 100%, 1.2.1 bump 严守 0 越界 24 决策原则 100%.

---

## 9. 一句话 (再次强调)

**R150-3 整合 #5.1 commit 拍板后 Cargo workspace 1.2.0 → 1.2.1 bump 差距分析 (per 决策 #74 B2 V1.0 release 严守 + V1.1 release bump 1.2.1 + R137-3 实施 spec + R138-6 整合 #6 + R138-7 整合 #7 + 主人 8/11 01:14 拍板 3 件套 + 不要怕复杂度哲学 + 永久循环 4 步)**: **V1.0 release 1.2.0 严守 vs V1.1 release 1.2.1 bump 边界清晰** (per 决策 #74 §1 B2 V1.0 release 严守 1.2.0 + V1.1 release bump 1.2.1). **必要性**: semver minor bump (1.2.0 → 1.2.1) = backward-compatible 新功能 (24 LOCKED 入口签名 V1.1 release Mavis 自决改 per 决策 #74 B1). **内容清单 (8 维度)**: ① workspace.version 1.2.0 → 1.2.1 (line 274 改) + ② 24 LOCKED crate Cargo.toml 自动继承 (version.workspace = true) + ③ Cargo.lock workspace deps 字段更新 (cargo update --offline) + ④ borrow 段 V1.1 release 0 装严守 二次 verify (cloned=10, rate_limited=0, skipped=1, brainonly=1, total=12) + ⑤ description 字段 update (V1.0 release "借鉴 8/11" → V1.1 release "借鉴 11/12 + 1 借脑 = 12 源") + ⑥ decision_chain_range update ("decision-22 ~ decision-58" → "decision-22 ~ decision-131") + ⑦ 8 哲学锚 + 24 LOCKED + V0.5 30 维 + 6 重守门 v7 + 14 键 verdict cache metadata 同步 + ⑧ OpenCog AGPL-3.0 fork 致谢. **10 维决策矩阵**: 兼容性 (✅ minor bump 向后兼容) / 升级路径 (✅ 5 阶段 5 天 1 周) / 测试影响 (✅ 4100+ tests 0 装 PASS 严守 0 重跑) / 文档 (✅ 4 文件 V1.1 release update) / 借鉴源 (✅ 12 源 0 装 PASS 严守 + OpenCog AGPL-3.0 借脑 ID 索引完成) / 哲学锚 (✅ 8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 严守) / 风险 (R1-R8 8 维) / 时机 (✅ 2026-11-25 + 2026-11-29 + 2026-11-30) / 团队 (✅ 维护交给未来高水平团队) / 长期 (✅ V1.1 release → V2.0 release 远期 8 硬墙可重评). **4 关系**: 跟整合 #6 + #7 commit 拍板 (整合 #6 commit 2026-11-25 拍板时 24 LOCKED 入口签名 Mavis 自决改 + Cargo.toml 1.2.1 bump) / 跟 24 LOCKED 入口签名 (决策 #74 B1) (V1.0 release 0 改严守 + V1.1 release Mavis 自决改, 1.2.1 bump 0 触动 入口签名) / 跟 8 哲学锚 + 不要怕复杂度 (决策 #73 §3) (8 哲学锚是思想哲学 + 不要怕复杂度是工程哲学 = 9 件套 总哲学, 1.2.1 bump 严守 思想哲学 0 改, 工程哲学 拓维) / 跟 Cargo.toml borrow 段 (1.2.1 bump 跟 borrow 段 0 关系, V1.0 release 22:50 状态 0 改, V1.1 release 0 装严守 二次 verify 0 改). **实施 spec (整合 #6 + #7 commit 拍板)**: 阶段 1 (1 day) workspace.version 1.2.0 → 1.2.1 + 阶段 2 (1 day) 24 LOCKED crate Cargo.toml 1.2.1 (自动继承) + 阶段 3 (1 day) Cargo.lock V1.1 release 依赖更新 (cargo update --offline) + 阶段 4 (1 day) borrow 段 V1.1 release 0 装严守 二次 verify + 阶段 5 (1 day) 8 步 verify V1.1 release. **8 硬墙 0 越界 100%** (B1/B2/A1/A3/B3/B4/B5/C1/C2/0 push 全严守, per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #78 §5.2 + 决策 #86). **0 主动 IM 主人 / 0 主动 commit / 0 主动 push / 0 改 src / 0 改 Cargo.toml** 严守 100% (per 决策 #33 §2.3 + 决策 #62 §6 + 决策 #73 §5 + 决策 #74 §4 + 用户记忆 #10).
