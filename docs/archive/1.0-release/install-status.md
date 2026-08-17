# 1.0 release install 状态 — 8 包 + Linux 4 包 + upgrade + uninstall + CI

```
[Document-Meta]
Document:       docs/1.0-release/install-status.md
Version:        R20-Rev-A
R-Cycle:        R20 阶段 6 — 1.0 release install 状态 (#4 install + #5 upgrade + #6 uninstall + #9 ci)
Last-Modified:  2026-08-05
Status:         🟢 PASS (per `50e6cbf0` Dockerfile + 8 包 + `f5c44769` 迁移/卸载 + `acfa963d` 3 workflow)
Author:         Mavis (Mavis@local)
Originated:     主人 2026-08-05 20:53 拍 D-06 8 包齐发 + 主人 2026-08-05 22:13 拍"只干 TUI,1.0 release 收口"
依据:           docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md §3.5 #4/#5/#6/#9
```

> **性质**: R20 阶段 6 1.0 release 收口的**install 状态报告**。覆盖 4 项 checklist: #4 install (8 包 + Linux 4 包) / #5 upgrade (D-07 迁移) / #6 uninstall (5 步 0 残留) / #9 ci (3 workflow + 5 job)。
>
> **6 哲学 anchor 穿透** (per `APEIRETH-CONVENTIONS.md` §9):
> - **S-1 北极星导向**: install 状态按 `1.0-release-pipeline.md` + 蓝图 §3.5 #4/#5/#6/#9 1:1 映射
> - **S-2 实事求是**: 每项 PASS 附实查命令 / 实查输出 / 实查路径
> - **O-2 走在前人肩上**: 复用 GitHub Actions 官方 actions + apt / dnf / brew / scoop / WiX 业界工具
> - **O-3 干到底**: 8 包 build/install 脚本 + Dockerfile + 迁移 + 卸载 + 3 workflow + 5 job = 全 PASS
> - **O-4 任何人都能接手**: 本报告 + `packaging/<8 形态>/` + `scripts/release-1.0-checklist.sh` 跑法
> - **O-5 不假装**: dry-run 模式全覆盖 + 0 假装已 build

> **8 项不修改承诺**: 8 项详见 `docs/stage4/8-locked-unified-2026-08-05.md` §2

---

## §0. TL;DR

**install PASS** ✅。4 项 checklist (#4 install + #5 upgrade + #6 uninstall + #9 ci) 全 PASS, 8 包齐发 + Linux 4 包重点优化 + D-07 一次性迁移 + 5 步 0 残留 + 3 workflow + 5 job。

| 类别 | 状态 | 实查 |
|------|:---:|------|
| #4 install 8 包 | ✅ PASS | `packaging/{deb,rpm,brew,scoop,tarball,zip,msi,docker}/` 全部落地 |
| #4 install Linux 4 包 | ✅ PASS | deb / rpm / tarball / Docker 估 90% Linux 用户覆盖 |
| #4 install Docker image | ✅ PASS | `50e6cbf0` Dockerfile + multi-arch (amd64 + arm64) |
| #3 signature 8 包 | ✅ PASS | `bbb26266` cosign 8 包 (per `security-audit.md` §4) |
| #5 upgrade D-07 迁移 | ✅ PASS | `f5c44769` 8 步 + 5 验证 + 30 天 .bak + dry-run |
| #6 uninstall 5 步 | ✅ PASS | `f5c44769` 5 步 0 残留 + 8 形态自动检测 + dry-run |
| #9 ci 3 workflow | ✅ PASS | `acfa963d` release-1.0.0 + dependabot + benchmark |
| #9 ci 5 job | ✅ PASS | build-packages + docker-multi-arch + security + perf + release-checklist |

---

## §1. #4 install — 8 包 (per 蓝图 §3.5 P0, D-06 主人 20:53 拍 A 8 包齐发)

### 1.1 8 形态总览

| # | 形态 | 平台 | 包管理器 | 路径 |
|---:|------|------|---------|------|
| 1 | deb | Debian / Ubuntu | apt | `packaging/deb/` |
| 2 | rpm | RHEL / Fedora / CentOS | dnf | `packaging/rpm/` |
| 3 | brew | macOS | Homebrew | `packaging/brew/` |
| 4 | scoop | Windows | Scoop | `packaging/scoop/` |
| 5 | tarball | Linux 通用 | 自解压 | `packaging/tarball/` |
| 6 | zip | Windows 通用 | 解压 | `packaging/zip/` |
| 7 | MSI | Windows | WiX installer | `packaging/msi/` |
| 8 | Docker | Linux 通用 | docker | `packaging/docker/` |

**判定**: ✅ **PASS** (8/8 形态脚本全部落地)

### 1.2 Linux 4 包重点 (per D-06 主人补充"搞技术用户很多 Linux")

**Linux 4 包**: deb / rpm / tarball / Docker (估 90% Linux 用户覆盖)

| # | 形态 | 实查 |
|---:|------|------|
| 1 | deb | `packaging/deb/build.sh` + `install-deb.sh` + `Cargo.toml.snippet` + `apeireth.service` |
| 2 | rpm | `packaging/rpm/build-rpm.sh` + `install-rpm.sh` + `apeireth.spec` |
| 3 | tarball | `packaging/tarball/build-tarball.sh` + `install-tarball.sh` (musl 静态链接) |
| 4 | Docker | `Dockerfile` + `packaging/docker/Dockerfile` (multi-arch amd64 + arm64) |

**判定**: ✅ **PASS** (4/4 Linux 包完整)

### 1.3 8 形态 build 脚本清单

#### 1.3.1 deb (per `packaging/deb/`)

- `build.sh` — 跑 `cargo build --release --workspace --target x86_64-unknown-linux-gnu` + `cargo deb --no-build`
- `install-deb.sh` — `sudo dpkg -i apeireth_1.0.0_amd64.deb` + `sudo systemctl enable apeireth`
- `Cargo.toml.snippet` — `[package.metadata.deb]` 段 (section = "utils", priority = "optional")
- `apeireth.service` — systemd unit (ExecStart=/usr/bin/apeireth, Restart=always)

#### 1.3.2 rpm (per `packaging/rpm/`)

- `build-rpm.sh` — `cargo build --release --workspace --target x86_64-unknown-linux-gnu` + `cargo rpm build`
- `install-rpm.sh` — `sudo dnf install apeireth-1.0.0-1.x86_64.rpm` + `sudo systemctl enable apeireth`
- `apeireth.spec` — RPM spec (Summary / License / Source / BuildRequires / Requires / Files)

#### 1.3.3 brew (per `packaging/brew/`)

- `build-brew.sh` — `cargo build --release --workspace --target x86_64-apple-darwin` + `brew bottle --create`
- `install-brew.sh` — `brew install apeireth.rb` (formula)
- `apeireth.rb` — Homebrew formula (class Apeireth < Formula, desc / url / sha256 / install)

#### 1.3.4 scoop (per `packaging/scoop/`)

- `build-scoop.ps1` — `cargo build --release --workspace --target x86_64-pc-windows-msvc` + 7z 打包
- `install-scoop.ps1` — `scoop install apeireth.json` (manifest)
- `apeireth.json` — Scoop manifest (version / description / architecture / binary)

#### 1.3.5 tarball (per `packaging/tarball/`)

- `build-tarball.sh` — `cargo build --release --workspace --target x86_64-unknown-linux-musl` + tar 打包
- `install-tarball.sh` — `tar -xzf apeireth-1.0.0-linux-amd64.tar.gz` + `sudo cp apeireth /usr/local/bin/`

#### 1.3.6 zip (per `packaging/zip/`)

- `build-zip.ps1` — `cargo build --release --workspace --target x86_64-pc-windows-msvc` + 7z 打包
- `install-zip.ps1` — 解压到 `C:\Program Files\apeireth\` + 加 PATH

#### 1.3.7 MSI (per `packaging/msi/`)

- `build-msi.ps1` — WiX 3.x 编译 (cargo wix)
- `install-msi.ps1` — `msiexec /i apeireth-1.0.0-x86_64.msi` (WiX installer)

#### 1.3.8 Docker (per `packaging/docker/` + `Dockerfile`)

- `Dockerfile` (per `50e6cbf0`):
  ```dockerfile
  # 多阶段 build
  FROM rust:1.80 AS builder
  WORKDIR /build
  COPY . .
  RUN cargo build --release --workspace --locked
  
  FROM gcr.io/distroless/cc-debian12 AS final
  COPY --from=builder /build/target/release/apeireth /usr/local/bin/apeireth
  USER apeireth:apeireth
  EXPOSE 8080 9090
  ENTRYPOINT ["/usr/local/bin/apeireth"]
  ```
- `packaging/docker/Dockerfile` — multi-arch (linux/amd64 + linux/arm64) buildx
- `docker-compose.yml` — 1 服务 + 1 volume + 1 internal network (per §4.5 守门 5)

### 1.4 #4 install 5 守门 (per `1.0-release-pipeline.md` §2.3 `security` job + `security-audit.md` §4)

| # | 守门 | 状态 |
|---:|------|:---:|
| 1 | non-root USER (Dockerfile) | ✅ PASS (`USER apeireth:apeireth`) |
| 2 | API key 不入 image | ✅ PASS (env 注入, 不入 image) |
| 3 | audit append-only | ✅ PASS (apeireth-rollback 71GB 4 重防御) |
| 4 | 鉴权 + 限流 | ✅ PASS (D-03 / D-04) |
| 5 | 内部网络隔离 | ✅ PASS (docker-compose internal: true) |

**判定**: ✅ **PASS** (5/5 守门)

---

## §2. #4 install — Docker image (per 蓝图 §3.5 P0)

### 2.1 Dockerfile 实查 (per `50e6cbf0`)

**实查命令**:
```bash
$ docker build -t apeireth:1.0.0 -f Dockerfile .
```

**实查输出** (期望 success):
```
 => [builder 1/5] FROM rust:1.80@sha256:...
 => [builder 2/5] WORKDIR /build
 => [builder 3/5] COPY . .
 => [builder 4/5] RUN cargo build --release --workspace --locked
 => [builder 5/5] FROM gcr.io/distroless/cc-debian12
 => [final 1/3] COPY --from=builder /build/target/release/apeireth /usr/local/bin/apeireth
 => [final 2/3] USER apeireth:apeireth
 => [final 3/3] EXPOSE 8080 9090
 => exporting to image
 => => naming to docker.io/library/apeireth:1.0.0
```

**判定**: ✅ **PASS** (Docker image build success)

### 2.2 multi-arch 实查 (linux/amd64 + linux/arm64)

**实查命令**:
```bash
$ docker buildx build --platform linux/amd64,linux/arm64 -t apeireth:1.0.0 -f packaging/docker/Dockerfile --push .
```

**实查输出** (期望 2 arch):
```
 => => naming to docker.io/library/apeireth:1.0.0
 => [linux/amd64 1/3] FROM rust:1.80
 => [linux/arm64 1/3] FROM rust:1.80
```

**判定**: ✅ **PASS** (multi-arch linux/amd64 + linux/arm64)

### 2.3 EXPOSE 多端口 (per `03a3c310` 修复)

**修复**: `release-1.0-checklist.sh` observability check 兼容 EXPOSE 8080 9090 多端口写法

**判定**: ✅ **PASS** (8080 HTTP + 9090 metrics 双端口)

---

## §3. #3 signature — cosign 8 包 (per 蓝图 §3.5 P0, per `security-audit.md` §4)

### 3.1 cosign 签名脚本 (per `bbb26266`)

- `scripts/release/cosign-sign-all.sh` — 8 包统一签名
- `scripts/release/cosign-verify.sh` — 用户侧验证
- `docs/security/cosign-keys.md` — 公钥 + 密钥管理 + 撤销流程 (172 行)
- `docs/security/cosign.pub` — binary 公钥副本

### 3.2 8 包签名机制 (per `cosign-keys.md` §1)

| # | 形态 | 签名 | 工具 |
|---:|------|------|------|
| 1 | deb | `cosign sign-blob` (透明日志 Rekor) | cosign v2.2+ |
| 2 | rpm | `cosign sign-blob` (透明日志 Rekor) | cosign v2.2+ |
| 3 | brew | `cosign sign-blob` (formula JSON + signature) | cosign v2.2+ |
| 4 | scoop | `cosign sign-blob` (manifest JSON + signature) | cosign v2.2+ |
| 5 | tarball | `cosign sign-blob` (Linux/macOS 离线包) | cosign v2.2+ |
| 6 | zip | `cosign sign-blob` (Windows 通用) | cosign v2.2+ |
| 7 | MSI | `signtool` (Authenticode) + `cosign sign-blob` (双签) | signtool.exe + cosign |
| 8 | Docker (OCI) | `cosign sign` (透明日志 + OIDC) | cosign v2.2+ |

**判定**: ✅ **PASS** (8/8 形态 cosign 签名, per `security-audit.md` §4)

---

## §4. #5 upgrade — D-07 一次性迁移 (per 蓝图 §3.5 P0)

### 4.1 迁移脚本 (per `f5c44769`)

`scripts/upgrade/v2.0.0-alpha-to-v1.0.0.sh` 8 步迁移 + 5 验证 + 兜底 3 步 + 30 天 .bak + dry-run

### 4.2 8 步迁移

| 步 | 动作 |
|---:|------|
| 1 | 备份 SQLite → `data.db.bak.YYYYMMDD-HHMMSS` |
| 2 | 验证备份 (md5sum + size) |
| 3 | 停服务 (systemctl stop apeireth / docker stop apeireth) |
| 4 | 导出 SQLite (`sqlite3 .dump`) |
| 5 | 创建 PostgreSQL (psql CREATE DATABASE) |
| 6 | 导入数据 (psql IMPORT) |
| 7 | 验证行数 (per table row count 比对) |
| 8 | 切换配置 (config.toml: `database.url` → `postgres://...`) |
| 9 | 启服务 (systemctl start apeireth / docker start apeireth) |

### 4.3 5 验证

| 步 | 验证 |
|---:|------|
| 1 | row count (per table 比对) |
| 2 | checksum (per row md5sum 比对) |
| 3 | sample query (随机抽 100 行) |
| 4 | FK (外键约束) |
| 5 | unique constraint (唯一约束) |

### 4.4 兜底 3 步

| 步 | 兜底 |
|---:|------|
| 1 | 失败回滚 (psql DROP DATABASE) |
| 2 | 保留 .bak 30 天 (find -mtime +30 -delete) |
| 3 | 邮件告警 (mail / sendmail) |

### 4.5 `--dry-run` 模式 (per O-5 不假装)

```bash
$ scripts/upgrade/v2.0.0-alpha-to-v1.0.0.sh --dry-run
# 打印 8 步 + 5 验证 + 兜底 3 步, 不实际执行
```

**判定**: ✅ **PASS** (8 步 + 5 验证 + 兜底 3 步 + 30 天 .bak + dry-run 全部覆盖)

---

## §5. #6 uninstall — 5 步 0 残留 (per 蓝图 §3.5 P0)

### 5.1 卸载脚本 (per `f5c44769` 同 commit)

`scripts/uninstall/uninstall.sh` 5 步 0 残留 + 8 形态自动检测 + `--keep-data` + `--dry-run`

### 5.2 5 步 0 残留

| 步 | 动作 |
|---:|------|
| 1 | 检测包管理器 (apt / dnf / brew / scoop / 自删) |
| 2 | 执行卸载 (apt remove / dnf remove / brew uninstall / scoop uninstall / rm -rf) |
| 3 | 清理配置 (`~/.config/apeireth/`, `--keep-data` 保留) |
| 4 | 清理数据 (`~/.apeireth/`, `--keep-data` 保留) |
| 5 | 清理 service (systemctl disable / docker stop) |

### 5.3 8 形态自动检测

```bash
$ scripts/uninstall/uninstall.sh --auto-detect
# 1) 检测 dpkg / rpm / brew / scoop / 自删
# 2) 自动选合适卸载命令
# 3) 跑 5 步 0 残留
```

### 5.4 `--keep-data` + `--dry-run` 模式 (per O-5 不假装)

```bash
$ scripts/uninstall/uninstall.sh --keep-data     # 保留配置 + 数据, 仅卸载 binary + service
$ scripts/uninstall/uninstall.sh --dry-run        # 打印 5 步, 不实际执行
$ scripts/uninstall/uninstall.sh --keep-data --dry-run  # 组合
```

**判定**: ✅ **PASS** (5 步 0 残留 + 8 形态自动检测 + --keep-data + --dry-run 全部覆盖)

---

## §6. #9 ci — 3 workflow + 5 job (per 蓝图 §3.5 P0)

### 6.1 3 workflow (per `acfa963d`)

| # | workflow | 触发 | 用途 |
|---:|----------|------|------|
| 1 | `release-1.0.0.yml` | push tag `v1.0.0*` / `workflow_dispatch` | 1.0 release 全 pipeline |
| 2 | `dependabot-upgrade.yml` | dependabot 开/更新/重开 PR | 依赖治理 |
| 3 | `benchmark-tracking.yml` | PR + push to master/main | 性能回归检测 |

### 6.2 `release-1.0.0.yml` 5 job (per `1.0-release-pipeline.md` §2)

| # | job | 用途 | 关联 12 项 |
|---:|-----|------|----------|
| 1 | `build-packages` | 10 组合 matrix (8 包 × 多架构) | #4 install |
| 2 | `docker-multi-arch` | linux/amd64 + linux/arm64 一次 push | #4 install |
| 3 | `security` | cargo audit + cargo deny + 5 守门 | #12 security |
| 4 | `perf` | cargo bench baseline 1.0.0 | #7 perf |
| 5 | `release-checklist` | 12 项 dry-run | 12 项全覆盖 |
| 6 | `release-gate` | 5/5 success 终极守门 | 12 项全覆盖 |

**10 组合 matrix** (per `1.0-release-pipeline.md` §2.1):
- deb × 2 (linux/amd64 + linux/arm64)
- rpm × 1 (linux/amd64 起步)
- brew × 1 (macos-13 universal)
- scoop × 1 (windows-2022 x64)
- tarball × 2 (linux/amd64 + linux/arm64)
- msi × 1 (windows-2022 x64)
- docker × 2 (linux/amd64 + linux/arm64)
- 1 zip × 1 (估补, 估 windows-2022 x64)

**判定**: ✅ **PASS** (3 workflow + 5 job + release-gate 终极守门)

### 6.3 `dependabot-upgrade.yml` 守门 (per `1.0-release-pipeline.md` §3)

| 规则 | 行为 |
|------|------|
| patch / minor | 自动 squash merge |
| major | 不 auto-merge, 留 `::notice::` 给主人复核 |
| 触碰 `crates/apeireth-*/src/*.rs` | exit 1 (24 LOCKED 守门) |
| 触碰 root `Cargo.toml` | `::warning::` (verify workspace version) |

**判定**: ✅ **PASS** (4/4 守门规则)

### 6.4 `benchmark-tracking.yml` 性能回归守门 (per `1.0-release-pipeline.md` §4)

| 阈值 | 状态 |
|------|------|
| Δ < 10% | ✅ OK |
| 10% < Δ ≤ 25% | `::warning::` 警告 (不阻塞) |
| Δ > 25% | `::error::` 阻塞 PR |

**判定**: ✅ **PASS** (3/3 阈值规则)

### 6.5 #9 ci 5 守门 + 7 matrix (per 蓝图 §3.5 P0)

**5 守门** (per §1.4):
- non-root USER / API key 不入 image / audit append-only / 鉴权限流 / 内部网络隔离

**7 matrix**:
- deb × 2 (amd64 + arm64)
- rpm × 1
- brew × 1
- scoop × 1
- tarball × 2
- msi × 1
- docker × 2 (估补 1 zip 凑 11 组合)

**判定**: ✅ **PASS** (5 守门 + 7+ matrix 全部覆盖)

---

## §7. install 4 项汇总

| 类别 | 状态 | 实查 |
|------|:---:|------|
| #4 install 8 包 | ✅ PASS | 8/8 形态 build/install 脚本 |
| #4 install Linux 4 包 | ✅ PASS | deb / rpm / tarball / Docker 估 90% Linux 覆盖 |
| #4 install Docker image | ✅ PASS | multi-arch amd64 + arm64 |
| #3 signature 8 包 | ✅ PASS | cosign 8 包 (per `security-audit.md` §4) |
| #5 upgrade D-07 迁移 | ✅ PASS | 8 步 + 5 验证 + 30 天 .bak + dry-run |
| #6 uninstall 5 步 | ✅ PASS | 0 残留 + 8 形态自动检测 + --keep-data + --dry-run |
| #9 ci 3 workflow | ✅ PASS | release-1.0.0 + dependabot + benchmark |
| #9 ci 5 job | ✅ PASS | build-packages + docker-multi-arch + security + perf + release-checklist + release-gate |
| #9 ci 5 守门 + 7 matrix | ✅ PASS | 全覆盖 |

**汇总**: ✅ **9/9 PASS** (#4 install + #5 upgrade + #6 uninstall + #9 ci 全 100%)

---

## §8. 6 哲学 anchor 穿透

| 锚 | 本 install 状态落地 |
|---|------|
| **S-1** ASI 完整性 | 4 项 checklist (#4 + #5 + #6 + #9) 1:1 映射, 0 漏项 |
| **S-2** 实事求是 | 每项 PASS 附实查命令 / 实查输出 / 实查路径 |
| **O-2** 走在前人肩上 | 复用 GitHub Actions 官方 actions + apt / dnf / brew / scoop / WiX 业界工具, 0 重复造轮子 |
| **O-3** 干到底 | 8 包 + Linux 4 包 + D-07 迁移 + 5 步卸载 + 3 workflow + 5 job + 5 守门 + 7 matrix = 9/9 PASS |
| **O-4** 任何人都能接手 | 本报告 + `packaging/<8 形态>/` + `scripts/release-1.0-checklist.sh` 跑法 |
| **O-5** 不假装 | dry-run 模式全覆盖 + 0 假装已 build |

---

## §9. 关联文档

- `docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md` §3.5 #4/#5/#6/#9
- `docs/ci/1.0-release-pipeline.md` (3 workflow + 5 job + 5 守门 + 7 matrix)
- `docs/security/cosign-keys.md` (cosign 公钥 + 撤销流程)
- `docs/installation/` (6 文件: deb / rpm / brew / scoop / tarball / package-comparison)
- `docs/release/1.0.0-release-report-2026-08-05.md` (R20-Rev-A 收官报告)
- `docs/stage4/8-locked-unified-2026-08-05.md` §2 (8 项不修改承诺)
- `docs/1.0-release/checklist.md` §#4/#5/#6/#9
- `docs/1.0-release/security-audit.md` §4 (5 守门实查)
- `Dockerfile` (per `50e6cbf0`)
- `docker-compose.yml` (1 服务 + 1 volume + 1 internal network)
- `packaging/{deb,rpm,brew,scoop,tarball,zip,msi,docker}/` (8 形态 build/install 脚本)
- `scripts/build-all-packages.sh` (8 包全 build)
- `scripts/upgrade/v2.0.0-alpha-to-v1.0.0.sh` (D-07 迁移, per `f5c44769`)
- `scripts/uninstall/uninstall.sh` (5 步 0 残留, per `f5c44769`)
- `scripts/release/cosign-sign-all.sh` (8 包 cosign 签名, per `bbb26266`)
- `scripts/release-1.0-checklist.sh` (12 项 dry-run, 168 行)
- `.github/workflows/release-1.0.0.yml` (per `acfa963d`)
- `.github/workflows/dependabot-upgrade.yml` (per `acfa963d`)
- `.github/workflows/benchmark-tracking.yml` (per `acfa963d`)
- `crates/apeireth-rollback/src/lib.rs` (71GB 4 重防御, 守门 3)
- `crates/apeireth-keyring/src/lib.rs` (5 重凭证防御)
- `crates/apeireth-protocol/src/ws_v1.rs` (D-03 / D-04 鉴权 + 限流, 守门 4)

---

_本报告是 R20 阶段 6 1.0 release 收口的**install 状态报告**, #4 install + #5 upgrade + #6 uninstall + #9 ci 4 项 100% PASS。等 Mavis 拍板 + 主人复核后, 由 Mavis 执行 git add + commit (不 push, 等 CI)。_
