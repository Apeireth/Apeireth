#!/usr/bin/env python3
"""env 预算口径的**单一真源**：区分「能力开关」与「测试夹具 / 已裁决名」。

# 为什么有这个模块（2026-11-07 口径修正）

同一件事，两道守卫原先**各算一份**，而且算得不一样：

* `arch_guard.py`（预算闸门）—— 把 workspace 各包 `src/**/*.rs` 里**全文 findall** 到的
  每个 `APEIRETH_*` 都算一个名额 ⇒ 连**测试夹具 / 密钥标记**（如
  `APEIRETH_IMAGE_UNSHARP_*` 族、`APEIRETH_REG_*` 族、`APEIRETH_UNIQ_SECRET_MARKER_*`、
  `APEIRETH_IMAGE_REGEN_PROFILES`）与**已裁决「真实 env 但本仓无 direct 读取点」**的
  `APEIRETH_MASTER_TOKEN` 一起计入 ⇒ ⇒ **预算虚高 12**（162 而真实能力开关只有 150）。
* `wiring_report.py`（登记表生成/校验）—— 早已有一份**带理由**的排除表
  （`TEST_ONLY_RE` / `PENDING_EVIDENCE_RULING`），但只用于登记表 `kind` 分类，
  **不参与预算**。

⇒ ⇒ 两份账本口径不一致 ⇒ 预算数字**不代表能力开关数** ⇒ 会逼着后续按「假名额」删东西。

# 口径（**不是缩表**，是把尺子修准）

预算只数「**能力开关**」：

1. 排除 `TEST_ONLY_RE` 命中的测试 / 基准夹具与密钥标记 —— 它们不是能力开关，
   不该占「能力开关预算」；
2. 排除 `PENDING_EVIDENCE_RULING` 里已裁决的名字（真实 env，但本仓无 direct 读取点）。

⚠️ **登记面不变**：所有 `APEIRETH_*`（含被排除的）**仍必须登记**进
`docs/architecture/env-registry.toml`；`arch_guard` 的「新增未登记」检查按**全集**判 ——
**不因为不进预算就免登记**。

# 纪律

* 本模块是**唯一**的正则 / 排除表来源：`arch_guard.py` 与 `wiring_report.py` 都 import 它，
  **不许**任何一方再抄一份（否则又会漂移出第二套账）。
* 新增排除项必须写**理由**并说明判据（是夹具？是已裁决？），不接受「为了预算好看」。
"""

from __future__ import annotations

import re
from typing import Iterable

#: `APEIRETH_*` 环境变量名（与两道守卫原口径一致）。
ENV_RE = re.compile(r"APEIRETH_[A-Z0-9_]+")

#: ADR-0004 §7.7/§7.8：`kind=test-only` 的测试 / 基准夹具与密钥标记（**非能力开关**）。
TEST_ONLY_RE = re.compile(
    r"^APEIRETH_IMAGE_UNSHARP_|^APEIRETH_UNIQ_SECRET_MARKER_"
    r"|^APEIRETH_REG_|^APEIRETH_IMAGE_REGEN_PROFILES$"
)

#: 待架构线裁决：**真实 env 但本仓无 direct 读取点**（不在产品 direct 校验里静默通过）。
PENDING_EVIDENCE_RULING = frozenset({"APEIRETH_MASTER_TOKEN"})


def is_budget_exempt(name: str) -> bool:
    """该名是否**不计入 env 预算**（测试夹具 / 密钥标记 / 已裁决名）。"""
    return bool(TEST_ONLY_RE.search(name)) or name in PENDING_EVIDENCE_RULING


def capability_env_names(names: Iterable[str]) -> set[str]:
    """从实测 env 名集合里取出**能力开关**子集（= 预算真正要卡的那一批）。

    ⚠️ 只用于**预算计数**；登记 / 基线 / 「新增未登记」检查仍用**全集**。
    """
    return {name for name in names if not is_budget_exempt(name)}
