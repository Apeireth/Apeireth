# 引擎侧上屏接入说明（W0）

> 给谁看：接手上屏第一屏的施工方（人或 AI）——**不读任何对话记录也能开工**。
> 不是什么：不是设计稿（视觉在 [前端视觉总攻计划书](05-plans/frontend-rebuild-plan.md) 的 W0 产出）、
> 不是排期承诺、不是「已开工」声明。
> 口径真源：能力边界 = [ADR-0015](../adr/0015-pure-rust-ui-engine.md) 增补；
> 能力状态 = [L1 完成度体检表](ui-engine-l1-readiness.md)；缺陷账 = [引擎缺陷台账](ui-engine-defect-ledger.md)。

> **产品接入补充（2026-11-18）**：下文 §2.1b 和 §3 的输入边界已按桌面前端实际接入复核。
> 对照引擎 pin `c22622e2221fa82ce83fe22a5be23b5fb86def3f` 与本次基线 `d13c9bcdd`：
> `git diff c22622e2221fa82ce83fe22a5be23b5fb86def3f d13c9bcdd -- crates-ui` 无差异。
> 此比较仅覆盖 `crates-ui/`，不代表两次提交的其他目录相同。以下关于迟到事件的例子是应用状态层合成实验，
> **不是** Windows WebView2/中文输入法中已复现的故障，也不构成引擎已修复的声明。

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

### 2.1b ⭐ 双向绑定（信号 ⇄ DOM property）

```rust
use apeireth_ui_core::Runtime;
use apeireth_ui_dom::binding::Binding;
use apeireth_ui_dom::dom::EventKind;
use apeireth_ui_dom::events::ElementHandler;

let runtime = Runtime::new();
let name = runtime.signal(String::from("初始值"));

VNode::element("input")
    .bind(Binding::text(name.clone()))          // ⭐ 双向：信号 ⇄ DOM 的 value
    .handler(EventKind::Input, ElementHandler::new(move |event| {
        // 你自己的处理器**照样会被调用**（引擎把绑定写回与它**组合**成一个监听器）
        println!("用户输入：{:?}", event.target_value());
    }))
    .build()
```
> **行为**：信号变 ⇒ 重新渲染 ⇒ DOM 跟着变（**下行**）；
> 用户输入 ⇒ `input` 事件 ⇒ 信号被写回（**上行**）。
> ⭐ 引擎**不会**用内部监听器覆盖你的处理器 —— 两者**组合**成一个监听器
> ⇒ 你的处理器与绑定写回**都在**，且写回**先**于你的处理器（你读信号能读到新值）。
> ⚠️ 这里的「组合期间不写」指渲染侧 `apply_bindings` 暂不把信号写到 DOM；
> 它不表示 Input 事件不会写回信号。`compose_handler` 收到事件后仍先调用绑定写回，
> 再调用用户处理器。两条方向不同，见下方 §2.1c。

> ⭐ **依赖不用手动声明**：core 的 effect 是依赖追踪式的，只有在 effect 体里**读到**的信号
> 才被订阅 ⇒ 而 `Binding` 只是把信号放进 VNode、**不读值** ⇒ ⇒ 引擎提供
> `VNode::touch_bindings()`（递归读一遍树里所有绑定）⇒ ⇒ **在 effect 体里调它一次**，
> 屏幕就**自动**订阅了所有绑定信号 ⇒ ⇒ **不需要**手动 `signal.get()`（手动声明极易漏）。

### 2.1c 编辑器身份与组合输入：实际接入边界

正文属于哪一稿应由应用状态负责。编辑器切稿、跨项目、折叠、阅读模式、删除或恢复时，
需要考虑旧节点的输入事件与当前编辑器的身份、生命周期和最终上屏顺序；`key` 和固定 Signal
槽不应当被视为自动提供业务身份验证。特别是 `compose_handler` **先**执行捕获绑定的写回，
**后**执行应用处理器；只在处理器里拒绝过期稿件，不能保证绑定槽此前未被写入。

桌面前端曾用两个合成稿件记录这一风险：捕获甲稿的 VNode Input handler 和 Signal，
切换到乙稿后，先模拟渲染器写 Signal，再调用旧 handler；甲稿正文被更新。取消甲稿导致槽位
重排的单次迟到事件实验则通过，不能据此断定该场景已发生污染。实验使用的是手动构造事件，
并非实际渲染器事件调度或真实输入法复现；可复验产品仓的
`crates/apeireth-desktop-ui/src/c2/editor_identity_risk_tests.rs`，其中已知失败的切稿实验显式忽略，
运行时须显式包含 ignored 测试。产品仓与本引擎仓是不同仓库，先确认对应版本和工作树。

不要靠 `compositionend` 单独推断最后一次 Input 已提交，也不要未经真实输入法验证就把 `.bind` 换成
`.prop("value", …)`：在本基线中，`patch_props` 组合期会把 `value`/`checked` 变更排队，
下一次 `patch` 开始时 `flush_deferred_value_writes` 按队列写入，而非重新验证最终用户输入。
绑定下行则在组合期跳过；这两条路径不是同一套队列。桌面前端的位置恢复脚本仅维护选区/滚动，
不能当作正文事务或输入提交保证。

建议以 `{work, store, draft, editor-instance/epoch}` 建模应用级身份，在隔离资料中验证切换与取消，
并先测真实输入法的选词、失焦、组合中导航和最终 Input 顺序。任何守卫都需同时核对原稿、目标稿、
实际绑定槽与持久化结果，确保正常连打和最后一次合法输入不丢失；可复现后再确定是否需要引擎契约变更。
源码入口：[`compose_handler`、`apply_bindings`、`patch_props` 和 `flush_deferred_value_writes`](../../crates-ui/apeireth-ui-dom/src/renderer.rs)，
以及 [`WebDom` 的输入/组合事件载荷](../../crates-ui/apeireth-ui-dom/src/web.rs)。

### 2.2 状态变了怎么重渲染

```rust
// 状态变 → 重建树 → patch（**差分**，不是整棵重建）
let view = build_view(&state.borrow());
renderer.patch(view)?;
```
> `patch` 可以处理条件变化及节点类型变化；标签变化时会卸载旧节点并挂载新节点，
> 不能假设原节点、焦点或输入组合仍然存在。路径（`path`）按**子节点下标**寻址，
> 条件渲染或重排后必须重新确认路径；有稳定身份的列表应使用稳定 key，且遵守 keyed 差分约束。

**产品调用经验**：如果宿主选择每动画帧构建视图，视图函数也会每帧执行；差分写入少，
不等于生成 VNode、解析长文本或计算列表的成本消失。对可复用的纯投影按输入做有界缓存，
限制条目和总大小，切换作用域或来源消失时清理；超限应保留完整内容并明确降级。
这是一项调用方性能措施，不表示引擎自动缓存计算结果；应在目标宿主、典型长对话下实测。

引擎管理的子树不要由宿主脚本用 `textContent` 等方式重写来制造动画：这会移除渲染器持有的
子节点，后续差分并不等于重建被外部改动的树。动画优先作用于独立样式或引擎明确管理的状态；
若必须让外部代码管理一块 DOM，先定义清楚所有权与卸载方式，并测试重新渲染、复制及可访问性。

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

### 2.4 焦点与模态（弹层/模态）—— ⭐ 一等口径（2026-11-11 改）

```rust
// ⭐ 模态作用域：**按子树身份**打开，初始焦点是引擎的一等参数
renderer.open_modal_scope_at_identity(
    &[IdentitySegment::from_key(&Key::Str("command-palette".into()))], // 作用域根（子树根）
    Some(&[IdentitySegment::from_key(&Key::Str("command-palette-close".into()))]), // 初始焦点
)?;
// 关闭：close_modal() 会关陷阱 + 归还焦点（restore_focus）
renderer.close_modal();
```

> ⭐ **为什么是这套口径（真实事故，2026-11-11）**：旧的四调用
> （`open_focus_trap` + `focus_at_path` + `close_focus_trap`）把**三件事**推给调用方 ——
> ① 路径白名单要自己算；② 初始焦点要自己算路径；③ 关闭时的焦点归还要自己存。
> ⇒ 真实产品线第一次用满一轮时，这三件事**分别出了三次错**（见 `ui-engine-defect-ledger.md` 的 E-193/E-195 与
> `engineering-experience/02-failure-anatomy.md` 死法 8）⇒ ⇒ **模态编排是引擎的一等能力，调用方只描述意图**。

**调用方必须知道的五条（都是实测踩出来的）**：

1. ⭐ **别在宿主自建第二套焦点系统** —— 引擎已有模态作用域 + Tab 归属；
   宿主若自己接管 Tab（捕获阶段拦截 + 自己算可聚焦元素），结果是**引擎的 Tab 彻底失效**
   （根节点收到 0 次 Tab）⇒ 两套系统互相拆台。
2. ⭐ **身份是「逐层直配」** —— `IdentitySegment` 逐段匹配**直接子节点**：
   元素藏在 `dialog → frame → header → button` 时，身份要**逐段给**，不能只给首尾两段。
   ⇒ 想省心就用 `*_at_identity` 系列，**不要自己拼 `[root, leaf]`**。
3. ⭐ **作用域根按身份跟随** —— 树在模态打开期间发生位移（窄屏拉宽、遮罩撤掉导致下标变化）时，
   引擎按**身份**重新解析根的位置 ⇒ Tab 不会跑偏；⇒ 这条是 E-193 修完才成立的。
4. ⭐ **模态激活期间 Tab 由引擎独占** —— 引擎在模态激活时**即使没移动焦点也会消费 Tab**
   （`should_consume_tab`）⇒ 浏览器原生 Tab 不会逃出陷阱（E-194 修完才成立）。
5. ⭐ **「调用返回成功」≠「焦点真的移动」** —— `focus()` 对 `inert` 子树**静默无效**；
   ⇒ 宿主拿到焦点请求后要**读回 `document.activeElement` 对账**，对不上就**不消费请求**（下一帧再试）。

### 2.5 字体/图片未就绪 ⇒ 别急着算布局

```rust
renderer.set_metrics_status(MetricsStatus::Unknown);   // 字体未就绪 ⇒ 显式声明「度量未定」
// 字体 ready 之后：
renderer.set_metrics_status(MetricsStatus::Ready);
```
> ⚠ 度量未定时 `virtual_window_for` 会**显式拒绝**（返回 `Undecidable`）
> ⇒ ⇒ 这是**故意的**：宁可不窗口化，也别用错误度量算出的窗口（会造成布局跳动）。

### 2.5++ ⭐⭐⭐ 虚拟列表：**列表内容必须由「当前窗口」产出**（E-164 修的真缺陷）

⚠️⚠️ **两个坑连在一起，只有第三个写法能同时躲开**：

| 写法 | 整树 `patch` 后 | 后果 |
|---|---|---|
| 列表塞 **1 个占位行** | 窗口化结果被**覆盖** ⇒ 塌成 1 行 | E-161 |
| 列表**空着** | 列表被**清空** ⇒ 内容高度塌陷 ⇒ ⇒ **浏览器把 `scrollTop` 夹回 0** ⇒ ⇒ **滚动位置丢失** | E-164 |
| ⭐ **列表 = 当前窗口**（窗口存在 state 里） | 差分 = 「旧窗口 vs 同一窗口」⇒ **不变** | ✅ 两个坑都躲开 |

⇒ ⇒ **正解**：
```rust
// ① 窗口化时把窗口**写回状态**
let plan = renderer.patch_windowed_at_path(&[1], &heights, 2, |i| /* 行 */)?;   // ← 拿到 plan
state.borrow_mut().window = WindowState { first: plan.first, last: plan.last, /* … */ };

// ② 重建树时输出**当前窗口**（不是占位行、也不是空）
fn build_list_node(state) -> VNode {
    let w = state.borrow().window;          // ← 当前窗口
    let mut kids = vec![];
    if w.valid {
        if w.offset > 0.0 { kids.push(spacer_before(w.offset)); }
        for i in w.first..=w.last { kids.push(row(i)); }
        if w.after > 0.0 { kids.push(spacer_after(w.after)); }
    }
    VNode::element("div").attr("id", "w0-list").child(VNode::Fragment(kids)).build()
}
```
⇒ ⇒ **顺序也关键**：**先窗口化（更新 state）⇒ ⇒ 再重建树 ⇒ ⇒ 再 `patch`**
⇒ ⇒ 否则「树里的窗口」与「DOM 里的窗口」不一致 ⇒ ⇒ ⇒ **窗口来回跳**。

⇒ ⇒ **为什么这条必须知道**：浏览器的 `scrollTop` 在**内容高度塌陷**时会被**夹回 0**
⇒ ⇒ ⇒ 这是**浏览器机制**，不是引擎 bug ⇒ ⇒ ⇒ 整树重建时若动了列表区域，**必然**踩到。

⭐⭐⭐ **但现在有引擎级保证了**（E-165）⇒ ⇒⇒ **上面那条「树要带当前窗口」的纪律**：

| 用法 | 现在还需要吗 |
|---|---|
| 用 `patch_windowed_at_path` / `patch_windowed`（**推荐**） | ❌ **不需要** ⇒ ⇒ 引擎**自动**标记「虚拟化接管」⇒ ⇒ ⇒ 整树 patch **自动跳过**该子树 ⇒ ⇒ ⇒ **滚动位置不会丢、窗口不会塌**（真实浏览器已验证） |
| 自己管窗口内容 | ⚠️ 需要手动 `.virtualized()` 标记 |

⇒ ⇒ ⇒ **结论**：**用引擎的虚拟列表 API ⇒ ⇒ ⇒ 整树重建就是安全的** ⇒ ⇒ ⇒
⇒ ⇒ ⇒ 「树要带当前窗口」那套状态同步**只在你自管窗口时**才需要。

### 2.5+ ⭐⭐ 虚拟列表：**每次改完树都要重新窗口化**（E-161 修的真缺陷）

⚠️⚠️ **「整树重建」模式与「虚拟列表」不兼容**，除非你知道「列表区域由窗口化管」。

```rust
// ⚠️ 反例（E-161 的真缺陷）：树里塞了占位行 ⇒ ⇒ patch 会覆盖窗口化结果
let tree = build_demo_tree(&state, &[]);   // 列表里只有 1 个占位行
renderer.patch(tree)?;                      // ⭐ 把 7 行 + 留白 覆盖成 1 行！

// ✅ 正例：改完树**立刻**重新窗口化
renderer.patch(tree)?;
render_windowed_list(&mut renderer, &[1], 1000)?;   // ← 抽成共用函数，两条路径一致
```
> ⇒ ⇒ 症状：虚拟列表**在第一次交互（输入/滚动）后塌成 1 行**
> ⇒ ⇒ 用户看到列表**不再滚动、不再虚拟化**。
> ⇒ ⇒ 参考实现：`crates-ui/evidence/p5-b/harness/src/lib.rs` 的 `render_windowed_list()`
> （**首屏与 tick 共用**）与 `w0_tick()`（改完树**立刻**重新窗口化）。
> ⇒ ⇒ 防回归断言：`l1_w0_window_probe` 断言「tick 前后窗口行数**完全一致**」。

⚠️ **另一条**（E-162，**已更正过一次**）：`patch_windowed_at_path` 内部要读
`scrollTop`/`clientHeight` ⇒ ⇒ **触发强制回流** ⇒ ⇒ 成本**随页面规模增长**
（实测：干净页 **15.7 ms** / 同页加 20,000 节点 **140.2 ms**）⇒ ⇒
⇒ ⇒ **别把页面的首次 layout 成本算到引擎头上**。

⚠️⚠️ **曾经以为「延后到 paint 之后再窗口化能省掉那次回流」—— 已撤回**（E-162，三次复现）：
**立即** t_dom ≈ 15~20 ms / **延后一帧** ≈ 36~42 ms ⇒ ⇒ ⇒ **延后没有收益，只是晚一帧**
⇒ ⇒ ⇒ 首屏**立即窗口化**即可。

⇒ ⇒ **真正要避开的**是「**每次**都重新窗口化」：那等于每 tick 一次强制回流
⇒ ⇒ 大页面上会把主线程占满 ⇒ ⇒⇒ **连宿主自己的 `requestAnimationFrame` 都会饿死**
⇒ ⇒ ⇒ ⇒ **只在滚动（窗口位置变了）时才窗口化**（E-163）。

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
2. **IME 组合期间**渲染侧会门控受控值写向 DOM；property 路径的延迟队列可在组合结束后补写，
   binding 下行则跳过当前写入。事件仍可能向 Signal 写回；最终事件顺序与业务身份要由宿主和应用验证（见 §2.1c）。
   产品当前 pin `c22622e` 的 Web 适配仅从 `KeyboardEvent` 填充 `is_composing`；
   本批修复后的引擎也从 `InputEvent` 读取该事件自带的标志。旧 pin 不会自动获得此行为；
   标志反映事件当刻，不证明最后一次合法输入已经落盘或组合事务已安全结束。

---

## §4 五个坑（都是本项目实测踩出来的）

| # | 坑 | 症状 | 正确做法 |
|---|---|---|---|
| 1 | 手写窗口行、**不留白** | 列表「不滚动」（`scrollTop` 被浏览器夹回 0） | 用 `patch_windowed_at_path`（留白由引擎算） |
| 2 | 未协调组合态就改 `value` 或卸载编辑器 | 可能打断输入或丢失最后一次上屏 | 区分 property 延迟队列与 binding 下行跳过；以真实输入法验证提交与导航顺序（§2.1c） |
| 3 | 字体没加载就窗口化 | 窗口位置错、加载完跳一下 | 字体未就绪时**显式** `set_metrics_status(Unknown)`，ready 后 `Ready`（⚠️ 渲染器**默认是 `Ready`** —— DOM 度量是真实几何；「度量未定」只有宿主知道 ⇒ 必须由宿主显式声明） |
| 4 | 以为 `scroll` 会冒泡 | 委托根上收不到滚动 | 直连容器 |
| 5 | `path` 下标语义搞错 | 读不到度量（`NotImplemented`） | `node_at_path(&[0])` = **树根的第 0 个子节点**；度量必须落在**同一个**节点上 |
| 6 | ⭐ **宿主自建焦点陷阱**（2026-11-11 实测） | 两套焦点系统互相拆台：引擎根节点收到 **0 次 Tab**，模态期焦点乱跑 | 用引擎的 `open_modal_scope*` / `close_modal`；宿主只描述意图，不接管 Tab |
| 7 | ⭐ **身份路径按「跳中间容器」拼**（2026-11-11 实测） | 模态作用域**根本没建立**（解析失败被静默忽略）⇒ Tab 在错的子树里走 | 身份**逐层直配**；用 `*_at_identity`；**解析失败必须报错，不许静默忽略** |

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
- [ ] ⭐ 模态作用域：打开 ⇒ 引擎**自己**消费 Tab（根节点 keydown 的 `defaultPrevented` 为真）⇒ Tab 落点全在作用域内 ⇒ 跳禁用项 ⇒ 可循环回起点
- [ ] ⭐ 焦点请求：请求一个目标 ⇒ **读回 `document.activeElement` 对账** ⇒ 对不上就不算生效（`inert` 子树里 `focus()` 静默无效）
- [ ] 长列表：滚到底/顶 ⇒ 窗口位置对、内容不空白、总高度恒定
- [ ] 错误边界：故意让一屏抛错 ⇒ 看到兜底 UI，不是白屏
- [ ] 布局跳动：CLS 累计在预算内（亚像素噪声不计）

---

## §6 你可以依赖什么、不能依赖什么

**可以依赖**（有浏览器/真实 GPU 证据）：受控输入、事件面与载荷、焦点与陷阱、IME 静默期、
测量与命中（含 DPR 乘数）、虚拟列表、错误边界、像素等价（五面 + intro 逐像素 0 差异）。

**模态作用域（2026-11-11 新增，真实产品线用满一轮后）**：`open_modal_scope` / `open_modal_scope_at_identity`
/ `close_modal` / `focusable_paths_under` / `should_consume_tab` —— 作用域按**子树身份**打开、**初始焦点是引擎参数**、
**模态期间 Tab 由引擎独占**、**根按身份跟随树位移** ⇒ 宿主不再需要自建陷阱或自管焦点栈。

**不能依赖**（尚未验证）：
- 真实 Tauri 宿主（macOS WKWebView / Linux WebKitGTK / Windows WebView2）上的行为
  ⇒ ⇒ **引擎会自动降级到 WebGL 后端**（代码里已实现），但降级后的画面**尚未在真机验证**（⭐ **四级口径**：**IMPLEMENTED**，**未** HARDWARE VALIDATED）；
- 像素级结论只在**本地真实 GPU**上成立 ⇒ CI runner 是软件渲染，**不能**拿它的像素当结论；
- panic 之后的恢复运行（wasm `panic=abort` 下不可恢复，只保证消息可达）。

---

## §7 与视觉总攻计划的关系

视觉总攻计划书（`docs/04-internal/05-plans/frontend-rebuild-plan.md`）定义**做什么、做成什么样**；
本文只定义**引擎侧怎么接、怎么自证**。
两者不冲突：W0 产设计 → W1 打样时按本文接引擎 → 每批按 §5 自证。
