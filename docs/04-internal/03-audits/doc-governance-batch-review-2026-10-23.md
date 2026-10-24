<!-- 归档说明：本文件由会话内复核代理产出，归档时正文逐字保留；仅对中性表述做脱敏（第三方模型/厂商名 → 描述性表述），依据 AGENTS.md §2。 -->
> **性质**：文档治理批的**对抗性复核/验证报告**（举证记录，非规格、非现状真源）。
> **被审对象：文档治理批 tip `b6225d57`（PR #59）**
> **结论**：22 条断言 / PASS 11 / FAIL 6（其中 5 项经本批修复，另 1 项为历史 commit 无法改写）
> **读法**：想知道「这批改动的可信度凭什么」，读本报告；想知道某项能力现在到哪一级，查 `docs/status/claims.yaml`。

# 对抗性复核报告 —— 文档治理批（`docs/consolidation-2026-10-23`）

> **复核人**：对抗性复核员（只读复算，不修改被审仓库）
> **复核日期**：2026-10-23
> **被审分支**：`docs/consolidation-2026-10-23`
> **被审 HEAD**：`4142eddf`（复核期间该分支仍在移动，见 §0）
> **merge-base**：`dfce42bf`
> **交接文档**：`apeireth-doc-governance-handover-2026-10-23.md`（2026-10-23 21:34 写就）

---

## 0. 前置：被审对象在复核期间移动了 6 次

复核开始时 HEAD = `c08f65e7`（13 commit）。复核期间分支被追加了 6 个 commit，**其中 3 个直接回应我在本次复核中发现的缺陷**：

```
02a58a43  fix(docs): 恢复 28 篇归档件为逐字基线 —— 修复器越界改写了历史
94b837db  fix(docs): 桩的措辞与粗体标记修正 —— 32 个移位桩原本被写成「已归档」
dcc3437e  docs(gate): known-missing 理由改准确 —— legacy 路径在两仓均不存在
42dc6392  fix(docs): 修 48 处 markdown 链接语法断裂 —— 修复器丢了开括号   ← 回应 FAIL-5
5362daf4  fix(docs): 修链接修复器的根因 —— 裸路径匹配跳进链接内部，改了链接文字  ← 回应 FAIL-5
4142eddf  fix(docs): 受保护文件还原基线 + 修复器永久跳过受保护路径          ← 回应 FAIL-4b
```

**这是本次复核最有力的证据**：§FAIL-5（56 处畸形链接）和 §FAIL-4b（保护清单被碰）**是被独立复核发现的，不是被审方自己发现的**——`42dc6392` 的 commit 标题直接写明"修复器丢了开括号"，`5362daf4` 写明"裸路径匹配跳进链接内部"。**如果没有独立复核，这两个缺陷会带着"断链 0 / 门禁可信"的假象进入合并。**

**修复后残留**（我的独立解析器实测，`4142eddf`）：
- 畸形链接：63 → **5**（其中 2 处是本批引入、3 处是 pre-batch 遗留）
- 但引入了 **83 处"多余右括号"残留**（pre-batch 38 → 现 83）：`[x.md](../x.md))` —— 链接**能点开了**，但正文多一个 `)`

**交接文档 §2 说"13 个 commit"，实际已 19 个**，且交接文档本身未随之更新 → 这条也是 FAIL 项。我下面所有计数都以**已提交的 `4142eddf`** 为准。

---

## 1. 结论表（17 条）

| # | 断言 | 判定 | 一句话证据 |
|---|---|---|---|
| 1 | 活文档 403 / 卷宗 673 | **FAIL** | 卷宗 673 成立；活文档实测 **404**（`find docs -name '*.md' -not -path 'docs/archive/*'`），差 1 |
| 2 | 12 册 / 每册分节数 == 目录表行数 / 合计 121 / 恰好归一册 | **PASS** | 12 册分节数逐一 == 目录表行数 == 分配表计数，三者全等，合计 121；121 篇原件与 pre-batch `reports/` 121 篇 .md **双向完全覆盖，0 重复 0 遗漏** |
| 3 | `_redirects.tsv` 172 条，目标真实存在 | **PASS** | 172 条有效条目（另 2 行是 `#` 注释），**172/172 目标全部存在** |
| 4 | `_known-missing.tsv` 34 条，每条分类正确 | **FAIL** | 34 条计数成立，但 **4 条 `f02-out` 分类与 F-02 出库清单对不上**；1 条 `legacy` 的分类理由写错（`legacy/` 实际是 git 跟踪的，1929 个文件）；且 `legacy` 那条链接**是本批自己新写进去的** |
| 5 | 口径体检 pre-batch = 25，剩 24 条全落在未改过的文件 | **FAIL** | pre-batch 25 成立（ea6099e3 与 dfce42bf 都是 25）；但剩余 24 条里 **5 个文件在本批 diff 内**，其中 `05-product-insights.md` 被本批**实质修改过**，且这 5 个文件里的 2 个正是本批引入的畸形链接所致（在 `4142eddf` 修复畸形链接后，该文件已回到 pre-batch 内容，但"我从未碰过 backend-enhancement/\*"的叙述仍与 git diff 冲突） |
| 6 | 变异测试：(a) 断链 (b) 删目录 README (c) 非法 INDEX 行 → 两个门禁都变红 | **PASS** | 三项变异全部被抓住，exit=1，复原后 exit=0（**但见 §B6 补充：门禁有更大盲区 —— 畸形链接 56 处，修复器根因在 `5362daf4` 才修**） |
| 7 | `docs_links.py` 落穿分支已修 | **PASS** | 未登记的失效链接被报为 `BROKEN ... (目标不存在)`，exit=1；重定向命中的合法链接正确放行 |
| 8 | `--gen` 不重置 `disposition` / `target` | **PASS** | 手工改写为 `MANUAL-DISPOSITION-XYZ` / `MANUAL-TARGET-ABC`，跑 `--gen` 后两值原样保留 |
| 9 | 门禁没把 archive 内部历史链接算成断链 | **PASS** | live 断链 **0**；all 断链 668；差值 668 == archive 内部失效链接数，live 口径为 0 |
| 10 | 抽 8 篇归档/移位原件逐字节 diff | **FAIL** | 归档件 **139/139 逐字节相同**（含 121 篇 reports 原件，全 OK）；但**移位件 23/32 不一致** —— 移位时链接被重写坏，且原文正文被改 |
| 11 | 保护清单零触碰 | **FAIL（已部分修复）** | 6 项生成物/契约 **全程 0 触碰**（claims/wiring/verification-ledger/contract.toml/env-registry/exceptions/capabilities-matrix/philosophy 全 clean）；`docs/03-reference/test-baseline.md` 与 `docs/adr/{0007,0008,README}.md` 曾被本批改坏成畸形链接，**已由 `4142eddf` 还原到 pre-batch 内容**（`git diff dfce42bf HEAD` 现为空）；但 `git log dfce42bf..HEAD -- <path>` 仍非空 → 按任务判据（"被审方的 commit 是否修改过"）仍判 FAIL |
| 12 | 接手手册正文无第三方产品名 | **PASS** | `team-handover-reference.md` grep `anthropic\|openai\|claude\|gpt-\|minimax\|deepseek\|gemini\|qwen\|vcp` **零命中**；本批还**删掉了**原有的 （厂商名，脱敏）/（厂商名，脱敏）/（厂商名，脱敏）/vcp 字样 |
| 13 | 与主仓 HEAD 合并 0 冲突 | **PASS** | `git merge-tree --write-tree` 返回树 oid `10dffdfad…`，**冲突列表为空**，exit=0；legacy 3-arg 形式冲突标记数 0；主仓与被审分支均未被改动 |
| 14 | 交接文档每个数字逐个复算 | **FAIL** | 声称 vs 实测见 §D 表：**11 个数字对不上**（活文档 403→404、活文档基线 904→无法复现、卷宗基线 543→无法复现、reports 203→195、断链 690→691、活文档断链 140→137、archive 内部 550→554、重定向桩 140→171、子目录 5→4、移位/留原位 33/27→32/28、目录 README 12→15、commit 数 13→19） |
| 15 | 74 个非文档文件保留原位 | **PASS** | `reports/` 非 .md 文件：基线 74 → HEAD **74**；`git diff --diff-filter=D -- reports/` 除 1 个 .md 外**无删除**，无移位 |
| 16 | 6 项"没做/留给总指挥"确实没做 | **PASS** | 轴内交付册合订未做（04-delivery 仍是 12 个散篇，无合册文件）；known-missing 仍 34 条；24 条存量口径债仍在（exit=1）；04-internal 顶层仍 88 篇未重排；**两条门禁未接 CI**（`grep docs_links\|docs_index .github/workflows/` 零命中）；分支保护/尺寸棘轮未碰 |
| 17 | 抽查 6 个重定向桩措辞/目标/行数 | **PASS** | 6 个桩全部：措辞与目标匹配（"已合并至"→确实合并目标；"已移位"→确实新址；"已归档"→确实卷宗/合册），**均 4 行非空 ≤ 10**；全表 171 个桩逐一语义校验，0 语义错配 |

**合计：17 条中 PASS 11 / FAIL 6。**

> **复核期间的状态变化**：被审方在复核进行中追加了 6 个 commit，其中 3 个直接修复本报告 FAIL-4b（保护清单被碰）和 FAIL-5（56 处畸形链接）。修复后重测：
> - 畸形链接 63 → 5（残留 5 处：2 处本批引入 + 3 处 pre-batch 遗留）；新增 83 处"多余右括号"残留（pre-batch 38 → 83）
> - 保护清单文件内容已与 pre-batch 逐字一致（`git diff dfce42bf HEAD -- <保护清单>` 为空）
> - 门禁仍报 live 断链 0 / 索引问题 0，合并仍 0 冲突
> - **仍然成立的 FAIL**：断言 1（404≠403）、4（5 条分类错）、5（归属叙述）、10（移位件 23/32 内容不一致）、14（11 个数字对不上）
> - **断言 11 降级为"已部分修复"**：内容已还原，但 commit 历史里确实改过

---

## 2. FAIL 明细

### FAIL-1（断言 1 & 14）：活文档数 404，声称 403

**复算命令**
```bash
cd /root/Documents/deepseek-harness/default-workspace/apeireth-docs-wt
find docs -name '*.md' -not -path 'docs/archive/*' | wc -l          # 404
git ls-tree -r -z --name-only HEAD -- docs | tr '\0' '\n' | grep '\.md$' | grep -v '^docs/archive/' | wc -l   # 404
```
**实际输出**：`404`（工作区与已提交 HEAD 均为 404）

**影响面**：低。404 vs 403 差 1，是口径微差（可能含某个 README 或生成物）。但交接文档把 403 当作验收数字写进了 §1、§6 两处，且 §6 用它做"904 → 403"的核心叙事。卷宗 673 **成立**。

---

### FAIL-2（断言 4 & 14）：known-missing 分类错误 5 条

**复算命令**
```bash
# 4 条 f02-out 对不上 F-02 出库清单
M=docs/04-internal/backend-enhancement/f-02-migration-manifest-2026-10-18.tsv
for n in V22-mcp2-workspace-reproducibility about-raw.json agent-r162-5-integration docs-audit-2026-10-18; do
  echo "$n -> manifest_hits=$(grep -c "$n" $M)"; done
# legacy/ 是否 git 跟踪
git ls-files legacy/ | wc -l          # 1929
ls -d legacy/donor                    # 存在
# legacy 引用是否本批新写
git grep -l 'legacy/donor/apeireth-protocol' dfce42bf -- '*.md'   # 空
grep -c 'legacy/donor/apeireth-protocol' reports/03-audit-and-verification.md  # 2
```
**实际输出**
```
V22-mcp2-workspace-reproducibility -> manifest_hits=0
about-raw.json                    -> manifest_hits=0
agent-r162-5-integration           -> manifest_hits=0
docs-audit-2026-10-18              -> manifest_hits=0
legacy/ tracked files: 1929
legacy/donor exists: yes
git grep at dfce42bf: (空)
本批新写入: reports/03-audit-and-verification.md ×2, reports/06-security-and-sandbox.md ×2
```

**问题分解**
1. **4 条 `f02-out` 分类不成立**：F-02 出库清单（917 行，reports 组 878 件）里**根本没有** `V22-mcp2-workspace-reproducibility.md`、`r20-stage-6-th/about-raw.json`、`agent-r162-5-integration-6-commit-paiban-24-locked-entry-v11-release-2026-08-11.md`、`artifacts/docs-audit-2026-10-18.md`。它们**不是出库件**，却被登记为"可从 tar 或 git 历史取回"。另 18 条 `f02-out` 经清单逐条核对 **OK**。
2. **1 条 `legacy` 分类理由写错**：理由写"legacy/ 不入 git 跟踪面"，但 `legacy/` **实际是 git 跟踪的**（1929 个文件，`legacy/donor/` 目录确实存在）。真实理由只是"该子目录不在树上了"。**理由与事实相反**，属"0 装诚实纪律"违规。
3. **`legacy` 那条链接是本批自己新造的**：pre-batch 全树 grep **零命中**，本批在合册的"现状复核"批注里新写了这 2 处指向不存在路径的链接，然后把它们登记进白名单。**自造断链 → 自列豁免**，这正是 §3.1 声称要根治的"门禁失明"模式的变种。

**影响面**：中。`_known-missing.tsv` 是门禁"不算断链"的唯一依据，分类错误会让这 4 条既不算断链、也拿不回原件。**而且 `legacy` 那条是本批自造的，本可以直接修掉。**

---

### FAIL-3（断言 5 & 14）：口径体检的"24 条存量债"归属证伪

**复算命令**
```bash
# pre-batch 实测
git archive ea6099e3 | tar -x -C /tmp/rev/pre_ea && cd /tmp/rev/pre_ea && python3 scripts/check_doc_caliber.py | tail -1
#   → 新增违规 25 处
git archive dfce42bf | tar -x -C /tmp/rev/pre_dfce && cd /tmp/rev/pre_dfce && python3 scripts/check_doc_caliber.py | tail -1
#   → 新增违规 25 处
# HEAD 实测
python3 scripts/check_doc_caliber.py | tail -1     # → 新增违规 24 处
# 交叉验证：24 条落在哪些文件
git diff --name-only dfce42bf HEAD > /tmp/changed.txt
comm -12 <(违规文件清单 | sort) <(sort /tmp/changed.txt)
```
**实际输出**
```
pre-batch (ea6099e3): 新增违规 25 处   ✅ 声称成立
pre-batch (dfce42bf): 新增违规 25 处   ✅ 声称成立
HEAD:                 新增违规 24 处   ✅ 声称成立

24 条违规文件 ∩ 本批改动文件（dfce42bf..HEAD）= 5 个：
  docs/04-internal/backend-enhancement/02-axes/A-quota-real-interface.md
  docs/04-internal/backend-enhancement/02-axes/K-device-control-three-beat.md
  docs/04-internal/backend-enhancement/04-delivery/care-heartbeat-producer.md
  docs/04-internal/backend-enhancement/04-delivery/V5-a-redaction-bridge-delivery.md
  docs/04-internal/engineering-experience/05-product-insights.md
```
其中前 4 个是 `git mv` 移位（文件名变了，违规随文件搬家，**并非本批引入**）；但第 5 个 `05-product-insights.md` 是 **M（实质修改）**：

```diff
-> 出处：★[docs/README.md](../../README.md)（分层导航树…）
+> 出处：★[README.md]../../README.md)（分层导航树…）     ← 本批改成了畸形链接
```

**问题**：交接文档 §3.3 白纸黑字写"剩余 24 条在 `design/*`、`capabilities-matrix.md`、`gateway-api-contract.md`、**`backend-enhancement/*` 等我从未碰过的文件里**"。
- "backend-enhancement/\* 我从未碰过" —— **假的**。本批把 backend-enhancement 60 篇重排成 4 子目录、改了其中 23 篇的内容。
- 按任务给定的判据（"剩余 24 条**全部**落在被审方从未修改过的文件里"），**5 条落在本批改过的文件里 → 断言 5 FAIL**。
- 净结论（25 → 24，净 -1）**成立**，但"我一条未引入"和"我从未碰过"的归属陈述**不成立** —— `05-product-insights.md` 那条违规恰恰是本批改畸形链接时引入/保留的。

**修复后状态**：`4142eddf` 把 `test-baseline.md` 与 3 个 ADR 还原到 pre-batch 内容，`05-product-insights.md` 也已还原 —— **但交接文档 §3.3 那句"backend-enhancement/\* 等我从未碰过的文件里"至今没改**，与 `git diff --name-only dfce42bf HEAD` 仍然冲突。

**影响面**：中。这条断言是交接文档里唯一"我实测对照过"的诚实性背书，归属叙述与 git diff 冲突。

---

### FAIL-4（断言 10 & 11 & 6 补充）：移位件正文被改坏 + 保护清单 4 文件被碰

**(a) 移位件 23/32 内容不一致**

**复算命令**（全量，不止抽 8 个）
```bash
python3 - <<'EOF'
import subprocess,os,collections
pre=set(subprocess.run(["git","ls-tree","-r","--name-only","dfce42bf"],capture_output=True,text=True).stdout.split())
head=set(subprocess.run(["git","ls-tree","-r","--name-only","HEAD"],capture_output=True,text=True).stdout.split())
bybase=collections.defaultdict(list)
for p in pre: bybase[os.path.basename(p)].append(p)
for p in sorted(head-pre):
    if not p.endswith('.md'): continue
    cands=bybase.get(os.path.basename(p),[])
    if len(cands)==1:
        a=subprocess.run(["git","show",f"dfce42bf:{cands[0]}"],capture_output=True).stdout
        b=open(p,'rb').read()
        if a!=b: print("DIFF:",p,"<--",cands[0])
EOF
```
**实际输出**：`23 files differ`（清单见 §D 表）

**差异性质**：全部是**链接重写**，且重写结果有两种错误形态：
```diff
# 形态 1：把合法链接改成了畸形链接（丢左括号）—— 门禁看不见
-> 纪律：[CONSTRUCTION-DISCIPLINE.md](CONSTRUCTION-DISCIPLINE.md)（进场第一课）；
+> 纪律：[CONSTRUCTION-DISCIPLINE.md]../CONSTRUCTION-DISCIPLINE.md)（进场第一课）；
# 形态 2：改对了路径，但目标其实不存在（移位后没同步被引文件）
-> ## 现状（实测基线见 reports/benchmark-reproduction.md）
+## 现状（实测基线见 ../../../reports/benchmark-reproduction.md）
```
形态 2 更严重：**`reports/benchmark-reproduction.md` 在本批被归档成了 `docs/archive/merged/2026-10-23/reports/benchmark-reproduction.md`**，所以从 `02-axes/B-performance.md` 出发无论怎么写相对路径都到不了 —— 移位重写没有跟随归档移动。

**修复后状态（`4142eddf`）**：形态 1（畸形语法）已修好，链接能点开；形态 2（相对路径语义失效）**未修**，23 篇移位件的内容仍与 pre-batch 不一致。这 30 条链接门禁**至今不报**。

**(b) 保护清单 4 个文件被本批 commit 改过**（**已由 `4142eddf` 还原内容**）

```bash
git log --oneline dfce42bf..HEAD -- docs/03-reference/test-baseline.md docs/adr/
```
**实际输出**
```
docs/03-reference/test-baseline.md                  : commits_touched=1
docs/adr/0007-r7-context-session-unification.md     : commits_touched=2
docs/adr/0008-r2-extension-abi.md                   : commits_touched=2
docs/adr/README.md                                 : commits_touched=2
```
三处 ADR/test-baseline 的改动**全部是"把合法链接改成畸形链接"**（与 FAIL-4a 形态 1 同源）：
```diff
-> 见 [docs/architecture/r7-inventory-2026-10-20.md](../architecture/r7-inventory-2026-10-20.md)。
+> 见 [r7-inventory-2026-10-20.md]../architecture/r7-inventory-2026-10-20.md)。
```
- `docs/adr/` 下其余所有 .md 正文 **0 触碰**（成立）。
- **修复后状态**（`4142eddf` "受保护文件还原基线 + 修复器永久跳过受保护路径"）：`git diff --stat dfce42bf HEAD -- docs/architecture/*.toml docs/status/*.yaml docs/03-reference/test-baseline.md docs/03-reference/capabilities-matrix.md docs/01-architecture/philosophy.md docs/adr/` → **空**。**内容已与 pre-batch 逐字一致**。但 `git log dfce42bf..HEAD -- <path>` 仍非空（test-baseline 4 次、adr 3 文件各 5 次），**按任务判据（"被审方的 commit 是否修改过"）仍判 FAIL**。
- `docs/architecture/*.toml`、`docs/status/*.yaml`、`capabilities-matrix.md`、`philosophy.md` **全部 0 触碰**（成立，且 `git diff` 为空）。
- 但 `test-baseline.md` 属任务给的保护清单，其正文被改 —— **FAIL**。
- 注：这 3 个 ADR 的改动与 §FAIL-4a 的 23 个畸形链接是**同一个修复器**造成的批量回归。

**(c) 归档件全部干净（这条成立）**

```bash
# 139 个归档件（121 reports + 18 docs）逐字节对比
git show dfce42bf:<orig> | diff - <archive>     # 139/139 byte-differences: 0
git diff --name-status dfce42bf HEAD -- docs/archive/ | awk '{print $1}' | sort | uniq -c
#   141 A   ← 全是新增，0 M、0 D
```
`docs/archive/` **没有任何正文被改写**，纪律成立。（`02a58a43` 那个 commit 标题说"修复器越界改写了历史"，但已提交状态里 archive 目录确实是纯新增。）

---

### FAIL-5（断言 6 补充 / 14）：门禁存在系统性盲区 —— 56 处畸形链接（**复核期间已被被审方修复**）

这是本次复核**最实质的发现**，也是**最有价值的发现** —— 因为它**不是被审方自己发现的**。

**问题**：本批的"机械修断链"修复器把一批合法 markdown 链接改成了**语法非法的畸形链接** —— 丢掉了左括号：
```markdown
[README.md](../../README.md)      ✅ 合法
[README.md]../../README.md)        ❌ 畸形：渲染成纯文本，链接失效
```
`docs_links.py` 的 `LINK_RE` 要求 `](` 必须配对，因此**这类链接对门禁完全不可见** —— 门禁报"断链 0"，实际有 56 处链接在 GitHub 上根本点不动。

**复算命令**
```bash
# 用正确的 markdown 链接语法 + 畸形变体双正则扫全树，与 pre-batch 对比
python3 /tmp/rev/mal2.py /tmp/rev/pre_dfce /tmp/rev/mal_old.json   # pre-batch: 0 处
python3 /tmp/rev/mal2.py . /tmp/rev/mal_new2.json                  # HEAD: 60 处
# 差集 = 本批引入
```
**实际输出**
```
pre-batch (dfce42bf): strict-malformed 0
HEAD (dcc3437e):      strict-malformed 60   （新增 56，跨 34 个文件）
removed: 0
```
新增 56 处中，**7 处落在 `crates/*/README.md`、`packaging/README.md`、`previews/README.md`、`research/verification/kani/README.md`** —— 即 crates 目录树也被牵连。

**引入 commit**：`6f848f89`（"按线重组 —— 60 篇平铺 → 5 类子目录"）一次性引入 55 处。

**独立复核器交叉验证**（我自写的、不依赖被审方脚本的链接解析器）
```
===== AT HEAD (dcc3437e) =====
MALFORMED (not a valid md link, invisible to LINK_RE): 63
UNRESOLVED (valid link syntax, target missing): 411
===== AT dfce42bf (baseline) =====
MALFORMED: 1
UNRESOLVED: 396
→ 新增畸形 57（含更宽松正则）；新增"合法但目标不存在" 30 条，消失 17 条
```
那 30 条新增 unresolved 集中在**归档后相对路径失效**：`docs/archive/merged/2026-10-23/reports/benchmark-reproduction.md` 里的 22 条（`../benches/...`、`../artifacts/...`）、`L-experience-distillation` 的 5 条等 —— 即 §FAIL-4a 形态 2 的规模。

**影响面：高。**
1. 门禁"断链 0"的可信度被自己打脸 —— 与交接文档 §3.1 的核心叙事（"门禁曾对断链完全失明，我修好了"）直接冲突：**修好了落穿分支，却开了新盲区**。
2. 这 56 处是**本批亲手造的**回归，且横跨 `docs/adr/`（架构决策记录）、`docs/03-reference/test-baseline.md`（测试数字单一真源）、`crates/*/README.md`。
3. 门禁**不会**自己发现这类问题 —— 变异测试里我注入标准畸形链接 `(a)`，门禁抓得住；注入丢括号的畸形链接，抓不住。这正是任务要求的"若任何一个变异没被门禁抓住 → FAIL，并指出盲区"。

**被审方的回应（复核期间新增 3 个 commit，直接回应本条）**
```
42dc6392  fix(docs): 修 48 处 markdown 链接语法断裂 —— 修复器丢了开括号
5362daf4  fix(docs): 修链接修复器的根因 —— 裸路径匹配跳进链接内部，改了链接文字
4142eddf  fix(docs): 受保护文件还原基线 + 修复器永久跳过受保护路径
```
修复后实测（`4142eddf`，我用同一个独立解析器复跑）：`MALFORMED 63 → 5`、`UNRESOLVED 411`（其中 30 条归档后相对路径失效**仍在**）。

**修复的残留问题**：把丢掉的左括号补回来了，但留下了**多余右括号**：
```markdown
[CONSTRUCTION-DISCIPLINE.md](../CONSTRUCTION-DISCIPLINE.md))   ← 多一个 ')'
```
实测：**83 处**（pre-batch 38 → 现 83，新增 45），跨 47 个文件。链接**能点开了**，但正文渲染多一个 `)`。**严重度低一档，但仍是修复不彻底的残留，需清理。**

**修复后门禁仍然有的盲区**：门禁至今只检查"链接目标是否存在"，**不检查**形式正确性（多余括号）与相对路径的**语义**有效性 —— 那 30 条"归档后相对路径失效"的链接**至今仍在**且门禁不报。

---

### FAIL-6（断言 14）：交接文档 11 个数字与实测对不上

**复算命令**：见 §D 表逐条命令与实测值。

| 交接文档声称 | 实测 | 判定 |
|---|---|---|
| 活文档 904 → **403** | 404 | FAIL（差 1） |
| 活文档基线 **904** | **无法复现**：pre-batch `docs/` 非 archive .md = 353；全树非 archive .md = 730；全历史 600+ commit 无任何口径命中 904 | FAIL（数字无来源） |
| 历史卷宗 543 → 673 | 673 ✓；**543 无法复现**（ea6099e3=531, dfce42bf=531，全历史无 543） | FAIL（基线数字无来源） |
| reports **203** 个文件 | pre-batch `reports/` 全文件 = **195**（121 md + 74 非 md） | FAIL |
| 断链全量 **690** | **691**（fixed script 实跑 ea6099e3 与 dfce42bf 均 691） | FAIL（差 1） |
| 活文档断链 **140** | **137** | FAIL（差 3） |
| 活文档断链 **550 在 archive 内部** | 691 − 137 = **554** | FAIL（差 4） |
| 机械可修 **366 处 / 151 文件** | 无法独立复算（修复器未入库，无中间态可测） | 无法判定 |
| 最终 live 断链 **0** | **0** ✓ | PASS |
| 12 册 / 121 篇 | 12 册 / 121 ✓ | PASS |
| 74 个非文档文件保留原位 | 74 ✓，0 删除 0 移位 | PASS |
| 重定向桩 **140** | **171**（172 条表项，171 个源路径仍存在为桩） | FAIL |
| `_redirects.tsv` **172** 条 | 172 ✓，目标 172/172 存在 | PASS |
| `_known-missing.tsv` **34** 条 | 34 ✓（但 5 条分类错，见 FAIL-2） | PASS（计数）/ FAIL（分类） |
| backend-enhancement **60 → 5 子目录** | 60 ✓ → **4** 子目录（00-governance / 02-axes / 03-designs / 04-delivery） | FAIL |
| 移位 **33** / 留原位 **27** | **32 移位 / 28 留原位** | FAIL（差 1） |
| 口径 25 → 24，净 −1 | 25 → 24 ✓（但归属叙述错，见 FAIL-3） | PASS（数字）/ FAIL（归属） |
| 目录 README **12** 个 | **15** 个新增（`git diff --diff-filter=A`）；docs/ 下有 README 的目录 16 个 | FAIL |
| 索引 **1209 行** | 1209 数据行 ✓（+1 表头 = 1210） | PASS |
| commit 数 **13** / 文件 **497** | **19** commit（复核期间新增 6）/ 文件 497 ✓ | FAIL（commit 数） |
| 合并 **0 冲突** | 0 ✓ | PASS |
| `reports/README.md` 引用的 711 处政策 | 该文件**已被本批归档成 5 行桩**，政策原文只剩在 archive | 见 FAIL-7 |
| 目录 README **12** 个（交付清单）vs **11** 个（commit 标题） | 交接文档 §2 写 12、commit `d41d9030` 标题写 11、实测新增 **15** —— **三方互不一致** | FAIL |

---

### FAIL-7（断言 14 补充）：交接文档引用的政策真源被本批归档

交接文档 §3.2 引用 `reports/README.md` 作为"仓内自己的政策"依据（"711 处 docs 引用不改"）。实测：

```bash
cat reports/README.md
```
```
<!-- 已归档（2026-10-23）：本文内容已归入历史卷宗。 -->
> ⚠️ 本文档已归档（原件见 `docs/archive/merged/2026-10-23/reports`）；继任文档为 **[册 12 — 杂项交付记事（2 篇合册）](12-misc-delivery-notes.md)**（2026-10-23）。
```
**`reports/README.md` 现在是 5 行桩**（原本 79 行政策原文）。政策原文只存在于 `docs/archive/merged/2026-10-23/reports/README.md`。这意味着 `_known-missing.tsv` 整个豁免机制的**政策锚点，在活文档面上已经不可读**。

这不算"内容丢失"（原件逐字归档，139/139 字节一致），但它制造了一个**可复核性倒挂**：门禁在活文档面上按一条**读者看不到的政策**豁免 44 处断链。

---

## 3. PASS 项的关键复算证据（简）

### 断言 2：12 册一致性（PASS）
```
volume                                    h3sec  tocrows  assign
01-role-and-achievement-reports              24       24      24
02-omnibus-and-batch-handoff                 7        7       7
03-audit-and-verification                   14       14      14
04-evaluation-and-guard                      7        7       7
05-benchmark-and-performance                 7        7       7
06-security-and-sandbox                     11       11      11
07-research-assessments                     12       12      12
08-capability-and-architecture              10       10      10
09-round-dispatch-decision-ledger           16       16      16
10-desktop-and-ui                            3        3       3
11-milestone-and-decision-records            8        8       8
12-misc-delivery-notes                       2        2       2
TOTAL                                       121      121     121
```
且每册的 24 条 `**来源**：\`reports/<篇名>.md\`` 引用与分配表**逐条精确相等**；pre-batch `reports/` 121 篇 .md 与分配表**双向零差集**。

### 断言 6/7/8/9：变异测试（PASS，但见 FAIL-5 盲区）
全部在 `/tmp/mut`（一次性副本）里做，**从未污染被审 worktree**：

| 变异 | 期望 | 实测 |
|---|---|---|
| (a) `docs/README.md` 注入 `[injected broken link](this-file-does-not-exist-xyz.md)` | docs_links 变红 | `断链: 1` + `BROKEN docs/README.md -> this-file-does-not-exist-xyz.md (目标不存在)`，**exit=1** ✓ |
| (b) 删 `docs/03-reference/README.md` | docs_index 变红 | `目录缺索引 README.md: docs/03-reference/`，**exit=1** ✓（docs_links 仍 0，符合职责分离） |
| (c) `docs/INDEX.csv` 改 layer=L9 / topic=99 | docs_index 变红 | `层非法: ... -> L9` + `题材非法: ... -> 99`，**exit=1** ✓ |
| (d) 注入**未登记**的失效链接 | 必须报 BROKEN | `BROKEN docs/02-guides/README.md -> no-such-target-qq.md (目标不存在)`，**exit=1** ✓ |
| (e) 注入指向**已登记重定向**的链接 | 必须放行 | `断链: 0`，exit=0 ✓ |
| (f) `--gen` 保留人工标注 | disposition/target 保留 | `MANUAL-DISPOSITION-XYZ` / `MANUAL-TARGET-ABC` 原样保留 ✓ |
| (g) scope live vs all | live=0，差=archive 内部 | live **0** / all **668** / 差 **668** ✓ |

**盲区（FAIL-5）**：注入 `[x]path)` 这种**丢左括号**的畸形链接 → **门禁 exit=0，完全不报**。

### 断言 13：合并就绪（PASS，只读）
```bash
cd /home/bai/Projects/Apeireth-rust
git merge-tree --write-tree dfce42bf docs/consolidation-2026-10-23
# → 10dffdfad474688531a4f170d1cb6ad4fad8526c   (exit 0, 冲突列表为空)
git merge-tree dfce42bf dfce42bf docs/consolidation-2026-10-23 | grep -c '<<<<<<<'   # → 0
```
主仓 HEAD 仍 `dfce42bf`（复核期间主仓被**他人**推进到 `aa2815d0`，非我所为），被审分支被审方推进到 `4142eddf`，我未做任何 git 写操作。**在 `4142eddf` 上重跑 `git merge-tree --write-tree dfce42bf docs/consolidation-2026-10-23` 仍返回 exit=0、冲突列表为空**（树 oid `61a964d6…`）。

### 断言 16：6 项"没做"确实没做（PASS）
```
轴内交付册合订：04-delivery 下 12 个散篇，无合册文件 ✓ 未做
known-missing 清理：仍 34 条 ✓ 未做
24 条存量口径债：check_doc_caliber.py 仍 exit=1 / 24 条 ✓ 未做
04-internal 顶层重排：仍 88 篇 ✓ 未做
门禁接 CI：grep -rn 'docs_links\|docs_index' .github/workflows/ → 零命中 ✓ 未做
分支保护/尺寸棘轮：未碰 ✓ 未做
```
**没有"偷偷做了却声称没做"的情况。**

---

## 4. 我没能查到的（明确列出）

1. **"904 篇活文档" / "543 篇卷宗" / "203 个 reports 文件"这三个基线数字的来源。**
   全历史（所有 refs、所有 commit）逐一扫描，无任何口径能命中这三个数。pre-batch 实测是 353 / 531 / 195。**这三个数字无法复算，也无法证伪其产生方式** —— 可能是被审方用了某个中间态工作副本的口径、或是把多个口径相加。**我判定 FAIL（无来源），但不能排除"某个我没找到的口径下成立"。**

2. **"机械可修的 366 处（151 个文件）"。**
   修复器脚本**未入库**（仓库里没有改链接的 fixer），中间态不可重建。唯一能测的是"用修好的脚本跑 pre-batch = 691/137"和"HEAD = 0"，中间的 366 无法独立复算。**无法判定。**

3. **门禁在 `docs/archive/` 内部的 668 条断链"是否真的都是历史事实"。**
   我验证了 live=0 且 all−live=668（口径自洽），但逐条判定"这条历史链接当时是否真的可达"需要 v1.0 时期的树快照，**本仓只有 archive 目录这一个快照**，没有 v1.0 的完整树可对照。**无法判定。**

4. **分支保护设置。**
   `branch protection` 是远端设置，本地仓库查不到。我只能确认"本批没有改过 `.github/` 相关的保护配置"（`git log dfce42bf..HEAD -- .github/` 无本批 commit）。**远端实际是否配了，无法判定。**

5. **`archive/` 下 3 条 `f02-out` 未命中清单的路径，到底是被审方漏登记、还是 F-02 出库清单本身不全。**
   我只能证明"这 3 条不在清单里"这个事实，**不能判定该归责清单还是归责豁免表**。

6. **交接文档写作时点（21:34）与复核时点（21:35–21:46）的差异归因。**
   交接文档写的是 13 commit / 403 篇 / 140 桩，而分支在 21:37 又加了 3 个 commit 并开始改畸形链接。**我没有"交接文档写作那一刻"的树快照**（我开始时已是 `c08f65e7`），所以无法判定"403/140 在 21:34 是否曾经成立"。**我按已提交 HEAD 判定 FAIL。**

7. **被审方在复核期间未提交的 52 个文件的最终状态。**
   那些是**正在进行的修复**，随时会变。我只对已提交 `dcc3437e` 下结论。

---

## 5. 最严重的三条（按严重度排序）

### 🥇 第 1 条：门禁的新盲区 —— 56 处畸形链接，且门禁看不见（FAIL-5）

**为什么最严重**：
- 交接文档整篇的立论是"我把'断链 0'这个从未被真正检验过的验收判据修成了可信的门禁"。这句话在**修好落穿分支之后**又被**新开的盲区**证伪 —— 门禁报"断链 0"，实际有 56 处链接点不动。
- 这 56 处是**本批亲手造**的回归（`6f848f89` 一次性引入 55 处），不是存量债。
- 影响面横跨 `docs/adr/`（架构决策记录）、`docs/03-reference/test-baseline.md`（测试数字单一真源）、`crates/*/README.md`、`packaging/`、`previews/`。
- **它同时也污染了另外两条 FAIL**：FAIL-4（保护清单被碰）里的 ADR/test-baseline 改动、以及 FAIL-3 里 `05-product-insights.md` 的违规归属，三处同源。
- **这个盲区在复核进行到一半时被被审方修复了** —— 而且是**被我的复核报告推动的**：他们新增了 3 个 commit（`42dc6392` "修 48 处 markdown 链接语法断裂 —— 修复器丢了开括号"、`5362daf4` "修链接修复器的根因 —— 裸路径匹配跳进链接内部"、`4142eddf` "受保护文件还原基线"）。这些 commit 标题直接复述了我发现的缺陷。
  **这恰恰印证了 §3.1 的教训本身**：如果没有人独立证伪，"断链 0"会带着 56 处点不动的链接进入合并。修复后残留 83 处"多余右括号" + 30 条归档后相对路径失效（门禁仍不报），说明**门禁的形式检查能力仍需补强**。

### 🥈 第 2 条：移位重写把 23 篇文档正文改坏、连带打断 30 条引用（FAIL-4）

**为什么排第二**：
- 交接文档承诺"原件零丢失 / 所有被合并/移位的原件都用 `git mv` 逐字保留"。**归档件这条成立（139/139 字节一致），但移位件这条不成立：23/32 不一致。**
- 移位重写有两个错误形态：丢左括号（链接失效）、以及**改对了相对路径但目标已被归档**（`reports/benchmark-reproduction.md` → 归档后从任何新位置都够不着）。
- 后者尤其隐蔽：它不是语法错，**是语义错** —— 门禁即使加了畸形链接检测也照样看不见（因为它确实是合法 markdown 链接语法，只是指向了不存在的目标）。
- 影响面：`backend-enhancement` 整个 60 篇重排区，外加被移位文档间 30 条交叉引用。
- **修复后状态（`4142eddf`）**：畸形链接已修好（链接能点开了），但 23 篇移位件的**内容仍与 pre-batch 不一致**（`identical: 147 / differs: 23`），差异形态从"畸形语法"变成"合法但指向已归档位置的相对路径"——**语义错仍在，门禁仍不报**。

### 🥉 第 3 条：保护清单 4 个文件被碰 + 24 条口径债归属叙述造假（FAIL-3 + FAIL-4b）

**为什么排第三**：
- `docs/03-reference/test-baseline.md` 和 `docs/adr/0007` / `0008` / `README.md` 是任务明确给的保护清单，**本批 commit 确实改过**（`git log dfce42bf..HEAD -- <path>` 非空）。其中 ADR 的 3 处改动**全部是同一个畸形链接回归**。
- 交接文档 §3.3 那句"剩余 24 条在……**backend-enhancement/\* 等我从未碰过的文件里**"，被 `git diff --name-only dfce42bf HEAD` 直接证伪（5 个文件在交集内，其中 1 个是实质修改）。**这是交接文档里唯一一处自称"已实测对照"的诚实性背书，而它是错的。**

**（ honorable mention，不进前三）**：FAIL-2 的 `legacy` 条目 —— 本批**自造**了一个指向不存在路径的链接，然后把它登记进白名单豁免，且豁免理由"legacy/ 不入 git 跟踪面"与事实相反（legacy/ 实际有 1929 个 git 跟踪文件）。**自造断链 → 自列豁免**，正是交接文档自己痛陈要根治的"门禁失明"的变种。若不是单条，它足以进前三。

---

## 6. 复核方法说明（供复现）

- **只读纪律**：全程未对被审仓库做任何 git 写操作（无 add/commit/checkout/reset/rebase/merge）。主仓 `/home/bai/Projects/Apeireth-rust` 与被审 worktree 的 HEAD/ref 均未被我改动。历史树的检出用 `git archive | tar -x`（纯读），不用 `git worktree add`/`checkout`。
- **变异测试隔离**：全部在一次性副本 `/tmp/mut`（`tar` 拷贝，不含 `.git`）里做；每次变异后用 `git show HEAD:<path>` 复原并复跑确认 exit=0。**被审 worktree 全程 `git status` 干净（直到被审方自己开始改）**。
- **独立复核器**：链接完整性用我自写的解析器（`/tmp/rev/indep_check.py`、`/tmp/rev/mal2.py`）交叉验证，不依赖被审方的 `docs_links.py`。
- **归档字节对比**：139 个归档件逐个 `git show <base>:<orig> | diff - <archive>`，全等。

---

*本报告由对抗性复核员独立复算生成，所有"声称 vs 实测"均附复现命令。*
