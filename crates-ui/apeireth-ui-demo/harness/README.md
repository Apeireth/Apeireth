# 对照 harness（P3 试点 + 工具卡面批）

> **这是「组件 = Rust 函数」这条主张的证明性装置**：把 Rust 引擎渲染的卡面与
> 生产前端的 Svelte 卡面**端到端**跑一遍 —— **逐像素** + **逐帧** + **DOM 全等**
> + **JSON 差分**对照。
>
> 本目录含**两套**对照：
>
> | 批 | 对象 | 页面 | 驱动 |
> |---|---|---|---|
> | P3 试点 | 审批卡 `ApprovalRequestCard` | `reference/index.html` / `engine/index.html` | `tools/compare.py` |
> | 工具卡面 | 工具卡 `ToolCallCard` + 工具调用生命周期卡 `ToolCallLifecycleCard` | `reference/tool/index.html` / `engine/tool/index.html` | `tools/compare_toolcard.py` |

## 两侧

| 侧 | 页面 | 内容 |
|---|---|---|
| **参考侧** | `reference/index.html`、`reference/tool/index.html` | 生产组件（**只读引入**，由 vite 编译）+ 同一批设计 CSS |
| **引擎侧** | `engine/index.html`、`engine/tool/index.html` | wasm 引擎（`apeireth-ui-demo`）渲染同样的卡 + 同一批设计 CSS |

两侧喂**同一份**固定数据，跑在**同一个浏览器实例**的同一个视口设定下 ⇒ 差异只
可能来自渲染实现。

## 单一真源（构建期抽取，脚本 `scripts/extract-assets.mjs`）

| 资产 | 来源（**只读**） | 去向 |
|---|---|---|
| 卡片 scoped CSS | 生产组件的 `<style>` 块 | `public/pilot-card.css`（试点：**原文**）、`public/tool-card.css` / `public/lifecycle-card.css`（工具卡面：**作用域化**，见下） |
| 设计令牌 CSS | `frontend/companion-desktop/src/lib/design/*.css` | `public/design/*.css`（两侧同源） |
| 图标几何数据 | 上游图标包的 `dist/icons/*.svelte` 里的 `iconNode` | `crates-ui/apeireth-ui-kit/src/icon_data.rs`（生成物，带上游许可署名） |
| 固定数据 | `fixtures/pilot-request.json`、`fixtures/toolcard-cases.json` | `crates-ui/apeireth-ui-demo/src/fixture_generated.rs`、`…/toolcard_fixture_generated.rs`（生成物） |

**不复制、不手抄、不改生产文件**；生成物可复核：`node scripts/extract-assets.mjs --check`
（漂移 ⇒ 退出码 1）。

### 图标别名跟随

生产按**导入名**取图标（PascalCase ⇒ kebab）；上游包里一部分只有**编译产物**
（`check-circle-2.js` ⇒ 再导出 `circle-check.svelte`）⇒ 抽取器跟随再导出找到
**真实** `.svelte`，并取其 `<Icon name="…">` 作为图标名与 class 后缀
（`lucide-circle-check`）⇒ 与生产渲染出来的 class **逐字一致**。

### 工具卡面的样式作用域化（为什么不是「原文」）

两个组件**共用若干 class 名**（`.tool-header` / `.tool-body` / `.section-label` /
`.code-block` / `.error-text` …）且**同名规则的取值不同**（例：`.tool-body` 的
`gap` 一个 8px、一个 10px）⇒ 组件编译器会给每个组件生成作用域 class 并改写
选择器（`.tool-body.svelte-x`）⇒ 两个组件互不干扰。

引擎页把样式当**普通样式表**注入 ⇒ 没有作用域 class ⇒ 必须**机械作用域化**：
抽取器按「前缀根 class」改写选择器（根自身的规则保持原样；`:global(…)` 保持全局；
`@` 规则原样）⇒ 规则写在 `scripts/extract-assets.mjs` 的 `scopeCss()` 里
（确定性、可复核）。试点组件的样式**原文注入**（页面上没有同名 class 冲突）。

## 一键复跑

```bash
bash build.sh
```

五步：抽取资产 ⇒ 构建 wasm ⇒ wasm-bindgen ⇒ vite build ⇒ 跑对照（试点 + 工具卡面）。

依赖：cargo（wasm32 target）、`wasm-bindgen-cli 0.2.129`、node + vite、
python3（`websocket-client` + `Pillow`）、系统 Chromium。
**不需要** Playwright，**不需要**下载浏览器。

## 对照项与判据

### 试点（`tools/compare.py`）

| # | 对照项 | 判据 |
|---|---|---|
| T1 | 静态像素对照 | 差异像素 / 总像素 **≤ 1%**（任一通道绝对差 > 2/255 即记「不同」） |
| T2 | DOM 结构对照 | 归一化后**完全相等**（D1 结构 / D2 属性 / D3 文本必须为零） |
| T3 | 相位冻结对照 | 5 个相位点（0 / 150 / 300 / 450 / 600 ms），每点 **≤ 1%** |
| T4 | 逐帧对照 | 更新期间每帧 **≤ 1%**（按时间对齐，容差 25 ms） |
| T5 | 动画结构对照 | `getAnimations()` 的动画集合 / 时长 / 延迟 / 关键帧**逐条相等** |
| T6 | 卸载时机对照 | 关闭卡片 ⇒ 两侧在**同一时刻**摘掉节点；且**不产生业务决策** |

### 工具卡面（`tools/compare_toolcard.py`）

| # | 对照项 | 判据 |
|---|---|---|
| T1 | 静态像素对照 | 5 组用例 × 3 态（收起 / 展开 / 展开 + 原始 JSON），每态 **≤ 1%** |
| T2 | DOM 结构对照 | 归一化后**完全相等**（**文本节点逐字**比较，不折叠空白） |
| T3 | 逐帧对照 | 展开动作期间每帧 **≤ 1%** |
| T4 | JSON 差分 | 引擎侧序列化器 vs 浏览器侧 `JSON.stringify`：**逐字节相同**（紧凑 / 缩进 / 超长截断三种形态） |

**环境固定**（设计文档 §9.3 步骤 2）：视口 `1440×900`、`devicePixelRatio = 1`、
时区 `UTC`（两侧取同一面钟）、`prefers-reduced-motion: no-preference`。

## 归一化规则（设计文档 §9.3 步骤 7）

- 剔除 Svelte 作用域 class（`svelte-<hash>`）；
- class token **排序**（token 顺序不影响 CSS）；
- 属性键**排序**；
- 注释锚点**丢弃**（组件编译器的块锚点，不是渲染内容）；
- 纯空白文本节点**丢弃**（flex / inline-flex 容器里不渲染 ⇒ 不影响像素）；
- 其余文本节点**逐字保留**（工具卡面批的判据比试点更严：前导空格等差异会被抓住）。

## 仅测试注入的样式

`public/pilot-test.css`（试点）与 `public/toolcard-test.css`（工具卡面）——
**不进产品资产**，两侧**逐字相同**地链入：

- `html[data-pilot-motion="on"]` ⇒ 卡片入场动画（**挂载时机**的可观测代理）；
- `html[data-pilot-transition="on"]` ⇒ 按钮过渡（**更新时机**的可观测代理）；
- `html[data-*-freeze="1"]` ⇒ 把动画**冻结**在 `--pilot-phase` / `--toolcard-phase` 相位。

⚠️ 两个开关**分开**用：入场动画会让卡片进入合成层（阴影的抗锯齿与层提升有关）
⇒ 与逐帧对照混用会把噪声算进差异里。

## 产物

| 路径 | 内容 |
|---|---|
| `evidence/report.json` | **入库**的试点对照报告（全部数值） |
| `evidence/{reference,engine}-static.png` | **入库**的两张静态截图（卡片区域） |
| `evidence/toolcard/report.json` | **入库**的工具卡面对照报告 |
| `evidence/toolcard/{reference,engine}-case-*.png` | **入库**的逐用例逐态截图 |
| `evidence/toolcard/console.json` | **入库**的两侧浏览器控制台消息 |
| `out/`、`out/toolcard/` | 完整产物（逐帧 PNG、DOM 归一化结果、动画结构）—— 不入库 |

## 诚实边界

- 像素对照是**截图层面**的证据，**不等于** owner 的主观电影感验收（P4 的输入）。
- 逐帧对照只覆盖**试点组件的挂载 / 卸载时机与一次状态更新**，**不覆盖整页**。
- 工具卡面批的逐帧对照覆盖**展开动作**（收起 → 展开）这一条时间线。
- harness **只读引入**生产组件 ⇒ 它**不验证生产前端本身**。
- 真后端（`/v1/apeireth/approval-requests` + SSE）**未接线**：两侧都喂固定
  fixture（引擎不碰 HTTP / SSE 契约，设计文档 §1.3 第 6 条）。
- 工具卡面批**不含容器页**（拉数据 / 能力门控 / 授权变更 / 模态）⇒ 容器面
  另批做（需要 mock runtime）。
- 浏览器对照在 **系统 Chromium** 里做 ⇒ **未在 Tauri WebView**（真实宿主）里
  验证；**真实 GPU** 未验（本批是 DOM 渲染路径，不走 GPU 管线）。

