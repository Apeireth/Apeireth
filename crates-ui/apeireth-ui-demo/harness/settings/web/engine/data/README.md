# 引擎侧 · 数据与用户（`data`）

> 本目录是**设置页 · 数据子面**的引擎侧入口目录。

## 本目录里有什么

| 文件 | 归属 | 说明 |
|---|---|---|
| `index.html` | **生成物**（`scripts/gen-entry-pages.mjs`） | 引擎侧入口页 ⇒ **勿手改**（改了就漂移 ⇒ `pages:check` 报红） |
| `compare_data.py` | **本面独占** | **面级对照驱动**（像素 + DOM 全等 + 宽度同源 + 意图断言） |
| `vite.data.config.mjs` | **本面独占** | **面内 vite 配置**（面内替身 + 面内补丁样式 ⇒ 见下） |
| `settings-data-face.css` | **本面独占** | **抽取器漏掉的那一条规则**的补丁（逐字取自生产） |
| `mock/*.js` | **本面独占** | **面内替身**（由共享的生成物派生 ⇒ 见下） |

## 引擎侧怎么被唤起

```
web/engine/main.js（**共享文件**）
  ├─ window.__APP_MODE__ = "settings:data"      ← 必须在 wasm 初始化**之前**设定
  ├─ import('/engine/pkg/apeireth-ui-demo.js')  ← 构建期产物（wasm-bindgen 胶水）
  ├─ window.__SETTINGS__ = createSettingsApi(...) ← 与参考侧**同名同义**
  └─ await module.default()                     ← wasm 初始化 ⇒ Rust 的 main()
       └─ wasm_app::start()（**共享**）
            └─ crate::settings_dispatch::mount("data", &container)（**共享**）
                 └─ crate::settings_data::wasm::mount(container)  ← **本面实现**
```

⇒ **本面只需实现 `crate::settings_data::wasm::mount(container)`**
（`crates-ui/apeireth-ui-demo/src/settings_data.rs`）。

## 组件与数据

| 项 | 落点 |
|---|---|
| 组件（展示面，**纯函数**） | `crates-ui/apeireth-ui-kit/src/settings_data.rs`（`DataPanels`） |
| 宿主（固定数据 + 读数映射 + 意图记录） | `crates-ui/apeireth-ui-demo/src/settings_data.rs`（`DataHostState` + `data_fixture`） |
| 固定数据 | `crates-ui/apeireth-ui-demo/harness/settings/fixtures/data.json`（**两侧同一份**） |
| 图标几何数据 | `ui-kit/src/settings_data.rs` 的 `icons` 模块（**构建期抽取**的生成物） |
| 作用域 CSS | `public/settings-data.css`（**构建期抽取**，根 class `.sp-data`） |

## 两态对照（本面的重点）

| 态 | 参考侧 | 引擎侧 |
|---|---|---|
| **有读数** | 替身喂固定数据的读数 | 宿主喂固定数据的读数 |
| **读不到** | 替身返回 `null` ⇒ 4 行 `—` + 「仅桌面版可读」 | 宿主判定「读不到」⇒ 同样的占位 |

⇒ 参考侧用**页面全局** `globalThis.__DATA_READINGS__` 切换（对照脚本在**装载
页面之前**用 CDP 注入）⇒ 引擎侧调 `window.__DATA__.setReadings(...)`
⇒ **两侧读的是同一份固定数据**。

## ⚠️ 三处**面内绕开**（共享文件的缺陷 ⇒ 只报不改）

| # | 共享文件 | 缺陷 | 本面的绕开 |
|---|---|---|---|
| 1 | `mock/<模块>.js`（生成物） | 生成器把「桩」的**数组**直接 push 進行列表 ⇒ join 出 `{, throw … ,}` ⇒ **语法错误** ⇒ 一旦替身真被解析，vite 构建直接失败 | `mock/*.js`：**面内替身**（同一份导出面 ⇒ 语法正确） |
| 2 | `vite.config.mjs` | 仓库根推算**少一级** ⇒ 替身的 `frontendDir` 指到不存在的目录 ⇒ 替身**永远不被解析** ⇒ 参考侧一直跑**生产真模块** | `vite.data.config.mjs`：**面内配置**（按目录内容推算仓库根 ⇒ 替身真正生效） |
| 3 | `scripts/extract-settings-assets.mjs` | 规则归属判据是「**全部** class 选择器都在本面线索集里」⇒ 一条规则里有多个选择器分支、而分支属于不同面时 ⇒ **整条规则被漏掉**（本面撞上 `.form-group label`） | `settings-data-face.css`：**面内补丁**（只补漏掉的那一条 ⇒ 逐字取自生产） |

⇒ 三条都需要**主线修**（见交付报告的「需要主线改的共享文件」清单）。

## 裁剪口径

- 参考侧 / 引擎侧裁剪选择器：**`.setting-block`** / **`.sp-data .setting-block`**。
- 两侧裁剪框宽度**必须相等**（**828px**）⇒ 不等 ⇒ **直接判失败**。
- ⚠️ **对齐**：参考侧是**生产整页** ⇒ 内容块在文档里的位置**带小数**
  （`x = 236`、`y = 219.765625`）；引擎侧是**独立挂载** ⇒ 位置 `(0, 0)`
  ⇒ 两边的**设备像素格**错开 0.765625px ⇒ 抗锯齿 / 字形栅格化**必然不同**
  ⇒ 那是**装置伪差** ⇒ 对照脚本把引擎侧的舞台**对齐**到参考侧的文档位置。
  ⚠️ **对齐前 / 对齐后两个数字都在报告里**（`alignment` 段）⇒ **不隐藏**。

## 对照脚本

`web/engine/data/compare_data.py` 是**本面独占**的驱动 ⇒ 共享入口
（`tools/compare_settings.py`）按约定**发现并转发**到它：

```bash
python3 tools/compare_settings.py --panel data     # 面级（共享入口）
python3 web/engine/data/compare_data.py            # 本面独立跑（写全量证据）
```

## 诚实边界

- 数据 = **固定 fixture** ⇒ 引擎不碰 HTTP / SSE 契约。
- **危险动作不执行**：引擎侧只发意图（`window.__DATA__.getIntents()` 可读）
  ⇒ 二次确认弹层是**共享覆盖层**（不属本面）⇒ 引擎侧不渲染。
- 头像的**压缩**、工作区目录的**读写**、剪贴板 ⇒ **宿主能力** ⇒ 引擎只发意图。
- 运维读数的**取值**由**宿主算**（生产把这段映射放在 `lib/daemon-ops.ts`
  ⇒ 本模块是它的**同构实现**）⇒ 引擎只渲染。
- 启停指引与零装注记 ⇒ **按生产原文**渲染（共享固定数据里的那两处是**草案**
  ⇒ 与生产不一致 ⇒ 已在交付报告里登记）。
