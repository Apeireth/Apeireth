# 引擎侧 · 页面框架与导航（`frame`）

> 本目录是**设置页 · 框架面**的引擎侧入口目录。

## 本目录里有什么

| 文件 | 归属 | 说明 |
|---|---|---|
| `index.html` | **生成物**（`scripts/gen-entry-pages.mjs`） | 引擎侧入口页 ⇒ **勿手改**（改了就漂移 ⇒ `pages:check` 报红） |

## 引擎侧怎么被唤起

```
web/engine/main.js（**共享文件**）
  ├─ window.__APP_MODE__ = "settings:frame"      ← 必须在 wasm 初始化**之前**设定
  ├─ import('/engine/pkg/apeireth-ui-demo.js')    ← 构建期产物（wasm-bindgen 胶水）
  ├─ window.__SETTINGS__ = createSettingsApi(...) ← 与参考侧**同名同义**
  └─ await module.default()                       ← wasm 初始化 ⇒ Rust 的 main()
       └─ wasm_app::start()（**共享**）
            └─ crate::settings_dispatch::mount("frame", &container)（**共享**）
                 └─ crate::settings_frame::wasm::mount(container)  ← **本面实现**
```

⇒ **本面只需实现 `crate::settings_frame::wasm::mount(container)`**
（`crates-ui/apeireth-ui-demo/src/settings_frame.rs`）。

## 组件与数据

| 项 | 落点 |
|---|---|
| 组件（展示面，**纯函数**） | `crates-ui/apeireth-ui-kit/src/settings_frame.rs`（`SettingsFrame`） |
| 宿主（导航三态 + 固定数据） | `crates-ui/apeireth-ui-demo/src/settings_frame.rs`（`FrameState` + `data`） |
| 固定数据 | `crates-ui/apeireth-ui-demo/harness/settings/fixtures/nav-groups.json`（**两侧同一份**） |
| 图标几何数据 | `ui-kit/src/settings_frame.rs` 的 `icons` 模块（**构建期抽取的生成物**） |
| 作用域 CSS | `public/settings-frame.css`（**构建期抽取**，根 class `.settings-view`） |

## 几何（**本面是几何基准** ⇒ 其它内容面对齐的依据）

两侧**实测**（证据：`evidence/frame/geometry.json`）：

| 事项 | 参考侧 | 引擎侧 |
|---|--:|--:|
| 侧栏 `.settings-subnav` 宽 | **200px** | **200px** |
| 内容列 `.settings-content` border-box 宽 | **900px** | **900px** |
| 内容盒（900 − 36 × 2） | **828px** | **828px** |
| 侧栏 / 内容列顶边 y | **117.53125** | **117.53125** |

⇒ **宽度同源**成立 ⇒ 其它内容面（M4–M7）裁剪 `.setting-block` 时两侧都是 **828px**
⇒ 文字换行位置一致 ⇒ 逐像素对照才有意义。

⚠️ **顶边 y = 117.53125 是页头带来的**（生产的 `.page-head` 高 117.53px）
⇒ 引擎侧由**宿主注入同形页头节点**（`PageHeader` 组件**未迁**）
⇒ 否则导航行落在**整数相位**上 ⇒ 两侧同一批字形的**抗锯齿**不同
⇒ 逐像素对照会出现**约 2% 的假差异**（与字形无关；实测过）。

## 裁剪口径

- 参考侧 / 引擎侧裁剪选择器：**`.settings-subnav`**（两侧同名）。
- 逐像素对照裁的是**侧栏的导航内容**（最后一个 `.subnav-group` 的底边）
  ⇒ 理由：参考侧的内容列装着**别的面**的面板 ⇒ 网格行把侧栏拉伸到
  1271.92px；引擎侧本面的内容列没有别的面板 ⇒ **整框高度两侧必然不等**
  ⇒ 逐像素只针对导航内容，**宽度**仍按元素框断言。

## 对照脚本

`tools/compare_settings.py` 是**共享文件**（切分线的骨架，`collect_pair()` 仍是
`NotImplementedError`）⇒ **本面不改共享文件** ⇒ 对照驱动放在
`target/frame-scratch/compare_frame.py`（scratch 区，**复用**批一的浏览器
驱动）⇒ 见报告里的「需要主线改的共享文件」清单。

## 诚实边界

- 数据 = **固定 fixture** ⇒ 引擎不碰 HTTP / SSE 契约。
- `PageHeader` / `ErrorSolutionBanner` 是**外部组件** ⇒ **未迁**（页头由宿主
  注入**同形节点**只为几何对齐；错误横幅位留空）。
- 内容列归**各内容面**（M4–M7）⇒ 本面**不拥有**任何内容区。
- `selectSection('runtime')` 触发的深度诊断（HTTP）⇒ **宿主能力** ⇒ **不迁**。
