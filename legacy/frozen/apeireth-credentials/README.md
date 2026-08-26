# apeireth-credentials

> **R20 阶段 6 估缺: 多 provider 凭证 skeleton (1:1 翻译 v0.9.21 `@anthropic-ai/credentials` 商业版)**
>
> 5 Provider (Anthropic / OpenAI / Google / Azure / Local) + 5 鉴权 (API key / OAuth / JWT / IAM / mTLS)
> + 4 轮换策略 (manual / time / count / hybrid) + 5 Scope (read/write/admin/owner/root) + audit 4 事件 (get/put/rotate/revoke).
>
> ⚠️ **STUB MODE**: 当前 crate 是 **skeleton** — API 表面按 v0.9.21 商业版 `out/main` 集成面 1:1 翻译,
> 但所有 5 Provider 实现都是 `Err(CredentialsError::NotImplemented)`. **0 真接商业版 credentials SDK**,
> 留 R21+ 续真接.

## 状态: ⏳ skeleton (R20 阶段 6 实施)

- ✅ API 表面 1:1 翻译 v0.9.21 `@anthropic-ai/credentials`
- ✅ 5 Provider trait 实现 (stub 返 `NotImplemented`)
- ✅ 5 鉴权方式 (API key / OAuth 2.0 / JWT / IAM / mTLS)
- ✅ 4 轮换策略 (manual / time / count / hybrid)
- ✅ 5 Scope 级别 (read / write / admin / owner / root)
- ✅ Token 管理 (get / refresh / revoke / is_valid / expires_at)
- ✅ Audit 4 事件 (get / put / rotate / revoke)
- ✅ 25+ 集成测试 (5 Provider 各 1 + 5 鉴权 K-1 + 4 轮换 + 5 Scope + 6 集成)
- ⏳ R21+ 续真接商业版 credentials SDK

## 5 Provider (per v0.9.21 商业版 1:1 翻译)

| # | Provider | 鉴权方式 | 头/字段 | 商业版源 |
|---|---------|---------|--------|---------|
| 1 | `Anthropic` | API key | `x-api-key: <key>` | `@anthropic-ai/sdk` |
| 2 | `OpenAI` | API key | `Authorization: Bearer <key>` | `openai` SDK |
| 3 | `Google` | OAuth 2.0 + API key | `Authorization: Bearer <oauth>` | `@google/generative-ai` |
| 4 | `Azure` | API key | `api-key: <key>` | `@azure/openai` |
| 5 | `Local` | custom header | `X-Apeireth-Token: <token>` | self-hosted |

## 5 鉴权方式 (per RFC 6749 / RFC 7519 / RFC 8705)

| # | 方式 | 关键字段 | 1:1 翻译 |
|---|------|---------|---------|
| 1 | `ApiKey` | `api_key: String` | static 静态凭证 |
| 2 | `OAuth2` | `client_id / client_secret / refresh_token / access_token` | RFC 6749 |
| 3 | `Jwt` | `token / audience / issuer` | RFC 7519 (signed JWT) |
| 4 | `Iam` | `role_arn / session_name / region` | AWS IAM / GCP IAM / Azure AD |
| 5 | `Mtls` | `cert_path / key_path / ca_path` | RFC 8705 (mutual TLS) |

## 4 轮换策略 (per OWASP 2023 密钥管理指南)

| # | 策略 | 触发条件 | 默认值 |
|---|------|---------|--------|
| 1 | `Manual` | admin 手动 | 0 (关闭) |
| 2 | `Time` | 每 N 天 | 30 天 |
| 3 | `Count` | 每 N 次使用 | 1000 次 |
| 4 | `Hybrid` | time + count 任意 | 30 天 OR 1000 次 |

## 5 Scope 级别 (per RBAC 行业标准)

| # | Scope | 权限 | 示例 |
|---|-------|------|------|
| 1 | `Read` | 只读 | list / get |
| 2 | `Write` | 写 | create / update |
| 3 | `Admin` | 管理 | delete |
| 4 | `Owner` | 所有者 | transfer |
| 5 | `Root` | 全权 | account delete |

## 引用文档

1. `.openclaw\workspace\promethean\Apeireth-rust\docs\stage4\v09021-rust-translation-blueprint-2026-08-05.md` (RIVAL §2.x credentials 部分)
2. `.minimax-agent-cn\spectrai\commercial-nsis\v0901\app-64\app-extracted\out\main\chunks\credentials-*.js` (v0.9.21 1:1 翻译源)
3. `.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-keyring\src\lib.rs` (凭证存储参考, P0 安全铁律)

## P0 安全铁律 (跟 keyring 一致)

1. **凭证绝不存明文** — 当前 skeleton 不存凭证, 留 R21+ 真接时 100% 走 keyring
2. **5 K-1 强校验** — 5 鉴权各自空值校验, 编译期 hardcode
3. **0 假装已对接** — 所有 Provider stub 返 `Err(CredentialsError::NotImplemented)`, 警告日志
4. **4 audit 事件必记录** — get / put / rotate / revoke 不可绕过
5. **scope 不可越权** — read 不能 write, write 不能 admin, 等

## 6 哲学 anchor

- **S-1 北极星导向**: 1:1 翻译 v0.9.21 `@anthropic-ai/credentials` 商业版, 0 业务重设计
- **S-2 实事求是**: 估 800-1000 LOC, skeleton 阶段 ~700 LOC, R21+ 续真接
- **O-2 走在前人肩上**: 借鉴 RFC 6749 (OAuth 2.0) / RFC 7519 (JWT) / RFC 8705 (mTLS) 工业标准
- **O-3 干到底**: 5 Provider × 5 鉴权 × 4 轮换 × 5 Scope × 4 audit = 8 工具 + 25+ 测试
- **O-4 任何人都能接手**: 跟 keyring / i18n / voice 同骨架, 引用 v0.9.21 路径完整
- **O-5 不假装**: 所有 stub 返 `NotImplemented` + `warn!` 日志, 0 假装已接

## 8 项不修改承诺

1. **0 触碰 24 LOCKED crate**
2. **0 改 workspace version / Cargo.toml 其他字段**
3. **0 写真实凭证 (key / secret / token)**
4. **0 改 K-1 强校验守门 (5 鉴权空值校验必保留)**
5. **0 改 8 工具白名单**
6. **0 改 5 Provider 枚举**
7. **0 改 5 Scope 顺序**
8. **0 改 4 轮换策略**

## 运行

```bash
# 编译期检查
cargo check -p apeireth-credentials

# 跑所有 25+ 集成测试
cargo test -p apeireth-credentials

# 跑 demo
cargo run -p apeireth-credentials --example credentials_demo
```

## 文件清单

```
crates/apeireth-credentials/
├── Cargo.toml                                 (显式 version, 整合时改 workspace)
├── README.md                                  (本文件)
├── src/
│   ├── lib.rs                                 (主入口, 700+ 行)
│   ├── error.rs                               (CredentialsError, 10+ 错误)
│   ├── provider.rs                            (Provider trait + 5 实现)
│   ├── auth.rs                                (5 鉴权方式)
│   ├── token.rs                               (TokenManager trait)
│   ├── scope.rs                               (5 Scope 级别)
│   ├── rotation.rs                            (4 轮换策略)
│   └── audit.rs                               (AuditEvent + 4 事件)
├── tests/
│   └── test_credentials_in_process.rs         (25+ 集成测试)
└── examples/
    └── credentials_demo.rs                    (5 provider 切换 demo)
```
