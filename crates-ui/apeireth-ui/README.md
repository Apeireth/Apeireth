# apeireth-ui

Apeireth 全栈纯 Rust 前端引擎的**门面**：re-export 响应式核心 / DOM 绑定 / GPU 桥 / 组件库。

## 在引擎里的位置

引擎依赖拓扑（ADR-0015 §3.3，单向下行无环）：

```
ui-macro(0) ← ui-core(1) ← {ui-dom(2), ui-gpu(2)} ← ui-kit(3) ← ui(4, 门面) ← ui-demo(5)
```

`apeireth-ui` 是**拓扑秩 4** 的门面：

- **零逻辑**——门面里出现任何一行非 `pub use` 的实现代码即违规；门面只是「让试点应用
  不必写 5 条 `use`」；
- 依赖 core / dom / gpu / kit / macro **全部下层**。

## 导出的层次

| 层 | 拓扑秩 | 门面里的名字 |
|---|:--:|---|
| 响应式核心（信号 / 派生 / 依赖图 / 调度 / effect） | 1 | `core` |
| DOM 绑定（虚拟节点 / 差分 / 委托 / 绑定 / 渲染器） | 2 | `dom` |
| GPU 桥（状态 → 像素） | 2 | `gpu` |
| 组件库（共享组件 + 注册表） | 3 | `kit` |
| 过程宏（`#[component]` / `view!`） | 0 | `component` / `view` |

常用类型同时在门面根部具名导出（`Signal` / `VNode` / `UiComponent` …），
试点应用通常只需要 `use apeireth_ui::…;`。

## 口径说明（诚实边界）

- **宏的运行时路径约定**：`#[component]` 与 `view!` 的展开物引用
  `::apeireth_ui_core::…` / `::apeireth_ui_dom::…`（**绝对路径**）⇒ 使用这两个宏的
  crate 须把 `apeireth-ui-core` / `apeireth-ui-dom` 列为**直接依赖**
  （门面本身**不参与**展开物的路径解析）。
- **真实 DOM 后端（`WebDom`）只在 wasm32 目标存在** ⇒ 门面的具名导出也按目标条件化
  （native 编译时**没有**这个名字，不是「导出了但不能用」）。
- 门面**不提供**任何渲染实现——那在各层 crate 里。

## Install

```toml
[dependencies]
apeireth-ui = "2.0.0-rc.4"
```

## License

Apache-2.0 OR MIT

## Links

- crate 目录：`crates-ui/apeireth-ui`
- 决策真源：`docs/adr/0015-pure-rust-ui-engine.md`
- 门禁：`.github/workflows/ui-engine-ci.yml`（本 workspace 独立 CI）
