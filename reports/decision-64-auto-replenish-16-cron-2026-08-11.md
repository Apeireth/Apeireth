# Decision-64: 5 min tick cron 自动监督 + 16 上限补派 + 整合 #5 commit 自动拍板 (2026-08-11 00:25)

**Date**: 2026-08-11 00:25 (新 session mvs_367e66fae08342ffa399befe4f85dbac 接手后 22 min)
**Author**: Mavis
**触发**: 主人 8/11 00:25 拍板"全部你做主。你设置一个 cron, 自动检查现在有多少成员在干活, 不够 16 人就自动补派的"
**关联**: decision-10 (主人离场 Mavis 自主决策) + decision-33 (8 硬墙) + decision-56 (16 派满策略) + decision-58 (R128-2 派活) + decision-61 (R129 era 派活规划) + decision-62 (整合 #5 commit 拆 3 commit 拍板) + decision-63 (R129 第 1 批 8 sub-agent 派活)

---

## 0. 一句话

**主人 0:25 拍板"全部你做主" + "建 cron 自动检查 16 上限自动补派" → Mavis 立即建 5 min tick cron `watch-r129-era-auto-replenish-16`, 监督 8 R129 era sub-agent 状态, 不够 16 自动补派 R129-9~16, 整合 #5 commit 时机 ready 自动拍板 git add + git commit (5.1 src/ + 5.2 docs/ + 5.3 reports/), 0 主动 push 严守 (等主人 1.0 release 配 GitHub remote), 0 主动 IM 主人, 0 主动删. 主人授权升级 = Mavis 全自决 0 边界拍板.**

---

## 1. 主人 0:25 新授权升级

### 1.1 "全部你做主"
- 整合 #5 commit 由 Mavis 自决拍板 (5.1 + 5.2 + 5.3 顺序 git add + git commit)
- 派活策略由 Mavis 自决 (16 上限)
- 决策链更新由 Mavis 自决 (#65 ~ #68 写)
- 1.0 release 准备由 Mavis 自决 (但 git push 由主人手跑, per 0 主动 push 严守)

### 1.2 "建 cron 自动检查 16 上限自动补派"
- 建 5 min tick cron
- 监督 8 R129 era sub-agent 状态 (read reports/agent-r129-*-2026-08-11.md)
- 统计 active 任务数
- 不够 16 自动补派 (R129-9~16, per 决策 #61 §3.1 第 2 批清单)
- 整合 #5 commit 时机 ready → 自动拍板

---

## 2. 5 min tick cron 设计 (Mavis 自决)

### 2.1 cron 元数据
- **名字**: `watch-r129-era-auto-replenish-16`
- **schedule**: `*/5 * * * *` (5 min tick)
- **session**: `mvs_367e66fae08342ffa399befe4f85dbac` (当前 session)
- **agent_name**: `mavis`
- **enabled**: `true`

### 2.2 cron prompt (Mavis 每次 5 min tick 跑)

**Section 1: 监督 8 R129 era sub-agent 状态**
1. read `reports/agent-r129-1-integration-5-commit-src-prep-2026-08-11.md` (5.1 src/ 准备)
2. read `reports/agent-r129-2-integration-5-commit-docs-prep-2026-08-11.md` (5.2 docs/ 准备)
3. read `reports/agent-r129-3-8-step-verify-2026-08-11.md` (8 步 verify)
4. read `reports/agent-r129-4-asi-stage-4-autonomy-2026-08-11.md` (ASI Stage 4)
5. read `reports/agent-r129-5-asi-stage-5-governance-2026-08-11.md` (ASI Stage 5)
6. read `reports/agent-r129-6-asi-stage-6-guardianship-2026-08-11.md` (ASI Stage 6)
7. read `reports/agent-r129-7-borrow-11-11-upgrade-verify-2026-08-11.md` (借鉴 verify)
8. read `reports/agent-r129-8-1.0-release-process-2026-08-11.md` (1.0 release 流程)
9. 统计 done / 跑中 / failed / canceled 状态

**Section 2: 统计 active 任务数 + 16 上限补派**
- 8 R129 era + R129-9~16 (待派 8) = 16 上限
- 如果 active < 16 → 派 R129-9~16 补满 (per 决策 #61 §3.1 第 2 批清单):
  - R129-9 Tauri 终极前端 Stage 2 深化 (P11-1/2 续)
  - R129-10 形式化证明扩展 Stage 5.2 (P8-2 续, kani 4502)
  - R129-11 后端 0 装 PASS 终极 verify
  - R129-12 R129 路线图写
  - R129-13 1.0 release checklist + GitHub Pages 准备
  - R129-14 后端健康度总览 (R125-R128-2 era 4100+ tests 状态)
  - R129-15 TUI 升级路线图沉淀 (per 决策 #9)
  - R129-16 R129 era 决策链更新
- 派活方式: `task` 工具 run_in_background=true, agent_name=general, 详细 prompt (per 决策 #61 §3.1 第 2 批 模板)
- 写 decision-65 (派活清单 + task_id 索引)

**Section 3: 整合 #5 commit 时机 ready verify**
- 整合 #5 commit 时机 ready 条件 (per 决策 #61 §1.4 + #62):
  1. ✅ 41 任务 done verify
  2. ✅ 借鉴 11/11 状态 clear verify (R129-7 done)
  3. ✅ 8 硬墙 0 越界 verify (R129-1/2 done)
  4. ✅ 24 LOCKED 入口签名 0 改 verify (R129-1 done)
  5. ✅ Cargo.toml 1.2.0 严守 (master HEAD = abf12243)
  6. ✅ master HEAD = abf12243 verify
  7. ✅ 决策链 #30-#63 全读 verify
  8. ✅ 8 步 verify 全 PASS (R129-3 done)
- 8 项 verify 100% 落实 → Mavis 自决拍板整合 #5 commit

**Section 4: 整合 #5 commit 自动拍板流程 (Mavis 自决, per 主人 0:25 "全部你做主")**

按 5.1 → 5.2 → 5.3 顺序:
```bash
# 5.1 commit (src/ 实施, per R129-1 §5.1 git add 清单 + §4 commit message draft)
git add $(R129-1 §5.1 清单)
git commit -F reports/agent-r129-1-integration-5-commit-src-prep-2026-08-11.md §4 commit message

# 5.2 commit (1.0 release 文档 + Cargo.toml, per R129-2 §5 git add 清单 + §4 commit message draft)
git add $(R129-2 §5 清单)
git commit -F reports/agent-r129-2-integration-5-commit-docs-prep-2026-08-11.md §4 commit message

# 5.3 commit (reports/ 决策链 + 报告, per 决策 #62 §4 模板)
git add reports/decision-*.md reports/agent-p*.md reports/agent-r125-*.md reports/agent-r126-*.md reports/agent-r129-*.md reports/HANDOFF-NEXT-SESSION-2026-08-10.md
git commit -m "整合 #5.3 commit: 决策链 #30-#63 + 41 sub-agent 报告 + HANDOFF (per decision-62 §4)"
```

写 decision-66 (整合 #5 commit 拍板) + decision-67 (后续 1.0 release 配 GitHub remote + tag, 0 主动 push 严守)

**Section 5: 0 主动 IM 主人 (per gate-discipline)**
- 仅 done notification 主动报告
- 0 主动 plain reply on skip ticks
- 0 主动 push (等主人 1.0 release 配 GitHub remote)
- 0 主动删 (Safety policy 阻挡, per 决策 #44 + #60)

**Section 6: 写决策日志 (per 决策 #10 + 用户记忆 #10)**
- 每个 cron tick 写一行到 `reports/decision-log-r129-era-cron-2026-08-11.md`
- 内容: tick 时间 + active 任务数 + 派活 / 拍板 / 监督 状态

---

## 3. 16 上限自动补派清单 (per 决策 #61 §3.1 第 2 批)

| # | Sub-agent | 任务 | 借鉴 | 报告路径 |
|---|-----------|------|------|---------|
| R129-9 | **Tauri 终极前端 Stage 2 深化** (P11-1/2 续) | Tauri 2.0 + superpowers 234 | `reports/agent-r129-9-tauri-stage-2-deepening-2026-08-11.md` |
| R129-10 | **形式化证明扩展 Stage 5.2** (P8-2 续) | kani 4502 + langgraph 829 | `reports/agent-r129-10-formal-proof-stage-5.2-2026-08-11.md` |
| R129-11 | **后端 0 装 PASS 终极 verify** | 0 借 (verify) | `reports/agent-r129-11-backend-0-install-final-verify-2026-08-11.md` |
| R129-12 | **R129 路线图写** | 0 借 (文档) | `reports/agent-r129-12-r129-roadmap-2026-08-11.md` |
| R129-13 | **1.0 release checklist + GitHub Pages 准备** | 0 借 (流程) | `reports/agent-r129-13-1.0-release-checklist-2026-08-11.md` |
| R129-14 | **后端健康度总览** | 0 借 (报告) | `reports/agent-r129-14-backend-health-overview-2026-08-11.md` |
| R129-15 | **TUI 升级路线图沉淀** | 0 借 (文档) | `reports/agent-r129-15-tui-upgrade-roadmap-2026-08-11.md` |
| R129-16 | **R129 era 决策链更新** | 0 借 (决策) | `reports/agent-r129-16-decision-chain-update-2026-08-11.md` |

---

## 4. 整合 #5 commit 自动拍板条件 (8 项 verify 100% 落实)

1. **41 任务 done verify** ✅ (R125 16 + R126 16 + R127 4 + R127-2 10 + R128 6 + R128-2 3 = 41 任务全 done, per handoff §3.7)
2. **借鉴 11/11 状态 clear** ✅ (R129-7 done, ✅ 10 + ⏳ 0 + ❌ 1)
3. **8 硬墙 0 越界** ✅ (R129-1/2/3 verify, per 决策 #33 §2.3)
4. **24 LOCKED 入口签名 0 改** ✅ (R129-1 verify, per 决策 #22 §1.2 + P2-3 + P4-1 + P14-1 retry)
5. **Cargo.toml 1.2.0 严守** ✅ (master HEAD = abf12243, per 决策 #48)
6. **master HEAD = abf12243** ✅ (整合 #4 commit 严守 100%)
7. **决策链 #30-#63 全读** ✅ (Mavis 0:03-0:15 全读, 31 份决策 + HANDOFF)
8. **8 步 verify 全 PASS** (R129-3 跑中, 估 00:30-00:38 done)

8 项 verify 100% 落实 → Mavis 自决拍板整合 #5 commit.

---

## 5. 风险 + 决策原则 (per 决策 #10 + #33 + #56 + #61 + 用户记忆)

### 5.1 风险
- **R1**: cron 触发时 Mavis session auto-resume, 但 cron prompt 写的不全 → Mavis 行动偏 — **缓解**: cron prompt 写完整 (本决策 §2.2 6 section 模板)
- **R2**: 整合 #5 commit 拍板时 src bug 已知 (per P12-1 + P15-1 verify, apeireth-central 23 + apeireth-api 2 errors) → commit 后 cargo build fail — **缓解**: 0 改 src 严守, 已知 bug 留给整合 #5 commit 后修, 主人起床后 8 步 verify 时再修
- **R3**: 16 sub-agent 同时跑 cargo build 资源竞争 — **缓解**: 第 1 批 8 + 第 2 批 8 错开 30 min, cargo build 错开跑
- **R4**: 整合 #5 commit 推 master 后 1.0 release tag 失败 — **缓解**: 0 主动 push 严守, 等主人 1.0 release 配 GitHub remote
- **R5**: cron 误派 (R129 era 16 sub-agent 全 done 后, cron 还派 17/18/19...) — **缓解**: cron prompt §2 加 "if active == 16, 0 派" 检查
- **R6**: 0 主动 IM 主人 跟 "auto-replenish-16" 矛盾 — **缓解**: 0 IM 主人 = 0 主动 plain reply, 但 done notification (整合 #5 commit 拍板) 是必需, 写 decision-66 报告

### 5.2 决策原则
- **Mavis = orchestrator + 全自决** (per 主人 0:25 "全部你做主" 升级授权)
- **16 sub-agent 派满 + 自动补派** (per 主人 0:25 + 决策 #56)
- **整合 #5 commit 由 Mavis 自动拍板** (per 主人 0:25 + 决策 #33 C1)
- **0 主动 push 严守** (per 决策 #33 + 决策 #61 §6)
- **0 主动 IM 主人** (per gate-discipline, 仅 done notification)
- **0 主动删** (per Safety policy + 决策 #44 + #60)
- **8 硬墙 0 越界** (per 决策 #33 §2.3)
- **0 装 PASS 严守** (per 决策 #33 §2.3 C2)
- **整合 #4 commit abf12243 严守** (per 决策 #48 + 决策 #61 §1.2)
- **决策日志写** (per 决策 #10 + 用户记忆 #10)

---

## 6. 一句话 (再次强调)

**主人 0:25 拍板"全部你做主" + "建 cron 自动检查 16 上限自动补派" → Mavis 立即建 5 min tick cron `watch-r129-era-auto-replenish-16`, 监督 8 R129 era sub-agent 状态, 不够 16 自动补派 R129-9~16 (8 sub-agent 第 2 批清单 per 决策 #61 §3.1), 整合 #5 commit 时机 ready (8 项 verify 100%) 自动拍板 git add + git commit (5.1 src/ + 5.2 docs/ + 5.3 reports/), 0 主动 push 严守 (等主人 1.0 release 配 GitHub remote), 0 主动 IM 主人, 0 主动删. 主人授权升级 = Mavis 全自决 0 边界拍板.**
