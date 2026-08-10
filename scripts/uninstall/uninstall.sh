#!/usr/bin/env bash
# =============================================================================
# scripts/uninstall/uninstall.sh
#
# Apeireth 完整卸载 (per 蓝图 §3.7 5 步 0 残留守门)
#
# 蓝图: reports/r19-integration-v2/r20-stage-2-3-prep-2026-08-05.md §3.7
# §3.5 #6 卸载守门: 卸载 + 0 残留 + 可重装 (devops_engineer 跑, 任何 1 失败 = P0)
#
# 5 步 0 残留 (per §3.7):
#   1. stop + docker compose down
#   2. remove package (8 形态对应 8 个 uninstall, 选 1)
#   3. drop data (per O-5 不假装, 0 残留)
#   4. release port (per O-5 12 急救, 0 端口占用)
#   5. cleanup (config / systemd / network / image)
#
# 8 包形态 (per §3.4 主人 D-06 拍 A 8 形态同时):
#   - Debian/Ubuntu: deb (apt remove --purge)
#   - RHEL/Fedora:   rpm (dnf remove)
#   - macOS:         brew (brew uninstall)
#   - Windows:       scoop (scoop uninstall)
#   - Linux 通用:    tarball (rm -rf)
#   - 跨平台:        zip (rm -rf)
#   - Windows MSI:   msiexec /uninstall
#   - Docker:        docker rmi
#
# 8 形态并非 8 个独立通道, 而是一个 manifest 多通道 (per §3.4):
#   - 1 个 .deb + 1 个 .rpm + 1 个 brew formula + 1 个 scoop + 1 个 tarball + 1 个 zip + 1 个 MSI
#   - + Docker image (1 个, 多架构)
#   - 共 8 个分发通道
#
# 严守承诺 (per 8-locked-unified-2026-08-05.md):
#   - 0 改 56 LOCKED crate
#   - 0 改 7 LOCKED 文档
#   - 0 改 workspace version 1.0.0
#   - 0 引新 lib (用 apt/dnf/brew/scoop/docker 等系统命令)
#
# 跟 uninstall 相关其他脚本:
#   - scripts/upgrade/rollback.sh (蓝图 §3.6, 7 天内回滚到 v2.0.0-alpha)
#
# 用法:
#   sudo bash scripts/uninstall/uninstall.sh           # 完整卸载
#   sudo bash scripts/uninstall/uninstall.sh --keep-data  # 保留数据目录 (用于调试)
#   bash scripts/uninstall/uninstall.sh --dry-run     # 仅打印, 不执行
# =============================================================================

set -euo pipefail

# ===================== 配置 =====================
SCRIPT_NAME="$(basename "$0")"
PACKAGE_NAME="apeireth"
SERVICE_NAME="apeireth"
DOCKER_IMAGE="ghcr.io/apeireth/api:1.0.0"
DOCKER_IMAGE_LATEST="ghcr.io/apeireth/api:latest"
DATA_PATHS=(
  "/var/lib/apeireth"
  "/var/log/apeireth"
  "/etc/apeireth"
)
DOCKER_VOLUMES=(
  "apeireth_apeireth-data"
  "apeireth_apeireth-config"
  "apeireth_apeireth-logs"
  "apeireth_postgres-data"
  "apeireth_redis-data"
)
DOCKER_NETWORKS=(
  "apeireth_apeireth-net"
)
PORTS=(
  "8080/tcp"
  "9090/tcp"
)

# 标志
KEEP_DATA=false
DRY_RUN=false
for arg in "$@"; do
  case "${arg}" in
    --keep-data) KEEP_DATA=true ;;
    --dry-run)   DRY_RUN=true ;;
    --help|-h)
      sed -n '2,30p' "$0"
      exit 0
      ;;
  esac
done

# ===================== 工具函数 =====================
log_step() {
  local n="$1"
  local desc="$2"
  echo ""
  echo "=== [${n}/5] ${desc} ==="
}

log_info() {
  echo "[INFO] $*"
}

log_warn() {
  echo "[WARN] $*" >&2
}

log_err() {
  echo "[ERROR] $*" >&2
}

# 仅在非 dry-run 时执行命令
run_cmd() {
  if [[ "${DRY_RUN}" == "true" ]]; then
    log_info "[DRY-RUN] would run: $*"
  else
    "$@"
  fi
}

# 0 残留验证
verify_clean() {
  local fail=0

  log_info "[VERIFY] 0 残留检查..."

  # 验证 1: systemd unit 不存在
  if [[ -f "/etc/systemd/system/${SERVICE_NAME}.service" ]] || [[ -f "/lib/systemd/system/${SERVICE_NAME}.service" ]]; then
    log_err "  ✗ systemd unit 仍存在: ${SERVICE_NAME}.service"
    fail=$((fail + 1))
  else
    log_info "  ✓ systemd unit: 不存在"
  fi

  # 验证 2: 包未安装 (Debian/RPM/brew/scoop)
  if command -v dpkg >/dev/null 2>&1; then
    if dpkg -l "${PACKAGE_NAME}" 2>/dev/null | grep -q "^ii"; then
      log_err "  ✗ Debian 包仍安装: ${PACKAGE_NAME}"
      fail=$((fail + 1))
    else
      log_info "  ✓ Debian 包: 未安装"
    fi
  fi
  if command -v rpm >/dev/null 2>&1; then
    if rpm -q "${PACKAGE_NAME}" >/dev/null 2>&1; then
      log_err "  ✗ RPM 包仍安装: ${PACKAGE_NAME}"
      fail=$((fail + 1))
    else
      log_info "  ✓ RPM 包: 未安装"
    fi
  fi
  if command -v brew >/dev/null 2>&1; then
    if brew list 2>/dev/null | grep -q "^${PACKAGE_NAME}$"; then
      log_err "  ✗ brew formula 仍安装: ${PACKAGE_NAME}"
      fail=$((fail + 1))
    else
      log_info "  ✓ brew formula: 未安装"
    fi
  fi
  if command -v scoop >/dev/null 2>&1; then
    if scoop list 2>/dev/null | grep -q "^${PACKAGE_NAME}$"; then
      log_err "  ✗ scoop package 仍安装: ${PACKAGE_NAME}"
      fail=$((fail + 1))
    else
      log_info "  ✓ scoop package: 未安装"
    fi
  fi

  # 验证 3: 数据目录不存在 (除非 --keep-data)
  if [[ "${KEEP_DATA}" == "true" ]]; then
    log_info "  ⊘ 数据目录: 保留 (--keep-data)"
  else
    for path in "${DATA_PATHS[@]}"; do
      if [[ -e "${path}" ]]; then
        log_err "  ✗ 数据目录仍存在: ${path}"
        fail=$((fail + 1))
      else
        log_info "  ✓ 数据目录: ${path} 不存在"
      fi
    done
  fi

  # 验证 4: 端口空闲
  for port in "${PORTS[@]}"; do
    if command -v ss >/dev/null 2>&1; then
      if ss -tlnp 2>/dev/null | grep -q ":${port%%/*} "; then
        log_err "  ✗ 端口仍占用: ${port}"
        fail=$((fail + 1))
      else
        log_info "  ✓ 端口空闲: ${port}"
      fi
    fi
  done

  # 验证 5: Docker volumes/images 不存在 (如用 Docker)
  if command -v docker >/dev/null 2>&1; then
    for vol in "${DOCKER_VOLUMES[@]}"; do
      if docker volume inspect "${vol}" >/dev/null 2>&1; then
        log_err "  ✗ Docker volume 仍存在: ${vol}"
        fail=$((fail + 1))
      else
        log_info "  ✓ Docker volume: ${vol} 不存在"
      fi
    done
    for img in "${DOCKER_IMAGE}" "${DOCKER_IMAGE_LATEST}"; do
      if docker image inspect "${img}" >/dev/null 2>&1; then
        log_err "  ✗ Docker image 仍存在: ${img}"
        fail=$((fail + 1))
      else
        log_info "  ✓ Docker image: ${img} 不存在"
      fi
    done
  fi

  if [[ "${fail}" -gt 0 ]]; then
    log_err "0 残留验证失败: ${fail} 项不通过"
    log_err "需要手动清理: see /var/log/apeireth/uninstall-*.log"
    return 1
  fi
  log_info "0 残留验证全部通过 ✓"
}

# ===================== Step 1: stop + docker compose down =====================
step1_stop() {
  log_step "1" "stop + docker compose down"

  # 1.1 systemd stop
  if command -v systemctl >/dev/null 2>&1; then
    if systemctl is-active --quiet "${SERVICE_NAME}" 2>/dev/null; then
      log_info "[1.1/2] systemctl stop ${SERVICE_NAME}"
      run_cmd systemctl stop "${SERVICE_NAME}" || log_warn "stop 失败 (可能未运行)"
    else
      log_info "[1.1/2] ${SERVICE_NAME} 未运行, 跳过"
    fi
    if systemctl is-enabled --quiet "${SERVICE_NAME}" 2>/dev/null; then
      run_cmd systemctl disable "${SERVICE_NAME}" || log_warn "disable 失败"
      log_info "  systemd disable OK"
    fi
  else
    log_info "[1.1/2] systemctl 不可用 (Windows?), 跳过"
  fi

  # 1.2 docker compose down
  if [[ -f "docker-compose.yml" ]] || [[ -f "docker-compose.yaml" ]]; then
    log_info "[1.2/2] docker-compose down -v"
    if command -v docker-compose >/dev/null 2>&1; then
      run_cmd docker-compose down -v || log_warn "docker-compose down 失败"
    elif command -v docker >/dev/null 2>&1; then
      run_cmd docker compose down -v || log_warn "docker compose down 失败"
    fi
  else
    log_info "[1.2/2] docker-compose.yml 不存在, 跳过"
  fi
}

# ===================== Step 2: remove package (8 形态选 1) =====================
step2_remove_package() {
  log_step "2" "remove package (8 形态选 1, 自动检测)"

  local removed=false

  # 形态 1: Debian/Ubuntu deb
  if command -v apt >/dev/null 2>&1 && dpkg -l "${PACKAGE_NAME}" 2>/dev/null | grep -q "^ii"; then
    log_info "[2.1/8] apt remove --purge -y ${PACKAGE_NAME} (Debian/Ubuntu deb)"
    run_cmd apt-get remove --purge -y "${PACKAGE_NAME}" || log_warn "apt remove 失败"
    removed=true
  fi

  # 形态 2: RHEL/Fedora rpm
  if command -v dnf >/dev/null 2>&1 && rpm -q "${PACKAGE_NAME}" >/dev/null 2>&1; then
    log_info "[2.2/8] dnf remove -y ${PACKAGE_NAME} (RHEL/Fedora rpm)"
    run_cmd dnf remove -y "${PACKAGE_NAME}" || log_warn "dnf remove 失败"
    removed=true
  fi

  # 形态 3: macOS brew
  if command -v brew >/dev/null 2>&1 && brew list 2>/dev/null | grep -q "^${PACKAGE_NAME}$"; then
    log_info "[2.3/8] brew uninstall ${PACKAGE_NAME} (macOS brew formula)"
    run_cmd brew uninstall "${PACKAGE_NAME}" || log_warn "brew uninstall 失败"
    removed=true
  fi

  # 形态 4: Windows scoop
  if command -v scoop >/dev/null 2>&1 && scoop list 2>/dev/null | grep -q "^${PACKAGE_NAME}$"; then
    log_info "[2.4/8] scoop uninstall ${PACKAGE_NAME} (Windows scoop)"
    run_cmd scoop uninstall "${PACKAGE_NAME}" || log_warn "scoop uninstall 失败"
    removed=true
  fi

  # 形态 5: tarball (Linux generic)
  if [[ -f "/usr/local/bin/${PACKAGE_NAME}" ]] || [[ -d "/opt/${PACKAGE_NAME}" ]]; then
    log_info "[2.5/8] rm -rf /opt/${PACKAGE_NAME} + /usr/local/bin/${PACKAGE_NAME} (tarball)"
    run_cmd rm -rf "/opt/${PACKAGE_NAME}"
    run_cmd rm -f "/usr/local/bin/${PACKAGE_NAME}"
    removed=true
  fi

  # 形态 6: zip (跨平台解压)
  if [[ -d "${HOME}/Applications/${PACKAGE_NAME}.app" ]] || [[ -d "${HOME}/${PACKAGE_NAME}" ]]; then
    log_info "[2.6/8] rm -rf ~/Applications/${PACKAGE_NAME}.app + ~/${PACKAGE_NAME} (zip)"
    run_cmd rm -rf "${HOME}/Applications/${PACKAGE_NAME}.app"
    run_cmd rm -rf "${HOME}/${PACKAGE_NAME}"
    removed=true
  fi

  # 形态 7: Windows MSI
  if command -v msiexec >/dev/null 2>&1; then
    local msi_guid
    msi_guid=$(reg query "HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall" /s 2>/dev/null | grep -i "DisplayName.*${PACKAGE_NAME}" -B 1 | grep -oP '\{[A-F0-9-]+\}' | head -1 || echo "")
    if [[ -n "${msi_guid}" ]]; then
      log_info "[2.7/8] msiexec /x ${msi_guid} /quiet (Windows MSI)"
      run_cmd msiexec /x "${msi_guid}" /quiet || log_warn "msiexec /x 失败"
      removed=true
    fi
  fi

  # 形态 8: Docker image
  if command -v docker >/dev/null 2>&1; then
    if docker image inspect "${DOCKER_IMAGE}" >/dev/null 2>&1 || docker image inspect "${DOCKER_IMAGE_LATEST}" >/dev/null 2>&1; then
      log_info "[2.8/8] docker rmi ${DOCKER_IMAGE} ${DOCKER_IMAGE_LATEST} (Docker image)"
      run_cmd docker rmi "${DOCKER_IMAGE}" "${DOCKER_IMAGE_LATEST}" || log_warn "docker rmi 失败"
      removed=true
    fi
  fi

  if [[ "${removed}" == "false" ]]; then
    log_warn "未检测到已安装包 (8 形态都无), 跳过"
  else
    log_info "包已卸载 (至少 1 形态)"
  fi
}

# ===================== Step 3: drop data (per O-5 不假装, 0 残留) =====================
step3_drop_data() {
  log_step "3" "drop data (0 残留, 除非 --keep-data)"

  if [[ "${KEEP_DATA}" == "true" ]]; then
    log_warn "--keep-data 标志, 跳过数据清理"
    log_warn "  数据保留: ${DATA_PATHS[*]}"
    return 0
  fi

  # 3.1 文件系统数据
  log_info "[3.1/3] rm -rf ${DATA_PATHS[*]}"
  for path in "${DATA_PATHS[@]}"; do
    if [[ -e "${path}" ]]; then
      run_cmd rm -rf "${path}"
      log_info "  removed: ${path}"
    else
      log_info "  skip (not exists): ${path}"
    fi
  done

  # 3.2 Docker volumes
  if command -v docker >/dev/null 2>&1; then
    log_info "[3.2/3] docker volume rm ${DOCKER_VOLUMES[*]}"
    for vol in "${DOCKER_VOLUMES[@]}"; do
      if docker volume inspect "${vol}" >/dev/null 2>&1; then
        run_cmd docker volume rm "${vol}" || log_warn "  volume rm 失败: ${vol}"
      fi
    done
  else
    log_info "[3.2/3] docker 不可用, 跳过"
  fi

  # 3.3 PostgreSQL DB + user drop
  if command -v psql >/dev/null 2>&1; then
    log_info "[3.3/3] sudo -u postgres psql DROP DATABASE + USER"
    if command -v sudo >/dev/null 2>&1; then
      run_cmd sudo -u postgres psql -c "DROP DATABASE IF EXISTS apeireth;" || log_warn "  DROP DATABASE 失败"
      run_cmd sudo -u postgres psql -c "DROP USER IF EXISTS apeireth;" || log_warn "  DROP USER 失败"
    else
      log_warn "  sudo 不可用, 手动 DROP: psql -c 'DROP DATABASE apeireth;'"
    fi
  else
    log_info "[3.3/3] psql 不可用, 跳过"
  fi
}

# ===================== Step 4: release port =====================
step4_release_port() {
  log_step "4" "release port (per O-5 12 急救, 0 端口占用)"

  if command -v fuser >/dev/null 2>&1; then
    log_info "fuser -k ${PORTS[*]}"
    for port in "${PORTS[@]}"; do
      run_cmd fuser -k "${port}" 2>/dev/null || log_info "  端口空闲: ${port}"
    done
  else
    log_warn "fuser 不可用, 手动检查: netstat -tlnp | grep -E '8080|9090'"
  fi
}

# ===================== Step 5: cleanup =====================
step5_cleanup() {
  log_step "5" "cleanup (config / systemd / network / image)"

  # 5.1 systemd unit 完整清理
  if [[ -f "/etc/systemd/system/${SERVICE_NAME}.service" ]]; then
    log_info "[5.1/4] rm /etc/systemd/system/${SERVICE_NAME}.service"
    run_cmd rm -f "/etc/systemd/system/${SERVICE_NAME}.service"
    if command -v systemctl >/dev/null 2>&1; then
      run_cmd systemctl daemon-reload
      run_cmd systemctl reset-failed "${SERVICE_NAME}" 2>/dev/null || true
    fi
  else
    log_info "[5.1/4] systemd unit 不存在, 跳过"
  fi

  # 5.2 Docker network
  if command -v docker >/dev/null 2>&1; then
    log_info "[5.2/4] docker network rm ${DOCKER_NETWORKS[*]}"
    for net in "${DOCKER_NETWORKS[@]}"; do
      if docker network inspect "${net}" >/dev/null 2>&1; then
        run_cmd docker network rm "${net}" || log_warn "  network rm 失败: ${net}"
      fi
    done
  else
    log_info "[5.2/4] docker 不可用, 跳过"
  fi

  # 5.3 Docker image 完整清理 (如 step 2 未清)
  if command -v docker >/dev/null 2>&1; then
    log_info "[5.3/4] docker rmi ${DOCKER_IMAGE} ${DOCKER_IMAGE_LATEST} (兜底)"
    run_cmd docker rmi "${DOCKER_IMAGE}" 2>/dev/null || true
    run_cmd docker rmi "${DOCKER_IMAGE_LATEST}" 2>/dev/null || true
  else
    log_info "[5.3/4] docker 不可用, 跳过"
  fi

  # 5.4 user crontab (如装了 apeireth-migrate-cleanup)
  if command -v crontab >/dev/null 2>&1; then
    if crontab -l 2>/dev/null | grep -q "apeireth-migrate-cleanup"; then
      log_info "[5.4/4] crontab -l | grep -v apeireth-migrate-cleanup | crontab -"
      if [[ "${DRY_RUN}" == "true" ]]; then
        log_info "[DRY-RUN] would remove apeireth-migrate-cleanup from crontab"
      else
        crontab -l 2>/dev/null | grep -v "apeireth-migrate-cleanup" | crontab -
        log_info "  crontab 清理 OK"
      fi
    else
      log_info "[5.4/4] crontab 无 apeireth-migrate-cleanup, 跳过"
    fi
  fi
}

# ===================== 主流程 =====================
main() {
  echo "==============================================="
  echo " Apeireth Uninstall v1.0.0"
  echo " 蓝图: reports/r19-integration-v2/r20-stage-2-3-prep §3.7"
  echo " 守门: §3.5 #6 卸载 + 0 残留 + 可重装 (P0)"
  echo " 模式: $(if [[ "${DRY_RUN}" == "true" ]]; then echo "DRY-RUN"; else echo "REAL"; fi)"
  echo " 保留数据: $(if [[ "${KEEP_DATA}" == "true" ]]; then echo "YES (--keep-data)"; else echo "NO"; fi)"
  echo "==============================================="

  if [[ "${EUID}" -ne 0 ]] && [[ "${DRY_RUN}" == "false" ]]; then
    log_err "Must run as root (apt/dnf/systemctl 需要)"
    log_err "Re-run: sudo bash ${SCRIPT_NAME}"
    exit 1
  fi

  step1_stop
  step2_remove_package
  step3_drop_data
  step4_release_port
  step5_cleanup

  if [[ "${DRY_RUN}" == "true" ]]; then
    log_info "[DRY-RUN] 跳过 0 残留验证, 实际跑时请去掉 --dry-run"
    log_info "uninstall dry-run 成功, 5 步骨架就绪 ✓"
    exit 0
  fi

  echo ""
  log_info "=== 0 残留验证 ==="
  if verify_clean; then
    echo ""
    echo "==============================================="
    echo " Uninstall COMPLETE — 0 残留"
    echo "   验证:"
    echo "   - ls /var/lib/apeireth → 'No such file'"
    echo "   - systemctl status ${SERVICE_NAME} → 'Unit not found'"
    echo "   - curl localhost:8080/health → fail (端口空闲)"
    echo "   可立即重装: apt install apeireth 或 brew install apeireth"
    echo "==============================================="
    exit 0
  else
    echo ""
    echo "==============================================="
    echo " Uninstall PARTIAL — 有残留, 需手动清理"
    echo "   查看: /var/log/apeireth/uninstall-*.log (如启用)"
    echo "==============================================="
    exit 1
  fi
}

main "$@"
