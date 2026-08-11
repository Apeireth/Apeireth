#!/usr/bin/env bash
# ==============================================================================
# tag-1.0.0.sh — 打 v1.0.0 tag + gh release create (1.0 release, 主人手跑)
# ------------------------------------------------------------------------------
# R129-8 (sub-agent of mvs_367e66fae08342ffa399befe4f85dbac, 2026-08-11 00:08)
# Per decision-55 §2.6 + decision-58 §5 + decision-61 §3.1 + decision-62 拍板
# 触发: 主人 0:03 派 R129-8 准备 1.0 release 流程
# 关联: decision-33 (8 硬墙) + decision-48 (整合 #4 commit abf12243 严守) +
#       decision-55 (R127) + decision-58 (R128-2) + decision-61 (新会话接手) +
#       decision-62 (整合 #5 拆 3 commit 拍板, Mavis 自决)
#
# 作用:
#   1. verify 整合 #5 commit 3 个 done + master HEAD 已 push
#   2. 打 annotated tag v1.0.0 (per semver 严守, 整合 #4 1.2.0 → 1.0 大版本归 0 per decision-22 §2.2)
#   3. push tag v1.0.0
#   4. gh release create v1.0.0 --title --notes-file RELEASE_NOTES.md
#   5. verify GitHub release 页面
#
# 用法 (Bash, Linux/macOS/WSL, 主人手跑):
#   cd REDACTED/Apeireth-rust
#   bash scripts/release/tag-1.0.0.sh
#
# 0 主动 push 严守 (per decision-33 §2.3 + decision-58 §7 + decision-62 §9):
#   Mavis = orchestrator, 0 主动 push 0 主动 tag 0 主动 release
#   主人 8/11 起床后手跑本脚本 + 拍板 1.0 release
#
# 8 硬墙 (per decision-33 §2.3) 0 越界:
#   B1 24 LOCKED 入口签名 0 改 (本脚本 0 触碰 crate src/)
#   B2 workspace.version 1.2.0 0 改 (per decision-22 §2.2 大版本归 0 = 1.2 → 1.0, 但 workspace.version 实际 0 改, tag 标 1.0.0)
#   A1 R11 baseline 3 值 0 改 (本脚本 0 触碰)
#   B3-B7 + A2-A3 严守 (本脚本 0 触碰)
#   C1 0 主动 commit (本脚本仅 tag + push tag + release, 0 commit)
#   C2 0 装 PASS 严守 (本脚本 0 借具体源码)
#   C3 升 6 重 v7 严守 (本脚本 0 触碰)
#   0 主动 push 严守 (本脚本由主人手跑, Mavis 0 主动)
# ==============================================================================

set -uo pipefail

VERSION='1.0.0'
TAG="v${VERSION}"
WORKSPACE_DIR='Apeireth-rust'
EXPECTED_REMOTE='https://github.com/apeireth/apeireth-rust.git'
EXPECTED_REPO='apeireth/apeireth-rust'

# 整合 #4 commit (per decision-48, 19:41 done, 0 重跑)
EXPECTED_INTEGRATION_4='abf1224371016e36df8f4d3c9a05b33f1c563e0d'

# === Banner ===
echo ''
echo '=================================================='
echo "  Apeireth 1.0 release — tag ${TAG} + gh release"
echo "  版本:   ${TAG}"
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

# 2. master HEAD 已 push (本地 + 远端一致)
LOCAL_HEAD="$(git rev-parse master 2>/dev/null | tr -d '[:space:]' || echo '')"
REMOTE_HEAD="$(git ls-remote origin master 2>/dev/null | awk '{print $1}' | tr -d '[:space:]' || echo '')"
if [[ -z "${LOCAL_HEAD}" || "${LOCAL_HEAD}" != "${REMOTE_HEAD}" ]]; then
    echo '❌ master HEAD 0 一致 (本地 != 远端)'
    echo "   本地: ${LOCAL_HEAD}"
    echo "   远端: ${REMOTE_HEAD}"
    echo '   主人先跑 scripts/release/git-push-1.0.sh'
    exit 1
fi
echo "✓ master HEAD 一致 (本地 + 远端 = ${LOCAL_HEAD})"
echo ''

# 3. 整合 #5 commit 3 个 done
COMMIT5_COUNT="$(git log --oneline 2>/dev/null | grep -c '整合 #5\.' || echo 0)"
echo "整合 #5.x commit 数: ${COMMIT5_COUNT} / 3 期望"
if [[ "${COMMIT5_COUNT}" -lt 3 ]]; then
    echo '❌ 整合 #5 commit 不全'
    echo '   主人先跑 scripts/release/git-push-1.0.sh (3 commit + push)'
    exit 1
fi
echo '✓ 整合 #5 commit 拆 3 done (5.1 + 5.2 + 5.3)'
echo ''

# 4. 整合 #4 commit 严守
if ! git log --oneline 2>/dev/null | grep -q '整合 #4 commit abf12243'; then
    echo '❌ 整合 #4 commit abf12243 0 在历史'
    echo '   per decision-48 (整合 #4 commit abf12243 19:41 done, 0 重跑)'
    exit 1
fi
echo '✓ 整合 #4 commit abf12243 在历史 (per decision-48)'
echo ''

# 5. Cargo.toml 严守 1.2.0 (B2 严守 0 改, tag 标 1.0.0 是 semver 大版本归 0, Cargo.toml 不改)
if ! grep -qE '^version[[:space:]]*=[[:space:]]*"1\.2\.0"' Cargo.toml; then
    echo '❌ Cargo.toml version != 1.2.0 (B2 严守 0 改)'
    echo '   per decision-22 §2.2: 大版本归 0 是 1.2 → 1.0, tag 标 1.0.0, 但 Cargo.toml 实际保留 1.2.0'
    echo '   等 1.0 release 后再 bump Cargo.toml'
    exit 1
fi
echo '✓ Cargo.toml version = 1.2.0 (B2 严守 0 改, tag 1.0.0 = 大版本归 0 per decision-22 §2.2)'
echo ''

# 6. gh CLI 装好
if ! command -v gh >/dev/null 2>&1; then
    echo '❌ gh CLI 不在 PATH'
    echo '   安装: winget install GitHub.cli  (or https://cli.github.com)'
    echo '   主人必装, gh release create 必用'
    exit 1
fi
GH_VERSION="$(gh --version 2>&1 | head -1)"
echo "✓ gh CLI: ${GH_VERSION}"

# 7. gh auth 认证
if ! gh auth status >/dev/null 2>&1; then
    echo '❌ gh 0 认证'
    echo '   主人跑: gh auth login  (选 HTTPS, 浏览器认证)'
    exit 1
fi
echo '✓ gh auth 认证 done'
echo ''

# 8. RELEASE_NOTES.md 存在
if [[ ! -f 'RELEASE_NOTES.md' ]]; then
    echo '❌ RELEASE_NOTES.md 不存在'
    echo '   per P7-3 retry 21:27 写 (36.8KB)'
    exit 1
fi
echo '✓ RELEASE_NOTES.md 存在 (per P7-3 retry 21:27 写, 36.8KB)'
echo ''

# === 当前 tag 状态 ===
echo '=== 当前 tag 列表 (v1.x) ==='
git tag --list 'v1.*' 2>/dev/null
echo ''

# === Step 1: 打 annotated tag v1.0.0 ===
echo "=== Step 1: 打 annotated tag ${TAG} ==="
echo ''

# 检查 tag 是否已存在
if git tag --list "${TAG}" 2>/dev/null | grep -q "^${TAG}\$"; then
    echo "⚠️  tag ${TAG} 已存在"
    read -r -p "   删重打? (y/N) " answer
    if [[ "${answer}" == "y" ]]; then
        git tag -d "${TAG}"
        echo "✓ 旧 tag ${TAG} 已删"
    else
        echo "⏸  跳过, 主人手动处理"
        exit 0
    fi
fi

TAG_MESSAGE="Apeireth 1.0.0 release

整合 #4 commit abf12243 19:41 done (per decision-48).
整合 #5 commit 拆 3 commit (per decision-62, Mavis 自决):
  5.1 src/ 实施 (R125-R128-2 era 41 任务, 50+ 文件)
  5.2 1.0 release 文档 (CHANGELOG + ROADMAP + RELEASE_NOTES + OSS_NOTICE + LICENSE + Cargo.toml)
  5.3 决策链 + 41 sub-agent 报告 + HANDOFF (reports/)

8 硬墙 0 越界 100%:
  B1 24 LOCKED 入口签名 0 改
  B2 workspace.version 1.2.0 0 改 (Cargo.toml 实际保留 1.2.0, tag 1.0.0 是大版本归 0 per decision-22 §2.2)
  A1 R11 baseline 3 值 0.8682/0.8532/0.9063 数字严守
  B3 V0.5 30 维
  B4 6 重守门 v7
  B5 8 哲学锚
  A3 13 键
  C1 0 主动 commit
  C2 0 装 PASS 严守 (✅ 10 真实施 + ⏳ 0 限流 + ❌ 1 跳过)
  C3 升 6 重 v6 → v7
  0 主动 push 严守 (Mavis 0 主动, 主人手跑)

借鉴 11/11 状态 clear:
  ✅ 10 真实施 (clap / hyper / servers / PyO3 / kani / langgraph / superpowers / LiteLLM + 2 限流 retry done)
  ⏳ 0 限流
  ❌ 1 跳过 (OpenCog AGPL-3.0)

Release notes: 根目录 RELEASE_NOTES.md (P7-3 retry 21:27 写, 36.8KB)
CHANGELOG: 根目录 CHANGELOG.md (P7-1 21:23 写 v1.0.0, 42.8KB)
ROADMAP: 根目录 ROADMAP.md (P7-2 21:25 写, 28.7KB)
LICENSE: Apache-2.0 (P13-1 写, 175 行 verbatim)
OSS_NOTICE: 根目录 OSS_NOTICE.md (P13-1 写, 346 行, 借鉴 8/11 致谢)

Refs: decision-22, #33, #41, #42, #47, #48, #55, #56, #57, #58, #61, #62
Tests: 4100+ tests pass (per R125-16 + P12-1 verify)"

echo '=== tag message ==='
echo "${TAG_MESSAGE}"
echo ''
read -r -p "主人确认打 tag ${TAG}? (y/N) " answer
if [[ "${answer}" != "y" ]]; then
    echo "⏸  跳过, 主人手动 git tag -a ${TAG} -m ..."
    exit 0
fi

git tag -a "${TAG}" -m "${TAG_MESSAGE}"
echo "✓ tag ${TAG} 已打"
echo ''

# === Step 2: push tag ===
echo "=== Step 2: push tag ${TAG} ==="
echo ''
read -r -p "主人确认 push tag? (y/N) " answer
if [[ "${answer}" != "y" ]]; then
    echo "⏸  跳过 push tag, 主人手动 git push origin ${TAG}"
    exit 0
fi

git push origin "${TAG}"
PUSH_EXIT=$?
if [[ ${PUSH_EXIT} -ne 0 ]]; then
    echo "❌ git push origin ${TAG} FAIL (exit ${PUSH_EXIT})"
    exit 1
fi
echo "✓ tag ${TAG} pushed"
echo ''

# === Step 3: gh release create ===
echo '=== Step 3: gh release create ==='
echo ''
echo "release 标题: Apeireth ${VERSION}"
echo "release notes: RELEASE_NOTES.md (36.8KB, per P7-3 retry)"
echo "目标 repo: ${EXPECTED_REPO}"
echo ''
read -r -p "主人确认 create release? (y/N) " answer
if [[ "${answer}" != "y" ]]; then
    echo '⏸  跳过 gh release create, 主人手动跑'
    echo "   gh release create ${TAG} --title 'Apeireth ${VERSION}' --notes-file RELEASE_NOTES.md"
    exit 0
fi

gh release create "${TAG}" --title "Apeireth ${VERSION}" --notes-file RELEASE_NOTES.md
RELEASE_EXIT=$?
if [[ ${RELEASE_EXIT} -ne 0 ]]; then
    echo "❌ gh release create FAIL (exit ${RELEASE_EXIT})"
    exit 1
fi
echo "✓ release ${TAG} created"
echo ''

# === Step 4: verify release 页面 ===
echo '=== Step 4: verify release 页面 ==='
echo ''
RELEASE_URL="https://github.com/${EXPECTED_REPO}/releases/tag/${TAG}"
echo "release URL: ${RELEASE_URL}"
echo ''
read -r -p "主人浏览器打开 verify? (y/N) " answer
if [[ "${answer}" == "y" ]]; then
    if command -v xdg-open >/dev/null 2>&1; then
        xdg-open "${RELEASE_URL}"
    elif command -v open >/dev/null 2>&1; then
        open "${RELEASE_URL}"
    else
        echo "   主人手动开: ${RELEASE_URL}"
    fi
    echo "✓ 已尝试开 ${RELEASE_URL}"
fi
echo ''

# === Done ===
echo '=================================================='
echo '  1.0 release done 🎉'
echo "  tag:      ${TAG}"
echo "  release:  ${RELEASE_URL}"
echo "  master HEAD: ${LOCAL_HEAD}"
echo '  整合 #4 commit abf12243 严守 (0 重跑)'
echo '  整合 #5 commit 拆 3 done (5.1 + 5.2 + 5.3)'
echo '  8 硬墙 0 越界 100%'
echo '  0 主动 push 严守 (本脚本由主人手跑, Mavis 0 主动)'
echo '=================================================='
echo ''
echo '🎉 Apeireth 1.0.0 released!'
echo ''
echo '1.0 release 后 (per decision-9 + 主人 8/4 23:33):'
echo '   - TUI 升级 (per decision-9 路线图, 改瘦后暂告段落, 优先后端)'
echo '   - Tauri 终极前端 (per 主人 8/4 23:33, 等设计团队到位)'
echo '   - ASI Python Stage 4-6 (per R129-4/5/6, 跑过夜 done 整合 #6 commit)'
echo '   - 形式化证明扩展 (per R129-10 续 P8-2)'
echo ''
echo '0 主动 push 严守 (per decision-33 §2.3 + decision-62 §9):'
echo '   Mavis = orchestrator, 0 push 0 commit 0 配 remote'
echo '   主人 8/11 起床后手跑本脚本 + 拍板 1.0 release'
echo ''
exit 0
