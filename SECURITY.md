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

- **GitHub Security Advisories**: <https://github.com/Apeireth/Apeireth/security/advisories>
- **项目 Release Notes**: <https://github.com/Apeireth/Apeireth/releases>
- **RustSec advisory database**: <https://github.com/RustSec/advisory-db> (即 `cargo-audit`)

## 适用范围 (Scope)

以下组件被认为是"安全边界"，其漏洞属于本政策范围（当前 16-crate 工作区）：

- `apeireth-core`（`crates/foundation/core`）— 稳定域原语 + kernel（IDs/时间/生命周期/事件）；13 键 verdict cache 与洋葱/守门语义（v1 脊椎，接线状态见 ROADMAP P0/P2）
- `apeireth-protocol`（`crates/foundation/protocol`）— 规范化协议 DTO 与 vendor wire 翻译（不含 HTTP client）
- `apeireth-plugin`（`crates/foundation/plugin`）— Plugin/Capability 契约、凭据解析契约、capability 注册唯一权威
- `apeireth-governance`（`crates/foundation/governance`）— Allow/Deny/RequireApproval 决策、PII/注入检测、防篡改审计哈希链
- `apeireth-runtime`（`crates/engine/runtime`）— agent loop 单一执行入口、approval 生命周期、execution trace
- `apeireth-provider`（`crates/engine/provider`）— 供应商凭据解析、认证头构造、wire 适配
- `apeireth-tools-canonical`（`crates/capabilities/tools`）— 进程执行唯一边界（Job Object/进程组）、egress 策略、受控 fetch（DNS 钉扎）
- `apeireth-gateway` / `apeireth-cli`（`crates/adapters/`）— 传输与入口面，不拥有业务逻辑

v1 时代的安全组件（`apeireth-sovereignty`、`apeireth-tool-approval`、`apeireth-bus`、`apeireth-api`、`apeireth-memory/vector` 等）现整体位于 `legacy/`（参考代码，不参与构建），其实现的安全语义将在 ROADMAP §4 对应阶段移植回主链；移植完成前，其漏洞按 v1 政策处理（安全边界仍视为有效）。

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

_Last updated_: 2026-08-27 (reconstruct_v2 收敛后，13-crate 范围重写)
