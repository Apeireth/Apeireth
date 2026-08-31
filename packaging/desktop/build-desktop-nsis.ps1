# packaging/desktop/build-desktop-nsis.ps1
# Builds and stages Tauri Desktop NSIS installer (setup.exe)
param(
    [string]$Version = "2.0.0-rc.1",
    [string]$Target = "x86_64-pc-windows-msvc"
)

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot\..\..

Write-Host "=== Apeireth Tauri Desktop NSIS Packaging v$Version ==="

# 1. Stage frontend, canonical CLI, and sidecar
& "$PSScriptRoot\stage-desktop.ps1" -Version $Version -Target $Target
if ($LASTEXITCODE -ne 0) { throw "Desktop staging failed with exit code $LASTEXITCODE" }

# 2. Build Tauri desktop bundle (NSIS)
$DesktopDir = "frontend\companion-desktop"
$OutNsisDir = "target\desktop-nsis"
if (Test-Path $OutNsisDir) { Remove-Item $OutNsisDir -Recurse -Force }
New-Item -ItemType Directory -Path $OutNsisDir -Force | Out-Null
$TauriNsisDir = Join-Path $DesktopDir "src-tauri\target\$Target\release\bundle\nsis"
if (Test-Path $TauriNsisDir) { Remove-Item $TauriNsisDir -Recurse -Force }

Write-Host "[1/3] Building Tauri NSIS bundle..."
if (-not (Get-Command pnpm -ErrorAction SilentlyContinue)) {
    throw 'pnpm is required for the Tauri NSIS build.'
}
Push-Location $DesktopDir
try {
    cmd.exe /c pnpm tauri build --bundles nsis --target $Target
    if ($LASTEXITCODE -ne 0) { throw "Tauri NSIS build failed with exit code $LASTEXITCODE" }
} finally {
    Pop-Location
}

# 3. Collect NSIS artifact
$tauriNsisCandidates = @(
    "$DesktopDir\src-tauri\target\$Target\release\bundle\nsis\*.exe"
)

$foundNsis = $false
foreach ($pattern in $tauriNsisCandidates) {
    $exeFiles = Get-ChildItem -Path $pattern -ErrorAction SilentlyContinue
    if ($exeFiles) {
        foreach ($f in $exeFiles) {
            $dest = Join-Path $OutNsisDir $f.Name
            Copy-Item $f.FullName $dest -Force
            $sha = (Get-FileHash -Path $dest -Algorithm SHA256).Hash
            "${sha}  $($f.Name)" | Out-File -FilePath "${dest}.sha256" -Encoding UTF8
            Write-Host "    Desktop NSIS artifact: $dest"
            Write-Host "    SHA256:                $sha"
            $foundNsis = $true
        }
        break
    }
}

if (-not $foundNsis) {
    throw "Tauri NSIS build completed without producing an installer under $DesktopDir\src-tauri\target."
}

Write-Host "[3/3] Desktop NSIS packaging step complete."
exit 0
