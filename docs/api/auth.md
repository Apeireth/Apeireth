# Apeireth API 鉴权规范（5 组件 token 体系）

> **性质**: v1 API 鉴权设计 + 5 组件实现细节
> **依据**: `crates/apeireth-api/src/auth.rs` + D-03 主人拍板（链接 token 5min TTL）
> **最后更新**: 2026-08-05

---

## 1. 5 鉴权组件

| 组件 | 字段 | 用途 | 生命周期 |
|---|---|---|---|
| **access_token** | `Authorization: Bearer <token>` | REST 鉴权（短命） | 15 min |
| **refresh_token** | `/v1/auth/refresh` body | 续 access_token（长命） | 7 d |
| **scope** | token 签发时绑定的权限集 | 工具调用白名单 | 跟随 token |
| **expire** | `exp` claim（Unix epoch） | 过期校验 | 跟随 token |
| **refresh-on-use** | 自动 refresh 中间件 | 续命机制（per D-03 拍板） | 服务端 |

---

## 2. Token 形状

```json
{
  "sub": "user-uuid",
  "scope": ["calendar:read", "calendar:write", "message:send", "search:query"],
  "iat": 1754438400,
  "exp": 1754439300,
  "iss": "apeireth-api",
  "aud": "apeireth-tools"
}
```

- **签名**: HS256（HMAC-SHA256）
- **密钥**: `APEIRETH_JWT_SECRET` 环境变量（启动时校验非空，缺 = 启动失败，编译期 hardcode 拒空）
- **算法**: 不支持 `none`，服务端强制 `alg` 校验

---

## 3. REST 鉴权流程

```
Client                              Apeireth API
  |                                       |
  |  POST /v1/auth/login (用户名+密码)    |
  |-------------------------------------->|
  |                                       |  校验 → 签发 access + refresh
  |  200 { access_token, refresh_token,   |
  |       expires_in: 900 }               |
  |<--------------------------------------|
  |                                       |
  |  GET /v1/tools/calendar/invoke         |
  |  Authorization: Bearer <access_token> |
  |-------------------------------------->|
  |                                       |  校验 exp + scope
  |  200 { events: [...] }                |
  |<--------------------------------------|
  |                                       |
  | (15 min 后)                            |
  |                                       |
  |  POST /v1/auth/refresh                 |
  |  { "refresh_token": "..." }           |
  |-------------------------------------->|
  |                                       |  校验 refresh 7d 未过期
  |  200 { access_token: "...",            |
  |       expires_in: 900 }               |
  |<--------------------------------------|
```

---

## 4. WebSocket 鉴权（per D-03 拍板）

**问题**: 浏览器 WebSocket API 不支持自定义 header（`new WebSocket(url)` 不带 header）
**解法**: 链接 token 5 min TTL，**URL query string 携带**

```
wss://api.apeireth.dev/v1/ws?token=<short_lived_token>
```

**生成**:
```
POST /v1/auth/ws-token
{ "access_token": "..." }
→ 200 { "ws_token": "5min_ttl_single_use_token" }
```

**约束**:
- ws_token 单次使用（5 min 内首次连 = 有效；二次连 = 401）
- 5 min 后过期
- scope 必须包含 `ws:connect`
- 走 `apeireth-keyring` PBKDF2 600_000 派生 key

---

## 5. 5 组件详细

### 5.1 access_token

| 属性 | 值 |
|---|---|
| 算法 | HS256 |
| TTL | 900 s (15 min) |
| Claim | `sub` / `scope` / `iat` / `exp` / `iss` / `aud` |
| 校验 | exp < now() → 401 + `error_code: AUTH_TOKEN_EXPIRED` |
| 续命 | 自动（refresh-on-use 模式） |

### 5.2 refresh_token

| 属性 | 值 |
|---|---|
| 长度 | 32 字节随机 (base64url 编码) |
| TTL | 604_800 s (7 d) |
| 存储 | 服务端 SQLite (apeireth-memory 表 `auth_refresh_tokens`) |
| 轮转 | 每次 refresh 触发 → 旧 token 立即失效（防重放） |
| 一次性 | ✅ 用过的 token 立即标记 `used_at` |

### 5.3 scope

**工具 scope 格式**: `<tool>:<action>`

```yaml
calendar:read        # 读日历事件
calendar:write       # 写日历事件
message:send         # 发送消息
message:read         # 读消息
contact:read         # 读联系人
contact:write        # 写联系人（501 stub）
task:read            # 读任务
task:write           # 写任务（501 stub）
search:query         # 搜索查询
drive:upload         # 上传文件
drive:download       # 下载文件
drive:list           # 列举文件
ws:connect           # WebSocket 鉴权
admin:debug          # 调试端点（仅开发模式）
```

**校验时机**: 每次工具 invoke 时查 scope 是否覆盖

### 5.4 expire (exp)

- Unix epoch 秒
- 服务端每 60 s GC 一次过期 refresh token
- 客户端 SDK 自动 refresh-on-use（access_token 剩余 60 s 时自动调 `/v1/auth/refresh`）

### 5.5 refresh-on-use 中间件

**触发**:
- access_token 剩余寿命 < 60 s 时
- 下次工具调用前自动 refresh
- 透明（对调用方零感知）

**实现**: `crates/apeireth-http-client/src/auth.rs::RefreshOnUseLayer`（tower middleware）

```rust
// 简化示意
async fn call_with_refresh(req: Request) -> Response {
    let token = current_token().await;
    if token.exp - now() < 60 {
        let new = refresh(&token.refresh).await?;
        save(new).await;
    }
    send_with_token(req, &new).await
}
```

---

## 6. 错误码

| 错误 | 触发 | 状态码 |
|---|---|---|
| `AUTH_TOKEN_MISSING` | 无 Authorization header | 401 |
| `AUTH_TOKEN_INVALID` | 签名错 / 格式错 | 401 |
| `AUTH_TOKEN_EXPIRED` | exp 已过 | 401 |
| `AUTH_TOKEN_REVOKED` | 主动撤销 | 401 |
| `AUTH_SCOPE_INSUFFICIENT` | scope 不覆盖工具 | 403 |
| `AUTH_REFRESH_INVALID` | refresh token 错 / 过期 / 已用过 | 401 |
| `AUTH_WS_TOKEN_INVALID` | ws token 错 / 过期 / 已用 | 401 |

详见 [`error-codes.md`](error-codes.md) §1.x。

---

## 7. 不修改承诺

- ✅ **不假装已实现**: 仅描述已 commit 代码
- ✅ **编译期 hardcode**: JWT secret 启动期非空校验
- ✅ **不改 LOCKED**: APEIRETH-CONVENTIONS.md §9 §10 严守
- ✅ **不依赖 NewAPI**: 自建 JWT，不引外部 IdP（per R17 砍 NewAPI 决策）

---

## 8. 相关

- 实现: `crates/apeireth-api/src/auth.rs`
- SDK 鉴权集成: `crates/apeireth-sdk/src/wire.rs`
- 凭据存储: `crates/apeireth-keyring` (PBKDF2 600_000 + AES-256-GCM)
- 决策: [`docs/adr/0014-d-03-ws-auth-link-token.md`](../adr/0014-d-03-ws-auth-link-token.md) (D-03 拍板)
