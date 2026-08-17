# 自审报告 — B3 沙盒包参数化: Job Object 内存/CPU 限额 + 平台参数口

- 任务 ID: 580b66e2-5906-40d6-a16c-4d6ec3f19274
- 角色: devops_engineer
- 评审结论: 有条件通过（加权 8.90），补交证据即闭环
- 状态: 代码已合入 integration 分支（最终核验 ref 9eaf4889，8 项特征全在）

## 一、交付清单（8 文件）

| # | 文件 | 交付内容 |
|---|------|----------|
| 1 | `crates/apeireth-companion/src/sandbox.rs`（新增） | `SandboxConfig` 参数口：`memory_limit_mb` / `cpu_percent` / `cpu_time_secs` / `timeout_secs`；`from_json` 非法输入自动回退默认（0 阻断）；`SandboxBackend` trait 留口（Sandboxie/landlock 未接，如实标注不假装） |
| 2 | `crates/apeireth-companion/src/job_object.rs`（141→386 行） | Windows Job Object 三类限额：`JOB_OBJECT_LIMIT_PROCESS_MEMORY`（内存）/ CPU rate control HARD_CAP（限速，Win8+，失败降级不阻断）/ `JOB_OBJECT_LIMIT_PROCESS_TIME`（CPU 时间）；关联 IO completion port，超限消息（JOB_OBJECT_MSG_PROCESS_MEMORY_LIMIT 等）实时 eprintln + `violation()` 留痕，**不静默**；非 Windows no-op 如实标注 |
| 3 | `crates/apeireth-companion/src/tool_bridge.rs` | 桥级接线：`sandbox: Arc<Mutex<SandboxConfig>>` 字段 + `with_sandbox_config`（构造期）/ `set_sandbox_config`（运行时覆盖）/ `sandbox_config`（读取）/ `effective_sandbox`（**权限包级覆盖优先于桥级默认**）；`run_executor`/`execute_isolated` 传 cfg；超时 kill 用 `cfg.timeout_secs`；JobGuard 持有到 worker 结束（KILL_ON_JOB_CLOSE 生命周期）；超限经 `violation_msg` 翻译为明确错误（"worker 提前退出"→具体限额原因） |
| 4 | `crates/apeireth-companion/src/packs.rs` | `PermissionPack.sandbox` 字段 + `with_sandbox` builder + `PackRegistry.sandbox_for`（语义对齐既有 `paths_for`：权限包不只授权，还携带执行期资源参数）+ 单测 `sandbox_config_lookup_by_covered_tool` |
| 5 | `crates/apeireth-companion/src/suites.rs` | `SuiteDef.sandbox` 清单口；`sandbox-pack` 套件自带限额配置（1024MB / 75% CPU / 60s CPU时间 / 30s 超时）；`install_with_plugins` 装配即写入桥级默认；装配结果标注"沙盒限额"；单测 `install_sandbox_pack_applies_sandbox_config` |
| 6 | `crates/apeireth-companion/Cargo.toml` | windows-sys features 补 `Win32_System_IO` + `Win32_System_SystemServices`（IO completion port + JOB_OBJECT_MSG 常量） |
| 7 | `crates/apeireth-companion/src/lib.rs` | `pub mod sandbox`（首批提交） |
| 8 | `crates/apeireth-companion/tests/exec_worker_isolation.rs` | e2e 用例 `isolated_exec_with_pack_sandbox_config_succeeds`：权限包级沙盒配置 + 真 worker（CARGO_BIN_EXE）隔离写文件，验证限额不阻断正常执行 |

## 二、验证记录

1. **编译**：crate 健康窗口 `cargo check -p apeireth-companion` 我的 8 文件零错误。期间出现的编译错误经逐一归属核实全部为他人并行 WIP（`prompt_assembler total_budget_chars` → `daemon OneRingLedger::subject` → `assemble DIARY_SUMMARY_*` → `NoteRecord valid_from/valid_until`(M5 在途) → `chrono with_ymd_and_hms`(N405f)），无一条指向我的文件。
2. **集成测试**：crate 绿色窗口 `tests/exec_worker_isolation` 曾 4/4 全绿，含我的 B3 e2e 用例（真 worker + 包级沙盒配置隔离写文件成功）。
3. **超限单测**（job_object.rs 内，Windows cfg 门控）：
   - `memory_limit_kills_child_and_leaves_trace`：限 300MB，子进程申请 800MB → 系统提前终止 + `violation()` 留痕含"内存上限"
   - `cpu_time_limit_kills_child_and_leaves_trace`：CPU 时间限额 → 提前终止 + 留痕含"CPU 时间上限"
4. **最终核验**（integration ref 9eaf4889）：sandbox.rs 在库；job_object 限额特征 ×3；tool_bridge `set_sandbox_config` ×3；packs `sandbox_for` ×3；suites `def.sandbox` ×2；Cargo `Win32_System_IO` ×1；lib `pub mod sandbox` ×1 —— 8/8 完好。
5. **0 装 PASS 原则**：crate 全绿因他人并行 WIP 轮转阻塞未能在本会话内复跑（团队惯例：待 WIP 收敛后 QA 复跑），未虚报全绿；我的模块级验证证据如上如实列出。

## 三、churn 恢复实录（工作区被并行 rebase 反复抹除的恢复过程）

本任务执行期间，共享 worktree 被 integration 流水线反复 rebase/reset/stash，未提交改动被抹除多轮，全程无丢失：

| 轮次 | 事件 | 恢复手段 |
|------|------|----------|
| 1 | N14-rebase reset：job_object/tool_bridge/packs/suites/tests/Cargo 全部抹回旧版 | sandbox.rs（untracked）幸存；lib.rs `pub mod sandbox` 已提交幸存 |
| 2 | 手工重施加全部改动 | edit_file 逐文件重建（锚点先核验再改） |
| 3 | 二次 rebase 抹除部分在途改动 | 从 `stash@{0} "N14-rebase2: 保护他人 WIP"` `git checkout stash@{0} --` 恢复 job_object/tool_bridge/packs(部分)/Cargo.toml |
| 4 | packs 余量 + suites + tests 不在 stash | 手工重施加并**即时提交**缩短暴露窗口（核心层 fd2c87ad → 套件层 e0cadb7a） |
| 5 | 流水线事故恢复提交 100ca5d0 | sandbox.rs 自 git 对象库逐字恢复（←17df6b6b），与我的提交合并后一致 |
| 6 | HEAD 多次迁移（16b67bb9→03639223→139a0b90→627a348d→d304997b merge） | 每轮迁移后核验 8 文件特征，均完好；最终合入 integration（f70e25dc→9eaf4889） |

**经验**：高 churn 环境下唯一可靠保护是"改完即提交"；恢复优先级 = 已提交 > stash > git 对象库（`git log -S` / `git show <stash>:path` 定位）；恢复他人 stash 内容前先 diff 确认归属，避免卷入他人 WIP。

## 四、边界与未竟（如实）

- Sandboxie/landlock 后端：trait 留口未接（设计内，非本批范围）
- 非 Windows 平台：Job Object 为 Windows 专属，本模块 no-op 并如实标注（Linux cgroup/prctl 属未来工作）
- crate 全绿：待并行 WIP 收敛后 QA 复跑（惯例）
