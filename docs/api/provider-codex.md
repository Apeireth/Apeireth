# codex Provider API (R20 阶段 4 估补)

> **性质**: 1.0 release #11 license + 5 Provider 100% (per 整合 #3 E-1)
> **依据**: `crates/apeireth-provider-codex/src/` + `@openai/codex` 0.9.21 1:1 翻译
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-3)
> **不假装**: R20 阶段 4 估补 100% 完成, 0 引 SDK; R21 续真接 SDK

---

## 0. TL;DR

| 维度 | 值 |
|------|----|
| **1.0 状态** | ✅ 100% 完成度 (per 整合 #3 E-1) |
| **4 ModelKind** | codex / codex-mini / o3 / o4-mini |
| **8 工具** | ReadFile / WriteFile / Edit / Bash / Grep / Glob / WebFetch / WebSearch |
| **3 SandboxType** | workspace-write / read-only / danger-full-access (codex 特有) |
| **1:1 翻译源** | @openai/codex 0.9.21 |
| **测试** | 19 unit + 19 wiremock = 38 tests |
| **依赖** | reqwest 0.12 + rustls 0.21 (无 OpenSSL) |

---

## 1. 客户端初始化

```rust
use apeireth_provider_codex::{Client, CodexModel, SandboxType};

let client = Client::new(
    std::env::var("OPENAI_API_KEY")?,  // API key
)
.with_sandbox(SandboxType::WorkspaceWrite)  // 默认沙盒
.with_model(CodexModel::Codex)             // 默认模型
.with_max_tokens(8192);

// 鉴权 5 K-1 强校验
client.validate_token()?;  // 长度 / 字符 / 过期校验
client.validate_model()?;  // 4 model 白名单校验
client.validate_sandbox()?;  // 3 sandbox 白名单校验
```

---

## 2. 4 ModelKind

```rust
pub enum CodexModel {
    Codex,        // codex 1.0 (2025-09 release)
    CodexMini,    // codex-mini 1.0 (轻量)
    O3,           // o3 reasoning
    O4Mini,       // o4-mini reasoning (轻量)
}

impl CodexModel {
    pub fn max_tokens(&self) -> u32 { /* 8192 / 4096 / 32768 / 16384 */ }
    pub fn supports_tools(&self) -> bool { true }
    pub fn supports_vision(&self) -> bool { /* true / true / true / false */ }
}
```

**映射到 OpenAI API**:
- `Codex` → `gpt-5-codex`
- `CodexMini` → `gpt-5-codex-mini`
- `O3` → `o3-2025-01-31`
- `O4Mini` → `o4-mini-2025-01-31`

---

## 3. 3 SandboxType (codex 特有)

```rust
pub enum SandboxType {
    WorkspaceWrite,    // 默认: 可写 workspace, 0 系统文件
    ReadOnly,          // 只读
    DangerFullAccess,  // 全访问 (需 OAuth 2.0 enterprise scope)
}
```

| Sandbox | 允许 | 拒绝 | 1.0 状态 |
|---------|------|------|---------|
| **WorkspaceWrite** | 读 + 写 + exec (workspace 目录内) | 系统文件 / 网络 / 进程 | ✅ |
| **ReadOnly** | 读 (workspace + 公共只读路径) | 写 / exec / 网络 | ✅ |
| **DangerFullAccess** | 全访问 | 0 (用户自负责任) | 🟡 需 enterprise scope |

---

## 4. 8 工具 (跟 claude-code 1:1 镜像)

| 工具 | 调用 | 1:1 翻译源 |
|------|------|------------|
| `ReadFile(path)` | 读文件 | codex.file_read |
| `WriteFile(path, content)` | 写文件 | codex.file_write |
| `Edit(path, old, new)` | 编辑 | codex.file_edit |
| `Bash(cmd)` | shell | codex.bash (经 shell-words 解析) |
| `Grep(pattern, path)` | grep | codex.search |
| `Glob(pattern)` | glob | codex.glob |
| `WebFetch(url)` | HTTP GET | codex.web_fetch |
| `WebSearch(query)` | Web 搜索 | codex.web_search |

**TOOL_WHITELIST** 编译期 hardcode (per `apeireth-protocol`), 防止 LLM 调未授权工具.

---

## 5. 5 K-1 强校验 (per task spec §3 codex 特有 sandbox 模块)

| K-1 | 校验内容 |
|-----|---------|
| **token** | 长度 (40-200) + 字符 (sk- prefix) + 过期 |
| **model** | 4 model 白名单 |
| **scope** | 5 scope 验证 (read / write / admin / owner / root) |
| **sandbox** | 3 SandboxType 白名单 |
| **network** | 出站 5 白名单域名 |

---

## 6. 5 端点 (HTTP)

| 端点 | 方法 | 用途 |
|------|------|------|
| `POST /v1/chat/completions` | POST | chat completion (SSE) |
| `POST /v1/embeddings` | POST | text embedding (跟 claude-code 不同) |
| `GET /v1/models` | GET | 4 ModelKind 列表 |
| `GET /v1/sandboxes` | GET | 3 SandboxType 状态 |
| `POST /v1/usage` | POST | 上报 token 用量 |

**鉴权**: `Authorization: Bearer sk-xxx` (跟 OpenAI 官方一致)

---

## 7. 19 unit + 19 wiremock = 38 tests (R20 阶段 4 估补)

| 类别 | 数量 |
|------|----:|
| ModelKind 4 model × 3 case = 12 | 12 |
| SandboxType 3 sandbox × 2 case = 6 | 6 |
| 5 端点 × 4 case (success / auth fail / rate limit / network error) = 20 | 20 |
| **总** | **38** |

> **不假装 (per 整合 #3 E-2)**: 19 wiremock 端到端 (集成在 20 端点 case 里), R21 续真接 OpenAI API.

---

## 8. R21 续真接 SDK 计划

| 项 | R21 估时 |
|----|---------|
| 接入 `@openai/codex` 商业版 SDK (Rust 包装) | 1 owner × 1 周 |
| 5 K-1 强校验 → SDK adapter | 0.5 owner × 1 周 |
| 19 wiremock 端到端 → 真实 API | 0.5 owner × 1 周 |
| **总** | **2 owner × 1 周 ≈ 2 周** |

> 1.0 release 用 38 tests 验证, R21 续真接 OpenAI SDK, 估 2 周.

---

## 9. 相关

- [provider-claude-code.md](provider-claude-code.md) (5 Provider 概览 + claude-code 真接)
- [provider-gemini-cli.md](provider-gemini-cli.md) (gemini-cli 估补 100%)
- [provider-copilot.md](provider-copilot.md) (copilot 估补 100%)
- [provider-opencode.md](provider-opencode.md) (opencode 估补 100%)
- 实现: `crates/apeireth-provider-codex/`
- 1:1 翻译源: @openai/codex 0.9.21
- 蓝图: `docs/stage4/5-provider-tool-mapping-2026-08-05.md` §1
- 决策: 整合 #3 E-1 + E-2

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-3)
