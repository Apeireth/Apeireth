<!-- 归档说明：本文件由会话内复核代理产出，归档时正文逐字保留；仅对中性表述做脱敏（第三方模型/厂商名 → 描述性表述），依据 AGENTS.md §2。 -->
> **性质**：文档治理批的**对抗性复核/验证报告**（举证记录，非规格、非现状真源）。
> **被审对象：后续批 `087cf72c..315bcff1`**
> **结论**：22 条断言 / PASS 18 / FAIL 4（A2 口径划错、B2、B7、C2 门禁盲区，均已被 `eadcc841` 修复）
> **读法**：想知道「这批改动的可信度凭什么」，读本报告；想知道某项能力现在到哪一级，查 `docs/status/claims.yaml`。

# 对抗性复核报告 —— docs/governance-followup-2026-10-24

- **被审工作树**：`/root/Documents/deepseek-harness/default-workspace/apeireth-docs-wt`（worktree，分支 `docs/governance-followup-2026-10-24`）
- **基点**：`b6225d57`　**tip**：`315bcff1`　**范围**：`087cf72c`、`315bcff1` 两个 commit
- **方法**：只读；历史内容一律 `git show <rev>:<path>`；变异测试在 `/tmp/mut`（`git archive HEAD | tar -x`）里做，被审工作树零写入（`git status --short` 复核前后均为 0 行）。
- **结论**：**22 条断言，PASS 18 / FAIL 4**；FAIL = **A2、B2、B7、C2**。

---

## 1. FAIL 条数与编号

| 断言 | 判定 | 一句话 |
|---|---|---|
| A1 | PASS | 基点 24 处新增 / tip 0 处，逐字复现 |
| **A2** | **FAIL** | K-device-approval-im-push.md:167 把「设备审批卡→IM 推送链」标成 PRODUCTION WIRED，真源矩阵同一句明写「IMPLEMENTED 但未接线、生产装配无注入点」 |
| A3 | PASS | `4 0` 纯追加，原句零改动 |
| A4 | PASS | 13→6 只减不增；7 条被删条目在基点就已失效；tip 基线 = tip 实际违规集（6/6 完全相等） |
| A5 | PASS | 4 条术语引用全部是纪律/图例句子，未见拿它当挡箭牌逃门禁 |
| B1 | PASS | 67 篇对 `087cf72c` 归一化后 59 同 / 8 仅链接 / **0 内容变** |
| **B2** | **FAIL** | 「20 篇因 CI/脚本/crates 引用而留原位」不成立：6/20 无任何 CI/脚本/crates/status 引用，3/20 本来就是 4 行桩；「口径门禁脚本直接读它」为假 |
| B3 | PASS | 67 个新桩、每个 4 非空行（5 物理行）、67/67 目标存在 |
| B4 | PASS | 238 条、目标 0 缺失、源 0 重复 |
| B5 | PASS | 88 篇集合运算无丢件（2 处名义不匹配均有解释） |
| B6 | PASS | 6 个子目录 声明=实有=表列=6/8/21/12/2/18，0 漏列 0 幻列 0 断链 |
| **B7** | **FAIL** | 顶层实际是 1 索引 + **17 真文件 + 70 桩**；README 写「1 + 20 真文件 + 67 桩」——把 3 个 2026-10-23 遗留桩当成真文件 |
| B8 | PASS | 52 可独立复现（8 篇移位件内被改写的链接目标数之和恰为 52） |
| C1 | PASS | 注入断链 → exit 1 |
| **C2** | **FAIL** | 删 `docs/04-internal/04-logs/README.md` → `docs_index.py --check` **exit 0、问题数 0**（门禁只查 `docs/*` 一层，不查新子目录） |
| C3 | PASS | 注入「已实现」→ exit 1 |
| C4 | PASS | 豁免表字段不全 → exit 1 |
| D1 | PASS | `crates/` 0 改动；19 个 `crates/*/README.md` 全同 |
| D2 | PASS | 139/139 blob hash 逐字节相同 |
| D3 | PASS | 7 项受保护资产全 SAME |
| D4 | PASS | status 干净、恰 2 commit、无 crates/Cargo/docs-status（附注：`scripts/doc-caliber-baseline.txt` 在 docs 外，属本批 A4 申报件） |
| D5 | PASS | 两 commit 均 Mavis，改动面只有 docs + 基线文件，无他人工作 |

---

## 2. 每条 FAIL 一句话

- **A2**：`docs/04-internal/backend-enhancement/K-device-approval-im-push.md:167` 标 `已接线〔四级口径：PRODUCTION WIRED〕：设备审批卡→IM 推送链`；真源 `docs/03-reference/capabilities-matrix.md:70` 对**同一条链**写 `IMPLEMENTED 但未接线：…全仓仅测试调用、生产装配无注入点`。声称"级别与真源一致"不成立，且是把真源改得更不准（IMPLEMENTED→PRODUCTION WIRED）。
- **B2**：「20 篇留原位，因为被 CI workflow / 仓内脚本 / crates/ 源码路径直接引用」实测：**6/20 在 scripts/、.github/、crates/、docs/status/ 里零命中**（ENGINEER-MANIFESTO、HANDOFF-NOTES、TO-NEW-TEAM、github-fetch-guide、next-team-handbook、o6-session-log-2026-08-27）；其中 **3/20 在基点就是 4 行重定向桩**（不是真文件）；另 `engineering-review-handoff-2026-10-06.md` 的留位理由「**口径门禁脚本直接读它**」为假——`scripts/check_doc_caliber.py` 只在第 2 行 docstring 提及，`.github/workflows/doc-caliber.yml` 只在第 3 行注释提及，无任何脚本打开该文件。
- **B7**：`docs/04-internal/README.md:32` 写「88 = 1 篇目录索引 + **20 篇留原位的真文件** + **67 个 4 行桩**」。实测顶层 88 个 .md = 1 README + **17 真文件** + **70 桩**（67 个本批新桩 + 3 个基点遗留桩 HANDOFF-NOTES / TO-NEW-TEAM / next-team-handbook）。真文件多算 3、桩少算 3；6 个子目录 6/8/21/12/2/18 的数字则完全正确。
- **C2**：删掉 `docs/04-internal/04-logs/README.md` 后 `python3 scripts/docs_index.py --check` 输出 `[docs_index] 问题数: 0`、`EXIT=0`。对照实验：删**顶层** `docs/04-internal/README.md` 会被抓（`目录缺索引 README.md: docs/04-internal/`）。根因：`scripts/docs_index.py` 的 `check()` 规则 1 只遍历 `(ROOT/"docs").iterdir()` 一层（第 269–275 行），本批新增的 6 个二级子目录不在检查面内。

---

## 3. 最严重的一条：A2（把「未接线」标成 PRODUCTION WIRED）

这是本批唯一一处**反向污染真源**的改动：留欠账只是账不平，错标级别会让下游读者/脚本把未闭合的链路当生产可用。

证据链（命令 + 实测输出）：

1. **被改的标注**
   ```
   $ sed -n '167p' docs/04-internal/backend-enhancement/K-device-approval-im-push.md | cut -c1-120
   - **已接线〔四级口径：PRODUCTION WIRED〕**：设备审批卡→IM 推送链（同链同信封 + plan_id 幂等 + 失败留账重试）、
   ```

2. **真源对同一条链的判定**（`docs/03-reference/capabilities-matrix.md:70`，本批未改该行）
   ```
   $ sed -n '70p' docs/03-reference/capabilities-matrix.md | grep -o "设备动作计划卡 → IM 推送链.\{0,260\}"
   设备动作计划卡 → IM 推送链（subject `tool.device_control.plan`，…）**IMPLEMENTED 但未接线**：
   `ImBridge::with_device_approval_face` / `push_device_approval_cards` / `retry_device_approval_pushes`
   全仓仅测试调用、生产装配无注入点，桥内 face 缺位照实回「未接线」——故 money 档的 IM 卡出卡来源在生产装配下未闭合
   ```

3. **代码实况**（crates/ 本批 0 改动，故此判定对 tip 仍成立）
   ```
   $ grep -rn "push_device_approval_cards\|with_device_approval_face\|retry_device_approval_pushes" crates/ --include=*.rs | sed 's/:.*//' | sort | uniq -c
        12 crates/adapters/gateway/tests/im_device_cards.rs
         4 crates/adapters/gateway/src/im_bridge.rs
         1 crates/adapters/gateway/src/im_device.rs
   ```
   无 runtime-assembly / cli 生产装配调用点 → 矩阵「生产装配无注入点」成立。

4. **同一文档自相矛盾**
   ```
   $ sed -n '5p' docs/04-internal/backend-enhancement/K-device-approval-im-push.md
   > 报备的两条未接线缝：**IM 审批卡推送面**（本批由 gateway 消费 …
   $ sed -n '170,173p' …
   - **未接线**（如实）：… 运行时侧「预览拍产卡 → 自动提交推送」的触发器——本批交付
     `ImBridge::push_device_approval_cards` 显式提交口（谁看见卡谁提交），随 IM 装配批挂生产桥时接；
   ```
   文档头把「IM 审批卡推送面」列为**未接线缝**，第 167 行却标 PRODUCTION WIRED。

对照：同一批里 V5 与 care-heartbeat 两篇的标注是**窄口径**的（V5:147 只标「机制面 + tools 后置出站面」，紧邻的 149 行照实列「未接线：网关/补全链路的出境挂接…」），说明作者知道该怎么划边界——K-device-approval-im-push 这一处是划错了。

---

## 4. 变异测试结果表（工作副本 `/tmp/mut`，`git archive HEAD` 解出）

| # | 注入内容 | 命令 | 期望 | 实测退出码 | 判定 |
|---|---|---|---|---|---|
| C1 | 向 `docs/gateway-api-contract.md` 追加 `[断链探针](no-such-file-c1-probe.md)` | `python3 scripts/docs_links.py --check` | 1 | **1**（`断链: 1`，`BROKEN … no-such-file-c1-probe.md (目标不存在)`） | PASS |
| C2 | `rm docs/04-internal/04-logs/README.md` | `python3 scripts/docs_index.py --check` | 1 | **0**（`问题数: 0`） | **FAIL** |
| C3 | 向 `docs/03-reference/capabilities-matrix.md` 追加独立段 `本模块已实现。` | `python3 scripts/check_doc_caliber.py` | 1 | **1**（`capabilities-matrix.md:231 … 新增违规 1 处`） | PASS |
| C4 | 向 `docs/_known-missing.tsv` 追加三字段行 `docs/foo/bar-probe.md\tv1-moved\tprobe with only three fields` | `python3 scripts/docs_links.py --check` | 1 | **1**（`豁免表行 49 字段不足 5 …`） | PASS |

补充对照（证明 C2 不是脚本缺失而是覆盖面缺失）：
- 删**顶层** `docs/04-internal/README.md` → `问题数: 1 ｜ 目录缺索引 README.md: docs/04-internal/`
- 删**二级子目录** `04-logs/README.md` → `问题数: 0`
- 删二级子目录里的**正文** `04-logs/engineering-log-2026-09-22.md` → exit 1，但抓它的是**重定向表规则**（`重定向目标不存在（行 68）`），不是 INDEX 陈旧检测；即：没有重定向条目的文件被删，`docs_index --check` 仍无感。

变异后已全部还原，三个门禁在 `/tmp/mut` 回到 `links=0 / index=0 / caliber=0`。

---

## 5. 额外发现（真实缺陷，但不属于 22 条断言，单列不计 FAIL）

1. **67 个新桩全部带生成缺陷**：每个桩第 1 行都是
   `<!-- 已移位（2026-10-23）：本文内容已已移至。 -->`
   ——「已已」重复，且「移至」后面**没有目标**（目标被漏掉）。67/67 命中：
   ```
   $ python3 … 统计: stubs with 已已移至 typo: 67 of 70
   ```
   3 个基点遗留桩（已合并/已归档模板）无此缺陷，可确认是本批新模板引入。

2. **backlog.md 的桩指向另一个桩，且与重定向表互相矛盾**：
   - 桩：`docs/04-internal/backlog.md` → `docs/04-internal/05-plans/backlog.md`
   - 但 `05-plans/backlog.md` 本身是基点遗留的「已归档」4 行桩（内含 `自动重定向生成` 1 处），正文指向 `backend-enhancement/00-backlog.md`
   - 重定向表：`docs/04-internal/backlog.md → docs/04-internal/backend-enhancement/00-backlog.md`（直达终址，与桩不一致）
   - 后果：点桩落到第二个桩（2 跳），且该桩写「内容以 **backlog** 为准」，而内容其实不在那里。
   - 连带：`05-plans/` 的「18 篇」里含这 1 个桩，不是 18 篇正文。

3. **67 次移位在 git 里没有 rename 记录**：`git show --name-status -M --format='' 315bcff1` 只有 `73 A / 70 M / 0 R`（旧路径留桩，git 无法判重命名）。内容一致性可用 blob 比对证明（B1 已做），但 `git log --follow` / blame 追不到新址。

4. **commit message 的口径算术小错**（087cf72c）：正文写「真能力声明（20 条）」，实为 **19 条**（12 PRODUCTION WIRED + 7 IMPLEMENTED）；「human-side-design-review ×3」实为 **4 条** IMPLEMENTED（:28/:47/:81/:85）。总数 24 与「23 条就地标注 + 1 条 ADR 只增不改」对得上，不影响门禁数值。

---

## 6. 我未能查到的

1. **A2 里 7 条 IMPLEMENTED 中的 4 条没有可对表的真源**：`human-side-design-review.md:28/47/81/85`（情绪/性格/记忆/机制）与 `founder-ai-qa.md:72`（星群=每户独立伙伴实例）在 `claims.yaml` 29 条 claim 与能力矩阵里都没有对应条目。我只能证明它们**不构成过度声称**（矩阵/claims 里没有更高的级别与之冲突，且 claims 的 `voice_emotion_tune` 注记写明「情绪源未接线」），**不能**证明 IMPLEMENTED 就是真源指定值。若这 4 条实际已是 PRODUCTION WIRED，则属"反向标低"，我无法从仓内真源判定。
2. **A2 里另外 4 条 PRODUCTION WIRED 缺直接真源**：`capabilities-matrix:69/:146`（care 消费端）、`A-quota-real-interface.md:32`（CircuitBreak）、`gateway-api-contract.md:25`（§2 端点整节）、`00-PHILOSOPHY.md:114`（gateway presence）。矩阵对应行/文档自述与 PRODUCTION WIRED 不冲突，但 claims.yaml 无对应 claim，我无法独立确证到"生产装配"粒度。
3. **B4 的 `backlog.md` 重定向该以哪条为准**：桩与表给出两个不同终址，仓内没有第三方记录能判定哪个是"本意"；我只能报不一致，不能判定谁错。
4. **B1 的"8 篇仅链接被改写"是否每一条改写都语义正确**：`docs_links.py --check` exit 0 只证明目标存在，不证明改写后指向的是"原本想指的那一篇"；我逐个人工看了 8 篇的 diff，未发现指错，但这属人工判断，不是机器判据。
5. **"0 装真账"里 docs_truth / arch_doc / arch_guard 三项**：不在 22 条断言内，我未逐项复跑（只复跑了 docs_links / docs_index / check_doc_caliber 三道门禁）。

---

## 7. 逐条判据（命令 + 实测输出）

### A1 —— PASS
```
$ cd /tmp/aud-base && python3 scripts/check_doc_caliber.py ; echo EXIT=$?
[caliber] 扫描 361 个活跃文档 (跳过 1105 个历史文档), 新增违规 24 处 (基线内 6 处)
EXIT=1

$ cd /tmp/aud-head && python3 scripts/check_doc_caliber.py ; echo EXIT=$?
[caliber] 扫描 397 个活跃文档 (跳过 1142 个历史文档), 新增违规 0 处 (基线内 6 处)
EXIT=0
```
24→0 逐字复现。扫描面 361→397（+36 活跃）与 1105→1142（+37 历史）之差 = 73 = 67 桩 + 6 个子目录 README，自洽。

### A2 —— FAIL
见第 3 节。全部 23 条标注的级别分布实测：`PRODUCTION WIRED 12 / IMPLEMENTED 7 / 术语引用 4`，其中仅 `K-device-approval-im-push.md:167` 与真源冲突。抽查覆盖了 12 条 PW 与 7 条 IMPLEMENTED（不是抽样 6 条，是全查），其余 22 条未发现与 claims/矩阵冲突，例如：
- `K-device-control-three-beat.md:143` PW ← 矩阵 `四级口径 = 工具面/三拍/… = IMPLEMENTED + PRODUCTION WIRED`（一致）
- `V5-a-redaction-bridge-delivery.md:147` PW/默认 OFF ← claims `sec.egress_gate` = PW / OFF（一致，且标注窄到「机制面 + tools 后置出站面」，紧邻照实列未接线面）
- `care-heartbeat-producer.md:177/181` PW/默认 OFF ← claims `bio.circadian` = PW / OFF（一致）
- `stage123-gap-matrix.md:98` IMPLEMENTED ← claims `sec.governance_chain_drill` = IMPLEMENTED（一致）
- `relationship-state-interface.md:76` IMPLEMENTED ← 原句自带「已实现未接线」（一致）

### A3 —— PASS
```
$ git diff --numstat b6225d57 315bcff1 -- docs/adr/0010-r8-governance-pipeline.md
4	0	docs/adr/0010-r8-governance-pipeline.md
```
0 删除行；hunk 头 `@@ -239,6 +239,10 @@`，新增 4 行在段末，原句逐字未动。补注内容自称「上句 = IMPLEMENTED」「接生产装配是另一层事实（见 claims `gov.chain_hook_adapter` = PRODUCTION WIRED）」——claims 实值为 PRODUCTION WIRED，引用正确。

### A4 —— PASS
```
基点基线 13 条；tip 基线 6 条；tip 实际违规集 6 条；diff = 只删 7 行、零新增
comm -12 b0 v0  → 基点就在违规的 6 条（= 保留下来的 6 条）
comm -23 b0 v0  → 被删的 7 条，在基点**已不是违规**：
  backlog.md:156 / backlog.md:351 / HANDOFF-NOTES.md:161 / HANDOFF-NOTES.md:7 /
  00-PHILOSOPHY.md:112 / gateway-api-contract.md:111 / gateway-api-contract.md:23
diff b1 v1 → IDENTICAL（tip 基线 == tip 实际违规集，无藏匿、无死条目）
```
② 被删 7 条在基点已是死条目（行号漂移/文件已被前批缩成 4 行桩），不是把活违规删掉；③ `b1 ⊂ b0`，无新增条目。

### A5 —— PASS
4 条术语引用原文：
```
05-product-insights.md:35  差距审计两级化（"库级已实现但未接生产装配"〔四级口径术语引用〕与"真缺口"分开记账）
05-product-insights.md:39  写"可用/已实现"〔四级口径术语引用〕之前先答**哪一级**，答不出就不写
05-product-insights.md:50  不许把"已实现未接线"〔四级口径术语引用〕说成 0 装
human-side-design-review.md:5  标记：✅=已实现〔四级口径术语引用，逐条标注见各节〕
```
四条都是讲**纪律词汇/图例**本身，无主语能力声明；同一文档里真正的能力主张（:28/:47/:81/:85）另标了 IMPLEMENTED，没有被"术语引用"吞掉。未发现挡箭牌式滥用。

### B1 —— PASS
```
$ python3（blob 比对，LINK 归一化后）
vs 087cf72c: identical=59 link-only=8 REAL=0
link-only: absorption-candidates-2026-10.md, backlog.md, handover-ledger.md,
           migration-v1-to-v2.md, plugin-authoring-guide.md, release-checklist.md,
           release-plan.md, team-work-doc.md
```
若按断言字面对 `b6225d57` 比，会得到 1 篇"内容变"（stage123-gap-matrix.md）——那是 `087cf72c` 本批第一条 commit 的口径标注，不是移位改的；以移位父提交 `087cf72c` 为基准，0 篇内容变。

### B2 —— FAIL
逐篇 grep（scope：`scripts/ .github/ crates/ docs/status/ docs/architecture/ docs/03-reference/` + AGENTS.md）：

| 篇 | CI/脚本/crates/status 命中 | 仅散文文档/AGENTS.md |
|---|---|---|
| ENGINEER-MANIFESTO | 无 | docs/architecture ×2、docs/03-reference ×1、AGENTS.md ×2 |
| HANDOFF-NOTES（**已是桩**） | 无 | docs/architecture ×1、docs/03-reference ×1 |
| TO-NEW-TEAM（**已是桩**） | 无 | 同上 |
| ai-os-landscape-2026-10-18 | scripts/check-neutral-terms.ps1（路径数组） | — |
| async-trait-decision-matrix | crates/foundation/plugin/src/llm_factory.rs | — |
| ci-fix-log-2026-08 | .github ×2 + scripts/release-prep.sh（git pathspec exclude） | — |
| cognitive-module-wiring | crates ×3 | — |
| engineering-review-handoff-2026-10-06 | .github 注释 + scripts docstring（**非"直接读"**） | — |
| github-fetch-guide | 无 | docs/03-reference ×1、AGENTS.md ×1 |
| governance-chain-drill-2026-10-20 | docs/status ×3 | — |
| kani-verification-playbook | .github/workflows/kani.yml | — |
| live-verification-ledger | scripts/doc-caliber-baseline.txt + docs/status | — |
| maintenance-guide | scripts ×2 + crates/Cargo.toml | — |
| next-team-handbook（**已是桩**） | 无 | docs/03-reference ×1 |
| o6-session-log-2026-08-27 | 无 | docs/03-reference ×1 |
| scene-d-v2-plan | crates ×6 | — |
| secret-management-policy | .github + scripts ×2 | — |
| v1-vs-v2-capability-gap-audit | crates ×2 | — |
| v2-unabsorbed-features | crates ×12 | — |
| v2.0.0-rc-roadmap | crates ×12 | — |

6/20 无机器引用；3/20 基点即桩（`git show b6225d57:<path>` 与 tip 逐字节相同，均为「已合并/已归档」模板）。另：
```
$ grep -rn "engineering-review-handoff" scripts/ .github/ crates/
scripts/check_doc_caliber.py:2:"""文档四级口径守门 (W8③, per docs/04-internal/engineering-review-handoff-2026-10-06.md §5 W8).
.github/workflows/doc-caliber.yml:3:# W8③ (docs/04-internal/engineering-review-handoff-2026-10-06.md §5 W8):
crates/adapters/cli/tests/production_knobs.rs:141:// ---- 2026-10-06 W2 接线批旋钮 …
crates/adapters/cli/src/lib.rs:190:// 2026-10-06 W2 接线批 …
```
无 `open()`/`read_text()` 读该文件 → README:46「**口径门禁脚本直接读它**」不成立。

### B3 —— PASS（附缺陷，见第 5 节）
```
顶层 .md 88；含「重定向生成」的桩 70；其中带本批模板（已已移至）67；基点遗留桩 3
桩物理行分布 {5:70}；非空行分布 {4:70} → ≤10 行
67/67 目标存在；目标分布 02-research 21 / 05-plans 18 / 03-audits 12 / 01-handoffs 8 / 00-governance 6 / 04-logs 2 = 67
```
「4 行桩」按非空行计为 4，按物理行计为 5（含 1 空行）。

### B4 —— PASS
```
data rows 238；两字段不全行 0；目标不存在 0；源重复 0
67 个新桩的旧路径 100% 在表中登记；其中 1 条（backlog.md）表的终址与桩目标不一致（见第 5 节第 2 条）
```

### B5 —— PASS
```
基点顶层 .md 88 个名字；tip 顶层同名 88 个（67 变桩、20 原位、README 重写）
内容/blake 比对：86/88 在 tip 找到同内容（或链接归一化后同内容）对应件
2 个名义未匹配：README.md（本批重写，预期）、stage123-gap-matrix.md（087cf72c 加了口径标注；新址 03-audits/ 同名文件存在）
→ 88 篇一篇不少；基点原有的 2 组内容重复（HANDOFF-NOTES=TO-NEW-TEAM、backlog=next-team-handbook）是基点既有，不是本批产生
```

### B6 —— PASS
```
00-governance 声明6 / 实有6 / 表列6
01-handoffs  声明8 / 实有8 / 表列8
02-research  声明21/ 实有21/ 表列21
03-audits    声明12/ 实有12/ 表列12
04-logs      声明2 / 实有2 / 表列2
05-plans     声明18/ 实有18/ 表列18
缺列 0、幻列 0、表内链接断 0
```

### B7 —— FAIL
```
$ grep -n "88 个条目" docs/04-internal/README.md
32:顶层共 **88 个条目** = **1 篇目录索引（本文件）+ 20 篇留原位的真文件**（下表）+ **67 个 4 行重定向桩**
实测：88 = 1 README + 17 真文件 + 70 桩（67 新 + 3 遗留）
真文件 17 个 = ENGINEER-MANIFESTO, ai-os-landscape, async-trait-decision-matrix, ci-fix-log,
  cognitive-module-wiring, engineering-review-handoff, github-fetch-guide, governance-chain-drill,
  kani-verification-playbook, live-verification-ledger, maintenance-guide, o6-session-log,
  scene-d-v2-plan, secret-management-policy, v1-vs-v2-capability-gap-audit,
  v2-unabsorbed-features, v2.0.0-rc-roadmap
表里被当成真文件的 3 个桩 = HANDOFF-NOTES.md, TO-NEW-TEAM.md, next-team-handbook.md
6 个子目录篇数 6/8/21/12/2/18 —— 全部正确
```

### B8 —— PASS
52 可独立复核，口径 = 8 篇"仅链接被改写"的移位件内，`](...)` 目标发生变化的数量之和：
```
absorption-candidates-2026-10  4/4
backlog.md                     1/1
handover-ledger.md             15 链接 / 8 改
migration-v1-to-v2             2/2
plugin-authoring-guide         6/6
release-checklist              4/4
release-plan                   15 / 13 改
team-work-doc                  14/14
合计 52
```

### C1–C4 —— 见第 4 节（C2 FAIL）

### D1 —— PASS
```
$ git diff --name-only b6225d57 HEAD -- crates/ | wc -l
0
$ git ls-tree -r --name-only b6225d57 -- crates/ | grep -c '/README.md$'
19
$ git diff --quiet b6225d57 HEAD -- 'crates/**/README.md' && echo ALL-SAME
ALL-SAME
```

### D2 —— PASS
```
139 个文件；逐文件比较 b6225d57:<f> 与 HEAD:<f> 的 blob hash：checked=139 mismatches=0
```

### D3 —— PASS
```
SAME docs/03-reference/test-baseline.md
SAME docs/status/claims.yaml / wiring.yaml / verification-ledger.yaml
SAME docs/architecture/contract.toml / env-registry.toml / exceptions.toml
```

### D4 —— PASS
```
$ git status --short | wc -l          → 0
$ git rev-list --count b6225d57..HEAD → 2
$ git diff --name-only b6225d57 HEAD -- crates/ Cargo.toml Cargo.lock docs/status/ | wc -l → 0
```
附注：`087cf72c` 改了 `scripts/doc-caliber-baseline.txt`（在 docs 之外），该文件是本批 A4 明确申报的对象，不计夹带；因此"都是 docs 范围"字面不严格成立，但括号内三条禁令全部成立。

### D5 —— PASS
```
$ git log --format='%h|%an|%ae|%s' b6225d57..HEAD
315bcff1|Mavis|Mavis@apeireth.local|refactor(docs): 04-internal …
087cf72c|Mavis|Mavis@apeireth.local|fix(docs): 清零 24 条四级口径欠账 …
改动面：087cf72c = 14 个 .md + 1 个基线文件；315bcff1 = 67 移位 + 67 桩 + 6 子目录 README + 顶层 README + INDEX.csv + _redirects.tsv
无 crates/、无 Cargo.toml、无 docs/status/、无他人 commit
```
