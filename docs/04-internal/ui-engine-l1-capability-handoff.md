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
| G1 | ✅✅ **完成（2026-11-07 收口）**：`ElementBox` / `ScrollMetrics` + `Dom::bounding_box/scroll_metrics/set_scroll_top`（默认 `NotImplemented`）；`InMemoryDom` 脚本化测量（测试专用）；`WebDom` 接真实 DOM ⇒ ⇒ **API 面**（2026-10-28）· **浏览器逐值一致**（E-98：引擎盒 ≡ JS `getBoundingClientRect`）⇒ ⇒ ⭐⭐⭐ **生产侧消费已落地**（2026-11-06，产品仓）：宿主在新工作项插入后，用 **三个 API 各承担一个真判定**完成「把被滚出可视区的列表滚回顶部」 ⇒ ⇒ `bounding_box`（首项顶部是否高于滚动容器顶部） + `scroll_metrics`（还能不能滚） + `set_scroll_top`（执行） ⇒ ⇒ 依据：用户滚到列表下方点「新建」⇒ 新项插在最前但**在可视区外** ⇒ 用户以为没建成（与「过滤态新建不可见」是**同一类**缺陷，成因从筛选变成滚动） ⇒ ⇒ **实测**：`corrections = 1`、`scrollTopAfter = 0`、`first_item_top: -959 → 0` ⇒ ⇒⇒ 定位**不依赖脆弱下标路径**（按 `data-*` 在已挂载树里找，找不到返回 `None` 不猜） | `grep` 产品仓 ⇒ ⇒ 消费点已接线；浏览器证据 `layoutProbe.corrected = true` + `corrections = 1` + `scrollTopAfter = 0` ⇒ ⇒ **G1 的「余下两件」（①上层消费 ②逐值一致）均已满足** | 第 1 批（**完成**） |
| G2 | **事件面与载荷** | ✅ **2026-10-28 扩到 17 类**（含 pointer 族/scroll/wheel/submit/contextmenu）并补齐载荷（`checked` / `client_position` / `button` / `pointer_id` / `alt_key`）；浏览器后端已填充；`scroll` 标注**不可委托**。**IMPLEMENTED；浏览器内合成事件证据待补** | ⚠️ **本行「余下」曾经过期**（2026-11-07 核对）：①「浏览器内合成事件断言」**早已完成** —— E-98 的 `dom.payload` 实测 `value=Some("a");pos=None;checked=Some(false)`（`pos` 为 `None` 属**正确**：合成事件无坐标）⇒ ⇒ 该子项**关闭**；②**仍余**：「命中测试消费方」—— 实测 `hit_test_device` 只在 `renderer.rs` 的**测试**里被调用（生产侧 **0 消费**）⇒ ⇒ 与 G7 修前的病同型 （能力在、只有测试在用）| 第 1 批（**①已完成；②待接消费方**） |
| G3 | **焦点 / IME / a11y 通道** | ✅ **2026-10-28 逻辑层已落地**：`FocusTracker`（栈 + 陷阱）、`CompositionGate`（组合中禁止写回）、`AriaChannel`（24 条白名单）；事件面加 `CompositionStart/End`。**IME 静默期 + 焦点均接线且浏览器已实测**（E-98：`focus_at_path` 驱动 `document.activeElement`；`compositionstart/end` 翻转组合态）| 真实中文输入法硬件级实测仍留待（当前用合成事件） | 第 1 批（已完成） |
| G4 | **受控输入闭环（双向）** | **已接线**：`set_property` 在 `renderer.rs` 出现 12 处、`patch_props` 先 `get_property` 回读再写、宏支持 `bind:value` / `bind:checked`（`apeireth-ui-macro/src/view.rs:322`） | ✅✅ **完成（2026-11-07）**：两项证据均已采到 —— ①**全链闭环**：`l1_binding_probe` 新增 ④，一次跑完「真实 `input` 事件 ⇒ 信号 ⇒ **应用加工**（`signal.update`）⇒ 重渲染 ⇒ DOM」三跳，实测 `loop_before: "uplink"` → `loop_after: "uplink-applied"` → `signal_value: "uplink-applied"` ⇒ ⇒ 且**不需要**手工再写一次 DOM ⇒ ⇒ 反向断言「状态本身也必须是加工后的值」（防 DOM 对了状态没对）；②**IME 期间不打断**：`l1_integration_probe` 的 ② 实测「组合中不写回（DOM 停在 `hello2`）、组合结束后补写（变 `hello3`）」⇒ ⇒ 两项合起来把 G4 关掉 | `drive_l1.py` **EXIT 0**，`binding.ok = true` ⇒ ⇒ **G4 完成** | 第 1 批（**完成**） |
| G5 | ✅ ✅ **完成**：窗口化纯计算 + 真实滚动状态窗口 + `patch_windowed`（只挂窗口子项、总高度恒定）+ 子级就地替换（嵌套列表）+ **`scroll` 自动监听（信号式）** + **1k 行只挂 O(窗口)** 均已接线 | —（G5 完成；1k 行 wall-clock 基准未做，只做了规模不变量） | 余下：窗口 → 渲染器的挂载/卸载接线 + `scroll` 事件监听接线 + 1k 行 patch 基准 | 第 2 批（进行中） |
| G6 | **缩放 / 密度标量** | ✅ **2026-10-28 已落地**：`MeasurementScale`（dpr × density，非法值回落 1.0）+ 渲染器接线（`bounding_box_in_device` / `hit_test_device`）。**IMPLEMENTED + WIRED + 浏览器已实测**（E-98：2.0×1.25 换算正确） | — | 第 1 批（已完成） |
| G7 | ✅✅ **完成（2026-11-06）**：`ResourceRegistry`（句柄/状态机/失败原因保留）+ **CLS 观测**（亚像素/视口外不计分，接真实几何）+ **解码/上传钩子**（`ImageData` 校验 + `upload_decoded`，「Ready 必须意味着真的上传成功」） ⇒ ⇒ ⭐⭐⭐ **本批补上最后一段**：①**`TextureUploader` 的 GPU 实现 `GpuTextureUploader` 已在引擎内**（`ui-gpu/src/upload.rs`）；②**搬家**：资源句柄/加载状态/纹理上传**契约**由 `ui-dom::resource` 搬到 **`ui-gpu::resource`** ⇒ ⇒ 依据（实测）：`ui-dom` 内部 **0 消费**、真正需要它的是**像素投影侧**、而 `ui-dom` 与 `ui-gpu` 是**平级**（ADR-0015 §3.3）⇒ ⇒ 平级 crate 不能互相依赖 ⇒ ⇒ 跨 crate 适配器**无处可放** ⇒ ⇒⇒ 此前唯一一份实现躺在 `evidence/p5-b/harness` ⇒ ⇒ **「能力只有测试装置在用」**（这正是 G7 缺口）；搬家后实现与契约**同 crate** ⇒ ⇒ **零适配器**；③**布局度量 `MetricsStatus` 留在 `ui-dom`**（布局面的事，`renderer.rs` 消费） ⇒ ⇒ 模块一分为二，各归其位；④**证据**：`drive_l1.py` 的 `upload` 段（G7 纹理上传端到端）**真浏览器 EXIT 0** ⇒ ⇒ harness 已**删掉自己那份适配器**、改用引擎的 `GpuTextureUploader`（「能力必须落在引擎 API 内，不接受宿主打补丁」⇒ ⇒ 现在成立）  ⭐⭐⭐⭐ **CLS 浏览器证据已补齐（同日）**：`l1_layout_shift_probe` 三条对照**同时成立** ⇒ ⇒ ①**真实下移 100px** ⇒ 得分 **0.00080375**（> 0） ⇒ ⇒ ②**亚像素 0.5px** ⇒ **0**（阈值 1px，不把渲染噪声当缺陷，E-54 那类 1 LSB） ⇒ ⇒ ③**两次观测都在视口外** ⇒ **0**（用户看不见的位移不计分） ⇒ ⇒ 手算核对：impact=(100×150)/(1440×900)=0.011574、distance=100/max(1440,900)=0.069444 ⇒ 乘积 **0.00080376** ⇒ ⇒ **与实测逐位一致** ⇒ ⇒ ⇒ 三条缺一不可 ⇒ 否则 CLS 观测就是「**永远返回 0 的摆设**」 ⇒ ⇒ ⚠️ 探针两处踩坑已记：①`node_at_path` 从**挂载根**起算（`[1]` 而非 `[0,1]`）；②元素必须 `position:fixed` 钉在视口内（否则 y≈1250 落在视口外 ⇒ 恒 0）| `grep -rn 'ui_dom::resource' crates-ui` ⇒ ⇒ **0 命中**；`cargo test --workspace --locked` **1240 passed / 0 failed**；clippy native + **wasm32** 各 **0 警告**；fmt **0 diff**；`drive_l1.py` **EXIT 0**（含 `upload` 段）⇒ ⇒ **G7 完成** | 第 2 批（**完成**）|
| G9 | **测量目标的稳定寻址**（G1 收尾时实测出的**真缺口**） | ⚠️ **引擎目前只能按 `&[usize]` 下标路径寻址** ⇒ ⇒ 而路径随重排变化 ⇒ ⇒ 上层要测某个元素只能**自己遍历已挂载树**（实测：产品仓为此写了约 30 行 `find_mounted_by_attr`）⇒ ⇒⇒ 关键事实：①引擎**已有** `key` → 身份段 → **反解当前路径**的完整机制（`identity_segments` / `resolve_identity_path`），但它是**私有**的、只服务焦点；②`Dom` trait **没有读属性的方法**（只有 `set_attribute` / `remove_attribute`）；③`MountedNode::key_ref` 是**私有**的 ⇒ ⇒ 上层拿不到 `key`，也就无法复用引擎的身份机制。⇒ ⇒⇒ 后果：**每个宿主都要重写一遍同样的树遍历** ⇒ 且各自写法可能不一致（有人按 class 分词、有人按 `data-*`；按 class 分词**容易静默测错**） | 暴露一个**公开的「按稳定标识取测量目标」API**（候选：①公开 `key_ref` + 提供 `path_for_key`；②或提供 `bounding_box_by_attr` 这类便捷入口）⇒ ⇒ 判据：上层**不再需要自己写树遍历**即可测量任意目标；且**重排后仍能找到同一逻辑元素** | 第 3 批（**待排**） |
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
