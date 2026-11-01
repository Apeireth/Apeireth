# apeireth-ui-demo

Apeireth 全栈纯 Rust 前端引擎的**试点应用**（审批卡 / 工具卡面 / 设置面）兼**装配级冒烟**。

## 在引擎里的位置

引擎依赖拓扑（ADR-0015 §3.3，单向下行无环）：

```
ui-macro(0) ← ui-core(1) ← {ui-dom(2), ui-gpu(2)} ← ui-kit(3) ← ui(4, 门面) ← ui-demo(5)
```

`apeireth-ui-demo` 是**拓扑秩 5** 的最上层试点载体：

- 依赖门面 + 组件库；**不依赖任何主仓 crate**；
- 组件 = Rust 函数 ⇒ 整条渲染链在 native `cargo test` 里直接跑（这是「38 处源码镜像测试」的根解法：断言从「读源码文本」升级为「钉行为」）。

## 已实现（IMPLEMENTED：库级已实现，未接线进生产路径）

- **审批卡试点组件**：class 名与生产实现**逐字一致**；结构、属性、文本节点逐条对齐；
- **`view!` 虚拟树宏**的实际使用 + 组件库（按钮 / 徽标 / 卡片壳 / 图标）的实际消费；
- **四条交互**：允许 / 拒绝 / 关闭（**只关卡片、不做业务决策**）/ 复制；
- **固定数据**：两侧喂同一份 fixture ⇒ 差异只可能来自引擎；
- **工具卡面与设置面**：工具卡 / 工具网格 / 状态栏 / 治理页 / 设置各页的 Rust 宿主侧实现。

## 未接线（诚实边界）

- **真后端**（`/v1/apeireth/approval-requests` + SSE `approval_required`）：引擎不碰契约 ⇒ 数据面归既有后端，试点只喂固定 fixture。
- **浏览器内对照**（像素 / 逐帧 / DOM 全等）：由试点 harness 承担，**不是**本 crate 的 `cargo test` ⇒ native 侧只断言虚拟树与事件行为。
- **宿主能力**（钥匙串 / 本地存储 / HTTP）：本模块**不实现** ⇒ 生产侧仍是「未接线」的真实状态，试点只把**意图**（`on_complete` / `on_skip`）暴露出去。

## 发布状态（owner 裁决）

- **`publish = false`**：试点应用**不上 crates.io**（owner 裁决）。
- **`exclude = ["harness/"]`**：`harness/` 是 vite 浏览器对照台（实测打包 332 个文件）⇒ 不进源码包。
- `fixtures/` 是**产品仓（私有）生产源文件的逐字快照副本**（ADR-0017 S2a 跨仓断链修复）⇒ **「生产一改 ⇒ 断言立刻红」的生产漂移检测能力在产品仓建立前暂时失效**（已知临时回归，登记在 ADR-0017；该断言由产品仓承接）。**本 crate 不再直接引用产品仓的文件**——否则分仓后**编译直接失败**。

## Install

本 crate **不发布到 crates.io**（`publish = false`）⇒ 无 crates.io 安装方式；在仓库内以 path 依赖消费：

```toml
[dependencies]
apeireth-ui-demo = { path = "crates-ui/apeireth-ui-demo" }
```

## License

Apache-2.0 OR MIT

## Links

- crate 目录：`crates-ui/apeireth-ui-demo`
- 对照 harness：`crates-ui/apeireth-ui-demo/harness/`（vite + CDP 驱动）
- 决策真源：`docs/adr/0015-pure-rust-ui-engine.md`、`docs/adr/0017-engine-product-repo-split.md`
