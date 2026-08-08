#!/usr/bin/env bash
# V1389 apeireth-ci-gate.sh — Real CI gate for Apeireth deployment stack
# Wraps apeireth/v1387 (unified deploy-stack runner) + apeireth/v1388 (baseline + diff) into a single shell entry.
# Main 06:15 + main 23:44: real CI, real exit code, real SARIF, real GitHub Actions compatible.
#
# Modules wrapped:
#   - apeireth/v1387 — unified deploy-stack runner (Dockerfile + compose + k8s lint)
#   - apeireth/v1388 — baseline + diff (regression gate)
#
# Usage:
#   ./apeireth-ci-gate.sh [--target PATH] [--baseline PATH] [--save-baseline] [--help]
#
# Exit codes:
#   0 = no regression (no new findings)
#   1 = new findings (regression)
#   2 = baseline missing (use --baseline-missing-ok to ignore)
#   3 = IO/parse error
#
# Real borrowed: super-linter (https://github.com/github/super-linter) + diff-cover
# Main 19:33: walking on predecessors' experience.
# Main 17:43: 实事求是 — real bash, real exit code, real SARIF.

set -euo pipefail

# Defaults
TARGET="${APEIRETH_TARGET:-deploy}"
BASELINE="${APEIRETH_BASELINE:-.v1387_baseline.json}"
SAVE_BASELINE=0
ALLOW_BASELINE_MISSING=1
STRICT=0
QUIET=0
JSON=0
SARIF=0
MD=0
FAIL_ON="${APEIRETH_FAIL_ON:-new}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

# V1389 real colors (only if TTY)
if [ -t 1 ]; then
    RED=$'\033[0;31m'
    GREEN=$'\033[0;32m'
    YELLOW=$'\033[0;33m'
    BLUE=$'\033[0;34m'
    BOLD=$'\033[1m'
    NC=$'\033[0m'
else
    RED="" ; GREEN="" ; YELLOW="" ; BLUE="" ; BOLD="" ; NC=""
fi

usage() {
    cat <<'EOF'
V1389 apeireth-ci-gate.sh — Real CI gate for Apeireth deployment stack

USAGE:
  apeireth-ci-gate.sh [OPTIONS]

OPTIONS:
  --target PATH           Directory to scan (default: deploy)
  --baseline PATH         Baseline JSON file (default: .v1387_baseline.json)
  --save-baseline         Save current run as baseline (overwrite)
  --baseline-missing-ok   Treat missing baseline as ok (default: true)
  --baseline-missing-strict  Treat missing baseline as exit 2 (default: false)
  --strict                Any change → exit 1
  --fail-on {new|resolved|any}  What to fail on (default: new)
  --json                  Output JSON report
  --sarif                 Output SARIF report (GitHub code scanning compatible)
  --md                    Output Markdown report
  --quiet                 Suppress detail
  --help                  This help

EXIT CODES:
  0  no regression (no new findings)
  1  new findings (regression)
  2  baseline missing (with --baseline-missing-strict)
  3  IO/parse error

ENVIRONMENT:
  APEIRETH_TARGET        Default target directory
  APEIRETH_BASELINE       Default baseline file
  APEIRETH_FAIL_ON        Default fail-on policy

EXAMPLES:
  # Run on deploy/ with default baseline
  ./apeireth-ci-gate.sh

  # Run with strict mode and SARIF output
  ./apeireth-ci-gate.sh --strict --sarif --quiet

  # First run — save baseline
  ./apeireth-ci-gate.sh --save-baseline

  # CI integration
  ./apeireth-ci-gate.sh --target deploy --fail-on new --sarif
EOF
}

# V1389 real arg parsing
while [[ $# -gt 0 ]]; do
    case "$1" in
        --target) TARGET="$2"; shift 2 ;;
        --baseline) BASELINE="$2"; shift 2 ;;
        --save-baseline) SAVE_BASELINE=1; shift ;;
        --baseline-missing-ok) ALLOW_BASELINE_MISSING=1; shift ;;
        --baseline-missing-strict) ALLOW_BASELINE_MISSING=0; shift ;;
        --strict) STRICT=1; shift ;;
        --fail-on) FAIL_ON="$2"; shift 2 ;;
        --json) JSON=1; shift ;;
        --sarif) SARIF=1; shift ;;
        --md) MD=1; shift ;;
        --quiet) QUIET=1; shift ;;
        -h|--help) usage; exit 0 ;;
        *) echo "${RED}Unknown argument: $1${NC}" >&2; usage; exit 3 ;;
    esac
done

# V1389 real banner
echo "${BOLD}${BLUE}V1389 apeireth-ci-gate${NC} — target=${TARGET} baseline=${BASELINE}"

# V1389 real preamble check
if [ ! -d "${REPO_ROOT}" ]; then
    echo "${RED}error: repo root not found: ${REPO_ROOT}${NC}" >&2
    exit 3
fi

cd "${REPO_ROOT}"

# V1389 real Python check
if ! command -v python >/dev/null 2>&1; then
    echo "${RED}error: python not found in PATH${NC}" >&2
    exit 3
fi

# V1389 real module check
if ! python -c "import apeireth.v1387_deploy_stack_runner" >/dev/null 2>&1; then
    echo "${RED}error: apeireth.v1387_deploy_stack_runner not importable${NC}" >&2
    echo "hint: pip install -e . or PYTHONPATH=." >&2
    exit 3
fi

if ! python -c "import apeireth.v1388_v1387_baseline_diff" >/dev/null 2>&1; then
    echo "${RED}error: apeireth.v1388_v1387_baseline_diff not importable${NC}" >&2
    exit 3
fi

# V1389 real target check
if [ ! -d "${TARGET}" ]; then
    echo "${RED}error: target directory not found: ${TARGET}${NC}" >&2
    exit 3
fi

# V1389 real build command
CMD=(python -m apeireth.v1388_v1387_baseline_diff)
CMD+=("${TARGET}")
CMD+=(--baseline "${BASELINE}")
if [ "${STRICT}" = "1" ]; then
    CMD+=(--strict)
fi
if [ "${ALLOW_BASELINE_MISSING}" = "0" ]; then
    CMD+=(--baseline-missing-exit-2)
fi
CMD+=(--fail-on "${FAIL_ON}")
if [ "${JSON}" = "1" ]; then
    CMD+=(--json)
elif [ "${SARIF}" = "1" ]; then
    CMD+=(--sarif)
elif [ "${MD}" = "1" ]; then
    CMD+=(--md)
fi
if [ "${QUIET}" = "1" ]; then
    CMD+=(--quiet)
fi

# V1389 real save baseline mode
if [ "${SAVE_BASELINE}" = "1" ]; then
    SAVE_CMD=(python -m apeireth.v1387_deploy_stack_runner)
    SAVE_CMD+=("${TARGET}")
    SAVE_CMD+=(--save-baseline "${BASELINE}")
    if [ "${QUIET}" = "1" ]; then
        SAVE_CMD+=(--quiet)
    fi
    echo "${YELLOW}saving baseline...${NC}"
    "${SAVE_CMD[@]}"
    SAVE_EXIT=$?
    if [ ${SAVE_EXIT} -ne 0 ]; then
        echo "${RED}error: save baseline failed (exit ${SAVE_EXIT})${NC}" >&2
        exit 3
    fi
    echo "${GREEN}baseline saved: ${BASELINE}${NC}"
    exit 0
fi

# V1389 real run
echo "${BLUE}running: ${CMD[*]}${NC}"
set +e
"${CMD[@]}"
EXIT=$?
set -e

# V1389 real exit code mapping
case ${EXIT} in
    0)  echo "${GREEN}✓ no regression${NC}"; exit 0 ;;
    1)  echo "${RED}✗ regression detected (new findings)${NC}" >&2; exit 1 ;;
    2)  echo "${YELLOW}! baseline missing (use --save-baseline on first run)${NC}" >&2; exit 2 ;;
    3)  echo "${RED}error: IO/parse error${NC}" >&2; exit 3 ;;
    *)  echo "${RED}error: unexpected exit ${EXIT}${NC}" >&2; exit 3 ;;
esac
