# E-2 草稿 — 根 README "## 🏛️ 借鉴" 节

```
[Document-Meta]
Document:       docs/1.0-release-prep/02-borrow.md
Version:        R20-Rev-A
R-Cycle:        R20 阶段 6 — 1.0 release 12 项 #1 doc E-2 续补
Last-Modified:  2026-08-06
Status:         🟢 草稿 (根 README.md LOCKED, 等 Mavis 整合 #3 拍板)
Author:         Mavis (Mavis@local)
Source:         续 reports/1.0-release-doc-30-2026-08-06.md §1.2 E-2
Target:         接手者看到 Apeireth 站在前人肩上 (业界主流 + 商业版 1:1 翻译 + 协议借鉴)
```

> **性质**: 根 README.md **缺"借鉴"节** 草稿 (per 续补报告 §1.2 E-2: DEPENDENCY 170 行 + THIRD-PARTY-NOTICES.md 1709 行已建全, 但 README 0 引).
>
> **本节草稿目标**: 让接手者 **1 跳** 看到 Apeireth 借鉴的 4 层 — (1) 业界主流 crate / (2) 商业版 1:1 翻译 / (3) 协议借鉴 / (4) 测试借鉴, 然后跳 THIRD-PARTY-NOTICES.md 看完整 attribution.
>
> **不假装**: 借鉴列表全部基于 `DEPENDENCY` §2 (workspace 直接依赖) + `THIRD-PARTY-NOTICES.md` §A (561 transitive crates) 实查, 0 编造.

---

## §0. 草稿内容 (建议合入根 README 架构节后)

> **合入位**: 根 README 架构节 (line 263) 后, **新增** 1 个 H2 节 "## 🏛️ 借鉴".

```markdown
## 🏛️ 借鉴 (站在前人肩上)

Apeireth 站在 4 层前人肩上 (per `DEPENDENCY` §2 + `THIRD-PARTY-NOTICES.md` §A 实查 561 crate attribution):

### 1. 业界主流 crate (per `Cargo.toml` [workspace.dependencies])

| 类别 | 借鉴 | 用在哪 | 协议 |
|------|------|--------|------|
| **P0 HTTP / 异步** | `tokio` 1.40 + `reqwest` 0.12 + `axum` 0.7 + `hyper` 1.0 + `tower` 0.4 | async runtime + HTTP client/server | MIT / Apache-2.0 |
| **P0 序列化** | `serde` 1.0 + `serde_json` 1.0 | config / API JSON / DB rows | MIT / Apache-2.0 |
| **P0 错误** | `anyhow` 1.0 + `thiserror` 1.0 | app / lib 错误 | MIT / Apache-2.0 |
| **P0 WS** | `tokio-tungstenite` + `tungstenite` | WebSocket 8 帧协议 (D-03) | MIT / Apache-2.0 |
| **P0 搜索引擎** | `tantivy` 0.22 | 全文搜索 (apeireth-graph) | MIT |
| **P1 时间 / ID** | `chrono` 0.4 + `uuid` 1.10 | 时间 + UUID v4 | MIT / Apache-2.0 |
| **P1 基准** | `criterion` 0.5 | 性能 bench (1.0 release #7 必做) | MIT / Apache-2.0 |
| **P1 属性测试** | `proptest` 1.5 | property-based testing | MIT / Apache-2.0 |
| **P2 Python 桥** | `pyo3` 0.29 | Python 互操作 (apeireth-pybridge) | Apache-2.0 |
| **P2 SQLite** | `rusqlite` 0.32 | memory / vector / api / mcp 4 crate 用 | MIT |
| **P3 工具** | `lru` 0.12 + `shell-words` 1.1 + `fs_err` 3.0 | LRU 缓存 + safe argv + fs error | MIT / BSD-3-Clause |

### 2. 商业版 1:1 翻译 (SpectrAI 前身)

| 借鉴 | 用在哪 | 协议 |
|------|--------|------|
| **SpectrAI 0.9.21** (前身, 2024-2025) | 全部 67 crate 1:1 翻译 (per `docs/stage3-blueprints/borrowed-from-r11.md`) | Apache-2.0 |
| **RIVAL VERSION 蓝图** (`8a643778` commit) | 604 行翻译蓝图, R20 阶段 1 启动 | (内部文档) |
| **v09021-rust-translation-blueprint** | 16 估缺 crate 体检 3🔴 + 2🟡 真实缺口 | (内部文档) |

### 3. 协议 + 工具注册借鉴

| 借鉴 | 用在哪 | 协议 |
|------|--------|------|
| **VCPChat** (Electron 桌面 app, chat-first) | 19 文件分析 (chatCompletionHandler / protocolBridge / toolRegistry), 协议 + 工具注册 + TUI | Apache-2.0 |
| **Yinta fork** (权限分心) | 5 步权限发放 (apeireth-supervisor crate 实现) | Apache-2.0 |
| **Hermes 团队** | 早期 R11 借鉴 (per `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md`) | (内部致敬) |
| **m3-hallucination-defense** | 24 LOCKED crate src/ 0 触碰守门 (per ROADMAP.md §m3 §3) | (内部规范) |
| **cosign (sigstore)** | 8 包 cosign 签名 (1.0 release #3 signature) | Apache-2.0 |

### 4. 测试 + 文档借鉴

| 借鉴 | 用在哪 | 协议 |
|------|--------|------|
| **Keep a Changelog 1.1.0** | 根 CHANGELOG.md 格式 | MIT |
| **MADR 4.0** (Architectural Decision Records) | docs/adr/ 19 文件 (0001 ~ 0027) | CC0-1.0 |
| **semver 2.0** | workspace version 1.0.0 (per APEIRETH-VERSIONING.md) | (CC0-1.0) |
| **RFC 9116** (security.txt) | `.well-known/security.txt` 漏洞报告 | (IETF) |
| **cosign key transparency** | `docs/security/cosign-keys.md` 公钥 + 撤销流程 | (sigstore 规范) |

**完整 561 crate attribution**: 见 [`THIRD-PARTY-NOTICES.md`](./THIRD-PARTY-NOTICES.md) (1709 行, 12 unique SPDX, cargo-deny 0 violation).

**完整 workspace 依赖表**: 见 [`DEPENDENCY`](./DEPENDENCY) (170 行, 17 直接依赖 + 67 成员 crate).
```

---

## §1. 草稿要点 (Mavis 整合 #3 拍板用)

| # | 要点 | 依据 |
|---:|------|------|
| 1 | **4 层借鉴结构**: 业界主流 crate + 商业版 1:1 + 协议借鉴 + 测试/文档借鉴 | per DEPENDENCY §2 + THIRD-PARTY-NOTICES.md §A + ROADMAP.md §R20 阶段 1 |
| 2 | **17 直接依赖 + 561 transitive**: 完整 attribution 不在 README 重复 | DEPENDENCY line 27-47 + THIRD-PARTY-NOTICES.md §B |
| 3 | **SpectrAI 0.9.21 前身 1:1 翻译**: 67 crate 全部来自前身翻译 | per `docs/stage3-blueprints/borrowed-from-r11.md` (LOCKED 阶段 3) |
| 4 | **VCPChat 19 文件分析**: chatCompletionHandler / protocolBridge / toolRegistry | per `docs/v2-strategy/07-VCP-GAP-UPGRADE-PLAN.md` (R17 战役 4 收口) |
| 5 | **cosign (sigstore) 借鉴**: 8 包签名 (1.0 release #3 signature) | per `docs/security/cosign-keys.md` (10364 字节) + commit `bbb26266` |
| 6 | **5 测试/文档规范借鉴**: Keep a Changelog + MADR + semver + RFC 9116 + cosign | 业界主流, 沿用不重造 |
| 7 | **协议完整列 12 SPDX**: MIT / Apache-2.0 / BSD-2/3 / MPL-2.0 / Zlib / CC0-1.0 / ISC / Unicode / 0BSD / Unlicense | per THIRD-PARTY-NOTICES.md §B (130 text variant) |

---

## §2. 守门表

| 守门 | 本草稿 | 验证 |
|------|--------|:----:|
| **0 触碰根 README.md** (LOCKED) | 草稿在本文件, 不动根 README | ✅ |
| **0 触碰根 DEPENDENCY** (1.0 release #11 收口) | 草稿仅引用, 不复制 | ✅ |
| **0 触碰根 THIRD-PARTY-NOTICES.md** (1709 行) | 草稿仅引用, 不复制 | ✅ |
| **0 改 workspace version** | 草稿不动 Cargo.toml | ✅ |
| **6 哲学锚穿透** (S-1/S-2/O-2/O-3/O-4/O-5) | S-1 北极星 (前人肩上) + O-2 走在前人肩上 + S-2 实查 4 层 | ✅ |
| **8 项不修改承诺** | 不假装 + 编译期 hardcode (semver 严守) + 不重复造轮子 (5 测试规范全部业界主流) | ✅ |
| **不依赖 NewAPI** | 草稿全列自建 client, 0 引商业版 SDK | ✅ |
| **诚实标缺** | 借鉴表基于实查 DEPENDENCY §2 (17 直接依赖, 0 编造) | ✅ |

---

## §3. R21 续合入动作

1. 主解除根 README.md LOCKED
2. R21 sub-agent 在根 README line 263 (架构节末) 后**新增** 1 个 H2 "## 🏛️ 借鉴" (per §0 草稿)
3. 估 commit: `docs: R21 续 — 根 README 加"借鉴"节 (per #1 doc 续补 E-2, 4 层借鉴表)`
4. 工时估: 0.5h (新增 H2 + 复刻 §0 草稿)

---

_本草稿路径: `docs/1.0-release-prep/02-borrow.md`_
_生成时间: 2026-08-06_
_续: `reports/1.0-release-doc-30-2026-08-06.md` §1.2 E-2 (根 README 缺"借鉴"节, 估补 1h → 草稿 0.5h, 合入 0.5h)_
_6 哲学锚穿透 + 8 项不修改承诺 0 触碰 + 0 改 workspace version + 0 主动 commit_
