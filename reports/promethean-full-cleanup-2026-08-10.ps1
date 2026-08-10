# promethean-full-cleanup-2026-08-10.ps1
# 主人 21:58 自执行, per 决策 #59
# 删除 `.openclaw\workspace\promethean\` 整个目录 (32,960 文件 / 42.6 MB)
# 不动: borrowed-repos / apeireth-debug / 新主仓 `Apeireth-rust\`

$ErrorActionPreference = 'Stop'

$promethean = '.openclaw\workspace\promethean'
$borrowedRepos = '.openclaw\workspace\borrowed-repos'
$apeirethDebug = '.openclaw\workspace\apeireth-debug'
$newMaster = 'Apeireth-rust'

Write-Host '=== Before ==='
Write-Host "promethean exists: $(Test-Path -LiteralPath $promethean -PathType Container)"
Write-Host "borrowed-repos exists: $(Test-Path -LiteralPath $borrowedRepos -PathType Container)"
Write-Host "apeireth-debug exists: $(Test-Path -LiteralPath $apeirethDebug -PathType Container)"
Write-Host "new master exists: $(Test-Path -LiteralPath (Join-Path $newMaster '.git\refs\heads\master') -PathType Leaf)"

if (-not (Test-Path -LiteralPath $promethean -PathType Container)) {
    Write-Host "promethean/ 0 存在, 0 必删, exit"
    exit 0
}

Write-Host '=== Deleting promethean/ ==='
$items = Get-ChildItem -LiteralPath $promethean -Force
$count = 0
foreach ($item in $items) {
    if ($item.PSIsContainer) {
        Remove-Item -LiteralPath $item.FullName -Recurse -Force
    } else {
        Remove-Item -LiteralPath $item.FullName -Force
    }
    $count++
    if ($count % 10 -eq 0) {
        Write-Host "deleted $count items..."
    }
}
Write-Host "deleted $count items"
Remove-Item -LiteralPath $promethean -Force

Write-Host '=== After ==='
Write-Host "promethean exists: $(Test-Path -LiteralPath $promethean -PathType Container)"
Write-Host "borrowed-repos exists: $(Test-Path -LiteralPath $borrowedRepos -PathType Container)"
Write-Host "apeireth-debug exists: $(Test-Path -LiteralPath $apeirethDebug -PathType Container)"
Write-Host "new master HEAD: $((Get-Content (Join-Path $newMaster '.git\refs\heads\master')).Trim())"

Write-Host '=== promethean/ 全删 done ==='
