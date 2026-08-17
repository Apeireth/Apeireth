# TP29 工具声明式配置 (Composio 借鉴, 插件 YAML 声明) — 验收报告 (任务 ID 36f97bc8)

**任务**: `36f97bc8-0046-4d11-9760-be1bee933344` (TP29, 生态批) — 与任务 `cf0194b6-f842-4d69-ba73-48cc6893b5f4` 同一份工作
**角色**: 后端工程师
**分支**: `task/tp12-schema-guardrail-rework-final`
**提交**: `b3a2cb6` feat(tools+companion): TP29 工具声明式配置 (Composio 借鉴, 插件 YAML 声明)

> **本任务与 cf0194b6 任务为同一份工作 (TP29 工具声明式配置)**。完整报告见:
> [`reports/cf0194b6-f842-4d69-ba73-48cc6893b5f4-backend_engineer-report.md`](./cf0194b6-f842-4d69-ba73-48cc6893b5f4-backend_engineer-report.md)
>
> 本文件仅作为任务 ID 对应的指针文件, 内容精简。

---

## 1. 交付物 (指针)

- **新模块**: `crates/apeireth-tools/src/yaml_spec.rs` (~1040 行, 22 测试)
- **tool_bridge 衔接**: `ToolBridge::register_yaml_spec` + `ToolBridge::register_yaml_spec_dir` (2 个新方法)
- **companion 集成测试**: 4 个 `tp29_tests` (合法注册 / 非法不破坏 / 同名冲突 / dir 批量)
- **台账**: `docs/backlog.md` TP29 ✅ 条目 (含提交 b3a2cb6 引用)

## 2. 三种绿 (复验)

| 验证项 | 结果 |
|---|---|
| `cargo test -p apeireth-tools --lib` | ✅ 190 passed; 0 failed |
| `cargo test -p apeireth-companion --lib` | ✅ 546 passed; 0 failed |
| `cargo check --workspace --all-targets` | ✅ 0 errors |

## 3. 纪律核对

- ✅ 不破坏现有 tool_bridge API (2 个新方法, 向后兼容)
- ✅ 真实密码不入 yml (TP33 纪律: `${VAR:?msg}` 形式, CredentialSpec::validate 兜底)
- ✅ 单测绿 + all-targets 绿 + 0 错 (纪律 #1 三种绿全数达成)

## 4. Re-apply 复验 (2026-08-17 post-rebase)

上轮 rebase 过程中 `b3a2cb6` 提交意外丢失 (仅在 `backup-pre-rebase` 分支保留)。本轮通过 `git cherry-pick b3a2cb6` 重新挂回 + 修复 1 个编译警告 (`YamlFile` 私有类型公开方法), 重新跑三种绿全绿。详见主报告 §11。

---

— 后端工程师 / TP29 (任务 36f97bc8 指针报告)
