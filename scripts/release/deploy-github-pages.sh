#!/usr/bin/env bash
# ==============================================================================
# deploy-github-pages.sh — GitHub Pages 部署 (mkdocs build + gh-pages branch, 1.0 release 配套)
# ------------------------------------------------------------------------------
# R129-23 (sub-agent of mvs_367e66fae08342ffa399befe4f85dbac, 2026-08-11 00:34)
# Per decision-55 §2.6 + decision-58 §5 + decision-61 §3.1 + decision-62 +
#      R129-13 §3.3 GitHub Pages 部署 5 步 + 主人 8/4 23:33 Tauri 终极
# 触发: 主人 0:34 拍板"已经 done 的不能算正在跑的，正在跑的达到 16 个" →
#       cron watch-r129-era-auto-replenish-16 派 R129-17~23 7 sub-agent 补满 16 跑中
# 关联: decision-33 (8 硬墙) + decision-48 (整合 #4 commit abf12243 严守) +
#       decision-55 (R127 阶段 F 1.0 release) + decision-58 (R128-2 1.0 release Cargo) +
#       decision-61 (R129 era 派活) + decision-62 (整合 #5 拆 3 commit) +
#       R129-8 (scripts/release/ 10 文件) + R129-13 (docs/pages-source/ 7 文档 + mkdocs.yml)
#
# 作用:
#   1. 验证前置 (当前目录 + .git + Cargo.toml 1.2.0 + master HEAD = abf12243)
#   2. 一次性: pip install mkdocs mkdocs-material (check + install if not exist)
#   3. mkdocs build (生成 site/ 目录)
#   4. 创建 gh-pages orphan branch (git checkout --orphan gh-pages)
#   5. commit gh-pages + push origin gh-pages --force
#   6. 提示主人 GitHub repo Settings → Pages → 选 gh-pages branch + Folder: / (root)
#
# 用法 (Bash, Linux/macOS/WSL, 主人手跑):
#   cd REDACTED/Apeireth-rust
#   bash scripts/release/deploy-github-pages.sh
#
# 0 主动 push 严守 (per decision-33 §2.3 + decision-58 §7 + decision-61 §6 + decision-62 §9):
#   Mavis = orchestrator, 0 主动 push 0 主动 build 0 主动配 Pages
#   主人 8/11 起床后手跑本脚本 + 浏览器配 GitHub Pages 设置
#
# 0 主动 build 严守 (per R129-13 §3.2):
#   Mavis 0 主动 mkdocs build, 主人手跑 `mkdocs build` 生成 site/ 目录
#
# 8 硬墙 (per decision-33 §2.3) 0 越界:
#   B1 24 LOCKED 入口签名 0 改 (本脚本 0 触碰 crate src/)
#   B2 workspace.version 1.2.0 0 改 (本脚本 0 改 Cargo.toml)
#   A1 R11 baseline 3 值 0 改 (本脚本 0 触碰 17 baseline 文件)
#   B3-B7 + A2-A3 严守 (本脚本 0 触碰)
#   C1 0 主动 commit (本脚本仅 build + commit gh-pages, 0 碰主 master)
#   C2 0 装 PASS 严守 (本脚本 0 借具体源码, 仅 mkdocs build)
#   C3 升 6 重 v7 严守 (本脚本 0 触碰)
#   0 主动 push 严守 (本脚本由主人手跑, Mavis 0 主动)
#   0 主动 build 严守 (本脚本由主人手跑, Mavis 0 主动)
# ==============================================================================

set -euo pipefail

# === 0 主动 push 严守: 本脚本由主人手跑, 0 主动 build / 0 主动 push ===
WORKSPACE_DIR="REDACTED/Apeireth-rust"
REPO_URL="https://github.com/apeireth/apeireth-rust.git"
EXPECTED_USER="apeireth"
EXPECTED_REPO="apeireth-rust"
VERSION="1.0.0"
GH_PAGES_BRANCH="gh-pages"

# 整合 #4 commit (per decision-48, 19:41 done, 0 重跑)
EXPECTED_INTEGRATION_4="abf1224371016e36df8f4d3c9a05b33f1c563e0d"

# === Banner ===
echo ""
echo "=================================================="
echo "  Apeireth 1.0 release — GitHub Pages 部署"
echo "  源:   docs/pages-source/ (7 文档, R129-13 写)"
echo "  配置: mkdocs.yml (4133 bytes, R129-13 写)"
echo "  部署: $GH_PAGES_BRANCH branch (主人手跑)"
echo "  版本: v$VERSION"
echo "  模式: 主人手跑 (0 主动 build 严守 + 0 主动 push 严守)"
echo "=================================================="
echo ""

# === 前置检查 (per O-5 不假装) ===

# 1. 当前目录 = Apeireth-rust 主仓
ExpectedPath="Apeireth-rust"
CurrentDir=$(basename "$(pwd)")
if [ "$CurrentDir" != "$ExpectedPath" ]; then
    echo "❌ 当前目录不是 Apeireth-rust 主仓: $(pwd)"
    echo "   期望: REDACTED/Apeireth-rust/"
    echo "   cd REDACTED/Apeireth-rust/"
    exit 1
fi
echo "✓ 当前目录: $(pwd)"

# 2. .git 存在 (整合 #4 commit 19:41 后 .git 在主仓根)
if [ ! -d ".git" ]; then
    echo "❌ .git 不在主仓根"
    echo "   整合 #4 commit 19:41 后 .git 应该在 REDACTED/Apeireth-rust/.git/"
    echo "   per decision-46 (git mv done) + decision-48 (整合 #4 commit abf12243)"
    exit 1
fi
echo "✓ .git 在主仓根"

# 3. git 可执行
if ! command -v git &> /dev/null; then
    echo "❌ git 不在 PATH"
    echo "   请安装 Git: https://git-scm.com/downloads"
    exit 1
fi
echo "✓ git 可执行: $(git --version)"

# 4. python 可执行 (mkdocs 依赖)
if ! command -v python3 &> /dev/null && ! command -v python &> /dev/null; then
    echo "❌ python 不在 PATH"
    echo "   请安装 Python 3.8+: https://www.python.org/downloads/"
    exit 1
fi
PYTHON_CMD=$(command -v python3 || command -v python)
echo "✓ python 可执行: $($PYTHON_CMD --version 2>&1)"

# 5. master HEAD = abf12243 (整合 #4 commit 严守)
CurrentHead=$(git rev-parse HEAD 2>&1)
if [ "$CurrentHead" != "$EXPECTED_INTEGRATION_4" ]; then
    echo "❌ master HEAD ≠ 整合 #4 commit abf12243"
    echo "   当前 HEAD: $CurrentHead"
    echo "   期望:      $EXPECTED_INTEGRATION_4"
    echo "   整合 #4 commit 严守 (per decision-48)"
    exit 1
fi
echo "✓ master HEAD = $EXPECTED_INTEGRATION_4 (整合 #4 commit 严守)"

# 6. Cargo.toml version = 1.2.0 (B2 严守)
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Cargo.toml 不在主仓根"
    exit 1
fi
CargoVersion=$(grep -m 1 '^version' Cargo.toml | sed -E 's/^version\s*=\s*"([^"]+)".*/\1/')
if [ -z "$CargoVersion" ]; then
    echo "❌ Cargo.toml 中找不到 version 字段"
    exit 1
fi
if [ "$CargoVersion" != "1.2.0" ]; then
    echo "❌ Cargo.toml version ≠ 1.2.0 (B2 严守)"
    echo "   当前: $CargoVersion"
    echo "   期望: 1.2.0"
    exit 1
fi
echo "✓ Cargo.toml version = 1.2.0 (B2 严守)"

# 7. mkdocs.yml 存在 (R129-13 写)
if [ ! -f "mkdocs.yml" ]; then
    echo "❌ mkdocs.yml 不在主仓根"
    echo "   R129-13 已写到 REDACTED/Apeireth-rust/mkdocs.yml (4133 bytes)"
    exit 1
fi
echo "✓ mkdocs.yml 在主仓根"

# 8. docs/pages-source/ 存在 (R129-13 写 7 文档)
if [ ! -d "docs/pages-source" ]; then
    echo "❌ docs/pages-source/ 不在主仓 docs/ 下"
    echo "   R129-13 已写到 REDACTED/Apeireth-rust/docs/pages-source/ (7 markdown)"
    exit 1
fi
PageSourceCount=$(find docs/pages-source -maxdepth 1 -name "*.md" -type f | wc -l)
if [ "$PageSourceCount" -lt 7 ]; then
    echo "❌ docs/pages-source/ 下 .md 文件 < 7 (R129-13 写 7 文档)"
    echo "   实际: $PageSourceCount 个 .md 文件"
    exit 1
fi
echo "✓ docs/pages-source/ 下 $PageSourceCount 个 .md 文件 (R129-13 7 文档)"

# === 0 主动 build 严守: 检查 mkdocs 是否已安装, 缺则主人手动安装 ===
echo ""
echo "[0 主动 build 严守] 检查 mkdocs + mkdocs-material 安装状态..."
if ! command -v mkdocs &> /dev/null; then
    echo "⚠ mkdocs 未安装, 请主人手跑: pip install mkdocs mkdocs-material"
    echo "  (一次性, Mavis 0 主动, per R129-13 §3.2 0 主动 build 严守)"
    read -p "  现在安装吗? (y/n) " install
    if [ "$install" = "y" ]; then
        pip install mkdocs mkdocs-material
    else
        echo "❌ mkdocs 未安装, 主人手跑 pip install mkdocs mkdocs-material 后重试"
        exit 1
    fi
fi
echo "✓ mkdocs 已安装: $(mkdocs --version)"

# === Step 1: mkdocs build (生成 site/ 目录, 0 主动 build 严守) ===
echo ""
echo "[Step 1] mkdocs build (生成 site/ 目录)..."
echo "  源:   docs/pages-source/ (7 文档, R129-13 写)"
echo "  配置: mkdocs.yml (Material theme, 5 nav + 3 链式页)"
echo "  输出: site/ 目录 (HTML + CSS + JS + assets)"
echo ""

# 清理旧 site/ (避免脏数据)
if [ -d "site" ]; then
    echo "  清理旧 site/ 目录..."
    rm -rf site
fi

mkdocs build
echo "✓ mkdocs build done (site/ 目录生成)"

# verify site/ 目录
if [ ! -f "site/index.html" ]; then
    echo "❌ site/index.html 不存在 (mkdocs build 失败)"
    exit 1
fi
SiteSize=$(du -sk site | cut -f1)
echo "✓ site/index.html 存在, site/ 总大小 ${SiteSize} KB"

# === Step 2: 创建 gh-pages orphan branch ===
echo ""
echo "[Step 2] 创建 gh-pages orphan branch (git checkout --orphan)..."
echo "  警告: gh-pages 是 orphan branch, 跟 master 无关"
echo "  警告: 切换分支前请确认 master 上无未提交改动"
echo ""

# 检查 master 工作目录是否干净
GitStatus=$(git status --porcelain 2>&1)
if [ -n "$GitStatus" ]; then
    echo "❌ master 有未提交改动, 请先 git stash 或 git commit"
    echo "   改动:"
    echo "$GitStatus" | sed 's/^/   /'
    exit 1
fi
echo "✓ master 工作目录干净"

# 保存当前分支 (master)
CurrentBranch=$(git rev-parse --abbrev-ref HEAD)
echo "  当前分支: $CurrentBranch"

# 检查 gh-pages branch 是否已存在
if git show-ref --verify --quiet "refs/heads/$GH_PAGES_BRANCH"; then
    echo "⚠ gh-pages branch 已存在 (本地)"
    read -p "  切换到已存在的 gh-pages branch? (y/n) " useExisting
    if [ "$useExisting" = "y" ]; then
        git checkout "$GH_PAGES_BRANCH"
    else
        echo "❌ 主人拍板不切换, 退出脚本"
        exit 1
    fi
else
    # 创建 orphan branch
    git checkout --orphan "$GH_PAGES_BRANCH"
    echo "✓ gh-pages orphan branch 创建"
fi

# 清理 orphan branch 的所有文件
git rm -rf . > /dev/null 2>&1 || true
echo "✓ 清理 orphan branch 文件"

# 复制 site/ 内容到根
cp -r site/* . 2>/dev/null || cp -r site/. .
SiteFiles=$(find . -maxdepth 1 -type f | wc -l)
echo "✓ 复制 site/* 到根目录 ($SiteFiles 文件)"

# 添加并 commit
git add -A
git commit -m "GitHub Pages 1.0 release

Apeireth 1.0 release 配套 GitHub Pages 文档站:
- 源: docs/pages-source/ (7 文档, R129-13 写)
- 配置: mkdocs.yml (Material theme, R129-13 写)
- 部署: gh-pages branch (主人手跑 per deploy-github-pages.sh)

7 文档:
1. index.md (Home) - 1.0 release 介绍
2. getting-started.md (Getting Started) - 快速开始
3. api.md (API) - API 文档 (13 键 + 30 维 + 6 重 v7 + 24 LOCKED)
4. roadmap.md (Roadmap) - 1.0→2.0 路线图
5. changelog.md (Changelog) - v1.0.0 changelog
6. borrowed-repos.md (Borrowed Repos) - 借鉴 11/11 致谢
7. architecture.md (Architecture) - 8 哲学锚 + 24 LOCKED + 决策链

0 主动 push 严守: Mavis 0 主动 push, 主人手跑 git push origin gh-pages
0 主动 build 严守: Mavis 0 主动 mkdocs build, 主人手跑本脚本
整合 #4 commit abf12243 严守: master HEAD = abf12243
Cargo.toml 1.2.0 严守: B2 严守, tag 1.0.0 = semver 大版本归 0

Refs: decision-22, #33, #48, #55, #58, #61, #62 + R129-8 + R129-13 + R129-23"
echo "✓ gh-pages commit done"

# === Step 3: push gh-pages (主人手跑, 0 主动 push 严守) ===
echo ""
echo "[Step 3] git push origin gh-pages --force (主人手跑, 0 主动 push 严守)..."
echo "  警告: --force 会覆盖远程 gh-pages branch"
echo "  警告: 主人确认配好 GitHub remote 后再 push"
echo "  远程: $REPO_URL"
echo ""

read -p "  确认 push gh-pages 到 origin? (y/n) " pushConfirm
if [ "$pushConfirm" != "y" ]; then
    echo "❌ 主人不确认, 退出脚本"
    echo "   提示: 主人切换回 master 继续后续步骤"
    echo "   git checkout master"
    exit 1
fi

git push origin "$GH_PAGES_BRANCH" --force
echo "✓ git push origin gh-pages --force done"

# === Step 4: 提示主人配 GitHub Pages 设置 ===
echo ""
echo "=================================================="
echo "  GitHub Pages 部署完成, 主人手配 GitHub Pages 设置"
echo "=================================================="
echo ""
echo "  下一步: 主人浏览器打开 GitHub repo Settings → Pages"
echo "  URL: $REPO_URL"
echo ""
echo "  设置项:"
echo "    Source:        Deploy from a branch"
echo "    Branch:        $GH_PAGES_BRANCH"
echo "    Folder:        / (root)"
echo "    [Save]"
echo ""
echo "  等待 1-2 分钟 GitHub Pages 构建完, 然后 verify:"
echo "    URL: https://$EXPECTED_USER.github.io/$EXPECTED_REPO/"
echo ""
echo "  7 文档 verify:"
echo "    1. Home (index.md) - 1.0 release 介绍"
echo "    2. Getting Started (getting-started.md) - 快速开始"
echo "    3. API (api.md) - API 文档"
echo "    4. Roadmap (roadmap.md) - 1.0→2.0 路线图"
echo "    5. Architecture (architecture.md) - 8 哲学锚 + 24 LOCKED"
echo "    6. Changelog (changelog.md) - v1.0.0 changelog"
echo "    7. Borrowed Repos (borrowed-repos.md) - 借鉴 11/11 致谢"
echo ""

# === Step 5: 切换回 master 避免影响其他工作 ===
echo "[Step 5] 切换回 master 分支 (避免影响其他工作)..."
git checkout master
echo "✓ 已切换回 master"

echo ""
echo "=================================================="
echo "  🎉 GitHub Pages 部署 done (主人 verify 后)"
echo "=================================================="
echo ""
echo "  下一步:"
echo "    1. 主人浏览器配 GitHub Pages 设置 (Settings → Pages → gh-pages branch)"
echo "    2. 主人 verify https://$EXPECTED_USER.github.io/$EXPECTED_REPO/"
echo "    3. 主人发 release announcement (微信群 / Twitter / 邮件)"
echo "    4. 整合 #6+ commit 时机由 Mavis 拍板 (per decision-64 §2.2)"
echo ""
echo "  0 主动 push 严守 100% — GitHub Pages 部署流程 0 主动, 主人手跑 + 配设置"
echo ""
