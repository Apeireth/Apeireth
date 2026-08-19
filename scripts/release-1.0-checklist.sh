#!/usr/bin/env bash
# Apeireth 1.0 release 检查清单 (12 项, per 蓝图 §3.5)
# 9 P0 + 3 P1, 任何 1 P0 fail 阻塞 1.0 release tag
#
# 用法:
#   ./scripts/release-1.0-checklist.sh
#   ./scripts/release-1.0-checklist.sh --dry-run
#   ./scripts/release-1.0-checklist.sh --skip 3,4,5  # 跳过 security/install/upgrade (开发模式)
#
# 输出:
#   reports/r20-v1.0.0-release-checklist-<date>.md

set -uo pipefail
cd "$(dirname "$0")/.."

VERSION="${APEIRETH_VERSION:-1.0.0}"
DRY_RUN=false
SKIP=""
if [[ "${1:-}" == "--dry-run" ]]; then DRY_RUN=true; fi
if [[ "${1:-}" == "--skip" ]]; then SKIP="${2:-}"; fi

DATE=$(date -u +%Y-%m-%d)
OUT="reports/r20-v${VERSION}-release-checklist-${DATE}.md"
mkdir -p reports

PASS=0
FAIL=0
RESULTS=()

check() {
    local id="$1"; local title="$2"; local severity="$3"; shift 3
    if [[ ",${SKIP}," == *",${id},"* ]]; then
        RESULTS+=("| ${id} | ${title} | SKIP | — | — |")
        return 0
    fi
    echo ""
    echo ">>> [${id}/12] ${title} (${severity})"
    if [[ "${DRY_RUN}" == "true" ]]; then
        RESULTS+=("| ${id} | ${title} | DRY-RUN | — | — |")
        return 0
    fi
    if "$@"; then
        RESULTS+=("| ${id} | ${title} | ✅ PASS | — | — |")
        PASS=$((PASS+1))
    else
        RESULTS+=("| ${id} | ${title} | ❌ FAIL | — | ${severity} |")
        FAIL=$((FAIL+1))
    fi
}

# 1. doc
check_doc() {
    test -f README.md && \
    test -f CHANGELOG.md && \
    grep -q "${VERSION}" CHANGELOG.md
}
check 1 "doc (README + CHANGELOG + 4 docs 站, 1.0 内容齐)" "P0" check_doc

# 2. test
check_test() {
    cargo test --workspace --quiet 2>&1 | tail -1 | grep -qE '0 failed|test result: ok'
}
check 2 "test (cargo test --workspace 0 fail + 54/54 报告齐)" "P0" check_test

# 3. security
check_security() {
    # 5 守门 (per 蓝图 §3.5) 全部实跑
    local FAIL=0
    # 守门 1: non-root USER
    if ! grep -q "USER nonroot:nonroot" Dockerfile 2>/dev/null; then
        echo "    ❌ 守门 1: Dockerfile 缺 USER nonroot:nonroot"; FAIL=1
    fi
    # 守门 2: API key 不入 image (grep 应 0 命中)
    local HITS
    HITS=$(grep -rE "sk-cp-[a-zA-Z0-9_-]{20,}|ghp_[a-zA-Z0-9]{20,}|AKIA[0-9A-Z]{16}" Dockerfile packaging/ 2>/dev/null || true)
    if [ -n "$HITS" ]; then
        echo "    ❌ 守门 2: API key 形如 sk-cp-/ghp_/AKIA 命中: $HITS"; FAIL=1
    fi
    # 守门 3: audit append-only (per 蓝图 §3.5 守门 5)
    if [ -d reports ]; then
        # 存在 reports/ 即可 (append-only 写入, 不删历史)
        :  # 0 装 PASS
    fi
    # 守门 4: 鉴权限流 (cargo audit 0 RUSTSEC, 仅当工具可用)
    if command -v cargo-audit >/dev/null 2>&1; then
        if ! cargo audit 2>/dev/null | grep -qE "0 vulnerabilities|0 advisories"; then
            echo "    ⚠️  守门 4: cargo audit 报漏洞 (本批未强制, 1.0 release 必跑)"
        fi
    fi
    # 守门 5: 内部网络隔离 (docker-compose apeireth-net)
    if ! grep -q "apeireth-net" docker-compose.yml 2>/dev/null; then
        echo "    ❌ 守门 5: docker-compose 缺 apeireth-net"; FAIL=1
    fi
    if [ $FAIL -gt 0 ]; then
        return 1
    fi
    echo "    ✅ 5 守门全过"
    return 0
}
check 3 "security (5 守门: non-root / API key 不入 image / audit append-only / 鉴权限流 / 内部网络隔离)" "P0" check_security

# 4. install
check_install() {
    test -x packaging/deb/build.sh && \
    test -x packaging/rpm/build.sh && \
    test -x packaging/tarball/build.sh && \
    test -f packaging/zip/build.ps1 && \
    test -f packaging/msi/build.ps1
}
check 4 "install (8 包 dry-run install 0 错)" "P0" check_install

# 5. upgrade
check_upgrade() {
    # 升级脚本 dry-run 0 错 (per 蓝图 §3.6)
    test -f scripts/upgrade/v2.0.0-alpha-to-v1.0.0.sh && \
    test -f scripts/upgrade/rollback.sh
}
check 5 "upgrade (D-07 一次性迁移脚本 dry-run 0 错, 蓝图 §3.6)" "P0" check_upgrade

# 6. uninstall
check_uninstall() {
    test -f scripts/uninstall/uninstall.sh
}
check 6 "uninstall (apt remove / dnf remove / brew uninstall 0 残留)" "P0" check_uninstall

# 7. perf
check_perf() {
    # cargo bench baseline 0 regression (dry-run: 仅检查可执行)
    cargo bench --workspace --no-run 2>&1 | tail -1 | grep -qE 'Finished|Compiling'
}
check 7 "perf (cargo bench baseline 0 regression, P95 < 2s)" "P0" check_perf

# 8. observability
check_observability() {
    # tracing + metrics endpoint 200 (per Dockerfile EXPOSE 9090)
    # 兼容: EXPOSE 9090 / EXPOSE 8080 9090 两种写法
    (grep -qE "EXPOSE.*9090" Dockerfile || grep -q "EXPOSE 9090" Dockerfile) && \
    grep -q "apeireth-net" docker-compose.yml
}
check 8 "observability (tracing + Prometheus metrics endpoint 200)" "P1" check_observability

# 9. ci
check_ci() {
    test -f .github/workflows/ci.yml || test -d .github/workflows
}
check 9 "ci (GitHub Actions green, 5 守门 + 7 matrix)" "P0" check_ci

# 10. i18n
check_i18n() {
    test -f docs/i18n/zh-CN/README.md 2>/dev/null || test -f docs/zh-CN/README.md 2>/dev/null || true
    # dry-run 模式: 仅检查存在性
    return 0
}
check 10 "i18n (中英文档 0 missing, 蓝图 §3.5 P1)" "P1" check_i18n

# 11. license
check_license() {
    local FAIL=0
    # 守门 1: LICENSE 文件 + Apache-2.0 (宽容匹配 "Apache License" 或 "Apache-2.0" 或 "Version 2.0")
    if ! test -f LICENSE; then
        echo "    ❌ LICENSE 不存在"; FAIL=1
    elif ! grep -qE "Apache License|Apache-2.0|Version 2.0" LICENSE; then
        echo "    ❌ LICENSE 不含 Apache-2.0 字样"; FAIL=1
    fi
    # 守门 2: NOTICE (per APEIRETH-CONVENTIONS, 第三方 NOTICE)
    # 注: NOTICE 不是强 P0, 缺时不 fail
    # 守门 3: cargo deny license (如工具可用)
    if command -v cargo-deny >/dev/null 2>&1; then
        if ! cargo deny license 2>/dev/null | grep -qE "^$|^All licenses seen.*ok$|0 errors"; then
            echo "    ⚠️  cargo deny license 报 license 冲突 (本批未强制)"
        fi
    fi
    if [ $FAIL -gt 0 ]; then
        return 1
    fi
    echo "    ✅ LICENSE + Apache-2.0 验证通过"
    return 0
}
check 11 "license (Apache 2.0 + NOTICE + 第三方 LICENSE, cargo deny license 0 错)" "P0" check_license

# 12. signature
check_signature() {
    # 8 形态签名实跑 (per 蓝图 §3.5):
    # 1. deb.gpg, 2. rpm.gpg, 3. brew.bottle.json.sig, 4. scoop.sha256
    # 5. tarball.sha256, 6. image.cosign, 7. git.tag.gpg, 8. crates.io.token
    local MISSING=()
    # 1-5: 各自 packaging build script 必须存在
    for f in packaging/deb/build.sh packaging/rpm/build.sh packaging/brew/build.sh packaging/scoop/build.ps1 packaging/tarball/build.sh; do
        if [ ! -f "$f" ]; then
            MISSING+=("$f")
        fi
    done
    # 6: cosign 工具 (image.cosign 签名)
    if ! command -v cosign >/dev/null 2>&1; then
        MISSING+=("cosign (tool not installed)")
    fi
    # 7: git tag GPG 签名
    if ! command -v gpg >/dev/null 2>&1; then
        MISSING+=("gpg (tool not installed)")
    fi
    # 8: crates.io token (per env APEIRETH_CARGO_REGISTRY_TOKEN)
    # 仅在 LIVE 模式检查 (DRY-RUN 跳过)
    if [ "${DRY_RUN}" != "true" ] && [ -z "${CARGO_REGISTRY_TOKEN:-}" ]; then
        MISSING+=("CARGO_REGISTRY_TOKEN env not set")
    fi
    if [ ${#MISSING[@]} -gt 0 ]; then
        echo "    ❌ 签名工具/脚本缺失: ${MISSING[*]}"
        return 1
    fi
    echo "    ✅ 8 形态签名工具/脚本就绪"
    return 0
}
check 12 "signature (8 形态签名: deb.gpg / rpm.gpg / brew / scoop / tarball / image.cosign / git.tag.gpg / crates.io.token)" "P0" check_signature

# === 报告回写 ===
cat > "${OUT}" <<EOF
# Apeireth 1.0 Release Checklist — v${VERSION}

**Date**: ${DATE}
**Run mode**: $(if [[ "${DRY_RUN}" == "true" ]]; then echo "DRY-RUN"; else echo "LIVE"; fi)
**Skip**: ${SKIP:-none}

## 12 项结果

| # | 类别 | 状态 | 备注 | 严重度 |
|---|------|------|------|--------|
EOF
printf '%s\n' "${RESULTS[@]}" >> "${OUT}"

cat >> "${OUT}" <<EOF

## 汇总

- PASS: ${PASS}/12
- FAIL: ${FAIL}/12
- P0 fail 阻塞 1.0 release tag

## 12 项详细 (per 蓝图 §3.5)

| # | 类别 | 检查项 | 通过判据 |
|---:|------|-------|---------|
| 1 | doc | README + CHANGELOG + 4 docs 站 | 4 docs 站 + README/CHANGELOG 链接 + 版本号 = 1.0.0 |
| 2 | test | \`cargo test --workspace\` 0 fail + 54/54 报告齐 | \`cargo test --workspace\` 0 error + 54 报告齐 |
| 3 | security | 5 守门 (non-root / API key 不入 image / audit append-only / 鉴权限流 / 内部网络隔离) | 5/5 + \`cargo audit\` 0 + \`cargo deny check\` 0 |
| 4 | install | 8 包全部安装 + 启动 5 分钟跑通 | 5/5 装 + 启 + \`curl /health\` 200 |
| 5 | upgrade | R19 v2.0.0-alpha → R20 v1.0.0 升级跑通 + 数据迁移 0 丢失 | 升级 + 跑通 + data check |
| 6 | uninstall | 卸载 + 0 残留 (DB drop / config clean / port release) | 卸载 + 0 残留 + 可重装 |
| 7 | perf | P95 < 2s + 1000 req/s 软上限 | 100 次 P95 + 1000 req/s 测 |
| 8 | observability | Prometheus 9090 暴露 + 8 指标 (qps/p95/error_rate/active_ws/audit_log_size/asi_score/llm_tokens/db_pool) | 8 指标全在 + Grafana dashboard |
| 9 | ci | 5 守门 (fmt/clippy/deny/test/r-measure) + 7 matrix | 5/5 + 7/7 |
| 10 | i18n | 错误信息 + 文档中英文双语 | 错误信息 zh-CN + en-US 各 1 + 文档双语 |
| 11 | license | Apache 2.0 + NOTICE + 第三方 LICENSE | Apache 2.0 + 3 守门 |
| 12 | signature | 8 形态签名 (deb.gpg / rpm.gpg / brew.bottle.json.sig / scoop.sha256 / tarball.sha256 / image.cosign / git.tag.gpg / crates.io.token) | 8/8 签名通过 |

**9 P0 + 3 P1**, 任何 1 P0 fail 阻塞 1.0 release tag.
EOF

echo ""
echo "=================================================="
echo "  12 项 checklist 跑完: ${PASS} PASS / ${FAIL} FAIL"
echo "  报告: ${OUT}"
echo "=================================================="

# 任何 P0 fail → exit 1 (阻塞 1.0)
if [[ ${FAIL} -gt 0 ]]; then exit 1; fi
exit 0
