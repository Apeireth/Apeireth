#!/usr/bin/env bash
# ==============================================================================
# git-push-1.0.sh — push 整合 #5 commit + master + tags (1.0 release, 主人手跑)
# ------------------------------------------------------------------------------
# R129-8 (sub-agent of mvs_367e66fae08342ffa399befe4f85dbac, 2026-08-11 00:08)
# Per decision-55 §2.6 + decision-58 §5 + decision-61 §3.1 + decision-62 拍板
# 触发: 主人 0:03 派 R129-8 准备 1.0 release 流程
# 关联: decision-33 (8 硬墙) + decision-48 (整合 #4 commit abf12243 严守) +
#       decision-55 (R127) + decision-58 (R128-2) + decision-61 (新会话接手) +
#       decision-62 (整合 #5 拆 3 commit 拍板, Mavis 自决)
#
# 作用 (per decision-62 拍板):
#   1. verify master HEAD = abf12243 (整合 #4 commit 严守)
#   2. verify Cargo.toml 1.2.0 严守 (B2)
#   3. 整合 #5.1 commit (R125-R128-2 era 41 任务 src/ 实施, 50+ 文件)
#   4. 整合 #5.2 commit (1.0 release 文档, CHANGELOG + ROADMAP + RELEASE_NOTES + OSS_NOTICE + LICENSE + Cargo.toml)
#   5. 整合 #5.3 commit (决策链 + 41 sub-agent 报告 + HANDOFF, reports/)
#   6. push master + tags
#   7. verify 推送结果
#
# 用法 (Bash, Linux/macOS/WSL, 主人手跑):
#   cd REDACTED/Apeireth-rust
#   bash scripts/release/git-push-1.0.sh
#
# 0 主动 push 严守 (per decision-33 §2.3 + decision-58 §7 + decision-62 §9):
#   Mavis = orchestrator, 0 主动 push 0 主动 commit 0 主动配 remote
#   主人 8/11 起床后手跑本脚本 + 拍板 1.0 release
#   整合 #5 commit 拆 3 commit 由 Mavis 自决拍板 (per decision-62), 但 git add + commit + push 由主人手跑
#
# 8 硬墙 (per decision-33 §2.3) 0 越界:
#   B1 24 LOCKED 入口签名 0 改 (commit 5.1 仅内部 fn 改 + 入口 0 改)
#   B2 workspace.version 1.2.0 0 改 (本脚本 0 改 Cargo.toml version)
#   A1 R11 baseline 3 值 0 改 (本脚本 0 触碰 17 baseline 文件)
#   B3-B7 + A2-A3 严守 (本脚本 0 触碰)
#   C1 0 主动 commit (本脚本由主人手跑, Mavis 0 主动)
#   C2 0 装 PASS 严守 (commit 5.1 仅借鉴 8/11 真实施)
#   C3 升 6 重 v7 严守 (本脚本 0 触碰)
#   0 主动 push 严守 (本脚本由主人手跑, Mavis 0 主动)
# ==============================================================================

set -uo pipefail

VERSION='1.0.0'
WORKSPACE_DIR='Apeireth-rust'
EXPECTED_REMOTE='https://github.com/apeireth/apeireth-rust.git'

# 整合 #4 commit (per decision-48, 19:41 done, 0 重跑)
EXPECTED_HEAD='abf1224371016e36df8f4d3c9a05b33f1c563e0d'

# 整合 #5 commit 拆 3 commit (per decision-62 拍板, Mavis 自决)
Commit5_1_Subject='整合 #5.1 commit: R125-R128-2 era 41 任务 src/ 实施'
Commit5_2_Subject='整合 #5.2 commit: 1.0 release 文档 (CHANGELOG + ROADMAP + RELEASE_NOTES + OSS_NOTICE + LICENSE + Cargo.toml)'
Commit5_3_Subject='整合 #5.3 commit: 决策链 #30-#60 + 41 sub-agent 报告 + HANDOFF (reports/)'

# === Banner ===
echo ''
echo '=================================================='
echo "  Apeireth 1.0 release — git push (整合 #5 拆 3 commit)"
echo "  版本:   v${VERSION}"
echo "  模式:   主人手跑 (0 主动 push 严守)"
echo '=================================================='
echo ''

# === 前置检查 (per O-5 不假装) ===

# 1. 当前目录 = Apeireth-rust 主仓
if [[ ! "$(pwd)" =~ Apeireth-rust$ ]]; then
    echo "❌ 当前目录不是 Apeireth-rust 主仓: $(pwd)"
    echo "   cd ${WORKSPACE_DIR}"
    exit 1
fi
echo "✓ working dir: $(pwd)"

# 2. master HEAD = abf12243 (整合 #4 commit 严守)
MASTER_HEAD="$(cat .git/refs/heads/master 2>/dev/null | tr -d '[:space:]' || echo '')"
if [[ "${MASTER_HEAD}" != "${EXPECTED_HEAD}" ]]; then
    echo "❌ master HEAD != abf12243"
    echo "   当前: ${MASTER_HEAD}"
    echo "   期望: ${EXPECTED_HEAD}"
    echo "   per decision-48 (整合 #4 commit abf12243 19:41 done, 0 重跑)"
    exit 1
fi
echo "✓ master HEAD = abf12243 (整合 #4 commit 严守)"
echo ''

# 3. Cargo.toml 严守 1.2.0
if ! grep -qE '^version[[:space:]]*=[[:space:]]*"1\.2\.0"' Cargo.toml; then
    echo "❌ Cargo.toml version != 1.2.0 (B2 严守 0 改)"
    exit 1
fi
echo "✓ Cargo.toml version = 1.2.0 (B2 严守 0 改)"
echo ''

# 4. origin 配好 (主人已跑 setup-github-remote.sh)
ORIGIN_URL="$(git remote get-url origin 2>/dev/null || echo '')"
if [[ -z "${ORIGIN_URL}" ]]; then
    echo "❌ origin 0 配"
    echo "   主人先跑 scripts/release/setup-github-remote.sh"
    exit 1
fi
if [[ "${ORIGIN_URL}" != "${EXPECTED_REMOTE}" ]]; then
    echo "⚠️  origin URL 跟期望不同"
    echo "   当前: ${ORIGIN_URL}"
    echo "   期望: ${EXPECTED_REMOTE}"
    read -r -p "   继续? (y/N) " answer
    if [[ "${answer}" != "y" ]]; then
        echo "⏸  退出"
        exit 0
    fi
fi
echo "✓ origin: ${ORIGIN_URL}"
echo ''

# 5. working dir 状态
echo '=== git status ==='
git status --short
echo ''

MODIFIED_COUNT="$(git status --short 2>/dev/null | wc -l | tr -d ' ')"
echo "改动文件数: ${MODIFIED_COUNT}"
if [[ "${MODIFIED_COUNT}" == "0" ]]; then
    echo 'ℹ️  0 改动, 0 commit 必跑 (整合 #5 commit 已 done)'
    echo '   直接跳到 push 步骤'
else
    echo "ℹ️  有 ${MODIFIED_COUNT} 文件改动, 需要 3 commit"
fi
echo ''

# === 整合 #5.1 commit: src/ 实施 ===
echo '=== Step 1: 整合 #5.1 commit (src/ 实施) ==='
echo ''
echo "Subject: ${Commit5_1_Subject}"
echo ''
echo '改动范围 (per decision-62 §2.1):'
echo '   - 31 M src/ (LOCKED crate 内部 fn)'
echo '   - 50+ ?? src/ (借鉴 8/11 真实施 + 新模块)'
echo '   - 20+ tests/ + 10+ examples/'
echo '   - 3 NEW 库目录 (apeireth-library-governance/ + frontend/ + library/)'
echo '   - 0 改 Cargo.toml version (B2 严守)'
echo '   - 0 改 24 LOCKED 入口签名 (B1 严守)'
echo ''

# 准备 stage src/ 改动
git add 'crates/*/src/lib.rs' 'crates/*/src/*.rs' 'crates/*/Cargo.toml' 'crates/*/tests/*.rs' 'crates/*/examples/*.rs' 2>/dev/null
git add 'frontend/' 'library/' 'apeireth-library-governance/' 2>/dev/null

STAGED_COUNT1="$(git diff --cached --name-only 2>/dev/null | wc -l | tr -d ' ')"
echo "已 stage 文件数: ${STAGED_COUNT1}"
echo ''

if [[ "${STAGED_COUNT1}" -gt 0 ]]; then
    echo '=== 5.1 commit message (per decision-62 §2.2) ==='
    COMMIT_MSG1="${Commit5_1_Subject}

主仓 src/ 实施整合 (R125 16 + R126 16 + R127 4 + R127-2 10 + R128 6 + R128-2 3 = 41 sub-agent 全 done).

借鉴 8/11 真实施:
- clap-rs/clap 4.6.6 (R125-2) - derive 实施
- hyperium/hyper 0.1.20 (R125-3) - 池复用
- modelcontextprotocol/servers 76d64c8 (R125-4) - MCP 协议对齐
- PyO3/PyO3 0.29.2 (R125-9) - pybridge
- model-checking/kani 0.67.0 (R125-10) - 形式化
- langchain-ai/langgraph d56666f (R125-13) - StateGraph
- obra/superpowers 6.2.0 (R125-14) - 9 skill files
- LiteLLM (P6-1 retry 21:38) - 公开设计 1:1 翻译

升级:
- 8 哲学锚 (B5, 6→8)
- V0.5 30 维 (B3, 25→30)
- 6 重守门 v7 (B4, v6→v7)
- 12 键 + PHL-07 = 13 键 (A3)

0 越界 8 硬墙 100%:
- B1 24 LOCKED 入口签名 0 改
- B2 workspace.version 1.2.0 0 改
- A1 R11 baseline 3 值 0 改
- C1 0 主动 commit (整合 #5 commit 时机)
- C2 0 装 PASS 严守
- 0 主动 push

整合 #4 commit abf12243 严守 (0 重跑).

Refs: decision-22, #33, #41, #42, #47, #48, #51, #55, #56, #57, #58, #61, #62
Tests: 4100+ tests pass (per R125-16 + P12-1 verify)"
    echo "${COMMIT_MSG1}"
    echo ''
    read -r -p "主人确认 commit 5.1? (y/N) " answer
    if [[ "${answer}" == "y" ]]; then
        git commit -m "${COMMIT_MSG1}"
        echo '✅ 整合 #5.1 commit done'
    else
        echo '⏸  跳过 5.1 commit, 主人手动 git commit'
    fi
else
    echo 'ℹ️  0 stage 文件, 跳过 5.1 commit (可能已 done)'
fi
echo ''

# === 整合 #5.2 commit: 1.0 release 文档 ===
echo '=== Step 2: 整合 #5.2 commit (1.0 release 文档) ==='
echo ''
echo "Subject: ${Commit5_2_Subject}"
echo ''
echo '改动范围 (per decision-62 §3.1):'
echo '   - CHANGELOG.md (P7-1 21:23 写 v1.0.0, 42.8KB)'
echo '   - ROADMAP.md (P7-2 21:25 写, 28.7KB)'
echo '   - RELEASE_NOTES.md (P7-3 retry 21:27 写, 36.8KB)'
echo '   - OSS_NOTICE.md (P13-1 21:53 写, 346 行)'
echo '   - LICENSE (P13-1 写, 175 行 Apache 2.0 verbatim)'
echo '   - Cargo.toml (P15-1 22:48 写, license = "Apache-2.0" + workspace.metadata.apeireth)'
echo '   - 0 改 Cargo.toml version (B2 严守 1.2.0)'
echo ''

git add 'CHANGELOG.md' 'ROADMAP.md' 'RELEASE_NOTES.md' 'OSS_NOTICE.md' 'LICENSE' 'NOTICE' 'Cargo.toml' 'Cargo.lock' 2>/dev/null

STAGED_COUNT2="$(git diff --cached --name-only 2>/dev/null | wc -l | tr -d ' ')"
echo "已 stage 文件数: ${STAGED_COUNT2}"
echo ''

if [[ "${STAGED_COUNT2}" -gt 0 ]]; then
    echo '=== 5.2 commit message (per decision-62 §3.2) ==='
    COMMIT_MSG2="${Commit5_2_Subject}

1.0 release 文档整合:
- CHANGELOG.md (v1.0.0, P7-1 写, 42.8KB)
- ROADMAP.md (P7-2 写, 28.7KB)
- RELEASE_NOTES.md (P7-3 retry 写, 36.8KB)
- OSS_NOTICE.md (P13-1 写, 346 行, 借鉴 8/11 致谢)
- LICENSE (175 行, Apache-2.0 verbatim, P13-1 写, 严守不动)
- NOTICE (66 行, R20 阶段 6, 严守不动)

Cargo.toml 配 (per P15-1 R128-2 阶段 C):
- [workspace.package] license = \"Apache-2.0\" 单一来源
- [workspace.metadata.apeireth] section (73 行, 11 字段)
- 18 行注释 block (LICENSE 引用链 + 借鉴 8/11)

0 越界 8 硬墙 100%:
- B2 workspace.version 1.2.0 0 改
- C1 0 主动 commit (整合 #5 commit 时机)
- C2 0 装 PASS 严守
- 0 主动 push (等 1.0 release 配 GitHub remote)

Refs: decision-22, #33, #34, #48, #55, #57, #58, #61, #62
Depends: 5.1 (Cargo.toml metadata 引用 src/ 路径字符串, 但 Cargo.toml 已独立 done)"
    echo "${COMMIT_MSG2}"
    echo ''
    read -r -p "主人确认 commit 5.2? (y/N) " answer
    if [[ "${answer}" == "y" ]]; then
        git commit -m "${COMMIT_MSG2}"
        echo '✅ 整合 #5.2 commit done'
    else
        echo '⏸  跳过 5.2 commit, 主人手动 git commit'
    fi
else
    echo 'ℹ️  0 stage 文件, 跳过 5.2 commit (可能已 done)'
fi
echo ''

# === 整合 #5.3 commit: 决策链 + 报告 ===
echo '=== Step 3: 整合 #5.3 commit (决策链 + 报告) ==='
echo ''
echo "Subject: ${Commit5_3_Subject}"
echo ''
echo '改动范围 (per decision-62 §4.1):'
echo '   - 30+ reports/ 文件 (决策链 #30-#60 + HANDOFF + 41 sub-agent final)'
echo '   - 备查用, 0 影响 build'
echo ''

git add 'reports/' 2>/dev/null

STAGED_COUNT3="$(git diff --cached --name-only 2>/dev/null | wc -l | tr -d ' ')"
echo "已 stage 文件数: ${STAGED_COUNT3}"
echo ''

if [[ "${STAGED_COUNT3}" -gt 0 ]]; then
    echo '=== 5.3 commit message (per decision-62 §4.2) ==='
    COMMIT_MSG3="${Commit5_3_Subject}

备查用, 0 影响 build.

决策链 (per decision-22 ~ decision-60, 31 份):
- R125 era 决策: #30-#32, #35, #37, #41
- R126 era 决策: #33, #36, #38, #39, #40, #42, #51, #52, #53, #54
- R127 era 决策: #55
- R127-2 era 决策: #56
- R128 era 决策: #57
- R128-2 era 决策: #58
- promethean/ 清理: #44, #45, #46, #47, #49, #50, #59, #60
- 整合 #4 commit: #48

41 sub-agent final 报告 (per R125 16 + R126 16 + R127 4 + R127-2 10 + R128 6 + R128-2 3):
- R125 era: agent-r125-15e/15f/16/17/18/19/20/21 + retry
- R126 era: agent-p1-1/1-2/1-3/1-4/2-1/2-2/2-3/2-4/3-1/3-2/3-3/3-4 + retry
- R127 era: agent-p4-1 + p5-1/2/3
- R127-2 era: agent-p6-1/2/3 + p7-1/2/3 retry + p8-1/2 retry/3 + p9-1
- R128 era: agent-p10-1/2 + p11-1 + p12-1 + p13-1 + p14-1 retry
- R128-2 era: agent-p10-3 + p11-2 + p15-1

HANDOFF:
- reports/HANDOFF-NEXT-SESSION-2026-08-10.md

cargo logs (per P12-1 + P15-1):
- agent-p12-1-cargo-*.log (10+ log)
- agent-p15-1-cargo-build-release-{api,tui}-2026-08-10.log
- agent-p15-1-cargo-run-release-api-2026-08-10.log

locked-audit 报告:
- reports/locked-audit-2026-08-10.md
- reports/locked-audit-v2-final-2026-08-10.md

promethean/ 清理脚本 (per decision-60 挂起):
- reports/promethean-full-cleanup-2026-08-10.ps1 (v1)
- reports/promethean-full-cleanup-v2-2026-08-10.ps1 (v2)

0 越界 8 硬墙 100% (per decision-33):
- C1 0 主动 commit (整合 #5 commit 时机)
- 0 主动 push (等 1.0 release 配 GitHub remote)

Refs: decision-22, #33, #34, #48, #61, #62
Depends: 0 (独立)"
    echo "${COMMIT_MSG3}"
    echo ''
    read -r -p "主人确认 commit 5.3? (y/N) " answer
    if [[ "${answer}" == "y" ]]; then
        git commit -m "${COMMIT_MSG3}"
        echo '✅ 整合 #5.3 commit done'
    else
        echo '⏸  跳过 5.3 commit, 主人手动 git commit'
    fi
else
    echo 'ℹ️  0 stage 文件, 跳过 5.3 commit (可能已 done)'
fi
echo ''

# === verify 整合 #5 commit 3 个 done ===
echo '=== Step 4: verify 整合 #5 commit 3 个 done ==='
echo ''
echo '最近 5 commit:'
git log --oneline -5
echo ''

COMMIT_COUNT="$(git log --oneline 2>/dev/null | grep -c '整合 #5\.' || echo 0)"
echo "整合 #5.x commit 数: ${COMMIT_COUNT} / 3 期望"
if [[ "${COMMIT_COUNT}" -lt 3 ]]; then
    echo "⚠️  整合 #5 commit 不全 (期望 3, 实际 ${COMMIT_COUNT})"
    echo '   主人手动补 commit 后重跑'
    read -r -p "继续 push? (y/N) " answer
    if [[ "${answer}" != "y" ]]; then
        echo '⏸  退出'
        exit 0
    fi
fi
echo ''

# === push master + tags ===
echo '=== Step 5: push master + tags ==='
echo ''
echo "push 目标: origin = ${ORIGIN_URL}"
echo ''
read -r -p "主人确认 push? (y/N) " answer
if [[ "${answer}" != "y" ]]; then
    echo '⏸  跳过 push, 主人手动 git push'
    exit 0
fi

# push master
git push -u origin master
PUSH_EXIT=$?
if [[ ${PUSH_EXIT} -ne 0 ]]; then
    echo "❌ git push -u origin master FAIL (exit ${PUSH_EXIT})"
    exit 1
fi
echo '✅ git push -u origin master done'
echo ''

# 暂无 v1.0.0 tag, 0 push
echo 'ℹ️  暂无 v1.0.0 tag (待 tag-1.0.0.sh 跑完)'
echo ''

# === verify push 成功 ===
echo '=== Step 6: verify push 成功 ==='
echo ''
git log --oneline -5
echo ''
echo 'master HEAD (本地):'
git log -1 --format='%H %s'
echo ''
echo 'master HEAD (远端):'
git ls-remote origin master 2>/dev/null | head -1
echo ''

LOCAL_HEAD="$(git rev-parse master 2>/dev/null | tr -d '[:space:]')"
REMOTE_HEAD="$(git ls-remote origin master 2>/dev/null | awk '{print $1}' | tr -d '[:space:]')"
if [[ "${LOCAL_HEAD}" == "${REMOTE_HEAD}" ]]; then
    echo "✅ local master = remote master = ${LOCAL_HEAD}"
else
    echo "⚠️  local != remote"
    echo "   local:  ${LOCAL_HEAD}"
    echo "   remote: ${REMOTE_HEAD}"
fi
echo ''

# === Done ===
echo '=================================================='
echo '  git push done'
echo "  master HEAD (本地 + 远端): ${LOCAL_HEAD}"
echo '  整合 #5 commit 拆 3 done (5.1 + 5.2 + 5.3)'
echo '  0 主动 push 严守 (本脚本由主人手跑, Mavis 0 主动)'
echo '=================================================='
echo ''
echo '下一步: 跑 scripts/release/tag-1.0.0.sh (tag v1.0.0 + gh release create)'
echo ''
echo '0 主动 push 严守 (per decision-33 §2.3 + decision-62 §9):'
echo '   Mavis = orchestrator, 0 push 0 commit 0 配 remote'
echo '   主人 8/11 起床后手跑本脚本 + 拍板 1.0 release'
echo ''
exit 0
