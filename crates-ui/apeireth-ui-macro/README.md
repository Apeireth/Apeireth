# apeireth-ui-macro

Apeireth 全栈纯 Rust 前端引擎的**过程宏**：`#[component]`（函数即组件）与 `view!`（虚拟树构造）。

## 在引擎里的位置

引擎依赖拓扑（ADR-0015 §3.3，单向下行无环）：

```
ui-macro(0) ← ui-core(1) ← {ui-dom(2), ui-gpu(2)} ← ui-kit(3) ← ui(4, 门面) ← ui-demo(5)
```

`apeireth-ui-macro` 是**拓扑秩 0** 的最底层 crate：

- **零内部依赖、零外部依赖**（只用 std 的 `proc-macro`）；
- 职责边界：**只做「语法 → 语法」的翻译**，**不产生运行时逻辑**；
- 宏展开后需要的运行时辅助代码一律住在 `ui-core` / `ui-dom`。

## 已实现（IMPLEMENTED：库级已实现，未接线进生产路径）

- **`#[component]`** — 「函数即组件」：展开为「props 结构体 + 组件类型 + 构造器 + 按名构造器 + `UiComponent` 实现」。两种构造形态：

  | 形态 | 用途 | 缺参数时 |
  |---|---|---|
  | `Name::new(必填…)` | 位置构造 | 编译期缺参数（少传就编译不过） |
  | `Name::builder().字段(值)….build()` | 按名构造（`view!` 用） | 运行期 `expect` 报错（消息带参数名） |

- **`view!`** — 虚拟树构造宏：展开成**运行时路径**上的构造调用（`::apeireth_ui_dom::VNode::element(...)` 等）⇒ 展开物里**没有**任何运行逻辑被复制。
- **手写 token 解析**：**不引 `syn` / `quote`**（会带入 `proc-macro2` / `unicode-ident` 传递依赖）⇒ 语法面窄但错误可读；语法层是**纯文本函数** ⇒ 可单元测试。

## 未接线（诚实边界）

- **双向绑定 `bind:value` / `bind:checked`**：编译期**显式报错**（不是静默忽略）—— 渲染阶段的 property 回写尚未实现。
- **泛型 props**：不支持（避免类型推断地狱）。
- **具名插槽**：不实现（用普通泛型函数参数表达）。

## 运行时路径约定（重要）

展开物引用 `::apeireth_ui_core::…` 与 `::apeireth_ui_dom::…`（**绝对路径**）⇒ 使用这两个宏的 crate 须把 `apeireth-ui-core` / `apeireth-ui-dom` 列为**直接依赖**（门面本身不参与展开物的路径解析）。

## Install

```toml
[dependencies]
apeireth-ui-macro = "2.0.0-rc.4"
```

## License

Apache-2.0 OR MIT

## Links

- crate 目录：`crates-ui/apeireth-ui-macro`
- 决策真源：`docs/adr/0015-pure-rust-ui-engine.md`
- 门禁：`.github/workflows/ui-engine-ci.yml`（本 workspace 独立 CI）
