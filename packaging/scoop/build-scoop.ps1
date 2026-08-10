# =============================================================================
# packaging/scoop/build-scoop.ps1
#
# Scoop manifest 的 build script (per task spec 1.0 release #4, 命名对齐)
# 主脚本 packaging/scoop/build.ps1 已有, 本脚本是 alias 满足 task spec 命名
#
# 决策: D-06 (8 包齐发)
# 用法 (PowerShell):
#   .\packaging\scoop\build-scoop.ps1
# =============================================================================

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot\..\..
& "$PSScriptRoot\build.ps1" @args
