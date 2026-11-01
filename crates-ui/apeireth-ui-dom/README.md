# apeireth-ui-dom

Apeireth 全栈纯 Rust 前端引擎的 **DOM 绑定层**：虚拟节点 / 后端抽象 / 差分 / 事件委托 / 双向绑定。

## 在引擎里的位置

引擎依赖拓扑（ADR-0015 §3.3，单向下行无环）：

```
ui-macro(0) ← ui-core(1) ← {ui-dom(2), ui-gpu(2)} ← ui-kit(3) ← ui(4, 门面) ← ui-demo(5)
```

`apeireth-ui-dom` 是**拓扑秩 2**，是引擎里**唯一**接触 DOM 的一层：

- 只回答「**状态变成 DOM**」；不回答状态从哪来（那是 `ui-core` 与数据层）；
- 与 `ui-gpu` **平级**（不是父子）：DOM 显影与 GPU 显影是同一份状态的两条**正交**出路。

## 为什么 DOM 操作必须被抽象在 trait 后面

差分 / 委托 / 绑定 / 渲染逻辑全部写成泛型代码（`D: Dom`）⇒ 100% 可 native 单测，**不需要浏览器、不需要 wasm target**。这是「消灭源码镜像测试」的结构性前提。

## 已实现（IMPLEMENTED：库级已实现，未接线进生产路径）

- **虚拟节点**（元素 / 文本 / 分组）+ 元素节点的身份 key（`Key`）+ **属性值语义**（布尔 `false` ⇒ 不写该属性）、class token 合并；
- **差分**：class 按 token + 属性键级 + 子节点按索引对齐（unkeyed 形态）；
- **keyed 列表差分**：按 key 配对 + **最长递增子序列（LIS）**最小移动；
- **事件委托表**（目标 → 祖先的冒泡顺序 + `stop_propagation`）；
- **双向绑定**（`bind:value` / `bind:checked` 的两条方向语义）+ **`bind:this`**（挂载写引用 / 卸载前清引用）；
- **内存 DOM 后端**（`InMemoryDom`，native 永远可用）+ **操作序列记录**（`DomOp`）+ **真实 DOM 后端**（`WebDom`，web-sys，**wasm32 目标专用**）；
- **渲染器**（`Renderer`）：把 keyed 差分计划落成实际 DOM 操作序列，挂载 / 卸载**成对执行**；
- **命名空间建节点**（`create_element_ns`）⇒ SVG 元素建在 SVG 命名空间里；
- **失败显式化**：`Dom` 的每个操作都返回 `Result`（五类错误各自分明）⇒ **没有静默降级**。

## 未接线（诚实边界）

- **事件委托的单监听器形态**：本批按**元素直挂**（每个带处理器的元素一个监听器）。
- **真实 DOM 后端的运行时验证**：`WebDom` 被 `#[cfg(target_arch = "wasm32")]` 隔离 ⇒ **native 编译不参与**。浏览器内的验证（真实 DOM 行为 / 像素对照 / 逐帧对照）由试点对照 harness 承担（`crates-ui/apeireth-ui-demo/harness/`）。
- **事件委托接到真实 DOM**：`Dom` trait 目前**没有**监听器接口 ⇒ 委托表与真实 DOM 之间**没有连线**；事件语义仍只在 native 的委托表层面可测。
- **双向绑定的写回接线**：两条方向语义**已实现（IMPLEMENTED）**且可测，但「渲染阶段把信号值写进 DOM property」这一步**尚未接进渲染器**。
- **wasm 侧微任务调度器**：实现在 wasm 宿主侧，不在本 crate。

## 外部依赖（仅 wasm32 目标参与编译）

`wasm-bindgen` / `js-sys` / `web-sys` **精确硬钉**（web-sys 对 wasm-bindgen 是 1:1 硬钉，错配在依赖解析期直接失败）：`=0.2.129` / `=0.3.106` / `=0.3.106`。

## Install

```toml
[dependencies]
apeireth-ui-dom = "2.0.0-rc.4"
```

## License

Apache-2.0 OR MIT

## Links

- crate 目录：`crates-ui/apeireth-ui-dom`
- 决策真源：`docs/adr/0015-pure-rust-ui-engine.md`；门禁：`.github/workflows/ui-engine-ci.yml`
