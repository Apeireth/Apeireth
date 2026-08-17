# ADR 0014: D-03 WebSocket 鉴权 = 链接 token 5 min TTL 单次使用

> **状态**: 🟢 Accepted (主人 2026-08-05 20:53 拍板)
> **commit 锚**: `r20-stage-2-3-prep-2026-08-05.md` §3.1 (决策记录) + `crates/apeireth-api/src/ws_v1.rs` 实施
> **最后更新**: 2026-08-05

---

## 1. 背景 (Context)

Apeireth WebSocket (`/v1/ws`) 用于流式 LLM 响应 + 工具调用。

**问题**:
- 浏览器 WebSocket API `new WebSocket(url)` **不支持自定义 header**
- 不能用 `Authorization: Bearer <token>` 头
- 必须在 URL 里传 token（query string）

**约束**:
- token 不能长 TTL（URL 容易泄漏到 logs / referer / 中间代理）
- token 不能无 TTL（防重放）
- 浏览器场景必须能用

---

## 2. 决策 (Decision)

**链接 token (link token) 5 min TTL，单次使用 (single-use)**

**流程**:
```
1. 客户端 POST /v1/auth/ws-token { "access_token": "..." }
   → 200 { "ws_token": "5min_ttl_single_use_token" }

2. 客户端 wss://api.apeireth.dev/v1/ws?token=<ws_token>
   → 服务端校验 ws_token:
      - 在 5 min 内?
      - 未使用过?
      - scope 含 ws:connect?
   → 校验通过 → 立即标记 ws_token 已用 → 建立 WS
   → 校验失败 → 401 + close

3. WS 建立后, 后续帧 (chat / tool_result / ping) 不再查 token
   (用 session_id 隐式鉴权)
```

**5 组件协同**:
- access_token: 15 min, 用于调 `/v1/auth/ws-token`
- ws_token: 5 min 单次, 用于建立 WS
- scope: 必须含 `ws:connect`
- expire: ws_token 服务端校验 < 5 min
- refresh-on-use: access_token 剩余 < 60 s 时自动 refresh

**存储**:
- ws_token 存 `apeireth-memory` SQLite (表 `auth_ws_tokens`)
- 字段: `token_hash`, `user_id`, `scope`, `expires_at`, `used_at`
- 5 min 后 GC

---

## 3. 后果 (Consequences)

### 3.1 正面

- ✅ **浏览器可用**: 解决 `new WebSocket(url)` 不支持 header 问题
- ✅ **短 TTL**: 5 min 远低于 access_token 15 min
- ✅ **单次使用**: 即使泄漏到 logs, 已用就失效
- ✅ **scope 校验**: 复用现有 scope 体系
- ✅ **服务端可追踪**: ws_token 关联 user_id + session_id

### 3.2 负面

- ⚠️ **多步交互**: 客户端需要先调 `/v1/auth/ws-token`，多 1 RTT
- ⚠️ **5 min 内必须连**: 客户端拿到 ws_token 后不能延迟太久
- ⚠️ **5 min 后 GC**: 服务端定时清理 (每 60 s)

### 3.3 风险

- 用户在 5 min 内不连，ws_token 浪费（但仅多 1 RTT, 代价小）

---

## 4. 备选 (Alternatives Considered)

### A. URL query string 带长 TTL token (15 min 同 access_token)
- 优点: 1 步交互
- 否决: 浏览器 logs / referer / 中间代理泄漏, 15 min TTL 太长, 不安全

### B. cookie 带 token
- 优点: 不污染 URL
- 否决: 浏览器 cookie 跨域难, SameSite / CORS 配置复杂; WS upgrade 不带 cookie 行为依赖浏览器

### C. URL 带一次性签名 (HMAC)
- 优点: 服务端 stateless
- 否决: 客户端需持 secret, 不适合多用户/多设备场景

### D. 子协议 (Sec-WebSocket-Protocol)
- 优点: 浏览器不污染 URL
- 否决: `new WebSocket(url, [protocols])` 协议头容易被中间代理剥; 跨域限制更多

### E. 客户端 TLS 双向认证
- 优点: 最强
- 否决: 浏览器不支持 (除特殊 enterprise 场景)

---

## 5. 6 哲学锚穿透

- ✅ **S-1 走在前人经验上**: JWT 短 TTL + 一次性 token 业界惯例
- ✅ **S-2 实事求是**: 浏览器 WS 限制是不争事实
- ✅ **O-2 用户看结果不看哲学**: 用户只看 WS 通不通, 不看 token 怎么传
- ✅ **O-3 信息密度"高"**: 5 min TTL + 单次使用 + scope 三件套简洁
- ✅ **O-4 干净状态 = 没有历史包袱**: 不存长 TTL ws_token
- ✅ **O-5 6 哲学锚穿透**: 本节自检

---

## 6. 8 项不修改承诺

- ✅ **不假装已实现**: 5 min TTL + 单次使用 是已 commit 设计
- ✅ **编译期 hardcode**: `auth_ws_tokens` 表 schema 编译期固定
- ✅ **不改 LOCKED**: APEIRETH-CONVENTIONS.md §9 6 哲学锚保留
- ✅ **不改 workspace version**: v1.0.0 严守
- ✅ **6 哲学锚穿透**: §5 自检
- ✅ **不依赖 NewAPI**: 自建 JWT + ws_token
- ✅ **不重复造轮子**: 沿用 5 鉴权组件框架
- ✅ **诚实标缺**: 5 min 内不连会浪费的边界已说明

---

## 7. 引用

- 决策 ID 体系: `docs/stage4/pending-decisions-overview-2026-08-05.md` (D-03)
- 蓝图: `docs/stage4/r20-stage-2-3-prep-2026-08-05.md` §3.1
- 实施: `crates/apeireth-api/src/ws_v1.rs`
- 文档: [`docs/api/auth.md`](../api/auth.md) §4
- 文档: [`docs/api/v1-websocket.md`](../api/v1-websocket.md) §1
