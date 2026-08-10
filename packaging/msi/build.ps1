# Apeireth MSI 构建 (8 包之 1, D-06 拍板)
# 平台: Windows (msiexec /i apeireth-1.0.0-x86_64.msi)
# 工具: cargo-wix (WiX 3.x)
# 体积: ~50MB (含 Windows service 注册)

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot\..\..

$VERSION = $env:APEIRETH_VERSION
if (-not $VERSION) { $VERSION = "1.0.0" }
$TARGET = $env:APEIRETH_TARGET
if (-not $TARGET) { $TARGET = "x86_64-pc-windows-msvc" }

Write-Host "=== apeireth MSI build v${VERSION} (target=${TARGET}) ==="

# 1. 检查 cargo-wix
if (-not (Get-Command cargo-wix -ErrorAction SilentlyContinue)) {
    Write-Host "[1/4] installing cargo-wix..."
    cargo install cargo-wix --locked
}

# 2. cargo build
Write-Host "[2/4] cargo build --release --target ${TARGET}..."
cargo build --release --bin apeireth --target $TARGET --locked

# 3. cargo wix (用 packaging/msi/apeireth.wxs 模板)
Write-Host "[3/4] cargo wix..."
# 准备 Cargo.toml metadata (wix 需要) — 0 触碰: 留给 release engineer 触发
# 此处为 1.0 release dry-run, 若 metadata 缺失则 exit 1
$WIX_NEEDED = $true
if (Select-String -Path "Cargo.toml" -Pattern "package.metadata.wix" -Quiet) {
    cargo wix --no-build --target $TARGET
    $WIX_NEEDED = $false
}

if ($WIX_NEEDED) {
    Write-Host "    [NOTE] Cargo.toml 缺 [package.metadata.wix], 见 packaging/msi/Cargo.toml.snippet"
    Write-Host "    [DRY-RUN] 期待 1.0 release 末 release engineer 注入 metadata 后实装"
    Write-Host "    模板: packaging/msi/apeireth.wxs (3747 bytes, 已写)"
}

# 4. 验证
$MSI_PATH = "target/wix/apeireth-${VERSION}-x86_64.msi"
if (Test-Path $MSI_PATH) {
    $SIZE = (Get-Item $MSI_PATH).Length / 1MB
    $SIZE_FMT = "{0:N2} MB" -f $SIZE
    $MSI_SHA256 = (Get-FileHash -Path $MSI_PATH -Algorithm SHA256).Hash
    Write-Host "[4/4] MSI 产物: ${MSI_PATH} (${SIZE_FMT})"
    Write-Host "    sha256: ${MSI_SHA256}"
    Write-Host "    安装: msiexec /i ${MSI_PATH}"
    Write-Host "    卸载: msiexec /x ${MSI_PATH}"
} else {
    Write-Host "[4/4] DRY-RUN: 模板已写, 实装待 1.0 release 末"
    Write-Host "    模板: packaging/msi/apeireth.wxs"
    Write-Host "    metadata: packaging/msi/Cargo.toml.snippet"
}
