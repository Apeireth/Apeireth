# Apeireth API 统一错误码

> **性质**: v1 API 全局 error_code 规范（12 类）
> **依据**: `crates/apeireth-api/src/lib.rs` error 枚举 + D-04 主人拍板
> **最后更新**: 2026-08-05

---

## 1. 错误响应结构

```json
{
  "error": {
    "code": "TOOL_NOT_FOUND",
    "message": "Tool 'foo' not registered",
    "details": {
      "tool_name": "foo",
      "registered": ["calendar", "message", "contact", "task", "search", "drive"]
    },
    "request_id": "req-7c5a3b2e-9f8d"
  }
}
```

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `code` | string | ✅ | 大写 snake_case，错误分类 |
| `message` | string | ✅ | 人类可读（i18n: 中英双语，per D-04） |
| `details` | object | 🟡 可选 | 上下文键值对 |
| `request_id` | string | ✅ | 服务端 trace ID（关联 logs/metrics） |

---

## 2. HTTP 状态码语义

| 状态码 | 含义 | 触发 |
|---|---|---|
| 200 | OK | 业务成功 |
| 201 | Created | 资源创建成功（如 calendar event） |
| 204 | No Content | 删除成功 |
| 400 | Bad Request | 请求 schema 校验失败 |
| 401 | Unauthorized | 鉴权失败（5 类 auth 错误） |
| 403 | Forbidden | scope 不覆盖 |
| 404 | Not Found | 资源不存在 |
| 409 | Conflict | 资源冲突（如重复） |
| 422 | Unprocessable Entity | 业务规则拒绝 |
| 429 | Too Many Requests | 限流触发（per `apeireth-constraint`） |
| 500 | Internal Server Error | 服务端未捕获 |
| 501 | Not Implemented | stub 端点（per D-05） |
| 503 | Service Unavailable | 维护中 / 启动期 |

---

## 3. 12 类 error_code

### 3.1 鉴权类（AUTH_*）

| Code | HTTP | 触发 | 客户端动作 |
|---|---|---|---|
| `AUTH_TOKEN_MISSING` | 401 | 无 Authorization | 跳转登录 |
| `AUTH_TOKEN_INVALID` | 401 | 签名错 / 格式错 | 重新登录 |
| `AUTH_TOKEN_EXPIRED` | 401 | exp 已过 | 自动 refresh |
| `AUTH_TOKEN_REVOKED` | 401 | 服务端主动撤销 | 重新登录 |
| `AUTH_SCOPE_INSUFFICIENT` | 403 | scope 不覆盖 | 申请权限 |
| `AUTH_REFRESH_INVALID` | 401 | refresh token 错 / 过期 | 重新登录 |
| `AUTH_WS_TOKEN_INVALID` | 401 | ws token 错 | 重新获取 ws_token |

### 3.2 工具类（TOOL_*）

| Code | HTTP | 触发 |
|---|---|---|
| `TOOL_NOT_FOUND` | 404 | 工具名未注册 |
| `TOOL_DISABLED` | 503 | 工具临时关闭（如升级期） |
| `TOOL_PERMISSION_DENIED` | 403 | 工具内置权限拒绝（4 守门之一） |
| `TOOL_TIMEOUT` | 504 | 工具执行超时（默认 30 s） |
| `TOOL_RATE_LIMITED` | 429 | 工具级限流（per token bucket） |
| `TOOL_NOT_IMPLEMENTED` | 501 | stub 端点 |

### 3.3 校验类（VALIDATION_*）

| Code | HTTP | 触发 |
|---|---|---|
| `VALIDATION_FAILED` | 400 | JSON schema 校验失败 |
| `VALIDATION_MISSING_FIELD` | 400 | 必填字段缺失 |
| `VALIDATION_INVALID_TYPE` | 400 | 字段类型错 |
| `VALIDATION_OUT_OF_RANGE` | 400 | 数值/字符串越界 |
| `VALIDATION_REGEX_FAILED` | 400 | 正则校验失败 |

### 3.4 资源类（RESOURCE_*）

| Code | HTTP | 触发 |
|---|---|---|
| `RESOURCE_NOT_FOUND` | 404 | 资源不存在 |
| `RESOURCE_ALREADY_EXISTS` | 409 | 重复创建 |
| `RESOURCE_LOCKED` | 423 | 资源被锁（如 DB write lock） |
| `RESOURCE_QUOTA_EXCEEDED` | 429 | 配额超限（per D-05 quota stub 501 R21 实装） |

### 3.5 限流类（RATE_LIMIT_*）

| Code | HTTP | 触发 |
|---|---|---|
| `RATE_LIMIT_GLOBAL` | 429 | 全局 qps 超限 |
| `RATE_LIMIT_PER_USER` | 429 | 用户级 qps 超限 |
| `RATE_LIMIT_PER_TOOL` | 429 | 工具级 qps 超限 |
| `RATE_LIMIT_BURST_EXHAUSTED` | 429 | token bucket 突发耗尽 |

### 3.6 上游类（UPSTREAM_*）

| Code | HTTP | 触发 |
|---|---|---|
| `UPSTREAM_TIMEOUT` | 504 | LLM/Provider 超时 |
| `UPSTREAM_RATE_LIMIT` | 429 | Provider 端限流 |
| `UPSTREAM_INVALID_RESPONSE` | 502 | Provider 返回 schema 错 |
| `UPSTREAM_AUTH_FAILED` | 502 | Provider API key 失效 |

### 3.7 数据类（DATA_*）

| Code | HTTP | 触发 |
|---|---|---|
| `DATA_CORRUPTION` | 500 | DB 完整性错误 |
| `DATA_MIGRATION_FAILED` | 500 | 迁移失败（per D-07 一次性迁移） |
| `DATA_BACKUP_FAILED` | 500 | 备份失败 |
| `DATA_DISK_FULL` | 507 | 磁盘满（per rollback TTL 7 天硬上限） |

### 3.8 内部类（INTERNAL_*）

| Code | HTTP | 触发 |
|---|---|---|
| `INTERNAL_PANIC` | 500 | 服务端 panic |
| `INTERNAL_BUG` | 500 | 已知 bug（应上报 issue） |
| `INTERNAL_NOT_IMPLEMENTED` | 501 | 代码未实装 |

### 3.9 协议类（PROTOCOL_*）

| Code | HTTP | 触发 |
|---|---|---|
| `PROTOCOL_VERSION_MISMATCH` | 400 | 协议版本不匹配 |
| `PROTOCOL_INVALID_FRAME` | 400 | WS 帧格式错 |
| `PROTOCOL_TIMEOUT` | 408 | WS 心跳超时 |

### 3.10 配置类（CONFIG_*）

| Code | HTTP | 触发 |
|---|---|---|
| `CONFIG_INVALID` | 500 | 配置加载失败 |
| `CONFIG_MISSING_REQUIRED` | 500 | 启动期必填配置缺失 |

### 3.11 安全类（SECURITY_*）

| Code | HTTP | 触发 |
|---|---|---|
| `SECURITY_SELF_DISABLE` | 503 | Self-Disable 触发（per 双洋葱统一体） |
| `SECURITY_PERMISSION_REVOKED` | 403 | 权限被回收 |
| `SECURITY_SUSPICIOUS_ACTIVITY` | 429 | 异常活动检测（rate limit） |

### 3.12 维护类（MAINTENANCE_*）

| Code | HTTP | 触发 |
|---|---|---|
| `MAINTENANCE_IN_PROGRESS` | 503 | 升级期 |
| `MAINTENANCE_SHUTTING_DOWN` | 503 | 优雅关闭期 |
| `MAINTENANCE_BACKUP_RUNNING` | 503 | 备份期（读仍可用） |

---

## 4. i18n（per 1.0 release #10）

错误信息支持双语：
- `Accept-Language: zh-CN` → 中文
- `Accept-Language: en-US` → 英文（默认）

`message` 字段走 `apeireth-i18n` crate（per 5 Locale: en/zh-CN/ja/de/fr）。

```json
// Accept-Language: zh-CN
{ "code": "TOOL_NOT_FOUND", "message": "工具 'foo' 未注册" }

// Accept-Language: en-US
{ "code": "TOOL_NOT_FOUND", "message": "Tool 'foo' not registered" }
```

---

## 5. request_id 关联

每个响应（含错误）都带 `request_id`，客户端记录后可：
- 服务端 logs: `apeireth-api.log` 按 `request_id` grep
- 指标: `apeireth_request_duration_seconds` 带 label
- tracing: 关联到 OpenTelemetry trace span

---

## 6. 不修改承诺

- ✅ 不假装已实现：所有 code 都有对应已 commit 实现或 TODO R21 标记
- ✅ 编译期 hardcode：12 类 code 在 `apeireth-api::Error` 枚举编译期固定
- ✅ 不改 LOCKED：API 协议层 + 错误 schema 严守向后兼容

---

## 7. 相关

- 实现: `crates/apeireth-api/src/lib.rs` (Error 枚举)
- i18n: `crates/apeireth-i18n` (5 Locale)
- 限流: `crates/apeireth-constraint` (token bucket)
- 1.0 release #10: [i18n 12 项 checklist](../1.0-release/checklist.md#10-i18n)
