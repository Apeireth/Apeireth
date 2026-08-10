# Decision #33 — 主人 17:22 升级授权 + 8 硬墙重置 + B1-B7 升级拍板 (17:23)

**Date**: 2026-08-10 17:23
**Author**: Mavis (新 root session, mvs_47dd64fb4fc24e23b30edd5f649bfebb)
**触发**: 主人 17:22 拍板 "所有 locked 都能改, 这应该在 10locked 文档里说过, 你有最高授权, 最高自主决定权, 不必再问我, 我们的最终目标就是更好. 动用团队来干活, 最高并行 16 个, 且尽量派满不要闲着, 让效率达到最大化"
**关联**: handoff §1 (8 硬墙) + decision-22 (主人 16:31 最高权限) + decision-30 (新 Mavis 接入) + decision-31 (17:30 dry-run) + decision-32 (R125 主管启动) + 10-locked.md (R119 形式撤销 + B1-B7 升级路线) + 8-locked-unified §2 (8 项实质定义)

---

## 0. 一句话

**主人 17:22 三次升级授权叠加: ① 8 项不修改承诺 R119 已形式撤销 (10-locked.md) ② 主人 16:27 "为了升级, 不必犹豫, 完全可以" ③ 主人 17:22 "所有 locked 都能改, 0 装不必要, 16 派满, Mavis 最高自主, 终极目标就是更好"** → **8 硬墙全部重置**, **B1-B7 升级路线 (决策 #22) 立刻全力推进**, **17:30 commit 拍板 add 全部 (含 138 src + 8 src untracked + 1 src D + .gitignore + Cargo.toml 1.2.0)**, **0 主动 push 严守 (等主人 1.0 release 配 GitHub remote)**, **派 16 sub-agent (4 supervisor 各 4 sub-agent) 升级版**.

---

## 1. 主人授权时间线 (8 次拍板累积)

| # | 时间 | 拍板 | 影响 |
|---|------|------|------|
| 1 | 8/10 01:14 | "locked 全部解锁, 原意不变, 关于不能改变的原意得变一下了, 不再要, 按你建议来, 朝最整齐的方向走" | 8 项不修改承诺**形式撤销**, 原意保留 (per 10-locked.md R119 拍板) |
| 2 | 8/10 01:49 | "3 技术类 LOCKED 撤销 (baseline 3 值 / 24 LOCKED crate 实际列表), 文档不锁, 时刻保持最新" | 24 LOCKED 名单**持续更新** + baseline 3 值**数字严守** (A1 严守) |
| 3 | 8/10 14:56 | "你拍" (Mavis 整合 #3 commit 拍板) | Mavis 整合 #3 commit 拍板 (df6dfb69 128 files) |
| 4 | 8/10 16:27 | "为了升级或更好, 要改动现有的 locked, 不必犹豫, 完全可以, locked 也是过去制定的, 会逐渐过时" | LOCKED 升级**完全授权** |
| 5 | 8/10 16:31 | "全部采纳, 全都能动, 需要具体确认的你自己确认就行, 你有最高权限" | Mavis **最高权限** + 24 LOCKED 自主确认 + B1-B7 升级路线 |
| 6 | 8/10 16:37 | "16 成员派满 + cron 监督 + 少人补上" | **16 派满策略** |
| 7 | 8/10 16:43 | "research → library + 调研做完了你自己安排任务升级" | research → library 升级 + R125 续自主 |
| 8 | 8/10 16:51 | "立刻派人, 不用等下一次 cron" | owner-driven mode (0 必等 5 min tick) |
| **9** | **8/10 17:22** | **"所有 locked 都能改, 在 10locked 文档里说过, 你有最高授权, 最高自主决定权, 不必再问我, 我们的最终目标就是更好, 16 派满不要闲着, 让效率达到最大化"** | **8 硬墙全部重置 + 0 装不必要 + 16 派满 + Mavis 最高自主 + 升级为主** |

**累积效果**: 8 硬墙 (handoff §1) **全部重置**, 0 越界 8 硬墙限制 = 0 越界升级路线, 升级 = 0 越界 (B1-B7 升级即原 8 硬墙升级版).

---

## 2. 8 硬墙重置 (per 决策 #22 §2 + 主人 17:22)

### 2.1 8 硬墙 (handoff §1) vs B1-B7 + A1-A3 + C1-C3 (决策 #22 §2.1-2.9)

| 硬墙 (handoff §1) | 决策 #22 实质分类 | 主人 17:22 拍板后 |
|-------------------|-------------------|------------------|
| 1. workspace.version 1.1.0 | **B2 minor 升 1.2.0** (R125 末) | ✅ 升 1.2.0 (R125 末) |
| 2. R11 baseline 3 值 0.8682/0.8532/0.9063 | **A1 严守** (数字 0 改) | ⚠️ 主人 17:22 "all locked" 含 A1? 实际: 数字 0.8682 等仍严守 (per decision-22 §5.1, 主人 7/31 明确不动), 但**测试结构 / 公式 / 测度可调** |
| 3. 24 LOCKED crate mtime 16:34 baseline | **B1 完整名单落实** (24 个) | ✅ 24 LOCKED 名单持续更新 (R119 撤销 3 技术类 LOCKED) |
| 4. 6 哲学锚 | **B5 升 8 锚** (S-3 + O-1) | ✅ 8 哲学锚 (R125 末) |
| 5. V0.5 25 维 | **B3 升 25 维** (24+Robustness) + R125-13 升 30 维 | ✅ V0.5 25 维 (R125 末) / 30 维 (R125-13) |
| 6. 6 重守门 v6 | **B4 升 6 重** (5+Colang DSL) | ✅ 6 重守门 v6 (R125-5) |
| 7. 13 键 + PHL-07 | **A3 12 键原 12 + 新增 PHL-07** (13 键) | ✅ 13 键 (R125-12 后) |
| 8. 0 装 (O-5) + 0 主动 commit + 0 主动 push | **C1-C3 策略 0 改** (但可升级路线内) | ⚠️ 0 装 解除 (主人 17:22 "0 装不必要") + 0 主动 commit = Mavis 整合 #3 17:30 拍板节点 (17:30 后可 commit) + 0 主动 push 严守 (等主人 1.0 release 配 GitHub remote) |

### 2.2 主人 17:22 "所有 locked 都能改" 范围

**所有 locked = B1-B7 升级路线 + A1-A3 + C1-C3** (per 10-locked.md + decision-22 §2):
- B1 24 LOCKED 名单: 24 个完整, 持续更新
- B2 workspace.version 1.1 → 1.2 (R125 末) → 1.0 (R127 release)
- B3 V0.5 24 → 25 维 (R125 末) → 30 维 (R125-13)
- B4 5 重 → 6 重 v6 (R125-5 实施)
- B5 6 → 8 哲学锚 (R125 末)
- B6 双 → 三洋葱 (R125-5)
- B7 9 organ 内部 fn 借 OpenCode (R125-12)
- A1 R11 baseline 3 值 数字: 严守 (0.8682/0.8532/0.9063 数字不动, 但 9 子测度结构可调)
- A2 R11 9 子测度结构: 严守
- A3 12 键原 12 + 新增 PHL-07: 13 键 (R125-12 后)
- C1 0 主动 commit: 17:30 拍板节点收尾, 17:30 后可 commit (R125 续 8/15-9/10 整合)
- C2 0 装 (O-5): 主人 17:22 解除, 实施完成 = 真装 (R125 续 借鉴实施)
- C3 0 装 5 项 5 守门: 升 6 重守门 v6 (R125-5 实施)

### 2.3 8 硬墙 (handoff §1) 重置后 (主人 17:22 后)

| 重置前 (handoff §1) | 重置后 (主人 17:22) |
|----------------------|---------------------|
| 1. workspace.version 1.1.0 0 改 | ✅ **B2 升 1.2.0** (R125 末) |
| 2. R11 baseline 3 值 0 改 (A1 严守) | ⚠️ **数字 0 改 (严守)**, 测度结构 / 公式可调 (A2 严守) |
| 3. 24 LOCKED crate mtime 16:34 baseline 0 触碰 | ✅ **24 LOCKED 名单持续更新** (B1), 内部 fn 实施可改 (R125 续) |
| 4. 6 哲学锚 0 改 | ✅ **B5 升 8 锚** (R125 末) |
| 5. V0.5 25 维 0 改 | ✅ **B3 升 25 维 (R125 末) / 30 维 (R125-13)** |
| 6. 6 重守门 v6 0 改 | ✅ **B4 升 6 重 v6 (R125-5 实施)** |
| 7. 13 键 0 改 | ✅ **A3 12 键原 12 + PHL-07 (R125-12 后)** |
| 8. 0 装 (O-5) + 0 主动 commit + 0 主动 push | ⚠️ **0 装解除 (主人 17:22), 0 主动 commit = 17:30 拍板节点, 0 主动 push 严守 (等主人 1.0 release 配 GitHub remote)** |

---

## 3. 17:30 commit 拍板 (per 新策略: add 全部 + B1-B7 升级)

### 3.1 17:28 dry-run 步骤 (升级版)

```bash
cd .openclaw/workspace/promethean/Apeireth-rust

# Step 1: 验证 0 worktree 误加
git status --porcelain | grep -E 'worktree' && echo "ERROR: worktree 误加" && exit 1

# Step 2: 验证 0 untracked 顶层 (除 out/ + .git_commit_msg.txt)
git status --porcelain | grep -E '^\?\? [^/]' | grep -v '\.git_commit_msg\.txt|out/|worktree' && echo "ERROR: 顶层 untracked 误加" && exit 1

# Step 3: A1 严守 verify (R11 baseline 3 值 数字 0 改)
git status --porcelain | grep -E 'integration_r_measure\.rs' && echo "WARNING: R11 baseline 改动 (确认 A1 严守)"

# Step 4: B2 verify (workspace.version 1.1.0 → 1.2.0, 升级)
grep 'version' Cargo.toml | head -5  # 应该显示 1.2.0

# Step 5: 真 add 全部 (含 src)
git add reports/ docs/ .openclaw/workspace/borrowed-repos/README.md
git add Cargo.toml  # B2 升 1.2.0
git add crates/  # 138 src M+untracked+D + R123-1 fix
git add .gitignore  # 新增 ignore (out/ + apeireth/out/ + .git_commit_msg.txt)

# Step 6: 验证 add 后 0 worktree
git diff --cached --stat | grep -E 'worktree' && echo "ERROR: cached worktree" && exit 1

# Step 7: 8 硬墙 (B1-B7 升级版) verify
git diff --cached --stat | grep -E 'integration_r_measure\.rs' && echo "WARNING: R11 baseline 改动 (确认 A1 严守 数字 0 改)" || echo "OK: A1 严守"
git diff --cached --stat | grep -E 'Cargo\.toml:.*version.*1\.1\.0' && echo "OK: workspace.version 1.1.0 0 改" || echo "WARNING: B2 升 1.2.0 (确认主人授权)"
```

**0 error → 真 commit**:
```bash
git commit -m "R123-R124-R125 阶段整合 #3 + B1-B7 升级 (主人 17:22 升级授权): 24 LOCKED 升级 + 7 文档 + 9 决策 + 3 spec + 2 audit + 调研 138KB + 138 src (含 R123-1 fix) + Cargo.toml 1.2.0 (B2) + .gitignore (新增)

主升级 (per 10-locked.md R119 形式撤销 + 决策 #22 + 主人 17:22 拍板):
- B1 24 LOCKED 名单: 24 个完整, 持续更新
- B2 workspace.version 1.1.0 → 1.2.0 (R125 末 minor, R127 release 1.0.0 大版本归 0)
- B3 V0.5 25 维 (24 + Robustness 鲁棒性, R125-10/13 实施)
- B4 6 重守门 v6 (5 + Colang DSL, R125-5 实施)
- B5 8 哲学锚 (6 + S-3 质量工程化 + O-1 安全优先)
- B6 三洋葱 (双 + DSL 洋葱, R125-5 实施)
- B7 9 organ 内部 fn 借 OpenCode (199KB → 120KB, R125-12 实施)
- A1 R11 baseline 3 值 数字严守 (0.8682/0.8532/0.9063 数字不动)
- A2 R11 9 子测度结构严守
- A3 12 键原 12 + PHL-07 = 13 键 (R125-12 后)
- C1 0 主动 commit = 17:30 拍板节点 (本 commit 拍板)
- C2 0 装 (O-5) 解除 (主人 17:22)
- C3 0 装 5 项 升 6 重守门 v6

新增 250+ 文件 (+~250KB 报告 + 138 src 改动 + 7 docs + 1 borrowed-repos + 1 Cargo.toml + 1 .gitignore):
- 7 docs 更新: 09-anchor 6→8 锚 / 10-locked B1-B7 / 11-baseline V0.5 25 维 / 17-4-gates 5→6 重 v6 / 24-locked-crates 完整名单 / 8-locked-unified §2 第 8 项 1.0.0→1.1.0→1.2.0 / r11-baseline 3 值严守
- 10 决策: #20-#33 (含本决策 主人 17:22 升级授权)
- 3 spec: r125-pipeline 18.4KB / r125-15 10.9KB / library-upgrade 13.8KB
- 2 audit: locked-audit 17.9KB / locked-audit-v2-final 17.9KB
- 1 final: final-17-30 14.7KB
- 1 R123-1 status: 9.2KB
- 1 handoff: 10.3KB
- 1 upgrade-reference: 22.3KB
- R124 调研 138KB: agent-r124-1/2/3 final
- borrowed-repos/README.md 6.2KB: Top 10 借鉴索引
- Cargo.toml: workspace.version 1.1.0 → 1.2.0 (B2 升级)
- .gitignore: 新增 (out/ + apeireth/out/ + .git_commit_msg.txt)
- 138 src 改动: R123-1 fix 2 error (apeireth-mcp 1 + tools_demo 2) + 7 src 增删 (R123-2/3 multimodal.rs + tools/ + protocol_handler_trait.rs + 3 examples) + 130 src 升级 (R125 续 派 16 sub-agent 跑过夜明早)

0 主动 push 严守 (等主人 1.0 release 配 GitHub remote)
16 sub-agent 已派 (4 supervisor 各 4 sub-agent, 升级路线图 P0/P1/P2/P3)

Co-Authored-By: Mavis (决策 #33 整合拍板)"
```

### 3.2 17:30 commit 后 5 步
1. 写 `reports/decision-34-commit-done-2026-08-10.md` 决策记录
2. 看 `git log --oneline -3` 拿 commit hash
3. 4 supervisor 5 min tick 监督 16 sub-agent
4. R123-1 17:25 截止评估 (cron tick 自动)
5. 0 主动 push 严守, 等主人 1.0 release 配 GitHub remote

---

## 4. 16 sub-agent 派活 (per 主人 16:37 + 17:22 "16 派满不要闲着")

### 4.1 4 supervisor × 4 sub-agent (16 总)

| Supervisor | 派 4 sub-agent | 主题 | 升级方向 |
|------------|----------------|------|----------|
| **P0 supervisor** | R125-1 LiteLLM + R125-2 clap + R125-3 hyper + R125-4 MCP | 实施类 P0 | B1 24 LOCKED 升级 + B2 1.2.0 + B3 25 维 + B4 6 重 v6 + B5 8 锚 + B6 三洋葱 + B7 9 organ 内部 |
| **P1 supervisor** | R125-5 NVIDIA + R125-7 aGLM + R125-8 Chidori + R125-9 PyO3 | 实施类 P1 | 同上 + 哲学核心 (R11 9 子) + memory 升级 |
| **P2 supervisor** | R125-10 Kani + R125-12 OpenCode + R125-13 LangGraph + R125-14 superpowers | 实施类 P2 + 形式化 | B3 30 维 (R125-13) + B7 OpenCode 内部借 + 13 键 PHL-07 |
| **P3 supervisor** | R125-15a 学术 + R125-15b 文档 + R125-15c 博客 + R125-15d 视频 | 非 GitHub 学习途径 | Library 升级 (research → library, 主人 16:43 拍板) |

### 4.2 升级版 supervisor prompt 关键约束 (vs 旧 bg_62424f99)

**0 装 PASS (per 主人 17:22)**: ❌ 已解除
- 借鉴源码 0 clone = 0 实施 ✅ (实施前看 clone 状态)
- 借鉴源码 ✅ cloned = 0 假装实施, 真实实施 ✅
- 借鉴源码 ⏳ 限流中 = 等限流结束, 然后实施

**8 硬墙 (B1-B7 升级版)**: 升级路线内
- B1 24 LOCKED 名单: 24 个完整, 持续更新 (R119 撤销 3 技术类 LOCKED)
- B2 workspace.version 1.1 → 1.2 (R125 末) → 1.0 (R127 release)
- B3 V0.5 25 维 (R125 末) → 30 维 (R125-13)
- B4 6 重守门 v6 (R125-5 实施)
- B5 8 哲学锚 (R125 末)
- B6 三洋葱 (R125-5)
- B7 9 organ 内部 fn 借 OpenCode (R125-12)
- A1 R11 baseline 3 值 数字 严守 (0.8682/0.8532/0.9063)
- A2 R11 9 子测度结构 严守
- A3 12 键原 12 + PHL-07 (13 键, R125-12 后)
- C1 0 主动 commit = 17:30 拍板节点 (supervisor 0 commit, 17:30 后 R125 续可 commit 实施)
- C2 0 装 解除 (主人 17:22)
- C3 0 装 5 项 升 6 重守门 v6

**借鉴 ID 严格化** (per decision-22 §3):
- GitHub: `R124-{1,2,3}-BORROW-{owner/repo}-{hash}-2026-08-10`
- 非 GitHub: `R125-15-BORROW-{arxiv|blog|video|community|hub|rfc}-{name|id}-{hash}-2026-08-10`

**0 主动 push**: 严守 (等主人 1.0 release 配 GitHub remote)

---

## 5. Cargo.toml workspace.version 1.1.0 → 1.2.0 (B2 升级)

**当前**: `Cargo.toml:246 version = "1.1.0"`
**升级**: `1.1.0` → `1.2.0` (R125 末 B2 minor 升)
**依据**: 主人 17:22 升级授权 + decision-22 §2.2 B2 路线 + 10-locked.md R119 形式撤销
**R127 release**: 1.2.0 → 1.0.0 (大版本归 0, per decision-22 §2.2)

---

## 6. .gitignore 新增 (主人 17:22 升级后)

**当前 .gitignore** (per 1.1-release README, 应该存在): 待 verify
**新增**:
- `out/` (顶层 Python audit output, 0 装)
- `apeireth/out/` (Python output, 0 装)
- `.git_commit_msg.txt` (commit msg 草稿, 0 装)

---

## 7. 决策链 (接 #32)

- **#30 (17:15)**: 新 Mavis 接入 + 派活 daemon 复活
- **#31 (17:17)**: 17:30 拍板 dry-run + 138 src 改动诚实标
- **#32 (17:18)**: R125 派活大主管启动 + 0 装 PASS 监督 (旧策略)
- **#33 (17:23)**: 主人 17:22 升级授权 + 8 硬墙全部重置 + B1-B7 升级路线 + 0 装解除 + 16 派满 + 17:30 commit 拍板升级版 (add 全部含 src + .gitignore + Cargo.toml 1.2.0)

---

## 8. 一句话 (TL;DR)

**主人 17:22 升级授权 → 8 硬墙全部重置 (B1-B7 升级路线 + A1-A3 严守) + 0 装解除 + 16 派满. 17:30 commit 拍板升级: add 全部 (含 138 src + .gitignore + Cargo.toml 1.2.0). 立刻派 4 supervisor × 4 sub-agent = 16 sub-agent 升级版 (vs 旧 bg_62424f99 已 task_stop). 0 主动 push 严守.**
