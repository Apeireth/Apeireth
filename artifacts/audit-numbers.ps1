# 数字漂移审计：对 diff 中成对的 -/+ 行比较数字串多重集，报告数字有变化的行对
# 用法: pwsh -File artifacts/audit-numbers.ps1   （预存改动文件会一并报出，人工甄别）
$ErrorActionPreference = 'Continue'
$diff = git diff -U0 HEAD
$pendingDel = @()
$results = @()
$file = ''
foreach ($row in $diff) {
  if ($row -match '^\+\+\+ b/(.+)$') { $file = $matches[1]; $pendingDel = @(); continue }
  if ($row -match '^@@') { $pendingDel = @(); continue }
  if ($row.StartsWith('---')) { continue }
  if ($row.StartsWith('-')) { $pendingDel += $row.Substring(1); continue }
  if ($row.StartsWith('+')) {
    $add = $row.Substring(1)
    if ($pendingDel.Count -gt 0) {
      $del = $pendingDel[0]; $pendingDel = $pendingDel[1..($pendingDel.Count)]
      if ($null -eq $del) { $del = $pendingDel; $pendingDel = @() }
      # 先剥掉禁词自带的数字（1:1）与含数字的品牌名（mem0），再比较剩余数字串
      $normDel = $del -replace '1:1','' -replace 'mem0',''
      $normAdd = $add -replace '1:1','' -replace 'mem0',''
      $dDel = ([regex]::Matches($normDel, '\d+') | ForEach-Object { $_.Value }) -join ','
      $dAdd = ([regex]::Matches($normAdd, '\d+') | ForEach-Object { $_.Value }) -join ','
      if ($dDel -ne $dAdd) {
        $results += "NUM  $file`n  - $del`n  + $add"
      }
    }
  }
}
if ($results.Count -eq 0) { "OK    成对改动行数字串全部一致" } else { "REVIEW  $($results.Count / 3) 处数字差异:"; $results }
