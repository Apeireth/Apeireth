# Getting Started — 5 分钟跑通

> **目标**: 5 分钟内 clone + cargo build + cargo run, 跑通 `apeireth-tui` 跟 `apeireth-api`.
> **前提**: Rust 1.80 stable + Cargo + Git + (可选) GitHub CLI (`gh`).

## 1. 环境要求

| 工具 | 版本 | 验证命令 |
|------|------|---------|
| Rust | 1.80 stable | `rustc --version` |
| Cargo | 1.80+ | `cargo --version` |
| Git | 2.30+ | `git --version` |
| GitHub CLI (1.0 release 配 remote 用) | 2.0+ | `gh --version` |

详见 [`rust-toolchain.toml`](https://github.com/apeireth/apeireth-rust/blob/main/rust-toolchain.toml) (主仓 1.80 stable).

## 2. Clone (1.0 release 后)

```bash
# 1.0 release 后从 GitHub clone (per 决策 #55 §2.6 + 决策 #58 §5)
git clone https://github.com/apeireth/apeireth-rust.git
cd apeireth-rust
```

**当前**: 1.0 release 未发布, 主仓在 `Apeireth-rust/`, 等整合 #5 commit + 1.0 release tag 后 git push 公开.

## 3. Build (per 决策 #55 §8 + handoff §8.2)

```bash
# 完整 workspace build (~3-5 min, 第一次编译)
cargo build --workspace --release

# 等价:
cargo build --workspace
```

**预期**: 0 error, 90+ sub-crate 全部编译通过.

## 4. Test (4100+ tests)

```bash
# 完整 workspace test (~5-10 min)
cargo test --workspace --release

# 快速子集:
cargo test -p apeireth-core
```

**预期**: 0 failed, 4100+ tests pass (per R125-16 + P12-1 verify).

## 5. Run TUI (瘦客户端, per 决策 #11 阶段 4 frontend-proposal)

```bash
# TUI 启动 (默认连 localhost:8080)
cargo run --bin apeireth-tui --release

# 或先 install:
cargo install --path crates/apeireth-tui --locked
apeireth-tui
```

**预期**: TUI 启动, 5s smoke 不自退 (per 8 步 verify Step 4).

## 6. Run API (HTTP server)

```bash
# API 启动 (默认监听 0.0.0.0:8080)
cargo run --bin apeireth-api --release

# 或先 install:
cargo install --path crates/apeireth-api --locked
apeireth-api
```

**预期**: API 启动, 5s smoke 不自退 (per 8 步 verify Step 5).

**Endpoints**: 见 [API Reference](api.md).

## 7. 完整 8 步 verify (1.0 release 前必跑)

```bash
# PowerShell (Windows 优先, 主人 8/10 跑过夜)
pwsh scripts/release/verify-1.0-pre-tag.ps1

# Bash (Linux/macOS/WSL)
bash scripts/release/verify-1.0-pre-tag.sh
```

**8 步**:
1. 修 session working dir + master HEAD + Cargo.toml
2. `cargo build --workspace`
3. `cargo test --workspace` (4100+ tests)
4. `cargo run --bin apeireth-tui` 5s smoke
5. `cargo run --bin apeireth-api` 5s smoke
6. `cargo audit + cargo deny`
7. 24 LOCKED 入口签名 0 改 verify
8. 8 硬墙 0 越界 + 0 装 PASS 严守 verify

**8 步全 PASS → 拍板整合 #5 commit (Mavis 自决, per 决策 #62)**.

## 常见问题

### Q: `cargo build --workspace` 失败

**A**: 检查 Rust 版本 (`rustc --version` >= 1.80) + 网络代理 + Cargo.lock 锁文件更新 (`cargo update`).

### Q: `cargo test --workspace` 部分 test fail

**A**: 0 装 PASS 严守 (✅ 8 真实施 + ⏳ 0 + ❌ 1), fail 必为已知问题. 见 [`reports/verify-1.0-pre-tag-YYYY-MM-DD-HHMM.md`](https://github.com/apeireth/apeireth-rust/tree/main/reports).

### Q: 端口 8080 占用

**A**: 设置 `APEIRETH_API_PORT=8888` 环境变量.

### Q: TUI 启动后立即退

**A**: 检查 API 是否启动 (`cargo run --bin apeireth-api`), TUI 默认连 `http://localhost:8080`.

## 下一步

- 📖 读 [API Reference](api.md) — 13 键 verdict cache + 30 维 V0.5 + 6 重守门 v7
- 🗺️ 读 [Roadmap](roadmap.md) — 1.0 → 2.0 路线图
- 🏛️ 读 [Architecture](architecture.md) — 8 哲学锚 + 24 LOCKED + 决策链

## 必读

- 📄 [README.md](https://github.com/apeireth/apeireth-rust/blob/main/README.md) — 项目主页
- 📄 [INSTALLATION_GUIDE-1.0.md](https://github.com/apeireth/apeireth-rust/blob/main/docs/1.0-release-prep/INSTALLATION_GUIDE-1.0.md) — 详细安装指南
- 📄 [CONTRIBUTING.md](https://github.com/apeireth/apeireth-rust/blob/main/CONTRIBUTING.md) — 贡献指南
- 📄 [CHANGELOG.md](changelog.md) — 变更日志
- 📄 [ROADMAP.md](roadmap.md) — 路线图
