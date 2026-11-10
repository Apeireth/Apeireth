# Cargo audit workflow 分诊（2026-11-03 实测基线）

[Document-Meta]
Document:        cargo-audit-workflow-triage-2026-11-03.md
Version:         v1 (2026-11-03 实测分诊)
Last-Modified:   2026-11-03
Status:          🟢 活跃 (cargo-audit workflow 长期红因的分诊结论)
Class:           AUDIT

> **给谁看**：接这条线的人或 AI。**本文自足** —— 不需要读对话记录。
> **本文是什么**：GitHub Actions 上 `cargo-audit` 这条 workflow **长期红**的**实测分诊**：
> 到底红在哪一步、是代码问题还是 CI 噪声、以及**可执行的最小修法**。
> **口径纪律**：每条判据都来自**当场实测**（GitHub API 取 run/job、取 run 页错误原文、
> 严格重复键扫描、`bash -n`），**不引用 PR #108 的历史判断**。
> **复核日**：2026-11-03。**run 列表会变** ⇒ 下面的「当前读数」是**快照**，
> 复核时请重跑本文末尾的**复核命令**。
> **本文只诊断，不改代码** ⇒ 仓库里**只有本文件**是本次新增。

---

## 0. 一句话结论

**这是 (C) 配置错误 —— 不是 CI 基础设施噪声，也不是真实漏洞报告。**

`.github/workflows/cargo-audit.yml` 的 **`Convert JSON report to SARIF (R66)`** 这一步里，
**同一个 step 写了两次 `if: always()`**（第 **148** 行与第 **162** 行）
⇒ **YAML 同层重复键** ⇒ **GitHub 的 workflow 解析器拒绝这个文件**
⇒ 整条 workflow **一个 job 都没起来** ⇒ run **秒失败**（`jobs = 0`）。

**GitHub 自己给出的原文**（run #1035 页面）：

```text
Invalid workflow file: .github/workflows/cargo-audit.yml#L1 (Line: 162, Col: 9): 'if' is already defined
```

⇒ ⇒ **失败发生在「文件解析」这一步**，早于 checkout / action 下载 / 权限 / 漏洞扫描。
⇒ ⇒ **既不是 action 下载失败，也不是权限问题，更没有产生任何漏洞报告**
⇒ ⇒ **此前「jobs=0 空跑 ⇒ 既有 CI 基础设施噪声」的历史判断不成立**：
「空跑」这个**症状**是真的，但它的**病因在仓库内的 YAML 里**，不是 GitHub 侧的噪声。

---

## 1. 最近 3 次 `cargo-audit` run 的实测读数（2026-11-03）

| run | id | 事件 | 分支 | head_sha | created == updated | jobs | conclusion |
|---|---|---|---|---|---|---|---|
| **#1039**（最新） | `41613129991` | push | `debt-round8` | `03827a1ffc` | **是**（秒失败） | **0** | `failure` |
| #1035 | `41610897882` | push | `debt-round8` | `4b2ecd2d0f` | **是**（秒失败） | **0** | `failure` |
| #1032 | `41608062967` | push | `debt-round8` | `4c9e307b11` | **是**（秒失败） | **0** | `failure` |
| #1031 | `41607875780` | push | `debt-round8` | `9ff0f31286` | **是**（秒失败） | **0** | `failure` |

> #1039 的 `head_sha` `03827a1ffc` = 复核时 `debt-round8` 的**最新 tip**
> ⇒ ⇒ **当前 tip 上仍在复现**，且**不是**某个已消失的历史 run 的残留。

**逐条实测依据**：

* `GET /repos/Apeireth/Apeireth/actions/runs/<run_id>/jobs` ⇒ **`"total_count": 0`**
  （三次全部如此）⇒ **没有 job id**。
* `created_at == run_started_at == updated_at`（例如 #1035 全部是 `2026-11-03T09:25:25Z`）
  ⇒ **run 同一秒创建、同一秒失败** ⇒ **从未调度过任何 runner**。
* `GET /repos/Apeireth/Apeireth/actions/runs/<run_id>/logs` ⇒ **HTTP 404**（`"Not Found"`）
  ⇒ **整条 workflow 没有任何日志可下载**。

> ⚠️ **取日志的那条 curl 配方在这里用不上**：它需要 `<job_id>`，
> 而这三次 run **一个 job 都没有** ⇒ 没有 `job_id` 可填 ⇒ **不是 302 重定向的问题，是根本没有 job**。
> 真正的错误信息只能从 **run 详情页**取（见 §2）。

---

## 2. GitHub 侧的**错误原文**（这是定案证据）

run #1035 的详情页（`https://github.com/Apeireth/Apeireth/actions/runs/41610897882`）
里嵌着一条**红色 annotation**：

```text
Invalid workflow file: .github/workflows/cargo-audit.yml#L1 (Line: 162, Col: 9): 'if' is already defined
```

⇒ **行号 162** 与本地扫描的结果**逐字对上**（§3）⇒ **定位闭环**。

> 这条 annotation **不出现在 API 的 jobs/annotations 里**（因为 0 job ⇒ 没有 check-run）
> ⇒ **必须抓 HTML 页面**才能看到 ⇒ 这是本病此前「查日志查不出原因」的原因。

---

## 3. 根因定位：**同一个 step 里的重复 `if:` 键**

当前 `debt-round8`（复核时 tip `03827a1ffc`；run #1035 的 `4b2ecd2d0f` 同款，
文件 md5 `2fabf4b57e528fca9c955053875cf418`）的 `.github/workflows/cargo-audit.yml`：

```yaml
      - name: Convert JSON report to SARIF (R66)     # ← 第 147 行
        if: always()                                  # ← 第 148 行（重复键 之一）
        # 语义对齐 GitHub `github/codeql-action` SARIF schema + qdrant-spark audit SARIF 转换模式.
        ... （8 行说明注释）...
        # audit-report.json 在审计失败时同样已落盘, 故本步必须 always 跑, 让 SARIF 照常产出并上传。
        if: always()                                  # ← 第 162 行（重复键 之二 ⇒ GitHub 报错在这一行）
        run: |
```

**严格重复键扫描**（自写 `yaml.SafeLoader` 覆盖构造器，**不用** PyYAML 默认行为）：

```text
duplicate keys found: 1
  DUP key 'if': first at line 148, repeated at line 162
```

**全仓扫描**（`.github/workflows/*.yml`，共 **46** 个文件）：

```text
DUP .github/workflows/cargo-audit.yml: key 'if' first@L148 repeated@L162
total duplicate keys across all workflows: 1
```

⇒ ⇒ **全仓只有这一处重复键** ⇒ 不是普遍性问题，是**单点**。

### 为什么本地「一直是通过的」

| 解析器 | 对重复键的行为 | 结果 |
|---|---|---|
| `python3 -c "import yaml; yaml.safe_load(...)"` | **静默取最后一个** | **「解析 OK」**（假阴性） |
| `ruby -ryaml -e "YAML.load_file(...)"` | **静默取最后一个** | **「解析 OK」**（假阴性） |
| **GitHub Actions 解析器** | **拒绝 ⇒ Invalid workflow file** | **红灯** |

⇒ ⇒ **这正是它能活过 CI 自检的原因**：仓库里若用 `yaml.safe_load` 验 workflow，
**永远验不出来** ⇒ 门禁必须用**严格模式**（§9 修 B）。

---

## 4. 分支**自然实验**：红/绿与「有没有重复键」**100% 对齐**

同一时刻、同一套 workflow，只换 **ref**（workflow 文件内容取自该 ref）：

| ref | 该 ref 的 workflow 文件 | 重复键 | 实测 run | 结论 |
|---|---|---|---|---|
| `main` | 217 行 / 8 步 | **无** | #1027 `schedule` **success**、#1030 `pull_request` **success** | 🟢 |
| `fix/audit-nojob-run` | 244 行 / 8 步 | **无** | #1016 `push` **success** | 🟢 |
| `debt-round8`（本线） | 238 行 / 9 步 | **有**（L148/L162） | #1031/#1032/#1035 **failure, jobs=0** | 🔴 |
| `ui-engine-p0-2026-10-24` | 227 行 / 8 步 | **有 2 处**（`Upload audit report` L211/212、`Upload SARIF` L221/222） | #1036~#1038 **failure** | 🔴 |

⇒ ⇒ **判别式**：`该 ref 的 workflow 文件有无重复 `if:` 键` ⇔ `run 是红是绿`，
**4/4 对齐，零例外** ⇒ ⇒ **因果成立**，不是巧合。

---

## 5. 根因溯源：**第三次复发，且这次是「合并引入」的**

这不是第一次。历史上**同一种病**已经犯过两轮：

| 轮次 | 引入 | 修复 | 病灶 |
|---|---|---|---|
| 第 1 轮 | `7146f4ad`（2026-10-23「三处『绿灯骗人』」） | `0dd5bf52`（2026-10-24） | `Upload audit report` / `Upload SARIF` 两步各写两次 `if: always()` |
| 第 2 轮 | `89d5e3b8`（2026-10-25）**又**修一轮 | 同上 | 同上两处 |
| **第 3 轮（当前）** | **`62f33288`（2026-10-31 20:34 的合并）** | **未修** | **`Convert JSON report to SARIF (R66)` 一步两行 `if: always()`** |

**第 3 轮的机制**（两个**互不相干**的改动在同一个 step 上撞车）：

* `50576a88`（2026-10-24 12:02，Xephylos，走 **PR #108** ⇒ `main` 侧）
  ⇒ 在 `Convert JSON report to SARIF (R66)` 的**注释块之后**加了一行 `if: always()`。
* `981e9fe7`（2026-10-25 16:06，Mavis，副总指挥线侧）
  ⇒ 在**同一个 step 的 step 名之后**又加了一行 `if: always()`。
* `62f33288`（2026-10-31 20:34，`merge: **main**（PR #108）`）
  ⇒ 把两条线**合到一起**。两行 `if: always()` 在文件里**位置不同**
  ⇒ **git 看不见冲突** ⇒ **自动合并把两行都留下了** ⇒ **重复键出生**。

**逐父验证**（`git show <parent>:.github/workflows/cargo-audit.yml`）：

```text
=== parent eca5aa37 (merge: 副总指挥线) ===   Convert-step if: count=1   (L136)
=== parent cf2e94d0 (Merge PR #108) ===       Convert-step if: count=1   (L112)
=== merge result 62f33288 ===                 Convert-step if: count=2   (L148 + L162)
```

**`git blame` 交叉验证**：

```text
981e9fe7f (Mavis    2026-10-25 148)         if: always()
50576a88e (Xephylos 2026-10-24 162)         if: always()
```

⇒ ⇒ **「两个各自正确的单行修复，被一次无冲突合并叠成了一处重复键」** ——
这解释了**为什么两轮人工复查都没抓住**：前两轮病灶在别的 step，这一轮藏在**合并产物**里。

---

## 6. 内嵌 shell 脚本的语法核查：**全部通过**（排除 (C) 之外的脚本因素）

把 YAML 里 3 个 `run:` 块抽出来做 `bash -n`（**只读核查，未改动仓库**）：

```text
bash -n OK: /tmp/audit_scripts/step_4.sh   (Run cargo audit (json + exit code))
bash -n OK: /tmp/audit_scripts/step_5.sh   (审计判定（按 cargo audit 退出码判红/绿）)
bash -n OK: /tmp/audit_scripts/step_6.sh   (Convert JSON report to SARIF (R66))
```

⇒ ⇒ **脚本语法没问题** ⇒ 失败**不是**脚本写错导致的。

> 顺带说明：这两个 heredoc（`python3 - <<'PYEOF'`）在 YAML 块标量里被**去缩进**后
> 终止符落在**行首**，是合法的 ⇒ 这也是它此前一直能跑的原因。

---

## 7. 逐一排除其它可能（为什么不是 A / B）

| 候选病因 | 实测排除依据 |
|---|---|
| **action 下载失败**（`taiki-e/install-action` / `actions/cache@v6` / `upload-artifact@v7` / `codeql-action/upload-sarif@v4`） | 这些 step **从未开始**（0 job、run 秒失败）⇒ **没有任何 action 被下载** |
| **权限问题**（`security-events: write`） | 同上 ⇒ 未进入 runner ⇒ 与权限无关 |
| **真实漏洞报告** | `cargo audit` **一次都没执行** ⇒ **本次分诊对「当前依赖有无漏洞」不提供任何信息** |
| **第三方 action 挂了 / GitHub 侧噪声** | `main` / `fix/audit-nojob-run` 同期**同版本** action **跑绿** ⇒ 排除 |
| **`paths` / `branches` 过滤导致 0 job** | 本次 0 job 的 run 里 **GitHub 明确报了 `Invalid workflow file`** ⇒ 是**解析失败**，不是「触发被过滤」。（`branches` 过滤的另一种空跑机制见 §8 修 C 的说明） |
| **YAML 语法错误（缩进/标点）** | PyYAML / Ruby / 严格扫描 **均能解析**；YAML **语法**是对的，错的是**键唯一性** |

---

## 8. 最小修复建议（**可执行**，本报告只诊断、未改仓库）

### 修 A（**最小、推荐**）：删掉**重复的那一行**

在 `.github/workflows/cargo-audit.yml` 的 `Convert JSON report to SARIF (R66)` 步里，
**二选一**删掉一行 `if: always()`：

* 删**第 148 行**（step 名之后那行），**保留**第 162 行（注释块之后那行）；**或**
* 删**第 162 行**（GitHub 报错的那行），**保留**第 148 行。

**语义完全等价**（同一步上两行重复键本就不可能同时生效，保留一行即可，该步依旧恒执行）。

**已实测验证**（在 `/tmp` 副本上做，**仓库未改**）：

```text
已删掉原 L148 的 if: always()（保留 L162 那行）
parse OK, steps: 9
duplicate keys: NONE
bash -n OK: step_4.sh / step_5.sh / step_6.sh
```

⇒ **删一行 ⇒ 文件可被 GitHub 解析 ⇒ workflow 会真正跑起来。**

### 修 B（**防复发**，建议同时做）

仓库里**没有**「YAML 重复键」门禁 ⇒ 这病才复发三轮。加一个守卫（§10 的脚本可直接用），
在 CI 里对 `.github/workflows/*.yml` 做**严格**重复键扫描 ⇒ 复发即红。

### 修 C（**降噪**，可与 A 并行）

`on.push.branches` / `on.pull_request.branches` 这类**分支过滤**会造成另一类
「0 job 的占位失败」（run 被创建但**不调度 job** ⇒ GitHub 判 `failure` ⇒ 每次 push 一封红邮件）。
**开放中的 PR #111**（`fix/audit-nojob-run` → `main`，标题「去掉 cargo-audit 的 branches 限制（保留 paths）」）
正是针对**这一类**噪声。

> ⚠️ **但请注意**：**PR #111 不是本病的修复**。它之所以把 run 变绿，
> 是因为**它的分支文件恰好没有重复键**（它从更早的树上切出），**不是因为它修了重复键**。
> ⇒ ⇒ **不要把 #111 的「转绿」当成本病已修** ⇒ ⇒ 修 A 仍需单独做。

### ⚠️ 风险提示（**未修项，要当心**）

**PR #94（`debt-round8` → `main`）当前 `mergeable = true`**（`mergeable_state = blocked`，
被失败的门禁挡住）⇒ ⇒ **若带着这个重复键合进 `main`**，
`main` 的 `cargo-audit` 也会立刻变成秒红（`main` 目前的文件是干净的）
⇒ ⇒ **修 A 应当在做 #94 合并之前落地**。

---

## 9. 结论三选一（**明确证据**）

| 选项 | 判定 | 证据 |
|---|---|---|
| (A) CI 基础设施噪声 | ❌ **否** | 同期 `main` / `fix/audit-nojob-run` 用**同样的 action** 跑绿；GitHub 明确报的是**本仓文件的解析错误** |
| (B) 真实漏洞报告 | ❌ **否** | `cargo audit` **从未执行**（0 job、run 秒失败）⇒ **没有产生任何漏洞条目** |
| **(C) 配置错误** | ✅ **是** | **重复 `if:` 键**（L148 / L162，同一 step）⇒ GitHub 原文 `Invalid workflow file … (Line: 162, Col: 9): 'if' is already defined` ⇒ 严格扫描全仓仅此 1 处 ⇒ 4/4 分支自然实验对齐 |

**⇒ 最小修复 = 删掉那一行多余的 `if: always()`（§8 修 A，1 行）。**

---

## 10. 复核命令（下次复查直接跑）

### ① 取最近 3 次 run 及其 job 数（`jobs=0` 是本病的签名）

```bash
TOK=$(head -1 ~/.git-credentials | sed 's|.*://[^:]*:\([^@]*\)@.*|\1|')
# 本仓的 ~/.git-credentials 归 root 所有 ⇒ 按实际属主调整
curl -sSL -H "Authorization: token $TOK" \
  "https://api.github.com/repos/Apeireth/Apeireth/actions/workflows/cargo-audit.yml/runs?per_page=20" \
  | python3 -c 'import json,sys; [print(r["id"], r["run_number"], r["event"], r["conclusion"], r["created_at"], r["head_branch"], r["head_sha"][:10]) for r in json.load(sys.stdin)["workflow_runs"]]'

for rid in <上面拿到的 run id>; do
  curl -sSL -H "Authorization: token $TOK" \
    "https://api.github.com/repos/Apeireth/Apeireth/actions/runs/$rid/jobs" \
    | python3 -c 'import json,sys; print("jobs:", json.load(sys.stdin)["total_count"])'
done
```

### ② 取**真正的错误原文**（不在 API 里 ⇒ 抓 run 页面）

```bash
curl -sSL -H "Authorization: token $TOK" \
  "https://github.com/Apeireth/Apeireth/actions/runs/<run_id>" \
  | grep -oE "Invalid workflow file[^<]{0,120}"
```

### ③ 严格重复键扫描（**PyYAML 默认行为会漏**，必须覆盖构造器）

```bash
cd /home/bai/wt-apeireth-r3
cat > /tmp/dupscan.py <<'PYEOF'
import yaml, glob
dups = []
class L(yaml.SafeLoader): pass
def cm(loader, node, deep=False):
    m = {}
    for k, v in node.value:
        key = loader.construct_object(k, deep=True)
        if key in m:
            dups.append((key, m[key][1], k.start_mark.line + 1))
        m[key] = (loader.construct_object(v, deep=True), k.start_mark.line + 1)
    return {k: v for k, (v, _) in m.items()}
L.add_constructor("tag:yaml.org,2002:map", cm)
total = 0
for p in sorted(glob.glob(".github/workflows/*.yml")):
    dups.clear()
    yaml.load(open(p), Loader=L)
    for k, a, b in dups:
        total += 1
        print(f"DUP {p}: key {k!r} first@L{a} repeated@L{b}")
print("total duplicate keys:", total)
PYEOF
python3 /tmp/dupscan.py
```

### ④ 内嵌 shell 脚本语法核查

```bash
cd /home/bai/wt-apeireth-r3
python3 - <<'PYEOF'
import yaml, os
d = yaml.safe_load(open(".github/workflows/cargo-audit.yml"))
os.makedirs("/tmp/audit_scripts", exist_ok=True)
for i, s in enumerate(d["jobs"]["cargo-audit"]["steps"]):
    if "run" in s:
        open(f"/tmp/audit_scripts/step_{i}.sh", "w").write(s["run"])
PYEOF
for f in /tmp/audit_scripts/step_*.sh; do bash -n "$f" && echo "OK $f"; done
```

### ⑤ 根因溯源（合并引入）

```bash
cd /home/bai/wt-apeireth-r3
git log --oneline -- .github/workflows/cargo-audit.yml
git blame -L 145,165 --date=short .github/workflows/cargo-audit.yml
git show eca5aa37:.github/workflows/cargo-audit.yml | grep -c "^        if:"   # 父 1
git show cf2e94d0:.github/workflows/cargo-audit.yml | grep -c "^        if:"   # 父 2
git show 62f33288:.github/workflows/cargo-audit.yml | grep -c "^        if:"   # 合并结果 ⇒ 多了 1
```

### ⑥ 修好后的验收（**修复后必跑**）

```bash
# 修好后应看到：job 数 ≥ 1、run 里出现 "cargo audit (RUSTSEC database)"、有 audit-report.json
curl -sSL -H "Authorization: token $TOK" \
  "https://api.github.com/repos/Apeireth/Apeireth/actions/runs/<new_run_id>/jobs" \
  | python3 -c 'import json,sys; d=json.load(sys.stdin); print("jobs:", d["total_count"]); [print(" step:", s["name"], s["conclusion"]) for j in d["jobs"] for s in (j.get("steps") or [])]'
```

### ⑦ 本仓文档门禁（本次新增文档后须保持绿）

```bash
cd /home/bai/wt-apeireth-r3
python3 scripts/check_doc_class.py --check
python3 scripts/check_doc_paths.py --check
```

---

## 11. 仍欠的（**如实**）

| 项 | 状态 | 说明 |
|---|---|---|
| 根因定位（重复 `if:` 键 + 合并引入链 + GitHub 错误原文） | **已修**（本文完成） | 诊断层面已闭环：行号与 GitHub 报错**逐字对上** ⇒ 复核命令见 §10 |
| 删掉重复的 `if: always()`（§8 修 A） | **未修** | 本次**只诊断**，按指令**未改任何 workflow**；**1 行**改动 |
| 重复键门禁（§8 修 B） | **未修** | 仓库无此门禁 ⇒ 已复发三轮 ⇒ 建议随修 A 一起落地 |
| `on.push.branches` 空跑噪声（§8 修 C） | **暂缓** | 由 **PR #111** 处理中 ⇒ 但它**不解决**本病 ⇒ 别混淆 |
| **当前依赖到底有没有 RUSTSEC 漏洞** | **未测** | 0 job ⇒ `cargo audit` **从未执行** ⇒ 本次分诊**不提供**该结论 ⇒ **修好后必须重跑一次**拿真结论 |
| PR #94 合入 `main` 前落地修 A | **未做** | `mergeable=true` ⇒ 带病合入会让 `main` 也秒红 |

> **一句话给下一棒**：**先在 `debt-round8` 删掉 `cargo-audit.yml` 第 148 行（或第 162 行）
> 那一行 `if: always()`**（1 行、语义等价、已实测可解析），
> **再重跑一次 workflow 拿真实的漏洞结论** —— 这次分诊**没有**给出漏洞结论。
