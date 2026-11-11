# 状态条面对照 harness（**状态条面 · 并行面 A**）

生产 Svelte 组件（**只读引入**）与 Rust 引擎组件的**逐像素 / DOM 逐字 / 逐帧**
对照装置。

## 两侧

| 侧 | 入口 | 内容 |
|---|---|---|
| **参考侧** | `reference/statusbar/index.html` | 生产 `StatusBar.svelte` + `StatusBadge.svelte`（**只读 import**）+ 同一份固定数据 |
| **引擎侧** | `engine/statusbar/index.html` | wasm 引擎渲染 Rust 的 `StatusBar` / `StatusBadge` + 同一份固定数据 |

## 目录

```
statusbar/
├── face.manifest.json              # 抽取清单（单一真源）
├── scripts/extract-statusbar-assets.mjs   # 本地抽取器（**不改共享抽取器**）
├── fixtures/statusbar-cases.json   # 固定数据（两侧喂**同一份**）
├── shared/statusbar-api.js         # 两侧**同名同义**的控制 API（**同一份源码**）
├── reference/statusbar/            # 参考侧（Svelte）
├── engine/statusbar/               # 引擎侧（wasm 装载）
├── engine-host/                    # **独立** wasm 宿主 crate（自带 [workspace]）
├── public/                         # 抽取产物（CSS）+ 对照专用样式
├── tools/compare_statusbar.py      # 对照驱动（T1/T2/T3/T4）
├── evidence/                       # 判据 + 两张画布 + 控制台日志
└── out/                            # 中间产物（不入库）
```

## 对照项

| # | 对照项 | 判据 |
|---|---|---|
| T1 | 静态像素 | 每用例差异占比 **≤ 1%**（阈值：任一通道绝对差 > 2/255） |
| T2 | DOM | 归一化后**完全相等**（文本节点**逐字**；剔除 `svelte-*` class、注释锚点；class token 与属性键排序） |
| T3 | 逐帧 | 悬停过渡期间每帧差异 **≤ 1%**；且**过渡两侧都真的起播**、采集窗口**真的覆盖**了动画 |
| T4 | 点击回调 | 点第 0 格 ⇒ 两侧回调计数都 **+1**（证明 Rust 侧回调**真的**接上了） |

## 跑法

```bash
bash build.sh          # 抽取 → wasm → vite → 对照（一步到位）
```

或分步：

```bash
node scripts/extract-statusbar-assets.mjs          # 写入产物
node scripts/extract-statusbar-assets.mjs --check  # 只校验（漂移 ⇒ 退出码 1）
python3 tools/compare_statusbar.py                  # 只跑对照
```

依赖：系统 Chromium、`websocket-client`、Pillow、Node（**22.x**）、wasm-bindgen-cli
0.2.129。**不需要** Playwright。

## 为什么自带 vite 配置与 wasm 宿主 crate

- 共享的 `harness/vite.config.mjs` 与 `harness/scripts/` 是**共享点**（别的面正在
  施工）⇒ 本面**不改** ⇒ 自带 `vite.config.mjs`。
- 共享的 `apeireth-ui-demo` 用 `window.__APP_MODE__` 挂多种应用形态 ⇒ 它的
  `src/main.rs` 是**共享点** ⇒ 本面**另起** `engine-host/`（自带 `[workspace]` ⇒
  不进 `crates-ui` 的依赖图、不进 deny 图、不进 CI 门禁；形状照
  `crates-ui/evidence/p5-b/harness/`）。

## 资产作用域化的两条口径

| 组件 | `scope` | 理由 |
|---|---|---|
| 状态条 | `.status-bar`（前缀法） | 它的规则（`.sb-item` 等）都命中**后代** ⇒ 前缀法正确 |
| 状态徽标 | `null`（**不作用域化**） | 它的变体规则（`.amber` 等）作用在**根元素自身** ⇒ 前缀法会改写成后代选择器 ⇒ **样式全丢** |

⚠️ 抽取器必须**先把注释摘出来**再判定「选择器是否以根 class 开头」：否则
「根规则前的注释」会让根选择器被改写成 `.根 .根`（后代）⇒ 根样式整条丢失。
（共享抽取器的 `scopeCss` **没有**这一步 ⇒ 那是一条**潜伏的**共享缺陷：批一/批二
的组件样式块里恰好没有注释 ⇒ 没被触发。）

## 诚实边界

- 本 harness 的 wasm 宿主**未经 native 侧验证**（`wasm_app` 整体被
  `#[cfg(target_arch = "wasm32")]` 隔离）⇒ 它的行为**只由浏览器内对照覆盖**。
- 引擎侧喂的是**固定 fixture** ⇒ **真后端未接线**（引擎不碰 HTTP / SSE 契约）。
- 状态徽标的 variant class 在引擎侧**未作用域化** ⇒ 生产合流前需要主线决定
  作用域策略（建议：给引擎侧 DOM 加 scope class，并把 variant 规则改写成复合
  选择器）。
