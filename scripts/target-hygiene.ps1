# ============================================================================
# scripts/target-hygiene.ps1 — 构建缓存治理脚本 (台账 #51, 主人 2026-08-17 指示)
# ============================================================================
# 用途: 报告 target/ 体积分布 + 安全清理项清单; 默认只报告, -Apply 才真删。
# 测量方法: robocopy /L 摘要 (实测 45.9 万文件 1.1s; Windows 下 du 极慢已弃用)。
# 解析说明: robocopy 摘要按固定顺序输出 (目录/文件/字节 三行), 按行序取值,
#           不匹配中文标签 —— PowerShell 5.1 对无 BOM 文件的中文匹配不可靠。
# 铁边界 (maintenance-guide §target 治理):
#   ① 禁止在成员活跃编译期全量 cargo clean (会打断在途任务)
#   ② 本脚本不碰 deps/ build/ .fingerprint/ (活跃编译正确性依赖)
#   ③ 只清"可再生"项: incremental 缓存 / criterion / tmp / 顶层垃圾文件
param([switch]$Apply)

$ErrorActionPreference = 'Stop'
$target = Join-Path $PSScriptRoot '..\target' | Resolve-Path
if (-not (Test-Path $target)) { Write-Host "target not found: $target"; exit 0 }

function Measure-Dir($dir) {
    if (-not (Test-Path $dir)) { return @{ Files = 0; Bytes = 0 } }
    $out = robocopy $dir "$env:TEMP\null-hygiene" /L /S /BYTES /NFL /NDL /NJH /NP /R:0 /W:0 2>&1
    $nums = @()
    foreach ($l in $out) {
        if ($l -match '^\s*\S+\s*:\s*(\d+)\s') { $nums += [long]$Matches[1] }
    }
    # 行序: [0]=目录数 [1]=文件数 [2]=字节数
    $files = 0; $bytes = 0
    if ($nums.Count -ge 2) { $files = $nums[1] }
    if ($nums.Count -ge 3) { $bytes = $nums[2] }
    return @{ Files = $files; Bytes = $bytes }
}
function Fmt-GB($b) { '{0:N2} GB' -f ($b / 1GB) }

Write-Host ("== target baseline (robocopy /L, {0}) ==" -f (Get-Date -Format 'yyyy-MM-dd HH:mm'))
$total = Measure-Dir $target
Write-Host ("TOTAL: {0} files, {1}" -f $total.Files, (Fmt-GB $total.Bytes))

Write-Host "`n== subdir breakdown =="
foreach ($sub in @('debug', 'release', 'debug\deps', 'debug\incremental', 'debug\build', 'debug\.fingerprint', 'release\deps', 'release\incremental')) {
    $d = Join-Path $target $sub
    if (Test-Path $d) {
        $m = Measure-Dir $d
        Write-Host ("{0,-24} {1,8} files  {2}" -f $sub, $m.Files, (Fmt-GB $m.Bytes))
    }
}

# ---- safe cleanup candidates (regenerable; cost = slower next build) ----
$cleanItems = @()
# incremental: only scan one level per profile dir (target/<profile>/incremental); no full recurse (460k files too slow)
foreach ($prof in (Get-ChildItem $target -Directory)) {
    $inc = Join-Path $prof.FullName 'incremental'
    if (Test-Path $inc) { $cleanItems += [pscustomobject]@{ Path = $inc; Why = 'incremental cache (regenerable)' } }
}
foreach ($name in @('criterion', 'tmp')) {
    $d = Join-Path $target $name
    if (Test-Path $d) { $cleanItems += [pscustomobject]@{ Path = $d; Why = "$name (bench/temp)" } }
}
# stray top-level junk files (keep structural files like CACHEDIR.TAG)
foreach ($f in (Get-ChildItem $target -File -ErrorAction SilentlyContinue |
        Where-Object { $_.Extension -in '.log', '.tmp', '.py', '.txt', '.rs' })) {
    $cleanItems += [pscustomobject]@{ Path = $f.FullName; Why = 'top-level junk' }
}

Write-Host ("`n== safe cleanup candidates ({0} items) ==" -f $cleanItems.Count)
$candBytes = 0
foreach ($i in $cleanItems) {
    $m = Measure-Dir $i.Path
    $candBytes += $m.Bytes
    Write-Host ("{0,-10} {1}" -f (Fmt-GB $m.Bytes), $i.Path)
    Write-Host ("           +-- {0}" -f $i.Why)
}
Write-Host ("reclaimable total: {0}" -f (Fmt-GB $candBytes))

if ($Apply) {
    Write-Host "`n== -Apply: deleting =="
    foreach ($i in $cleanItems) {
        Remove-Item $i.Path -Recurse -Force -ErrorAction SilentlyContinue
        Write-Host "deleted: $($i.Path)"
    }
    $after = Measure-Dir $target
    Write-Host ("after: {0} files, {1}" -f $after.Files, (Fmt-GB $after.Bytes))
} else {
    Write-Host "`n(report only; add -Apply to delete. Full cargo clean must wait for compile quiet window, see maintenance-guide)"
}
