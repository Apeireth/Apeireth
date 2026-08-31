# Apeireth CLI Portable ZIP Packaging Script (APEIRETH 2.0 RC)
# Platform: Windows x86_64 (Portable, Extract-and-Run)
# Output: target/apeireth-${VERSION}-windows-x86_64.zip
# Contents: apeireth.exe, LICENSE, NOTICE, README.md, CHANGELOG.md, config/, bin/
#
# Usage:
#   .\packaging\zip\build.ps1
#   $env:APEIRETH_VERSION = "2.0.0-rc.1"; .\packaging\zip\build.ps1

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot\..\..

$VERSION = $env:APEIRETH_VERSION
if (-not $VERSION) { $VERSION = "2.0.0-rc.1" }
$TARGET = $env:APEIRETH_TARGET
if (-not $TARGET) { $TARGET = "x86_64-pc-windows-msvc" }

$ARCH_NAME = "windows-x86_64"
$PACK_NAME = "apeireth-${VERSION}-${ARCH_NAME}"

Write-Host "=== Apeireth Portable ZIP Packaging v${VERSION} (target=${TARGET}) ==="

# 1. Build the target-qualified canonical release binary
Write-Host "[1/5] Building release binary..."
$EXE_SRC = "target/${TARGET}/release/apeireth.exe"
cargo build --release --bin apeireth --target $TARGET --locked
if ($LASTEXITCODE -ne 0) { throw "cargo build failed with exit code $LASTEXITCODE" }
if (-not (Test-Path $EXE_SRC)) {
    throw "Target-qualified binary not found: ${EXE_SRC}"
}
$FINAL_EXE_SRC = $EXE_SRC

# 2. Stage packaging directory
Write-Host "[2/5] Staging packaging files into target/zip-stage/${PACK_NAME}..."
$STAGE_DIR = Join-Path "target" "zip-stage" $PACK_NAME
if (Test-Path $STAGE_DIR) { Remove-Item $STAGE_DIR -Recurse -Force }
New-Item -ItemType Directory -Path (Join-Path $STAGE_DIR "bin") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $STAGE_DIR "config") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $STAGE_DIR "share") -Force | Out-Null

# Copy binary to root and bin/ for maximum user convenience
Copy-Item $FINAL_EXE_SRC (Join-Path $STAGE_DIR "apeireth.exe")
Copy-Item $FINAL_EXE_SRC (Join-Path $STAGE_DIR "bin" "apeireth.exe")

# Copy mandatory license and notices (LICENSE, NOTICE, README.md)
if (Test-Path "LICENSE") { Copy-Item "LICENSE" (Join-Path $STAGE_DIR "LICENSE") }
if (Test-Path "NOTICE") { Copy-Item "NOTICE" (Join-Path $STAGE_DIR "NOTICE") }
if (Test-Path "README.md") { Copy-Item "README.md" (Join-Path $STAGE_DIR "README.md") }
if (Test-Path "CHANGELOG.md") { Copy-Item "CHANGELOG.md" (Join-Path $STAGE_DIR "CHANGELOG.md") }

# Also keep share/ copies for standard layout
if (Test-Path "LICENSE") { Copy-Item "LICENSE" (Join-Path $STAGE_DIR "share" "LICENSE") }
if (Test-Path "NOTICE") { Copy-Item "NOTICE" (Join-Path $STAGE_DIR "share" "NOTICE") }
if (Test-Path "README.md") { Copy-Item "README.md" (Join-Path $STAGE_DIR "share" "README.md") }

# 3. Config template + quick-start script
@"
# Apeireth OS 2.0 — Environment Configuration Example
APEIRETH_HOME=%USERPROFILE%\.apeireth
APEIRETH_CONFIG=%APEIRETH_HOME%\config.toml
APEIRETH_GATEWAY_PORT=8080
APEIRETH_LOG_LEVEL=info
"@ | Out-File -FilePath (Join-Path $STAGE_DIR "config" "apeireth.env.example") -Encoding UTF8

@"
@echo off
rem Apeireth OS 2.0 — Windows Serve Launcher
setlocal
set "APEIRETH_HOME=%USERPROFILE%\.apeireth"
if not exist "%APEIRETH_HOME%" mkdir "%APEIRETH_HOME%"
"%~dp0apeireth.exe" serve %*
endlocal
"@ | Out-File -FilePath (Join-Path $STAGE_DIR "bin" "apeireth-serve.bat") -Encoding ASCII

@"
Apeireth OS v${VERSION} — Portable Windows Distribution (x86_64)

Contents:
  - apeireth.exe       : Canonical Apeireth CLI / Server binary
  - LICENSE            : Apache 2.0 License
  - NOTICE             : Third-party notices and attribution
  - README.md          : Project documentation
  - bin\               : Launcher scripts and duplicate binary
  - config\            : Configuration templates

Getting Started:
  1. Extract archive to desired location (e.g. C:\Tools\Apeireth or C:\Program Files\Apeireth)
  2. Add extraction directory to your PATH
  3. Run:
       apeireth --version
       apeireth serve

Optional Windows Service (via NSSM):
  nssm install Apeireth "%~dp0bin\apeireth.exe" serve
  nssm start Apeireth

Lifecycle:
  - Upgrade: Replace folder contents with new version.
  - Uninstall: Remove extracted folder and remove from PATH.
"@ | Out-File -FilePath (Join-Path $STAGE_DIR "README.txt") -Encoding UTF8

# 4. Create ZIP archive
Write-Host "[3/5] Compressing archive..."
$TARGET_DIR = "target"
if (-not (Test-Path $TARGET_DIR)) { New-Item -ItemType Directory -Path $TARGET_DIR -Force | Out-Null }

$ZIP_PATH = "target/${PACK_NAME}.zip"
if (Test-Path $ZIP_PATH) { Remove-Item $ZIP_PATH -Force }
Compress-Archive -Path "$STAGE_DIR\*" -DestinationPath $ZIP_PATH -CompressionLevel Optimal

# Also create legacy/canonical target named zip for target compatibility
$TARGET_ZIP_PATH = "target/apeireth-${VERSION}-${TARGET}.zip"
if ($ZIP_PATH -ne $TARGET_ZIP_PATH) {
    Copy-Item $ZIP_PATH $TARGET_ZIP_PATH -Force
}

# 5. Checksum generation
Write-Host "[4/5] Computing SHA256 checksums..."
$ZIP_SHA256 = (Get-FileHash -Path $ZIP_PATH -Algorithm SHA256).Hash
"${ZIP_SHA256}  ${PACK_NAME}.zip" | Out-File -FilePath "${ZIP_PATH}.sha256" -Encoding UTF8
if (Test-Path $TARGET_ZIP_PATH) {
    "${ZIP_SHA256}  apeireth-${VERSION}-${TARGET}.zip" | Out-File -FilePath "${TARGET_ZIP_PATH}.sha256" -Encoding UTF8
}

# 6. Report summary
$SIZE = (Get-Item $ZIP_PATH).Length / 1MB
$SIZE_FMT = "{0:N2} MB" -f $SIZE
Write-Host "[5/5] Portable ZIP generated successfully:"
Write-Host "    File:   ${ZIP_PATH} (${SIZE_FMT})"
Write-Host "    SHA256: ${ZIP_SHA256}"
Write-Host "    Stage:  ${STAGE_DIR}"

exit 0
