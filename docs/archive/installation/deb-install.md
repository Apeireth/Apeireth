# Debian / Ubuntu 安装指南 (.deb)

> **平台**: Debian 11+ / Ubuntu 20.04+
> **包格式**: `.deb` (Debian Package)
> **服务管理**: systemd (Type=notify)
> **D-06 决策**: 主人 2026-08-05 20:53 拍 A, 8 包齐发 + Linux 4 包重点 (deb 是第 1 重点)

---

## 0. 系统要求

| 组件 | 最低版本 | 推荐版本 | 备注 |
|---|---|---|---|
| Debian | 11 (bullseye) | 12 (bookworm) | libc6 ≥ 2.31 |
| Ubuntu | 20.04 (focal) | 22.04 (jammy) | libc6 ≥ 2.31 |
| systemd | 245+ | 252+ | Type=notify 需要 systemd 245+ |
| 磁盘 | 200 MB | 500 MB | 含 PostgreSQL 16 / Redis 7 |
| 内存 | 2 GB | 4 GB | 12-Factor 进程隔离 |

**架构**: `amd64` (x86_64) / `arm64` (aarch64) — `cargo-deb` 上游支持

---

## 1. 一行安装 (推荐)

```bash
# 1.1 从 GitHub release 下载 (v1.0.0)
curl -fsSL -O https://github.com/apeireth/apeireth-rust/releases/download/v1.0.0/apeireth_1.0.0_amd64.deb

# 1.2 校验 sha256 (建议)
curl -fsSL -O https://github.com/apeireth/apeireth-rust/releases/download/v1.0.0/apeireth_1.0.0_amd64.deb.sha256
sha256sum -c apeireth_1.0.0_amd64.deb.sha256

# 1.3 装
sudo apt install ./apeireth_1.0.0_amd64.deb
```

或用仓库内脚本 (CI 产物 / 本地 build):

```bash
sudo ./scripts/install/install-deb.sh
```

或显式指定路径:

```bash
sudo ./scripts/install/install-deb.sh ./target/x86_64-unknown-linux-gnu/debian/apeireth_1.0.0_amd64.deb
```

---

## 2. systemd 集成 (自动)

`apt install` 完成后, packaging/deb/apeireth.service 自动部署到 `/etc/systemd/system/apeireth.service`, 含:

- **Type=notify** — sd_notify 协议, 启动完成才标 active
- **User=apeireth / Group=apeireth** — 非 root (per 蓝图 §3.4 5 守门 non-root)
- **ProtectSystem=strict** — `/usr` 只读
- **ReadWritePaths=/var/lib/apeireth /var/log/apeireth** — 数据/日志专属
- **LimitNOFILE=65536** — 高并发 WS 连接 (per 12-Factor)
- **EnvironmentFile=-/etc/apeireth/env** — 可选覆盖

启用 + 启动:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now apeireth
```

---

## 3. 健康检查

```bash
# 3.1 状态
systemctl status apeireth
# 期望: Active: active (running)

# 3.2 HTTP /health (默认 :8080)
curl -fsS http://localhost:8080/health
# 期望: {"status":"ok","version":"1.0.0"}

# 3.3 指标 (per Dockerfile EXPOSE 9090)
curl -fsS http://localhost:9090/metrics
# 期望: Prometheus 格式
```

**`/health` 期望 200, 3 次重试失败则查日志**:

```bash
journalctl -u apeireth -n 50 --no-pager
```

---

## 4. 配置文件

| 路径 | 用途 | 是否覆盖 |
|---|---|---|
| `/etc/apeireth/config.toml` | 主配置 | `%config(noreplace)` — 升级不覆盖 |
| `/etc/apeireth/env` | 环境变量 (可选) | 自管 |
| `/var/lib/apeireth/` | 数据 (SQLite/PG) | 自管 |
| `/var/log/apeireth/` | 日志 | 自管 |

**`/etc/apeireth/env` 示例**:

```bash
APEIRETH_HOME=/var/lib/apeireth
APEIRETH_CONFIG=/etc/apeireth/config.toml
APEIRETH_DB_URL=postgresql://apeireth:secret@localhost:5432/apeireth
APEIRETH_REDIS_URL=redis://localhost:6379/0
APEIRETH_LLM_BACKEND=scripted
```

---

## 5. 升级

```bash
# 5.1 拉新 .deb
curl -fsSL -O https://github.com/apeireth/apeireth-rust/releases/download/v1.0.1/apeireth_1.0.1_amd64.deb

# 5.2 apt 升级 (自动重启服务)
sudo apt install ./apeireth_1.0.1_amd64.deb
```

**数据迁移**: v2.0.0-alpha → v1.0.0 SQLite→PostgreSQL 一次性迁移见 `scripts/upgrade/v2.0.0-alpha-to-v1.0.0.sh`

---

## 6. 卸载

```bash
# 6.1 简版 (包 + 保留配置)
sudo apt remove apeireth

# 6.2 完整 (包 + 配置 + 数据 + 日志)
sudo apt remove --purge apeireth
sudo rm -rf /var/lib/apeireth /var/log/apeireth /etc/apeireth
```

或用通用卸载 (自动检测已装通道):

```bash
sudo ./scripts/install/uninstall-all.sh
```

---

## 7. 故障排查

| 症状 | 排查 |
|---|---|
| `apt install` 报依赖缺 | `apt --fix-broken install` (常见: libc6 / libssl3) |
| `systemctl start` 失败 | `journalctl -u apeireth -n 100` |
| `/health` 不响应 | `ss -tlnp \| grep 8080` (端口占用?) / `journalctl -u apeireth` |
| `Type=notify` 没激活 | systemd < 245 升级, 或 `systemd-analyze` 验证 |
| 升级后配置丢 | 检查 `/etc/apeireth/config.toml.dpkg-old` (旧配置保留) |

---

## 8. 参考

- 蓝图: [`docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md §3.4`](../stage4/v09021-rust-translation-blueprint-2026-08-05.md)
- packaging: [`packaging/deb/`](../../packaging/deb/)
- 兄弟文档: [`rpm-install.md`](rpm-install.md) / [`linux-tarball-install.md`](linux-tarball-install.md) / [`package-comparison.md`](package-comparison.md)
- Debian Policy: <https://www.debian.org/doc/debian-policy/>
