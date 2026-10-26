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
| **E-30** | `expiresAtMs: null` ⇒ TS 判过期、**Rust 不判** | 实测：TS `null` ⇒ `true`；Rust `None` ⇒ `false` | 待修（**契约边界外**：后端 `skip_serializing_if` ⇒ 字段省略而非 null） |
| **E-31** | wasm 与内存 DOM **语义不一致**（class 全删 / 子树摘除标记范围 / `get_property` 实时值 / `target_path` 起点与下标） | 实测 native `None` vs 浏览器 `Some("")` | 在修 |
| **E-32** | `patch_props`：property 消失 ⇒ **整帧中止** | 实测 `Err(Unsupported("移除 property `value` 未支持"))` | 在修 |
| **E-33** | `is_dirty()` 状态**撒谎**（跑完后无条件清脏） | 实测自激 effect 跑完后 `is_dirty()==false`（应 true） | 在修 |

---

## 3. P2 —— 文档不实 / 诊断粒度（0 装纪律）

| # | 缺陷 | 位置 |
|---|---|---|
| **E-34** | `ui-dom` 文档仍写「事件委托未接线 / `Dom` 没有监听器接口」⇒ **与实况脱节** | `ui-dom/src/lib.rs:56-68`、`renderer.rs:203-209` |
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
| **E-47** | ⭐ **响应式系统缺一条不变量守卫**：「状态对象的信号必须建在**宿主给的** runtime 上」⇒ 该缺陷**类型层面完全合法** ⇒ ⇒ 编译器**不可能**发现 | `harness/scripts/check_runtime_ownership.py` ⇒ **已加**（零误报的近似判据 + 红-绿实测） |
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

---

## 6. 引用

- 设计约束：`docs/adr/0015-pure-rust-ui-engine.md`
- 迁移计划与缺陷登记：`docs/04-internal/ui-engine-plan.md`（§三之六）
