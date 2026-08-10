#!/usr/bin/env bash
# =============================================================================
# packaging/tarball/build-tarball.sh
#
# tarball 的 build script (per task spec 1.0 release #4)
# 主脚本 packaging/tarball/build.sh 已有, 本脚本是 alias 满足 task spec 命名
#
# 决策: D-06 (8 包齐发)
# 用法:
#   ./packaging/tarball/build-tarball.sh                    # 默认 x86_64 + musl
#   APEIRETH_TARGET=aarch64-unknown-linux-musl ./packaging/tarball/build-tarball.sh
# =============================================================================

set -euo pipefail
cd "$(dirname "$0")/../.."
exec ./packaging/tarball/build.sh "$@"
