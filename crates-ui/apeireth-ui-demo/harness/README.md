# 试点对照 harness（P3）

> **这是 P3 的证明性装置**：把「组件 = Rust 函数」这条主张**端到端**跑一遍 ——
> Rust 引擎渲染的审批卡 vs 生产前端的 Svelte 审批卡，**逐像素** + **逐帧**对照。

## 两侧

| 侧 | 页面 | 内容 |
|---|---|---|
| **参考侧** | `reference/index.html` | 生产组件 `ApprovalRequestCard.svelte`（**只读引入**，由 vite 编译）+ 同一批设计 CSS |
| **引擎侧** | `engine/index.html` | wasm 引擎（`apeireth-ui-demo`）渲染同一张卡 + 同一批设计 CSS |

两侧喂**同一份**固定数据（`fixtures/pilot-request.json`），跑在**同一个浏览器实例**
的同一个视口设定下 ⇒ 差异只可能来自渲染实现。

## 单一真源（构建期抽取，脚本 `scripts/extract-assets.mjs`）

| 资产 | 来源（**只读**） | 去向 |
|---|---|---|
| 卡片 scoped CSS | 生产组件的 `<style>` 块（**原文**） | `public/pilot-card.css`（引擎侧注入） |
| 设计令牌 CSS | `frontend/companion-desktop/src/lib/design/*.css` | `public/design/*.css`（两侧同源） |
| 图标几何数据 | 上游图标包的 `dist/icons/*.svelte` 里的 `iconNode` | `crates-ui/apeireth-ui-kit/src/icon_data.rs`（生成物，带上游许可署名） |
| 固定数据 | `fixtures/pilot-request.json` | `crates-ui/apeireth-ui-demo/src/fixture_generated.rs`（生成物） |

**不复制、不手抄、不改生产文件**；生成物可复核：`node scripts/extract-assets.mjs --check`
（漂移 ⇒ 退出码 1）。

## 一键复跑

```bash
bash build.sh
```

五步：抽取资产 ⇒ 构建 wasm ⇒ wasm-bindgen ⇒ vite build ⇒ 跑对照。

依赖：cargo（wasm32 target）、`wasm-bindgen-cli 0.2.129`、node + vite、
python3（`websocket-client` + `Pillow`）、系统 Chromium。
**不需要** Playwright，**不需要**下载浏览器。

## 对照项与判据

| # | 对照项 | 判据 |
|---|---|---|
| T1 | 静态像素对照 | 差异像素 / 总像素 **≤ 1%**（任一通道绝对差 > 2/255 即记「不同」） |
| T2 | DOM 结构对照 | 归一化后**完全相等**（D1 结构 / D2 属性 / D3 文本必须为零） |
| T3 | 相位冻结对照 | 5 个相位点（0 / 150 / 300 / 450 / 600 ms），每点 **≤ 1%** |
| T4 | 逐帧对照 | 更新期间每帧 **≤ 1%**（按时间对齐，容差 25 ms） |
| T5 | 动画结构对照 | `getAnimations()` 的动画集合 / 时长 / 延迟 / 关键帧**逐条相等** |
| T6 | 卸载时机对照 | 关闭卡片 ⇒ 两侧在**同一时刻**摘掉节点；且**不产生业务决策** |

**环境固定**（设计文档 §9.3 步骤 2）：视口 `1440×900`、`devicePixelRatio = 1`、
时区 `UTC`（两侧取同一面钟）、`prefers-reduced-motion: no-preference`。

## 归一化规则（设计文档 §9.3 步骤 7）

- 剔除 Svelte 作用域 class（`svelte-<hash>`）；
- class token **排序**（token 顺序不影响 CSS）；
- 属性键**排序**；
- 纯空白文本节点**折叠**（flex / inline-flex 容器里不渲染 ⇒ 不影响像素）。

## 仅测试注入的样式

`public/pilot-test.css`（**不进产品资产**）：

- `html[data-pilot-motion="on"]` ⇒ 卡片入场动画（**挂载时机**的可观测代理）；
- `html[data-pilot-transition="on"]` ⇒ 按钮过渡（**更新时机**的可观测代理）；
- `html[data-pilot-freeze="1"]` ⇒ 把动画**冻结**在 `--pilot-phase` 相位。

⚠️ 两个开关**分开**用：入场动画会让卡片进入合成层（阴影的抗锯齿与层提升有关）
⇒ 与逐帧对照混用会把噪声算进差异里。

## 产物

| 路径 | 内容 |
|---|---|
| `evidence/report.json` | **入库**的对照报告（全部数值） |
| `evidence/{reference,engine}-static.png` | **入库**的两张静态截图（卡片区域） |
| `out/` | 完整产物（逐帧 PNG、DOM 归一化结果、动画结构）—— 不入库 |

## 诚实边界

- 像素对照是**截图层面**的证据，**不等于** owner 的主观电影感验收（P4 的输入）。
- 逐帧对照只覆盖**试点组件的挂载 / 卸载时机与一次状态更新**，**不覆盖整页**。
- harness **只读引入**生产组件 ⇒ 它**不验证生产前端本身**。
- 真后端（`/v1/apeireth/approval-requests` + SSE）**未接线**：两侧都喂固定
  fixture（引擎不碰 HTTP / SSE 契约，设计文档 §1.3 第 6 条）。
