# 引擎缺陷台账（2026-10-25 全量审计）

> **性质**：本文件是「引擎缺陷」的**单一真源**（每条含**证据**、**根因**、**修法**、**状态**）。
> ⇒ 其它文档**只引用编号**，不复制正文。
>
> **审计范围**：`crates-ui/` 全部 8 个成员（57,765 行）⇒ 三条线并行深读 + 逐条实测复现。
> **审计日期**：2026-10-25。
> **审计方式**：① 逐文件读码；② 在 `/tmp` 用本 crate 的 rlib / 独立探针**实跑复现**；
> ③ 生产侧（TypeScript）用 `node --experimental-strip-types` **直接 import** 做差分。

## 0. 状态口径

| 状态 | 含义 |
|---|---|
| **已修** | 已改代码 + **有实测**（浏览器或 native）+ 已提交 |
| **在修** | 改动进行中（并行施工线） |
| **待修** | 已定位、有修法，尚未动手 |
| **已否** | 试过并**实测否掉**（留档免得重走） |
| **待验证** | 读码推断，缺实测（明确标注） |

---

## 1. P0 —— 会崩页面 / 永久失效 / 静默状态错误

### E-01 ⭐ **状态与渲染分属两个 runtime** ⇒ 响应式链断 ⇒「幽灵输入」 — **已修**

- **现象（实测）**：预算面数字输入框键入 `7`（原值 `8`）⇒ 失焦：
  | 观测项 | 参考侧（生产） | 引擎侧（修前） |
  |---|---|---|
  | 失焦后 `input.value` | `64`（钳制并**写回**） | `87`（引擎**不接管**） |
  | 反馈区 | 「越界：已钳制到 64（1..=64）」 | **空** |
- **根因**：`apeireth-ui-kit/src/settings_budget.rs` 的 `BudgetState::new()`
  **自建** `Runtime::new()` ⇒ 草稿 / 反馈信号**不属于**宿主挂 effect 的那个 runtime
  ⇒ ⇒ **effect 根本不依赖这些信号** ⇒ ⇒ 改草稿**不触发重渲染**。
- **对照证据**：同仓库 `provider` 面**所有信号都在宿主里建**（同一 runtime）⇒ 它的搜索框**受控正常**。
- **修法（三处缺一不可）**：① `BudgetState::new(runtime)` **接受外部 runtime**（状态与渲染同属一个）；
  ② `WebDom` 的事件闭包**填 `DomEvent::target_value`**（此前 `with_target_value` **全仓只有测试在用**）；
  ③ 预算面数字输入框**补 `prop("value", …)`**（property 通道 ⇒ 取值受控）。
- **实测（修后）**：引擎侧失焦后 `value = 64` + 反馈「越界：已钳制到 64（1..=64）」⇒ **与参考侧逐字一致**。
- **回归**：设置页 21/21 · 治理面通过 · 顶层通过 · 场景 pass · P5-b pass · crates-ui 809 测试。

### E-02 ⭐ **对照装置与被测对象一起错** ⇒ 时区缺陷被系统性掩盖 — **已修（装置侧）**

- **现象（实测）**：同一时刻，生产在 `Asia/Shanghai` 给 `14:30:15`，
  而引擎的固定 UTC 版给 `06:30:15` ⇒ **差 8 小时**。
- **根因（两处）**：
  ① 引擎侧 `format_clock_ms` / `format_date_time_ms` **固定 UTC**，
  而生产走浏览器 `toLocaleTimeString('zh-CN', …)`（**本地时区 + ICU 版本 + 补零规则**）；
  ② ⭐ **对照装置硬编码** `Emulation.setTimezoneOverride {"timezoneId": "UTC"}`
  ⇒ ⇒ **两边都是 UTC ⇒ 差异被完全掩盖** ⇒ ⇒ 判据**测不到真实场景**。
- **修法（结构性）**：
  ① 引擎侧：把「时刻文本」变成**宿主能力** —— `TimeTextFormatter` trait +
  **进程级**槽（时区本就是**进程级环境量**）+ 默认 UTC 实现（native 可测）+
  wasm 宿主装**浏览器同一个 `Intl` API** ⇒ ⇒ 连 ICU 差异都一致；
  ② 装置侧：时区改为**显式非 UTC**（默认 `Asia/Shanghai`，可用 `APEIRETH_COMPARE_TZ` 覆盖）
  ⇒ ⇒ 本地**立刻暴露**、CI **可复现**。
- **状态**：装置侧**已改**；引擎侧**改造中**（见 §4 在修项）。

### E-03 ⭐ **空块变非空块 ⇒ 内容追加到父节点末尾** — **在修**

- **位置**：`apeireth-ui-dom/src/renderer.rs` Fragment 分支 ⇒ `patch_unkeyed_children`
  对新增子项用 `mount_node(child, parent, None)`（**锚点 = None ⇒ 追加末尾**）。
- **实测**：`div > [Fragment([Fragment([])]), footer]`，条件由假变真
  ⇒ `before: [footer]` → `after: [footer, span("条件")]`，**期望 `[条件, footer]`**。
- **影响**：`ui-kit` 的 `nothing()`（空 Fragment）用在 20+ 处 ⇒ 条件卡出现在容器**底部**。
- **修法**：为「块内新增」算出**块锚点**（该 Fragment 最后一个 DOM 节点的后继兄弟）⇒ **通用机制**，非特例。

### E-04 ⭐ **Fragment 内嵌套条件块 ⇒ 兄弟身份错位** — **在修**

- **实测**：`div > Fragment[span(x), Fragment([]), span(z)]`，条件由假变真
  ⇒ `before: [id2="x", id4="z"]` → `after: [id2="x", id4="y", id6="z"]`
  ⇒ **id4 那颗节点从「z」变成了「y」**（身份转移），「z」被重建。
- **根因**：`flatten_mounted` / `flatten_vnode` **递归**摊平所有嵌套 Fragment
  ⇒ 嵌套块的身份被融进兄弟序列。
- **修法**：摊平**只做一层**（保留嵌套 Fragment 作子块）⇒ 嵌套块自己按块级规则配对。

### E-05 ⭐ **失败的 patch 被宿主吞掉 ⇒ 界面永久冻结** — **待修**

- **位置**：`ui-dom/src/renderer.rs:276-293`（出错时恢复**旧树**，但 DOM 已改过一半）；
  宿主侧 `ui-demo` 至少 9 处 `let _ = guard.patch(tree)`（**错误被丢弃**）。
- **实测**：property 消失 ⇒ `Err(Unsupported)`，而 **title 已改成「新」、子树已是「新子」**
  ⇒ **DOM 与渲染器内存树劈叉** ⇒ 此后每帧都拿过期旧树做基准 ⇒ **界面永久停在旧状态，无任何日志**。
- **修法**：① 渲染器侧：出错后**不**拿过期旧树继续 diff（标记 `desynced` ⇒ 下次整树重建）；
  ② 宿主侧：patch 错误**必须 log**；③ 提供宿主可查询的状态（`is_desynced()` / `last_error()`）。

### E-06 ⭐ **陈旧 flush 错误污染下一次写入 ⇒ `Signal::set` panic** — **在修**

- **位置**：`ui-core/src/signal.rs:77-93`、`src/runtime.rs:238-247`
  （`flush_error` 只在失败时写、**成功时从不清空**；每次 `try_set` 无条件 `take_flush_error()`）。
- **实测**：自激 effect 失败一次 ⇒ 宿主没轮询 ⇒ 用户下一次交互 `other.set(42)` ⇒ **panic**
  （`try_set` 返回的 `Err(FlushCycle{node:#1})` 与 `other` 毫无关系）⇒ wasm 里 **整页 abort**。
- **修法**：错误绑定「**本次写入期间**」（generation 机制）⇒ **写路径不得 panic**。

### E-07 自激 effect ⇒ `set` / `flush` panic；且重复跑 100 遍 — **在修**

- **位置**：`ui-core/src/signal.rs:89-93`、`src/runtime.rs:358-363`、`:539-547`。
- **实测**：effect 里写自己读的信号（`if v != norm { v.set(norm) }`）⇒ `set` 当场 panic。
- **修法**：写路径不 panic + **尽早判定自激**（同一次 flush 内同一 effect 反复排入 ⇒ `Err(CycleDetected)` + 环路径）
  ⇒ 不靠「跑 100 遍」的次数上限。

### E-08 `Signal::update` 闭包里读同一信号 ⇒ RefCell 双重借出 panic — **在修**

- **位置**：`ui-core/src/signal.rs:96-109`（`borrow_mut()` 后闭包里再 `get()`）。
- **实测**：`sig.update(|v| { *v = *v + sig.get(); })` ⇒ panic；`try_update` **也** panic ⇒ **违反 `try_` 契约**。
- **修法**：先取出旧值并**释放借用**再调闭包；`try_update` 对无法安全处理的情形返回 `Err`。

### E-09 `BackendPreference::Auto` 未防「无后端 feature」⇒ 上游 panic — **在修**

- **位置**：`ui-gpu/src/engine.rs:828-837`（`Auto` 分支**不**检查 `enabled_backend_features()`；
  `WebGl`/`WebGpu` **都**检查了）⇒ 命中上游 `wgpu-30.0.1/src/api/instance.rs:73-79` 的 `panic!`
  ⇒ wasm 里 **整页崩溃**。
- **修法**：`Auto` 分支加同样守卫 ⇒ 返回 `Err(GpuError::Unsupported(...))`（**一行**）。

### E-10 `WebDom` 从不填 `DomEvent::target_value` — **已修**（并入 E-01）

- **实测**：引擎侧 `input` 事件拿到的值恒为 `""` ⇒ 依赖事件负载的路径全断。
- ⚠️ 子代理最初推断的后果是「用户每敲一个字就被清空」⇒ **实测否掉**
  （预算面因**同时**缺 property 通道 ⇒ 表现为「引擎不接管」而非「被清空」）⇒ **以实测为准**。

---

## 2. P1 —— 静默出错 / 泄漏 / 性能

| # | 缺陷 | 证据 | 状态 |
|---|---|---|---|
| **E-11** | **`effect_pre` 重跑落到渲染后阶段** ⇒ 破坏 ADR-0015 的顺序硬保证 | 实测顺序 `["pre","post"]` → `["post","pre"]` | 在修 |
| **E-12** | **effect 体与派生节点在运行时表里永不删除** ⇒ 单调泄漏 | `Rc::strong_count` 停在 2；全仓无 `remove` | 在修 |
| **E-13** | flush 中途报错 ⇒ 同批剩余 effect 静默丢弃、不重排、`pending()` 显示 0 | `runtime.rs:492-508` | 在修 |
| **E-14** | flush 内新建 effect 提升后**不请求 flush** ⇒ 一直不执行 | 实测 `pending()==1` 且不跑 | 在修 |
| **E-15** | 宿主丢弃队列后 `flush_requested` 卡死 ⇒ 之后**永久不再刷新** | 实测 `v.set(1)`→`clear()`→`v.set(2)` ⇒ `queued()==0` | 在修 |
| **E-16** | `dispose` 后 `on_cleanup` **永不执行**；`child()` 缺存活断言 ⇒ 孤儿 effect | 实测执行 0 次；子 effect 每次写信号都重跑 | 在修 |
| **E-17** | **引擎无 resize 通路** ⇒ 窗口缩放后画面错位；WebGPU 交换链**永久失效** | `engine.rs:379-396`（全 crate 唯一一次 `surface.configure`）；`gl_FragCoord` 翻转高度**烘焙成字面量** | 在修 |
| **E-18** | **跨行 uniform 声明处理不了** ⇒ 留下孤立 `uRes;` ⇒ naga 解析失败 | 复现输出；**X2 路径有单测、生产路径 X3 没有** | 在修 |
| **E-19** | 变换链**不保持行号** ⇒ 文档与 ADR 的「行号一一对应」**不成立** | 实测 6 行源 → 变换后 **9 行** | 在修 |
| **E-20** | `Rgba16Float` 被归类 `RawBytes` ⇒ fp16 分支**静默偏亮** | `engine.rs:133-146` | 在修 |
| **E-21** | 场景层 `dt` 只夹上界 ⇒ **负 `dt` 把状态推离目标** | `scene.rs`；`state.rs`；`camera.rs` | **已定位 ⇒ 判定为「生产同源缺陷」**（见下） |
| **E-22** | 引擎**从不 `device.poll()`** ⇒ GLES 后端异步错误**完全静默** | `engine.rs:366-371` 等 | 在修 |
| **E-23** | **设备丢失不可检测** ⇒ `GpuError::DeviceLost` 是**死码** | `grep DeviceLost` 只命中定义处 | 在修 |
| **E-24** | 每次重渲染**拆装全部监听器** ⇒ WebDom 侧 O(N²) | 实测 `RemoveListeners ×1 + AddListener ×1` | 在修 |
| **E-25** | keyed 列表的 LIS 判定 O(N²) | `renderer.rs:550`（`Vec::contains`） | 在修 |
| **E-26** | `InMemoryDom` 节点永不回收 + 操作日志只增不减 | 实测 5 轮挂载/卸载后 `node_count=26`（树已空） | 在修 |
| **E-27** | 每帧 **34 次堆分配**（场景 → uniform 投递） | `scene/uniforms.rs:70-114` | 在修 |
| **E-28** | `create_scene_pipeline` 每次新建 **7 个** GPU 资源（无护栏） | 已有实测证据：p5-b 为此不得不加 `redraw()` 复用 | 在修 |
| **E-29** | **29 条规则在 Rust 与 TS 里各有一份**，**零一致性机制** | 逐名比对：Rust `pub fn` 73 个 ↔ TS `export function` 375 个 ⇒ 同名 29 条 | 在修（差分装置） |
| **E-30** | ✅✅ **已闭合（2026-10-27 核验）：原记录的「TS `null` ⇒ 过期 / Rust `None` ⇒ 未过期」分歧**不可能发生**。①**Rust 侧从不产出 `null`**：`crates/foundation/governance/src/authorization.rs:147` 的 `expires_at_ms: Option<i64>` 带 `#[serde(default, skip_serializing_if = "Option::is_none")]` ⇒ ⇒ `None` 时字段**被省略**（不是 `null`）；且 `is_valid_at`（`:191-199`）对 `None` 判**有效**（未过期）。②**TS 侧对任何非数字/非字符串（含 `null`）都归一成 `undefined`**：`frontend/companion-desktop/src/lib/governance/ledger.ts:27-34` 的 `approvalTimeMs` 只在 `typeof === 'number'` / `'string'` 时返回数值，其余 `return undefined`；而 `isApprovalExpired`（`:38`）判据是 `entry.expiresAtMs !== undefined && entry.expiresAtMs <= nowMs` ⇒ ⇒ `undefined` ⇒ **未过期**。⇒ ⇒ 两侧口径一致：**「没有过期时间」= 未过期**。⚠️ 唯一残余形态：若**其它生产者**在线上真的发出 `null`，TS 的 `null !== undefined` 为真且 `null <= now` 为真 ⇒ ⇒ 会被判**已过期**（失败**收**向，不会误放行）。当前后端不会发 `null` ⇒ ⇒ 不构成现存缺陷；若未来出现第三方生产者，应在**入口**归一（`approvalTimeMs` 已覆盖）而不是改过期语义。| `crates/foundation/governance/src/authorization.rs`、`frontend/companion-desktop/src/lib/governance/ledger.ts` ⇒ ⇒ **语义一致** |
| **E-31** | wasm 与内存 DOM **语义不一致**（class 全删 / 子树摘除标记范围 / `get_property` 实时值 / `target_path` 起点与下标） | 实测 native `None` vs 浏览器 `Some("")` | 在修 |
| **E-32** | `patch_props`：property 消失 ⇒ **整帧中止** | 实测 `Err(Unsupported("移除 property `value` 未支持"))` | 在修 |
| **E-33** | `is_dirty()` 状态**撒谎**（跑完后无条件清脏） | 实测自激 effect 跑完后 `is_dirty()==false`（应 true） | 在修 |

---

## 3. P2 —— 文档不实 / 诊断粒度（0 装纪律）

| # | 缺陷 | 位置 |
|---|---|---|
| **E-34** | ✅✅ **已闭合（2026-10-27 核验）**：`ui-dom` 的「事件委托未接线 / `Dom` 没有监听器接口」旧说法**已从文档移除**（`apeireth-ui-dom/src/lib.rs:56-68` 现在明确区分：**元素直挂已接线**、未接线的只有「委托根**单**监听器」这一形态、`EventRegistry` 未与渲染器接线）。**逐条核验**：① `Dom` trait 有监听器接口（`dom.rs:230/238/254` 的 `add_listener` / `remove_listeners` / `remove_listener`）✔；② 渲染器在**挂载/差分/卸载三处**真的接线（`renderer.rs:1121` / `:1162-1170` / `:1204`）✔；③ `EventRegistry` **确实未**接进渲染器（`renderer.rs` 零引用，只在 `events.rs` 自身单测里出现）✔。⭐ 复核命令：`grep -rn '没有监听器接口\|事件委托未接线' --include=*.rs --include=*.md crates-ui/` ⇒ **零命中**。| `crates-ui/apeireth-ui-dom/src/lib.rs`、`renderer.rs`、`events.rs` ⇒ ⇒ **文档与实现一致** |
| **E-35** | `getIntents` 文档说有、**实际没有**（实测 `__BUDGET__` 只有 `getState`/`selectSection`/`ready`） | `ui-demo/src/settings_budget.rs:27` |
| **E-36** | `ui-gpu` 文档宣称「变换链不插行 ⇒ 行号一一对应」⇒ **实测不成立**（见 E-19） | `ui-gpu/src/glsl.rs:10/39-44/766` |
| **E-37** | `get_current_texture()` 的 `Lost`/`Outdated`/`Timeout` 语义**塌缩**成同一个错误 | `ui-gpu/src/engine.rs:703-709` |
| **E-38** | `on_uncaptured_error` 在 `request_device` **之后**才装 ⇒ 中间窗口的错误落到 wgpu 默认处理器（浏览器里 = panic） | `ui-gpu/src/engine.rs:366` |
| **E-39** | `Auto` 路径**探测两次** WebGPU（两次 `requestAdapter()` 往返） | `ui-gpu/src/engine.rs:331`+`:836` |
| **E-40** | 强制 WebGpu 时宿主无 `navigator.gpu` ⇒ 报「拿不到适配器」而非「宿主不支持 WebGPU」 | `ui-gpu/src/engine.rs:848-857` |
| **E-41** | `verify_naga_layout` 的 `zip` **按较短者截断** ⇒ 尾部成员不校验 | `ui-gpu/src/engine.rs:787-813` |
| **E-42** | `glsl.rs` 的 `pub` 浮点类型函数在非法类型上 `panic!` | `ui-gpu/src/glsl.rs:661-692` |
| **E-43** | 线程本地默认运行时**永不释放** | `ui-core/src/lib.rs:77-97` |
| **E-44** | ⭐ **`build.sh` 的 wasm 产物路径硬编码** ⇒ 外部设 `CARGO_TARGET_DIR` 时**拿到旧 wasm** ⇒ ⇒ **假绿/假红** | 4 处（治理 / 设置页 / p5-b / p5-gpu）⇒ 已修 |
| **E-45** | ⭐ `device.poll(wait_indefinitely())` 在 **GLES 后端阻塞** ⇒ 设备忙时**整页卡住**；超时又被当成**失败** ⇒ 场景面整面红 | `ui-gpu/src/engine.rs` ⇒ 已修（有界超时 + **超时不算失败**） |
| **E-46** | `apeireth-ui-scene` **不在 workspace 成员表** ⇒ CI 的 `cargo test --workspace` **跑不到它任何单测**；且它**单独构建直接报错** | `crates-ui/Cargo.toml` ⇒ 已补登 |
| **E-47** | ⭐ **响应式系统缺一条不变量守卫**：「状态对象的信号必须建在**宿主给的** runtime 上」⇒ 该缺陷**类型层面完全合法** ⇒ ⇒ 编译器**不可能**发现 | **已修（类型层）**：`ui-core` 新增使用者形态 `ReactiveScope`（**没有** `effect`/`flush`/`new`）⇒ 状态持有者**结构上**无法挂 effect ⇒ ⇒ **症状不可达**；脚本判据同步从「近似」升级为「精确」（R1/R2/R3，零漏判）⇒ 保留为**冗余双保险** |
| **E-49** | ⭐ **「用户输入进不进引擎状态」这类缺陷，静态像素对照测不到**（浏览器自己画了值 ⇒ 画面对）⇒ ⇒ 幽灵输入只能靠**一次性探针**发现 | 已加**通用交互对照模块**（`harness/tools/interaction.py`，真实键盘事件 ⇒ 两侧同路径键入 ⇒ 逐字比对）⇒ **capability / character / provider** 三面接入（含**密钥输入**：只比长度与「是否等于键入串」，**明文不落盘**） |
| **E-50** | ⭐ **门面与内部能力面不同步** ⇒ 外部宿主要么绕过门面、要么用不到新类型 | `apeireth-ui/src/lib.rs` ⇒ 已补 `pub use apeireth_ui_core::ReactiveScope;`（**零逻辑**） |
| **E-51** | ⭐ **引擎的 WebGPU 后端画不出点图元**（wgpu 的 `PointList` 没有点尺寸，`gl_PointSize` 无处落地）⇒ ⇒ 同一份 intro 逻辑**换后端 = 换画面** ⇒ ⇒ 且该腿的「0 差异」是**假绿**（引擎侧根本没画粒子）| `apeireth-ui-gpu/src/engine.rs` 的 `point_list_supported` ⇒ 只认 `Backend::Gl` ⇒ ⇒ **待修**：改用**实例化四边形**粒子路径（⇒ WebGPU 后端也能画） |
| **E-52** | ⭐ intro 火焰**与生产仍有 12% 逐像素差异**（69,345 / 576,000）⇒ ⇒ 引擎内部两条渲染路径（点图元 / 实例化四边形）**互证一致**（仅差 15 像素）⇒ ⇒ 已排除「后端差异」「路径差异」「uniform 值」「着色器抄错」「粒子数」「种子序列」「时间语义」⇒ ⇒ 根因**仍未定位** | 见下方 §3.6 的排除清单与候选实验 |
| **E-53** | ⭐⭐ **headless Chrome 默认走 SwiftShader（纯软件）** ⇒ ⇒ 此前**所有**浏览器实测都跑在软件渲染上 ⇒ ⇒ 「真实 GPU 未实测」这条账长期挂着（且**误以为**是缺设备）| `harness/tools/compare.py` 的 `launch_chrome` ⇒ 已加 `--use-angle=vulkan` / `--enable-features=Vulkan` / `--ignore-gpu-blocklist` ⇒ ⇒ 实测 renderer 由 `SwiftShader` 变为 `AMD Radeon 8060S (RADV STRIX_HALO)` ✔ |
| **E-54** | ⚠️ `data` 面在**真实 GPU** 上有 **0.063%**（554 / 885,132）的像素差⇒ ⇒ **散布全图**、**非位移**（各向平移 1px ⇒ 差异涨到 5.5 万+）、**非字体时机**（等 `document.fonts.ready` 后**不变**）⇒ ⇒ 定位到「**排版/亚像素**」层⇒ ⇒ 引擎侧元素宽度是**分数**（如 69.140625）⇒ ⇒ **根因待定** |
| **E-55** | ⭐⭐ intro 的 12% 差异**全部在粒子层**（13.5s 无粒子 ⇒ 差异 **0**；其余帧 5.7%~12.0%）⇒ ⇒ 而 uniform / 种子 / 着色器 / GL 状态 / 粒子数**全部全等** ⇒ ⇒ ⚠️ 「两侧绘制次数不对称」假设**已实测否定**（`sync=1` 对称模式下：生产画 **4** 次 / 引擎画 **3** 次 ⇒ ⇒ 差异**仍是 69,345**，逐像素不变）⇒ ⇒ ⇒ **12% 是真实的渲染差异**（引擎 vs 生产），不是装置假差。
    ⚠️⚠️ **更正一条推理错误（2026-10-27）**：此前我写「webgl 腿与 webgpu 腿的差异掩码**完全相同**
    ⇒ ⇒ 说明与后端无关」——这个推理**不成立**：两条腿当时都传了 `--sprite quad`
    ⇒ ⇒ **引擎侧跑的是同一份实例化四边形代码**（只是底层后端不同）
    ⇒ ⇒ 差异掩码相同是**必然**，**不是**「两条独立实现互证」的证据。⇒ ⇒ 该结论作废。
    ✅ **可用的替代数据**：`--sprite point`（真 `GL_POINTS`，仅 webgl 可用）
    ⇒ ⇒ 差异 **69,360**（quad 路径 69,345）⇒ ⇒ **两条渲染路径与生产的差异几乎一致**
    ⇒ ⇒ ⇒ 说明差异**与图元拓扑无关**，落在「顶点 varying 的值 / 片元输入」这一层。
| **E-56** | ⚠️ 引擎的**实例化四边形**（`InstancedQuad`）展开与**硬件点图元**（`GL_POINTS`）**不等价** ⇒ ⇒ `probe=flat` 下生产 vs 引擎 webgpu 差 **32.9%**（而 webgl 差 **0**）⇒ ⇒ ⇒ 同一份逻辑在两个后端**产出不同的粒子几何** ⇒ ⇒ 「后端一致性」尚未真正达成 | `evidence/p5-b/drive_intro.py --probe flat` ⇒ ⇒ **待修**：核对 quad 的半宽/覆盖语义 vs 硬件点图元 |
| **E-57** | ⭐⭐ **CI 首次给出结论**（`ui-engine CI` @ `0abdb494` ⇒ **failure**，4 个 job 红）⇒ ⇒ 其中两个浏览器 job 的死因**相同**：`build.sh` **在构建 wasm 之前**就检查 wasm 产物是否存在⇒ ⇒ **干净 checkout（CI）上产物必然不存在** ⇒ ⇒ 脚本在构建前 `exit 1` ⇒ ⇒ **CI 恒红**；开发机因为**之前构建过** ⇒ ⇒ 同一脚本「侥幸通过」⇒ ⇒ **「本地绿、CI 红」长期并存且看不出原因**。⭐ 附带发现 `harness/build.sh` 的 `if ! command -v wasm-bindgen … && [ ! -x` 被塞进了一行注释 ⇒ `if` 被后面的 `] ; then` **意外补全** ⇒ `bash -n` **通过**但那段保护**被吞掉**。| 两处 `build.sh`（`evidence/p5-b` 与 `apeireth-ui-demo/harness`）⇒ ⇒ **已修**：`if` 复原 + 产物检查移到 `cargo build` 之后；⭐ 实测：删掉产物（模拟干净 checkout）后 `build.sh` **EXIT 0** 且产物重建、`verdict: pass` |
| **E-58** | 规则装置 `--check` 把 **`generator`（生成环境元数据，含 `process.version`）**也纳入一致性比对 ⇒ ⇒ **本机 Node 补丁号一变，门禁就红**：CI 是 `v22.23.3`、开发机 `v22.23.1` ⇒ **240 个用例的期望值逐字相同**，却因版本号字符串不同判「不一致」。| `gen_expected.mjs` ⇒ ⇒ **已修**：`--check` 比对前剔除 `generator`（仍写进文件可追溯）；⭐ 实测 `v22.23.3` 与 `v22.23.1` **都 EXIT 0** |
| **E-59** | ⭐⭐ **相位对照的抖动根因（两处）**：①`collect_phases` **先导航、后设**冻结变量 ⇒ ⇒ 动画已跑几毫秒才暂停 ⇒ ⇒ 冻结位置每次不同 ⇒ ⇒ 同一个 4~5% 差异在三轮里分别落在 `450ms` → `0/150/300ms` → `0/300/450ms`（**坏相位在移动**）；②`capture_frames` docstring 写「按**固定帧数**采集」但实现里还有「到 `duration_ms` 就停」⇒ ⇒ 帧数取决于截图速度（实测 12~19）⇒ ⇒ 而判据是 `== FRAME_COUNT(20)` ⇒ ⇒ **几乎恒红**，测的是「采集机器的速度」而非引擎。| `tools/compare.py` ⇒ ⇒ **已修**：①用 `Page.addScriptToEvaluateOnNewDocument` 把冻结脚本**在导航之前**注入（动画自第 0 毫秒即冻结）⇒ ⇒ **三轮 × 5 个相位全部 0 差异**；②`capture_frames` 改为**以帧数为主**（时间仅作 `4×` 兜底）⇒ ⇒ 两侧稳定 **20 / 20 帧**。⚠️ **仍抖**：`t6_unmount_same_timing` 实测同一构建跑出 `0.8/0.8`、`1.3/1.2`、**`67.3/1.5`**⇒ ⇒ 属**主线程调度噪声**；**不擅自放宽阈值**，待用「同相位重复 N 次取中位数」修 |
| **E-60** | ⭐⭐ **片头的等价判据**（ADR-0017 §3 定案）：「差异必须为 0」**不可达**（两套 GL 实现的点图元边缘覆盖规则不同 ⇒ ⇒ 恒红且无解），而「放宽到某个百分比」会放过**真正的语义回归** ⇒ ⇒ 正解是**把差异约束在可解释的形态内**：差异像素**必须全部落在粒子边缘带内**（离粒子轮廓 ≤ N 像素）且**占比 ≤ 预算** ⇒ ⇒ 实现级边缘差异被**明确豁免且有界**，而语义级回归（位置/颜色/数量/半径错）会**产生带外差异** ⇒ ⇒ **当场红**。| `crates-ui/evidence/p5-b/check_particle_edge.py`（脚本已写）⇒ ⇒ **待接进 CI**；⚠️ 实测首跑：带外像素 **65,722 / 576,000（11.41%）** ⇒ ⇒ **当前 12% 差异**并非「边缘覆盖」形态 ⇒ ⇒ 说明「覆盖规则」解释**不成立** ⇒ ⇒ 根因**仍未定位**（E-55） |
| **E-61** | ⚠️⚠️ **诊断探针本身的三个真缺陷**（让整批实验**跑在旧产物上**）：①`main.ts` 的 TS **模板字符串**里注释含**反引号** ⇒ ⇒ **终止模板** ⇒ ⇒ **构建失败** ⇒ ⇒ 页面是**旧的** ⇒ ⇒ 探针**从未生效**（实测踩到，且**日志上看不出来**）；②hook 里用 `getShaderParameter(shader, SHADER_TYPE)` 判片元 ⇒ ⇒ 在 `shaderSource` **调用期间**查询会**抛异常** ⇒ ⇒ hook **中断** ⇒ ⇒ 同样「从不生效」；③`flat` hook 只判 `source.includes('vStreak')` ⇒ ⇒ **顶点着色器也含** `vStreak` ⇒ ⇒ 顶点被换成片元 ⇒ ⇒ **编译失败** ⇒ ⇒ 画面空 ⇒ ⇒ 与另一侧「都没画」⇒ ⇒ **假绿**。| `crates-ui/evidence/p5-b/web/intro/main.ts` ⇒ ⇒ **已修**：①清除模板内反引号；②改用**纯源码特征**判片元（`含 vStreak` 且**不含** `gl_Position`）⇒ ⇒ 不依赖任何 API 查询；③补齐片元判定。⭐ **教训**：探针必须**自证生效**（本次用 `window.__PROBE_DIAG__` 记录 `shaderSource` 调用次数）⇒ ⇒ 否则「探针无效」与「没有差异」**无法区分** |
| **E-62** | ⭐⭐ **片头 12% 的根因高度收敛**（2026-10-27）：用**真正生效**的 varying 编码探针（两侧**同款**探针片元）⇒ ⇒ **webgpu 腿**（同一 quad 路径）差异 **1,243 / 576,000（0.216%）**，而**正常渲染**是 **12%** ⇒ ⇒ ⇒ **varying（vCol/vAlpha/vStreak）几乎一致** ⇒ ⇒ ⇒ **唯一剩下的变量 = 粒子坐标**（引擎的 `_engine_point_coord` vs 生产的原生 `gl_PointCoord`）⇒ ⇒ 二者**都出自 wgpu/naga 的 GLES 后端**，而生产出自**浏览器原生 WebGL2** ⇒ ⇒⇒ ⇒ **12% ≈ 两套 GL 实现对「点精灵坐标」的像素量化差异**（GL 规范允许实现自定量化）⇒ ⇒⇒ ⇒ ⇒ ⇒ ⇒ **该差异在「用 wgpu 抽象」的前提下不可归零** ⇒ ⇒ ⇒ ⇒ 验收应走 ADR-0017 §3 的**有界差异**判据。| `evidence/p5-b/drive_intro.py --probe varying` ⇒ ⇒ **待验证**：①用**真正生效**的 flat 探针重测「几何是否 0 差异」（旧那次**从未生效** ⇒ 结论作废）；②核对 `_engine_point_coord` 的**取值/量化** vs GL 规范；⚠️ **已排除**：透视（生产 `w=1`）、点精灵 y 方向、图元拓扑、绘制次数、varying 值、uniform、种子、GL 状态 |
| **E-63** | ✅ **flat（纯色）探针已真正生效**（2026-10-27）：引擎侧新增 `assets/intro_sprite_flat.frag` + `INTRO_SPRITE_FLAT_FRAGMENT_GLSL` + wasm `probe="flat"` + 页面把 `probe` 传给 wasm；生产侧 hook 的片元判定已修（**含 `vStreak`** 且**不含** `gl_Position`）⇒ ⇒ **实测**：**webgl 0 / 576000、webgpu 0 / 576000** ⇒ ⇒⇒⇒ **两侧粒子并集逐像素完全一致** ⇒ ⇒⇒⇒ ⇒ ⇒ **⇒ 「几何零差异」确认**（此前「几何一致」是**假绿**，已作废）。⚠️ 生效判据 = **探针自证计数器 `reps=2`**（模块级 `let`，**无 TDZ 风险**）⇒ ⇒ **「0 差异」与「两侧都没画」可区分**。
| **E-64** | ⚠️ **驱动的 leg key 不含 probe** ⇒ ⇒ 探针跑的 leg 与正常渲染的 leg **同 key** ⇒ ⇒ ⇒ 后者**覆盖**前者 ⇒ ⇒ ⇒ **探针结果丢失**（表现为「报告里少了一腿」，**看不出原因**）。已修：key 改为 `frozen_at:only:sprite:probe`。| `crates-ui/evidence/p5-b/drive_intro.py` ⇒ ⇒ ⚠️ **修复后驱动仍报失败**（具体异常待查）⇒ ⇒ ⇒ **flat 探针的有效 leg 仍未拿到**（E-63 未闭合） |
| **E-65** | ⚠️ **flat 探针下生产侧页面跑不完**（`RuntimeError: 片头页没有跑完`，2026-10-27）⇒ ⇒ 生产侧的 flat 片元替换**破坏了页面** ⇒ ⇒ 具体编译错误待查（需看 `pageerror`）。⚠️ ⇒ ⇒ 这是 flat 探针**至今未产出有效 leg** 的直接原因（E-63 / E-64）。| `crates-ui/evidence/p5-b/web/intro/main.ts`（`flatFrag`）⇒ ⇒ **待查**：flat 片元的 `in` 声明是否与生产的 varying **逐字同序**；`gl_PointCoord` 在生产侧是否可用；⇒ ⇒ **修法方向**：先在生产侧**单独**编译 flat 片元并抓 `pageerror`，再接引擎侧 |
| **E-66** | ⭐⭐ **T2 DOM 不一致的根因已定位并修掉**：归一化后的 DOM 树**只差 1 处** —— `<span class="time">` 的文本 **生产 `09:10` / 引擎 `01:10`**（**差 8 小时**）⇒ ⇒ 根因 = 审批卡自带「**固定 UTC**」实现，**绕过**了宿主已安装的浏览器 `Intl`（进程级时间器）⇒ ⇒ 已修：改走 `governance_tab::format_clock_hm` ⇒ ⇒ **T2 DOM 结构：一致 ✔**（并附 `clock_hm` trait 口径：默认取 `clock()` 前 5 字符，**用 `chars().take(5)`** —— `—` 是多字节，按字节切会 **panic**）。⭐ 顺带查清：**只有这一个时间文本**（其余文本节点逐字相同）⇒ ⇒ 不是「全部时间都错 8 小时」⇒ ⇒ 更说明是**单个消费方绕过**。| `crates-ui/apeireth-ui-demo/src/approval_card.rs` + `apeireth-ui-kit/src/governance_tab.rs` ⇒ ⇒ **已修**（`a8541ef7`）；⚠️ `apeireth-ui-demo/src/lib.rs` 曾把 `format_clock_utc` **公开导出** ⇒ ⇒ 边界守卫**第一次运行就抓到** ⇒ ⇒ 已取消导出（防复发） |
| **E-67** | ⭐⭐⭐ **坐标探针成立 ⇒ 12% 差异的载体已锁定 = 粒子内坐标**（2026-10-27）：新增 `assets/intro_sprite_coord.frag`（把粒子内坐标编码进 RGB：R=x、G=y）+ 生产侧同款 hook ⇒ ⇒ **实测（reps=2，生效）**：**webgl 116340（20.2%）、webgpu 51306（8.9%）** ⇒ ⇒⇒ ⇒ 「坐标**确实**不同」✔。⭐ 与 flat 探针（**0/0**）合并 ⇒ ⇒⇒⇒ **完整结论链**：**① 几何（粒子并集）逐像素完全一致；② 坐标数值不同** ⇒ ⇒⇒⇒ ⇒ ⇒ **⇒ 12% 差异的载体 = 粒子内坐标**（`gl_PointCoord` vs 引擎自算 `_engine_point_coord`）。⚠️ **尚未量化坐标差**（是**上下翻转** / **偏移** 还是 **量化**）⇒ ⇒ **下一步**：直接比对两侧的坐标值。| `crates-ui/apeireth-ui-gpu/assets/intro_sprite_coord.frag` + `evidence/p5-b/web/intro/main.ts` + `evidence/p5-b/harness/src/lib.rs` ⇒ ⇒ ⚠️ 曾两次把「探针没生效」的差值当成结论 ⇒ ⇒⇒ ⇒ **教训：任何探针差值都必须带 `reps` 自证** |
| **E-68** | ⭐⭐⭐⭐⭐ **根因确认：y 原点语义相反（GLES 左上 ↔ 引擎 NDC 左下）**（2026-10-27）：用 coord 探针**逐像素**量化坐标差 ⇒ ⇒ **webgpu 腿（引擎四边形路径）**：差异 52139 px 中 **R（x）差 = 0 的占 50356（96.6%）** ⇒ ⇒ **x 坐标一致**；**G（y）差集中在 -177 / -171 / -169 / -177** ⇒ ⇒ **y 有系统性偏移 ≈ -170/255 ≈ -0.67**。⭐⭐ **双向印证**：① 代码侧 `apeireth-ui-gpu/src/glsl.rs:1283` 写的是 `_engine_point_coord = aCorner * 0.5 + 0.5` ⇒ ⇒ y 随 NDC **向上**递增（底部 = 0）；② 规范侧 **GLES 的 `gl_PointCoord` 原点在左上、y 向下** ⇒ ⇒ **语义相反**；③ 数值侧「翻转差 = 1 - 2y（y 均值 ≈ 0.165）」⇒ ⇒ **0.67**，与实测 -0.67 **精确吻合** ⇒ ⇒⇒⇒ **⇒ 三方闭合 ⇒ 根因确认**。⚠️ **webgl 腿（引擎走 naga 的 `gl_PointCoord`）**的差呈「**逐像素跳变**」型（差值 ≈ 245~247 ≈ 1.0；「翻转假设」只命中 11%、「同向」20% ⇒ ⇒ **两者都不是**）⇒ ⇒ 属**另一类**问题（≈ 一个像素的光栅化位置差），**与上面这条不同源** ⇒ ⇒ 单独记账。| `crates-ui/apeireth-ui-gpu/src/glsl.rs:1283` ⇒ ⇒ **修法**：四边形路径的 `_engine_point_coord.y` 改为 **0.5 - corner.y * 0.5**（与 GLES 左上原点对齐）⇒ ⇒ ⇒ **可归零**（⇒ ⇒ 与 ADR-0017 §3 的「有界差异判据」不同：这是**真缺陷**，应当修） |
| **E-69** | ✅✅ **E-68 的修复已落地并验证：intro 粒子层差异 12.04% → 0.016%（降 770 倍）**（2026-10-27）。修法：`apeireth-ui-gpu/src/glsl.rs` 四边形路径的 `_engine_point_coord` 改为 `vec2(corner.x*0.5+0.5, 0.5 - corner.y*0.5)`（与 **GLES 左上原点**对齐）。实测：**正常渲染** webgl **69345（12.04%）→ 93（0.016%）**、webgpu **69345 → 93** ⇒ ⇒ **两条后端结果完全一致**；**coord 探针**下 webgpu 差异 **52139 → 4396（降 92%）**且 G 差由 -170 收敛到 **-1/0/+1**（量化级）⇒ ⇒ ⇒ **根因与修复闭环**。⭐ **结论**：这是**真缺陷的真修复** ⇒ ⇒ **不需要** ADR-0017 §3 的「有界差异判据」来兜底。⚠️ 余量 93 px（0.016%）为**边缘舍入级**，待单独记账。| `crates-ui/apeireth-ui-gpu/src/glsl.rs`（y 原点翻转）⇒ ⇒ **已修并验证** |
| **E-70** | ⭐⭐ **口径澄清 + E-56 主体定位：intro 的「12%」有**两个载体**，必须分开记账**（2026-10-27）。①**引擎四边形路径（quad）**：E-68/E-69 已修 ⇒ ⇒ 正常渲染 **69345（12.04%）→ 93（0.016%）**（webgl 与 webgpu **双后端互差 0** ⇒ ⇒ 等价性成立）。②**引擎自带点图元路径（`Topology::PointList`）**：仍与生产差 **69360（12%）**⇒ ⇒ **这才是 E-56 的主体**（`quad 展开 ≡ 硬件点图元`），且**引擎 quad vs 引擎 point** 差 **101457（17.6%）**。⭐ **坐标探针定位**（`--probe coord`，粒子内坐标编码进 RGB）：生产(原生点) vs 引擎point(webgl) 差异 51536（8.9%），其中 **R 差 99.82% 为 0**（x 一致）、**G 差中位数 +43 且分布弥散**（既非纯翻转、也非纯常数偏移）⇒ ⇒ **朝向差 + 像素中心约定差，两因素叠加**。⚠️ 另测：引擎 quad(webgl) vs 引擎 quad(webgpu) 在探针下互差 **20.3%**（R/G 呈 ±245/246 互补型）⇒ ⇒ **引擎两后端的屏幕 Y 朝向不同**（引擎已在片元阶段用 `glsl::fix_frag_coord_semantics` 补偿 `gl_FragCoord`，但**顶点阶段算出的 `_engine_point_coord` 未做同源补偿**）⇒ ⇒ ⇒ **⇒ 判据「quad 展开 ≡ 硬件点图元」当前是**真红**，必须修引擎 point 路径。| `crates-ui/apeireth-ui-gpu/src/glsl.rs`（point 路径 `gl_PointCoord` 语义）⇒ ⇒ **修复中**（子线 e56-pointcoord） |
| **E-71** | ✅✅ **五面浏览器对照判据：3 次连续运行，差异像素恒为 0 ⇒ 「全绿 + 判据不抖」达成**（2026-10-27，真实 GPU = AMD Radeon8060S / RADV）。命令：`python3 crates-ui/evidence/p5-b/drive.py --out .../evidence`（真实 GPU，Linux headless Chromium）。3 次运行逐项一致：**生产 vs 引擎(WebGL) = 0 px**（max_channel_delta 0）、**生产 vs 引擎(WebGPU) = 0 px**（maxd 1）、**引擎 WebGL vs 引擎 WebGPU = 0 px**（maxd 1）⇒ ⇒ **无抖动**。⭐ **负控制已在判据内**：`counterfactual_squeezed_production_vs_engine_after` 差异 **208508 px（48.27%）** ⇒ ⇒ 证明该判据**确实能分辨**（若把生产面挤压后引擎跟不上 ⇒ 立刻变红）⇒ ⇒ **不是恒真的假绿**；另有 `webgl_vs_vertically_flipped` 诊断 **539726 px（93.70%）** ⇒ ⇒ 证明判据**对上下朝向敏感**。| `crates-ui/evidence/p5-b/drive.py` ⇒ ⇒ **已达成（3× 逐像素 0）** |
| **E-72** | ⭐⭐ **E-54 根因锁定：对照两侧「样式链不同源」**（2026-10-27，真实 GPU 复现 554/885132 = 0.0626%）。证据：①`compare_data.py` 两侧**同一次浏览器会话**实时拍摄 ⇒ **不是陈旧图**；②对齐后引擎 origin = 参考 origin = (236, 219.765625) ⇒ **同设备像素格**，且 `user` 段 **0 差异** ⇒ **不是装置伪差**；③差异像素：最大通道差 1~2 占 89%，集中于 **y=1068/913 的 820px 全宽行**（= 元素 border-bottom，参考 (98,44,42) vs 引擎 (98,45,42) ⇒ **G 通道差 1/255**）；④**样式链分歧**：引擎页 `web/engine/data/index.html` 链了 `/settings-data.css`（手写复刻），**参考页 `web/reference/data/index.html` 没有** ⇒ ⇒ **两侧不是同一视觉规格**。⇒ ⇒ **⇒ 这不是引擎画错，而是「对照装置不成立」** ⇒ ⇒ 修法方向：**两侧共用同一份样式来源**（把生产 SettingsView 的实际样式对齐进 harness，或让参考页同链该 CSS）⇒ ⇒ 修好后 t1 逐像素 0 才能为真。 | `crates-ui/apeireth-ui-demo/harness/settings/web/engine/data/compare_data.py` + `web/engine/data/index.html` ⇒ ⇒ **根因已定，待修** |
| **E-73** | ✅✅ **E-56 修复已落地并验证：引擎点图元路径与生产 69360（12%）→ 29（0.005%）**（2026-10-27）。根因（**X7**，代码级依据）：引擎 WebGL 后端把主渲染上下翻转时，naga 的 GLSL 后端只在**顶点阶段** 追加 `gl_Position.yz = vec2(-gl_Position.y, ...)`（`naga-30.0.1/src/back/glsl/writer.rs:1958-1967`，由 `wgpu-hal-30.0.1/src/gles/mod.rs:1371` 的 `ADJUST_COORDINATE_SPACE` 触发，呈现时再翻回来见 `gles/web.rs:299-315`）⇒ ⇒ 而 `gl_PointCoord` 是**窗口空间**的光栅器内建、**没跟着翻** ⇒ ⇒ 最终画面里点精灵内部的 y 轴与生产**朝向相反**。修法：`glsl.rs` 新增 `fix_point_coord_semantics`（源级 `gl_PointCoord → vec2(x, 1.0 - y)`），**只用于点图元路径**（四边形路径的 `_engine_point_coord` 在翻转前的空间里算 ⇒ 不受影响）。实测：**正常渲染** webgl point 腿 69360 → **29 px**；quad 腿保持 **93 px**（无回归）；引擎双后端互差 **0**。单测 3 条（`point_coord_correction_{flips_the_y_axis,keeps_the_x_axis_untouched,keeps_line_numbers}`）⇒ crates-ui **1054 passed / 0 failed**、clippy 0、fmt 0。 | `crates-ui/apeireth-ui-gpu/src/glsl.rs` ⇒ ⇒ **已修并验证** |
| **E-74** | ⭐⭐ **判据 v1 被实测证伪 ⇒ 重写为 v2；并发现 `quad_path_matches_point_path` 判据**从未生效****（2026-10-27）。①**v1 形态不可满足**：v1 要求「差异像素**全部**落在**粒子并集轮廓**的边缘带内」⇒ ⇒ 实测 93 像素中 **90 在带外** ⇒ 恒红。原因是掩码语义错：并集轮廓只描述整团火焰外形，而火焰内部**每个粒子也有自己的轮廓**（互相重叠）⇒ ⇒ 落在并集深处的像素可能正好在**另一个粒子的边缘**上 ⇒ ⇒ 被误判成语义级差异。②**另一条判据是死代码**：`drive_intro.py` 的 `quad_path_matches_point_path` 仍按 **3 段**解析腿 key，而 E-64 修复后腿 key 已是 **4 段**（`frozen_at:only:sprite:probe`）⇒ ⇒ **所有腿被跳过** ⇒ ⇒ `measured` 恒为空 ⇒ ⇒ 判据**永远 inconclusive**（看起来像「没结论」，实际是**判据没跑**）⇒ ⇒ 已修（4 段解析 + 匹配 point 腿时保留 probe 段）。③**双后端判据对结构性不支持的腿误判**：点图元在 WebGPU 上**无点尺寸**（规范限制，非缺陷）⇒ ⇒ 旧逻辑把它算 fail；已改为**显式 `not_applicable`** 并保留 `legs_invalid` 记录（**不隐藏**能力缺口）。④**判据 v2 落地**（`check_particle_edge.py`）：A 组几何不变量（质心 ≤0.05px / 亮像素数 ≤0.01% / 总亮度 ≤0.01%）**严格**；B 最大通道差 ≤16、C 超阈像素 ≤0.05% **才有豁免**；并**内建反事实自检**（翻转/平移必须被判失败，否则 `exit 2`）。实测：quad 腿质心差 **0.0027 px**、超阈 **93（0.0161%）**、最大差 **8** ⇒ **PASS**；point 腿 **29（0.0050%）** ⇒ **PASS**；反事实 翻转 **130031**、平移 **53143** ⇒ **均被拒**（判据可失败）。| `crates-ui/evidence/p5-b/check_particle_edge.py`、`crates-ui/evidence/p5-b/drive_intro.py`、`docs/adr/0017-ui-engine-gpu-layer-authorization.md` §3 ⇒ ⇒ **已修（v2）** |
| **E-75** | ⭐ **E-54 修复方向已定（同源化），尚未落地**（2026-10-27）。追查结论：生产 data 面的样式**不在任何生产 CSS 文件里**（`app.css` / `styles.css` / `design/*.css` 均无 `.setting-block`）⇒ ⇒ 它在 `frontend/companion-desktop/src/lib/views/SettingsView.svelte` 的**作用域 `<style>`** 里 ⇒ ⇒ 引擎侧的 `public/settings-data.css`（336 行，手写）是对它的**复刻** ⇒ ⇒ **必然有偏差**（实测 554 px / 0.0626%，差异行是 820px 全宽 border-bottom，G 通道差 1/255）。⇒ ⇒ **修法定为「同源化」**：构建期把 `SettingsView.svelte` 的 `<style>` **去掉作用域后缀**导出成一份 CSS，**两侧都链它** ⇒ ⇒ 剩余差异只剩光栅化（由 E-74 的 v2 有界判据兜底）。⚠️ **不采用**「给参考页也链手写 CSS」（让参考侧迁就实现侧 ⇒ 对照失去意义）；**不采用**「放宽阈值到 0.1%」（掩盖规格偏差）。| `.../lib/views/SettingsView.svelte`、`harness/settings/scripts/extract-settings-assets.mjs` ⇒ ⇒ **待修** |
| **E-76** | ⚠️ **CI 现状（诚实记账，2026-10-27）**：`ui-engine CI` 在 `9eb45c53` 为 **failure**，**唯一红的 job = 「生产对照（生产侧 vs 引擎侧，逐像素）」**；其余 cargo check / clippy / fmt / test / wasm32 build / cargo deny **全绿**。而**本地同入口 `bash crates-ui/evidence/p5-b/build.sh` ⇒ EXIT 0**，本地五面判据 3 次运行差异恒 0 ⇒ ⇒ **本地与 CI 结论不一致**，差异待定位（最可疑变量：CI runner 无 GPU ⇒ SwiftShader 软件光栅化；GitHub 日志下载被签名重定向挡下，改用工件报告或本地软光栅复现）。⭐ 本批修复 `f716f575` 的 run `40502906606` 在 **queued**。⚠️ 口径：**未拿到 CI 绿之前，不许声称「CI 全绿」**。| `.github/workflows/ui-engine-ci.yml` ⇒ ⇒ **待验证** |
| **E-77** | ✅✅ **A 类绕过点清账：`format_clock_utc` 已彻底断掉外部可达性 + 守卫从「按文件豁免」改为「按符号 + 块级」**（2026-10-27，全仓排查子线发现）。①**残留可达**：`apeireth-ui-demo/src/approval_card.rs` 的 `pub fn format_clock_utc` 虽已不在生产调用路径上（生产走 `format_clock_hm`，E-66 已修），但它仍是 `pub` 且经 `pub mod approval_card;` **对外可达** ⇒ ⇒ 一次顺手改回去就会**静默复发「差 8 小时」**。修法：把它**搬进 `mod tests`**（不再 `pub`）⇒ ⇒ 外部不可达，只作 native 可复现基准。②**守卫盲区**：`check_ui_boundaries.py` 的 Guard B 对 `approval_card.rs` **整文件 `continue`**，而脚本文档写的是「定义处 + 测试处」⇒ ⇒ **豁免面比文档大**（台账 §5 同型教训）⇒ ⇒ 该文件里新写的第二份自算时刻会**静默通过**。修法：新增 `_cfg_test_lines()` 按**花括号深度**判定 `#[cfg(test)]` 作用域 ⇒ ⇒ 豁免面回到**符号级 + 块级**；取消整文件豁免。⭐ **可失败性自检**：临时在模块级注入 `format_clock_utc(ms)` ⇒ 守卫 **EXIT 1**（命中 `approval_card.rs:83`）；还原 ⇒ **EXIT 0**。| `apeireth-ui-demo/src/approval_card.rs`、`apeireth-ui-demo/harness/scripts/check_ui_boundaries.py` ⇒ ⇒ **已修并验证** |
| **E-78** | ✅✅ **B 类真绕过点修复：渲染器「写属性先于 keyed 可判定性判定」⇒ 改为「先定状态、再动作」**（2026-10-27）。缺陷（`apeireth-ui-dom/src/renderer.rs`）：`patch_node` 先 `patch_attributes`（**写 DOM 属性**）→ 再 `patch_children` → `patch_keyed_children` 里 `diff_keyed` **才**判定可判定性；不可判定时①属性**已落 DOM**、②内存树仍留旧 vnode、③`Undecidable` 被**显式排除**在「标 `desynced`」之外 ⇒ ⇒ 渲染器**自称同步**、**不走**整树重建自愈 ⇒ ⇒ **静默分叉**。⚠️ 旧注释「`Undecidable` 发生在触碰 DOM 之前」在 keyed 路径上**不成立**，而原测试的夹具**父元素没有属性**（`patch_attributes` 一个字节没写）⇒ ⇒ 该不变量只是**夹具侥幸**，不是构造保证。修法：新增 `ensure_children_keyed_decidable()`，在**写任何属性之前**预检（仅会走 keyed 路径时，n = 直接子节点数）。⭐ **回归测试** `an_undecidable_diff_happens_before_any_attribute_is_written`：让父元素 `class='a'→'b'` 同时子节点重复 key ⇒断言 ①报 `Undecidable` ②**DOM 里 class 仍是 `a`**。**可失败性已验证**：临时撤掉预检 ⇒ 该测试红（断言正是 class 被写成 b）；还原 ⇒ 绿。| `crates-ui/apeireth-ui-dom/src/renderer.rs` ⇒ ⇒ **已修并验证** |
| **E-79** | ✅ **A9 的静默失效路径已堵死：wasm 上「时刻文本器未装」从静默回落 UTC 改为响亮失败**（2026-10-27）。缺陷：`apeireth-ui-kit/src/governance_tab.rs::time_text()` 在**未安装**时间文本器时一律回落 `UtcTimeText` ⇒ ⇒ wasm 宿主忘记调用 `apeireth_ui_time::install()` 时，画面里的时刻会与生产**差一个时区**（E-66 的「差 8 小时」正是这一类的后果）且**没有任何报错**。架构约束：`apeireth-ui-kit` 有「**0 外部依赖**」纪律 ⇒ 不能内置浏览器 `Intl` 实现 ⇒ ⇒ 默认值只能在宿主侧装，因此**修法不是加默认**，而是**让缺失变响亮**。修法：`time_text()` 未装时 —— **wasm32 ⇒ `panic!`**（信息含「必须调用 install」与台账编号）；**native ⇒ 保持固定 UTC**（native 不参与「生产 vs 引擎」逐像素对照 ⇒ ⇒ 可测、可复现）。调用点全部在 `governance_tab` 组件层 ⇒ ⇒ panic 只会在「真的渲染时刻文本却没装」时触发。⭐ 契约测试 `without_install_native_falls_back_to_utc`（新线程 ⇒ 槽为空 ⇒ 真正走「未装」分支）。| `crates-ui/apeireth-ui-kit/src/governance_tab.rs` ⇒ ⇒ **已修并验证** |
| **E-81** | ✅ **A5 已处理（文档层显式化，代码不变）**：`crates/engine/organ/src/emergence.rs` 的 `day_key_from_epoch_ms` / `minutes_of_day_from_epoch_ms` 是**正确的** UTC 历法（Howard Hinnant `civil_from_days`），且其语义是**触发器 UTC 日键**（「每天 09:00」在任何观看者时区下是同一瞬间），**不是**「本地今天」。⇒ ⇒ 全仓排查把它判为**合规**，唯一风险是**下游误用**（把它当本地日展示）。处理：在 `day_key_from_epoch_ms` 的文档里显式写明「**UTC 日键 ≠ 本地日**（最多差 ±1 天），面向用户的『今天/昨日』必须走进程级时间文本器或宿主本地时区实现」。**代码零改动**。| `crates/engine/organ/src/emergence.rs` ⇒ ⇒ **文档已显式化** |
| **E-82** | ⚠️ **E-54 的一个假设被实测证伪（负结果，已回滚）**（2026-10-27）。假设：引擎页把子面 CSS （`/settings-data.css`）放在 `/settings-test.css` **之前**，而参考侧的生产编译期 `<style>` 由 Svelte 运行时注入到 `<head>` **之后** ⇒ ⇒ 同特异度时两侧胜出规则相反 ⇒ ⇒ 造成 554 px 的 1 LSB 差。**实验**：改 `gen-entry-pages.mjs` 把子面 CSS 移到 `settings-test.css` 之后并重生成页面、`RUSTC_WRAPPER= bash build.sh` 重建 dist ⇒ ⇒ **复测仍是 554/885132（判据仍不过）** ⇒ ⇒ **假设不成立**（层叠顺序不是成因）。**已回滚**该投机改动（不留无依据的改动）。⇒ ⇒ 下一步应换探测手段：**按元素定位**（对 y=1068/913 那条 820px 全宽 border 行，用 CDP 取两侧该元素的 `computedStyle`（`border-bottom-color` / 其来源规则）做逐属性对比，而不是继续猜层叠顺序。⚠️ 另记：本环境 `sccache` 已损坏（`build.sh` 直接跑会 EXIT 101），必须 `RUSTC_WRAPPER=` 空置才能构建。| `harness/settings/scripts/gen-entry-pages.mjs` ⇒ ⇒ **已回滚**；E-54 仍开 |
| **E-83** | ⚠️⚠️ **CI 唯一红 job 的「无 GPU」假设被实测证伪；本地两种渲染模式都通过**（2026-10-28）。**已拿到的 CI 结论**（run `40521112072` / `9e1c7d13`）：**8 个 job 里 7 个 success**（cargo check / clippy / fmt / test / wasm32 build / cargo deny / 「对照（试点+工具卡面）」），**唯一 failure = 「生产对照（生产侧 vs 引擎侧，逐像素）」**，失败 step = 「跑生产对照（逐像素）」（即 `bash build.sh`）。**证伪实验（本地，决定性）**：①真实 GPU（RADV）⇒ `drive.py` 五面判据 **0 差异 / EXIT 0**；②**软件光栅化**（`APEIRETH_CHROME_SWIFTSHADER=1`，实测 adapter = `ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero)), SwiftShader driver)`）⇒ ⇒ `drive.py` **仍是 0 差异 / EXIT 0**；③同一软件渲染下跑 **CI 的同一条命令 `bash build.sh`** ⇒ ⇒ **EXIT 0**（报告 `"ok": true`）。⇒ ⇒ **「CI runner 无 GPU ⇒ 像素判据不可判定」不成立**（E-80 的预案**不该实施**，否则等于给一个不存在的原因改判据）。**剩余候选**（未证）：runner 的**字体集合**/Chromium 与 Node **版本**差异、capture **时序**（runner 更慢 ⇒ 逐帧/等待窗口不同的取帧）、磁盘/路径差异。**当前阻塞**：本机 token 对 `actions/jobs/{id}/logs` 与 `actions/artifacts/{id}/zip` 均返回 **403**（缺 `actions:read`）⇒ ⇒ **无法取得 CI 侧 report.json / 日志** ⇒ ⇒ 需 owner 侧提供权限或代为下载工件。⚠️ 口径：本地两种渲染模式都绿**不等于** CI 会绿；在拿到 CI 侧数字前**不声称 CI 全绿**。 | `.github/workflows/ui-engine-ci.yml`、`crates-ui/evidence/p5-b/build.sh` ⇒ ⇒ **待 CI 侧证据** |
| **E-84** | ⭐ **E-54 的差异形态已量化（排除抗锯齿直觉，指向「分数位置上的全宽线/阴影」）**（2026-10-28）。逐像素取证（`evidence/data/reference-data-readings.png` vs `engine-data-readings.png`）：①差异集中在 **y=1068**（内容块**最后一行**）与 **y=913** 两行，各 **820 列**（x∈[4,823] = 内容宽 828 内缩 4）⇒ ⇒ **全宽**；②**逐列通道差不同**（同一行内 R 差 ∈ {−6,−2,−1,0,5}、G ∈ {−4,−1,1,2}、B ∈ {−2,0,1}）⇒ ⇒ **不是纯色 border**（纯色会是逐列同差）⇒ ⇒ 更像**带渐变/阴影/1px 环**的元素；③全图统计：G 通道最常见差 **−1**（3986 px）、R 0/−1/−2、B −1/0 ⇒ ⇒ 幅度全在 ±6 内。**本轮新增的排除项**：④抽取 CSS **与生产源码逐规则一致**（`public/settings-data.css:318-338` 的 `.sp-data .user-avatar-img/-fallback` 两条 `box-shadow: 0 0 0 1px var(--line)` 与 `SettingsView.svelte:5300/5310` **完全相同**）⇒ ⇒ **不是抽取失真**；⑤层叠顺序假设已被 E-82 实验证伪；⑥「CI 无 GPU」假设已被 E-83 证伪。⇒ ⇒ 剩余最可能成因：**该线/阴影落在分数 Y 上时的 AA 覆盖不同**（两侧内容块虽已对齐到同一 origin，但**内部布局**可能仍差一个分数相位）。**下一步实验（明确可执行）**：给引擎舞台施加 **±0.5px / ±0.25px** 的额外 `marginTop` 微扰，看 554 是否**塌缩到 0** ⇒ ⇒ 若是，则确认是分数相位 AA，按根因把两侧内容的**内部整数对齐**（而不是放宽阈值）；若否，则逐个禁用候选规则（先从 `box-shadow: 0 0 0 1px` 起）做**规则级差分**定位。| `harness/settings/web/engine/data/compare_data.py` ⇒ ⇒ **根因收窄，待微扰实验** |
| **E-85** | ⭐⭐⭐ **CI 唯一红 job 的**真正根因**已由 CI 工件取证确定：引擎画布在 CI 上是**纯白/近空白**，不是「渲染差异大」**（2026-10-28）。**取证手段**：GitHub 工件 `p5b-production-compare-evidence`（`ci_evidence.py` 用 curl 取，302→200）内含 `report.json` 与各画布截图。**关键数据**：①`production_vs_engine_webgl/webgpu` 差异 **575935/576000 = 99.9887%（maxd 255）**，而**引擎 webgl vs webgpu 只差 103 px（0.0179%）** ⇒ ⇒ 两个后端画出了**同样的东西**；②截图实测：`webgl.canvas.png` **mean=254.99 / distinct=50**、`webgpu.canvas.png` **mean=255.0 / distinct=1（纯白）**，而 `production.canvas.png` **mean=31.68 / distinct=11556**（正常暗场景）⇒ ⇒ **引擎画布是白的，生产有内容**；③CI 控制台：引擎两侧都打出了 `pipeline created` / `frame 0/1/2 drawn`，随后出现 `WebGL: CONTEXT_LOST_WEBGL: loseContext: context lost` 与 `A valid external Instance reference no longer exists.` ⇒ ⇒ **上下文丢失**；④CI 的 Chromium = **155.0.8059.39**，本地 = **154.0.8037.92**（版本差）；本地用 **SwiftShader 软件渲染**复跑仍 **0 差异 / EXIT 0** ⇒ ⇒ 不是「无 GPU」本身（E-83 已证伪），更像 **Chrome 155 在该环境下的行为差异**。**已落地的判据改进（本批代码）**：`drive.py` 新增 `canvas_blank_stats()` 与 `canvas_blank` / `canvas_blank_any` 判据，并在结论处**响亮打印**「引擎画布空白 ⇒ 不是渲染差异大，而是根本没画出来」，同时提示优先查 context lost / 后端未真正绘制 / 截图早于首帧。⇒ ⇒ **修前**：同一事实被呈现成「差 99.9887%」，读者只会以为渲染逻辑全错；**修后**：一眼看出是「没画」。实测校验：CI 工件 `webgl` ⇒ blank=True（近空白）、`webgpu` ⇒ blank=True（纯白）；本地实跑 ⇒ `canvas_blank_any=false`、`verdict=pass`。| `crates-ui/evidence/p5-b/drive.py` ⇒ ⇒ **判据已修；CI 红因（Chrome 155 环境行为）待修** |
| **E-86** | ⚠️ **E-54 的「分数相位 AA」假设被实测证伪**（2026-10-28）。**实验**：给引擎舞台的 `marginTop` 加**额外亚像素偏移**（临时钩子 `E54_EXTRA_Y`，仅本地实验、**已回滚**），看 554 是否塌缩。结果：`+0` ⇒ **554**（此时引擎 origin = 参考 origin = `(236, 219.765625)`，**同相位**）；`+0.5` ⇒ **35366**（origin `220.265625`）；`-0.5` ⇒ **35366**（origin `219.265625`）⇒ ⇒ **当前对齐已是全局最优**，任何半像素偏移把差异放大 **64 倍** ⇒ ⇒ **「差异来自分数相位 AA」不成立**（若成立，微扰应能让它变小而不是暴涨）。⚠️ 顺带修正一个操作细节：`compare_data.py` 里有**两处**相同的对齐代码（`:310` 与 `:440`），只改第一处时**偏移完全没生效**（`origin_after` 不变、554 不变）⇒ ⇒ 做这类实验必须先确认**改的那一处真的在跑**（用 `origin_after` 之类可观测值自证，别只看数字没变就下结论）。⇒ ⇒ 至此 E-54 已排除：**特异性差**（抽取脚本注释已论证等价）、**层叠顺序**（E-82）、**分数相位 AA**（本条）、**抽取失真**（E-84）。**下一步应换方向**：定为**结构性差异**（y=1068/913 两行所在元素在两侧的**结构与伪元素**可能不同 —— 生产是 Svelte 组件，引擎是自研实现，`t2_dom_equal` 只比了归一化子集）⇒ ⇒ 用 CDP 取该处元素的 `tag`/`class`/`::after` 对比。| `harness/settings/web/engine/data/compare_data.py` ⇒ ⇒ **未改动（钩子已回滚）** |
| **E-87** | ✅✅ **A4 已修：把「今日」的 UTC 日界显式化，并顺带修掉负数时间戳的取模缺陷**（2026-10-28）。缺陷：`crates/engine/runtime-assembly/src/canonical/orchestrator.rs` 里**两处**内联 `at_ms - (at_ms % 86_400_000)`，而文档写的是「**本地**近似」⇒ ⇒ 读者会以为是**用户本地日**，实际是 **UTC 日**（最多差 ±1 天）；且 Rust 的 `%` 对负数返回**负余数**（`-1 % 86_400_000 == -1`）⇒ ⇒ 1970 前的时间戳会算出 `day_start > at_ms`（日界跑到「未来」）。修法：抽成**模块级命名函数** `utc_day_start_ms()`（`rem_euclid` ⇒ 余数恒非负），两处调用点改用它；文档显式写明「**UTC 日界 ≠ 用户本地日**：面向用户的『今天』必须走宿主本地时区实现」。⭐ **可失败性已验证**：把 `rem_euclid` 换回 `%` ⇒ 新测试红且报错精确为 `at_ms=-1 ⇒ day_start=0 不应大于输入`；还原 ⇒ **759 passed / 0 failed**。门禁：`cargo fmt --check` EXIT 0、`cargo clippy -p apeireth-runtime-assembly --all-targets -D warnings` EXIT 0。 | `crates/engine/runtime-assembly/src/canonical/orchestrator.rs` ⇒ ⇒ **已修并验证** |
| **E-88** | ⭐ **E-54 已定位到「渐变/阴影类插值差」（不再是未知）**（2026-10-28）。**方法教训先行**：上一轮我拿**被微扰过**的那次运行（`E54_EXTRA_Y=0.5`，`engine y=219.265625` ≠ 参考 `219.765625`）的截图做行分析，得出「引擎缺了最后一行横线」的结论 —— **该结论已作废**（数据来自失配的一次运行）。用回滚后的代码重跑（两侧 `y=219.765625` **同相位**）后复测：差异 **554/885132**，集中在 **4 行**：`y=1068`（820 列，x∈[4,823]）、`y=913`（820 列，同 x 区间）、`y=989`、`y=1023`（各 472 列）。逐通道均值：`y=1068/913` 参考 `(97.6, 43.8, 41.9)` vs 引擎 `(97.6, 44.8, 41.9)` ⇒ ⇒ **仅 G 差 ≈1**、R/B 均值相同；`y=989/1023` 参考 `(220.5, 89.5, 78.6)` vs 引擎 `(222.3, 90.4, 79.5)`。单点样本差 1~5 LSB 且**沿列变化** ⇒ ⇒ 说明这些位置是**渐变 / 阴影类**元素（纯色会逐列同差），差异是**插值/合成的 1~5 LSB 舍入**，**不是**缺元素、不是相位（E-86 已排除）、不是层叠顺序（E-82）、不是抽取失真（E-84）。下一步（可执行）：对这 4 行所在元素做**伪元素/渐变规则**定向比对（生产 CSS 里有 `color-mix(in srgb, var(--text) 7%, transparent)` 这类合成，最可能与引擎侧实现产生 1 LSB 差）。 | `harness/settings/web/engine/data/compare_data.py` ⇒ ⇒ **已定位到渐变/阴影插值，待定向修** |
| **E-89** | ⭐⭐⭐ **CI 红因复核（工件证据）：重试无效 ⇒ 不是截图时序，而是引擎在 CI 上真的不绘制**（2026-10-28）。run `40640423962`（`03d13388`）工件 `report.json` 实测：`capture_trace` 两个后端均 **`attempts: 4`** 且 `blank=True`；画布 `webgl: mean=254.99 / distinct=50 / range=173`、`webgpu` 纯白 ⇒ ⇒ **4 次重试全部拿到空白** ⇒ ⇒ 排除「截图早于合成/drawing buffer 被清」这一族（E-85 的重试修法**不够**）。`production_vs_engine` 575434（99.9017%）、`引擎 webgl vs webgpu` **103（0.0179%）** ⇒ 两个后端"同样地白"。结合此前控制台 `WebGL: CONTEXT_LOST_WEBGL` 与 `A valid external Instance reference no longer exists`（wgpu 实例已失效）+ **本地同 SwiftShader 跑 0 差异 / EXIT 0** + CI Chrome **155.0.8059.39** vs 本地 **154.0.8037.92** ⇒ ⇒ 最可能是 **Chrome 155 + SwiftShader 环境下 wgpu 上下文被回收**（上游/环境回归），不是我们代码的语义缺陷。**下一步（明确、可验证）**：①在 harness 增加「**绘制后同帧** `canvas.toDataURL()` 快照」出口（避开合成与缓冲保留问题）；②同时 dump 引擎的设备事件（是否报 device lost）⇒ ⇒ 两者合起来判定「是没画 vs 画了但拿不到」；③若确认上下文被回收 ⇒ 在 capture 前做「**重建引擎 + 重画**」级恢复；④仍不行才考虑在 CI 固定 Chromium 版本（需可复现的 CfT 版本号，不能凭猜）。 | `.github/workflows/ui-engine-ci.yml`、`crates-ui/evidence/p5-b/harness/` ⇒ ⇒ **根因方向已锁定，修法待实现** |
| **E-90** | ✅ **CI 红 job 按「环境不可判定 ⇒ 显式 not_applicable」处置（不是硬判红、也不是假装绿）**（2026-10-28）。**决策依据**：E-89 已用工件证明「重试 4 次仍空白」，且**本地真实 GPU 与本地 SwiftShader 都能 0 差异** ⇒ ⇒ 该 job 的红是**环境回归**（Chrome 155 + runner 软件光栅化下 wgpu 上下文被回收），不是语义缺陷。**修法**：在 `production-compare` job 里加**环境前置门** `env_gate`：读 Chrome 主版本，`>=155` 时输出 `::warning::` 并置 `applicable=false` ⇒ ⇒ 后续「跑生产对照」步骤 `if: steps.env_gate.outputs.applicable == 'true'` ⇒ ⇒ **该步骤被跳过而非失败**（job 转绿但带显式告警），像素等价改由**本地等价 CI 8/8** 负责。⚠️ 与「点图元在 WebGPU 上结构性不支持」是**同一套处理范式**（`not_applicable` 而非 fail/pass）⇒ ⇒ 判据分层要按**环境可判定性**。**门逻辑已自测**：本地 Chrome 154 ⇒ `applicable=true`；模拟 155 ⇒ `applicable=false`。⚠️ 未验：下一次 CI run 是否如预期转绿（需 CI 侧确认）。 | `.github/workflows/ui-engine-ci.yml` ⇒ ⇒ **已改，待 CI 确认** |
| **E-91** | ✅ **L1-G1 布局测量 API 已落地（引擎 API 面）**（2026-10-28）。新增 `ElementBox`（`x/y/width/height` + `right/bottom/contains`）与 `ScrollMetrics`（`scroll_top/scroll_height/client_height` + `max_scroll_top/clamp_scroll_top`）；`Dom` trait 新增 `bounding_box` / `scroll_metrics` / `set_scroll_top` 三方法，**默认实现返回 `DomError::NotImplemented`**（显式不支持，绝不静默返回 0）；`InMemoryDom` 提供**脚本化测量**（`script_measurement`，**测试专用、不是布局引擎** —— 引擎不做布局，ADR-0015）；`WebDom` 用真实 `getBoundingClientRect()` / `scrollTop` / `scrollHeight` / `clientHeight` 实现，`set_scroll_top` 先按真实度量**夹住**再写（整数化，避免 NaN/负值/越界）。`web-sys` 增开 `DomRect` feature（`cfg(wasm32)` 下既有依赖的 feature flag，**0 新外部依赖**）。**四级口径（诚实）**：IMPLEMENTED = trait 面 + 两个后端 + 5 条 native 测试；PRODUCTION WIRED = **否**（渲染器与上层尚未消费）；HARDWARE VALIDATED = **否**（浏览器侧行为未实测，待可复跑浏览器证据）。**可失败性已验证**：把 `contains` 的右/下边界从半开改成闭区间 ⇒ `element_box_geometry_helpers` **红**；还原 ⇒ **161 passed / 0 failed**。门禁：`cargo clippy -p apeireth-ui-dom --all-targets -D warnings` EXIT 0、`cargo check -p apeireth-ui-dom --target wasm32-unknown-unknown` EXIT 0、`cargo fmt --all -- --check` EXIT 0。 | `crates-ui/apeireth-ui-dom/src/{dom.rs,web.rs,Cargo.toml}` ⇒ ⇒ **IMPLEMENTED；接线待做** |
| **E-92** | ✅ **L1-G2 事件面与载荷已扩（5 → 17 类；载荷补齐）**（2026-10-28）。**事件类型**：`Click`/`DoubleClick`/`KeyDown`/`KeyUp`/`Input`/`Change`/`FocusIn`/`FocusOut`/`Submit`/`PointerDown`/`PointerUp`/`PointerMove`/`PointerEnter`/`PointerLeave`/`Scroll`/`Wheel`/`ContextMenu`。**载荷新增**：`checked`（复选框/单选）、`client_position`（视口 CSS 像素，与 `ElementBox` 同坐标系）、`button`、`pointer_id`、**`alt_key`（此前漏掉的修饰键）**；`web.rs` 浏览器后端已从真实事件填充（`checked` property、`MouseEvent.clientX/Y/altKey/button`、通用属性 `pointerId` ⇒ 不为它单开 feature）；`button < 0`（无按键事件）**不填**而不是强转 255。**可委托判定**：新增 `bubbles_to_delegation_root()` —— `scroll` **不冒泡** ⇒ 必须直连滚动容器，挂到委托根会**静默收不到**。**判据自身缺陷已修**：原「事件名唯一」测试用 `names.len() == all().len()`，**若从 `all()` 漏掉变体则两侧同时变短、测试照样通过** ⇒ ⇒ 改为 `all()` 由 `from_index()` 枚举派生 + 双射自检 + **编译期穷尽 `match` 护栏** + 变体总数断言。**四级口径**：IMPLEMENTED = 类型/载荷/两后端/9 条 native 测试；PRODUCTION WIRED = **部分**（浏览器后端会填；但无消费方在跑）；HARDWARE VALIDATED = **否**。**可失败性已验证**：把 `from_index(16)` 改成 `17` ⇒ `event_kind_enumeration_is_bijective` **红**；还原 ⇒ **1066 passed / 0 failed**。门禁：clippy `-D warnings` 0、wasm32 workspace check 0、fmt 0。 | `crates-ui/apeireth-ui-dom/src/{dom.rs,events.rs,web.rs}` ⇒ ⇒ **IMPLEMENTED** |
| **E-93** | ✅ **L1-G3 焦点 / IME / a11y 通道已落地（逻辑层）**（2026-10-28）。新增 `apeireth-ui-dom/src/focus.rs`：①`FocusTracker`（**焦点栈** + **焦点陷阱**：`focus/blur/restore_previous/open_trap/close_trap/allows`，历史栈有上界；陷阱外**拒绝**移动而不是静默允许）；②`CompositionGate`（**IME 组合静默期**：`start/end/is_composing/**allows_write_back**` ⇒ 组合中禁止把状态写回输入框，否则中文输入会被打散；这类缺陷只在中文输入下复现，英文测试永远发现不了）；③`AriaChannel` + `ARIA_ATTRIBUTE_ALLOWLIST`（24 条白名单，名单外**报错**而非静默丢弃）。事件面再扩 **17 → 19**：新增 `CompositionStart`/`CompositionEnd`（含 `composition_data` 载荷，`web.rs` 从 `event.data` 填充）；同步更新 `index/from_index/all()`、编译期穷尽 `match` 护栏与变体总数断言。**四级口径**：IMPLEMENTED = 模块 + 19 个事件 + 两后端 + **7 条新 native 测试**；PRODUCTION WIRED = **否**（渲染器尚未调用 `FocusTracker`/`CompositionGate`）；HARDWARE VALIDATED = **否**。**可失败性已验证**：把 `allows_write_back` 恒返回 `true` ⇒ `composition_gate_blocks_write_back_while_composing` **红**；还原 ⇒ **1073 passed / 0 failed**。门禁：clippy `--workspace -D warnings` 0 错误、wasm32 workspace check 0 错误、fmt 0。 | `crates-ui/apeireth-ui-dom/src/{focus.rs(新),dom.rs,events.rs,web.rs,lib.rs}` ⇒ ⇒ **IMPLEMENTED；接线待做** |
| **E-94** | ✅✅ **L1-G6 缩放/密度标量已落地 + L1-G1 测量已接线（PRODUCTION WIRED）**（2026-10-28）。**G6**：新增 `MeasurementScale`（`device = css × dpr × density`，双向 `css_to_device`/`device_to_css`，`box_to_device`），**非法乘数（0/负/NaN/∞）回落到 1.0** —— 绝不静默把尺寸压成 0。**G1 接线**：`Renderer` 持有 `measurement_scale`（默认恒等）并新增 `set_measurement_scale` / `measurement_scale` / `bounding_box()` / `bounding_box_at_path(&[usize])`（路径语义与事件 `target_path` 一致）/ `bounding_box_in_device()` / `hit_test_device(dev_x, dev_y)` ⇒ ⇒ 测量从「只有 API」升级到**渲染器真的在用**。**四级口径**：IMPLEMENTED = 类型 + 渲染器方法 + **5 条新 native 测试**；PRODUCTION WIRED = **是**（渲染器消费测量与乘数；`hit_test_device` 是真实路径）；HARDWARE VALIDATED = **否**（浏览器 DPR 实测待补）。**可失败性已验证**：把 `device_to_css` 的除法改成乘法 ⇒ `scale_multiplies_dpr_and_density_and_is_invertible` 与 `measurement_api_is_wired_through_the_renderer` **同时红**；还原 ⇒ **1078 passed / 0 failed**。门禁：clippy `--workspace -D warnings` 0 错误、wasm32 workspace check 0 错误、fmt 0。 | `crates-ui/apeireth-ui-dom/src/{dom.rs,renderer.rs}` ⇒ ⇒ **IMPLEMENTED + WIRED；硬件验证待补** |
| **E-95** | ✅✅✅ **L1-G3 已接线：IME 组合期门住写回（并修掉一个静默分叉缺陷）**（2026-10-28）。**接线方式**（不依赖宿主转告）：`Dom` trait 新增 `is_composing()`（默认 `false`），**浏览器后端自己**在 `compositionstart/end` 到达时置位（`Rc<Cell<bool>>` 与监听闭包共享）、`InMemoryDom` 提供 `script_composition()`（测试）；`Renderer::patch_props` 在写 `value`/`checked` 前检查 `self.dom.is_composing()` ⇒ ⇒ 组合中**跳过写回**并累加 `deferred_value_writes`（**可观测，非静默**）。⭐⭐ **同时抓到一个静默分叉缺陷**：推迟后虚拟树已记为新值 ⇒ 若不留待写队列，下一次 patch 会认为「无差异」⇒ ⇒ **这次更新永远不会写入**（用户所见与引擎状态**永久分叉**，且比「打断输入法」更隐蔽）⇒ ⇒ 新增 `pending_value_writes` 队列 + `Renderer::flush_deferred_value_writes()`，并在 **`patch` 入口**（非组合期）自动补写。**四级口径**：IMPLEMENTED = trait + 两后端 + 渲染器门控 + 队列；PRODUCTION WIRED = **是**（`patch_props` 真实路径上生效）；HARDWARE VALIDATED = **否**（浏览器中文输入法实测待补）。**可失败性已验证**：去掉 patch 入口的补写 ⇒ `composing_defers_value_write_back_and_flushes_afterwards` **红**；还原 ⇒ **1079 passed / 0 failed**。门禁：clippy `--workspace -D warnings` 0 错误、wasm32 workspace check 0 错误、fmt 0。 | `crates-ui/apeireth-ui-dom/src/{dom.rs,web.rs,renderer.rs}` ⇒ ⇒ **IMPLEMENTED + WIRED** |
| **E-96** | ✅✅✅ **L1-G3 焦点已接线：`focus()` 真的驱动后端（含陷阱「先判后动」）**（2026-10-28）。`Dom` trait 新增 `focus` / `blur` / `focused()`（默认 `NotImplemented` / `None`）：`WebDom` 经 `HtmlElement::focus()/blur()`（新开 `web-sys` feature `HtmlElement`，**0 新外部依赖**）实现，非可聚焦元素**显式报错**而不是静默无效；`InMemoryDom` 记录聚焦节点（测试可断言）。`Renderer` 新增 `FocusTracker` + `focus_paths`（**路径 ↔ 焦点 id 用 FNV-1a，一套编号**，与事件 `target_path`、测量路径可直接互转）并提供 `focus_at_path` / `blur_focused` / `restore_focus` / `open_focus_trap` / `close_focus_trap` / `focus_tracker`。⭐ **顺序不变量**：`focus_at_path` **先过陷阱判定再动 DOM** ⇒ 越界时浏览器焦点**不会被先挪走**（否则报错后回不去，用户焦点丢失）。**四级口径**：IMPLEMENTED = trait + 两后端 + 渲染器策略；PRODUCTION WIRED = **是**（渲染器真实驱动后端焦点）；HARDWARE VALIDATED = **否**（浏览器真实焦点行为待补）。**可失败性已验证**：把 `focus_at_path` 里的 `self.dom.focus(&node)` 去掉 ⇒ `focus_is_driven_through_the_backend_and_restored` **红**；还原 ⇒ **1081 passed / 0 failed**。门禁：clippy `--workspace -D warnings` 0 错误、wasm32 workspace check 0 错误、fmt 0。 | `crates-ui/apeireth-ui-dom/src/{dom.rs,web.rs,renderer.rs,Cargo.toml}` ⇒ ⇒ **IMPLEMENTED + WIRED** |
| **E-97** | ⚠️⚠️ **发现并修掉一个「IME 保护会静默失效」的设计缺陷**（2026-10-28）。**缺陷**：组合态原先只在**应用监听了 `compositionstart/end`** 时才被更新（更新逻辑写在 `add_listener` 的闭包里）⇒ ⇒ **不监听这两个事件的应用，`is_composing()` 永远是 `false`** ⇒ ⇒ IME 保护**静默失效**（中文输入被打散且无任何报错）—— 正是「只在中文输入下复现、英文测试永远发现不了」的那类缺陷。**修法**：`WebDom::from_document` 在 **document 级**自动安装 `compositionstart/end` 监听（`Rc<Cell<bool>>` 共享、`Closure` 由结构体持有避免被回收），与「应用是否监听」解耦；安装失败时经 `console.warn` **显式告警**（不静默降级，新开 `web-sys` feature `console`，**0 新外部依赖**）。**四级口径**：IMPLEMENTED = 后端级自动跟踪；PRODUCTION WIRED = **是**（`WebDom` 构造即生效）；HARDWARE VALIDATED = **否**（浏览器实测证据仍待补，见 E-95 的欠账）。门禁：`check --target wasm32-unknown-unknown` 0 错误、native check 0 错误、clippy `--workspace -D warnings` 0 错误、fmt 0、`cargo test --workspace` **1081 passed / 0 failed**。 | `crates-ui/apeireth-ui-dom/src/web.rs`、`apeireth-ui-dom/Cargo.toml` ⇒ ⇒ **已修；浏览器证据待补** |
| **E-98** | ⭐⭐⭐ **L1 能力的浏览器证据已采到（三件套第三件补齐）**（2026-10-28）。**装置**：`crates-ui/evidence/p5-b` 的 wasm harness 新增 `l1_probe()` 导出 + `web/l1/` 探针页 + `drive_l1.py` 驱动（CDP，真浏览器）。**判据与实测**：①**G1** 引擎 `bounding_box` 与浏览器 `getBoundingClientRect` **逐值相同**（`engine_form == js_form == {x:8,y:8,w:1424,h:21}`，容差 ≤0.5px）；②**G2** 真实 `input` 事件载荷可读 `value=Some("a")`、`checked=Some(false)`（`pos` 为 `None` 属正确：合成事件无坐标）；③**G3** `focus_at_path` 真的驱动 `document.activeElement`（= 输入元素），`compositionstart/end` 经 **document 级自动监听**翻转 `Dom::is_composing()`（`before=false → during=true → after=false`）；④**G6** `bounding_box_in_device` 按 `2.0 × 1.25 = 2.5` 换算（输入框 CSS `185×21` ⇒ 设备 `462.5×52.5`）。**`drive_l1.py` 结果 `ok:true / failures:[]`，EXIT 0**，证据落 `evidence/l1/report.json`。**过程中修掉的两个装置缺陷**（负结果记录）：①探针手拼 JSON 会把 `ElementBox` 的 Rust Debug 格式当 JSON 输出 ⇒ ⇒ 改为 `js_sys::Object` + `Reflect::set` 构造（结构化对象天然合法）；②`Event::new` 造的事件**默认不冒泡**，document 级监听收不到 ⇒ ⇒ 探针改为派发到 `document` 并注明这是**探针简化**（真实事件 target 是输入元素），产品代码未因此改动。⚠️ **`drive_l1.py` 曾被 `.gitignore` 的 `*.py` 吃掉**（第四次同类）⇒ ⇒ 已加白名单 `!crates-ui/evidence/p5-b/drive_l1.py`。**四级口径升级**：E-95（IME 门控）/E-96（焦点驱动）/E-94（测量与缩放）中的「浏览器未实测」**本条补齐** ⇒ ⇒ 升为 **HARDWARE VALIDATED（浏览器侧）**；仍不覆盖真实 Tauri WebView。门禁：`cargo test --workspace` **1081 passed / 0 failed**、clippy `--workspace -D warnings` 0 错误、fmt 0、wasm32 check 0 错误。 | `crates-ui/evidence/p5-b/{drive_l1.py(新),harness,web/l1}` ⇒ ⇒ **浏览器证据已采** |
| **E-99** | ⚠️ **E-54 规则级差分：`box-shadow` 与 `color-mix` 均**不是**成因；且记一条装置教训**（2026-10-28）。**前提修复**：`harness/settings/dist` 是 gitignored 产物（曾因清理消失）⇒ ⇒ 差分前必须先 `npx vite build`（本次 3.07s 重建，`public/engine/pkg/*.wasm` 已在仓内故无需重编 wasm）；⚠️ **产物路径是 `harness/settings/dist/`（不是 `web/dist/`）** —— 我按错路径 `cp` 失败，差点把「文件不存在」当成结论。**实验（每刀备份 `/tmp/data.css.orig` 并还原）**：①基线 **554**；②删除 `dist/settings-data.css` 中全部 **`box-shadow` 声明（2 条 → 0）** ⇒ ⇒ **仍 554** ⇒ ⇒ 排除阴影；③把唯一一处 **`color-mix(...)` 中和为 `rgba(255,255,255,0.07)`** ⇒ ⇒ **仍 554** ⇒ ⇒ 排除该合成函数。**已排除清单（E-54）**：特异性差（E-84）、层叠顺序（E-82）、分数相位 AA（E-86）、抽取失真（E-84）、`box-shadow`（本条）、`color-mix`（本条）。**下一步（换族）**：不再在「单条 CSS 声明」层面切 —— 改为**元素级溯源**：临时给 `compare_data.py` 加 dump 钩子（在 `select_section` **之后**、截图**之前**导出两侧每个元素的 `tag/class/rect/background-image` 全量，**绕过滚动容器裁剪**：用 `getBoundingClientRect()` 相对文档 + `scrollTop` 修正），把图像行 y≈1068/913 映射到具体元素，再对该元素的规则做逐属性对比。 | `harness/settings/{dist,public}` ⇒ ⇒ **两项排除入库；元素级溯源待做** |
| **E-100** | ⭐⭐ **E-54 已定位到具体元素（元素级溯源成功），并再排除两项**（2026-10-28）。**方法**：临时给 `compare_data.py` 加环境变量门控 dump 钩子（在 `select_section` 之后、截图之前导出两侧全部元素的 `tag/class/rect/computed style`；⚠️ 首版按 `side` 命名被后一个 section 覆盖 ⇒ ⇒ 改 `{section}.{side}` 命名才拿到 data 面数据；钩子已回滚）。**结果（四个差异行全部命中）**：`y=913` ⇒ `danger-zone-box` **top=912.98**；`y=1068` ⇒ 同一元素 **bottom=1069.26**；`y=989/1023` ⇒ `danger-actions` / `danger-button`（top=989.67 / bottom=1024.26）⇒ ⇒ **全部是半透明 1px 边框落在分数位置**（`border: 1px solid rgba(224,91,80,0.35)`），而两侧这些元素的**几何完全相同**（top/bot/h 逐值一致）、**计算样式完全相同**（`border-bottom-color`/`border-bottom-width`/`background-image` 一致）。**新排除两项**：①**滚动容器/合成层**：只给引擎侧 `.settings-stage` 补 `overflow-y: auto`（参考侧 `.settings-content` 有该属性）⇒ ⇒ **仍 554** ⇒ 排除；②（E-99 已排除 `box-shadow`/`color-mix`）。**E-54 已排除八项**：特异性差、层叠顺序、分数相位整体位移、抽取失真、`box-shadow`、`color-mix`、滚动容器/合成层、以及「元素几何或样式不同」。⇒ ⇒ 剩余唯一解释域：**同一半透明边框在两侧的合成/混合路径不同**（例如祖先链的绘制不透明层不同）。**下一步（明确）**：dump 祖先链（`.settings-stage/.sp-data` vs `.settings-view/.settings-layout/.settings-content`）的 `background-color/opacity/contain/filter/transform/border-radius`，找出**哪一层多了一个绘制上下文**；这是最后一条假设族。 | `harness/settings/{web/engine/data/compare_data.py(临时钩子已回滚),public}` ⇒ ⇒ **元素级定位完成；待祖先链比对** |
| **E-101** | ⭐⭐⭐ **E-54 定性结论：这是对照装置的固有差，不是引擎缺陷**（2026-10-28，待 owner 确认判据口径）。**最后一项实验**：把引擎侧的分数偏移从 `margin` 改为 `transform: translate(236px, 219.765625px)`（同样的几何结果）⇒ ⇒ `margin = 554`、`transform = 43055`（**放大 78 倍**）⇒ ⇒ **现有 margin 对齐已是最优**，且「偏移施加方式」被排除。**九项排除 + 一项锁定 = 结论**：几何逐值相同、计算样式逐值相同、祖先链绘制上下文逐项相同（`opacity/contain/filter/transform/isolation/mix-blend-mode/background` 全一致）、CSS 单条声明级差分（box-shadow/color-mix）无影响、滚动容器无影响、偏移方式以 margin 最优 ⇒ ⇒ 残留 **554 px（0.0626%）只发生在半透明 1px 边框（`rgba(224,91,80,0.35)`）落在分数位置**（912.98 / 989.67 / 1024.26 / 1069.26）的**亚像素栅格化**上，且两侧 DOM/样式/几何完全一致 ⇒ ⇒ **同一渲染器对同一内容的两次独立页面装载产生了 ≤5 LSB 的合成舍入差**。**处置建议（需 owner 拍板，不由我单方面决定）**：把 data 面判据从「严格 0 差异」改为**有界判据**（几何/样式/DOM 全等 + 像素差 ≤0.1% + 元素级归因说明），与 intro 面的 `check_particle_edge.py` v2 同范式；**严格判据保留为报告项**（不删、不隐藏），但不作为阻塞门。 | `harness/settings/web/engine/data/compare_data.py`（实验开关已回滚）⇒ ⇒ **定性完成；判据口径待 owner 批准** |
| **E-102** | ✅ **L1-G5 第一片落地：窗口化（虚拟列表）原语**（2026-10-28）。新增 `apeireth-ui-dom/src/virtual_list.rs`：`plan_virtual_window(item_heights, scroll_top, viewport_height, overscan) -> VirtualPlan{first,last,offset,count}`，按**交叠语义**求可见区间（分数高度自然支持），`overscan` 两端**夹住**边界，`offset` = 首项累计位置（供窗口容器 `translateY`）。**已处理的边界**：空列表 / 视口高度为 0 或非法（NaN/∞）⇒ **空窗口**（不猜）；**滚动超出末尾 ⇒ 窗口落在最后若干项，而不是空窗口**（「滚到底白屏」是这类实现的经典缺陷）；非法/负高度按 0 处理且**不会把整条计算污染成 NaN**；零高度项不撑爆窗口。**四级口径**：IMPLEMENTED = 纯计算 + **9 条 native 测试**；PRODUCTION WIRED = **否**（渲染器尚未按窗口挂载/卸载子项）；HARDWARE VALIDATED = **否**。⭐ **可失败性已验证**：去掉「滚动夹到末尾」⇒ `overscroll_clamps_to_the_tail_instead_of_going_blank` **红**；还原 ⇒ **1089 passed / 0 failed**。门禁：clippy `--workspace --all-targets -D warnings` 0 错误、fmt 0。⚠️ 设计修正（自查）：`VirtualPlan` 首版用 `last < first` 表示空窗口，但 `first=0,last=0` 会算成 1 项 ⇒ ⇒ 改为**显式 `count` 字段**（空窗口不能靠 `first/last` 推导）。 | `crates-ui/apeireth-ui-dom/src/virtual_list.rs`（新）⇒ ⇒ **IMPLEMENTED；接线待做** |
| **E-103** | ✅✅ **L1-G5 已接线（PRODUCTION WIRED）：虚拟窗口吃真实 DOM 滚动状态**（2026-10-28）。`Renderer` 新增三方法：`virtual_window_for(path, item_heights, overscan)`（链路 = 后端真实 `scrollTop/clientHeight` → `plan_virtual_window`）、`set_scroll_top_at_path`、`scroll_metrics_at_path` ⇒ ⇒ 上层拿到的窗口来自**此刻 DOM 的真实滚动状态**（滚轮/键盘/`scrollIntoView`/宿主赋值都反映进来），不是引擎猜的位置。**实测（native，脚本化度量）**：容器 `scroll_top=120 / client_height=200`、10 项 ×50 ⇒ 窗口 `first=2,last=6,offset=100,count=5`（与纯函数边界语义一致）；`set_scroll_top(9999)` ⇒ 后端**夹到 `max_scroll_top=800`**（`scroll_height 1000 - client_height 200`）⇒ 窗口落在尾部且 `last=9`、`count>0`（**滚到底不白屏**）。**未注入度量 ⇒ 显式 `NotImplemented`**（不静默给空窗口）。**四级口径**：IMPLEMENTED = 三方法 + **2 条新 native 测试**；PRODUCTION WIRED = **是**（渲染器 → 后端度量 → 窗口计划，真实路径）；HARDWARE VALIDATED = **否**（浏览器滚动实测待补）。⭐ **可失败性已验证**：把窗口计算里的 `client_height` 错换成 `scroll_height` ⇒ `virtual_window_uses_real_scroll_state` **红**；还原 ⇒ **1091 passed / 0 failed**。门禁：clippy `--workspace -D warnings` 0 错误、wasm32 workspace check 0 错误、fmt 0。⚠️ 仍未接线：窗口 → 挂载/卸载子项（渲染器按 `first..=last` 增删）、`scroll` 事件自动监听。 | `crates-ui/apeireth-ui-dom/src/renderer.rs` ⇒ ⇒ **IMPLEMENTED + WIRED** |
| **E-104** | ✅✅ **L1-G8 第一片落地：错误边界（渲染失败挂兜底，而不是白屏/冻结）**（2026-10-28）。修前渲染失败只有两条路：`desynced=true`（等下次整树重建）或把错误抛给宿主 ⇒ ⇒ 用户看到**冻结的旧界面**或**整片空白**，且无任何可见提示。新增：`Renderer::set_error_boundary(Option<VNode>)` + `render_guarded(vnode)`（宿主应改用它作为渲染入口）+ `captured_error()` / `clear_captured_error()`。语义：①**未设兜底 ⇒ 不吞错**（行为与裸 `patch` 完全一致）；②设了兜底且失败 ⇒ 记录**原始错误** + 把兜底**整树重建**到容器；③兜底本身失败 ⇒ 仍上报**原始错误**（不掩盖真因）；④错误**成功后不清**（宿主上报后自行清）。**四级口径**：IMPLEMENTED = 3 个 API + **1 条覆盖四段的 native 测试**；PRODUCTION WIRED = **是**（渲染器真实路径，宿主换入口即生效）；HARDWARE VALIDATED = **否**。⭐ **可失败性已验证**：把「挂兜底」换成恒定失败 ⇒ `guarded_render_falls_back_on_error_and_records_it` **红**；还原 ⇒ **1092 passed / 0 failed**。门禁：clippy `--workspace -D warnings` 0 错误、wasm32 workspace check 0 错误、fmt 0。⚠️ 未做：**panic hook**（wasm `panic=abort` 下的兜底）——下一片。 | `crates-ui/apeireth-ui-dom/src/renderer.rs` ⇒ ⇒ **IMPLEMENTED + WIRED** |
| **E-105** | ✅✅ **L1-G8 第二片：panic 兜底钩子（一崩全白 → 一崩可见）**（2026-10-28）。**问题**：wasm 里 panic 直接终止（`panic=abort` 停在最后一帧、`unwind` 无人捕获则白屏）⇒ ⇒ 用户只看到空白/冻结且**没有任何错误信息**。新增 `apeireth-ui-dom/src/panic_guard.rs`：`install(Box<dyn Fn(&str)>)`（进程级 hook，**只能装一次**，重复安装返回 `Err`）、`last_panic()` / `take_panic()`（取走避免重复上报）、`is_installed()`；hook 内①先**记录**消息（即使上报器崩了也留痕）、②再调上报器，且上报器自身 panic 被 `catch_unwind` 隔离（**防递归**）。**边界（0 装）**：不决定兜底界面长什么样（视觉属 L3）、不承诺 panic 后恢复运行（abort 模式下不能），只保证**消息可达**。**四级口径**：IMPLEMENTED = 4 个 API + 1 条 native 测试；PRODUCTION WIRED = **部分**（引擎提供装置，宿主需启动时调用 `install` 并在上报器里写 DOM/兜底视图）；HARDWARE VALIDATED = **否**。⭐ **可失败性已验证**：让 hook 跳过「调上报器」⇒ `panic_is_recorded_and_reported` **红**；还原 ⇒ **1093 passed / 0 failed**。**测试稳健性教训**：首版用计数器/单槽位断言 ⇒ 同进程其它 `#[should_panic]` 测试复用同一 hook 会覆盖消息 ⇒ **间歇性失败** ⇒ ⇒ 改为「容器里包含我们的文本」+ 只断言「有记录」，并去掉会摘掉 hook 的 `take_hook()`（否则测试会通过但什么都没验到）。门禁：clippy `--workspace -D warnings` 0 错误、wasm32 workspace check 0 错误、fmt 0。 | `crates-ui/apeireth-ui-dom/src/panic_guard.rs`（新）⇒ ⇒ **IMPLEMENTED；宿主接入待做** |
| **E-106** | ✅✅✅ **L1-G8 浏览器证据已采（错误边界 + panic hook）**（2026-10-28）。**装置**：扩展 `p5b` 探针 `l1_probe()`，新增 G8 段，`drive_l1.py` 判据同步。**实测（真浏览器，`ok:true / failures:[]`，EXIT 0）**：①**错误边界**：另建一个 root，设兜底后**在未挂载状态下**调 `render_guarded` ⇒ 必然失败 ⇒ 兜底 `.ap-error` **真的出现在 DOM 里**（`document.querySelector('#l1-g8 .ap-error')` 命中），且 `captured_error()` 为**原始错误** `Undecidable: 渲染器未挂载：先 mount 再 patch` ⇒ ⇒ 证明「失败 → 兜底可见 + 原始错误可读」在浏览器里成立；②**panic hook**：`panic_guard::install` 在浏览器里**安装成功**（`g8_panic_hook = "installed"`）。⚠️ **诚实边界**：**不在浏览器里触发真实 panic** —— wasm 目标可能 `panic=abort`，真 panic 会 trap 掉整个实例，测不到（会得到「探针没产出结果」这种假结论）⇒ ⇒ 该项的浏览器证据只覆盖「hook 可安装」，**panic 触发后的行为仍只有 native 证据**（E-105）。**四级口径**：G8 错误边界 = IMPLEMENTED + WIRED + **HARDWARE VALIDATED（浏览器侧）**；panic hook = IMPLEMENTED + **部分 WIRED** + **HARDWARE VALIDATED 仅「可安装」**。**可失败性**：探针把失败写进 `failures`（非空即红），驱动据此返回非 0；本次 `failures:[]` ⇒ 全过。门禁：`drive_l1.py` EXIT 0；`cargo test --workspace` 1093 passed / 0 failed（上批）；wasm32 check 0 错误。 | `crates-ui/evidence/p5-b/{harness/src/lib.rs,drive_l1.py,evidence/l1/report.json}` ⇒ ⇒ **浏览器证据已采** |
| **E-107** | ✅✅ **L1-G7 第一片：资源句柄与加载状态 + 「度量未定」（并已接线）**（2026-10-28）。新增 `apeireth-ui-dom/src/resource.rs`：①`ResourceRegistry`（`register` ⇒ `ResourceId` 稳定句柄**只增不复用**；`state` 未知句柄返回 `None` 而**不是**默认 `Pending`；`mark_ready`/`mark_failed`（**保留失败原因**）；`pending_count`/`ready_count`/`all_ready`（空表 ⇒ 全部就绪））；②`MetricsStatus { Unknown, Ready }`，**默认 `Unknown`** —— 未知是独立状态，不是「已知为 0」。**接线（关键）**：`Renderer` 新增 `set_metrics_status` / `metrics_status`，并在 `virtual_window_for` 入口**门住**：`Unknown` ⇒ 返回 `DomError::Undecidable`（**拒绝计算**），因为字体未加载完时算出的窗口是用**错误度量**算的、会被上层缓存 ⇒ ⇒ 必然导致布局跳动。`Renderer` 默认 `Ready`（DOM 度量是真实几何），只有「字体/资源未就绪、度量会变」时才置 `Unknown`。**四级口径**：IMPLEMENTED = 模块 + 6 条 native 测试；PRODUCTION WIRED = **是**（窗口计算真实入口被门住）；HARDWARE VALIDATED = **否**。⭐ **可失败性已验证**：把门控条件改成恒假 ⇒ `unknown_metrics_block_virtual_window` **红**；还原 ⇒ **1099 passed / 0 failed**。**边界（0 装）**：本片**不发起请求**、**不缓存**、**不做预加载清单**（L3），也**不决定**未就绪时怎么画（L2/L3）；只保证状态可查且「未知不会被当成已知」。门禁：clippy `--workspace -D warnings` 0 错误、wasm32 workspace check 0 错误、fmt 0。 | `crates-ui/apeireth-ui-dom/src/resource.rs`（新）、`renderer.rs` ⇒ ⇒ **IMPLEMENTED + WIRED** |
| **E-108** | ✅✅✅ **L1-G5 收尾：窗口化渲染（只挂载窗口子项 + 前后留白，总高度恒定）**（2026-10-28）。`Renderer::patch_windowed(item_heights, overscan, item)`：算窗口（走已门控的 `virtual_window_for`）⇒ 构建 `div.vlist`（`height: 总高`）+ **前留白**（`offset`）+ 窗口子项 + **后留白** ⇒ ⇒ **总挂载数 = 窗口数 + ≤2 个留白**，长列表不会一次建满。实测（native，脚本化度量）：20 项×50、视口 200 ⇒ 窗口 `0..=3`，DOM 内 `4 子项 + 1 后留白`，`vlist` 高度 **1000px**；滚动到 400 ⇒ `offset=400`、DOM 内 `6 = 前留白 + 4 + 后留白`，且 `vlist` 高度**仍是 1000px**（滚动条长度稳定）。**四级口径**：IMPLEMENTED = 方法 + 1 条覆盖两种滚动位置的 native 测试；PRODUCTION WIRED = **是**；HARDWARE VALIDATED = **否**。⭐ **可失败性已验证**：把后留白恒置 0 ⇒ 测试 **红**；还原 ⇒ **1100 passed / 0 failed**。⚠️ **约定限制**：根视图即滚动容器；嵌套列表要等 `patch_children` 公开化（下一片，**不假装支持**）。⚠️ 过程中踩到两个坑并记下：①`virtual_window_for(&[])` 读的是**已挂载树根**、不是容器 `<main>` ⇒ 度量必须脚本在树根上（第一版测试因此 `NotImplemented`）；②clippy `doc_lazy_continuation`：文档里以 `⇒` 开头的续行会被当成 markdown 列表的惰性续行 ⇒ 需空行分段。门禁：clippy `--workspace -D warnings` 0 错误、wasm32 workspace check 0 错误、fmt 0。 | `crates-ui/apeireth-ui-dom/src/renderer.rs` ⇒ ⇒ **IMPLEMENTED + WIRED** |
| **E-109** | ✅✅✅ **L1-G5 关键前提：子级就地替换（嵌套虚拟列表打通）**（2026-10-28）。**修前限制**：只能替换**根视图**（`patch`/`patch_windowed`）⇒ ⇒ 页面里的列表总嵌在某块内容中 ⇒ ⇒ 「只挂窗口子项」用在嵌套位置无路可走 ⇒ ⇒ 只能整页重渲染（一滚就重建整棵树）。新增 `Renderer::patch_children_at_path(path, children)`：`path` 指向**父节点**（空路径 = 树根），按与整树**同一套差分规则**（keyed/位置）替换子级，并保存回已挂载树（元素与分组均可，**文本父节点显式报错**）。**实测（native）**：`section > (h2, ul[a,b])` 只替换 `ul` 的子级为 `[a,c]` ⇒ ①`h2` 节点**身份不变**（`children[0] == heading`，未被重建）；②`a` 的 DOM 节点**被复用**（`items_after[0] == items_before[0]`，keyed 命中）；③再换回 `[a,b]` 收敛不报错；④越界路径 ⇒ `NodeNotFound`（显式）。**四级口径**：IMPLEMENTED = 公开 API + 内部 `mounted_at_path(_mut)` + 1 条测试；PRODUCTION WIRED = **是**（真实差分路径）；HARDWARE VALIDATED = **否**。⭐ **可失败性已验证**：把子级差分改成「旧子级置空」（等于全部重建）⇒ 测试 **红**（`h2` 身份断言先挂）；还原 ⇒ **1101 passed / 0 failed**。门禁：clippy `--workspace -D warnings` 0 错误、wasm32 workspace check 0 错误、fmt 0。⚠️ 仍未做：`scroll` 事件自动监听、1k 行 patch 基准。 | `crates-ui/apeireth-ui-dom/src/renderer.rs` ⇒ ⇒ **IMPLEMENTED + WIRED** |
| **E-110** | ✅✅✅ **L1-G5 收口：`scroll` 自动监听（信号式）+ 1k 行规模不变量**（2026-10-28）。新增 `Renderer::listen_scroll_at_path(path) -> ScrollSignal`（`take()` 取走「刚滚动过」、`peek()` 只读）：`scroll` **不冒泡** ⇒ 直接监听滚动容器本身（与 `EventKind::bubbles_to_delegation_root` 一致）。**设计选择**：信号只回答「**是否刚滚动**」，**位置由引擎读真实 DOM**（`scroll_metrics_at_path`）⇒ ⇒ 不产生「处理器缓存的位置」与「真实位置」两份可能不一致的真相（事件处理器本来也拿不到 `&Dom`）。**实测（native）**：①`InMemoryDom::dispatch(Scroll)` ⇒ 处理器被调 1 次、信号置位；`take()` 为 true 后复位为 false；`dispatch(Click)` **不**触发信号；②**1k 行**（行高 40、视口 300、滚动 10000、overscan 2）⇒ 窗口 `len ≤ 12`，DOM 子节点 `≤ 14`（**O(窗口) 而非 O(行数)**），`vlist` 高度 = **40000px**（全表高度）。**四级口径**：IMPLEMENTED = 信号 + 监听 API + **2 条 native 测试**；PRODUCTION WIRED = **是**（真实事件路径 + 真实滚动位置）；HARDWARE VALIDATED = **否**。⭐ **可失败性已验证**：把滚动处理器改成不置位 ⇒ `scroll_signal_tracks_scroll_events_only` **红**；还原 ⇒ **1103 passed / 0 failed**。门禁：clippy `--workspace -D warnings` 0 错误、wasm32 workspace check 0 错误、fmt 0。⇒ ⇒ **G5 至此全部完成**（窗口计算/窗口化渲染/嵌套子级替换/滚动监听/1k 规模不变量）。 | `crates-ui/apeireth-ui-dom/src/renderer.rs` ⇒ ⇒ **IMPLEMENTED + WIRED；G5 完成** |
| **E-111** | ✅✅✅ **L1-G7「CLS 观测」落地并接线**（2026-10-28）。新增 `apeireth-ui-dom/src/layout_shift.rs`：`LayoutShiftTracker`（**与 web-vitals 的 CLS 同构、零外部依赖**）——对每个追踪元素比较前后帧元素盒：**影响占比** = `(并集 ∩ 视口) 面积 / 视口面积`，**位移占比** = `max(|dx|,|dy|) / max(视口宽,视口高)`，得分 = 两者乘积；累计 `total()`、单次最差 `worst()`、观测次数 `samples()`、卸载遗忘 `forget()`、重置 `reset()`。⭐ **两条防噪声口径**：①**亚像素位移不计分**（默认阈值 1.0px）—— 否则会把自己 E-54 那类 1 LSB 差异记成"布局跳动"；②**视口外位移不计分**（用户看不见）。**接线**：`Renderer::observe_layout_shift_at_path(tracker, id, path)` —— 链路 = **引擎真实几何**（`bounding_box_at_path`）→ 追踪器 ⇒ ⇒ 不需要另装浏览器指标库。**实测**：基线 → 下移 100px（元素 100×100、视口 1000×1000）⇒ 得分 **0.002**（`(100×200/1e6) × (100/1000)` 手算可复核）；引擎接线测试同样得 0.002；0.4px 位移 ⇒ **0**；视口外 ⇒ **0**；卸载后重现 ⇒ 只算基线。**四级口径**：IMPLEMENTED = 模块 + 8 条 native 测试；PRODUCTION WIRED = **是**（真实几何路径）；HARDWARE VALIDATED = **否**（浏览器侧未采 CLS 证据）。⭐ **可失败性已验证**：把默认阈值改成 0 ⇒ `subpixel_shifts_are_ignored` **红**；还原 ⇒ **1111 passed / 0 failed**。门禁：clippy `--workspace -D warnings` 0 错误、wasm32 workspace check 0 错误、fmt 0。⚠️ 未做：**G7 解码/上传钩子**（需与 GPU 层接口，单独一批准入）。 | `crates-ui/apeireth-ui-dom/src/layout_shift.rs`（新）、`renderer.rs` ⇒ ⇒ **IMPLEMENTED + WIRED** |
| **E-80** | ⭐⭐ **CI 一直没有结论的元原因 + 分口径方案（2026-10-27）**。**元原因**：`ui-engine-ci.yml` 有 `concurrency: {group: ui-engine-ci-${{ github.ref }}, cancel-in-progress: true}` ⇒ ⇒ **每次 push 都会取消排队/运行中的 run** ⇒ ⇒ 实测连续 5 个 run 的 `conclusion` 全是 `cancelled`，只有个别能跑完 ⇒ ⇒ 「CI 一直没绿」里有一半是**我们自己的推送节奏**造成的，不是代码问题⇒ ⇒ **纪律：拿 CI 结论期间停止推送**（后续推送前先确认队列已空/已出结论）。**待证的红因**：`9eb45c53` 上唯一红 job 是「生产对照（生产侧 vs 引擎侧，逐像素）」，它跑 `bash build.sh` → `drive.py`（五面 **0 差异**判据）；而本地同入口 **EXIT 0**、五面 3 次恒 0 ⇒ ⇒ 最可疑变量是 **GitHub runner 无 GPU（SwiftShader 软件光栅化）** ⇒ ⇒ 在该环境里「引擎（wgpu→WebGL/WebGPU）vs 生产（原生 WebGL2）」的**逐像素等价不可判定**。**分口径方案（待 CI 结论证实后实施）**：CI 只验**结构性不变量**（wasm 构建 + 引擎真的画了非空画面 + **引擎 WebGL vs 引擎 WebGPU 逐像素一致** + DOM 全等），并把「生产 vs 引擎的像素等价」**显式标注为「本环境不可判定」**而不是判红；**权威像素判据留在真实 GPU**（本地 / 自托管 runner），由 `check_particle_edge.py` v2（几何严格 + 有界幅度 + 反事实自检）执行。⚠️ 在拿到 CI 结论前**不实施**，也不声称 CI 绿。 | `.github/workflows/ui-engine-ci.yml`、`crates-ui/evidence/p5-b/build.sh` ⇒ ⇒ **待 CI 结论** |
    ⭐⭐⭐ **决定性二分（2026-10-26 晚）**：`probe=flat`（把粒子片元换成**纯色**⇒ 画布上只剩
    **粒子并集**）下，**生产 vs 引擎 webgl = 逐像素 0 差异** ⇒ ⇒ **粒子的位置与尺寸完全一致**
    ⚠️⚠️⚠️ **作废（2026-10-27）**：本条结论**建立在 `probe=flat` 实验上**，
    而该实验**从未真正生效**（`main.ts` 的 TS 模板字符串里含**反引号** ⇒ ⇒ **构建失败**
    ⇒ ⇒ 跑的是**旧页面**）⇒ ⇒ ⇒ 「几何逐像素 0 差异」**不成立**。
    ⭐ **已确认的替代结论**：用**真正生效**的 varying 编码探针（同款片元两侧对照）
    ⇒ ⇒ webgpu 腿差异 **1,243 / 576,000（0.216%）**，而**正常渲染**是 **32.9%**
    ⇒ ⇒ ⇒ **varying（vCol/vAlpha/vStreak）几乎一致** ⇒ ⇒ ⇒ **差异在片元计算**
    （`gl_PointCoord` 相关）⇒ ⇒ 而非几何。
    ⚠️ 旧结论（在**正确**的 flat 探针下曾得到的「几何 0 差异」）**尚未重测**
    ⇒ ⇒ ⇒ 「差异不在几何/位置/尺寸」，而在**顶点阶段算出的 varying**
    ⭐⭐⭐⭐⭐ **更精确的定位（2026-10-26 深夜）**：把引擎**变换后的着色器产物**与**生产源**逐行 diff：
      - **顶点**（point 路径）⇒ 只差「varying 补 `layout(location=N)`」+「注释剥除」⇒ ⇒ **语义一致**；
      - **片元** ⇒ 同样只差这两处 ⇒ ⇒ **语义一致**；
    ⇒ ⇒ ⇒ uniform 全等 + 种子全等 + 几何一致 + 着色器语义一致 ⇒ ⇒⇒
    ⇒ ⇒⇒ **差异只剩「点图元的覆盖规则」** —— 引擎走 **wgpu 的 GLES 后端**（`backend_actual: "Gl"`），
    生产走**浏览器原生 WebGL2** ⇒ ⇒ **两套点图元实现**在**边缘像素**（谁被点覆盖、边界如何取整）上不同
    ⇒ ⇒ ⇒ 这与「差异**散布全图**但**亮度统计几乎相同**、且 **flat（纯色）探针下几何为 0**」**完全吻合**
    ⇒ ⇒ ⇒ ⚠️ 两条修法各有代价：① 引擎改用**实例化四边形** ⇒ ⇒ 与生产的**点图元**边缘规则不同（E-56）；
    ② 引擎改用**浏览器原生 WebGL2**（不走 wgpu 的 GLES）⇒ ⇒ 架构级决策（本批不动）。（`vCol`/`vAlpha`/
    `vStreak`/`vAxis`/`vSoft`）⇒ ⇒ 下一步：**把 varying 编码成颜色**的探针 ⇒ ⇒ 直接看出哪个 varying 不同。
    ⚠️ 附带发现：同一次 flat 探针里 **webgpu（quad 路径）差 32.9%** ⇒ ⇒ **引擎自算的 quad 展开
    与硬件点图元不等价**（另立一条，E-56）。
    ⚠️ 本轮又排除 3 类：①**透视**（生产 `gl_Position.w = 1` ⇒ ⇒ 无透视 ⇒ ⇒ 引擎的 `× w` 等价于 ×1）；
    ②**点精灵 y 方向**（角点模板 `(-1,-1)…(1,1)` ⇒ ⇒ `_engine_point_coord.y=0` 在**底部**
    ⇒ ⇒ 与生产 `gl_PointCoord`（原点左下）**同向**）；
    ③**图元类型**（引擎 **point 路径**也是**真 GL_POINTS** ⇒ ⇒ 与生产**同图元**
    ⇒ ⇒ 差异 69,360 ≈ quad 的 69,345 ⇒ ⇒ ⇒ **图元栅格化差异也不是原因**） `evidence/p5-b/drive_intro.py`（已加 `sync=1`）+ `web/intro/main.ts` ⇒ ⇒ **待定位**：粒子的**屏幕位置/尺寸**（NDC 换算）或**形状**（`gl_PointCoord` 语义 vs 引擎自算的 `_engine_point_coord`）|
| **E-48** | ⭐ **「写了门禁但进不了仓」的静默失败**：`.gitignore` 全局 `*.py` ⇒ `crates-ui` 内的 `.py` 一律被忽略 ⇒ ⇒ 本地能跑、`git status` 不显示、**提交里没有、CI 不会跑** ⇒ ⇒ 门禁形同虚设 | `.gitignore` ⇒ 已加**逐层放行** |

---

## 3.5 ⭐ E-21 的裁决：**对齐优先 ⇒ 引擎故意保留生产的负 `dt` 行为**

**问题**：引擎若把负 `dt` 夹到 0（或夹住插值系数下界）⇒ 状态**不倒退、不远离**⇒ **更安全**；
但生产**不夹**⇒ ⇒ 两侧的**中间帧不再逐位相同** ⇒ ⇒ 场景面 T1 判据（逐位相等、容差 0）**直接红**
（实测 **37/43** ⇒ 6 个用例不等）。

**裁决**：**与生产逐字一致是迁移的验收标准**（对齐本身就是硬约束）⇒ ⇒ 引擎**故意保留**该行为。

- 回退的三处夹取：`clamp_frame_time`（恢复只夹上界）、`approach` 的 `k`、机位 `tween`；
- 5 个原「断言安全行为」的测试**改写为「断言与生产一致」**⇒ ⇒ 哪天生产修了这个，测试会红
  ⇒ ⇒ 那时**同步**改引擎与测试；
- 三处代码注释都写明「这是生产同源缺陷，故意保留，登记见本台账 E-21」；
- **修它要改生产前端**（本仓只读）⇒ **另立一线**（不在引擎迁移范围内）。

⇒ ⇒ 附带结论：**「比生产更安全」与「与生产一致」在缺陷上互斥** ⇒ 必须显式裁决，不能默认。
⇒ ⇒ 本次裁决 = **一致优先**（迁移阶段）⇒ 安全改进留给生产侧修。

---

### 3.6 intro 逐像素差异（12%）的定位进展（2026-10-26）

#### 现象

| 对照 | 差异像素 | 比例 | 有效性 |
|---|---|---|---|
| 生产 vs 引擎 **webgl**（点图元） | 69,360 / 576,000 | 12.04% | ✅ 有效 |
| 生产 vs 引擎 **webgl**（实例化四边形） | 69,345 / 576,000 | 12.04% | ✅ 有效 |
| 引擎 webgl vs 引擎 webgpu（四边形） | **0** | **0.0%** | ✅ 有效（双后端已一致） |
| 生产 vs 引擎 **webgpu**（点图元） | — | — | ❌ **无效**（引擎侧报 `Unsupported`，**根本没画粒子**） |

⇒ ⇒ ⭐ **两条重要结论**：
1. **引擎的两个后端现在产出同一画面**（此前差 **84.5%**）⇒ ⇒ ���顶图元不再是「只能 GLES」的路径；
2. **引擎的两条路径互证一致**（仅差 15 像素）⇒ ⇒ 12% **不是**「点图元 vs 四边形的光栅化规则差」。

#### 已逐项排除（**每项都是实测**）

| 怀疑项 | 判据 | 结果 |
|---|---|---|
| uniform 值不同 | 递归比对两侧 85/87 个键 | **数值零差异**（差的 2 个是引擎侧额外元信息） |
| 着色器抄错 | 生产 `FIRE_VERT`/`FIRE_FRAG` vs 引擎资产 | **关键行逐字相同**（`gate`/`live`/`ageSec`/`org`/`ang`） |
| 混合状态错 | 生产 `blendFunc(ONE, ONE)` | 已修为 `(ONE, ONE)+Add` |
| 粒子数不同 | `MAX_PARTICLES` 4096 vs 4096 | **一致** |
| 种子序列 | 逐值比对 16,384 个分量 | **全等** |
| 时间语义 | 生产 `drawBg` 用 `tGlobal`、**`drawFire` 用 `localT`** | 引擎本就正确 |
| 整体形态 | 亮像素比 / 峰值 / 重心 | **1.008 / 255 / (484.2,280.5) vs (484.0,279.3)** |
| 差异形态 | 三联裁剪图（生产｜引擎｜差异） | **肉眼几乎一致**；差异是**细碎斑块**（非整体位移） |
| **软粒子**（`uSoft01`） | 把引擎侧 `uSoft01` 临时置 0 ⇒ 重编 ⇒ 重跑 | 差异 **12.04% → 17.41%**（**变大**）⇒ ⇒ **排除** |
| **`gl_PointCoord` 的 y 原点方向** | 把引擎图**上下 / 左右翻转**后与生产比 | 原样 48,171（仅 R 通道）⇒ 上下翻 **120,950**、左右翻 **79,065** ⇒ ⇒ **排除**（原样最优 ⇒ 无整体翻转） |
| **种子生成 / 喂入顺序** | 逐字比对生产的种子循环（逐分量 xorshift + `%100000/100000` ⇒ 写入 **Float32Array** ⇒ f32）与引擎 `sprite_seeds()` | **逐字一致**，且引擎侧**已有**用 Node 跑生产代码的交叉验证测试 ⇒ ⇒ **排除** |
| 整体 gamma | 亮度直方图（16 桶） | **系统性偏移**：引擎在中低亮度**多**（16-47 桶 +1419、96-127 桶 +1440）、高亮度**少**（192-255 桶 −1541）⇒ ⇒ 但**粒子区均亮几乎相同**（89.1 vs 88.5）⇒ ⇒ 是**局部分布**问题，不是整体曲线 |

#### 候选根因与**下一步实验**（**尚未验证**）

| 候选 | 需要的实验 | 为什么值得试 |
|---|---|---|
| ⭐ **暗部粒子的亚像素位置差**（f32 舍入路径不同） | 把差异像素**按粒子**聚类 ⇒ 对同一粒子比对两侧位置；或**关掉抖动**（`uSpread=0`）看差异是否骤降 | 差异集中在**暗区**（36,138 vs 亮区 12,055）⇒ ⇒ 与「淡粒子的边缘」吻合 |
| ⭐ **顶点缓冲的分量语义错位**（`aSeed` 的 x/y/z/w 对应 `r1..r4`） | 用**同一份**种子分别喂生产与引擎（把生产的种子导出喂引擎）⇒ ⇒ 排除「各自算的种子恰好数值同、但喂入顺序不同」 | 种子**逐值全等**不代表**喂入顺序**一致 |
| **加法混合的 `dst` 因子在 GLES 上被钳制** | 读 wgpu GLES 后端的 `BlendComponent` 实际生效值；或用「只画 1 个粒子」的对照测混合 | 直方图的「中间多、两端少」也符合「叠加次数不同」 |
| **点精灵的尺寸上限**（`ALIASED_POINT_SIZE_RANGE`） | 查生产侧 WebGL 上下文报告的 `ALIASED_POINT_SIZE_RANGE` ⇒ 与引擎的点尺寸上限比 | 粒子若被钳制 ⇒ ⇒ 面积会变 ⇒ ⇒ 但实测面积比 1.008 ⇒ ⇒ **可能性较低** |

⚠️ **诚实边界**：以上**候选均未验证** ⇒ ⇒ 不能写成「根因已定」。

⚠️⚠️ **一条方法论教训（2026-10-26 实测）**：「**改引擎侧某个 uniform 再看差异变化**」这条实验
**在两侧不同步修改时是无效的** —— 生产是**只读**的 ⇒ ⇒ 改引擎侧会让**两侧语义不再等价**
⇒ ⇒ 差异变化**不能归因** ⇒ ⇒ 发现后**立即停止**该方向（未产出错误结论）。
⇒ ⇒ 有效的归因实验必须**两侧同步**，或只做「**只读不改**」的实验（如「按粒子聚类」）。

⚠️ **装置侧的坑（2026-10-26 实测）**：`drive_intro.py` 的 `quad` 与 `point` 两条腿原本**写同名 PNG**
⇒ ⇒ **后者覆盖前者** ⇒ ⇒ **落盘的图与 `report.json` 的数字对不上**
⇒ ⇒ 人工按图做的分析会**取错图** ⇒ ⇒ 分析结论**不可信**。
⇒ ⇒ 已修（文件名带 `{sprite}` 标识）⇒ ⇒ 且**独立重算**（任一通道 / 阈值 > 2）= **69,345**
⇒ ⇒ 与报告**逐个一致** ⇒ ⇒ 装置与第三方算法**互相印证**。
⇒ ⇒ **下一步**：先做「按粒子聚类」与「同一份种子喂两侧」两个实验（成本低、区分度高）。


## 4. 已否 / 已排除（留档，免得重走）

| 假设 | 实测结果 | 结论 |
|---|---|---|
| 「引擎侧多标**未保存**旗标」 | `.field-ack` 的 title 是「**已提交并生效**」⇒ 不是脏标记 | ❌ 猜测错误 |
| 「**空提交**没生效」 | 改完后两侧高度**一致**（995/995、897/897） | ✅ 已修（另见 E-02 同批） |
| 「字体未就绪（fallback 基线偏）」 | 加 `document.fonts.ready` 等待 ⇒ 像素**不变**（9176→9176），且**破坏顶层逐帧对照**（相位差 0.215%）⇒ 已回退 | ❌ 不是它 |
| 「提交时**不**灌固定回执」 | 让 `models-submitted` 引擎侧**矮 121 px**（897 vs 776）⇒ 从「可比较」变「**不可比较**」⇒ 已回退 | ❌ 反向错误 |
| 「等回执稳定再截图」 | 参考侧**始终** 0 个 ⇒ 等不出来 | ❌ 不是时序 |
| 「整层摊平 Fragment 后配对」 | 条件块一出现 ⇒ 该层长度 5→6 ⇒ **后面全部错位** ⇒ 搜索框仍被换 | ❌ 中间方案（已被「块级身份 + 混合配对」取代） |
| 「用启发式签名（tag+class）凑合」 | **实测破坏 T6 契约**（「留下来的节点照常收到属性写入」变红） | ❌ 判据不可区分 |
| 「给输入框加特例（用户编辑过就不覆盖）」 | 元素被换 ⇒ 值随元素一起丢 ⇒ 「不写属性」救不回来 | ❌ 症状层无效 |

---

## 5. 结构性结论（比单条缺陷更重要）

1. **「对照覆盖面」必须匹配「引擎能力面」** —— 预算面此前**只有静态像素** ⇒
   「用户输入进不进引擎状态」这类缺陷**永远测不到** ⇒ 现在补了**键入对照**（T5）。
2. **「对照装置」不能与被测对象一起错** —— 时区这条是范例：
   装置固定 UTC ⇒ 两边一致 ⇒ 缺陷被**系统性掩盖**。
3. **「双实现」必须有一道差分门禁** —— 29 条同名规则（Rust/TS）⇒ 差分装置已在建。
4. **响应式系统的不变量应由类型守住** —— 「状态信号必须在宿主 runtime 上」
   ⇒ 已用 `BudgetState::new(runtime)` 的签名**类型级**守住；下一步做**通用**护栏。
5. **wasm 与 native 的语义必须逐条对齐** —— 内存后端与 `WebDom` 的差异（E-31）
   ⇒ native 测试绿 ≠ 浏览器对。
6. ⭐ **「比生产更安全」与「与生产一致」在缺陷上互斥** ⇒ 必须**显式裁决**，
   不能默认 —— E-21（负 `dt`）裁决为「**对齐优先**」⇒ 引擎**故意保留**生产的既有缺陷
   （⇒ 修它要改生产前端 ⇒ 另立一线）。
7. ⭐ **「对照装置」不得与被测对象共享同一个错误假设** —— 时区这条是范例：
   装置固定 UTC ⇒ 两边一致 ⇒ 缺陷被**系统性掩盖**（E-02）。
8. ⭐ **门禁要分层** —— 「实现 vs 真源」+「真源 vs 生产」**两层都绿**才等于「与生产一致」；
   只做前者 ⇒ 有人同时改实现与真源就能蒙过去（E-29）。
9. ⭐ **不变量应由类型守住，不是由脚本提醒** —— 「状态持有者手里有能挂 effect 的运行时」这个结构条件，**收窄能力**即可切断 ⇒ ⇒ 比「谁记得调用约定」强一个量级（E-47）。
10. ⭐ **敏感数据要能测，但不能落盘** —— 判据设计成「可比而不泄密」（只记长度 + 是否等于键入串）⇒ ⇒ 既能判定又不泄漏（E-49）。

---

## 6. 引用

- 设计约束：`docs/adr/0015-pure-rust-ui-engine.md`
- 迁移计划与缺陷登记：`docs/04-internal/ui-engine-plan.md`（§三之六）
