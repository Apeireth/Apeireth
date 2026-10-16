# 磁盘与构建缓存治理建议书 — 交叉验证验收报告 — 2026-10-16

- **日期**: 2026-10-16 18:25-18:40（CST, +08:00）
- **验收方**: DSH 会话（受主人委托，独立交叉验证）
- **验收对象**: `reports/disk-build-cache-advisory-2026-10-16.md` §5 清单 + 主工程师整改工作集（未提交态）
- **工具链**: cargo 1.97.1 (c980f4866 2026-06-30) / rustc 1.97.1 (8bab26f4f 2026-07-14)，`-j 2` 口径
- **结论**: **通过（复验收口）** —— 初验 5✅/1⚠️/1❌，R1/R2 整改当日复验关闭（§4），终态 **7 项全通过**

---

## 1. 验收总表

| # | 验收项 | 结论 | 关键证据 |
|---|---|---|---|
| 1 | 依赖调试信息关闭（P0-1） | ✅ 通过 | 工作区 9 重编单元全 `-C debuginfo=2`；依赖侧对照实验 `-C strip=debuginfo` |
| 2 | 构建/测试不回退 | ✅ 通过 | BUILD/TEST 双 exit 0；**5392 passed / 0 failed**；fmt 0 diff；clippy `-D warnings` 0 警告 |
| 3 | 默认工具链健康（P0-2） | ✅ 通过 | 项目外 `cargo new && cargo build` 成功；rustc 1.97.1 正常响应 |
| 4 | 清理脚本落地（P1-1） | ✅ 通过（复验） | 双脚本 + 5 步实测链全过；初验缺口 pdb 档经 R1 整改复验关闭（§4） |
| 5 | worktree 台账（P1-2） | ✅ 通过 | 台账建立、回收条件正确、分支未合并故不 remove（正确） |
| 6 | companion-desktop 构成表+清理（P1-3） | ✅ 通过（复验） | R2 整改：构成表入 maintenance-guide + 脚本纳管 + 构建可恢复（§4） |
| 7 | target 体积趋势 | ✅ 通过 | 186.7GB 基线 → 实测 147.1GB；C 盘 free 176.9GB；阈值机制就位 |

## 2. 逐项证据（每数字可复核：命令 + 日期 + 工具链）

### #1 依赖调试信息关闭 ✅

- **配置落点（方案 A）**：`Cargo.toml:219-220` `[profile.dev.package."*"] debug = 0`
- **真实构建实测**（`cargo build --workspace --all-targets --locked -j 2 -v`，2026-10-16 17:5x）：
  本轮 9 个重编单元**全部为工作区 crate**（apeireth_tui / apeireth_bench_harness / 6 个 example+test bin），全部 `-C debuginfo=2`——自有代码完整调试保住。
  （注：曾按 `-L native=...\.cargo\registry\...` 误分类出 3 个"registry 单元"，复核原始行确认为 windows_x86_64_msvc 原生库路径，非依赖编译。）
- **依赖侧语义（对照实验）**：临时工程同款 `[profile.dev.package."*"] debug = 0` + 1 个 registry 依赖，`cargo build -v` 实测：依赖 rustc 行**无 `-C debuginfo`** 且带 `-C strip=debuginfo`（调试信息剥离），本地 crate `-C debuginfo=2`。语义正确。
- **体积实证**：配置生效后 target 186.7GB → 154.5GB（依赖调试信息剥离 + 主工程师清理），与预期方向一致。

### #2 构建/测试不回退 ✅

- `cargo build --workspace --all-targets --locked -j 2` → **exit 0**
- `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` → **exit 0**，`test result: ok` 汇总 **5392 passed / 0 failed**（逐条扫描 "N failed" 非零行 = 0）
- `cargo fmt --check` → exit 0（0 diff）
- `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` → **exit 0**（0 警告）

### #3 默认工具链健康 ✅

- `rustup toolchain list` → `1.97.1-x86_64-pc-windows-msvc (active, default)`（损坏的 nightly 不再是默认）
- 项目外 `%TEMP%`：`cargo new ext-smoke --bin && cargo build` → `Finished dev profile`（此前该路径直接报 `rustc.exe ... not applicable`）
- 项目外 `rustc -vV` → 1.97.1 (8bab26f4f 2026-07-14)

### #4 清理脚本落地 ⚠️ 有条件通过

**合格面（5 步实测链 2026-10-16 18:25-18:3x）**：

| 步 | 动作 | 实测结果 |
|---|---|---|
| 1 | `scripts\target-hygiene.ps1`（报告模式） | 体积分布 + 3 清理候选，robocopy /L 快速；铁边界（不碰 deps/build/.fingerprint）表述清楚 |
| 2 | `scripts\target-sweep.ps1 -DryRun` | 准确预报"将删 incremental 10.2GB" |
| 3 | `scripts\target-sweep.ps1`（真删） | **回收 10.2GB**（157.3 → 147.1GB） |
| 4 | `cargo check --workspace --all-targets --locked -j 2` 冒烟 | **exit 0**，1m07s（清理后管线完好，无全量重编） |
| 5 | `scripts\target-sweep.ps1` 复跑（幂等） | 无报错，仅回收 check 新建的 1.3GB incremental |

**缺口（整改项 R1）**：两脚本均不覆盖 stale **`*.pdb`**。实测 `target/*.pdb` = **1418 个 / 113.9GB**（占 target 74%），恰是 `live-verification-ledger.md` #40（2026-10-06）实证过的头号元凶（当时 1768 个/108.4GB；删除后 `cargo test --workspace --no-run` **Compiling 次数 = 0** 零重编、40s 恢复，已证安全可再生）。sweep 当前口径只回收 10.2GB，而 pdb 档是百 GB 级。

> **建议书自身纠偏（交叉验证价值）**：建议书 P1-1 按 8/20 先例开的"只删 incremental/doc"口径**低估了真元凶**——8/20 先例早于 10/06 的 PDB 发现。整改 R1 = sweep/hygiene 增补 pdb 清理档（复用台账 #40 已验证语义：可随时由 cargo 重建，`RUST_BACKTRACE=1` 符号化触发重链）。

### #5 worktree 台账 ✅

- 台账落地：`docs/04-internal/maintenance-guide.md` §worktree 台账（标注"2026-10-16 建，建议书 P1-2"）：worktree/分支/用途/回收条件四列齐全 + 磁盘纪律（50GB 阈值、引 `target-sweep.ps1`、引先例）
- 对账：`git worktree list` = 2 条（主仓 main + apeireth-debt fix/historical-debt），与台账一致
- 回收时机正确：`rev-list --left-right --count origin/main...origin/fix/historical-debt` = `4 / 11`（未合并）→ 不 remove = **正确执行**

### #6 companion-desktop 构成表 + 清理 ❌ 不通过（整改项 R2）

- reports/ 无新增交付报告；docs 无构成表（`grep companion-desktop.*(GB|体积)` 仅命中无关 CI 行）
- 实测构成（2026-10-16 18:2x）：`frontend\companion-desktop` = **18.4GB**，其中 `src-tauri` = **18.04GB（99.9%）**= 桌面壳独立 `src-tauri/target` 构建缓存（node_modules 仅 0.14GB）
- 台账 #40 记录该目录 2026-10-06 清过 14.9GB，如今回长到 18.4GB，且**两个治理脚本只作用于主 `target/`，不覆盖它**
- 整改 R2 = 附构成表 + 清理（可复用 sweep 口径）；建议一并纳入脚本覆盖范围

### #7 target 体积趋势 ✅

- 建议书基线 186.7GB（2026-10-16 上午）→ 验收实测 **147.1GB**（18:3x，sweep 后）
- C 盘 free：147.5GB（应急清理后）→ **176.9GB**（本轮验收收尾）
- 增长管控机制（50GB 阈值 + sweep/hygiene 双脚本 + 台账纪律）已就位；长效仍取决于整改 R1（pdb 档）

## 3. 整改项与复验口径

| 编号 | 整改项 | 复验口径 |
|---|---|---|
| R1 | sweep/hygiene 增补 stale `*.pdb` 清理档（台账 #40 已验证语义） | 跑 sweep 后 `cargo check` 冒烟 exit 0 + 回收量与 pdb 体量同数量级；重跑幂等 |
| R2 | companion-desktop 构成表 + src-tauri/target 纳管清理 | 构成表入 reports/ 或 maintenance-guide；清理后构建可恢复；脚本覆盖该目录 |

整改完成后知会，本会话对 R1/R2 复验并更新本报告结论。

**→ R1/R2 已于 2026-10-16 晚复验关闭，见 §4。**

---

## 4. 复验记录（R1/R2 收口，2026-10-16 晚）

### R1 sweep v2 增补 pdb 档 ✅ 关闭

- **整改件**：`scripts/target-sweep.ps1` v2（头注明标"验收整改 R1/R2"）——口径扩为 incremental/doc/**pdb** 双 target 同档（主 `target/` + `src-tauri/target`），保留 deps/build/.fingerprint，与台账 #40 语义一致；`maintenance-guide.md` §磁盘纪律同步更新
- **复验证据**（独立实测，不采信自报数字）：
  - pdb 存量：初验实测 1418 个 / 113.9GB → 复测 **0 个 / 0GB**（src-tauri 同为 0）
  - 主 `target/` 147.1GB → **33.22GB**；C 盘 free 176.9GB → **294.7GB**（回收量与 pdb 体量同数量级 ✅）
  - 清理后管线完好：`cargo check --workspace --all-targets --locked -j 2` **exit 0，0.53s**（零重编）✅
  - 幂等：sweep 连跑两次无害（第二次仅回收 check 新建的 0.1GB incremental）✅
  - 阈值机制：合计 45.3GB < 50GB 自动跳过（`-All`/`-Force` 可强制）✅

### R2 companion-desktop 纳管 ✅ 关闭（1 非阻塞备注）

- **构成表**：`docs/04-internal/maintenance-guide.md` §companion-desktop 体积构成表（标注"验收整改 R2"）✅
- **脚本纳管**：sweep v2 将 `src-tauri/target` 与主 target 同档处理 ✅
- **清理后构建可恢复**：`frontend/companion-desktop/src-tauri` 下 `cargo check --all-targets` **exit 0，3.74s** ✅
- **备注（非阻塞）**：构成表"18.0GB → 4.5GB"是 sweep 后即时值；复验实测 **12.08GB**（debug 9.01 + release 1.82 + x86_64-pc-windows-msvc 1.26）——构建验证后产物回长所致。建议数字标时点（0 装纪律"每个数字可复核"口径）。

### 终态

**7 项验收全部通过，R1/R2 关闭。** 长效保持建议：sweep v2 例行化（并入 `make` 目标或月度维护清单），target 合计 >50GB 阈值已能自动提示。

---

## 附录 A：证据复现命令

```powershell
# #1 调试信息（真实构建 + 对照实验）
cargo build --workspace --all-targets --locked -j 2 -v | Select-String 'Running `.*rustc'
# 对照实验: 临时工程 [profile.dev.package."*"] debug = 0 + 1 registry 依赖, cargo build -v

# #2 测试
cargo test --workspace --all-targets --locked --no-fail-fast -j 2   # 5392 passed / 0 failed
cargo fmt --check                                                     # 0 diff
cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings  # 0 警告

# #3 工具链
rustup toolchain list; Push-Location $env:TEMP; cargo new ext-smoke --bin; cd ext-smoke; cargo build

# #4 脚本链
& scripts\target-hygiene.ps1; & scripts\target-sweep.ps1 -DryRun; & scripts\target-sweep.ps1
cargo check --workspace --all-targets --locked -j 2; & scripts\target-sweep.ps1   # 幂等

# #5 worktree
git worktree list
git rev-list --left-right --count origin/main...origin/fix/historical-debt

# #6/#7 体积
Get-ChildItem target -Recurse -Filter *.pdb | Measure-Object Length -Sum
Get-ChildItem frontend\companion-desktop -Recurse -File | Measure-Object Length -Sum
Get-PSDrive C
```

## 附录 B：本轮验收产生的记录

- 测试日志：`%TEMP%\accept-test.log` / 构建日志：`%TEMP%\accept-build-v.log` / clippy：`%TEMP%\accept-clippy.log`（一次性证据，未入库）
- 本报告：`reports/disk-build-cache-advisory-acceptance-2026-10-16.md`
- 被验收建议书：`reports/disk-build-cache-advisory-2026-10-16.md`
