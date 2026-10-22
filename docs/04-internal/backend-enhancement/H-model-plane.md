# H. 模型面

> `Status: ♻️ 历史+实施记录（隶属 00-backlog #H；2026-10-20 第三线源码对账：P2/P3/P4 已交付，见文末对账节）`

## 现状
- 三 provider 真实现（openai-compatible/anthropic 小米系）、凭据热更、reasoning 适配面、usage 计量已接；
- 交付状态（2026-10-20 源码对账）：fallback ✅ `ProviderRouter`（`crates/engine/runtime/src/canonical/provider.rs`：候选过滤/fallback 顺序/健康 EMA/retryable 才下抛，2026-09 基线移植）；成本分级与路由 ✅ H-P2 `CostRouter`（`crates/foundation/core/src/cost_router.rs`：四档→候选→选定，默认关 `APEIRETH_COST_ROUTING`，CLI chat 任务选型已接）；余：网关路径任务源路由、显式档 CLI 旗标、缓存经济（`PromptCacheStabilizer` 未接线）。

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

## H-P2 实施记录（成本分级路由·后端批）

> 本节为叠加记录（原有各节不动）。模型面轴第二件「更省」——**任务按成本/能力
> 自动选模型**：分级词表 + 路由规则 + 配置面 + self_status 路由节；依据数据面
> = H-P1 画像表（路由/成本都只认画像表事实，**绝不编价**）。批次命名按模型面轴
> 施工序（H-P1 画像表 → H-P2 成本分级路由），与上方「分期」草拟的编号不同源。

**分级词表**（`apeireth-core::cost_router::CostTier`，四档闭合：`frugal|standard|capable|premium`）
每档 = 模型筛选条件（能力需求 × 单价上限；表内数字是**预算策略常量**，不是任何模型的价格/规格事实）：

| 档位 | 任务语义 | 能力需求 | 窗口下限 (token) | 单价上限 (微元/千 token，比较单价合计口径) |
|------|----------|----------|------------------|--------------------------------------------|
| `frugal` | 杂活/轻任务 | 不强制工具 | `FRUGAL_MIN_CONTEXT_WINDOW` = 8_192 | `FRUGAL_PRICE_CEILING_MICROS` = 5_000 |
| `standard` | 常规工作（默认档） | 需工具 | `STANDARD_MIN_CONTEXT_WINDOW` = 32_768 | `STANDARD_PRICE_CEILING_MICROS` = 20_000 |
| `capable` | 重活（多步工具/大上下文） | 需工具 | `CAPABLE_MIN_CONTEXT_WINDOW` = 65_536 | `CAPABLE_PRICE_CEILING_MICROS` = 80_000 |
| `premium` | 深思（最难任务） | 需工具 | `PREMIUM_MIN_CONTEXT_WINDOW` = 131_072 | `PREMIUM_PRICE_CEILING_MICROS` = 400_000 |

窗口下限 = routing floor（同 H-P1「保守下界」口径，**不是**规格书）；单价上限按
比较单价（`input + output` 等权合计）比较，缓存读列不入比较（同 `CostRateTable`
形状口径）。

**路由规则表**（`CostRouter::route`，决策可解释：`selection_reason`/`fallback_reason` 真值）
1. **任务描述 → 档位**（`infer_tier`）：显式档（`RoutingRequest::explicit_tier`，`TierSource::explicit`）> 规则推断（工具调用数/上下文规模启发式，`TierSource::inferred`）> 默认 `standard`（信号缺位，`TierSource::default`）。推断阶梯：工具调用数 ≥ 32 或上下文 ≥ 200_000 → `premium`；≥ 8 或 ≥ 64_000 → `capable`；**零**工具调用且上下文 ≤ 8_192（双轻信号）→ `frugal`；其余/缺位 → `standard`。口径：**重信号单证即升档（能力风险优先），轻档需双证（缺信号不猜轻）**；
2. **档位 → 候选**（画像表过滤，逐行三查）：支持工具（档位或任务要求工具时必须 `supports_tools == Some(true)`，未知不等于支持）/ 窗口够（`context_window` 显式 ≥ max(档位下限, 任务上下文规模)，未知不算够）/ 单价 ≤ 档上限（**无价/缺价行不参与价格比较**，按能力入候选）；
3. **候选 → 选定**（顺序即优先级）：① 档位映射配置的首选模型（**在候选内才生效**；落选如实注明后顺延）→ ② 有价候选里比较单价最低者（平手取画像表顺序；理由注明「比较只覆盖有价候选」）→ ③ 默认模型在候选内 = 保持默认（无价格证据不换模型）→ ④ 画像表顺序第一个候选（能力合格，价格照实未提供）；
4. **无候选 = 回退默认模型 + 如实标注**（`used_fallback: true` + `fallback_reason` 报过滤条件真值）：默认模型也不过档位过滤时照实注明「仅作显式回退」；默认模型缺席则如实选不出（`selected_model: None`），不硬凑。

**诚实口径**（绝不编价延续）
- 无价模型（单价列 `None`）= **不参与价格比较**：按能力入候选，价格标注照实「未提供」；
- 缺一半（如只给输入价）同样算不出成本 = 不参与价格比较（与 H-P1 冻结的
  `CostRateTable` 换算口径同源：缺一半回 `None`），**缺的另一半永不补**；
- 档位单价上限/窗口下限是**档位语义（预算策略常量）**，不冒充任何模型的价格或规格事实；
- 路由决策三真值（当前档/选定模型/回退标注）全部进自述，回退不粉饰、未路由不编造。

**配置面**（fail-closed）
1. `APEIRETH_COST_ROUTING`（`COST_ROUTING_ENV`）：主开关，**只有精确 `1` 开**，其他取值一律关（garbage 绝不误开）；
2. `APEIRETH_COST_ROUTING_PREFERRED`（`COST_ROUTING_PREFERRED_ENV`）：档位映射配置（每档首选模型覆盖，JSON 对象 `{"frugal": "<model id>", ...}`）；坏 JSON/非对象/未知档位词/值非字符串/空 id = **整体拒开**（启动错误正式通道，值自由报错），空白 = 无覆盖。

**self_status 路由节**（`cost_routing`，第十六节冻结合同的演进，附理由：路由决策可解释）
`enabled`（开关真值）/ `tier` + `tier_source`（当前档 + 来源）/ `selected_model` + `price_display`（选定模型 + 价格标注「未提供」照实）/ `used_fallback`（回退标注真值；无决策显式 null 不谎报 false）/ `fallback_reason` / `selected_reason`（选定理由）/ `candidates[]`（候选全集，逐行 `price_display` 诚实标注）/ `reason`（关闸或「尚无任务被路由」的如实原因）；无来源整节显式 null + 原因。

**消费面接线图**
```text
APEIRETH_COST_ROUTING (开关) ──┐
APEIRETH_COST_ROUTING_PREFERRED (档位映射) ──┼─► CostRoutingConfig::from_env_values (fail-closed)
                                              │        │
model-profiles 三层折叠 (H-P1) ───────────────┼─► CostRouter::new (画像表 + 生效模型 + 配置)
active_model_id() (回退目标, 同一解析口径) ───┘        │
                                                      ├─► resolve_turn_model (CLI chat 任务选型:
                                                      │     显式 --model > 路由选定 > 默认模型)
                                                      └─► cost_routing_probe ─► self_status `cost_routing` 节
                                                                (与任务选型同一台路由器, 同源现读)
```
- CLI chat 面已接任务源路由（`dispatch_canonical_chat`：先装配路由器→`resolve_turn_model`→回合模型）；任务信号口径照实：上下文规模 = 提示字符数/4 启发式估计（非计量），工具调用数信号缺位（`None`，不猜）。**未实测标注**：`resolve_turn_model` 选型行为已测（显式覆盖/开关关/有价选省），真实 provider 回合的 live 全链未跑；
- 显式档入口在路由 API 面（`RoutingRequest::explicit_tier`）；CLI 旗标面暂未开（如实：后续批）；
- **本批显式未接线**：网关路径的任务源路由（战场外，其自述路由节照实报「尚无任务被路由」）、fallback 链（错误分类驱动的主备切换，属另一件）、按会话预算的最优配比（与 A 轴花费维联动）。

**测试证据**（18 个新测试；既有套件零回归另跑全量，实测值随批回报）
- 分级词表/档位条件矩阵：四档闭合 + 未知词拒 + 能力需求×单价上限×窗口下限钉死（`cost_router.rs`）；
- 四档筛选矩阵：支持工具/窗口够/单价≤档上限三查逐档候选全集精确断言；
- 规则推断（启发式档位）：重信号单证升档、轻档双证、缺信号默认 standard（来源真值 `explicit|inferred|default`）；
- 显式档覆盖：显式档压过重信号启发式；
- 无候选回退 + 如实标注：回退默认模型/回退标注真值/过滤条件如实/默认缺席如实选不出；
- 无价模型语义（不编价）：无价/缺价行按能力入候选、价格字段照实缺省永不补、有价超上限出局、无价格证据保持默认；
- 配置覆盖：每档首选模型胜出（大小写不敏感）/ 落选如实注明顺延；比较单价最低者胜出；
- 开关 fail-closed：只有精确 `1` 开（core + CLI env 双层）；关闸 `route` 不路由 + 自述照实报关闸；
- 档位映射 fail-closed：坏形状整体拒开 + 值自由报错（误粘贴值不进错误帧）+ 装配拒开（`production_knobs.rs`）；
- 自述路由节：当前档/选定模型/回退标注真值投影 + 候选逐行「未提供」标注 + 关闸/未路由显式 null + 原因 + 无来源显式 null（`self_status.rs`）+ 探测口同源进快照（`self_status_source.rs`）；
- CLI 任务选型：显式 `--model` > 路由选定 > 默认模型（`production_knobs.rs`）。

**跨范围报备**（最小改动 + 理由，见施工纪律 §二.4/§六.3）
- `crates/capabilities/tools/src/self_status.rs`（战场外，验收必经）：路由节入十六节冻结合同（快照 2 字段 + 渲染 + 节表 + fixture + 2 测试 + 工具描述一句），照 care/backup 节同款最小增量；
- `crates/adapters/cli/src/lib.rs` 网关装配处 + `tests/care_heartbeat_host.rs`/`tests/canonical_cli_bootstrap.rs` 各一处**解包机械补位**（组装出口新增路由器绑定位，编译断点，网关语义零改动）；
- `docs/03-reference/capabilities-matrix.md` self_status 行节表同步（口径随行）。

## 2026-10-20 状态对账（第三线；不改上文历史记录）

- `00-backlog` H 行「P2 fallback / P3 路由 / P4 成本分级待」为 **2026-10-13 旧状态**，已按源码更新（H 行改写）。
- ProviderRouter：`crates/engine/runtime/src/canonical/provider.rs`（fallback 顺序 / 健康 EMA / permanent 立即失败；Runtime 持有实例，请求路径消费）。
- CostRouter：`crates/foundation/core/src/cost_router.rs`（H-P2 四档路由，`APEIRETH_COST_ROUTING` 默认关；CLI `resolve_turn_model` 已接）。
- 剩余与 §H-P2「本批显式未接线」一致：网关任务源路由、显式档 CLI 旗标、缓存经济（`PromptCacheStabilizer` 生产未接线）。
