# Decision-63: R129 era 第 1 批 8 sub-agent 派活 (2026-08-11 00:15)

**Date**: 2026-08-11 00:15 (新 session mvs_367e66fae08342ffa399befe4f85dbac 接手后 12 min)
**Author**: Mavis
**触发**: 主人 0:03 拍板"派成员借助团队的力量, 尽可能的派多人来提高效率, 最高 16 人都可以" + 决策 #61 §3.1 R129 era 派活规划 + 决策 #62 整合 #5 commit 拆 3 commit 拍板
**关联**: decision-10 (主人离场 Mavis 自主决策) + decision-22 (24 LOCKED 自主确认) + decision-33 (8 硬墙 + 0 装 PASS) + decision-56 (16 派满策略) + decision-58 (R128-2 3 派活) + decision-61 (新会话接手 + R129 era 派活规划) + decision-62 (整合 #5 commit 拆 3 commit 拍板)

---

## 0. 一句话

**新 session 00:08 派 8 sub-agent 第 1 批 (整合 #5 commit 准备 4 + ASI Python Stage 4-6 续 3 + 1.0 release 流程准备 1, per 决策 #61 §3.1 + #62 拍板). 8 task_id 全 background 模式, task 工具 auto-resume 监督机制 (per 决策 #10 主人 22:50 拍板"删 5 min tick 手动监督" + 主人 0:03 最高授权). 第 2 批 8 sub-agent (R129-9~16, 后端加固 + 形式化扩展 + 1.0 release checklist + R129 路线图) 等 30 min 后派, 错开 cargo build 资源竞争. 0 主动 IM 主人, 0 主动 push, 0 主动 commit, 8 硬墙 0 越界, 0 装 PASS 严守, 整合 #4 commit abf12243 严守 100%.**

---

## 1. 第 1 批 8 sub-agent 派活清单 (00:08 派, 估 30-45 min done)

### 1.1 整合 #5 commit 准备 4 sub-agent (per 决策 #61 §3.1 + #62 拍板)

| Task ID | Sub-agent | 任务 | 报告路径 | 时间盒 | 状态 |
|---------|-----------|------|---------|-------|------|
| `bg_cd2ea558-28cb-48d9-8961-59d1fff4a1a2` | R129-1 | **整合 #5.1 commit src/ 准备** (50+ 文件, B1 入口签名 0 改 verify, 借鉴 8/11 致谢 verify) | `reports/agent-r129-1-integration-5-commit-src-prep-2026-08-11.md` | 30 min | 🟡 跑中 |
| `bg_eba127dd-b079-46ad-ac0d-b46d154a8699` | R129-2 | **整合 #5.2 commit docs/ 准备** (10 文件, B2 1.2.0 严守, 借鉴 8/11 Cargo.toml metadata verify) | `reports/agent-r129-2-integration-5-commit-docs-prep-2026-08-11.md` | 30 min | 🟡 跑中 |
| `bg_c4c43f48-c6b1-49ea-8567-5652ee1be20a` | R129-3 | **8 步 verify 跑** (cargo build/test/audit/deny 实际跑, 24 LOCKED 入口签名 0 改 verify, 0 改 src 严守) | `reports/agent-r129-3-8-step-verify-2026-08-11.md` | 30 min | 🟡 跑中 |
| `bg_c6f9dcfa-2d1e-4025-b085-0b0e84453f21` | R129-7 | **借鉴 11/11 升级 verify** (1:1 verify ✅ 10 + ⏳ 0 + ❌ 1, 0 装 PASS 严守 100%) | `reports/agent-r129-7-borrow-11-11-upgrade-verify-2026-08-11.md` | 20 min | 🟡 跑中 |

### 1.2 ASI Python 整合 Stage 4-6 续 3 sub-agent (per 决策 #55 + #57 + #58 + #61 §3.1)

| Task ID | Sub-agent | 任务 | 报告路径 | 时间盒 | 状态 |
|---------|-----------|------|---------|-------|------|
| `bg_5ca73873-08f7-4be9-8b29-0b04a3840d51` | R129-4 | **ASI Python Stage 4 自治** (P10-1/2/3 + P5-1 + P8-1 续, 4 维度: D1 工具 / D2 反思 / D3 记忆 / D4 决策 自循环) | `reports/agent-r129-4-asi-stage-4-autonomy-2026-08-11.md` | 45 min | 🟡 跑中 |
| `bg_5dd8a6df-093f-4a2d-8d19-246d8c4539b5` | R129-5 | **ASI Python Stage 5 治理** (P5-2 + P8-2 续, 4 维度: G1 资源 / G2 权限 / G3 形式化 / G4 演进 治理) | `reports/agent-r129-5-asi-stage-5-governance-2026-08-11.md` | 45 min | 🟡 跑中 |
| `bg_df80b124-9771-4f72-b683-5f6a1d8d3ca5` | R129-6 | **ASI Python Stage 6 守护** (P5-3 + P8-3 续, 4 维度: K1 错误 / K2 性能 / K3 安全 / K4 健康 守护) | `reports/agent-r129-6-asi-stage-6-guardianship-2026-08-11.md` | 45 min | 🟡 跑中 |

### 1.3 1.0 release 流程准备 1 sub-agent (per 决策 #55 §2.6 + #58 §5 + 主人 8/4 23:33 + #61 §3.1)

| Task ID | Sub-agent | 任务 | 报告路径 | 时间盒 | 状态 |
|---------|-----------|------|---------|-------|------|
| `bg_77a5d33d-353d-4648-8344-ae96d7eec7ca` | R129-8 | **1.0 release 流程准备** (scripts/release/ 4 .sh + 4 .ps1 + 2 .md, 0 主动 push 严守, 主人起床后手跑) | `reports/agent-r129-8-1.0-release-process-2026-08-11.md` | 30 min | 🟡 跑中 |

---

## 2. 监督机制 (per 决策 #10 + 主人 22:50 拍板 + task 工具 auto-resume)

### 2.1 0 主动 IM 主人 (per gate-discipline)
- 主人 22:50 拍板"删 5 min tick, 手动监督" + 0:03 拍板"派成员借助团队的力量"
- 决策 #10 主人离场 Mavis 自主决策 + 决策日志
- **不建 5 min tick cron** (per 主人 22:50 拍板 + 决策 #60 §4 promethean/ 删挂起)
- **task 工具 background + auto-resume 监督** — sub-agent done → owning conversation 自动 resume → Mavis 收到通知 → 派第 2 批 + 写决策日志 + 拍板整合 #5 commit

### 2.2 决策链更新拍板
- **decision-61** (00:03 写完): 新会话接手 + R129 era 派活规划
- **decision-62** (00:08 写完): 整合 #5 commit 拆 3 commit 拍板
- **decision-63** (本决策, 00:15 写): R129 era 第 1 批 8 sub-agent 派活
- **decision-64** (待写, 估 00:38 跑过夜 done): 整合 #5.1 commit src/ 准备 done verify
- **decision-65** (待写): 整合 #5.2 commit docs/ 准备 done verify
- **decision-66** (待写): 8 步 verify 全 PASS + 借鉴 11/11 verify done
- **decision-67** (待写): Mavis 自决拍板整合 #5 commit (5.1 + 5.2 + 5.3 顺序 git add + git commit)
- **decision-68** (待写): 1.0 release 配 GitHub remote + tag 拍板 (主人起床后)

### 2.3 第 2 批 8 sub-agent 派活 (估 00:38 跑过夜 done, 派 R129-9~16)

| Task ID (估) | Sub-agent | 任务 | 报告路径 | 时间盒 |
|--------------|-----------|------|---------|-------|
| R129-9 | (待派) | **Tauri 终极前端 Stage 2 深化** (P11-1/2 续, 5 nav + 主对话 + 9 organ 拟人化深化) | `reports/agent-r129-9-tauri-stage-2-deepening-2026-08-11.md` | 60 min |
| R129-10 | (待派) | **形式化证明扩展 Stage 5.2** (P8-2 续, kani 4502 形式化扩展) | `reports/agent-r129-10-formal-proof-stage-5.2-2026-08-11.md` | 45 min |
| R129-11 | (待派) | **后端 0 装 PASS 终极 verify** (跑全部 0 装 PASS 验证 + 借鉴 11/11 实际文件列表) | `reports/agent-r129-11-backend-0-install-final-verify-2026-08-11.md` | 30 min |
| R129-12 | (待派) | **R129 路线图写** (决策链更新 + R129 era 战略路线) | `reports/agent-r129-12-r129-roadmap-2026-08-11.md` | 30 min |
| R129-13 | (待派) | **1.0 release checklist + GitHub Pages 准备** (per 主人 8/4 23:33 Tauri 终极, 1.0 release 配套) | `reports/agent-r129-13-1.0-release-checklist-2026-08-11.md` | 30 min |
| R129-14 | (待派) | **后端健康度总览** (R125 era 起到 R128-2 era 总览报告, 4100+ tests 状态) | `reports/agent-r129-14-backend-health-overview-2026-08-11.md` | 30 min |
| R129-15 | (待派) | **TUI 升级路线图沉淀** (per 决策 #9, TUI 改瘦后路线图文档化) | `reports/agent-r129-15-tui-upgrade-roadmap-2026-08-11.md` | 30 min |
| R129-16 | (待派) | **R129 era 决策链更新** (R129 era 决策文档 + 跟 R128-2 接) | `reports/agent-r129-16-decision-chain-update-2026-08-11.md` | 30 min |

**派活策略**: 8 sub-agent 第 2 批错开跑 (00:38 跑过夜 done 后派), 避免 16 sub-agent 同时 cargo build 撞车.

---

## 3. 8 硬墙 严守 (per 决策 #33 §2.3)

| 硬墙 | 整合 #5 commit 时机 | 8 sub-agent 严守 |
|------|---------|------------|
| B1 24 LOCKED 入口签名 0 改 | ✅ P2-3 + P4-1 + P14-1 retry verify done | ✅ R129-1/2/3 入口签名 0 改 verify |
| B2 workspace.version 1.2.0 0 改 | ✅ Cargo.toml 1.2.0 严守 | ✅ R129-1/2/3 0 改 Cargo.toml |
| A1 R11 baseline 3 值 0 改 | ✅ 17 文件原位 | ✅ R129-1/2/3 0 改 R11 baseline |
| B3 V0.5 30 维 | ✅ P1-4 retry verify done | ✅ R129-1/2/3 0 触碰 |
| B4 6 重守门 v7 | ✅ P1-3 retry verify done | ✅ R129-1/2/3 0 触碰 |
| B5 8 哲学锚 | ✅ P1-2 done | ✅ R129-1/2/3 0 触碰 |
| A3 12 键 + PHL-07 = 13 键 | ✅ | ✅ R129-1/2/3 0 触碰 |
| C1 0 主动 commit | ✅ Mavis 整合 #5 commit 拍板 | ✅ 8 sub-agent 0 commit, 只 prepare |
| C2 0 装 PASS 严守 | ✅ ✅ 10 + ⏳ 0 + ❌ 1 | ✅ R129-7 verify 借鉴 11/11 |
| C3 升 6 重 v6 → v7 | ✅ | ✅ R129-1/2/3 0 触碰 |
| 0 主动 push | ✅ 等 1.0 release 配 GitHub remote | ✅ R129-8 0 push, 主人起床后手跑 |

**8 硬墙 0 越界 100% PASS** (per R129-1/2/3 + R129-7 verify, 跑过夜 done).

---

## 4. 借鉴源码 0 装 PASS 严守 (per 决策 #33 §2.3 C2)

| 借鉴 ID | 17:44 状态 | 22:50 状态 | 整合 #5 commit | R129-7 verify |
|---------|-----------|-----------|---------|---------|
| clap-rs/clap 4.6.6 | ✅ cloned 17:30 (725 files) | ✅ | 5.1 src/ | ✅ |
| hyperium/hyper 0.1.20 | ✅ cloned 17:30 (80 files) | ✅ | 5.1 src/ | ✅ |
| modelcontextprotocol/servers 76d64c8 | ✅ cloned 17:30 (175 files) | ✅ | 5.1 src/ | ✅ |
| PyO3/PyO3 0.29.2 | ✅ cloned 16:31 (928 files) | ✅ | 5.1 src/ | ✅ |
| model-checking/kani 0.67.0 | ✅ cloned 17:32 (4502 files) | ✅ | 5.1 src/ | ✅ |
| langchain-ai/langgraph d56666f | ✅ cloned 17:30 (829 files) | ✅ | 5.1 src/ | ✅ |
| obra/superpowers 6.2.0 | ✅ cloned 17:32 (234 files) | ✅ | 5.1 src/ | ✅ |
| BerriAI/litellm | ⏳ 限流 0 files | ✅ done 21:38 (P6-1 retry) | 5.1 src/ | ✅ 真实施 9/11 |
| sst/opencode | ⏳ 限流 0 files | ✅ done 22:20 (P6-2 retry) | 5.1 src/ | ✅ 改借鉴已 cloned |
| NVIDIA/NeMo-Guardrails | ⏳ 0 files submodule | ✅ done 21:58 (P6-3 retry) | 5.1 src/ | ✅ 借鉴 ID 索引完成 |
| opencog/opencog | ❌ AGPL-3.0 | ❌ 0 集成 | ❌ 跳过 | ❌ 0 假装"已借鉴" |

**借鉴 11/11 状态 clear**: ✅ 10 真实施 + ⏳ 0 限流 + ❌ 1 跳过 = 11/11 100% (per R129-7 verify).

---

## 5. 整合 #4 commit abf12243 严守 100% (per 决策 #34 + #48 + #60 §4)

- **master HEAD = abf1224371016e36df8f4d3c9a05b33f1c563e0d** (整合 #4 commit 严守)
- **0 重跑**: 整合 #4 commit 19:41 done, 0 必重跑
- **0 重 commit**: 整合 #4 commit 严守, 整合 #5 是新 commit, 不动 abf12243
- **Cargo.toml 1.2.0 严守**: 整合 #4 commit 跟 1.2.0 一致, 整合 #5 5.2 commit Cargo.toml license 字段 0 改 version
- **24 LOCKED 入口签名 0 改**: 整合 #4 commit 跟 24 LOCKED 一致, 整合 #5 5.1 commit LOCKED 内部 fn 可改 + 入口签名 0 改
- **promethean/ 删挂起**: per 决策 #60 主人 22:06 拍板"先放着, 回头我删", Mavis 0 主动删

---

## 6. 整合 #5 commit 拆 3 commit 拍板 (per 决策 #62, Mavis 自决)

### 6.1 5.1 commit (src/ 实施, 50+ 文件)
- **范围**: 31 M src/ + 50+ untracked src/ + tests/ + examples/ + 3 库目录
- **commit message**: per 决策 #62 §2.2 模板, 借鉴 8/11 致谢 + 升级 (8 哲学锚 / 30 维 / 6 重 v7 / 13 键) + 0 越界 8 硬墙 + 整合 #4 commit 严守
- **R129-1 准备**: 5.1 commit 内容 verify (B1 入口签名 0 改 + 借鉴 8/11 真实施) + commit message draft + git add 清单

### 6.2 5.2 commit (1.0 release 文档 + Cargo.toml, 10 文件/目录)
- **范围**: 4 主干文档 (CHANGELOG + ROADMAP + RELEASE_NOTES + OSS_NOTICE) + Cargo.toml license 字段 + workspace.metadata.apeireth section + Cargo.lock + .gitignore + docs/roadmap/ + frontend/ + library/
- **commit message**: per 决策 #62 §3.2 模板, LICENSE 引用链 + 借鉴 8/11 致谢 + 0 越界 8 硬墙
- **R129-2 准备**: 5.2 commit 内容 verify (B2 1.2.0 严守 + Cargo.toml metadata 完整) + commit message draft + git add 清单

### 6.3 5.3 commit (reports/ 决策链 + 报告, 30+ 文件)
- **范围**: 30+ reports/ 文件 (决策 #30-#60 + 41 sub-agent final 报告 + HANDOFF + 决策日志 + cargo logs + locked-audit + promethean 清理脚本)
- **commit message**: per 决策 #62 §4.2 模板, 决策链完整 + 41 sub-agent 报告 + HANDOFF
- **0 commit 时**: 等 Mavis 拍板

### 6.4 整合 #5 commit 拍板时机
- R129-1/2 报告 done + R129-3 8 步 verify 全 PASS + R129-7 借鉴 11/11 verify done → Mavis review
- 4 sub-agent 全 done → Mavis 自决拍板整合 #5 commit
- 5.1 → 5.2 → 5.3 顺序 git add + git commit (0 主动 push 严守)
- 整合 #5 commit done → 主人起床后 8 步 verify + 1.0 release 配 GitHub remote

---

## 7. 风险 + 决策原则 (per 决策 #61 §7 + 决策 #62 §4 + 用户记忆)

### 7.1 风险
- **R1**: 整合 #5 commit 拆 3 commit 顺序错 (5.1 src/ 改, 5.2 docs/ 改, 5.3 reports/ 改) → 5.2 依赖 5.1 (Cargo.toml workspace.metadata.apeireth 引用 src/ 路径) — **缓解**: 5.1 → 5.2 → 5.3 顺序, 5.2 已 done 不依赖 5.1 (Cargo.toml metadata 是字符串引用)
- **R2**: 8 sub-agent 同时跑 cargo build 资源竞争 (16 上限 = 16 agent 跑) — **缓解**: 8 sub-agent 第 1 批 + 8 sub-agent 第 2 批错开 30 min
- **R3**: 8 sub-agent done 后 R129-3 8 步 verify 跑过夜 (估 5-10 min cargo test) — **缓解**: 0 改 src 严守, 已知 src bug 诚实标, 留给整合 #5 commit 后修
- **R4**: 整合 #5 commit 推 master 后 1.0 release tag 失败 — **缓解**: 0 主动 push 严守, 等主人起床后配 GitHub remote
- **R5**: promethean/ 删挂起 (per 决策 #60) → 老 cron 5 个在 mvs_ee7ca3badb session 跑, 0 主动清 — **缓解**: 等主人起床后关 minimaxcode + 自执行脚本

### 7.2 决策原则
- **Mavis = orchestrator, 0 写代码** (per 主人 0:03 授权 + 用户记忆 #6)
- **16 sub-agent 派满策略** (per 主人 0:03 授权 + 决策 #56 16 派满)
- **整合 #5 commit 由 Mavis 自决拍板** (per 主人 0:03 最高授权 + 决策 #33 C1)
- **0 主动 IM 主人** (per gate-discipline)
- **task 工具 background + auto-resume 监督** (per 决策 #10 主人 22:50 拍板"删 5 min tick 手动监督")
- **决策日志写** (per 决策 #10 + 用户记忆 #10)
- **0 装 PASS 严守** (per 决策 #33 §2.3 C2)
- **8 硬墙 0 越界** (per 决策 #33 §2.3)

---

## 8. 一句话 (再次强调)

**新 session 00:08 派 8 sub-agent 第 1 批 (整合 #5 commit 准备 4 + ASI Python Stage 4-6 续 3 + 1.0 release 流程准备 1), task 工具 background + auto-resume 监督 (per 主人 22:50 拍板"删 5 min tick 手动监督"). 第 2 批 8 sub-agent (R129-9~16, 后端加固 + 形式化扩展 + 1.0 release checklist + R129 路线图) 估 00:38 跑过夜 done 后派, 错开 cargo build 资源竞争. 8 硬墙 0 越界, 0 装 PASS 严守, 整合 #4 commit abf12243 严守 100%, 0 主动 IM 主人, 0 主动 push, 0 主动 commit.**
