# 星槎打包脚本自测（test-make-portable.ps1）——真跑布局断言，不装真产物。
# 断言：DryRun 不动盘；真跑造出 portable.flag + data\ + PORTABLE-README.txt；
#       0 装边界（缺 exe / 输出落源内 / 输出为根）一律拒绝。

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$failed = 0

function Check([bool]$Cond, [string]$What) {
    if ($Cond) {
        Write-Host "  [OK] $What"
    } else {
        Write-Host "  [FAIL] $What"
        $script:failed += 1
    }
}

$root = Join-Path ([System.IO.Path]::GetTempPath()) ("starport-portable-test-" + [System.Guid]::NewGuid().ToString('N').Substring(0, 8))
$src = Join-Path $root 'src'
New-Item -ItemType Directory -Path $src | Out-Null
[System.IO.File]::WriteAllBytes((Join-Path $src 'companion-desktop.exe'), [byte[]](0x4D, 0x5A))
$scriptPath = Join-Path $PSScriptRoot 'make-portable.ps1'

# ---- 1. DryRun 不动盘 ----
Write-Host '[1] DryRun 不动盘'
$out1 = Join-Path $root 'out-dry'
& $scriptPath -SourceDir $src -OutDir $out1 -DryRun | Out-Null
Check (-not (Test-Path -LiteralPath $out1)) 'DryRun 不创建输出目录'

# ---- 2. 真跑布局 ----
Write-Host '[2] 真跑布局'
$out2 = Join-Path $root 'out-real'
& $scriptPath -SourceDir $src -OutDir $out2 | Out-Null
Check (Test-Path -LiteralPath (Join-Path $out2 'portable.flag')) 'portable.flag 在位'
Check (Test-Path -LiteralPath (Join-Path $out2 'data') -PathType Container) 'data\ 目录在位'
Check (Test-Path -LiteralPath (Join-Path $out2 'PORTABLE-README.txt')) '随身说明在位'
Check (Test-Path -LiteralPath (Join-Path $out2 'companion-desktop.exe')) '产物已复制'

# ---- 3. 0 装边界 ----
Write-Host '[3] 0 装边界'
$empty = Join-Path $root 'empty'
New-Item -ItemType Directory -Path $empty | Out-Null
$rejected = $false
try { & $scriptPath -SourceDir $empty -OutDir (Join-Path $root 'x') 2>$null | Out-Null } catch { $rejected = $true }
Check $rejected '缺 exe 的源目录拒绝'

$rejected = $false
try { & $scriptPath -SourceDir $src -OutDir (Join-Path $src 'inner') 2>$null | Out-Null } catch { $rejected = $true }
Check $rejected '输出落源目录内拒绝'

$rejected = $false
try { & $scriptPath -SourceDir $src -OutDir ([System.IO.Path]::GetPathRoot([System.IO.Path]::GetTempPath())) 2>$null | Out-Null } catch { $rejected = $true }
Check $rejected '输出为文件系统根拒绝'

Remove-Item -LiteralPath $root -Recurse -Force
if ($failed -eq 0) {
    Write-Host '--- make-portable self-test PASSED ---'
    exit 0
}
Write-Host "--- make-portable self-test FAILED ($failed) ---"
exit 1
