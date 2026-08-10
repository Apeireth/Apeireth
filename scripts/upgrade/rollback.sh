#!/usr/bin/env bash
# Apeireth 回滚脚本 (per 蓝图 §3.6, 7 天内可回滚到 v2.0.0-alpha)
# 用法:
#   sudo ./scripts/upgrade/rollback.sh

set -euo pipefail

BACKUP_DIR=$(ls -td /var/backups/apeireth/upgrade-* 2>/dev/null | head -1)
if [[ -z "${BACKUP_DIR}" ]]; then
    echo "ERROR: 无 backup dir 在 /var/backups/apeireth/upgrade-*"
    exit 1
fi

echo "Rolling back to: ${BACKUP_DIR}"

systemctl stop apeireth || true

if [[ -f "${BACKUP_DIR}/apeireth.v2.0.0-alpha" ]]; then
    cp "${BACKUP_DIR}/apeireth.v2.0.0-alpha" /usr/local/bin/apeireth
fi
if [[ -d "${BACKUP_DIR}/data" ]]; then
    cp -a "${BACKUP_DIR}/data/." /var/lib/apeireth/data/ 2>/dev/null || true
fi
if [[ -d "${BACKUP_DIR}" ]]; then
    cp -a "${BACKUP_DIR}/." /etc/apeireth/ 2>/dev/null || true
fi

systemctl start apeireth || true
sleep 3
HEALTH=$(curl -fsS http://localhost:8080/health 2>/dev/null || echo '{"version":"unknown"}')
echo "Rollback done. Health: ${HEALTH}"
