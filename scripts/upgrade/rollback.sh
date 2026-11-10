#!/usr/bin/env bash
# Apeireth 回滚脚本 (按安装规范, 7 天内可回滚到 v2.0.0-alpha)
#
# 用法:
#   sudo ./scripts/upgrade/rollback.sh                    # 回滚到最新备份
#   sudo ./scripts/upgrade/rollback.sh <BACKUP_DIR>       # 回滚到指定备份目录
#   ./scripts/upgrade/rollback.sh --dry-run               # **只打印计划，不做任何变更**
#
# 纪律（2026-10-25 修）:
# 1. **未知参数一律拒绝（exit 2）** —— 破坏性脚本不允许静默忽略参数。
#    修前本脚本**零参数解析**：传 `--dry-run` 会被静默忽略并**真执行**回滚
#    （停/起服务 + 覆盖二进制与配置），这是危险的失败模式。
# 2. `--dry-run` = 零变更（不 stop/start、不 cp）；日志逐条标 `[DRY-RUN] would`。
# 3. 非 dry-run 必须 root；非 root 直接拒绝（修前是部分失败后仍打印 "Rollback done"）。

set -euo pipefail

usage() {
    sed -n '3,7p' "$0" | sed 's/^# \{0,1\}//'
}

DRY_RUN=false
BACKUP_DIR=""
for arg in "$@"; do
    case "${arg}" in
        --dry-run) DRY_RUN=true ;;
        -h|--help) usage; exit 0 ;;
        -*)
            echo "ERROR: 未知参数 ${arg}（破坏性脚本拒绝静默忽略参数；用 --help 查看用法）" >&2
            exit 2
            ;;
        *)
            if [[ -n "${BACKUP_DIR}" ]]; then
                echo "ERROR: 只能指定一个备份目录（已给 ${BACKUP_DIR}，又收到 ${arg}）" >&2
                exit 2
            fi
            BACKUP_DIR="${arg}"
            ;;
    esac
done

if [[ "${DRY_RUN}" != "true" ]] && [[ "$(id -u)" -ne 0 ]]; then
    echo "ERROR: 回滚需要 root（当前 uid=$(id -u)）。想先看计划请加 --dry-run。" >&2
    exit 1
fi

if [[ -z "${BACKUP_DIR}" ]]; then
    BACKUP_DIR=$(ls -td /var/backups/apeireth/upgrade-* 2>/dev/null | head -1 || true)
fi
if [[ -z "${BACKUP_DIR}" ]]; then
    echo "ERROR: 无 backup dir 在 /var/backups/apeireth/upgrade-*（可用参数显式指定）"
    exit 1
fi
if [[ ! -d "${BACKUP_DIR}" ]]; then
    echo "ERROR: 备份目录不存在: ${BACKUP_DIR}"
    exit 1
fi

echo "Rolling back to: ${BACKUP_DIR}"
if [[ "${DRY_RUN}" == "true" ]]; then
    echo "模式: DRY-RUN（不做任何变更）"
fi

run() {
    if [[ "${DRY_RUN}" == "true" ]]; then
        echo "[DRY-RUN] would run: $*"
    else
        "$@"
    fi
}

run systemctl stop apeireth || true

if [[ -f "${BACKUP_DIR}/apeireth.v2.0.0-alpha" ]]; then
    run cp "${BACKUP_DIR}/apeireth.v2.0.0-alpha" /usr/local/bin/apeireth
else
    echo "[WARN] 备份内无 apeireth.v2.0.0-alpha 二进制，跳过二进制还原"
fi
if [[ -d "${BACKUP_DIR}/data" ]]; then
    run cp -a "${BACKUP_DIR}/data/." /var/lib/apeireth/data/
fi
run cp -a "${BACKUP_DIR}/." /etc/apeireth/

run systemctl start apeireth || true

if [[ "${DRY_RUN}" == "true" ]]; then
    echo "DRY-RUN 完成：以上为计划，**未执行任何变更** ✓"
    exit 0
fi

sleep 3
HEALTH=$(curl -fsS http://localhost:8080/health 2>/dev/null || echo '{"version":"unknown"}')
echo "Rollback done. Health: ${HEALTH}"
