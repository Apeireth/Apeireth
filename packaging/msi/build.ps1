# Apeireth Windows MSI Packaging Script (APEIRETH 2.0 RC)
# Platform: Windows x86_64 (msiexec /i apeireth-2.0.0-rc.1-windows-x86_64.msi)
# Tool: WiX Toolset / cargo-wix
# Target Path: %ProgramFiles%\Apeireth\

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot\..\..

$VERSION = $env:APEIRETH_VERSION
if (-not $VERSION) { $VERSION = "2.0.0-rc.1" }
$TARGET = $env:APEIRETH_TARGET
if (-not $TARGET) { $TARGET = "x86_64-pc-windows-msvc" }

$ARCH_NAME = "windows-x86_64"
$MSI_NAME = "apeireth-${VERSION}-${ARCH_NAME}.msi"

Write-Host "=== Apeireth MSI Packaging v${VERSION} (target=${TARGET}) ==="

# 1. Build the target-qualified canonical release binary
Write-Host "[1/5] Building release binary..."
$EXE_SRC = "target/${TARGET}/release/apeireth.exe"
cargo build --release --bin apeireth --target $TARGET --locked
if ($LASTEXITCODE -ne 0) { throw "cargo build failed with exit code $LASTEXITCODE" }
if (-not (Test-Path $EXE_SRC)) { throw "Target-qualified binary not found: $EXE_SRC" }


# 2. Stage MSI components
Write-Host "[2/5] Staging MSI components..."
$LegacyWixBinary = Join-Path "target" "release\apeireth.exe"
New-Item -ItemType Directory -Path (Split-Path $LegacyWixBinary) -Force | Out-Null
Copy-Item $EXE_SRC $LegacyWixBinary -Force
$STAGE_DIR = Join-Path "target" "msi-stage"
if (Test-Path $STAGE_DIR) { Remove-Item $STAGE_DIR -Recurse -Force }
New-Item -ItemType Directory -Path (Join-Path $STAGE_DIR "bin") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $STAGE_DIR "config") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $STAGE_DIR "share") -Force | Out-Null

Copy-Item $EXE_SRC (Join-Path $STAGE_DIR "bin" "apeireth.exe") -Force
if (Test-Path "LICENSE") { Copy-Item "LICENSE" (Join-Path $STAGE_DIR "share" "LICENSE") -Force }
if (Test-Path "NOTICE") { Copy-Item "NOTICE" (Join-Path $STAGE_DIR "share" "NOTICE") -Force }
if (Test-Path "README.md") { Copy-Item "README.md" (Join-Path $STAGE_DIR "share" "README.md") -Force }
if (Test-Path "packaging/msi/icon.ico") { Copy-Item "packaging/msi/icon.ico" (Join-Path $STAGE_DIR "icon.ico") -Force }

# 3. WiX Compilation (if WiX or cargo-wix is installed)
Write-Host "[3/5] Compiling WiX MSI installer..."
$WIX_OUT_DIR = "target/msi"
if (-not (Test-Path $WIX_OUT_DIR)) { New-Item -ItemType Directory -Path $WIX_OUT_DIR -Force | Out-Null }
$MSI_OUTPUT_PATH = Join-Path $WIX_OUT_DIR $MSI_NAME

$hasWix = (Get-Command candle.exe -ErrorAction SilentlyContinue) -and (Get-Command light.exe -ErrorAction SilentlyContinue)
$hasWix4 = (Get-Command wix.exe -ErrorAction SilentlyContinue)
$hasCargoWix = (Get-Command cargo-wix -ErrorAction SilentlyContinue)

if ($hasWix) {
    Write-Host "    Found WiX 3.x toolset. Running candle & light..."
    & candle.exe -ext WixUtilExtension -out target/msi/apeireth.wixobj packaging/msi/apeireth.wxs
    & light.exe -ext WixUIExtension -ext WixUtilExtension -out $MSI_OUTPUT_PATH target/msi/apeireth.wixobj
} elseif ($hasWix4) {
    Write-Host "    Found WiX 4.x/5.x toolset. Running wix build..."
    & wix.exe build packaging/msi/apeireth.wxs -ext WixToolset.Util.wixext -ext WixToolset.UI.wixext -o $MSI_OUTPUT_PATH
} elseif ($hasCargoWix) {
    Write-Host "    Running cargo wix..."
    cargo wix --no-build --target $TARGET -o $MSI_OUTPUT_PATH
} else {
    throw 'WiX toolset or cargo-wix is required to produce the CLI MSI.'
}

# 4. Checksum & report
Write-Host "[4/5] Verifying MSI packaging artifacts..."
if (Test-Path $MSI_OUTPUT_PATH) {
    $MSI_SHA256 = (Get-FileHash -Path $MSI_OUTPUT_PATH -Algorithm SHA256).Hash
    "${MSI_SHA256}  ${MSI_NAME}" | Out-File -FilePath "${MSI_OUTPUT_PATH}.sha256" -Encoding UTF8
    $SIZE = (Get-Item $MSI_OUTPUT_PATH).Length / 1MB
    $SIZE_FMT = "{0:N2} MB" -f $SIZE
    Write-Host "    MSI artifact: ${MSI_OUTPUT_PATH} (${SIZE_FMT})"
    Write-Host "    SHA256:       ${MSI_SHA256}"
} else {
    throw "MSI build completed without producing $MSI_OUTPUT_PATH"
}

Write-Host "[5/5] MSI Packaging step complete."
exit 0

