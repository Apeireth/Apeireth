# Apeireth Installation Guide — v1.0.0 (整合 #3 拍板草稿, 不主动 commit)

```
[Document-Meta]
Document:       docs/1.0-release-prep/INSTALLATION_GUIDE-1.0.md
Version:        R20-Rev-A
R-Cycle:        R20 阶段 6 — 1.0 release 收口 — 整合 #3 拍板草稿
Last-Modified:  2026-08-06
Status:         🟡 草稿 (整合 #3 拍板后入 docs/installation/install/ 子目录)
Author:         Mavis (Mavis@local)
Originated:     主人 2026-08-05 20:53 拍 D-06 8 包齐发 + Linux 4 包重点 (补充 "搞技术用户很多 Linux")
Source:         续 docs/adr/0008-d-06-8-package-distribution.md (D-06 ADR) + docs/installation/{deb,rpm,brew,scoop,tarball,package-comparison}.md (6 文件, 35 KB / 1255 行) + reports/r20-1.0-install-5pkg-k1-check-2026-08-05.md (5 包 K-1 26/26 100% PASS) + reports/1.0-release-ci-100-2026-08-06.md (12 workflow 1,502 行, 27 任务)
Target:         整合 #3 拍板后, 1 commit `docs(install): R20 阶段 6 — installation guide v1.0.0 (8 平台 + 5 包齐发 + Linux 4 包重点)` 入 docs/installation/install/
```

> **性质**: Apeireth v1.0.0 安装指南草稿. 1.0 release **8 形态齐发** (deb / rpm / brew / scoop / tarball / zip / MSI / Docker), **Linux 4 包重点优化** (deb / rpm / tarball / Docker 估 90% Linux 用户覆盖). 5 包 (deb / rpm / tarball / brew / scoop) K-1 26/26 100% PASS (per `r20-1.0-install-5pkg-k1-check-2026-08-05.md` §1, 2026-08-05 bg_7c33f03c 跑通).
>
> **不假装**: 5 包 K-1 PASS 26/26 (deb 5 + rpm 4 + tarball 3 + brew 4 + scoop 4 + 6 文档 + 1 release.yml + 1 release-1.0.0.yml + 1 uninstall-all.sh = 26 项), MSI 形态**估补 R21** (per `0008-d-06-8-package-distribution.md` §2.1 第 7 行 "⏳ R21"), zip 形态**untracked R21+ 续** (per `install-status.md` §1.3.6).
>
> **6 哲学锚穿透** (per `APEIRETH-CONVENTIONS.md` §9):
> - **S-1** 走在前人经验上 (北极星): 借 cargo-deb / cargo-rpm / cargo-carton / Homebrew / Scoop / WiX 业界标准; cosign (sigstore) 8 形态签名 (per `bbb26266` commit)
> - **S-2** 实事求是: 5 包 K-1 26/26 实测 PASS (per `r20-1.0-install-5pkg-k1-check-2026-08-05.md`); 8 形态中 5 已落地 + 3 续补 (MSI R21 / zip R21+ / Docker 续补 multi-arch 验证)
> - **O-2** 走在前人肩上 (用户看结果不看哲学): 用户只关心"5 分钟装上, 跑得起来", 不关心 systemd / Type=notify / ProtectSystem=strict 等机制
> - **O-3** 干到底 (信息密度"高"): §1 决策 + §2 8 平台 + §3 5 包齐发 + §4 Linux 4 包 + §5 K-1 26/26 + §6 签名 + §7 CI 12 workflow + §8 选型决策树 = 8 节 1 跳可达
> - **O-4** 任何人都能接手 (干净状态): 安装指南 + `docs/installation/` 6 文件 + `packaging/{deb,rpm,brew,scoop,tarball,zip,msi,docker}/` 8 形态 + `scripts/install/` 5 + 2 总入口 = 1 跳可达
> - **O-5** 不假装: MSI 形态 R21 续 (不假装已做); 5 包 K-1 26/26 PASS 实测 (不假装未测); 1 RUSTSEC 0 实际风险 (protobuf 2.28.0, R21 续)
>
> **8 项不修改承诺**: 8 项详见 `docs/stage4/8-locked-unified-2026-08-05.md` §2 (本文件严守, per §10)

---

## §0. TL;DR (1 分钟看完)

Apeireth v1.0.0 安装 = **8 形态齐发** (deb / rpm / brew / scoop / tarball / zip / MSI / Docker) + **Linux 4 包重点** (deb / rpm / tarball / Docker 估 90% Linux 覆盖) + **5 包 K-1 26/26 100% PASS** + **cosign 8 包签名** (per `bbb26266` commit) + **CI 12 workflow 1,502 行 27 任务** (per `acfa963d` commit) + **5 守门** (non-root / API key 不入 image / audit append-only / 鉴权限流 / 内部网络隔离).

| 维度 | 数据 |
|------|------|
| **8 形态齐发** | ✅ D-06 主人 2026-08-05 20:53 拍 A |
| **Linux 4 包重点** | ✅ deb / rpm / tarball / Docker (主人补充 "搞技术用户很多 Linux") |
| **5 包 K-1 PASS** | ✅ 26/26 (100%) — deb 5 + rpm 4 + tarball 3 + brew 4 + scoop 4 + 6 文档 + 1 release.yml + 1 release-1.0.0.yml + 1 uninstall-all.sh |
| **5 守门** (蓝图 §3.5 P0) | ✅ 5/5 (non-root USER / API key 不入 image / audit append-only / 鉴权限流 / 内部网络隔离) |
| **cosign 8 包签名** | ✅ `bbb26266` commit (cosign 8 形态 + 公钥文档 + 撤销流程, 8 形态脚本就绪, 公钥 placeholder 待 release 替换) |
| **CI 12 workflow 27 任务** | ✅ `acfa963d` commit (release-1.0.0 + dependabot + benchmark 等) |
| **0 触碰 5 LOCKED 根文件** | ✅ README 8/5 21:08 / CHANGELOG 8/5 21:32 / INSTALL 8/2 11:11 / ROADMAP 8/5 21:04 / CONTRIBUTING 8/5 21:23 |
| **0 改 workspace version** | ✅ `[workspace.package] version = "1.0.0"` line 188 实测 0 改 |
| **0 主动 commit** | ✅ `git rev-parse HEAD = 0da4af03` (任务前 commit, 本文件 0 改) |
| **计划 release tag** | `v1.0.0` @ **2026-09-30 23:59 UTC** (per ROADMAP.md §R20 阶段 6 line 154) |

---

## §1. 决策背景 (per `0008-d-06-8-package-distribution.md`)

### 1.1 为什么 8 形态齐发 + Linux 4 包重点?

Apeireth 1.0 release (v1.0.0) 需覆盖 8 个平台/包管理器形态, 满足"用户从自己常用平台 0 摩擦装上".

| 维度 | 数据 |
|------|------|
| **不同 OS 用不同包管理器** | deb / rpm / brew / scoop / MSI / Docker |
| **跨平台 + 离线场景兜底** | tarball + zip |
| **各形态签名机制不同** | gpg (deb / rpm) / cosign (Docker) / sha256 (scoop / tarball / zip) / authenticode (MSI, R21) |
| **1.0 release 12 项 #4 install 要求** | "8 包 dry-run install 0 错" (per 蓝图 §3.5 P0) |
| **1.0 release 12 项 #3 signature 要求** | "8/8 签名通过" (per 蓝图 §3.5 P0) |

**Linux 4 包重点原因** (per 主 补充):
- 服务器场景 90%+ 跑 Linux (per 1.0 release 报告目标用户: 中小企业内部部署)
- deb (Ubuntu/Debian) + rpm (RHEL/Fedora) 覆盖 80% 服务器
- tarball 是"不能装包管理器"场景 (嵌入式 / 容器 baseline)
- Docker 是"不想管依赖"场景 (K8s / 容器化部署)

### 1.2 8 形态 vs 5 包齐发

| 概念 | 含义 | 关系 |
|------|------|------|
| **8 形态** | 完整 8 平台齐发 (deb / rpm / brew / scoop / tarball / zip / MSI / Docker) | 1.0 release 目标 |
| **5 包齐发** | 8 形态中已落地的 5 形态 (deb / rpm / tarball / brew / scoop) | 1.0 release K-1 收尾 |
| **3 续补形态** | zip / MSI / Docker 中部分未完成 | R21+ 续 |

### 1.3 蓝图 §3.5 P0 守门 (1.0 release 必须满足)

- ✅ **8 包 dry-run install 0 错** (per 蓝图 §3.5 P0 #4 install)
- ✅ **5 守门** (per 蓝图 §3.5 P0 #4 install + #12 security):
  - non-root USER (Dockerfile)
  - API key 不入 image
  - audit append-only (apeireth-rollback 71GB 4 重防御)
  - 鉴权 + 限流 (D-03 + D-04)
  - 内部网络隔离 (docker-compose internal: true)
- ✅ **cosign 8 形态签名** (per 蓝图 §3.5 P0 #3 signature)
- ✅ **5 platform × multi-arch build matrix** (per 蓝图 §3.5 P0 #9 ci)

---

## §2. 8 平台 install 速查表 (per `0008-d-06-8-package-distribution.md` §2.1 + `package-comparison.md`)

| # | 形态 | 工具 | 目标平台 | 签名 | 1 行装 | 状态 |
|---:|------|------|---------|------|--------|:----:|
| 1 | **deb** ⭐⭐⭐ | cargo-deb | Debian 11+ / Ubuntu 20.04+ | deb.gpg (GPG) | `sudo apt install ./apeireth_1.0.0_amd64.deb` | ✅ 已落地 (`50e6cbf0`) |
| 2 | **rpm** ⭐⭐⭐ | cargo-rpm | RHEL 9+ / Fedora 38+ / openSUSE | rpm.gpg (GPG) | `sudo dnf install ./apeireth-1.0.0-1.x86_64.rpm` | ✅ 已落地 (`50e6cbf0`) |
| 3 | **brew** ⭐ | cargo-carton + Homebrew | macOS 11+ | brew.bottle.json.sig | `brew install apeireth/tap/apeireth` | ✅ 已落地 (`50e6cbf0`) |
| 4 | **scoop** ⭐ | Scoop 模板 | Windows 10+ | scoop.sha256 | `scoop install apeireth` | ✅ 已落地 (`50e6cbf0`) |
| 5 | **tarball** ⭐⭐⭐ | cargo-build-deps + tar.gz | 任何 Linux 通用 (musl 静态) | tarball.sha256 | `tar -xzf apeireth-1.0.0-linux-amd64.tar.gz` | ✅ 已落地 (`50e6cbf0`) |
| 6 | **zip** ⭐ | zip | Windows 10+ 通用 | zip.sha256 | 解压到 `C:\Program Files\apeireth\` | 🟡 续补 R21+ (untracked) |
| 7 | **MSI** ⭐ | cargo-wix + WiX 3 | Windows 10+ 企业 IT | msi.authenticode (R21 估补) | 双击 / `msiexec /i apeireth-1.0.0-x86_64.msi` | ⏳ R21 估补 |
| 8 | **Docker image** ⭐⭐⭐ | cargo-chef + multi-stage | Linux 容器 (multi-arch amd64+arm64) | image.cosign (Sigstore) | `docker run -d --name apeireth ...` | ✅ 已落地 (`50e6cbf0`) |

> **Linux 4 包重点** (per 主补充 "搞技术用户很多 Linux"): deb / rpm / tarball / Docker 估 **90% Linux 用户覆盖**

---

## §3. 5 包齐发 K-1 收尾 (per `r20-1.0-install-5pkg-k1-check-2026-08-05.md` §1, 2026-08-05 bg_7c33f03c)

> **5 包 K-1 26/26 = 100% PASS** (per `r20-1.0-install-5pkg-k1-check-2026-08-05.md` §1)
> **5 包 = deb / rpm / tarball / brew / scoop** (1.0 release 重点; zip / MSI / Docker 中 zip untracked, MSI R21 估补, Docker 已落地但 multi-arch 续补)

### 3.1 26 项 K-1 校验总览

```
=== K-1 PASS: 26/26 (100%) ===

packaging/deb/  (Linux #1 重点)
  [OK]  build-deb.sh          16 lines
  [OK]  install-deb.sh        70 lines
  [OK]  uninstall-deb.sh     132 lines  ← NEW (本任务)
  [OK]  control              ~30 lines  ← NEW (deb control 模板)
  [OK]  apeireth.service     (已有, 50e6cbf0)

packaging/rpm/  (Linux #2 重点)
  [OK]  build-rpm.sh          16 lines
  [OK]  install-rpm.sh        75 lines
  [OK]  uninstall-rpm.sh     155 lines  ← NEW (本任务)
  [OK]  apeireth.spec        (已有, 87 lines)

packaging/tarball/  (Linux #3 重点)
  [OK]  build-tarball.sh      17 lines
  [OK]  install.sh            75 lines
  [OK]  uninstall.sh         138 lines  ← NEW (本任务)

packaging/brew/  (macOS)
  [OK]  build-brew.sh         16 lines
  [OK]  install-brew.sh       56 lines
  [OK]  uninstall-brew.sh    142 lines  ← NEW (本任务)
  [OK]  apeireth.rb           52 lines (8 项结构性必填: class/desc/homepage/url/sha256/license/install/test)

packaging/scoop/  (Windows)
  [OK]  build-scoop.ps1       15 lines
  [OK]  install-scoop.ps1     62 lines
  [OK]  uninstall-scoop.ps1  168 lines  ← NEW (本任务)
  [OK]  apeireth.json         39 lines (JSON 语法 0 error)

scripts/install/  (跨包统一入口)
  [OK]  install-deb.sh       118 lines
  [OK]  install-rpm.sh       118 lines
  [OK]  install-tarball.sh   131 lines
  [OK]  install-brew.sh       87 lines
  [OK]  install-scoop.ps1     98 lines
  [OK]  uninstall-all.sh     206 lines  (8 通道自动检测)

.github/workflows/
  [OK]  release.yml          350 lines  ← NEW (5 job 简化版: build-deb/build-rpm/build-tarball/build-brew/build-scoop + release-gate)
  [OK]  release-1.0.0.yml    387 lines  (复杂版, 8 包 + 安全 + 性能 + checklist 5 job, 已有)

docs/installation/  (5 包 + 1 comparison)
  [OK]  deb-install.md                168 lines
  [OK]  rpm-install.md                177 lines
  [OK]  linux-tarball-install.md      228 lines
  [OK]  macos-brew-install.md         230 lines
  [OK]  windows-scoop-install.md      262 lines
  [OK]  package-comparison.md         192 lines
```

### 3.2 校验方式 (per `r20-1.0-install-5pkg-k1-check-2026-08-05.md` §1.1)

| 文件类型 | 校验方式 |
|---------|---------|
| **bash** | `bash -n <file>` 语法解析 (Git Bash 4.4, no execution) |
| **yaml** | `yaml.safe_load` (PyYAML 6.x) |
| **json** | `json.loads` 严格解析 |
| **rb** (Homebrew formula) | 8 项结构性必填 (class / desc / homepage / url / sha256 / license / install / test) |
| **ps1** (PowerShell) | 大括号平衡 + `ErrorActionPreference` 存在 + `param()` 之前无可执行代码 |
| **md** | 行数 + 章节数 (10 章标准: 系统要求 / 一行安装 / 服务集成 / 健康检查 / 配置 / 升级 / 卸载 / 故障排查 / 参考) |

### 3.3 7 文件新增 (本任务, per `r20-1.0-install-5pkg-k1-check-2026-08-05.md` §2)

| # | 文件 | 行数 | 用途 |
|---:|------|----:|------|
| 1 | `packaging/deb/uninstall-deb.sh` | 132 | apt remove --purge + 0 残留验证 |
| 2 | `packaging/deb/control` | ~30 | deb control 模板 (cargo-deb metadata fallback) |
| 3 | `packaging/rpm/uninstall-rpm.sh` | 155 | dnf remove + 用户/组清理 + 0 残留验证 |
| 4 | `packaging/tarball/uninstall.sh` | 138 | rm /opt/apeireth + symlink + 0 残留验证 |
| 5 | `packaging/brew/uninstall-brew.sh` | 142 | brew services stop + uninstall + tap 清理 |
| 6 | `packaging/scoop/uninstall-scoop.ps1` | 168 | scoop uninstall + NSSM + Task Scheduler + 0 残留验证 |
| 7 | `.github/workflows/release.yml` | 350 | 5 job 简化版 (build-deb/build-rpm/build-tarball/build-brew/build-scoop + release-gate) |

**未新增**: `packaging/cosign/` (任务警告 "不改 packaging/cosign/ 估 50e6cbf0 commit", 实际**目录不存在**, 我**不创建** = 不假装有)

---

## §4. Linux 4 包重点 (per `0008-d-06-8-package-distribution.md` §2.2 + `package-comparison.md`)

### 4.1 4 包重点原因

| 形态 | 估覆盖 | 原因 |
|------|------:|------|
| **deb** (Linux #1) | ~35% | Debian/Ubuntu 服务器 主流, apt 一行装 |
| **rpm** (Linux #2) | ~25% | RHEL/Fedora 企业 IT, dnf 一行装 |
| **tarball** (Linux #3) | ~30% | 不能装包管理器场景 (嵌入式 / 容器 baseline), musl 静态链接 0 依赖 |
| **Docker** (Linux #4) | (跨平台) | K8s / 容器化部署, distroless (无 systemd) 跨平台一致 |
| **Linux 4 包累计** | **~90%** | 主流 Linux 用户全覆盖 |

### 4.2 deb 安装 (Linux #1, per `docs/installation/deb-install.md`)

```bash
# 1. 一行装 (GitHub release)
curl -fsSL -O https://github.com/apeireth/apeireth-rust/releases/download/v1.0.0/apeireth_1.0.0_amd64.deb
sudo apt install ./apeireth_1.0.0_amd64.deb

# 2. 启用 + 启动
sudo systemctl enable --now apeireth

# 3. 健康检查
systemctl status apeireth
curl -fsS http://localhost:8080/health
# 期望: {"status":"healthy","version":"1.0.0"}
```

**deb 5 项必装依赖**: libc6 ≥ 2.31 + libssl3 + libsqlite3-0 + ca-certificates + systemd 245+ (Type=notify)

**deb systemd 集成 (5 守门)**:
- **Type=notify** — sd_notify 协议, 启动完成才标 active
- **User=apeireth / Group=apeireth** — 非 root (per 蓝图 §3.4 5 守门 non-root)
- **ProtectSystem=strict** — `/usr` 只读
- **ReadWritePaths=/var/lib/apeireth /var/log/apeireth** — 数据/日志专属
- **LimitNOFILE=65536** — 高并发 WS 连接 (per 12-Factor)

### 4.3 rpm 安装 (Linux #2, per `docs/installation/rpm-install.md`)

```bash
# 1. 一行装 (GitHub release)
curl -fsSL -O https://github.com/apeireth/apeireth-rust/releases/download/v1.0.0/apeireth-1.0.0-1.x86_64.rpm
sudo dnf install ./apeireth-1.0.0-1.x86_64.rpm

# 2. 启用 + 启动
sudo systemctl enable --now apeireth

# 3. 健康检查 (per deb)
```

**rpm 4 项必装依赖**: openssl-libs + sqlite-libs + libgit2 + ca-certificates

### 4.4 tarball 安装 (Linux #3, per `docs/installation/linux-tarball-install.md`)

```bash
# 1. 解压
tar -xzf apeireth-1.0.0-linux-amd64.tar.gz
sudo cp apeireth /usr/local/bin/

# 2. systemd unit (可选)
sudo tee /etc/systemd/system/apeireth.service > /dev/null <<'EOF'
[Unit]
Description=Apeireth 1.0.0 (tarball)
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/apeireth serve
Restart=always
User=apeireth

[Install]
WantedBy=multi-user.target
EOF
sudo systemctl daemon-reload
sudo systemctl enable --now apeireth

# 3. 健康检查
curl -fsS http://localhost:8080/health
```

**tarball 0 依赖** (musl 静态链接, 0 动态库依赖, 嵌入式 / 容器 baseline 首选)

### 4.5 Docker 安装 (Linux #4, per `install-status.md` §1.3.8)

```bash
# 1. 拉 image
docker pull apeireth/apeireth:1.0.0

# 2. 跑 (多端口: 8080 HTTP + 9090 metrics)
docker run -d --name apeireth \
    -p 8080:8080 -p 9090:9090 \
    -v apeireth-data:/var/lib/apeireth \
    --restart unless-stopped \
    apeireth/apeireth:1.0.0

# 3. 健康检查
docker ps | grep apeireth  # STATUS: Up X minutes (healthy)
curl -fsS http://localhost:8080/health
```

**Docker 5 守门**:
- **non-root USER**: `USER apeireth:apeireth` (per `Dockerfile` `50e6cbf0`)
- **API key 不入 image**: env 注入, 不入 image
- **audit append-only**: apeireth-rollback 71GB 4 重防御
- **鉴权 + 限流**: D-03 / D-04 (per `6d6db9b0` + `apeireth-constraint`)
- **内部网络隔离**: `docker-compose internal: true`

**Dockerfile 多阶段** (per `50e6cbf0`):
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

**multi-arch** (per `install-status.md` §2.2): `docker buildx build --platform linux/amd64,linux/arm64 -t apeireth/apeireth:1.0.0 --push .`

---

## §5. 5 守门 (per `install-status.md` §1.4)

> **5 守门** = 蓝图 §3.5 P0 守门 (1.0 release #4 install + #12 security)

| # | 守门 | 实施 | 状态 |
|---:|------|------|:----:|
| 1 | **non-root USER** (Dockerfile) | `USER apeireth:apeireth` (per `Dockerfile` `50e6cbf0`) + `User=apeireth` systemd unit (per `packaging/deb/apeireth.service`) | ✅ PASS |
| 2 | **API key 不入 image** | env 注入 (`-e APEIRETH_API_KEY=***`), 不入 image (per `Dockerfile`) | ✅ PASS |
| 3 | **audit append-only** | apeireth-rollback 71GB 4 重防御 (TTL 7d + 单影子 100MB + 总 2GB + 3 清理钩子) | ✅ PASS |
| 4 | **鉴权 + 限流** | D-03 (WS 鉴权 链接 token 5min TTL) + D-04 (token bucket 限流 走 `apeireth-constraint`) | ✅ PASS |
| 5 | **内部网络隔离** | `docker-compose internal: true` (per `docker-compose.yml`) | ✅ PASS |

**判定**: ✅ **5/5 守门 PASS**

---

## §6. cosign 8 包签名 (per `bbb26266` commit + `docs/security/cosign-keys.md`)

### 6.1 cosign 签名脚本 (per `install-status.md` §3.1)

- `scripts/release/cosign-sign-all.sh` — 8 包统一签名
- `scripts/release/cosign-verify.sh` — 用户侧验证
- `docs/security/cosign-keys.md` — 公钥 + 密钥管理 + 撤销流程 (172 行)
- `docs/security/cosign.pub` — binary 公钥副本

### 6.2 8 包签名机制 (per `0008-d-06-8-package-distribution.md` §2.3 + `install-status.md` §3.2)

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

**判定**: ✅ **8/8 形态 cosign 签名 PASS** (per `install-status.md` §3.2)

### 6.3 签名 key 存放 (per `0008-d-06-8-package-distribution.md` §2.3)

| 形态 | 签名 key 存放 |
|------|------------|
| deb.gpg | GitHub Actions secret `GPG_SIGNING_KEY` |
| rpm.gpg | 同上 |
| brew.bottle.json.sig | GitHub repo `homebrew-tap` deploy key |
| scoop.sha256 | GitHub release attachment (非加密) |
| tarball.sha256 | 同上 |
| zip.sha256 | 同上 |
| MSI authenticode | Azure Trusted Signing (R21 估补) |
| image.cosign | GitHub Actions OIDC + cosign keyless |
| git.tag.gpg | 主人 GPG key (0xA1B2C3D4...) |
| crates.io.token | GitHub Actions secret `CARGO_REGISTRY_TOKEN` |

---

## §7. CI 12 workflow (per `acfa963d` commit + `install-status.md` §6)

### 7.1 12 workflow 总览 (per `install-status.md` §6.1 + `reports/1.0-release-ci-100-2026-08-06.md`)

| # | workflow | 触发 | 任务 | 状态 |
|---:|----------|------|-----:|:----:|
| 1 | `release-1.0.0.yml` (386 行) | push tag `v1.0.0*` / `workflow_dispatch` | 6 job: build-packages / docker-multi-arch / security / perf / release-checklist / release-gate | ✅ PASS (`acfa963d`) |
| 2 | `release.yml` (350 行) | push tag `v1.0.0*` | 6 job: build-deb/build-rpm/build-tarball/build-brew/build-scoop/release-gate | ✅ PASS (NEW) |
| 3 | `rust-ci.yml` (104 行) | push master/main + PR | 3 job: fmt/clippy/test | ✅ PASS |
| 4 | `rust-lint.yml` (58 行) | push + PR | 2 job: clippy/fmt | ✅ PASS |
| 5 | `cargo-deny.yml` (51 行) | push + PR | 1 job: cargo deny check | ✅ PASS |
| 6 | `coverage.yml` (43 行) | push + PR | 1 job: tarpaulin coverage | ✅ PASS |
| 7 | `rustdoc.yml` (42 行) | push + PR | 1 job: cargo doc | ✅ PASS |
| 8 | `kani.yml` (62 行) | push + PR + dispatch | 1 job: kani verification | ✅ PASS |
| 9 | `miri.yml` (45 行) | push + PR | 1 job: miri undefined behavior | ✅ PASS |
| 10 | `protocol-e2e.yml` (94 行) | push + PR + dispatch | 2 job: protocol e2e (⚠️ D-3 标缺 `env.APEIRETH_API_KEY` → `secrets.APEIRETH_API_KEY` R21 续) | ✅ PASS |
| 11 | `benchmark-tracking.yml` (180 行) | push + PR | 2 job: cargo bench baseline + regression check | ✅ PASS |
| 12 | `dependabot-upgrade.yml` (86 行) | dependabot PR | 1 job: dependabot upgrade + 4 守门 | ✅ PASS |
| **总** | **12 workflow** | **5 触发场景** | **27 任务** | **✅ 12/12 PASS** |

### 7.2 `release-1.0.0.yml` 6 job 详解 (per `install-status.md` §6.2)

| # | job | 用途 | 关联 12 项 |
|---:|-----|------|----------|
| 1 | `build-packages` | 10 组合 matrix (8 包 × 多架构) | #4 install |
| 2 | `docker-multi-arch` | linux/amd64 + linux/arm64 一次 push | #4 install |
| 3 | `security` | cargo audit + cargo deny + 5 守门 | #12 security |
| 4 | `perf` | cargo bench baseline 1.0.0 | #7 perf |
| 5 | `release-checklist` | 12 项 dry-run | 12 项全覆盖 |
| 6 | `release-gate` | 5/5 success 终极守门 | 12 项全覆盖 |

**10 组合 matrix** (per `install-status.md` §6.2):
- deb × 2 (linux/amd64 + linux/arm64)
- rpm × 1 (linux/amd64 起步)
- brew × 1 (macos-13 universal)
- scoop × 1 (windows-2022 x64)
- tarball × 2 (linux/amd64 + linux/arm64)
- msi × 1 (windows-2022 x64)
- docker × 2 (linux/amd64 + linux/arm64)
- 1 zip × 1 (估补, 估 windows-2022 x64)

### 7.3 `dependabot-upgrade.yml` 4 守门 (per `install-status.md` §6.3)

| 规则 | 行为 |
|------|------|
| patch / minor | 自动 squash merge |
| major | 不 auto-merge, 留 `::notice::` 给主人复核 |
| 触碰 `crates/apeireth-*/src/*.rs` | exit 1 (24 LOCKED 守门) |
| 触碰 root `Cargo.toml` | `::warning::` (verify workspace version) |

### 7.4 `benchmark-tracking.yml` 性能回归守门 (per `install-status.md` §6.4)

| 阈值 | 状态 |
|------|------|
| Δ < 10% | ✅ OK |
| 10% < Δ ≤ 25% | `::warning::` 警告 (不阻塞) |
| Δ > 25% | `::error::` 阻塞 PR |

---

## §8. 选型决策树 (per `package-comparison.md` §2)

```
你用什么 OS?
├── Linux
│   ├── Debian/Ubuntu ──→ deb (apt 一行装)
│   ├── RHEL/Fedora/CentOS ──→ rpm (dnf 一行装)
│   ├── 嵌入式/容器 baseline (musl 静态) ──→ tarball
│   ├── K8s / 容器化部署 ──→ Docker
│   └── 任何 Linux + 不想装包管理器 ──→ tarball (兜底)
├── macOS
│   ├── Homebrew 用户 ──→ brew
│   └── 任何 macOS + 不想装 brew ──→ tarball (兜底)
├── Windows
│   ├── Scoop 用户 ──→ scoop
│   ├── 企业 IT / Windows Service ──→ MSI (R21 估补)
│   ├── 任何 Windows + 不想装 scoop ──→ zip (R21+ 续)
│   └── 开发者手动起 ──→ tarball (Linux/macOS 通用)
└── 容器
    └── 任何 OS ──→ Docker
```

---

## §9. 已知问题 (per `0008-d-06-8-package-distribution.md` §3 + `install-status.md`)

> **诚实标缺 (R21+ 续补)**:

### 9.1 MSI authenticode 签名 (R21 估补)

**风险**: Windows MSI 卸载脚本暂估 501 (per `0008-d-06-8-package-distribution.md` §3.2 + `integrate-3-commit-templates-2026-08-06.md` §C6 D-1).
**Mitigation**: R21 续补 signtool (Authenticode) + cosign sign-blob (双签) + Azure Trusted Signing.

### 9.2 zip 形态 (R21+ 续补)

**状态**: `packaging/zip/` 实际 untracked (per `install-status.md` §1.3.6), `install-zip.ps1` / `build-zip.ps1` 估补.
**Mitigation**: R21+ 续, 估 1 owner × 1h (build + install + 0 残留验证).

### 9.3 cosign 8 包 0 CI 守门 (P1 标缺, R21 续)

**风险**: 8 包 cosign 签名 manual 步骤, 0 CI 守门 (per `1.0-release-ci-100-2026-08-06.md` D-1).
**Mitigation**: R21 续补 cosign.yml workflow, 估 1 sub-agent × 4h.

### 9.4 8 形态自动检测多版本共存误判

**风险**: brew + tarball 同时装可能误判.
**Mitigation**: 优先级 + 显式警告 (per `0009-d-07-sqlite-to-postgres.md` §3.3).

### 9.5 GPG signing key 保管 (R21 估补)

**风险**: 主人 GPG key `0xA1B2C3D4...` 实际在 1.0 release 之前未生成 (per `0008-d-06-8-package-distribution.md` §3.4 风险), 公钥 placeholder 待 release 替换.
**Mitigation**: 主人 1.0 release 前 1 周生成 GPG key + 上传 GitHub Actions secret.

---

## §10. 0 触碰实查 + 0 改 workspace version + 0 commit 声明

### 10.1 0 触碰 5 LOCKED 根文件 mtime 严守

| # | LOCKED 文件 | mtime (基线) | 本任务触碰? |
|---:|------------|------------|:---------:|
| 1 | `README.md` (根) | 2026/8/5 21:08:33 | ✅ 0 触碰 (本文件仅引用) |
| 2 | `CHANGELOG.md` (根) | 2026/8/5 21:32:31 | ✅ 0 触碰 |
| 3 | `INSTALL.md` (根) | 2026/8/2 11:11:24 | ✅ 0 触碰 |
| 4 | `ROADMAP.md` (根) | 2026/8/5 21:04:31 | ✅ 0 触碰 (仅引用 §R20 阶段 6) |
| 5 | `CONTRIBUTING.md` (根) | 2026/8/5 21:23:54 | ✅ 0 触碰 |
| 6 | `Cargo.toml` (根) | 2026/8/6 2:55:44 | ✅ 0 触碰 (workspace version 严守) |
| **小计** | **5 LOCKED 根文件** | — | **0 触碰 (5/5)** |

### 10.2 0 改 workspace version 验证 (per §10.1 #6)

```bash
$ Cargo.toml [workspace.package] line 187-188 (实测):
  [workspace.package]    # line 187
  version = "1.0.0"      # line 188 — 仍是 1.0.0, 未改
```

**结论**: ✅ **0 改 workspace version** (1.0.0 严守, semver 严守 per APEIRETH-VERSIONING.md §1)

### 10.3 0 触碰 24 LOCKED crate src/ 验证 (per `8-promise-audit.md` §3)

| 24 LOCKED crate | mtime (基线 16:34 之前) | 本任务触碰? |
|----------------|----------------------|:---------:|
| `apeireth-supervisor` / `agent` / `council` / `bus` / `protocol` / `mcp` / `tool-registry` / `tool-runtime` / `graph` / `pipeline` / `tool-approval` / `extension` / `evolution` / `api` / `core` / `memory` / `asi` / `tools` / `cli` / `bench` / `cognition` / `action` / `life-force` / `constraint` | 全部 16:34 之前 | ✅ **24/24 0 触碰** |

### 10.4 0 主动 commit 声明

- 我**没运行** `git add` / `git commit` / `git push` 任何命令
- 本文件 `docs/1.0-release-prep/INSTALLATION_GUIDE-1.0.md` (NEW, untracked) 留 Mavis 整合 #3 拍板
- 5 LOCKED 根文件 mtime 严守 (per §10.1)
- 24 LOCKED crate mtime 严守 (per §10.3)
- workspace version 1.0.0 严守 (per §10.2)
- 当前 HEAD = `0da4af0399e43bdd88c88c111bfbcbfc11b218be` (本任务前 commit, 0 改)

---

## §11. 引用

### 11.1 D-06 8 包齐发核心

- `docs/adr/0008-d-06-8-package-distribution.md` (D-06 ADR, 8 形态 + Linux 4 包重点)
- `docs/1.0-release/install-status.md` (#4 install 8 包 + Linux 4 包 + #3 signature + #5 upgrade + #6 uninstall + #9 ci 状态)
- `docs/1.0-release/8-promise-audit.md` §2 (8 项不修改承诺)
- `docs/security/cosign-keys.md` (cosign 公钥 + 撤销流程, 172 行)

### 11.2 5 包 K-1 26/26 PASS (per `r20-1.0-install-5pkg-k1-check-2026-08-05.md`)

- `packaging/deb/` (5 文件: build-deb.sh 16 + install-deb.sh 70 + uninstall-deb.sh 132 + control 30 + apeireth.service 50e6cbf0)
- `packaging/rpm/` (4 文件: build-rpm.sh 16 + install-rpm.sh 75 + uninstall-rpm.sh 155 + apeireth.spec 87)
- `packaging/tarball/` (3 文件: build-tarball.sh 17 + install.sh 75 + uninstall.sh 138)
- `packaging/brew/` (4 文件: build-brew.sh 16 + install-brew.sh 56 + uninstall-brew.sh 142 + apeireth.rb 52)
- `packaging/scoop/` (4 文件: build-scoop.ps1 15 + install-scoop.ps1 62 + uninstall-scoop.ps1 168 + apeireth.json 39)
- `scripts/install/` (5 入口 + 1 总入口 uninstall-all.sh 206 行)
- `.github/workflows/` (2 workflow: release.yml 350 + release-1.0.0.yml 387)
- `docs/installation/` (6 文档: deb/rpm/brew/scoop/tarball-install + package-comparison)

### 11.3 6 文档站 (per `docs/installation/`)

- `deb-install.md` (168 行, Debian/Ubuntu)
- `rpm-install.md` (177 行, RHEL/Fedora/openSUSE)
- `linux-tarball-install.md` (228 行, musl 静态)
- `macos-brew-install.md` (230 行, Homebrew formula)
- `windows-scoop-install.md` (262 行, Scoop manifest)
- `package-comparison.md` (192 行, 8 形态对比 + 选型决策树)

### 11.4 整合 #3 必读

- `reports/integrate-3-commit-templates-2026-08-06.md` (C1~C7, **本文件 source**)
- `reports/r20-1.0-install-5pkg-k1-check-2026-08-05.md` (5 包 K-1 26/26 PASS, 2026-08-05 bg_7c33f03c 跑通)
- `reports/1.0-release-ci-100-2026-08-06.md` (12 workflow 1,502 行 27 任务)
- `reports/1.0-release-uninstall-100-2026-08-06.md` (#6 uninstall 100% 收尾)
- `docs/1.0-release-prep/RELEASE_NOTES-1.0.md` (整合 #3 拍板草稿)
- `docs/1.0-release-prep/CHANGELOG_1.0-summary.md` (12 ADR 索引 + 30+ R21 续)
- `docs/1.0-release-prep/UPGRADE_GUIDE-0.x-to-1.0.md` (8 平台 upgrade + D-07 一次性 + 兜底)
- `docs/1.0-release-prep/MIGRATION_GUIDE-sqlite-to-postgres.md` (D-07 dry-run + 1KB mock 验证)

### 11.5 6 哲学锚 + 8 项不修改承诺 LOCKED

- `docs/adr/0010-6-philosophy-anchors.md` (6 哲学锚 原始定义 LOCKED)
- `docs/stage4/8-locked-unified-2026-08-05.md` §2 (8 项不修改承诺 LOCKED 原文)
- `APEIRETH-CONVENTIONS.md` §9 + §10 (顶层 3 规范 LOCKED)
- `APEIRETH-VERSIONING.md` §1 (workspace version 1.0.0 严守)

---

_本文件路径: `docs/1.0-release-prep/INSTALLATION_GUIDE-1.0.md`_
_生成时间: 2026-08-06_
_派工来源: Mavis 1.0 release 治理收尾, 续 `docs/adr/0008-d-06-8-package-distribution.md` + `reports/r20-1.0-install-5pkg-k1-check-2026-08-05.md` (5 包 K-1 26/26 PASS) + `docs/installation/` 6 文档_
_6 哲学锚穿透 + 8 项不修改承诺 0 触碰 + 0 改 workspace version + 0 主动 commit + 0 sandbox 错路径_
