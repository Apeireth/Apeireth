#!/usr/bin/env bash
# ==============================================================================
# setup-github-remote.sh — 配置 GitHub remote (1.0 release 准备, 主人手跑)
# ------------------------------------------------------------------------------
# R129-8 (sub-agent of mvs_367e66fae08342ffa399befe4f85dbac, 2026-08-11 00:08)
# Per decision-55 §2.6 + decision-58 §5 + decision-61 §3.1 + 主人 8/4 23:33
# 触发: 主人 0:03 派 R129-8 准备 1.0 release 流程
# 关联: decision-33 (8 硬墙) + decision-48 (整合 #4 commit abf12243 严守) +
#       decision-55 (R127) + decision-58 (R128-2) + decision-61 (新会话接手) +
#       decision-62 (整合 #5 拆 3 commit 拍板)
#
# 作用:
#   1. 创建 GitHub repo (主人浏览器 https://github.com/new)
#   2. 加 remote origin = https://github.com/apeireth/apeireth-rust.git
#   3. verify: git remote -v 显示 origin
#
# 用法 (Bash, Linux/macOS/WSL, 主人手跑):
#   cd REDACTED/Apeireth-rust
#   bash scripts/release/setup-github-remote.sh
#
# 0 主动 push 严守 (per decision-33 §2.3 + decision-58 §7 + decision-62 §9):
#   Mavis = orchestrator, 0 主动 push 0 主动 commit 0 主动配 remote
#   主人 8/11 起床后手跑本脚本 + 拍板 1.0 release
#
# 8 硬墙 (per decision-33 §2.3) 0 越界:
#   B1 24 LOCKED 入口签名 0 改 (本脚本 0 触碰 crate src/)
#   B2 workspace.version 1.2.0 0 改 (本脚本 0 改 Cargo.toml)
#   A1 R11 baseline 3 值 0 改 (本脚本 0 触碰 17 baseline 文件)
#   B3-B7 + A2-A3 严守 (本脚本 0 触碰)
#   C1 0 主动 commit (本脚本 0 git commit, 仅配 remote)
#   C2 0 装 PASS 严守 (本脚本 0 借具体源码)
#   C3 升 6 重 v7 严守 (本脚本 0 触碰)
#   0 主动 push 严守 (本脚本仅配 remote, 0 push, push 见 git-push-1.0.sh)
# ==============================================================================

set -euo pipefail

# === 0 主动 push 严守: 本脚本仅配 remote, 0 push ===
REPO_URL='https://github.com/apeireth/apeireth-rust.git'
EXPECTED_USER='apeireth'
EXPECTED_REPO='apeireth-rust'
VERSION='1.0.0'

# === Banner ===
echo ''
echo '=================================================='
echo "  Apeireth 1.0 release — GitHub remote 配置"
echo "  仓库:   ${REPO_URL}"
echo "  版本:   v${VERSION}"
echo "  模式:   主人手跑 (0 主动 push 严守)"
echo '=================================================='
echo ''

# === 前置检查 (per O-5 不假装) ===

# 1. 当前目录 = Apeireth-rust 主仓
if [[ ! "$(pwd)" =~ Apeireth-rust$ ]]; then
    echo "❌ 当前目录不是 Apeireth-rust 主仓: $(pwd)"
    echo "   期望: REDACTED/Apeireth-rust/  (or /mnt/c/.../Apeireth-rust/ on WSL)"
    echo "   cd REDACTED/Apeireth-rust/"
    exit 1
fi
echo "✓ 当前目录: $(pwd)"

# 2. .git 存在 (整合 #4 commit 19:41 后 .git 在主仓根)
if [[ ! -d '.git' ]]; then
    echo "❌ .git 不在主仓根"
    echo "   整合 #4 commit 19:41 后 .git 应该在 Apeireth-rust/.git/"
    echo "   per decision-46 (git mv done) + decision-48 (整合 #4 commit abf12243)"
    exit 1
fi
echo "✓ .git 在主仓根"

# 3. git 可执行
if ! command -v git >/dev/null 2>&1; then
    echo "❌ git 不在 PATH"
    echo "   安装: https://git-scm.com/download/  (or apt install git on Linux)"
    exit 1
fi
GIT_VERSION="$(git --version 2>&1 | head -1)"
echo "✓ git: ${GIT_VERSION}"

# 4. master HEAD = abf12243 (整合 #4 commit 严守, per decision-48)
MASTER_HEAD="$(cat .git/refs/heads/master 2>/dev/null | tr -d '[:space:]' || echo '')"
if [[ "${MASTER_HEAD}" != 'abf1224371016e36df8f4d3c9a05b33f1c563e0d' ]]; then
    echo "❌ master HEAD != abf12243 (整合 #4 commit 严守)"
    echo "   当前: ${MASTER_HEAD}"
    echo "   期望: abf1224371016e36df8f4d3c9a05b33f1c563e0d"
    echo "   per decision-48 (整合 #4 commit abf12243 19:41 done, 0 重跑)"
    exit 1
fi
echo "✓ master HEAD = abf12243 (整合 #4 commit 严守)"

# 5. Cargo.toml 严守 1.2.0 (B2 严守, per decision-33 §2.3)
if ! grep -qE '^version[[:space:]]*=[[:space:]]*"1\.2\.0"' Cargo.toml; then
    echo "❌ Cargo.toml version != 1.2.0 (B2 严守 0 改)"
    echo "   per decision-33 §2.3 B2 + decision-48 (整合 #4 commit abf12243 严守)"
    exit 1
fi
echo "✓ Cargo.toml version = 1.2.0 (B2 严守 0 改)"

# === 当前 remote 状态 ===
echo ''
echo '=== 当前 git remote 状态 ==='
git remote -v
ORIGIN_EXISTS=false
if git remote 2>/dev/null | grep -q '^origin$'; then
    ORIGIN_EXISTS=true
fi
echo ''

# === Step 1: 创建 GitHub repo (主人浏览器) ===
if [[ "${ORIGIN_EXISTS}" == "true" ]]; then
    echo "ℹ️  origin 已存在, 跳过 GitHub repo 创建步骤"
    EXISTING_URL="$(git remote get-url origin 2>/dev/null || echo '')"
    echo "   当前 origin: ${EXISTING_URL}"
    if [[ "${EXISTING_URL}" != "${REPO_URL}" ]]; then
        echo "⚠️  origin URL 跟期望不同: ${REPO_URL}"
        read -r -p "   是否更新? (y/N) " answer
        if [[ "${answer}" == "y" ]]; then
            git remote set-url origin "${REPO_URL}"
            echo "✓ origin URL 已更新: ${REPO_URL}"
        else
            echo "ℹ️  保留当前 origin: ${EXISTING_URL}"
        fi
    fi
else
    echo '=== Step 1: 主人创建 GitHub repo ==='
    echo ''
    echo '主人需要浏览器手动创建 GitHub repo:'
    echo ''
    echo "   1. 打开 https://github.com/new"
    echo "   2. Owner: ${EXPECTED_USER}"
    echo "   3. Repository name: ${EXPECTED_REPO}"
    echo '   4. Visibility: Public (推荐, 跟 Apache-2.0 开源一致)'
    echo '   5. ⚠️  0 勾选任何初始化 (README / .gitignore / license)'
    echo '      主仓已有 README.md / .gitignore / LICENSE, 0 让 GitHub 覆盖'
    echo '   6. 点 Create repository'
    echo ''
    read -r -p "主人创建完 GitHub repo 后按 Enter 继续 (q = 退出) " ready
    if [[ "${ready}" == "q" ]]; then
        echo "⏸  退出, 主人创建后重跑本脚本"
        exit 0
    fi
fi

# === Step 2: 加 origin remote (0 push) ===
echo ''
echo '=== Step 2: 加 origin remote (0 push) ==='
if [[ "${ORIGIN_EXISTS}" != "true" ]]; then
    git remote add origin "${REPO_URL}"
    echo "✓ origin 已加: ${REPO_URL}"
else
    echo "ℹ️  origin 已存在, 跳过 add"
fi

# === Step 3: verify remote ===
echo ''
echo '=== Step 3: verify remote ==='
echo ''
git remote -v
echo ''

VERIFY_URL="$(git remote get-url origin 2>/dev/null || echo '')"
if [[ "${VERIFY_URL}" != "${REPO_URL}" ]]; then
    echo "❌ origin URL verify 失败"
    echo "   当前: ${VERIFY_URL}"
    echo "   期望: ${REPO_URL}"
    exit 1
fi
echo "✓ origin URL verify PASS: ${VERIFY_URL}"

# === Step 4: 准备 git push 认证 ===
echo ''
echo '=== Step 4: git push 认证 (主人手配) ==='
echo ''
echo '主人需要 git push 认证, 推荐 2 选 1:'
echo ''
echo '   方式 A: GitHub CLI (推荐, 主人 8/10 跑过 Windows PowerShell 熟悉)'
echo '     1. winget install GitHub.cli  (or https://cli.github.com)'
echo "     2. gh auth login  (选 HTTPS, 浏览器认证, 0 配 SSH key)"
echo '     3. gh auth status  (verify 已认证)'
echo ''
echo '   方式 B: GitHub Personal Access Token (PAT)'
echo '     1. https://github.com/settings/tokens/new 生成 PAT (scopes: repo + workflow)'
echo '     2. git credential-manager store (Windows 凭据管理器存 PAT)'
echo '     3. 下次 push 自动用 PAT'
echo ''
echo '   方式 C: SSH key (0 推荐, 主人 0 必配)'
echo '     ssh-keygen + 贴公钥到 GitHub'
echo ''
read -r -p "主人认证配好按 Enter 继续 (q = 退出) " auth_ready
if [[ "${auth_ready}" == "q" ]]; then
    echo "⏸  退出, 主人认证后重跑本脚本"
    exit 0
fi

# === Done ===
echo ''
echo '=================================================='
echo '  GitHub remote 配置 done'
echo "  origin: ${REPO_URL}"
echo '  master HEAD: abf12243'
echo '  0 push 严守 (push 见 git-push-1.0.sh)'
echo '=================================================='
echo ''
echo '下一步: 跑 scripts/release/verify-1.0-pre-tag.sh (8 步 verify)'
echo '   8 步全 PASS → 拍板整合 #5 commit → 跑 scripts/release/git-push-1.0.sh'
echo '   整合 #5 commit done → 跑 scripts/release/tag-1.0.0.sh'
echo ''
echo '0 主动 push 严守 (per decision-33 §2.3 + decision-62 §9):'
echo '   Mavis = orchestrator, 0 push 0 commit 0 配 remote'
echo '   主人 8/11 起床后手跑本脚本 + 拍板 1.0 release'
echo ''
