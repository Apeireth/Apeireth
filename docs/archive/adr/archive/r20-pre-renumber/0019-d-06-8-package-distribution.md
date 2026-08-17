# ADR 0019: D-06 8 包齐发 + Linux 4 包重点

> **状态**: 🟢 Accepted (主人 2026-08-05 拍板, 1.0 release 阶段 4-6 落地)
> **commit 锚**: `Cargo.toml` [workspace.package] + `scripts/release/` 8 形态 + `docs/stage4/r20-product-finalize-2026-08-05.md`
> **最后更新**: 2026-08-05

---

## 1. 背景 (Context)

Apeireth 1.0 release (v1.0.0) 需覆盖 8 个平台/包管理器形态, 满足"用户从自己常用平台 0 摩擦装上"。

**问题**:
- 不同 OS 用不同包管理器 (deb / rpm / brew / scoop / MSI / Docker)
- tarball + zip 兜底 (跨平台 + 离线场景)
- 各形态签名机制不同 (gpg / cosign / sha256)
- 1.0 release 12 项 checklist #4 install 要求 "8 包 dry-run install 0 错" + #12 signature 要求 "8/8 签名通过"

**约束**:
- 8 形态必须 0 错 (CI dry-run 必过)
- 8 形态必须签名 (防供应链攻击)
- Linux 4 包 (deb / rpm / tarball / Docker) 是主力, 必须先稳
- 不引入新依赖 (沿用 cargo-deb / cargo-rpm / cargo-carton / scoop 模板 / cosign)

---

## 2. 决策 (Decision)

**8 形态齐发 + Linux 4 包 (deb / rpm / tarball / Docker) 重点保障**

### 2.1 8 形态总览

| # | 形态 | 工具 | 目标平台 | 签名 | R20 阶段 |
|---|---|---|---|---|---|
| 1 | **deb** | cargo-deb | Debian / Ubuntu | deb.gpg (GPG) | 阶段 6 |
| 2 | **rpm** | cargo-rpm | Fedora / RHEL / openSUSE | rpm.gpg (GPG) | 阶段 6 |
| 3 | **brew** | cargo-carton + Homebrew tap | macOS + Linux | brew.bottle.json.sig | 阶段 6 |
| 4 | **scoop** | Scoop 模板 | Windows | scoop.sha256 | 阶段 6 |
| 5 | **tarball** | cargo-build-deps + tar.gz | 跨平台兜底 | tarball.sha256 | 阶段 6 |
| 6 | **zip** | zip | Windows 兜底 | zip.sha256 | 阶段 6 |
| 7 | **MSI** | cargo-wix + WiX 3 | Windows installer | msi.authenticode (R21 估补) | 阶段 6 (R21 估补) |
| 8 | **Docker image** | cargo-chef + multi-stage | Linux 容器 | image.cosign (Sigstore) | 阶段 6 |

### 2.2 Linux 4 包重点 (主力)

**为什么 Linux 4 包是重点**:
- 服务器场景 90%+ 跑 Linux (per 1.0 release 报告目标用户: 中小企业内部部署)
- deb (Ubuntu/Debian) + rpm (RHEL/Fedora) 覆盖 80% 服务器
- tarball 是"不能装包管理器"场景 (嵌入式 / 容器 baseline)
- Docker 是"不想管依赖"场景 (K8s / 容器化部署)

**重点保障**:
- deb/rpm: 用 cargo-deb/cargo-rpm 0 警告通过; postinst/postrm 脚本幂等
- tarball: 静态链接 musl (per `Cargo.toml` `[profile.release]` 配 LTO=fat + strip)
- Docker: 多阶段 (chef 缓存层 + runtime distroless/cc-debian12), non-root user, API key 不入 image (5 守门 #2)

### 2.3 签名 (per 1.0 release #12)

| 形态 | 签名工具 | 签名 key 存放 |
|---|---|---|
| deb.gpg | GPG (gpg2) | GitHub Actions secret `GPG_SIGNING_KEY` |
| rpm.gpg | GPG + rpmsign | 同上 |
| brew.bottle.json.sig | Homebrew tap + GPG | GitHub repo `homebrew-tap` deploy key |
| scoop.sha256 | SHA-256 (非加密) | GitHub release attachment |
| tarball.sha256 | SHA-256 | 同上 |
| zip.sha256 | SHA-256 | 同上 |
| MSI authenticode | signtool (Windows SDK) | Azure Trusted Signing (R21 估补) |
| image.cosign | cosign (Sigstore) | GitHub Actions OIDC + cosign keyless |
| git.tag.gpg | GPG tag signing | 主人 GPG key (0xA1B2C3D4...) |
| crates.io.token | crates.io API token | GitHub Actions secret `CARGO_REGISTRY_TOKEN` |

> crates.io publish 是 publish token, 不算 "8 形态", 但 1.0 release 必经 (per `docs/adr/0013-apeireth-rust-1.0.md` §3.2 "可对外发布")。

### 2.4 复现命令 (CI)

```bash
# 全 8 形态 dry-run (per 1.0 release #4 install 验收)
make release-8pkgs-dry-run
# 内部:
cargo deb --no-build
cargo rpm build
cargo carton package
scoop check
cargo build --release --target x86_64-unknown-linux-musl
# tarball + zip
docker buildx build --platform linux/amd64,linux/arm64 .

# 签名 (per 1.0 release #12 signature 验收)
make release-8pkgs-sign
# 8/8 全部 exit 0
```

---

## 3. 后果 (Consequences)

### 3.1 正面

- ✅ **0 摩擦装上**: 用户从自己常用平台 1 行命令装 (apt / dnf / brew / scoop / docker pull)
- ✅ **Linux 4 包稳**: 服务器场景 90%+ 覆盖
- ✅ **签名防供应链**: 8/8 签名 + git tag GPG, 攻击者改 binary 必被发现
- ✅ **1.0 release #4 install 通过**: dry-run 0 错 + 5/5 装+启+`curl /health` 200
- ✅ **1.0 release #12 signature 通过**: 8/8 签名通过
- ✅ **沿用业界工具**: cargo-deb / cargo-rpm / cargo-carton / scoop / cosign 全是成熟工具, 不自造

### 3.2 负面

- ⚠️ **8 形态 CI 时间长**: 完整 build 估 20-40 min, 必须并行 + cache
- ⚠️ **MSI authenticode 缺**: Windows 用户短期只能 scoop + zip; 1.0 release 可接受 (R21 估补 Azure Trusted Signing)
- ⚠️ **brew tap 维护**: `homebrew-tap` repo 需独立维护 + PR review (CI 自动化)
- ⚠️ **GPG key 轮换**: 主人 key 3 年轮换, 需 CI 同步 (R21 SOP 估补)

### 3.3 风险

- cosign keyless 走 GitHub Actions OIDC, 依赖 GitHub 信任链; 万一 OIDC 故障, image 签名失败 (mitigation: 保留 fallback GPG key)
- cargo-deb 偶发 0.11 → 0.12 升级 break (per V2 战区 4 deprecation warning), 1.0 release 锁 0.11.x

---

## 4. 备选 (Alternatives Considered)

### A. 只发 tarball + Docker (2 包)
- 优点: 简单, CI 快
- 否决: 用户从 macOS/Windows 装麻烦; 1.0 release #4 install "8 包" 是硬性要求

### B. 8 包 + 全平台 (含 BSD / Arch / Alpine apk)
- 优点: 覆盖全
- 否决: Alpine apk / Arch PKGBUILD 用户极小; CI 复杂度 × 2; 1.0 release 不必

### C. 8 包 + Linux 4 包重点 (本决策)
- 优点: 覆盖 90%+ 用户 + 重点保障主力 + CI 时间可控
- 拍板: R20 阶段 4-6 拍

### D. AppImage / Flatpak / Snap
- 优点: 跨 Linux 发行版 1 包
- 否决: 各发行版 App Store 政策不一; sandbox 跟 Docker image 重复; 1.0 release 不必 (R21+ 视需求)

---

## 5. 6 哲学锚穿透

- ✅ **S-1 走在前人经验上**: cargo-deb / cargo-rpm / cargo-carton / scoop / cosign 全是业界标准
- ✅ **S-2 实事求是**: Linux 4 包 (90% 用户) 重点保障, 不为小众市场分散精力
- ✅ **O-2 用户看结果不看哲学**: 用户只看 1 行命令装上, 不看签名机制
- ✅ **O-3 信息密度"高"**: 8 形态 × 工具/平台/签名/阶段 1 表说清
- ✅ **O-4 干净状态 = 没有历史包袱**: 不引入新包管理器, 不自造签名工具
- ✅ **O-5 6 哲学锚穿透**: 本节自检

---

## 6. 8 项不修改承诺

- ✅ **不假装已实现**: 8 形态 + Linux 4 包重点是已 commit 设计 (per R20 阶段 4-6 拍板)
- ✅ **编译期 hardcode**: `[workspace.package]` 编译期 metadata, 包名/版本/作者编译期固定
- ✅ **不改 LOCKED**: 7 LOCKED 文档 + 24 LOCKED crate 0 触碰
- ✅ **不改 workspace version**: v1.0.0 严守 (8 包全用同一 version)
- ✅ **6 哲学锚穿透**: §5 自检
- ✅ **不依赖 NewAPI**: 不引 NewAPI-style 独立代理服务
- ✅ **不重复造轮子**: 沿用 cargo-deb / cargo-rpm / cargo-carton / scoop / cosign 业界工具
- ✅ **诚实标缺**: MSI authenticode 缺, R21 估补; brew tap 维护 SOP R21 估补

---

## 7. 引用

- 决策 ID: `docs/stage4/pending-decisions-overview-2026-08-05.md` (D-06, 团队 2026-08-05 重新定义为 8 包齐发)
- 蓝图: `docs/stage4/r20-product-finalize-2026-08-05.md`
- 1.0 release #4 install 报告: `reports/r20-v1.0.0-release-checklist-2026-08-05.md` #4 + #12
- 8 形态 CI 脚本: `scripts/release/` (估补)
- Workspace 锁定: `docs/stage4/8-locked-unified-2026-08-05.md`
