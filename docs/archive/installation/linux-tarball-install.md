# 通用 Linux / Unix 安装指南 (tarball)

> **平台**: 任何 Linux (AUR / Alpine / Devuan / 老 RHEL) / WSL2 / musl 兼容 Unix
> **包格式**: `.tar.gz` (gzip 压缩 tarball)
> **链接方式**: musl 静态 (零运行时依赖)
> **D-06 决策**: 主人 2026-08-05 20:53 拍 A, 8 包齐发 + Linux 4 包重点 (tarball 是第 3 重点, Docker 是第 4)

---

## 0. 为什么选 tarball?

| 场景 | 推荐 |
|---|---|
| **AUR** (Arch / Manjaro) | tarball (makedepends 走 packaging/tarball/build.sh) |
| **Alpine** (musl 原生) | tarball (apt/dnf 都没) |
| **Devuan** (无 systemd) | tarball (跳过 systemd 部署) |
| **老发行版** (RHEL 7 / Debian 9) | tarball (新 systemd 不可用) |
| **WSL2** (Windows Linux 子系统) | tarball (无 systemd, 手起进程) |
| **容器镜像 scratch** | tarball (单 binary, 比 Docker 更小) |
| **embedded / NAS** (synology / QNAP) | tarball (定制路径) |

**musl 静态 = 0 运行时依赖** — ldd 应显示 `not a dynamic executable`, 可放到任何 Linux 跑。

---

## 1. 系统要求

| 组件 | 最低 | 推荐 |
|---|---|---|
| 内核 | Linux 3.10+ | 5.10+ |
| glibc / musl | 任意 (静态) | musl 1.2+ |
| systemd (可选) | 任意 | 245+ (用 Type=notify) |
| 磁盘 | 100 MB | 200 MB |
| 内存 | 2 GB | 4 GB |

**无 systemd 也可装** — 跳过 systemd 部署, 手起进程即可 (见 §6)

---

## 2. 一行安装 (推荐)

```bash
# 2.1 下载 (默认 amd64)
curl -fsSL -O https://github.com/apeireth/apeireth-rust/releases/download/v1.0.0/apeireth-1.0.0-x86_64-linux.tar.gz

# 2.2 校验 sha256
curl -fsSL -O https://github.com/apeireth/apeireth-rust/releases/download/v1.0.0/apeireth-1.0.0-x86_64-linux.tar.gz.sha256
sha256sum -c apeireth-1.0.0-x86_64-linux.tar.gz.sha256

# 2.3 解包 + 装 (官方脚本)
sudo ./scripts/install/install-tarball.sh
```

或手动:

```bash
sudo mkdir -p /opt/apeireth
sudo tar -xzf apeireth-1.0.0-x86_64-linux.tar.gz -C /opt
sudo mv /opt/apeireth-1.0.0-x86_64-linux/* /opt/apeireth/
sudo ln -sf /opt/apeireth/bin/apeireth /usr/local/bin/apeireth

# 验证 0 依赖
ldd /opt/apeireth/bin/apeireth
# 期望: "not a dynamic executable" (musl 静态)
```

---

## 3. systemd 集成 (可选, 推存在)

如果系统有 systemd (e.g. Ubuntu 22.04 / Fedora 40 / Arch):

```bash
# 3.1 拷 unit
sudo cp /opt/apeireth/systemd/apeireth.service /etc/systemd/system/

# 3.2 启用 + 启动
sudo systemctl daemon-reload
sudo systemctl enable --now apeireth
```

如无 systemd (Alpine / Devuan / WSL2), 跳到 §6 手起。

---

## 4. 健康检查

```bash
# 4.1 状态
systemctl status apeireth
# 或 (无 systemd): pgrep -af apeireth

# 4.2 /health
curl -fsS http://localhost:8080/health
# 期望: {"status":"ok","version":"1.0.0"}
```

---

## 5. 配置文件

```bash
# 5.1 配置模板
cat /opt/apeireth/config/apeireth.env.example

# 5.2 拷到 /etc/apeireth/env
sudo mkdir -p /etc/apeireth
sudo cp /opt/apeireth/config/apeireth.env.example /etc/apeireth/env
sudo vi /etc/apeireth/env  # 改 DB_URL / REDIS_URL 等

# 5.3 重启
sudo systemctl restart apeireth
```

---

## 6. 无 systemd 手起 (Alpine / Devuan / WSL2)

```bash
# 6.1 准备数据目录
sudo mkdir -p /var/lib/apeireth /var/log/apeireth
sudo chown -R apeireth:apeireth /var/lib/apeireth /var/log/apeireth 2>/dev/null || sudo useradd -r -s /sbin/nologin apeireth

# 6.2 起进程
sudo -u apeireth /opt/apeireth/bin/apeireth serve
# 或前台跑: /opt/apeireth/bin/apeireth serve

# 6.3 后台跑 (无 systemd)
nohup sudo -u apeireth /opt/apeireth/bin/apeireth serve > /var/log/apeireth/apeireth.out 2>&1 &
```

**Alpine 用 OpenRC** (如需):

```bash
sudo apk add openrc
sudo rc-update add apeireth default
# /etc/init.d/apeireth 由 packaging/tarball/build.sh R20 阶段 4 估补
```

---

## 7. AUR 安装 (Arch / Manjaro)

AUR `PKGBUILD` 由 R20 阶段 4 估补 (per packaging/tarball/build.sh 注释), 暂用 makepkg:

```bash
# 暂方案: 下载 tarball 走 §2
yay -S apeireth-bin  # R20 阶段 4 估补
```

`PKGBUILD` 骨架 (估补时实现):

```bash
pkgname=apeireth
pkgver=1.0.0
source=("https://github.com/apeireth/apeireth-rust/releases/download/v${pkgver}/apeireth-${pkgver}-x86_64-linux.tar.gz")
sha256sums=('REPLACE_AT_RELEASE_TIME')

package() {
    install -Dm755 apeireth-${pkgver}-x86_64-linux/bin/apeireth "${pkgdir}/usr/bin/apeireth"
    install -Dm644 apeireth-${pkgver}-x86_64-linux/systemd/apeireth.service "${pkgdir}/usr/lib/systemd/system/apeireth.service"
}
```

---

## 8. 升级

```bash
# 8.1 停服
sudo systemctl stop apeireth  # 或 pkill apeireth

# 8.2 备份旧版
sudo mv /opt/apeireth /opt/apeireth.bak.$(date +%Y%m%d)

# 8.3 解新版
sudo ./scripts/install/install-tarball.sh /path/to/apeireth-1.0.1-x86_64-linux.tar.gz

# 8.4 验证 + 删旧
/opt/apeireth/bin/apeireth --version
sudo rm -rf /opt/apeireth.bak.*
```

---

## 9. 卸载

```bash
# 9.1 停服 + 禁 systemd
sudo systemctl disable --now apeireth 2>/dev/null

# 9.2 删文件
sudo rm -rf /opt/apeireth /usr/local/bin/apeireth
sudo rm -f /etc/systemd/system/apeireth.service
sudo systemctl daemon-reload

# 9.3 删数据
sudo rm -rf /var/lib/apeireth /var/log/apeireth /etc/apeireth
```

通用卸载:

```bash
sudo ./scripts/install/uninstall-all.sh --channel tarball
```

---

## 10. 故障排查

| 症状 | 排查 |
|---|---|
| `ldd` 显示 `libc.so.6 => /lib/...` | 不是 musl 静态, 走 deb/rpm 渠道 |
| WSL2 端口 8080 转不过 | Windows 防火墙 / `netsh interface portproxy add v4tov4 listenport=8080 listenaddress=0.0.0.0 connectport=8080 connectaddress=127.0.0.1` |
| Alpine 缺 `ca-certificates` | 装的是 musl 静态, 但 TLS 需 `/etc/ssl/certs/ca-certificates.crt`, `apk add ca-certificates` |
| 启动报 `Permission denied: /var/log/apeireth` | `chown apeireth:apeireth /var/log/apeireth` |
| musl 不兼容某个 .so | 全静态编译, 无 .so 依赖, 应 0 错; 否则走 deb/rpm |

---

## 11. 参考

- 蓝图: [`docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md §3.4`](../stage4/v09021-rust-translation-blueprint-2026-08-05.md)
- packaging: [`packaging/tarball/`](../../packaging/tarball/)
- 兄弟文档: [`deb-install.md`](deb-install.md) / [`rpm-install.md`](rpm-install.md) / [`package-comparison.md`](package-comparison.md)
- musl libc: <https://musl.libc.org/>
- AUR: <https://aur.archlinux.org/>
