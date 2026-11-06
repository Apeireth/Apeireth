[Document-Meta]
Document: docs/04-internal/01-handoffs/memory-release-checklist-2026-11-06.md
Class: DECISION
Status: 🟡 待裁决
Last-Modified: 2026-11-06
Author: deputy

# 记忆引擎发布验收清单（2026-11-06）

> **性质**：这是**验收清单**，不是发布公告。它把「记忆引擎可发布」拆成**可证伪的判据**、逐项给出**可复跑命令**与**实测数字**，最后给**选项与建议**，由 owner / 总指挥拍板。
> **本文件不改任何代码、不改任何既有文档**（只新建本文件）。
>
> **数字的两个来源，分开读**：
>
> | 标注 | 含义 |
> |---|---|
> | 2026-11-05 实测 | 记忆线收尾时的既有实测值，本文件照抄，不重测、不推测 |
> | 2026-11-06 复跑 | 本次在本 worktree 现场跑闸门得到的退出码与输出（复跑命令逐条给出） |
>
> **四级口径**（本文件统一用这一套，不许用「基本可用」「差不多成熟了」这类没有口径的话）：
> `NOT IMPLEMENTED < IMPLEMENTED < PRODUCTION WIRED < HARDWARE VALIDATED`，
> `DEFAULT ENABLED` 是**横切维度**（不是级），表示「默认打开」还是「默认关」。

---

## 一、「记忆引擎可发布」的可证伪定义

### 1.1 定义

**记忆引擎可发布**（本清单口径）= 下面 13 条判据**全部为绿**，且每条都有**一条可复跑命令**能当场判红：

| 编号 | 判据 | 可证伪形式 |
|---|---|---|
| P1 | 能力可达率不低于**已拍板的阈值** | 一个整数百分比，且阈值写在发布说明里 |
| P2 | 孤儿面**不增**（棘轮只减不增） | 孤儿条目数 ≤ 基线条数 |
| P3 | 融合通道**不许不涨** | 每个通道的 delta 不为负 |
| P4 | 单路 BM25 基线**不许相对劣化 > 1%** | 一个 recall@10 数字与基线逐字对照 |
| P5 | 状态声明（claims）零失败、零 stale | `fail=0` 且 `stale=0` |
| P6 | 文档口径**新增违规 0** | 一条扫描命令的退出码 |
| P7 | 守卫清单与实现一致 | 生成物与源同源（`--check` 说 in sync） |
| P8 | 门禁契约**required 条目全绿** | 38 条门禁 / 36 条 required 的清单 |
| P9 | fuzz 登记与 [[bin]] 一致 | 57 个 bin 的名字、路径、CI 矩阵三方对齐 |
| P10 | workspace 测试全绿 | `cargo test --workspace` 退出码 0 |
| P11 | 文档链接零断链 | 断链计数 |
| P12 | 文档分类绿 | 每份活跃文档有 Class 且满足该类强制项 |
| P13 | 文档真值零 stale | `stale=0` |

### 1.2 什么**不算**发布（防自欺）

| 不算发布的说法 | 为什么不算 |
|---|---|
| 「模块多、名字像、目录好看」 | 与生产可达性无关 |
| 「门禁全绿」 | 绿只说明**没有变坏**，不等于**有大用**（见 §四） |
| 「通道接上了」 | 接上不等于会触发；查表键错就是空接线 |
| 「内部卷分数涨了」 | 内部卷的题是从记录里挖出来的，带构造红利 |
| 「感觉成熟了」 | 不可复跑、不可证伪 |

---

## 二、发布候选物落在四级口径的哪一级

| 发布物 | 当前口径 | DEFAULT ENABLED | 证据（可查） |
|---|---|---|---|
| 联想召回通道（第三条召回路） | `PRODUCTION WIRED` | 未标注（本次未核） | `crates/engine/memory/src/associative_recall.rs`；查表键 = 问句实词，与写入侧 `associate:` 标记同口径 |
| 时间锚点对图事实生效（双时态） | `PRODUCTION WIRED` | 未标注（本次未核） | `KnowledgeGraphStore::facts_from_as_of`（trait 默认实现 + sqlite 真 SQL 覆写），区间判据 `fact_valid_at` 与 `bitemporal_graph` 的 valid-time 口径一致 |
| `signal_sources` 进运行时 | `NOT IMPLEMENTED`（生产未接线；库级 `IMPLEMENTED`） | 不适用（裁决不接） | 实测 NO-GO，见 §四 |
| 融合通道闸门（不许不涨的尺子） | `IMPLEMENTED`（已落地为脚本与基线） | 不适用 | `scripts/check_fusion_channels.py` + `scripts/fusion-channels-baseline.json` |
| 能力可达率 | 混合：133 个顶层模块里 70 个 `PRODUCTION WIRED` | 混合 | `scripts/check_memory_reachability.py` |
| 记忆引擎整体 | `PRODUCTION WIRED`（仅在「通道层」这个意义上） | 未标注 | 本文件 §一 P1 与 §六的裁决项 |

**整体口径的一句话版本**：记忆引擎当前是「**通道层 PRODUCTION WIRED，引擎层未达可发布阈值**」。

---

## 三、逐项验收清单

### 3.1 能力与效果面

| 验收项 | 判据（四级口径） | 怎么验（可复跑命令） | 当前状态（实测） | 差距 |
|---|---|---|---|---|
| **能力可达率** | 达到 `PRODUCTION WIRED` 的模块数 / 顶层模块数；口径方向：宁可漏报未接线，不可虚报已接线 | `python3 scripts/check_memory_reachability.py --check` | 2026-11-05 实测：70/133 = 52%。2026-11-06 复跑：**70/134 = 52%**，闸门绿（可达 70 > 基线 69，脚本提示建议抬基线） | 52% 远低于「多数能力在生产链上」的直觉；**阈值未拍板**（见 §五 P1-1）。基线文件仍钉在 69/132，落后实际一步 |
| **可达率基线同步** | 棘轮只减不增；基线不许比现状还低 | `python3 scripts/check_memory_reachability.py --check` 的输出末行；抬基线用 `--update` | 2026-11-06 复跑：脚本明确输出「建议 --update 抬基线」 | 基线低 1 个模块，则未来真掉 1 个仍会绿；抬基线会改 `scripts/memory-reachability-baseline.txt`（不是本文件能动的） |
| **孤儿面** | 孤儿 API 条目数不增（棘轮只减不增） | `python3 scripts/audit_unwired.py --check` | 2026-11-06 复跑：孤儿 API 基线一致（102 条，无新增、无过期） | 102 条孤儿里只有 53 条带逐条理由，其余 49 条**仍无逐条理由**（分组 G3=25 / G4=23 / G5=5 / G6=46 / G7=3） |
| **孤儿分组自洽** | 每条基线条目**恰好**落进一个组（不重不漏） | `python3 scripts/group_unwired_baseline.py --check` | 2026-11-06 复跑：分组一致（102 条） | 分组是人工判断，脚本只保证自洽，不保证判断对 |
| **融合通道效果** | 每条新通道 delta 不为负（不许不涨）；delta=0 允许但**绝不虚报它涨了** | `APEIRETH_DATA_DIR=/home/bai/.apeireth python3 scripts/check_fusion_channels.py` | 2026-11-05 实测：redchamber recall@10 0.2983 到 0.2983（delta +0.0000）；watershed 0.6875 到 0.6875（delta +0.0000）。2026-11-06 复跑：**完全复现**，退出码 0 | delta 全为 0.0000，则内部卷**区分不出**通道价值；只能证明「没有害」 |
| **单路 BM25 基线** | 相对劣化不超过 1% 即拦下；口径为**单路 BM25**（`golden_run::run` 直接调 BM25 源） | `APEIRETH_DATA_DIR=/home/bai/.apeireth python3 scripts/check_golden_baseline.py` | 基线 `scripts/golden-baseline.json` 钉住 redchamber recall@10 = 0.2983、watershed recall@10 = 0.6875；`python3 scripts/check_doc_numbers.py` 实测 6 个数字与基线**逐字一致**（含 delivery_rate 0.3099 / 0.6921，groundedness 0.0924 / 0.4758）。2026-11-06 复跑：退出码 0（当前 redchamber 476 题 recall@10 = 0.2983 / nDCG@10 = 0.1615，watershed 248 题 recall@10 = 0.6875 / nDCG@10 = 0.4817，判「OK: 检索分数未低于基线」；送达率是**观察项**，不参与闸门） | **通道级改动在这个闸门上看不见**（它只量单路 BM25），这正是新增融合通道闸门的原因，不能拿它当通道效果的证据 |
| **状态声明（claims）** | 声明零失败 | `python3 scripts/check_claims_verify.py --check`；配套 `python3 scripts/docs_truth.py --verify` | 2026-11-06 复跑（`docs_truth.py --verify`）：claims=51，fail=0，review_due=0，suspected_missing=1 | suspected_missing=1 提示「同目录下有未列出的文件变过」，需人工复核。`check_claims_verify.py --check`（要真跑 cargo 验声明）本轮**没跑完**：它要给每条声明编译并执行验证命令，编译量与整个 workspace 同级 |

### 3.2 工程与文档面

| 验收项 | 判据（四级口径） | 怎么验（可复跑命令） | 当前状态（实测） | 差距 |
|---|---|---|---|---|
| **文档真值（stale）** | 声明所引用的源文件哈希与现状一致 | `python3 scripts/docs_truth.py --verify --block-stale` | 2026-11-05 收尾实测：stale = 0。**2026-11-06 复跑：stale = 3，退出码 1（红）** | 3 条 stale：`mem.registered_namespace`（IMPLEMENTED）、`mem.retrieval_ranking`（PRODUCTION WIRED）、`mem.injection_contract`（PRODUCTION WIRED），共同原因是 `crates/engine/memory/src/coordinator.rs` 在 2026-11-05 的三笔融合接线（2858f82e / 467ff914 / 59bf7428）里变过，而 `docs/status/STATUS.md` 里记录的哈希仍是更早的 `42d3addfa0b8` / `65fd1caedc4b`，收尾时没有重新盖章。**这是发布前的硬阻塞项** |
| **文档口径** | 含「已实现 / 已接线」主张的段落必须同时带四级口径标注；新增违规 0 | `python3 scripts/check_doc_caliber.py` | 2026-11-06 复跑：新增违规 0（基线内欠账另计） | 本文件因文件名含日期被口径脚本**跳过扫描**，所以本文件的合规是**人工自律**，不是机器背书 |
| **文档分类** | 每份活跃文档有 Class，且满足该类强制项（DECISION 必须含「选项」与「建议」） | `python3 scripts/check_doc_class.py --check` | 2026-11-06 复跑：通过 | 无 |
| **文档链接** | 活跃范围内断链 0 | `python3 scripts/docs_links.py` | 2026-11-06 复跑：扫描 1550 个 markdown，断链 0，已知缺失 44（政策保留，不计断链） | 44 条「已知缺失」需定期复核，否则会沉淀成永久豁免 |
| **守卫清单** | 守卫清单与实现同源（生成物不许漂） | `python3 scripts/gen_guard_inventory.py --check` | 2026-11-06 复跑：in sync；58 guards（generator 8 / guard 36 / ratchet 14） | 无 |
| **门禁契约** | 清单里的门禁条数与 required 条数必须与实际 workflow 对齐 | `python3 scripts/gen_gate_manifest.py --check`；清单 `scripts/gate-manifest.json` | 2026-11-06 复跑：38 条门禁 / 36 条 required | 2 条非 required 的门禁**不在必跑集合**内，需确认是有意降级还是漏登记 |
| **fuzz 登记** | [[bin]] 的名字、路径、CI 矩阵三方一致 | `python3 scripts/check_fuzz_bins.py` | 2026-11-06 复跑：57 个 [[bin]] 全部一致（无孤儿、CI 矩阵覆盖） | 「登记一致」不等于「fuzz 跑过且有发现」，后者是另一条判据 |
| **workspace 测试** | 全 workspace 测试退出码 0 | `TMPDIR=/home/bai/tmp-test RUSTC_WRAPPER="" cargo test --workspace --locked` | 本次未跑（耗时长，交给 §五 P2-4 的执行批次） | 必须带 `TMPDIR=/home/bai/tmp-test`：`/tmp` 是 tmpfs，曾 100% 满，导致 `reflexion::tests::file_store_reports_store_busy_without_stealing_a_live_lock` 报 `No space left on device`（**不是代码缺陷**）。必须带 `RUSTC_WRAPPER=""`（sccache 坏） |
| **全量门禁** | 仓外门禁脚本退出码 0 | `bash /root/Documents/deepseek-harness/default-workspace/gates.sh /home/bai/Projects/apeireth-debt94-wt` | 本次未全跑（逐条挑跑，结果见 §3.1 / §3.2） | 该脚本在**仓外**，不在本仓库版本控制内 |

---

## 四、还不能发布的地方（诚实列）

| 缺口 | 实测数字 | 为什么它是缺口 |
|---|---|---|
| **融合通道量不出价值** | redchamber delta +0.0000、watershed delta +0.0000 | 闸门语义是 delta < 0 红、delta = 0 绿，则绿只说明**没有害**；内部卷区分不出这条通道有用还是没用 |
| **delta 是下界，不是真实收益** | 内部卷的题是从记录里挖出来的（主体 = 该记录的最高频实体） | 天然偏向词面命中，对「词面不像但该在场」的通道不利，所以 delta 只能当下界读 |
| **可达率只有 52%** | 2026-11-05 实测 70/133 = 52%；2026-11-06 复跑 70/134 = 52% | 近一半的顶层模块能力**没有生产消费点**；「引擎」这个词目前大于它的接线面 |
| **模块计数口径随提交变** | 2026-11-05 实测口径：133 个顶层模块（当时 HEAD = 4dea36ad，134 个 `.rs` 去掉清单文件 `lib.rs`）；2026-11-06 复跑口径：134 个（HEAD = f3914677，135 个 `.rs` 去 `lib.rs`），可达数仍是 70 | 两个口径的比率都是 52%，但**分母随提交变**；发布说明必须钉死「哪个提交 + 怎么数」，否则复核的人会以为自己数错了 |
| **融合链仍缺零件** | 17 个零件（清单见 `docs/04-internal/01-handoffs/memory-fusion-gap-2026-11-05.md` 第三节） | 其中 `bitemporal_graph`、`amem_graph`、`graph_algo`、`signal_sources`、`calibration`、`reflexion`、`dreaming`、`meta_thinking`、`persistent_vector`、`vector_rebuild`、`vector_distance`、`offline_embed`、`ensemble`、`session_lifecycle`、`principles`、`admission_gate`、`identity` 在 2026-11-05 当时是「建好了但没进生产链」。同日 A1（联想召回通道）与 A2（时间锚点对图事实生效）已接线、A3（`signal_sources`）实测 NO-GO 判不接，所以**缺口的最新数字需要重新推导**（仓内没有更新后的零件清单） |
| **`signal_sources` 实测是负的** | 同主题历史 -0.2056、实体共现 -0.3004、时间锚 +0.0000（出词 0 题）、段落首句 -0.0867、原文首句 -0.0907；基线 recall@10 = 0.6875 | 把信号拼进一个查询等于稀释加噪声；正确形态是**每个信号占一条独立通道、候选并集、统一打分**。生产运行时已是这个结构，所以没这个毛病 |
| **孤儿面没有清零** | 102 条，其中 53 条带逐条理由 | 另外 49 条只有分组号没有逐条理由，「有意保留」与「真债」仍然混在一起 |
| **文档真值现在是红的** | stale = 3，退出码 1 | 三条状态声明与 `coordinator.rs` 的现状不符；这是「发布说明说的和代码不是一回事」 |
| **可达率基线落后实际一步** | 基线 69/132，现状 70/134 | 真掉 1 个模块闸门仍会绿，棘轮暂时失效 |

---

## 五、发布前必须完成的动作（按依赖排序）

| 序 | 动作 | 依赖 | 性质 | 判据（做完必须为绿） |
|---|---|---|---|---|
| P0-1 | 决定工作树四个脏文件的去留：`crates/engine/memory/src/lib.rs`、`reflexion.rs`、`reflexion_tuning.rs`、`retrieval_tuning.rs` 均为已修改未提交 | 无 | **owner / 总指挥拍板** | 未提交的改动不进基线（否则棘轮把未发布代码钉进去） |
| P0-2 | 刷新 `docs/status/STATUS.md` 里三条声明的 source 哈希（`mem.registered_namespace` / `mem.retrieval_ranking` / `mem.injection_contract`） | P0-1 | **owner / 总指挥执行**（那是别人的文件） | `python3 scripts/docs_truth.py --verify --block-stale` 退出码 0、stale = 0 |
| P0-3 | 抬可达率基线：`python3 scripts/check_memory_reachability.py --update` | P0-1 | 纯执行（记忆线） | 基线写成与现状一致；再跑 `--check` 应为「一致」而不是「建议抬基线」 |
| P1-1 | 拍板「可发布」的**能力可达率阈值**（草案：通道层 rc 不设硬阈值，引擎级 release 设阈值并写进发布说明） | P0-3 | **owner / 总指挥拍板** | 阈值写成一句可判定的话，不写「基本接全了」 |
| P1-2 | 拍板发布**层级**：只发「通道层 rc」还是发「引擎级 release」 | P1-1 | **owner / 总指挥拍板** | 发布说明里写清发布层级与对应口径 |
| P2-1 | 建一份**不挖自内部卷**的外部考卷（题目来源与语料正文无关），才有资格量通道真实收益 | P1-2 | 纯执行（记忆线） | 新考卷上跑出非零 delta，或**如实报告仍为 0**（不许编） |
| P2-2 | 按外部考卷重算融合通道闸门，并按需抬 `scripts/fusion-channels-baseline.json` | P2-1 | 纯执行 | `scripts/check_fusion_channels.py` 退出码 0 |
| P1-3 | 拍板融合链剩余零件的去留（沿用 A3 先例：先量后接，量出负的就 NO-GO） | P2-1 | **owner / 总指挥拍板** | 每个零件有一句裁决（接 / 不接 / 待量） |
| P2-3 | 补齐孤儿面 49 条无逐条理由条目的逐条理由 | 无 | 纯执行 | `python3 scripts/audit_unwired.py --check` 与 `group_unwired_baseline.py --check` 同时绿 |
| P2-4 | 跑全量门禁与 workspace 测试 | P0-1, P0-2 | 纯执行 | `cargo test --workspace` 与 `gates.sh` 退出码 0（带 `TMPDIR=/home/bai/tmp-test` 与 `RUSTC_WRAPPER=""`） |
| P3-1 | 回写模块图谱：现文档仍写「132 顶层模块 + 81108 行」（`docs/04-internal/01-handoffs/memory-module-atlas-2026-11-05.md`），与 133/134 的现状不一致 | P0-3 | 纯执行（记忆线） | 图谱头部数字与 `check_memory_reachability.py` 的输出一致 |

> **表按依赖链排序**（P0-1 是唯一无依赖的起点）：P0-1 → P0-2 / P0-3 → P1-1 → P1-2 → P2-1 → P2-2 与 P1-3 → P2-4。
> **「81108 行」这个数字要连算法一起定**：2026-11-06 复跑用 `wc -l` 在几个变体下都没得到 81108（HEAD = f3914677 提交树去 `lib.rs` 递归 = 81160 行；当前工作树去 `lib.rs` 递归 = 81603 行；当前工作树只算顶层 = 72014 行），回写图谱时必须写明**行数是怎么数的**，否则下一个人复核会得出第三个数字。

### 5.1 复跑环境（不设这三个，跑出来的红不是产品的红）

| 变量 / 脚本 | 必须设成 | 为什么 |
|---|---|---|
| `TMPDIR` | `/home/bai/tmp-test` | `/tmp` 是 tmpfs，曾 100% 满，测试报 `No space left on device`（**不是代码缺陷**） |
| `RUSTC_WRAPPER` | 空字符串 | sccache 坏：不设它时 `cargo` 类门禁会**挂住**（实测 `sccache rustc -vV` 进程停在 S 状态不出，`check_claims_verify.py --check` 20 分钟零输出） |
| `APEIRETH_DATA_DIR` | `/home/bai/.apeireth` | 语料在仓外；默认 `~/.apeireth/novels` 在 root 下是残缺副本（只有 `redchamber.txt`、且字节流在偏移 1292092 处不是合法 UTF-8、且**缺 `watershed.txt`**），融合闸门会以「报告跑失败」退出 1，而不是给出可比分数 |
| 门禁脚本 | `/root/Documents/deepseek-harness/default-workspace/gates.sh`（**仓外**） | 全量门禁清单的单一来源 |

---

## 六、选项与建议

### 选项 A：现在就发 rc，把 52% 与 delta = 0.0000 写成「已知限制」

- 收益：最快拿到外部反馈，最早暴露接线问题。
- 代价：外部拿到的「记忆引擎」有一半能力没有生产消费点；融合通道的价值**不可区分**这件事会被带到发布说明里，等于把一个未解的度量问题公开挂账。

### 选项 B：先补一份「区分得出通道价值」的考卷再发

- 内容：建一份题目来源与语料正文**无关**的外部考卷（内部卷的题是从记录里挖出来的，带构造红利，delta 只能是下界）。
- 收益：第一次拿到通道的**真实**收益数字（涨 / 不涨 / 跌），此后「该不该接某条通道」才有尺子。
- 代价：需要新建考卷与题目，工作量集中在记忆线；发版时间后延。

### 选项 C：先把可达率拉过某个阈值再发

- 内容：例如把阈值定在 70%，则 133 × 70% ≈ 93 个模块需在生产链路上，按 70 计**还要再接 23 个模块**。
- 收益：发布时「引擎」这个词与接线面相符。
- 代价：阈值本身**未拍板**；接 23 个模块不是接线路，是接**消费点**，会横跨别的区；而且**接上去不等于有用**（A3 就是接上去反而降召回的反例）。

### 选项 D（分层发布）：只发「通道层 rc」，不声称「引擎级 release」

- 内容：把这次的三笔接线（联想召回通道、时间锚点对图事实生效、融合通道闸门）作为**通道层 rc** 发布；引擎级 release 的门槛写成「可达率阈值 + 外部考卷 + 剩余零件裁决」三件事。
- 收益：发布物与证据**一一对应**：通道层 rc 的三条新接线各有实测支撑，闸门各自可复跑；不需要为「52%」找说法。
- 代价：对外叙事变复杂；需要 owner 接受「先发小的」。

### 建议

**建议选项 D，并把选项 B 的最小版本（一份不挖自内部卷的外部考卷）作为「引擎级 release」的前置条件；不采用选项 A，也不采用选项 C。**

理由三条：

1. **证据只支撑通道层**。通道层 rc 的三条新接线各有可复跑命令与实测输出（联想通道的查表键修正、时间锚的 `facts_from_as_of` 双时态、融合闸门的 delta 全 0.0000 绿）。引擎级 release 需要的是「整体接上了、且有用」的证据，这两样目前都不存在：可达率 52% 且阈值未拍板，通道收益在内部卷上量不出来。
2. **选项 A 的代价被低估**。delta = 0.0000 在闸门语义里是绿，但绿的含义是「没有害」。把「没有害」当成「有大用」写进发布说明，正是这份清单要防的事。
3. **选项 C 的方向可能反了**。A3 已经证明「接上去」可能是负的（-0.09 到 -0.30）；先冲一个数字门槛再接消费点，很可能把有害通道一起接进来。若要提可达率，应该**一个模块一个模块量**（选项 D 的做法），而不是批量接。

---

## 七、我这次没有做什么（明确边界）

| 没做 | 原因 |
|---|---|
| 没有改任何代码 | 本文件是**清单**，不是实施 |
| 没有改 `crates/engine/runtime-assembly` 的接线 | 跨区，接线要别的区拍板 |
| 没有改 `docs/INDEX.csv` / `docs/status/**` | 那是别人的文件；本次**只报告**「索引需要重新生成」，不代跑 `docs_index.py --gen` |
| 没有抬任何基线（可达率 / 融合通道 / 孤儿） | 棘轮基线是总指挥的裁决面；本次只跑 `--check` 与 `--report`，不跑 `--update` |
| 没有跑 `cargo test --workspace` 与全量 `gates.sh` | 耗时；本次只跑了可当场复算的闸门逐条，结果逐条记在 §3 |
| 没有跨机器复现 | 只有本 worktree 一台机器的环境实测 |
| 没有新建语料、没有建新考卷 | 选项 B 是**建议**，不是本次交付 |
| 没有替上游盖章 | 本清单不替任何模块、任何区、任何上游结论背书 |
| 没有重测 A1 / A2 / A3 的历史结论 | 照抄 2026-11-05 的实测值 |

---

## 八、风险栏：判据假阳性 6 连（系统性，必须随判据一起读）

2026-11-05 记忆线出现过 **6 次判据假阳性**。这不是手滑，是**判据本身的系统性缺陷**，所以本清单的每条数字都必须带这三问：

| 假阳性模式 | 这次怎么咬人 | 本清单怎么防 |
|---|---|---|
| 限定名直查 | 漏**根级重导出** | 可达率脚本走「根导出映射 + 模块级传递闭包」 |
| 正则 `[a-z_0-9]+` | 漏**大写导出**（曾把可达率低估到 10%） | 同上，且判据写在脚本头部 |
| `\.name(` 判引用 | 漏「`use` 导入后直接调用」（差点误删活代码） | 孤儿面以脚本判定为准，且基线棘轮只减不增 |
| 逐文件 `git checkout` 落盘 | **静默覆盖上游同文件新代码**（吃掉过上游的安全修复） | 本清单不建议任何 checkout 式回退 |
| 「同名即同一物」 | `search` 命中的是**图像算法**（把「10 个生产入口」算成了假的） | 生产入口按 crate 边界与文件路径判定，不按名字 |
| 只扫 `crates/` | 漏 `fuzz/` / `examples/` / `benches/`（fuzz 编译红） | P9 单独跑 `check_fuzz_bins.py` |

**一条纪律**（适用于本清单全部判据）：

> 任何「计数 / 零引用 / 可达」类判据，**跨 crate 都不可信**；必须核三件事：**宿主类型**、**全部消费者面**、**落盘方式**。

---

## 九、相关文件

| 文件 | 作用 |
|---|---|
| `docs/04-internal/01-handoffs/memory-fusion-decisions-2026-11-05.md` | A1 / A2 / A3 的裁决记录（实测 NO-GO 的出处） |
| `docs/04-internal/01-handoffs/memory-fusion-gap-2026-11-05.md` | 17 个缺零件清单的出处 |
| `docs/04-internal/01-handoffs/memory-module-atlas-2026-11-05.md` | 逐模块图谱（头部数字已过期，见 §五 P3-1） |
| `docs/04-internal/01-handoffs/memory-engine-closure-ledger-2026-11-01.md` | 记忆引擎收尾台账 |
| `docs/status/STATUS.md` | 状态声明与 source 哈希（stale = 3 的所在地） |
| `scripts/memory-reachability-baseline.txt` | 能力可达率棘轮基线 |
| `scripts/unwired-baseline.txt` | 孤儿 API 棘轮基线（102 条） |
| `scripts/fusion-channels-baseline.json` | 融合通道基线（k=10，delta 全 0.0000） |
| `scripts/gate-manifest.json` | 门禁契约（38 条 / 36 条 required） |
