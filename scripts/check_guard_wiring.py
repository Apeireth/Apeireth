#!/usr/bin/env python3
"""护栏（`apeireth-guard`）**接线台账**（机器可检）。

# 为什么要有这个脚本

`crates/engine/guard` 是一个完整 crate（chain / enforcement / decision_log /
classifier / command_effect …），它是「**越权命中恒 0**」这条**安全红线**的执行者。

⚠️ **但没人验证过它到底接到了哪些生产路径** ⇒ 「护栏写了这么多，是不是真在拦？」
**目前无人能答** ⇒ **这本身就是一个缺口**。

⇒ 本脚本把「**哪些护栏能力真在生产路径上被调用**」变成**可机检的台账**：

* **已接生产**（有生产调用点）⇒ 打印证据（文件:行）；
* **只有测试 / 离线评测在用** ⇒ 打印出来（**不假装有**）；
* **完全没接** ⇒ 打印出来（**缺口如实列出**）。

# 口径（与 `scripts/check_quality_metrics.py` 同口径）

* 「有调用点」= 仓里**有代码真的调用了**该护栏能力（用**具体类型 / 函数名**当锚点，
  **不用**泛词 —— 泛词到处命中 = 假绿）；
* 「生产」= `crates/**/src/**` 里的**非测试**代码（`#[cfg(test)]` 块、`tests/`
  目录、`examples/`、`benches/` 一律**不算生产**）；
* **crate 内生产路径** = `apeireth-guard` 自己那些**从 `hook.rs` 可达**的模块；
  `scenario` / `oracle` 是**离线评测面**，**不算**；
* **只 import 不调用** ⇒ **不算调用点**（`use` 语句整体剔除后再匹配）；
* **定义行不算调用点**（`pub fn X` / `pub struct X` / `impl ... for X`）——
  否则「自己定义自己」会把每个能力都染成「已接生产」；
* 台账文件：`scripts/guard-wiring-ledger.json`
  ⇒ **可手改**（人工裁决结果），但**每条都必须有代码锚点支撑**；
  台账与代码对不上 ⇒ **退出码 1**。

# 用法

```sh
python3 scripts/check_guard_wiring.py            # 查台账与代码是否一致（不一致退出码 1）
python3 scripts/check_guard_wiring.py --report   # 只打印覆盖率报告
python3 scripts/check_guard_wiring.py --update   # 重扫代码锚点，重写台账
```
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass, field
from datetime import date
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
LEDGER_PATH = REPO_ROOT / "scripts" / "guard-wiring-ledger.json"
CRATES_ROOT = REPO_ROOT / "crates"
GUARD_CRATE = "crates/engine/guard"

# ── 护栏 crate 内「生产闭环」的模块集合 ────────────────────────────────────
#
# 判据：从 `hook.rs`（判定主体）出发**可达**的模块 ⇒ 属于护栏的生产路径。
# `scenario` / `oracle` 是**离线评测面**（场景目录 + oracle 对账），
# 它们只被测试与 examples 调用 ⇒ **不算生产接线**。
GUARD_PRODUCTION_MODULES = frozenset(
    {
        "chain",
        "chain_guard",
        "classifier",
        "command_effect",
        "dataset",
        "decision",
        "decision_log",
        "enforcement",
        "fast_guard",
        "features",
        "features_v2",
        "fusion",
        "hook",
        "immune_memory",
        "intent",
        "introspection",
        "lib",
        "observation",
        "semantics",
        "session",
        "snapshot",
        "snapshot_store",
    }
)

# ── 状态标签 ───────────────────────────────────────────────────────────────
ST_PRODUCTION = "PRODUCTION WIRED"  # 其它 crate 的生产代码真的调用了
ST_INTERNAL = "PRODUCTION WIRED (crate 内生产路径)"  # 只在护栏自己的生产闭环里被调用
ST_TEST_ONLY = "TEST ONLY"  # 只有测试 / 离线评测在用
ST_NOT_WIRED = "NOT WIRED"  # 零调用点


@dataclass(frozen=True)
class Capability:
    """一个「护栏公开能力」。

    粒度 = 护栏对外公开的一组**具体**类型 / 函数（不用泛词）。

    * `scope="external"`：只看护栏 crate **之外**的引用（用来问「有没有外部消费者」）；
    * `call_form=True`：锚点必须以**调用形态**出现（`name(`），避免字段名 / 类型名误命中；
    * `closure=()`：显式声明该能力**不在**护栏生产闭环内（⇒ crate 内命中也不算生产）。
    """

    id: str
    module: str
    title: str
    anchors: tuple[str, ...]
    scope: str = "any"  # "any" | "external"
    call_form: bool = False
    closure: frozenset[str] | None = None  # None = 用默认闭环集合
    note: str = ""

    @property
    def external_only(self) -> bool:
        return self.scope == "external"

    @property
    def production_modules(self) -> frozenset[str]:
        if self.closure is None:
            return GUARD_PRODUCTION_MODULES
        return self.closure


OFFLINE_ONLY = frozenset()

CAPABILITIES: tuple[Capability, ...] = (
    # ── hook：守门本体与它的接线开关 ──────────────────────────────────────
    Capability(
        "hook.guard_hook",
        "hook",
        "守门本体（实现 GovernanceHook，决定动作放行 / 拦截）",
        ("BehaviorChainGuardHook",),
        note="生产治理栈的护栏闸；CLI 装配根把 hook 挂进 pipeline。",
    ),
    Capability(
        "hook.evaluate_detailed",
        "hook",
        "单动作明细判定读口（evaluate_detailed）",
        ("evaluate_detailed",),
        call_form=True,
    ),
    Capability(
        "hook.dry_run",
        "hook",
        "试算面（dry_run：算但不落链、不写账）",
        ("dry_run",),
        call_form=True,
        note="网关面板 SafetyGuardQuery 的试算口。",
    ),
    Capability(
        "hook.status",
        "hook",
        "运行状态自述面（GuardStatusDto）",
        ("GuardStatusDto",),
    ),
    Capability(
        "hook.recent_events",
        "hook",
        "近判定事件读口（recent_events）",
        ("recent_events",),
        call_form=True,
    ),
    Capability(
        "hook.update_action_execution",
        "hook",
        "执行结果回写（把动作成败写回行为链）",
        ("update_action_execution",),
        call_form=True,
    ),
    Capability(
        "hook.with_decision_log",
        "hook",
        "判定落盘面挂载（with_decision_log）",
        ("with_decision_log",),
        call_form=True,
    ),
    Capability(
        "hook.with_immune_memory",
        "hook",
        "免疫记忆挂载（with_immune_memory，默认关）",
        ("with_immune_memory",),
        call_form=True,
    ),
    Capability(
        "hook.with_dataset_recorder",
        "hook",
        "脱敏数据集记录器挂载（with_dataset_recorder，默认关）",
        ("with_dataset_recorder",),
        call_form=True,
    ),
    Capability(
        "hook.with_classifier",
        "hook",
        "本地模型分类器挂载（with_classifier，默认关）",
        ("with_classifier",),
        call_form=True,
    ),
    Capability(
        "hook.with_safety_provider",
        "hook",
        "能力安全元数据源注入（with_safety_provider）",
        ("with_safety_provider",),
        call_form=True,
    ),
    Capability(
        "hook.bind_turn_intent",
        "hook",
        "回合可信意图绑定（bind_turn_intent）",
        ("bind_turn_intent",),
        call_form=True,
    ),
    Capability(
        "hook.set_declared_scope",
        "hook",
        "声明任务范围（hook.set_declared_scope）",
        ("hook.set_declared_scope",),
        call_form=True,
        note="锚点带 `hook.` 前缀：与 `BehaviorChain::set_declared_scope` 区分开。",
    ),
    Capability(
        "hook.clear_session",
        "hook",
        "会话结束清理（clear_session）",
        ("clear_session",),
        call_form=True,
    ),
    Capability(
        "hook.session_readouts",
        "hook",
        "会话行为摘要 / 风险史 / 行为链读口",
        (
            "session_behavior_summary",
            "session_risk_history",
            "chain_for_trace",
            "session_history",
        ),
        call_form=True,
    ),
    Capability(
        "hook.last_feature_snapshot",
        "hook",
        "特征快照诊断读口（last_feature_snapshot）",
        ("last_feature_snapshot",),
        call_form=True,
    ),
    Capability(
        "hook.last_immune_signal",
        "hook",
        "免疫信号读口（last_immune_signal）",
        ("last_immune_signal",),
        call_form=True,
    ),
    # ── intent：意图解释与对齐 ─────────────────────────────────────────────
    Capability(
        "intent.rule_interpreter",
        "intent",
        "规则意图解释器（RuleIntentInterpreter）",
        ("RuleIntentInterpreter",),
        note="网关 / CLI 生产路径用它把用户原话变成可信信封。",
    ),
    Capability(
        "intent.alignment_guard",
        "intent",
        "意图-动作对齐守卫（IntentAlignmentGuard）",
        ("IntentAlignmentGuard",),
    ),
    Capability(
        "intent.constrain_to_trusted",
        "intent",
        "模型提案向可信信封收窄（constrain_to_trusted）",
        ("constrain_to_trusted",),
        call_form=True,
    ),
    Capability(
        "intent.negation_extractor",
        "intent",
        "否定敏感操作抽取（NegationAwareOperationExtractor）",
        ("NegationAwareOperationExtractor",),
    ),
    Capability(
        "intent.confidence_policy",
        "intent",
        "意图置信度保守化（IntentConfidencePolicy）",
        ("IntentConfidencePolicy",),
    ),
    # ── 两段判定内核 ──────────────────────────────────────────────────────
    Capability(
        "fast_guard.stage_a",
        "fast_guard",
        "A 段快守卫（FastGuard：确定性毫秒级风险筛）",
        ("FastGuard",),
    ),
    Capability(
        "chain_guard.stage_b",
        "chain_guard",
        "B 段链守卫（ChainGuard：复合 / 升级 / 出网风险）",
        ("ChainGuard",),
    ),
    Capability(
        "fusion.decision_fusion",
        "fusion",
        "判定融合（DecisionFusion：规则结论 + 模型预测）",
        ("DecisionFusion",),
    ),
    # ── classifier ────────────────────────────────────────────────────────
    Capability(
        "classifier.joint",
        "classifier",
        "联合风险模型分类器（JointRiskClassifier）",
        ("JointRiskClassifier",),
    ),
    Capability(
        "classifier.threshold",
        "classifier",
        "阈值分类器（ThresholdClassifier）",
        ("ThresholdClassifier",),
    ),
    Capability(
        "classifier.no_classifier",
        "classifier",
        "空分类器（NoClassifier：护栏默认）",
        ("NoClassifier",),
    ),
    Capability(
        "classifier.artifact_stamp",
        "classifier",
        "模型工件盖章 / 哈希（stamp_artifact_file / canonical_artifact_sha256 / feature_schema_hash）",
        ("stamp_artifact_file", "canonical_artifact_sha256", "feature_schema_hash"),
        closure=OFFLINE_ONLY,
        note="离线模型工件工具面；护栏生产闭环里没有它。",
    ),
    # ── decision / enforcement ────────────────────────────────────────────
    Capability(
        "decision.guard_decision",
        "decision",
        "判定结构与 verdict 映射（GuardDecision）",
        ("GuardDecision",),
    ),
    Capability(
        "enforcement.directive",
        "enforcement",
        "收容义务指令**计算**（EnforcementDirective::from_decision）",
        ("EnforcementDirective",),
    ),
    Capability(
        "enforcement.directive_consumer",
        "enforcement",
        "收容义务**执行面**（撤销会话授权 / 标记高风险 trace / 补偿线索）",
        ("EnforcementDirective",),
        scope="external",
        note="只看护栏 crate 之外：仓内**零消费者** ⇒ 义务算了没人执行。",
    ),
    # ── decision_log / dataset ────────────────────────────────────────────
    Capability(
        "decision_log.ledger",
        "decision_log",
        "判定落盘账（GuardDecisionLog，append-only JSONL）",
        ("GuardDecisionLog",),
    ),
    Capability(
        "decision_log.risk_label",
        "decision_log",
        "风险分→档映射（risk_label_for_score）",
        ("risk_label_for_score",),
    ),
    Capability(
        "dataset.recorder",
        "dataset",
        "脱敏数据集记录器（DatasetRecorder，默认关）",
        ("DatasetRecorder",),
    ),
    Capability(
        "dataset.execution_outcome",
        "dataset",
        "执行结果归类（GuardExecutionOutcome）",
        ("GuardExecutionOutcome",),
    ),
    # ── semantics / command_effect / observation ──────────────────────────
    Capability(
        "semantics.registry",
        "semantics",
        "能力安全注册表（CapabilitySafetyRegistry）",
        ("CapabilitySafetyRegistry",),
    ),
    Capability(
        "semantics.descriptor_lookup",
        "semantics",
        "单能力描述符查询（descriptor_for_capability）",
        ("descriptor_for_capability",),
        closure=OFFLINE_ONLY,
        note="自由函数面只被评测 / 测试用；生产路径走的是注册表方法。",
    ),
    Capability(
        "semantics.coverage_report",
        "semantics",
        "描述符覆盖率报告（canonical_descriptor_coverage）",
        ("canonical_descriptor_coverage",),
        closure=OFFLINE_ONLY,
    ),
    Capability(
        "command_effect.analyzer",
        "command_effect",
        "命令效应解析（CommandEffectAnalyzer）",
        ("CommandEffectAnalyzer",),
    ),
    Capability(
        "observation.model",
        "observation",
        "安全观察模型（SafetyObservation）",
        ("SafetyObservation",),
    ),
    # ── 链 / 特征 / 会话 / 快照 ───────────────────────────────────────────
    Capability(
        "chain.behavior_chain",
        "chain",
        "行为链（BehaviorChain）",
        ("BehaviorChain",),
    ),
    Capability(
        "features.v1",
        "features",
        "V1 特征（AgentChainFeatureV1）",
        ("AgentChainFeatureV1",),
    ),
    Capability(
        "features.v2",
        "features_v2",
        "V2 特征 + 跨轮摘要（AgentChainFeatureV2）",
        ("AgentChainFeatureV2",),
    ),
    Capability(
        "session.history",
        "session",
        "跨轮行为史（SessionBehaviorHistory）",
        ("SessionBehaviorHistory",),
    ),
    Capability(
        "snapshot.store",
        "snapshot_store",
        "特征快照存储（SnapshotStore）",
        ("SnapshotStore",),
    ),
    # ── 离线评测面 ────────────────────────────────────────────────────────
    Capability(
        "scenario.catalog",
        "scenario",
        "场景目录与跑批（ScenarioCatalog / run_scenario / family_split）",
        ("ScenarioCatalog", "run_scenario", "run_scenario_checked", "family_split"),
        closure=OFFLINE_ONLY,
    ),
    Capability(
        "oracle.security_scenario",
        "oracle",
        "安全场景 oracle（SecurityScenarioOracle / ActionOracleLabel）",
        ("SecurityScenarioOracle", "ActionOracleLabel"),
        closure=OFFLINE_ONLY,
    ),
    # ── 自述面 ────────────────────────────────────────────────────────────
    Capability(
        "introspection.dto",
        "introspection",
        "自述 DTO（GuardEventDto / GuardDryRunRequest / GuardDryRunResponse）",
        ("GuardEventDto", "GuardDryRunRequest", "GuardDryRunResponse"),
    ),
)


# ── 源码扫描 ───────────────────────────────────────────────────────────────


def strip_comments(text: str) -> str:
    """去掉行注释与块注释（保留行数，便于定位行号）。"""
    out: list[str] = []
    index = 0
    length = len(text)
    in_block = False
    while index < length:
        char = text[index]
        nxt = text[index + 1] if index + 1 < length else ""
        if in_block:
            if char == "*" and nxt == "/":
                in_block = False
                out.append("  ")
                index += 2
                continue
            out.append("\n" if char == "\n" else " ")
            index += 1
            continue
        if char == "/" and nxt == "/":
            while index < length and text[index] != "\n":
                out.append(" ")
                index += 1
            continue
        if char == "/" and nxt == "*":
            in_block = True
            out.append("  ")
            index += 2
            continue
        out.append(char)
        index += 1
    return "".join(out)


def strip_string_literals(text: str) -> str:
    """把字符串 / 字符字面量的**内容**替换成空格（保留行数）。

    为什么：字面量里的花括号会打乱「作用域配平」，导致后面的行被误判成测试代码。
    """
    out: list[str] = []
    index = 0
    length = len(text)
    while index < length:
        char = text[index]
        if char == '"':
            raw_hashes = 0
            probe = index - 1
            while probe >= 0 and text[probe] == "#":
                raw_hashes += 1
                probe -= 1
            is_raw = probe >= 0 and text[probe] == "r"
            out.append('"')
            index += 1
            terminator = '"' + ("#" * raw_hashes if is_raw else "")
            while index < length:
                if text[index] == "\\" and not is_raw:
                    out.append(" ")
                    if index + 1 < length:
                        out.append("\n" if text[index + 1] == "\n" else " ")
                    index += 2
                    continue
                if text.startswith(terminator, index):
                    out.append(" " * len(terminator))
                    index += len(terminator)
                    break
                out.append("\n" if text[index] == "\n" else " ")
                index += 1
            continue
        if char == "'":
            nxt = text[index + 1] if index + 1 < length else ""
            if nxt and nxt != "\\" and index + 2 < length and text[index + 2] == "'":
                out.append("   ")
                index += 3
                continue
            if nxt == "\\":
                end = index + 3
                while end < length and text[end] != "'":
                    end += 1
                out.append(" " * (end - index + 1))
                index = end + 1
                continue
        out.append(char)
        index += 1
    return "".join(out)


def strip_use_statements(text: str) -> str:
    """整段剔除 `use` / `pub use` 语句（含跨行花括号形式），**保持行数不变**。

    为什么：`use apeireth_guard::Foo;` 只是**导入**，不是**调用**。
    把导入算成调用点 ⇒ 「只 import 就算接生产」⇒ 假绿。
    """
    lines = text.split("\n")
    kept: list[str] = []
    index = 0
    while index < len(lines):
        stripped = lines[index].strip()
        if (
            stripped.startswith("use ")
            or stripped.startswith("pub use ")
            or stripped.startswith("pub(crate) use ")
        ):
            start = index
            while index < len(lines) and ";" not in lines[index]:
                index += 1
            if index < len(lines):
                index += 1
            kept.extend([""] * (index - start))
            continue
        kept.append(lines[index])
        index += 1
    return "\n".join(kept)


_MODULE_RE = re.compile(r"^\s*(pub(\([^)]*\))?\s+)?(async\s+)?mod\s+\w+")
_FN_RE = re.compile(r"^\s*(pub(\([^)]*\))?\s+)?(async\s+)?(unsafe\s+)?fn\s+\w+")


def split_cfg_test(text: str) -> tuple[str, str]:
    """把 `#[cfg(test)]` 块从生产文本里剥出来。

    返回 (生产文本, 测试文本)，两者**行数一一对应**，便于按行号定位。
    """
    lines = text.split("\n")
    production: list[str] = []
    test: list[str] = []
    # 栈：每个作用域是不是测试作用域
    stack: list[bool] = [False]
    pending_test = False
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("#[cfg(test)]"):
            pending_test = True
            production.append("")
            test.append("")
            continue
        opens_test_block = pending_test and bool(
            _MODULE_RE.match(stripped) or _FN_RE.match(stripped)
        )
        if pending_test and not opens_test_block:
            # `#[cfg(test)]` 挂在 use / const / 宏上：整行归测试文本
            pending_test = False
            test.append(line)
            production.append("")
            continue
        if opens_test_block:
            pending_test = False
            test.append(line)
            production.append("")
            stack.append(True)
            opens = line.count("{")
            closes = line.count("}")
            for _ in range(max(0, opens - 1)):
                stack.append(stack[-1])
            for _ in range(closes):
                if len(stack) > 1:
                    stack.pop()
            continue
        in_test = stack[-1]
        if in_test:
            test.append(line)
            production.append("")
        else:
            production.append(line)
            test.append("")
        opens = line.count("{")
        closes = line.count("}")
        for _ in range(opens):
            stack.append(stack[-1])
        for _ in range(closes):
            if len(stack) > 1:
                stack.pop()
    return "\n".join(production), "\n".join(test)


def classify_path(relative: str) -> str:
    """文件归类：production / test / tool。"""
    parts = relative.split("/")
    name = parts[-1]
    if "examples" in parts or "benches" in parts:
        return "tool"
    if "tests" in parts or name == "tests.rs" or name.endswith(".test.rs"):
        return "test"
    return "production"


def guard_module_of(relative: str) -> str | None:
    """若文件属于护栏 crate，返回它所属的模块名。"""
    prefix = f"{GUARD_CRATE}/src/"
    if not relative.startswith(prefix):
        return None
    rest = relative[len(prefix) :]
    if "/" in rest:
        return rest.split("/")[0]
    stem = rest[:-3] if rest.endswith(".rs") else rest
    return stem


def is_definition_line(line: str, anchor: str) -> bool:
    """定义行 / trait 实现头 ⇒ 不算「调用点」。"""
    escaped = re.escape(anchor)
    if re.match(
        rf"^\s*pub(\([^)]*\))?\s+(async\s+)?(fn|struct|enum|trait|const|type|static)\s+{escaped}\b",
        line,
    ):
        return True
    if re.match(rf"^\s*impl\b.*\bfor\s+{escaped}\b", line):
        return True
    if re.match(rf"^\s*impl\s+{escaped}\b", line):
        return True
    return False


@dataclass
class FileScan:
    relative: str
    kind: str  # production / test / tool
    guard_module: str | None
    guard_imports: set[str]
    guard_modules: set[str]
    mentions_guard: bool
    production_text: str
    test_text: str


def guard_import_names(text: str) -> tuple[set[str], set[str]]:
    """抽出「本文件从 `apeireth_guard` 导入了哪些名字」。

    返回 (导入项名集合, 导入的模块名集合)。

    # 为什么必须这么做

    护栏的公开类型名**不是唯一的**：`apeireth_memory` 也有一个 `GuardDecision`
    （访问许可裁决，与护栏的判定结构**完全无关**）。裸名匹配 ⇒ 护栏的
    `GuardDecision` 会因为内存 crate 的用法而被误判成「已接生产」= **假绿**。

    ⇒ 规则：**护栏 crate 之外**的文件里，裸名只有在
    「①本文件从 `apeireth_guard` 导入了它」或「②写成 `apeireth_guard::X`」
    或「③写成 `M::X`（M 是本文件导入的护栏模块）」时才算命中。
    """
    items: set[str] = set()
    modules: set[str] = set()
    for match in re.finditer(r"\buse\s+apeireth_guard\s*(?:::|as\b)", text):
        index = match.end()
        depth = 0
        end = index
        while end < len(text):
            char = text[end]
            if char == "{":
                depth += 1
            elif char == "}":
                depth -= 1
            elif char == ";" and depth <= 0:
                break
            end += 1
        segment = text[index:end]
        for identifier in re.findall(r"[A-Za-z_][A-Za-z0-9_]*", segment):
            if identifier in {"self", "crate", "super", "as"}:
                continue
            items.add(identifier)
        for module in re.findall(r"\b([a-z][a-z0-9_]*)\s*::", segment):
            modules.add(module)
    return items, modules


def scan_files() -> list[FileScan]:
    scans: list[FileScan] = []
    for path in sorted(CRATES_ROOT.rglob("*.rs")):
        relative = path.relative_to(REPO_ROOT).as_posix()
        raw = path.read_text(encoding="utf-8", errors="replace")
        cleaned = strip_string_literals(strip_comments(raw))
        kind = classify_path(relative)
        imports, modules = guard_import_names(cleaned)
        if kind == "production":
            production, test = split_cfg_test(cleaned)
        else:
            # `tests/` `examples/` `benches/` 里的**整个文件**都不是生产代码
            production, test = cleaned, cleaned
        production = strip_use_statements(production)
        test = strip_use_statements(test)
        if kind != "production":
            production = "\n".join("" for _ in production.split("\n"))
        scans.append(
            FileScan(
                relative=relative,
                kind=kind,
                guard_module=guard_module_of(relative),
                guard_imports=imports,
                guard_modules=modules,
                mentions_guard="apeireth_guard" in cleaned,
                production_text=production,
                test_text=test,
            )
        )
    return scans


def anchor_pattern(anchor: str, call_form: bool) -> re.Pattern[str]:
    if call_form:
        return re.compile(rf"\b{re.escape(anchor)}\s*\(")
    return re.compile(rf"\b{re.escape(anchor)}\b")


def anchor_hits(
    text: str,
    anchor: str,
    call_form: bool,
    scan: "FileScan | None" = None,
) -> list[int]:
    """返回 anchor 真正被「调用 / 使用」的行号（1-based）。

    # 两级规则（都是为了不出现假绿 / 假红）

    * **类型名锚点**（`call_form=False`）：护栏 crate 之外，裸名必须**确实是护栏
      的名字**（本文件导入自 `apeireth_guard`，或写成 `apeireth_guard::X` /
      `M::X`）—— 否则 `apeireth_memory` 的 `GuardDecision` 会把护栏的
      `GuardDecision` 染成「已接生产」。
    * **方法名锚点**（`call_form=True`）：方法名**不会**被 import（它挂在对象上），
      所以要求「本文件确实碰过 `apeireth_guard`」（import 或限定路径），
      才允许裸名匹配。
    """
    bare = anchor_pattern(anchor, call_form)
    qualified = re.compile(rf"apeireth_guard\s*::\s*(?:\w+\s*::)*{re.escape(anchor)}\b")
    module_qualified = None
    if scan is not None and scan.guard_modules:
        alternatives = "|".join(
            re.escape(module) for module in sorted(scan.guard_modules)
        )
        module_qualified = re.compile(
            rf"(?:{alternatives})\s*::\s*(?:\w+\s*::)*{re.escape(anchor)}\b"
        )
    imported = bool(scan is not None and anchor in scan.guard_imports)
    inside_guard = bool(scan is not None and scan.guard_module is not None)
    touches_guard = inside_guard or imported or bool(
        scan is not None and scan.mentions_guard
    )

    hits: list[int] = []
    for number, line in enumerate(text.split("\n"), start=1):
        matched = False
        if call_form:
            # 方法名：文件碰过护栏即可裸名匹配
            matched = touches_guard and bool(bare.search(line))
        else:
            if inside_guard or imported:
                matched = bool(bare.search(line))
        if not matched and qualified.search(line):
            matched = True
        if not matched and module_qualified is not None and module_qualified.search(line):
            matched = True
        if not matched:
            continue
        if is_definition_line(line, anchor):
            continue
        hits.append(number)
    return hits


@dataclass
class CapabilityResult:
    capability: Capability
    status: str = ST_NOT_WIRED
    production_sites: list[str] = field(default_factory=list)  # 其它 crate 的生产调用点
    internal_sites: list[str] = field(default_factory=list)  # 护栏 crate 生产闭环内调用点
    test_sites: list[str] = field(default_factory=list)  # 测试 / 离线评测 / 工具


def evaluate(scans: list[FileScan]) -> list[CapabilityResult]:
    results: list[CapabilityResult] = []
    for capability in CAPABILITIES:
        result = CapabilityResult(capability=capability)
        closure = capability.production_modules
        for scan in scans:
            if capability.external_only and scan.guard_module is not None:
                continue
            for anchor in capability.anchors:
                production_hits = anchor_hits(
                    scan.production_text, anchor, capability.call_form, scan
                )
                test_hits = anchor_hits(scan.test_text, anchor, capability.call_form, scan)
                if production_hits:
                    sites = [f"{scan.relative}:{number}" for number in production_hits[:4]]
                    if scan.guard_module is None:
                        result.production_sites.extend(sites)
                    elif scan.guard_module in closure:
                        result.internal_sites.extend(sites)
                    else:
                        # 护栏 crate 内但不在生产闭环（scenario / oracle / 离线工具）
                        result.test_sites.extend(sites)
                if test_hits:
                    result.test_sites.extend(
                        f"{scan.relative}:{number}" for number in test_hits[:4]
                    )
        result.production_sites = sorted(set(result.production_sites))
        result.internal_sites = sorted(set(result.internal_sites))
        result.test_sites = sorted(set(result.test_sites))
        if result.production_sites:
            result.status = ST_PRODUCTION
        elif result.internal_sites:
            result.status = ST_INTERNAL
        elif result.test_sites:
            result.status = ST_TEST_ONLY
        else:
            result.status = ST_NOT_WIRED
        results.append(result)
    return results


def summarize(results: list[CapabilityResult]) -> dict[str, int]:
    counts = {
        ST_PRODUCTION: 0,
        ST_INTERNAL: 0,
        ST_TEST_ONLY: 0,
        ST_NOT_WIRED: 0,
    }
    for result in results:
        counts[result.status] += 1
    counts["total"] = len(results)
    return counts


def print_report(results: list[CapabilityResult]) -> None:
    counts = summarize(results)
    print("=" * 78)
    print("护栏接线覆盖率台账（`apeireth-guard` 公开能力 → 生产接线）")
    print("=" * 78)
    for status, mark in (
        (ST_PRODUCTION, "✅ 已接生产（其它 crate 的生产代码真的调用）"),
        (ST_INTERNAL, "🟡 已接生产（只在护栏 crate 自己的生产闭环里被调用）"),
        (ST_TEST_ONLY, "⚠️ 只有测试 / 离线评测在用"),
        (ST_NOT_WIRED, "❌ 完全没接（零调用点）"),
    ):
        print(f"  {mark}: {counts[status]} 项")
    print("-" * 78)
    print(f"  能力总数: {counts['total']}")
    print()
    for status in (ST_PRODUCTION, ST_INTERNAL, ST_TEST_ONLY, ST_NOT_WIRED):
        group = [result for result in results if result.status == status]
        if not group:
            continue
        print(f"【{status}】")
        for result in group:
            print(f"  - {result.capability.id}: {result.capability.title}")
            evidence = result.production_sites or result.internal_sites or result.test_sites
            for site in evidence[:3]:
                print(f"      · {site}")
            if len(evidence) > 3:
                print(f"      · …（共 {len(evidence)} 处）")
            if result.capability.note:
                print(f"      ※ {result.capability.note}")
        print()


def build_payload(results: list[CapabilityResult]) -> dict:
    return {
        "version": "guard-wiring-ledger/v1",
        "generated_at": date.today().isoformat(),
        "generator": "scripts/check_guard_wiring.py",
        "guard_crate": GUARD_CRATE,
        "note": (
            "本台账由 `python3 scripts/check_guard_wiring.py --update` 生成；"
            "「已接生产」必须有**代码锚点**支撑，手改后跑检查会因锚点对不上而退出码 1。"
        ),
        "summary": summarize(results),
        "capabilities": [
            {
                "id": result.capability.id,
                "module": result.capability.module,
                "title": result.capability.title,
                "anchors": list(result.capability.anchors),
                "scope": result.capability.scope,
                "call_form": result.capability.call_form,
                "status": result.status,
                "production_sites": result.production_sites,
                "internal_sites": result.internal_sites,
                "test_sites": result.test_sites,
                "note": result.capability.note,
            }
            for result in results
        ],
    }


def load_ledger() -> dict:
    if not LEDGER_PATH.exists():
        return {}
    try:
        return json.loads(LEDGER_PATH.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        print(f"台账 JSON 坏了：{error}", file=sys.stderr)
        return {}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", action="store_true", help="只打印覆盖率报告")
    parser.add_argument("--update", action="store_true", help="重扫锚点并重写台账")
    args = parser.parse_args()

    scans = scan_files()
    results = evaluate(scans)
    print_report(results)

    if args.update:
        payload = build_payload(results)
        LEDGER_PATH.write_text(
            json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )
        print(f"  ✓ 台账已重写：{LEDGER_PATH.relative_to(REPO_ROOT)}")
        return 0

    if args.report:
        return 0

    ledger = load_ledger()
    if not ledger:
        print("\n[check_guard_wiring] 台账缺失 ⇒ 先跑 `--update` 生成", file=sys.stderr)
        return 1

    recomputed = {result.capability.id: result for result in results}
    ledger_rows = {row["id"]: row for row in ledger.get("capabilities", []) if "id" in row}

    problems: list[str] = []
    for capability_id, row in sorted(ledger_rows.items()):
        result = recomputed.get(capability_id)
        if result is None:
            problems.append(
                f"台账里的能力 `{capability_id}` 在脚本能力表里已不存在 ⇒ 台账过期"
            )
            continue
        claimed = row.get("status")
        if claimed != result.status:
            problems.append(
                f"台账说 `{capability_id}` 是「{claimed}」，但代码锚点扫出来是"
                f"「{result.status}」⇒ **台账撒谎了**（代码被删 / 被接，或台账该改；"
                f"跑 --update 更新）"
            )
            continue
        has_production = bool(result.production_sites or result.internal_sites)
        if claimed in (ST_PRODUCTION, ST_INTERNAL):
            if not has_production:
                problems.append(
                    f"台账说 `{capability_id}`「{claimed}」但代码里已无调用点 ⇒ **台账撒谎了**"
                )
            if not (row.get("production_sites") or row.get("internal_sites")):
                problems.append(
                    f"台账说 `{capability_id}`「{claimed}」但没给任何代码锚点 ⇒ **台账撒谎了**"
                )
        if claimed == ST_TEST_ONLY and has_production:
            problems.append(
                f"台账说 `{capability_id}`「只有测试在用」，但代码里已有生产调用点 ⇒ 台账过期了"
            )
        if claimed == ST_NOT_WIRED and (
            result.production_sites or result.internal_sites or result.test_sites
        ):
            problems.append(
                f"台账说 `{capability_id}`「完全没接」，但代码里已有引用 ⇒ 台账过期了"
            )

    for capability_id in sorted(set(recomputed) - set(ledger_rows)):
        problems.append(
            f"台账漏了能力 `{capability_id}`（当前状态「{recomputed[capability_id].status}」）"
        )

    if problems:
        print("\n[check_guard_wiring] 发现问题：", file=sys.stderr)
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        return 1
    print("\nOK: 台账与代码一致")
    return 0


if __name__ == "__main__":
    sys.exit(main())
