# 真源矿 · 四级口径（「已实现 / 已接线」这类话）怎么保持为真（批 L4 沉淀）

```text
[Document-Meta]
Document:        06-truth-and-guard.md
Version:         1.0（批 L4：2026-10-20 → 2026-11-08 战役的真源与守卫经验）
Last-Modified:   2026-11-08
Class:           GUIDE
Status:          🟢 活跃（任何要写「已实现 / 已接线 / 数字 = N」的人开工前过一遍）
Author:          经验总装批施工者
```

> **一事**：把 2026-10-20 → 2026-11-08 这段战役里「**怎么让一句话保持为真**」的经验挖成一矿。
> 这段战役的主线不是功能，是**度量与声明的自我治理**：声明真源（`claims.yaml`）、棘轮（尺寸 / 依赖边 / env 预算）、
> 可达率量具、失败知识库、族级双判据守卫 ⇒ 它们的共同病灶只有一个——**数字与状态会悄悄变成散文**。
>
> **读法**：01 矿记协同、02 矿记死法、03 矿记裁定、04 矿记活法、05 矿记产品洞察；
> **本矿记「度量面」**——你写的每个数字、每句「已接线」，凭什么还是真的。
> 重叠处互相链接不重抄（[05-product-insights.md](05-product-insights.md) 洞察 2 已讲四级口径的**阶梯**，
> 本矿讲**怎么守住阶梯不塌**）。
>
> **上位文件**：[CONSTRUCTION-DISCIPLINE.md](../backend-enhancement/CONSTRUCTION-DISCIPLINE.md)（施工宪章）、
> [ENGINEER-MANIFESTO.md](../ENGINEER-MANIFESTO.md)（工程总纲）；
> **机器可读短指令**：[AGENTS.md](../../../AGENTS.md)。
> **姊妹矿**：[07-workline-and-merge.md](07-workline-and-merge.md)（工作线认领 / 去重 / 合并收口）。
>
> **出处标注**：**★** = 仓内锚点（可链接复核）；**☆** = 会话记录侧（案例现场未落仓，不编造）。
> **机器层**：本矿每条都对应 `.harness/knowledge/failure-patterns.yaml` 里的**失败模式 id**（`FP:<id>`）——
> 两层必须互引，守卫 `scripts/check_experience_anchors.py` 会判（见条目 9）。

---

## 条目 1 · 盖章不是刷时间戳：先实跑 verify_cmd 再重盖

【情境】一次收口批改了某条声明 `sources` 覆盖范围内的源文件 ⇒ 声明真源立刻报 `STALE` ⇒
手滑就会「批量把 `verified_at` 刷成今天的提交」——绿是绿了，但**盖章的含义被掏空了**。

【做法】① 盖章语义固定为「**这条声明在该提交上被复核过**」⇒ `verified_at` 是**证据基线**，**不是时间戳**
⇒ 必须**逐条实跑它的 `verify_cmd`**，记录实跑 passed 数；② 复核后**只改 `verified_at`**（除非复核真的发现
`level` / `default` / `switch_env` 变了 ⇒ 那才改，且要在 `notes` 里写清依据）；③ `notes` 追加「本批复核结论」
——让下一个人知道**谁在哪个树上核过什么**；④ **盖章只能填在代码提交之后**（`verified_at` 是 commit SHA，
填当前 HEAD 时 diff 还在 ⇒ 判据无效）⇒ 代码提交在前、盖章提交在后，**两笔分开**。
**拒方案**：批量刷 `verified_at`（盖章即装饰，机制失效，且会把真实的状态变化一起掩盖）；
顺手把 `suspected_missing` 也清零（那 2 条经查是启发式过宽，补列反而稀释 ⇒ 保持「提示不判失败」，留人工判断）。

【效果】`stale` 归零且**语义为真**；声明阶梯不被无谓降级；`notes` 留下可追溯的复核史。

【锚点】声明真源 = [claims.yaml](../../status/claims.yaml)（头部规则「`verified_at` 必须是 commit hash；
改完对应源码后必须更新它并跑 verify」）；首次大规模重盖的实战记录 =
[merge-integration-closeout-2026-11-07.md](../../status/merge-integration-closeout-2026-11-07.md) §3.4（**历史口径：2026-11-07**，
29 条 → 5 条 → 11 条三轮重盖）。

【守卫】`python3 scripts/docs_truth.py --verify --block-stale`（期望 `fail=0 stale=0`）。

> 出处：★[claims.yaml](../../status/claims.yaml)（头部 0 装口径）· ★[merge-integration-closeout-2026-11-07.md](../../status/merge-integration-closeout-2026-11-07.md) §3.4 · ★`git show 507653b4`（11 条逐条实跑表：passed 数 12/6/43/16/24/7/249/4/12/61/20）· ★[docs_truth.py](../../../scripts/docs_truth.py)（`--verify --strict --block-stale` 三档判定）

## 条目 2 · 「被触及」≠「已过期」：声明的复核判据是语义，不是时间

【情境】我删掉了一个 API ⇒ 它出现在某条声明的 `sources` 里 ⇒ 那条声明立刻 `STALE` ⇒
但声明**要说的那件事**（这个能力 `PRODUCTION WIRED`）**仍然为真**。

【做法】把两件事分开判：① **是否被触及**（`git diff <verified_at> -- <sources>` 有 diff ⇒ 只是「该复核了」）；
② **语义是否仍为真**（实跑 `verify_cmd` + 查生产装配点：能力**真被装配**且**真能生效**）。
⚠️ ② 有**前提**：**声明描述的入口本身没被删**——若被删符号命中本条声明，那不是「外壳被删」而是「**能力被删**」
⇒ ⇒ 复核前先逐条查「本条的 sources / notes 有没有命中这次删掉的符号」（实测 9/9 未命中才敢盖）。
只在 ② 变了才动 `level` / `default`；`level` 与 `default` 是**两个正交维度**（可以 `PRODUCTION WIRED` 且默认关）
⇒ 只改 `default` 时**不动 `level`**。合并期两处默认档订正都是**收紧方向**（ON→OFF，附代码取证）。
另：**不越权改别人的声明面**——只盖自己动过的那几条，其余报给归口线。

【效果】声明阶梯不被「文件动了一下」误伤降级；默认档的方向永远是收紧。

【锚点】★`git show 19bcc044`（「我删过的 API 在其 sources 里 ⇒ 声明仍为真」）·
★[merge-integration-closeout-2026-11-07.md](../../status/merge-integration-closeout-2026-11-07.md) §3.4（两处 ON→OFF 的代码取证）·
★[claims.yaml](../../status/claims.yaml)（`level` / `default` / `switch_env` 三字段并列）。

【守卫】`python3 scripts/docs_truth.py --verify --check` + 逐条实跑该声明的 `verify_cmd`。

> 出处：★`git show 19bcc044` · ★`git show 4ee386e7`（29 条复核记录 + 两处默认档订正）· ★[docs_truth.py](../../../scripts/docs_truth.py)（`LEVELS` 与 `default` 分离定义）

## 条目 3 · `verify_cmd` 空跑比红测更危险（红测会被人看见，空跑不会）

【情境】某条声明的 `verify_cmd` 用子串过滤一个**并不存在于任何用例名里**的标识 ⇒
实跑 `0 passed / 3 filtered out` ⇒ **exit 0 看起来是绿的，其实零校验力**。

【做法】① 每条声明的 `verify_cmd` 都必须**实跑一次并看 passed 数 > 0**（不是看 exit code）；
② 过滤串要**精确到用例名**（不许用「功能名」这种想当然的子串）；
③ 一次批里把全部声明的 `verify_cmd` **全量实跑一遍**，把「静默失效」变成「显式失效」。

【效果】声明的证据链真的有校验力；「盖章即装饰」这个病从制度上被堵住。

【锚点】★`git show 06771e74`（`gap.memory_update` 的 `verify_cmd` 空跑 → 换成真用例名 `update_episode`）·
★[claims.yaml](../../status/claims.yaml)（每条带 `verify_cmd` 字段）。

【守卫】`cargo test -p apeireth-gateway --locked --test panel_routes update_episode`（期望 1 passed，**不是** 0 passed / exit 0）。

> 出处：★[claims.yaml](../../status/claims.yaml)（每条带 `verify_cmd` 字段）· ★`git show 06771e74`「总体最优」②（空跑实例）与「系统最优」段（全量实跑一遍的动作）

## 条目 4 · 量具先自证：判据自污染会把数字变成谎话

【情境】新造一个「能力可达率」量具，第一版算出 51%，而另一份人工图谱是 42% ⇒ 两个数打架。

【做法】**不猜，逐条对账**⇒ 查出根因：`lib.rs` 是「**模块清单**」不是「模块」⇒ 它的 `pub mod` / `pub use`
让**每个**模块都成了它的出边 ⇒ 而 `lib.rs` 又被生产引用 ⇒ ⇒ **所有模块都变「可达」**（虚高）。
修法 = `lib.rs` / `mod.rs` **不作依赖边来源、且不入分母**；判据方向刻意选「**宁可漏报未接线，不可虚报已接线**」
（此处「接线」指**四级口径**里的 `PRODUCTION WIRED`，不是「有没有这个文件」）。
对账结果两边**各有偏差**：脚本可达 & 图谱不可达 = 25 条（图谱低估，漏了「生产直接引用模块路径」这种形态）·
脚本不可达 & 图谱可达 = 10 条（图谱判错，脚本更准）。

【效果】数字从「有个数」变成「**这个数可信**」；量具本身成了可审对象。

【锚点】★`git show bd944a9f` §三（判据自污染的根因与修法）与 §四（两两边逐条对账）·
★[check_memory_reachability.py](../../../scripts/check_memory_reachability.py) + ★[memory-reachability-baseline.txt](../../../scripts/memory-reachability-baseline.txt)（棘轮钉住可达模块数**下限** 69）。

【守卫】`python3 scripts/check_memory_reachability.py --check`（期望 69/132 = 52%，**历史口径：2026-11-05**）；
闸门有牙的实证 = 把基线抬到 70 ⇒ 退出码 1 + 点名「有 1 个能力掉出生产链路」。

> 出处：★`git show bd944a9f`（⭐「过程中抓到并修掉一个判据自污染」段 + 「与图谱 42% 的差异」逐条对账表）· ★[check_memory_reachability.py](../../../scripts/check_memory_reachability.py)

## 条目 5 · 「零引用」不是判据：判据修正比删代码更值钱

【情境】删一批死 API 后，「零引用」判据**第 6 次假阳性**：fuzz target 用了其中一个已删 API ⇒
「仓内零引用」的结论是错的。

【做法】① 判据本身要修正：`pub(crate)` **不是公开 API** ⇒ 不该进孤儿基线（用 `with_used` 定位收口）⇒
收集时**跳过受限可见性，但计数上报**（不静默丢弃 ⇒ 可审计）；② **「零引用」必须扫全部消费者面**：
`crates/` · `fuzz/` · `examples/` · `benches/` · `tests/` ⇒ **少扫一个面 = 漏一个消费者**（第 6 次假阳性就是漏了 `fuzz/`）；
③ **「零引用」≠「没接电」**：实测有 5 种假阳性形态（同名遮蔽 / 间接消费链 / 语言边界（如 PyO3 模块由 Python import 调用，
Rust 侧零引用是正常态）/ 模块自述 default-off / 名字出现≠真调用）⇒ 必须**限定名 + 宿主生产使用双重核对**；
④ 删 API **之前**在「**删除前的树上**」重验判据（级联会让中间层突然变孤儿 ⇒ 先在旧树验完才知道级联深度与停止条件）；
⑤ 删完**必须回头查** fuzz target / examples / 测试（因为「零引用」只说明「仓内没有」，不说明「没人用」）；
⑥ 基线要**逐条写明删除理由**（仓内有 `dead-code-reasons.txt` 专档），不许只删数字；
⑦ **分类错 ⇒ 处置错 ⇒ 后果错**：「缺接线」不能混进「死 API」清单（混了会被批量删 ⇒ 变成能力缺失）；
「判据误收」不能塞进「owner 裁定不接」通道（那会把判据错误伪装成 owner 决策）。

【效果】孤儿基线从 119 → 103 且**判据正确**；假阳性不再反复消耗施工力。

【锚点】★`git show 499a1ec7`（判据修正：`pub(crate)` 不是公开 API）· ★`git show f45f4d6f`（fuzz target 用了已删 API ⇒ 第 6 次假阳性）·
★[dead-code-baseline.txt](../../../scripts/dead-code-baseline.txt) + ★[dead-code-reasons.txt](../../../scripts/dead-code-reasons.txt)。

【守卫】`python3 scripts/check_dead_code.py`（期望 0 违规）。

> 出处：★`git show 499a1ec7` · ★`git show f45f4d6f` · ★[check_dead_code.py](../../../scripts/check_dead_code.py)

## 条目 6 · 棘轮只减不增：想加额度时先还债

【情境】合并后 13 条 size-grow、env 计数 161 > 预算 160 —— 看起来「抬一下额度就完了」。

【做法】① **先逐条分类再处置**（合并树行数 > 两侧 ⇒ 真增长；== 对方线行数且对方基线早已记着 ⇒ **导入已有裁决**，不是抬上限；
低于对方基线 ⇒ 纯假阳性），**绝不 `--update-baseline` 一把梭**（那会把「并集新增」洗成「正常」）；
② 真增长 ⇒ **拆文件降回基线以下**（按关注点切，不按凑行数切；用 `pub use` 原样再导出 ⇒ 外部调用点零改动）⇒ 棘轮**净收紧**；
③ env 撞线 ⇒ 先问「这个开关**该不该占产品命名空间**」⇒ 该改名的改名（bench 内部诊断开关不该占 `APEIRETH_*`），
**不抬预算、不拼前缀绕守卫**；④ **例外登记是临时通道**（带 `expires_wave`）⇒ 不用它装「永久性的并集新增」；
基线重生成后**同步退休**已失效的例外。

【效果】预算表不虚高；「想加额度」被翻译成「先还债 / 先改名」；棘轮真的只减不增。

【锚点】★[merge-integration-closeout-2026-11-07.md](../../status/merge-integration-closeout-2026-11-07.md) §3.3（13 条 size-grow 的分类与处置）·
★[contract.toml](../../architecture/contract.toml)（`[budgets]` / `[env].max_distinct_names = 160` = 唯一真源）·
★[arch-baseline.txt](../../../scripts/arch-baseline.txt)。

【守卫】`python3 scripts/arch_guard.py --strict`（期望 `VIOLATIONS 0 / STALE 0`，输出含 `exceptions=0`）。

> 出处：★[contract.toml](../../architecture/contract.toml)（`[budgets]` / `[env].max_distinct_names = 160` = 唯一真源）· ★`git show 80190015`（逐条分类表 + 拒 `--update-baseline` / 拒例外挂账 / 拒抬 env 预算）· ★`git show 53f0bbc4`（拆文件降回基线以下，size 84→79）

## 条目 7 · 守卫的三种形态与四条硬要求

【情境】想让一份文档「不许漂」，但不知道该写成什么形态的检查。

【做法】**三类形态**：① **一致性**（文档说的 == 代码 / 台账里的：声明真源互校、指标台账互校）；
② **完整性**（该有的都得有：断链检查、索引覆盖、`mod` 声明必须有文件、文档里的源文件路径必须真实存在）；
③ **棘轮**（只能变好，不能变差：尺寸 / 依赖边 / 孤儿面 / env 预算）。
**四条硬要求**：**机器可判定**（"文档写得好"不可判定 ⇒ 不能当守卫）· **失败要响**（非 0 退出 + 说哪一条不一致 + 给修法）·
**有真源**（要有可比对的另一半，不能自说自话）· **不许自我豁免**（发现不一致改代码或改文档，**不是**改基线）。

【效果】进度页里写「❌ 没做」反而是**守卫友好**的形态（可判定）；守卫真正要防的是
「**悄悄变成"做了"（或悄悄退回"没做"）而页面没更新**」。

【锚点】★[workline-registry.md](../../status/workline-registry.md) §三（有守卫的文档从没悄悄漂过 / 没守卫的文档漂了一整天）·
§四（守卫是什么 + 四条硬要求）· ★[docs_links.py](../../../scripts/docs_links.py) / ★[docs_index.py](../../../scripts/docs_index.py) / ★[check_doc_paths.py](../../../scripts/check_doc_paths.py)（完整性类实例）。

【守卫】`python3 scripts/check_guard_inventory.py`（清单与仓库现状同步；新增守卫必须已登记）。

> 出处：★[workline-registry.md](../../status/workline-registry.md) §三/§四 · ★[check_guard_inventory.py](../../../scripts/check_guard_inventory.py)

## 条目 8 · 守卫自己也会错：漏报比误报更危险

【情境】接线守卫按 `grep <内核助手字面量>` 认族 ⇒ 抽象一收口到共享助手，走共享助手的族**不被枚举** ⇒
守卫退化成「**少报型**」⇒ 批次 2 的 5 个族**全漏**⇒ 门禁绿着，而实际没接。

【做法】① 守卫的认族模式会**随抽象层次漂移** ⇒ 抽象收口后**必须回头扩模式**并把期望集写死（枚举型，不是少报型）；
② 族判定要按「**首段目录名**」而不是 `file_stem`（某族可能把接线放在子模块里 ⇒ 只取文件名会算成 `tripwire`）；
③ 共享模块**不得自己绑定**内核助手字面量（否则族枚举会多出一族 ⇒ 守卫红且**报错看不出真因**）
⇒ 入口由**族文件注入**真实函数项 + 守卫里加显式断言 + 做**变异检验**（注入违规 ⇒ 红；还原 ⇒ 绿）；
④ 守卫的红**先查真因**再判定「守卫坏了」——放宽守卫是最坏处置；
⑤ ⚠️ **「检测未接线的工具自己没接线」**：曾有一个孤儿审计脚本，其 `--check` 与 `--update` 写的键格式不一致
⇒ `--check` **永远报「新增 N 条」⇒ 这个闸从来没可能通过 ⇒ 也就从来没接进 CI** ⇒ **一个红着的、永远红的闸
比没有闸更坏**（它训练人忽略告警）⇒ 新守卫上线前必须先证明「它能绿」；
⑥ 告警噪声也是病：兜底规则要先在真数据上量噪声再收窄（仓内实测 29 条里 17 条命中 ⇒ 噪声 = 训练人忽略告警）。

【效果】守卫从「看起来在跑」变成「**确实在挡**」；收口抽象时不漏族。

【锚点】★[credential-tripwire-coverage-2026-11-07.md](../../status/credential-tripwire-coverage-2026-11-07.md) §2.5.3（弱化如实登记）·
§2.6.5（修回枚举型 + 三处刻意留白）· ★[evidence_faces.rs](../../../crates/capabilities/tools/src/evidence_faces.rs)（共享模块：扫描层）·
★[guardrail.rs](../../../crates/capabilities/tools/src/guardrail.rs)（共享助手）。

【守卫】`cargo test -p apeireth-tools-canonical --locked --test result_face_credential_tripwire`（含「恰为期望族集」断言）。

> 出处：★[credential-tripwire-coverage-2026-11-07.md](../../status/credential-tripwire-coverage-2026-11-07.md) §2.5.3 / §2.6.3 / §2.6.5 · ★`git show 4a500758`（共享模块上收 + 变异检验记录）

## 条目 9 · 人读层与机器层必须互引：一条教训不许有两个互不知情的副本

【情境】经验矿（人读层）与 `.harness/knowledge/failure-patterns.yaml`（机器层）各存一份教训 ⇒
机器层更新了、人读层没更新 ⇒ 下次排查照旧扑空；而 `docs/README.md` 早就写着这条流水线的最后一环
（「门禁检查同类坑二次出现时是否被引用」），只是**一直没实现**。

【做法】① 机器层每条失败模式带 `guard:` 字段（**哪个守卫能拦住它**）⇒ 把「知识指向的守卫必须真实存在」变成机器判定
（守卫改名 / 搬家 ⇒ 知识库悄悄过期 ⇒ 照着排查会去找一个不存在的检查器）；② 反向也要钉：机器层的每个 id
必须**被人读层引用**（`FP:<id>`）⇒ 两层不许各说各话；③ 引用 `FP:<id>` 的矿条目，锚点必须能指回机器层的真实文件。

【效果】一条教训只有**一个真源（yaml）+ 一处可读面（矿）**⇒ 两层同步；新增教训时**必须**同时落两层。

【锚点】★[failure-patterns.yaml](../../../.harness/knowledge/failure-patterns.yaml)（机器层真源）·
★[check_failure_knowledge.py](../../../scripts/check_failure_knowledge.py)（知识 → 守卫存在性）·
★[check_experience_anchors.py](../../../scripts/check_experience_anchors.py)（矿 → 机器层互引 + 段型 + 锚点可达，本矿自带的守卫）·
★[docs/README.md](../../README.md)（流水线的最后一环：门禁检查同类坑二次出现时是否被引用）。

【守卫】`python3 scripts/check_failure_knowledge.py` + `python3 scripts/check_experience_anchors.py --check`。

> 出处：★[failure-patterns.yaml](../../../.harness/knowledge/failure-patterns.yaml) · ★[check_failure_knowledge.py](../../../scripts/check_failure_knowledge.py) · ★[check_experience_anchors.py](../../../scripts/check_experience_anchors.py) · ★[docs/README.md](../../README.md)

## 条目 10 · 量具的假阳性六连（造量具前先对照这六条）

【情境】造量具（探针 / 评分 / 判重 / 排序实验）时最容易自欺 ⇒ 本仓已攒出六个**同源**变体。

【做法】造量具前逐条对照：
① `as i64` 把 f64 分数截断 ⇒ 信号恒为 0 ⇒ **假无差异**（`FP:as-i64-truncates-recency`）；
② 把「现在」写死成年份 ⇒ 与语料年份不符 ⇒ 又一次假无差异（`FP:hardcoded-now-vs-corpus-year`）；
③ 用**字符集** Jaccard 判重 ⇒ 短英文句子字符集高度重叠 ⇒ 大量误判重复（`FP:char-jaccard-overuse-for-duplicate`）；
④ 按 query 挑最相关段 ⇒ 题面只有主体词 ⇒ 挑出的段恰好不是答案段（`FP:o2-squared-characters-vs-dedup`）；
⑤ 只改索引顺序当排序实验 ⇒ 对排序完全无效 ⇒ Δ 精确为 0（`FP:diff-of-index-order-no-effect-on-bm25`）；
⑥ 把压缩子集传给按全局索引取值的函数 ⇒ 索引错位 ⇒ 一度得出荒谬「最优」（`FP:subset-index-mismatch`）。
**外加一条**：全库两两比较 = O(n²) × 题数 ⇒ 跑十分钟没结果 ⇒ 量具必须先估复杂度（`FP:quadratic-jaccard-times-cases`）。

【效果】每次造量具前先过这七条 ⇒ 不重复发明假阳性；量具的判据写得**别人能复核**。

【锚点】★[failure-patterns.yaml](../../../.harness/knowledge/failure-patterns.yaml)（七条 `FP:` id 逐条带 `prevention` 与 `guard`）·
★[spec-batch-status.md](../../status/spec-batch-status.md)（假无差异 / 假掉分的实测记录）·
★[d4_fusion_probe.rs](../../../crates/engine/memory/examples/d4_fusion_probe.rs)（量具本体）。

【守卫】`python3 scripts/check_experience_anchors.py --check`（保证七条 id 都在矿里，且矿的锚点可达）。

> 出处：★[failure-patterns.yaml](../../../.harness/knowledge/failure-patterns.yaml) · ★[spec-batch-status.md](../../status/spec-batch-status.md) · ★[injection-oracle-bound-2026-10-27.md](../../status/injection-oracle-bound-2026-10-27.md)

## 条目 11 · 基线与守卫自身也要被守（守门人自己不漏人）

【情境】守卫/生成物自己也会骗人 ⇒ 三条已踩：生成器输出了**已不存在的路径**（人改副本后守卫立刻红，真源本身错了）；
`verified_at` 写成 64 位 sha（git 报 `bad revision` ⇒ claim 永远判 STALE）；**只含文档的变更集**让校验器直接
`return 0`（文档类门禁被整体跳过 ⇒「SKIPPED 当 PASS」）。

【做法】① 生成器输出**必须先验证路径存在**再落盘；② `verified_at` 用**缩写 sha**（与文件既有风格一致）
⇒ 且该 sha 必须**在本地 git 对象库可解析**（远端 API 重建的提交本地没有 ⇒ `git diff` 报 `bad revision` ⇒ 永远判 STALE）；
③ **docs-only 变更也必须跑文档类门禁**（"跳过"只能写成显式的跳过，不许写成通过）；
④ YAML 的布尔味字面量**必须带引号**（`default: "OFF"` 而不是裸 `OFF`——裸写会让语义从字符串变成布尔，
机械替换时最容易踩）；⑤ ⚠️ **改了真源就要重生成生成物**：`claims.yaml` 是唯一真源，`STATUS.md` / `wiring.yaml`
是**生成物（禁手改）** ⇒ 改完真源不跑 `--gen` ⇒ **两道门禁同时红**（仓内活例：2026-11-07/08 两笔只改了
`claims.yaml` 而没重生成 ⇒ `--check` 与 `wiring_report --verify` 双红）——**这是「生成物与真源必须成对提交」
最省事的证据**。

【效果】守门人自己不漏人；机器可读层的「静默改语义」被堵住。

【锚点】★[failure-patterns.yaml](../../../.harness/knowledge/failure-patterns.yaml)（`FP:generator-emits-stale-path` ·
`FP:stale-verified-at-sha-too-long` · `FP:docs-only-return-zero-in-verifier`）· ★[claims.yaml](../../status/claims.yaml)（`default` 带引号）·
★[verify_ledger.py](../../../scripts/verify_ledger.py)。

【守卫】`python3 scripts/check_failure_knowledge.py` + `python3 scripts/docs_truth.py --verify --check`。

> 出处：★[failure-patterns.yaml](../../../.harness/knowledge/failure-patterns.yaml) · ★`git show 4ee386e7`（YAML 1.1 布尔味字面量 ⇒ 逐条补回引号，8 处）

---

## 条目 12 · 「要么真跑，要么显式声明为什么不能跑」：高口径声明不许靠豁免成立

【情境】声明里写着 `verify_cmd`，但校验器**只做 `git grep` + `git diff`，从不执行它** ⇒
一条声明可以标 `PRODUCTION WIRED` 而验证命令早已失败或被删，**没有守卫会发现**。

【做法】① 每条声明的 `verify_cmd` 要么**真跑**（要求 exit 0 + 输出有 `test result: ok` 且 0 failed），
要么在声明里**显式写 `verify_skip_reason`**（跑不了就说为什么跑不了）⇒ 不许沉默跳过；
② **高口径声明不许靠豁免/延后成立**（`PRODUCTION WIRED` / `HARDWARE VALIDATED` 必须真验通）
⇒ 否则「已接线」只是个词 ⇒ 二选一：把口径降到 `IMPLEMENTED`，或让 `verify_cmd` 真能跑通；
③ **探针不是真服务器** ⇒ 探针跑通**不构成** `HARDWARE VALIDATED`；
④ **组合门禁必须与参数顺序无关**（早退式分派会把组合调用变成假绿）⇒ 顺序依赖本身就是那次 bug 的成因。

【效果】四级口径里的「已接线」有真实校验力；跑不了的声明**自己说自己是跑不了的**，而不是看起来很绿。

【锚点】★[check_claims_verify.py](../../../scripts/check_claims_verify.py)（真值闸 H5 + `verify_skip_reason` 通道）·
★[claims.yaml](../../status/claims.yaml)（`verify_cmd` 字段）· ★`git show 15084b08`（拒「因为探针跑通就升 HARDWARE VALIDATED」）·
★[docs_truth.py](../../../scripts/docs_truth.py)（`--verify` 的组合参数分派：顺序无关修正）。

【守卫】`python3 scripts/check_claims_verify.py --check` + `python3 scripts/check_claims_verify.py --wired-gate`。

> 出处：★[check_claims_verify.py](../../../scripts/check_claims_verify.py) · ★`git show 15084b08` · ★`git show 113bca2c`（顺序依赖本身的成因与反证四步）

---

## 附一：条目 ↔ 机器层失败模式 对照速查

| 条目 | 一行口径 | 机器层 id（`FP:`） |
|---|---|---|
| 1 盖章不是刷时间戳 | 盖章 = 「该提交上被复核过」⇒ 先实跑 verify_cmd | `FP:blind-verified-at-bump` |
| 2 被触及 ≠ 已过期 | 复核判据是语义；`level` 与 `default` 正交 | （口径类，无 id） |
| 3 verify_cmd 空跑 | 看 passed 数，不看 exit code | `FP:verify-cmd-empty-run` |
| 4 量具先自证 | 根重导出致虚高 ⇒ 逐条对账 | `FP:root-reexport-inflates-reachability` |
| 5 零引用不是判据 | `pub(crate)` 不是公开 API；零引用 ≠ 没接电 | `FP:zero-reference-not-wired` |
| 6 棘轮只减不增 | 先分类再处置；拆文件不抬上限 | `FP:env-namespace-budget-exhausted` |
| 7 守卫三形态四硬要求 | 一致性 / 完整性 / 棘轮 + 四条硬要求 | （口径类，无 id） |
| 8 守卫自己也会错 | 漏报比误报危险；永远红的闸比没闸更坏 | `FP:guard-never-green-key-mismatch` |
| 9 人读层 ⇄ 机器层互引 | 一条教训不许有两个互不知情的副本 | （制度类，见条目 9） |
| 10 假阳性六连 | 造量具前对照七条 | `FP:as-i64-truncates-recency` · `FP:hardcoded-now-vs-corpus-year` · `FP:char-jaccard-overuse-for-duplicate` · `FP:o2-squared-characters-vs-dedup` · `FP:diff-of-index-order-no-effect-on-bm25` · `FP:subset-index-mismatch` · `FP:quadratic-jaccard-times-cases` |
| 11 基线与守卫自身 | 生成器先验路径；缩写 sha；docs-only 也跑门禁；改了真源要 `--gen` | `FP:generator-emits-stale-path` · `FP:stale-verified-at-sha-too-long` · `FP:docs-only-return-zero-in-verifier` · `FP:yaml-boolean-literal-quoting` · `FP:changed-truth-source-without-regenerating-artifacts` |
| 12 要么真跑要么说清 | 高口径声明不许靠豁免成立 | （口径类，无 id） |

## 附二：本矿的取材口径（如实声明）

- **精读**：[claims.yaml](../../status/claims.yaml)（头部与两条样例）· [merge-integration-closeout-2026-11-07.md](../../status/merge-integration-closeout-2026-11-07.md)（全文）·
  [credential-tripwire-coverage-2026-11-07.md](../../status/credential-tripwire-coverage-2026-11-07.md)（§2.5–§2.6、§3c、§5）·
  [failure-patterns.yaml](../../../.harness/knowledge/failure-patterns.yaml)（全文）· [repair-recipes.yaml](../../../.harness/knowledge/repair-recipes.yaml)（全文）·
  [workline-registry.md](../../status/workline-registry.md)（全文）· [docs_truth.py](../../../scripts/docs_truth.py) 头部与口径定义。
- **转引（经提交正文核对，未逐字读全文）**：`git show 507653b4` / `19bcc044` / `06771e74` / `bd944a9f` / `499a1ec7` /
  `f45f4d6f` / `80190015` / `53f0bbc4` / `4a500758` / `4ee386e7` / `check_failure_knowledge.py` / `check_memory_reachability.py` / `check_dead_code.py`。
- **未读**：51 条 claim 的全部 `notes` 全文 · `docs/architecture/exceptions.toml` 全文 · `crates/foundation/governance/**` 实现细节。
- **数字口径**：本矿引用的测试 / 可达率数字均标注了**日期与出处**；工作树当前有**他线在途改动**
  ⇒ 本矿**不复述**当日门禁的绿/红状态（那属于他线战场）。
