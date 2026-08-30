# Apeireth Scoop Manifest Build Script (APEIRETH 2.0 RC)
# Platform: Windows (scoop install apeireth)
#
# Usage:
#   .\packaging\scoop\build.ps1
#   $env:APEIRETH_VERSION = "2.0.0-rc.1"; .\packaging\scoop\build.ps1

$ErrorActionPreference = 'Continue'
Set-Location $PSScriptRoot\..\..

$VERSION = $env:APEIRETH_VERSION
if (-not $VERSION) { $VERSION = "2.0.0-rc.1" }
$BUCKET_REPO = $env:APEIRETH_BUCKET_REPO
if (-not $BUCKET_REPO) { $BUCKET_REPO = "apeireth/scoop-bucket" }

Write-Host "=== Apeireth Scoop Manifest Build v${VERSION} ==="

# 1. Compute zip SHA256 (from local target or remote release)
$LOCAL_ZIP = "target\apeireth-${VERSION}-windows-x86_64.zip"
$ZIP_SHA256 = $null

if (Test-Path $LOCAL_ZIP) {
    Write-Host "[1/4] Using local ZIP artifact for sha256: ${LOCAL_ZIP}"
    $ZIP_SHA256 = (Get-FileHash -Path $LOCAL_ZIP -Algorithm SHA256).Hash
} else {
    $ZIP_URL = "https://github.com/apeireth/apeireth-rust/releases/download/v${VERSION}/apeireth-${VERSION}-windows-x86_64.zip"
    Write-Host "[1/4] Attempting to download ZIP for sha256: ${ZIP_URL}"
    $TEMP_ZIP_PATH = Join-Path $env:TEMP "apeireth-${VERSION}.zip"
    try {
        Invoke-WebRequest -Uri $ZIP_URL -OutFile $TEMP_ZIP_PATH -UseBasicParsing -TimeoutSec 10
        if (Test-Path $TEMP_ZIP_PATH) {
            $ZIP_SHA256 = (Get-FileHash -Path $TEMP_ZIP_PATH -Algorithm SHA256).Hash
            Remove-Item $TEMP_ZIP_PATH -Force
        }
    } catch {
        Write-Host "    [NOTE] Remote release not reachable yet; placeholder preserved for release pipeline."
    }
}

if (-not $ZIP_SHA256) {
    $ZIP_SHA256 = "REPLACE_WITH_RELEASE_SHA256_AT_TAG_TIME"
}

Write-Host "    SHA256: ${ZIP_SHA256}"

# 2. Inject version & sha256 into manifest
$MANIFEST_FILE = "packaging\scoop\apeireth.json"
$content = Get-Content $MANIFEST_FILE -Raw
$content = $content -replace '"version":\s*"[^"]+"', "`"version`": `"${VERSION}`""
if ($ZIP_SHA256 -ne "REPLACE_WITH_RELEASE_SHA256_AT_TAG_TIME") {
    $content = $content -replace '"hash":\s*"[^"]+"', "`"hash`": `"${ZIP_SHA256}`""
}
$content | Set-Content $MANIFEST_FILE -Encoding UTF8
Write-Host "[2/4] Updated ${MANIFEST_FILE} with version ${VERSION} and hash."

# 3. Prepare bucket repo staging
Write-Host "[3/4] Preparing bucket repo: ${BUCKET_REPO}..."
$BUCKET_DIR = "target\scoop-bucket"
if (Test-Path $BUCKET_DIR) { Remove-Item $BUCKET_DIR -Recurse -Force }
$BucketManifestDir = Join-Path $BUCKET_DIR "bucket"
New-Item -ItemType Directory -Path $BucketManifestDir -Force | Out-Null
Copy-Item $MANIFEST_FILE (Join-Path $BucketManifestDir "apeireth.json") -Force

# 4. Report
Write-Host "[4/4] Scoop manifest staged:"
Write-Host "    Manifest: ${MANIFEST_FILE}"
Write-Host "    Staged:   ${BucketManifestDir}\apeireth.json"
Write-Host "    Usage:    scoop bucket add apeireth https://github.com/${BUCKET_REPO}; scoop install apeireth"

exit 0

