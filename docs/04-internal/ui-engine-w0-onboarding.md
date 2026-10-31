# 引擎侧上屏接入说明（W0）

> 给谁看：接手上屏第一屏的施工方（人或 AI）——**不读任何对话记录也能开工**。
> 不是什么：不是设计稿（视觉在 [前端视觉总攻计划书](05-plans/frontend-rebuild-plan.md) 的 W0 产出）、
> 不是排期承诺、不是「已开工」声明。
> 口径真源：能力边界 = [ADR-0015](../adr/0015-pure-rust-ui-engine.md) 增补；
> 能力状态 = [L1 完成度体检表](ui-engine-l1-readiness.md)；缺陷账 = [引擎缺陷台账](ui-engine-defect-ledger.md)。

---

## §0 三分钟速览

**第一屏要用到的引擎能力，已经全部落地且有浏览器证据。** 你需要做的只有三件事：

1. 用 `Renderer` 把一棵 `VNode` 树挂到真实 DOM（**受控**：值由你给，不由 DOM 自己跑）；
2. 用户交互时**改状态 → 重新 `patch`**，不要直接改 DOM；
3. 长列表用 `patch_windowed_at_path`（**不要**手写留白，见 §4 坑 1）。

**示范实现（都是能跑的）**：
- [`crates-ui/evidence/p5-b/harness/src/lib.rs`](../../crates-ui/evidence/p5-b/harness/src/lib.rs)
  里的 `l1_integration_probe()` —— 已在**真实浏览器**跑通以下组合（受控输入 + IME 静默期 +
  焦点陷阱 + 虚拟列表窗口化，同一棵渲染树上）：`crates-ui/evidence/p5-b/drive_l1.py` **EXIT 0**；
- 同一文件里的 **`w0_first_screen()` + `w0_tick()`** —— **第一屏最小示例**
  （受控输入 + 虚拟列表 + 滚动联动 + 错误边界），网页入口
  `crates-ui/evidence/p5-b/web/w0/index.html`（`vite build` 后用任意静态服务器打开即可）。
  实测（真浏览器）：挂载 11 个元素、列表窗口 7 行、**后留白 39720px**（容器真的能滚）、
  改输入框 ⇒ 引擎**受控回写** `hello`。

---

## §1 第一屏最小可用集（按「必须 / 建议 / 可后置」）

| 能力 | 何时必须 | API | 口径 |
|---|---|---|---|
| 挂载 / 差分写回 | **必须** | `mount` / `patch` / `patch_children_at_path` | WIRED + 五面逐像素验证 |
| 受控输入（值/勾选） | **必须**（有表单） | VNode 的 `prop("value", …)` + `handler(EventKind::Input, …)` | WIRED + 浏览器验证 |
| 焦点管理 | **必须**（有弹层/模态） | `focus_at_path` / `open_focus_trap` / `close_focus_trap` / `blur_focused` | WIRED + 浏览器验证（`activeElement`） |
| 错误边界 | **必须**（任何一屏都可能抛） | `render_guarded` + `captured_error` | WIRED + 浏览器验证（兜底进 DOM） |
| 度量未定门控 | **必须**（有字体/图片） | `set_metrics_status(MetricsStatus::Ready)` | WIRED |
| 虚拟列表 | 建议（长列表） | `patch_windowed_at_path` / `virtual_window_for` | WIRED + 1k 行规模不变量 |
| 滚动联动 | 建议 | `listen_scroll_at_path` → `ScrollSignal::take()` | WIRED |
| 布局跳动观测 | 建议（要「不跳」的质量线） | `observe_layout_shift_at_path` + `LayoutShiftTracker` | WIRED |
| **销毁一屏** | **必须**（换屏/退出） | `dispose()`（= 卸载 DOM + 清焦点/陷阱/IME 队列/捕获错误，幂等且可复用） | IMPLEMENTED + native 测试 |
| 缩放/密度 | 自动（跟随宿主） | `set_measurement_scale`（DPR × density） | WIRED + 浏览器验证 |
| 纹理上传 | 按需（图片） | `ImageData` + `TextureUploader` + `upload_decoded` | 接口就绪，GPU 侧实现待接 |

**明确不在 L1（不要指望引擎给你）**：表单控件族（select/checkbox/switch/日期…）、
Markdown、代码高亮、图标、富文本编辑、弹层与虚拟列表的**组件**版本
⇒ ⇒ 这些属 **L2**，归 `apeireth-ui-kit`（组件层），**不进引擎验收口径**。
令牌数据/网络缓存/打包/3D/排版曲线字体整形 = **L3**，明确排除。

---

## §2 骨架（可直接抄；每段都标注了已验证来源）

### 2.1 挂一棵受控表单树

```rust
use apeireth_ui_dom::dom::EventKind;
use apeireth_ui_dom::events::{DomEvent, ElementHandler};
use apeireth_ui_dom::vnode::{PropValue, VNode};
use apeireth_ui_dom::web::WebDom;
use apeireth_ui_dom::Renderer;

// ① 建 DOM 适配层 + 挂载根节点
let dom = WebDom::new()?;                       // 接真实 DOM（浏览器）
let root = dom.create_element("div")?;
dom.set_attribute(&root, "id", "app")?;
document.body().append_child(&root);
let mut renderer = Renderer::new(dom, root);

// ② 建树（**值由我们给** ⇒ 受控）
let view = VNode::element("form")
    .child(
        VNode::element("input")
            .prop("value", PropValue::Text(state.name.clone()))
            .handler(EventKind::Input, ElementHandler::new(move |event| {
                // ⚠ 处理器里**不要**碰 renderer（拿不到 &mut）⇒ 只改状态
                state.borrow_mut().name = event.target_value().unwrap_or_default().to_owned();
            }))
            .build(),
    )
    .build();

// ③ 渲染（**用 render_guarded**：失败要兜底，不是崩）
//    ⚠️ 未挂载时它会走**首次挂载**（2026-10-30 改：旧契约下第一屏必须先 `mount`，
//    ⇒ ⇒ 首次挂载就没有错误边界 ⇒ 一屏抛错就是白屏）
renderer.render_guarded(view)?;                 // 已验证：浏览器强制失败 ⇒ 兜底进 DOM
```
> 已验证来源：`l1_probe()`（挂载 + 受控输入写回）与 `l1_integration_probe()`（受控输入 + IME + 焦点 + 列表）。

### 2.2 状态变了怎么重渲染

```rust
// 状态变 → 重建树 → patch（**差分**，不是整棵重建）
let view = build_view(&state.borrow());
renderer.patch(view)?;
```
> ⚠ **`patch` 的输入必须是「同一形状的树」**：路径（`path`）是按**子节点下标**寻址的
> ⇒ ⇒ 条件渲染时注意别让下标漂移（引擎按块锚点做了补偿，但**列表要带 key**）。

### 2.3 长列表（**留白交给引擎**）

```rust
let plan = renderer.patch_windowed_at_path(
    &[1],                 // 滚动容器的路径
    &item_heights,        // 每项高度（行高固定时传同值）
    2,                    // overscan（上下各多渲染几行）
    |index| VNode::element("div")
        .attr("class", "row")
        .child(VNode::text(format!("{index}")))
        .build(),
)?;
// 滚动联动：scroll 不冒泡 ⇒ 直连容器；信号只答「刚滚过没」，**位置自己读**
let signal = renderer.listen_scroll_at_path(&[1])?;
if signal.take() {
    let metrics = renderer.scroll_metrics_at_path(&[1])?;   // 真实 DOM 读位置
    // → 用 metrics 重新算窗口并 patch
}
```
> 已验证来源：`l1_integration_probe()`（同款 API 组合，浏览器 EXIT 0）。

### 2.4 焦点陷阱（弹层/模态）

```rust
renderer.open_focus_trap(&[&[0], &[0, 1]]);   // 允许聚焦的路径白名单
renderer.focus_at_path(&[0, 1])?;             // 聚焦
// …关闭时
renderer.close_focus_trap();
```
> 已验证来源：`l1_probe()`（`document.activeElement` 被引擎驱动）。

### 2.5 字体/图片未就绪 ⇒ 别急着算布局

```rust
renderer.set_metrics_status(MetricsStatus::Unknown);   // 字体未就绪 ⇒ 显式声明「度量未定」
// 字体 ready 之后：
renderer.set_metrics_status(MetricsStatus::Ready);
```
> ⚠ 度量未定时 `virtual_window_for` 会**显式拒绝**（返回 `Undecidable`）
> ⇒ ⇒ 这是**故意的**：宁可不窗口化，也别用错误度量算出的窗口（会造成布局跳动）。

### 2.6 「这一屏没跳」怎么自证

```rust
let mut tracker = LayoutShiftTracker::new(viewport_w, viewport_h);
let score = renderer.observe_layout_shift_at_path(&mut tracker, id, &[0])?;
println!("本次位移得分 {score}，累计 {}", tracker.total());
```
> ⚠ 亚像素位移（<1px）与视口外位移**不计分**（默认阈值 1px，可配）
> ⇒ ⇒ 别用它抓渲染噪声。

---

## §3 事件面（19 类）

`click / dblclick / input / change / keydown / keyup / focus / focusin / blur /
submit / pointerdown / pointermove / pointerup / pointercancel / wheel / scroll /
contextmenu / compositionstart / compositionend`
（**准确名单以 `EventKind::all()` 为准** —— 代码里是唯一真源。）

载荷访问器：`target_value()` / `checked()` / `client_position()` / `button()` / `pointer_id()` / `alt_key()` /
`composition_data` 等，缺失一律 `None`。

⭐ **两条必须知道的规则**：
1. **`scroll` 不冒泡** ⇒ 只能直连滚动容器（`listen_scroll_at_path` 已经这么做了）；
2. **IME 组合期间**引擎会**门住**受控值的写回（防打断输入法），组合结束后补写
   ⇒ ⇒ 你的状态机**不要**在 `compositionstart..end` 之间反复改 `value`。

---

## §4 五个坑（都是本项目实测踩出来的）

| # | 坑 | 症状 | 正确做法 |
|---|---|---|---|
| 1 | 手写窗口行、**不留白** | 列表「不滚动」（`scrollTop` 被浏览器夹回 0） | 用 `patch_windowed_at_path`（留白由引擎算） |
| 2 | 在 composition 期间改 `value` | 输入法被打断 / 字符丢失 | 组合期间只改状态不改值；引擎会补写 |
| 3 | 字体没加载就窗口化 | 窗口位置错、加载完跳一下 | 字体未就绪时**显式** `set_metrics_status(Unknown)`，ready 后 `Ready`（⚠️ 渲染器**默认是 `Ready`** —— DOM 度量是真实几何；「度量未定」只有宿主知道 ⇒ 必须由宿主显式声明） |
| 4 | 以为 `scroll` 会冒泡 | 委托根上收不到滚动 | 直连容器 |
| 5 | `path` 下标语义搞错 | 读不到度量（`NotImplemented`） | `node_at_path(&[0])` = **树根的第 0 个子节点**；度量必须落在**同一个**节点上 |

---

## §5 自证清单（这一屏做完了吗）

```bash
cd crates-ui
cargo test --workspace --locked            # 基线（2026-10-30：1121 passed / 0 failed）
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check

cd evidence/p5-b && python3 drive_l1.py    # L1 浏览器探针（EXIT 0 = 全过）
```
页面级自证（你写的那一屏）：
- [ ] 受控输入：改状态 ⇒ DOM 值跟着变；DOM 手动改 ⇒ 下一帧被引擎改回（受控）
- [ ] IME：中文输入法组合中不丢字、不打断
- [ ] 焦点：模态打开 ⇒ Tab 走不出；关闭 ⇒ 焦点还原
- [ ] 长列表：滚到底/顶 ⇒ 窗口位置对、内容不空白、总高度恒定
- [ ] 错误边界：故意让一屏抛错 ⇒ 看到兜底 UI，不是白屏
- [ ] 布局跳动：CLS 累计在预算内（亚像素噪声不计）

---

## §6 你可以依赖什么、不能依赖什么

**可以依赖**（有浏览器/真实 GPU 证据）：受控输入、事件面与载荷、焦点与陷阱、IME 静默期、
测量与命中（含 DPR 乘数）、虚拟列表、错误边界、像素等价（五面 + intro 逐像素 0 差异）。

**不能依赖**（尚未验证）：
- 真实 Tauri 宿主（macOS WKWebView / Linux WebKitGTK / Windows WebView2）上的行为
  ⇒ ⇒ **引擎会自动降级到 WebGL 后端**（代码里已实现），但降级后的画面**尚未在真机验证**；
- 像素级结论只在**本地真实 GPU**上成立 ⇒ CI runner 是软件渲染，**不能**拿它的像素当结论；
- panic 之后的恢复运行（wasm `panic=abort` 下不可恢复，只保证消息可达）。

---

## §7 与视觉总攻计划的关系

视觉总攻计划书（`docs/04-internal/05-plans/frontend-rebuild-plan.md`）定义**做什么、做成什么样**；
本文只定义**引擎侧怎么接、怎么自证**。
两者不冲突：W0 产设计 → W1 打样时按本文接引擎 → 每批按 §5 自证。
