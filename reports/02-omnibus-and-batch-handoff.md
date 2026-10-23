# 册 02 — 大 Omnibus 附录与批次交接（7 篇合册）

> **一句话定位**：R12 接手第一步的**附录 N 四路同行评审**（架构 / 安全 / 性能 / 全栈）+ M2 同行审查 + 两篇主线程与装配批次交接件——一组「同一份草稿、四把尺子」的交叉校验记录。
>
> **来源篇数**：7（分配表 §02，逐篇唯一归册）｜**原篇日期跨度**：2026-07-30 ~ 2026-08-17
> **合册生成日**：2026-10-23｜**复核口径（现行真源）**：[test-baseline](../docs/03-reference/test-baseline.md) §1 ／ [claims.yaml](../docs/status/claims.yaml) ／ [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) ／ [capabilities-matrix](../docs/03-reference/capabilities-matrix.md)

## 目录

| # | 原标题 | 原日期 | 角色 |
|---|---|---|---|
| 1 | 架构审稿报告 — 附录 N 草稿结构一致性审查 | 2026-07-30 | architect (M3) |
| 2 | M2.5-SEC — R12 附录 N 安全声明校验 | 2026-07-30 | security_reviewer |
| 3 | M2.5-PERF — 附录 N 性能评审 | 2026-07-30 | perf reviewer |
| 4 | M2.5-FE 全栈评审报告 | 2026-07-30 | Agent 编排专家 |
| 5 | M2 同行审查 — 附录 N 草稿 | 2026-07-30 | code_reviewer (M2) |
| 6 | 主线程验收报告 — TP21-TP30 + W4/W6 + TP31/32/33 | 2026-08-17 | 主线程 |
| 7 | N17/TP2 装配统一注册件 — 完成总结 | 原篇未标日期 | 施工线 |

---

### 架构审稿报告 — 附录 N 草稿结构一致性审查 (2026-07-30)

**来源**：`reports/apeireth-omnibus-appendix-n-r12-handoff-arch-check.md`（约 460 行）｜architect (M3)｜审稿时间 2026-07-30 18:00 +0800，master HEAD = `6b67629e`。
**结论/要点**
- 审稿对象 = 附录 N 草稿（249 行 / 23.4 KB）；主文档 = `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md`（6241 行，附录 M 之后追加，**不可修改之前内容**）；参照附录 M 模式稿（307 行 / 9 章 / 8.2 分）。
- **7 维评分**：章节编号延续 M→N 9/10；风格一致性 9/10（§0–§5 + §6 硬约束独立 7 章；§5 子节 A/B/C/D 4 个 vs 附录 M 5 个，合理简化）；与主文档锚点无缝 9/10（6 哲学 anchor × 6 落地证据全表）；不与主文档冲突 **10/10**（D1–D4 已知差异全透明标注、**不回改附录 M 任何字段**）；下一团队接手可读性 9/10；新增 D1–D4 透明化机制 9/10；**综合 9.0/10**（vs 附录 M 草稿初版 8.2/10）。
- **必改项 6 条**：#1（P0）§5.A 优先级 3>1>4>2 解释缺口；#2（P0）§1.6 集成 worktree 双轨同步表补 `a7805bf` 标注；#3（P0）§5 子节数与附录 M §5 对应关系声明；#4（P0）§5.D 与 §6 硬约束 4 条去重；#5（P1）§0 表加 dat_diff 行；#6（P1）§5.D 末尾「优先级 3>1>4>2」与 §5.A 同步。另 8 条「小改进（建议但非必须）」。
- **风险与未完成项**：CRLF 行尾副作用（Windows edit_file 混入 CRLF，建议 `dos2unix` 或 `core.autocrlf false`，内容不变、字节级一致）；必改项 #1 解释 ≤3 句 ≤60 字（避免干预 R12 自主决策权）；#3 章节映射 ≤5 行；**未关闭的工程缺口（主人明确不修，留给 R12）全部以「已知差异 / 已知 ceiling / 已知信息性」透明列出，不掩盖不升级**。
- **一句话结论**：草稿整体结构 9.0/10，可 append 到主文档末尾；4 条 P0 必改项每条 1–3 句小改，**不改结构 / 不改数字 / 不改哲学 anchor / 不回改附录 M 任何字段**；四大创新点 = D1–D4 透明化机制 + §5.A 优先级建议 + §6 硬约束独立章 + 集成 worktree 双轨同步实测。
**现状复核（2026-10-23）**：**已过时（被审对象整体不在现行树）**——附录 N 草稿 `reports/apeireth-omnibus-appendix-n-r12-handoff-draft.md` 与主文档 `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md` 均**不在现行 `reports/` 121 篇内**（已随 F-02 资产出库，见 [README.md](README.md) 的迁出说明与 [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) D17），故其字段与数字在现行树内**无对照面**。**仍成立**：「同一草稿多路评审 + 必改项按 P0/P1 分级 + 不改结构不改数字 + 未关闭缺口以『已知差异 / 已知 ceiling』透明列出」的交接纪律，与现行 [CONSTRUCTION-DISCIPLINE.md](../docs/04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md) 及 [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) 登记债面一致。**未核实**：附录 M/N 引用到的 `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md` 是否可从 git 历史取回——本轮未检索。
**归属链接**：../docs/04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md、../docs/04-internal/backend-enhancement/00-backlog.md、../docs/04-internal/backend-enhancement/f-02-migration-manifest-2026-10-18.tsv、../README.md；历史（已迁出）`reports/apeireth-omnibus-appendix-n-r12-handoff-draft.md`、`APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md`；同册相关 本册 §2–§5

### M2.5-SEC — R12 附录 N 安全声明校验 (2026-07-30)

**来源**：`reports/apeireth-omnibus-appendix-n-r12-handoff-sec-check.md`（约 110 行）｜security_reviewer｜任务 `6866bb3a-ab6f-42ca-8a5b-2267af7d5d97`｜**性质：read-only 文档数据校验，未改代码/草案/工程手册**。
**结论/要点**
- **9 项核对表**：4 项完全一致 —— #1 V1121 fake-KPI detector（`keys_present=9, fake_kpi_attempts=3, runner_confusion_attempts=0, v03_v04_confusion=3, n_threats=2, gate_passed=False` 模块自身 / `dashboard=yellow` 为 V1138 综合且 `overall_gate_passed=True`）；#2 V1121 ASI 9 键复用（keys_present=9 但 gate_passed=False，**未越界写成「已通过」**）；#3 dashboard yellow = V1121 信息性漂移、非阻断；#4 R11-SEC-002 self-claim 补充 `covered/total = 4/4`。
- **5 项 P0/P1 范围漏盖**：#5 ❌**P0** R11-SEC-001 三类修复（fake-KPI regex 重写 + path traversal + secret-leak）全文无 R11-SEC-001 字样；#6 ❌**P0** V1132 部署 validator 语义门禁（`canonical_bundle_valid` + `offline_valid`/`runtime_valid`/`passed` 三分裂 + **18 跨文件语义断言** + daemon probe 全 MISSING）；#7 ❌**P0** V1132 SSRF allowlist（`_LOOPBACK_HOSTS` + `_LOOPBACK_PORTS` 含 8765；`file://` / `gopher://` / `169.254.169.254` 全拒）零命中；#8 ❌**P1** R11-SEC-001/002 是 R11 安全事件全集的串联关系缺失；#9 ❌**P0** serve.py HTTP 边界硬化（Content-Length 1 MiB cap + 100 messages + 32 KiB 单消息；非 JSON→415 / 缺 Content-Length→411 / body 超限→413；OWASP A05 DoS + multipart 旁路）零命中。
- **最小修正 7 条**（4×P0 + 2×P1 + 1×P2），每条给最小改法与依据。**数字偏差结论**：9 / 3 / 2 / 0 / 3 / 4 / 9-9 / 5-5 全部能在 T1 报告实测输出找到同值，**字段与数值无偏差，错位仅为资产漏盖与清晰度**。
**现状复核（2026-10-23）**：**已过时（被核对稿与依据稿均已不在现行树）**——附录 N 草稿、附录 M 所在主手册、`r12-baseline-verification-2026-07-30.md`、`r11-security-review.md` 等依据件均**不在现行 121 篇**内。**仍成立（方法论，且是本册最可复用的一条）**：把「模块自身 gate」与「综合 gate」**分层标注**、把安全资产按「设计层已知清单 vs 运行时真查询」区分、把 HTTP 边界状态码（415 / 411 / 413）与旁路防护写成**可核条目**——与现行 [capabilities-matrix.md](../docs/03-reference/capabilities-matrix.md) §0 的 `supported/available/reason` 分层、[claims.yaml](../docs/status/claims.yaml) 的 `notes` 残余标注做法同向；现行同类能力面见 [claims.yaml](../docs/status/claims.yaml) `sec.provider_proxy_ssrf`（SSRF 目标类闸 + DNS 钉址 + 重定向逐跳闸 + 头白名单）与 `sec.sql_readonly_gate`（fail-closed 拒绝）。**未核实**：V1132 / serve.py / R11-SEC-001 这些具体资产在现行树中是否有对应实现面——本轮未检索，不作断言。
**归属链接**：../docs/status/claims.yaml、../docs/03-reference/capabilities-matrix.md、../docs/04-internal/backend-enhancement/00-backlog.md、../README.md；历史（已迁出）`reports/apeireth-omnibus-appendix-n-r12-handoff-draft.md`、`APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md`、`reports/r11-security-review.md`；同册相关 本册 §2、§5

### M2.5-PERF — 附录 N 性能评审 (2026-07-30)

**来源**：`reports/apeireth-omnibus-appendix-n-r12-handoff-perf-check.md`（约 160 行）｜perf reviewer｜**未改代码或 Omnibus**，仅产出评审报告。
**结论/要点**
- **Leader 指定 6 项重点核对**：#1 V1130 wallclock **5407.30ms（degraded）**，failed_codes `['IC_V1130_UNREACHABLE']`；#2 V1136 真测 **0.97s**（`elapsed_v1136=0.9729`）、composite `v05_total_v1136 = 0.8682`；#3 V1074 真测 **9.30s**（`elapsed_v1074=9.3046`）、`0.8957`；#4 automation **200/2/49.20s**；#5 V1136 dashboard render 5 轮 × 100 trials（cold/warm/combined **81.5 / 40.8 / 72.4 µs**）；#6 V1141 IC-001 **18 字段 LOCKED** + composite drift **3e-05**。
- **一处口径瑕疵（⚠ P1）**：5407.30ms 与 5.4287s 差 **21.4ms**——前者是 CLI 输出的 dashboard **timeout 检测点**（退化触发瞬间），后者是 `time.perf_counter()` 包的**总 elapsed**（含 timeout 触发后清理窗口），**两者非简单四舍五入**，读者可能误算。
- **性能差距量化**：V1130 真实 8.7s（8695ms）→ R12 5.43s（5428.7ms）→ 目标 2.5s；**改善 3.27s / −37.6%，但距 2.5s target 仍差 2.93s（+117%）**——是 ceiling 不是 regression。
- **必改项 8 项**（P1 5 + P2 3）。**13 项核对表结论**：4 ✓ / 3 ⚠P1 / 2 △P2 / 4 不在范围；**8/13 ✓ 数字可追溯，0 P0 硬错**。
- **口径纪律提醒（关键）**：「微秒 render 指标达标不代表 V1130 build 或 HTTP 进程链路达标」——render 81.5/40.8/72.4 µs 与 wallclock 5.43s 是**完全不同口径**，接手团队不要混淆。
**现状复核（2026-10-23）**：**已过时（依据件已不在现行树）**——`r12-baseline-verification-2026-07-30.{md,json}`、`r11-architecture-integration-contract.md`、`r11-performance.md`、`r11-automation.md`、`r11-qa-acceptance.json`、`r11-v1138-delivery-summary.md` 均**不在现行 121 篇**内；且 V1130 / V1136 / V1074 / V1141 属 v1.0 时代 Python 侧编号体系，现行仓内无对应登记面（**未核实**，不作断言）。**已过时（数字口径）**：本册所引全部秒级数字（9.30s / 0.97s / 49.20s / 81.5µs 等）均为 2026-07 语境，**不得外引**；现行性能相关基线另见册 05（benchmark / 冷启动剖析 / 磁盘缓存建议），测试数字真源为 [test-baseline.md](../docs/03-reference/test-baseline.md) §1。**仍成立（方法论）**：①「同一指标给两个数必须注明测量点差异」；②「渲染级微秒指标 ≠ 链路级 wallclock，两者不可混谈」；③「改善幅度量化（−37.6%）与剩余差距量化（+117%）同时给出」；④「0 P0 硬错 / N 项需补标注」的分级结论格式——这四条在现行性能与债栏登记中仍应沿用。
**归属链接**：../docs/03-reference/test-baseline.md、../docs/04-internal/backend-enhancement/00-backlog.md、../README.md；历史（已迁出）`reports/r12-baseline-verification-2026-07-30.{md,json}`、`reports/apeireth-omnibus-appendix-n-r12-handoff-draft.md`；同册相关 本册 §5、册 05

### M2.5-FE 全栈评审报告 (2026-07-30)

**来源**：`reports/apeireth-omnibus-appendix-n-r12-handoff-fe-check.md`（约 210 行）｜Agent 编排专家（fullstack_engineer2 不在团队，由 M2.5-FE 承担）｜任务 `8cc93cb9-c0dc-4df2-bd5b-e820af2d6032`｜快照 2026-07-30｜**只读校验，0 代码改动，0 commit**。
**结论/要点**
- **7 维 PASS/FAIL 矩阵**：§5.A 字段一致性（8 项）**6 ✅ + 2 ⚠️**（D1/D4 透明标注，不算硬错）；§5.B（6 命令）**5 ✅ + 1 ⚠️**（命令 1 缺 §1.0 子节）；§5.C（4 遗留工程）**3 ✅ + 1 ⚠️**（row 4 严重度判定差异）；§5.D（4 ceiling）**3 ✅ + 1 ⚠️**（row 4 实际已闭合，透明标注 D4）；p0_workflow 五阶段 **✅ 字段准 + ⚠️ 缺 5 阶段命名**（soft S4）；R11 编排状态机 3 stages **✅ 完全准**（`Stage.MEASUREMENT` / `DASHBOARD` / `QA_GATE` 源码确认，`apeireth/r11_orchestration.py` line 31-37）；commit_delta=26 **⚠️ §0 快照表缺此字段**（硬错 H2）。
- **总判定**：**结构正确，字段基本 1:1，4 项硬/软错需修（3 硬 + 1 软）**；附录 N 可继续 M-final，但 M-final 前必须吸收 **H1+H2+H3+S1+S2 共 5 条必改项**。
- **准项统计**：**1:1 准项 6+5+3+3+4+6+1 = 28/35（80%）**；透明差异 D1–D5 共 5 项（不算硬错）；必改项 3 硬（H1–H3）+ 2 软（S1–S2）+ 2 结构（S5–S6）= 7 条；源码交叉验证 1 处；git log 真实数据 8 个 commit 时间戳全列出；评估来源 4 类。
- **已知差异 D1**：master HEAD `7fbc97d0…`（2026-07-30 15:50:39 +0800）vs 附录 N `6b67629e…`（17:34:15 +0800），差 1 commit = 附录 M append 自身，双轨同步成立 ✅。
**现状复核（2026-10-23）**：**已过时（被审对象与依据件均已迁出）**——同本册 §2 / §5，附录 N 草稿与主手册不在现行 121 篇内；`apeireth/r11_orchestration.py` 属 v1.0 Python 侧，**未核实**其是否仍在仓内（不作断言）。**已过时（`commit_delta=26` / `master HEAD` 类字段）**：现行树以 git 提交链与 [Cargo.toml](../Cargo.toml) members 为准（30 members），此类「快照 commit 差 N」字段现行无登记面。**仍成立**：①「字段一致性按 §5.A/B/C/D 四类分表核对并给准项比例（28/35 = 80%）」；②「源码交叉验证」（本篇专门交叉 `r11_orchestration.py` 的 3 stages 枚举）；③「只读校验 0 代码改动 0 commit」的自证；④「透明差异单列 D1–D5，不计硬错」的区分法——与现行 [docs_truth.py](../scripts/docs_truth.py) `--verify`（claims 证据 / staleness 检查）同向。
**归属链接**：../scripts/docs_truth.py、../Cargo.toml、../docs/03-reference/test-baseline.md、../README.md；历史（已迁出）`reports/apeireth-omnibus-appendix-n-r12-handoff-draft.md`、`apeireth/r11_orchestration.py`；同册相关 本册 §1、§2、§5

### M2 同行审查 — 附录 N R12 接手第一步草稿 (2026-07-30)

**来源**：`reports/apeireth-omnibus-appendix-n-r12-handoff-peer-review.md`（约 260 行）｜M2 (code_reviewer)｜任务 `27982a9d-62b5-400e-9e96-01f956fa5123`｜master HEAD = `6b67629e`。
**结论/要点**
- **P0 数据硬伤总数 = 0**：**30 项真值**（10 数字 + 10 措辞 + 10 跨章节）全部与 T1 / 附录 M / M3 / M2.5-SEC / M2.5-PERF / M2.5-FE / T5 **1:1 比对，±0 偏差**；仅 2 项报告行数声明 off-by-one ±1（**P3，不致命**）。
- **吸收来自 5 份评审的必改项 15 条（5 P0 + 5 P1 + 5 P2）**：草稿已吸收 **8/15**（附录 M §5.A 过期注 2 / append 双轨 master HEAD + integration / 0.9063 vs 0.8682 vs 0.8532 三值并存 / pytest 24→107 / D1–D4 已知差异透明化 /「信息性漂移非阻断」三处 / `a7805bf` 已 orphaned / 集成 worktree 双轨同步 §1.6）；**未吸收 7/15**（5 P0 安全资产漏盖 + §1.0 命令 1 缺 + §0 commit_delta 26 缺）。
- **措辞一致性**：6 主哲学 anchor 在草稿 §0 + §1 + §3 + §5.D + §6 **全部贯穿**，V1130/V1136/V1074/V1131/V1121/V1132/V1141 模块措辞与主手册 + 5 评审全部对齐。**跨章节引用 28 项全部准确**（1 双向 + 22 单向 + 5 表格内），0 处破引用。
- **必改项 12 条**（5 P0 + 5 P1 + 2 P2）；P0 = 5 类安全资产漏盖 + 1 类文档硬错（§1.0 命令 1 子节缺）+ 1 类字段缺（§0 commit_delta 26）。**总改动量 ≤ 30 行**（6 表格新增行 + 1 §1.0 子节 + 1 句章节映射声明 + 1 句优先级修正 + 1 行行数修正），**不重写**任何结构 / 数字 / 哲学 anchor / 已落地 D1–D4 标注。
- **一句话给 M-final 决策者**：**草稿可 append**（主 17:43 实事求是：30 项真值 1:1、0 P0 数据硬错）+ 主 17:58 不假装（12 处显式 + D1–D4 全透明）。
**现状复核（2026-10-23）**：**已过时（被审对象与全部真值源已迁出）**——附录 N 草稿、主手册、T1 报告（`r12-baseline-verification-2026-07-30.md` 466 行 / `.json` 242 行）、T5 交叉验证报告均**不在现行 121 篇**内，故其 30 项真值在现行树中**无对照面**。**仍成立（本册方法论集大成）**：①「P0 硬伤 0 + P3 off-by-one ±1 不致命」的三级分类；②「多评审必改项合并成单一清单并统计已吸收 N/M」；③「改动量上限声明（≤30 行、不重写结构）」；④「跨章节引用逐条核、破引用必须为 0」；⑤「行数声明 off-by-one 也要记」——这五条与现行 [CONSTRUCTION-DISCIPLINE.md](../docs/04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md)、[00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) 的登记与核账纪律一致。
**归属链接**：../docs/04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md、../docs/04-internal/backend-enhancement/00-backlog.md、../scripts/docs_truth.py、../README.md；历史（已迁出）`reports/apeireth-omnibus-appendix-n-r12-handoff-draft.md`、`reports/r12-baseline-verification-2026-07-30.{md,json}`、`reports/r12-sec-cross-validation-2026-07-30.md`；同册相关 本册 §1–§4

### 主线程验收报告（最终版）— 团队批次 TP21-TP30 + W4/W6 + TP31/32/33 (2026-08-17)

**来源**：`reports/main-thread-acceptance-batch-TP31-33-etc.md`（约 65 行）｜主线程｜验收时间 2026-08-17 22:50（团队 `team_finalize` 收工后）｜验收基准 = integration 树 HEAD `2612398c` 实测，**不依赖团队自报**（团队自报 21/21 完成，integration HEAD = `3d893e4a`）。
**结论/要点**
- **验收总评**：**全量 368 组测试全绿（0 FAILED）+ 模拟 4/4 PASS + `--all-targets` 编译干净**；但**团队报告 21/21 与树不符：3 项报告夸大（TP21 / TP25 / TP26 只有文档/调研 README，无实现）**——与团队自己「要查 git 实际产物」的教训一致。
- **批次核对表（声称 vs 树实测，13 批）**：✅ TP31 W1 文本模拟器（`world_model.rs`：CounterfactualChain / TimelineLlm / Brier 拒绝 / CalibratedResolver，11 测试绿）；✅ TP32 W2+W3 因果推演（`causal_world_model.rs` 1020 行，MCTS 因果图 + 挖边，11 测试绿）；✅ TP33 发布门槛（compose 外部化 + `.env.example` + Dockerfile 补成员，纪律正确，docker 待实测）；❌ **TP21 渐进式披露**（声称「context.rs 扩展」→ **树零匹配**：catalog / 两级注入 / 按需展开全无，报告夸大）；✅ TP22 Observer/W5（`observer_capture.rs` 657 行 + PostExecuteHook）；✅ TP23 纪律技能（skills `lib.rs` SkillKind / DisciplineCheck 17 处）；✅ TP24 记忆来源链（`memory/provenance.rs`，V4 migration + 时间元数据）；❌ **TP25 时序预测器**（**全树 TimesFM/Kronos 零实现**，仅 research README）；❌ **TP26 投资事件架构**（**全树零实现**，仅 research README）；✅ TP27 标的元数据（`apeireth-stock`，refresh + V6 migration，61 测试）；✅ TP28 Markdown 知识库（`apeireth-wiki` 套件）；✅ TP29 工具声明式配置（`yaml_spec.rs` 1040 行 + ToolSpec trait + 22 测试）；✅ TP30 待评估清单（22 项调研，GitNexus 实测 + 9 项未实测诚实标注）；✅ W4 记忆主动推销（`proactive_memory.rs` 841 行 + 30 测试，TopicPredictor + 4 PreloadChannel）。
- **已修复并 cherry-pick 入 integration `1f6830d3`** 的问题（§3）。
- **剩余缺口（最终树实测）**：含「A4 事件流统一打通 ⬜ 缺打通」「W6 Brier 自我诊断 ⬜ 零实现（团队自标，下轮 7-10 人天）」「N20 ApprovalBridge 已知丢失 ⬜ 0.5 人天（团队估）」。
- **结论 5 条**：① 发布前置完成（TP31/32 真实现 + 全绿）；② **11/15 批次真实现**，W4 是亮点；③ **3 项报告夸大**，验收必须查树（主线程第一轮预验收也误判，以树为准修正）；④ **下批建议顺序（主人拍板过的优先级）**：E4 好奇→F4 假设检验（她本身，团队估 10-15 + 5-7 人天）→ TP21/25/26 补实现（按原 spec）→ W6（7-10 人天）→ F1/F6 → S4/A4 并行；⑤ **0 装 PASS 保持**：TP33 docker 待实测诚实标注 ✓、W6 诚实标未做 ✓、TP30 未实测项明示 ✓。
**现状复核（2026-10-23）**：**已过时（对象全部迁出 + 数字口径换代）**——`apeireth-stock` / `apeireth-wiki` / `apeireth-tool-registry` 等现分别在 [legacy/donor/apeireth-stock](../legacy/donor/apeireth-stock)、[apeireth-wiki](../legacy/donor/apeireth-wiki)、[apeireth-tool-registry](../legacy/donor/apeireth-tool-registry)；`world_model.rs` / `causal_world_model.rs` / `observer_capture.rs` / `proactive_memory.rs` / `yaml_spec.rs` 同属该批 crate 内文件，随 donor 迁移。「368 组测试全绿 / 0 FAILED」为 2026-08-17 workspace 口径，现行真源 = **341 套件 / 8570 passed / 0 failed / 51 ignored（2026-10-21）**。**仍成立（最重要的一条）**：**验收必须查树、不采信自报**——「3 项报告夸大（TP21/25/26 只有 README 无实现）」是本篇最有价值的负面证据，与现行 [AGENTS.md](../AGENTS.md) §1「0 装诚实纪律」、[claims.yaml](../docs/status/claims.yaml) 的 `sources` / `verified_at` 机制、债栏 D17「已跟踪历史交付物保留原位」纪律同向；现行对应的机器化防线是 `python scripts/docs_truth.py --verify --check`（claims 证据存在性 + staleness）与 `python scripts/arch_guard.py --strict`。**仍成立**：「0 装 PASS 保持」三项自检（待实测诚实标注 / 未做诚实标未做 / 未实测项明示）——与现行 [AGENTS.md](../AGENTS.md) §1 完全一致。**未核实**：所列剩余缺口（A4 事件流统一 / W6 Brier / N20 ApprovalBridge）在现行树中是否已被后续批次关闭——本轮未逐项核对。
**归属链接**：../legacy/donor/apeireth-stock、../legacy/donor/apeireth-wiki、../legacy/donor/apeireth-tool-registry、../docs/status/claims.yaml、../docs/03-reference/test-baseline.md、../scripts/docs_truth.py、../scripts/arch_guard.py、../AGENTS.md；历史（已迁出）`crates/apeireth-{stock,wiki,tool-registry}/**`；同册相关 本册 §7

### N17/TP2 装配统一注册件 — 完成总结 (原篇未标日期)

**来源**：`reports/n17-tp2-assembly-summary.md`（约 60 行）｜施工线｜**3 commit（cumulative `9251e80`）**：`0f18541` feat(N17/TP2): 9 工具子 crate 装配（register/catalog 接入 ToolBridge，18 files, 169+）、`5996d51` fix: ImageGenParams 加 serde Deserialize/Serialize、`9251e80` fix: persist `open_in_memory` 补回 status 列。
**结论/要点**
- **装配三件套（§10 铁边界）**：① **Tool trait 适配器**（`register.rs` × 9）：EnhancedShellTool / FetchEngineTool / EnhancedBrowserTool / CodeIntelligenceTool / ImageGenEnhancedTool / ImageProcessTool / VSearchTool / EnhancedFileOpsTool / RepoQualityAnalyzerTool——每个持原 crate 现有增强层，走原引擎**不重复发明**；② **register/unregister**（同 `register.rs`）：真注册到 `ToolRegistry`，**不打 log 空响**；`unregister` 返 bool，**重复卸返 false（幂等，0 残留）**；③ **catalog**（`tool-registry/src/catalog.rs`）：`CapabilityCatalog::from_registry(&registry)` 派生只读 snapshot + `names()` / `len()` / `render_markdown()` 三个 R/O 接口。
- **ToolBridge 接入（`companion/src/tool_bridge.rs`）**：`apeireth_tools::register_all` 之后串行 9 件 `register()`，**失败 eprintln 不阻断**；4 只读扩入默认日常包白名单（VSearch / CodeIntelligence / RepoQualityAnalyzer / ImageProcess）；5 件高危（EnhancedShell / FetchEngine / EnhancedBrowser / ImageGenEnhanced / EnhancedFileOps）仍走 RiskRule **5min 超时分级**。
- **3 个真 bug 修**（§3）：ImageGenParams serde derive 补齐、persist `open_in_memory` 补回 status 列等。
- **测试**：分 crate 测试数表（tool-codesearch 112、tool-image-gen 19、tool-image-process 38、tool-search 29、tool-filesystem 143、repo-tools 31、tool-registry 39，**合计 614 passed / 0 failed**）；companion N17 集成测试 2 件（`n17_tool_bridge_registers_all_nine_and_catalog_reflects` + `n17_nine_register_unregister_round_trip_zero_residue`）**已写在 `tool_bridge.rs` 末尾，待 WIP 解锁**（`memory_graph.rs` / `assemble.rs` 阻塞 lib 编译）。
- **跳过的（ponytail）**：`register.rs` 未补 serde derive `ToolDescription { brief }` 文案——留给 N18 文档生成器接走。
**现状复核（2026-10-23）**：**已过时（对象全部迁出 + 数字口径换代）**——9 个工具子 crate 现位于 [legacy/donor/apeireth-tool-shell](../legacy/donor/apeireth-tool-shell)、[apeireth-tool-fetch](../legacy/donor/apeireth-tool-fetch)、[apeireth-tool-browser](../legacy/donor/apeireth-tool-browser)、[apeireth-tool-codesearch](../legacy/donor/apeireth-tool-codesearch)、[apeireth-tool-image-gen](../legacy/donor/apeireth-tool-image-gen)、[apeireth-tool-image-process](../legacy/donor/apeireth-tool-image-process)、[apeireth-tool-search](../legacy/donor/apeireth-tool-search)、[apeireth-tool-filesystem](../legacy/donor/apeireth-tool-filesystem)、[apeireth-repo-tools](../legacy/donor/apeireth-repo-tools)、[apeireth-tool-registry](../legacy/donor/apeireth-tool-registry)；「合计 614 passed / 0 failed」为其历史口径，现行真源 341 套件 / 8570 passed / 0 failed / 51 ignored（[test-baseline.md](../docs/03-reference/test-baseline.md)）。**部分成立（能力口径需重述）**：现行工具面已换代——按 [capabilities-matrix.md](../docs/03-reference/capabilities-matrix.md) §0.2，只读工具面（`filesystem` / `search` / `repo`）**默认 ON**，而 shell（`APEIRETH_ENABLE_SHELL=1`）/ fetch（`APEIRETH_ENABLE_FETCH=1`）/ MCP 等**默认 OFF**（注册即闸），即本篇「4 只读扩入默认日常包白名单 / 5 件高危分级」的划分在现行树**须按默认开关表重述**，不能沿用「默认日常包」表述。**仍成立**：①「真注册到 registry，不打 log 空响」；②「unregister 幂等返 bool，重复卸 0 残留」；③「失败 eprintln 不阻断」；④「走原引擎不重复发明」；⑤「ponytail 明确列出跳过项并指定接手方（N18）」——五条均与现行 [AGENTS.md](../AGENTS.md) §1 诚实纪律一致，且被现行 [claims.yaml](../docs/status/claims.yaml) `tool.mcp_bridge`（默认 OFF，管理 UI/健康检查面未接线）等的 `notes` 写法延续。
**归属链接**：../legacy/donor/apeireth-tool-registry、../legacy/donor/apeireth-tool-shell、../legacy/donor/apeireth-tool-fetch、../legacy/donor/apeireth-tool-filesystem、../legacy/donor/apeireth-repo-tools、../docs/03-reference/capabilities-matrix.md、../docs/status/claims.yaml、../docs/03-reference/test-baseline.md、../AGENTS.md；历史（已迁出）`crates/apeireth-tool-registry/**`、`crates/apeireth-companion/src/tool_bridge.rs`

---

## 现状对照（2026-10-23）— 本册 7 篇横向口径

| 维度 | 原篇口径 | 现行口径（真源） | 判定 |
|---|---|---|---|
| 被评审草稿 / 主手册 | 附录 N 草稿（249 行）+ Omnibus 主文档 6241 行 | **均已迁出**，不在现行 121 篇内（[00-backlog](../docs/04-internal/backend-enhancement/00-backlog.md) D17 / [README](README.md)） | 已过时 |
| R11 资产（V1130/V1132/V1141/serve.py/R11-SEC-001/002） | 逐项引用并给数字 | 现行树无对应登记面（**未核实**是否另有实现） | 未核实 |
| Python 侧编排源码 | `apeireth/r11_orchestration.py` 3 stages | 现行主树为 Rust workspace 30 members；Python 侧不在现行 crate 面 | 已过时 |
| 测试数字 | 368 组全绿 / 614 passed（2026-08） | **341 套件 / 8570 passed / 0 failed / 51 ignored（2026-10-21）** | 已过时 |
| 工具默认开关 | 「4 只读入默认日常包 / 5 件高危分级」 | 按 [capabilities-matrix §0.2](../docs/03-reference/capabilities-matrix.md)：只读面默认 ON，shell / fetch / MCP **默认 OFF** | 已过时（须重述） |
| 验收纪律 | 「必须查树、不采信自报」+ 3 项报告夸大 | **仍成立**（[AGENTS.md](../AGENTS.md) §1、docs_truth / arch_guard 机器化防线） | 仍成立 |
| 交叉评审纪律 | 4 路评审 + 合并必改项 + 改动量上限声明 | **仍成立**（[CONSTRUCTION-DISCIPLINE.md](../docs/04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md)、债栏登记面） | 仍成立 |
| 口径分层纪律 | 模块级 gate vs 综合 gate 分开标注 | **仍成立**（能力矩阵 `supported/available/reason`、claims `notes` 残余标注） | 仍成立 |

**本册复核统计**：7 篇中标「**已过时**」结论 **11 条**、「**仍成立**」**7 条**、「**未核实**」**3 条**。

*本册为 `reports/` 合册产物；来源 7 篇与分配表 `doc-audit/reports-assignment.txt` §02 一致（逐篇唯一归册）。历史内容按原文保留、不追溯改写；现行口径一律另起「现状对照（2026-10-23）」并指明真源。*
