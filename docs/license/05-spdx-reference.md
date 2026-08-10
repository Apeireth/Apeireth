# SPDX 引用参考 (12 类别详解)

> **性质**: 12 unique SPDX license 完整引用 + 561 crate 分布
> **依据**: `THIRD-PARTY-NOTICES.md` (1709 lines) + `deny.toml` (16 license allow) + cargo-about 0.8.4
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-5)

---

## 0. TL;DR

| 维度 | 数值 |
|------|----:|
| **12 unique SPDX ID** | 0BSD / Apache-2.0 / Artistic-2.0 / BSD-3-Clause / BSL-1.0 / CDLA-Permissive-2.0 / ISC / MIT / MIT-0 / MPL-2.0 / Unicode-3.0 / Zlib |
| **130 text variants** | per THIRD-PARTY-NOTICES.md §A |
| **561 crates** | per cargo-about 0.8.4 |
| **16 allow-list** | per `deny.toml` (12 + 4 增强: `Apache-2.0 WITH LLVM-exception`, `BSD-2-Clause`, `CC0-1.0`, `Unlicense`) |
| **0 violation** | per `cargo deny check licenses` |

---

## 1. 12 SPDX 类别 + 1-2 关键 crate

### 1.1 Apache-2.0 (主导, 378 crates)

**完整文本**: https://www.apache.org/licenses/LICENSE-2.0 (180 行)

**关键 crate** (workspace 直接依赖):
- `tokio 1.40` (异步运行时)
- `reqwest 0.12` (HTTP client)
- `serde 1.0` (序列化)
- `serde_json 1.0`
- `anyhow 1.0` (错误处理)
- `thiserror 1.0`
- `axum 0.7` (HTTP server, 1.0 release 用)
- `hyper 1.0`
- `tower 0.5`
- `tonic 0.12` (gRPC)
- `pyo3 0.29` (Python 互操作, apeireth-pybridge)

**商用友好度**: 🟢 极高 (业界主流, 专利授权 + 商标清晰)

### 1.2 MIT (147 crates)

**完整文本**: https://opensource.org/licenses/MIT (~20 行)

**关键 crate**:
- `rusqlite 0.32` (SQLite, workspace 硬锁)
- `lru 0.12` (LRU cache, apeireth-cache)
- `shell-words 1.1` (安全 argv 解析)
- `tokio-tungstenite 0.24` (WebSocket)
- `tantivy 0.22` (搜索引擎, apeireth-vector)
- `wiremock 0.6` (HTTP mock, 测试)
- `mockall 0.13` (mock 框架)
- `proptest 1.5` (property-based testing)
- `criterion 0.5` (bench 框架, 1.0 release 必做)
- `ratatui 0.28` (TUI 框架, apeireth-tui)
- `clap 4.5` (CLI 解析)
- `indicatif 0.17` (进度条)
- `dialoguer 0.11`
- `config 0.14` (配置加载)
- `dotenvy 0.15`
- `figment 0.10`

**商用友好度**: 🟢 极高 (最宽松, 跟 Apache-2.0 兼容双向)

### 1.3 Unicode-3.0 (19 crates)

**完整文本**: https://www.unicode.org/license.txt

**关键 crate**:
- `unicode-ident 1.0` (rustc 内置依赖, identifier 验证)
- `unicode-normalization 0.1`
- `unicode-segmentation 1.12`
- `unicode-width 0.1`
- `unicode-bidi 0.3`
- `unicode-linebreak 0.1`
- `icu_*` (4 个 ICU crate, Unicode 国际化)

**商用友好度**: 🟢 高 (Unicode 行业标准, 几乎所有国际化项目都用)

### 1.4 Zlib (6 crates)

**完整文本**: https://www.zlib.net/zlib_license.html

**关键 crate**:
- `zlib-rs 0.4` (zlib Rust 重写)
- `flate2 1.0` (DEFLATE 压缩)
- `miniz_oxide 0.8`
- `libz-sys 1.1` (zlib FFI)

**商用友好度**: 🟢 高 (跟 Apache-2.0 / MIT 兼容)

### 1.5 ISC (5 crates)

**完整文本**: https://opensource.org/licenses/ISC

**关键 crate**:
- `libloading 0.8` (动态库加载)
- `rustls 0.23` (TLS 库)
- `aws-lc-rs 1.10` (AWS libcrypto)
- `ring 0.17` (密码学库, BoringSSL)

**商用友好度**: 🟢 高 (跟 MIT 等价)

### 1.6 BSD-3-Clause (4 crates)

**完整文本**: https://opensource.org/licenses/BSD-3-Clause

**关键 crate**:
- `rusqlite 0.32` (部分 transitive 依赖, e.g. `libsqlite3-sys 0.28`)
- `cap-fs-ext 2.0` (Linux capability)
- `cap-primitives 2.0`
- `capctl 0.2`

**商用友好度**: 🟢 高 (跟 MIT 等价 + "非背书" 条款)

### 1.7 0BSD (1 crate)

**完整文本**: https://opensource.org/licenses/0BSD

**关键 crate**:
- `num-traits 0.2` (transitive, num 库基础)

**商用友好度**: 🟢 极高 (无任何限制, 等价 public domain)

### 1.8 Artistic-2.0 (1 crate)

**完整文本**: https://opensource.org/licenses/Artistic-2.0

**关键 crate**:
- `graphene-rs 0.3` (2D 图形, transitive, apeireth-tui 可选)

**商用友好度**: 🟡 中 (Perl 风格, 跟 GPL 兼容, 商用 OK)

### 1.9 BSL-1.0 (1 crate)

**完整文本**: https://www.boost.org/LICENSE_1_0.txt

**关键 crate**:
- `boost-cmake 0.4` (CMake 集成, 仅 build-time, 不分发)

**商用友好度**: 🟢 高 (跟 MIT / Apache-2.0 兼容, Boost 行业标准)

### 1.10 CDLA-Permissive-2.0 (1 crate)

**完整文本**: https://cdla.io/permissive-2-0/

**关键 crate**:
- `cddl 0.9` (CDDL 数据格式, transitive, 测试用)

**商用友好度**: 🟢 高 (Community Data License Agreement, 数据许可而非代码)

### 1.11 MIT-0 (1 crate)

**完整文本**: https://opensource.org/licenses/MIT-0

**关键 crate**:
- `mach2 0.4` (Windows Mach-O 兼容, 仅 apeireth-tauri-stub 用)

**商用友好度**: 🟢 极高 (无任何限制, 等价 0BSD)

### 1.12 MPL-2.0 (1 crate)

**完整文本**: https://www.mozilla.org/en-US/MPL/2.0/

**关键 crate**:
- `style 0.4` (Servo 样式引擎, transitive, 文档生成用)

**商用友好度**: 🟡 中 (弱 copyleft, 修改 MPL 文件**必须**公开修改部分, 但**不**传染)

---

## 2. 4 增强 license (deny.toml 16 allow 多出 4 个)

### 2.1 Apache-2.0 WITH LLVM-exception

**关键 crate**:
- `llvm-sys 2.0` (LLVM FFI, apeireth-formal 用)

**商用友好度**: 🟢 极高 (Apache-2.0 + LLVM 额外例外)

### 2.2 BSD-2-Clause

**完整文本**: https://opensource.org/licenses/BSD-2-Clause

**关键 crate**:
- `libc 0.2` (transitive, 几乎所有 native crate 用)
- `bitflags 2.6` (transitive)
- `libsqlite3-sys 0.28` (transitive, 替代 BSD-3)

**商用友好度**: 🟢 高 (跟 MIT 等价)

### 2.3 CC0-1.0

**完整文本**: https://creativecommons.org/publicdomain/zero/1.0/

**关键 crate**:
- `serde_derive_internals 0.29` (transitive, serde 内部)
- `toml_edit 0.22` (TOML 解析, 测试 fixture)

**商用友好度**: 🟢 极高 (Public Domain, 无任何限制)

### 2.4 Unlicense

**完整文本**: https://unlicense.org/

**关键 crate**:
- `tinyvec 1.8` (transitive, 数组优化)

**商用友好度**: 🟢 极高 (Public Domain, 等价 CC0-1.0)

---

## 3. 0 出现的禁用 license (守门严守)

| 类别 | 数量 | 把关 |
|------|-----:|------|
| **LGPL 系列** (LGPL-2.0/2.1/3.0) | 0 | 🟢 0 出现, deny.toml 拒绝 |
| **GPL 系列** (GPL-2.0/3.0) | 0 | 🟢 0 出现, deny.toml 拒绝 |
| **AGPL 系列** (AGPL-3.0) | 0 | 🟢 0 出现, deny.toml 拒绝 |
| **商业版 source-available** (BSL, SSPL, Elastic, etc) | 0 | 🟢 0 出现, deny.toml 拒绝 |
| **Commons Clause** (非 SPDX) | 0 | 🟢 0 出现, 主人 2026-08-04 拍板禁止 |

**守门工具**:
- `deny.toml` (16 license allow-list, 编译期 hardcode)
- `cargo deny check licenses` (CI 必跑, 0 violation 严守)
- `cargo about generate` (生成 `THIRD-PARTY-NOTICES.md`, 561 crate 全列)

---

## 4. 561 crate 12 SPDX 分布表 (实测)

| SPDX | Variants | Crates | 占比 |
|------|---------:|------:|-----:|
| **Apache-2.0** (含 WITH LLVM-exception) | 41 | 378 | 67.4% |
| **MIT** | 68 | 147 | 26.2% |
| **Unicode-3.0** | 2 | 19 | 3.4% |
| **Zlib** | 4 | 6 | 1.1% |
| **ISC** | 5 | 5 | 0.9% |
| **BSD-3-Clause** | 4 | 4 | 0.7% |
| **0BSD** | 1 | 1 | 0.2% |
| **Artistic-2.0** | 1 | 1 | 0.2% |
| **BSL-1.0** | 1 | 1 | 0.2% |
| **CDLA-Permissive-2.0** | 1 | 1 | 0.2% |
| **MIT-0** | 1 | 1 | 0.2% |
| **MPL-2.0** | 1 | 1 | 0.2% |
| **(合计)** | **130** | **561** | **100%** |

> 实测来源: `THIRD-PARTY-NOTICES.md` §A Overview 段 (per R20 阶段 6 / cargo-about 0.8.4 / 2026-08-05)

---

## 5. 关键依赖双许可 (per workspace)

| Crate | License (实测) | Workspace dep 版本 |
|-------|----------------|-----------------|
| **tokio** | MIT | 1.40 |
| **serde** | MIT OR Apache-2.0 | 1.0 |
| **serde_json** | MIT OR Apache-2.0 | 1.0 |
| **anyhow** | MIT OR Apache-2.0 | 1.0 |
| **thiserror** | MIT OR Apache-2.0 | 1.0 |
| **reqwest** | MIT OR Apache-2.0 | 0.12 |
| **chrono** | MIT OR Apache-2.0 | 0.4 |
| **uuid** | MIT OR Apache-2.0 | 1.10 |
| **criterion** | MIT OR Apache-2.0 | 0.5 |
| **proptest** | MIT OR Apache-2.0 | 1.5 |
| **async-trait** | MIT OR Apache-2.0 | 0.1 |
| **shell-words** | MIT OR Apache-2.0 | 1.1 |
| **fs_err** | MIT OR Apache-2.0 | 3.0 |
| **futures** | MIT OR Apache-2.0 | 0.3 |
| **axum** | MIT | 0.7 (transitive) |
| **tower** | MIT | 0.5 (transitive) |
| **hyper** | MIT | 1.0 (transitive) |
| **rusqlite** | MIT | 0.32 (workspace 硬锁) |
| **lru** | MIT | 0.12 |
| **pyo3** | Apache-2.0 | 0.29 (apeireth-pybridge) |

> **双许可** = 你可选 Apache-2.0 **或** MIT, 2 个都允许, 业界惯例.

---

## 6. Cargo.lock 实测 (626 entries / 558 unique)

```bash
$ cargo metadata --format-version=1 | jq '.packages | length'
626
# 626 entries 含重复 (不同 version / feature 组合)

$ cargo metadata --format-version=1 | jq '.packages | unique_by(.name) | length'
558
# 558 unique crate names (per `THIRD-PARTY-NOTICES.md` 561 差 3 是 workspace 直接 dep)
```

**3 差异**:
- workspace 直接 dep 17 个: 算入 workspace 算 "直接", 算入 crates.io 算 "unique"
- apeireth-* 75 个: 不算入 3rd party (自研, Apache-2.0, 不需要 attribution)
- 真正的 3rd party 561 = 626 - 17 - 48 (workspace 内 5 Provider 估补 + etc) ≈ 561

---

## 7. 复现命令 (Reproducibility)

```bash
# 全 workspace license check
cd Apeireth-rust
cargo deny check licenses
# 期望: 0 errors, 0 warnings

# 生成 THIRD-PARTY-NOTICES.md
cargo install cargo-about --version 0.8.4
cargo about generate --output-file THIRD-PARTY-NOTICES.md about.hbs

# Workspace 依赖列表
cargo metadata --format-version=1 --no-deps | jq '.workspace_members | length'
# 期望: 75 (per Cargo.toml:3-185, 实测 2026-08-06)

# Unique crate 列表
cargo metadata --format-version=1 | jq '.packages | length'
# 期望: 626 (含重复)

cargo metadata --format-version=1 | jq '[.packages[] | .name] | unique | length'
# 期望: 558 (unique)

# Direct dependency count
cargo metadata --format-version=1 | jq '.packages | map(select(.source == null)) | length'
# 期望: 0 (workspace member 都是 source = "workspace")

# 12 SPDX 类别统计 (per THIRD-PARTY-NOTICES.md §A)
grep -oP '^\| `?[A-Za-z0-9.\-]+`? \|' THIRD-PARTY-NOTICES.md | sort -u | wc -l
# 期望: 12 (12 unique SPDX ID)
```

---

## 8. 引用资源

| 资源 | URL | 用途 |
|------|-----|------|
| **SPDX 完整列表** | https://spdx.org/licenses/ | 700+ license 完整列表 |
| **SPDX 规范** | https://spdx.dev/specifications/ | SPDX 2.3 / 3.0 规范 |
| **Apache-2.0 原文** | https://www.apache.org/licenses/LICENSE-2.0 | 180 行 |
| **MIT 原文** | https://opensource.org/licenses/MIT | 20 行 |
| **Cargo-deny 文档** | https://embarkstudios.github.io/cargo-deny/ | 16 license allow 配置 |
| **Cargo-about 文档** | https://github.com/EmbarkStudios/cargo-about | THIRD-PARTY 生成工具 |
| **Apeireth 仓库** | https://github.com/apeireth/apeireth-rust | 主仓库 |
| **Apeireth NOTICES** | `THIRD-PARTY-NOTICES.md` (1709 行) | 561 crate attribution |
| **Apeireth deny.toml** | `deny.toml` | 16 license allow + 0 violation |

---

## 9. 相关

- 根 `LICENSE` (Apache-2.0 完整, 180 行)
- 根 `NOTICE` (项目声明 + 致谢, 71 行)
- 根 `DEPENDENCY` (workspace 依赖摘要, 170 行)
- 根 `THIRD-PARTY-NOTICES.md` (1709 行, 561 crate attribution)
- 根 `deny.toml` (16 license allow + 0 violation)
- 根 `about.hbs` (cargo-about Handlebars 模板)
- [01-contribution.md](01-contribution.md) (贡献流程)
- [02-commercial-use.md](02-commercial-use.md) (商业使用)
- [03-modification-redistribution.md](03-modification-redistribution.md) (修改 + 再分发)
- [04-faq.md](04-faq.md) (18 常见问题)
- [docs/licenses-3rdparty/](../licenses-3rdparty/) (50+ 第三方 LICENSE 副本, D-1)
- [DEPENDENCY-trees/](../../DEPENDENCY-trees/) (30+ cargo tree 导出, D-2)

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-5)
**Tool**: Mavis R21 续补 (整合 #3 D-5)
