#!/usr/bin/env bash
# ==============================================================================
# scripts/check-release-version.sh — Apeireth Release Version Validator
# ==============================================================================
# Validates release version consistency across all authoritative sources:
#   1. Root Cargo.toml ([workspace.package].version)
#   2. 16 Workspace Crates (crates/*/*/Cargo.toml -> version.workspace = true)
#      - CLI, Gateway, SDK, Core, Runtime, Storage, Memory, Organ, Tools, etc.
#   3. Desktop UI (frontend/companion-desktop/package.json)
#   4. Desktop Tauri Shell (frontend/companion-desktop/src-tauri/Cargo.toml)
#   5. Desktop Tauri Config (frontend/companion-desktop/src-tauri/tauri.conf.json)
#
# Usage:
#   bash scripts/check-release-version.sh
#   bash scripts/check-release-version.sh --expected 1.2.0
#   bash scripts/check-release-version.sh --expected 2.0.0-rc.1 --strict
# ==============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

EXPECTED_VERSION=""
STRICT_MODE=0
HAS_ERRORS=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --expected|-e)
      EXPECTED_VERSION="$2"
      shift 2
      ;;
    --strict|-s)
      STRICT_MODE=1
      shift
      ;;
    *)
      if [[ -z "$EXPECTED_VERSION" ]]; then
        EXPECTED_VERSION="$1"
      fi
      shift
      ;;
  esac
done

echo "================================================================================"
echo "                  APEIRETH RELEASE VERSION CONSISTENCY REPORT                   "
echo "================================================================================"
echo "  Repository Root:    ${REPO_ROOT}"

# 1. Root Cargo.toml
ROOT_CARGO="${REPO_ROOT}/Cargo.toml"
ROOT_VERSION=""
if [[ -f "$ROOT_CARGO" ]]; then
  ROOT_VERSION="$(grep -E '^\s*version\s*=' "$ROOT_CARGO" | head -1 | sed -E 's/.*version\s*=\s*"([^"]+)".*/\1/')"
  if [[ -n "$ROOT_VERSION" ]]; then
    echo "  Workspace Version:  ${ROOT_VERSION}"
    if [[ -n "$EXPECTED_VERSION" ]] && [[ "$ROOT_VERSION" != "$EXPECTED_VERSION" ]]; then
      echo "  [FAIL] Root Cargo.toml version (${ROOT_VERSION}) != Expected (${EXPECTED_VERSION})"
      HAS_ERRORS=1
    else
      echo "  [PASS] Root Cargo.toml: ${ROOT_VERSION}"
    fi
  else
    echo "  [FAIL] Root Cargo.toml: Missing [workspace.package].version"
    HAS_ERRORS=1
  fi
else
  echo "  [FAIL] Root Cargo.toml: File not found"
  HAS_ERRORS=1
fi

echo "--------------------------------------------------------------------------------"
echo ">>> Checking 16 Workspace Crates..."

CRATES_DIR="${REPO_ROOT}/crates"
if [[ -d "$CRATES_DIR" ]]; then
  while IFS= read -r toml; do
    REL_PATH="${toml#"${REPO_ROOT}/"}"
    CRATE_NAME="$(grep -E '^\s*name\s*=' "$toml" | head -1 | sed -E 's/.*name\s*=\s*"([^"]+)".*/\1/')"
    if grep -q 'version.workspace\s*=\s*true' "$toml"; then
      printf "  [PASS] %-32s -> Inherits workspace (%s)\n" "$CRATE_NAME" "$ROOT_VERSION"
    else
      EXPLICIT_VER="$(grep -E '^\s*version\s*=' "$toml" | head -1 | sed -E 's/.*version\s*=\s*"([^"]+)".*/\1/')"
      if [[ "$EXPLICIT_VER" == "$ROOT_VERSION" ]]; then
        printf "  [PASS] %-32s -> Explicit version matches (%s)\n" "$CRATE_NAME" "$EXPLICIT_VER"
      else
        printf "  [FAIL] %-32s -> Explicit version (%s) != Workspace (%s)\n" "$CRATE_NAME" "$EXPLICIT_VER" "$ROOT_VERSION"
        HAS_ERRORS=1
      fi
    fi
  done < <(find "$CRATES_DIR" -name "Cargo.toml" | sort)
fi

echo "--------------------------------------------------------------------------------"
echo ">>> Checking Companion Desktop Components..."

DESKTOP_PKG="${REPO_ROOT}/frontend/companion-desktop/package.json"
DESKTOP_PKG_VER=""
if [[ -f "$DESKTOP_PKG" ]]; then
  DESKTOP_PKG_VER="$(grep -E '"version":' "$DESKTOP_PKG" | head -1 | sed -E 's/.*"version":\s*"([^"]+)".*/\1/')"
  if [[ -n "$DESKTOP_PKG_VER" ]]; then
    if [[ $STRICT_MODE -eq 1 ]] && [[ -n "$ROOT_VERSION" ]] && [[ "$DESKTOP_PKG_VER" != "$ROOT_VERSION" ]]; then
      printf "  [FAIL] companion-desktop UI: %s (Strict mode: != Workspace %s)\n" "$DESKTOP_PKG_VER" "$ROOT_VERSION"
      HAS_ERRORS=1
    else
      printf "  [PASS] companion-desktop UI: %s (package.json)\n" "$DESKTOP_PKG_VER"
    fi
  fi
else
  echo "  [WARN] frontend/companion-desktop/package.json not found"
fi

DESKTOP_CARGO="${REPO_ROOT}/frontend/companion-desktop/src-tauri/Cargo.toml"
DESKTOP_CARGO_VER=""
if [[ -f "$DESKTOP_CARGO" ]]; then
  DESKTOP_CARGO_VER="$(grep -E '^\s*version\s*=' "$DESKTOP_CARGO" | head -1 | sed -E 's/.*version\s*=\s*"([^"]+)".*/\1/')"
  if [[ -n "$DESKTOP_CARGO_VER" ]]; then
    if [[ -n "$DESKTOP_PKG_VER" ]] && [[ "$DESKTOP_CARGO_VER" != "$DESKTOP_PKG_VER" ]]; then
      printf "  [FAIL] companion-desktop Tauri Shell: %s != package.json (%s)\n" "$DESKTOP_CARGO_VER" "$DESKTOP_PKG_VER"
      HAS_ERRORS=1
    else
      printf "  [PASS] companion-desktop Tauri Shell: %s (src-tauri/Cargo.toml)\n" "$DESKTOP_CARGO_VER"
    fi
  fi
else
  echo "  [WARN] frontend/companion-desktop/src-tauri/Cargo.toml not found"
fi

DESKTOP_TAURI="${REPO_ROOT}/frontend/companion-desktop/src-tauri/tauri.conf.json"
DESKTOP_TAURI_VER=""
if [[ -f "$DESKTOP_TAURI" ]]; then
  DESKTOP_TAURI_VER="$(grep -E '"version":' "$DESKTOP_TAURI" | head -1 | sed -E 's/.*"version":\s*"([^"]+)".*/\1/')"
  if [[ -n "$DESKTOP_TAURI_VER" ]]; then
    if [[ -n "$DESKTOP_PKG_VER" ]] && [[ "$DESKTOP_TAURI_VER" != "$DESKTOP_PKG_VER" ]]; then
      printf "  [FAIL] companion-desktop Tauri Config: %s != package.json (%s)\n" "$DESKTOP_TAURI_VER" "$DESKTOP_PKG_VER"
      HAS_ERRORS=1
    else
      printf "  [PASS] companion-desktop Tauri Config: %s (tauri.conf.json)\n" "$DESKTOP_TAURI_VER"
    fi
  fi
else
  echo "  [WARN] frontend/companion-desktop/src-tauri/tauri.conf.json not found"
fi

echo "================================================================================"
if [[ $HAS_ERRORS -ne 0 ]]; then
  echo "  RESULT: FAILED - Version inconsistencies detected!"
  echo "================================================================================"
  exit 1
else
  echo "  RESULT: SUCCESS - All checked version sources are consistent."
  echo "================================================================================"
  exit 0
fi
