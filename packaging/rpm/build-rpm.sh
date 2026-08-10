#!/usr/bin/env bash
# =============================================================================
# packaging/rpm/build-rpm.sh
#
# rpm 的 build script (per task spec 1.0 release #4, 命名对齐)
# 主脚本 packaging/rpm/build.sh 已有, 本脚本是 alias 满足 task spec 命名
#
# 决策: D-06 (8 包齐发)
# 用法:
#   ./packaging/rpm/build-rpm.sh
# =============================================================================

set -euo pipefail
cd "$(dirname "$0")/../.."
exec ./packaging/rpm/build.sh "$@"
