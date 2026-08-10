# ADR 0011: TUI 瘦客户端 — Apeireth CLI 即 TUI (HTTP to apeireth-api)

> **状态**: 🟢 Accepted (R25 阶段改瘦 2026-08-04 拍板, 1.0 release 阶段 4 续)
> **commit 锚**: `crates/apeireth-tui/` + `crates/apeireth-cli/` + `crates/apeireth-http-client/` (R25 改瘦)
> **最后更新**: 2026-08-05 22:13 (主人 22:13 拍"只干 TUI" 进一步确认)
> **原版 ADR**: [`archive/r20-pre-renumber/0022-tui-as-thin-client.md`](archive/r20-pre-renumber/0022-tui-as-thin-client.md) (v0; v1 本 ADR 引用新编号 0001/0006/0007)

---

## 1. 背景 (Context)

Apeireth 1.0 release (v1.0.0) 客户端形态有 2 个候选: **TUI** (Terminal UI) + **Tauri 桌面**。

**问题**:
- 主人 2026-08-04 拍板: "我们最后要做的前端应该是 Tauri, 但现在手头的 AI 团队没有适合干审美设计的, 所以 web 和桌面都搁置, 先做好 TUI 来为桌面做准备"
- 主人 2026-08-05 22:13 再确认: "只干 TUI, 1.0 release 收口"
- TUI 改瘦前是 **TUI 直接调 lib** (apeireth-tui → apeireth-core 直调), 集成测试床 (Tauri 来了无缝换 UI 层) 角色不清晰
- 改瘦后 TUI = **HTTP to apeireth-api** (与 Tauri 共享同一 backend API surface)

**约束**:
- TUI 是 dev 自己干, 后端优先级更高 (TUI 是"集成测试床", 后端是真正价值)
- TUI 必须跟 Tauri 共享同一套 backend API (否则 Tauri 来了要重写 UI 层)
- TUI 5 nav + 9 器官拟人化 + 主对话 (per 主人 2026-08-04 拍板, per `docs/stage4/architecture-frontend-design-proposal.md`)

---

## 2. 决策 (Decision)

**TUI = 瘦客户端, HTTP to apeireth-api, 跟 Tauri 共享同一 backend API surface**

### 2.1 改瘦前后对比 (R20 阶段 1-3 vs R25 改瘦)

| 维度 | 改瘦前 (R20 阶段 1-3) | 改瘦后 (R25 阶段, 1.0 release) |
|---|---|---|
| **集成方式** | `apeireth-tui` → `apeireth-core` 直接调 lib | `apeireth-tui` → `apeireth-http-client` → `apeireth-api` (HTTP) |
| **启动** | TUI 跟 core 同进程, 共享内存 | TUI 起 apeireth-api 子进程, 走 localhost:8080 |
| **共享 backend** | ❌ TUI 独享 core, Tauri 要重写 | ✅ TUI + Tauri 共享 HTTP API |
| **集成测试床** | ❌ TUI 测的是 core, 不是 API | ✅ TUI 测的是 API, 跟 Tauri 测的一样 |
| **集成测试 5 nav + 9 器官** | 估补, 跟 core 测 | ✅ ratatui TestBackend 测 TUI 设计契约, 25+ 测试 (per `crates/apeireth-tui-e2e/`) |
| **SSE 流式** | 直调 (MemoryChannel) | 走 `reqwest` `stream` feature (per `Cargo.toml` line 169) |

### 2.2 TUI 5 nav + 9 器官 (per `docs/stage4/architecture-frontend-design-proposal.md`)

**5 nav** (主导航):
1. **主页** — 状态 (9 器官 + 健康环 + 神经图)
2. **对话** — 主对话区 (per 主人拍板 "主 AI" 是 AI 跟用户一同成长)
3. **历史** — 对话历史
4. **设置** — 5 鉴权 / 限流 / Provider 切换
5. **工具** — 6 工具调用面板 (calendar / message / contact / task / search / drive, per [0006-d-01-tool-endpoint-real.md](0006-d-01-tool-endpoint-real.md))

**9 器官** (AI 状态拟人化, per 主人 2026-08-04 "器官很有意思"):
- 1 屏多卡片, 关键数字一眼 (per O-3 信息密度"高")
- 仅显示 "状态 + 主对话结果", 不暴露哲学 / 守门 / 内部机制 (per O-2 用户看结果不看哲学)
- 1 屏 = 主页 (per O-3 主页, 不是"功能列表")

### 2.3 改瘦技术细节

```rust
// crates/apeireth-tui/src/main.rs (改瘦后)
#[tokio::main]
async fn main() -> Result<()> {
    // 1. 启动 apeireth-api 子进程 (如未启动)
    let api = ensure_api_subprocess().await?;
    
    // 2. 初始化 HTTP client
    let client = ApeirethClient::new("http://localhost:8080")
        .with_token(load_token()?)
        .build();
    
    // 3. 启动 ratatui
    let mut terminal = ratatui::init();
    let app = App::new(client);
    let result = app.run(&mut terminal).await;
    ratatui::restore();
    result
}
```

### 2.4 TUI 集成测试 (per `crates/apeireth-tui-e2e/`)

- 25+ 测试用 ratatui `TestBackend`, 模拟 5 nav 切换 + 9 器官渲染
- 0 触碰 24 LOCKED crate + 0 改 workspace version
- 0 干 Tauri, 0 主动 commit (per 主人 2026-08-05 拍"测一下先, 升级计划沉淀成文档")
- 集成测试测 TUI 设计契约, 不测 apeireth-core 内部 (那部分 core 自己测)

### 2.5 Tauri 未来接入 (R21+ 估补)

当 Tauri 团队到位时:
1. Tauri 起 `apeireth-api` 子进程 (跟 TUI 同模式)
2. Tauri 用同一套 `apeireth-http-client` (或 TS 版的 `apeireth-sdk`, 估补)
3. Tauri 共享 TUI 的 5 nav + 9 器官设计, 但用 web 前端实现
4. TUI 集成测试 25+ 套可直接迁到 Tauri (测 API 不测 UI)

---

## 3. 后果 (Consequences)

### 3.1 正面

- ✅ **集成测试床**: TUI 测的就是 API, Tauri 来了 0 改 backend
- ✅ **TUI 5 nav + 9 器官 落地**: 主人 2026-08-04 拍板设计契约可实装
- ✅ **TUI 集成测试 25+ 套**: 测 TUI 设计契约, 不污染 core
- ✅ **SSE 流式共享**: `reqwest` `stream` feature TUI / Tauri / 第三方客户端 都能用
- ✅ **dev 主线清晰**: TUI 是 dev 自己干, 后端是真价值, 角色分工清晰

### 3.2 负面

- ⚠️ **多 1 RTT**: TUI → API → core, 跟直调 core 比多 1 RTT (估 1-5 ms, 不可察)
- ⚠️ **子进程管理**: TUI 负责启 / 停 / 重启 API, 失败处理略复杂
- ⚠️ **本地端口冲突**: 8080 已被占时, TUI 要 fallback 8081+ (mitigation: 启动时 socket 检测)

### 3.3 风险

- TUI 5 nav + 9 器官 视觉化 1.0 release 不必全做完 (per 主人 2026-08-04 拍板 "TUI 集成测试床 + 后端先收"), 视觉化 R21+ 估补
- Tauri 团队 R21+ 估补, 1.0 release 仅 TUI
- 主人 22:13 拍"只干 TUI" 进一步确认 TUI 是 1.0 release 唯一前端 (per [0001-apeireth-rust-1.0.md](0001-apeireth-rust-1.0.md) §1 决策驱动)

---

## 4. 备选 (Alternatives Considered)

### A. TUI 直调 core (改瘦前)
- 优点: 0 RTT, 简单
- 否决: TUI 跟 Tauri 不共享 backend, Tauri 来了要重写; 集成测试床角色不清晰

### B. TUI + Tauri 并行开发
- 优点: 1.0 release 完整前端体验
- 否决: 主人 2026-08-04 拍板 "缺审美设计前 Tauri 不上, TUI 自己干"; 22:13 再确认"只干 TUI"

### C. 仅 Tauri, 不做 TUI
- 优点: 直接终极
- 否决: 主人 2026-08-04 拍板 "Tauri 是终极, TUI 是过渡", TUI 必做

### D. TUI = 瘦客户端 (本决策)
- 优点: 集成测试床 + Tauri 来了无缝换
- 拍板: R25 阶段主人 2026-08-04 拍板; 22:13 再确认

### E. TUI + web (e.g. React SPA)
- 优点: 跨平台
- 否决: 主人 2026-08-04 拍板 "web 也搁置, 缺审美设计"

### F. TUI + Tauri + Web 三端
- 优点: 覆盖全
- 否决: 1 owner × 1 端可做, 3 端要 3 owner; 主人 2026-08-04 拍板 "AI 团队没有适合干审美设计的"

---

## 5. 6 哲学锚穿透

- ✅ **S-1 走在前人经验上 (北极星)**: TUI 改瘦 = 业界 "thin client" 模式 (e.g. VSCode remote, Chrome remote desktop)
- ✅ **S-2 实事求是**: 主人 2026-08-04 拍板 "Tauri 终极, TUI 过渡", 不凭想象; 22:13 再确认"只干 TUI"
- ✅ **O-2 走在前人肩上 (用户看结果不看哲学)**: TUI 5 nav + 9 器官仅显示状态, 不暴露哲学
- ✅ **O-3 干到底 (信息密度"高")**: 1 屏 = 主页, 5 nav + 9 器官关键数字一眼
- ✅ **O-4 任何人都能接手 (干净状态)**: 拒绝 "TUI 直调 core" 旧模式 (改瘦前)
- ✅ **O-5 不假装 (6 哲学锚穿透)**: 本节自检

---

## 6. 8 项不修改承诺

- ✅ **不假装已实现**: TUI 5 nav + 9 器官 视觉化估 R21+ 完成, 1.0 release 仅核心
- ✅ **编译期 hardcode**: 5 nav 列表 + 9 器官 enum 编译期固定
- ✅ **不改 LOCKED**: 7 LOCKED 文档 + 24 LOCKED crate 0 触碰
- ✅ **不改 workspace version**: v1.0.0 严守
- ✅ **6 哲学锚穿透**: §5 自检
- ✅ **不依赖 NewAPI**: TUI HTTP client 自建 (`apeireth-http-client`)
- ✅ **不重复造轮子**: 沿用 ratatui / tokio / reqwest / anyhow 业界标准
- ✅ **诚实标缺**: 9 器官视觉化 R21+ 估补, Tauri R21+ 估补

---

## 7. 引用

- 主人 2026-08-04 拍板: "Tauri 终极, TUI 过渡" (per self-stance log + user memory)
- 主人 2026-08-05 22:13 拍板: "只干 TUI, 1.0 release 收口" (per [0001-apeireth-rust-1.0.md](0001-apeireth-rust-1.0.md) §1 决策驱动)
- 改瘦前 TUI 实施: `crates/apeireth-tui/src/` (R20 阶段 1-3)
- 改瘦后 HTTP client: `crates/apeireth-http-client/` (R25 阶段)
- 集成测试: `crates/apeireth-tui-e2e/` (R20 阶段 5 估补, 25+ 测试)
- 前端设计: [`docs/stage4/architecture-frontend-design-proposal.md`](../../docs/stage4/architecture-frontend-design-proposal.md)
- SSE 流式: `Cargo.toml` line 169 (`reqwest` `stream` feature)
- TUI 升级路线图: `reports/tui-upgrade-roadmap-2026-08-04.md` (估补)
- D-01 6 工具真接: [`0006-d-01-tool-endpoint-real.md`](0006-d-01-tool-endpoint-real.md)
- D-02 6 工具 1 URL 子路径: [`0007-d-02-v1-tools-subpath.md`](0007-d-02-v1-tools-subpath.md)
- 1.0 release 总览: [`0001-apeireth-rust-1.0.md`](0001-apeireth-rust-1.0.md)
- 1:1 翻译原则: [`0012-spectrAI-reverse-engineering.md`](0012-spectrAI-reverse-engineering.md)
- 原版 ADR v0: [`archive/r20-pre-renumber/0022-tui-as-thin-client.md`](archive/r20-pre-renumber/0022-tui-as-thin-client.md)

---

## 8. 附录

### 8.1 TUI 5 nav + 9 器官 quick ref (per 主人 2026-08-04 拍板)

**5 nav** (主导航, 1 屏 1 nav, 切换不重渲染):
```
1. 主页 (Home)        - 9 器官 + 健康环 + 神经图 (O-3 信息密度"高" 主页)
2. 对话 (Chat)        - 主对话区, AI 跟用户一同成长
3. 历史 (History)     - 对话历史列表 + 搜索
4. 设置 (Settings)    - 5 鉴权 / 限流 / Provider 切换
5. 工具 (Tools)       - 6 工具调用面板 (per [0006-d-01](0006-d-01-tool-endpoint-real.md) + [0007-d-02](0007-d-02-v1-tools-subpath.md))
```

**9 器官** (AI 状态拟人化, per 主人 2026-08-04 "器官很有意思"):
```
1. 心跳 (Heartbeat)   - AI 活跃度, 1 BPM = 1 LLM 调用
2. 记忆 (Memory)      - 5 表行数 + 容量使用率
3. 神经 (Neurons)     - 24 trait 互锁状态 (per archive/r14/0003-trait-interlock-22-enum.md)
4. 免疫 (Immune)      - m3 5 道防御状态 (per docs/stage4/m3-hallucination-defense-2026-08-05.md)
5. 呼吸 (Breath)      - token bucket 状态 (per D-04)
6. 听觉 (Ear)         - 6 工具 call 频率
7. 视觉 (Eye)         - 6 工具 read 频率
8. 触觉 (Touch)       - 6 工具 write 频率 (1.0 release 501 stub)
9. 智慧 (Wisdom)      - ASI score + V0.5 24 维
```

### 8.2 改瘦前后技术对比 (per §2.1)

| 维度 | 改瘦前 (R20 阶段 1-3) | 改瘦后 (R25 阶段, 1.0 release) | 改瘦原因 |
|---|---|---|---|
| 集成方式 | TUI → core 直调 lib | TUI → HTTP client → API | 共享 backend, Tauri 来了无缝换 |
| 启动 | 同进程, 共享内存 | TUI 起 API 子进程, 走 localhost:8080 | Tauri 来了同模式 |
| 共享 backend | ❌ TUI 独享 core | ✅ TUI + Tauri 共享 HTTP API | Tauri 不重写 UI 层 |
| 集成测试床 | ❌ TUI 测 core | ✅ TUI 测 API, Tauri 测一样 | 测 1 套覆盖 2 端 |
| 集成测试 | 估补, 跟 core 测 | ✅ 25+ 测试 (ratatui TestBackend) | 测 TUI 设计契约 |
| SSE 流式 | 直调 MemoryChannel | 走 reqwest stream feature | TUI / Tauri / 第三方共享 |
| 1 RTT | 0 | +1 (1-5 ms) | 不可察 |

### 8.3 TUI 集成测试 25+ 套 (per `crates/apeireth-tui-e2e/`)

| 测试类型 | 数量 | 工具 |
|---|---|---|
| 5 nav 切换渲染 | 5 | ratatui TestBackend |
| 9 器官状态卡片 | 9 | ratatui TestBackend |
| SSE 流式渲染 | 3 | wiremock + reqwest |
| 6 工具调用面板 | 6 | wiremock + reqwest |
| 错误处理 | 2 | anyhow + ratatui |
| **合计** | **25** | |
