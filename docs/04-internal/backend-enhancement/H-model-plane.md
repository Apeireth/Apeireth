# H. 模型面

> `Status: 📋 想法细节（隶属 00-backlog #H）`

## 现状
- 三 provider 真实现（openai-compatible/anthropic 小米系）、凭据热更、reasoning 适配面、usage 计量已接；
- 缺：多模型路由（按任务选模型）、fallback（主模型挂了换备）、成本分级。

## 想法
1. **多模型路由**：任务分级选模型（杂活便宜模型/深思贵模型）——省钱第一杠杆；
2. **fallback 链**：主备自动切换（错误分类驱动：认证失败换钥、超时换路、限流退避）；
3. **成本分级策略**：会话预算×模型单价的最优配比（与 A 轴花费维联动）；
4. **缓存经济**：前缀冻结已做——补"缓存感知的提示组装"（省下的钱可见，遥测条已有命中率）。

## 缺口
- 模型能力画像（每模型的工具支持/上下文窗/单价表）——路由的依据。

## 分期
P1 模型画像表 → P2 fallback 链 → P3 多模型路由 → P4 成本分级。

## 验收
fallback 切换带错误帧（不静默）、路由决策可解释（为什么选这个模型）、成本对比实测。

## P1 实施记录（模型画像表·后端小批）

> 本节为叠加记录（原有各节不动）。零前端触碰；花费维计量属 A 轴 P2，本批只交**单价表来源**。

**画像结构**（`apeireth-core::model_profile::ModelProfile`，单价单位=微元/千 token）
`ModelProfile{model_id, context_window, max_output, supports_tools, supports_streaming, input_price_micros, output_price_micros, cache_read_price_micros, capabilities_flags}`。
- **缺省 = `None` = 未提供**：窗口/输出/两布尔/三单价全部可缺省；**单价绝不编价**（缺一半也不补另一半）；
- `capabilities_flags`：snake_case 开放词表（`vision`/`system_prompt`/…）；`tool_calls`/`streaming` 是保留词——路由关键能力走显式布尔，不进 flags（防同义重复与矛盾）；
- **未知模型 = 显式 unknown 行**（`ModelProfile::unknown`：除 id 外全 `None`，`is_unknown()` 可查），不编造任何事实；
- model id 匹配 ASCII 大小写不敏感（与提供商面规范 id 小写化同口径），行内保留接口拼写。

**配置面说明**（三层覆盖合并；env 最上；任一层坏=整体拒开 fail-closed）
1. **内置基线**（`ModelProfileTable::baseline`）：已知模型 = 本库三家提供商默认模型表声明的 id（接口事实镜像，`BASELINE_*_MODELS` 三族常量与 `DEFAULT_MODELS` 由测试钉逐 id 一致）。保守默认值：能力布尔取族声明（含 anthropic 族 wire 适配面只声明 `system_prompt` 的照实口径，不拔高）、上下文窗/输出取**保守下界**（`BASELINE_CONTEXT_WINDOW_FLOOR=32768` / `BASELINE_MAX_OUTPUT_FLOOR=8192`，routing floor，**不是**规格书）、单价一律未提供；
2. **数据目录 `model-profiles.json`**（`MODEL_PROFILES_FILE`）：StoredDoc 拒开语义（`open_single`：坏 JSON / 身份不符 / 版本不符**拒绝打开**，绝不静默回退基线）；body 形状 = JSON 数组或 `{"profiles":[...]}`；
3. **env `APEIRETH_MODEL_PROFILES`**（`MODEL_PROFILES_ENV`，JSON 覆盖）：同形状；解析失败 fail-closed。
覆盖合并口径：**逐模型整行替换**（非字段级混血），同一 id 上层行胜；层内重复 id（大小写不敏感）= 拒开。

**消费面接线图**
```text
model-profiles.json (StoredDoc 拒开) ──┐
APEIRETH_MODEL_PROFILES (env 覆盖)  ──┼─► ModelProfileTable::load (三层折叠)
内置基线 (族声明+保守下界)          ──┘        │
                                              ├─► resolve(active_model_id) ─► ProductionBackends.self_status_model_profile
                                              │        ─► self_status `model` 节 (第七节)
                                              └─► impl CostRateTable (单价表来源)
                                                       └─► A-P2 计量点安装处 (本批未接线)
```
- **self_status 模型节**（输出合同第七节，形状冻结随 H-P1 演进）：`model_id`/窗口/输出/两布尔/flags 真值或显式 null；单价三字段缺省显式 null + `prices_display` 明说「未提供」（部分提供逐项标注）；未知模型显式 unknown + 原因；无来源整节显式 null + 原因；
- **配额花费维 = 单价表就绪**：`impl CostRateTable for ModelProfileTable`（有价行给价、缺一半/无价/未知回 `None`）+ `CostRateMicros::cost_micros`（每侧向上取整、饱和——计量点接线时的冻结换算口径）。**花费维计量与超限即帧属 A 轴 P2，本批显式未接线**（上限恒不可达、花费列无计量点、自述沿用"暂无接口"），接线路径见 [A-quota-real-interface.md](A-quota-real-interface.md) H-P1 接头记录；
- 生效模型 id 口径：`APEIRETH_MODEL` 显式覆盖 > minimax 族首个默认模型，与运行时 `with_default_model` 同一条 `active_model_id()` 解析（口径唯一）。

**测试证据**（24 个新测试；既有套件零回归另跑全量）
- 画像解析：拒开（坏 JSON/身份不符/未来版本/未知字段/保留词 flags/重复 id）、覆盖合并+env 优先、env 与文件双语法、fail-closed（`model_profile.rs`）；
- 内置基线命中：保守下界+族能力布尔+单价未提供；`tests/model_profile_baseline.rs` 钉「基线 == 提供商默认模型表」口径唯一；
- 未知模型 unknown 如实：core `resolve`/`is_unknown` + assembly 快照 + self_status 渲染三层钉死；
- CostRateTable 画像来源：有价算成本（换算+取整口径）/ 缺一半回 `None` / 无价 `None` / 未知 `None`；
- 花费维：显式未接线钉死（上限恒不可达、花费列不写数、检查点不出花费超限事实）；
- self_status 画像节：真值投影/缺省「未提供」/无来源显式 null + 原因/e2e 工具真调；
- 脱敏（无密钥面）：画像面无凭据字段、误粘贴凭据字段拒开且错误帧不回显值（值自由报错）、自述帧字段全表无凭据面字段；
- CLI 消费面接线：`active_model_id` 单一解析口径（显式覆盖/空白回退/族模型表 env 同源）、坏 `APEIRETH_MODEL_PROFILES` 装配拒开（`production_knobs.rs`）。
