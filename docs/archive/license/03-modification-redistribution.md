# 修改 + 再分发 Apeireth (Modification & Redistribution)

> **性质**: 修改源码 / fork / Docker 镜像 / 包管理 等再分发场景
> **依据**: Apache-2.0 §1-4 (版权 + 许可 + 条件) + §3 (Source form)
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-5)

---

## 0. TL;DR

| 场景 | 允许? | 关键条件 |
|------|:----:|---------|
| **Fork Apeireth 改 + 公开仓库** | ✅ | 保留版权 + LICENSE + NOTICE + 改动说明 |
| **Fork Apeireth 改 + 闭源仓库** | ✅ | 客户能拿到你修改后的源码 (per §3) |
| **Docker 镜像分发** | ✅ | 镜像层附 LICENSE + NOTICE + 源码可获取 |
| **APT / YUM / Homebrew 包** | ✅ | 包内附 LICENSE + NOTICE + Source 链接 |
| **PyPI / crates.io / npm 发包** | ✅ | 包 description 引 Apache-2.0 + Source 链接 |
| **App Store / Play Store 上架** | ✅ | About / Legal 页列 Apache-2.0 + 链接 |
| **预装到设备 / 操作系统** | ✅ | 设备 "About" / "Legal" 附 |
| **嵌入到更大的软件** | ✅ | 你大软件的 "Third-party" 列 Apeireth |
| **改 Apeireth 名字再分发** | ✅ | 改后**不**能用 "Apeireth" 名 (per §6) |
| **去掉版权 / LICENSE 再分发** | ❌ | **禁止** (per §4(a)) |

---

## 1. 你必须做的 5 件事 (per §4)

### 1.1 保留版权 (per §4(a))

每个你分发的文件**必须**保留原始版权声明:

```text
// 原文件 (per apeireth-core/src/lib.rs):
// Copyright 2026 Apeireth Team
// Licensed under the Apache License, Version 2.0 (the "License");
// ...
```

**你的修改后文件**:

```text
// Copyright 2026 Apeireth Team
// Modified by YourCompany on 2026-xx-xx  ← 加这行
// Licensed under the Apache License, Version 2.0 (the "License");
// ...
```

### 1.2 附 LICENSE 副本 (per §4(a))

你分发的**每个**包 (Docker 镜像 / deb / rpm / tarball / pip / crate / app bundle / etc) **必须**包含:
- 完整 `LICENSE` 文件 (180 行 Apache-2.0 原文)
- 完整 `NOTICE` 文件 (71 行项目声明 + 致谢)
- 完整 `THIRD-PARTY-NOTICES.md` (1709 行 561 crate attribution)

**实操**: 在你打包脚本里加:

```bash
# Debian / Ubuntu
cp ../Apeireth-rust/LICENSE /usr/share/doc/yourpackage/LICENSE
cp ../Apeireth-rust/NOTICE /usr/share/doc/yourpackage/NOTICE
cp ../Apeireth-rust/THIRD-PARTY-NOTICES.md /usr/share/doc/yourpackage/THIRD-PARTY-NOTICES.md

# Docker
COPY LICENSE NOTICE THIRD-PARTY-NOTICES.md /usr/share/apeireth/

# pip / PyPI
include LICENSE NOTICE THIRD-PARTY-NOTICES.md in MANIFEST.in

# cargo
在 workspace 的 package 段, 加 license-file = "LICENSE"
```

### 1.3 改动说明 (per §4(b))

如果你**改了** Apeireth 源码, **必须**在 NOTICE 或单独 CHANGELOG 写清:

```text
APEIRETH MODIFICATION NOTICE
============================
This software is a modified version of Apeireth (v1.0.0,
Copyright 2026 Apeireth Team, Apache-2.0).

Modifications made by YourCompany (2026-xx-xx):
- Added feature X (commit abc123)
- Fixed bug Y (commit def456)
- Removed deprecated API Z (commit ghi789)

Full diff: https://github.com/yourcompany/yourfork/compare/v1.0.0...main
```

### 1.4 Source form 可见 (per §3)

如果你分发**对象形式** (编译后的二进制 / Docker 镜像 / deb / rpm), 你**必须**让第三方能拿到**对应的源代码** (你修改后的):

| 方式 | 怎么做 | 备注 |
|------|--------|------|
| **仓库公开** | GitHub / GitLab 公开你的 fork | 🟢 最简单 |
| **下载链接** | 你的产品页加 "Source code" 链接 | 🟢 推荐 |
| **客户 portal** | 客户登录可下载 | 🟡 B2B 常见 |
| **CD 镜像** | 跟二进制一起分发 .tar.gz | 🟡 老派 |
| **写邮件索取** | 邮箱发请求 | 🔴 不推荐 (慢) |

### 1.5 不假背书 (per §6)

你的再分发**不能**假借 "Apeireth Team 认证" / "Apeireth 官方版" / "Apeireth 合作伙伴" 之类 (per §6 商标边界).

**实操**:
- 你的产品名**不**能叫 "Apeireth" / "Apeireth Pro" / "Apeireth Enterprise"
- 你的产品名可叫 "YourProduct (基于 Apeireth)" / "YourProduct for Apeireth" (描述性)
- 你的产品名**不**能含 Apeireth logo

---

## 2. 8 再分发场景详解

### 2.1 Fork + 公开 GitHub 仓库

**场景**: 你 fork `apeireth/apeireth-rust` 到 `yourcompany/apeireth-rust-fork`, 改 100 处, 公开仓库.

**许可要点**:
- ✅ **完美合法** (per §1 授予 "prepare Derivative Works")
- 你的 fork 仍**必须**保留 LICENSE + NOTICE (per §4)
- 你的 commit message 可写 "Modified by YourCompany" (但 git author 也算)

**实操**:
- [ ] `git fork` 后, 你的 `LICENSE` / `NOTICE` / `THIRD-PARTY-NOTICES.md` **不**动
- [ ] 你改的文件加 `Modified by YourCompany on <date>` 注释头
- [ ] 你的 `README.md` 加 "本项目修改自 [Apeireth](https://github.com/apeireth/apeireth-rust) (Apache-2.0)"
- [ ] 你的 `CHANGELOG.md` 写清 "基于 v1.0.0 的所有改动"

### 2.2 Fork + 闭源仓库

**场景**: 你改 Apeireth, 内部用, **不**公开仓库, 但卖给客户.

**许可要点**:
- ✅ **合法** (Apache-2.0 **不**强制公开, 不像 GPL/AGPL)
- ⚠️ 你**必须**让客户能拿到你修改后的源码 (per §3 Source form)
- 你**不**需要公开你的整个产品源码
- 你的客户**不**能再分发你修改的源码 (per §2 终止条款)

**实操**:
- [ ] 客户合同明确 "Apeireth 修改部分按 Apache-2.0 许可, 客户可向 YourCompany 索取源码"
- [ ] 你的商业产品源码不公开, 但 Apeireth 修改部分**单独**打成 source tarball 给客户
- [ ] 你的 `LICENSE` 写 "Apeireth 部分 Apache-2.0, YourProduct 部分 Copyright 2026 YourCompany"

### 2.3 Docker 镜像分发

**场景**: 你 `docker build -t yourcompany/apeireth:v1 .` 推到 Docker Hub, 公开分发.

**许可要点**:
- ✅ **合法** (per §1 + §3)
- ⚠️ 你的 Dockerfile **必须** `COPY LICENSE NOTICE THIRD-PARTY-NOTICES.md /usr/share/apeireth/`
- ⚠️ 你的 Docker Hub 描述引 "基于 Apeireth v1.0.0 (Apache-2.0)"
- ⚠️ 你的镜像 tag **不能**用 "official" / "verified" 之类 (per §6)

**实操 Dockerfile**:
```dockerfile
FROM apeireth/apeireth-rust:v1.0.0

# 保留法律文件
COPY --from=apeireth/apeireth-rust:v1.0.0 /Apeireth-rust/LICENSE /usr/share/apeireth/LICENSE
COPY --from=apeireth/apeireth-rust:v1.0.0 /Apeireth-rust/NOTICE /usr/share/apeireth/NOTICE
COPY --from=apeireth/apeireth-rust:v1.0.0 /Apeireth-rust/THIRD-PARTY-NOTICES.md /usr/share/apeireth/THIRD-PARTY-NOTICES.md

# 你的修改
COPY your-modifications/ /opt/your-app/
RUN /opt/your-app/install.sh

# 你的改动说明
COPY MODIFICATION_NOTICE.md /usr/share/apeireth/

# 默认 License prompt
LABEL org.opencontainers.image.licenses="Apache-2.0"
LABEL org.opencontainers.image.source="https://github.com/apeireth/apeireth-rust"
LABEL org.opencontainers.image.title="YourCompany Apeireth v1.0.0 Modified"
```

### 2.4 APT / YUM / Homebrew 包

**场景**: 你打包 `yourcompany-apeireth` 给 Debian / Ubuntu / Fedora / CentOS / macOS 用户.

**许可要点**:
- 跟 Docker 镜像一样, **必须**附 LICENSE + NOTICE + THIRD-PARTY-NOTICES.md

**实操 Debian control 文件**:
```
Package: yourcompany-apeireth
Version: 1.0.0-1
Section: net
Priority: optional
Architecture: amd64
Depends: libssl3
Maintainer: Your Company <ops@yourcompany.com>
Description: YourCompany modified Apeireth
  This package is a modified version of Apeireth v1.0.0 (Apache-2.0).
  .
  Apeireth is a long-running AI growth platform.
  See /usr/share/doc/yourcompany-apeireth/LICENSE for the full license.
Copyright: 2026 Apeireth Team (Apache-2.0), 2026 YourCompany (modifications)
```

### 2.5 crates.io / PyPI / npm 发包

**场景**: 你做一个 wrapper crate / pip package / npm package 包 Apeireth.

**许可要点**:
- ✅ 你的 wrapper package 可用你公司 license (e.g. MIT / Proprietary)
- ⚠️ 但你的 package **依赖**声明里**必须**列 Apeireth = Apache-2.0
- ⚠️ 你的 README 引 "本 package 基于 Apeireth v1.0.0 (Apache-2.0)"

**实操 Cargo.toml**:
```toml
[package]
name = "yourcompany-apeireth-wrapper"
version = "1.0.0"
edition = "2021"
license = "MIT"  # 你 wrapper 的 license
authors = ["Your Company"]

[dependencies]
apeireth-sdk = "1.0"  # 这是 Apache-2.0
```

**实操 setup.py**:
```python
setup(
    name='yourcompany-apeireth-wrapper',
    version='1.0.0',
    license='MIT',  # 你 wrapper 的 license
    install_requires=[
        'apeireth-sdk>=1.0.0',  # 这是 Apache-2.0
    ],
    long_description='''
    This package wraps Apeireth SDK v1.0.0 (Apache-2.0).
    See https://github.com/apeireth/apeireth-rust for the full license.
    ''',
)
```

### 2.6 App Store / Play Store 上架

**场景**: 你做移动 app 嵌入 Apeireth, 上 Google Play / Apple App Store.

**许可要点**:
- ✅ 合法
- ⚠️ App 的 "Open source licenses" 页面 (Android: Settings → About → Open source licenses; iOS: 在你 app 内置页面) 列 Apeireth + Apache-2.0
- ⚠️ App Store Connect / Play Console 的 "Acknowledgements" 字段引 Apache-2.0

**实操**:
- 你的 app 启动时展示一次 "Open source licenses" 页面
- 该页面列所有依赖 + license, 包括 Apeireth (Apache-2.0)
- 链接到 https://www.apache.org/licenses/LICENSE-2.0

### 2.7 预装到设备 / 操作系统

**场景**: 你做路由器 / NAS / IoT 设备, 预装 Apeireth.

**许可要点**:
- 跟 2.2 嵌入商业产品完全一样
- 设备的 "About" / "Legal" / "Settings → Open source" 页加 Apeireth (Apache-2.0) + 链接
- 设备的用户手册 (纸质或 PDF) 附完整 LICENSE (或 QR 码)

### 2.8 嵌入到更大的软件

**场景**: 你做 IDE / SaaS 平台, 把 Apeireth 嵌进去.

**许可要点**:
- 跟 2.6 / 2.7 一样
- 你的软件的 "Third-party software" / "Open source licenses" 页列 Apeireth
- 你**不**需要公开你整个软件的源码 (Apache-2.0 **不**传染)

---

## 3. 商标边界 (再分发场景, per §6)

### 3.1 你能用的

| 用途 | 例 | 允许? |
|------|-----|:----:|
| 描述来源 | "本产品基于 Apeireth v1.0.0 修改" | ✅ |
| 描述兼容 | "兼容 Apeireth 协议" / "支持 Apeireth API" | ✅ |
| 描述分支 | "Apeireth fork by YourCompany" | ✅ |
| 比较 | "比 Apeireth 多 X 功能" | ✅ |

### 3.2 你不能用的

| 用途 | 例 | 不允许? |
|------|-----|:----:|
| 暗示官方 | "Apeireth 官方合作伙伴" | ❌ |
| 注册商标 | 注册 "Apeireth-Pro" 商标 | ❌ |
| 用 logo | 用 Apeireth logo 当你产品 logo | ❌ |
| 域名 | 域名 `apeireth-enterprise.com` | ❌ |
| 营销 | "Apeireth Team 推荐的 X" | ❌ |
| 改写标语 | 把 "Apeireth, 试一试" 改写当你的 | ❌ |

### 3.3 安全命名

| 你的产品名 | 是否侵权? |
|-----------|:--------:|
| `YourProduct` | ✅ |
| `YourProduct (基于 Apeireth)` | ✅ |
| `Apeireth-YourProduct` | ⚠️ 看具体用法, 描述性 OK, 营销 ❌ |
| `Apeireth YourProduct` | ⚠️ 同上 |
| `YourApeireth` | ❌ (暗示你拥有) |
| `Apeireth Pro` | ❌ (暗示官方) |
| `Official Apeireth Build` | ❌ (假背书) |

---

## 4. 链接 / 镜像合规

### 4.1 GitHub 镜像

**场景**: 你在中国 / 内部网络, 镜像 GitHub 仓库到 GitLab / Gitee / 你公司 Git.

**许可要点**:
- ✅ 合法 (per §1 授予 "reproduce" + "distribute")
- ⚠️ 你的镜像 README 保留原版权 + LICENSE + NOTICE
- ⚠️ 你的镜像**不**能改 commit history (commit author + date 必须保留)

### 4.2 二进制下载站

**场景**: 你运营下载站, 放 Apeireth 编译版供用户下载.

**许可要点**:
- ✅ 合法
- ⚠️ 下载页**必须**有 "Source code" 链接 (指向 GitHub)
- ⚠️ 下载包**必须**含完整 LICENSE + NOTICE + THIRD-PARTY-NOTICES.md

### 4.3 CDN 分发

**场景**: 你用 CloudFlare / AWS CloudFront 分发 Apeireth release tarball.

**许可要点**:
- 跟 4.2 一样
- 你的 CDN 缓存**不**改变文件内容
- 你的 CDN URL 公开可达

---

## 5. 专利边界 (再分发场景, per §3)

### 5.1 你拿到什么

再分发场景**不**改变 §3 专利授权, 你仍享受:
- ✅ 制造 / 使用 / 卖 / 进口 Apeireth 的专利许可
- ⚠️ 你**不能**告 Apeireth 专利侵权 (否则你的授权终止 per §3(b))

### 5.2 你的修改的专利

你修改的代码 = 你的版权, **你拥有**你修改部分的专利 (如果你申请了).
- 你的客户用你修改的 Apeireth 时, **不**自动获得你修改部分的专利许可 (除非你明确授予)
- 你**应该**在 LICENSE 写 "YourCompany modifications are licensed under MIT" (或你公司 license)

### 5.3 专利反诉条款 (§3(b))

| 场景 | 你的 Apache-2.0 专利许可 |
|------|---------------------:|
| 你**不**告 Apeireth 专利侵权 | ✅ 继续有效 |
| 你告 Apeireth 专利侵权 | ❌ **立即终止** |

---

## 6. 不允许的再分发 (4 红线)

### ❌ 红线 1: 去掉 LICENSE

```text
# 错误: 你把 LICENSE 删了
$ ls yourpackage/
bin/ etc/ lib/ var/  # 没有 LICENSE NOTICE THIRD-PARTY-NOTICES.md
```

→ **违反** Apache-2.0 §4(a), 你的客户**没**法验证 license, 没法合规使用.

### ❌ 红线 2: 改版权

```text
# 错误: 你把版权改成你的, 删了原版权
# Copyright 2026 YourCompany  # ← 删了原 "Copyright 2026 Apeireth Team"
```

→ **违反** §4(a) "must give any other recipients of the Work... a copy of this License" + 改了原始 attribution.

### ❌ 红线 3: 加额外限制

```text
# 错误: 你加限制 "本软件仅 YourCompany 客户可商用"
# 违反 Apache-2.0 §4 限制条款
```

→ **违反** §1 "no additional restrictions" (per §2 终止条款).

### ❌ 红线 4: 假背书

```text
# 错误: 你说 "本产品是 Apeireth Team 官方认证"
```

→ **违反** §6 商标边界.

---

## 7. 实操清单 (再分发)

| # | 任务 | 工具 | 必须? |
|---|------|------|:----:|
| 1 | 改动的文件加 `Modified by YourCompany on <date>` | git | ✅ |
| 2 | 附 `LICENSE` (180 行) | 打包 | ✅ |
| 3 | 附 `NOTICE` (71 行) | 打包 | ✅ |
| 4 | 附 `THIRD-PARTY-NOTICES.md` (1709 行) | 打包 | ✅ |
| 5 | 加 `MODIFICATION_NOTICE.md` 改动说明 | 打包 | ✅ (如改了) |
| 6 | 让客户能拿到你修改后的源码 (Source form) | git / 客户 portal | ✅ |
| 7 | 0 用 "Apeireth" 商标做营销 | 品牌 | ✅ |
| 8 | 0 假背书 (不用 "official" / "verified") | 描述 | ✅ |
| 9 | 不加额外限制 | LICENSE | ✅ |
| 10 | 0 改原 copyright | 打包 | ✅ |

---

## 8. 相关

- 根 `LICENSE` (Apache-2.0 完整, 180 行)
- 根 `NOTICE` (项目声明 + 致谢, 71 行)
- 根 `THIRD-PARTY-NOTICES.md` (1709 行, 561 crate attribution)
- [01-contribution.md](01-contribution.md) (贡献流程)
- [02-commercial-use.md](02-commercial-use.md) (商业使用)
- [04-faq.md](04-faq.md) (18 常见问题)
- [05-spdx-reference.md](05-spdx-reference.md) (12 SPDX 类别)
- https://www.apache.org/licenses/LICENSE-2.0 (Apache-2.0 原文)
- https://www.apache.org/foundation/marks/ (Apache 商标政策)

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-5)
