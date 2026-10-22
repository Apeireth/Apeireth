<#
.SYNOPSIS
    Apeireth 发布资产校验器（PowerShell）——SHA-256 哈希比对 + 签名状态如实报
.DESCRIPTION
    校验下载/构建得到的发布资产是否与官方 SHA-256 清单一致，并如实报告
    Windows Authenticode 签名状态。0 装纪律：哈希不一致、清单缺项、未签名
    一律如实外显，绝不装过、不假装有签名。本脚本只做校验，不改任何被检文件。

    能力边界（如实声明）：
      1. 哈希一致只证明「文件与清单发布者给出的哈希相同」，不证明二进制来自
        某个源码提交（可复现构建证明 = 排期待办，见批次文档）；
      2. 签名状态只如实转述平台校验结果，未签名不是失败（当前发布物即此状态），
        除非显式 -RequireSignature；
      3. 清单本身来自哪个通道由使用者负责（建议与安装包分通道获取）。

.PARAMETER ArtifactPath
    待校验发布资产（一个或多个文件路径）。
.PARAMETER ManifestPath
    官方 SHA-256 清单文件。两种格式均收：SHA256SUMS（每行 "<哈希>  <文件名>"）
    或单行 .sha256 sidecar（构建脚本产物，如 "<哈希>  apeireth-x.y.z.msi"）。
    省略时自动查找与资产同目录的 "<资产文件名>.sha256" sidecar。
.PARAMETER ExpectedSha256
    直接给定期望哈希（64 位十六进制，大小写不敏感）。仅单资产时可用。
.PARAMETER RequireSignature
    附加门槛：签名状态非「有效」即判失败。默认签名状态只如实报告、不参与成败
    （当前发布物未签名是既成事实，如实报，不假装）。
.EXAMPLE
    pwsh -File scripts/verify-release-assets.ps1 -ArtifactPath dist\apeireth-2.0.0-rc.4.msi -ManifestPath dist\SHA256SUMS
.EXAMPLE
    pwsh -File scripts/verify-release-assets.ps1 .\apeireth-companion-setup.exe -ExpectedSha256 3a7f...（64 位完整哈希）
.OUTPUTS
    退出码 0 = 全部资产哈希比对一致（-RequireSignature 时签名亦须有效）；
    退出码 1 = 哈希不一致 / 清单缺项 / 参数不合法 / -RequireSignature 下签名非有效。
#>
[CmdletBinding()]
param(
    [Parameter(Position = 0, Mandatory = $true)]
    [string[]]$ArtifactPath,

    [Parameter()]
    [string]$ManifestPath,

    [Parameter()]
    [string]$ExpectedSha256,

    [Parameter()]
    [switch]$RequireSignature
)

$ErrorActionPreference = "Stop"

# ---- 参数校验（fail-closed：不合法直接拒，不猜） ----
if (-not $ArtifactPath -or $ArtifactPath.Count -eq 0) {
    Write-Host "FAIL  未提供待校验资产（-ArtifactPath）" -ForegroundColor Red
    exit 1
}
if ($ExpectedSha256 -and $ArtifactPath.Count -gt 1) {
    Write-Host "FAIL  -ExpectedSha256 仅支持单资产；多资产请用 -ManifestPath 清单" -ForegroundColor Red
    exit 1
}
if ($ExpectedSha256 -and ($ExpectedSha256 -notmatch '^[0-9a-fA-F]{64}$')) {
    Write-Host "FAIL  -ExpectedSha256 不是 64 位十六进制哈希：$ExpectedSha256" -ForegroundColor Red
    exit 1
}
if ($ExpectedSha256 -and $ManifestPath) {
    Write-Host "FAIL  -ExpectedSha256 与 -ManifestPath 二选一，不叠加（避免双口径）" -ForegroundColor Red
    exit 1
}

function Get-ManifestTable {
    # 解析 SHA256SUMS / .sha256 sidecar：每行 "<哈希>  <文件名>"（兼容 "*" 前缀与多余空白）。
    param([string]$Path)
    $table = @{}
    foreach ($line in (Get-Content -Path $Path)) {
        if ($line -match '^\s*([0-9a-fA-F]{64})\s+\*?(.+?)\s*$') {
            $hash = $Matches[1].ToLowerInvariant()
            $name = Split-Path -Leaf $Matches[2]
            $table[$name] = $hash
        }
    }
    return $table
}

function Get-SignatureReport {
    # 签名状态如实报：查得到平台校验面就转述结果，查不到就说「未验证」。
    # 未签名 = 如实报「未签名」，不假装、不降级成「大概没问题」。
    param([string]$Path)
    $cmd = Get-Command Get-AuthenticodeSignature -ErrorAction SilentlyContinue
    if (-not $cmd) {
        return @{ Display = "签名状态未验证（本平台无 Authenticode 校验面）"; IsSigned = $false; IsValid = $false }
    }
    $sig = Get-AuthenticodeSignature -FilePath $Path
    switch ($sig.Status.ToString()) {
        "Valid" {
            $subject = if ($sig.SignerCertificate) { $sig.SignerCertificate.Subject } else { "（无证书主体）" }
            return @{ Display = "签名有效（$subject）"; IsSigned = $true; IsValid = $true }
        }
        "NotSigned" {
            return @{ Display = "未签名（如实报告：无任何 Authenticode 签名）"; IsSigned = $false; IsValid = $false }
        }
        default {
            return @{ Display = "签名状态：$($sig.Status) — $($sig.StatusMessage)"; IsSigned = $true; IsValid = $false }
        }
    }
}

# ---- 总表头（与 check-release-version.ps1 / generate-release-manifest.ps1 同风格） ----
Write-Host ""
Write-Host "================================================================================" -ForegroundColor Cyan
Write-Host "                     APEIRETH RELEASE ASSET VERIFICATION                        " -ForegroundColor Cyan
Write-Host "================================================================================" -ForegroundColor Cyan
Write-Host "  Manifest:        $(if ($ManifestPath) { $ManifestPath } elseif ($ExpectedSha256) { '<-ExpectedSha256 直给>' } else { '<sidecar 自动查找>' })"
Write-Host "  Signature Gate:  $(if ($RequireSignature) { '必需（非有效即失败）' } else { '仅如实报告（未签名不判失败）' })"
Write-Host "--------------------------------------------------------------------------------"

$allPassed = $true
$summary = [System.Collections.Generic.List[string]]::new()

foreach ($artifact in $ArtifactPath) {
    Write-Host ""
    if (-not (Test-Path -Path $artifact -PathType Leaf)) {
        Write-Host "[FAIL] $artifact" -ForegroundColor Red
        Write-Host "        -> 资产文件不存在（不装过）" -ForegroundColor Red
        $allPassed = $false
        $summary.Add("FAIL  $artifact（文件不存在）")
        continue
    }
    $item = Get-Item -Path $artifact
    $computed = (Get-FileHash -Path $item.FullName -Algorithm SHA256).Hash.ToLowerInvariant()

    # ---- 期望哈希解析：直给 > 指定清单 > 同目录 .sha256 sidecar ----
    $expected = $null
    $expectedSource = ""
    if ($ExpectedSha256) {
        $expected = $ExpectedSha256.ToLowerInvariant()
        $expectedSource = "-ExpectedSha256 直给"
    } else {
        $manifestFile = $ManifestPath
        if (-not $manifestFile) {
            $sidecar = "$($item.FullName).sha256"
            if (Test-Path -Path $sidecar -PathType Leaf) {
                $manifestFile = $sidecar
                $expectedSource = "sidecar $(Split-Path -Leaf $sidecar)"
            } else {
                Write-Host "[FAIL] $($item.Name)" -ForegroundColor Red
                Write-Host "        -> 无期望哈希来源（缺 -ManifestPath / -ExpectedSha256，且无同目录 .sha256 sidecar）——无法比对即不通过" -ForegroundColor Red
                $allPassed = $false
                $summary.Add("FAIL  $($item.Name)（无期望哈希来源）")
                continue
            }
        } else {
            $expectedSource = "清单 $(Split-Path -Leaf $manifestFile)"
        }
        if (-not (Test-Path -Path $manifestFile -PathType Leaf)) {
            Write-Host "[FAIL] $($item.Name)" -ForegroundColor Red
            Write-Host "        -> 清单文件不存在：$manifestFile" -ForegroundColor Red
            $allPassed = $false
            $summary.Add("FAIL  $($item.Name)（清单不存在）")
            continue
        }
        $table = Get-ManifestTable -Path $manifestFile
        if ($table.ContainsKey($item.Name)) {
            $expected = $table[$item.Name]
        }
    }

    if (-not $expected) {
        Write-Host "[FAIL] $($item.Name)" -ForegroundColor Red
        Write-Host "        -> 清单中无此项（$expectedSource）——缺项不装过" -ForegroundColor Red
        $allPassed = $false
        $summary.Add("FAIL  $($item.Name)（清单缺项）")
        continue
    }

    # ---- 哈希比对 ----
    $hashOk = ($computed -eq $expected)
    Write-Host "[$(if ($hashOk) { 'PASS' } else { 'FAIL' })] $($item.Name)" -ForegroundColor $(if ($hashOk) { "Green" } else { "Red" })
    Write-Host "        期望（$expectedSource）: $expected"
    Write-Host "        实算（SHA-256）:        $computed"
    if (-not $hashOk) {
        Write-Host "        -> 哈希不一致：文件与官方清单不符（可能被篡改/损坏/拿错版本），不要安装" -ForegroundColor Red
    }

    # ---- 签名状态如实报 ----
    $sig = Get-SignatureReport -Path $item.FullName
    $sigColor = if ($sig.IsValid) { "Green" } elseif ($sig.IsSigned) { "Yellow" } else { "Gray" }
    Write-Host "        签名: $($sig.Display)" -ForegroundColor $sigColor
    $sigOk = (-not $RequireSignature) -or $sig.IsValid
    if ($RequireSignature -and -not $sig.IsValid) {
        Write-Host "        -> -RequireSignature 门槛下签名非有效即失败" -ForegroundColor Red
    }

    $passed = $hashOk -and $sigOk
    if (-not $passed) { $allPassed = $false }
    $summary.Add("$(if ($passed) { 'PASS' } else { 'FAIL' })  $($item.Name)（哈希: $(if ($hashOk) { '一致' } else { '不一致' })；签名: $($sig.Display)）")
}

# ---- 汇总（成败只看比对事实，不粉饰） ----
Write-Host ""
Write-Host "================================================================================" -ForegroundColor Cyan
foreach ($row in $summary) { Write-Host "  $row" }
Write-Host "--------------------------------------------------------------------------------"
if ($allPassed) {
    Write-Host "  RESULT: PASS — 全部资产哈希比对通过。" -ForegroundColor Green
    Write-Host "  边界：哈希一致 ≠ 二进制来源可证（可复现构建证明 = 排期待办）；签名状态见上，未签名如实记载。" -ForegroundColor Yellow
    Write-Host "================================================================================" -ForegroundColor Cyan
    exit 0
} else {
    Write-Host "  RESULT: FAIL — 存在哈希不一致/清单缺项/门槛未过，详见上表。" -ForegroundColor Red
    Write-Host "================================================================================" -ForegroundColor Cyan
    exit 1
}
