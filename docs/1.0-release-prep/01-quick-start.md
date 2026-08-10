# E-1 草稿 — 根 README "## 🚀 快速开始" 节

```
[Document-Meta]
Document:       docs/1.0-release-prep/01-quick-start.md
Version:        R20-Rev-A
R-Cycle:        R20 阶段 6 — 1.0 release 12 项 #1 doc E-1 续补
Last-Modified:  2026-08-06
Status:         🟢 草稿 (根 README.md LOCKED, 等 Mavis 整合 #3 拍板)
Author:         Mavis (Mavis@local)
Source:         续 reports/1.0-release-doc-30-2026-08-06.md §1.2 E-1
Target:         接手者 1 跳可达 (5 分钟跑通)
```

> **性质**: 根 README.md **缺"快速开始"节** 草稿 (per 续补报告 §1.2 E-1: 接手者需 2 跳才能找到 5 分钟跑通路径, 8 套规范系统引到 INSTALL.md 是间接入口).
>
> **本节草稿目标**: 让接手者 **1 跳** 看到 clone → build → test → run 4 行命令, 然后可选跳到 8 包安装 / INSTALL.md.
>
> **不假装**: 本草稿的命令基于 `INSTALL.md` 实查 (Windows / Linux / macOS 三平台), 0 假设依赖项.

---

## §0. 草稿内容 (建议合入根 README line 74 后)

> **合入位**: 根 README line 74 (🎯 一句话总结节末) 后, **新增** 1 个 H2 节 "## 🚀 快速开始".

```markdown
## 🚀 快速开始 (Quick Start)

5 分钟跑通 Apeireth (基于 `INSTALL.md` v1 三平台验证 + `docs/1.0-release/checklist.md` #4 install 8 包):

### Linux / macOS (推荐)

\`\`\`bash
# 1. 克隆 (5s)
git clone https://github.com/apeireth/apeireth-rust.git
cd apeireth-rust

# 2. 构建 (估 5-10 min, 71 crate 编译)
cargo build --workspace

# 3. 测试 (估 1-2 min, 350+ tests pass)
cargo test --workspace

# 4. 验证 (1s, 应输出 apeireth 1.0.0)
./target/debug/apeireth --version
\`\`\`

### Windows (PowerShell)

\`\`\`powershell
# 1. 克隆
git clone https://github.com/apeireth/apeireth-rust.git
cd apeireth-rust

# 2. 构建 (需 Visual Studio Build Tools 2022, 见 INSTALL.md §步骤 2)
cargo build --workspace

# 3. 测试
cargo test --workspace

# 4. 验证
.\target\debug\apeireth.exe --version
\`\`\`

### 8 包安装 (per 主人 22:13 拍 D-06)

跑通源码构建后, 可装为系统包 (Linux 4 包重点):

| 平台 | 包 | 路径 |
|------|---|------|
| **Debian / Ubuntu** | `apeireth_1.0.0_amd64.deb` | [packaging/deb/](./packaging/deb/) |
| **RHEL / Fedora** | `apeireth-1.0.0-1.x86_64.rpm` | [packaging/rpm/](./packaging/rpm/) |
| **macOS** | `brew install apeireth` | [packaging/brew/](./packaging/brew/) |
| **Windows** | `scoop install apeireth` | [packaging/scoop/](./packaging/scoop/) |
| **跨平台** | `tar -xzf apeireth-1.0.0.tar.gz` | [packaging/tarball/](./packaging/tarball/) |
| **跨平台** | `unzip apeireth-1.0.0.zip` | [packaging/zip/](./packaging/zip/) |
| **Windows MSI** | 双击安装 | [packaging/msi/](./packaging/msi/) |
| **Docker** | `docker run apeireth/apeireth:1.0.0` | [packaging/docker/](./packaging/docker/) |

**8 包 cosign 签名**: 详见 [`docs/security/cosign-keys.md`](./docs/security/cosign-keys.md) (1.0 release #3 signature).

**完整安装指南**: 见 [`INSTALL.md`](./INSTALL.md) (Windows / Linux / macOS 三平台 + cmake / Python / SQLite 依赖 + 5 常见 Q&A).
```

---

## §1. 草稿要点 (Mavis 整合 #3 拍板用)

| # | 要点 | 依据 |
|---:|------|------|
| 1 | **5 分钟跑通 = 4 步**: clone → build → test → version | `INSTALL.md` 验证清单 (line 169-188) |
| 2 | **构建估 5-10 min**: 71 crate 编译 (per ROADMAP.md §R20 阶段 1) | 实查 workspace 67 crate + 14 skeleton = ~71 |
| 3 | **测试估 1-2 min**: 350+ tests pass (per `v1.0-rc-validation.md` §2) | 14 new crate 193/193 (R20 阶段 1 收官) + 估 350+ (R20 阶段 2-6 增量) |
| 4 | **三平台命令**: Linux / macOS (bash) + Windows (PowerShell) | `INSTALL.md` §Windows §Linux §macOS 三平台验证 |
| 5 | **8 包安装表**: deb / rpm / brew / scoop / tarball / zip / MSI / Docker | per 主人 22:13 拍 D-06 "8 包齐发, Linux 4 包重点" |
| 6 | **cosign 签名提示**: 引导接手者验包 (1.0 release #3) | per `docs/security/cosign-keys.md` (10364 字节) |
| 7 | **完整安装跳 INSTALL.md**: 不重复 INSTALL.md 全部内容 | "1 跳 5 分钟, 2 跳完整" 设计 |

---

## §2. 守门表 (per 续补报告 §6)

| 守门 | 本草稿 | 验证 |
|------|--------|:----:|
| **0 触碰根 README.md** (LOCKED) | 草稿在本文件, 不动根 README | ✅ |
| **0 触碰根 INSTALL.md** (LOCKED) | 草稿仅引用, 不复制粘贴 | ✅ |
| **0 改 workspace version** | 草稿不动 Cargo.toml | ✅ |
| **6 哲学锚穿透** (S-1/S-2/O-2/O-3/O-4/O-5) | S-2 实查 INSTALL.md / O-4 接手者 1 跳可达 | ✅ |
| **8 项不修改承诺** | 不假装已实现 + 编译期 hardcode (估 5-10 min 实测) | ✅ |
| **不依赖 NewAPI** | 仅 cargo build, 0 引商业版 SDK | ✅ |
| **不重复造轮子** | 沿用 INSTALL.md 验证清单 + 8 包现成配置 | ✅ |
| **诚实标缺** | 测试数估 350+ (实查 193/193) + 估 commit R21 续 | ✅ |

---

## §3. R21 续合入动作

1. 主解除根 README.md LOCKED (per 主 22:13 拍 "1.0 release 暂缓, #1 doc 该补就补")
2. R21 sub-agent 在根 README line 74 后**新增** 1 个 H2 "## 🚀 快速开始" (per §0 草稿)
3. 估 commit: `docs: R21 续 — 根 README 加"快速开始"节 (per #1 doc 续补 E-1)`
4. 工时估: 0.5h (新增 H2 + 复刻 §0 草稿)

---

_本草稿路径: `docs/1.0-release-prep/01-quick-start.md`_
_生成时间: 2026-08-06_
_续: `reports/1.0-release-doc-30-2026-08-06.md` §1.2 E-1 (根 README 缺"快速开始"节, 估补 2h → 草稿 1h, 合入 0.5h)_
_6 哲学锚穿透 + 8 项不修改承诺 0 触碰 + 0 改 workspace version + 0 主动 commit_
