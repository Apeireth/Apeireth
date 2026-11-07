# 任务清单（task manifest）—— **可选**、非阻塞

> **owner 2026-10-30 裁定**：任务清单**可入仓**，但**非强制**、**不接 required gate**。
> 用途**仅限**：给 AI 提供「**任务边界 + 最小上下文**」。

## 为什么有它

Astra 的效率型意见 §2/§9：AI 失败常常不是能力问题，而是**边界不清**
（同时改产品代码和 CI / 跨多个不相关 crate / 迁移与新功能混在一起）。

⇒ ⇒ 把边界**声明出来**，机器就能校验；校验器是**非阻塞**的（提醒，不是门禁）。

## 格式

```yaml
# .harness/tasks/<日期>-<短名>.yaml
task_id: 2026-10-30-injection-contract
summary: 把注入契约接进生产路径（渲染形态可换）
base_sha: 158cf5ac
allowed_paths:            # 允许修改（相对仓库根）
  - crates/engine/memory/src/**
  - crates/engine/memory/tests/**
forbidden_paths:          # 禁止触碰
  - Cargo.lock
  - crates/foundation/core/src/eight_anchors.rs
required_checks:          # 本任务必须过的门禁（契约 [ci_gates] 的 id）
  - internal-golden
  - golden-baseline
tier: contract            # A=产品 B=harness C=证据/文档 D=修复/整合（见 §PR 分层）
```

## 怎么用

```sh
# 生成上下文包（含任务边界）
python3 scripts/harness_context.py --task .harness/tasks/2026-10-30-injection-contract.yaml

# 校验所有任务清单（非阻塞提醒）
python3 scripts/check_task_manifest.py
```

## 纪律

* 清单**只声明边界**，**不代替**真源：门禁清单在契约 `[ci_gates]`，不在这里；
* 清单**进仓**（它就是「为什么要动这些路径」的记录），上下文包**不入仓**（每次重生成）；
* `forbidden_paths` 命中 **受锁源码**（契约 `[locked]`）⇒ 校验器**直接报错**。
