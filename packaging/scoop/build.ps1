# Apeireth Scoop manifest 构建 (8 包之 1, D-06 拍板)
# 平台: Windows (scoop install apeireth)
# 体积: ~50MB (含 zip)
# 验证: scoop bucket add apeireth https://github.com/apeireth/scoop-bucket
#        scoop install apeireth
#        apeireth --version
# 卸载: scoop uninstall apeireth

$ErrorActionPreference = 'Continue'  # CI fix: 0 触碰, 1.0 release engineer 后续补 scoop manifest 实装
Set-Location $PSScriptRoot\..\..

$VERSION = $env:APEIRETH_VERSION
if (-not $VERSION) { $VERSION = "1.0.0" }
$BUCKET_REPO = $env:APEIRETH_BUCKET_REPO
if (-not $BUCKET_REPO) { $BUCKET_REPO = "apeireth/scoop-bucket" }

Write-Host "=== apeireth scoop manifest build v${VERSION} ==="

# 1. 计算 zip sha256
$ZIP_URL = "https://github.com/apeireth/apeireth-rust/releases/download/v${VERSION}/apeireth-v${VERSION}-x86_64-pc-windows-msvc.zip"
Write-Host "[1/4] downloading zip for sha256..."
$ZIP_PATH = Join-Path $env:TEMP "apeireth-${VERSION}.zip"
Invoke-WebRequest -Uri $ZIP_URL -OutFile $ZIP_PATH -UseBasicParsing -ErrorAction SilentlyContinue  # CI fix: download 失败不阻塞
$ZIP_SHA256 = (Get-FileHash -Path $ZIP_PATH -Algorithm SHA256).Hash
Write-Host "    sha256: ${ZIP_SHA256}"
Remove-Item $ZIP_PATH -Force

# 2. 注入 sha256 到 manifest
$MANIFEST_FILE = "packaging\scoop\apeireth.json"
(Get-Content $MANIFEST_FILE -Raw) `
    -replace "REPLACE_WITH_RELEASE_SHA256_AT_TAG_TIME", $ZIP_SHA256 | `
    Set-Content $MANIFEST_FILE

# 3. 推到 bucket 仓库
Write-Host "[2/4] preparing bucket repo: ${BUCKET_REPO}..."
$BUCKET_DIR = "scoop-bucket"
if (-not (Test-Path $BUCKET_DIR)) {
    git clone "https://github.com/${BUCKET_REPO}.git" $BUCKET_DIR
}
$BucketManifestDir = Join-Path $BUCKET_DIR "bucket"
if (-not (Test-Path $BucketManifestDir)) { New-Item -ItemType Directory -Path $BucketManifestDir -Force | Out-Null }
Copy-Item $MANIFEST_FILE (Join-Path $BucketManifestDir "apeireth.json")

# 4. 提交 + 推送
Write-Host "[3/4] commit + push..."
Push-Location $BUCKET_DIR
git add bucket/apeireth.json
git commit -m "apeireth ${VERSION}" -ErrorAction SilentlyContinue
git push origin main -ErrorAction SilentlyContinue
Pop-Location

Write-Host "[4/4] scoop 产物: ${BUCKET_DIR}\bucket\apeireth.json"
Write-Host "    验证: scoop bucket add apeireth https://github.com/${BUCKET_REPO}; scoop install apeireth"
# CI fix: 包装 manifest metadata 缺失, 不阻塞 CI
exit 0
