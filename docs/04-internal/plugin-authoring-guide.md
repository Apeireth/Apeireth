# Apeireth 社区插件开发规范（plugin-authoring-guide, 2026-08-16）

> **给谁看**: 社区开发者 + 官方套件作者。对应 team-work-doc §5.6（官方交付文档）。
> **定位**: 三层交付模型（team-work-doc §1.3）中**插件层**的开发规范——官方交付整合过的整件，细小特殊需求给社区。
> **读法**: 先读 §1（最小可运行插件）→ §2（授权规则，决定你的工具能不能被调）→ §3/§4（测试与卸载，决定能不能合入）→ §5（数据源模板）→ §6（发布检查单）。
> **0 假装红线**: 本文档所有示例均**摘自真实代码**（每节标注来源文件）；未实装的机制如实标注「未接」，不虚构 API。

---

## 0. 插件在三层层级中的位置

| 层 | 谁开发 | 改动方式 | 示例 |
|---|---|---|---|
| 模块（lib 核心） | 官方 | 整体大改，编译期强绑定 | companion lib（记忆/反思/工具桥） |
| 套件（suite） | 官方 | 拼积木：套件 = 插件组 + 权限包 + 校验 | 教育/渗透/预测机套件 |
| **插件（plugin）** | **社区为主** | **最小单元热插拔** | 翻译器、体育预测数据源、dx 检查 |

- **插件 = 最小贡献单元**：工具注册 + 权限预设 + 生命周期（来源：`crates/apeireth-companion/src/plugin.rs:1-11` 模块头）。
- **套件 = 插件组的官方打包**：`SuiteDef.plugins` 声明组成插件，装配前校验插件已装（来源：`crates/apeireth-companion/src/suites.rs:38-53`）。
- **当前插件载体（0 假装）**：Rust 编译期单元。社区插件 = 向 `apeireth-companion` crate 提交新模块（`src/<your_plugin>.rs` + `lib.rs` 注册 `pub mod`）的 PR。**动态加载（运行时装载外部二进制插件）未接**——`apeireth-tool-registry` 的 `watch_plugin_dir` 目前只记录文件事件到日志（来源：`crates/apeireth-tool-registry/src/registry.rs:75-77` 注释），不是插件装载器。

---

## 1. Plugin trait 用法 + ToolBridge 注册示例

### 1.1 Plugin trait（5 方法）

来源：`crates/apeireth-companion/src/plugin.rs:18-26`（原文摘录）

```rust
/// 插件: 最小贡献单元.
pub trait Plugin: Send + Sync {
    fn id(&self) -> &str;
    fn version(&self) -> &str;
    fn description(&self) -> &str;
    /// 加载: 注册工具 + 授权. 失败 → 安装拒绝 (0 装 PASS).
    fn on_load(&self, bridge: &ToolBridge) -> Result<(), String>;
    /// 卸载: 清理 (权限撤销等). 幂等.
    fn on_unload(&self, bridge: &ToolBridge) -> Result<(), String>;
}
```

生命周期语义（来源：`plugin.rs:44-89`）：

- `PluginRegistry::install` 调 `on_load`，**成功才登记；失败不登记**（0 装 PASS：不假装装上了）。
- `PluginRegistry::install_all`（套件装配用）：**全部成功才整体生效，任一失败回滚已装的**。
- `uninstall` 调 `on_unload`，要求**幂等**。

### 1.2 Tool trait（4 方法）

来源：`crates/apeireth-tool-registry/src/trait_def.rs:27-45`（原文摘录）

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    /// 工具唯一名 (e.g. `"FileOperator"`, `"WebSearch"`, `"DailyNoteWrite"`)
    fn name(&self) -> &str;

    /// 6 类 enum (VCP `pluginType` 字段级)
    fn kind(&self) -> ToolKind;

    /// 5 轴正交属性
    fn axes(&self) -> ToolAxes;

    /// 异步执行入口
    async fn call(&self, args: Value) -> Result<Value, String>;
}
```

`ToolKind` 6 类（来源：`crates/apeireth-tool-registry/src/types.rs:42-66`）：
`Sync` / `Async` / `Static` / `Service` / `MessagePreprocessor` / `Hybridservice`。

`ToolAxes` 5 轴（来源：`types.rs` 各轴 enum + `tool_axes_default` 测试 lines 433-442）：

| 轴 | 取值 | 默认 |
|---|---|---|
| trigger（触发） | OnDemand / Periodic / EventDriven | OnDemand |
| awaiting（等待） | Immediate / Deferred / Streaming | Immediate |
| resident（驻留） | Ephemeral / Cached / Persistent | Ephemeral |
| transport（传输） | Local / Ipc / Network | Local |
| output（输出） | Value / Stream / SideEffect | Value |

纯规则层工具用 `ToolAxes::default()`（Local）；要发网络请求的工具把 `transport` 改成 `Network`（真实示例见 §1.4 与 `crates/apeireth-tools/src/github_accel.rs:226-234`）。

### 1.3 完整示例：EducationDxPlugin（真代码摘录）

来源：`crates/apeireth-companion/src/education.rs:152-206`（原文摘录；`analyze`/`to_json` 内部规则逻辑见源文件）

```rust
#[async_trait::async_trait]
impl Tool for DxCheckTool {
    fn name(&self) -> &str {
        "dx_check"
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Sync
    }
    fn axes(&self) -> ToolAxes {
        ToolAxes::default()
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        let problem = args.get("problem").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let substitution = args.get("substitution").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let after = args.get("after").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if problem.trim().is_empty() && after.trim().is_empty() {
            return Err("需要 problem (原题) 和/或 after (换元后的式子)".to_string());
        }
        Ok(Self::to_json(&Self::analyze(&problem, &substitution, &after)))
    }
}

/// 教育套件插件: 注册 dx_check 工具 + 授权日常调用.
/// 装配路径: PluginRegistry.install → on_load → ToolBridge.registry.register + packs.grant.
pub struct EducationDxPlugin;

impl Plugin for EducationDxPlugin {
    fn id(&self) -> &str {
        "education-dx-check"
    }
    fn version(&self) -> &str {
        "0.1.0"
    }
    fn description(&self) -> &str {
        "换元法 dx 检查: 忘换 dx / 混用 / 缺微分 / 残留 x / 根号模式提示 (规则层)"
    }
    fn on_load(&self, bridge: &ToolBridge) -> Result<(), String> {
        bridge.registry.register("dx_check".to_string(), Arc::new(DxCheckTool));
        bridge.packs.grant(crate::packs::PermissionPack::permanent(
            "教育插件授权",
            vec!["dx_check".to_string()],
        ));
        Ok(())
    }
    fn on_unload(&self, bridge: &ToolBridge) -> Result<(), String> {
        // 真清理: 注销工具 + 撤销授权 (幂等; 卸载后 dx_check 不可再调)
        bridge.registry.unregister("dx_check");
        bridge.packs.revoke_by_name("教育插件授权");
        Ok(())
    }
}
```

`on_load` 的两步是插件的标准动作：**注册工具**（`bridge.registry.register`）+ **授权**（`bridge.packs.grant`）。第二个真实例子 `GhAccelPlugin` 包装了 `apeireth-tools` 里已有的工具实现（来源：`crates/apeireth-companion/src/gh_accel.rs:20-45`）——社区插件也可以只装配已有工具，不重写逻辑。

### 1.4 安装/运行入口（真代码摘录）

来源：`crates/apeireth-companion/examples/education_suite_demo.rs:19-48`（原文摘录）

```rust
let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
let bridge = Arc::new(ToolBridge::new(store));
let plugins = PluginRegistry::new();

// 1) 先装插件 (生态最小单元)
plugins.install(&bridge, Arc::new(EducationDxPlugin)).unwrap();

// 2) 套件装配 (校验: 插件已装 + 工具已注册 + 授权)
let cat = SuiteCatalog::builtin();
let r = cat.install_with_plugins(&bridge, Some(&plugins), "education-suite").unwrap();

// 3) 真工具调用
let bad = ParsedToolCall {
    tool_name: "dx_check".into(),
    args: json!({
        "problem": "∫ x·√(1-x²) dx",
        "substitution": "令 t = 1-x²",
        "after": "∫ x·√(t) dx"
    }),
    raw_marker: String::new(),
    archery: false,
    archery_no_reply: false,
};
let r = bridge.execute_if_allowed(&bad).await;
```

`ToolBridge` 公开的插件装配面（来源：`crates/apeireth-companion/src/tool_bridge.rs:358-376`）：`registry`（工具注册表）、`packs`（权限包注册表）、`gate`/`sovereignty`/`records`。执行入口 `execute_if_allowed(&ParsedToolCall) -> ExecutionResult`（字段：`tool_name`/`success`/`output`/`error`/`duration_ms`）。

### 1.5 依赖与注册清单

- Cargo 依赖（最小集）：`apeireth-companion`（plugin/tool_bridge/packs）、`apeireth-tool-registry`（Tool/ToolKind/ToolAxes）、`serde_json`、`async-trait`；测试另需 `apeireth-memory`（`SqliteMemoryStore::open_in_memory`）、`apeireth-tool-runtime`（`ParsedToolCall`）、`tokio`、`chrono`。
- 注册：`crates/apeireth-companion/src/lib.rs` 加 `pub mod <your_plugin>;`（共享文件改动**先通知集成守门员**，per team-work-doc §2.3）。
- 模块头 `//!` 必写：职责 + **0 假装标注**（什么没做）。

---

## 2. 白名单与日常包规则

### 2.1 执行链路（你的工具被调用时过哪些闸）

`bridge.execute_if_allowed` 的真实顺序（来源：`tool_bridge.rs:552-699`，逐段核实）：

1. **主权总闸**：`sovereignty.is_frozen()` → 熔断即全拒（lines 553-562）。
2. **洋葱门**：`gate.check("tool_call", "调用工具 X", 风险级别, target)`（SecurityGate，lines 563-578）。
3. **宪法硬门**：`ConstitutionGate::check(&desc)`——编译期规则表，15 个前缀命中即拦 + sovereignty 记违规（lines 581-591）。规则表（来源：`crates/apeireth-companion/src/constitution_gate.rs:14-30`）：
   自我复制 / 复制自己 / 多开分身（E-4）、绕过洋葱 / 绕过权限 / 越权 / 脱离沙盒（E-6）、删除全部 / 删库 / 格式化（E-2）、假装（PHL-01）、掩盖 / 篡改日志（E-5）、执行外部代码 / 下载并执行（E-4）。
   **注意**：`desc = "调用工具 X 参数 Y"`——**参数也在检查范围内**，且描述由系统侧拼接、调用方不可伪造。**你的工具名与参数都不要包含这些词**。
4. **动态原则层**（自成长 Level 2）：主人批准过的 active 原则做前缀匹配，命中 → 拦截 + 记违反（lines 592-606）。
5. **宪法评审（真 LLM）**：风险 Medium+ 且配置了 judicator → 按原则判案；评审失败 → **保守拒绝**（lines 607-635）。风险映射按工具名关键词（`tool_risk`，lines 541-550）：名含 `exec`/`shell` → High；含 `file`/`patch`/`task` → Medium；其余 Low。
6. **权限包**：`packs.check_and_consume(tool, now_ms)` → 被活跃包覆盖即**免现场审批**直接执行；文件类工具另做执行级路径校验（`paths` 前缀 + `..` 穿越防护，lines 636-666）。
7. **审批规则**（未被权限包覆盖时才走，ApprovalManager 顺序：黑名单 → 白名单 → 风险，来源：`tool_bridge.rs:436-468`）：
   - `BlacklistRule`：当前为空表（最严闸，先于一切）；
   - `WhitelistRule`：内置白名单 16 个工具（`recall_memory`/`save_memory`/`propose_capability`/`simulate`/`forecast`/`audit_log`/`save_experience`/`list_experience`/`verify_experience`/`propose_principle`/`approve_principle`/`goal_create`/`goal_status`/`goal_complete`/`goal_pause`/`goal_block`）；
   - `RiskRule`：类别含 `system`/`network`/`file`/`shell`/`exec`/`patch`/`task` 的调用 → 需主人批准（5 分钟批准窗口）；此时系统会写一条**授权请求**（`approval_requests::record_request`），前端轮询展示、主人一键批准（lines 672-688）——权限洋葱的真实载体，不是虚构的弹窗。
8. **执行**：`run_executor`（可配隔离 worker / spill 超大输出）。

**给插件作者的推论**：
- 权限包（第 6 闸）只豁免第 7 闸的现场审批——**第 2-5 闸（洋葱/宪法硬门/动态原则/LLM 评审）对所有工具生效**，自授权救不了违禁词。
- 不在白名单、又没有权限包覆盖的工具，每次调用都可能触发主人批准——体验差且可能被拒。所以 §2.3 的标准动作（on_load 自授权）是必做项。
- 工具命名避开风险关键词（除非它真的是那类工具）：含 `file`/`patch`/`task` 会升为 Medium 风险，配了 judicator 的部署里每次调用都过真 LLM 评审（token 成本）。

### 2.2 日常包（默认授权面）

`PackRegistry::default_daily_pack()`（来源：`crates/apeireth-companion/src/packs.rs:149-164`）：永久包，覆盖 9 个工具——
`recall_memory`、`save_memory`、`propose_capability`、`simulate`、`forecast`、`WebSearch`、`Grep`、`WebFetch`、`Git`。

权限包三种有效期（来源：`packs.rs:16-23`）：`Permanent`（永久，90 天提醒续签）/ `Hours(u64)`（限时）/ `SingleUse`（单次）。构造器：`PermissionPack::permanent(name, tools)` / `timed(name, tools, hours, budget)`；可叠 `.with_paths(前缀)`（文件类工具执行级路径校验）与 `.with_spend_budget(上限)`（花钱类工具）。

### 2.3 插件自授权的标准动作

```rust
// on_load 里:
bridge.packs.grant(crate::packs::PermissionPack::permanent(
    "你的插件授权",               // ← 名字是稳定锚点 (UUID 每次不同, 不可靠)
    vec!["your_tool".to_string()],
));
// on_unload 里:
bridge.packs.revoke_by_name("你的插件授权");
```

为什么按**名字**撤销：pack 的 `id` 是每次生成的 UUID，`name` 才是稳定键（来源：`packs.rs:176-179` 注释与实现）。三个真插件（education/pentest/gh_accel）全部遵循这一模式。

---

## 3. 测试模板（含 0 装 PASS 写法）

### 3.1 全链路测试模板（正常路径 + 卸载清理，真代码摘录）

来源：`crates/apeireth-companion/src/education.rs:284-316`（原文摘录）——这是插件测试的**标准形状**：装 → 断言注册 → 断言授权 → 桥全链路执行 → 卸载 → 断言真清理。

```rust
#[tokio::test]
async fn plugin_registers_tool_and_pack() {
    use apeireth_memory::SqliteMemoryStore;
    let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let bridge = ToolBridge::new(store);
    let reg = crate::plugin::PluginRegistry::new();
    reg.install(&bridge, Arc::new(EducationDxPlugin)).unwrap();
    assert!(reg.is_installed("education-dx-check"));
    assert!(bridge.registry.list().iter().any(|n| n == "dx_check"));
    // 授权包覆盖 → 免现场审批直接执行
    assert!(bridge.packs.check_and_consume("dx_check", chrono::Utc::now().timestamp_millis()));
    // 全链路: 桥执行 dx_check (忘换 dx 场景)
    let call = apeireth_tool_runtime::parser::ParsedToolCall {
        tool_name: "dx_check".into(),
        args: json!({
            "problem": "∫ x·e^(x²) dx",
            "substitution": "令 t = x²",
            "after": "∫ e^t dx"
        }),
        raw_marker: String::new(),
        archery: false,
        archery_no_reply: false,
    };
    let r = bridge.execute_if_allowed(&call).await;
    assert!(r.success, "dx_check 应可执行: {:?}", r.error);
    assert_eq!(r.output["verdict"], json!("fix"));
    // 卸载 → 真清理: 工具注销 + 授权撤销 (幂等)
    reg.uninstall(&bridge, "education-dx-check").unwrap();
    assert!(!bridge.registry.list().iter().any(|n| n == "dx_check"), "卸载后工具应注销");
    assert!(!bridge.packs.check_and_consume("dx_check", chrono::Utc::now().timestamp_millis()), "卸载后授权应撤销");
    let r = bridge.execute_if_allowed(&call).await;
    assert!(!r.success, "卸载后 dx_check 不可再调");
}
```

### 3.2 失败路径测试（0 装 PASS 的机制保障，真代码摘录）

加载失败必须**拒绝安装且不登记**（来源：`plugin.rs:114-124` + `143-150`）：

```rust
/// 加载失败的插件 (on_load 报错 → 安装拒绝).
struct BadPlugin;
impl Plugin for BadPlugin {
    fn id(&self) -> &str { "bad-plugin" }
    fn version(&self) -> &str { "0.0.1" }
    fn description(&self) -> &str { "加载必失败" }
    fn on_load(&self, _bridge: &ToolBridge) -> Result<(), String> {
        Err("加载失败: 缺依赖".into())
    }
    fn on_unload(&self, _bridge: &ToolBridge) -> Result<(), String> { Ok(()) }
}

#[test]
fn failing_plugin_not_registered() {
    // ... bridge/reg 构造同 3.1 ...
    assert!(reg.install(&bridge, Arc::new(BadPlugin)).is_err());
    assert!(!reg.is_installed("bad-plugin"));
}
```

套件批量安装的**回滚**语义也要测（来源：`plugin.rs:152-162`）：`install_all(&bridge, &[ok, bad])` → `bad` 失败 → 断言 `ok` 也被回滚（`!reg.is_installed(...)`）。

### 3.3 0 装 PASS 写法（规范）

| 禁止 | 正确写法 |
|---|---|
| 做不到返回 `Ok` 假装成功 | 返回 `Err` + **可行动提示**（如 education 的 `"需要 problem (原题) 和/或 after (换元后的式子)"`） |
| 文档/注释写「已支持」实际没有 | 标注「trait 口已备，实现未接」 |
| 静默吞错 | `eprintln!` 记录 + 降级路径说明 |
| 解析不了的输入硬猜 | 如实归类（如 pentest `scan_report` 把不认识的行归入 `unknown_lines`，来源：`pentest.rs:192-199`） |

模块头 0 假装标注示例（来源：`education.rs:6-9`，原文摘录）：

```rust
//! 0 假装 (诚实标注):
//! - v1 是**字符串级规则表**, 不是真实符号计算 (无 CAS 引擎, 不宣称能解积分)
//! - 覆盖四个检查: 忘换 dx / dx 与 dt 混用 / 缺微分 / 残留 x; 外加常见根号模式提示 (三角换元表)
//! - 模式匹配用简单字符串扫描 (√( 括号配对), 复杂公式请让 AI 自己拆解后调用
```

### 3.4 测试命令

```powershell
cargo test -p apeireth-companion -j 4 <你的测试名>
```

`-j 4` 降并行防页文件耗尽；**不要**跑 `cargo test --workspace` 全量（集成守门员专属，per team-work-doc §2.2/§6.3）。

---

## 4. 卸载真清理（不留注册残留）

### 4.1 标准组合

`on_load` 里你注册了什么，`on_unload` 就必须清掉什么。当前机制下就两样：

```rust
fn on_unload(&self, bridge: &ToolBridge) -> Result<(), String> {
    bridge.registry.unregister("your_tool");        // 注销工具 (顺带清类别索引, registry.rs:107-115)
    bridge.packs.revoke_by_name("你的插件授权");      // 撤销授权 (按名, 不按 UUID)
    Ok(())
}
```

### 4.2 要求

1. **幂等**：`on_unload` 可能被重复调用，不得 panic（trait 契约，`plugin.rs:24-25`）。
2. **无残留**：卸载后 `bridge.registry.list()` 不得含你的工具名；`packs.check_and_consume` 必须返回 false；再调用必须失败（§3.1 的最后三个断言就是验收式）。
3. **自建状态自清**：若插件将来在 on_load 里创建文件/缓存/定时器等额外状态，on_unload 必须一并清理（当前三个官方插件都只有「工具+授权」两样，无额外状态）。

### 4.3 验收断言（复制自 education.rs:310-315）

```rust
reg.uninstall(&bridge, "education-dx-check").unwrap();
assert!(!bridge.registry.list().iter().any(|n| n == "dx_check"), "卸载后工具应注销");
assert!(!bridge.packs.check_and_consume("dx_check", chrono::Utc::now().timestamp_millis()), "卸载后授权应撤销");
let r = bridge.execute_if_allowed(&call).await;
assert!(!r.success, "卸载后 dx_check 不可再调");
```

---

## 5. 数据源 adapter 模板（对接预测机套件 §5.2）

### 5.1 现状（0 假装，先说实话）

team-work-doc §5.2 规划的「**数据源 adapter trait + adapter registry 热插拔 + 4 旗舰适配器**」中，**adapter trait 目前未接**：

- `crates/apeireth-companion/src/oracle.rs` 只有预测核心：`Forecast`（可证伪断言，lines 95-131）、`ForecastRegistry`（登记/resolve/Brier 校准，lines 135-215）、`UncertaintyResolver` trait 口（lines 52-54，LLM 语义裁决注入点）。
- 全仓库无 `DataSource`/`ForecastSource` 类 adapter trait（2026-08-16 grep 核实）。
- `ForecastRegistry` 也不向插件暴露注入入口（ToolBridge 内部自建，session 固定 `"me"`，`tool_bridge.rs:403-406`）——**插件不能直接登记预测**。

**当前可行的数据源插件形态**：数据源 = 普通的取数 Tool 插件（本节的模板）；AI 拿到数据后，用内置 `forecast` 工具（在白名单 + 日常包内，`tool_bridge.rs:443` / `packs.rs:157`）登记可证伪预测。等官方 adapter trait 落地后再迁移（升级路径见 §5.4）。

### 5.2 单文件模板（填一个文件 = 新插件）

把下面的模板存为一个文件（如 `crates/apeireth-companion/src/weather_source.rs`），填三处 `TODO`，加 `lib.rs` 注册，就是一个新的数据源插件。

> 标注：**模板代码**（基于上文已核实的真实 API 编写，非现有文件摘录；所用 API 来源：`trait_def.rs:27-45`、`types.rs` 5 轴、`education.rs:182-206` 装配模式、`github_accel.rs:226-234` Network 轴写法）。

```rust
//! `apeireth-companion::weather_source` — 天气数据源插件 (社区模板).
//!
//! 0 假装 (诚实标注): <TODO: 写清什么没做 — 如「v1 只 mock, 未接真 API」>
//!
//! 装配: on_load 注册 `weather_fetch` 工具 + 授权; on_unload 真清理.

use std::sync::Arc;

use apeireth_tool_registry::{Tool, ToolAxes, ToolKind};
use apeireth_tool_registry::types::{AwaitingAxis, OutputAxis, ResidentAxis, TransportAxis, TriggerAxis};
use serde_json::{json, Value};

use crate::plugin::Plugin;
use crate::tool_bridge::ToolBridge;

/// 天气数据源工具 — mock 先行: 测试用内置数据, 真 API 可选.
pub struct WeatherFetchTool;

#[async_trait::async_trait]
impl Tool for WeatherFetchTool {
    fn name(&self) -> &str { "weather_fetch" }
    fn kind(&self) -> ToolKind { ToolKind::Async }
    fn axes(&self) -> ToolAxes {
        ToolAxes {
            trigger: TriggerAxis::OnDemand,
            awaiting: AwaitingAxis::Immediate,
            resident: ResidentAxis::Ephemeral,
            transport: TransportAxis::Network, // 纯 mock/本地数据源 → 改 Local
            output: OutputAxis::Value,
        }
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        let city = args.get("city").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        if city.is_empty() {
            return Err("需要 city (城市名)".to_string()); // 0 装 PASS: 可行动提示
        }
        // TODO: 接真 API (如 Open-Meteo 免费 API); 未接时返回 mock 并如实标注
        Ok(json!({
            "city": city,
            "mock": true,
            "note": "模板占位数据: 发布前接真数据源, 或如实保留 mock 标注",
            "temperature_c": 20.0,
            "condition": "clear"
        }))
    }
}

/// 天气数据源插件: 注册工具 + 授权.
pub struct WeatherSourcePlugin;

impl Plugin for WeatherSourcePlugin {
    fn id(&self) -> &str { "weather-source" }
    fn version(&self) -> &str { "0.1.0" }
    fn description(&self) -> &str { "天气数据源: <TODO: 数据源与能力范围>" }
    fn on_load(&self, bridge: &ToolBridge) -> Result<(), String> {
        bridge.registry.register("weather_fetch".to_string(), Arc::new(WeatherFetchTool));
        bridge.packs.grant(crate::packs::PermissionPack::permanent(
            "天气数据源授权",
            vec!["weather_fetch".to_string()],
        ));
        Ok(())
    }
    fn on_unload(&self, bridge: &ToolBridge) -> Result<(), String> {
        bridge.registry.unregister("weather_fetch");
        bridge.packs.revoke_by_name("天气数据源授权");
        Ok(())
    }
}
```

模板纪律（对齐 team-work-doc §5.2 验收与 §4 任务包「数据源/外部依赖策略」）：

- **mock 先行可测**：单测用内置数据跑通，不依赖网络（限流环境下不阻塞验收）。
- **真 API 可选**：接真 API 后在模块头如实标注数据来源与限流。
- **预测对接**：AI 用取数结果调内置 `forecast` 工具登记可证伪预测（`Forecast::new(statement, probability, deadline_ms)`，到期对照实际结果算 Brier，来源：`oracle.rs:95-131`）。插件本体不碰 `ForecastRegistry`（未暴露注入口，见 §5.1）。
- **测试照抄 §3.1 模板**（把工具名/插件 id 换掉即可）。

### 5.3 §5.2 规划的 8 个社区数据源

体育 / 选举 / 房价 / 航班 / 能源 / 流失 / 农业 / 销售（来源：team-work-doc §5.2）——每个都按本节模板填一个文件。

### 5.4 升级路径（adapter trait 落地后）

官方 adapter trait 实装后（§5.2 欠账，见 team-work-doc），数据源插件迁移为 adapter 实现；届时本节更新迁移对照表。**在此之前，不要自行发明平行的 adapter 体系**（集成而非分立，team-work-doc §1.2）。

---

## 6. 发布检查单

提交社区插件 PR 前逐项自查（对齐 team-work-doc §7 验收总纲 + maintenance-guide §三）：

| # | 项 | 标准 | 证据 |
|---|---|---|---|
| 1 | 测试全绿 | `cargo test -p apeireth-companion -j 4` 通过，含失败路径（加载失败/非法输入） | 测试输出 |
| 2 | 全链路测试 | §3.1 形状：装 → 注册 → 授权 → 桥执行 → 卸载 → 清理断言 | 测试代码 |
| 3 | 卸载真清理 | `registry.unregister` + `revoke_by_name`，幂等，卸载后调用必失败 | §4.3 断言 |
| 4 | 0 装 PASS | 模块头 `//!` 标注没做什么；错误 = `Err` + 可行动提示；未接的 trait 口如实标注 | 模块头 |
| 5 | 命名合规 | 工具名/参数不触发宪法硬门前缀（§2.1 第 3 闸）；不滥用风险关键词（§2.1 第 5 闸） | 自查 |
| 6 | 授权完整 | on_load 自授权 pack（按名可撤销）；不依赖白名单特例 | on_load 代码 |
| 7 | 文档同步 | maintenance-guide 模块地图加行；本指南如涉及新形态则更新 | PR diff |
| 8 | 提交纪律 | 小步提交 + 中文 message（为什么 + 做了什么 + 测试结果）；无调试输出；`git status` 只含自己的文件 | commit |
| 9 | 自审报告 | 改动文件 / 测试结果 / 集成点 / 0 假装标注 / 给守门员的合并提示 | 报告文本 |

合入流程：PR → **集成守门员**合并（`cargo check --workspace` + 相关 crate 测试 + 规范执法，per team-work-doc §6.1/§6.2）。

---

## 附：来源索引（本文档引用的真实代码）

| API / 示例 | 位置 |
|---|---|
| `Plugin` trait / `PluginRegistry` | `crates/apeireth-companion/src/plugin.rs:18-26` / `29-90` |
| `Tool` trait | `crates/apeireth-tool-registry/src/trait_def.rs:27-45` |
| `ToolKind` 6 类 / `ToolAxes` 5 轴 | `crates/apeireth-tool-registry/src/types.rs:42-66` / 各轴 enum |
| `ToolBridge`（registry/packs/执行链/审批规则/风险映射） | `crates/apeireth-companion/src/tool_bridge.rs:358-483` / `552-699` / `436-468` / `541-550` |
| `PermissionPack` / `PackRegistry`（日常包/revoke_by_name） | `crates/apeireth-companion/src/packs.rs:27-124` / `141-222` |
| `ConstitutionGate` 硬门规则表 | `crates/apeireth-companion/src/constitution_gate.rs:14-30` |
| `SuiteDef` / `SuiteCatalog::install_with_plugins` | `crates/apeireth-companion/src/suites.rs:38-53` / `164-203` |
| `Forecast` / `ForecastRegistry` / `UncertaintyResolver` 口 | `crates/apeireth-companion/src/oracle.rs:95-215` / `52-54` |
| EducationDxPlugin（完整真插件） | `crates/apeireth-companion/src/education.rs:152-316` |
| PentestReconPlugin / PentestScanPlugin（双插件 + E-1 范围闸） | `crates/apeireth-companion/src/pentest.rs:219-242` |
| GhAccelPlugin（包装已有工具 + Network 轴） | `crates/apeireth-companion/src/gh_accel.rs:20-45` + `crates/apeireth-tools/src/github_accel.rs:219-234` |
| 套件端到端演示 | `crates/apeireth-companion/examples/education_suite_demo.rs` |

> 相关文档：[team-work-doc.md](team-work-doc.md)（§1.3 三层模型 / §5.6 本规范任务源 / §7 验收总纲）· [maintenance-guide.md](maintenance-guide.md)（模块地图 / 加新模块规范）· [release-plan.md](release-plan.md)（三件套发布规划）· [oracle-suite-design.md](oracle-suite-design.md)（预测机套件设计）
