# gemini-cli Provider API (R20 阶段 4 估补)

> **性质**: 1.0 release #11 license + 5 Provider 100% (per 整合 #3 E-1)
> **依据**: `crates/apeireth-provider-gemini-cli/src/` + `@google/gemini-cli` 0.9.21 1:1 翻译
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-3)
> **不假装**: R20 阶段 4 估补 100% 完成, 0 引 SDK; R21 续真接 SDK

---

## 0. TL;DR

| 维度 | 值 |
|------|----|
| **1.0 状态** | ✅ 100% 完成度 (per 整合 #3 E-1) |
| **3 ModelKind** | Gemini25Pro / Gemini25Flash / Gemini20Flash |
| **8 工具** | ReadFile / WriteFile / Edit / Bash / Grep / Glob / WebFetch / WebSearch |
| **1:1 翻译源** | @google/gemini-cli 0.9.21 |
| **测试** | 19 unit + 19 wiremock = 38 tests |
| **依赖** | reqwest 0.12 + rustls 0.21 (无 OpenSSL) |

---

## 1. 客户端初始化

```rust
use apeireth_provider_gemini_cli::{Client, GeminiModel};

let client = Client::new(
    std::env::var("GOOGLE_API_KEY")?,
)
.with_model(GeminiModel::Gemini25Pro)
.with_max_tokens(8192);
```

---

## 2. 3 ModelKind

```rust
pub enum GeminiModel {
    Gemini25Pro,     // gemini-2.5-pro (2025-01)
    Gemini25Flash,   // gemini-2.5-flash (轻量)
    Gemini20Flash,   // gemini-2.0-flash (legacy 兼容)
}
```

**映射到 Google API**:
- `Gemini25Pro` → `gemini-2.5-pro`
- `Gemini25Flash` → `gemini-2.5-flash`
- `Gemini20Flash` → `gemini-2.0-flash`

---

## 3. 8 工具 (跟 claude-code 1:1 镜像)

| 工具 | 调用 | 1:1 翻译源 |
|------|------|------------|
| `ReadFile(path)` | 读文件 | gemini.file_read |
| `WriteFile(path, content)` | 写文件 | gemini.file_write |
| `Edit(path, old, new)` | 编辑 | gemini.file_edit |
| `Bash(cmd)` | shell | gemini.bash |
| `Grep(pattern, path)` | grep | gemini.search |
| `Glob(pattern)` | glob | gemini.glob |
| `WebFetch(url)` | HTTP GET | gemini.web_fetch |
| `WebSearch(query)` | Web 搜索 | gemini.web_search |

---

## 4. 4 K-1 强校验

| K-1 | 校验内容 |
|-----|---------|
| **token** | 长度 (39) + 字符 (AIza prefix) |
| **model** | 3 model 白名单 |
| **scope** | 5 scope (跟 codex 一致) |
| **region** | 3 region 白名单 (us-central1 / europe-west4 / asia-northeast1) |

---

## 5. 5 端点 (HTTP)

| 端点 | 方法 | 用途 |
|------|------|------|
| `POST /v1beta/models/{model}:generateContent` | POST | text generation |
| `POST /v1beta/models/{model}:streamGenerateContent` | POST | SSE 流式 |
| `POST /v1beta/models/{model}:embedContent` | POST | embedding |
| `GET /v1beta/models` | GET | 3 model 列表 |
| `POST /v1beta/models/{model}:countTokens` | POST | token 计数 |

**鉴权**: `?key=AIzaXXX` query param (跟 Google AI 官方一致)

---

## 6. 19 unit + 19 wiremock = 38 tests (R20 阶段 4 估补)

| 类别 | 数量 |
|------|----:|
| ModelKind 3 model × 3 case = 9 | 9 |
| 5 端点 × 4 case (success / auth fail / rate limit / network error) = 20 | 20 |
| Vision 5 case (image input 测) | 5 |
| 4 区域 region × 1 case = 4 | 4 |
| **总** | **38** |

---

## 7. 5 关键差异 (vs claude-code)

| 维度 | gemini-cli | claude-code |
|------|-----------|------------|
| **vision** | ✅ (image input) | ⚪ (per R21+) |
| **多模态** | ✅ 文本 + 图像 + 音频 | ✅ 文本 + 图像 |
| **function calling** | ✅ 8 工具 | ✅ 8 工具 |
| **stream** | ✅ SSE | ✅ SSE |
| **embedding** | ✅ 768 维 | ⚪ |

---

## 8. R21 续真接 SDK 计划

| 项 | R21 估时 |
|----|---------|
| 接入 `@google/gemini-cli` 商业版 SDK (Rust 包装) | 1 owner × 1 周 |
| 4 K-1 强校验 → SDK adapter | 0.5 owner × 1 周 |
| 19 wiremock 端到端 → 真实 API | 0.5 owner × 1 周 |
| **总** | **2 owner × 1 周 ≈ 2 周** |

---

## 9. 相关

- [provider-claude-code.md](provider-claude-code.md)
- [provider-codex.md](provider-codex.md)
- [provider-copilot.md](provider-copilot.md)
- [provider-opencode.md](provider-opencode.md)
- 实现: `crates/apeireth-provider-gemini-cli/`
- 1:1 翻译源: @google/gemini-cli 0.9.21
- 决策: 整合 #3 E-1 + E-2

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-3)
