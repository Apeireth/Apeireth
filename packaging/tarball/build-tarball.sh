#!/usr/bin/env bash
# =============================================================================
# packaging/tarball/build-tarball.sh
#
# tarball 的 build script (Apeireth 2.0.0-rc.1)
# 产出目标: apeireth-2.0.0-rc.1-linux-x86_64.tar.gz
# 主脚本 packaging/tarball/build.sh 已有, 本脚本是 alias 满足 spec 命名
#
# 用法:
#   ./packaging/tarball/build-tarball.sh                    # 默认 x86_64 + musl
#   APEIRETH_TARGET=aarch64-unknown-linux-musl ./packaging/tarball/build-tarball.sh
# =============================================================================

set -euo pipefail
cd "$(dirname "$0")/../.."
exec ./packaging/tarball/build.sh "$@"

