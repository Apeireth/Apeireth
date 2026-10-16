# I1 书→技能沉淀 —— 交付说明（技能卡模型 + 沉淀管线 + L2 封套调用面）

> `Status: ✅ 已交付（2026-10-13）`
> I 轴（吸收自动化）第一件：把知识吸收成**可调用技能**——「吸收战役」的产品化。
> 分期与后续各期见 [I-absorption-automation.md](I-absorption-automation.md)；
> 上游候选见 [吸收候选池](../absorption-candidates-2026-10.md)（TOP 候选 2「书→技能流水线」）。
>
> **战场声明（write_scope）**：主战场 = `crates/capabilities/tools/src/skill/**`（新件 6 文件）+
> 六个新测试套件；随行 = `tools/src/lib.rs`（模块登记）、`tools/Cargo.toml`（+ foundation/orchestration，
> 共享引用封套复用）、`tools/src/worktree_plan.rs`（`fnv1a64` 可见性 `pub(crate)`，零复制复用，行为零变）、
> `runtime-assembly` 四处接线（`tool_modules` / `production` / `self_status_source` / `permission_preset`）、
> `cli/src/lib.rs`（旋钮 + 授权 + 配置装配）；文档随行 = 本文 + I 轴分期 + user-manual / api /
> capability-switch-power-audit + 候选池与台账回写。**禁区零触碰**（frontend/**、CHANGELOG、
> .github/**、docs/design/**、HANDOFF-NOTES、kani playbook、research/verification、
> memory `reflexion`/`river_topology`、W3 与 J1 战场）。

---

## 1. 技能卡模型（知识沉淀单元）

`SkillCard{id, name, summary, triggers[], body, source, created_at}`（`skill/card.rs`）：

| 字段 | 语义 | 承诺 |
|---|---|---|
| `id` | 稳定身份（`skill-` + 卡名归一键的 FNV-1a 64 hex，**非密码学**，同 `plan_id` 口径） | 同名演进不变；hex 输出天然文件名安全 |
| `name` | 卡名（技能库唯一键，归一键 = 去空白 + 大小写折叠） | 创建撞同名即拒；演进按同名走 |
| `summary` | 一句话摘要 | 非空、限长 |
| `triggers[]` | 触发词（检索式注入的命中面） | 归一（去空/去重/保序）后 ≥1，逐词限长 |
| `body` | 正文（要点 / 操作步骤——沉淀的本体） | 非空、限长 |
| `source` | 来源 = **内容输入**三选一：`document` / `book` / `article` + 标题 | 标题非空；来源种类是枚举，命名即承诺 |
| `created_at` | 首次沉淀时刻（epoch 毫秒） | 版本演进不变 |

**坏卡拒**（`card_draft_refusal`，纯判定）：空名/空摘要/空正文/归一后空触发词/空来源标题、
超长（名 100 / 摘要 400 / 正文 40 000 / 触发词 64×24 / 标题 200 字符）、控制符、卡名带路径分隔符
——任一命中即**拒绝即帧、零落盘**。技能卡是长期资产，半成品不收。

**存储语义**（`skill/library.rs`，数据目录 `skills/`）：

- **每卡一 StoredDoc**（`<id>.json`，信封 `{name, version, compatible_versions, body}`），
  信封身份 = `skill-card:<id>`——串档打开即拒；**拒开语义**原样沿用 `stored_doc`：坏 JSON /
  串档 / 版本不符一律拒绝打开并报结构化错误，不静默迁移、不回退默认；
- 库扫描不砖化：打不开的卡**如实列拒**（路径 + 原因），好卡照读（确定性排序）；
- **重复名拒绝**：创建撞同名（含大小写/空白变体）即拒，既有卡一字不改；
- **版本演进 = 同名更新留旧版审计**：`update` 先把被取代的**整张旧卡**追加进 `audit.jsonl`
  （逐记录 jsonl，坏行读作不存在），再持久档原子写新版；`id` / `created_at` 不变，
  当前版本号 = 已存档条数 + 1；找不到同名卡即拒（fail-closed，创建/演进两条路，无静默 upsert）。

## 2. 沉淀管线（`tool.skill_distill`）

**分工（命名即承诺）**：提炼触发词 / 要点 / 操作步骤 = **模型侧**（调用方给 prompt 模板，产出草稿）；
**管线与落盘 = 工具侧**（`distill_into` 三拍）。来源=文档/书/文章（内容输入）。

```text
内容输入（文本 | 文件路径·只读通道）
   │ ① 来源采集: 二选一（多给/少给都拒）+ 词法限域（禁绝对路径/盘符 UNC/`..`）
   │    + 敏感面拒（凭据/密钥面共享拒帧）+ 超限拒（512 KiB，大书分章沉淀）
   ▼
模型侧草稿 card{name, summary, triggers[], body, source{kind,title}}
   │    （提炼 = 调用方 prompt 模板的活; 工具侧不冒充语义理解）
   ▼
② 结构化 structure_card: 坏卡拒 → 触发词归一（去空/去重/保序）→ 派生 id + created_at
   ▼
③ 入技能库 skills/（每卡一 StoredDoc, 拒开语义）
   ├─ create: 同名存在 → 重复名拒绝（既有卡一字不改）
   └─ update: 同名演进 → 旧版先留 audit.jsonl（留旧版审计）→ 新版持久档原子写
        （id / created_at 不变; 版本号 = 已存档条数 + 1; 审计先行, 存档失败即不演进）
   ▼
SkillCard{id, name, summary, triggers[], body, source, created_at}
   ▼ 调用面（上下文预算纪律: 绝不无差别全量注入）
触发命中（查询文本 ⊇ 触发词）/ 按需拉取（点名卡名, 查询键 ≥2 字符）
   │ select_for_context: 只取命中卡（命中数降序 + 卡名键/id 确定性次序, 截 limit）
   ▼
build_skill_disclosure: 逐卡 UntrustedEnvelope（固定警告头 + 显式边界 + 边界伪造转义）
   × EnvelopeBudget::from_total_budget_chars（每源 max(400, 总预算/4)，超额走分级省略措辞）
   ├─ tool.skill_search（只读档零审批: 命中披露 + no_match 零注入 + 卡名轻清单只列名字）
   └─ select_for_context + build_skill_disclosure = 复用点（I2 TurnStart 注入面挂这里）
```

## 3. 调用面 = L2 封套披露（检索式注入）

技能卡进入模型上下文**只有这一条路**（`skill/invocation.rs`）：

- **共享引用封套**：与记忆检索披露同一套 `UntrustedEnvelope`（`foundation/orchestration`）——
  固定警告头（内容无法影响措辞）+ 显式边界 + 边界伪造转义，卡内指令只能活在边界里；
- **共享预算换算**：`EnvelopeBudget::from_total_budget_chars` 与上下文预算同一参数、同一单位
  （字符），每源 `max(400, 总/4)`，超额走分级省略措辞（不装全、不假精确）；
- **绝不无差别全量注入**：未命中卡**零注入**（正文一个字不进上下文）；`skill_search` 的
  `catalog` 是卡名轻清单（只列名字、零卡体），命中卡才展开——上文「预算化注入」先例同口径；
- **复用既有检索融合面的披露原语**（封套 + 预算 + 省略措辞），最小新增只读工具
  `skill_search` 承载按需拉取；触发命中选择器是纯函数，I2 的 TurnStart 自动注入直接挂接。

## 4. 治理档（命名即承诺）

| 工具 | 档位 | 语义 | 落点 |
|---|---|---|---|
| `tool.skill_distill` | **写文件面（file_write 总闸族 + 审批档）** | 注册即闸：开关 + `file_write` 唯一写总闸 + 工作区根 + 技能库目录齐备才注册（子依赖主 + file_write，fail-closed）；**每次沉淀都停人工审批**（`SkillDistillApprovalHook` 唯一承载，无自动放行档，载荷 = 技能卡摘要）；风险映射（未授权冻结 `pipeline.pre_ask`）+ 单调边界 Guard（坏卡/来源越界拒绝即帧） | `skill/distill.rs` + `production.rs` + `cli/lib.rs` |
| `tool.skill_search` | **只读档** | 零审批（无 ask 档钩子）、零写盘（调用前后目录逐字节等价）；`read_only` 会话预设不拒（不入写能力名单） | `skill/search.rs` + `permission_preset.rs` |

## 5. 测试证据

（数字取自复跑输出；口径 = 各套件 `running N tests` 计数）

| 套件 | 例数 | 钉住的行为 |
|---|---|---|
| `tools/src/skill/*` 单测 | 11 | 结构化/身份稳定/坏卡拒/披露载荷/触发命中/选择器/库层创建·演进 |
| `tools/tests/skill_library.rs` | 6 | 落盘往返 / 坏卡拒开不砖库 / 版本戳·串档拒开 / 重复名拒绝 / 版本演进审计（两轮）/ 演进 fail-closed |
| `tools/tests/skill_distill.rs` | 9 | 文本→结构化卡+触发词归一 / 文件只读通道（来源字节不变）/ 越界·敏感面拒 / 来源二选一 / 坏卡零落盘零审计 / 创建重复名+演进审计 / 审批载荷 / 共用校验 / 错误即帧 |
| `tools/tests/skill_search.rs` | 7 | 触发命中注入且未命中零注入 / 封套披露+指令隔离+边界外零泄漏 / 预算内截断（分级省略）/ 按需点名拉取 / no_match 零注入+卡名轻清单 / 只读零写盘 / 空查询·越限拒绝即帧 |
| `tools/tests/skill_governance.rs` | 7 | 每次沉淀都人批（载荷=技能卡摘要）/ 坏请求不进审批 / 风险映射冻结未授权 / 单调边界 Guard / 只读档无 ask 档 / 授权播种语义 / 声明面合同 |
| `runtime-assembly/tests/skill_registration.rs` | 4 | 注册即闸（写总闸/开关/目录逐项）/ 声明面名实相符 / 名册如实（含缺目录 false）/ read_only 预设名单（写拒·读放） |
| `cli/tests/skill_knob.rs` | 3 | 旋钮链 fail-closed（沉淀子依赖写总闸）/ 治理档（沉淀人批·检索零审批·未开拒）/ 文档随行（手册+API 契约同文） |
| **合计** | **47** | 既有套件零回归另跑全量（见下） |

**门禁（校验带 `-j 2`）**：`cargo test --workspace --locked -j 2` / `cargo clippy --workspace
--all-targets --locked -j 2 -- -D warnings` / `cargo fmt --all -- --check` /
`scripts/check-neutral-terms.ps1` —— 复跑输出见 §7。

## 6. I 轴分期更新

- [I-absorption-automation.md](I-absorption-automation.md)：I1（本批 ✅）→ I2 调用面深化
  （TurnStart 触发注入 + 技能卡生命周期）→ I3 历史即记忆（与 C 轴协同）→ I4 技能生态
  （船坞市场面 + 星群匿名技能共享）；
- 随手记已由 C 轴入口面交付（[quick-capture-delivery.md](quick-capture-delivery.md)），本轴不重复；
- 台账回写：[质感吸收台账](../absorption-ledger.md)；开关审计表随行：
  [capability-switch-power-audit](../../02-guides/capability-switch-power-audit.md)。

## 7. 全量门禁复跑（2026-10-13 亲跑）

| 门禁 | 命令 | 结果 |
|---|---|---|
| 全量测试 | `cargo test --workspace --locked -j 2` | ✅ **exit 0；199 个测试目标 / 5360 例通过 / 0 失败 / 22 忽略**（2026-10-13 亲跑）。本批新增 6 套件 47 例在内；既有套件全数通过 = **零回归**（任务书「既有 186 套件」为批前基线口径，套件数随各线在途批继续增长，逐批自报实测数） |
| Clippy | `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | ✅ exit 0（零警告） |
| 格式 | `cargo fmt --all -- --check` | ✅ 干净（零 diff） |
| 中性词 | `scripts/check-neutral-terms.ps1` | ✅ 零命中（`OK ... added lines all neutral`） |
| Secret 扫描 | 随批不变（无新增凭据面；技能库只收内容输入） | ✅ 无新增敏感面（来源只读通道拒凭据/密钥面） |

**共树在途红记账（口径诚实）**：本批施工窗口内共树多线并行，工具面曾先后出现三处在途编译红 /
一处在途测试红（`content_feed/tool.rs` 与 `object_access.rs` 的半程态、`write_intent.rs` 的族词表
同源迁移半程态、`sdk/src/im/card.rs` 审批卡族标注渲染半程态）——**复跑时均已由对应线收敛**，
最终全量以 exit 0 收口。均非本批文件；本批对其零改动（宪章「他人文件不乱动」）。
