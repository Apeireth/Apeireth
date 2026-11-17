# 构建产物槽位纪律（磁盘漂移根治）

[Document-Meta]
Document: docs/design/build-artifact-discipline-2026-11-17.md
Class: SPEC
Version: 0.1
Last-Modified: 2026-11-17
Status: 🟢 活跃
Author: lead

## 目标与范围

把「构建产物落在哪里」从**靠人记得**的约定，变成**工具链默认值 + 单入口 + 机械守卫 + 宪章条款**四层。
依据：2026-11-17 现场实测（C 盘 924.6 G，Rust 构建产物散落在 8+ 个目录），以及
[AGENTS.md §5.2](../../AGENTS.md) 已记录的一次全量 nextest 产出 1129 个测试可执行文件（256.3 G）的事实。

范围限于**构建产物的落盘位置与回收方式**，不涉及业务代码、不改构建语义、不改 CI 行为。

## 批次与验收

| 批次 | 交付目标 | 验收要求 | 当前进度 |
|---|---|---|---|
| L1 | 工具链默认值：cargo 全局落盘位置与调试信息体量固定 | 用户级 `~/.cargo/config.toml` 写 `[build] target-dir` 指向登记槽位；CI 不读该文件 ⇒ CI 行为不变；覆盖优先级可查 | ✅ 已落地（2026-11-17）：`C:\Users\Dream\.cargo\config.toml` 此前不存在，已写入 `[build] target-dir = "C:\Apeireth\cargo-target"`。**未改根 `Cargo.toml`**——理由见 §不变量第 3 条。`[profile.dev] debug = "line-tables-only"` 同文件生效（依据 `docs/archive/merged/2026-10-23/reports/disk-build-cache-advisory-2026-10-16.md` 对 MSVC 1.97.1 的实测记录） |
| L2 | 单入口封装：一条命令同时做磁盘预检、槽位解析与环境卫生 | `scripts/cargo.ps1` 可用；磁盘不足时 **fail-closed**（不跑 cargo）；`RUSTC_WRAPPER=` 置空；退出码透传 | ✅ 已落地（2026-11-17）：`scripts/cargo.ps1`（360 行）。实测 `-CargoArgs "--version"` ⇒ `cargo 1.97.1` ⇒ `cargo-exit=0` |
| L3 | 机械守卫：游离目录、绝对可用量、每日哨兵 | `python scripts/target_slot.py --audit` 检出游离目录时非 0 退出；`scripts/disk_guard.py` 用绝对可用量阈值；`scripts/disk-watch.ps1` 可注册每日计划任务 | ✅ 已落地（2026-11-17）：`target_slot.py`（470 行，审计+棘轮）· `disk_guard.py`（绝对阈值+回收+`--audit-targets`）· `disk-watch.ps1`（285 行，只读哨兵）；计划任务 `Apeireth-DiskWatch` 已注册（每日 03:00）；守卫已接进 `[ci_gates]`（`target-slots`，scope=* 总是执行） |
| L4 | 宪章条款与工作线登记 | `AGENTS.md` 有判定式条款与四层落点；工作线登记表有本行；守卫命令可退出码判定 | ✅ 已落地（2026-11-17）：`AGENTS.md` §5 收口 wave 口径说明、§5.2 Windows 等价与「产物目录不许写死」、新增 §5.4 槽位纪律；工作线登记表已加本行（设计稿 / 进度页 / 守卫命令） |

## 不变量

1. **产物位置是机器保证，不是记忆保证**。任何依赖「代理记得用哪个 target」的规则都算漂移源；
   新规则必须落在工具链默认值、单入口或机械守卫里。
2. **他仓产物只报告不判红**。私有桌面前端仓等非本仓工作树自己长出来的 `target/` 不属本线，
   不替别人的线做决定；他线在途的新目录登记成「在途他线产物」，**只报告不判红但必须带迁移动作**，
   迁进登记槽位后从登记表移除。**已停用**的历史遗留目录适用棘轮（体量上限，**只许缩不许涨**）。
   ⚠️ 区别这两类不是为了让守卫变绿：在途目录按上限判红 ⇒ 守卫会长期红 ⇒ 长期红的守卫等于没有守卫。
3. **不把全局调试信息策略写进仓内 manifest**。`debug = "line-tables-only"` 会失去自有代码的变量级调试，
   历史档案已明确要求「只写进 config.toml 并知会全员」⇒ 本批只落用户级配置，不改根 `Cargo.toml`，
   不影响全体贡献者与 CI。
4. **CI 行为不变**。CI 在容器内自建 target，不读用户级配置；仓内所有 CI 既有 env 口径
   （`CARGO_INCREMENTAL=0` / `CARGO_PROFILE_DEV_DEBUG=0` / `CARGO_PROFILE_TEST_DEBUG=0` /
   `CARGO_PROFILE_*_STRIP=symbols`）维持不动。
5. **守卫失效比红更坏**。脚本里把产物目录写死 `<repo>\target`，一旦目录真实位置改变就会**静默失效**
   （找不到文件却不报错）⇒ 本批把这类硬编码改为「先解析生效目录再拼接」，回退时**显式提示**，
   不许静默。
6. **空间不足时宁可明确说没验**。磁盘硬线以下不跑构建；`harness.py` 的每条门禁前都过磁盘守卫，
   不足即把该门禁记 UNVERIFIED。
7. **LOCKED 项、既有构建语义、既有 CI 口径保持**。本批不改任何 crate 源码、不改构建产物内容、
   不改测试数字。

## 四层落点（判定式）

| 层 | 载体 | 作用 |
|---|---|---|
| L1 工具链默认值 | `~/.cargo/config.toml`（`[build] target-dir`） | 机器上任何 cargo 调用默认落在 base 槽位，不需要人记得 |
| L2 单入口封装 | `scripts/cargo.ps1 -Slot <base\|ui\|tool> [-Line <slug>]` | 磁盘预检（fail-closed）· 槽位解析（`-Line` 分子目录避开共享构建目录的文件锁争用）· 环境卫生（`RUSTC_WRAPPER=` 置空、`CARGO_BUILD_JOBS=2`、`CARGO_INCREMENTAL=0`、`CARGO_PROFILE_TEST_DEBUG=0`、`PYTHONUTF8=1`、`PYTHONIOENCODING=utf-8`）· 退出码透传 |
| L3 哨兵检测 | `scripts/target_slot.py --audit`、`scripts/disk_guard.py`、`scripts/disk-watch.ps1` | 游离目录判红 · 绝对可用量硬线 · 每日哨兵输出游离清单与总量 |
| L4 宪章条款 | `AGENTS.md` §5 / §5.2 / §5.4 | 判定式写清「只允许落在登记槽位」「先登记再长」「产物目录不许写死」 |

## 槽位真源

`docs/architecture/target-slots.json` 是**唯一真源**（登记槽位、扫描根、阈值、历史遗留棘轮、证据保留项）。
解析「生效产物目录」的唯一真源是 `scripts/target_dir.py`，口径三级：

1. `$env:CARGO_TARGET_DIR`
2. `cargo metadata --no-deps --format-version 1 --offline` 的 `target_directory`
3. 回退 `<repo>\target`，**并在 stderr 显式提示**

## 验收命令

```powershell
python scripts/target_dir.py                      # 打印生效产物目录（单行）
python scripts/target_slot.py --audit              # 守卫：游离目录 ⇒ 非 0
python scripts/target_slot.py --list               # 列登记槽位
python scripts/disk_guard.py --ensure 20000        # 绝对可用量硬线（默认 10 GB）
python scripts/disk_guard.py --audit-targets       # 复用 target_slot 审计并沿用退出码
pwsh -NoProfile -File scripts/disk-watch.ps1       # 每日哨兵
```

## 不做

- 不改任何 crate 源码、不改构建产物内容、不改测试数字、不改 CI 工作流。
- 不删他仓产物，不替别人的线决定目录归属。
- 不把「清理磁盘」当成纪律：回收只是补救，纪律是**不产生**游离目录。
- 不在本批引入新的第三方依赖。

## 进度与验收记录

### 2026-11-17 L1/L3/L4 落地与反向验伪

- **审计 RED**（真实输出）：`python scripts/target_slot.py --audit` 报
  「❌ 游离（未登记）：`C:\Apeireth\wt\backend-hardening-completion-20261113\target`（0.0G）」⇒ `AUDIT_EXIT=1`。
- **处置**：核实该目录只含 4 个文件（`.rustc_info.json` 1388 B + `nextest\ci\junit.xml` 2 018 524 B），
  仓内无任何文档引用其绝对路径 ⇒ 删除该空壳目录（已核对解析后的绝对路径在 `C:\Apeireth\wt\` 之下）。
- **审计 GREEN**（真实输出）：生效产物目录 = `C:\Apeireth\cargo-target`（来源：cargo metadata）· 归属槽位 base；
  已登记 cargo-target(4.6G)、cargo-target-ui-desk(10.09G)；历史遗留四项均未超上限
  （c13-model 0.41/1G、c23 3.93/4G、c4-ui 1.49/2G、check 0.19/1G）；证据保留 ui-desk2 2.5G；
  他仓 `C:\Apeireth\wt\ui-desktop-private\target` 6.01G 只报告；游离总量 0G ⇒ `AUDIT_EXIT=0`。
- **L1 生效证据**：写入用户级配置**前**，`cargo metadata` 解析出的是本工作树自建的 `target`（漂移源）；
  写入**后**审计显示生效目录为 `C:\Apeireth\cargo-target` ⇒ 全局 `target-dir` 确实生效。
- **回收路径修复**：`scripts/disk_guard.py` 旧版 `reclaim()` 调 `clean_test_binaries.py` **不传 `--target`**，
  而该脚本的候选根是 Linux 口径（`/home/bai`、`/var/tmp`、`/tmp`）⇒ 在 Windows 上**回收从来没真发生过**；
  现改为按解析出的生效目录与登记槽位拼 `--target` 传入。
- **守卫抓到的真实漂移（2026-11-17 中午）**：第二轮审计时 `cargo-target-c23` 已由 3.93 G 涨到 5.03 G，
  超出登记上限 4 G ⇒ 审计判红。这暴露了「棘轮」语义用错了对象：那是**他线在途**产物（仍在写入），
  不是已停用的历史遗留 ⇒ 登记表新增 `active` 字段（**只报告不判红**，但必须写明迁移动作），
  `target_slot.py` 据此区分两类并改用不同标记（🔵 在途他线产物 / 🟡 历史遗留 / 🔴 超上限）。
  修正后审计 GREEN（`AUDIT_EXIT=0`），且 c23 的迁移债登记在[进度页](../status/build-artifact-discipline-2026-11-17.md)。
- **本批改动清单**（工作树 `chore/build-artifact-discipline-20261117`）：
  `AGENTS.md` · `scripts/target_dir.py`（新建）· `scripts/target_slot.py`（新建）·
  `scripts/cargo.ps1`（新建）· `scripts/disk-watch.ps1`（新建）·
  `scripts/disk_guard.py` · `scripts/check_stale_binaries.py` · `scripts/measure-session-startup.ps1` ·
  `scripts/target-hygiene.ps1` · `scripts/target-sweep.ps1` · `scripts/test-installed-e2e.ps1` ·
  `scripts/verify-provider-path.ps1` · `scripts/windows-live-check.ps1` ·
  `scripts/gen_gate_manifest.py`（修 LF 翻写缺陷）· `docs/architecture/contract.toml`（新增一条门禁）·
  `docs/architecture/target-slots.json`（新建）· `docs/design/build-artifact-discipline-2026-11-17.md`（本文件）·
  `docs/status/build-artifact-discipline-2026-11-17.md`（进度页）· `docs/status/workline-registry.md` ·
  `docs/INDEX.csv` · `scripts/gate-manifest.json` · `.github/PULL_REQUEST_TEMPLATE.md`。

### 2026-11-17 下午：守卫接进门禁链 + 三个真实缺陷

- **守卫进 `[ci_gates]`**：`docs/architecture/contract.toml` 新增
  `[[ci_gates.gate]] id = "target-slots"`（`cmd = ["python3","scripts/target_slot.py","--audit"]`、
  `stage = "fast"`、`scope = ["*"]`、`blocking = true`、`required = true`）
  ⇒ 门禁总数 44 → **45**，required 42 → **43**；`scripts/gen_gate_manifest.py --gen` 后 `--check` 通过；
  `scripts/harness.py plan` 输出 `target-slots | yes | * | yes — scope=* 总是执行`
  ⇒ 守卫从「靠人记得跑」变成「每轮门禁前自动跑」。
  ⚠️ 该登记表是**本机布局** ⇒ CI（Linux 容器）上没有槽位根 ⇒ 审计**跳过并退出 0** ⇒ **CI 行为不变**。
- **缺陷一：生成器在 Windows 上把 LF 翻成 CRLF**：`scripts/gen_gate_manifest.py` 用 `Path.write_text` 回写
  PR 模板与门禁清单 ⇒ Windows 上整文件被重写 ⇒ git diff 变成 172 行 / 751 行的**纯空白噪声**
  ⇒ 已改为 `open(..., newline="\n")` 强制 LF ⇒ 重生成后 diff 收敛为 4 行 + 16 行。
- **缺陷二：在途他线目录按体量上限判红** ⇒ 守卫长期红 ⇒ 等于没有守卫 ⇒ 登记表加 `active` 字段区分
  「在途他线产物（只报告）」与「已停用历史遗留（只许缩不许涨）」。
- **缺陷三：回收在 Windows 上从来没真发生过**（`reclaim()` 不传 `--target` ⇒ 候选根是 Linux 口径 ⇒ 空转）
  ⇒ 已重写为按解析出的生效目录与登记槽位拼 `--target`。
- **缺陷四（CI 例外）**：`ensure_disk = max(args.ensure, DISK_HARD_FREE_MB)` 会让 harness 的 `--ensure 2000`
  在托管 runner 上变成 10 GB 硬线 ⇒ 空间一低 ⇒ 每条门禁全变 UNVERIFIED
  ⇒ 已加 CI 例外：`CI` 非空且用户**未**显式给值 ⇒ 不套用本机底线；显式给值 ⇒ 以用户值为准（不许自我豁免）
  ⇒ 三种情形均实测（EXIT=0 / EXIT=0 / EXIT=1）。

### 还差什么

见[进度页](../status/build-artifact-discipline-2026-11-17.md)。
