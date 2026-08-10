# Security Policy

## 报告安全问题 (Report a security issue)

Apeireth 团队欢迎安全报告，并承诺及时处理安全问题。

**请联系**: apeireth-security@apeireth.org (private disclosure)
**不要**通过公开 GitHub Issue tracker 报告安全问题。

## 漏洞协调 (Vulnerability coordination)

漏洞修复由项目团队优先处理。我们通过 [GitHub Security Advisories](https://help.github.com/en/code-security/security-advisories/working-with-global-security-advisories-from-the-tooling-side) 协调修复，第三方利益相关方包括：

- **漏洞报告者**（原始发现者）
- **直接 / 间接受影响用户**（Apeireth 部署者）
- **上游依赖维护者**（如 tokio / reqwest / serde_json / pyo3 等关键 crate）

下游项目维护者 / Apeireth 用户可通过发送邮箱 + GitHub username + 相关背景信息到 apeireth-security@apeireth.org 申请参与漏洞协调。参与权限由 Apeireth 团队决定。

## 安全公告 (Security advisories)

Apeireth 团队承诺漏洞披露过程透明，通过以下渠道公告：

- **GitHub Security Advisories**: <https://github.com/apeireth/apeireth-rust/security/advisories>
- **项目 Release Notes**: <https://github.com/apeireth/apeireth-rust/releases>
- **RustSec advisory database**: <https://github.com/RustSec/advisory-db> (即 `cargo-audit`)

## 适用范围 (Scope)

以下组件被认为是"安全边界"，其漏洞属于本政策范围：

- `apeireth-core` (L0 HA 核心 + 12 键 + Self-Disable 5 大机制 + 双洋葱统一体)
- `apeireth-sovereignty` (HumanAuthority + MultiHuman M-of-N + 三域分离 BCD + Physical Multisig)
- `apeireth-tool-approval` (5 规则工具审批 + fuzzy matching)
- `apeireth-bus` (5 层总线的权限隔离)
- `apeireth-api` / `apeireth-protocol` (4 LLM 协议归一化 — 含 token / key 处理)
- `apeireth-memory` / `apeireth-vector` (持久化的用户画像 / 长期记忆 — 可能含敏感数据)

**不在范围**: 业务逻辑 bug (非安全), 性能问题, doc typo, 等等. 这些走普通 GitHub Issue.

## 响应时间承诺 (Response time SLA)

| 严重程度 | 首次响应 | 修复目标 |
|---|---|---|
| **Critical** (远程代码执行 / L0 HA bypass) | < 24 小时 | < 7 天 |
| **High** (权限提升 / Self-Disable 绕过) | < 48 小时 | < 30 天 |
| **Medium** (信息泄露 / DoS) | < 1 周 | < 90 天 |
| **Low** (最佳实践违反 / 文档错误) | < 1 月 | 下一次 release |

## 披露政策 (Disclosure policy)

我们采用 **coordinated disclosure** (90 天默认窗口)：

1. 收到报告 → 24h 内确认
2. 私有修复 → 协调报告者验证
3. CVE 申请 (如需) → 联系 MITRE
4. 90 天后（或修复 ready 后） → 公开公告 + Release
5. 安全更新通过 `cargo audit` 自动告警（已配 `.github/workflows/cargo-deny.yml`）

## 参考业界 (References)

本政策参照以下项目的安全政策：
- [tokio/SECURITY.md](https://github.com/tokio-rs/tokio/blob/master/SECURITY.md) — Rust 异步运行时事实标准
- [wasmtime/SECURITY.md](https://github.com/bytecodealliance/wasmtime/blob/main/SECURITY.md) — Bytecode Alliance
- [Rust Security Advisory Working Group](https://github.com/rustsec/advisory-db) — RustSec 标准
- [GitHub Security Advisories 文档](https://docs.github.com/en/code-security/security-advisories)

---

_Last updated_: 2026-08-05 (R18 第 0 阶段第 5 项)
