# apeireth-ui-core

Apeireth 全栈纯 Rust 前端引擎的**响应式核心**：信号 / 派生 / 依赖图 / 调度器 / effect / 作用域。

## 在引擎里的位置

引擎依赖拓扑（ADR-0015 §3.3，单向下行无环）：

```
ui-macro(0) ← ui-core(1) ← {ui-dom(2), ui-gpu(2)} ← ui-kit(3) ← ui(4, 门面) ← ui-demo(5)
```

`apeireth-ui-core` 是**拓扑秩 1** 的纯逻辑层：

- 只回答「**什么时候、哪些东西需要重新算**」；
- **不回答**「算出来之后放到哪」——那是 `ui-dom`（DOM 投影）与 `ui-gpu`（像素投影）两条**平级**的路。

**零外部依赖**（含 0 依赖 web-sys）⇒ 永远能 native 单测，**不需浏览器、不需 wasm target**。

## 已实现（IMPLEMENTED：库级已实现，未接线进生产路径）

- **信号**：读登记依赖 / 写主动失效 / 版本号 / 只读视图 / 等值不触发；
- **派生**：惰性求值 + 主动失效 + 求值环检测（带人名路径）；
- **依赖图**：双向边 + 节点标签 + 条件依赖重收集；
- **调度**：待办双队列（pre / post）+ 去重 + 延迟队列；
- **宿主调度器注入**：`HostScheduler` trait + 同步默认实现 + 手动队列实现（注入是 **trait 边界**，不是依赖 ⇒ 本 crate 的 `[dependencies]` 段仍为 0 条）；
- **effect**：生命周期 + 清理回调（逆序）+ 幂等销毁 + 作用域 LIFO 销毁；
- **批次**：多次写合并为一次 flush（可嵌套）；
- **实例引用槽位**：`InstanceRef`（`bind:this` 的语义内核：挂载写引用、卸载前清空）；
- **组件 trait 边界**：`UiComponent`；**组件回调**：`Callback`（类型擦除的 `Fn()`）。

## 未接线（诚实边界）

- **wasm 侧微任务调度器**：`HostScheduler` 的 trait 边界已就位，但「把 flush 排进浏览器微任务队列」的实现落在 **wasm 宿主侧**（`ui-dom` 的 web-sys 后端 / 试点宿主）⇒ **本 crate 不含该实现**；native 侧用同步默认实现。
- **DOM 写入批次挂载点**：flush 的「渲染前」队列就是 DOM 写入的落点，但**真正的 DOM 写入在 `ui-dom`** ⇒ 本 crate 只给队列与顺序保证。
- **派生闭包的自引用环**：派生闭包若捕获**自身所在运行时的信号** ⇒ `Rc` 引用环 ⇒ 运行时不被释放（**已登记、未修**；修法是改成弱引用，属独立批次）。

## 引擎里不出现业务名词

公开类型里没有会话 / 审批 / presence 之类的概念——一旦出现，引擎就退化成「带业务的项目」。

## Install

```toml
[dependencies]
apeireth-ui-core = "2.0.0-rc.4"
```

## License

Apache-2.0 OR MIT

## Links

- crate 目录：`crates-ui/apeireth-ui-core`
- 决策真源：`docs/adr/0015-pure-rust-ui-engine.md`
- 门禁：`.github/workflows/ui-engine-ci.yml`（本 workspace 独立 CI）
