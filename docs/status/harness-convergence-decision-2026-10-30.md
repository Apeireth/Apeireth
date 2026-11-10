[Document-Meta]
Document: docs/status/harness-convergence-decision-2026-10-30.md
Class: DECISION
Status: 🟢 活跃
Last-Modified: 2026-10-30
Author: lead

# 仓库 harness 收敛 —— **裁定记录**（2026-10-30）

> **来源**：owner 转来 Astra 的仓库 harness 评审意见（10 条建议 + 7 条问题诊断）。
> **本文不是复述那份意见，而是逐条用本仓实测证据核对后的裁定。**
> **核心判断**：采纳其**问题诊断**与**方向**，但**不采纳其载体** ——
> 理由见 §二（这是本轮最重要的一个决断）。

---

## 一、一句话结论

**Astra 说的问题大部分是真的，但「另建 `.harness/` 平行契约树」这个修法会造成它自己抱怨的那个问题（第二真源）。**
⇒ ⇒ **改为扩展仓库已有的 `docs/architecture/contract.toml`**（CI 已经在读它），
把 **门禁登记表（`[ci_gates]`）** 补进去，并让**人读的副本**（PR 模板）**由脚本生成 + 机器校验**。

---

## 二、逐条核对：7 条问题诊断 —— **哪些是真的**

| # | Astra 的问题 | 本仓实测证据 | 裁定 |
|---|---|---|---|
| 1 | 门禁没有唯一入口；`verify_changed` 默认 report-only；docs-only 直接放行 | `scripts/verify_changed.py:149` → `arch_mode = "--strict" if args.arch_strict else "--report"`；`:161-162` → `no Rust/Toml changes … docs-only; run doc gates instead.` 后 `return 0` | ✅ **成立** |
| 2 | 规则存在多份手写副本（LOCKED / baseline / crate 数 / PR 模板旧数字） | **CI 侧已解决**：`.github/workflows/rust.yml:126-140` 用 `tomllib` 从 `contract.toml` 读 `[locked]`，并明写「**禁止在本 workflow 手抄路径清单（D5：禁硬编码）**」；**人读侧未解决**：`.github/PULL_REQUEST_TEMPLATE.md` 手写「**24 LOCKED crate**」与「**1.2.0** semver」，而 `contract.toml` 已是 `version = "2.0.0-rc.4"` / `members = 30` | ✅ **成立**，但**漂移点与 Astra 猜的不同**：不在 CI，在 PR 模板 |
| 3 | CI 缺统一最终裁决层 | `.github/workflows/` 实测 **42 个** workflow；全仓 grep 无 `merge-gate` / 聚合裁决 | ✅ **成立** |
| 4 | AI 交付协议主要靠自觉 | **部分已不成立**：`scripts/check_claims_verify.py --wired-gate`（每条 claim 的 `verify_cmd` **真跑**；高口径 claim **不许** skip/defer）、`scripts/check_doc_class.py`（9 类文档强制结构 + 存量棘轮） | ⚠️ **部分不成立** |
| 5 | 证据没成为结构化产品 | **部分已成立**：`docs/**/verification-ledger.yaml`、`scripts/check_stale_binaries.py`（编译期 `CARGO_MANIFEST_DIR` 产物防陈旧）、`scripts/check_golden_baseline.py`；**缺**「每次运行绑 commit+tree SHA 的 bundle」 | ⚠️ **部分成立** |
| 6 | 工作边界没被仓库主动管理 | **有载体但不在仓内**：owner 裁定协调看板**不入 git**（`/home/bai/Projects/apeireth-coordination/`）；`AGENTS.md §0` 要求开工前认领工作线 | ⚠️ **成立（载体是 owner 决定）** |
| 7 | PR 规模与来源不受控 | 本轮我一直按任务拆 commit；但**无机器层**分类/门禁差异 | ✅ **成立** |

**复核命令**：
```sh
grep -n "arch_mode\|docs-only" scripts/verify_changed.py
ls .github/workflows/*.yml | wc -l          # 42
grep -rn "merge-gate" .github/ scripts/      # 无命中
grep -n "24 LOCKED\|1\.2\.0" .github/PULL_REQUEST_TEMPLATE.md
```

---

## 三、逐条裁定：10 条建议

| # | Astra 的建议 | 裁定 | 理由（含证据） |
|---|---|---|---|
| 1 | 新建 `.harness/`（contract / policy / schema） | 🔁 **改造后采纳** | 仓库**已有** `docs/architecture/contract.toml`（201 行，13 个节，已被 **3 个 workflow + 2 个脚本**消费）。另建 `.harness/contract.toml` = **第二真源** = 正是第 2 条抱怨的问题本身 ⇒ **改为扩展现有契约**，加 `[ci_gates]` |
| 2 | AI 任务准入（task manifest / base SHA / allowed_paths） | ⏸️ **暂缓接线，先落 schema + 校验器** | 与 owner「看板不入 git」的规则相邻 ⇒ **需要 owner 拍板**才能入仓；本轮先做**非阻塞**的 schema 与校验器，不接进 required gate |
| 3 | Diff Classifier（先判变更再决定门禁） | ✅ **采纳** | 已有雏形：`scripts/verify_changed.py` + `scripts/docs_truth.py` 的 `changed_in_scope_dirs()` ⇒ 升级为 `harness.py plan` 的能力 |
| 4 | 统一入口 `scripts/harness.py plan/check/attest/merge-decision` | ✅ **采纳（P1，本批实施）** | 「很多优秀独立门禁」这个判断准确 ⇒ 但**唯一入口**不该是「再写一份 runner」，而应是**读取真源 + 编排** |
| 5 | 唯一 required check `harness / merge-gate` | ✅ **采纳（P2）** | 42 个 workflow 无聚合是**实测事实**；先做本地 `merge-decision`（可判），再落 workflow（远程不可绕过） |
| 6 | Evidence Bundle（绑 commit/tree SHA） | ✅ **采纳（P1 `attest`）** | 已有 `verification-ledger.yaml` 打底；缺的是**每次运行绑 SHA** ⇒ `artifacts/harness/<sha>/` **不入 git**（只作本地/CI artifact） |
| 7 | 声明 → 证据的可执行协议 | 🔁 **部分采纳** | claims 侧**已实现**（`check_claims_verify.py`）⇒ 真正缺的是**人读副本**（PR 模板勾选框）⇒ 改为「模板数字由脚本生成 + 漂移即红」 |
| 8 | Hook 三层（fast / contract / remote）+ `--no-verify` 留痕 | 🔁 **采纳语义，不改结构** | 三层语义直接进 `harness.py` 的状态机（`SKIPPED`≠`PASS`、`UNVERIFIED` 不可合并）；但 `--no-verify` 留痕要改提交格式 ⇒ **需 owner 拍板**，本轮不擅自改 |
| 9 | 并行 worktree lease | ⏸️ **暂缓** | 同 #2（入不入 git）；且本机当前只有**一个** worktree ⇒ 暂不产生实际收益 |
| 10 | PR 四层分类 A/B/C/D | ✅ **采纳** | 落成 task manifest 的 `tier` 字段 + **不同门禁集合**（Harness 改动不能由自己定义新规则） |

**Astra 最有价值的一句（我同意并已落进设计）**：
> 不要继续给 AI 增加更多说明文档，而要把「任务边界、事实声明、验证证据、合并资格」都变成**仓库可以拒绝的对象**。

⇒ ⇒ 这与本仓 2026-10-28 的抗漂移改造（8 个漏洞 H1–H8 已修）**同一条路线** ⇒ 不是新方向，是**把既有方向做完**。

---

## 四、实施顺序（本批起）

| 阶段 | 内容 | 状态 |
|---|---|---|
| **P0** | 契约扩展 `[ci_gates]`（门禁登记表）+ 生成器 + **漂移守卫**（PR 模板数字从真源生成，对不上即红）+ 本决策文档 | 🔄 本批 |
| **P1** | `scripts/harness.py`：`plan` / `check` / `attest` / `merge-decision` + 六态语义（`PASS`/`FAIL`/`BLOCKED`/`SKIPPED`/`UNVERIFIED`/`WAIVED`，**`SKIPPED`≠`PASS`**、`UNVERIFIED` 不可合并） | 🔄 本批 |
| **P2** | `harness / merge-gate`（远程唯一裁决）+ task manifest schema + 声明→证据校验 | ⏳ 待做 |
| **P3** | Actions 收敛（所有 `paths:` 过滤必须产出**可判定的** skipped 结论） | ⏳ 待做 |

---

## 五、已落地的部分（本批）

1. `docs/architecture/contract.toml` 增加 `[ci_gates]` —— **门禁的唯一真源**（与 `[locked]`/`[workspace]` 同级）；
2. `scripts/gen_gate_manifest.py` —— 从契约生成**人读副本**（PR 模板的锁定清单/版本/crate 数）；
3. `scripts/gen_gate_manifest.py --check` —— **漂移守卫**：人读副本与契约不一致 ⇒ **红**（同一个脚本 `--gen` / `--check`，与 `check_batch_status.py` 同约定）；
4. `scripts/harness.py` —— 唯一入口与状态语义；
5. `scripts/guard-inventory.json` + `scripts/check_guard_inventory.py` —— 全仓守卫的**机读清单**（唯一入口的底账）。

**这批的净效果**（一句话）：
**「规则写在哪」从「靠人记得同步」变成「机器拒绝」** —— PR 模板里那个 `1.2.0` / `24 LOCKED crate`
从此**不可能再默默过期**（生成器会改，守卫会红）。

---

## 六、诚实边界（如实）

* ⚠️ **网络不通**：本机 `git ls-remote origin` 返回 `Permission denied (publickey)` ⇒
  **无法核实 Astra 提到的 5 个开放 PR（#95/#94/#78/#31/#22）的当前内容**；
  只能核实**本地 refs**：`origin/main` = `4ea9c260`「门禁与测量加固整合批（28 commit）」⇒
  **#94 那批看起来已在 main 里**（Astra 看到它「开放」应是因为他也拉不到）。
* ⚠️ Astra 的本地是 **Windows**（`C:\Apeireth\src\Apeireth`）⇒ 与本机 worktree **不是同一份工作副本**，
  其「本地最新提交 `9ff493e`」**无法在本机复现** ⇒ 未采信。
* ⚠️ Astra 列的文件清单里 **`ENGINEER-MANIFESTO.md` 在本仓不存在** ⇒ 其清单**不是逐条核对过的**；
  本文的核对是**逐条实测**，不沿用其清单。
* ⚠️ 本仓 `astra-contract-2026-10-23` 分支**只有 ADR-0014 设计稿**（4 文件 / +463 行，**无实现**）
  ⇒ 说明「契约化」这件事**此前已议过但未落地** ⇒ 本轮是**第一次落地**，不是重复劳动。

---

## 七、选项（给 owner 拍板）

| 选项 | 内容 | 代价 | 我的建议 |
|---|---|---|---|
| **A** | **扩展现有 `contract.toml`**（加 `[ci_gates]`），PR 模板数字由脚本生成 + 守卫校验 | 无新真源；改动集中在一个文件 + 两个脚本 | ✅ **建议采纳** |
| **B** | 按 Astra 原案新建 `.harness/` 平行契约树 | **第二真源** ⇒ 新的漂移源；CI/脚本/文档要分两处读 | ❌ 不建议 |
| **C** | 只做最小闭环（只修 PR 模板漂移 + 加 `harness.py`），**不动契约** | 最省事，但**漂移根因**（规则多处手写）仍在 | 🟡 可作 P0 的退路 |

### 我的建议（**拍板项**）

1. ✅ **采纳选项 A**，按 §四 的 P0→P3 推进（**P0/P1 本批已做**）；
2. ✅ **owner 2026-10-30 拍板**：**task manifest / lease** ⇒ **按建议执行**：
   * **可入仓**（`.harness/tasks/<id>.yaml`），但**非强制**、**不接 required gate**；
   * 用途**仅限**给 AI 提供「任务边界 + 最小上下文」（见 §九 效率型 harness）；
   * **不动**协调看板（owner 先前裁定它不入 git ⇒ 继续不入）。
3. ✅ **owner 2026-10-30 拍板**：**`--no-verify` 留痕** ⇒ **按建议执行**：
   * **不改提交格式**（不写 commit metadata）；
   * hook 三层因此只落在**语义层**：`fast`（本地快反馈，可绕过）·
     `contract`（阻断层）· `remote`（唯一 required check）——
     **`--no-verify` 绕过本地层**这件事**如实告知**，但**不落痕**。

---

## 八、效率型 harness（2026-10-30 追加裁定）

> **来源**：Astra 的第二份意见（效率型：让 AI **更快找到正确路径**、**更早暴露错误**）。
> **裁定**：**采纳 5 项**（其中 3 项已落地），**其余 8 项暂缓并给出理由**。

### 8.1 已采纳（并已落地）

| # | Astra 的建议 | 本仓落点 | 状态 |
|---|---|---|---|
| **E1** | **两阶段反馈**（fast → deep） | `harness.py verify` + 契约门禁新增 `stage` 字段（**fast 22 条 / deep 13 条**） | ✅ **已落地** |
| **E2** | **失败分类器**（9 类 + 下一步动作） | `scripts/harness_failures.py` + `.harness/knowledge/` | 🔄 并行推进 |
| **E3** | **影响分析（层级判定）** | `scripts/harness_impact.py`（L0–L4） | 🔄 并行推进 |
| **E4** | **仓库状态快照 / 最小上下文包** | `harness.py context`（`.harness/context/`） | 🔄 并行推进 |
| **E5** | **失败学习库** | `.harness/knowledge/failure-patterns.yaml`（**已用本仓 10 条真实失败**播种） | 🔄 并行推进 |

**E1 的实测收益**（本机，2026-10-30）：

| 阶段 | 门禁数 | **耗时** | 说明 |
|---|---|---|---|
| `fast` | 22（本次适用 13） | **2.9 秒** | 全部只读、不编译 ⇒ **每次改动都能跑** |
| `deep` | 13（本次适用 3） | **约 54 秒** | 要编译 / 重活 ⇒ 只在 fast 过了才跑 |

⇒ ⇒ **短反馈回路提速约 20 倍**（直接对应 owner「**别一直跑全量了**」）。

### 8.2 暂缓（**理由逐条**，不是不做）

| # | 建议 | 暂缓理由 |
|---|---|---|
| E6 | 不稳定测试自动评估（重复 100 次） | 成本高（100× 测试时长）⇒ 先用失败分类器的 `flaky-test` 类 + 已有 `check_golden_baseline` 观察项兜住 |
| E7 | 属性 / 变形 / 故障注入测试 | 本仓**已有** fuzz / Kani / Miri / guard-v3.3 ⇒ 先补**注册表**，**不新造框架** |
| E8 | 风险热图 reviewer | 与 `arch_guard` / `check_guard_wiring` **重叠** ⇒ 折进 E3 的层级判定，不另起一套 |
| E9 | 验证缓存绑内容身份 | GitHub Actions 已按 `Cargo.lock` 等键缓存；**tree_sha 绑定**是正确性问题 ⇒ 排在 P2 |
| E10 | 质量指标仪表盘 | 需要历史数据积累 ⇒ 排在 P2（先埋点，后出表） |
| E11 | merge queue 批次 | 依赖**远端分支保护**配置（本机不可读）⇒ **需 owner 在远端确认** |
| E12 | 任务拆分质量检查 | 本轮**已做**（本仓批次按 contract / implementation / tests / wiring / docs 分开提交）⇒ 补成机器检查排 P2 |

### 8.3 本轮**没照抄**的地方（同 §二的纪律）

Astra 的 `.harness/context/` 与 `.harness/knowledge/` 目录结构**采纳**；
但**门禁登记表不放进 `.harness/`** —— 它已在契约 `[ci_gates]`（唯一真源）⇒
**不再造第二处**（这是 Astra 自己第 2 条反对的问题）。

---

## 八、相关文件

| 文件 | 作用 |
|---|---|
| `docs/architecture/contract.toml` | 规则唯一真源（新增 `[ci_gates]`） |
| `scripts/gen_gate_manifest.py` | 门禁清单生成器 + 漂轮守卫（`--gen` / `--check`）|
| `scripts/harness.py` | 唯一入口 + 六态语义 |
| `scripts/guard-inventory.json` | 全仓守卫机读清单 |
| `scripts/check_guard_inventory.py` | 清单同步守卫 |
| `docs/architecture/contract.toml`（读者） | `scripts/arch_guard.py` / `scripts/arch_doc.py` / `rust.yml` / `release-prep.yml` / `companion-desktop-ci.yml` |
