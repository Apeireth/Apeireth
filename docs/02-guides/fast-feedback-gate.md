# 快速反馈门禁（fast feedback gate）

> `Status: ✅ 已交付（2026-10-20，效率 P0）`
> 目的：AI 高速迭代下，每批只验证"改动 crate + 反向依赖"；全量门禁留给 wave 收口/夜间。
> 工具：`scripts/verify_changed.py`、`scripts/commit.ps1`、仓库现有 `.githooks`。

## 1. 变更影响面验证

```powershell
# 只看工作树改动会影响哪些 crate（不跑命令）
python scripts/verify_changed.py --dry-run

# 跑 fast gate：cargo check + clippy(-D warnings) + nextest（仅受影响 crate）
python scripts/verify_changed.py

# 只做编译检查（更快的反馈）
python scripts/verify_changed.py --no-clippy --no-test

# 按 base..HEAD 的 diff 验证（PR/收口前）
python scripts/verify_changed.py --base origin/main

# 显式指定改动文件（测试/手动）
python scripts/verify_changed.py --files crates/adapters/tui/src/lib.rs
```

口径：

- `.rs` 改动 → 最近 `Cargo.toml` 所属 crate；`Cargo.toml` / `Cargo.lock` 改动 → 全 workspace。
- 受影响 = 改动 crate + **传递反向依赖**（依赖方的测试可能暴露接口破坏）。
- test 阶段优先 `cargo nextest run`（per-test 超时/重试/并行），没有 nextest 时回退 `cargo test`。
- 输出全量落盘 `logs/verify-changed.log`（logs/ 已被 .gitignore 忽略），**判定基于落盘退出码**，不做管道直判。
- 退出码 0 = 全部通过；非 0 = 失败（首因阶段写在末行，详细见日志）。

## 2. 提交收口入口

```powershell
pwsh scripts/commit.ps1 -MessageFile $env:TEMP\msg.txt
pwsh scripts/commit.ps1 -MessageFile msg.txt -AuthorName Mavis -AuthorEmail Mavis@apeireth.local
```

它做三件事：

1. 显式设置 `GIT_AUTHOR_*` / `GIT_COMMITTER_*`（默认取 `git config user.*`，避免 pre-commit 的 author 检查误伤）；
2. 对 staged 改动跑 `secret-scan -Mode scan-staged` + `check-neutral-terms`；
3. `git -c core.hooksPath=.githooks commit` —— **按次启用**仓库 hooks（commit-msg 查 O-6 三段；pre-commit 查 author / LOCKED 警告）。

> 为什么不是全局 `git config core.hooksPath .githooks`：本工作树同时有并行 AI 线，
> 全局启用会让其他线的 commit 也被 pre-commit 的 author 检查拦住（需各自显式设 env）。
> 按次启用只影响本线提交；待 owner/各线协调后再决定是否全局化（一条命令的事）。

## 3. 与全量门禁的关系

| 阶段 | 命令 | 何时 |
|---|---|---|
| fast | `scripts/verify_changed.py` | 每批提交前 |
| full | `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` 等三件套 | wave 收口 / 夜间 / 发布前 |
| 文档 | `check-neutral-terms` / `secret-scan` / `check_doc_caliber` / `docs_truth.py --verify --check` | 改文档的批 |

全量数字仍是单一真源 [test-baseline.md](../../03-reference/test-baseline.md)；fast gate 不产生对外数字。

## 4. 已知边界（0 装）

- fast gate 只证明"受影响 crate 的现有测试通过"，**不证明**全 workspace 无回归（跨 crate 的 E2E/端口类测试仍靠全量）。
- 共享 target 目录时可能遇到 `Blocking waiting for file lock on build directory`（他线并行构建占用）；fast gate 不掩盖，锁等待原样落日志。缓解：等他线构建空窗再跑，或（P1）给本线独立 `CARGO_TARGET_DIR` / worktree。
- D26 类环境 flake：nextest 的 retry 可缓解，但根因（Windows 保留端口区间）仍需环境侧处置；fast gate 不掩盖——失败原样落盘。
- `--base` 用 `base...HEAD`（merge-base 语义）；跨 rebase 后请重跑。
