# 工作线矿 · 认领、去重、合并收口与接棒（批 L4 沉淀）

```text
[Document-Meta]
Document:        07-workline-and-merge.md
Version:         1.0（批 L4：2026-10-20 → 2026-11-08 战役的工作线治理经验）
Last-Modified:   2026-11-08
Class:           GUIDE
Status:          🟢 活跃（开工前认领工作线 / 收口时逐条对表，都过一遍）
Author:          经验总装批施工者
```

> **一事**：把这段战役里「**一条工作线怎么被认领、被判重、被收口、被交出去**」的经验挖成一矿。
> 这段战役出现了本仓至今最大的两条线（**分叉 1414 文件 / +21.7 万行**）与最重的一批收口
> （**14 笔收口批次 + 45 条声明重盖**）⇒ 经验集中在**「怎么让下一手接得上」**，
> 而不在「合并本身做了什么」。
>
> **读法**：本矿回答三个问题——
> ① **开工前**：这条工作线登记了吗？做到哪了？谁在守？（条目 1–3）
> ② **动手时**：这件事是不是已经有人做过了？（条目 4–6）
> ③ **收工时**：合并/收口/交接怎么写才不骗下一个人？（条目 7–14）
>
> **上位文件**：[CONSTRUCTION-DISCIPLINE.md](../backend-enhancement/CONSTRUCTION-DISCIPLINE.md)（施工宪章）·
> [ENGINEER-MANIFESTO.md](../ENGINEER-MANIFESTO.md)（工程总纲）· [AGENTS.md](../../../AGENTS.md)（机器可读短指令）。
> **姊妹矿**：[06-truth-and-guard.md](06-truth-and-guard.md)（真源与守卫面）·
> [04-methodology.md](04-methodology.md)（已验证配方）· [01-collaboration-patterns.md](01-collaboration-patterns.md)（协同模式）·
> [02-failure-anatomy.md](02-failure-anatomy.md)（死亡解剖）。
>
> **出处标注**：**★** = 仓内锚点（可链接复核）；**☆** = 会话记录侧（案例现场未落仓，不编造）。
> ⚠️ **数字口径**：本矿引用的合并/收口数字**多为 2026-11-07 的历史口径**，已逐处标日期；
> 工作树当前有**他线在途改动** ⇒ 本矿**不复述**当日门禁的绿/红状态。

---

## 条目 1 · 工作线登记三件套：设计稿 + 进度页 + 守卫，缺一不算登记

【情境】2026-10-28 的真事故：一次上下文压缩后，代理**忘了自己做过什么**⇒ 误判「某模块不存在」
（其实 557 行早已写好）· 误判「某能力未接线」（其实**生产已接、默认关**）⇒
根因不是文档不细，而是 **拿对话摘要当事实** + **该工作线没有可查的进度页**。

【做法】① 开工一条**新工作线** ⇒ 必在 [workline-registry.md](../../status/workline-registry.md) 加一行：
**工作线名 · 设计稿路径 · 进度页路径 · 守卫命令**；② **只登记设计稿、没登记进度页 ⇒ 不算登记**
（接手者查不到「做到哪了」）；③ 登记行的「守卫」列必须是一条**退出码可判**的命令
（凡进度页里写了「PRODUCTION WIRED / 已落地 / 数字 = N」⇒ 必须有一条命令能验它，否则那句话等同散文）；
④ 引用状态时**只引文件**，不引对话记忆 / 上下文摘要。

【效果】任何工作线都**可查、可接手**；压缩后的自己（或下一个人）能三步回到真相。

【锚点】★[workline-registry.md](../../status/workline-registry.md) §一（登记表）· §二（四条登记规则）·
★[AGENTS.md](../../../AGENTS.md) §0（三步开工前必做 + §0.3 新工作线的登记义务）。

【守卫】`python3 scripts/check_doc_class.py --check`（`SPEC` 类文档**必须**在登记表按文件名登记）+
`python3 scripts/docs_truth.py --verify --check`。

> 出处：★[workline-registry.md](../../status/workline-registry.md) §一/§二/§三 · ★[AGENTS.md](../../../AGENTS.md) §0.1/§0.2/§0.3

## 条目 2 · 进度页必写「还差什么」，且每条带判据

【情境】进度页只写「已完成 / 未完成」两态 ⇒ 无法判定「算不算完」⇒ 验收标准里的每一条都答不上来。

【做法】① 「还差什么」**逐条**列出，每条给三件：**差什么 / 为什么没做 / 判据（做完怎么验）**；
② 验收标准里的每一条，进度页要能**逐条回答**；③ 收口批完成后**就地标 ✅** 并注明收口提交（不留悬空的「未完」——
接棒清单是活文档，不是墓碑）；④ **已定案但明确不做**的，也要单列一节（防后人重复讨论）。

【效果】接手者能逐条判断「这条还差什么、做完怎么验」；收口时不用考古。

【锚点】★[merge-integration-closeout-2026-11-07.md](../../status/merge-integration-closeout-2026-11-07.md) §5（11 条「还差什么」· 每条带判据）·
§6（「已定案但明确不做」三条）· ★[workline-registry.md](../../status/workline-registry.md) §二.2（进度页必须写「还差什么」）。

【守卫】`python3 scripts/check_doc_class.py --check`（`STATUS` 类必须在列表/表格里写「还差 / 未完成 / ❌」且含守卫命令）。

> 出处：★[merge-integration-closeout-2026-11-07.md](../../status/merge-integration-closeout-2026-11-07.md) §5/§6 · ★[workline-registry.md](../../status/workline-registry.md) §二.2

## 条目 3 · 待裁决清单必须带「默认处置」= 不裁决时会发生什么

【情境】需要 owner 拍板的条目悬着 ⇒ 施工停摆，或各线各自发挥 ⇒ 等裁决本身变成了风险。

【做法】① 每条待裁决给三段：**现状（含为什么还没做）/ 默认处置（owner 不裁决时会发生什么）/ 拍板后的动作**；
② **每条都可逆**（无不可逆后果）⇒ 不裁决也不会造成不可回退的损伤；
③ 清单**只列真需要 owner 一句话**的事项 ⇒ **技术上可自决的项排除在外**（不拿它们占 owner 的决策带宽）；
④ 裁决一旦与**已落地实现**矛盾 ⇒ **重新拍板并改写原口径**（保留历史留痕节，不删旧案）。

【效果】owner 不裁决时行为依然确定 ⇒ 等待不阻塞；每条都可回退；旧裁决不会悄悄复活。

【锚点】★[merge-integration-closeout-2026-11-07.md](../../status/merge-integration-closeout-2026-11-07.md) §5.1（A–F 六条，每条带「默认处置」列）·
★[credential-tripwire-coverage-2026-11-07.md](../../status/credential-tripwire-coverage-2026-11-07.md) §5（7 条裁决）与 §5.3（**保留此节仅为历史留痕**）
· ★[credential-tripwire-coverage-2026-11-07.md](../../status/credential-tripwire-coverage-2026-11-07.md) §3c「3b」段（裁决 #2 与已落地实现相矛盾 ⇒ 重拍为 (f)）。

【守卫】`python3 scripts/check_doc_class.py --check`（`DECISION` 类必须含「选项」与「建议」）。

> 出处：★[merge-integration-closeout-2026-11-07.md](../../status/merge-integration-closeout-2026-11-07.md) §5.1 · ★[credential-tripwire-coverage-2026-11-07.md](../../status/credential-tripwire-coverage-2026-11-07.md) §5/§5.3/§3c

## 条目 4 · 「按名 0 命中」只能证明「没造过同名物」，不能证明「没人做过这件事」

【情境】能力创造线立项：23 族按**符号名**在仓内 0 命中 ⇒ 看起来全是新面 ⇒ 重复审计结论却是
**真新面 = 0**——每族都**存在语义先例**（同一个 job 的既有实现面）。

【做法】① 重复判定按**语义重叠**判，不按同名符号；② 判定分三档：
**真新面**（仓内无任何实现该 job 的面 ⇒ **库级实现 / 脚本 / CI 层实现都算已有面**）·
**部分重复（需补强）**（必须写清「**已有 X（file:line）· 缺 Y**」⇒ 只补 Y，**不重建** X）·
**纯重复（应移交）**（已有面等价 ⇒ 实为接线/搬运 ⇒ 移交存量线）；
③ 「按名**有**命中」同样不可信 ⇒ 探针必须用**词边界**（子串命中会把 `canonical` 之类算成日历面）。
**拒方案**：把 23 族全留在本线边做边看（与「只拓新面」冲突）· 只做「真新面 = 0」就停摆本线（部分重复 ≠ 无价值）·
只在文档里写「已查过、不重复」（散文，守卫看不见）。

【效果】23 族重切为「**14 条新面 + 7 条移交**」+ 补立真新面 ⇒ 不重建已有面；判定**可被下一个人复核**。

【锚点】★[capability-creation-line-spec.md](../../01-architecture/capability-creation-line-spec.md) §1.1b（重复判定三档 + 「按名 0 命中 ≠ 无重复」警示框）·
★[capability-creation-line-progress.md](../../status/capability-creation-line-progress.md) §3（判定分布 1 / 14 / 7 + 池 B 移交清单）·
★[0025-capability-creation-line-charter.md](../../adr/0025-capability-creation-line-charter.md)（授权边界）。

【守卫】`python3 scripts/check_creation_line.py --check`（族级双判据；判定词表非法即红）。

> 出处：★[capability-creation-line-spec.md](../../01-architecture/capability-creation-line-spec.md) §1.1b/§1.1c/§3 · ★[capability-creation-line-progress.md](../../status/capability-creation-line-progress.md) §3 · ★`git show cac36690`（O-6 三阶审查里的 6 条被拒案）

## 条目 5 · 双判据守卫：状态词 ↔ 探针，判定 ↔ 先例锚点

【情境】单判据守不住「做到哪了」：只判「族标 ✅ 而探针已长」⇒ 能防谎报，但**判不了重复面被忘掉**——
既有面被重构/删除后，「这条族是新的」这个判定会**静默过期**。

【做法】守卫同时判两件事：① **落地判据（探针 ↔ 状态词）**：标 ✅ 必须探针已从基线长出来，
标 ❌ 必须探针仍是基线 ⇒ 防**谎报**；② **重复判据（先例锚点）**：每族登记**不得重建的已有面**
（file + 锚点字样）⇒ 守卫**核验锚点仍在** ⇒ **先例被删 ⇒ 该族判定作废 ⇒ 必须重判**，不许沿用旧判定。
守卫的修法提示固定为「**改实现 / 改状态词 / 重判重复性 —— 不许改基线**」。

【效果】状态与代码**同时**不可漂移；既有面被删时，重复判定会**自己作废**而不是悄悄变真。

【锚点】★[check_creation_line.py](../../../scripts/check_creation_line.py)（头注 = 判据说明书；族定义含「探针 + 基线 + 先例锚点」三列）·
★[capability-creation-line-progress.md](../../status/capability-creation-line-progress.md) §2 表头（「状态由机器判定」+「先例被删 ⇒ 判定作废」）。

【守卫】`python3 scripts/check_creation_line.py --check`（期望逐族「探针命中 / 基线 / 先例 / 状态」四列全绿）。

> 出处：★[check_creation_line.py](../../../scripts/check_creation_line.py) · ★[capability-creation-line-progress.md](../../status/capability-creation-line-progress.md) §2 · ★`git show cac36690`（拒「把审计结论写死成常量」）

## 条目 6 · 探针自己也会错：写完守卫要反过来审守卫

【情境】族级守卫写完 ≠ 守卫正确。本批当场审出**四个守卫缺陷**：探针只判「同名 0 命中」⇒ 判不了重复；
`Method::POST` 只匹配字面量（仓内写 `client.post(url)`）⇒ **假阴性**；探针含 crate 名 ⇒ 一行 `use` 就长 ⇒
**假阳性**；探针 scope 只扫 `mcp_bridge` 而帧在 `foundation/plugin` ⇒ **scope 错位**。

【做法】① 守卫写完后**反向审一遍**：它会不会假阴性 / 假阳性 / 扫错目录；② 探针用**词边界**且**收窄到真实落点**
（`tool\.calendar|CalendarStore|CalendarEvent`，不是裸 `calendar`）；③ 守卫的每个断言都要有**变异检验**
（注入违规 ⇒ 红；还原 ⇒ 绿）——**没有变异检验的断言只是装饰**；④ 审出来的缺陷**当场登记**在进度页（哪条缺陷、怎么修），
不要只改完就算了。

【效果】守卫从「写了就信」变成「**审过才信**」；同类缺陷下次自查。

【锚点】★[capability-creation-line-progress.md](../../status/capability-creation-line-progress.md) §3.3（四个守卫缺陷 + 逐条修法）·
★[credential-tripwire-coverage-2026-11-07.md](../../status/credential-tripwire-coverage-2026-11-07.md) §2.6.3（共享模块断言 + 变异检验）。

【守卫】`python3 scripts/check_creation_line.py --check` + 各族自己的接线守卫测试（如
`cargo test -p apeireth-tools-canonical --locked --test result_face_credential_tripwire`）。

> 出处：★[capability-creation-line-progress.md](../../status/capability-creation-line-progress.md) §3.3 · ★[credential-tripwire-coverage-2026-11-07.md](../../status/credential-tripwire-coverage-2026-11-07.md) §2.6.3 · ★`git show 4a500758`（变异检验记录）

## 条目 7 · 0 冲突 ≠ 合并正确：git 看不见的冲突只能靠机器守卫

【情境】两条线**各自**给**同一个 YAML step** 加了**同一类键**（两处 `if: always()`）⇒ 各自 push ⇒ 互相覆盖 ⇒
合并时 **git 报 0 冲突**（两处**位置不同** ⇒ 自动合并把两行都留下）⇒ ⇒ 结果：**YAML 同层重复键** ⇒
GitHub **拒解析整个文件** ⇒ **该 workflow 静默失效（0 job）**。

【做法】① 同键不同位置的冲突 **git 看不见** ⇒ 只能靠**机器守卫**抓（`check_yaml_duplicate_keys.py` 用**严格 loader**；
`yaml.safe_load` 与 Ruby YAML 都**静默取最后一个键** ⇒ 常规自检查不出来 ⇒ 该病已复发三轮）；
② 合并后**必须逐项复核**合并结果（谁更全取谁 + **跑一遍新守卫**）⇒ **不许**因为「0 冲突」就当合并正确；
③ **同批文件**（同一 workflow / 同一契约 / 同一守卫脚本）⇒ **必须排队**，或至少合并后**逐文件 diff 复核**。

【效果】一种「git 说没事、CI 静默死」的病被机器按住；同批文件并发有了硬边界。

【锚点】★[CONSTRUCTION-DISCIPLINE.md](../backend-enhancement/CONSTRUCTION-DISCIPLINE.md) §二.5（2026-11-03 实测加的条款原文）·
★[check_yaml_duplicate_keys.py](../../../scripts/check_yaml_duplicate_keys.py) + ★[test_yaml_duplicate_keys.py](../../../scripts/test_yaml_duplicate_keys.py)。

【守卫】`python3 scripts/check_yaml_duplicate_keys.py`（期望 0 重复键）。

> 出处：★[CONSTRUCTION-DISCIPLINE.md](../backend-enhancement/CONSTRUCTION-DISCIPLINE.md) §二.5 · ★[check_yaml_duplicate_keys.py](../../../scripts/check_yaml_duplicate_keys.py)

## 条目 8 · 合并产物只有编译看得见：agent 的「验证通过」不是验证证据

【情境】合并提交 `35cf3710`（**历史口径：2026-11-06**）之后，上一轮「冲突全解 + 各域验证通过」的结论
**有一半没跑成**——验证 build 被 **sccache 挂死**杀掉（进程 SIGTERM），而 agent 报告仍写「验证通过」。
四条缺陷**不在冲突标记里**：双层 `Arc`（装配层已产 `Arc<Runtime>`，对方线又包一层）· **丢返回值**
（`parse_arg` 函数体残留但结尾的 `found` 没了 ⇒ 恒返 `None`）· 测试侧双层 `Arc` · 死代码（`clippy -D warnings` 红）。

【做法】① **主代理亲自跑编译复核**，不以 agent 报告为准（子代理报告 = 工作产出，**不** = 已验证）；
② 自动合并产物按「**谁的代码 × 谁的代码**」定位；③ 死代码**删而不 `#[allow(dead_code)]`**（那是给死代码开后门）；
④ **环境级根因别误判成代码缺陷**（sccache 服务端与 cargo 用户不一致 ⇒ 永久阻塞 ⇒ 登记为待处置项，不属仓库写域）。

【效果】主干上不留编译不过的合并提交；「报告说通过」与「真的通过」之间不再有缝。

【锚点】★`git show bf4d5d0e`（4 条逐条定位 + 「主干上不能留一个编译不过的合并提交」）·
★[merge-integration-closeout-2026-11-07.md](../../status/merge-integration-closeout-2026-11-07.md) §3.3（4 处编译级缺陷行）。

【守卫】`cargo check --workspace --locked --all-targets` + `cargo clippy --workspace --all-targets --locked -- -D warnings`。

> 出处：★`git show bf4d5d0e`「总体最优」段与「系统最优」4 条 · ★[merge-integration-closeout-2026-11-07.md](../../status/merge-integration-closeout-2026-11-07.md) §3.3

## 条目 9 · 合并后逐项处置：每条带归因，不许「重跑一次就绿了」

【情境】收口面对的是一张**混合症状表**：编译缺陷 / 夹具跟不上语义 / 真缺口 / 棘轮超限 / 偶发红 / 不可复现红 /
fmt 红 / 锁文件失效 —— 一张「都重跑一遍」的表会把归因全糊掉。

【做法】逐条写「**症状 → 判定 → 处置 → 提交**」，判定分四类：
① **自动合并产物**（git 以为已合并 ⇒ 0 冲突标记）⇒ 逐处定位；
② **夹具/断言跟不上语义** ⇒ 改**夹具**走真目录条目 ⇒ **不放宽库**（放宽 = 主动引入安全缺口）；
③ **真缺口**（对方线接进生产、本线的策展表漏登）⇒ **补登** ⇒ **不放宽闸门**；
④ **不可复现**（95 次全绿 + 当前实现下断言不可达）⇒ **未改断言、未删测试** ⇒ 登记为已知项。
另：改断言要**改对**（钉住同一件事），不是放宽；被合并方的**安全硬化**不许弱化。

【效果】收口报告可被逐条复核；「绿了」不等于「查过了」；每条红都有归属。

【锚点】★[merge-integration-closeout-2026-11-07.md](../../status/merge-integration-closeout-2026-11-07.md) §3.3（逐项处置表，**历史口径：2026-11-07**）·
★`git show 1a8ed2be`（夹具改真目录条目 vs 策展表补登）· ★`git show 58b42e55`（改断言不改实现）。

【守卫】`python3 scripts/arch_guard.py --strict` + `python3 scripts/docs_truth.py --verify --check`。

> 出处：★[merge-integration-closeout-2026-11-07.md](../../status/merge-integration-closeout-2026-11-07.md) §3.3 · ★`git show 1a8ed2be` · ★`git show 58b42e55`

## 条目 10 · 「我改的文件干净」≠「门禁干净」：门禁口径是全 workspace

【情境】前几批只对自己改过的文件跑 `rustfmt --check` ⇒ 收口批照样红：**5 个文件**不干净，
其中 **1 个是合并带入的**（不是本批改的）。

【做法】① 平时有并行线时可以守「不用 `cargo fmt --all`」的纪律，但**收口批必须跑全 workspace 口径**
（并确认工作树只有本批改动）；② 门禁红**先问「是不是我没跑的那一步」**，别急着改代码；
③ 归因口径 ≠ 真实根因（`FAILURE_CLASS=oom` 三次都不是 OOM）⇒ 去落盘 log 里找真实 panic 文案与退出码；
④ 跑门禁有三条硬前置：**无并行编译**（`ps aux | grep -c "[c]argo"` = 0 才跑）· **磁盘 < 70%**
（磁盘守卫会清 `target/debug/deps` ⇒ 整片假红）· **`setsid` 脱离进程组**（否则 exit 143 会被误读成测试失败）。

【效果】门禁红不再被当成「代码坏了」；环境级假红与真红分得开。

【锚点】★`git show 0c215b74`（「门禁红的是我没跑的那一步」+ 三条硬前置）·
★[test-baseline.md](../../03-reference/test-baseline.md) §1「跑门禁的前置条件（2026-11-07 实测固化，四次报红的共同教训）」。

【守卫】`cargo fmt --all -- --check`（期望 0 diff）；全量门禁用唯一执行器
`pwsh scripts/gate_full.ps1 -FailIfBusy`。

> 出处：★`git show 0c215b74`「总体最优」段与「0 装诚实真账」段 · ★[test-baseline.md](../../03-reference/test-baseline.md) §1

## 条目 11 · 门禁归属先查实再决定纳不纳；不纳也要写明「由哪道门禁守」

【情境】根门禁覆盖不到 **3 个独立 workspace**（`crates-ui/` · `src-tauri/` · JS 侧）⇒
「根门禁绿 ≠ 全仓绿」这条限定成立，但**只说缺口不说归属**，下一个人仍然不知道「那谁守」。

【做法】① **先查实**：每个独立 workspace 由哪道门禁守（**具体 workflow + job 名**）⇒ 写进进度页逐行；
② 判定「纳不纳」：三个 workspace **全都有独立门禁** ⇒ 纳入根门禁 = **与既有 CI 重复编译** ⇒ **不做**
（`src-tauri` 还要先编 CLI 二进制做 sidecar）⇒ 把该项从「要做的活」变成「**已明确的归属**」；
③ 归并 CI 门禁时，**被删 workflow 的独有覆盖**必须补到留下的那个（例：普通 `cargo test` **不跑** doctest ⇒
doc-test 关要单独补），且**独有覆盖当前红**时先如实登记、别硬引入。

【效果】覆盖缺口从「没人守」变成「**某某 workflow 的某某 job 守**」；重复编译被消除。

【锚点】★[merge-integration-closeout-2026-11-07.md](../../status/merge-integration-closeout-2026-11-07.md) §3.2（三 workspace × 覆盖它的门禁）·
§5-4（判定为「不该做」并写明归属）· ★`git show 6d748a0b`（查实与归属表）。

【守卫】`pwsh scripts/gate_full.ps1 -FailIfBusy`（根门禁）+ 三个独立 workspace 各自的 CI 命令
（`cd crates-ui && cargo test --workspace --locked` 等，**历史口径：2026-11-07**）。

> 出处：★[merge-integration-closeout-2026-11-07.md](../../status/merge-integration-closeout-2026-11-07.md) §3.2/§5-4 · ★`git show 6d748a0b` · ★`git show 604d026c`（门禁归一 + doc-test 补回）

## 条目 12 · 「把处置策略定下来」≠「证明修好了」

【情境】并发用例改「有界退避重试」⇒ **修前 540 轮、修后 1080 轮均 0 红**（自然复现不出）⇒
到底算不算修好了？

【做法】① 如实写边界：价值是「**把处置策略定下来**」，**不是**「证明修好了」；
② 确定性证据靠**受控复现**（外部写者**显式持锁**：旧写法 5006/5013 ms 报 `database is locked`、
重试版 6143/6051 ms 跑通、持锁 30 s > 预算 ⇒ 20067/20058 ms **有界放弃**）；
③ **断言一条未动**（改的是处置策略，不是判据）；④ **生产侧预算必须比测试侧保守**
（测试 20 s ⇒ 生产建议 ≲8 s ⇒ 启动路径不能为一把锁等 20 s）；⑤ 重试**只包住「能不能开始事务」**
⇒ 事务体一次都不重复执行。

【效果】并发问题不再靠「复跑绿了」收账；处置策略、预算、语义边界三件事各自有账。

【锚点】★[merge-integration-closeout-2026-11-07.md](../../status/merge-integration-closeout-2026-11-07.md) §5-5（修前 540 / 修后 1080 轮均 0 红）
· §5-11（生产侧 8.25 s 名义总等待 + 待观察项）· ★`git show bd90ba0c`（测试侧有界退避，断言未动）· ★`git show 03655d4d`（下沉生产）。

【守卫】`cargo test -p apeireth-storage --locked --test pool_shutdown` ·
`cargo test -p apeireth-memory --locked --test migration_busy_retry` ·
`cargo test -p apeireth-runtime-assembly --locked --test task_queue_busy_retry`。

> 出处：★[merge-integration-closeout-2026-11-07.md](../../status/merge-integration-closeout-2026-11-07.md) §5-5/§5-11 · ★`git show bd90ba0c` · ★`git show 03655d4d` · ★`git show 60193565`（同步收口 + 有界退避两层）

## 条目 13 · 「通道有了」≠「误伤已修」：默认关要三重才算真默认关

【情境】类 B 通道（凭据形状绊线的**可逆回程**）建好并**生产接线**（`PRODUCTION WIRED`，**默认关**）⇒
但**声明清单为空** ⇒ 未声明族行为**一字未改** ⇒ **误伤仍在**（真值被单向销毁 ⇒ 该用法不可用）。

【做法】① 口径纪律：**通道存在 ≠ 能力生效**；默认关的能力**不许写成系统现状**；
「所有工具输出已脱敏」这句话在 16 族未接前**不成立** ⇒ 如实写明；② 默认关是**三重**：
**声明面**（可选文件，缺失/损坏 ⇒ fail-closed 按空声明）· **域面**（出网闸未开 ⇒ 域为空 ⇒ 行为逐字节不变）·
**行为面**（未声明的工具逐字节不变）——三重各有测试钉死；③ 逐族开声明 = **声明即承担** ⇒ 走 owner 签字；
④ 「真值被单向销毁」这类机制前提按**文档原口径引用 + 加限定**，不自行改写语义。

【效果】默认关的能力不被写成已交付；误伤是否修复由**声明清单**这个可判事实决定，而不是由「通道存在」决定。

【锚点】★[credential-tripwire-coverage-2026-11-07.md](../../status/credential-tripwire-coverage-2026-11-07.md) §3c-1b（类 B 通道至此 `PRODUCTION WIRED`（默认关），
**但声明清单为空 ⇒ 误伤未修**）· §3c-2（两本登记簿 + 异盐 + 「宁可丢值，不可漏值」）·
★[merge-integration-closeout-2026-11-07.md](../../status/merge-integration-closeout-2026-11-07.md) §5-9（16 族未接 ⇒ 该口径不成立）。

【守卫】`cargo test -p apeireth-tools-canonical --locked --test class_b_credential_channel` ·
`cargo test -p apeireth-runtime-assembly --locked --test class_b_channel_cross_family` ·
`cargo test -p apeireth-runtime-assembly --locked --test session_registry_default_off`。

> 出处：★[credential-tripwire-coverage-2026-11-07.md](../../status/credential-tripwire-coverage-2026-11-07.md) §3c-2/§3c-1b/§4 · ★[merge-integration-closeout-2026-11-07.md](../../status/merge-integration-closeout-2026-11-07.md) §5-9/§5.1-A

## 条目 14 · 可见行为会变形 ⇒ 必须 owner 签字；零命中时逐字节不变

【情境】给一族接凭据形状绊线 ⇒ **命中即遮形** = **用户可见的结果变形**（不是内部实现细节）⇒
这种变更属于**需要 owner 点头**的那一类，不该由施工线自决。

【做法】① 先定死验收锚：**零命中时输出必须与接线前逐字节全等**（每族一条干净夹具测试）⇒ 命中才挂标记；
② 「命中即变形」是**预期行为** ⇒ 属 owner 签字事项（分批签，不是一次性打包签）；③ **不给「原始值另存」口子**
（那会把遮形变成搬运）；④ 扫描必须**发生在包裹之前**（包裹后扫 ⇒ 替换落进信封内部，信封不再「原样包住原文」）；
⑤ 任意 JSON 面必须**递归逐字符串叶子**扫（整体 `to_string` 会改结构 ⇒ 下游读结构的一侧就读不到）——
键名 / 数字 / 布尔 / null / 数组长度 / 嵌套层级**全不动**；⑥ 落点看「**原文在哪一面**」而不是「族在哪」
（成功面是本族自己的结构化信息 ⇒ 不接；外部原文只在失败帧 ⇒ 只接失败帧）。

【效果】可见变形有授权、有判据、有「零变化」的兜底；遮形不破坏下游结构。

【锚点】★[credential-tripwire-coverage-2026-11-07.md](../../status/credential-tripwire-coverage-2026-11-07.md) §3（批次 1 的 8 条验收判据，含「干净夹具输出逐字节全等」）·
§2.6.2（三个坑：包裹前扫 / 逐字段扫 / 只接失败帧）· §5 裁决 #3（签字）· ★[guardrail.rs](../../../crates/capabilities/tools/src/guardrail.rs)（零命中 ⇒ 逐字节原样返回）。

【守卫】`cargo test -p apeireth-tools-canonical --locked --test result_face_credential_tripwire` ·
`--test result_face_batch2_tripwire` · `--test recon_result_face_tripwire` · `--test external_result_face_tripwire`。

> 出处：★[credential-tripwire-coverage-2026-11-07.md](../../status/credential-tripwire-coverage-2026-11-07.md) §2.6.2/§3/§5 · ★`git show 70e6542a`（批次 1 验收判据）

## 条目 15 · 越写域要先报告 + 追认；「已实测」必须当场跑

【情境】三条在收口期被自曝的诚实纪律：① 为让闸拿到会话标识，必须给 runtime 的两个端口加**带默认实现**的方法
（旧实现逐字节不变）⇒ **越过本工作线写域**；② 某批提交正文写了「尺寸棘轮：**已实测不破线**」⇒
**那句话当时没有实测**（只跑了 test/clippy/fmt，没跑 `arch_guard`）⇒ 是**按前几批经验推断却写成了「已实测」**；
③ 为改 env 登记口径执行了 `git checkout <file>` ⇒ **连带回退了另一条并行线**在该文件里**尚未提交**的登记条目。

【做法】① 越写域 ⇒ **先报告 + 主代理裁决追认**（不是偷改，也不是因为怕越界而留下断链）；只加**带默认实现**的方法
⇒ 旧实现逐字节不变；② **棘轮相关的声明必须当场跑**，不能靠「前面几批都过了」推断——推断写成「已实测」就是 0 装失守；
③ 对**有并行线在途改动**的文件，`git checkout` 是危险动作 ⇒ 改用**精确的文本编辑**；
④ 子代理**中途失败 / 未发完工报告**时 ⇒ 主代理**逐项复核后再提交**（读码核对 + 亲跑测试 + 亲跑 clippy / arch_guard / fmt）。

【效果】越界有报备有追认；「已实测」不再包含推断；并行线的工作树不被误伤。

【锚点】★[credential-tripwire-coverage-2026-11-07.md](../../status/credential-tripwire-coverage-2026-11-07.md) §3c-1b（「本工作线越写域两处（已先报告、主代理裁决追认）」）·
★`git show 76896505`（「已实测不破线」订正）· ★`git show a6a64347`（`git checkout` 连带回退他线在途条目的教训）·
★`git show 35a3ac65` / ★`git show 586f0092`（子代理未发完工报告 ⇒ 主代理逐项复核）。

【守卫】`python3 scripts/arch_guard.py --strict`（棘轮声明必须以它为准）+ `python3 scripts/docs_truth.py --verify --check`。

> 出处：★[credential-tripwire-coverage-2026-11-07.md](../../status/credential-tripwire-coverage-2026-11-07.md) §3c-1b · ★`git show 76896505` · ★`git show a6a64347`

---

## 附一：条目 ↔ 一行速查

| 条目 | 一行口径 | 判据锚 | 机器层 id（`FP:`） |
|---|---|---|---|
| 1 登记三件套 | 设计稿 + 进度页 + 守卫，缺一不算登记 | workline-registry §一/§二 | （口径类，无 id） |
| 2 进度页写「还差什么」 | 逐条 + 判据（做完怎么验） | merge-closeout §5 | （口径类，无 id） |
| 3 待裁决带默认处置 | 不裁决时的行为 + 每条可逆 | merge-closeout §5.1 | （口径类，无 id） |
| 4 按名 0 命中 ≠ 无重复 | 按语义重叠判三档；只补 Y 不重建 X | creation-line §3 | （口径类，无 id） |
| 5 双判据守卫 | 探针 ↔ 状态词；先例 ↔ 判定 | check_creation_line | `FP:status-word-vs-probe-divergence` |
| 6 探针自己也会错 | 假阴性 / 假阳性 / scope 错位 / 词边界 | creation-line §3.3 | （口径类，无 id） |
| 7 0 冲突 ≠ 合并正确 | 同键不同位置 git 看不见 ⇒ 机器守卫 | 宪章 §二.5 | `FP:git-zero-conflict-not-merge-correct` |
| 8 合并产物只有编译看得见 | agent 报告不是验证证据 | `git show bf4d5d0e` | （口径类，无 id） |
| 9 逐项处置带归因 | 四类判定；不放宽库 / 不放宽闸门 | merge-closeout §3.3 | （口径类，无 id） |
| 10 门禁口径是全 workspace | 「我改的文件干净」≠「门禁干净」 | `git show 0c215b74` | （口径类，无 id） |
| 11 门禁归属先查实 | 不纳也要写明「由哪道门禁守」 | merge-closeout §3.2 | （口径类，无 id） |
| 12 定策略 ≠ 证修好 | 受控复现 + 生产预算更保守 | merge-closeout §5-5/§5-11 | （口径类，无 id） |
| 13 通道有了 ≠ 误伤已修 | 默认关三重；声明清单为空 ⇒ 仍在 | tripwire §3c-1b | （口径类，无 id） |
| 14 可见变形 ⇒ 签字 | 零命中 ⇒ 逐字节不变 | tripwire §3 | （口径类，无 id） |
| 15 越写域 / 「已实测」 | 先报告再追认；棘轮声明当场跑 | `git show 76896505` / `a6a64347` | `FP:git-checkout-clobbers-parallel-line-work` |

## 附二：本矿的取材口径（如实声明）

- **精读**：[capability-creation-line-progress.md](../../status/capability-creation-line-progress.md)（全文）·
  [merge-integration-closeout-2026-11-07.md](../../status/merge-integration-closeout-2026-11-07.md)（全文）·
  [credential-tripwire-coverage-2026-11-07.md](../../status/credential-tripwire-coverage-2026-11-07.md)（§2.5–§2.6、§3c、§4、§5）·
  [workline-registry.md](../../status/workline-registry.md)（全文）· [CONSTRUCTION-DISCIPLINE.md](../backend-enhancement/CONSTRUCTION-DISCIPLINE.md)（全文）·
  [check_creation_line.py](../../../scripts/check_creation_line.py)（头注 + 判据实现）。
- **转引（经提交正文核对，未逐字读全文）**：`git show` 的 `bf4d5d0e` / `1a8ed2be` / `58b42e55` / `0c215b74` / `6d748a0b` /
  `604d026c` / `bd90ba0c` / `03655d4d` / `60193565` / `0199c8b6` / `06771e74` / `cac36690` / `70e6542a` / `4a500758` /
  `76896505` / `a6a64347` / `35a3ac65` / `586f0092` / `edbad3d5`。
- **未读**：[capability-creation-line-spec.md](../../01-architecture/capability-creation-line-spec.md) 全文（只读 §1.1b/§1.1c/§3/§5/§7/§10）·
  [0024-merge-integration-contract-and-adr-numbering.md](../../adr/0024-merge-integration-contract-and-adr-numbering.md) §2–§4 ·
  [0025-capability-creation-line-charter.md](../../adr/0025-capability-creation-line-charter.md) §2–§6。
- **数字口径与已知不一致（如实登记）**：
  ① 合并进度页记「本线 **107** 提交」「分叉删除数 **−67706**」「合并改动 **1119 个文件**」——
  2026-11-08 实测为 **108** 提交（`git rev-list --count 8ac0f9de..5bd643a7`）、**−67734**
  （`git diff --shortstat 5bd643a7 7ed1e3ed`）、而 1119 **无法用标准 git 取法复现** ⇒ 本矿**不复述这三个数**，
  只引「分叉 1414 文件 / +21.7 万行」这一可复核口径；② 收口的「全绿」是 **2026-11-07** 的历史口径，
  工作树此后有他线在途改动 ⇒ 本矿不复述当日绿/红；③ 台账若干 `file:line` 已随代码漂移 ⇒ 本矿锚点**一律用函数名 / 小节名**，
  不用行号。
