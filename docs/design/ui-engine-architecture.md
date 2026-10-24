# Apeireth UI 引擎技术架构设计 v1.0

> **项目**：Apeireth（阿佩瑞斯）—— AGI 操作系统 / AI 伙伴
> **本文定位**：全栈纯 Rust 前端引擎（响应式 + DOM 绑定）的**技术架构设计**——定义「每个 crate 公开什么、语义是什么、边界画在哪里」。**不是计划书**：工序、里程碑、人力、排期归 `docs/04-internal/ui-engine-plan.md`（另一条施工线撰写）。
> **决策真源**：[docs/adr/0015-pure-rust-ui-engine.md](../adr/0015-pure-rust-ui-engine.md)（Accepted，2026-10-24）。本文是该 ADR 的**技术展开**，不新增决策、不推翻决策；两者冲突时以 ADR 为准，并回写本文。
> **父文档**：[00-PHILOSOPHY.md](00-PHILOSOPHY.md)（范式层，最高层，冲突以它为准）→ [01-DESIGN-SYSTEM.md](01-DESIGN-SYSTEM.md)（令牌层，视觉层数值的唯一来源）。
> **施工宪章**：[docs/04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md](../04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md)（0 装纪律 / 中性表述 / 测试即证据）。
> **核实日期**：2026-10-24（基于工作区当前 HEAD，实测范围 `frontend/companion-desktop/src/`）。
> **中性表述**：本文只出现**真实依赖与真实路径**（Svelte 5 / Tauri 2 / wgpu / web-sys / Rust），不出现在别处；不写「移植 / 借鉴 / 照抄」式措辞。

---

## 0. 读我：状态标注约定（0 装 PASS）

本文每一条设计按诚实度三级标注，读者可据此判断「这条能直接施工」还是「这条还在等」：

| 标注 | 含义 | 施工口径 |
|---|---|---|
| ✅ **已定** | 本文给出**确定语义**，无占位、无「以后完善」。实现时按此施工；要改，先改本文。 | 直接编码 |
| 🔵 **待测** | 事实依赖**尚未完成的实测**（P0 探测已回填 5 项半，见 §11.2；余下 3 项仍待测）。本文**不预设结论**，只写「结论回来后如何落地」的确定路径。 | 阻塞项登记在 §11.2.2 |
| 🟡 **提案** | 本文提出的设计主张，尚未经 owner / 施工线确认。方向可先施工，数值与文法须回写。 | 登记在 §11.3 |

**不写的东西**（本文一律不出现）：「应该可以」「预计」「理论上」「未来再完善」；任何未经实测的性能数值（延迟、帧耗时、内存）；任何未标注出处的数字。

**引用口径**：本文所有仓库内事实给 `file:line`；所有统计数字给**可复跑的 grep / find 命令**（见 §11.5 复算清单）。

---

## 1. 定位与边界

### 1.1 一句话定位

> **Apeireth UI 引擎 = 一层「Rust 函数 ⇒ DOM」的纯 Rust 响应式渲染层：它决定 DOM 长什么样，不决定 DOM 好不好看。**

引擎把「一段 Rust 函数」变成「一棵真实 DOM 树」，并在这段函数的依赖发生变化时**就地、增量**地更新这棵树。除此之外的一切——颜色、字体、模糊、辉光、缓动曲线、主题——都不在引擎的职权范围内。

### 1.2 引擎边界硬约束

承接 [ADR-0015 §2.1](../adr/0015-pure-rust-ui-engine.md)（原文引用）：

> **引擎只做三件事：响应式调度 / DOM 绑定 / 过渡时序。**
> **不写 CSS 引擎、不写滤镜层、不碰 shader、不碰排版、不碰主题。**

**本轮口径修正后的第三条**（依据 [ADR-0015 §2.2 口径修正](../adr/0015-pure-rust-ui-engine.md)）：「过渡时序」在本仓**用量为 0**（框架级过渡指令 0 处），动画责任 100% 在 CSS。因此第三条边界收窄为：

> **第三条 = 组件挂载 / 卸载时机**：保证 CSS 过渡与关键帧动画在其应有的时机被触发，且**不被引擎提前掐断**。引擎**不产生任何自主动画**。

边界表（每一行的「引擎是否触碰」列即硬约束）：

| 层 | 载体 | 仓内实测 | 引擎是否触碰 |
|---|---|---|:--:|
| GPU 场景 | `frontend/companion-desktop/src/lib/scene/blackhole.ts` 1,037 行 / 2 条 GLSL | WebGL2 | ❌ 零改写 |
| 场景合成 | `src/lib/scene/SceneLayer.svelte` 全屏 canvas 定位 | CSS 定位 | ❌ 零改写 |
| 暗角 + 颗粒 | `#vignette` DOM 节点 + `src/lib/design/shell.css` | CSS | ❌ 零改写 |
| 面板模糊 | `backdrop-filter` **80 处**（实测 `src/` 全树） | CSS | ❌ 零改写 |
| 排版 | 衬线栈 / 行高 2.1 / 字距 0.13em / 幽灵编号 | CSS | ❌ 零改写 |
| 主题 | 10 个主题 id + 4 个配色方案 + `--ap-*` 令牌（115 个唯一变量名，实测 `src/lib/design/`） | CSS + `src/lib/theme.ts` | ❌ 零改写 |

**引擎唯一允许的「样式面」输出**（三者之外一律不产出）：

1. `class` 属性（class 名与取值，必须与被替换实现逐字一致 —— 见 §9.1）；
2. `style` 属性（**仅当被替换实现原本就写了内联 style 时**才允许写，见 §9.2）；
3. `data-*` / `aria-*` 属性（无障碍与测试钩子，属 DOM 语义不属样式）。

**引擎不产出 CSS 文本、不计算层叠、不求值选择器、不读计算样式做决策**（§5.5.2 已确认：本设计下引擎**没有任何需要读计算样式的场景**，因此连「过渡时长探测」这条唯一例外也未被使用）。

### 1.3 不做什么清单（显式）

| # | 不做 | 理由 | 归属 |
|---|---|---|---|
| 1 | 不写 CSS 引擎（选择器匹配 / 层叠 / 特异性 / 计算样式） | 电影感载体全在 CSS 合成层，重写=重写视觉 | — |
| 2 | 不写滤镜层（`backdrop-filter` 80 处） | 同上 | — |
| 3 | 不碰 shader（那 2 条 GLSL 与未来的 WGSL） | 视觉零改写的最高风险区 | `ui-gpu` 只做**桥**，不改着色器数学 |
| 4 | 不碰排版（字族栈 / 行高 / 字距 / 断行） | 中文字形与断行是系统与浏览器的事 | — |
| 5 | 不碰主题（`html[data-theme=…]` / `html[data-accent=…]`） | 主题切换是 `<html>` 属性操作，与组件树无关 | — |
| 6 | 不碰 HTTP / SSE 契约 | 数据层是既有契约（`docs/02-guides/frontend-data-contract.md`） | 引擎只提供响应式原语 |
| 7 | 不碰 Tauri 宿主能力（窗口 / 托盘 / keychain / 透明窗 / 单实例 / 开机自启 / 快捷窗 / 桌宠穿透） | 宿主 13,476 行全复用 | — |
| 8 | 不实现框架级过渡指令（`in:` / `out:` / `transition:`） | 本仓用量 **0**（实测） | 见 §5.5 |
| 9 | 不实现 `$bindable` / `{#await}` / `use:` / `context=` / `setContext` / `$inspect` | 本仓用量 **0**（实测） | 见 §11.1 |
| 10 | 不做「全量接管」式的整页重绘 | 增量差分是性能与动画连续性的前提 | — |
| 11 | 不做「像素级还原 Svelte 内部实现」 | 只对齐**产出结果**（DOM + 视觉），不对齐实现手段 | 见 §9 |
| 12 | 不在引擎里塞业务概念（会话 / 审批 / presence 语义） | 引擎不认识 `PAD`、不认识 `approval`；它只认识信号 | 语义归 `ui-kit` / `ui-demo` |

**第 12 条是硬边界**：`ui-core` 的公开类型里不得出现任何业务名词（`Session` / `Approval` / `Pad` / `Ember` …）。一旦出现，引擎就退化成「带业务的项目」，可复用性与可测试性同时崩掉。

---

## 2. 与既有系统的关系

### 2.1 与 Tauri 宿主的关系

宿主（`frontend/companion-desktop/src-tauri/src/`，13,476 行 / 45 个 `#[tauri::command]`，实测口径见 [ADR-0015 §1.1](../adr/0015-pure-rust-ui-engine.md)）在引擎落地后**一行不改**。

| 职责 | 归属 | 说明 |
|---|---|---|
| 窗口生命周期 / 单实例 / 开机自启 / 托盘 / OS keychain / 透明窗 / 快捷窗 / 桌宠穿透 | **宿主** | 引擎不感知 |
| 45 个 `#[tauri::command]` 的参数与返回形状 | **宿主** | 契约不变，引擎不消费也不生产 |
| 「窗口里显示什么 DOM」 | **引擎** | 引擎产 DOM，宿主只提供容器 |
| DOM 刷新的调度时机 | **引擎** | 微任务批次（§4.2 规则 6：引擎**不跑自己的 rAF 循环**） |
| 场景绘制循环 / rAF | **引擎**（仅 `ui-gpu`） | 宿主不参与；DOM 写入不参与 rAF |

**接线形态**（确定）：引擎编译为 `wasm32-unknown-unknown` 产物，由宿主现有的 WebView 装载；宿主侧新增的只有「装载 wasm 产物 + 提供挂载点」两件事，**不新增 `#[tauri::command]`**。`ui-gpu` 的抽象使第二阶段的原生窗口路径（弃 WebView）成为可能，但**第二阶段不在本文承诺范围**（另出 ADR，见 [ADR-0015 §2.5](../adr/0015-pure-rust-ui-engine.md)）。

### 2.2 与 design-token 体系（`--ap-*`）的关系

**引擎只引用 class 与 CSS 变量，不生成样式。**

- 令牌真源 = CSS 变量（`frontend/companion-desktop/src/lib/design/tokens.css`，535 行，实测 115 个唯一 `--ap-*` 变量名）+ 机器可读镜像 `frontend/design-preview/design-tokens.json`（`01-DESIGN-SYSTEM.md` §10）。
- 引擎**不读**任何令牌值。引擎写出的 DOM 里，除了被替换实现本来就有的内联 `style`（见 §9.2），**不含任何颜色 / 尺寸 / 时长字面量**。
- 组件库（`ui-kit`）的 class 名 → 样式值的映射，**全部留在 CSS 资产里**（§9.5）。
- **例外条款（登记但未使用）**：若将来出现「必须等某个 CSS 过渡结束才能移除节点」的需求，引擎读**计算样式**（`getComputedStyle`）取过渡时长，而不是读令牌定义、也不是硬编码常数。**本设计下该需求不存在**（§5.5.2 规则 T5 已排除），故此条款登记在案、当前不启用。

**由此得到一条可机器检查的纪律**：`ui-core` / `ui-dom` / `ui-macro` 三个 crate 的源码里，出现颜色字面量（`#rrggbb` / `rgb(`）或时长字面量（`…ms` / `…s` 后缀的时长）即视为违规；`ui-kit` 只允许出现 class 名与结构。

### 2.3 与 presence 契约的关系

- 契约：`00-PHILOSOPHY.md` §10 `presence_state`（后端实现 `crates/adapters/gateway/src/presence.rs`，事件形状见 `docs/gateway-api-contract.md` §8a）。前端现役消费端 `frontend/companion-desktop/src/lib/presence.ts`（27,224 字节）。
- **引擎是 presence 的「下游显影管道」，不是 presence 的消费者**：presence 帧先被解析成 `ui-core` 的信号，再由 `ui-dom`（DOM 投影）与 `ui-gpu`（GPU 投影）两条正交路径各自订阅（[ADR-0015 §3.3](../adr/0015-pure-rust-ui-engine.md)）。
- 引擎**不认识** `PAD` / `stance` / `significance` 这些语义；语义映射发生在 `ui-kit` / `ui-demo` 层，引擎只提供「一个值变了 ⇒ 依赖它的 DOM 与 uniform 一起更新」的能力。
- 频率纪律（`00-PHILOSOPHY.md` §10：回合级事件 + ≤60s 低频心跳）**由后端保证**，引擎不缓冲、不补帧、不插值。

### 2.4 与 `frontend-data-contract.md` 的关系

- 契约真源：`docs/02-guides/frontend-data-contract.md`（HTTP / SSE 端点、鉴权、会话连续性头）。
- **引擎不碰契约**：不定义端点、不解析 SSE 帧、不发起请求、不处理鉴权。
- 引擎与数据层之间**只有一条缝**：数据层把远端帧翻译成 `ui-core` 的信号写入；引擎从信号往 DOM 走。缝的两侧都可以单独测试（数据层测「帧 → 信号」，引擎测「信号 → DOM」）。

### 2.5 与 14 套主题 / 三调性的关系

- 实测口径：主题 id 10 个（`frontend/companion-desktop/src/lib/theme.ts:3`：`origin` / `noir` / `heritage-void` / `essence` / `night` / `day` / `ocean` / `forest` / `paper` / `starship`），配色方案 4 个（`theme.ts:132`：`presence-gold` / `deep-space` / `sage` / `bone`），合计 14 —— 与 [ADR-0015 §2.1](../adr/0015-pure-rust-ui-engine.md)「14 套主题」口径一致。
- 三调性（场景 / 纸面档案 / 数据深舱）：`01-DESIGN-SYSTEM.md` §5.6。
- **零改写**：主题与调性的实现机制是 `<html>` 上的 `data-theme` / `data-accent` 属性 + CSS 变量覆写（`tokens.css` 的 `html[data-theme='…']` 块）。引擎**从不写 `<html>` 的属性**，也**从不读主题状态**。
- 结论：引擎迁移前后，主题系统、调性归属、金色纪律（`00-PHILOSOPHY.md` §7）**零改动**。

---

## 3. crate 拓扑与依赖方向

### 3.1 依赖图（照 [ADR-0015 §3.3](../adr/0015-pure-rust-ui-engine.md)，补全 API 形状）

```
                     ┌──────────────────────┐
                     │  apeireth-ui-macro   │  proc-macro，零内部依赖
                     │  #[component]/view!  │
                     └──────────┬───────────┘
                                │ (可选依赖)
                     ┌──────────▼───────────┐
                     │   apeireth-ui-core   │  信号 / 依赖图 / 调度 / effect
                     │   0 依赖 web-sys      │  ← 核心层永远能脱 WebView 单测
                     └──────┬────────┬──────┘
                            │        │
             ┌──────────────▼──┐  ┌──▼───────────────┐
             │ apeireth-ui-dom │  │  apeireth-ui-gpu │   ← 平级（正交投影）
             │ 差分/委托/绑定  │  │  wgpu 桥（场景）  │
             └──────────┬──────┘  └──────┬───────────┘
                        │                │
                     ┌──▼────────────────▼───┐
                     │   apeireth-ui-kit     │  组件库
                     └──────────┬───────────┘
                                │
                     ┌──────────▼───────────┐
                     │   apeireth-ui        │  门面：re-export 全部
                     └──────────┬───────────┘
                                │
                     ┌──────────▼───────────┐
                     │   apeireth-ui-demo   │  试点应用（审批卡）
                     └──────────────────────┘
```

### 3.2 单向下行、无环的证明

给每个 crate 一个**拓扑秩** `r`：

| crate | `r` | 出边（依赖） |
|---|:--:|---|
| `apeireth-ui-macro` | 0 | 无（proc-macro crate，不依赖任何内部 crate） |
| `apeireth-ui-core` | 1 | `ui-macro`（可选：仅在启用 `derive` 便利宏时） |
| `apeireth-ui-dom` | 2 | `ui-core` |
| `apeireth-ui-gpu` | 2 | `ui-core` |
| `apeireth-ui-kit` | 3 | `ui-dom` |
| `apeireth-ui`（门面） | 4 | `ui-core` + `ui-dom` + `ui-gpu` + `ui-kit`（+ `ui-macro` 的 re-export） |
| `apeireth-ui-demo` | 5 | `ui` + `ui-kit` |

**证明**：对每条边 `A → B`（A 依赖 B），必有 `r(A) > r(B)`。若存在有向环 `A₁ → A₂ → … → Aₙ → A₁`，沿环累加得 `r(A₁) > r(A₂) > … > r(Aₙ) > r(A₁)`，矛盾。故依赖图**无环**。∎

**边数与形态**：内部 path 依赖边共 **12 条** —— `macro→core` 1 + `core→{dom,gpu}` 2 + `{dom,gpu}→kit` 2 + `ui→{core,dom,gpu,kit,macro}` 5 + `demo→{ui,kit}` 2。若 `ui-demo` 只依赖门面 `ui`（`ui` 已 re-export `ui-kit`），则为 **11 条**。**此数为设计值，不是实测值**（`crates-ui/` 骨架属 P0 施工线）。

**与主仓的关系**：`crates-ui/` 是**独立 cargo workspace**（自有 `[workspace]` 表），不进主 workspace 依赖图 —— 先例 `frontend/companion-desktop/src-tauri/Cargo.toml:1-4`。因此主 workspace 的 `arch_guard` 对引擎 **0 影响**（[ADR-0015 §3.2](../adr/0015-pure-rust-ui-engine.md)），代价是引擎失去主门禁保护，因此**必须**为 `crates-ui/` 建独立门禁 CI（`check` / `clippy -D warnings` / `fmt --check` / `test` 四 job）。

### 3.3 各 crate 规格

#### 3.3.1 `apeireth-ui-core` —— 响应式核心

- **职责**：信号、派生、依赖图、调度器、effect、作用域（scope）。**不碰 DOM、不碰业务语义**。
- **公开类型**：`Signal<T>` / `ReadSignal<T>` / `Derived<T>` / `EffectHandle` / `Scope` / `ReactiveError` / `Scheduler`。
- **依赖**：内部 `ui-macro`（可选）；**外部依赖数 = 0**（这是 ADR §3.3「`ui-core` 0 依赖 web-sys」的加强版：不依赖任何 web/DOM 库，包括 `wasm-bindgen`）。
- **单一职责边界**：只回答「**什么时候、哪些东西需要重新算**」。不回答「算出来之后放到哪」。
- **何时可独立单测**：**永远**。它是纯 Rust，无 I/O、无平台 API ⇒ `cargo test` 直接跑，不需要浏览器、不需要 wasm target（[ADR-0015 §4 P1 验收门](../adr/0015-pure-rust-ui-engine.md)：≥20 例真测试，native 跑）。

#### 3.3.2 `apeireth-ui-dom` —— DOM 绑定层

- **职责**：虚拟节点 → 真实 DOM 的映射、差分、事件委托、双向绑定、组件挂载/卸载时机。
- **公开类型**：`VNode` / `Renderer<D>` / `Dom`（后端 trait）/ `EventKind` / `DomEvent` / `Children` / `Component`（trait）/ `AppHandle`。
- **依赖**：`ui-core`；外部依赖 `web-sys` **0.3.106** / `wasm-bindgen` **0.2.129**（**版本精确硬钉 1:1**，实测错配在依赖解析期即失败，见 §11.2 P0-e）。
- **单一职责边界**：只回答「**状态变成 DOM**」。不回答状态从哪来（那是 `ui-core` 与数据层）。
- **何时可独立单测**：**可以，且必须**——因为 DOM 操作被抽象在 `Dom` trait 后面，`ui-dom` 自带一个 native 的 `InMemoryDom` 实现 ⇒ 差分、委托、绑定逻辑全部能在 `cargo test`（native）里跑。**这是本设计能消灭源码镜像测试的结构性前提**（§10.2）。

#### 3.3.3 `apeireth-ui-macro` —— 过程宏

- **职责**：`#[component]`（props 结构生成 + 组件类型生成）与 `view!`（虚拟树构造）两个宏。
- **公开类型**：无运行时类型（proc-macro crate 只导出宏）。
- **依赖**：**零内部依赖**；外部依赖只有 proc-macro 三件套（`proc-macro` / `quote` / `syn`）。**多版本情况已查清：整棵树里唯一的双版本包是 `syn`（2.0.119 + 3.0.9）**；三件套的**具体版本待 P1 引入时定**（依赖白名单口径随 P1 一并落，见 §11.2 P0-d）。
- **单一职责边界**：只做**语法 → 语法**的翻译，**不产生运行时逻辑**。宏展开后若有运行时辅助代码，那段代码必须住在 `ui-core` / `ui-dom` 里。
- **何时可独立单测**：`trybuild` 风格的展开快照测试 + `ui-dom` 侧的端到端渲染测试。**展开快照 = 宏的输出文本比对**，与 §10.2 禁止的「源码镜像断言」不是一回事：它比对的是**引擎自己生成的代码**，用于捕捉宏的意外变更，不用于断言业务逻辑。

#### 3.3.4 `apeireth-ui-gpu` —— GPU 桥

- **职责**：把 `ui-core` 的状态翻译成 GPU uniform 并驱动绘制；承接场景的降级梯队。**不改着色器数学**。
- **公开类型**：`GpuContext` / `SceneUniforms` / `Tier` / `TierProbe`。
- **依赖**：`ui-core`；外部依赖 `wgpu` **30.0.1** + `naga` **30.0.1**（**已实测**：依赖树 108 条目 / 68 crate，唯一双版本包是 `syn`，wasm 侧 0 多版本；见 §11.2 P0-b）。
- **单一职责边界**：只回答「**状态 → 像素**」。与 `ui-dom` **平级**，两者互不知晓（§7.4）。
- **何时可独立单测**：**部分**——uniform 合成逻辑（状态 → uniform 缓冲字节布局）是纯计算，可在 native 单测；真实绘制必须在 wasm + 浏览器里验（§10.1 L3）。

#### 3.3.5 `apeireth-ui-kit` —— 组件库

- **职责**：把既有 31 个共享组件（§8）以 Rust 重写，class 名与 DOM 形状与被替换实现**逐字一致**。
- **公开类型**：`Button` / `List` / `Card` / `Input` / `Dialog` / `Switch` / `Badge` / `Tabs` / …（§8.2）。
- **依赖**：`ui-dom`（+ 可选 `ui-gpu`，仅场景类组件需要）。
- **单一职责边界**：只回答「**这块界面长什么样**」。不含业务数据获取。
- **何时可独立单测**：**可以**——组件是 `fn(Props) -> VNode` 的纯函数，native 直接调用 + 断言 VNode 树；配合 `InMemoryDom` 可断言最终 DOM 树。**这是 §6.4 的核心诉求**。

#### 3.3.6 `apeireth-ui` —— 门面

- **职责**：`pub use` 各层；对外只暴露一个 crate，让试点应用不必写 5 条 `use`。
- **公开类型**：各层类型的 re-export。
- **依赖**：`ui-core` + `ui-dom` + `ui-gpu` + `ui-kit`。
- **单一职责边界**：**零逻辑**。门面里出现任何一行非 `pub use` 的实现代码即违规。
- **何时可独立单测**：编译通过即可（`cargo check`）；无独立测试。

#### 3.3.7 `apeireth-ui-demo` —— 试点应用

- **职责**：P3 试点（审批卡，见 [ADR-0015 §4](../adr/0015-pure-rust-ui-engine.md)），兼作引擎的装配级冒烟。
- **依赖**：`ui` + `ui-kit`。
- **何时可独立单测**：装配级测试在浏览器里跑（§10.1 L3）。

---

## 4. 响应式核心设计（`ui-core`）

### 4.1 信号（Signal）模型

```rust
// 概念形状（示意，非最终签名）
pub struct Signal<T> { inner: Rc<SignalInner<T>> }

impl<T: Clone> Signal<T> {
    pub fn get(&self) -> T;                       // 读：在追踪域内则登记依赖
    pub fn set(&self, value: T);                  // 写：总是使能失效
    pub fn set_if_changed(&self, value: T) -> bool; // 写：相等则不失效（需 T: PartialEq）
    pub fn update(&self, f: impl FnOnce(&mut T));  // 读改写
    pub fn read_only(&self) -> ReadSignal<T>;     // 降级为只读（可跨线程/跨层传递）
}
```

**读语义**

- `get()` 在**追踪域**（effect / derived 的执行栈内）被调用时：返回当前值，**同时**把「当前追踪域 ↔ 本信号」的依赖边登记进依赖图。
- `get()` 在追踪域外被调用时：返回当前值，**不**登记任何依赖（等价于「这次读取不关心后续变化」）。
- 依赖边在**每次重跑时重新登记**（追踪域在每次执行开始时清空）⇒ 条件依赖（依赖随分支变化）是天然支持的。

**写语义**

- `set(v)`：**总是**使能失效并调度通知（与被替换实现的「对象类型恒不等」语义一致，避免「改了数组但引用相同导致不更新」这类坑）。
- `set_if_changed(v)`：要求 `T: PartialEq`，值相等时**不**失效，返回 `false`；不等时等同 `set`。**默认推荐 `set`**；只在明确需要「等值不触发」时用 `set_if_changed`。
- 写操作发生在**派生求值过程中** ⇒ `ReactiveError::MutationDuringDerivation`（硬错误，不静默）。
- 写操作发生在**自己订阅的派生**上 ⇒ 该派生被标脏 → 重算 → 若形成环，由 §4.4 的环检测报 `ReactiveError::CycleDetected`。

**版本号**：每个信号带单调递增 `version: u64`，`set` 时自增。用途：(a) 让下游在多次写入后能判断「值是否真的变过」；(b) 为 §5.2 的差分提供「本次更新由哪次写触发」的定位手段。

### 4.2 调度时机：微任务 vs 帧

**两级调度**（确定语义）：

| 级 | 触发 | 做什么 | 谁用 |
|---|---|---|---|
| **T1 微任务级（唯一调度级）** | 任一 `Signal::set` | ① 沿依赖图传播脏标记；② 重算被读到的脏派生（惰性，见 §4.3）；③ 把所有待渲染节点**合并成一次** DOM 写入批次；④ 写完后跑 effect | `ui-core` + `ui-dom` |
| **T2 帧级（不属于 DOM 写入）** | `ui-gpu` 的绘制循环 | 按 `requestAnimationFrame` 推进 uniform 与绘制 | `ui-gpu` |

**精确规则**

1. **写入 ⇒ 一次微任务**。`set()` 不立即同步渲染，而是把当前批标记为「脏」并安排一次微任务（wasm 下 `queueMicrotask`；native 下由 `Scheduler` 的执行器决定，测试里同步 drain）。⇒ **同一轮事件处理里的多次写入合并为一次 DOM 写入**。
2. **批次（batch）**：`batch(|| { … })` 内的所有写入合并，批次结束才触发一次 T1。批次可嵌套（内层批次结束时若外层仍在进行，则不触发 T1）。
3. **顺序保证**：一个 flush 内的顺序是 **① 脏传播 → ② 渲染（DOM 写入）→ ③ effect 执行**。**effect 永远在 DOM 更新之后跑** ⇒ effect 可以安全地读 DOM（量尺寸、聚焦、读 `getBoundingClientRect`），这与被替换实现的 `$effect` 时序一致。
4. **写入合并**：同一个 flush 内，所有待渲染节点只产生**一次** DOM 写入批次（一批连续的属性写入 + 一批节点插入/移除），中间不插入任何别的任务。CSS 动画与过渡因此不会被「拆成多帧写入」而抖动。
5. **事件处理中的写入** ⇒ 微任务 flush ⇒ DOM 写入发生在**同一次绘制之前**（微任务先于渲染）⇒ 用户点击后界面在同一帧响应，**无「先亮一下再更新」的中间帧**。
6. **DOM 写入在微任务内完成，引擎不跑自己的 rAF 循环**：这是与被替换实现**逐帧对齐**的前提（参考实现同样在微任务内刷新 DOM）。`requestAnimationFrame` 只出现在两处：`ui-gpu` 的绘制循环，以及确实需要「下一帧才能读到布局」的显式场景。
   - **被拒的 alternative（帧级合并 DOM 写入）**：把 DOM 写入推迟到 rAF 回调内，能把一帧内的多次写入压成一次。**拒理由**：① 页面不可见时 rAF 不触发 ⇒ 后台标签页里的状态更新（含 SSE 心跳、`visibilitychange` 场景）永远不落到 DOM；② 与参考实现的刷新时机不一致 ⇒ §9.4 的逐帧对照失去同相位基准；③ 收益仅在「一帧内多次独立写入」这一窄场景，而本仓的状态更新本就以事件为单位批量发生。
7. **effect 在 effect 阶段创建** ⇒ 推迟到**下一次** flush 执行（防重入）。
8. **flush 深度上限**：单个节点在一次 flush 内重算次数超过 **100**（设计常量，非实测）⇒ `ReactiveError::FlushCycle`。作用是兜住「effect 写自己的依赖」这类自激循环，而不是让它把浏览器挂死。

### 4.3 派生（Derived）：惰性 vs 主动

**决策：惰性求值（pull）+ 主动失效（push-marking）。**

- **失效是推的**：信号被写 ⇒ 立刻沿「订阅者」边把下游派生与 effect 标脏。O(边数)，一次。
- **求值是拉的**：派生只在**有人读它**时才重算，且**每次 flush 内至多重算一次**（重算后缓存值，直到下次被标脏）。

**为什么选惰性（而不是主动求值）**

1. **死枝不计算**：本仓 `{#if}` 377 处、`$derived` 162 处。条件分支里定义的派生，在分支不显示时没人读它；主动求值会把这些派生也全部算一遍，纯浪费。
2. **求值顺序不外泄**：主动求值要求全局拓扑排序（谁先谁后），一旦有环就难处理；惰性求值把每个派生保持成「其依赖的纯函数」，求值顺序由读取点决定，天然局部。
3. **与 `ui-core` 的零依赖目标一致**：惰性 + 缓存不需要额外的调度拓扑结构，核心层更小、更可测。

**代价（诚实登记）**：惰性求值要求「读取点即依赖登记点」，因此**在 `set()` 之后再读派生，与在 `set()` 之前读派生，得到的依赖登记结果相同**——依赖是**读时**建立的，不是**写时**建立的。这是本引擎与「编译期静态依赖」方案的差异：Rust 没有编译期模板 ⇒ 依赖图只能在运行时建立。运行时依赖图的代价是：**首次读取前不建立边 ⇒ 首次读取时必须把「当时的全部依赖」记下来**，这由 §4.1 的「每次重跑重新登记」规则覆盖。

### 4.4 依赖图与循环检测

**图结构**（两个方向的边都要维护）：

- `dependencies: HashMap<NodeId, HashSet<NodeId>>` —— 「我读了谁」；
- `dependents: HashMap<NodeId, HashSet<NodeId>>` —— 「谁读了我」（失效传播用）。

**环检测：两类**

| 类型 | 场景 | 检测方式 | 报错 |
|---|---|---|---|
| **求值环** | A 的派生读 B，B 的派生读 A | 求值栈（evaluation stack）：进入某节点求值时压栈，若再次遇到栈内节点 ⇒ 环 | `ReactiveError::CycleDetected { path: Vec<NodeId> }` |
| **写-读自激** | effect 写了自己的依赖 ⇒ 反复触发 | 单 flush 内重算计数超上限（§4.2 规则 7） | `ReactiveError::FlushCycle { node: NodeId }` |

**错误可读性**：节点可命名（`node.set_label("ApprovalRequestCard::visible_items")`）。报错时输出**人可读的环路径**，例如 `CycleDetected: A → B → A`。这条是硬要求：引擎里最难查的 bug 就是环，报错必须自带人名。

**不做的事**：**不做**静态拓扑排序、**不做**编译期环检测（Rust 无模板 ⇒ 无静态依赖图）。环一律在运行时首次命中时暴露（这是本引擎与编译期方案的诚实取舍，写在这里以免被误读成缺陷）。

### 4.5 effect 生命周期

```rust
// 概念形状
pub fn effect(f: impl FnMut() + 'static) -> EffectHandle;
pub fn effect_pre(f: impl FnMut() + 'static) -> EffectHandle;  // 渲染前执行
pub fn effect_root(f: impl FnMut() + 'static) -> Scope;       // 创建一个作用域
```

**创建时机**

- `effect()` 在**当前 flush 的 effect 阶段**首次执行（不在创建处同步执行）⇒ 创建 effect 的组件其 DOM 此刻已就位。
- 每次重跑前，追踪域清空并重建依赖边 ⇒ 条件依赖正确。

**重跑时机**

- 依赖被标脏 ⇒ effect 进入待运行队列。**若标脏发生在本次 flush 的「脏传播」阶段**（即本轮写入），effect 在**本次 flush 的 effect 阶段**执行（渲染之后）；**若标脏发生在 effect 阶段之中**，推迟到**下一次 flush**。
- 一个 flush 内被多次标脏 ⇒ **合并为一次**重跑。

**销毁时机**

- `EffectHandle::dispose()`：解除全部依赖订阅，从队列移除。**幂等**（重复调用安全）。
- **作用域（Scope）**是销毁的组织单位：
  - `Scope::new()` 创建一个作用域；`scope.child()` 创建子作用域（子作用域随父作用域一并销毁）。
  - 作用域内创建的 effect / 派生订阅，**全部登记在该作用域上**。
  - `scope.dispose()` ⇒ 按**创建逆序（LIFO）**销毁作用域内的 effect ⇒ 子组件的 effect 先于父组件销毁（父 effect 常读子组件产出的状态，逆序避免读到已销毁对象）。
- **与组件卸载的对应**：组件卸载 = 「销毁该组件的作用域 + 移除其 DOM 节点」。二者**必须成对**：只移除 DOM 而不销毁作用域 ⇒ 泄漏订阅；只销毁作用域而不移除 DOM ⇒ 残影。`ui-dom` 的卸载路径**强制成对执行**（§5.1.4）。

**effect 的清理回调**：`EffectHandle::on_cleanup(|_| { … })` 注册清理函数，在 dispose 时按逆序执行。（对应被替换实现里 `$effect` 的返回函数清理。）

### 4.6 与 Svelte 5 runes 语义子集的映射

> 本仓前端用的是 Svelte 5（`frontend/companion-desktop/package.json`：`"svelte": "^5.16.0"`）。引擎对齐的是**语义子集**，不是语法。Rust 没有编译器魔法，所以 `$state` / `$derived` 是**显式调用**，`$props` 是**宏生成的 struct**。

| Svelte 5 语义 | Rust 侧表达 | 语义要点 | 仓内用量（实测） |
|---|---|---|:--:|
| `$state(x)` | `signal(x)` → `Signal<T>` | 显式读写；`get()` 在追踪域内登记依赖；`set()` 总是失效 | 376 处 / 45 文件 |
| `$state.raw(x)` | 同上 | **无差别**：Rust 无深层代理，`Signal<T>` 天然是「raw」语义 | — |
| `$derived(expr)` | `derived(\|\| expr)` → `Derived<T>` | 惰性求值 + 主动失效；每次重跑重建依赖 | 162 处 / 36 文件 |
| `$derived.by(fn)` | `derived(\|\| { … })` | 同上，块体形式 | 同上 |
| `$effect(fn)` | `effect(\|\| { … })` → `EffectHandle` | flush 的 effect 阶段执行（DOM 更新之后）；依赖变化时重跑 | 40 处 / 19 文件 |
| `$effect.pre(fn)` | `effect_pre(\|\| { … })` | 渲染前执行 | 同上 |
| `$effect.root(fn)` | `scope(\|\| { … })` → `Scope` | 作用域 = 组件生命周期边界 | — |
| `$effect` 返回的清理函数 | `handle.on_cleanup(\|_\| { … })` | dispose 时按逆序执行 | — |
| `$props()` | `#[prop]` 标注的 struct 字段（§6.1） | 编译期类型检查；默认值 `#[prop(default = …)]` | 55 处 / 55 文件 |
| `$bindable` | **不实现** | 本仓用量 0 | 0 |
| `$inspect` | **不实现** | 本仓用量 0 | 0 |
| `{#await}` / `use:` / `context=` / `setContext` | **不实现** | 本仓用量 0 | 0 |

**对齐的边界（诚实登记）**：本表是**语义对齐**，不是**行为逐位对齐**。差异集中在两处：

1. **依赖登记是运行时的**：被替换实现的依赖在编译期确定；本引擎在首次读时确定。因此「读取顺序影响依赖图」——这是 Rust 无模板方案的固有代价（§4.3 已登记代价）。
2. **无编译期 `$state` 语法糖**：写 `signal(x)` 而不是 `$state(x)`，可读性略降，换来的是「状态是显式的、可被 grep 的、可被类型标注的」。

### 4.7 `ui-core` 公开 API 汇总

```rust
// ── 状态 ────────────────────────────────────────────
pub fn signal<T: Clone>(value: T) -> Signal<T>;
pub fn derived<T: Clone>(f: impl Fn() -> T + 'static) -> Derived<T>;

// ── 副作用 ──────────────────────────────────────────
pub fn effect(f: impl FnMut() + 'static) -> EffectHandle;
pub fn effect_pre(f: impl FnMut() + 'static) -> EffectHandle;
pub fn scope(f: impl FnOnce(&Scope) + 'static) -> Scope;

// ── 调度 ────────────────────────────────────────────
pub fn batch(f: impl FnOnce());          // 合并写入为一次 flush
pub fn flush_now();                      // 同步跑完当前所有待办（测试用；生产不用）
pub fn set_scheduler(s: Scheduler);      // 注入执行器（wasm: 微任务 + rAF；native test: 同步）

// ── 错误 ────────────────────────────────────────────
#[non_exhaustive]
pub enum ReactiveError {
    CycleDetected { path: Vec<NodeId> },
    FlushCycle { node: NodeId },
    MutationDuringDerivation { node: NodeId },
    DisposedScope,
}
```

**Scheduler 抽象的意义**：`ui-core` 不知道「微任务」和「帧」在目标平台上怎么实现。`Scheduler` 由宿主注入：wasm 侧 = `queueMicrotask`；native 测试侧 = 同步 drain（`flush_now`）。⇒ **同一份 `ui-core` 代码，在 native 测与 wasm 跑，语义完全一致**。

---

## 5. DOM 绑定层设计（`ui-dom`）

### 5.1 虚拟节点 → 真实 DOM 的映射

#### 5.1.1 虚拟节点（VNode）结构

```rust
// 概念形状
pub enum VNode {
    Element(ElementVNode),
    Text(TextVNode),
    Fragment(FragmentVNode),   // 无宿主节点的分组（对应列表/条件块的容器）
    Comment(CommentVNode),     // 占位锚点（`{#if}` 的 else 分支等）
}

pub struct ElementVNode {
    pub tag: &'static str,            // "div" / "button" / …
    pub key: Option<Key>,             // keyed 差分用（§5.2.3）
    pub attrs: AttrMap,               // 普通属性 + 特殊槽位（class / style / 事件 / 绑定）
    pub children: Vec<VNode>,
}

pub struct TextVNode { pub data: String }
pub struct FragmentVNode { pub children: Vec<VNode> }
pub struct CommentVNode { pub data: String }
```

**`AttrMap` 的四个分区**（决定差分与写入策略）：

| 分区 | 内容 | 差分策略 |
|---|---|---|
| `class` | class 名集合 | **按 token 差分**（增删单个 class，不整体覆写）——见 §5.2.1 |
| `style` | 内联样式字符串 | 整体字符串比对，不同才写 |
| `attrs` | 其余属性（`id` / `aria-*` / `data-*` / `type` / `href` …） | 键级差分：新增则设、变更则改、消失则删 |
| `handlers` | 事件绑定 | 键级差分：类型变了则重挂 |
| `bindings` | 双向绑定（`bind:value` / `bind:checked` / `bind:this`） | 键级差分 |

**为什么 class 必须 token 差分**：本仓大量使用「状态 class 切换」模式（`.drawer.open` / `#scrim.show` / `.wb-toggle.on` / `.wb:not(.closed)`，实测见 `src/lib/design/motion.css:123,135,187`）。若整体覆写 `class` 字符串，会连带抹掉别的 class，且在有 CSS `transition` 时会产生额外的中间态。token 差分保证「只动该动的那一个 class」。

#### 5.1.2 DOM 后端抽象（`Dom` trait）—— 可 native 单测的结构前提

```rust
pub trait Dom {
    type Node: Clone + PartialEq;

    fn create_element(&self, tag: &str) -> Self::Node;
    fn create_text(&self, data: &str) -> Self::Node;
    fn create_comment(&self, data: &str) -> Self::Node;

    fn set_attribute(&self, node: &Self::Node, name: &str, value: &str);
    fn remove_attribute(&self, node: &Self::Node, name: &str);
    fn set_property(&self, node: &Self::Node, name: &str, value: &str); // value / checked
    fn get_property(&self, node: &Self::Node, name: &str) -> String;

    fn append_child(&self, parent: &Self::Node, child: &Self::Node);
    fn insert_before(&self, parent: &Self::Node, child: &Self::Node, anchor: Option<&Self::Node>);
    fn remove_child(&self, parent: &Self::Node, child: &Self::Node);

    fn set_text(&self, node: &Self::Node, data: &str);
    fn text_content(&self, node: &Self::Node) -> String;

    fn parent(&self, node: &Self::Node) -> Option<Self::Node>;
    fn first_child(&self, node: &Self::Node) -> Option<Self::Node>;

    fn add_listener(&self, node: &Self::Node, kind: EventKind, handler: Rc<EventHandler>);
    fn remove_listeners(&self, node: &Self::Node);
}
```

**两个实现**：

| 实现 | 目标平台 | 用途 |
|---|---|---|
| `InMemoryDom` | **native（永远可用）** | 单元测试：差分正确性、事件委托、绑定、挂载/卸载，全部 `cargo test` 直接跑，**不需浏览器** |
| `WebDom`（`web-sys`） | `wasm32` only | 生产：真实 DOM |

**`InMemoryDom` 必须支持事件派发**（`dispatch(node, kind, payload) -> Vec<DomEvent>`）：否则事件处理器无法在 native 测试里被调用 ⇒ 事件逻辑仍然只能靠浏览器测 ⇒ 测试基座升级不彻底。**这是硬要求**。

**架构后果**：`ui-dom` 的全部逻辑都是 `D: Dom` 泛型代码 ⇒ **差分 / 委托 / 绑定逻辑 100% 可 native 单测**。这是 §10.2「消灭源码镜像测试」的结构性前提，也是 ADR §3.3「核心层永远无法脱离 WebView 单测」这条拒理由的正面回应。

#### 5.1.3 渲染器（`Renderer<D>`）

职责：持有一棵「已挂载树」的内部表示（旧 VNode + 对应 DOM 节点），在状态变更后执行**差分**并把结果写进 `D`。

```rust
pub struct Renderer<D: Dom> {
    dom: D,
    root: D::Node,          // 挂载容器
    tree: MountedTree,      // 旧 VNode ↔ DOM 节点 的对应
}

impl<D: Dom> Renderer<D> {
    pub fn mount(&mut self, vnode: VNode) -> MountedNode;
    pub fn patch(&mut self, old: &MountedNode, new: VNode) -> MountedNode;
    pub fn unmount(&mut self, node: &MountedNode);   // 销毁作用域 + 移除 DOM（成对）
}
```

**「旧 VNode ↔ DOM 节点」的对应关系**是差分的核心：引擎保留上一帧的虚拟树，diff 时逐节点比对虚拟树，并**就地**改真实 DOM。

#### 5.1.4 挂载 / 卸载的成对契约

- **挂载**：创建作用域 → 渲染 VNode → 写入 DOM → 建立事件绑定 → **创建 effect**（在 flush 的 effect 阶段）。顺序不可颠倒：effect 必须能在 DOM 就位后读它。
- **卸载**：**销毁作用域（effect 逆序销毁）→ 解除事件绑定 → 移除 DOM 节点**。顺序不可颠倒：先销毁 effect 可以避免 effect 在 DOM 已消失时读它。
- 组件被条件块移除时，这三步**必须成对执行**，由 `Renderer::unmount` 单点保证（不允许有绕过路径）。

### 5.2 差分算法

#### 5.2.1 属性差分

- **class**：token 差分。旧 token 集合与新 token 集合求差 ⇒ 只对差集调 `classList.add/remove`。
- **style**：整体字符串比较；不同才写（写的是完整 `style` 属性值）。
- **普通属性**：键级差分（新增 / 改值 / 删除）。
- **事件绑定**：键级差分（同一事件类型换了处理器 ⇒ 重新挂载）。
- **属性写入的原子性**：**同一节点的所有属性写入必须连续完成，中间不插入节点操作**（§5.5.2 规则 T2）。

#### 5.2.2 子节点差分（无 key 时）

按索引对齐：旧子节点 `[a, b, c]` 与新子节点 `[a, b′, d]` ⇒ `a` 打补丁、`b→b′` 打补丁、`c` 移除、`d` 新建（插到 `b′` 之后）。**类型不匹配**（元素↔文本）时 ⇒ 移除旧的、建新的。

#### 5.2.3 keyed vs unkeyed 列表（`{#each}` 的取舍）

**实测口径**（`frontend/companion-desktop/src/`，`.svelte` 文件）：`{#each}` 块 **111** 处 / 35 文件，其中 **带 key 的 82 处**、**不带 key 的 29 处**（15 个文件同时含两种）。

| 形态 | 语义 | 差分策略 |
|---|---|---|
| **keyed**（`{#each xs as x (x.id)}`） | 节点身份由 key 决定 | **最长递增子序列（LIS）** 驱动的最小移动：先按 key 匹配出「旧 → 新」的对应关系；未在 LIS 中的节点需要**移动**（`insertBefore`），LIS 中的节点**原地保留**。移动次数 = `n − LIS 长度`（下界）。 |
| **unkeyed**（`{#each xs as x}`） | 节点身份由位置决定 | 按索引对齐（§5.2.2）：位置 i 的新节点复用位置 i 的旧节点（若类型兼容）。**顺序变化会导致内容重排**（等价于参考实现的行为）。 |

**决策：两种都支持，实现 keyed 的 LIS 版本。**

- 理由：111 处里 82 处是 keyed（占 74%），且 keyed 的正是**会重排的动态列表**（会话列表、工具调用列表、审批列表）—— 这些正是「重排后状态错位」类 bug 的高发区，LIS 能把移动次数降到下界。
- unkeyed 的 29 处多为**静态顺序的短列表**（常量数组、筛选项），按索引对齐足够。
- **代价（诚实登记）**：LIS 是 O(n log n) 且需要分配临时数组；本仓列表长度在几十量级，代价可忽略。**未做性能实测**（不做无实测的性能承诺）。

**key 的类型**：`Key` 是不透明的可比较值（`Key::Str(&'static str)` / `Key::Int(i64)` / `Key::Uint(u64)`），由 `view!` 宏从 `key = expr` 推出（字符串 / 整数）。**不支持复合 key**（本仓 0 处需要）。

#### 5.2.4 文本节点

文本内容变化时**就地改文本**（`set_text`），**不**重建文本节点。理由：重建文本节点会丢失该节点上的 CSS 动画状态（`::first-letter` 动画、`@property` 过渡），且与参考实现行为不一致。

### 5.3 事件委托

#### 5.3.1 事件面（实测）

本仓用的是 Svelte 5 的事件属性语法（`onclick=` / `onkeydown=` …），**Svelte 4 的 `on:` 指令精确匹配仅 2 处**（`src/components/VoiceCallModal.svelte:159,171`）。实测分布（`frontend/companion-desktop/src/`，`.svelte`）：

| 事件属性 | 处数 | 冒泡 |
|---|---:|---|
| `onclick` | 304 | ✅ |
| `onkeydown` | 41 | ✅ |
| `oninput` | 24 | ✅ |
| `onchange` | 13 | ✅ |
| `onfocusout` | 9 | ✅ |
| 其余（`onerror` / `onmousemove` / `onblur` / `onscroll` / `onpointer*` / `onmousedown` / `onfocus` / `oncontextmenu` / `onload` / `onmessage` / `onopen`） | 12 | 混合 |

#### 5.3.2 委托根选择

**委托根 = 引擎的挂载容器**（宿主给引擎的那个元素）。

- 理由：容器是**所有引擎产出的 DOM 的唯一公共祖先** ⇒ 一次挂载即可覆盖全树；挂在 `document` 上会捕获到引擎无关的节点（宿主自身的 DOM、浏览器扩展注入的节点），需要额外的「归属判定」。
- **委托根之外**的引擎产物（若试点期需要把引擎挂到多个容器）：每个容器各挂一套委托根，容器即委托根。**多容器是支持的，但不是默认**。

#### 5.3.3 冒泡 / 捕获语义

- **冒泡事件**（click / keydown / input / change / focusout / pointer\* / mouse\* / contextmenu / wheel）：**委托**到根。每个事件类型在根上只挂**一个**监听器。
- **不冒泡事件**（focus / blur / load / error / message / open）：**不委托**，直接挂在元素上。`focus` / `blur` 的替代：Svelte 5 用 `onfocusout`（冒泡）已覆盖本仓 9 处需求；本仓另有 1 处 `onfocus` / 2 处 `onblur`，迁移时改写为 `onfocusout` 或保留直挂。
- **`currentTarget` 语义（委托下的确定解）**：`currentTarget` 在参考实现里 = **注册该处理器的那一个元素**。委托模型下，引擎在沿祖先链派发时**始终知道「当前正在执行的处理器注册在哪个元素上」**⇒ `event.current_target()` 定义为**「当前正在执行的处理器所注册的元素」**，与参考实现**语义完全一致**。
  - ⇒ 仓内 19 处 `e.currentTarget` 用法（典型是「点外面才关闭」的 `e.target === e.currentTarget` 判定，如 `src/lib/components/CommandPalette.svelte:86`、`src/lib/components/WorkspacePickerModal.svelte:63`）**可原样迁移，无需改写**。
  - 附带的便利方法：`event.target_is_within(node) -> bool`（「事件目标是否落在指定元素内部，含自身」），供「点外面才关闭」这类需求直接使用。**它不是 `current_target` 的替代**，两者语义不同、并存使用。
- **捕获阶段**：引擎**不提供**捕获阶段监听（参考实现也没有）。若将来需要，属于新增能力，另议。
- **调用顺序**：目标节点上的处理器先执行，然后沿祖先链向上（冒泡阶段）。任一处理器调用 `stop_propagation()` ⇒ 停止向上传播；`stop_immediate_propagation()` ⇒ 停止当前节点上剩余处理器。
- **`prevent_default()`**：由处理器显式调用；引擎**不自动**阻止默认行为（与参考实现一致）。

#### 5.3.4 事件对象 API 面（按实测的处理器用法反推）

处理器的参数在参考实现里是 DOM 原生事件对象；Rust 侧必须换成一个**自有包装类型**，否则 403 个处理器的函数体都要重写。实测处理器对事件属性的使用频次（`frontend/companion-desktop/src/`，`.svelte`）：

| 原生属性 | 实测出现次数 | 引擎侧 API |
|---|---:|---|
| `e.key` | 50 | `event.key() -> String`（键盘事件） |
| `e.stopPropagation()` | 27 | `event.stop_propagation()` |
| `e.preventDefault()` | 22 | `event.prevent_default()` |
| `e.currentTarget` | 19 | `event.current_target() -> ElementRef`（= **当前处理器注册在哪个元素上**，委托下与参考实现语义一致，见 §5.3.3） |
| `e.clientX` / `e.clientY` | 7 / 7 | `event.client_x()` / `event.client_y()` |
| `e.target` | 4 | `event.target() -> ElementRef` |
| `e.ctrlKey` / `e.shiftKey` / `e.metaKey` | 3 / 2 / 3 | `event.ctrl_key()` / `event.shift_key()` / `event.meta_key()` |
| `e.button` | 2 | `event.button() -> i16` |
| `e.newValue`（`<select>` 变更） | 2 | 由 `event.target_value() -> Option<String>` 表达 |
| `e.detail` | 0 | 不提供（按实测用量裁剪，不预留） |

> 计数口径：正则 `\b(e|ev|evt|event)\.[a-zA-Z]+` 扫 `.svelte` 全树（实测 2026-10-24）。该前缀在仓内也用于**领域对象**（如 `e.kind` / `e.payload`），故本表**只列 DOM 事件属性**；`target_value()` 覆盖仓内 4 处 `el.value` 取值场景。

**设计原则（可推广）**：事件包装类型**按实测用量裁剪**，不按「浏览器 API 全集」照抄。少一个字段就少一份迁移摩擦，也少一份未来维护面。

**修饰键组合语义**：`ctrl_key` / `shift_key` / `meta_key` / `alt_key` 是**独立布尔量**，由处理器自己组合判断（例如 `if event.ctrl_key() && event.key() == "k"`）。引擎**不提供**组合语法糖（参考实现也没有）。

### 5.4 双向绑定

**实测**：`bind:` 共 **70** 处 / 27 文件 —— `bind:value` **45**、`bind:this` **24**、`bind:checked` **1**。

#### 5.4.1 可绑定类型与语义

| 绑定 | 适用元素 | 源事件 | 写回方向 | 语义 |
|---|---|---|---|---|
| `bind:value` | `<input type=text/password/email/url/search/tel/number/range/date/time/color>`、`<textarea>` | `input` | 信号 → DOM：写 **property** `el.value` | 值变化 ⇒ 写信号；信号变化 ⇒ 写 DOM（值不等才写，避免打断输入法组合态） |
| `bind:value` | `<select>`（单选） | `change` | 同上 | 同上 |
| `bind:checked` | `<input type=checkbox>`、`<input type=radio>` | `change` | 写 property `el.checked` | 同上；单选组内互斥由浏览器保证 |
| `bind:this` | 任意元素 | —（挂载时赋值） | DOM → 变量 | 挂载后写入元素引用，卸载前清空。**不是双向数据绑定**，是元素引用绑定 |

**明确不支持**（本仓 0 处使用，不实现）：`<input type=file>`、`contenteditable`、`<select multiple>`、表单整体 `bind:value`（对象序列化）。

#### 5.4.2 Rust 侧表达

```rust
// 概念形状
let name: Signal<String> = signal(String::new());
view! {
    input(type = "text", bind:value = name)
}
```

- 宏要求被绑定的信号类型与元素的取值类型一致（`String` / `f64` / `bool`），**编译期检查**。
- `bind:this`：`let el = ElementRef::new(); view! { canvas(bind:this = el) }`，`el.get()` 返回 `Option<ElementRef<...>>`（挂载前为 `None`）。

#### 5.4.3 双向绑定与差分的交互（易错点，必须写死）

- 信号 → DOM 的写回发生在**渲染阶段**（DOM 写入批次内），且**只在 DOM 当前值 ≠ 信号值时**才写。理由：用户在输入框里打字时，DOM 值是「正在输入」的中间态；若无条件回写，会打断输入法与光标位置。
- DOM → 信号的写回发生在**事件处理器里**（`input` / `change`），即**事件回调内直接写信号**，随后由调度器统一触发一次渲染。⇒ 输入框内容不会被「回写再被信号覆盖」地抖动。

### 5.5 CSS 过渡与关键帧动画的触发时机（本节最关键）

#### 5.5.1 事实基线（先说清楚「要保护什么」）

- 本仓**框架级过渡指令用量为 0**（`in:` / `out:` / `transition:` 作为 Svelte 指令，实测 0 处，见 [ADR-0015 §2.2 口径修正](../adr/0015-pure-rust-ui-engine.md)）。
- 本仓的 **86 处 `transition:` 全部是 CSS 属性声明**（实测 `frontend/companion-desktop/src/`），例：`src/App.svelte:3295` `transition: opacity 0.8s ease;`、`src/App.svelte:3411` `transition: border-color 0.2s ease, color 0.2s ease, background 0.2s ease, box-shadow 0.2s ease;`。
- 关键帧动画集中在 `src/lib/design/motion.css`（257 行）：`ap-shell-in`（0.55s）、`ap-card-rise`（0.5s，错落延迟 0.04/0.10/0.16/0.22s）、`ap-fade-up`（0.6s）、`ap-msg-in`（0.42s）。
- 这些动画**全部被包在 `@media (prefers-reduced-motion: no-preference)` 里**（`motion.css:7,161,193`）⇒ 减少动效偏好是**纯 CSS 侧**的事。

**结论（确定）**：**引擎不实现过渡时序引擎**（无 `in:` / `out:` / `transition:` API，`ui-core` 无过渡调度器）。引擎要做的是**不破坏** CSS 的动画与过渡。

#### 5.5.2 六条保护规则（引擎必须满足，逐条可测）

| # | 规则 | 为什么 | 可测性 |
|---|---|---|---|
| **T1** | **同标签 ⇒ 原地打补丁，绝不重建元素。** tag 相同 ⇒ 复用同一个真实 DOM 节点，只改属性与子节点。 | 重建会让 CSS `animation` 重新开始、`transition` 归零、焦点与滚动位置丢失 ⇒ 可见差异 | 断言 DOM 节点身份不变（`InMemoryDom` 节点句柄比对） |
| **T2** | **同一节点的所有属性写入连续完成**，中间不插入节点增删操作。 | 避免产生中间计算样式 ⇒ 避免触发多余的 transition 起点 | 记录写入序列断言相邻性 |
| **T3** | **新插入节点先写完全部属性，再进入文档树**（或在同一批次内先写后插）。 | 若「先插入、后改 class」，浏览器第一次样式计算看到的是起始态，`base → active` 的 CSS `transition` **不会触发** ⇒ 与参考实现不一致 | 断言插入顺序（先属性后挂载） |
| **T4** | **引擎零自主动画**：不得用 JS 驱动任何 opacity / transform 插值，不得自建 rAF 动画循环（`ui-gpu` 的绘制循环除外）。 | `prefers-reduced-motion` 的门是 CSS 侧的；JS 动画会绕过它 ⇒ 违反 `01-DESIGN-SYSTEM.md` §7.3 | 代码审查 + 引擎层不出现插值 API |
| **T5** | **移除时机与参考实现一致**：参考实现立即移除的，引擎立即移除（**不加延迟移除**）。 | 参考实现没有 `out:` ⇒ 离场过渡本来就不存在；引擎若「更聪明」地延迟移除 ⇒ **产生参考实现没有的画面 ⇒ 破坏视觉零改写** | 逐帧对照（§9.4） |
| **T6** | **同一批次内「插入」与「移除」互不干扰**：被移除的节点不参与本批次的属性写入。 | 避免对已脱离文档的节点写入属性（无意义且可能报错） | `InMemoryDom` 断言无对已移除节点的后续写入 |

**关于「条件块销毁过早会丢离场动画」这一风险的正解**：

- 风险**真实存在**，但它属于**参考实现自身的行为**（无 `out:` ⇒ 离场动画必丢），不是引擎引入的。
- 引擎的职责**不是**修复它，而是**不引入新的不一致**：不延迟移除（T5）、不重建节点（T1）、不在插入时制造中间态（T3）。
- 引擎**也不需要**读 `getComputedStyle` 去探测过渡时长 —— 因为没有任何「需要等过渡结束再移除」的场景（T5 已排除）。⇒ §2.2 的「例外条款（唯一）」在本设计下**未被使用**。若将来确需框架级过渡（例如要让「面板收起」有离场动画），那是一次**新增需求**，须另出 ADR 并重新评估 §2.2 的例外条款。

**缓动曲线与时长的归属**：全部在 CSS。`--ap-ease-standard: cubic-bezier(0.645, 0.045, 0.355, 1)`（`src/lib/design/tokens.css:136`，即设计系统 §7.1 的 `easeInOutCubic`）、`--ap-dur-ui: 2s` / `--ap-dur-camera: 2.6s` / `--ap-dur-breathe: 2.8s` / `--ap-dur-pulse: 3.3s`（`tokens.css:137-140`）。**引擎不读、不写、不复制这些值。**

#### 5.5.3 与响应式调度的交互

- 状态变化 ⇒ 微任务 flush ⇒ DOM 写入批次（含 class token 差分）⇒ 浏览器在本次绘制前完成样式重算 ⇒ CSS `transition` 正常触发。
- **过渡进行中状态再变** ⇒ class 再次被差分（token 增删）⇒ 浏览器按 CSS 规则继续或反向过渡。引擎**不介入**、**不排队**、**不记忆**过渡状态 ⇒ 与参考实现一致。
- **过渡进行中节点被移除** ⇒ 节点连同其过渡一起消失。引擎不干预（T5）。

### 5.6 装配与调度交互小结

```
用户事件 ──▶ 事件处理器（委托根派发）
                 │
                 ├─▶ Signal::set ──▶ 标脏（微任务 flush）
                 │                        │
                 │                        ├─▶ 脏传播 → 派生重算（惰性）
                 │                        ├─▶ DOM 写入批次（差分 → 属性/节点操作）
                 │                        └─▶ effect 执行（读得到已更新的 DOM）
                 │
                 └─▶ 双向绑定：DOM → Signal（同一路径上行）
```

**不变式**（可作断言用）：
1. 一次事件 ⇒ 至多一次 DOM 写入批次；
2. effect 执行时，DOM 已是最新；
3. 双向绑定的回写不产生第二次 DOM 写入（值相等时跳过）。

---

## 6. 组件模型与过程宏（`ui-macro`）

### 6.1 `#[component]` 宏的输入输出形状

**输入**：一个自由函数，参数即 props，函数体用 `view!` 构造虚拟树。

```rust
// 输入（人写的）
#[component]
fn ApprovalRequestCard(
    /// 待签文书
    request: ApprovalRequest,
    /// 紧凑排版
    #[prop(default = false)]
    compact: bool,
    /// 子内容插槽
    children: Children,
) -> VNode {
    view! {
        div(class = "approval-card") {
            span(class = "approval-card__stamp") { "编号 " }
            ApprovalRequestBody(request = request, compact = compact)
            children()
        }
    }
}
```

**输出**（宏展开后等价于）：

```rust
pub struct ApprovalRequestCardProps {
    pub request: ApprovalRequest,
    pub compact: bool,
    pub children: Children,
}

pub struct ApprovalRequestCard { /* 无字段：组件即类型 */ }

impl ApprovalRequestCard {
    pub fn new(request: ApprovalRequest) -> Self { /* 填默认值 */ }
    pub fn compact(mut self, v: bool) -> Self { self.props.compact = v; self }
    pub fn children(mut self, c: Children) -> Self { self.props.children = c; self }
}

impl Component for ApprovalRequestCard {
    type Props = ApprovalRequestCardProps;
    fn render(props: &Self::Props) -> VNode { /* 原函数体 */ }
}
```

**约定**

| 约定 | 规则 |
|---|---|
| 哪些参数是 props | **全部**参数都是 props；`#[prop(default = …)]` 的参数可省略（构造器不强制传）。`children` 参数名固定为 `children`，类型 `Children`。 |
| props 的类型 | **必须是普通 Rust 类型**（可 `Clone`）。宏生成 `Props` struct，字段名 = 参数名，字段类型 = 参数类型。 |
| 组件的调用形态 | `Component::render(&props) -> VNode`，**纯函数**、无副作用、不碰 DOM。 |
| 组件的实例化 | `Name::new(必填参数).可选参数(值)…`；`children` 通过 `.children(c)` 传入。 |
| 泛型 props | **不支持**（宏展开的 struct 不带泛型参数，避免类型推断地狱）。 |
| 生命周期钩子 | **不提供**（`onMount` / `onDestroy` 一律用 `effect` + 作用域表达，见 §4.5）。 |

### 6.2 props / children / 插槽

- **props**：编译期类型检查的 struct。**这是 D3 债（前端手写 TS 类型对上 Rust 后端结构会漂移）的根解法**：Rust 结构 → UI 组件，中间没有第二份手写类型。
- **children（匿名插槽）**：`Children` = 一个「渲染出一棵子树」的闭包。

  ```rust
  pub type Children = Rc<dyn Fn() -> VNode>;
  ```

  语义：`children()` 在组件内被调用时，**当场**求值 ⇒ 子树在父组件的响应式上下文里渲染 ⇒ 子树能读到父组件的信号。
- **具名插槽（slots）**：**本期不实现**。理由：本仓 `{#snippet}` 仅 **2** 处（`src/lib/chat-shell/SessionListHome.svelte:356`、`src/lib/views/SettingsView.svelte:1968`），且两处都是「把一段行渲染逻辑参数化」，在 Rust 里用**普通泛型函数参数**即可表达（把渲染函数当参数传），不需要具名插槽机制。
  - 迁移写法：`{#snippet sessionRow(item, …)}` ⇒ `fn session_row(item: &HomeSessionItem, …) -> VNode`，作为普通函数参数传入。**这是普通函数，不是插槽。**

### 6.3 组件树与生命周期

```
AppHandle
 └── Scope(root)
      ├── Scope(App)          ── 卸载时按 LIFO 逆序销毁
      │    ├── Scope(Sidebar)
      │    └── Scope(Chat)
      └── Scope(StatusBar)
```

| 阶段 | 动作 |
|---|---|
| **mount** | 创建作用域 → `render(props)` 得到 VNode → `Renderer::mount` 写入 DOM → 建立事件绑定 → 在 flush 的 effect 阶段创建 effect |
| **update** | 依赖变化 ⇒ 该组件的渲染 effect 重跑 ⇒ 得到新 VNode ⇒ `Renderer::patch` 就地差分 ⇒ DOM 更新 |
| **unmount** | 作用域 dispose（effect 逆序销毁）→ 解除事件绑定 → 移除 DOM 节点（三步成对，§5.1.4） |

**组件树 = 作用域树**：每个组件实例持有一个 `Scope`；父组件卸载 ⇒ 子组件作用域随之销毁（子先于父，§4.5）。

### 6.4 组件能否被普通 `cargo test` 直接调用（核心诉求）

> **能。而且这是本设计的结构性目标，不是附加功能。**

三句话说清：

1. **组件是纯函数**：`fn(Props) -> VNode`。native 下直接 `ApprovalRequestCard::render(&props)` 就能拿到虚拟树，**不需要浏览器、不需要 wasm**。
2. **虚拟树是普通数据结构**：`VNode` / `ElementVNode` 都是可 `PartialEq`、可 `Debug` 的普通 Rust 类型 ⇒ 断言写成 `assert_eq!(vnode, expected)`，而不是 `assert!(src.contains("…"))`。
3. **整条渲染链也能 native 跑**：把 `InMemoryDom` 传给 `Renderer` ⇒ 挂载 → 差分 → 事件派发 → 双向绑定，**全在 native `cargo test` 里**。

**因此，38 个源码镜像测试的替代写法是确定的**：

| 镜像断言（病根） | 替代（行为断言） |
|---|---|
| `expect(src).toContain('await deleteSession(id, {')` | 调组件渲染出的处理器 → 断言**发出的请求载荷**（用假后端） |
| `expect(src).toMatch(/if \(.*\)\s*\{/)` | 渲染组件 → 断言 **VNode 树里有没有那个分支** |
| `expect(src).toContain('onclick=')` | 渲染组件 → 断言 **事件处理器已绑定**（`InMemoryDom` 派发事件 → 断言副作用发生） |
| `expect(src).toContain('class="foo"')` | 渲染组件 → 断言 **class token 集合** |
| `expect(src).toMatch(/\\$state/)` | 断言 **信号行为**（写信号 → 派生更新 → DOM 变化） |

**诚实的边界**：native 侧**测不出**的东西只有一类 —— **真实浏览器里的布局与合成结果**（像素、层叠、`backdrop-filter` 合成、焦点顺序、真实输入法）。这一类由 §10.1 的 L3（浏览器内对照测试）承担。**「组件可 native 测」不等于「视觉可 native 测」。**

### 6.5 `view!` 宏文法（确定）

```text
view! {
    // 元素
    div
    div(class = "a b")
    div(class:open = is_open)          // class token 条件增删
    input(type = "text", value = name, disabled = is_disabled)

    // 事件（与仓内 Svelte 5 事件属性同名，降低迁移摩擦）
    button(onclick = move |_| { … })
    input(onkeydown = move |ev| { … })

    // 双向绑定
    input(bind:value = name)
    input(bind:checked = flag)
    canvas(bind:this = canvas_ref)

    // key（供 keyed 差分）
    li(key = item.id)

    // 子节点
    div { "文本" }
    div { expr }                        // 动态文本（Display）
    div { { expr } }                    // 动态子节点
    div { ChildComponent(arg = v) }     // 子组件
    div { ChildComponent(arg = v) { "插槽内容" } }   // 子组件 + children

    // 结构块
    if cond { … } else { … }
    for item in items { … }             // 列表；可带 key = expr
}
```

**规则**

| 项 | 规则 |
|---|---|
| 属性值 | 字面量（字符串 / 数字 / 布尔）或表达式。布尔 `false` / `None` ⇒ **不写该属性**（而非写 `false`）。 |
| `class` | 可重复；`class = "a b"` 与 `class:cond = expr` 可混用；最终合成 token 集合（去重、保序）。 |
| `style` | 内联样式字符串。**纪律**：除参考实现原本就有的内联样式外，**禁止**使用（§9.2）。 |
| 事件名 | 采用 DOM 事件名（`click` / `keydown` / `input` / …），处理器签名 `Fn(&DomEvent)`（§5.3.4）。 |
| `if` / `for` | 编译为 `Fragment` 节点；`for` 可选 `key = expr`。 |
| 表达式求值时机 | **在渲染时求值**（不是宏展开时）⇒ 表达式里可以读信号。 |

**宏不做的事**：不生成内联样式、不生成 CSS、不做类型推导以外的魔法、不支持 `await`、不支持上下文（context）传递。

### 6.6 `ui-macro` 的零依赖约束

`ui-macro` 是 proc-macro crate：**不依赖任何内部 crate**（ADR §3.3：最底层，零内部依赖），外部依赖只有 proc-macro 三件套（`proc-macro` / `quote` / `syn`）。**P0 探测已查清多版本情况：整棵树里唯一的双版本包是 `syn`（2.0.119 来自 zerocopy-derive 链 / 3.0.9 来自 wasm-bindgen-macro-support 0.2.129）**；三件套的**具体版本待 P1 引入时定**（§11.2 P0-d）——它们会带入 `proc-macro2` / `unicode-ident` 等传递依赖，与主仓 `deny.toml` 的 `multiple-versions = "deny"` 策略相关（独立 workspace 内的 `deny.toml` skip 登记口径同样待定）。

---

## 7. GPU 桥设计（`ui-gpu`）

### 7.1 wgpu 30 的接入方式

- 依赖：`wgpu` **30.0.1** + `naga` **30.0.1**（同一发布序列）。**P0 探测已实测**：两个后端（`webgl` / `webgpu`）在 `wasm32-unknown-unknown` 下**均编译通过**（[ADR-0015 §2.6](../adr/0015-pure-rust-ui-engine.md) 表 2）。
- 接入路径（标准 wgpu 流程，形态确定）：

  ```
  Instance → request_adapter → request_device → create_shader_module
           → create_render_pipeline → create_bind_group → render loop
  ```

- **画布目标**：`wgpu::Surface` 的 `SurfaceTarget::Canvas`，画布来自 `web-sys` 的 `HtmlCanvasElement` / `OffscreenCanvas`。
- **后端选择（已定）**：**T2 梯队默认走 `webgl` 后端**（WebGL2 在所有 WebView 都有），**WebGPU 作为可选加速**。依据：宿主 WebView 是否提供 WebGPU **本机未测（无浏览器环境）**，而 `webgl` / `webgpu` 两后端均已实测编译通过 ⇒ 选**覆盖面确定**的那个做默认。
  - 产物尺寸实测（release `.wasm`，含真实导出非空壳）：`webgl` **4,534,454 B** / `webgpu` **2,860,060 B**（[ADR-0015 §2.6](../adr/0015-pure-rust-ui-engine.md) 表 5）。
  - **诚实边界**：后端**选型**已定；**画面**未在真实浏览器 WebView 里跑过（§7.6 P0-c）。
- **着色器语言**：见 §7.2。
- **`ui-gpu` 的职责边界**：只做「**把 `ui-core` 的状态翻译成 uniform，并驱动绘制**」。它**不改着色器数学**（光学数值禁改，见 `blackhole.ts:122` 的源码注释「光学数值禁止改动」）。

### 7.2 GLSL 输入路径与「1,037 行能否原样复用」的边界

**事实**（`frontend/companion-desktop/src/lib/scene/blackhole.ts`，实测 1,037 行）：

| 事实 | 出处 |
|---|---|
| 顶点着色器为 `#version 300 es`（GLSL ES 3.00），单行 `gl_Position` | `blackhole.ts:129-131` |
| 片元着色器同为 `#version 300 es`，首行 `precision highp float;` | `blackhole.ts:138-139` |
| 片元着色器 `void main()` | `blackhole.ts:256` |
| uniform 共 16 个（`uRes` / `uCenter` / `uScale` / `uTime` / `uRot` / `uBright` / `uTurb` / `uPulse` / `uIncl` / `uAzim` / `uDiskAzim` / `uDiskIncl` / `uAmbStr` / `uEclipse` / `uTlBright` / `uTlGold`） | `blackhole.ts:140-156` |
| uniform 名数组（可直接用作 binding 映射表的来源） | `blackhole.ts:337`（`UNIFORM_NAMES`） |
| 上下文获取为 `getContext('webgl2', …)` | `blackhole.ts:756` |

#### 7.2.1 P0 探测结论（已定，权威来源 [ADR-0015 §2.6 与 §8.2](../adr/0015-pure-rust-ui-engine.md)）

> **结论：naga 能吃下现有 GLSL，着色器本体不需要重写** —— 但需要 **2 步机械变换**，代价约 **30 行前置编译器**。
> （注：「1,037 行」是 `blackhole.ts` 这个 **TS 文件**的总行数，其中绝大部分是 JS 胶水与参数计算、不是着色器；「不需要重写」指的是**着色器主体**，详见 §7.2.2 的三层切分。）

**两步机械变换**（确定性，可写成纯函数 + 单测）：

| # | 变换 | 理由（上游事实） |
|---|---|---|
| **X1** | **删掉 `#version 300 es` 行** | naga 的 GLSL 前端**没有 ES profile**：`naga-30.0.1/src/front/glsl/parser.rs:267` 只收 `#version 440 \| 450 \| 460` + `core` |
| **X2** | **每个 uniform 注入 `layout(binding=N)`** | naga 要求 uniform 显式绑定：`naga-30.0.1/src/front/glsl/variables.rs:618` |

**三条硬约束（写入 `ui-gpu` 的前置设计）**：

| # | 约束 | 上游依据 |
|---|---|---|
| **C1** | **引擎须内置 GLSL ES 3.0 → naga IR 的前置变换**（即上表 X1 + X2，约 30 行） | `naga-30.0.1/src/front/glsl/parser.rs:267` |
| **C2** | **`glsl` feature 必须与 `wgsl` 同开** | 上游缺陷：`front/mod.rs:5` 把 `interpolator` 模块 gate 在 `spv-in`/`wgsl-in` 之后，而 `front/glsl/variables.rs:467` **无条件调用**它 ⇒ 只开 `glsl-in` **编译不过**。**naga 30.0.1 未修** |
| **C3** | **naga 的 IR 把每个 GLSL uniform 变成独立 binding**（`layout(std140) uniform` 块，binding 0..15）；**再吐出时 uniform 名被抹成 `_group_0_binding_N_fs`** ⇒ **引擎须自行维护「uniform 名 → binding 索引」映射表** | `blackhole.ts:337` 的 `UNIFORM_NAMES` 正好可用（顺序即 binding 0..15） |

**往返证据（实测）**：`naga` 解析 → 校验 → 用 `naga::back::glsl::Writer` 吐出 **17,099 字节 `#version 300 es`** —— 16 个 `layout(std140) uniform` 块（binding 0..15）、`fragColor` 与 `layout(location = 0) out vec4` **保住**、5 个 helper 函数（`hash21` / `vnoise` / `fbm` / `diskRadiance` / `starLayer`）**全在**；且**吐出的 GLSL ES 3.00 能被 naga 自己重新解析并重新校验通过**。同一份代码编成 wasm32 在 node 里跑，**结果与 native 完全一致**。（上游同一条路径：`wgpu-hal-30.0.1/src/gles/device.rs:364`。）

**依赖树实测**：wgpu-webgl **108 条目 / 68 crate**，**唯一多版本包只有 `syn`**（2.0.119 来自 zerocopy-derive 链 / 3.0.9 来自 wasm-bindgen-macro-support 0.2.129）；`hashbrown` / `bitflags` / `itertools` / `miniz_oxide` / `ahash` / `crossbeam-*` 实测**未进树**；`jni-sys` 双版本仅出现于 native 目标产物，**wasm 侧 0 多版本**。

#### 7.2.2 「原样复用」的准确含义（三层边界）

| 层 | 能否原样复用 | 说明 |
|---|---|---|
| **着色器数学主体**（片元着色器里的 smoothstep 混合曲线、双多普勒增亮、色温梯度、星野三层视差） | **能** | 它是纯算术，只依赖 uniform 与 `gl_FragCoord`。**光学数值一个字都不改**（这既是视觉纪律，也是「电影感零改写」的物理保证）。**P0 往返实测已证明：naga 往返后 5 个 helper 函数与输出变量全部保住** |
| **GLSL 源文本** | **能，但要两步机械变换** | X1（删版本行）+ X2（注入 `layout(binding=N)`）。改的是**方言外壳**，**不是数学**；变换规则是确定性的，可写成纯函数 + 单测 |
| **WebGL2 胶水层**（`createShader` / `uniform*` 上传 / VAO / `drawArrays`） | **不能，必须重写** | WebGL2 的 API 面与 wgpu 无对应关系。替换为 wgpu 的 pipeline / bind group / queue 写入 |

**因此本节的确定结论是**：

> **复用 = 「着色器数学主体原样复用 + GLSL 方言外壳两步机械变换（X1/X2）+ WebGL2 胶水层重写」。**
> 「1,037 行原样复用」这个说法**仍不成立**（那 1,037 行里绝大部分是 JS 胶水与参数计算，不是着色器）；**「着色器主体不需要重写」这个说法成立且已实测**。

**不做什么**：不把 GLSL 手工翻译成 WGSL（那是「改写光学数值」的风险面，且与 ADR §2.1 的零改写纪律冲突）。**已定路径 = naga 的 GLSL 前端 + X1/X2 前置变换**；WGSL 路径不启用。

**正面结论（光学数值零改动风险的论证）**：**WebGL 路线全程留在 GLSL ES，不经过 WGSL** ⇒ 手写 GLSL 的**光学数值零改动风险**成立。

**诚实边界（不得过度声称）**：以上全部是**编译期与解析期**实测（P0 探测在独立 scratch 工程 `target/ui-probe-scratch/` 内完成，0 写主仓），**未在真实浏览器 WebView 中跑过画面**。凡涉及「画面一致」的表述一律以 **§9 的浏览器内对照测试（P3 试点门 / P5 GPU 阶段）** 为准。

### 7.3 与 `ui-core` 的接口：presence 状态 → GPU uniform

**接口形态**（确定）：`ui-gpu` 订阅 `ui-core` 的信号，把信号值写进一个 uniform 缓冲；**`ui-gpu` 不认识 `PAD`、不认识 `presence`、不认识业务**。

```
presence_state 帧 ──▶ (ui-demo / ui-kit 层做语义映射) ──▶ ui-core 信号
                                                              │
                            ┌─────────────────────────────────┴──────────────────┐
                            ▼                                                        ▼
                     ui-dom（DOM 投影）                                        ui-gpu（GPU 投影）
```

- **信号 → uniform 的映射表**（在 `ui-kit` / `ui-demo` 层定义，**不在** `ui-gpu`）：PAD → 视觉量的映射规则来自 `01-DESIGN-SYSTEM.md` §4.1（转速 `speed` / 亮度 `bright` / 湍流 `turb` / 呼吸 `pulse`）；机位五元组来自 §4.3；鼠标视差与自动漂移来自 §4.4 / §4.5。
- **uniform 结构的 Rust 侧形状**：沿用仓内既有先例 `crates/adapters/gateway/src/ember_hud_driver.rs:18-29` 的 `EmberShaderUniforms`（`time_secs` / `base_intensity` / `breath_amplitude` / `current_glow_intensity` / `color_temperature_k` / `rgb_tint` / `vignette_radius` / `vignette_softness` / `stance_label`）——即「Rust 结构 → GPU uniform」这条路上仓内已经有成型做法，`ui-gpu` 沿用同一形状与同一纪律。
- **`ui-gpu` 的公开 API**：

  ```rust
  pub struct GpuContext;                    // 实例/适配器/设备/表面
  pub struct SceneUniforms { /* … */ };     // 与着色器 uniform 一一对应
  pub enum Tier { T0, T1, T2 };             // 渲染梯队（§7.5）
  pub trait TierProbe { fn probe(&self) -> Tier; }
  ```

- **binding 索引表（新增，确定）**：因 naga 把每个 uniform 变成独立 binding 且**抹掉原名**（§7.2.1 约束 C3），`ui-gpu` 必须持有「**uniform 名 → binding 索引**」的映射表。其权威来源是 `blackhole.ts:337` 的 `UNIFORM_NAMES`（数组顺序 = binding 0..15）。该表是**引擎侧常量**，与着色器源一一对应 ⇒ 着色器改 uniform 数量时**必须同步改表** ⇒ 由 §10.2 的行为断言（native 单测）钉住：表长度 = 着色器 uniform 声明数。

- **可 native 单测的部分**：uniform 合成（状态 → uniform 值 → 字节布局）是纯计算 ⇒ native 单测；X1/X2 前置变换是纯文本变换 ⇒ native 单测（输入 = 原始 GLSL 文本，输出 = naga 可解析的文本）。**真实绘制**必须在 wasm + 浏览器里验（§10.1 L3），且**画面一致性尚未在真实 WebView 中验证过**（§7.6 P0-c）。

### 7.4 与 `ui-dom` 的关系：平级，不是父子

```
                 ┌───────────────┐
                 │   ui-core     │   ← 唯一的状态源
                 └───────┬───────┘
             ┌───────────┴───────────┐
             ▼                       ▼
      ┌────────────┐          ┌────────────┐
      │  ui-dom    │          │  ui-gpu    │
      │ （DOM 投影）│          │ （GPU 投影）│
      └────────────┘          └────────────┘
             ▲                       ▲
             └─────── ui-kit ────────┘
```

- **平级的含义**：① `ui-dom` **不依赖** `ui-gpu`，`ui-gpu` **不依赖** `ui-dom`；② 两者只共享 `ui-core`；③ 谁也不知道谁的存在。
- **哲学对齐**：`00-PHILOSOPHY.md` §2「**四个投影显影同一份状态**」——DOM 显影与 GPU 显影是同一份 presence 状态的两种投影，**不允许任何一方成为另一方的子集**。
- **工程后果**：GPU 路径在 P5 之前根本不启用（ADR §3.3 拒理由 B），因此 **`ui-dom` 的构建与测试完全不受 `wgpu` 依赖树影响** —— 这正是「平级」带来的隔离收益。

### 7.5 三后端降级梯队（对齐 `00-PHILOSOPHY.md` §8）

**梯队定义**（范式层，最高优先，`00-PHILOSOPHY.md` §8）：

| 梯队 | 形态 | presence 显影 | 目标设备 | 引擎侧对应 |
|---|---|---|---|---|
| **T0 节能** | 纯聊天壳（CSS 余烬点 + 输入状态字） | 4s 呼吸余烬 | 任意能开浏览器的机器 | `ui-dom` only；`ui-gpu` **完全不启动** |
| **T1 氛围** | 壳 + Canvas2D 星野 / 微光 | +光晕、姿态变色 | 老集显 | `ui-gpu` 走 **Canvas2D 降级路径**（对应 `blackhole.ts:461,467` 的既有 2D 分支） |
| **T2 沉浸** | 完整黑洞场景 | 吸积盘转速 / 呼吸 / 湍流全参数 | 有像样 GPU | `ui-gpu` 走 **wgpu 路径**；**默认后端 = `webgl`**（§7.1），WebGPU 作可选加速 |

**梯队探测**（`00-PHILOSOPHY.md` §8）：WebGL2 支持 + 首帧帧率探针 + `session_settings` 手动覆盖；默认档位由首启向导确认。**探测与档位决策在应用层，`ui-gpu` 只接受一个 `Tier` 输入并按它选路径。**

**两套梯队表的关系（必须说清，避免误读）**：`01-DESIGN-SYSTEM.md` §7.2 另有一套 `t0`–`t3` 表（含「循环视频」「静态星图」两档），那是**令牌层**对原型的描述；`00-PHILOSOPHY.md` §8 的 T0/T1/T2 是**范式层**的档位，**冲突时以范式层为准**。对应关系：`T0 ≈ t3`、`T1 ≈ t1`、`T2 ≈ t0`。

**铁律（`01-DESIGN-SYSTEM.md` §7.2）**：**每档都保持状态驱动**。降级降的是渲染成本，不是真实性；任何一档都不做死壁纸。⇒ `ui-gpu` 的三条路径（T2 / T1 / T0）**都必须**接受同一份 `presence` 状态，只是表达方式不同。

### 7.6 本节的待测项（诚实边界）

| # | 待测事实 | 为什么还没测 | 何时测 |
|---|---|---|---|
| **P0-c** | **宿主 WebView 的画面一致性**（WebGPU 是否真在 WebView 里可用、T2 画面是否与既有 WebGL2 版一致） | P0 探测环境**无浏览器**；已测的只是**编译期与解析期** | P5 GPU 阶段 + §9.4 逐帧对照 |
| **P0-f** | **Canvas2D 降级路径移植到 `ui-gpu` 的成本** | 未实测（需读 `blackhole.ts` 的 2D 分支，与 `ui-gpu` 设计强相关） | P5 GPU 阶段开工前 |

> **不得过度声称**：§7.1–§7.2 的全部实测结论都是**编译期与解析期**的。**「画面一致」这件事在真实浏览器 WebView 里一次都没跑过** —— 凡涉及画面的断言，一律以 §9.3 / §9.4 的浏览器内对照测试为准，且 P0-f 未测前，T1 梯队**不得开工**。

---

---

## 8. 组件库规划（`ui-kit`）

### 8.1 组件清单的推导方法

**不凭感觉列清单**，而是从仓内既有实现反推：

1. 统计 `frontend/companion-desktop/src/` 的 `.svelte` 文件：**62** 个（实测）；
2. 其中**可复用组件**（不含页面级视图）分布在两处：
   - `src/lib/components/`：**26** 个（实测）；
   - `src/components/`：**5** 个（实测：`PageHeader` / `StatusDot` / `ExecutionTimeline` / `TaskCard` / `VoiceCallModal`）；
   - 合计 **31** 个共享组件；
3. 其余为页面级视图（`src/lib/views/`、各 feature 目录）与场景层（`src/lib/scene/` / `src/lib/bridge/` / `src/lib/cabin/` / `src/lib/intro/`）—— 这些是**试点迁移对象**，不是 `ui-kit` 的组件。

### 8.2 既有 31 个组件 → `ui-kit` 模块对照

| 既有组件（`.svelte`） | `ui-kit` 模块 | 类别 | 备注 |
|---|---|---|---|
| `ApprovalRequestCard` | `approval::ApprovalRequestCard` | 卡片 | **P3 试点目标**（ADR §4） |
| `ProposalCard` | `approval::ProposalCard` | 卡片 | 与审批卡共用「待签文书」语言（`00-PHILOSOPHY.md` §6 原则 4） |
| `CareCard` | `cards::CareCard` | 卡片 | |
| `TraitCard` | `cards::TraitCard` | 卡片 | |
| `TaskCard` | `cards::TaskCard` | 卡片 | |
| `ToolCallCard` | `cards::ToolCallCard` | 卡片 | 灰（工具执行卡，`00-PHILOSOPHY.md` §3） |
| `ToolCallLifecycleCard` | `cards::ToolCallLifecycleCard` | 卡片 | |
| `StatusBadge` | `feedback::StatusBadge` | 徽章 | 语义色只用 `--ap-semantic-*`（设计系统 §2.4） |
| `StatusDot` | `feedback::StatusDot` | 状态点 | |
| `StatusBar` | `feedback::StatusBar` | 状态条 | 余光投影 |
| `LoadingState` | `feedback::LoadingState` | 状态 | |
| `ErrorState` | `feedback::ErrorState` | 状态 | |
| `EmptyState` | `feedback::EmptyState` | 空态 | **空态即契约**（`00-PHILOSOPHY.md` §6 原则 5）：写「当 X 发生时这里会出现 Y」，不写「暂无数据」 |
| `ErrorSolutionBanner` | `feedback::ErrorSolutionBanner` | 横幅 | |
| `ConfirmDialog` | `overlay::ConfirmDialog` | 对话框 | |
| `RuntimeModal` | `overlay::RuntimeModal` | 对话框 | |
| `WorkspacePickerModal` | `overlay::WorkspacePickerModal` | 对话框 | |
| `VoiceCallModal` | `overlay::VoiceCallModal` | 对话框 | **唯一含 Svelte 4 `on:` 指令的组件**（`:159,171`） |
| `CommandPalette` | `nav::CommandPalette` | 导航 | Ctrl+K 唯一入口（原则 3） |
| `ComposerMenu` | `input::ComposerMenu` | 输入 | |
| `SessionModelPicker` | `input::SessionModelPicker` | 输入 | |
| `FirstRunWizard` | `input::FirstRunWizard` | 向导 | |
| `AvatarSettingsPanel` | `settings::AvatarSettingsPanel` | 面板 | |
| `ThemeSettingsPanel` | `settings::ThemeSettingsPanel` | 面板 | **零改写**：只操作 `html[data-theme]` |
| `UserAdminPanel` | `settings::UserAdminPanel` | 面板 | |
| `SkillShelfPanel` | `settings::SkillShelfPanel` | 面板 | |
| `StarportPanel` | `settings::StarportPanel` | 面板 | |
| `MetricsPanel` | `data::MetricsPanel` | 数据面板 | 深舱调 |
| `ExecutionTimeline` | `data::ExecutionTimeline` | 数据面板 | |
| `Workbench` | `shell::Workbench` | 工作台 | 深舱调 |
| `PageHeader` | `shell::PageHeader` | 页头 | |
| （无既有实现）| `input::Button` / `input::Switch` / `input::Tabs` / `input::TextField` | 基础件 | 既有实现里按钮/开关/标签页是**内联的**（散落在各视图），迁移时抽成基础件 |

**「列表 / 卡片 / 输入 / 对话框 / 开关 / 徽章 / 标签页」的落位**：

| 需求 | 落位 |
|---|---|
| 按钮 | `input::Button`（既有实现无独立组件，需抽取） |
| 列表 | `data::List`（承担 keyed 列表渲染，是 §5.2.3 的主要消费方） |
| 卡片 | `cards::*`（7 个既有卡片组件各自独立：`ApprovalRequestCard` / `ProposalCard` / `CareCard` / `TraitCard` / `TaskCard` / `ToolCallCard` / `ToolCallLifecycleCard`；共用 `cards::CardShell` 基础壳） |
| 输入 | `input::*`（`TextField` / `ComposerMenu` / `SessionModelPicker`） |
| 对话框 | `overlay::*`（3 个既有模态 + 共用 `overlay::ModalShell`） |
| 开关 | `input::Switch`（既有实现内联，需抽取） |
| 徽章 | `feedback::StatusBadge` |
| 标签页 | `input::Tabs`（既有实现在治理卷宗里内联，需抽取） |

### 8.3 组件的令牌消费纪律

**硬规则（写进 `ui-kit` 的代码评审清单）**：

| # | 规则 | 检查方式 |
|---|---|---|
| 1 | 组件**只输出 class 名**，样式值一律在 CSS 资产里 | 组件源码里出现颜色字面量 ⇒ 评审拒 |
| 2 | 颜色 / 尺寸 / 时长**一律走 `--ap-*` 变量**（`src/lib/design/tokens.css`） | 同上 |
| 3 | **不写内联 `style`**，除非参考实现原本就有内联样式（且逐字一致） | 与 §9.2 的 golden 比对 |
| 4 | **不设 `id`**（除非参考实现有），DOM 定位靠 class 与结构 | 避免与既有 `id`（如 `#vignette` / `#scrim` / `#drawerScrim`）冲突 |
| 5 | **不改 `<html>` 的 `data-theme` / `data-accent`**（主题切换是宿主/应用层的事） | 引擎层不碰 |
| 6 | 语义色只用 `--ap-semantic-*`；**存在金 `--ap-gold*` 只用于「他」相关** | 金色纪律（`00-PHILOSOPHY.md` §7） |
| 7 | 深舱识别色 ≤6 且不含金（设计系统 §5.6 纪律 4） | 复用既有 `--ap-register-deepops-*` |

### 8.4 `ui-kit` 的组织与产物

- **Rust 侧**：`ui-kit` 只含 Rust 模块（组件实现 + props struct）。**不含 CSS**。
- **CSS 侧**：样式资产分两类（详见 §9.5）——
  1. **共享设计层**：`src/lib/design/*.css`（10 个文件 / 4,567 行，实测）——**原样复用，零改动**；
  2. **组件层**：既有组件内联的 `<style>` 块（57 个 `.svelte` 文件 / 约 10,718 行，实测）——迁移时**抽取为普通 CSS 文件**（§9.5 的关键工作项）。
- **组件的公开形态**：`Props` struct + `Component` 实现；**不引入新的运行时依赖**。

---

## 9. 视觉零改写的保证机制

### 9.1 class 命名约定

**约定：引擎产出的 class 名，与被替换实现产出的 class 名逐字相同。**

- 不引入命名空间前缀（不加 `ap-`、不加哈希、不加 Rust 模块路径）。理由：class 名是**视觉契约的一部分**——CSS 资产按这些名字写死；改名即改视觉。
- 既有 class 名的例子（实测）：`shell` / `rail-btn` / `drawer` / `wb` / `composer` / `ai-card` / `sug` / `stamp` / `home` / `row-menu`（见 `src/lib/design/motion.css`、`src/lib/design/shell.css` 与各 `.svelte`）。
- **Svelte 作用域 class 的例外**：参考实现编译后，元素上会多出 Svelte 注入的作用域 class（形如 `svelte-<hash>`）。**引擎不产生这类 class** ⇒ 对照时必须先剔除（§9.3 的 diff 前置处理）。

### 9.2 CSS 变量契约与「不生成内联样式」

| 规则 | 内容 | 违反后果 |
|---|---|---|
| **C1** | 引擎**不产出 CSS 文本**。样式全部来自 CSS 资产。 | 视觉分叉 |
| **C2** | 引擎**不读**令牌值来做决策（唯一可能的读是过渡时长探测，本设计未使用，见 §5.5.2）。 | 主题/调性分叉 |
| **C3** | **不写内联 `style`**，除非参考实现原本就有内联样式，且内容逐字一致。 | 主题覆写失效（内联样式优先级高于 `html[data-theme]` 覆写） |
| **C4** | 不设置 `id`（除非参考实现有）。 | 与宿主既有 `id` 冲突 |
| **C5** | 不改 `<html>` / `<body>` 的属性（`data-theme` / `data-accent` / class）。 | 主题切换被抢 |

**C3 的机器化落地**：golden 里记录每个元素的 `style` 属性（参考实现为空则引擎必须为空），逐元素比对。

### 9.3 像素对照的测法

**目的**：证明「引擎产出的 DOM 与参考实现视觉一致」。

**方法（确定）**：

1. **同浏览器同会话**：两个版本（参考实现构建产物 / 引擎 wasm 构建产物）在**同一个浏览器实例的同一个页面上下文**里先后加载，或并排两个 iframe。**跨浏览器比较一律无效**（字体栅格化不同）。
2. **固定环境**：同一 viewport（对照基线 **1440×900**，本文设定的测试口径，非实测结论）、`devicePixelRatio = 1`、同一 `html[data-theme]`（基线 `night`）、同一 `prefers-reduced-motion` 取值、同一滚动位置（置顶）、同一时刻（关掉所有与时间相关的显示，如相对时间戳用固定 fixture 替换）。
3. **固定数据**：两端喂**同一份 JSON fixture**（不连真实后端）⇒ 渲染的是同一批内容 ⇒ 差异只可能来自引擎。
4. **截图**：全视口 PNG。
5. **逐像素 diff**：对两张 PNG 逐像素比较，**任一通道的绝对差 > 2/255** 即记该像素为「不同」。指标 = 不同像素数 / 总像素数。
   - **阈值：≤ 1%**（[ADR-0015 §4](../adr/0015-pure-rust-ui-engine.md) P3 验收门「像素对照 ≤1%」）。
   - 阈值 2/255 的作用：吸收同一浏览器内的抗锯齿抖动；**不做模糊比较**（模糊会把真实差异也吃掉）。
6. **差异归因（必须做，否则阈值没有意义）**：差异像素必须能归到四类之一 ——
   - **D1 结构差异**：DOM 树形状不同 ⇒ 用 golden DOM diff 定位到具体节点；
   - **D2 属性差异**：class / 属性值不同 ⇒ 用 golden 属性 diff 定位；
   - **D3 文本差异**：文本内容不同（多半是数据 fixture 不一致或时区/时间格式化）；
   - **D4 相位差异**：动画相位不同（见 §9.4）。
   **D1/D2/D3 归因必须为零**；D4 由相位固定消除。剩下的像素差才是「合成层的真实差异」。
7. **前置处理**：比对前，从参考实现的 DOM 中**剔除 Svelte 作用域 class**（形如 `svelte-<hash>`）；两侧都按同一规则规范化（属性排序、空白折叠）。

**阈值不可放宽**：像素对照是 P3 的硬门（ADR §4），超标即判试点不过（P4 的 go/no-go 依据）。

**范围边界（不外推）**：本节的像素对照针对 **DOM 投影**（P3 试点面）。**GPU 场景画面的像素一致性属另一件事**，其当前状态见 §7.6 P0-c（宿主 WebView 画面**尚未实测**）⇒ **GPU 阶段的画面验收门在 P5，且以本节同款方法（+ §9.4 逐帧）执行**。P0 探测已证明的是「着色器文本与光学数值在 naga 往返后不变」，**不是**「画面不变」。

### 9.4 过渡逐帧对照的测法

**为什么需要**：动画责任在 CSS，但**动画的触发条件在 DOM**（class 增删的时机、节点插入的时机、节点是否被重建）。帧不对 ⇒ 电影感退化为软件感。

**方法（确定）**：

1. **相位固定**：两端都注入一份**仅测试用的**冻结样式表，把 CSS 动画/过渡钉在指定相位：

   ```css
   /* 仅测试注入，不进产品资产 */
   * { animation-play-state: paused !important; animation-delay: -PHASEms !important; }
   ```

   `PHASE` 取一组采样点。**禁止用 `prefers-reduced-motion` 做冻结手段**——那会把动画整个关掉，等于没测（`motion.css:7,161,193` 的动画正是在 `no-preference` 媒体查询内）。
2. **采样点**：对每个被测动画，取 `0 / 1/4 / 1/2 / 3/4 / 1` 五个相位（或动画时长的等距 5 点）。动画时长来自 CSS 资产（`motion.css` 里 `ap-card-rise` 0.5s、`ap-fade-up` 0.6s、`ap-msg-in` 0.42s、`ap-shell-in` 0.55s），**不由引擎提供**。
3. **逐相位像素对照**：每个相位独立跑 §9.3 的 diff，阈值同样是 **≤1%**。
4. **额外断言（结构性，比像素更硬）**：
   - **动画元素集合一致**：两端处于「有 CSS 动画」状态的元素集合必须相同（用 `getAnimations()` 取当前动画列表比对）；
   - **动画时长与延迟一致**：逐个动画比对 `effect.getTiming()` 的 duration / delay / easing；
   - **关键帧一致**：比对 `effect.getKeyframes()`。
   这三条**不依赖像素**，能定位「时序错了」还是「画错了」。
5. **错落入场专项**：`ap-card-rise` 有 0.04 / 0.10 / 0.16 / 0.22s 的逐项延迟（`motion.css:51-61`）⇒ 相位采样必须覆盖这四个延迟点，否则会漏掉「第 3 张卡片比其他晚出现」这类差异。

### 9.5 CSS 资产的复用与抽取（视觉零改写的工作项）

**两笔账，必须分开算**：

| 资产 | 规模（实测） | 处理方式 |
|---|---|---|
| **共享设计层** `src/lib/design/*.css`（+ `app.css` / `styles.css` / `bridge.css`） | 10 个文件 / **4,567 行** | **原样复用，零改动**。这些文件已经是普通 CSS（令牌变量 + 类选择器），不依赖组件框架 |
| **组件层**：既有 `.svelte` 内的 `<style>` 块 | **57** 个文件 / 约 **10,718 行** | **必须抽取**（见下） |

**为什么组件层必须抽取**：这些样式写在 `.svelte` 文件里，由框架在编译期**加作用域**（给选择器追加哈希后缀、给元素追加作用域 class）。Rust 引擎产出的 DOM **没有**这些作用域 class ⇒ 若原样保留 `.svelte` 里的 `<style>`，样式**全部失配**。

**抽取规则（确定）**：

1. 把每个 `<style>` 块的规则**逐条**搬进一个普通 CSS 文件；选择器里的作用域后缀**去掉**，元素上依赖作用域 class 的选择器改为**显式类名**（如 `.approval-card__stamp`）。
2. **视觉必须不变**：因此抽取后的选择器必须与编译后的实际选择器**等价**（同样的特异性和同样的匹配集合）。这是抽取工作的**验收标准**，靠 §9.3 的像素对照兜底。
3. `:global(...)` 规则（实测 54 处）**原样搬出**（它们本来就不加作用域）。
4. 抽取后的 CSS 进 `crates-ui/` 的资产目录（或试点期的静态资源目录），**由宿主原样加载**——**不由引擎注入**（C1）。

**这是一笔真实的迁移成本，必须登记**（不在本文的「零改写」承诺之外 —— 「零改写」指的是**视觉结果不变**，不是「一个 CSS 字都不用动」）。

### 9.6 保证机制总表

| 机制 | 覆盖的风险 | 门禁位置 |
|---|---|---|
| class 逐字一致（§9.1） | 样式失配 | golden DOM diff（native 单测） |
| 不生成 CSS / 不写内联样式（§9.2 C1–C5） | 主题覆写失效、视觉分叉 | golden 属性 diff（native 单测） |
| DOM golden 比对（§9.3 步骤 6 的 D1/D2/D3） | 结构与属性差异 | native 单测（§10.1 L2） |
| 像素对照 ≤1%（§9.3） | 合成层差异 | 浏览器内对照测试（L3） |
| 逐帧对照（§9.4） | 动画触发与相位 | 浏览器内对照测试（L3） |
| 六条过渡保护规则 T1–T6（§5.5.2） | 动画被提前掐断 / 重建 | native 单测（L2，可断言节点身份与写入序列） |

---

## 10. 测试策略

### 10.1 三层

| 层 | 范围 | 运行环境 | 覆盖什么 | 门禁 |
|---|---|---|---|---|
| **L1 纯逻辑单测** | `ui-core` | **native**（`cargo test`，无需浏览器、无需 wasm target） | 信号读写语义、依赖图、脏传播、惰性派生、环检测、effect 生命周期、作用域销毁顺序、批次合并、调度顺序 | `crates-ui` CI 的 `test` job |
| **L2 DOM 模拟测试** | `ui-dom`（差分 / 委托 / 绑定 / 挂载卸载）、`ui-kit`（组件渲染）、`ui-macro`（展开快照） | **native**（`InMemoryDom`） | 差分正确性（含 keyed LIS）、class token 差分、T1–T6 过渡保护规则、事件委托（冒泡 / `stopPropagation` / 修饰键）、双向绑定回写、组件树生命周期、golden DOM 比对 | 同上 |
| **L3 装配级浏览器内对照测试** | `ui` + `ui-kit` + `ui-demo`（wasm 产物） | **浏览器**（wasm 测试运行器） | 真实 DOM 行为（真实事件派发、焦点、输入法、真实 CSS 动画触发）、**像素对照 ≤1%**、**逐帧对照** | `crates-ui` CI 的浏览器 job |

**L1 的规模下限**：[ADR-0015 §4](../adr/0015-pure-rust-ui-engine.md) P1 验收门写明「**≥20 例真测试**（native 跑，不需浏览器）」。本文把它作为**下限而非目标**（下限是合同线，不是工作量承诺）。

**分层纪律**：**L3 只做 L1/L2 做不到的事**。任何能在 L1/L2 表达的断言，不得下放到 L3 —— 否则测试基座又回到「离不开浏览器」的老路。

### 10.2 明令禁止源码镜像断言

**病根**（实测）：`frontend/companion-desktop/tests/` 共 **84** 个套件，其中 **38** 个用 `readFileSync` 读源码文本做断言。它们钉的是**字面量与物理位置**，任何语义等价的重构都会打碎它们（[ADR-0015 §1.2](../adr/0015-pure-rust-ui-engine.md) 真问题 1）。

**禁令（对 `crates-ui/` 与后续迁移后的前端同时生效）**：

> **禁止「读取被测对象的源文件文本，再对文本做断言」。**

**判据（三条同时满足即违规）**：
1. 测试读取的是**源码文件**（`.rs` / `.svelte` / `.ts`）的**文本**；
2. 断言对象是**文本内容**（包含 / 匹配 / 正则）；
3. 断言意图是「某段逻辑存在」。

**允许的例外（仅此三类）**：
- **E1 宏展开快照**：比对**引擎自己生成**的代码文本（`ui-macro` 的展开结果），用于捕捉宏的意外变更。对象是**生成物**，不是**被测业务源码**。
- **E2 契约快照**：比对**数据契约文件**（如 `contract.toml` / `design-tokens.json`）与代码的一致性 —— 这类断言的对象是**契约**本身，属于「契约即真相」纪律，不是行为断言。
- **E3 静态许可 / 中性表述扫描**：扫描源码里是否出现第三方产品名 —— 这是**合规扫描**，不是行为断言。

**替代做法（唯一合法路径）**：

| 想断言的 | ❌ 禁止 | ✅ 合法 |
|---|---|---|
| 「某函数会发某个请求」 | 读源码找字符串 | 调用组件渲染出的处理器，断言**假后端收到的载荷** |
| 「某分支存在」 | 读源码找 `if` | 渲染组件，断言 **VNode 树里有/无该分支** |
| 「某元素绑定了事件」 | 读源码找 `on:click` | `InMemoryDom::dispatch` 派发事件，断言**副作用发生** |
| 「某元素有某个 class」 | 读源码找 `class="x"` | 断言渲染结果的 **class token 集合** |
| 「状态会驱动视图」 | 读源码找 `$state` | 写信号 → `flush` → 断言 **DOM 变化** |
| 「样式走的是令牌」 | 读源码找颜色字面量 | 断言渲染结果**不含内联颜色**，且 class 在 CSS 资产里**有对应规则** |

**门禁**：该禁令需要一条**自动检测**（扫描测试目录里的「读源文件 + 文本断言」组合）。**检测口径由本文定，实现归属 P1 施工线**（`crates-ui/` 独立 CI 的一个 job）；本文不指定脚本文件名，以免与施工线的命名冲突。

### 10.3 wasm 产物的测试怎么跑

| 问题 | 答案 |
|---|---|
| wasm 代码怎么测？ | 用 `wasm-bindgen-test` 的测试运行器：测试函数与普通 Rust 测试同构，只是 `#[wasm_bindgen_test]` 标注；运行器在浏览器里执行。 |
| 在哪跑？ | headless 浏览器（Chrome），由 `wasm-pack test --headless --chrome` 一类命令驱动。 |
| CI 怎么配？ | `crates-ui` 的独立 CI（`ui-engine-ci.yml`）增加一个**浏览器 job**：固定版本的浏览器 + 对应驱动镜像；该 job 是 `crates-ui` 的必需门禁。 |
| native 侧要跑多少？ | **绝大部分**：L1 + L2 全部在 native 跑。wasm 侧只跑 L3（真实 DOM 行为 + 像素/逐帧对照）。 |
| wasm 产物体积要不要卡？ | **不卡**（本期无体积承诺）。若将来要卡，先做实测再定阈值（0 装纪律：无实测不写数字）。 |

**为什么不用「在 wasm 里跑全部测试」**：那会让每次改动都必须过浏览器 ⇒ 测试慢、CI 重、且**把本可以在 native 精确断言的逻辑变成浏览器里的近似断言**。分层是刻意的。

### 10.4 门禁汇总（`crates-ui/` 独立 CI）

| job | 命令口径 | 归属 |
|---|---|---|
| `check` | `cargo check --workspace --all-targets` | P0 |
| `clippy` | `cargo clippy --workspace --all-targets -- -D warnings` | P0 |
| `fmt` | `cargo fmt --all -- --check` | P0 |
| `test`（native） | `cargo test --workspace --all-targets`（L1 + L2） | P0 |
| `test`（browser） | wasm 测试运行器 + headless 浏览器（L3） | P1 |
| `source-mirror-lint` | §10.2 的禁令检测 | P1 |
| `deny` | `cargo deny check`（许可白名单 + 多版本 skip 登记） | P0 |

**主 workspace 的门禁对引擎 0 影响**（独立 workspace，`cargo metadata` 不含它）——这是 ADR §3.2 选独立 workspace 的既定代价与既定补偿。

---

## 11. 诚实边界

### 11.1 已定的设计（本文给出确定语义，可直接施工）

| # | 已定项 | 位置 |
|---|---|---|
| 1 | 引擎边界：只做响应式调度 / DOM 绑定 / 组件挂载卸载时机；不写 CSS 引擎、不写滤镜层、不碰 shader、不碰排版、不碰主题 | §1.2 |
| 2 | 不做什么清单（12 条，含「引擎层不出现业务名词」） | §1.3 |
| 3 | 7 crate 拓扑、依赖方向与无环证明（拓扑秩论证） | §3.1 / §3.2 |
| 4 | 各 crate 的职责 / 公开类型 / 依赖 / 单一职责边界 / 可单测时机 | §3.3 |
| 5 | 信号读写语义（读登记依赖、写总是失效、`set_if_changed` 可选） | §4.1 |
| 6 | 调度：微任务为唯一调度级；DOM 写入在微任务批次内完成；引擎不跑自己的 rAF 循环（帧级合并 DOM 写入被拒，理由已登记） | §4.2 |
| 7 | 派生：惰性求值 + 主动失效；取此舍彼的理由与代价已登记 | §4.3 |
| 8 | 依赖图两类环检测（求值环 / 写-读自激）与可读报错 | §4.4 |
| 9 | effect 创建 / 重跑 / 销毁时机；作用域 LIFO 销毁；卸载与作用域销毁成对 | §4.5 / §5.1.4 |
| 10 | Svelte 5 runes 语义子集的映射表 + 两处诚实差异 | §4.6 |
| 11 | `Dom` trait 抽象 + `InMemoryDom`（含事件派发）为 native 可测前提 | §5.1.2 |
| 12 | 差分：class token 差分、keyed 用 LIS、unkeyed 按索引、文本就地改 | §5.2 |
| 13 | 事件委托：委托根 = 挂载容器；冒泡事件委托 / 不冒泡事件直挂；调用顺序与 `stopPropagation` 语义；**委托下的 `currentTarget` 语义解**（= 当前处理器注册在哪个元素上 ⇒ 仓内 19 处用法可原样迁移） | §5.3 |
| 14 | 事件对象 API 面按实测用量裁剪（`key` / `stop_propagation` / `prevent_default` / `current_target` / `target` / `target_value` / `client_x` / `client_y` / 修饰键 / `button`） | §5.3.4 |
| 15 | 双向绑定：可绑定类型与语义、Rust 侧表达、与差分的交互（值不等才写） | §5.4 |
| 16 | **不实现框架级过渡**；六条过渡保护规则 T1–T6；缓动与时长归属 CSS | §5.5 |
| 17 | `#[component]` 输入输出形状；props / children / 无具名插槽的取舍；组件树生命周期 | §6.1–§6.3 |
| 18 | **组件可被普通 `cargo test` 直接调用**；38 个镜像测试的逐类替代写法；native 测不出什么的诚实边界 | §6.4 |
| 19 | `view!` 宏文法与规则 | §6.5 |
| 20 | GPU 桥：wgpu 接入路径、GLSL 复用边界的三层切分、uniform 接口、GPU 与 DOM 平级、三档梯队 | §7 |
| 21 | 组件库：从 31 个既有组件反推的清单 + 令牌消费纪律 7 条 | §8 |
| 22 | 视觉零改写：class 逐字一致、C1–C5、像素对照法（阈值 ≤1%）、逐帧对照法、CSS 抽取规则 | §9 |
| 23 | 测试三层 + 源码镜像禁令的判据与三类例外 + 替代写法表 | §10 |
| 24 | 门禁 job 清单 | §10.4 |
| 25 | **GLSL ES 3.0 → naga IR 的两步机械变换**（X1 删 `#version` 行 / X2 注入 `layout(binding=N)`），约 30 行前置编译器，纯文本函数可单测 | §7.2.1 |
| 26 | **`glsl` feature 必须与 `wgsl` 同开**（上游缺陷，naga 30.0.1 未修） | §7.2.1 约束 C2 |
| 27 | **uniform 名 → binding 索引映射表**（来源 `blackhole.ts:337` 的 `UNIFORM_NAMES`）；naga 往返会抹掉 uniform 名 | §7.2.1 约束 C3 / §7.3 |
| 28 | **T2 梯队默认后端 = `webgl`**（WebGPU 作可选加速）；`web-sys` / `wasm-bindgen` 版本硬钉 1:1 | §7.1 |
| 29 | **WebGL 路线全程留在 GLSL ES、不经过 WGSL ⇒ 光学数值零改动风险**（naga 往返实测保住 5 个 helper 函数与输出变量） | §7.2.1 / §7.2.2 |

### 11.2 P0 探测项的定案状态（2026-10-24 实测回填）

**权威来源**：[ADR-0015 §2.6（P0 探测实测结论）与 §8.2（P0 探测证据链）](../adr/0015-pure-rust-ui-engine.md)。探测工程位于独立 scratch `target/ui-probe-scratch/`（gitignore 目录，不入库）。

#### 11.2.1 已定案（P0-a…P0-e 五项；P0-a 拆三条子结论，P0-c / P0-d 为**部分定案**）

| # | 项 | 实测结论 | 落到本文哪里 |
|---|---|---|---|
| **P0-a** | naga 能否吃下本仓的 GLSL ES 3.00 | ✅ **能，着色器本体不用重写**（「1,037 行」是 TS 文件总行数，绝大部分是 JS 胶水，非着色器 —— §7.2.2）。需 **2 步机械变换**：①删 `#version 300 es` 行 ②每个 uniform 注入 `layout(binding=N)`；约 **30 行前置编译器**。上游事实：naga 的 GLSL 前端**无 ES profile**（`naga-30.0.1/src/front/glsl/parser.rs:267` 只收 `#version 440\|450\|460` + `core`）；uniform 必须显式绑定（`front/glsl/variables.rs:618`） | §7.2.1（约束 C1、X1/X2） |
| **P0-a′** | naga 往返是否保住着色器 | ✅ **保住**。`naga` 解析 → 校验 → `naga::back::glsl::Writer` 吐出 **17,099 字节 `#version 300 es`**：16 个 `layout(std140) uniform` 块（binding 0..15）、`fragColor` 与 `layout(location = 0) out vec4` 保住、5 个 helper 函数（`hash21` / `vnoise` / `fbm` / `diskRadiance` / `starLayer`）全在；**吐出的 GLSL ES 3.00 能被 naga 自己重新解析并重新校验通过**；同一份代码编成 wasm32 在 node 里跑，**结果与 native 完全一致** | §7.2.1 / §7.2.2 |
| **P0-a″** | `glsl` feature 能否单独开 | ❌ **不能**。上游缺陷：`front/mod.rs:5` 把 `interpolator` gate 在 `spv-in`/`wgsl-in` 后，而 `front/glsl/variables.rs:467` 无条件调用 ⇒ **只开 `glsl-in` 编译不过**。**naga 30.0.1 未修** ⇒ **`glsl` 必须与 `wgsl` 同开** | §7.2.1 约束 C2 |
| **P0-a‴** | naga 的 uniform 名会不会被抹掉 | ✅ **会**。naga 的 IR 把每个 GLSL uniform 变成独立 binding（`layout(std140) uniform` 块，binding 0..15），**再吐出时 uniform 名被抹成 `_group_0_binding_N_fs`** ⇒ 引擎须自行维护「**uniform 名 → binding 索引**」映射表（`blackhole.ts:337` 的 `UNIFORM_NAMES` 正好可用） | §7.2.1 约束 C3 / §7.3 binding 索引表 |
| **P0-b** | wgpu 依赖树规模与多版本 | ✅ **极干净**：wgpu-webgl **108 条目 / 68 crate**，**唯一多版本包只有 `syn`**（2.0.119 来自 zerocopy-derive 链 / 3.0.9 来自 wasm-bindgen-macro-support 0.2.129）；`hashbrown` / `bitflags` / `itertools` / `miniz_oxide` / `ahash` / `crossbeam-*` 实测**未进树**；`jni-sys` 双版本仅出现于 native 目标产物，**wasm 侧 0 多版本** | §7.1 / §7.2.1 |
| **P0-c（部分）** | wgpu 在 wasm32 的可用性 | ✅ `webgl` 与 `webgpu` **两个后端均编译通过**（`wasm32-unknown-unknown`）。产物体积实测（release `.wasm`，含真实导出非空壳）：`webgl` **4,534,454 B** / `webgpu` **2,860,060 B** ⇒ **T2 梯队默认走 `webgl`，WebGPU 作可选加速** | §7.1 / §7.5 |
| **P0-d（部分）** | `syn` / `quote` 版本 | ✅ 多版本情况已查清（**只有 `syn` 一处**）。**proc-macro 三件套的具体版本待 P1 引入时定**（依赖白名单口径随 P1 一并落） | §3.3.3 / §6.6 |
| **P0-e** | `wasm-bindgen` / `web-sys` 版本配对 | ✅ **精确硬钉 1:1**：`wasm-bindgen 0.2.129 ⟷ js-sys 0.3.106 ⟷ web-sys 0.3.106`；**错配在依赖解析期就失败**（0.2.129 + 0.3.104 实测失败）。另：`wasm-bindgen-cli` 0.2.129 实测 44.68 s 装上，wasm→js 转换 + node 加载实测通过 | §5.1.2 `WebDom` 实现的版本锁定 |

#### 11.2.2 仍然是待测项（**不得写成已定**）

| # | 项 | 状态 | 影响面 | 何时测 |
|---|---|---|---|---|
| **P0-c（余项）** | **宿主 WebView 的画面一致性**（WebGPU 是否真在 WebView 里可用、T2 画面是否与既有 WebGL2 版一致） | ❌ **未实测** —— P0 探测环境**无浏览器**；已测的只是**编译期与解析期** | §7.1 / §7.5 / §9 | P5 GPU 阶段 + §9.4 逐帧对照 |
| **P0-f** | **Canvas2D 降级路径（`blackhole.ts` 的 2D 分支）移植到 `ui-gpu` 的成本** | ❌ **未实测**（需读该 2D 分支，与 `ui-gpu` 设计强相关） | §7.5 的 T1 梯队 | P5 GPU 阶段开工前；**未测前 T1 不得开工** |
| **P0-g** | **浏览器 job 的运行环境**（固定版本浏览器 + 驱动的可用性与可复现性） | ❌ **未实测**（本机无浏览器） | §10.3 / §10.4 的浏览器 job | P1 搭 CI 时固化镜像 |

> **诚实边界（不得过度声称）**：§7.1–§7.2 的全部实测结论都是**编译期与解析期**的（naga 解析 / 校验 / 吐出 / 重新解析 / wasm32-node 跑通）。**「画面一致」这件事在真实浏览器 WebView 中一次都没有跑过** —— 凡涉及画面的断言，一律以 §9.3 / §9.4 的浏览器内对照测试为准。

> **纪律继续有效**：结论回来前尚未定案的位置，在代码里必须是**显式的未接线状态**（`unimplemented!()` / `Err(NotImplemented)` + 显式断言），不得以「先按能跑写」的方式落地。**P0-c（余项）/ P0-f / P0-g 三项适用本条。**

### 11.3 提案（本文主张，方向已定、细则待确认）

| # | 提案 | 需要谁确认 | 不确认的后果 |
|---|---|---|---|
| **R1** | `view!` 宏的具体文法（§6.5） | 施工线（实现可行性）+ owner（可读性） | 宏文法是实现自由度最大的一块；若大幅调整，只影响 §6.5 一节 |
| **R2** | `#[component]` 用「构造器 + 链式 setter」而非「`Props` struct 直接构造」 | 施工线 | 影响 §6.1 的 API 形状；两种都可实现 |
| **R3** | `InMemoryDom` 的具体 API（节点句柄表示、事件派发签名） | 施工线 | 影响 §5.1.2；语义边界已定，实现可变 |
| **R4** | 像素对照的**差异通道容差 2/255** 与**相位采样点取 5 个等距点** | owner（视觉验收口径） | 阈值偏松会放过真实差异，偏严会因抗锯齿误报 |
| **R5** | 组件层 CSS 抽取后的**显式类名命名约定**（如 `组件名__元素名`） | 施工线 + owner（可读性） | 命名不一致 ⇒ 抽取后的 CSS 难维护 |
| **R6** | 源码镜像禁令的**自动检测实现形态** | 施工线（P1） | 纪律靠人守会退化成靠自觉 |
| **R7** | `ui-kit` 的模块分组（按既有组件归并为 9 组） | 施工线 | 分组可调，组件清单本身（§8.2）已定 |

### 11.4 口径更正记录（本文采用的口径，及更正理由）

| 项 | 旧口径（已作废） | 本文采用的口径 | 依据 |
|---|---|---|---|
| `on:` 事件指令 | 814 处 | **Svelte 5 事件属性 `on*=` 共 403 处**（`onclick` 304 / `onkeydown` 41 / `oninput` 24 / `onchange` 13 / `onfocusout` 9 / 其余 12）；Svelte 4 形态的 `on:` 精确匹配 **2 处** | [ADR-0015 §2.2 口径修正](../adr/0015-pure-rust-ui-engine.md)（lead 亲验） |
| `in:` / `out:` 过渡指令 | 277 处 | **0 处** | 同上；实测 `in:`/`out:` 精确匹配 0 |
| `transition:` | 86 处框架指令 | **86 处全部是 CSS `transition:` 属性声明**；框架过渡指令 **0 处** | 同上；实测 `src/App.svelte:3295` 等 |
| 框架级过渡时序引擎 | 计划中（作为 §5 最关键） | **不实现**；改为「组件挂载/卸载时机 ⇒ CSS 过渡是否被正确触发」的六条保护规则 | 同上；[ADR-0015 §2.3](../adr/0015-pure-rust-ui-engine.md) |
| `backdrop-filter` | 15+ 处 | **80 处**（实测 `frontend/companion-desktop/src/` 全树） | 实测 |
| 工作量估算 | 9,000–18,000 行 | **7,000–14,000 行**（ADR 已更新） | [ADR-0015 §3.1](../adr/0015-pure-rust-ui-engine.md) |

**其余沿用未变的口径**：`{#if}` 377 / `$state` 376 / `$derived` 162 / `{#each}` 111 / `$props` 55 / `$effect` 40 / `bind:` 70（本文对 `{#each}` 进一步细分为 keyed 82 / unkeyed 29，对 `bind:` 进一步细分为 `value` 45 / `this` 24 / `checked` 1）。

**2026-10-24 P0 探测回填导致的口径变更**（本文 §7 / §11.2 已同步）：

| 项 | 回填前口径 | 回填后口径 |
|---|---|---|
| GLSL 源文本能否原样复用 | 「待 P0 探测确认」（三种分支） | **能，但需两步机械变换**（删 `#version` 行 + 注入 `layout(binding=N)`） |
| `glsl` feature 能否单独开 | 未涉及 | **不能**，必须与 `wgsl` 同开（上游缺陷） |
| uniform 名在 naga 往返后是否保留 | 未涉及 | **不保留**（被抹成 `_group_0_binding_N_fs`）⇒ 引擎须自维护映射表 |
| wgpu 依赖树多版本风险 | 「极可能引入多版本，直接撞红」（ADR §3.2 拒理由 A 的推测） | **实测极干净**：唯一双版本包是 `syn`；`hashbrown` / `bitflags` / `itertools` / `miniz_oxide` / `ahash` / `crossbeam-*` 未进树；wasm 侧 0 多版本 |
| T2 梯队后端 | 「WebGPU 可用性待测」 | **默认 `webgl`**，WebGPU 作可选加速 |

### 11.5 复算清单（本文所有统计数字的复跑命令）

工作目录 = 仓库根。命令均为只读。

```bash
# §0 / §2.5 主题与配色方案
grep -n 'VALID_THEMES'  frontend/companion-desktop/src/lib/theme.ts
grep -n 'VALID_ACCENTS' frontend/companion-desktop/src/lib/theme.ts

# §2.2 / §5.2 令牌与模糊
grep -rohE '\-\-ap-[a-z0-9-]+' frontend/companion-desktop/src/lib/design/ | sort -u | wc -l   # 115
grep -roh 'backdrop-filter' frontend/companion-desktop/src/ | wc -l                             # 80

# §4.6 / §6.4 特性使用面
grep -rohE '\$state'   frontend/companion-desktop/src/ | wc -l   # 376
grep -rohE '\$derived' frontend/companion-desktop/src/ | wc -l   # 162
grep -rohE '\$props'   frontend/companion-desktop/src/ | wc -l   # 55
grep -rohE '\$effect'  frontend/companion-desktop/src/ | wc -l   # 40
grep -rohE '\{#if'     frontend/companion-desktop/src/ | wc -l   # 377
grep -rohE '\{#each'   frontend/companion-desktop/src/ | wc -l   # 111
grep -rohE 'bind:[a-zA-Z]+' frontend/companion-desktop/src/ | sort | uniq -c                   # value 45 / this 24 / checked 1
grep -rohE '\bon[a-z]+\s*=' --include=*.svelte frontend/companion-desktop/src/ | sort | uniq -c   # onclick 304 …

# §5.2.3 {#each} 的 keyed / unkeyed 细分（111 = 82 keyed + 29 unkeyed）
grep -rohE '\{#each[^}]*\}' --include=*.svelte frontend/companion-desktop/src/ \
  | grep -cE '\bas\s+[A-Za-z_$][A-Za-z0-9_$]*\s*\('                    # 82（带 key）
grep -rohE '\{#each[^}]*\}' --include=*.svelte frontend/companion-desktop/src/ \
  | grep -vcE '\bas\s+[A-Za-z_$][A-Za-z0-9_$]*\s*\('                   # 29（不带 key）

# §5.3.4 事件属性用法频次
grep -rohE '\b(e|ev|evt|event)\.[a-zA-Z]+' --include=*.svelte frontend/companion-desktop/src/ \
  | sed -E 's/^[^.]*\.//' | sort | uniq -c | sort -rn                   # key 50 / stopPropagation 27 / preventDefault 22 / currentTarget 19 …

# §5.5 过渡与动画
grep -rnE 'transition:' --include=*.svelte frontend/companion-desktop/src/ | wc -l   # 86（全部为 CSS 属性）

# §7.2 GLSL 事实
grep -nE 'const VERT|const FRAG|#version 300 es|precision highp|uniform |void main' \
  frontend/companion-desktop/src/lib/scene/blackhole.ts                # 129-131 / 138-139 / 140-156 / 256

# §8 / §9.5 组件与 CSS 资产
ls frontend/companion-desktop/src/lib/components/ | wc -l            # 26
ls frontend/companion-desktop/src/components/   | wc -l            # 5
cat frontend/companion-desktop/src/lib/design/*.css frontend/companion-desktop/src/{app,styles,bridge}.css | wc -l   # 4567
grep -rl '<style' --include=*.svelte frontend/companion-desktop/src/ | wc -l               # 57

# §10.2 测试病根
ls frontend/companion-desktop/tests/*.mjs | wc -l                   # 84
grep -rlE 'readFileSync' frontend/companion-desktop/tests/ | wc -l   # 38

# §11.2 P0 探测结论的复验路径（探测工程在 gitignore 目录，不入库）
bash target/ui-probe-scratch/run-all.sh     # ADR-0015 §8.2「一键复验」
```

> 带 `#` 注释的是**本文记录时的实测值**（2026-10-24）。复跑结果若与注释不符 ⇒ 以复跑为准，并把本文的对应数字回写（0 装纪律：数字必须可复核）。

### 11.6 与计划书的分工

本文**只定架构**，不定工序。以下归 `docs/04-internal/ui-engine-plan.md`（另一条施工线）：

- 阶段划分与里程碑（P0–P5 的具体排期、人力、批次切分）；
- 各阶段的文件级施工清单与 write_scope；
- `crates-ui/` 骨架的目录与 `Cargo.toml` 细节；
- CI 文件的具体内容（本文只定 job 清单与口径）；
- §11.3 的 7 项提案的评审与落地排期；
- 源镜像禁令检测器的实现归属与命名。

---

*本文由前端重构项目组（总工：lead）指导撰写于 2026-10-24，配套 [ADR-0015](../adr/0015-pure-rust-ui-engine.md)（决策真源）与 `docs/04-internal/ui-engine-plan.md`（施工计划）。本文所有统计数字附复跑命令（§11.5），所有待定事实显式标注（§11.2），所有设计主张给出确定语义（§11.1）。*
