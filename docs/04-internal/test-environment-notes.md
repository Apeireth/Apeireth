# Test Environment Notes（平台敏感测试甄别手册）

> 用途：区分 **new regression** 与 **host limitation / timing flake**。
> 纪律：不自动重试、不放大超时、不 ignore 测试——用代码掩盖环境问题是被禁止的。

## Known platform-sensitive tests

| 测试 | 症状 | 性质 |
| --- | --- | --- |
| `apeireth-tools-canonical` `shell_execution` | `process supervision I/O error: timed out waiting for child output reader to finish` | Windows 下进程监督 I/O 超时；通过数随负载波动（同机观测过 6/4、3/7、2/8） |
| `apeireth-tools-canonical` `shell::tests::invoke_frozen_uses_frozen_cwd…` | `isolation requirement unsatisfied: missing [(FilesystemIsolation…), (NetworkIsolation…)]` | 本机 AppContainer 能力缺失（能力报告显示 `filesystem_isolation: Unsupported`） |
| `apeireth-tools-canonical` `process::appcontainer::tests::probe_result_matches_capability_report` | 能力报告与真实探针不一致 | AppContainer 探针随机器状态/系统版本漂移 |
| `apeireth-tools-canonical` `shell_execution`（变体） | `failed to attach process containment: CreateAppContainerProfile failed: HRESULT=0x8000ffff` | 同族 AppContainer 建档失败（与监督超时型交替出现，通过数随主机状态波动） |

## How to triage a failure

1. **查变更面**：失败文件/路径是否被本轮 diff 或上游 diff 触碰？都没有 → 大概率环境。
2. **隔离复跑**：`cargo test -p <pkg> --test <target>` 单跑；通过数波动即 flake 实证。
3. **对照 worktree**：在干净 `origin/main` worktree 跑同一套件；同型失败 = host limitation 闭环证明（2026-09-24 已对 `shell_execution` 做过此对照：纯净上游 3/7 同型失败）。
4. 归类四选一：`new regression` / `upstream regression` / `environment/platform failure` / `timing/load flake`，报告里写证据，不写"感觉是 pre-existing"。
