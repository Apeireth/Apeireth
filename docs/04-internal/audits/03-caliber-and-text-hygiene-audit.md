# 03 · 口径与文本可信度审计结论册

> **本册定位**：把「口径尾账（caliber / 守门级数 / 键值口径）/ 文档卫生（`.gitignore`）/
> 活跃文档过时扫描 / 宣称→证据矩阵」类审计的结论合成一册；保留逐条结论、清单、扫描口径与
> owner 决策位，并逐条复核「今天还成立吗」。
>
> **审计对象与日期**（各份来源日期按原样标注，不改写）：
>
> | # | 来源（原路径） | 日期 | 篇幅 |
> |---|---|---|---|
> | 1 | `docs/04-internal/backend-enhancement/audit-gap-caliber-hygiene-2026-10-16.md` | 2026-10-16（含 2026-10-18 守门口径补测附录） | 242 行 |
> | 2 | `docs/04-internal/gitignore-hygiene-audit-2026-08-28.md` | 2026-08-28 | 220 行 |
> | 3 | `reports/audit-2026-08-19/active_docs_audit.md` | 2026-08-19 | 480 行 |
> | 4 | `docs/04-internal/claims-evidence-matrix.md` | 文件未标日期（内含 2026-10-20 复核注记） | 306 行 |
>
> **审计方法**：现值面 vs 历史卷宗双口径扫描（现值面 = 根文件 + `00-backlog` + `docs/03-reference/`
> + `docs/02-guides/`；历史卷宗 = CHANGELOG 日期条目 / commit-msg 档案 / `docs/archive/` /
> `library/` / `legacy/` / `reports/` / 带日期的内部报告），命中即 file:line 复核；
> 宣称侧逐条压缩为可验证命题并按证据强度定级（A/B/C/D/E/F 六级）。
>
> **复核基线**：2026-10-23，仓内事实核验（工作树 HEAD `b4aabfc2`）。
> 测试数字一律引用 [test-baseline.md](../../03-reference/test-baseline.md)，不抄写。
>
> **结论摘要**
>
> 1. **「X 重守门」在库内有三个语义**，本批只统一「守门级数」一义；2026-10-18 的代码侧补测给出
>    第三种答案：**v2 实装是「门值集合」，不是固定层数**——层数（7 / 9）是 v1.0 时代 lineage 标签，
>    v2 不作工程断言。owner 三选一**仍未拍板**（建议 A：现值口径改为「门值集合 + 单调哨兵」）。
> 2. **键值口径已被 2026-10-11 之后的新事实覆盖**：口径尾账批把「13 键」定为现值，而今日现行键集是
>    **14 键**（代码常量实测 14 元素；哲学锚 O-1 描述已写「键集（现行 14 键）」）。
>    另发现一处**新残留**：根 `Cargo.toml` 的 `verdict_cache_keys` 仍写 13。
> 3. **字母双占（D13）已清**：吸收自动化「I 轴 → P 轴」，16 文件同步（文档 9 + src 注释 7），
>    两个文件名保留 + 内容别名注记防断链 —— 今日扫描零命中，**已闭环**。
> 4. **大体量资产（F-02）已部分执行**：方案表四选项曾「只出方案不动文件」；今日
>    `reports/` 已从千余件 / 数十 MB 降到百件级 / 数 MB，脚本大件亦已瘦身，
>    **但 `previews/`（数十 MB 实拍图）仍在库**。
> 5. **`.gitignore` 卫生结论三条仍未销**：3 个真漏（孤儿目录 / 半漏目录 / 根研究文件）**至今未加**，
>    `*.py` **过宽规则仍在**；`legacy/` 保持 tracked（v1 参考链，不 ignore）这一判断仍成立。
> 6. **对外文本的可信度战场已从「性能数字」转到「默认路径口径」**：宣称→证据矩阵的 77 条分级中，
>    A 级仅 1 条且带 4 项保留；对外发布物里的无源性能数字已处置（README 撤表 + 发布注记替换），
>    但「0 hollow stubs」「空引擎 = Allow」「回滚 100%」三条诚信红线**至今仍被实现自述否定**。

---

## 1. 落差清账·口径尾账批（B-04 / B-05 / D13 / F-02，2026-10-16）

**来源**：`docs/04-internal/backend-enhancement/audit-gap-caliber-hygiene-2026-10-16.md`
**性质**：外部审计「宣称与实现落差清单」尾账批（守门级数口径 / 键值口径 / 字母双占 /
大体量资产盘点），文档 + 轻文件操作批；不 commit。纪律：中性表述 / 扫描零命中（现值面）/
校验带 `-j 2` / 历史不追溯改写 / 测试口径单一真源 0 触碰。

### 结论（原文要点保留）

**四件对照表**

| # | 审计项 | 审计原状 | 本批动作 | 状态 |
|---|---|---|---|---|
| B-04 | **守门级数口径**：5 重 / 6 重 v6 / 7 重 v7 三口径并存（分布在 commit-msg 档案、CHANGELOG、主账 FG、OSS_NOTICE、v1.0 借用台账、ROADMAP 等） | 现值统一 **7 重守门 v7**（含 Colang DSL + 技能守卫），**ROADMAP §13「守门级数」行 = 唯一现值出处**；历史文档逐处加「历史口径（日期）」标注、原值不动；v1.0 时代 9 重 v9 留痕加历史口径标注或登记 0 触碰（LOCKED 锚本体） | ✅ 已清 |
| B-05 | **键值口径**：ROADMAP 现值写「12 键 → 13 键」，多处历史文档写「13 键 LOCKED」；R11 baseline 3 值只有标签、无可复现入口 | §13 键行现值统一「13 键（LOCKED）」（12 键降为行内历史口径）；R11 baseline 3 值加「**无公开复现入口，仅供内部对照**」如实标注 | ✅ 已清 |
| D13 | **字母双占**：`00-backlog` 想法清单「吸收自动化（I 轴）」与星港线「I」同占 I 字母 | 吸收自动化统一改名 **P 轴（Practice 沉淀）**，分期 **I1-I4 → P1-P4**，全库引用同步（16 文件：文档 9 + src 注释 7 文件 8 处）；两个文件名保留 + 内容别名注记防断链；星港线保留 I | ✅ 已清 |
| F-02 | **大体量资产盘点**：`reports/`（审计时 1069 件 38MB）、`previews/`（39MB）、`scripts/guard_ml/*.jsonl`（21MB）随库提交 | **只出方案表不动文件**：构成清单 + 四选项（迁 release 资产 / 迁 archive / 外置 / LFS）逐项建议；引用面与执行前置一并查明 | 🟡 方案已出，待 owner 拍板 |

**B-04 历史标注清单（14 处，原值全部保留）**：CHANGELOG 三处（2026-08-27 / 2026-08-27 / 2026-08-10）、
commit-msg 档案三处、主账 FG 一处、OSS_NOTICE 两处、v1.0 借用台账一处、
`Cargo.toml` 三处注释（值本体 0 改）、`docs/02-guides/` 四处「5 重 = CI 五门，非守门级数」口径注。

**B-04「X 重」三语义辨析（保留）**：①守门级数（layer lineage，现值统一）；②CI 五门
（保留，文档加口径注区分）；③`Gate` 枚举五层（代码语义本体，不动）。
**登记不代改的异口径留痕点**：哲学锚本体（`philosophy.md` O-1「9 重 v9 + 13 键 + 3 不可变脊柱」）、
`eight_anchors.rs` 锚描述字符串、`gate.rs` / `lib.rs` 核心注释与测试、v1.0 发布脚本族、
v1.0 资料库档案、`legacy/` 与 `reports/` 历史卷宗、`Cargo.toml` metadata 的 `hard_walls` 与
`guard_gates_version` 值（值不改，仅加行尾注释）。

**B-05 边界**：13 键为 LOCKED 5 项之一 —— **键集与 `verdict_cache_keys = 13` 值 0 触碰**，只统一表述；
R11 baseline 3 值数值 0 触碰（`0.8682 / 0.8532 / 0.9063` 原样保留），仅加诚实标注。

**D13 改名映射**：I 轴 → **P 轴（Practice 沉淀）**；I1 书→技能沉淀 → **P1**（已交付件，行为 0 变）；
I2/I3/I4 → **P2/P3/P4**（仅改名）；星港 I / I 批 1-5 **保留 I**。
全库引用同步清单 16 文件（含 `00-backlog.md`、`I-absorption-automation.md`、
`I1-skill-distill-delivery.md`、吸收候选/台账、quick-capture 交付记录、
`docs/03-reference/capabilities-matrix.md`、`docs/02-guides/user-manual.md`、
`docs/02-guides/frontend-data-contract.md`，以及 src 侧 `skill/{mod,card,distill}.rs`、
`tools/src/lib.rs`、`cli/src/lib.rs`、`production.rs`、`tool_modules.rs`）。

**F-02 构成清单（2026-10-16 实测）**
- `reports/`：**1071 tracked**（盘上 1079）/ **39,894,060 B（38.0 MiB）**；
  按类：`agent-*` 子代理报告 450 件 / 27.6MB；UUID 命名工作日志 ~150 件；`decision-*` 136 件 / 1.85MB；
  `rNN-*` 124 件 / 1.2MB；`1.0-release-*` 21 件；`round*` 30 件；`achievement-*` 18 件；
  `dispatch-*` 4 件；非 md 资产 37 件（PNG 截图最大 1.53MB + `.log/.html/.yml/.txt`）；
  子目录 7 个。按时代：v1.0 时代 797 件 / 32.5MB、UUID 工作日志 83 件 / 0.62MB、
  无日期/其他 189 件 / 4.7MB、2026-09 4 件、2026-10 6 件。
- `previews/`：**34 tracked**（全 PNG）/ **40,937,344 B（39.0 MiB）**；
  `intro-*` 21 件 / ~23.3MB、`mode-*` 7 件 / 5.7MB、`tauri-*` 3 件 / 5.8MB（单张最大 3.91MB）、
  `backend-*` 3 件 / 1.5MB、昼夜桥 2 件 / 2.4MB、旋转证明 0.36MB。
- `scripts/guard_ml/*.jsonl`：4 大件（组内共 13 文件 tracked）/ **21,997,011 B（21.0 MiB）**
  （动作样本 9.64MB / 生成特征 7.95MB / 轨迹样本 3.56MB / 对样本 0.85MB）+ 4 个脚本 + 若干小件。
- 旁注 `artifacts/`：361 件 / **157.2MB**（审计转储）—— 体量超上三组总和，建议并入同一处置批。

**F-02 引用面与硬依赖**：①**编译期硬依赖（必留库）**：guard 安全场景测试用 `include_str!` 引用
`scenarios.jsonl` + `fixture_artifact.json`，移走即测试编译失败；②**CI 重生成语义**：guard 工作流
每次先跑导出示例**重生成** 4 大 jsonl → 训练/自检 → 校验存在性与隐私不变量 → 上传 artifact，
故 4 大 jsonl 是**可再生证据副本**，出库后 CI 不缺件；③**文档引用面**：`docs/` 内指向 `reports/`
的引用行 **711**（活跃文档面约 30 件具体报告）；`previews/` 被设计交接文档与入场动画文档引用；
④**工作流引用**：发布工作流上传 `reports/` 下的发布清单与 TP20 输出，另有三个工作流把 `reports/**`
列为排除面（目录口径依赖）；⑤既有立场：`.gitignore` 卫生审计认定 `previews/` 应保留；
产品化评审已点名「`reports/` 里上千个 AI 工作日志被 git 跟踪」。

**F-02 去留方案表（四选项逐项评估，建议列）**
| # | 资产 | 建议 | 理由要点 |
|---|---|---|---|
| 1 | `reports/` v1.0 时代批量工作日志（~950 件 / ~35MB） | **迁 release 资产**（外置为备选） | 迁 archive 只挪目录不减仓且 711 处引用改写成本高；LFS 保路径零断链但历史 blob 不减 |
| 2 | `reports/` 近期证据/验收报告（~50 件 / ~5MB） | **留库** | 体积可控、活跃文档引用，出库即断链 |
| 3 | `reports/` 非 md 资产（37 件 / ~2MB） | **迁 release 资产** | 与 #1 同批同包 |
| 4 | `previews/` 34 PNG / 39MB | **LFS**（次选：迁 release 资产 + 留代表图） | 迁 archive 不减仓；外置会断 `docs/design` 引用；LFS 与「应保留」既有立场兼容 |
| 5 | `scripts/guard_ml/` 4 大 jsonl / 21MB | **外置**（CI 每次重生成 + artifact 兜底） | **必留库**：`scenarios.jsonl` + `fixture_artifact.json`（编译期依赖）+ 对/否定样本 + 影子夹具 + 4 个脚本 |
| 6 | `artifacts/` 361 件 / 157MB | **外置**（随同一处置批） | 同口径 |

**执行批最小动作清单（owner 拍板后）**：①引用审计（出 711 引用面中需改写/留 stub 的清单）；
②打包挂 Release + 目录口径同步；③`git lfs migrate` 或逐件 track；④大 jsonl 出库 + 工作流校验步措辞同步；
⑤逐批跑全量门禁收口。

**扫描与门禁记录（保留口径）**
- ①现值面守门级数旧口径未标注命中 = **0 命中**；②吸收自动化旧名未标注命中 = **0 命中**；
  ③中性表述扫描零命中；④测试口径行 0 触碰（改动行中仅一处含测试数字文本，原样保留）。
- 门禁（`-j 2`，2026-10-16 实测，rustc/cargo 1.97.1 + MSVC 14.44.35207）：全量测试 exit 0（0 failed）；
  clippy exit 0（0 警告）；fmt exit 0（0 diff）。**0 新增测试**；套件/例数不在该文件抄写
  （单一真源 = test-baseline.md，本批 0 触碰）。
- 纪律自检：0 装 PASS（报「已清」均有扫描 0 命中证据；F-02 报「方案已出、未执行」不假称已瘦身）；
  历史不追溯改写；LOCKED 5 项 0 触碰；不 commit；中性表述。

**顺带发现（登记不代改）**：①`Cargo.toml` description 仍写「18-crate 单一工作区」与现行口径不符；
②LOCKED 内部两口径并存（哲学锚「9 重 v9」vs ROADMAP「7 重 v7」），**升/不升需 owner 拍板**；
③`artifacts/` 157MB 超 F-02 三组之和；④共树在途改动（他线插入 5 行执行进度注记）已报备。

**owner 拍板项 4 项**：F-02 执行批；LOCKED 异口径拍板；`Cargo.toml` description「18-crate」残留；
v1.0 发布脚本族「6 重守门」留痕随归档批处理。

**2026-10-18 附录（守门口径代码实证，供 §7 第 2 项拍板）**
- 复现结果：A）v1.0 时代的三个守门承载 crate **均未迁入 v2**（0 命中）；
  B）治理目录 **22 个模块**（含 colang / tool_guard / evidence / action_binding /
  approval_closure / rate_limit / sandbox_ladder / untrusted_mark 等）；
  C）治理库自述「five gates in sequence … **generalized so gates are values**」——
  v2 明确把门做成**门值**，不再做固定层数装配；
  D）证据检查器文件头自述「**a library checker, not a ninth "fold"**」——v2 代码**显式否认**「第 9 重」；
  E）工具守卫自述「a guard may refuse, never allow」——守门是**单调哨兵**，可装任意多个，
  层数无工程意义。
- **三口径对照**：9 重 v9（v1.0 发布时代自描述）/ 7 重 v7（2026-10-16 统一后的现值标签）/
  **门值集合（v2 真实工程事实）**。
- **建议（owner 三选一）**：**建议 A（推荐，最小改动）** 现值口径改为「守门 = 治理门值集合 + 单调哨兵；
  层数（7/9）仅为 v1.0 lineage 标签，v2 不作工程断言」（需 owner 明示解锁 LOCKED 面）；
  **建议 B** 反向认 9 重 v9（成本高且与 v2 架构相悖，不建议）；
  **建议 C** 维持两口径并存（长期债，不建议）。
  对外文案已按建议 A 的**工程侧**执行：不把「7 重 / 9 重」写成现行能力事实。

### 现状复核（2026-10-23）

- **已过时（B-05 的「13 键 = 现值」）**：现行键集是 **14 键** —— 代码常量实测为 14 元素
  （`crates/foundation/core/src/philosophy.rs`，常量名为历史命名，注释明写「R125-12 时为 13，实际 14 元素」），
  哲学锚 O-1 描述已写「键集（现行 14 键）」，且「13 键」为 2026-10-11 之前口径（见 ADR-0006）。
  **新发现的一处残留**：根 `Cargo.toml` 的 `verdict_cache_keys` 仍写 `13`（metadata 层）；
  按 LOCKED 纪律 0 触碰，**登记不代改**，建议随口径收尾批一行修正。
- **仍成立（B-04 三语义辨析 + 门值集合结论）**：2026-10-18 的代码侧实证未被推翻 ——
  v2 治理是门值集合 + 单调哨兵，层数不是工程计数；**owner 三选一仍未拍板**（§7 第 2 项仍挂），
  对外文案继续不写层数。
- **仍成立（B-04 历史标注纪律）**：历史卷宗原值保留 + 「历史口径（日期）」标注的做法未变。
- **已闭环（D13）**：旧名未标注命中扫描今日为 0 命中；两个文件名按原案保留 + 内容别名注记。
- **部分已过时（F-02 资产盘点）**：今日 `reports/` 已降至**百件级 / 数 MB** 量级、
  `scripts/guard_ml/` 大件已不在库（体积量级从 21MB 降到百 KB 级）、`artifacts/` 亦大幅瘦身；
  **但 `previews/`（数十 MB 实拍 PNG）仍在库** ⇒ 方案表第 4 行（图片处置）**尚未执行**，
  其余行的执行结果本批**未逐项核实**（处置动作与 commit 归属不在本册可核范围）。
- **已过时（顺带发现 ①）**：description「18-crate」残留 —— 今日工作区规模口径见
  [AGENTS.md](../../../AGENTS.md) §3 与 [contract.toml](../../architecture/contract.toml)；
  该 metadata 行本批未复核是否已改（标「未核实」）。
- **仍成立（纪律）**：「报『已清』须有扫描 0 命中证据；报『方案已出』不得写成『已瘦身』」。

---

## 2. `.gitignore` 卫生审计（2026-08-28）

**来源**：`docs/04-internal/gitignore-hygiene-audit-2026-08-28.md`
**定位**：R14 sub-agent 调研账 —— 完整审计 `.gitignore` 状态 + 已修目录 + **真漏 ignore 目录**
（不假装 OK）。状态：🟡 调研账（no git add / commit / push），仅标真漏路径 + 推荐修法。

### 结论（原文要点保留）

- **section map（36 段，估 289 行）**：逐段判定 —— Rust 构建、IDE、OS、本地覆盖、SQLite、
  宏元数据、Python egg-info、AI 代理临时产物、SDK 虚拟环境、一次性 trash、ad-hoc 检查脚本、
  工作树临时、反编译源码、临时 typo、发布签名、伴侣凭据、密钥通配、研究内存区、
  R23 一次性、备份、Tauri、前代工作树、R119 抛掷物、out/.apeireth、集成 e2e 日志、
  TP20-S5 供应链产物、R215 借用件、crates 数据库、scratch、dist/build/coverage、前端产物、
  harness 消息区。**总评：34 ✅ 生效、1 ⚠️ 过宽、3 ❌ 真漏**。
- **真漏 ignore（3 条，R14 spec 标错）**：`reconstruction_v2/`（孤儿目录，0 tracked）；
  `_scripts/`（半漏：仅 `*.py` 命中，其他扩展不命中）；`.gitignore-research`（根级 tracked 研究文件）。
- **`.py` 过宽**：R126 批加的 `*.py` 规则太宽，**任何 `.py` 文件都被忽略**（含 Python 测试 /
  examples / 真源码）；原文标注子代理**未实测误伤范围**，只 flag 风险。
- **历史特殊目录（应 NOT ignore，但需标）**：`legacy/`（1857 tracked 文件）——
  R14 spec 标「Round 1 已 ignore legacy/」**标错**；但 ignore 也是错的（破坏 v1 参考链），
  子代理独立判断：**保留 tracked + 加 doc 标**；理由 = v1 真接代码是后续语义对齐的直接参考。
- **推荐修法（不真改）**：新增 3 行 ignore；`.py` 二选一（窄规则 `/_*.py`、`/scripts/_*.py`、
  `/_scripts/*.py`；或加例外 `!crates/**/test_*.py`、`!examples/**/*.py`、`!docs/**/*.py`）；
  `legacy/` 不加 ignore。**0 触碰 LOCKED**（0 改 `.gitignore`、0 `git rm`、0 `git add`）。
- **总结表**：总行数 289 / sections 36 / 真生效 34 / 真漏 3 / 过宽 1 / 历史特殊 1 /
  应 add 但未 add 8（新 crate 源码）/ 本 audit 0 触碰 git。

### 现状复核（2026-10-23）

- **仍成立（3 条真漏未销）**：今日 `.gitignore` 中**仍无** `reconstruction_v2/`、`_scripts/`、
  `.gitignore-research` 三条规则 ⇒ **真漏仍未修**（按原文「R15+ 真改 + git rm --cached」的处置口径，
  至今未执行；本批 0 改动）。
- **仍成立（`.py` 过宽）**：`*.py` 规则仍在 `.gitignore` 中（当前文件 339 行，规则位于中段）⇒ 过宽未收。
- **仍成立（`legacy/` 判断）**：`legacy/` 仍为 tracked 参考面，不 ignore 的判断继续成立
  （与 01 §2 的 legacy 考古口径一致）。
- **已过时（规模口径）**：289 行 / 36 段 / 34 生效 是 2026-08-28 计数；今日文件行数已变，
  且新增过若干轮 ignore（前端产物、供应链产物、R215 借用件等）。**具体 section 数本批未重数**。
- **未核实**：3 个真漏对应的目录/文件今日是否仍存在（孤儿目录可能已被清理）。
  本批**只查了 `.gitignore` 规则是否存在**，未查目标路径是否仍在磁盘。

---

## 3. 活跃文档过时引用扫描 + README 中英脱节 + 顶层一致性（2026-08-19）

**来源**：`reports/audit-2026-08-19/active_docs_audit.md`
**范围**：active 文档（`docs/01-architecture` / `02-guides` / `03-reference` / `04-internal` 顶层）
+ 顶层 `README.md` / `README.zh-CN.md` / `CHANGELOG.md` / `RELEASE_NOTES.md` / `INSTALL.md` /
`CONTRIBUTING.md`；**不动 `docs/archive/**`**（形式撤销纪律）；**仅读，没改任何文件**。

### 结论（原文要点保留）

- **速览（22 条）**：HIGH 20 条 + MEDIUM 1 条 + LOW 1 条；估「修 1 个文件 ≈ 5 min ~ 20 min」。
  - 顶层 5 条：`INSTALL.md` 写「6 锚」（实为 8 锚，R125 B5 升 8 锚）、写「Cargo 1.97.1 / 6+ tests pass」
    与实际测试规模不符、链向 4 处死路径（`docs/00-R14-START-HERE.md` / `docs/ROADMAP.md` /
    `docs/GLOSSARY.md` / `examples/hello_world.rs`）、「主哲学 6 锚穿透」；
    `CONTRIBUTING.md` 两处「6 锚」。
  - `docs/01-architecture` 4 条：`architecture.md` 头部「84 active crates」与同文件第 38 行「85」自相矛盾；
    `philosophy.md` 标题「The Six Anchors」且只列 6 个（缺 S-3 / O-1）；
    `security.md` 四处把 **S4 出站 default-deny + 审计链**写成已验证，而 backlog 显式标注
    **S4 ⬜ 未实施（实测网关无出站策略）** —— 原文称之为「最大的**安全谎言**」。
  - `docs/03-reference` 4 条：`crates.md` 写「12 键 verdict cache + 5 重守门」（实际 13 键 / 7 重 v7）；
    三处「6 哲学 anchor」；「24 LOCKED」为历史口径（R128 已形式撤销，仅保 3 项不可变脊柱）。
  - `docs/04-internal` 7 条：`design-intent.md`「81 个 crate」；`maintenance-guide.md`
    **整段内容重复一次**（1-156 行 = 158-344 行，删 187 行）；`release-plan.md`「81 crates」+
    指向不存在的 `docs/capability-packs.md`；`plugin-authoring-guide.md` 仍用旧插件载体描述 +
    「watch 只记录到日志」已过时；`team-work-doc.md` 器官命名双轨；`backlog.md` 自陈陈旧但已标 ✅。
  - `docs/02-guides` 1 条：`quick-start.md`「84 crates」。
- **Part A（过时引用扫描）**：13 个模式 grep → file:line → 上下文核对；估时 HIGH 必改 ~80 min、
  MEDIUM ~25 min，合计 ~2 小时。
- **Part B（README 中英脱节）**：两者结构 / 内容 / anchor 一致，仅英文版新 1 个 commit
  （桌面伴侣同步）；唯一差异 = 英文「What's Next」段提桌面伴侣链接 + 6 份集成阶段报告，
  中文版同位置未提 ⇒ 建议中文补 1 段（~3 min）。
- **Part C（顶层与 git 一致性）**：
  - **双轴制**：产品轴 = git tag `v1.0.0`；crate 轴 = `Cargo.toml` `workspace.version = 1.2.0`
    （拍板于 backlog #26，2026-08-17）；原文点明「主人说的 1.0.0 是产品轴，crate 轴实际 1.2.0」。
  - `CHANGELOG.md` **全部一致**（历史 banner 归位章节保留原文正确）。
  - `RELEASE_NOTES.md` 2 处必改（测试组数、S4 白名单表述）。
  - `INSTALL.md` **active 文档中问题最多**（5 处必改）。
  - `CONTRIBUTING.md` 3 处必改。
- **D 优先级/预算**：P0（INSTALL + CONTRIBUTING + philosophy + architecture + crates，5 文件 ~1 h）；
  P1（9 文件 ~40 min）；P2（7 文件 ~25 min）；P3（2 文件 ~5 min）；合计 ~20 文件 ~3 h。
- **E 单文件工时表**：1 行编辑 ~1 min / 2-3 行 ~2 min / 表格单元 ~3 min / 加 2 锚 ~5 min /
  章节重构 ~10 min / 删重复段 ~5 min / INSTALL 5 处 ~15 min。
- **F 微观提示 4 条**：`Cargo.toml` 版本行与主人表述的差异；`crates.md` 三段同款错述；
  **S4 出站策略跨 6 个 active 文档都标「已实装/verified」而 backlog 显式 ⬜** —— 最大安全谎言，
  必须优先修；哲学锚 6→8 跨 4 个文件传播。
- **G 不在范围（主动声明）**：`docs/archive/*`、源码、`reports/*`、`_research_mem/*`、`ROADMAP.md`。

### 现状复核（2026-10-23）

- **已过时（哲学锚数）**：今日锚数为 **9**（`docs/01-architecture/philosophy.md` 标题已是
  「The Nine Anchors」；`INSTALL.md` / `CONTRIBUTING.md` 已写「9 锚」）⇒ 原文「6 锚 → 8 锚」的
  发现链**已全部闭合**（且又经过一次 8→9 升级）。
- **已过时（crate 数口径 / 重复段）**：原文的 84 / 85 / 81 等数字均为 2026-08-19 口径（现值见 [AGENTS.md](../../../AGENTS.md) §3 与 [contract.toml](../../architecture/contract.toml)）；`maintenance-guide.md` 今日 307 行、重复标题已不存在 ⇒ 该结构 bug 已修。
- **已过时（S4 出站「安全谎言」）**：工具出网面已在 v2 落地（`crates/capabilities/tools/src/egress.rs` 及其 deny 测试），[claims.yaml](../../status/claims.yaml) 有 `sec.egress_gate`（`PRODUCTION WIRED`）；但**模型出网面**仍未治理（见 02 §6）⇒ 原文「S4 未实施」这句**对工具面已过时、对模型面仍有残余真**。
- **已过时（键数 / 守门级数）**：见 03 §1 复核（键集现 14 键；层数口径已改判为「门值集合」）。
- **仍成立（方法论与体例）**：①「文档自述不算证据，必须与台账对账」；
  ②「双轴制」的产品轴 / crate 轴区分思路（今天的版本轴口径见
  [AGENTS.md](../../../AGENTS.md) §3，以根 `Cargo.toml` 为准）；
  ③历史 banner / 历史锚**保留原文**的纪律；④「先修跨文件传播的口径错误，再修单点」。
- **仍成立（未销项）**：`team-work-doc.md` 器官命名双轨（待 ADR 桥）与 `plugin-authoring-guide.md`
  的插件载体描述 —— 本批**未逐条复核**（标「未核实」）。
- **未核实**：Part B 的中文 README 补段是否已补；`INSTALL.md` 4 处死链是否已修。

---

## 4. 宣称 → 证据对照矩阵（文件未标日期，内含 2026-10-20 复核注记）

**来源**：`docs/04-internal/claims-evidence-matrix.md`
**性质**：核心审计产出（只读代码核查 + 文档撰写，不修改任何既有文件）。
**目的**：把 README / 架构文档 / 能力文档中的宣称逐条压缩为**可验证命题**，按证据强度定级，
使「愿景 → 性质 → 证据」三者的差距被系统摊开。
**口径**：诚实第一，宁低勿高；文档自述不作证据；机制级测试 ≠ 效果验证；默认关 / 未接线一律降级；
命名强于实现的「证明」不计 A 级。

### 结论（原文要点保留）

**§0.2 证据等级（六级，定义保留）**
| 等级 | 定义 |
|---|---|
| A-Kani/TLA 已证 | 有真实形式化证明（含符号输入或领域不变量断言）/ TLA+ 模型经 TLC 穷举背书 |
| B-单测覆盖 | 有具体单测 / 集成测试断言行为（给出测试文件与测试名） |
| C-默认关闭未实证 | 代码已实现（含已接生产但默认关 / 未接 canonical 默认路径），产品效果无验证 |
| D-桩/未实现 | 明文 stub / mock / 未实现占位 |
| E-仅人工 | 只有人工验证途径（真机截屏 smoke、UI 点击流 —— 挂账 #2） |
| F-纯愿景 | 代码里找不到对应物，或宣称与代码直接矛盾 |

**§0.3 定级特别规则（4 条）**：①「命名强于实现」的填充式证明文件**不计 A**
（自述「不是形式化证明……`String::len()==4` 级填充测试」）；②证明工作流三步全部
`continue-on-error`（**证明失败不阻塞合入**）⇒ A 级证据附此保留；③C 级含两类
（已接生产但默认关 / 库级已实现但从未接入 canonical 运行时）；④**基准数字单独定级**
（无复现物证按 F 处理）。

**§11.1 分级计数（77 条宣称行：A 1 / B 37 / C 14 / D 9 / E 1 / F 15）**
严格口径提示：A 只有 1 条且带 4 项保留；名义 B 级中约 1/3 是「库级机制」而非「产品行为」；
**C+F+D+E 合计 39 条（约 51%）的宣称在默认产品路径上不可观察或已被反证**。

**§11.2 五条最重的差距发现（保留）**
1. **对外发布物里传播无来源的性能数字**：发布工作流把一批微秒/毫秒级数字写进 Release Notes，
   这些数字在 `reports/` / `scripts/` / `benches/` 中**零出处**，工作区无任何 bench target；
   而唯一独立实测（10K 混合检索 P99）与核心数字**矛盾 >10×**。
2. **「端到端加密漫游」是明文桩**：对外宣称的一整套加密/多跳/漫游能力，在源码里是明文 hex 编解码 +
   「单跳 stub」+ 「不验证 Merkle」；连宣称的导出 API 都不存在 —— 「文档说有、代码没有」的最大单点。
3. **默认路径上没有「认知闭环」**：主动召回 / 每轮整理 / reflexion / 反幻觉注入 / 器官 / shell /
   fetch / worktree / 洋葱层等全部默认关；三个编排器**连接线都没有**（各自只有库级测试）。
4. **形式化证据面窄且不设门**：真 A 级只有一处（三不变量，且自述不挂生产审批路径）；
   六个 crate 的证明文件是自认的「填充测试」；证明工作流三步全 `continue-on-error`；
   TLA 无归纳泛化、无 CI 跑 TLC。
5. **「0 hollow stubs / 真签名 / 着色器」三条诚信红线失实**：SDK 桩模式仍在、
   谱系签名是可伪造 mock 串、某着色器只存在于对外文档两行、四态色温表与代码四个温度全部不符 ——
   「命名/文档强于实现」是**成体系模式**，不是孤立笔误。

**§10 基准表逐条溯源（10 行，保留判定）**
| # | 对外基准项 | 声称 | 判定 |
|---|---|---|---|
| 1 | 混合检索（10,000 节点） | 1.82 ms（标 VERIFIED） | **未见复现脚本 + 与唯一独立实测矛盾（>10×）**；被引 bench 源文件已删除 |
| 2 | 配额抢占（上下文切换） | 8.40 µs | **未见复现脚本**；全仓无第二处来源 |
| 3 | 因果世界模型 CoW（分支 fork + 100 文件快照差分） | 0.035 ms | **未见复现脚本**；且「100 文件快照差分」与实现（内存快照 id 树）不符 |
| 4 | SAGA 补偿回滚（LIFO 内存执行） | 0.012 ms | **未见复现脚本**（同文档另称「<35µs」，口径不一） |
| 5 | 实时语音打断 | 0.18 ms | **未见复现脚本**（机制测试存在） |
| 6 | HUD 渲染 tick | 0.08 ms | **未见复现脚本** |
| 7 | 作业对象沙箱 spawn | 6.40 ms | **未见复现脚本**（测试不测耗时） |
| 8 | 微内核冷启动 | 4.20 ms | **未见复现脚本** |
| 9 | 运行时空闲占用 | ~18.2 MB | **未见复现脚本**（对照项是桌面空闲占用，对象不同） |
| 10 | 工作区测试通过率 | 100% | 测试在跑（B），但数字为**静态手填** |

横向漂移：对照表内「核心工作区 crate 数」与对外 README 不一致；发布工作流自述
「当前根工作区**没有 bench target**」却把空的基准目录当基线上传；发布注记把一批无来源数字直接写进
对外 Release Notes。

**§9 可形式化命题建议（6 条，保留）**：`P-DENY`（未显式授予 ⟹ 不得产出 Allow；**先决修复：空引擎语义
必须先定**）、`P-ROLLBACK`（补偿恰为逆序且每项一次 / 补丁失败 ⟹ 磁盘状态还原；**障碍：实现自述
best-effort**）、`P-EXC`（面向不可信输入的解析入口绝不 panic；首批挑 5 个纯解析器）、
`P-RACE`（逻辑互斥：同一会话写者 ≤ 1、claim 恰被一个调用者获得；把已有并发测试升为状态机证明）、
`P-APPROVE`（把三不变量从研究模型推广到生产审批路径 + 撤掉非阻塞设置 + 让镜像 crate 覆盖生产路径）、
`P-TERMINAL`（终态无出边；模型级已证，自停守仅有常量断言 + 负向测试）。
所有命题**都要求符号输入**，拒绝固定轨迹自证。

**§11.3 复核注记（2026-10-20，第三线；不追溯改写上文历史判定）**
1. **基准表处置已执行（部分销账）**：对外 README 的 P99 表已撤下，改为**可复现基准表**
   （脚本 + 复现报告），并**公开保留一条红行**（微内核冷启动实测 vs 目标差 2.2×，标 ❌）；
   发布工作流的无源数字注记已替换为「按可复现脚本策略重新测量」；§10 的旧判定按历史口径保留。
2. **未复核项**：P-DENY / P-ROLLBACK、证明工作流非阻塞、六处文档纠偏、记忆闭环评测仍维持原判定。
3. **外部对比核验**：2026-10-20 第三方对比文证据复核见当日证据文件；本文档仍为「宣称→证据」
   分级的内部真源之一。
4. 本注记 0 触碰 LOCKED；0 改上文历史数字。

**§11.4 建议的下一步（按杠杆排序，5 条）**：①先做 P-DENY / P-ROLLBACK（先决：定空引擎语义、
补丁回滚升级或宣称降级）；②撤掉证明工作流的非阻塞设置 + 让镜像 crate 覆盖生产审批路径；
③基准表要么补 bench target + 原始输出入库，要么整体降为「设计目标（未复现）」，并立即处理
发布注记里的无源数字；④**文档纠偏批**：明文桩 / 流式合成 / 伪装爬虫 / 色温表 / 不存在的导出 API /
「0 hollow stubs」六处按 0 装口径改写为「设计意图 vs 当前实现」双栏；⑤记忆闭环四件套给出默认开启的
验收评测，把 C 升 B 的判据固定为「效果可测」。

### 现状复核（2026-10-23）

- **仍成立（三条诚信红线）**：
  ①`P-DENY` 的先决问题**未解** —— 治理侧「空引擎 = 允许」的测试与判定**仍在**
  （`crates/foundation/governance/src/approval_policy.rs` 的空引擎放行路径与对应测试仍在）；
  ②`P-ROLLBACK` 的障碍**未解** —— 补丁应用器仍自述「回滚是 best-effort」；
  ③「0 hollow stubs」反证**仍成立** —— SDK 桩模式常量仍为编译期 `true`（改动它需经哲学锚 + 主人审）。
- **已执行（§11.3-1）**：对外 README 的性能表已撤（今日 README 中无该表与「VERIFIED」行）；
  可复现基准脚本与复现报告已落地；发布注记的无源数字已被替换 —— **§10 的旧判定按历史口径保留**。
- **已过时（键数口径）**：§3 的「13 键」表述已过时（现 14 键，见 03 §1 复核）；§3 关于「代码是 12 键常量 + 13 键仅存于文档」的记述**在当日成立**，今日代码常量已扩到 14 元素。
- **仍成立（分级框架本身）**：六级证据定义、特别规则（命名强于实现不计 A、证明工作流非阻塞、基准数字单独定级、C 级两类）、「机制级 ≠ 效果」与「默认关一律降级」—— 仍是仓内唯一成体系的宣称—证据分级面，可继续用于后续批次。
- **仍成立（§9 命题清单）**：6 条命题中 P-DENY / P-ROLLBACK / P-APPROVE / P-TERMINAL 仍待做；形式化面的「要求符号输入、拒绝固定轨迹自证」纪律不变。
- **部分已过时（§11.2-3 认知闭环）**：默认关矩阵仍在（见 01 §6 复核），但部分开关已接线并有默认关逃生门（旋钮矩阵以 [capabilities-matrix.md](../../03-reference/capabilities-matrix.md) §0.2 为准）。
- **未核实**：§11.2-4 的证明工作流非阻塞设置是否已撤、§11.4-4 六处文档纠偏是否已执行、§11.3-3 的外部对比核验文件是否存在 —— 本批未逐项查证。

---

## 附：本册与其它册的交叉点

- **测试数字真源（不抄写）**：[test-baseline.md](../../03-reference/test-baseline.md)。
- **能力声明真源（不抄写）**：[claims.yaml](../../status/claims.yaml)。
- **架构契约真源（不抄写）**：[contract.toml](../../architecture/contract.toml)。
- **守门 / 口径的机器守门**：`scripts/check_doc_caliber.py`（四级口径触发词检查，跳过历史卷宗与
  带日期文档）、`scripts/doc-caliber-baseline.txt`、`scripts/check-neutral-terms.ps1`
  （中性表述扫描，只查新增行）—— 口径纪律已从「人工守」转为「机器守」。
- **架构与能力侧结论**：见 [01-architecture-and-capability-audit.md](01-architecture-and-capability-audit.md)。
- **安全与治理侧结论**：见 [02-security-and-governance-audit.md](02-security-and-governance-audit.md)。
