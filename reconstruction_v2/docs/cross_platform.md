# Apeireth 2.0 全平台适配指南 (Windows, Linux, macOS)

> **核心原则**：Write Once, Run Everywhere · 平台原生隔离 · 零平台专有死锁

Apeireth 2.0 原生支持三大主流操作系统：**Windows (x86_64/aarch64)**、**Linux (x86_64/aarch64)** 和 **macOS (Apple Silicon / Intel)**。

---

## 一、 平台适配矩阵 (Platform Matrix)

| 功能领域 | Windows 适配方案 | Linux 适配方案 | macOS 适配方案 |
|---|---|---|---|
| **进程与资源沙箱** (`apeireth-tools/sandbox`) | `JobObject` (256MB 内存限制, KillOnClose) + `RestrictedToken` | `setrlimit` (`RLIMIT_AS`) + `prctl` (`PR_SET_NO_NEW_PRIVS`) | `setrlimit` (`RLIMIT_AS`, `RLIMIT_DATA`, `RLIMIT_NOFILE`) |
| **Shell 命令执行** (`apeireth-tools/builtin/shell`) | `cmd.exe /C` / `pwsh` 参数安全转义 | `/bin/sh -c` POSIX 转义 | `/bin/sh -c` POSIX 转义 |
| **沙箱文件系统** (`apeireth-tools/builtin/fs`) | 驱动器路径规范化 (`C:\...`) + 阻止 `..` 穿越 | POSIX 根目录 Jail + 阻止 `..` 穿越 | POSIX 根目录 Jail + 阻止 `..` 穿越 |
| **默认存储路径** (`apeireth-storage/pool`) | `%APPDATA%\apeireth\memory.sqlite` | `~/.local/share/apeireth/memory.sqlite` | `~/Library/Application Support/apeireth/memory.sqlite` |
| **网络与 S4 出站** (`apeireth-gateway/egress`) | 统一 IPv4/IPv6 私有网段检测 + 80/443 白名单 | 统一 IPv4/IPv6 私有网段检测 + 80/443 白名单 | 统一 IPv4/IPv6 私有网段检测 + 80/443 白名单 |
| **异步与事件驱动** (`apeireth-core/bus`) | Tokio 异步事件通道 | Tokio 异步事件通道 | Tokio 异步事件通道 |

---

## 二、 跨平台编译与运行

### 1. Windows
```powershell
# 编译并运行测试
cargo test --workspace
# 启动网关服务
cargo run -p apeireth-cli -- serve
```

### 2. Linux (Ubuntu / Debian / RHEL / Arch)
```bash
# 安装基础编译依赖 (libsqlite3 / openssl)
sudo apt-get install -y build-essential libsqlite3-dev pkg-config

# 编译并运行测试
cargo test --workspace
# 启动网关服务
cargo run -p apeireth-cli -- serve
```

### 3. macOS (Apple Silicon / Intel)
```bash
# 编译并运行测试
cargo test --workspace
# 启动网关服务
cargo run -p apeireth-cli -- serve
```

---

## 三、 CI/CD 多平台矩阵 (GitHub Actions 参考)

```yaml
name: Cross-Platform CI

on: [push, pull_request]

jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Run Tests
        working-directory: reconstruction_v2
        run: cargo test --workspace
```
