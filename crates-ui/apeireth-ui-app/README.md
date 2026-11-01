# apeireth-ui-app — 引擎**产品工程**

> 新前端的**实现目标工程**（交接包 §0：路径与工程基线由引擎线指定）。
> 完整规格见 [`docs/04-internal/ui-engine-product-baseline.md`](../../../docs/04-internal/ui-engine-product-baseline.md)。

## 三句话说清定位

1. **它是产品宿主**：屏幕装配 + 运行时契约（挂载 / 重渲染 / 销毁）；
2. **它不是试点**：`apeireth-ui-demo` 是试点载体兼装配级冒烟（ADR-0015 §4），
   被证据装置与 CI 引用 ⇒ 职责不同，**不合并**；
3. **它不是引擎**：`core` / `dom` / `gpu` / `kit` 是引擎层（ADR-0015 增补的三层边界），
   本 crate 只做**装配与宿主**。

## 依赖方向（拓扑秩 6，最高）

```
app  →  ui（门面）  →  core / dom / gpu / kit  →  …
```

⚠️ 本 crate **不得**被引擎层反向依赖。

## 当前状态（诚实口径）

| 项 | 状态 |
|---|---|
| 契约层（`Screen` / `HostState` / 信号接线） | ✅ IMPLEMENTED + native 测试 |
| wasm 薄壳（接真实 DOM 的入口） | ⬜ **未建**（骨架阶段不引入未验证路径；WebView2 验证未做） |
| 屏幕（具体业务界面） | ⬜ 未开始（由视觉线 W0 设计产出驱动） |
