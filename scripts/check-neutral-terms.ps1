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

# 结构性豁免：命中若仅来自这些**真实字符串**（真实路径/真实文件名/本表自身）
# 则按"路径保持准确"规则保留、不计违规；豁免计数随输出给出便于审计。
$exemptRe = 'legacy/donor/|donor/apeireth-|donor/\.\.\.|governance-donor-primitives\.md'
$exemptFiles = @('scripts/check-neutral-terms.ps1', 'docs/design/tui-power-design-research.md')  # 设计调研文档=事实引注特权（研究的天职是引注，同真实路径豁免原则）
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
  if ($untracked.ContainsKey($file)) {
    if ($exemptFiles -contains $file) { continue }
    $allLines = @(Get-Content $file)
    for ($i = 0; $i -lt $allLines.Count; $i++) {
      foreach ($pat in $forbidden) {
        if ($allLines[$i] -match $pat) {
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
          if ($row -match $exemptRe) { $exempted++; continue }
          Write-Output "HIT   ${file}:$lineNo  [$pat]  $($row.Substring(1).Trim())"
          $hits++
        }
      }
      $lineNo++
    }
  }
}

if ($exempted -gt 0) {
  Write-Output "EXEMPT  $exempted structural hit(s) skipped (real path/name strings, see `$exemptRe)"
}

if ($hits -eq 0) {
  Write-Output "OK    $($targets.Count) changed files: added lines all neutral"
  exit 0
} else {
  Write-Output "FAIL  $hits hit(s): added lines must stay neutral (see above)"
  exit 1
}