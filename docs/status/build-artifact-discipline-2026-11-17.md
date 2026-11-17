# 构建产物槽位纪律 · 进度页

[Document-Meta]
Document: docs/status/build-artifact-discipline-2026-11-17.md
Class: STATUS
Status: 🟢 活跃
Last-Modified: 2026-11-17
Author: lead

> **用途**：本工作线的「做到哪了」。断言只引文件（设计稿 / 台账 / 守卫输出），不引对话摘要。
> 逐条回答设计稿[批次与验收](../design/build-artifact-discipline-2026-11-17.md#批次与验收)的验收要求。

## 逐条状态（2026-11-17）

| 层 | 状态 | 证据（文件 / 命令 / 实测输出） |
|---|---|---|
| L1 工具链默认值 | ✅ 已落地 | `C:\Users\Dream\.cargo\config.toml`（本批新建，此前不存在）写 `[build] target-dir = "C:\Apeireth\cargo-target"`；审计输出「生效产物目录：C:\Apeireth\cargo-target（来源：cargo metadata）」⇒ 全局默认生效。根 `Cargo.toml` 未改（理由见设计稿 §不变量第 3 条） |
| L2 单入口封装 | ✅ 已落地 | `scripts/cargo.ps1`（360 行）：磁盘预检 fail-closed（空间不足 ⇒ 只读报告模式 ⇒ exit 3，不删文件）、槽位解析（`-Slot base|ui|tool` + `-Line` 分子目录）、环境卫生（`RUSTC_WRAPPER=` 置空 + `CARGO_BUILD_JOBS=2` + `CARGO_INCREMENTAL=0` + `CARGO_PROFILE_TEST_DEBUG=0` + `PYTHONUTF8=1` + `PYTHONIOENCODING=utf-8`）、退出码透传。**实测**：`pwsh -NoProfile -File scripts/cargo.ps1 -Slot base -CargoArgs "--version"` ⇒ 打印 `cargo-plan` / `cargo-env` 两行摘要 ⇒ `cargo 1.97.1 (c980f4866 2026-06-30)` ⇒ `cargo-exit=0` ⇒ EXIT=0 |
| L3 机械守卫（审计） | ✅ 已落地 | `scripts/target_slot.py --audit`：RED ⇒ `AUDIT_EXIT=1`（游离目录 `C:\Apeireth\wt\backend-hardening-completion-20261113\target`），处置后 GREEN ⇒ `AUDIT_EXIT=0`（游离总量 0G） |
| L3 机械守卫（磁盘硬线） | ✅ 已落地 | `scripts/disk_guard.py` 新增 `DISK_WARN_FREE_MB=40000` / `DISK_HARD_FREE_MB=10000`（env 可覆盖，硬线设 0 ⇒ 关闭）；`main()` 里 `ensure_disk = max(args.ensure, DISK_HARD_FREE_MB)` ⇒ harness 旧口径 `--ensure 2000` 不再等于「没有门禁」 |
| L3 机械守卫（回收） | ✅ 已落地 | `reclaim()` 旧版不传 `--target` ⇒ Windows 空转（回收从来没真发生过）；现按解析出的生效目录与登记槽位拼 `--target` 传给 `clean_test_binaries.py` |
| L3 每日哨兵 | ✅ 已落地 | `scripts/disk-watch.ps1`（285 行，只读：不删文件不杀进程）：磁盘/内存走 Win32_LogicalDisk + Win32_OperatingSystem；槽位体积用 `Start-Job` + 90 s 超时；**原样转述 `target_slot.py --audit` 并沿用其退出码**（不复制判定逻辑）。**实测**：`pwsh -NoProfile -File scripts/disk-watch.ps1` ⇒ 输出 disk/memory/slot×3 + 审计全文 + `disk-watch result=OK free_mb=372166 stray_gb=0 audit_exit=0` ⇒ EXIT=0 |
| L3 计划任务注册 | ✅ 已落地 | `pwsh -NoProfile -File scripts/disk-watch.ps1 -RegisterTask` ⇒ `SUCCESS: The scheduled task "Apeireth-DiskWatch" has successfully been created.`；`schtasks /query /tn Apeireth-DiskWatch` ⇒ Next Run Time `2026/11/18 3:00:00`、Status `Ready`、Task To Run 指向本工作树 `scripts\disk-watch.ps1 -Quiet` |
| L3 接入门禁清单 | ✅ 已落地 | `docs/architecture/contract.toml` 的 `[ci_gates]` 新增 `[[ci_gates.gate]] id = "target-slots"`（`cmd = ["python3","scripts/target_slot.py","--audit"]`、`stage = "fast"`、`scope = ["*"]`、`blocking = true`、`required = true`）⇒ **45 条门禁 / 43 条 required**；`python scripts/gen_gate_manifest.py --gen` 后 `--check` ⇒ OK；`python scripts/harness.py plan` ⇒ `target-slots \| yes \| * \| yes — scope=* 总是执行` ⇒ 守卫进入 harness 门禁自动链（不再靠人手动跑） |
| L4 宪章条款 | ✅ 已落地 | `AGENTS.md`：§5 代码块下新增「收口 wave ≠ 日常默认」说明；§5.2 ② 新增 Windows 等价 `clean_test_binaries.py --target` 与「产物目录不许写死 `<repo>\target`」；新增 §5.4 构建产物槽位纪律（四层落点） |
| L4 工作线登记 | ✅ 已落地 | `docs/status/workline-registry.md` 已加本行（设计稿 / 本进度页 / 守卫命令） |
| 硬编码产物目录清零 | ✅ 已落地 | 修完 `check_stale_binaries.py` · `measure-session-startup.ps1` · `target-hygiene.ps1` · `target-sweep.ps1` · `test-installed-e2e.ps1` · `verify-provider-path.ps1` · `windows-live-check.ps1`；全部改为「先解析生效目录再拼接」，回退时显式提示 |

## 交付形态（2026-11-17）

- 分支 `chore/build-artifact-discipline-20261117`（基线 `upstream/main = 5c373955c`），共 **6 条 commit**：
  `325a8339b`（门禁清单生成器 CRLF）· `5fce46c5b`（构建产物槽位纪律的机械层）·
  `6aaff28ec`（fast gate 把契约类 toml 当 Rust 改动的假红修复）· `efcfe9636`（宪章条款与仓库口径校正）·
  `f13349d7b`（进度页补记交付形态）· `2a3ef44d5`（合入 upstream/main，带上 E-198 修复）。
- 已推 fork（`https://github.com/Xephylos/Apeireth`）并开 **PR #131**（base `Apeireth/Apeireth:main`）⇒ **已合并**（合并提交 `e177bda67`，见下）。
- 门禁口径：**本机** doc/脚本门禁全绿（见本页各行证据）；**Rust 侧本批 0 行源码改动** ⇒ 本机未重跑
  `cargo test --workspace`（按 `AGENTS.md` §5.2「`--workspace` 只在收口 wave 跑」），由 CI 矩阵全量跑。
- **合并前 CI（head=`2a3ef44d5`）**：**24 pass / 0 fail / 2 skipping**（skipping＝dependabot 自动合并项 ＋ kani 不适用矩阵项）；
  三平台 nextest 全绿（ubuntu 21m14s / windows 21m58s / macos 32m32s）、合并裁决 pass（2m39s）、arch-guard pass、docs truth / wiring ledger / doc caliber / doc source path 全 pass。
- **合并依赖**：本 PR 的合并裁决曾因既有缺陷 **E-198** 假红（阶段选择只看「有没有 Rust 改动」，而门禁适用性看「diff 命中的文件类型标签」
  ⇒ docs/contract/ci-only PR 把 required 的 deep 门禁记成「适用却没跑」）⇒ 先合并修复 PR #133（`9e438190f`）
  ⇒ 本分支合入上游并重跑 ⇒ **转绿**（缺陷登记见 `docs/04-internal/ui-engine-defect-ledger.md` §3.9 / E-198）。

## 现场事实（2026-11-17 实测）

- C 盘：total 924.6 G / used 555.5 G / free 369.2 G。
- `C:\Apeireth` 下 Rust 产物根（体量）：cargo-target 4.60 G（base 槽位）· cargo-target-ui-desk 10.09 G（ui 槽位）·
  cargo-target-ui-desk2 2.50 G（证据保留）· cargo-target-c23 5.03 G（**他线在途，2026-11-17 仍在写入**
  ⇒ 登记为「在途他线产物」，只报告不判红，迁进登记槽位后从登记表移除）·
  cargo-target-c4-ui 1.49 G（已停用历史遗留，上限 2 G）· cargo-target-c13-model 0.41 G（上限 1 G）·
  cargo-target-check 0.19 G（上限 1 G）。
- 他仓产物（只报告不判红）：`C:\Apeireth\wt\ui-desktop-private\target` 6.01 G。
- 单体最大文件在 base 槽位 `debug\deps` 内为 `lowalt_agent.pdb` 196.7 MB ⇒ **调试信息（含 PDB）才是单体大头**，
  与 `AGENTS.md` §5.2「cargo 的磁盘占用 90% 来自测试二进制」同向。
- 本仓存在 5+ 个独立 cargo workspace（根 workspace · `crates-ui` · `frontend/companion-desktop/src-tauri` ·
  `fuzz` · 两个 crate 内嵌 `fuzz`）⇒ 全局 `target-dir` 会让它们共享同一个构建目录 ⇒ 并行线之间的
  **文件锁争用**是既有事实（`docs/02-guides/fast-feedback-gate.md:64` 已记录该症状）⇒ L2 的 `-Line` 分子目录
  就是为此存在。

## 还差什么

1. **计划任务指向的是工作树路径**：`Apeireth-DiskWatch` 的 Task To Run 指向
   `C:\Apeireth\wt\build-artifact-discipline-20261117\scripts\disk-watch.ps1`
   ⇒ ⇒ 本批合并进上游后**必须改指主仓路径**（`schtasks /delete` + 重新 `-RegisterTask`），否则工作树一旦删除
   任务即失效 ⇒ ⇒ 哨兵静默失联（**这正是本批要根治的那类「静默失效」**）。
2. **⚠️ 计划任务注册本身不持久（本轮实测）**：本批早期 `-RegisterTask` 曾返回
   `SUCCESS: The scheduled task "Apeireth-DiskWatch" has successfully been created.` 且复核到
   `Next Run Time 2026/11/18 3:00:00` / `Status Ready`；**但 2026-11-17 17:4x 复核时该任务已不存在**
   （`schtasks /query /tn Apeireth-DiskWatch` ⇒ `ERROR: The system cannot find the file specified.`；
   全部 376 条任务里无匹配项；`Schedule` 服务在运行）⇒ 已重新注册并复核通过（`Next Run Time 2026/11/18 3:00:00`）。
   ⇒ ⇒ **结论：每日哨兵只能算 best-effort，不许当作守门依据**；真正的守门是**已接进门禁链**的
   `python scripts/target_slot.py --audit`（版本受控、每次 PR 必跑、退出码可判定）⇒⇒
   **这也是本批最该记住的一条：规则/守卫的存活率 ∝ 它离执行点的距离**（`AGENTS.md` §5.4 的四层正是为此）。
3. **未做跨机器验证**：本批只在 Windows 本机实测；Linux/macOS 上 `target_dir.py` 的 `cargo metadata` 路径
   未实跑（CI 不读用户级配置 ⇒ CI 行为不变已论证，但未实测）。审计在「本机无任何登记槽位根」时
   **跳过并退出 0** 已用「登记表指向不存在根」的实测证过（Linux CI 路径）。
4. **未做多 workspace 共享 target-dir 的实测**：5+ 个独立 workspace 共用一个全局 `target-dir` 时的
   文件锁争用只做了设计防护（-Line），未实测争用强度。
5. **未清理既有历史遗留目录**：cargo-target-c23 / c4-ui / c13-model / check 仍占约 7 G，只登记成棘轮或在途项；
   何时回收取决于他线是否还在用（`c23` 在 2026-11-17 仍在写入）⇒ **不替别人的线做决定**。
6. **c23 迁移债未清**：`C:\Apeireth\cargo-target-c23`（他线在途，6.06 G）待该线迁入登记槽位
   ⇒ 届时从 `docs/architecture/target-slots.json` 移除该条（审计会从「🔵 在途他线产物」变为不再出现）。
7. **本批顺手修掉的既有缺陷**：`scripts/gen_gate_manifest.py` 在 Windows 上用 `Path.write_text` 回写
   ⇒ 把 LF 翻成 CRLF ⇒ 整个 PR 模板与门禁清单被「全文件重写」（172 行 / 751 行空白噪声 diff）
   ⇒ 现已改为 `open(..., newline="\n")` 强制 LF ⇒ 重生成后 diff 收敛为 4 行 + 16 行。
8. **本批自己的产物已按纪律回收**：2026-11-17 17:4x 用仓内工具
   `python scripts/clean_test_binaries.py --target …` 清掉本线各批次（`backend-contracts` / `-a3` / `-b1` / `-b2` / `-b3a`）
   共 **753 个测试二进制（15.4 G）**（保留 `*.so`），base 槽位由 **61.3 G** 回落到 **45.9 G**
   ⇒ ⇒ 即「cargo 的磁盘 90% 来自测试二进制」这条既有实测口径的第一次**真实回收**（此前 Windows 上从未真回收过）。

## 本批过程中被守卫抓到的真实缺陷（不是演练）

- **在途他线目录按体量上限判红**：`cargo-target-c23` 在 2026-11-17 从 3.93 G 涨到 5.03 G，超出登记上限 4 G
  ⇒ 审计判红 ⇒ 但那是**他线仍在写入的产物**，不是已停用的历史遗留
  ⇒ ⇒ 修正为：登记表加 `active = true` ⇒ **只报告不判红**（上限仅作膨胀提示）
  ⇒ ⇒ **已停用**的历史遗留才适用「只许缩不许涨」的体量上限棘轮
  ⇒ ⇒ 理由：在途目录按上限判红 ⇒ 守卫长期红 ⇒ **长期红的守卫等于没有守卫**（`AGENTS.md` §5.4 已固化这条）。
- **回收在 Windows 上从来没真发生过**：`disk_guard.py` 的 `reclaim()` 不传 `--target`
  ⇒ `clean_test_binaries.py` 在 Windows 上扫不到任何候选根 ⇒ 空转 ⇒ 已重写。
- **CI 会被本机底线误红**：`ensure_disk = max(args.ensure, DISK_HARD_FREE_MB)` 会让 harness 的 `--ensure 2000`
  在托管 runner 上变成 10 GB 硬线 ⇒ 空间一低 ⇒ **每条门禁全变 UNVERIFIED**
  ⇒ ⇒ 已加 CI 例外：`CI` 环境变量非空且用户**未**显式给值 ⇒ 不套用本机底线（打印「CI：不套用本机底线」）
  ⇒ ⇒ 显式给了值 ⇒ 以用户值为准（**不许自我豁免**）⇒ 三种情形均已实测（EXIT=0 / EXIT=0 / EXIT=1）。

## 守卫（机器判对错）

Windows（本机实测口径）：

```powershell
python scripts/target_slot.py --audit        # 游离目录 ⇒ 非 0；本机无槽位根 ⇒ 跳过并退出 0
python scripts/target_dir.py                 # 打印生效产物目录（单行）
python scripts/disk_guard.py --ensure 20000  # 绝对可用量硬线（默认 10 GB）
pwsh -NoProfile -File scripts/disk-watch.ps1 # 每日哨兵
```

Linux / macOS（等价，`python3` 解释器名不同）：

```bash
python3 scripts/target_slot.py --audit
python3 scripts/target_dir.py
python3 scripts/disk_guard.py --ensure 20000
```
