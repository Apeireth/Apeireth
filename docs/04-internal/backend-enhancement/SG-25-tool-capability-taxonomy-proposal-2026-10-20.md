# SG-25 工具能力 taxonomy 提案（v0 守望式接线；B 类 ADR 前置）

> 日期：2026-10-20 · 提出：Lead（生物机制族 W3.7 余项）
> 状态：**提案 / 待拍板**（账本 `system-gap-ledger-2026-10-18.md` SG-25 行 + 00-backlog 跟踪）
> 口径：本提案只解「观测面真实信号源」，**不做自动剪枝、不改工具契约**；四级口径目标 =
> SG-25 从 IMPLEMENTED → **PRODUCTION WIRED（默认关，观测面）**。

## 1. 缺口与堵点

生物机制族 14 条机制当前 **13 条 PRODUCTION WIRED（默认关）+ 1 条 IMPLEMENTED**。唯一未接的是
**SG-25 共生菌群 / 工具生态健康度**（`crates/capabilities/tools/src/ecosystem_health.rs`，纯函数内核
`analyze(&[ToolProfile])`）。它的输入需要一个工具的**能力标签集合**（`ToolProfile.capabilities`），
而当前契约面没有这个信息：

- `ToolCapability` trait（`crates/foundation/plugin/src/tool.rs:44`）只有
  `id()` / `declaration() -> NormalizedTool` / `invoke*` / `freeze_invocation()` / `invoke_in_session()`；
- `NormalizedTool`（`crates/foundation/protocol/src/normalized.rs:265`）只有
  `name` / `description` / `parameters` / `strict`；
- 两者都**没有能力标签字段**，运行时注册表里也没有第二来源。

已排除的「假接线」路径：

| 拒法 | 拒因 |
|---|---|
| 从 `description` 文本猜标签 | 不可复核、随文案漂移 ⇒ 伪接线（0 装红线） |
| 把所有工具都当同一能力 | diversity 恒 1、报告无信息量，等于没接 |
| 按 capability id 前缀当标签（`tool.git_* → git`） | 前缀是命名习惯不是语义，跨 crate 不一致，且会误判重叠 |

## 2. 四种候选方案

| 方案 | 做法 | 成本 | 判定 |
|---|---|---|---|
| **A. 契约声明式标签（目标态）** | 给 `NormalizedTool` / `ToolCapability` 加 `capabilities: Vec<CapabilityTag>`，工具注册时声明 | **B 类**：契约 + 扩展 ABI + 41 个工具逐一填 + 三方插件迁移 | **长期正解**，需 ADR |
| **B. 静态策展表（v0 推荐）** | 装配面维护一张 `(tool id → 能力标签, 来源)` 静态表，probe 按**实际注册清单**查表 | A 类：不动契约、新文件 + 测试 | **本提案先落这层** |
| C. 配置文件注入 | 用户/部署侧可编辑映射 | 新配置面 + 漂移面 + 泄漏面 | 拒（运维负担、不可复核） |
| D. 文本推断 | 见 §1 拒表 | — | 拒（伪接线） |

**结论**：A 立项为 B 类 ADR；B 立即落地为 v0。A 到位后 B 表删除，probe 改读契约标签，报告 schema
与观测语义兼容（`taxonomy_version` 递增即可）。

## 3. v0 设计（A 类，观测面）

### 3.1 落点

| 面 | 文件 | 内容 |
|---|---|---|
| 策展表 + 查表 | `crates/engine/runtime-assembly/src/canonical/tool_taxonomy.rs`（新） | `TaxonomyEntry { id, tags, source }`、`tags_for(id_or_name)`、`taxonomy_version()` |
| probe | 同目录 `tool_ecosystem_probe.rs`（新）或并入 `bio_mechanism_probes.rs` | `ecosystem_health_probe(profiles) -> Arc<dyn BioMechanismProbe>` |
| 装配 | `bio_production_wiring.rs` | capabilities 组装完成后注册 `BioMechanismKind::EcosystemHealth`；顺序不容许时用 W3.6 同款迟绑定槽 |
| 开关 | **不新增 env** | 复用 `APEIRETH_ENABLE_BIO_SELF_STATUS`（整节关 = 一个 probe 都不调用） |

**关键：标签表与运行时注册清单解耦**——probe 只对「本装配根真实注册了的工具」查表，**不把表里
所有工具都当已注册**（否则读数虚高）。

### 3.2 词汇 v0（12 族，一工具可多标签）

`filesystem`（读写/补丁/工作树）、`process`（shell/进程/监督）、`network`（fetch/爬取/出口）、
`search`（检索/仓库地图）、`memory`（记忆/会话/线索）、`media`（图像/语音）、`document`
（文档/表格/备份）、`governance`（审批/守卫/自述/报告）、`companion`（care/仪式/练习场/存在感）、
`self`（自述/诊断）、`mcp`（第三方桥接位）、`unknown`（表未命中，**不计入分数**）。

映射示例（v0 表首版，41+ 静态 id 全量建表）：

| 工具 id | 标签 | source |
|---|---|---|
| `tool.filesystem` | filesystem, search | builtin |
| `tool.apply_patch` / `tool.worktree_plan` / `tool.worktree_apply` | filesystem | builtin |
| `tool.shell` / `tool.process` / `tool.supervisor` | process | builtin |
| `tool.fetch` / `tool.web_fetch` | network | builtin |
| `tool.search` / `tool.repo` | search, filesystem | builtin |
| `tool.self_status` / `tool.trait_report` | self, governance | builtin |
| `tool.care_schedule` / `tool.ritual_done` / `tool.ritual_skip` / `tool.play_*` | companion | builtin |
| `tool.image_ops` / `tool.sheet_ops` / `tool.doc_ops` / `tool.backup_export` | media, document | builtin |
| `tool.mcp` / 未知动态 id | mcp | mcp |

### 3.3 读数与报告（写进 `capabilities.bio_mechanisms.ecosystem_health` 单格）

`value`（全部真实计算，非编造）：

```json
{"tool_count": 31, "tagged_count": 29, "untagged_count": 2,
 "diversity": 0.42, "redundancy": 0.03, "overlap_pairs": 4,
 "overlap_pressure": 0.11, "dysbiosis": 0.36,
 "taxonomy_version": 1, "weights": [0.4, 0.2, 0.4],
 "thresholds": {"overlap": 0.8, "redundancy_tolerance": 1, "alert": 0.6}}
```

- `threshold` = 0.6（失调告警线）；`last_action` = `null`（瞬时结构观测，无历史动作，如实）。
- `untagged_count > 0` 时在 `reason`/`gaps` 如实列「表未命中工具数」——**不猜、不糊**。
- `tagged_count == 0`（如全 MCP 动态环境）→ probe 返回 `Err` ⇒ 单格 `null` + 真因「无工具命中能力
  标签表（taxonomy 未覆盖）」，**不输出无意义的全 0 分数**。

### 3.4 不变量（测试钉死）

1. 默认关：`APEIRETH_ENABLE_BIO_SELF_STATUS != "1"` ⇒ 整节 `null` + 原因、零 probe 调用、零注册。
2. 关时 byte 级零变化（沿用 W3.2 的 golden 断言）。
3. 单格失败 = `null` + 真因，不整帧失败（沿用 W3.6 语义）。
4. 只读：无 `&mut`、0 I/O、0 系统时钟（probe 从已注册清单快照构造，不调工具）；不含参数/结果/
   用户文本（**零 PII**，只有工具名与静态标签）。
5. 确定性：工具按名字排序、标签去重排序后进 `analyze`；同输入同报告。
6. 表完整性：编译期常量清单（runtime-assembly canonical 的 8 个 `*_CAPABILITY_ID` + tools crate 16 个
   id）在测试里逐个断言可查表——新增工具忘了建表会在测试红，而不是静默 `untagged`。

## 4. 验收标准

- [ ] 默认关全绿（零注册/零调用/byte 零变化）。
- [ ] env 开 + 真实 capabilities：`ecosystem_health` 行 `wired: true`、`stage: "wired"`、value 为上述
  真实字段；把注册清单（测试桩）增删一个工具，`tool_count` 与分数随动。
- [ ] 未命中工具计入 `untagged_count` 且报告文本列出（诚实）。
- [ ] 全未命中 ⇒ 单格 `null` + 真因。
- [ ] 矩阵 §1.1 SG-25 行 IMPLEMENTED → PRODUCTION WIRED（默认关，观测面）；台账 W3.7 剩余项去掉
  「SG-25 taxonomy」，改为「A 方案 B 类 ADR」。
- [ ] gates：scoped test/clippy/fmt/arch_guard 全绿；全量只按 test-baseline 单一真源登记。

## 5. 待 B 类 ADR 决策（本提案不擅动）

1. **能力标签词汇与所有权**：谁定义、如何版本化、冲突如何裁（本提案建议：产品/架构共管 + 语义版本）。
2. **A 方案契约形态**：`NormalizedTool` 加字段 vs `ToolCapability` 加方法 vs 独立 `ToolTaxonomy` 注册面；
   对扩展 ABI / 三方插件 / LOCKED 边界（13 键等）的影响评估。
3. **MCP / 第三方工具治理**：缺省空标签（本提案）还是强制声明、还是用户覆盖；未声明是否拒绝注册。
4. **观测 vs 剪枝边界**：本提案坚持只读观测；若未来要做「合并建议 → 自动剪枝」，须新 ADR 并过审批门。

## 6. 工作量

- v0：新文件 ~350 行（含 41+ 条映射 + probe + 6~8 测试）+ 装配 20 行 + 矩阵/台账/CHANGELOG 4 处；
  1 个施工批次，与 W3.7 余项不冲突（A 类，不进契约变更队列）。
- A 方案：独立 ADR + 41 工具标签填充 + 插件迁移，单独立 wave，不在本批范围。
