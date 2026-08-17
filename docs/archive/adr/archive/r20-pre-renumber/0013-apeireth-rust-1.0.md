# ADR 0013: Apeireth-rust 1.0 release 收官

> **状态**: 🟢 Accepted (主人 2026-08-05 拍板)
> **commit 锚**: `02d5db6c` (1.0 release 12 项 checklist 落定) + `ae7bd2e5` (整合 #2)
> **最后更新**: 2026-08-05

---

## 1. 背景 (Context)

Apeireth R14 Rust 重写经过 5 阶段（灵感 → 想法 → 图纸 → 落实 → 施工），在 R17 ~ R20 阶段进入产品化 + 1.0 release 收口期。

**问题陈述**:
- R20 阶段 1 完成 14 new crate 入 workspace (5 P0 + 9 skeleton)
- R20 阶段 2-6 需补 1.0 release 12 项 checklist
- 缺一个总览 ADR 说明"1.0 release 是什么"和"做了什么"

**决策驱动**:
- ✅ 给后续接手者一个清晰的"1.0 release 全景"
- ✅ 锁定 1.0 release tag = `v1.0.0` 计划 2026-09-30
- ✅ 公开对外的 release notes / README / CHANGELOG 配套

---

## 2. 决策 (Decision)

**Apeireth-rust 1.0 release = 14 new crate + 6 工具 endpoint + 5 Provider + 8 包齐发 + 12 项 checklist 收口**

具体:
- **workspace v1.0.0** (semver 严守)
- **rust-version 1.80** (stable)
- **license Apache-2.0** (per 1.0 release #11)
- **58 → 67 crate** in workspace
- **5 Provider 客户端** (1 真接 + 4 stub)
- **6 工具 endpoint** (calendar / message / contact / task / search / drive)
- **3 observability 端点** (metrics / health / status)
- **WebSocket 8 帧** 流式 LLM
- **8 包齐发** (deb / rpm / brew / scoop / tarball / zip / MSI / Docker)
- **5 鉴权组件** (token / refresh / scope / expire / refresh-on-use)
- **3 档限流** (Global / Per-User / Per-Tool)
- **6 哲学锚 + 8 项不修改承诺** 严守贯穿

---

## 3. 后果 (Consequences)

### 3.1 正面

- ✅ **1.0 release 全景清晰**: 14 + 6 + 5 + 8 + 12 = 5 大块一目了然
- ✅ **可演示给主人**: 主人 2026-08-05 拍板"先做好后端", 1.0 release 满足
- ✅ **可对外发布**: GitHub release + crates.io + Docker Hub
- ✅ **R20 阶段 1-6 闭环**: 5/6 阶段全部收口
- ✅ **12 项 checklist 进度可追踪**: per `reports/r20-v1.0.0-release-checklist-2026-08-05.md`

### 3.2 负面

- ⚠️ **2 P0 fail 阻塞 tag**: 1.0 release 12 项 #2 test (cargo test) + #7 perf (P95 < 2s) 阻塞 release tag
- ⚠️ **4 P0 Provider stub**: 5 Provider 客户端只 1 真接，4 stub 留 R21
- ⚠️ **6 工具写操作 stub**: create/update/delete 写操作 1.0 全 501
- ⚠️ **TUI 没接通 9 organs 视觉化**: per 主人拍板"先做好后端"
- ⚠️ **Tauri 没真接**: per 主人拍板"缺审美设计前不上 Tauri"

### 3.3 风险

- 主人 2026-09-30 前必须解决 2 P0 fail
- R21 商业化版必须补 4 P0 Provider + 写操作 501 → 真接

---

## 4. 备选 (Alternatives Considered)

### A. 推迟 1.0 release 到 R21 之后
- 优点: 2 P0 fail + 4 Provider stub + 6 工具写操作 全解决
- 否决: 主人 2026-08-05 拍板"先收 1.0, 商业化走 R21", 推迟会丢失 momentum

### B. 仅发 0.9.21 alpha (per 现状 v2.0.0-alpha)
- 优点: 无 2 P0 fail 风险
- 否决: workspace.version 已锁 v1.0.0, semver 严守; alpha 0.9 路径被主人否决

### C. 1.0 release = TUI + Tauri 全接通
- 优点: 完整前端体验
- 否决: 主人 2026-08-04 拍板"缺审美设计, Tauri 暂搁, TUI 自己干", 1.0 release 后端先收

---

## 5. 6 哲学锚穿透

- ✅ **S-1 走在前人经验上**: semver / Keep a Changelog / MADR 业界惯例
- ✅ **S-2 实事求是**: 2 P0 fail 诚实登记, 不假装
- ✅ **O-2 用户看结果不看哲学**: 1.0 release 对外只展示可用功能, 内部 6 哲学锚不暴露
- ✅ **O-3 信息密度"高"**: 5 段结构 (14/6/5/8/12) vs 散文
- ✅ **O-4 干净状态 = 没有历史包袱**: 拒绝"先做后改"
- ✅ **O-5 6 哲学锚穿透**: 本节自检

---

## 6. 8 项不修改承诺

- ✅ **不假装已实现**: 2 P0 fail + 4 Provider stub + 写操作 501 全部诚实标注
- ✅ **编译期 hardcode**: 14 crate workspace v1.0.0, semver 严守
- ✅ **不改 LOCKED**: 7 LOCKED 文档 + 24 LOCKED crate 全保留
- ✅ **不改 workspace version**: v1.0.0 严守
- ✅ **6 哲学锚穿透**: §5 自检
- ✅ **不依赖 NewAPI**: 5 Provider 客户端自建
- ✅ **不重复造轮子**: 用 semver / Keep a Changelog / MADR
- ✅ **诚实标缺**: 2 P0 fail + 4 Provider stub + 写操作 501 全部明确标 TODO R21

---

## 7. 引用

- workspace: `Cargo.toml` v1.0.0 (line 121)
- CHANGELOG: [`CHANGELOG.md`](../../CHANGELOG.md)
- README: [`README.md`](../../README.md)
- 12 项 checklist: [`docs/1.0-release/checklist.md`](../1.0-release/checklist.md)
- 蓝图: `docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md`
- 阶段 2-3 准备: `docs/stage4/r20-stage-2-3-prep-2026-08-05.md`
- 12 项拍板事 ID: `docs/stage4/pending-decisions-overview-2026-08-05.md`
