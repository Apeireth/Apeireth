# 中性表述纪律扫描器
# 用途：本战役所有新改动的表述纪律自动过检——新增行中不得出现任何第三方
# 产品/项目标识或"移植/借鉴/参照实现"类来源措辞。只扫 git diff 的新增行
# （'+' 开头）与未跟踪新文件全文，历史存量行不误伤；产物与归档目录不参与。
# 用法：pwsh -File scripts/check-neutral-terms.ps1 （powershell 5.1 需文件带 BOM）
# 退出码：0 = 干净；1 = 有命中（命中行会打印出来）。

$ErrorActionPreference = 'Stop'

# 禁止出现的模式（大小写不敏感）：第三方产品/项目标识 + 来源/沿革类措辞
$forbidden = @(
  '\bDSH\b',
  'dsh-',
  'DeepSeek\s*Harness',
  'DeepSeekHarness',
  '移植自',
  '借鉴自',
  '参照实现',
  '原实现',
  '1:1\s*翻译',
  'donor'
)

# 许可搬运线补强（2026-10-18，宪章〇之二）：第三方产品/项目标识覆盖到搬运源身份——
# 代码/注释/散文零该类标识，署名只走「许可署名头」豁免（见下）。
$forbidden += @(
  'photocraft',
  'storytold',
  'ArtCraft'
)

# 许可署名头豁免（2026-10-18，宪章〇之二「许可署名头」类，法律义务 > 行文风格）：
# **只豁免版权/许可/商标声明行本身**（Copyright / SPDX / 许可文本 / 商标声明 /
# 版权主体行），不豁免任何叙述、命名或注释行。理由：搬运文件的版权头与 NOTICE
# 落档必须原样保留、改动必须声明，这是 Apache-2.0 §4 等许可条款的硬义务。
$licenseRe = 'Copyright \(c\)|Copyright ©|SPDX-License-Identifier|Licensed under|Permission is hereby granted|WITHOUT WARRANTY|Apache License, Version 2\.0|at your option|MIT OR Apache-2\.0|Apache-2\.0 OR MIT|MIT License|Redistribution and use in source|trademarks? of|Open Font License|SCOWL license|版权主体|版权所有|许可协议'
$licensed = 0

# 结构性豁免：命中若仅来自这些**真实字符串**（真实路径/真实文件名/本表自身）
# 则按"路径保持准确"规则保留、不计违规；豁免计数随输出给出便于审计。
# ⚠️ `legacy/donor` 后面跟 `(/|\b)` 而不是只跟 `/`：真实路径**整目录提及**时
# 常不带尾斜杠（如 `ls -d legacy/donor`、`legacy/donor exists: yes`）⇒ 旧式只匹配
# `legacy/donor/` 会把这些**真实路径**误报为违规（2026-10-24 合并 main 时实测）。
# `\b` 不会匹配 `legacy/donorXYZ`（`r` 与 `X` 之间无词边界）⇒ 豁免仍然精确。
$exemptRe = 'legacy/donor(/|\b)|donor/apeireth-|donor/\.\.\.|governance-donor-primitives\.md'
# 文件级豁免 = 事实引注特权（研究的天职是引注，同真实路径豁免原则）：
#   - 扫描器与搬运工具自身：替换规则必须含源标识字面量；
#   - 设计调研文档：事实引注；
#   - 许可搬运线四关报告：溯源/版权主体清单必须点名真实上游（引注）。
$exemptFiles = @(
  'scripts/check-neutral-terms.ps1',
  'scripts/vendor-image-core.ps1',
  # 研究引注豁免：调研文档点名真实项目=事实引用（tui 先例同类，2026-10-18）
  'docs/design/tui-power-design-research.md',
  'docs/04-internal/philosophy-study/05-craft-family.md',
  'docs/design/readme-research-notes.md',
  # 研究引注豁免：AI OS 市场对照（2026-10-18，第三方产品名仅作事实引注，不进代码/文案）
  'docs/04-internal/ai-os-landscape-2026-10-18.md',
  'crates/engine/image/NOTICE.md',
  'OSS_NOTICE.md',
  'docs/04-internal/license-port/README.md',
  'docs/04-internal/license-port/gate1-license.md',
  'docs/04-internal/license-port/gate2-quality.md',
  'docs/04-internal/license-port/gate3-security.md',
  'docs/04-internal/license-port/gate4-integration.md'
)
$exempted = 0

$changed = @()
$changed += (cmd /c "git diff --name-only HEAD 2>nul")
$changed += (cmd /c "git diff --cached --name-only 2>nul")
$changed += (cmd /c "git ls-files --others --exclude-standard 2>nul")
$changed = $changed | Where-Object { $_ } | Sort-Object -Unique

$targets = $changed | Where-Object {
  ($_ -match '\.(rs|ts|svelte|mjs|js|toml|md|ps1|json|yml|yaml)$') -and
  ($_ -notmatch '^(artifacts|research|reports|docs/archive|legacy)/') -and
  ($_ -ne 'Cargo.lock')
}

if (-not $targets) {
  Write-Output 'OK    no changed files to scan'
  exit 0
}

$untracked = @{}
foreach ($f in (cmd /c "git ls-files --others --exclude-standard 2>nul")) { if ($f) { $untracked[$f] = $true } }

$hits = 0
foreach ($file in $targets) {
  if (-not (Test-Path $file)) { continue }
  # 文件级豁免（事实引注特权）对两条分支一致生效
  if ($exemptFiles -contains $file) { $exempted++; continue }
  if ($untracked.ContainsKey($file)) {
    $allLines = @(Get-Content $file)
    for ($i = 0; $i -lt $allLines.Count; $i++) {
      foreach ($pat in $forbidden) {
        if ($allLines[$i] -match $pat) {
          if ($allLines[$i] -match $licenseRe) { $licensed++; continue }
          if ($allLines[$i] -match $exemptRe) { $exempted++; continue }
          Write-Output "HIT   ${file}:$($i+1)  [$pat]  $($allLines[$i].Trim())"
          $hits++
        }
      }
    }
    continue
  }
  $diff = @(cmd /c "git diff -U0 HEAD -- `"$file`" 2>nul")
  $lineNo = 0
  foreach ($row in $diff) {
    if ($row -match '^@@ -\d+(?:,\d+)? \+(\d+)') { $lineNo = [int]$matches[1]; continue }
    if ($row.StartsWith('+') -and -not $row.StartsWith('+++')) {
      foreach ($pat in $forbidden) {
        if ($row -match $pat) {
          if ($row -match $licenseRe) { $licensed++; continue }
          if ($row -match $exemptRe) { $exempted++; continue }
          Write-Output "HIT   ${file}:$lineNo  [$pat]  $($row.Substring(1).Trim())"
          $hits++
        }
      }
      $lineNo++
    }
  }
}

if ($licensed -gt 0) {
  Write-Output "EXEMPT  $licensed license-attribution line(s) skipped (许可署名头类：版权/许可/商标声明行，法律义务 > 行文风格，见 `$licenseRe)"
}
if ($exempted -gt 0) {
  Write-Output "EXEMPT  $exempted structural hit(s) skipped (real path/name strings & fact-citation files, see `$exemptRe / `$exemptFiles)"
}

if ($hits -eq 0) {
  Write-Output "OK    $($targets.Count) changed files: added lines all neutral"
  exit 0
} else {
  Write-Output "FAIL  $hits hit(s): added lines must stay neutral (see above)"
  exit 1
}