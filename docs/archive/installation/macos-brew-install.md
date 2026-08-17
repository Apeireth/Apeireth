# macOS Homebrew 安装指南

> **平台**: macOS 11 Big Sur+ (Intel / Apple Silicon)
> **包格式**: Homebrew Formula (Ruby DSL)
> **服务管理**: launchd (brew services)
> **D-06 决策**: 主人 2026-08-05 20:53 拍 A, 8 包齐发

---

## 0. 系统要求

| 组件 | 最低版本 | 推荐版本 | 备注 |
|---|---|---|---|
| macOS | 11 (Big Sur) | 14 (Sonoma) | `depends_on :macos => :high_sierra` (formula) |
| Homebrew | 4.0+ | 4.3+ | `brew --version` |
| Xcode CLT | 14+ | 15+ | `xcode-select --install` (编译依赖) |
| 磁盘 | 200 MB | 500 MB | |
| 内存 | 2 GB | 4 GB | |

**架构**: `x86_64` (Intel) / `arm64` (Apple Silicon) — Formula 走 `cargo build --release`, universal 估补

**Linuxbrew** (Linux 旁路): 见 `linux-tarball-install.md`, 同一 Formula 兼容, 但本节聚焦 macOS

---

## 1. 一行安装 (推荐)

```bash
# 1.1 加 tap (公式在 apeireth/homebrew-tap 仓库)
brew tap apeireth/tap

# 1.2 装
brew install apeireth/tap/apeireth

# 1.3 启服务 (launchd)
brew services start apeireth
```

或用仓库内脚本:

```bash
./scripts/install/install-brew.sh
```

---

## 2. Formula 依赖 (per packaging/brew/apeireth.rb)

**编译时** (`:build`):

- `rust` (Homebrew 装的 1.80+)
- `pkg-config`
- `openssl@3`
- `sqlite`
- `libgit2`

**运行时**:

- `openssl@3` (TLS)
- `sqlite` (本地数据库)
- `libgit2` (git 集成)

**bottle** 估补 (R20 阶段 4): pre-built binary, 跳过编译, 装更快

---

## 3. launchd 集成 (自动)

Formula 配 `service do ... end` 块, `brew services start apeireth` 自动:

- 写 `~/Library/LaunchAgents/homebrew.mxcl.apeireth.plist`
- `launchctl load` 启动
- 日志: `$(brew --prefix)/var/log/apeireth.log` + `apeireth.err`
- 进程退出自动重启 (keep_alive true)

查看状态:

```bash
brew services list | grep apeireth
# 期望: apeireth started <user> ~/Library/LaunchAgents/...

# 或直接看 launchd
launchctl list | grep apeireth
```

---

## 4. 健康检查

```bash
# 4.1 状态
brew services info apeireth

# 4.2 /health
curl -fsS http://localhost:8080/health
# 期望: {"status":"ok","version":"1.0.0"}

# 4.3 二进制
which apeireth
# 期望: /opt/homebrew/bin/apeireth (Apple Silicon) 或 /usr/local/bin/apeireth (Intel)

# 4.4 版本
apeireth --version
# 期望: apeireth 1.0.0
```

---

## 5. 配置文件

**默认** (per Formula `service do` 块):

- `working_dir = HOMEBREW_PREFIX` (`/opt/homebrew` 或 `/usr/local`)
- 日志: `$(brew --prefix)/var/log/apeireth.{log,err}`
- 环境变量: `HOMEBREW_PREFIX` 自动设置

**自定义配置** (`~/.apeireth/config.toml`):

```bash
mkdir -p ~/.apeireth
cat > ~/.apeireth/config.toml <<'EOF'
[server]
bind = "127.0.0.1:8080"
workers = 4

[storage]
db_url = "postgresql://apeireth:secret@localhost:5432/apeireth"
EOF

# 重启服务
brew services restart apeireth
```

---

## 6. 升级

```bash
# 6.1 拉新 formula + 二进制
brew update
brew upgrade apeireth/tap/apeireth

# 6.2 重启
brew services restart apeireth
```

**回滚** (如果新版有 bug):

```bash
brew uninstall apeireth
brew install apeireth/tap/apeireth@1.0.0  # 估补: 旧版 formula
# 或: brew switch apeireth 1.0.0
```

---

## 7. 卸载

```bash
# 7.1 简版
brew services stop apeireth
brew uninstall apeireth

# 7.2 完整 (含数据 + 配置)
brew services stop apeireth
brew uninstall apeireth
rm -rf ~/.apeireth /opt/homebrew/var/log/apeireth.*  # Apple Silicon
rm -rf ~/.apeireth /usr/local/var/log/apeireth.*      # Intel
brew untap apeireth/tap  # 删 tap
```

通用卸载:

```bash
./scripts/install/uninstall-all.sh --channel brew
```

---

## 8. 故障排查

| 症状 | 排查 |
|---|---|
| `brew install` 报 `cargo: command not found` | `brew install rust` 装 Rust |
| 编译报 `openssl/ssl.h: No such file` | Formula 已加 `depends_on "openssl@3" => :build`, 应自动解决 |
| 端口 8080 占用 | `lsof -i:8080` 查谁占, `brew services stop` 别的服务 |
| `brew services start` 无效 | `brew services list` 看状态, 手动: `apeireth serve &` |
| Apple Silicon 报 `Bad CPU type` | Rosetta 2 缺, `softwareupdate --install-rosetta` |
| bottle 估补后 SHA 不匹配 | `HOMEBREW_NO_AUTO_UPDATE=1 brew install` 强制重编 |

---

## 9. Linuxbrew 旁路

Homebrew 公式 99% 兼容 Linuxbrew, 但 launchd 不可用, 改 systemd:

```bash
# Linuxbrew 装
brew tap apeireth/tap
brew install apeireth/tap/apeireth

# 启服务 (Linuxbrew 走 systemd, 不是 launchd)
# 注: 需包装一下, 见 packaging/brew/build.sh 注释
sudo tee /etc/systemd/system/apeireth.service <<EOF
[Unit]
Description=Apeireth OS
After=network.target

[Service]
ExecStart=$(brew --prefix)/bin/apeireth serve
Restart=on-failure
User=$USER

[Install]
WantedBy=multi-user.target
EOF
sudo systemctl daemon-reload
sudo systemctl enable --now apeireth
```

---

## 10. 参考

- 蓝图: [`docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md §3.4`](../stage4/v09021-rust-translation-blueprint-2026-08-05.md)
- packaging: [`packaging/brew/`](../../packaging/brew/)
- 兄弟文档: [`windows-scoop-install.md`](windows-scoop-install.md) / [`package-comparison.md`](package-comparison.md)
- Homebrew Formula Cookbook: <https://docs.brew.sh/Formula-Cookbook>
- Homebrew Services: <https://github.com/Homebrew/homebrew-services>
