# 引擎能力边界与 L1 缺口交接（ADR-0015 增补依据）

> 状态：**生效中**（owner 2026-10-28 裁定：按「L1 进引擎 / L2 进 ui-kit / L3 排除」划线，并批准对
> [ADR-0015](../adr/0015-pure-rust-ui-engine.md) 做**增补**——不是重写）。
>
> 本文是给「上屏（新前端）线」与「引擎线」的共同契约，也是引擎侧排期的唯一依据。
> 四级口径（IMPLEMENTED / PRODUCTION WIRED / DEFAULT ENABLED / HARDWARE VALIDATED）沿用仓规。

## 0. 一句话

引擎负责**行为原语**（能被逐像素证明与生产等价的那一层）；**视觉规格、内容格式、网络策略、主题数据**归上层。
「能力缺口」不是「缺陷」，两者分别记账：引擎缺陷进台账，能力缺口进 §2 的批次表。

## 1. 三层边界（L1 / L2 / L3）

| 层 | 内容 | 归属 | 验收口径 |
|---|---|---|---|
| **L1 引擎核心** | 响应式调度、DOM 绑定与差值写回、事件面与事件载荷、**布局测量**、滚动容器与虚拟化原语、焦点 / IME / a11y 通道、**缩放与密度标量**、资源句柄与加载状态、错误边界与 panic hook、主题 **class 通道** | `apeireth-ui-core` / `-dom` / `-gpu` | native 行为测试 + 可复跑浏览器证据 + 四级口径标注 |
| **L2 组件层** | 表单控件族（select/range/date/radio/textarea 封装）、Markdown 渲染、代码高亮、图标、富文本编辑、弹层与虚拟列表**组件** | `apeireth-ui-kit` | 组件级测试 + 生产样本内容层对照（**不进引擎验收口径**） |
| **L3 不进引擎** | 设计令牌与主题**数据**、网络栈 / 缓存 / 打包命名、3D、动画编排内容、排版曲线与字体整形策略 | 宿主 / 产品线 | — |

**判定标准**（四条，其余都是口味）：①是否有跨消费者不变量；②是否必须与渲染帧/时序耦合；
③是否需要上层拿不到的特权能力（真实几何、GPU 纹理、原始事件载荷）；④是否是随设计稿高频变化的视觉规格。
前三条倾向 L1，第四条倾向 L2/L3。

**明确排除**：富文本进 L1 会把引擎拖进排版引擎领域（换行 / bidi / 字体整形 / 选区几何），
与 ADR-0015「不做 CSS 引擎 / 排版」直接冲突 ⇒ 富文本只在 L2，L1 只提供其**依赖的原语**（测量、可嵌入子树、选区通道、滚动容器）。

## 2. L1 缺口清单（引擎侧排期依据）

| # | 缺口 | 现状证据 | 判据（可执行） | 批次 |
|---|------|----------|----------------|------|
| G1 | **布局测量 API** | ✅ **2026-10-28 已落地 API 面**：`ElementBox` / `ScrollMetrics` + `Dom::bounding_box/scroll_metrics/set_scroll_top`（默认 `NotImplemented`）；`InMemoryDom` 脚本化测量（测试专用）；`WebDom` 接真实 DOM。**IMPLEMENTED + WIRED + 浏览器已实测**（E-98：引擎盒与 JS `getBoundingClientRect` 逐值相同） | 余下两件：①渲染器或上层**消费**该 API（PRODUCTION WIRED）；②浏览器侧逐值一致证据 | 第 1 批（API 已完成） |
| G2 | **事件面与载荷** | ✅ **2026-10-28 扩到 17 类**（含 pointer 族/scroll/wheel/submit/contextmenu）并补齐载荷（`checked` / `client_position` / `button` / `pointer_id` / `alt_key`）；浏览器后端已填充；`scroll` 标注**不可委托**。**IMPLEMENTED；浏览器内合成事件证据待补** | 余下：浏览器内合成事件断言（真实事件 → 载荷齐备）+ 命中测试消费方 | 第 1 批（主体已完成） |
| G3 | **焦点 / IME / a11y 通道** | ✅ **2026-10-28 逻辑层已落地**：`FocusTracker`（栈 + 陷阱）、`CompositionGate`（组合中禁止写回）、`AriaChannel`（24 条白名单）；事件面加 `CompositionStart/End`。**IME 静默期 + 焦点均接线且浏览器已实测**（E-98：`focus_at_path` 驱动 `document.activeElement`；`compositionstart/end` 翻转组合态）| 真实中文输入法硬件级实测仍留待（当前用合成事件） | 第 1 批（已完成） |
| G4 | **受控输入闭环（双向）** | **已接线**：`set_property` 在 `renderer.rs` 出现 12 处、`patch_props` 先 `get_property` 回读再写、宏支持 `bind:value` / `bind:checked`（`apeireth-ui-macro/src/view.rs:322`） | 补「输入 ⇒ 状态 ⇒ DOM」全链浏览器断言 + IME 期间不打断；**无需新建**，只需补证据与边界用例 | 第 1 批 |
| G5 | ✅ ✅ **完成**：窗口化纯计算 + 真实滚动状态窗口 + `patch_windowed`（只挂窗口子项、总高度恒定）+ 子级就地替换（嵌套列表）+ **`scroll` 自动监听（信号式）** + **1k 行只挂 O(窗口)** 均已接线 | —（G5 完成；1k 行 wall-clock 基准未做，只做了规模不变量） | 余下：窗口 → 渲染器的挂载/卸载接线 + `scroll` 事件监听接线 + 1k 行 patch 基准 | 第 2 批（进行中） |
| G6 | **缩放 / 密度标量** | ✅ **2026-10-28 已落地**：`MeasurementScale`（dpr × density，非法值回落 1.0）+ 渲染器接线（`bounding_box_in_device` / `hit_test_device`）。**IMPLEMENTED + WIRED + 浏览器已实测**（E-98：2.0×1.25 换算正确） | — | 第 1 批（已完成） |
| G7 | ⚠️ **基本可用**：`ResourceRegistry`（句柄/状态机/失败原因保留）+ `MetricsStatus`（默认 Unknown，**窗口计算入口被门住**）+ **CLS 观测**（`LayoutShiftTracker`，亚像素与视口外不计分，已接引擎真实几何）均已落地；**未做**：解码/上传钩子、宿主接入证据 | 余下：解码/上传钩子 + 宿主接入 + 浏览器 CLS 证据 | 余下：解码/上传钩子 + CLS 观测 + 宿主接入证据 | 第 2 批（进行中） |
| G8 | **错误边界与 panic hook** | ⚠️ **部分**：错误边界已落地并接线（`render_guarded` + `set_error_boundary` + `captured_error`，未设兜底时不吞错，兜底失败仍报原始错误）。**两片均已落地并采到浏览器证据（E-106）**：错误边界「失败 ⇒ 兜底进 DOM + 原始错误可读」；panic hook 浏览器内**可安装**（真 panic 行为仍只有 native 证据） | 余下：panic hook / 崩溃可见兜底 + 浏览器注入 panic 证据 | 第 2 批（进行中） |

> L2/L3 项（表单控件族、Markdown、代码高亮、图标、富文本、弹层/虚拟列表组件、令牌数据、网络策略、dev server/HMR、
> wasm 体积门）**不在**本表 ⇒ 由 `ui-kit` 或宿主线各自排期。其中「wasm 体积预算门」属**治理项**，归引擎线 CI 体系。

## 3. 对上屏线清单的三条复核纠正（附证据）

| 对方结论 | 复核结果 | 证据 |
|---|---|---|
| 「受控输入未接线（`bind:value/checked`）」 | **不成立** | `grep -c set_property apeireth-ui-dom/src/renderer.rs` ⇒ **12**；`patch_props` 先 `get_property` 回读再写；宏支持 `bind:value`/`bind:checked`（`apeireth-ui-macro/src/view.rs:322`）；对方引的 `view.rs:1120` 是**测试函数位置**，不是编译期报错点 |
| 「文档仍写『事件委托未接线』」 | **误读** | `apeireth-ui-dom/src/lib.rs:59-70` 明确写：**元素直挂已接线**（trait 有接口、渲染器三处接线），未接线的只是「**委托根单监听器**」这一形态与 `EventRegistry` 尚未接线 ⇒ 台账 E-34 已按逐条核验闭合 |
| 「7 处 `querySelector` 回写补丁」 | **数字虚高** | 排除 `Reflect::get(container, "querySelector")` 这类探测后剩 **2 处**，均在 `settings_theme.rs` 的 `querySelectorAll` + 错误映射路径（疑似校验而非回写）⇒ 应重述为「≤2 处待核」 |

**这三条必须在双方动工前对齐**，否则上屏线会按错误前提排期。

## 4. 交付形状要求（引擎线承诺）

1. 能力必须落在**引擎 API 内**，不接受 demo/宿主打补丁（`querySelector` 回写就是反面教材）；
2. 每项三件套：**native 行为测试**（禁读生产源码文本）+ **可复跑浏览器证据** + **四级口径标注**；
3. 新增 crate / 外部依赖走 ADR + `contract.toml` §[vendored].ui_engine 更新；crates-ui 维持 **0 外部依赖**纪律；
4. 判据代码**不得依赖 CI 未声明的包**（2026-10-28 实测：判据里加 numpy 会让 CI 的 `drive.py` 直接崩）；
5. 引擎**不承担**：视觉令牌与排版曲线、内容格式（Markdown/富文本模型）、网络与缓存策略、3D。

## 5. 当前可判定边界（防混淆）

| 判据 | 判定装置 | 现状 |
|---|---|---|
| 构建 / 类型 / clippy / fmt / 单测 / wasm 产物 | `scripts/run-ci-local.py`（本地等价 CI，8 job） | **8/8 通过** |
| 五面 + intro **逐像素等价** | 本地真实 GPU（RADV）+ SwiftShader 双模 | **0 差异，3 次稳定**；intro 12.04% → 0.016% |
| 双后端一致 | 本地 + CI | 绿 |
| 真实 Tauri WebView / 其它 GPU / Windows | **无装置** | **未验证**（不冒充） |
| GitHub CI 的像素 job | 该 runner 环境下不可判定（台账 E-89/E-90） | 已改为**显式 not_applicable + 告警**，判定权归本地等价 CI |
