# ADR 0018: Rust SDK 设计

> **状态**: 🟢 Accepted (R20 阶段 4 拍板)
> **commit 锚**: `crates/apeireth-sdk/src/{client,wire,error,version,abi}.rs`
> **最后更新**: 2026-08-05

---

## 1. 背景 (Context)

Apeireth v1 需 SDK 暴露给外部:
- 第三方集成 (TUI / Tauri / 内部工具)
- 减少 HTTP 协议细节认知负担
- 强类型 + 编译期检查

**问题**:
- 用什么语言? 几个 SDK?
- 设计 API 风格 (builder / 方法链 / trait)?

---

## 2. 决策 (Decision)

**SDK 总策略: Rust 主力 + 5 stub (Lark / Voice / LiveKit / Sandbox / 4 Provider)**

### 2.1 Rust SDK 设计原则

| 原则 | 体现 |
|---|---|
| **Builder 模式** | `Client::new(url).with_token(t).with_auth(a).build()` |
| **方法链** | `client.tool("calendar").action("list_events").params(json!({})).invoke()` |
| **强类型** | `invoke::<T>()` 反序列化到具体类型 |
| **async/await** | 全面 tokio |
| **错误统一** | `Error` enum 覆盖所有错误 |
| **自动 refresh** | refresh-on-use 中间件 |
| **重试策略** | `RetryPolicy` 可配置 |

### 2.2 SDK 模块结构

```
apeireth-sdk/
├── lib.rs              # 公共 API
├── client.rs           # Client 主入口
├── wire.rs             # 协议 + 序列化
├── abi.rs              # 4 协议 ABI
├── error.rs            # Error 类型
├── version.rs          # SDK_VERSION 常量
├── auth/               # 鉴权 (auto refresh)
├── ws/                 # WebSocket 客户端
├── tools/              # 6 工具便捷方法
└── provider/           # 5 Provider 客户端
```

### 2.3 版本策略

**SDK_VERSION 跟 workspace version 同步 (per D-05 拍板)**
- 当前: 1.0.0
- 1.0.0 → 1.0.1 patch (bug fix)
- 1.0.0 → 1.1.0 minor (新增方法, 向后兼容)
- 1.0.0 → 2.0.0 major (破坏性, 加 /v2 路径)

---

## 3. 后果 (Consequences)

### 3.1 正面

- ✅ **强类型**: 编译期查错
- ✅ **async/await**: 跟 Rust 生态一致
- ✅ **builder 模式**: 业界惯例, 易理解
- ✅ **refresh-on-use**: 用户零感知
- ✅ **多 SDK 覆盖**: Rust + 5 stub 满足主流场景
- ✅ **不破坏 workspace version 严守**: SDK 跟 v1.0.0 同步

### 3.2 负面

- ⚠️ **学习曲线**: builder + async + generic invoke 需熟悉
- ⚠️ **5 stub 后续维护**: R21 必须补 4 Provider + Lark 真接
- ⚠️ **Python SDK 缺**: 用户需 Python 集成时无 SDK
- ⚠️ **TS SDK 缺**: 前端集成时无 SDK (Tauri 估补时需要)

### 3.3 风险

- 第三方集成若用 Python/TS, 需走 HTTP 直调
- R21 估补 Python SDK (per roadmap 19 路)

---

## 4. 备选 (Alternatives Considered)

### A. 仅 HTTP 直调, 无 SDK
- 优点: 不维护 SDK
- 缺点: 用户需手写 HTTP + 错误处理
- 否决: 业界产品都有 SDK, 1.0 release 必须有

### B. Rust + Python + TS 三 SDK
- 优点: 覆盖广
- 缺点: 估时 + 维护成本 × 3
- 否决: 1.0 release 集中 Rust 主力, 多语言 R21+ 估补

### C. Rust + 5 stub (本决策)
- 优点: 主力 Rust 真接 + 5 stub 留 R21
- 缺点: 5 stub 后续要补
- 拍板: R20 阶段 4 拍板

### D. gRPC 一统江湖
- 优点: 强类型 + 多语言
- 缺点: 浏览器支持差, 跟 REST 体系割裂
- 否决: 跟 v1 API 风格不一致

### E. auto-generated OpenAPI SDK (e.g. openapi-generator)
- 优点: 0 维护, 多语言自动
- 缺点: 生成代码质量差, 错误处理不友好
- 否决: 业界共识 OpenAPI gen 适合 prototype 不适合产品

---

## 5. 6 哲学锚穿透

- ✅ **S-1 走在前人经验上**: builder + async + typed 业界惯例
- ✅ **S-2 实事求是**: 主力 Rust 真接, 多语言留 R21
- ✅ **O-2 用户看结果不看哲学**: 用户只看 SDK 好不好用
- ✅ **O-3 信息密度"高"**: 模块结构 1 图说清
- ✅ **O-4 干净状态 = 没有历史包袱**: 拒绝 gRPC / auto-gen
- ✅ **O-5 6 哲学锚穿透**: 本节自检

---

## 6. 8 项不修改承诺

- ✅ **不假装已实现**: Rust 真接 + 5 stub 诚实标注
- ✅ **编译期 hardcode**: 协议 schema + 工具白名单编译期固定
- ✅ **不改 LOCKED**: SDK 协议层 LOCKED
- ✅ **不改 workspace version**: v1.0.0 严守 (SDK 同步)
- ✅ **6 哲学锚穿透**: §5 自检
- ✅ **不依赖 NewAPI**: 自建 SDK
- ✅ **不重复造轮子**: 沿用 reqwest / serde / tokio
- ✅ **诚实标缺**: 5 stub + Python/TS SDK 缺 R21+ 估补

---

## 7. 引用

- 实施: `crates/apeireth-sdk/`
- 文档: [`docs/sdk/README.md`](../sdk/README.md)
- 决策 D-05: SDK version 跟 workspace version 同步 (`docs/stage4/pending-decisions-overview-2026-08-05.md`)
