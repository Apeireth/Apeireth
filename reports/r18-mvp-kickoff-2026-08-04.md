# R18 MVP Kickoff — Leptos SSR + WASM hydration,真浏览器试用 Apeireth

**日期**: 2026-08-04
**作者**: 楚零 (按主人 2026-08-03 22:44 授权 + 2026-08-03 23:41 启动 R18 实时反馈)
**R-Cycle**: R18
**目标**: 主人能 `cargo run -p apeireth-web` 打开浏览器,**真用 Apeireth** —— 看到 Council 7 advisor 真辩论

---

## 🎯 主人原话 (2026-08-03 23:41)

> "我希望能看到前端写好,我亲自试用 Apeireth的样子"

**关键解读**:
- "写好" = 不是 mock,真能跑
- "亲自试用" = 主人能在自己浏览器打开,亲自点点
- "Apeireth 的样子" = 看到 Council 7 advisor 真辩论,不是干巴巴的 JSON

---

## 📐 R18 选型决策

### 前端框架: **Leptos 0.7** (SSR + WASM hydration)

| 选项 | 优势 | 劣势 | 决策 |
|------|------|------|------|
| **Leptos 0.7** | ✅ 同 axum 社区作者 (Greg Johnston), SSR + WASM hydration, 真"配套高效"<br>✅ 后端 Council 结果直接 SSR 渲染<br>✅ 客户端 hydration 渐进增强 | ⚠️ 文档相对新, 0.7+ 还在快速演进 | ✅ **选** |
| Dioxus 0.5 | ✅ 跨端 (Web/Desktop/Mobile) | ❌ SSR 不如 Leptos 成熟, 跟 axum 整合需要多一层 | 备选 |
| Yew 0.21 | ✅ 成熟 | ❌ 纯 WASM 没 SSR,首屏需等待 hydration | 否 |
| Tauri (Rust 桌面) | ✅ 真桌面 app | ❌ 用户要"浏览器"试用, 不是桌面 | 否 |
| React/Next.js (非 Rust) | ✅ 生态最大 | ❌ 破坏"全 Rust"原则, 用户没明确 | 否 |

**理由 (用户 2026-08-03 23:30 倾向)**:
- 同生态: axum (后端) + Leptos (前端) 同一作者, 共享服务生态
- 真 SSR: Council 7 advisor 辩论结果后端直接 SSR 渲染, 前端只做 hydration
- Rust 单一语言: 不引入 JS/TS, 工程哲学一致 (Apeireth 全文 Rust)

### 后端整合: **axum 0.7** (已就绪)

apeireth-api 已有 axum 0.7 HTTP server (R16/R17 落地):
- 4 个 endpoint: `/health` / `/v1/chat/completions` / `/council/advise` / `/verdict`
- 真 minimaxi 双协议接通 (R17-01/02)
- 1675 tests passed

**R18 复用**: 同一个 axum server, 加 Leptos SSR router (合并进 `apeireth-web` crate, 启动时用 SSR mode 跑 Leptos)。

---

## 📋 R18 MVP 范围 (用户可见)

### 必须 (MVP 核心)

1. **`apeireth-web` 新 crate** (workspace 新成员)
   - Leptos 0.7 + axum 0.7 整合
   - SSR mode (`cargo run -p apeireth-web --features ssr`)
   - WASM hydration (浏览器客户端继续交互)
2. **1 个首页** (路由 `/`)
   - 输入框: 议题 (topic)
   - 提交按钮: 触发 Council 7 advisor 辩论
3. **Council 7 advisor 辩论可视化** (核心 UX)
   - 7 个 advisor 卡片 (safety / performance / philosophy / history / strategy / ethics / legal)
   - 每个卡片显示: 立场 (approve/reject/neutral) + 推理文字
   - 加载状态 (spinner) + 实时刷新 (WASM hydration 后)
4. **最终 verdict 显示**
   - approved / rejected / needs_more_review
   - V1+V2+V3 AND 门结果 (可选)

### 不做 (R19+ 范围)

- 多页面路由 (首页 + Memory timeline + Verdict panel)
- 用户登录 / 鉴权
- 主题切换 (亮/暗)
- 移动端响应式 (先用桌面)
- 实时 WebSocket (用 HTTP polling 即可)
- 6 类非 LLM API 集成 (R20+)

---

## 🏗️ 架构 (R18)

```
apeireth-web  (新 crate, workspace 新成员)
├── Cargo.toml         (leptos 0.7 + axum 0.7 + apeireth-api 依赖)
├── src/
│   ├── lib.rs          (Leptos 入口, server_fn 定义)
│   ├── main.rs         (axum 启动 + Leptos SSR 整合)
│   ├── app.rs          (App 组件: 首页 + Council 卡片)
│   ├── council.rs      (Council 7 advisor UI 组件)
│   └── verdict.rs      (Verdict 显示组件)
└── style/
    └── main.css        (基本样式, 不引入 Tailwind)
```

**关键整合点**:
- `apeireth-web/src/main.rs` 用 `leptos_axum::generate_route_list` 把 Leptos 路由转 axum router
- `lib.rs::server_fn` 用 `#[server]` 宏定义后端函数, 前端调用自动变成 HTTP 请求
- 前端组件用 `Resource` 异步加载 Council 辩论数据, SSR 渲染首屏, hydration 后客户端继续交互

---

## 🚀 跑法 (用户最终验收)

```powershell
# 1. 设置真 API key
$env:APEIRETH_API_KEY = (Get-Content .minimax-agent-cn\projects\apikey.txt)[0].Trim()

# 2. 启动 apeireth-web (SSR mode)
cd .openclaw\workspace\promethean\Apeireth-rust
cargo run -p apeireth-web --features ssr

# 3. 浏览器打开
# http://localhost:3000

# 4. 输入议题, e.g. "2026 学术研究项目应该优先关注什么?"
# 5. 点击提交 → 看到 Council 7 advisor 辩论 (7 LLM 真调用)
```

---

## 🛡️ R18 不破坏承诺 (守 8 项)

| 承诺 | R18 处理 |
|------|---------|
| **Cargo.toml `version = "0.14.0"`** | ✅ 不变 |
| **R11 baseline 三值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063)** | ✅ LOCKED |
| **阶段 1+2+3+4+5 LOCKED** | ✅ R18 是新阶段, 不动旧 LOCKED |
| **apeireth-api crate 名字** | ✅ 复用, 加新 crate `apeireth-web` |
| **Document-Meta 格式** | ✅ R18 报告按 v12 新规范 |
| **12 子规范系统** | ✅ v12 已含 R15+ 实践 |
| **R-Measure baseline** | ✅ 不动 |
| **架构图 P1-P5** | ✅ R18 加新图 P6 (前端架构) 不破坏旧的 |

---

## 📊 数字目标 (R18 收尾)

| 维度 | 目标 |
|------|------|
| 新增 crate | 1 个 (apeireth-web) |
| 前端代码 | ~600 行 (Leptos components) |
| 后端整合 | 0 行 (复用 axum 0.7 已有) |
| 真 minimaxi 验证 | 7 advisor 端到端 (Council 辩论) |
| 测试 | 1675 + 20 (前端 server_fn 单元测试) = ~1695 |
| 跑法 | `cargo run -p apeireth-web` 打开浏览器 |

---

## 🚧 R18 不做的事 (R19+ 计划)

- 多页面 (Memory timeline / Verdict panel / Settings)
- 6 类非 LLM API 集成 (R20+)
- 移动端响应式
- 用户登录 / 鉴权
- WebSocket 实时推送

---

## 📂 报告路径 (v12 规范)

- `r18-mvp-kickoff-2026-08-04.md` ← **本报告**
- `r18-week1-frontend-mvp-2026-08-04.md` (R18-02..07 业务)
- `r18-finalize-2026-08-04.md` (R18 收尾)

---

**作者**: 楚零 (按主人 2026-08-03 23:41 实时启动)
**R18 启动**: 2026-08-03 23:43
**R18 目标**: 主人能在浏览器真用 Apeireth
