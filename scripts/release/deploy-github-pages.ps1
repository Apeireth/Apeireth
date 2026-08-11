# ==============================================================================
# deploy-github-pages.ps1 — GitHub Pages 部署 (mkdocs build + gh-pages branch, 1.0 release 配套)
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
# 用法 (PowerShell, Windows 优先, 主人手跑):
#   cd Apeireth-rust
#   .\scripts\release\deploy-github-pages.ps1
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

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

# === 0 主动 push 严守: 本脚本由主人手跑, 0 主动 build / 0 主动 push ===
$WORKSPACE_DIR = 'Apeireth-rust'
$REPO_URL = 'https://github.com/apeireth/apeireth-rust.git'
$EXPECTED_USER = 'apeireth'
$EXPECTED_REPO = 'apeireth-rust'
$VERSION = '1.0.0'
$GH_PAGES_BRANCH = 'gh-pages'

# 整合 #4 commit (per decision-48, 19:41 done, 0 重跑)
$EXPECTED_INTEGRATION_4 = 'abf1224371016e36df8f4d3c9a05b33f1c563e0d'

# === Banner ===
Write-Host ''
Write-Host '==================================================' -ForegroundColor Cyan
Write-Host "  Apeireth 1.0 release — GitHub Pages 部署" -ForegroundColor Cyan
Write-Host "  源:   docs/pages-source/ (7 文档, R129-13 写)" -ForegroundColor Cyan
Write-Host "  配置: mkdocs.yml (4133 bytes, R129-13 写)" -ForegroundColor Cyan
Write-Host "  部署: $GH_PAGES_BRANCH branch (主人手跑)" -ForegroundColor Cyan
Write-Host "  版本: v$VERSION" -ForegroundColor Cyan
Write-Host "  模式: 主人手跑 (0 主动 build 严守 + 0 主动 push 严守)" -ForegroundColor Cyan
Write-Host '==================================================' -ForegroundColor Cyan
Write-Host ''

# === 前置检查 (per O-5 不假装) ===

# 1. 当前目录 = Apeireth-rust 主仓
$ExpectedPath = 'Apeireth-rust'
if (-not (Get-Location).Path.EndsWith($ExpectedPath)) {
    Write-Host "❌ 当前目录不是 Apeireth-rust 主仓: $(Get-Location)" -ForegroundColor Red
    Write-Host "   期望: Apeireth-rust\" -ForegroundColor Red
    Write-Host "   cd Apeireth-rust\" -ForegroundColor Red
    exit 1
}
Write-Host "✓ 当前目录: $(Get-Location)" -ForegroundColor Green

# 2. .git 存在 (整合 #4 commit 19:41 后 .git 在主仓根)
if (-not (Test-Path '.git')) {
    Write-Host "❌ .git 不在主仓根" -ForegroundColor Red
    Write-Host "   整合 #4 commit 19:41 后 .git 应该在 Apeireth-rust\.git\" -ForegroundColor Red
    Write-Host "   per decision-46 (git mv done) + decision-48 (整合 #4 commit abf12243)" -ForegroundColor Red
    exit 1
}
Write-Host "✓ .git 在主仓根" -ForegroundColor Green

# 3. git 可执行
try {
    $null = git --version 2>&1
    if ($LASTEXITCODE -ne 0) { throw 'git not found' }
} catch {
    Write-Host "❌ git 不在 PATH" -ForegroundColor Red
    Write-Host "   请安装 Git for Windows: https://git-scm.com/download/win" -ForegroundColor Red
    exit 1
}
Write-Host "✓ git 可执行" -ForegroundColor Green

# 4. python 可执行 (mkdocs 依赖)
try {
    $null = python --version 2>&1
    if ($LASTEXITCODE -ne 0) { throw 'python not found' }
} catch {
    Write-Host "❌ python 不在 PATH" -ForegroundColor Red
    Write-Host "   请安装 Python 3.8+: https://www.python.org/downloads/" -ForegroundColor Red
    exit 1
}
Write-Host "✓ python 可执行" -ForegroundColor Green

# 5. master HEAD = abf12243 (整合 #4 commit 严守)
$CurrentHead = git rev-parse HEAD 2>&1
if ($CurrentHead -ne $EXPECTED_INTEGRATION_4) {
    Write-Host "❌ master HEAD ≠ 整合 #4 commit abf12243" -ForegroundColor Red
    Write-Host "   当前 HEAD: $CurrentHead" -ForegroundColor Red
    Write-Host "   期望:      $EXPECTED_INTEGRATION_4" -ForegroundColor Red
    Write-Host "   整合 #4 commit 严守 (per decision-48)" -ForegroundColor Red
    exit 1
}
Write-Host "✓ master HEAD = $EXPECTED_INTEGRATION_4 (整合 #4 commit 严守)" -ForegroundColor Green

# 6. Cargo.toml version = 1.2.0 (B2 严守)
$CargoVersion = Select-String -Path 'Cargo.toml' -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
if ($null -eq $CargoVersion) {
    Write-Host "❌ Cargo.toml 中找不到 version 字段" -ForegroundColor Red
    exit 1
}
$VersionValue = ($CargoVersion -match 'version\s*=\s*"([^"]+)"') | Out-Null; $Matches[1]
if ($VersionValue -ne '1.2.0') {
    Write-Host "❌ Cargo.toml version ≠ 1.2.0 (B2 严守)" -ForegroundColor Red
    Write-Host "   当前: $VersionValue" -ForegroundColor Red
    Write-Host "   期望: 1.2.0" -ForegroundColor Red
    exit 1
}
Write-Host "✓ Cargo.toml version = 1.2.0 (B2 严守)" -ForegroundColor Green

# 7. mkdocs.yml 存在 (R129-13 写)
if (-not (Test-Path 'mkdocs.yml')) {
    Write-Host "❌ mkdocs.yml 不在主仓根" -ForegroundColor Red
    Write-Host "   R129-13 已写到 Apeireth-rust\mkdocs.yml (4133 bytes)" -ForegroundColor Red
    exit 1
}
Write-Host "✓ mkdocs.yml 在主仓根" -ForegroundColor Green

# 8. docs/pages-source/ 存在 (R129-13 写 7 文档)
if (-not (Test-Path 'docs/pages-source')) {
    Write-Host "❌ docs/pages-source/ 不在主仓 docs/ 下" -ForegroundColor Red
    Write-Host "   R129-13 已写到 Apeireth-rust\docs\pages-source\ (7 markdown)" -ForegroundColor Red
    exit 1
}
$PageSourceFiles = Get-ChildItem 'docs/pages-source' -Filter '*.md' | Measure-Object
if ($PageSourceFiles.Count -lt 7) {
    Write-Host "❌ docs/pages-source/ 下 .md 文件 < 7 (R129-13 写 7 文档)" -ForegroundColor Red
    Write-Host "   实际: $($PageSourceFiles.Count) 个 .md 文件" -ForegroundColor Red
    exit 1
}
Write-Host "✓ docs/pages-source/ 下 $($PageSourceFiles.Count) 个 .md 文件 (R129-13 7 文档)" -ForegroundColor Green

# === 0 主动 build 严守: 检查 mkdocs 是否已安装, 缺则主人手动安装 ===
Write-Host ''
Write-Host '[0 主动 build 严守] 检查 mkdocs + mkdocs-material 安装状态...' -ForegroundColor Cyan
try {
    $null = mkdocs --version 2>&1
    if ($LASTEXITCODE -ne 0) { throw 'mkdocs not found' }
    Write-Host "✓ mkdocs 已安装: $(mkdocs --version)" -ForegroundColor Green
} catch {
    Write-Host "⚠ mkdocs 未安装, 请主人手跑: pip install mkdocs mkdocs-material" -ForegroundColor Yellow
    Write-Host "  (一次性, Mavis 0 主动, per R129-13 §3.2 0 主动 build 严守)" -ForegroundColor Yellow
    $install = Read-Host "  现在安装吗? (y/n)"
    if ($install -eq 'y') {
        pip install mkdocs mkdocs-material
    } else {
        Write-Host "❌ mkdocs 未安装, 主人手跑 pip install mkdocs mkdocs-material 后重试" -ForegroundColor Red
        exit 1
    }
}

# === Step 1: mkdocs build (生成 site/ 目录, 0 主动 build 严守) ===
Write-Host ''
Write-Host '[Step 1] mkdocs build (生成 site/ 目录)...' -ForegroundColor Cyan
Write-Host "  源:   docs/pages-source/ (7 文档, R129-13 写)" -ForegroundColor Gray
Write-Host "  配置: mkdocs.yml (Material theme, 5 nav + 3 链式页)" -ForegroundColor Gray
Write-Host "  输出: site/ 目录 (HTML + CSS + JS + assets)" -ForegroundColor Gray
Write-Host ''

# 清理旧 site/ (避免脏数据)
if (Test-Path 'site') {
    Write-Host "  清理旧 site/ 目录..." -ForegroundColor Yellow
    Remove-Item -Recurse -Force 'site'
}

mkdocs build
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ mkdocs build 失败" -ForegroundColor Red
    exit 1
}
Write-Host "✓ mkdocs build done (site/ 目录生成)" -ForegroundColor Green

# verify site/ 目录
if (-not (Test-Path 'site/index.html')) {
    Write-Host "❌ site/index.html 不存在 (mkdocs build 失败)" -ForegroundColor Red
    exit 1
}
$SiteSize = (Get-ChildItem 'site' -Recurse | Measure-Object -Property Length -Sum).Sum
Write-Host "✓ site/index.html 存在, site/ 总大小 $([math]::Round($SiteSize / 1KB, 1)) KB" -ForegroundColor Green

# === Step 2: 创建 gh-pages orphan branch ===
Write-Host ''
Write-Host '[Step 2] 创建 gh-pages orphan branch (git checkout --orphan)...' -ForegroundColor Cyan
Write-Host "  警告: gh-pages 是 orphan branch, 跟 master 无关" -ForegroundColor Yellow
Write-Host "  警告: 切换分支前请确认 master 上无未提交改动" -ForegroundColor Yellow
Write-Host ''

# 检查 master 工作目录是否干净
$GitStatus = git status --porcelain 2>&1
if ($GitStatus) {
    Write-Host "❌ master 有未提交改动, 请先 git stash 或 git commit" -ForegroundColor Red
    Write-Host "   改动:" -ForegroundColor Red
    $GitStatus | ForEach-Object { Write-Host "   $_" -ForegroundColor Red }
    exit 1
}
Write-Host "✓ master 工作目录干净" -ForegroundColor Green

# 保存当前分支 (master)
$CurrentBranch = git rev-parse --abbrev-ref HEAD
Write-Host "  当前分支: $CurrentBranch" -ForegroundColor Gray

# 检查 gh-pages branch 是否已存在
$ExistingBranches = git branch --list $GH_PAGES_BRANCH
if ($ExistingBranches) {
    Write-Host "⚠ gh-pages branch 已存在: $ExistingBranches" -ForegroundColor Yellow
    $useExisting = Read-Host "  切换到已存在的 gh-pages branch? (y/n)"
    if ($useExisting -eq 'y') {
        git checkout $GH_PAGES_BRANCH
    } else {
        Write-Host "❌ 主人拍板不切换, 退出脚本" -ForegroundColor Red
        exit 1
    }
} else {
    # 创建 orphan branch
    git checkout --orphan $GH_PAGES_BRANCH
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ git checkout --orphan gh-pages 失败" -ForegroundColor Red
        exit 1
    }
    Write-Host "✓ gh-pages orphan branch 创建" -ForegroundColor Green
}

# 清理 orphan branch 的所有文件
git rm -rf . 2>&1 | Out-Null
Write-Host "✓ 清理 orphan branch 文件" -ForegroundColor Green

# 复制 site/ 内容到根
Copy-Item -Path 'site\*' -Destination '.' -Recurse -Force
$SiteFiles = Get-ChildItem '.' -Recurse | Measure-Object
Write-Host "✓ 复制 site/* 到根目录 ($($SiteFiles.Count) 文件)" -ForegroundColor Green

# 添加并 commit
git add -A
git commit -m "GitHub Pages 1.0 release

Apeireth 1.0 release 配套 GitHub Pages 文档站:
- 源: docs/pages-source/ (7 文档, R129-13 写)
- 配置: mkdocs.yml (Material theme, R129-13 写)
- 部署: gh-pages branch (主人手跑 per deploy-github-pages.ps1)

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
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ git commit 失败" -ForegroundColor Red
    exit 1
}
Write-Host "✓ gh-pages commit done" -ForegroundColor Green

# === Step 3: push gh-pages (主人手跑, 0 主动 push 严守) ===
Write-Host ''
Write-Host '[Step 3] git push origin gh-pages --force (主人手跑, 0 主动 push 严守)...' -ForegroundColor Cyan
Write-Host "  警告: --force 会覆盖远程 gh-pages branch" -ForegroundColor Yellow
Write-Host "  警告: 主人确认配好 GitHub remote 后再 push" -ForegroundColor Yellow
Write-Host "  远程: $REPO_URL" -ForegroundColor Gray
Write-Host ''

$pushConfirm = Read-Host "  确认 push gh-pages 到 origin? (y/n)"
if ($pushConfirm -ne 'y') {
    Write-Host "❌ 主人不确认, 退出脚本" -ForegroundColor Red
    Write-Host "   提示: 主人切换回 master 继续后续步骤" -ForegroundColor Yellow
    Write-Host "   git checkout master" -ForegroundColor Yellow
    exit 1
}

git push origin $GH_PAGES_BRANCH --force
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ git push origin gh-pages 失败" -ForegroundColor Red
    Write-Host "   检查: git remote -v 是否配置 origin (per setup-github-remote.ps1)" -ForegroundColor Red
    Write-Host "   检查: gh auth login 或 PAT 认证是否配置" -ForegroundColor Red
    exit 1
}
Write-Host "✓ git push origin gh-pages --force done" -ForegroundColor Green

# === Step 4: 提示主人配 GitHub Pages 设置 ===
Write-Host ''
Write-Host '==================================================' -ForegroundColor Magenta
Write-Host '  GitHub Pages 部署完成, 主人手配 GitHub Pages 设置' -ForegroundColor Magenta
Write-Host '==================================================' -ForegroundColor Magenta
Write-Host ''
Write-Host '  下一步: 主人浏览器打开 GitHub repo Settings → Pages' -ForegroundColor Cyan
Write-Host "  URL: $REPO_URL" -ForegroundColor Gray
Write-Host ''
Write-Host '  设置项:' -ForegroundColor Cyan
Write-Host "    Source:        Deploy from a branch" -ForegroundColor White
Write-Host "    Branch:        $GH_PAGES_BRANCH" -ForegroundColor White
Write-Host "    Folder:        / (root)" -ForegroundColor White
Write-Host "    [Save]" -ForegroundColor White
Write-Host ''
Write-Host '  等待 1-2 分钟 GitHub Pages 构建完, 然后 verify:' -ForegroundColor Cyan
Write-Host "    URL: https://$EXPECTED_USER.github.io/$EXPECTED_REPO/" -ForegroundColor White
Write-Host ''
Write-Host '  7 文档 verify:' -ForegroundColor Cyan
Write-Host '    1. Home (index.md) - 1.0 release 介绍' -ForegroundColor White
Write-Host '    2. Getting Started (getting-started.md) - 快速开始' -ForegroundColor White
Write-Host '    3. API (api.md) - API 文档' -ForegroundColor White
Write-Host '    4. Roadmap (roadmap.md) - 1.0→2.0 路线图' -ForegroundColor White
Write-Host '    5. Architecture (architecture.md) - 8 哲学锚 + 24 LOCKED' -ForegroundColor White
Write-Host '    6. Changelog (changelog.md) - v1.0.0 changelog' -ForegroundColor White
Write-Host '    7. Borrowed Repos (borrowed-repos.md) - 借鉴 11/11 致谢' -ForegroundColor White
Write-Host ''

# === Step 5: 切换回 master 避免影响其他工作 ===
Write-Host '[Step 5] 切换回 master 分支 (避免影响其他工作)...' -ForegroundColor Cyan
git checkout master
if ($LASTEXITCODE -ne 0) {
    Write-Host "⚠ git checkout master 失败, 主人手跑: git checkout master" -ForegroundColor Yellow
} else {
    Write-Host "✓ 已切换回 master" -ForegroundColor Green
}

Write-Host ''
Write-Host '==================================================' -ForegroundColor Green
Write-Host "  🎉 GitHub Pages 部署 done (主人 verify 后)" -ForegroundColor Green
Write-Host '==================================================' -ForegroundColor Green
Write-Host ''
Write-Host '  下一步:' -ForegroundColor Cyan
Write-Host '    1. 主人浏览器配 GitHub Pages 设置 (Settings → Pages → gh-pages branch)' -ForegroundColor White
Write-Host "    2. 主人 verify https://$EXPECTED_USER.github.io/$EXPECTED_REPO/" -ForegroundColor White
Write-Host '    3. 主人发 release announcement (微信群 / Twitter / 邮件)' -ForegroundColor White
Write-Host '    4. 整合 #6+ commit 时机由 Mavis 拍板 (per decision-64 §2.2)' -ForegroundColor White
Write-Host ''
Write-Host '  0 主动 push 严守 100% — GitHub Pages 部署流程 0 主动, 主人手跑 + 配设置' -ForegroundColor Magenta
Write-Host ''
