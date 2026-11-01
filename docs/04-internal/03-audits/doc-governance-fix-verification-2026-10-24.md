<!-- 归档说明：本文件由会话内复核代理产出，归档时正文逐字保留；仅对中性表述做脱敏（第三方模型/厂商名 → 描述性表述），依据 AGENTS.md §2。 -->
> **性质**：文档治理批的**对抗性复核/验证报告**（举证记录，非规格、非现状真源）。
> **被验对象：修复 commit `eadcc841`**
> **结论**：10 条断言 / **PASS 10 / FAIL 0**；另指出 2 处表述精度问题（已由 `e9b0861a` 修正）
> **读法**：想知道「这批改动的可信度凭什么」，读本报告；想知道某项能力现在到哪一级，查 `docs/status/claims.yaml`。

# 修复验证报告 —— eadcc841（10 条断言）

- **被验 commit**：`eadcc841` fix(docs): 对抗性复核 4 项 FAIL 全修
- **父 commit**：`315bcff1` refactor(docs): 04-internal 顶层 88 篇按题材重排
- **分支 / 工作树**：`docs/governance-followup-2026-10-24` @ `/root/Documents/deepseek-harness/default-workspace/apeireth-docs-wt`
- **验证时间**：2026-10-24 10:26–10:42 (+08:00)
- **铁律遵守**：全程只读，零 git 写操作；变异测试仅在 `rsync` 副本 `/tmp/tmp.AD32CcJfz6`（另 `/tmp/tmp.hwFnq039Cs`）中进行，源工作树未被修改。

---

## ① 统计

| 结果 | 数量 | 编号 |
|---|---|---|
| **PASS** | **10** | V1–V10 |
| **FAIL** | **0** | —— |

**FAIL 编号清单：无。**

全局印象（`git show --stat eadcc841`）：81 files changed, 537 insertions(+), 430 deletions(-)；`crates/` 零改动；`scripts/docs_index.py` 18 行改动；8 个新增 README；67 个 7 行变动的桩文件（与 commit message 的「67 桩重生成」一致）；`05-plans/backlog.md` 删除（5 行）。

---

## ② 逐条结论与实测

### A2 —— 口径划错

#### V1：设备审批卡→IM 推送链改标 IMPLEMENTED，与真源一致 —— **PASS**

命令与实测：

```
$ grep -n "四级口径" ../backend-enhancement/K-device-approval-im-push.md
167:- **库级已实现、生产装配未接线〔四级口径：IMPLEMENTED〕**：设备审批卡→IM 推送链（同链同信封 + plan_id 幂等 +
172:  > 口径校注（2026-10-24 文档治理批）：本条原标 `PRODUCTION WIRED` 属**划错边界**（把"库级有实现+有测试"当成"已接生产装配"），
173:  > 已按真源改为 `IMPLEMENTED`。这正是口径纪律要防的过度声称。
174:- **已接线〔四级口径：PRODUCTION WIRED〕**：`self_status` `device_control` 节 + `device_control_status_probe` 装配接线
```

真源同口径：

```
$ grep -n "IMPLEMENTED 但未接线" ../../03-reference/capabilities-matrix.md   # 命中 :70
:70 ... 计划卡 → IM 推送链与两本账 = **IMPLEMENTED 未接线**（生产无注入点/无调用点，见上）
```

一句话：IM 推送链已从 `PRODUCTION WIRED` 降为 `IMPLEMENTED`，且与 `capabilities-matrix.md:70` 的「IMPLEMENTED 未接线」逐字同口径；K 文件 :167 还指向矩阵 §0.2。**PASS**

#### V2：self_status 探针仍为 PRODUCTION WIRED，production.rs 确有调用 —— **PASS**

```
$ grep -rn "device_control_status_probe" crates/
crates/engine/runtime-assembly/src/canonical/production.rs:1349:  super::self_status_source::device_control_status_probe(Arc::clone(runner)),
crates/engine/runtime-assembly/src/canonical/self_status_source.rs:373: pub fn device_control_status_probe(
crates/engine/runtime-assembly/src/lib.rs:100: pub use canonical::self_status_source::device_control_status_probe;
（其余为测试与再导出）
$ sed -n '1348,1351p' .../canonical/production.rs
                source = source.with_device_control_probe(
                    super::self_status_source::device_control_status_probe(Arc::clone(runner)),
                );
```

K 文件 :174 独立标注 `PRODUCTION WIRED`。生产装配根确有调用，非「仅测试」。**PASS**

#### V3：三符号确无生产装配调用点（修复方向正确） —— **PASS**

```
$ grep -rn "with_device_approval_face\|push_device_approval_cards\|retry_device_approval_pushes" crates/ --include=*.rs | grep -v "/tests/"
crates/adapters/gateway/src/im_bridge.rs:505:    pub fn with_device_approval_face(...)   ← 定义
crates/adapters/gateway/src/im_bridge.rs:912:    ///   ([`Self::retry_device_approval_pushes`]) ← doc 注释
crates/adapters/gateway/src/im_bridge.rs:916:    pub async fn push_device_approval_cards  ← 定义
crates/adapters/gateway/src/im_bridge.rs:950:    pub async fn retry_device_approval_pushes ← 定义
crates/adapters/gateway/src/im_device.rs:21:    //! ... retry_device_approval_pushes ← doc 注释

$ grep -rn "device_approval_face\|device_approval_pushes" \
    crates/engine/runtime-assembly/src/canonical/production.rs crates/adapters/cli/src/lib.rs
（无输出，exit=1）
```

全部生产命中仅为「定义」与「文档链接」，无注入/调用点；`production.rs` 与 `cli/src/lib.rs` 零命中。**V1 的降级方向正确。PASS**

### C2 —— 门禁二级目录盲区

#### V4：README 规则已递归 —— **PASS**

`scripts/docs_index.py`：

```
269:    # 1. 每目录有 README（**递归**）
275:    for d in sorted(p for p in (ROOT / DOCS).rglob("*") if p.is_dir()):
276:        rel = d.relative_to(ROOT)
277:        if any(part in SKIP_DIRS for part in rel.parts): continue
279:        if "archive" in rel.parts: continue          # 历史卷宗不要求索引
281:        if not any(d.glob("*.md")): continue          # 只要求「含 .md 的目录」
283:        if not (d / "README.md").exists():
284:            problems.append(f"目录缺索引 README.md: {rel.as_posix()}/")
```

由 `iterdir()` 一层改为 `rglob("*")` 递归。**PASS**

#### V5：变异测试（副本内删 `docs/04-internal/04-logs/README.md`）—— **PASS**

副本 `/tmp/tmp.AD32CcJfz6`（`rsync -a --exclude=.git --exclude=target`）：

```
[1] 对照（未删）:  [docs_index] 问题数: 0    exit=0
[2] 删除后:        [docs_index] 问题数: 1
                   目录缺索引 README.md: docs/04-internal/04-logs/
                   exit=1
[3] 还原后:        [docs_index] 问题数: 0    exit=0
[4] 再删可复现:    exit=1（同上）
```

**PASS**（对照 exit=0、变异 exit=1 且精确报出该目录）。

### B7/B2 —— README 数字与理由

#### V6：88 = 1 + 17 + 70 自算成立 —— **PASS**

```
$ ls -1 docs/04-internal/*.md | wc -l
88
机器分类:
  total .md = 88
  新式重定向桩（含「重定向桩」+「本文件保留…」）= 67
  遗留合并/归档桩（HANDOFF-NOTES / TO-NEW-TEAM / next-team-handbook）= 3
  真文件（含 README）= 18
  check: 1 索引 + 17 真文件 + 70 桩 = 88  ✓
```

17 篇真文件与 README 表中 17 行逐名一致（`ai-os-landscape` … `v2.0.0-rc-roadmap`，含 `ENGINEER-MANIFESTO`/`github-fetch-guide`/`o6-session-log`）。**PASS**

#### V7：留位理由抽查 6 篇，引用真实；假说法已删 —— **PASS**

| 篇 | README 理由 | 实测引用 |
|---|---|---|
| `engineering-review-handoff-2026-10-06.md` | 口径门禁 workflow 与 `check_doc_caliber.py` 的**注释/出处**引用（不是脚本读取该文件） | `scripts/check_doc_caliber.py:2`（docstring）、`.github/workflows/doc-caliber.yml:3`（注释）、`crates/adapters/cli/src/lib.rs:190`、`crates/adapters/cli/tests/production_knobs.rs:141`（均为注释） |
| `live-verification-ledger.md` | 真源级：口径基线 + 多处引用 | `scripts/doc-caliber-baseline.txt:7`、`docs/status/verification-ledger.yaml:4`、`scripts/target-sweep.ps1:3` |
| `ENGINEER-MANIFESTO.md` | 地标：`AGENTS.md` 指向（文档引用，非机器引用） | `scripts/ .github/ crates/ docs/status/` **0 命中**；`AGENTS.md:4,45` 命中 2 处 |
| `github-fetch-guide.md` | 地标：`AGENTS.md` §4.5（文档引用） | 上述四目录 **0 命中**；`AGENTS.md:52` 命中 |
| `kani-verification-playbook.md` | `.github/workflows/kani.yml` 引用 | `kani.yml:104`、`docs/status/formal-coverage.md:40`、`docs/status/verification-ledger.yaml:56` |
| `scene-d-v2-plan.md` | `crates/foundation/core/src/philosophy.rs` 等源码注释引用 | `philosophy.rs:149`、`plugin/src/preference.rs:6`、`plugin/src/self_assessment.rs:6`、`core/src/onion.rs:82`、`orchestration/Cargo.toml:8`、`memory/src/backend/file.rs:9` |

假说法核查：

```
$ grep -n "口径门禁脚本\|直接读" ../README.md
46:| engineering-review-handoff... | ...**注释/出处**引用（不是脚本读取该文件） |
84:  `engineering-review-handoff` 的留位理由由「脚本直接读它」改为准确的「注释/出处引用」；
85:  「口径门禁脚本直接读它」等 3 处不成立的说法已删。
```

- :46 已是准确表述；脚本里对该文件**只有** :2 docstring 一处提及，无任何指向它的 `open()`/`read_text()`。
- 进一步实证：该文件名含日期，`check_doc_caliber.is_historical()` 返回 `"文件名含日期/轮次/backup"`，即门禁**根本不扫描**该文件 —— 「脚本读取该文件」确系假说法。
- :84–85 是**变更历史里对已删假说法的引号复述**，不是现行主张（naive grep 会误命中，报告在此澄明）。

**PASS**

### 另两处

#### V8：桩模板重复字已清 + 70 桩全部可达 —— **PASS**

```
stub total = 70（67 新式 + 3 遗留）
duplicate '已已' occurrences: NONE
stub '本文内容' 模板残留: NONE
unreachable targets: 0
reachable = 70 / 70
```

新桩首行为 `<!-- 重定向桩（2026-10-23）：本文件已移位，请勿在此写正文。 -->`，第二行 `⚠️ 本文档已移位至 **[标题](目标)**`，第三行 `本文件保留仅为维持既有链接可达`；父 commit 的 `本文内容已已移至。` 已消失。**PASS**

#### V9：2 跳链已直达，子目录冗余桩已删，重定向表一致 —— **PASS**

```
$ cat ../backlog.md          → 链接直指 backend-enhancement/00-backlog.md
$ ls ../../archive/merged/2026-10-23/docs/04-internal/backlog.md  → No such file or directory（已删）
$ grep -n "04-internal/backlog.md" docs/_redirects.tsv
61:../backlog.md	../backend-enhancement/00-backlog.md
$ ls -l ../backend-enhancement/00-backlog.md → 45592 bytes，存在
$ grep -rn "05-plans/backlog" .（md/py/tsv/rs/yml/ps1/toml）→ 0 命中
$ grep -n "05-plans/backlog" docs/INDEX.csv → 0 命中（无陈旧行）
```

三方（桩正文 / 重定向表 / 目标存在）一致，且无残留引用。**PASS**

### 不回归

#### V10：6 门禁全绿 + 工作树干净 + crates 零改动 —— **PASS**

| 门禁 | 实测 | exit |
|---|---|---|
| `docs_links.py --check` | 扫描 1426 个 md；断链 **0**；已知缺失 44（政策保留） | 0 |
| `docs_index.py --check` | 问题数 **0** | 0 |
| `docs_truth.py --verify --check` | OK: STATUS.md matches claims.yaml; README badges match baseline | 0 |
| `arch_doc.py --check` | OK: ARCHITECTURE.generated.md 与契约+metadata 一致 | 0 |
| `arch_guard.py --strict` | crates=30 edges=111 VIOLATIONS: 0（STALE 5 为可缩表提示） | 0 |
| `check_doc_caliber.py` | 扫描 402 活跃文档（跳过 1144 历史），新增违规 **0**（基线内 6） | 0 |

```
$ git status --short        →（空，工作树干净）
$ git diff 315bcff1 eadcc841 --name-only -- crates/ →（空）
```

**PASS**

---

## ③ 修复本身引入/残留的新问题

> 不影响 10 条判定，但属于「数字须可复算」的同一类瑕疵，建议下批顺手处理。

1. **（轻微·数字）README 说桩「3 行」，物理行数是 4 行。**
   `docs/04-internal/README.md:35`「每个桩 3 行」、`:67`「原路径留 3 行重定向桩」。
   实测 `wc -l`：67 个新桩 **全部 = 4 行**（3 行正文 + 末尾 1 空行，文件以 `\n\n` 结尾），3 个遗留桩 = 5 行。
   `docs_index.py` 的 `STUB_MAX_LINES = 10` 也是按物理行算。若按「正文非空行」计则 3 行成立，但字面与机器可复算值差 1。
   建议写成「3 行正文（文件尾 1 空行）」或直接给 `wc -l` 值。
   （注：父 commit 写「4 行」，与当时的 4 行正文一致；本 commit 改成 3 行时未同步物理行数。）

2. **（语义）「70 个重定向桩」含 3 个非重定向桩。**
   `HANDOFF-NOTES.md` / `TO-NEW-TEAM.md` 是「已合并」、`next-team-handbook.md` 是「已归档」，均非「移位重定向」。
   README 的 bullet 已如实说明这 3 个是更早批次遗留桩，但标题句「88 = 1 + 17 + 70 个重定向桩」略宽。

3. **（提示·非缺陷）README:85 以引号复述已删假说法**「口径门禁脚本直接读它」。
   它是「变更历史」中的删除记录，不是现行主张；但会命中任何对「直接读」的字符串扫描，复核脚本若做 naive grep 需排除变更历史段。

未发现功能性回归：新增 8 个 README 全部存在且非空（14–28 行）；`docs/` 下不存在「含 .md 却无 README」的目录（独立 `find` 复扫 0 命中）；删除 `05-plans/backlog.md` 后无断链、无陈旧 INDEX 行、无残留路径引用。

---

## ④ 变异测试原始输出（逐字）

```
### COPY=/tmp/tmp.AD32CcJfz6
### [1] control: no deletion
[docs_index] 问题数: 0
exit=0
### [2] mutation: rm ../04-logs/README.md
[docs_index] 问题数: 1
  目录缺索引 README.md: docs/04-internal/04-logs/
exit=1
### [3] restore: put README back
[docs_index] 问题数: 0
exit=0
### [4] delete again: reproducible
[docs_index] 问题数: 1
  目录缺索引 README.md: docs/04-internal/04-logs/
exit=1
```

（首次独立副本 `/tmp/tmp.hwFnq039Cs` 结果相同：baseline `问题数: 0 / exit=0`，删后 `问题数: 1 / exit=1`。）

---

## 结论

**eadcc841 的 6 处修复全部真实生效，10/10 断言 PASS，0 FAIL。** 最关键的 A2 口径划错已按真源降级为 `IMPLEMENTED`，且 `self_status` 探针的 `PRODUCTION WIRED` 有 `production.rs:1349` 生产调用实证、三符号生产零调用点，方向正确；C2 门禁盲区已递归化并经变异测试证实可拦；B7 的 88=1+17+70、B2 的留位理由与「脚本直接读」假说法均与实测一致；两处附带缺陷（桩重复字、2 跳链）亦已闭合。仅残留 3 条非阻塞的表述精度问题（桩行数 3 vs 4、「70 个重定向桩」含 3 个合并/归档桩、变更历史引号复述旧假说法）。
