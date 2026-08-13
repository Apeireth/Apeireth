# R208 VCP 5 类高层分类进 tool-registry (R185 调研推荐)

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R208
> **日期**: 2026-08-13
> **来源**: R185 VCP 官网调研
> **状态**: 实施完成, 10/10 单测全过 (累计 110/110)

---

## 0. 背景

apeireth-tool-registry/types.rs 现有 6 类 ToolKind enum (Sync/Async/Static/Service/MessagePreprocessor/Hybridservice, 字段级引用 VCP §6.2.1).

R185 VCP 官网调研提到 5 类插件 (工具/直觉反射/服务/消息预处理/消息分发, 描述性分类).

R208 加 VcpCategory 5 类高层 enum, 把 6 类 ToolKind 映射到 5 类.

---

## 1. 设计

### 1.1 VcpCategory 5 类

`
ust
pub enum VcpCategory {
    Tool,                 // 常规可调用
    ReactiveIntuition,    // 常驻 hook 自动响应 (systemd 风格)
    Service,              // 常驻后台
    MessagePreprocessor,  // 拦截 + 修改 (VCP 核心业务区)
    MessageDispatcher,    // 路由到不同 Agent/前端
}
`

### 1.2 6 类 -> 5 类映射

- Sync / Async / Static -> Tool
- Service / Hybridservice -> Service
- MessagePreprocessor -> MessagePreprocessor

### 1.3 API

- VcpCategory::COUNT = 5 编译期 hardcode
- VcpCategory::ALL[5]
- s_str() / s_legacy_str()
- rom_tool_kind(kind: ToolKind) -> VcpCategory

---

## 2. 0 触碰声明

- 3 不可变脊柱: 0 触碰
- workspace.version 1.2.0: 0 改
- types.rs 现有 6 类 ToolKind: 0 改
- 现有 7 模块 (async_task/classifier/registry/token_budget/trait_def/types + lib): 0 改
- lib.rs 改 1 行: pub mod vcp_category

---

## 3. 测试 (10/10 pass, 累计 110/110)

- t01: COUNT 5
- t02: as_str
- t03: as_legacy_str (借鉴 VCP 原名)
- t04-t09: from_tool_kind 6 个映射
- t10: all 5 类别 distinct

---

## 4. 中期路径 (R208+1 候选)

- 集成进 classifier.rs (按 VcpCategory 自动选择)
- 加 ReactiveIntuition trait (事件驱动 hook 抽象)
- 加 MessageDispatcher trait (路由抽象)