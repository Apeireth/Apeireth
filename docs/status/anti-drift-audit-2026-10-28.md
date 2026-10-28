# 抗漂移审计（2026-10-28）—— 仓库自身的约束够不够好？**8 个漏洞 + 修法**

[Document-Meta]
Document: docs/status/anti-drift-audit-2026-10-28.md
Class: AUDIT
Status: 🟢 活跃
Last-Modified: 2026-10-28
Author: lead

> **审计对象**：本仓库**自身**的抗漂移装置（不是 `AGENTS.md` 这类 AI 侧行为约束）。
> **审计问题**：**约束够不够好？** 哪些漏洞让"漂移"能悄悄发生？
> **产出**：8 个可命名的漏洞 + 每条的修法与实测效果 + **仍未修的边界**。
> 复核：见 §5 的命令。

---

## 1. 先看家底：装置确实很厚

| 类别 | 规模 |
|---|---|
| CI 工作流 | **41** 个 |
| 守卫脚本 | **38** 个 |
| 棘轮基线 | **7** 条（arch / unwired / unwired-modules / dead-code / doc-path / coverage / quality-metrics） |
| 契约 | `contract.toml`（分层 / 依赖边 / 尺寸 / 权重 / 审批策略）+ `exceptions.toml` |
| 行为锁定 | **280** 个文件含确定性 / 逐字节断言；schema **未知版本显式拒** |

**三条我认可的设计**：
1. 「**准确度一分不许掉**」+ **1% 容差**（错误代价不对称：记错 > 记不住）；
2. **棘轮只减不增** + 基线例外清单；
3. **唯一交换许可**：准确度不变的前提下省 token（纯赚才换）。

---

## 2. 但**判据严谨度中等偏弱**：8 个漏洞

> 一句话总结：**"防得住粗心，防不住取巧"** ——
> 多数守卫在检查**形式**（有没有引用 / 有没有标记 / 行数有没有涨），
> 而漂移者（包括我自己）会**顺着形式走**。

| 编号 | 漏洞 | 严重度 | 状态 |
|---|---|---|---|
| **H1** | 未接线判据把 `examples/`/`benches/` 当生产引用 ⇒ **写个 example 就算接线** | 🔴 高 | ✅ **已修** |
| **H2** | `claims.yaml` 的 `verify_cmd` **从不被执行** ⇒ 装饰品 | 🔴 高 | ✅ **已修** |
| **H3** | `STALE` 只查登记过的 source ⇒ **漏列即静默漂** | 🔴 高 | ✅ **已修** |
| **H8** | 文档**无分类** ⇒ 约束靠 AI 自觉 | 🔴 高 | ✅ **已修** |
| **H4** | 尺寸棘轮可被「搬家」满足（复杂度横向移动） | 🟠 中 | ✅ **已修** |
| **H5** | `check_doc_caliber` 是**形式守卫**（写标记就过） | 🟠 中 | ✅ **已修** |
| **H6** | **慢性红**（`cargo +nightly doc` 三次全红）⇒ 红失去信号价值 | 🟠 中 | ✅ **已修** |
| **H7** | 批次进度页**零守卫** ⇒ 漂一整天没人喊 | 🟠 中 | ✅ **已修** |

---

## 3. 已修的四条：修法与**实测效果**

### H1 ✅ 未接线判据排除 `examples/` `benches/`

* **漏洞**：`audit_unwired.py::is_test_file()` 只排 `/tests/`
  ⇒ 往 `examples/` 写一句 `use x::...`，`x` 就从「整块未接线」审计里**消失** ⇒ 基线"改善"（**假改善**）。
* **修法**：一并排除 `/examples/` 与 `/benches/`。
* **实测**：整块未接线模块 **81 → 93**（**+12 个此前被 example 藏住**）；
  记忆 crate 侧 **41 → 51**；孤儿 API（符号级）142 不变。
* ⚠️ **这是「修正判据」不是「新增欠债」**：数字涨不是代码变差，是此前看不清。

### H2 ✅ `verify_cmd` 真跑

* **漏洞**：29 条声明**每条**都有 `verify_cmd`，但 `docs_truth.py` **只用 git grep + git diff**，**从不执行**
  ⇒ 声明可以标 `PRODUCTION WIRED`，而验证命令**早已失败或被删**，无人发现。
* **修法**：`scripts/check_claims_verify.py`（新）+ 独立 CI job `claims-verify.yml`
  ⇒ 纪律：**要么真跑，要么显式声明为什么不能跑**。
  * 能跑 ⇒ 真跑，要求退出码 0；`cargo test` 系还要求 `test result: ok` 且 **0 failed**；
  * **跑不了**（`<占位符>` / 缺工具 / PowerShell cmdlet）⇒ 必须 `verify_skip_reason`；
  * **能跑但当前环境不允许**（缺系统库 / 由另一条流水线覆盖）⇒ `verify_defer_reason`
    ⇒ **单独统计、不算通过**（延后项必须人工复核）。
* **实测（首次真跑就抓到 6 条问题）**：现 **23 跑全过 · 2 豁免 · 4 延后**。
  ⇒ 4 条延后全是**前端**：子工程 `Cargo.lock` 与 `Cargo.toml` **不同步**（`--locked` 拒绝更新）
  + 本机缺 `cairo`/`webkit`/`gtk`；且 `pnpm test` **当前是红的**。

### H3 ✅ `STALE` 加**目录级兜底**

* **漏洞**：`changed_since()` 只比对**声明里列出的** sources
  ⇒ 真实依据文件没列进去 ⇒ **永远不会 STALE** ⇒ 声明与代码可无限脱节。
* **修法**：`changed_in_scope_dirs()` —— 取 source 的**所在目录**，看该目录下**未列出**的文件变过没有
  ⇒ 报「**疑似漏列 source**」（提示，不直接判失败）。
* ⚠️ **第一版太吵**：对所有目录兜底 ⇒ **29 条里 17 条命中** ⇒ 噪声 = 训练人忽略告警（与"慢性红"同病）
  ⇒ 收窄到**窄目录**（git 跟踪文件 ≤ 8 个）⇒ **17 → 2**，信号可用。
* **实测（抓到 2 处真漏列，已修）**：

| claim | 漏列的 source | 后果 |
|---|---|---|
| `sec.linux_containment` | `crates/capabilities/tools/src/process/mod.rs` | 进程执行总入口，改了遏制行为也会变 |
| `sec.governance_chain_drill` | `crates/foundation/governance/tests/rubric_advice.rs` | 合练用的测试辅助 |

⇒ 补进去后**两条都变 STALE**（守卫在正确工作）⇒ 重跑验证（**19 passed / 3 passed**）后 bump。

### H8 ✅ 文档分类守卫（九类 + 每类强制结构 + 存量棘轮）

* **漏洞**：文档约束全靠 `AGENTS.md` / 宪章的**行为要求**；`[Document-Meta]` 约定
  只有 **277/499** 份文档遵守，而**没有守卫** ⇒ 无人发现。
* **修法**：`scripts/check_doc_class.py` —— 每份活跃文档必须声明 `Class`，九类各有**可判定**强制项
  （`SPEC`/`STATUS`/`AUDIT`/`ROUTE`/`DECISION`/`EVIDENCE`/`GUIDE`/`LEDGER`/`INDEX`）；
  存量用棘轮基线（**491 份待分类**，只许缩表）。
* ⚠️ **我自己的守卫也差点「形式可满足」**：第一版用「关键词出现在正文里」
  ⇒ 我自己造的探针写「本文**没有**批次/验收/不做章节」**竟然通过了**
  ⇒ 改为**标题级 / 列表表格行级锚定**，并加两条反证（反着写的 `SPEC` 失败 · 散文冒充的 `AUDIT` 失败）。

---

## 4. 后修的四条：修法与实测效果

### H4 ✅ crate 体量预算（防「搬家式漂移」）

* **修法**：`scripts/check_crate_budget.py` —— **每 crate 的 `src/` 总量**（行数 + 文件数）进基线，**只减不增**。
  ⇒ 与 `arch_guard`（管**单文件**）分工：两者合起来，"拆文件"不再能规避尺寸约束。
* **基线**：**29 个 crate · 合计 528,097 行**。

### H5 ✅ 高口径声明**必须真验**（把 caliber 从形式升级为真值）

* **修法**：`check_claims_verify.py --wired-gate` ——
  口径为 `PRODUCTION WIRED` / `DEFAULT ENABLED` / `HARDWARE VALIDATED` 的声明
  **不许**靠 `verify_skip_reason` / `verify_defer_reason` 成立。
* **实测抓到 3 条**（`sec.sql_readonly_gate` / `sec.provider_proxy_ssrf` / `sec.desktop_file_roots`）：
  它们标着 `PRODUCTION WIRED` 却**验证被延后** ⇒ 按口径纪律**降级为 `IMPLEMENTED`**
  （不能验证就不能宣称已接生产）。
* 现已并入 `claims-verify.yml`（阻塞）。

### H6 ✅ 清掉慢性红（**本轮最值钱的一条**）

* **真实原因**（不是环境）：`crates/engine/memory/src/universal_forget.rs` 的一处
  **rustdoc 内联链接**触发 `-D rustdoc::redundant-explicit-links`
  ⇒ `cargo +nightly doc --workspace --no-deps --all-features` **exit 101**、`could not document apeireth-memory`。
* **修法**：把 `[`AccessGuard`](crate::forget_access_guard::AccessGuard)` 改成行内代码 `` `AccessGuard` ``。
* **实测**：`RUSTDOCFLAGS="-Dwarnings --cfg docsrs" cargo +nightly doc --workspace --no-deps --all-features`
  ⇒ **exit=0 · 0 error** ⇒ **慢性红消失，红重新有信号价值**。
* ⚠️ 另有 6 条 `unused workspace dependency`（nightly cargo 的 unstable lint）⇒ **不阻塞**，但记为**观察项**。

### H7 ✅ 批次进度页**自动事实块**

* **修法**：`scripts/check_batch_status.py` —— 进度页里嵌机器生成块
  `<!-- BEGIN:auto-facts --> … <!-- END:auto-facts -->`，共 **17 条事实**
  （D0 哈希链在写入路径 · D4 默认关 · B0-c 模块/候选/已测 · B1/B4 模块 · CLI 分派 ·
  B2 三接口 **3/3** · B3 影子双跑 · B0-a 适配器与 **`reproduce.sh` 无** · claims 规模）。
  ⇒ `--check` 不一致即红 ⇒ **手写散文无法覆盖事实值**。
* **反证**：手把「已测行数」改成 `99` ⇒ 守卫**当场红**并指出差异行。

**全部 8 条（H1–H8）均已修**；其余已知边界：
* H1 残余：`build.rs`、`#[cfg(test)]` 内联模块仍可能被算作生产 ⇒ 彻底解决需**从装配根做可达性分析**；
* 前端 4 条 claim 延后（见 H2），其中 `Cargo.lock` **不同步是真正的漂移**，应由前端侧修。

---

## 5. 复核命令

```sh
# H1：未接线判据是否排除了 examples
python3 scripts/audit_unwired_modules.py | head -1        # 应为 93

# H2：每条 claim 的 verify_cmd 真跑（慢；会编译）
python3 scripts/check_claims_verify.py --list            # 先看计划：23 跑 / 2 豁免 / 4 延后
python3 scripts/check_claims_verify.py --check --timeout 600

# H3：STALE 与「疑似漏列」
python3 scripts/docs_truth.py --verify                   # 看 suspected_missing=

# H8：文档分类
python3 scripts/check_doc_class.py --check
python3 scripts/check_doc_class.py --report
```

---

## 6. 结论

* **覆盖面：强**（41 CI / 38 脚本 / 7 棘轮，比多数生产仓库厚）；
* **判据严谨度：中等偏弱** —— 多处"形式可满足"（example 引用 / 写个标记 / 搬家 / 漏列 source）；
* **健康度：有慢性红** ⇒ 这是最该先修的（比加新守卫更值钱）。

⇒ 本轮的修法取向是**把约束从"人自觉"搬到"机器判定"**：
**文档要有分类 · 声明要有可跑验证 · 判定要锚定结构（不是关键词）· 兜底要窄到有信号。**
