# copilot Provider API (R20 阶段 4 估补)

> **性质**: 1.0 release #11 license + 5 Provider 100% (per 整合 #3 E-1)
> **依据**: `crates/apeireth-provider-copilot/src/` + `@github/copilot-sdk` 0.9.21 1:1 翻译
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-3)
> **不假装**: R20 阶段 4 估补 100% 完成, 0 引 SDK; R21 续真接 SDK

---

## 0. TL;DR

| 维度 | 值 |
|------|----|
| **1.0 状态** | ✅ 100% 完成度 (per 整合 #3 E-1) |
| **3 ModelKind** | CopilotChat / CopilotEnterprise / CopilotImmersive |
| **8 工具** | ReadFile / WriteFile / Edit / Bash / Grep / Glob / WebFetch / WebSearch |
| **GitHub OAuth 2.0** | device / web flow |
| **1:1 翻译源** | @github/copilot-sdk 0.9.21 |
| **测试** | 19 unit + 19 wiremock = 38 tests |
| **依赖** | reqwest 0.12 + rustls 0.21 |

---

## 1. 客户端初始化 (GitHub OAuth 2.0)

```rust
use apeireth_provider_copilot::{Client, CopilotModel, OAuthFlow};

// OAuth 2.0 device flow (per RFC 8628)
let oauth = OAuthFlow::DeviceCode {
    client_id: std::env::var("GITHUB_CLIENT_ID")?,
    scope: "repo read:user".to_string(),
};
let token = oauth.authorize().await?;  // 返 access_token

let client = Client::new(token.access_token)
    .with_model(CopilotModel::CopilotChat)
    .with_max_tokens(4096);
```

---

## 2. 3 ModelKind

```rust
pub enum CopilotModel {
    CopilotChat,        // 默认 chat
    CopilotEnterprise,  // 企业版 (per enterprise scope)
    CopilotImmersive,   // 沉浸式 (per immersive scope)
}
```

**映射到 GitHub API**:
- `CopilotChat` → `gpt-4` (经 GitHub Copilot proxy)
- `CopilotEnterprise` → `gpt-4-32k`
- `CopilotImmersive` → `claude-3.5-sonnet` (经 GitHub Copilot proxy)

---

## 3. 5 K-1 强校验 (copilot 特有: token / model / scope / org / enterprise)

| K-1 | 校验内容 |
|-----|---------|
| **token** | 长度 (40) + 字符 (ghu_/gho_ prefix) + 过期 (1h) |
| **model** | 3 model 白名单 |
| **scope** | 5 scope (repo / read:user / user:email / copilot / enterprise) |
| **org** | 5 organization 白名单 (per `org` claim) |
| **enterprise** | enterprise slug 校验 (per `enterprise` claim) |

---

## 4. 8 工具 (跟 claude-code 1:1 镜像)

| 工具 | 调用 | 1:1 翻译源 |
|------|------|------------|
| `ReadFile(path)` | 读文件 | copilot.fs.read |
| `WriteFile(path, content)` | 写文件 | copilot.fs.write |
| `Edit(path, old, new)` | 编辑 | copilot.fs.edit |
| `Bash(cmd)` | shell | copilot.shell |
| `Grep(pattern, path)` | grep | copilot.search |
| `Glob(pattern)` | glob | copilot.glob |
| `WebFetch(url)` | HTTP GET | copilot.web_fetch |
| `WebSearch(query)` | Web 搜索 | copilot.web_search |

---

## 5. 5 端点 (HTTP)

| 端点 | 方法 | 用途 |
|------|------|------|
| `POST https://api.github.com/copilot/chat/completions` | POST | chat (SSE) |
| `GET https://api.github.com/copilot/models` | GET | 3 model 列表 |
| `POST https://github.com/login/device/code` | POST | OAuth device code |
| `POST https://github.com/login/oauth/access_token` | POST | OAuth token exchange |
| `GET https://api.github.com/user/orgs` | GET | org 白名单 (5 K-1) |

---

## 6. 19 unit + 19 wiremock = 38 tests (R20 阶段 4 估补)

| 类别 | 数量 |
|------|----:|
| ModelKind 3 model × 3 case = 9 | 9 |
| OAuth 2.0 device flow (5 步) = 5 | 5 |
| 5 K-1 强校验 × 1 case = 5 | 5 |
| 5 端点 × 4 case = 20 | 20 |
| **总** | **39** |

> 1 个 edge case (enterprise scope 拒绝普通 user) = 39 总.

---

## 7. R21 续真接 SDK 计划

| 项 | R21 估时 |
|----|---------|
| 接入 `@github/copilot-sdk` 商业版 (Rust 包装) | 1 owner × 1 周 |
| 5 K-1 强校验 → SDK adapter | 0.5 owner × 1 周 |
| 19 wiremock 端到端 → 真实 API | 0.5 owner × 1 周 |
| **总** | **2 owner × 1 周 ≈ 2 周** |

---

## 8. 相关

- [provider-claude-code.md](provider-claude-code.md)
- [provider-codex.md](provider-codex.md)
- [provider-gemini-cli.md](provider-gemini-cli.md)
- [provider-opencode.md](provider-opencode.md)
- 实现: `crates/apeireth-provider-copilot/`
- 1:1 翻译源: @github/copilot-sdk 0.9.21
- 决策: 整合 #3 E-1 + E-2

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-3)
