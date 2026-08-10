# Decision-55: R127 升级路线 + 派活清单 (整合 #5 pre-check + Library Stage 4-6 + 借鉴 3 限流重试 + 1.0 release 准备)

**Date**: 2026-08-10 21:13
**Author**: Mavis (mvs_47dd64fb4fc24e23b30edd5f649bfebb)
**触发**: 主人 21:12 拍板"还有其他新任务没,有的话就把人派出去" + 主人 20:09 拍板"全按你的想法来, 开干" + 主人 20:40 拍板"人不够了就派着补上" + 主人 20:57 拍板"自己设个 cron"
**关联**: decision-33 (8 硬墙) + decision-41 (R125 16 sub-agent 全部 done verify) + decision-42 (整合 #5 pre-checklist) + decision-48 (整合 #4 commit abf12243 done) + decision-51 (16 sub-agent 派活清单) + decision-52 (16 真派 模式) + decision-53 (技术性 locked 解锁授权)

---

## 0. 一句话

**R126 16 sub-agent 派 20:25 + 21:11 补 2 retry (P1-1 R126 后端升级 + P1-3 R126 6 重守门 v7) 跑过夜明早 8/11-8/22 done. 主人 21:12 拍板"派出去" → Mavis 立即写 R127 spec + 派 4 sub-agent 干 (整合 #5 pre-check verify + Library Stage 4 自治 + Library Stage 5 治理 + Library Stage 6 守护),跑过夜明早 8/11-8/22 done. 决策 #55 = R127 = 整合 #5 commit 准备 + Library Stage 4-6 18 任务 + 借鉴 3 限流重试 + 1.0 release 准备 (CHANGELOG / ROADMAP / release notes). 0 装 PASS 严守 (✅ cloned 真实施 + ⏳ 限流持续重试 + ❌ OpenCog 0 集成) + 8 硬墙 0 越界 (B2 1.2.0 0 改 / A1 0.8682/0.8532/0.9063 0 删 0 改 / B1 24 LOCKED 入口签名 0 改 / B5 8 哲学锚 / B3 30 维 / B4 6 重 v7 / A3 13 键 / C1 0 主动 commit / C2 0 装 PASS / C3 升 v7 / 0 主动 push 严守). 整合 #5 commit 时机 = 18 任务 (16 R126 + 2 retry) 全 done + 4 R127 任务全 done + 0 装 PASS 严守 verify + 8 硬墙 0 越界 verify + 24 LOCKED 入口签名 0 改 verify, Mavis 拍板 OR 主人 8/15 拍板.**

---

## 1. 当前状态 (21:12)

### 1.1 R126 16 sub-agent 现状
- ✅ done 12 (P0-1 R125-15e fg_xxxx + P0-2 R125-15f bg_16a97b77 + P0-3 R125-16 retry bg_ff678db3 0 实施诚实标 + P0-4 R125-17 bg_891ffb29 + P1-2 R126 8 哲学锚 bg_77bafd5d + P1-4 R126 25→30 维 verify retry bg_e62f3e67 + P2-1 borrowed-repos 整合 bg_9790f9f8 + P2-2 .gitignore 修 bg_1f8d0ba1 + P2-3 B1 LOCKED verify retry bg_38d67325 24/24 LOCKED 入口签名 0 改 verify + P2-4 Library v1.0 礼物 bg_93832073 + P3-1 R125-18 bg_bfeb840c 含事故 #1 诚实标 + P3-2 R125-19 bg_68dcfdb9 + P3-3 R125-20 bg_b9337fc4 + P3-4 R125-21 retry bg_b9facf9a 30 经典书 9 organ 1:1)
- 🟡 跑中 2 (P1-1 R126 后端升级 retry bg_f8ee6f29 21:11 派 + P1-3 R126 6 重守门 v7 retry bg_b4c7a22f 21:11 派)
- ❌ failed 0 (5 retry 全 done 或 跑中,5 min tick 漏 2 个已补)

### 1.2 主仓状态
- 整合 #4 commit abf12243 19:41 done (46752 file changes, 18 决策文件 #30-#48 + 10 M src + 14 untracked src + .gitignore 升级版)
- master HEAD = abf12243, Cargo.toml 1.2.0 严守, 0 M+?? 异常
- 主仓位置 = `Apeireth-rust/`

### 1.3 借鉴源码 8/11 ✅ cloned (per 决策 #36 §1.1 + 决策 #47 §3.1)
- ✅ 8 真实施: clap 725 / hyper 80 / servers 175 / PyO3 928 / kani 4502 / langgraph 829 / superpowers 234
- ⏳ 3 限流持续: LiteLLM 0 / opencode 0 / Guardrails 0 files submodule
- ❌ 1 跳过: OpenCog AGPL-3.0

---

## 2. R127 升级路线 (4 阶段)

### 2.1 阶段 A: 整合 #5 pre-check verify (P4-1 派 1 sub-agent)
**目标**: 16 R126 sub-agent + 2 retry 全 done 后, verify 整合 #5 commit 准备就绪.
**任务**:
1. 24 LOCKED 入口签名 0 改 verify (cross-check P2-3 retry verify done)
2. 0 装 PASS verify (✅ 8 cloned + ⏳ 3 限流 + ❌ 1 跳过, 0 装"已实施")
3. 8 硬墙 0 越界 verify (B2 1.2.0 0 改 / A1 3 值 0 改 / B1 24 LOCKED / B5 8 哲学锚 / B3 30 维 / B4 6 重 v7 / A3 13 键 / 0 push)
4. 借鉴 8/11 verify (clap 725 / hyper 80 / servers 175 / PyO3 928 / kani 4502 / langgraph 829 / superpowers 234 真 src 改动 + tests pass)
5. Cargo.toml 1.2.0 严守 verify
6. master HEAD = abf12243 verify
7. 写 `reports/agent-p4-1-r127-integration-5-precheck-final-2026-08-10.md`

**借鉴**: 决策 #30-#54 全读 + 整合 #4 commit abf12243 严守

### 2.2 阶段 B: Library Stage 4 自治 (P5-1 派 1 sub-agent)
**目标**: Library Stage 4 自治 = 自演化 + 自升级 + 自修复 (per 决策 #33 §1.4 Stage 4 + 决策 #24)
**任务**:
1. 读 `reports/library-upgrade-plan-2026-08-10.md` + `reports/decision-24-r125-15-library-2026-08-10.md` 拿 Stage 4 spec
2. 实施自演化机制 (借鉴 superpowers 234 自治循环)
3. 实施自升级机制 (借鉴 aGLM 108 PODA cycle)
4. 实施自修复机制 (借鉴 Chidori journal rollback)
5. 真 src 改动 (有真 code 改动 + tests pass, 0 假装"已实施")
6. 写 `reports/agent-p5-1-r127-library-stage-4-autonomy-final-2026-08-10.md`

**借鉴**: superpowers 234 + aGLM 108 (P0/P3 R125-15e/f/18/19/20/21 真实施) + Chidori (R125-8 ✅ done)

### 2.3 阶段 C: Library Stage 5 治理 (P5-2 派 1 sub-agent)
**目标**: Library Stage 5 治理 = 治理策略 + 形式化验证 + 一致性 (per 决策 #33 §1.4 Stage 5)
**任务**:
1. 读 `reports/library-upgrade-plan-2026-08-10.md` + `reports/decision-24-r125-15-library-2026-08-10.md` 拿 Stage 5 spec
2. 实施治理策略 (借鉴 clap 725 derive 模式)
3. 实施形式化验证 (借鉴 Kani 4502 形式化模型)
4. 实施一致性检查 (借鉴 Kani proofs 模板)
5. 真 src 改动 (有真 code 改动 + tests pass, 0 假装"已实施")
6. 写 `reports/agent-p5-2-r127-library-stage-5-governance-final-2026-08-10.md`

**借鉴**: clap 725 (R125-2 ✅ done) + Kani 4502 (R125-10 ✅ done)

### 2.4 阶段 D: Library Stage 6 守护 (P5-3 派 1 sub-agent)
**目标**: Library Stage 6 守护 = 守护 + 跨语言桥 + 长期记忆 (per 决策 #33 §1.4 Stage 6)
**任务**:
1. 读 `reports/library-upgrade-plan-2026-08-10.md` + `reports/decision-24-r125-15-library-2026-08-10.md` 拿 Stage 6 spec
2. 实施守护机制 (借鉴 hyper 80 池复用守护)
3. 实施跨语言桥 (借鉴 PyO3 928 pybridge)
4. 实施长期记忆 (借鉴 servers 175 MCP 协议)
5. 真 src 改动 (有真 code 改动 + tests pass, 0 假装"已实施")
6. 写 `reports/agent-p5-3-r127-library-stage-6-guardianship-final-2026-08-10.md`

**借鉴**: hyper 80 (R125-3 ✅ done) + PyO3 928 (R125-9 ✅ done) + servers 175 (R125-4 ✅ done)

### 2.5 阶段 E: 借鉴 3 限流持续重试 (Mavis 自己写 spec + 下批派 P6-1/2/3)
**目标**: LiteLLM / opencode / Guardrails 3 限流 0 装"已实施" → 真实施 retry
**任务**:
1. Mavis 写 R127 spec for 借鉴 3 限流重试 (决策 #55-2 后续)
2. 下批派 3 sub-agent 干 LiteLLM Provider Registry + opencode 子代理 + Guardrails 6 重守门
3. 跑过夜明早 8/11-8/22 done

**借鉴**: LiteLLM (R125-1 ⏳ 限流) + opencode (R125-12 ⏳ 限流) + Guardrails (R125-5 ⏳ 限流)

### 2.6 阶段 F: 1.0 release 准备 (Mavis 自己写)
**目标**: CHANGELOG v1.0.0 + ROADMAP (1.0 → 2.0) + release notes + LICENSE + OSS NOTICE
**任务**:
1. Mavis 整合 R125-R127 决策写 CHANGELOG v1.0.0
2. Mavis 整合 1.0 → 2.0 路线图写 ROADMAP
3. Mavis 整合 24 LOCKED + 8 哲学锚 + 30 维 + 6 重 v7 + 13 键 + Library v1.0 写 release notes
4. Mavis 整合 R19 era LICENSE + OSS NOTICE 准备
5. 写到 `Apeireth-rust/CHANGELOG.md` + `ROADMAP.md` + `RELEASE_NOTES.md` + `LICENSE` + `OSS_NOTICE` (但不 commit, Mavis 整合 #5 commit 时机拍板)

### 2.7 阶段 G: Cargo build/test/run verify 文档 (Mavis 自己写)
**目标**: 主人起床后 8 步之 1 = Cargo build/test/run verify 文档
**任务**:
1. Mavis 写 Cargo build/test/run verify 文档, 写到 `reports/cargo-build-test-run-verify-2026-08-10.md`
2. 包含: 1) cargo build --workspace 2) cargo test --workspace 3) cargo run --bin apeireth-tui 4) cargo run --bin apeireth-api 5) cargo audit 6) cargo bench 7) 验证 24 LOCKED 入口签名 0 改 8) 验证 8 硬墙 0 越界

---

## 3. 借鉴源码 0 装 PASS 严守 (per 决策 #33 §2.3 C2 + 主人 17:22 升级授权 + 主人 20:32 "技术性 locked 都能解锁")

| 状态 | 借鉴源码 | sub-agent 任务 |
|---|---|---|
| ✅ cloned = 真实施 | clap 725 / hyper 80 / servers 175 / PyO3 928 / kani 4502 / langgraph 829 / superpowers 234 (8/11 ✅) | R125-2/3/4/8/9/10/13/14 真实施 + P0/P1/P3 R125-15e~R125-21 / R126 升级 / R127 Library Stage 4-6 |
| ⏳ 限流 = 准备 | LiteLLM 0 / opencode 0 / Guardrails 0 files submodule (3/11 限流) | R127 阶段 E 下批派 P6-1/2/3 retry 真实施 |
| ❌ 跳过 = 0 集成 | OpenCog AGPL-3.0 (1/11 跳过) | 0 集成 |

**0 装 PASS 严守**: ✅ cloned = 真实施 (有真 src 改动 + tests pass), ⏳ 限流 = 准备 (诚实标 "准备", 0 装"已实施"), ❌ 跳过 (OpenCog = 0 集成, 0 假装 "已实施").

---

## 4. 8 硬墙 (B1-B7 升级版 + A1-A3 严守 + C1-C3 策略) 0 越界

- B2 workspace.version 1.2.0 0 改 (整合 #4 commit abf12243 严守)
- A1 R11 baseline 3 值 0.8682/0.8532/0.9063 数字严守 (17 文件原位, 0 删 0 改)
- B1 24 LOCKED 持续更新, 内部 fn 实施可改, **入口签名 0 改** (P2-3 retry verify 24/24 LOCKED 入口签名 0 改 done)
- B5 6→8 哲学锚 (P1-2 R126 8 哲学锚升级 ✅ done)
- B3 V0.5 25→30 维 (P1-4 R126 25→30 维 verify retry ✅ done)
- B4 6 重守门 v6 → v7 (P1-3 R126 6 重守门 v7 retry 跑中)
- A3 12 键 + PHL-07 = 13 键 (整合 #4 commit done)
- C1 0 commit (Mavis 整合 #5 commit 时机拍板)
- C2 0 装 PASS 严守
- C3 升 6 重 v7
- 0 主动 push (等 1.0 release 配 GitHub remote)

---

## 5. 0 主动 commit + 0 主动 push 严守

- **sub-agent 0 commit** (Mavis 整合 #5 commit 时机拍板, 跑过夜明早 8/11-8/22 done 后)
- **0 主动 push git push** (等 1.0 release 配 GitHub remote)
- **整合 #4 commit abf12243 done** (per 决策 #48, 19:41 主人自执行, 46752 file changes, 0 必重跑)
- **整合 #5 commit 时机**: 18 任务 (16 R126 + 2 retry) 全 done + 4 R127 任务全 done + 0 装 PASS 严守 verify + 8 硬墙 0 越界 verify, Mavis 拍板 OR 主人 8/15 拍板

---

## 6. 5 min tick cron self 监督 (per 17:32 模式 + 主人 20:57 拍板 "自己设个 cron")

- **22 任务** (18 R126 + 4 R127 P4-1/P5-1/P5-2/P5-3) 跑过夜明早 8/11-8/22 done
- 5 min tick cron `watch-r126-16-sub-agents-20-25` 监督 (nextRun 21:15), 0 主动 IM 主人 (per gate-discipline)
- 整合 #5 commit 时机 = sub-agent 全 done + 0 装 PASS 严守 verify + 8 硬墙 0 越界 verify + 24 LOCKED 入口签名 0 改 verify
- 老 cron 5 个仍跑中 (mvs_ee7ca3badb session, 0 监督): dispatch-r125-r125-15-library-immediate (1 min tick) + dispatch-r125-now-min-tick (1 min tick) + watch-r121-1300 (5 min tick) + r123-1-deadline-1725 (5 min tick, R123-1 done 17:26) + R120-finalize-1000 (8 h)
- 新 4 R127 sub-agent task_id 待派活后回填 (per 决策 #35 16 真派 task_id 模式)

---

## 7. 0 主动 push 严守 (per 17:56 + 20:09 + 20:32 + 20:40 + 20:57 + 21:12 严守)

- **0 主动 commit 整合 #5**: 等 22 sub-agent done + 0 装 PASS 严守 + 8 硬墙 0 越界 verify, Mavis 拍板
- **0 主动 push git push**: 等 1.0 release 配 GitHub remote
- **0 主动讨论后续 (R128 升级 / 借鉴 3 限流重试)**: 等 22 sub-agent done 后主人主动问
- **0 主动 push 删 5 散文件 / 33 待删**: 0 必再删, 决策 #50 全 done
- **0 主动 push 整合 #4 commit**: 已 done (per 决策 #48 abf12243, 0 重跑)

---

## 8. 主人起床后 8 步 (per P0-3 retry 报告 + 决策 #55 阶段 G 准备)

1. 修 session working dir (`Apeireth-rust/`)
2. cargo build --workspace
3. cargo test --workspace
4. cargo run --bin apeireth-tui
5. cargo run --bin apeireth-api
6. cargo audit + cargo deny
7. 验证 24 LOCKED 入口签名 0 改
8. 验证 8 硬墙 0 越界 + 0 装 PASS 严守

**整合 #5 commit 时机**: 主人起床后 8 步全 PASS + 0 装 PASS verify + 8 硬墙 0 越界 verify, 主人拍板 OR Mavis 自决.

---

## 9. R127 派活清单 (4 sub-agent, 21:13 派)

| Sub-agent | 任务 | 借鉴 | 8 硬墙 |
|---|---|---|---|
| P4-1 | **整合 #5 pre-check verify** (R127 阶段 A) | 决策 #30-#54 全读 + 整合 #4 commit abf12243 严守 | 0 越界 |
| P5-1 | **Library Stage 4 自治** (R127 阶段 B) | superpowers 234 + aGLM 108 + Chidori | 0 越界 |
| P5-2 | **Library Stage 5 治理** (R127 阶段 C) | clap 725 + Kani 4502 | 0 越界 |
| P5-3 | **Library Stage 6 守护** (R127 阶段 D) | hyper 80 + PyO3 928 + servers 175 | 0 越界 |

派 4 sub-agent (run_in_background=true), 跑过夜明早 8/11-8/22 done.

---

## 10. 0 主动 IM 主人 (per gate-discipline)

- 仅 done notification 主动报告 (per 17:56 严守"仅报告 done 状态")
- 0 主动 plain reply on skip ticks (per gate-discipline)
- 0 主动 push / 0 主动 commit / 0 主动删 / 0 主动讨论后续
- 等 22 sub-agent done + 主人起床后 8 步全 PASS, 主动报告整合 #5 commit 时机
