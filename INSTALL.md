# INSTALL.md — Apeireth-rust 安装步骤

> **性质**: 接手团队第一份必读——三平台安装（Windows / Linux / macOS）+ 验证步骤。
> **依据**: 主人 2026-07-31 "开干前补齐 4 件套" + rust-toolchain.toml 锁定 Rust 1.80 stable。
> **commit 锚**: 23513387（v3 修订）。

---

## 📋 系统要求

| 平台 | 最低版本 | 推荐版本 |
|---|---|---|
| **Windows** | Windows 10 (1903+) | Windows 11 |
| **Linux** | Ubuntu 20.04 / Debian 11 | Ubuntu 22.04+ |
| **macOS** | macOS 11 Big Sur | macOS 13+ |

| 工具 | 最低版本 | 推荐版本 |
|---|---|---|
| **Rust** | 1.80 stable | 1.80 stable（rust-toolchain.toml 锁定）|
| **Cargo** | 1.80 | 1.80 |
| **Git** | 2.30+ | 2.40+ |
| **cmake** | 3.20+ | 3.25+（编译 sled）|
| **Python** | 3.11+ | 3.13（PyO3 桥）|
| **SQLite** | 3.35+ | 3.40+（apeireth-memory 持久化）|

---

## 🪟 Windows 安装

### 步骤 1：安装 Rust

```powershell
# 下载并安装 rustup-init.exe
# https://rustup.rs/

# 安装 1.80 stable（与 rust-toolchain.toml 锁定一致）
rustup install 1.80
rustup default 1.80

# 安装必要组件
rustup component add rustfmt clippy rust-src
```

### 步骤 2：安装 Visual Studio Build Tools

```powershell
# 下载并安装 Visual Studio Build Tools 2022
# https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022
# 必须勾选 "C++ build tools" + "Windows 11 SDK"
```

### 步骤 3：安装 Git + cmake

```powershell
# 通过 winget 或 choco 安装
winget install Git.Git
winget install Kitware.CMake
```

### 步骤 4：Clone + Build

```powershell
git clone https://github.com/apeireth/apeireth-rust.git
cd apeireth-rust
cargo build --workspace
cargo test --workspace
```

### 步骤 5：验证

```powershell
cargo run --bin apeireth-cli session
# 应该看到：欢迎信息 + 启动 session
```

---

## 🐧 Linux 安装（Ubuntu/Debian）

### 步骤 1：安装 Rust

```bash
# 安装 rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 1.80 stable
rustup install 1.80
rustup default 1.80

# 安装必要组件
rustup component add rustfmt clippy rust-src
```

### 步骤 2：安装系统依赖

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install -y build-essential cmake git pkg-config libssl-dev

# 安装 Python + pip（如需 PyO3 桥）
sudo apt install -y python3 python3-pip python3-dev
```

### 步骤 3：Clone + Build

```bash
git clone https://github.com/apeireth/apeireth-rust.git
cd apeireth-rust
cargo build --workspace
cargo test --workspace
```

### 步骤 4：验证

```bash
cargo run --bin apeireth-cli session
# 应该看到：欢迎信息 + 启动 session
```

---

## 🍎 macOS 安装

### 步骤 1：安装 Rust

```bash
# 安装 rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 1.80 stable
rustup install 1.80
rustup default 1.80

# 安装必要组件
rustup component add rustfmt clippy rust-src
```

### 步骤 2：安装 Xcode Command Line Tools

```bash
xcode-select --install
```

### 步骤 3：安装 Homebrew + cmake

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
brew install cmake pkg-config openssl
```

### 步骤 4：Clone + Build

```bash
git clone https://github.com/apeireth/apeireth-rust.git
cd apeireth-rust
cargo build --workspace
cargo test --workspace
```

### 步骤 5：验证

```bash
cargo run --bin apeireth-cli session
```

---

## ✅ 验证清单

成功安装后，运行以下命令验证：

```bash
# 1. Build（应该 0 error）
cargo build --workspace

# 2. Test（应该 6+ tests pass，apeireth-core 当前）
cargo test --workspace

# 3. Clippy（应该 0 warning）
cargo clippy --workspace -- -D warnings

# 4. Format（应该 0 diff）
cargo fmt --check

# 5. Hello World（应该看到欢迎信息）
cargo run --bin apeireth-cli session
```

如果全部通过 = **安装成功**，可以开始贡献代码。

---

## 🐛 常见问题

### Q1: `error: linker 'cc' not found`（Linux/macOS）

**解决**：安装 C 编译器
```bash
# Ubuntu/Debian
sudo apt install build-essential

# macOS
xcode-select --install
```

### Q2: `error: Microsoft Visual C++ 14.0 or greater is required`（Windows）

**解决**：安装 Visual Studio Build Tools 2022（见 Windows 步骤 2）

### Q3: `error: failed to run custom build command for openssl-sys`

**解决**：
```bash
# macOS
brew install openssl
export OPENSSL_DIR=$(brew --prefix openssl)

# Linux
sudo apt install libssl-dev pkg-config
```

### Q4: `cargo build` 慢/卡

**解决**：配置 cargo 国内镜像（可选）
```bash
mkdir -p ~/.cargo
cat > ~/.cargo/config.toml <<EOF
[source.crates-io]
replace-with = 'tuna'

[source.tuna]
registry = "sparse+https://mirrors.tuna.tsinghua.edu.cn/crates.io-index/"

[net]
git-fetch-with-cli = true
EOF
```

### Q5: `cargo test` 失败

**解决**：查看具体错误，可能是：
- Python 版本不符（需要 3.11+）
- SQLite 版本不符（需要 3.35+）
- PyO3 链接失败（需要 Python dev headers）

---

## 📂 下一步

安装完成后：

1. **读 README.md**（顶层入口）
2. **读 CONTRIBUTING.md**（PR 流程）
3. **读 docs/00-R14-START-HERE.md**（5/30/60/240 分钟路径）
4. **读 docs/ROADMAP.md**（路线图）
5. **读 docs/GLOSSARY.md**（17 项术语）
6. **运行 examples/hello_world.rs**（最小 demo）

---

_安装指南 v1 修订版（leader 亲自产出）._
_依据主人 2026-07-31 "开干前补齐 4 件套" + rust-toolchain.toml 1.80 锁定._
_主哲学 6 锚穿透._