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
