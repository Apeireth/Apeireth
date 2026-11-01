# apeireth-ui-kit

Apeireth 全栈纯 Rust 前端引擎的**组件库**：共享组件的 Rust 重写 + 组件注册表。

## 在引擎里的位置

引擎依赖拓扑（ADR-0015 §3.3，单向下行无环）：

```
ui-macro(0) ← ui-core(1) ← {ui-dom(2), ui-gpu(2)} ← ui-kit(3) ← ui(4, 门面) ← ui-demo(5)
```

`apeireth-ui-kit` 是**拓扑秩 3**：

- 只回答「**这块界面长什么样**」，**不含业务数据获取**；
- 组件是**纯函数** `fn(&Props) -> VNode` ⇒ native 下直接调用即可断言虚拟树，**不需要浏览器、不需要 wasm**（这是「38 处源码镜像测试」的根解法：断言从「读源码文本」升级为「钉行为」）。

## 已实现（IMPLEMENTED：库级已实现，未接线进生产路径）

- **按钮**（`Button`）：class 名由调用方给（class 名是视觉契约，组件库**不发明** class 名）；
- **徽标**（`Badge`）：class 名 + 条件态追加 class + 悬浮提示；
- **卡片壳**（`CardShell`）：容器 + class 名 + 可选角色 / 无障碍标签；
- **图标**（`Icon`）：SVG 结构（标签 / 属性 / 属性顺序）+ **构建期抽取**的几何数据（`icon_data`，带上游许可署名）；
- **组件注册表**（`ComponentRegistry`，契约登记的 UI 组件扩展点）：类型擦除的组件索引、按名渲染、props 类型不匹配的显式报错；
- **工具卡**（`ToolCallCard`）与**工具调用生命周期卡**（`ToolCallLifecycleCard`）：结构 / class 名 / 文本节点逐条对齐生产组件；
- **JSON 值类型与序列化器**（`json`）：对象**保插入序**，序列化结果与脚本侧**逐字节相同**；
- **脚本侧数值 / 文本语义**（`num`）：数值转文本、固定小数位、取整、UTF-16 长度与切片；
- **设置面与工具网格面**的组件（`settings_*` / `tool_grid` / `status_bar` / `governance_tab`）。

## 未接线（诚实边界）

- **其余共享组件**（列表 / 输入 / 对话框 / 开关 / 标签页等）：按迁移批逐面完成（class 名须与被替换实现**逐字一致**）。
- **组件的具名插槽**：不实现（普通泛型函数参数即可表达）。

## 纪律

- 只出现 **class 名与结构**，**不出现颜色字面量与时长字面量**（那些归 CSS 资产与设计令牌）；
- 组件里**不出现业务数据获取**（业务语义归 `ui-demo` 与后端契约）；
- **不设 `id`**、**不写内联样式**。

## Install

```toml
[dependencies]
apeireth-ui-kit = "2.0.0-rc.4"
```

## License

Apache-2.0 OR MIT

## Links

- crate 目录：`crates-ui/apeireth-ui-kit`
- 契约登记：`docs/architecture/contract.toml` `[extension_points]`（`ui_component`）
- 决策真源：`docs/adr/0015-pure-rust-ui-engine.md`
