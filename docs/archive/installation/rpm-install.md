# RHEL / Fedora / CentOS 安装指南 (.rpm)

> **平台**: RHEL 9+ / Fedora 38+ / CentOS Stream 9+ / openSUSE Leap 15.5+
> **包格式**: `.rpm` (RPM Package Manager)
> **服务管理**: systemd (Type=notify)
> **D-06 决策**: 主人 2026-08-05 20:53 拍 A, 8 包齐发 + Linux 4 包重点 (rpm 是第 2 重点)

---

## 0. 系统要求

| 组件 | 最低版本 | 推荐版本 | 备注 |
|---|---|---|---|
| RHEL | 9.0 | 9.4+ | openssl ≥ 3.0 |
| Fedora | 38 | 40+ | sqlite-libs ≥ 3.40 |
| CentOS Stream | 9 | 9+ | libgit2 ≥ 1.7 |
| openSUSE Leap | 15.5 | 15.6+ | openssl ≥ 3.0 |
| systemd | 245+ | 255+ | Type=notify 需要 systemd 245+ |
| 磁盘 | 200 MB | 500 MB | 含 PG 16 / Redis 7 |
| 内存 | 2 GB | 4 GB | 12-Factor |

**架构**: `x86_64` 起步, `aarch64` 估补 (R20 阶段 4)

---

## 1. 一行安装 (推荐)

```bash
# 1.1 从 GitHub release 下载
curl -fsSL -O https://github.com/apeireth/apeireth-rust/releases/download/v1.0.0/apeireth-1.0.0-1.x86_64.rpm

# 1.2 校验 sha256
curl -fsSL -O https://github.com/apeireth/apeireth-rust/releases/download/v1.0.0/apeireth-1.0.0-1.x86_64.rpm.sha256
sha256sum -c apeireth-1.0.0-1.x86_64.rpm.sha256

# 1.3 装 (dnf 自动解依赖: openssl-libs / sqlite-libs / libgit2 / ca-certificates)
sudo dnf install ./apeireth-1.0.0-1.x86_64.rpm
```

或用仓库内脚本:

```bash
sudo ./scripts/install/install-rpm.sh
```

**yum 兼容** (老 RHEL 7 / CentOS 7 估补, 8+ 走 dnf):

```bash
sudo yum install ./apeireth-1.0.0-1.x86_64.rpm
```

---

## 2. systemd 集成 (自动)

RPM spec 含 `%pre` 段创建 `apeireth` 系统用户 (无密码 / 无 shell), `%post` 段触发 systemd reload:

- **Type=notify** — sd_notify 协议
- **User=apeireth / Group=apeireth** — 非 root (per 5 守门)
- **ProtectSystem=strict** — 文件系统隔离
- **ReadWritePaths** — 数据/日志专属
- **LimitNOFILE=65536** — 高并发 WS

启用 + 启动:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now apeireth
```

**`%pre` 段创建用户的 trace** (可选验证):

```bash
getent passwd apeireth
# 期望: apeireth:x:997:997:Apeireth OS daemon:/var/lib/apeireth:/sbin/nologin
```

---

## 3. 健康检查

```bash
# 3.1 状态
systemctl status apeireth
# 期望: Active: active (running)

# 3.2 HTTP /health
curl -fsS http://localhost:8080/health
# 期望: {"status":"ok","version":"1.0.0"}

# 3.3 指标
curl -fsS http://localhost:9090/metrics
```

**失败排查**:

```bash
journalctl -u apeireth -n 50 --no-pager
```

---

## 4. 配置文件

| 路径 | 用途 |
|---|---|
| `/etc/apeireth/` | 配置目录 (`%ghost %config(noreplace)`) |
| `/var/lib/apeireth/` | 数据 (`%attr(750, apeireth, apeireth)`) |
| `/var/log/apeireth/` | 日志 (`%attr(750, apeireth, apeireth)`) |
| `/usr/lib/systemd/system/apeireth.service` | systemd unit |

**关键差异 vs deb**:

- RPM 走 `/var/lib` (FHS), 不是 `/var/lib/apeireth` 别名
- 配置标记 `%ghost` — 不入包, 首次启动生成
- 用户用 `useradd --system` 而非 `adduser`

---

## 5. 升级

```bash
# 5.1 拉新 .rpm
curl -fsSL -O https://github.com/apeireth/apeireth-rust/releases/download/v1.0.1/apeireth-1.0.1-1.x86_64.rpm

# 5.2 dnf 升级 (自动触发 systemd_postun_with_restart)
sudo dnf upgrade ./apeireth-1.0.1-1.x86_64.rpm
```

**重大升级** (e.g. v1.0.0 → v2.0.0): 先跑 `scripts/upgrade/v2.0.0-alpha-to-v1.0.0.sh` (如从 v2.0.0-alpha 升)

---

## 6. 卸载

```bash
# 6.1 简版
sudo dnf remove apeireth
# 注: %postun 段 systemd_postun_with_restart 自动停服

# 6.2 完整 (含数据)
sudo dnf remove apeireth
sudo rm -rf /var/lib/apeireth /var/log/apeireth /etc/apeireth
sudo userdel apeireth  # 删系统用户
sudo groupdel apeireth
```

通用卸载 (自动检测已装通道):

```bash
sudo ./scripts/install/uninstall-all.sh
```

---

## 7. 故障排查

| 症状 | 排查 |
|---|---|
| `dnf install` 报 `Requires: libcrypto.so.10` | RHEL 7 旧 openssl, 升 RHEL 9 / 装 `compat-openssl10` |
| `systemctl start` 失败: `unit not found` | `dnf reinstall apeireth` 触发 `%post` 重新部署 unit |
| `Type=notify` 没激活 | systemd < 245 (RHEL 8 早期) |
| `%pre` 创建用户失败: `user already exists` | 之前残留, `userdel apeireth` 删后再装 |
| SELinux 拒绝写 | `ausearch -m avc -ts recent \| grep apeireth` 查策略 |

**SELinux 守门** (per 蓝图 §3.4 5 守门): 我们的 `packaging/rpm/apeireth.spec` 默认 `Permissive` 模式, 如要 Enforcing 需配 `apeireth_selinux` policy module (R20 阶段 4 估补)

---

## 8. 参考

- 蓝图: [`docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md §3.4`](../stage4/v09021-rust-translation-blueprint-2026-08-05.md)
- packaging: [`packaging/rpm/`](../../packaging/rpm/)
- 兄弟文档: [`deb-install.md`](deb-install.md) / [`linux-tarball-install.md`](linux-tarball-install.md) / [`package-comparison.md`](package-comparison.md)
- Fedora Packaging: <https://docs.fedoraproject.org/en-US/packaging-guidelines/>
- RPM Reference: <https://rpm.org/documentation.html>
