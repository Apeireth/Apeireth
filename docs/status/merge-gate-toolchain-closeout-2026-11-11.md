# 合并裁决工具链收口（2026-11-11）

[Document-Meta]
Document: docs/status/merge-gate-toolchain-closeout-2026-11-11.md
Class: STATUS
Status: 🟢 全部适用 CI 通过，PR #121 已合并
Last-Modified: 2026-11-11
Author: Codex

## 目标与范围

PR #121 的合并裁决在独立的 CI 工具链目录中，按根 `rust-toolchain.toml`
的 channel、profile、components 完整装配。拒绝依赖 runner 共享目录中的半装状态，
不卸载共享工具链，不把 `rustup --force` 误作文件冲突清理。

写入范围：`.github/workflows/merge-gate.yml`、工具链装配脚本及行为测试、
`scripts/check_toolchain_pin.py`、生成清单和本页的导航/变更记录。
主干合入保留 #119 对 required workflow 触发的修复，不改变分支保护。

## 进度与验收

- 主干 `c8264db2` 已合入，收口提交为 `f0facab4dd8603fa40ea08d2d227e977659858cc`。
- 装配脚本按根声明安装，验证 rustc 版本及完整组件后才向 `GITHUB_ENV` 导出
  隔离的 `RUSTUP_HOME` 和 `RUSTUP_TOOLCHAIN`。原 `CARGO_HOME` 与代理路径保留。
- `python3 scripts/test_provision_ci_toolchain.py`：7 个行为用例通过，exit 0。
- `python3 scripts/check_toolchain_pin.py`、`python3 scripts/check_yaml_duplicate_keys.py`、
  `python3 scripts/check_guard_inventory.py`、`python3 scripts/gen_gate_manifest.py --check`
  均 exit 0；契约 required 门禁数量未改。
- 反向验伪在内存中逐项回退「隔离目录」「跟随根钉版」「完整组件检查」，
  每项均产生 `FAILED (failures=1)`；恢复后 `Ran 7 tests` / `OK`，反证脚本 exit 0。
  日志在施工机 `C:/Apeireth/logs/pr-closeout-20261111/pr121-negative-controls.log`，
  不作为仓库内可移植证据冒充远端 CI。
- 真实 Linux 装配与当前树的合并裁决已通过：[run 43013692340](https://github.com/Apeireth/Apeireth/actions/runs/43013692340)。证据绑定该 PR 的合并树，与收口提交 tree 一致。
- 还差什么：本批 PR 无剩余合并阻塞；7 项 required 与全部适用 CI 通过后，[PR #121](https://github.com/Apeireth/Apeireth/pull/121) 于 2026-11-11 普通合并，主干提交 `9344d5208bf78333f5d22a607cb7d648e3de13d2`。
- 不新增产品能力，不声明真实跨节点 E2E 或硬件验收。

## 审查中纠正的旧账

- PR #121 原始 run `42983832787` 的真实日志已经执行过新增装配步骤；原 PR
  正文「本 PR 仍跑旧 workflow、未在真实 runner 执行」不准确，提交收口时需改正。
- `rustup toolchain install --help` 对 `--force` 的定义是允许更新时缺少组件，
  不是删除目标目录里的文件冲突。装配改用新的临时目录，失败仍阻断。
- 旧 head 缺三项 required check；主干 #119 已取消两个 required workflow 的 PR
  顶层路径过滤。本次合入主干保留该修复，当前新 head 的 checks 仍须真实跑完。

## 复验入口

```sh
python3 scripts/check_toolchain_pin.py
python3 scripts/test_provision_ci_toolchain.py
python3 scripts/check_yaml_duplicate_keys.py
python3 scripts/check_guard_inventory.py
python3 scripts/gen_gate_manifest.py --check
python3 scripts/harness.py verify
```
