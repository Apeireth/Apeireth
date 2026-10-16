# 中性化清污 grep 审计工具（本文件放 artifacts/，不参与 check-neutral-terms 扫描）
# 用法: pwsh -File artifacts/audit-neutral.ps1 <label>   例: pwsh -File artifacts/audit-neutral.ps1 before
param([string]$Label = 'run')
$ErrorActionPreference = 'Continue'
$outDir = Join-Path $PSScriptRoot "audit-$Label"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

# 范围: tracked 源码 + docs + 根部 md（排除保留类目录/文件）
$protected = '^(docs/archive/|docs/_archived/|reports/|research/|artifacts/|legacy/|CHANGELOG\.md$|README|[^/]*README[^/]*\.md$|OSS_NOTICE\.md$|THIRD-PARTY-NOTICES\.md$|NOTICE$|LICENSE)'
$scope = @(git ls-files | Where-Object {
  $_ -notmatch $protected -and
  ($_ -match '\.(rs|ts|svelte|mjs|js|cjs|toml|md|ps1|sh|json|yml|yaml|h|html|css)$') -and
  ($_ -match '^(crates/|frontend/companion-desktop/(src|tests)/|scripts/|packaging/|docs/|deploy/|[^/]+$)')
})
$scopeSet = @{}; foreach ($f in $scope) { $scopeSet[$f] = $true }

$patterns = [ordered]@{
  'word-zhiyizi'     = '移植自'
  'word-zhiyi'       = '移植'
  'word-jiejianzi'   = '借鉴自'
  'word-jiejian'     = '借鉴'
  'word-yuanshixian' = '原实现'
  'word-canzhao'     = '参照实现'
  'word-111-fanyi'   = '1:1\s*(直接)?翻译'
  'word-111-all'     = '1:1'
  'word-donor'       = 'donor'
  'word-donor-path'  = 'legacy/donor'
  'word-qianren'     = '前任代码'
  'word-shangyou'    = '上游'
  'brand-forbidden'  = '\bDSH\b|dsh-|DeepSeek\s*Harness|DeepSeekHarness'
  'brand-common'     = 'gitleaks|LangGraph|CrewAI|LiteLLM|opencode|OpenCog|gemini-cli|claude-code|Harness-R1|ChatGPT|Copilot|Cursor|Notion|Obsidian|Slack|Discord|Telegram|Ollama|Llama|Qwen|LangChain|VCP|微信'
}
$lines = @("scope files: $($scope.Count)")
foreach ($k in $patterns.Keys) {
  $pat = $patterns[$k]
  $hits = @(git grep -n -i -E $pat -- crates frontend scripts packaging docs deploy '*.md' 2>$null |
    Where-Object { $scopeSet.ContainsKey(($_ -split ':')[0]) })
  $hits | Out-File -Encoding utf8 (Join-Path $outDir "$k.txt")
  $lines += "{0,-24} {1}" -f $k, $hits.Count
}
$lines | Out-File -Encoding utf8 (Join-Path $outDir 'summary.txt')
$lines -join "`n"
