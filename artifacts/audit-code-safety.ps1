# 代码语义安全审计：找出 diff 中被改动的「非注释」行（.rs/.ts/.svelte/.ps1/.sh）
# 用法: pwsh -File artifacts/audit-code-safety.ps1   输出可疑行供人工复核
$ErrorActionPreference = 'Continue'
$diff = git diff -U0 HEAD -- 'crates' 'frontend/companion-desktop/src' 'frontend/companion-desktop/tests' 'scripts' 'packaging'
$file = ''
$suspicious = @()
foreach ($row in $diff) {
  if ($row -match '^\+\+\+ b/(.+)$') { $file = $matches[1]; continue }
  if ($row -match '^@@') { continue }
  $isAdd = $row.StartsWith('+'); $isDel = $row.StartsWith('-')
  if (-not ($isAdd -or $isDel)) { continue }
  if ($row.StartsWith('+++') -or $row.StartsWith('---')) { continue }
  $text = $row.Substring(1)
  $trim = $text.Trim()
  if ($trim -eq '') { continue }
  # 注释行放行: //, /*, */, *, #, <!--, doc comments
  if ($trim -match '^(//|/\*|\*/|\*|#|<!--)') { continue }
  $ext = [System.IO.Path]::GetExtension($file)
  # .md 全部放行（文档正文）
  if ($ext -eq '.md') { continue }
  # .rs: 非注释行 = 代码行 → 可疑（除非是纯字符串/测试名改动，人工复核）
  if ($ext -in @('.rs','.ts','.svelte','.mjs','.cjs','.js','.ps1','.sh','.h')) {
    $suspicious += "$($isAdd ? '+' : '-') $file :: $trim"
  }
}
if ($suspicious.Count -eq 0) {
  "OK    diff 中无非注释行改动（代码语义零触碰）"
} else {
  "REVIEW  $($suspicious.Count) 处非注释行改动（逐条人工确认）:"
  $suspicious
}
