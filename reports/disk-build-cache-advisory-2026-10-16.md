# 磁盘与构建缓存治理建议书 — 2026-10-16

- **日期**: 2026-10-16
- **提出方**: DSH 会话（受主人委托做 C 盘磁盘压力排查）
- **交付对象**: 项目主工程师
- **状态**: 待主工程师处理
- **验收方式**: 处理完成后由本会话做交叉验证检查验收（验收清单见 §5）
- **先例参照**: `reports/target-cleanup-2026-08-20.md`（上一轮 target 手术式清理）

---

## 0. TL;DR

C 盘 952.9GB 曾仅剩 71.8GB。已做应急清理（约 145GB），现余 **147.5GB**。
遗留 5 个工程问题待处理，其中两个 P0：

1. **构建调试信息策略缺失** → `target/` 膨胀到 186.7GB（详见 §4 P0-1）
2. **rustup 默认 nightly 工具链损坏** → 项目外任何 Rust 编译直接报错（详见 §4 P0-2）

---

## 1. 背景

主人反馈"电脑没内存了"。排查结论：RAM 32GB 仅用 53%，**问题在 C 盘磁盘**（952.9GB 已用 881GB）。
大头是 Rust `target/` 编译缓存与若干个人大文件（游戏/本地模型）。本建议书聚焦**工程侧可根治项**。

## 2. 现状证据（2026-10-16 实测）

| 项 | 大小 | 说明 |
|---|---|---|
| `Apeireth-rust\target` | **186.7 GB** | 全盘最大可治理项；8/20 清理后至今重新膨胀 |
| `apeireth-debt\target` | 53.3 GB → **已删** | 见 §4 P1-2，worktree 分支缓存 |
| `frontend\companion-desktop` | 18.0 GB | 10/15 实测 3.11GB → 10/16 涨到 18GB |
| C 盘剩余 | 71.8 GB → **147.5 GB** | 应急清理后 |

**膨胀元凶（先例 + 实测交叉印证）**：
- `target/debug/incremental/`：增量编译中间文件。`target-cleanup-2026-08-20.md` 实测占 64.94GB / 77.32GB（84%）
- 依赖 crate 的完整调试信息（debug info 通常是产物体积的大头）
- 注：`strip = true` 只作用于 `[profile.release]`，dev/test 构建产物无此保护

## 3. 本会话已执行的应急清理（供对账，无需复核）

| 动作 | 释放 | 副作用 |
|---|---|---|
| 删除 `Apeireth-rust\target`（全量） | ~84 GB | 下次 cargo build 全量重编译 |
| 清空回收站 | ~12.5 GB | 不可恢复 |
| 删除 `apeireth-debt\target`（全量） | ~48 GB | 同上；源码（`fix/historical-debt` 分支）完好 |

## 4. 待办建议

### P0-1 构建调试信息策略落地（target 治理核心）

**问题**：dev/test 构建为所有第三方依赖生成完整调试信息 + incremental 缓存无上限增长，`target/` 已达 186.7GB。

**实测证据（本会话做过的对照实验）**：
- 临时项目验证：`debug` 同时出现在 `.cargo/config.toml` 与 `Cargo.toml` 时，**config 优先级更高**（最终 rustc 收到 `-C debuginfo=line-tables-only` 而非 manifest 的 `debug = 2`）
- `debug = "line-tables-only"` 在 MSVC 工具链（1.97.1）正常工作
- `incremental = true` **已是 dev profile 默认值**（编译命令已带 `-C incremental=...`），无需显式配置

**建议（按优先序）**：

1. **首选（副作用最小）**：去掉第三方依赖的调试信息，自己的代码保留完整调试：

   ```toml
   # 方案 A：落 workspace Cargo.toml（推荐 —— 全队 + CI 一致）
   [profile.dev.package."*"]
   debug = 0

   # 方案 B：落 C:\Users\31683\.cargo\config.toml（仅本机生效）
   ```

   效果：断点/变量/单步在自有代码中完全不受影响；只有调试进依赖库内部时看不到依赖的变量（少见场景）。target 体积预计大幅下降，编译写盘量也减少。

2. **慎用全局 `debug = "line-tables-only"`**：能进一步压缩，但会失去自有代码的变量级调试。若要用，建议只写进 `config.toml` 并知会全员（注意 §实测证据：config 会压过项目 Cargo.toml，想临时开满调试需一并改 config 或用 `CARGO_PROFILE_DEV_DEBUG=2` 环境变量）。

3. **不要写 `incremental = true`**：默认值，写了无效；若目标是省磁盘，反向考虑 `incremental = false`（代价：改一行整 crate 重编译，不推荐）或按 P1-1 定期手术清理。

**风险**：方案 A 使依赖库内部调试缺少变量信息；对 `cargo-llvm-cov`（source-based 覆盖率）无影响。
**验收标准**：
- `cargo build -v` 中，工作区 crate 收到 `-C debuginfo=2`，第三方依赖收到 `-C debuginfo=0`
- 全量 `cargo build` + `cargo test` 通过
- （观察期）连续开发一周后 `target/` 体积显著低于历史同期增速

### P0-2 rustup 默认 nightly 工具链损坏

**问题**：`rustup toolchain list` 显示默认是 `nightly-x86_64-pc-windows-msvc`，其 rustc 已损坏：

```
error: the 'rustc.exe' binary, normally provided by the 'rustc' component,
is not applicable to the 'nightly-x86_64-pc-windows-msvc' toolchain
```

目前被 `rust-toolchain.toml`（pin 1.97.1）兜底，**项目内无感；项目外任何 cargo/rustc 调用直接失败**（rustup 代理会选默认工具链）。

**建议**：`rustup default 1.97.1`（或 `rustup toolchain uninstall nightly` / 重装 nightly），并确认 `nightly-2026-07-14` 是否还有存在必要。
**验收标准**：
- `rustup toolchain list` 默认指向可用工具链
- 项目外任意目录 `cargo new tmp && cargo build` 成功
- `rustc -vV` 在任意目录返回版本信息而非报错

### P1-1 target 手术式清理策略（代替整目录删除）

**问题**：本轮应急清理是整目录删除（全量重编译成本高）。先例 `target-cleanup-2026-08-20.md` 已证明手术式更优：只删 `target/debug/incremental/`（64.94GB）+ `target/doc/`，保留 `deps/ build/ examples/ release/`，回收 63.67GB（82%）且 `cargo check` 立即通过。

**建议**：把手术式清理固化为脚本（`scripts/` 或 Makefile 目标），含体积阈值告警（建议 `target/` > 50GB 触发提示）。
**验收标准**：
- 脚本/Make 目标存在且幂等
- 执行后 `cargo check -p apeireth-companion --example companion_serve`（或等价冒烟）通过
- 附前后对比记录（对齐先例报告格式）

### P1-2 apeireth-debt worktree 生命周期管理

**问题**：`C:\Users\31683\apeireth-debt` 是主仓库的 **git worktree**（`.git` 指向 `Apeireth-rust/.git/worktrees/apeireth-debt`），检出 `fix/historical-debt` 分支。实测 `origin/main...origin/fix/historical-debt` = **4 / 11**（该分支 11 个提交未合并进 main，已推远端）。其 `target/` 53.3GB 已删，源码 ~200MB 必须保留。仓库曾有 worktree 困扰先例（`round12-14-push-v28-1-stage6-22-trait-interlock-stuck-in-worktree-architect2.md`）。

**建议**：
1. PR 合并进 main 后执行 `git worktree remove C:\Users\31683\apeireth-debt`（勿直接删目录，会留悬挂 worktree 元数据）
2. 在维护文档登记 worktree 台账（分支/用途/回收条件）
**验收标准**：
- `git worktree list` 与台账一致；合并后列表无残留
- `git branch -r --merged origin/main` 包含 `origin/fix/historical-debt` 后才允许 remove

### P1-3 frontend/companion-desktop 体积构成查明（18GB）

**问题**：10/15 → 10/16 从 3.11GB 涨到 18.0GB（疑为 node_modules / src-tauri/target / dist 产物堆积）。
**建议**：查明构成清单后对症清理（node_modules 可删可 `npm install` 恢复；src-tauri/target 同 P1-1 策略）。
**验收标准**：附构成表（各子目录大小）+ 清理后 `npm install` / 构建可恢复

## 5. 交叉验证验收清单（本会话执行）

| # | 验收项 | 命令 | 通过标准 |
|---|---|---|---|
| 1 | 依赖调试信息关闭 | `cargo build -v`（新 target 状态） | 依赖 `-C debuginfo=0`，工作区 crate `-C debuginfo=2` |
| 2 | 构建/测试不回退 | `cargo build && cargo test --workspace` | 全绿 |
| 3 | 默认工具链健康 | 项目外 `cargo new tmp && cargo build` | 成功，无 not applicable 报错 |
| 4 | 清理脚本落地 | 脚本执行 + `cargo check` 冒烟 | 幂等、冒烟通过、有前后对比 |
| 5 | worktree 对账 | `git worktree list` + `git branch -r --merged origin/main` | 台账一致，合并后 remove 无悬挂 |
| 6 | companion-desktop 构成 | 目录体积清单 | 有构成表，清理可恢复 |
| 7 | target 体积趋势 | `target/` 大小 + C 盘 free | 低于本轮 186.7GB 基线，增速受控 |

## 6. 附录：证据复现命令

```powershell
# 盘面
Get-PSDrive C | Select-Object @{n='Free(GB)';e={[math]::Round($_.Free/1GB,1)}}

# target 体积
Get-ChildItem C:\Users\31683\Apeireth-rust\target -Recurse -File -Force -EA SilentlyContinue |
  Measure-Object -Sum Length

# 调试信息实测（对照实验）
# 临时项目 Cargo.toml 设 [profile.dev] debug = 2，
# .cargo/config.toml 设 [profile.dev] debug = "line-tables-only"，
# cargo build -v 观察 -C debuginfo= 值 → 实测 config 胜出

# 工具链
rustup toolchain list; rustc -vV   # 项目外执行

# worktree
git -C C:\Users\31683\Apeireth-rust worktree list
git -C C:\Users\31683\Apeireth-rust rev-list --left-right --count origin/main...origin/fix/historical-debt
```

---

**处理完成后请知会，本会话按 §5 清单逐项交叉验证并出具验收结论。**
