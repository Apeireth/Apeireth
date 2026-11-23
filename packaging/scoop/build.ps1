# Apeireth Scoop Manifest Build Script (APEIRETH 2.0 RC)
# Platform: Windows (scoop install apeireth)
#
# Usage:
#   .\packaging\scoop\build.ps1
#   $env:APEIRETH_VERSION = "2.0.0-rc.5"; .\packaging\scoop\build.ps1

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot\..\..

$VERSION = $env:APEIRETH_VERSION
if (-not $VERSION) { $VERSION = "2.0.0-rc.5" }
$BUCKET_REPO = $env:APEIRETH_BUCKET_REPO
if (-not $BUCKET_REPO) { $BUCKET_REPO = "apeireth/scoop-bucket" }
$RELEASE_REPOSITORY = $env:GITHUB_REPOSITORY
if (-not $RELEASE_REPOSITORY) { $RELEASE_REPOSITORY = $env:APEIRETH_RELEASE_REPOSITORY }
if (-not $RELEASE_REPOSITORY) { $RELEASE_REPOSITORY = "Apeireth/Apeireth" }
if ($RELEASE_REPOSITORY -notmatch '^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$') {
    throw "Release repository must use the GitHub owner/repository form."
}

Write-Host "=== Apeireth Scoop Manifest Build v${VERSION} ==="

# 1. Compute zip SHA256 from the locally produced release artifact.
$LOCAL_ZIP = "target\apeireth-${VERSION}-windows-x86_64.zip"
if (-not (Test-Path $LOCAL_ZIP)) {
    throw "Local release ZIP is required: ${LOCAL_ZIP}. Build packaging\zip\build.ps1 first."
}
$ZIP_SHA256 = (Get-FileHash -Path $LOCAL_ZIP -Algorithm SHA256).Hash
Write-Host "[1/4] Using local ZIP artifact for sha256: ${LOCAL_ZIP}"

Write-Host "    SHA256: ${ZIP_SHA256}"

# 2. Inject version, canonical artifact URL and sha256 into manifest.
$MANIFEST_FILE = "packaging\scoop\apeireth.json"
$manifest = Get-Content $MANIFEST_FILE -Raw | ConvertFrom-Json
$repositoryUrl = "https://github.com/${RELEASE_REPOSITORY}"
$releaseZipUrl = "${repositoryUrl}/releases/download/v${VERSION}/apeireth-${VERSION}-windows-x86_64.zip"
$autoupdateUrl = "${repositoryUrl}/releases/download/v`$version/apeireth-`$version-windows-x86_64.zip"
$manifest.version = $VERSION
$manifest.homepage = $repositoryUrl
$manifest.architecture.'64bit'.url = $releaseZipUrl
$manifest.architecture.'64bit'.hash = $ZIP_SHA256
$manifest.checkver.github = $repositoryUrl
$manifest.autoupdate.architecture.'64bit'.url = $autoupdateUrl
$manifest | ConvertTo-Json -Depth 20 | Set-Content $MANIFEST_FILE -Encoding UTF8

# Re-read the serialized file and fail closed if its identity no longer binds to
# the exact ZIP built in this job.
$verified = Get-Content $MANIFEST_FILE -Raw | ConvertFrom-Json
if ($verified.version -ne $VERSION) { throw "Scoop manifest version mismatch" }
if ($verified.homepage -ne $repositoryUrl) { throw "Scoop manifest homepage mismatch" }
if ($verified.architecture.'64bit'.url -ne $releaseZipUrl) { throw "Scoop manifest URL mismatch" }
if ($verified.architecture.'64bit'.hash -ne $ZIP_SHA256) { throw "Scoop manifest hash mismatch" }
if ($verified.checkver.github -ne $repositoryUrl) { throw "Scoop manifest checkver repository mismatch" }
if ($verified.autoupdate.architecture.'64bit'.url -ne $autoupdateUrl) { throw "Scoop manifest autoupdate URL mismatch" }
Write-Host "[2/4] Bound ${MANIFEST_FILE} to the locally built ZIP."

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

