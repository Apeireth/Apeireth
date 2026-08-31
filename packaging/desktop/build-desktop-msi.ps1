# packaging/desktop/build-desktop-msi.ps1
# Builds and stages Tauri Desktop MSI installer
param(
    [string]$Version = "2.0.0-rc.1",
    [string]$Target = "x86_64-pc-windows-msvc"
)

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot\..\..

Write-Host "=== Apeireth Tauri Desktop MSI Packaging v$Version ==="

# 1. Stage frontend, canonical CLI, and sidecar
& "$PSScriptRoot\stage-desktop.ps1" -Version $Version -Target $Target
if ($LASTEXITCODE -ne 0) { throw "Desktop staging failed with exit code $LASTEXITCODE" }

# 2. Build Tauri desktop bundle (MSI)
$DesktopDir = "frontend\companion-desktop"
$OutMsiDir = "target\desktop-msi"
if (Test-Path $OutMsiDir) { Remove-Item $OutMsiDir -Recurse -Force }
New-Item -ItemType Directory -Path $OutMsiDir -Force | Out-Null
$TauriMsiDir = Join-Path $DesktopDir "src-tauri\target\$Target\release\bundle\msi"
if (Test-Path $TauriMsiDir) { Remove-Item $TauriMsiDir -Recurse -Force }

Write-Host "[1/3] Building Tauri MSI bundle..."
if (-not (Get-Command pnpm -ErrorAction SilentlyContinue)) {
    throw 'pnpm is required for the Tauri MSI build.'
}
Push-Location $DesktopDir
try {
    cmd.exe /c pnpm tauri build --bundles msi --target $Target
    if ($LASTEXITCODE -ne 0) { throw "Tauri MSI build failed with exit code $LASTEXITCODE" }
} finally {
    Pop-Location
}

# 3. Collect MSI artifact
$tauriMsiCandidates = @(
    "$DesktopDir\src-tauri\target\$Target\release\bundle\msi\*.msi"
)

$foundMsi = $false
foreach ($pattern in $tauriMsiCandidates) {
    $msiFiles = Get-ChildItem -Path $pattern -ErrorAction SilentlyContinue
    if ($msiFiles) {
        foreach ($f in $msiFiles) {
            $dest = Join-Path $OutMsiDir $f.Name
            Copy-Item $f.FullName $dest -Force
            $sha = (Get-FileHash -Path $dest -Algorithm SHA256).Hash
            "${sha}  $($f.Name)" | Out-File -FilePath "${dest}.sha256" -Encoding UTF8
            Write-Host "    Desktop MSI artifact: $dest"
            Write-Host "    SHA256:               $sha"
            $foundMsi = $true
        }
        break
    }
}

if (-not $foundMsi) {
    throw "Tauri MSI build completed without producing an installer under $DesktopDir\src-tauri\target."
}

Write-Host "[3/3] Desktop MSI packaging step complete."
exit 0
