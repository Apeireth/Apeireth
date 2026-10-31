# 引擎产品工程：路径与工程基线

> **本文件是交接包 §0 要求的「引擎侧指定」**（原文：「具体仓库路径与工程基线由引擎线在切换前指定；本文件不预设路径」）。
> 给谁看：接手上屏第一屏的施工方（人或 AI）——**不读任何对话记录也能开工**。
> 不是什么：不是设计稿（视觉在 [前端视觉总攻计划书](05-plans/frontend-rebuild-plan.md)）、
> 不是排期承诺、不是「已开工」声明。

---

## §1 一句话

**产品工程 = `crates-ui/apeireth-ui-app`**（本批新建）；引擎层在 `crates-ui/apeireth-ui-{core,dom,gpu,kit,macro,time,scene}`；
组件层（L2）在 `crates-ui/apeireth-ui-kit`；**旧前端 `frontend/companion-desktop` 只是工程背景，不是实现目标**。

---

## §2 路径与拓扑（谁能写什么）

```
crates-ui/                        ← 引擎独立 workspace（0 外部依赖；不进主仓 members）
├── apeireth-ui-app      ⭐ 产品工程（秩 6）屏幕装配 + 宿主运行时契约
├── apeireth-ui          门面（秩 4）具名导出 core/dom/gpu/kit
├── apeireth-ui-core     响应式核心（秩 1）Signal/Derived/Scheduler/Graph/Effect
├── apeireth-ui-dom      DOM 层（秩 2）VNode/Renderer/事件/焦点/IME/测量/虚拟列表/资源
├── apeireth-ui-gpu      GPU 桥（秩 3）wgpu 渲染 + 纹理上传
├── apeireth-ui-kit      组件层（秩 5，L2）表单控件族/Markdown/图标/富文本
├── apeireth-ui-macro    过程宏（秩 3）
├── apeireth-ui-time     宿主侧时刻文本（秩 5）
├── apeireth-ui-scene    场景层语义（秩 5）
├── apeireth-ui-demo     ⛔ **试点载体**（审批卡/设置子面/工具格）兼装配级冒烟
└── evidence/p5-b        证据装置（判据 + 浏览器探针 + 本地 CI 等价）
```

| 你要写的东西 | 放哪 | 为什么 |
|---|---|---|
| **具体界面（屏幕）** | `apeireth-ui-app/src/screens/` | 产品工程的职责 |
| **组件（按钮/输入框/卡片…）** | `apeireth-ui-kit/src/` | L2 组件层，**不进引擎验收口径** |
| **引擎能力（测量/事件/渲染）** | `apeireth-ui-dom` / `-gpu` / `-core` | L1；改动要带 native 测试 + 浏览器证据 |
| **判据 / 对照装置** | `crates-ui/evidence/p5-b/` | 引擎判据的归位 |

⚠️ **依赖方向单向**：`app → ui → {core, dom, gpu, kit}` ⇒ ⇒ 引擎层**不得**反向依赖 `app`
⇒ ⇒ 否则产品细节会污染引擎（这正是 `ui-demo` 与 `ui-app` 分开的理由之一）。

---

## §3 工程基线（每批必跑）

```bash
cd crates-ui
cargo test --workspace --locked            # 基线（2026-10-31：1125 passed / 0 failed）
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
cargo check --workspace --target wasm32-unknown-unknown --locked

cd evidence/p5-b && python3 drive_l1.py    # 浏览器能力探针（EXIT 0 = 全过；dom/upload/binding/integration 四段）
python3 ../../../scripts/run-ci-local.py --workflow ui-engine-ci.yml   # 8 job 本地等价
```

**硬边界**（来自仓库宪章）：

| 边界 | 口径 |
|---|---|
| LOCKED 5 项 | **0 触碰**（9 哲学锚本体 / 键集 / 3 不可变脊柱 / `workspace.version` / R11 baseline） |
| 外部依赖 | **crates-ui 维持 0 外部依赖**（`ui-app` 依赖的 `ui-core/ui-dom/ui` 都是**内部 crate**） |
| 版本轴 | crates-ui **独立声明**但与主仓口径对齐（`2.0.0-rc.4`）；**主仓 `workspace.package.version` 是 LOCKED** |
| 判据依赖 | 判据代码**不得**依赖 CI 未声明的依赖（例：CI 只装 pillow ⇒ 判据不许用 numpy） |
| 提交 | conventional commits；**1 commit = 1 task**；正文必带「0 装诚实真账 + O-6 三阶审查」 |
| 口径 | 四级：IMPLEMENTED / PRODUCTION WIRED / DEFAULT ENABLED / HARDWARE VALIDATED；**能力缺口与缺陷分账** |

---

## §4 垂直切片：7 环现状（交接包 §0 的切换前提）

| # | 环 | 状态 | 证据 |
|---|---|---|---|
| 1 | **Signal**（响应式状态） | ✅ core 早已完备（`Signal`/`Derived`/`Scheduler`/`Graph`/`Effect`） | core 内 100+ 条 native 测试 |
| 2 | **Signal ⇄ DOM**（双向绑定） | ✅ **闭环**（下行 signal→DOM、上行 DOM→signal） | E-124；`l1_binding_probe` 浏览器 `ok=true` |
| 3 | **VNode / RenderPlan** | ✅ VNode + 差分计划（keyed diff / 块锚点） | E-109 / E-110 |
| 4 | **DOM mount / update / unmount** | ✅ | 五面逐像素 0 差异（真实 GPU） |
| 5 | **event**（19 类 + 载荷） | ✅ 含 `target_value`/`checked`/坐标/组合数据 | E-092 / E-098 |
| 6 | **focus / IME** | ✅ 焦点栈 + 陷阱 + 组合静默期（浏览器已验证） | E-093 / E-095 / E-098 |
| 7 | **dispose** | ✅ 幂等 + 清全部内部状态 + 可复用 | E-122（native） |
| — | **wasm 验证** | ✅ `wasm32` check 0 错误 + 浏览器探针全过 | E-116 / E-124 |
| — | **产品宿主薄壳** | ✅ `boot`/`rerender`/`dispose` 三条**浏览器已验证**（`l1_app_probe`） | E-127 |
| — | **WebView2 验证** | ❌ **未做** | 无 Windows 装置（= 放行条件 **C2**，owner 决定先泡着） |

⇒ ⇒ **除 WebView2 外，垂直切片已具备并有证据**；
⚠️ **切到 PRODUCTION_IMPLEMENTATION 的前提仍差 C2**（真实 Windows WebView2 跑通）。

---

## §5 三层边界（写代码时对照）

| 层 | 内容 | 归位 | 是否进引擎验收口径 |
|---|---|---|---|
| **L1** | 调度 / DOM 绑定与事件面与载荷 / 布局测量 / 滚动与虚拟化原语 / 焦点·IME·a11y / 缩放密度标量 / 资源句柄 / 错误边界 / 主题 class 通道 | `ui-core` / `ui-dom` / `ui-gpu` | ✅ 是 |
| **L2** | 表单控件族 / Markdown / 代码高亮 / 图标 / 富文本编辑 / 弹层与虚拟列表的**组件**版本 | `ui-kit` | ❌ 否（组件层自测） |
| **L3** | 令牌数据 / 网络缓存打包 / 3D / 排版曲线字体整形 | — | ❌ 明确排除 |

⚠️ **别把 L2/L3 塞进 `ui-app` 的引擎依赖里**：`ui-app` 只做**装配**；
组件去 `ui-kit` 拿，缺组件就**在 `ui-kit` 加**（L2 自测口径，不占引擎验收账）。

---

## §6 屏幕怎么写（契约已在 `ui-app` 里）

```rust
use apeireth_ui_app::{HostState, Screen};
use apeireth_ui_dom::vnode::VNode;

pub struct SettingsScreen {
    pub state: apeireth_ui_core::Signal<HostState>,
}

impl Screen for SettingsScreen {
    fn id(&self) -> &str { "settings" }
    fn view(&self) -> VNode {
        // 每次调用返回**当前状态的投影**（状态在信号里 ⇒ 读信号即可）
        VNode::element("section").build()
    }
}
```

- [`Screen`](../../crates-ui/apeireth-ui-app/src/lib.rs)：屏幕契约（`id` + `view`）；
- [`HostState`](../../crates-ui/apeireth-ui-app/src/lib.rs)：宿主状态（当前屏幕 + 脏标记）；
  **一次变更只触发一次重渲染**（帧边界收口 ⇒ 5 次标脏 ⇒ 1 次重渲染，native 测试钉住）；
- `wire_signal_to_host(runtime, host, source)`：把「状态信号 ⇒ 标脏 ⇒ 重渲染」接上
  （core 的 effect 是**依赖追踪式**的 ⇒ 闭包里读哪个 signal 就订阅哪个）。

---

## §7 需要 owner 拍板的两件（其余我自行决策）

| # | 事项 | 我的建议 |
|---|---|---|
| 1 | **产品工程路径** = `crates-ui/apeireth-ui-app`（本批已按此落地骨架） | 建议**通过**（与 `ui-demo` 分开，避免污染证据链） |
| 2 | **WebView2 验证**（= C2）何时做 | 建议在**第一屏实现前**做（否则「引擎可用」的结论只覆盖 Chromium） |

---

## §8 施工方该读的四份（按序）

1. 本文件（路径与基线）；
2. [引擎侧上屏接入说明](ui-engine-w0-onboarding.md)（**API 怎么用 + 五个坑 + 自证清单**）；
3. [L1 完成度体检表](ui-engine-l1-readiness.md)（能力到哪一级 + 什么还不能依赖）；
4. [前端视觉总攻计划书](05-plans/frontend-rebuild-plan.md)（做什么、做成什么样）。
