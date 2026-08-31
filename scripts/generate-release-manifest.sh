#!/usr/bin/env bash
# ==============================================================================
# scripts/generate-release-manifest.sh — Apeireth 2.0 RC Release Manifest Generator
# ==============================================================================
# Computes SHA256SUMS and generates dist/release-manifest.json for staged artifacts.
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

DIST_DIR="${1:-${REPO_ROOT}/dist}"
VERSION="${2:-}"
COMMIT_SHA="${3:-}"

mkdir -p "${DIST_DIR}"

if [[ -z "${VERSION}" ]]; then
  if [[ -f "${REPO_ROOT}/frontend/companion-desktop/package.json" ]]; then
    VERSION="$(grep -E '"version":' "${REPO_ROOT}/frontend/companion-desktop/package.json" | head -1 | sed -E 's/.*"version":\s*"([^"]+)".*/\1/')"
  elif [[ -f "${REPO_ROOT}/Cargo.toml" ]]; then
    VERSION="$(grep -E '^\s*version\s*=' "${REPO_ROOT}/Cargo.toml" | head -1 | sed -E 's/.*version\s*=\s*"([^"]+)".*/\1/')"
  else
    VERSION="2.0.0-rc.1"
  fi
fi

RELEASE_TAG="v${VERSION#v}"

if [[ -z "${COMMIT_SHA}" ]]; then
  COMMIT_SHA="$(git -C "${REPO_ROOT}" rev-parse HEAD 2>/dev/null || echo "UNKNOWN")"
fi

echo "================================================================================"
echo "             APEIRETH 2.0 RC RELEASE MANIFEST & CHECKSUM GENERATOR              "
echo "================================================================================"
echo "  Release Version:    ${VERSION}"
echo "  Release Tag:        ${RELEASE_TAG}"
echo "  Commit SHA:         ${COMMIT_SHA}"
echo "  Dist Directory:     ${DIST_DIR}"
echo "--------------------------------------------------------------------------------"

SHA256SUMS_FILE="${DIST_DIR}/SHA256SUMS"
MANIFEST_FILE="${DIST_DIR}/release-manifest.json"

> "${SHA256SUMS_FILE}"

ARTIFACTS_JSON="[]"

cd "${DIST_DIR}"
for f in *; do
  [[ ! -f "$f" ]] && continue
  [[ "$f" == "SHA256SUMS"* ]] && continue
  [[ "$f" == "release-manifest.json"* ]] && continue

  if command -v sha256sum >/dev/null 2>&1; then
    HASH="$(sha256sum "$f" | awk '{print $1}')"
  elif command -v shasum >/dev/null 2>&1; then
    HASH="$(shasum -a 256 "$f" | awk '{print $1}')"
  else
    HASH="UNSUPPORTED_HASH"
  fi

  echo "${HASH}  ${f}" >> "${SHA256SUMS_FILE}"
  SIZE="$(wc -c < "$f" | tr -d ' ')"

  PLATFORM="agnostic"
  ARCH="any"
  TYPE="binary"

  case "$f" in
    *linux*|*.deb|*.rpm|*musl*) PLATFORM="linux" ;;
    *windows*|*.msi|*.exe|*.zip) PLATFORM="windows" ;;
    *darwin*|*macos*|*.dmg|*.rb) PLATFORM="macos" ;;
  esac

  case "$f" in
    *x86_64*|*amd64*|*x64*) ARCH="x86_64" ;;
    *aarch64*|*arm64*) ARCH="aarch64" ;;
    *universal*) ARCH="universal" ;;
  esac

  case "$f" in
    *.deb) TYPE="deb-package" ;;
    *.rpm) TYPE="rpm-package" ;;
    *.msi) TYPE="msi-installer" ;;
    *.dmg) TYPE="dmg-installer" ;;
    *.AppImage) TYPE="appimage" ;;
    *.tar.gz) TYPE="tarball-archive" ;;
    *.zip) TYPE="zip-archive" ;;
    *.rb) TYPE="homebrew-formula" ;;
    *.json) TYPE="json-manifest" ;;
    *.md) TYPE="documentation-notice" ;;
  esac

  if command -v jq >/dev/null 2>&1; then
    ENTRY="$(jq -n \
      --arg fn "$f" \
      --argjson sz "$SIZE" \
      --arg h "$HASH" \
      --arg p "$PLATFORM" \
      --arg a "$ARCH" \
      --arg t "$TYPE" \
      '{filename: $fn, size_bytes: $sz, sha256: $h, platform: $p, arch: $a, type: $t}')"
    ARTIFACTS_JSON="$(jq --argjson entry "$ENTRY" '. + [$entry]' <<< "$ARTIFACTS_JSON")"
  fi

  printf "  -> %-45s (SHA: %s...)\n" "$f" "${HASH:0:16}"
done

ISO_TIMESTAMP="$(date -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || echo "2026-08-30T12:00:00Z")"
WORKSPACE_VERSION="$(sed -n '/^\[workspace\.package\]/,/^\[/p' "${REPO_ROOT}/Cargo.toml" | grep -E '^version\s*=' | head -1 | sed -E 's/.*version\s*=\s*"([^"]+)".*/\1/')"
DESKTOP_VERSION="$(sed -n 's/.*"version": "\([^"]*\)".*/\1/p' "${REPO_ROOT}/frontend/companion-desktop/package.json" | head -1)"
if [[ -z "${WORKSPACE_VERSION}" || -z "${DESKTOP_VERSION}" || "${WORKSPACE_VERSION}" != "${VERSION}" || "${DESKTOP_VERSION}" != "${VERSION}" ]]; then
  echo "release version mismatch: workspace=${WORKSPACE_VERSION:-UNKNOWN} desktop=${DESKTOP_VERSION:-UNKNOWN} requested=${VERSION}" >&2
  exit 1
fi

if command -v jq >/dev/null 2>&1; then
    jq -n \
    --arg name "Apeireth 2.0 Release Candidate" \
    --arg ver "$VERSION" \
    --arg tag "$RELEASE_TAG" \
    --arg sha "$COMMIT_SHA" \
    --arg ts "$ISO_TIMESTAMP" \
    --arg workspace "$WORKSPACE_VERSION" \
    --arg desktop "$DESKTOP_VERSION" \
    --argjson artifacts "$ARTIFACTS_JSON" \
    '{
      schema_version: "2.0.0",
      release: {
        name: $name,
        version: $ver,
        tag: $tag,
        commit_sha: $sha,
        created_at: $ts,
        generator: "scripts/generate-release-manifest.sh"
      },
      components: {
        workspace: $workspace,
        cli: $workspace,
        gateway: $workspace,
        sdk: $workspace,
        companion_desktop: $desktop
      },
      artifacts: $artifacts,
      checksums: {
        sha256sums_file: "SHA256SUMS",
        total_artifacts: ($artifacts | length)
      },
      metadata: {
        sbom_standard: "CycloneDX 1.5 JSON",
        license: "Apache-2.0",
        notices: "THIRD-PARTY-NOTICES.md"
      }
    }' > "${MANIFEST_FILE}"
  echo "  Generated JSON release manifest: ${MANIFEST_FILE}"
else
  # Fallback raw JSON writer
  cat <<EOF > "${MANIFEST_FILE}"
{
  "schema_version": "2.0.0",
  "release": {
    "name": "Apeireth 2.0 Release Candidate",
    "version": "${VERSION}",
    "tag": "${RELEASE_TAG}",
    "commit_sha": "${COMMIT_SHA}",
    "created_at": "${ISO_TIMESTAMP}",
    "generator": "scripts/generate-release-manifest.sh"
  },
  "components": {
    "workspace": "${WORKSPACE_VERSION}",
    "cli": "${WORKSPACE_VERSION}",
    "gateway": "${WORKSPACE_VERSION}",
    "sdk": "${WORKSPACE_VERSION}",
    "companion_desktop": "${DESKTOP_VERSION}"
  },
  "checksums": {
    "sha256sums_file": "SHA256SUMS"
  },
  "metadata": {
    "sbom_standard": "CycloneDX 1.5 JSON",
    "license": "Apache-2.0",
    "notices": "THIRD-PARTY-NOTICES.md"
  }
}
EOF
  echo "  Generated fallback release manifest: ${MANIFEST_FILE}"
fi

echo "================================================================================"
echo "  Manifest and SHA256SUMS generation complete."
echo "================================================================================"
