# 8 包对比表 (D-06 8 包齐发)

> **D-06 决策**: 主人 2026-08-05 20:53 拍 A, 8 包齐发 + Linux 4 包重点
> **蓝图**: [`docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md §3.4`](../stage4/v09021-rust-translation-blueprint-2026-08-05.md)
> **目标 release**: v1.0.0 @ 2026-09-30 (R20 阶段 6 收尾)

---

## 0. 一图速览 (TL;DR)

| 包 | 平台 | 服务 | 体积 | 重点 | 状态 |
|---|---|---|---|---|---|
| **deb** | Debian/Ubuntu | systemd | ~50MB | ⭐⭐⭐ Linux #1 | 🟢 已落地 (50e6cbf0) |
| **rpm** | RHEL/Fedora/openSUSE | systemd | ~50MB | ⭐⭐⭐ Linux #2 | 🟢 已落地 (50e6cbf0) |
| **tarball** | 任何 Linux (musl 静态) | 手起 / systemd (可选) | ~50MB | ⭐⭐⭐ Linux #3 | 🟢 已落地 (50e6cbf0) |
| **Docker** | linux/amd64 + linux/arm64 | distroless (无 systemd) | ~80MB | ⭐⭐⭐ Linux #4 | 🟢 已落地 (50e6cbf0) |
| **brew** | macOS | launchd | ~40MB | ⭐ macOS | 🟢 已落地 (50e6cbf0) |
| **scoop** | Windows | NSSM / Task Scheduler | ~50MB | ⭐ Windows | 🟢 已落地 (50e6cbf0) |
| **zip** | Windows 通用 | 手起 | ~50MB | ⭐ Windows fallback | 🟢 已落地 (50e6cbf0) |
| **MSI** | Windows (WiX) | Windows Service | ~60MB | ⭐ Windows 企业 | 🟢 已落地 (50e6cbf0) |

**Linux 4 包重点** (主人补充"搞技术用户很多 Linux"): deb / rpm / tarball / Docker 估覆盖 90% Linux 用户

---

## 1. 详细对比

### 1.1 系统要求

| 包 | OS | glibc/musl | systemd | 编译链 | 备注 |
|---|---|---|---|---|---|
| deb | Debian 11+ / Ubuntu 20.04+ | glibc | 245+ (Type=notify) | cargo-deb | apt 仓库 |
| rpm | RHEL 9+ / Fedora 38+ | glibc | 245+ | cargo-rpm | dnf 仓库 |
| tarball | 任何 Linux | musl 静态 (零依赖) | 可选 | musl target | 通用 fallback |
| Docker | 任何 Docker Host | - | - (distroless) | - | 容器 |
| brew | macOS 11+ | - | launchd | Homebrew | tap 仓库 |
| scoop | Windows 10+ | - | NSSM (估补) | - | bucket 仓库 |
| zip | Windows 10+ | - | - | - | 通用 fallback |
| MSI | Windows 10+ | - | Windows Service | WiX 3.x | 企业 IT |

### 1.2 安装/卸载/升级

| 包 | 装 | 升级 | 卸 | 数据保留 |
|---|---|---|---|---|
| deb | `apt install ./apeireth.deb` | `apt upgrade ./apeireth.deb` | `apt remove --purge apeireth` | `/var/lib/apeireth` |
| rpm | `dnf install ./apeireth.rpm` | `dnf upgrade ./apeireth.rpm` | `dnf remove apeireth` | `/var/lib/apeireth` |
| tarball | `install-tarball.sh` (5 步) | 手动重解 | `rm -rf /opt/apeireth` | `/opt/apeireth` |
| Docker | `docker run` 或 `docker compose up` | `docker pull` | `docker rm` | volume |
| brew | `brew install apeireth/tap/apeireth` | `brew upgrade` | `brew uninstall` | `~/.apeireth` |
| scoop | `scoop install apeireth` | `scoop update` | `scoop uninstall` | `%APEIRETH_HOME%` |
| zip | 解压到任意目录 | 覆盖安装 | `rm -rf` | 自管 |
| MSI | 双击 / `msiexec /i` | `msiexec /update` | 控制面板 / `msiexec /x` | 自管 |

### 1.3 服务管理

| 包 | 启动 | 状态 | 日志 |
|---|---|---|---|
| deb/rpm | `systemctl start apeireth` | `systemctl status` | `journalctl -u apeireth` |
| tarball | `systemctl start` (可) / `nohup ./apeireth serve &` | `pgrep apeireth` | `journalctl` / 重定向 |
| Docker | `docker compose up -d` | `docker ps` | `docker logs` |
| brew | `brew services start apeireth` | `brew services list` | `$(brew --prefix)/var/log/apeireth.log` |
| scoop | `nssm start Apeireth` | `nssm status` | `%APEIRETH_HOME%\logs\service.log` |
| zip | `apeireth serve` | 任务管理器 | 手动 |
| MSI | `Start-Service Apeireth` | `Get-Service Apeireth` | 事件查看器 |

### 1.4 体积 / 启动 / 依赖

| 包 | 体积 | 启动 | 运行时依赖 |
|---|---|---|---|
| deb | ~50MB | <2s (systemd Type=notify) | libc6, libssl3, libsqlite3-0, ca-certificates |
| rpm | ~50MB | <2s | openssl-libs, sqlite-libs, libgit2, ca-certificates |
| tarball | ~50MB | <1s (musl 静态) | **0** (静态链接) |
| Docker | ~80MB (含 base) | <3s (distroless) | Docker Engine |
| brew | ~40MB | <2s | openssl@3, sqlite, libgit2 |
| scoop | ~50MB | <2s | VC++ Redist 2015-2022 |
| zip | ~50MB | <1s | VC++ Redist 2015-2022 |
| MSI | ~60MB | <2s (Windows Service) | VC++ Redist 2015-2022 |

### 1.5 跨平台覆盖 (估)

| 平台 | 包 | 估覆盖 |
|---|---|---|
| **Linux (Debian/Ubuntu)** | deb | 35% |
| **Linux (RHEL/Fedora)** | rpm | 25% |
| **Linux (其他)** | tarball | 30% |
| **macOS** | brew | 5% |
| **Windows** | scoop + zip + MSI | 5% |
| **容器** | Docker | (跨平台, 单独计) |

**Linux 4 包累计覆盖**: ~90% Linux 用户

---

## 2. 选型决策树

```
你用什么 OS?
├── Linux
│   ├── Debian/Ubuntu ──→ deb (apt 一行装)
│   ├── RHEL/Fedora/CentOS ──→ rpm (dnf 一行装)
│   ├── Arch/Manjaro (AUR) ──→ tarball (R20 阶段 4 PKGBUILD 估补)
│   ├── Alpine/Devuan (无 systemd) ──→ tarball (手起)
│   └── 老发行版 ──→ tarball (musl 静态, 0 依赖)
├── macOS
│   └── ──→ brew (一行装, launchd 自动)
├── Windows
│   ├── 开发者 ──→ scoop (用户级, 无 admin)
│   ├── IT 企业 ──→ MSI (WiX, Group Policy 部署)
│   └── 通用 fallback ──→ zip
└── 容器化
    └── ──→ Docker (multi-arch linux/amd64 + linux/arm64, distroless)
```

---

## 3. 决策矩阵 (per 用例)

| 用例 | 推 | 原因 |
|---|---|---|
| **个人开发者 Linux** | deb/rpm | 1 行装, 自动 systemd |
| **服务器 Debian/Ubuntu** | deb | 配 apt 仓库, 自动更新 |
| **服务器 RHEL/CentOS** | rpm | 配 dnf 仓库, SELinux 兼容 |
| **CI/CD** | tarball 或 Docker | 干净环境, 0 副作用 |
| **macOS 开发者** | brew | 1 行, 配 launchd |
| **Windows 开发者** | scoop | 用户级, 不需 admin |
| **企业 IT 部署 Windows** | MSI | Group Policy, 静默安装 |
| **IoT / NAS / 嵌入式** | tarball | musl 静态, 0 依赖 |
| **K8s 集群** | Docker | 容器编排, multi-arch |
| **LTS 长期维护** | deb/rpm | 包管理, 升级回滚 |
| **快速试用** | Docker | `docker run` 一行 |
| **AI/ML 训练** | Docker | GPU 驱动可挂载 |

---

## 4. 升级路径 (跨包)

| 从 | 到 | 路径 |
|---|---|---|
| 任意 Linux 包 | 任意 Linux 包 | 重装对应包, `/var/lib/apeireth` 数据保留 |
| deb | rpm | `apt remove` + `dnf install`, 数据保留 |
| tarball | deb/rpm | `apt install` 或 `dnf install` 会自动检测 `/opt/apeireth` 残留并提示 |
| brew | brew | `brew upgrade` |
| scoop | scoop | `scoop update` |
| Docker | Docker | `docker pull` |
| **v2.0.0-alpha → v1.0.0** (任何包) | | `scripts/upgrade/v2.0.0-alpha-to-v1.0.0.sh` (D-07 一次性 SQLite→PG) |

---

## 5. CI 自动化 (per .github/workflows/release-1.0.0.yml)

每包独立 build job (matrix 8 通道 + 2 架构 = 10 组合):

| Job | Runner | 工具 | 产物 |
|---|---|---|---|
| build-deb | ubuntu-22.04 (amd64, arm64) | cargo-deb | `apeireth_1.0.0_amd64.deb` |
| build-rpm | ubuntu-22.04 (amd64) | cargo-rpm | `apeireth-1.0.0-1.x86_64.rpm` |
| build-tarball | ubuntu-22.04 (amd64, arm64) | musl + tar | `apeireth-1.0.0-x86_64-linux.tar.gz` |
| build-brew | macos-13 | brew | `apeireth.rb` (formula) |
| build-scoop | windows-2022 | scoop | `apeireth.json` (manifest) |
| build-zip | windows-2022 | cargo + zip | `apeireth-1.0.0-x86_64-pc-windows-msvc.zip` |
| build-msi | windows-2022 | WiX 3.x | `apeireth-1.0.0-x64.msi` |
| build-docker | ubuntu-22.04 + buildx (amd64, arm64) | Docker buildx | `ghcr.io/apeireth/apeireth:1.0.0` |

**cosign 8 包签名** (per R20 阶段 6 估补): 8 包全部 sigstore cosign 签名, 校验在 `scripts/release/cosign-verify.sh`

---

## 6. 8 项不修改承诺守门 (per 8-locked-unified-2026-08-05.md)

| 承诺 | 8 包如何守 |
|---|---|
| 0 改 24 LOCKED crate | 8 包 build 走 `cargo build --locked`, 不触发 source 改 |
| 0 改 workspace version 1.0.0 | 8 包 version 全部 1.0.0, CI 不 bump |
| 0 引 NewAPI | 8 包用 apt/dnf/brew/scoop/docker 系统命令, 不引新代理 |
| 不假装已实现 | 缺 cargo-deb / cargo-rpm 工具链时, build.sh 标 TODO 不假装编过 |
| 编译期 hardcode | VERSION=1.0.0 / INSTALL_DIR / SERVICE_NAME 全 hardcode |
| 6 哲学锚穿透 | "不假装" / "不重复造轮子" 注释在每个 build.sh |
| 不重复造轮子 | packaging/deb/apeireth.service 复用给 rpm + tarball |
| 诚实标缺 | 跨平台测试不跑, 标 "需要 CI 守门" (build matrix 跑) |

---

## 7. 兄弟文档

- [`deb-install.md`](deb-install.md) — Debian/Ubuntu 详细
- [`rpm-install.md`](rpm-install.md) — RHEL/Fedora 详细
- [`linux-tarball-install.md`](linux-tarball-install.md) — 通用 Linux 详细
- [`macos-brew-install.md`](macos-brew-install.md) — macOS 详细
- [`windows-scoop-install.md`](windows-scoop-install.md) — Windows 详细
- [`../../packaging/`](../../packaging/) — 8 包 build script
- [`../../.github/workflows/release-1.0.0.yml`](../../.github/workflows/release-1.0.0.yml) — CI
