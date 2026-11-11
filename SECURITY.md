# Security Policy

## 报告安全问题 (Report a security issue)

Apeireth 团队欢迎负责任的安全报告。请使用 **[GitHub 私密漏洞报告](https://github.com/Apeireth/Apeireth/security/advisories/new)**（需要 GitHub 账号；2026-11-11 已核验仓库开启此功能）。

**不要**通过公开 Issue 或 Discussion 发布漏洞细节、可利用代码、密钥或私人数据。报告可包含受影响版本或 commit、运行环境、预期与实际行为、最小复现和可能影响；不确定的部分如实说明，附件先脱敏。

旧邮箱 `apeireth-security@apeireth.org` 尚未启用，请勿向它发送报告。社区行为举报与申诉请私密联系 [yintasde@qq.com](mailto:yintasde@qq.com)，处理规则见 [行为准则](CODE_OF_CONDUCT.md#报告与申诉渠道)；产品安全漏洞仍优先使用上方 GitHub 私密漏洞报告。普通使用问题见 [SUPPORT.md](SUPPORT.md)。

## 漏洞协调 (Vulnerability coordination)

漏洞修复由项目团队优先处理。我们通过仓库的私密 Security Advisory 对话协调修复；按需要、在限制披露范围的前提下邀请相关方参与，包括：

- **漏洞报告者**（原始发现者）
- **直接 / 间接受影响用户**（Apeireth 部署者）
- **上游依赖维护者**（如 tokio / reqwest / serde_json / pyo3 等关键 crate）

报告者可在私密报告中补充受影响的下游项目及需要参与协调的 GitHub 账号。维护者核实后按需邀请协作者，不要求公开个人背景或额外敏感信息。

## 安全公告 (Security advisories)

Apeireth 团队承诺漏洞披露过程透明，通过以下渠道公告：

- **GitHub Security Advisories**: <https://github.com/Apeireth/Apeireth/security/advisories>
- **项目 Release Notes**: <https://github.com/Apeireth/Apeireth/releases>
- **RustSec advisory database**: 对符合收录条件的 Rust 包漏洞，视情况向 <https://github.com/RustSec/advisory-db> 提交公告；不保证每项项目修复都会被收录。

## 适用范围 (Scope)

以下组件被认为是"安全边界"，其漏洞属于本政策范围（组件与 workspace 成员以当前 `Cargo.toml` 及 [crates 索引](docs/03-reference/crates.md) 为准）：

> **独立 workspace 备案（如实）**：另有**独立** Cargo workspace `crates-ui/`（**不计入**根 workspace、与根 workspace 无依赖边），含 **1 个 crate** `apeireth-ui-core`（纯响应式内核，0 外部依赖）；范围 = IMPLEMENTED、**尚无生产前端消费点**（ADR-0021，owner 2026-10-29；自带独立 CI `ui-core-ci.yml`）⇒ **不在本政策上述安全边界范围内**（不随主产品构建/分发），此处仅作范围备案。

- `apeireth-core`（`crates/foundation/core`）— 稳定域原语 + kernel（IDs/时间/生命周期/事件）；键集（现行 14 键）verdict cache 与洋葱/守门语义（v1 脊椎，接线状态见 ROADMAP P0/P2）
- `apeireth-protocol`（`crates/foundation/protocol`）— 规范化协议 DTO 与 vendor wire 翻译（不含 HTTP client）
- `apeireth-plugin`（`crates/foundation/plugin`）— Plugin/Capability 契约、凭据解析契约、capability 注册唯一权威
- `apeireth-governance`（`crates/foundation/governance`）— Allow/Deny/RequireApproval 决策、PII/注入检测、防篡改审计哈希链
- `apeireth-guard`（`crates/engine/guard`）— 行为链安全 Guard：两阶段行为链分类、决策融合、命令效果与会话行为分析，经 `BehaviorChainGuardHook` 接生产治理管线；威胁建模与误判降级语义属本政策范围
- `apeireth-runtime`（`crates/engine/runtime`）— agent loop 单一执行入口、approval 生命周期、execution trace
- `apeireth-provider`（`crates/engine/provider`）— 供应商凭据解析、认证头构造、wire 适配
- `apeireth-tools-canonical`（`crates/capabilities/tools`）— 进程执行唯一边界（Job Object/进程组）、egress 策略、受控 fetch（DNS 钉扎）
- `apeireth-gateway` / `apeireth-cli`（`crates/adapters/`）— 传输与入口面，不拥有业务逻辑

v1 时代的安全组件（`apeireth-sovereignty`、`apeireth-tool-approval`、`apeireth-bus`、`apeireth-api`、`apeireth-memory/vector` 等）现整体位于 `legacy/`（参考代码，不参与构建），其实现的安全语义将在 ROADMAP §4 对应阶段迁回主链；迁回完成前，其漏洞按 v1 政策处理（安全边界仍视为有效）。

**不在范围**: 业务逻辑 bug (非安全), 性能问题, doc typo, 等等. 这些走普通 GitHub Issue.

## 响应与优先级 (Response and priority)

维护者会优先处理正在被利用、远程代码执行、权限绕过和敏感数据泄露等风险，并在私密报告中沟通确认结果、临时缓解方式和后续安排。当前尚无经过确认的全天候值守与固定修复 SLA，因此不承诺所有报告在 24/48 小时内响应或在固定天数内修复。

未收到回复时，可在同一私密报告中补充提醒，避免将细节转贴到公开 Issue。是否接受报告、严重程度与修复范围由实际复现和影响评估决定，不因报告暂不完整而要求编造证据。

## 披露政策 (Disclosure policy)

我们采用 **coordinated disclosure**（协调披露）：

1. 在私密报告中确认受影响版本、复现条件和风险。
2. 讨论缓解措施、修复计划与披露时间；如有变化，继续向报告者说明。
3. 私下修复并邀请报告者协助验证；需要时协调申请 CVE。
4. 在约定时间发布公告、受影响版本、修复版本或缓解办法；致谢前征得报告者同意。
5. `cargo audit` 等工具只检查其采用的漏洞数据库，不能替代关注项目公告，也不保证自动提示所有 Apeireth 安全更新。

## 参考业界 (References)

本政策参照以下项目的安全政策：
- [tokio/SECURITY.md](https://github.com/tokio-rs/tokio/blob/master/SECURITY.md) — Rust 异步运行时事实标准
- [wasmtime/SECURITY.md](https://github.com/bytecodealliance/wasmtime/blob/main/SECURITY.md) — Bytecode Alliance
- [Rust Security Advisory Working Group](https://github.com/rustsec/advisory-db) — RustSec 标准
- [GitHub Security Advisories 文档](https://docs.github.com/en/code-security/security-advisories)

---

_Last updated_: 2026-10-18 (文档审计批：crate 口径更新为 workspace 31 members（30 crates + bench harness；image 家族 +11）；测试基线只引单一真源 [docs/03-reference/test-baseline.md](docs/03-reference/test-baseline.md) 不抄写数字；`apeireth-guard` 安全边界 2026-10-06 已核验；workspace.version 2.0.0-rc.4、仓库 URL `Apeireth/Apeireth` 不变)

> **2026-11-04 口径更正（文档真源批）**：上一行的「31 members（30 crates + bench harness）」是
> 2026-10-18 当时的**历史口径**（R4b 归档 `apeireth-perception` 之前），已过期 ⇒
> **现行 = 30 members（29 crates + bench harness）**，与本文「适用范围」段一致；
> 历史口径不改写、只在此标注。实测：`cargo metadata --no-deps` = 30 包（29 产品 crate + `apeireth-bench-harness`）。

> **2026-11-11 更新**：以上 2026-11-04 更正保留为历史记录，不再作为当前成员数量声明；现行成员清单以根 `Cargo.toml` 为准。本次更新核验私密漏洞入口，纠正未启用邮箱、响应目标与披露流程，不改变运行时安全边界。
