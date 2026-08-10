# Apeireth zip 通用包 (8 包之 1, D-06 拍板)
# 平台: Windows 通用 (解压即用, 不走 MSI)
# 工具: cargo build + 7zip
# 体积: ~50MB (单 .exe + LICENSE + config 模板)
#
# 用法:
#   .\packaging\zip\build.ps1
# 验证:
#   Expand-Archive apeireth-1.0.0-x86_64-pc-windows-msvc.zip
#   .\apeireth-1.0.0-x86_64-pc-windows-msvc\bin\apeireth.exe --version
# 卸载:
#   Remove-Item -Recurse C:\Program Files\Apeireth

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot\..\..

$VERSION = $env:APEIRETH_VERSION
if (-not $VERSION) { $VERSION = "1.0.0" }
$TARGET = $env:APEIRETH_TARGET
if (-not $TARGET) { $TARGET = "x86_64-pc-windows-msvc" }

Write-Host "=== apeireth zip build v${VERSION} (target=${TARGET}) ==="

# 1. cargo build
Write-Host "[1/5] cargo build --release --target ${TARGET}..."
cargo build --release --bin apeireth --target $TARGET --locked

# 2. 打包目录
$PACK_NAME = "apeireth-${VERSION}-${TARGET}"
$STAGE_DIR = Join-Path "target" "zip-stage" $PACK_NAME
if (Test-Path $STAGE_DIR) { Remove-Item $STAGE_DIR -Recurse -Force }
New-Item -ItemType Directory -Path (Join-Path $STAGE_DIR "bin") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $STAGE_DIR "config") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $STAGE_DIR "share") -Force | Out-Null

$EXE_SRC = "target/${TARGET}/release/apeireth.exe"
if (-not (Test-Path $EXE_SRC)) {
    throw "binary not found: ${EXE_SRC}, 期待 cargo build 成功"
}
Copy-Item $EXE_SRC (Join-Path $STAGE_DIR "bin" "apeireth.exe")
Copy-Item "LICENSE" (Join-Path $STAGE_DIR "share" "LICENSE")
Copy-Item "README.md" (Join-Path $STAGE_DIR "share" "README.md")
if (Test-Path "CHANGELOG.md") {
    Copy-Item "CHANGELOG.md" (Join-Path $STAGE_DIR "share" "CHANGELOG.md")
}

# 3. config 模板 + 启动脚本
@"
# Apeireth OS — 环境变量示例
APEIRETH_HOME=%USERPROFILE%\.apeireth
APEIRETH_CONFIG=%APEIRETH_HOME%\config.toml
APEIRETH_DB_URL=postgresql://apeireth:secret@localhost:5432/apeireth
APEIRETH_REDIS_URL=redis://localhost:6379/0
APEIRETH_LLM_BACKEND=scripted
APEIRETH_LLM_API_URL=https://api.minimaxi.com
APEIRETH_LLM_MODEL=MiniMax-M3
"@ | Out-File -FilePath (Join-Path $STAGE_DIR "config" "apeireth.env.example") -Encoding UTF8

@"
@echo off
rem Apeireth OS — Windows 启动脚本 (nssm 安装为服务见 docs/)
setlocal
set "APEIRETH_HOME=%USERPROFILE%\.apeireth"
if not exist "%APEIRETH_HOME%" mkdir "%APEIRETH_HOME%"
"%~dp0apeireth.exe" serve
endlocal
"@ | Out-File -FilePath (Join-Path $STAGE_DIR "bin" "apeireth-serve.bat") -Encoding ASCII

@"
Apeireth OS ${VERSION} — Windows 通用包 (解压即用)

安装:
  1. 解压到 C:\Program Files\Apeireth
  2. 把 C:\Program Files\Apeireth\bin 加到 PATH
  3. (可选) 用 nssm 注册 Windows 服务:
       nssm install Apeireth "C:\Program Files\Apeireth\bin\apeireth.exe" serve
       nssm start Apeireth

验证:
  curl http://localhost:8080/health

卸载:
  nssm stop Apeireth ; nssm remove Apeireth confirm
  Remove-Item -Recurse "C:\Program Files\Apeireth"
"@ | Out-File -FilePath (Join-Path $STAGE_DIR "README.txt") -Encoding UTF8

# 4. 打 zip
Write-Host "[2/5] Compress-Archive..."
$ZIP_PATH = "target/${PACK_NAME}.zip"
if (Test-Path $ZIP_PATH) { Remove-Item $ZIP_PATH -Force }
Compress-Archive -Path $STAGE_DIR -DestinationPath $ZIP_PATH -CompressionLevel Optimal

# 5. sha256
Write-Host "[3/5] sha256..."
$ZIP_SHA256 = (Get-FileHash -Path $ZIP_PATH -Algorithm SHA256).Hash
"${ZIP_SHA256}  ${ZIP_PATH}" | Out-File -FilePath "${ZIP_PATH}.sha256" -Encoding UTF8

# 6. 验证
$SIZE = (Get-Item $ZIP_PATH).Length / 1MB
$SIZE_FMT = "{0:N2} MB" -f $SIZE
Write-Host "[4/5] zip 产物: ${ZIP_PATH} (${SIZE_FMT})"
Write-Host "    sha256: ${ZIP_SHA256}"
Write-Host "    解压: Expand-Archive ${ZIP_PATH}"
Write-Host "    二进制: ${STAGE_DIR}\bin\apeireth.exe"
Write-Host "[5/5] DONE"
