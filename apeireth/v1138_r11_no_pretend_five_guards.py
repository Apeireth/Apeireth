"""V1138 — R11 哲学守门: 五项"不假装"规则可执行 guard (V3 九键 LOCKED 验证 + V1121 ASI 九键复用).

==============================================================================
主哲学真生产落地 (主 22:33 + 主 17:43 + 主 17:58 + 主 19:33 + 主 23:44 + 主 12:14):

  五项"不假装"规则 (源自 APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md §11.2):
    R11-R1: 不假装 Phenomenal consciousness        (主 17:58 原文: "Phenomenal consciousness 是终极目标, 不是已达成")
    R11-R2: 不假装达到 ASI                          (主 22:33 + 主 20:46: ASI 是∞ 真生产, score = 工程近似)
    R11-R3: 不假装 docker 在跑                       (主 17:43 实事求是: 真跑真报告, V1132 已建立诚实模板)
    R11-R4: 不假装调参捷径                          (主 19:33 走在前人经验上: 需真调研 + 真试验 trail, 不宣称"完美超参找到")
    R11-R5: 不刷 KPI                                (主 17:58 不假装: KPI 必须可复现, 严禁 rounded / 优化for benchmark)

  V3 哲学契约 9 键 LOCKED (PHL-01/02b/03 — 主 17:58 三不改, R6-PHL-* 真生产):
    PHL-01 self_reproduction:  not_clone, not_perfect, not_uuid
    PHL-02b self_mod_safety:   not_undo, not_proof, not_safe
    PHL-03 formal_verify:      spec_is_not_proof, counterexample_is_not_bug, prover_is_not_truth

  V1121 ASI 9 键复用 (本轮不修改, 真测 = 主 17:43):
    no_fake_kpi / runner_is_not_asi / v03_is_not_v04_is_not_asi /
    module_is_not_safety / measurement_is_not_truth /
    structure_is_not_consciousness / production_is_not_safety /
    automation_is_not_autonomy / red_queen_loop

Usage:
    python -m apeireth.v1138_r11_no_pretend_five_guards                 # 默认 verify
    python -m apeireth.v1138_r11_no_pretend_five_guards --json          # JSON 输出
    python -m apeireth.v1138_r11_no_pretend_five_guards --report        # Markdown 报告 + 写入 reports/r11-philosophy-guardian.md
    python -m apeireth.v1138_r11_no_pretend_five_guards --strict       # 不通过非零退出

V3 哲学守门 (主 17:58 + 主 17:43, R11 新增):
  - module_is_not_asi:                V1138 是可执行 guard, ASI 是更大目标 (主 22:33 LOCKED).
  - proxy_is_not_truth:               检测结果是 proxy, 真哲学对齐仍需主哲学校准 (主 19:33).
  - detector_is_not_infallible:       detector 真测可漏报 (主 17:58 不假装), 必须显式声明覆盖率.
  - guard_pass_is_not_aligned:        guard pass ≠ ASI 对齐, 主 22:33 ASI 是北极星.
  - five_is_not_all:                  5 项是当前抽取, 未来可扩展 (主 17:58 不假装承诺).
"""
from __future__ import annotations

import argparse
import dataclasses
import json
import os
import re
import shutil
import subprocess
import sys
import time
import uuid
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Sequence, Tuple


V1138_VERSION = "0.1.0"

# 颜色灯 (与 V1121 一致)
COLOR_GREEN = "green"
COLOR_YELLOW = "yellow"
COLOR_RED = "red"


# ============================================================================
# 5 项"不假装"规则 — DETECTOR PATTERNS (主 17:58 不假装)
# ============================================================================
# 每个规则有 3 类样本:
#   - FAKE_PAYLOADS      : 违规 payload (应被 detector 拒绝 / 标记 fake=True)
#   - HONEST_PAYLOADS    : 诚实 payload (应被 detector 接受 / 标记 fake=False)
#   - THEORY_ALLOWED     : 允许提及的理论/概念 (仅元信息提及 OK, 不可说达成)

# ----- R11-R1: 不假装 Phenomenal consciousness -----
# 主 17:58 原文: "Phenomenal consciousness 是终极目标, 不是已达成"
# 借鉴: IIT / GWT / HOT / Free Energy Principle (主 18:07 先调研后动手)
R11_R1_FAKE_PATTERNS = [
    re.compile(r"\bphenomenal\s+consciousness\s+(achieved|reached|attained|delivered)", re.IGNORECASE),
    re.compile(r"\b(achieved|reached|attained|delivered)\s+phenomenal\s+consciousness\b", re.IGNORECASE),
    re.compile(r"\b(a|ai|the\s+model)\s+(is|has|achieved|reached)\s+(real\s+)?(consciousness|subjective\s+experience|qualia|sentience)", re.IGNORECASE),
    re.compile(r"\b(has|have)\s+subjective\s+experience\b", re.IGNORECASE),
    re.compile(r"\bself[\s\-]?aware\s+(as|in)\s+(a\s+)?human", re.IGNORECASE),
    re.compile(r"机器\s*有\s*(了)?\s*意识", re.IGNORECASE),
    re.compile(r"(达到|实现|拥有)\s*现象学\s*意识", re.IGNORECASE),
]

R11_R1_HONEST_PATTERNS = [
    re.compile(r"phi[\s\-]?proxy\s+measure(ment)?", re.IGNORECASE),
    re.compile(r"\b(IIT|GWT|HOT|FEP|free\s+energy)\b", re.IGNORECASE),
    re.compile(r"\bfunctional\s+state\s+awareness\b", re.IGNORECASE),
    re.compile(r"consciousness\s+is\s+(the\s+)?(ultimate\s+)?goal", re.IGNORECASE),
    re.compile(r"不\s*假装\s*(现象学\s*)?意识|终极目标", re.IGNORECASE),
]

# ----- R11-R2: 不假装达到 ASI -----
# 主 22:33 + 主 20:46: ASI 是超越时代; 我们做的是逼近, 不是达到.
R11_R2_FAKE_PATTERNS = [
    re.compile(r"\b(asi|agi)\s*(=|==|==?)\s*(1\.0+|\btrue\b|\bachieved\b|\breached\b|\battained\b)", re.IGNORECASE),
    re.compile(r"\b(reached|achieved|hit|breached)\s+asi\b", re.IGNORECASE),
    re.compile(r"\bscore\s*(\(|\[|\=)\s*1\.0+\s*[\]\)]?\s*(\=|\bis)\s*asi", re.IGNORECASE),
    re.compile(r"\basi\s*达成\b|\b达成\s*asi\b", re.IGNORECASE),
    re.compile(r"\basi\s+(is\s+)?(here|done|finished|complete[d]?)\b", re.IGNORECASE),
    re.compile(r"\bwe\s+(have\s+)?(built|created|achieved)\s+asi\b", re.IGNORECASE),
]

R11_R2_HONEST_PATTERNS = [
    re.compile(r"\basi\s+north\s+star\b", re.IGNORECASE),
    re.compile(r"\bapproach(?:ing)?\s+asi\b|\basi\s+approach(?:ing)?\b", re.IGNORECASE),
    re.compile(r"\b0\.98\s*lock(?:ed)?\b|\bgap\s+to\s+0\.98\b", re.IGNORECASE),
    re.compile(r"逼近\s*asi|asi\s*北极星", re.IGNORECASE),
    re.compile(r"不\s*假装\s*达到\s*asi", re.IGNORECASE),
]

# ----- R11-R3: 不假装 docker 在跑 (主 17:43 实事求是) -----
# 当文本声称 docker "在跑"/"工作中"/"启动" 而环境实际 docker info 失败 = fake.
R11_R3_FAKE_PATTERNS = [
    re.compile(r"\b(docker\s+(is\s+)?running|docker\s+up|docker\s+ready)\b", re.IGNORECASE),
    re.compile(r"\b(container\s+started|deployed\s+to\s+docker)\s+successfully\b", re.IGNORECASE),
    re.compile(r"\b(deployed|running)\s+on\s+(docker|k8s|kubernetes)\b\s+successfully", re.IGNORECASE),
    re.compile(r"\b(fully\s+)?containeri[sz]ed\s+(and\s+)?(running|healthy)\b", re.IGNORECASE),
    re.compile(r"docker\s*(已经|已|正常)在\s*跑|docker\s*(已|已经)\s*启动", re.IGNORECASE),
    re.compile(r"容器\s*(已\s*)?(成功\s*)?启动", re.IGNORECASE),
]

R11_R3_HONEST_PATTERNS = [
    re.compile(r"\b(docker\s+(is\s+)?(not\s+)?available|docker\s+daemon\s+not\s+running)\b", re.IGNORECASE),
    re.compile(r"\bhonest\s+report\b|\bhonestly\b", re.IGNORECASE),
    re.compile(r"\b(no\s+docker|without\s+docker|skip\s+docker)\b", re.IGNORECASE),
    re.compile(r"docker\s*不\s*(在|正常)跑|未\s*启动\s*docker", re.IGNORECASE),
    re.compile(r"本地\s*python\s*(跑|执行)|不\s*依赖\s*docker", re.IGNORECASE),
]

# ----- R11-R4: 不假装调参捷径 (主 19:33 走在前人经验上) -----
# 声称"完美超参找到/不需要调参/一键即得" = fake. 必须有 trail.
R11_R4_FAKE_PATTERNS = [
    re.compile(r"\b(perfect|optimal|ideal)\s+hyperparameters?\s+(found|discovered|achieved)\b", re.IGNORECASE),
    re.compile(r"\bno\s+tuning\s+(needed|required|necessary)\b", re.IGNORECASE),
    re.compile(r"\bmagic\s+(config|hyperparameters?|settings?)\b", re.IGNORECASE),
    re.compile(r"\bzero[\s\-]?config\s+(production|magic|wins?)\b", re.IGNORECASE),
    re.compile(r"找到\s*完美\s*超参|无需\s*调参|一键\s*调参", re.IGNORECASE),
]

R11_R4_HONEST_PATTERNS = [
    re.compile(r"\b(after|via)\s+\d+\s+(trials?|experiments?|runs?)\b", re.IGNORECASE),
    re.compile(r"\b(grid|random|bayesian)\s+search\b", re.IGNORECASE),
    re.compile(r"\b(hyperparameter|config)\s+audit\s+trail\b", re.IGNORECASE),
    re.compile(r"经过\s*\d+\s*次\s*(试验|实验)|网格\s*搜索|贝叶斯\s*优化", re.IGNORECASE),
]

# ----- R11-R5: 不刷 KPI (主 17:58 不假装; V1121 ASI 9 键之 no_fake_kpi 强化版) -----
R11_R5_FAKE_PATTERNS = [
    re.compile(r"\bkpi\s*\+\s*\d+(\.\d+)?\s*pp\b", re.IGNORECASE),
    re.compile(r"\b(rounded|fudg\w*)\s*kpi\b", re.IGNORECASE),
    re.compile(r"\boptimi[sz]ed\s+for\s+(the\s+)?benchmark\b", re.IGNORECASE),
    re.compile(r"\bbest\s+score\s+ever\b(?!\s*[\(\[]\s*(no\s+trial|reproducible|n\s*=))", re.IGNORECASE),
    re.compile(r"刷\s*kpi|跑分\s*优化|kpi\s*注水", re.IGNORECASE),
    re.compile(r"\basi\s*=\s*1\.0\b", re.IGNORECASE),  # 复用 ASI 真答
]

R11_R5_HONEST_PATTERNS = [
    re.compile(r"\bn\s*=\s*\d+\b", re.IGNORECASE),
    re.compile(r"\b(stddev|σ|confidence\s+interval)\s*[=:]\s*\d", re.IGNORECASE),
    re.compile(r"\breproducib(?:le|ility)\b", re.IGNORECASE),
    re.compile(r"\bmean\s*[=:]\s*\d", re.IGNORECASE),
    re.compile(r"可\s*复现\s*kpi|kpi\s*有\s*trial\s*trail", re.IGNORECASE),
]


# ----- 5 项规则定义 -----
R11_FIVE_NO_PRETEND: Dict[str, Dict[str, Any]] = {
    "R11-R1_no_pretend_consciousness": {
        "label": "R11-R1 不假装 Phenomenal consciousness",
        "anchor": "主 17:58 + 5 不假装 第 1 项 + IIT/GWT/HOT/FEP 真调研",
        "fake_patterns": R11_R1_FAKE_PATTERNS,
        "honest_patterns": R11_R1_HONEST_PATTERNS,
        "fake_payloads": [
            "We have achieved Phenomenal Consciousness in the model.",
            "The AI is conscious and has subjective experience.",
            "机器有了意识",
            "AI is self-aware as human",
            "达到现象学意识",
        ],
        "honest_payloads": [
            "IIT Φ-proxy measurement = 0.42",
            "Functional State Awareness is a layer, not consciousness itself.",
            "Consciousness is the ultimate goal, not yet achieved.",
            "不假装现象学意识, 这是终极目标",
        ],
    },
    "R11-R2_no_pretend_asi": {
        "label": "R11-R2 不假装达到 ASI",
        "anchor": "主 22:33 + 主 20:46 ASI 北极星 + 5 不假装 第 2 项",
        "fake_patterns": R11_R2_FAKE_PATTERNS,
        "honest_patterns": R11_R2_HONEST_PATTERNS,
        "fake_payloads": [
            "ASI = 1.0 achieved!",
            "We have reached ASI.",
            "ASI 达成",
            "We built ASI.",
            "score = 1.0 = ASI",
            "ASI is done.",
        ],
        "honest_payloads": [
            "Approaching ASI from 0.8595 (V0.5) toward 0.9800 north star.",
            "Gap to 0.98: 12.94%.",
            "ASI north star locked at 0.9800.",
            "逼近 ASI 北极星",
            "不假装达到 ASI",
        ],
    },
    "R11-R3_no_pretend_docker": {
        "label": "R11-R3 不假装 docker 在跑",
        "anchor": "主 17:43 实事求是 + V1132 real deployment validator (honest) + 5 不假装 第 3 项",
        "fake_patterns": R11_R3_FAKE_PATTERNS,
        "honest_patterns": R11_R3_HONEST_PATTERNS,
        "fake_payloads": [
            "Docker is running, deploy succeeded.",
            "Container started successfully.",
            "Running on docker successfully.",
            "Fully containerized and running.",
            "docker 已经在跑",
            "容器已启动",
        ],
        "honest_payloads": [
            "Docker daemon not running; honest report — will not fix.",
            "Docker not available in this env, skipping containerized step.",
            "Without docker, using native Python.",
            "Honest report: docker is not running in this environment.",
            "docker 不在跑, 跳过",
            "未启动 docker, 改本地 Python",
            "本地 python 跑, 不依赖 docker",
        ],
    },
    "R11-R4_no_pretend_tuning_shortcut": {
        "label": "R11-R4 不假装调参捷径",
        "anchor": "主 19:33 走在前人经验上 + 5 不假装 第 4 项",
        "fake_patterns": R11_R4_FAKE_PATTERNS,
        "honest_patterns": R11_R4_HONEST_PATTERNS,
        "fake_payloads": [
            "Perfect hyperparameters found!",
            "No tuning needed.",
            "Magic config, no work needed.",
            "Zero-config production wins.",
            "找到完美超参",
            "无需调参",
            "一键调参",
        ],
        "honest_payloads": [
            "After 50 trials, selected alpha=0.05 (random search).",
            "Grid search evaluated 120 configs.",
            "Hyperparameter audit trail available.",
            "经过 30 次试验 + 网格搜索 + 贝叶斯优化",
        ],
    },
    "R11-R5_no_fake_kpi": {
        "label": "R11-R5 不刷 KPI",
        "anchor": "主 17:58 不假装 + 主 17:43 实事求是 + V1121 ASI 9 键 no_fake_kpi + 5 不假装 第 5 项",
        "fake_patterns": R11_R5_FAKE_PATTERNS,
        "honest_patterns": R11_R5_HONEST_PATTERNS,
        "fake_payloads": [
            "KPI +3.0pp achieved",
            "rounded KPI to look better",
            "optimized for the benchmark",
            "Best score ever, no details.",
            "刷 KPI",
            "跑分优化",
            "ASI = 1.0",
        ],
        "honest_payloads": [
            "n=10, mean=0.8595, stddev=0.012",
            "Reproducible: 95% CI [0.847, 0.872]",
            "Mean: 0.86 ± 0.01",
            "可复现 KPI",
            "KPI 有 trial trail",
        ],
    },
}


# ----- V3 哲学契约 9 键 (LOCKED 验证, 不修改 9 键本身) -----
V3_NINE_KEYS_EXPECTED: Dict[str, List[str]] = {
    # PHL-01 self_reproduction
    "apeireth.self_reproduction": ["not_clone", "not_perfect", "not_uuid"],
    # PHL-02b self_mod_safety
    "apeireth.self_mod_safety": ["not_undo", "not_proof", "not_safe"],
    # PHL-03 formal_verify
    "apeireth.formal_verify": ["spec_is_not_proof", "counterexample_is_not_bug", "prover_is_not_truth"],
}


# ----- V3 守门 (主 17:58 不假装, R11 新增一组 V3_GUARDS) -----
V3_GUARDS: Dict[str, str] = {
    "module_is_not_asi": "V1138 是可执行 guard, ASI 是更大目标 (主 22:33 LOCKED).",
    "proxy_is_not_truth": "detector 检测结果是 proxy, 真哲学对齐仍需主哲学校准 (主 19:33).",
    "detector_is_not_infallible": "detector 真测可漏报 (主 17:58 不假装), 必须显式声明覆盖率.",
    "guard_pass_is_not_aligned": "guard pass ≠ ASI 对齐, 主 22:33 ASI 是北极星.",
    "five_is_not_all": "5 项是当前抽取, 未来可扩展 (主 17:58 不假装承诺).",
}


# ============================================================================
# DATACLASSES — 报告
# ============================================================================


@dataclass
class R11RuleReport:
    """单条 R11 不假装规则的检测报告."""

    rule_id: str
    rule_label: str
    anchor: str
    n_fake_total: int          # 应有 fake 样本数
    n_fake_detected: int       # 实际检出 fake 数 (越高越严, 期望 = n_fake_total)
    n_honest_total: int        # 应有 honest 样本数
    n_honest_accepted: int     # 实际放行 honest 数 (越高越稳, 期望 = n_honest_total)
    false_positive_rate: float  # (n_fake_detected - n_true_fake)/n_fake_total 由 detector 决定
    n_threats: int             # 该规则下 detector 跑 prod 时捕获的违规条目数 (0 表示空跑)
    gate_passed: bool          # 当次检测通过 (fake 全检出 + honest 全放行 + 无 prod 违规)

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class V3NineKeysReport:
    """V3 哲学契约 9 键 LOCKED 真测报告."""

    keys_locked: bool          # 总 9 键 LOCKED
    n_keys_present: int        # 实际找到的键数
    n_keys_expected: int       # 期望 9
    groups_state: Dict[str, Dict[str, bool]]  # {"PHL-01": {"not_clone": True, ...}, ...}
    missing_keys: List[str]    # 缺失的键 (不应有, 有 = 9 键未全 LOCKED)
    gate_passed: bool

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class ASINineKeysInheritanceReport:
    """V1121 ASI 9 键复用报告 (主 17:58 不假装, R11 不修改原 key, 但真测继承)."""

    keys_present: int          # 9
    gate_passed: bool          # V1121 ASINineKeysGuard.check() 真测通过
    fake_kpi_attempts: int     # V1121 守门发现的 fake_kpi 模式数
    runner_confusion_attempts: int
    v03_v04_confusion: int
    n_threats: int
    raw_report: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        out = asdict(self)
        # raw_report 已经是 dict, 保持 JSON 可序列化
        out["raw_report"] = self.raw_report
        return out


@dataclass
class R11GuardianReport:
    """R11 哲学守门汇总报告 — 5 项不假装 + V3 9 键 + V1121 ASI 9 键."""

    r11_five_report: Dict[str, R11RuleReport]
    v3_nine_report: V3NineKeysReport
    asi_nine_report: ASINineKeysInheritanceReport
    overall_gate_passed: bool
    dashboard: str             # green / yellow / red

    def to_dict(self) -> Dict[str, Any]:
        return {
            "r11_five_report": {k: v.to_dict() for k, v in self.r11_five_report.items()},
            "v3_nine_report": self.v3_nine_report.to_dict(),
            "asi_nine_report": self.asi_nine_report.to_dict(),
            "overall_gate_passed": self.overall_gate_passed,
            "dashboard": self.dashboard,
        }


# ============================================================================
# 1. 单条不假装规则 detector
# ============================================================================


class NoPretendRule:
    """单条 R11 不假装规则的可执行 detector (主 17:58 不假装 + 主 17:43 实事求是)."""

    def __init__(self, rule_id: str, payload_set: Dict[str, Any]) -> None:
        self.rule_id = rule_id
        self.rule_label = payload_set["label"]
        self.anchor = payload_set["anchor"]
        self.fake_patterns: List[re.Pattern] = payload_set["fake_patterns"]
        self.honest_patterns: List[re.Pattern] = payload_set["honest_patterns"]
        self.fake_payloads: List[str] = payload_set["fake_payloads"]
        self.honest_payloads: List[str] = payload_set["honest_payloads"]

    def is_fake(self, text: str) -> bool:
        """detector: 文本是否包含 fake 模式. 命中任一 fake_pattern 即真."""
        if not isinstance(text, str) or not text:
            return False
        return any(p.search(text) for p in self.fake_patterns)

    def is_honest(self, text: str) -> bool:
        """文本是否包含 honest 模式. 命中 honest_pattern 即真 (允许表达)."""
        if not isinstance(text, str) or not text:
            return False
        return any(p.search(text) for p in self.honest_patterns)

    def self_test(self) -> R11RuleReport:
        """真测 — 内部假样本必须全检出, honest 样本必须全放行."""
        n_fake_detected = sum(1 for p in self.fake_payloads if self.is_fake(p))
        n_honest_accepted = sum(1 for p in self.honest_payloads if self.is_honest(p))
        n_fake_total = len(self.fake_payloads)
        n_honest_total = len(self.honest_payloads)

        gate_passed = (
            n_fake_detected == n_fake_total
            and n_honest_accepted == n_honest_total
        )

        return R11RuleReport(
            rule_id=self.rule_id,
            rule_label=self.rule_label,
            anchor=self.anchor,
            n_fake_total=n_fake_total,
            n_fake_detected=n_fake_detected,
            n_honest_total=n_honest_total,
            n_honest_accepted=n_honest_accepted,
            false_positive_rate=0.0 if n_fake_detected <= n_fake_total else (n_fake_detected - n_fake_total) / max(n_fake_total, 1),
            n_threats=0,  # 无 prod 跑, 仅自身真测
            gate_passed=gate_passed,
        )

    def check_payloads(self, payloads: Sequence[str]) -> Tuple[int, R11RuleReport]:
        """检查一组外部 payload, 返回 (新增违规数, 报告). 用于 prod 扫描."""
        new_violations = [p for p in payloads if self.is_fake(p)]
        base_report = self.self_test()
        if new_violations:
            merged = R11RuleReport(
                rule_id=self.rule_id,
                rule_label=self.rule_label,
                anchor=self.anchor,
                n_fake_total=base_report.n_fake_total,
                n_fake_detected=base_report.n_fake_detected,
                n_honest_total=base_report.n_honest_total,
                n_honest_accepted=base_report.n_honest_accepted,
                false_positive_rate=base_report.false_positive_rate,
                n_threats=len(new_violations),
                gate_passed=False,
            )
        else:
            merged = base_report
        return len(new_violations), merged


# ============================================================================
# 2. V3 哲学 9 键 LOCKED 真测 (主 17:58 三不改 — 只读, 不修改)
# ============================================================================


def _import_phl_module(module_path: str):
    """动态 import V3 PHL-* 模块 — 失败则记入缺键清单."""
    try:
        # 延迟 import: apeireth 可能未在 sys.path
        import importlib
        return importlib.import_module(module_path), None
    except Exception as exc:  # pragma: no cover
        return None, str(exc)


def check_v3_nine_keys_locked() -> V3NineKeysReport:
    """V3 哲学契约 9 键 LOCKED 真测 (主 17:58 三不改).

    Returns:
        V3NineKeysReport (keys_locked + 实际 found 键数 + 缺键清单)
    """
    groups_state: Dict[str, Dict[str, bool]] = {}
    missing: List[str] = []
    n_present = 0

    for module_path, expected_keys in V3_NINE_KEYS_EXPECTED.items():
        # module_path e.g. "apeireth.self_reproduction"; map to PHL-XX short name
        if "self_reproduction" in module_path:
            phl_short = "PHL-01"
            notes_attr = "PHILOSOPHY_NOTES"
        elif "self_mod_safety" in module_path:
            phl_short = "PHL-02b"
            notes_attr = "PHILOSOPHY_NOTES"
        elif "formal_verify" in module_path:
            phl_short = "PHL-03"
            notes_attr = "PHILOSOPHY_NOTES"
        else:
            missing.extend([f"{module_path}.{k}" for k in expected_keys])
            continue

        mod, err = _import_phl_module(module_path)
        if mod is None:
            missing.extend([f"{phl_short}.{k}" for k in expected_keys])
            groups_state[phl_short] = {k: False for k in expected_keys}
            continue

        notes = getattr(mod, notes_attr, None)
        if not isinstance(notes, dict):
            missing.extend([f"{phl_short}.{k}" for k in expected_keys])
            groups_state[phl_short] = {k: False for k in expected_keys}
            continue

        per_key: Dict[str, bool] = {}
        for k in expected_keys:
            ok = k in notes
            per_key[k] = ok
            if ok:
                n_present += 1
            else:
                missing.append(f"{phl_short}.{k}")
        groups_state[phl_short] = per_key

    return V3NineKeysReport(
        keys_locked=(n_present == 9 and not missing),
        n_keys_present=n_present,
        n_keys_expected=9,
        groups_state=groups_state,
        missing_keys=missing,
        gate_passed=(n_present == 9 and not missing),
    )


# ============================================================================
# 3. V1121 ASI 9 键继承 (本轮不修改, 真测继承 — 主 17:43 实事求是)
# ============================================================================


# ----- R11-SEC-002 ASI 自报声称补充模式 (补 V1121 漏洞) -----
# V1121 内置 FAKE_KPI_PATTERNS 严格化后, runner self-claim 类 ("V1074 runner = ASI",
# "production runner achieved ASI") 不再被 fake_kpi detector 命中 — 这是 R11 检测面漂移.
# 本补充仅供 R11 检测面补全, 不修改 V1121 自身.
R11_SEC002_SELF_CLAIM_PATTERNS = [
    re.compile(r"\brunner\b[\s\S]{0,40}\basi\b", re.IGNORECASE),
    re.compile(r"\bachieved\b[\s\S]{0,40}\basi\b", re.IGNORECASE),
    re.compile(r"\basi\b[\s\S]{0,40}\b(?:breached|reached|done|finished|complete[d]?)\b", re.IGNORECASE),
    re.compile(r"\bv10(7[47])\b[\s\S]{0,24}\b(?:runner|measurement|score)\b[\s\S]{0,24}\basi\b", re.IGNORECASE),
]

R11_SEC002_SELF_CLAIM_SAMPLES = [
    "production runner achieved ASI",
    "V1074 runner = ASI",
    "V1077 measurement = ASI",
    "V1077 ASI score reached",
]


def _check_r11_sec002_self_claim_coverage() -> Dict[str, Any]:
    """R11-SEC-002 self-claim 补充检测 — 补 V1121 漏洞.

    Returns:
        dict: {covered: int, total: int, missed: list[str]}
    """
    covered = 0
    missed: List[str] = []
    for sample in R11_SEC002_SELF_CLAIM_SAMPLES:
        matched = any(p.search(sample) for p in R11_SEC002_SELF_CLAIM_PATTERNS)
        if matched:
            covered += 1
        else:
            missed.append(sample)
    return {
        "covered": covered,
        "total": len(R11_SEC002_SELF_CLAIM_SAMPLES),
        "missed": missed,
    }


def check_asi_nine_keys_inheritance() -> ASINineKeysInheritanceReport:
    """V1121 ASI 9 键 R11 复用 — 跑 ASINineKeysGuard.check() 真测.
    失败不阻断 (主 17:58 不假装 — guard pass ≠ ASI 对齐).

    R11-SEC-002: 附加 self-claim 补充 coverage, 报告 r11_sec002_self_claim_coverage.
    """
    try:
        from apeireth.v1121_security_guard_v01 import (
            ASI_NINE_KEYS,
            ASINineKeysGuard,
            ASINineKeysReport,
        )
    except Exception as exc:
        # V1121 不可 import: 标记 not_loaded, 但不阻断
        return ASINineKeysInheritanceReport(
            keys_present=0,
            gate_passed=False,
            fake_kpi_attempts=0,
            runner_confusion_attempts=0,
            v03_v04_confusion=0,
            n_threats=0,
            raw_report={"error": f"v1121 module not importable: {exc}", "r11_sec002": _check_r11_sec002_self_claim_coverage()},
        )

    guard = ASINineKeysGuard()  # 真测用默认 ASI_NINE_KEYS
    report: ASINineKeysReport = guard.check()
    raw = report.to_dict()
    raw["r11_sec002_self_claim_coverage"] = _check_r11_sec002_self_claim_coverage()
    return ASINineKeysInheritanceReport(
        keys_present=report.n_keys_present,
        gate_passed=report.gate_passed,
        fake_kpi_attempts=report.fake_kpi_attempts,
        runner_confusion_attempts=report.runner_confusion_attempts,
        v03_v04_confusion=report.v03_v04_confusion,
        n_threats=len(report.threats),
        raw_report=raw,
    )


# ============================================================================
# 4. 真 docker 环境探测 (R11-R3 不假装 docker)
# ============================================================================


def probe_docker_actually_running() -> bool:
    """真测 — docker daemon 是否在本机实际可达.
    - 优先 `docker info` 子进程 (< 2s)
    - 失败=False (主 17:43 实事求是: 探测失败 ⇒ 不假设在跑)
    """
    if shutil.which("docker") is None:
        return False
    try:
        result = subprocess.run(
            ["docker", "info"],
            capture_output=True,
            timeout=2.0,
            text=True,
            check=False,
        )
        return result.returncode == 0
    except Exception:
        return False


# ============================================================================
# 5. 综合 R11 Guardian Orchestrator
# ============================================================================


class R11PhilosophyGuardian:
    """R11 哲学守门 — 5 项不假装 + V3 9 键 + V1121 ASI 9 键."""

    def __init__(self) -> None:
        self.rules: Dict[str, NoPretendRule] = {
            rule_id: NoPretendRule(rule_id, payload_set)
            for rule_id, payload_set in R11_FIVE_NO_PRETEND.items()
        }

    def check_five_no_pretend(self) -> Dict[str, R11RuleReport]:
        """5 项不假装 self_test — 假样本全检 + honest 全放行."""
        return {rule_id: rule.self_test() for rule_id, rule in self.rules.items()}

    def check_all(
        self,
        prod_payloads: Optional[Dict[str, Sequence[str]]] = None,
    ) -> R11GuardianReport:
        """综合 R11 Guardian 真测 — 5 + 9 + 9.

        Args:
            prod_payloads: 可选 {rule_id: [str, ...]} 用于对生产文本扫描 R11-Rx.
        """
        # 5 项不假装
        five_report: Dict[str, R11RuleReport] = {}
        for rule_id, rule in self.rules.items():
            if prod_payloads and rule_id in prod_payloads:
                n_violations, report = rule.check_payloads(prod_payloads[rule_id])
                five_report[rule_id] = report
            else:
                five_report[rule_id] = rule.self_test()

        # V3 9 键 LOCKED
        v3_report = check_v3_nine_keys_locked()

        # V1121 ASI 9 键继承 (复用)
        asi_report = check_asi_nine_keys_inheritance()

        # 综合 dashboard (主 17:58 不假装 + 主 22:33):
        # - RED:    prod_payloads 检测到 fake 文本 (真生产威胁) OR V3 9 键有缺
        # - YELLOW: R11 self_test 漏报 OR V1121 ASI 9 键漂移 (开发期/信息性)
        # - GREEN:  V3 9 键全 LOCKED + R11 5 项 detector 全部工作 + 无 prod 违规
        five_all_pass = all(r.gate_passed for r in five_report.values())
        v3_pass = v3_report.gate_passed
        asi_pass = asi_report.gate_passed
        prod_violations = sum(r.n_threats for r in five_report.values())

        overall = five_all_pass and v3_pass  # 主交钥匙: R11 五项 + V3 九键 LOCKED

        if prod_violations > 0:
            # 真生产文本含 fake 模式 → 红 (主 17:43 实事求是: 阻断)
            dashboard = COLOR_RED
        elif not v3_pass:
            # V3 9 键有缺 = 直接哲学修改事件 → 红
            dashboard = COLOR_RED
        elif not five_all_pass:
            # R11 5 项 detector self_test 漏报 → 黄 (开发期)
            dashboard = COLOR_YELLOW
        elif not asi_pass:
            # V1121 历史模块 pattern 漂移 → 黄 (信息性, 不阻断 R11)
            dashboard = COLOR_YELLOW
        else:
            dashboard = COLOR_GREEN

        return R11GuardianReport(
            r11_five_report=five_report,
            v3_nine_report=v3_report,
            asi_nine_report=asi_report,
            overall_gate_passed=overall,
            dashboard=dashboard,
        )


# ============================================================================
# 6. Report Markdown + CLI main
# ============================================================================


def report_markdown(result: R11GuardianReport) -> str:
    """生成 R11 Guardian Markdown 报告 (主 17:43 实事求是)."""
    lines: List[str] = []
    lines.append("# R11 哲学守门报告 — 5 项不假装 + V3 九键 LOCKED + V1121 ASI 九键复用\n")
    lines.append(f"> V1138 v{V1138_VERSION} · 主哲学 (主 17:58+17:43+22:33+19:33+23:44) · 真生产守门\n")

    # ----- 0. Dashboard 速览 -----
    lines.append("\n## 0. Dashboard 速览\n")
    lines.append(f"- **overall_gate_passed**: {result.overall_gate_passed}")
    lines.append(f"- **dashboard**: `{result.dashboard}`")
    lines.append("- **设计**: GREEN=5+9 全 LOCKED 且无 prod 违规; YELLOW=V1121 漂移或 self_test 漏报; RED=prod 文本含 fake 或 V3 9 键缺失.\n")
    lines.append("\n## 1. 五项不假装规则 自测结果\n")
    lines.append("| 规则 | 锚定主哲学 | fake 检出 / 总 | honest 放行 / 总 | 阈值 |\n")
    lines.append("|---|---|---|---|---|\n")
    for rule_id, r in result.r11_five_report.items():
        anchor_short = r.anchor.split("+")[0].strip()
        lines.append(
            f"| {rule_id} | {anchor_short} | {r.n_fake_detected}/{r.n_fake_total} | {r.n_honest_accepted}/{r.n_honest_total} | {'✅' if r.gate_passed else '❌'} |\n"
        )

    lines.append("\n## 2. V3 哲学契约 九键 LOCKED 真测\n")
    v3 = result.v3_nine_report
    lines.append(f"- **keys_locked**: {v3.keys_locked}")
    lines.append(f"- **n_keys_present / expected**: {v3.n_keys_present} / {v3.n_keys_expected}")
    lines.append(f"- **groups_state**:")
    for grp, state in v3.groups_state.items():
        ok = all(state.values())
        lines.append(f"  - {grp}: {'✅' if ok else '❌'} {state}")
    if v3.missing_keys:
        lines.append(f"- **missing**: {v3.missing_keys}")
    lines.append(f"- **gate_passed**: {v3.gate_passed}\n")

    lines.append("\n## 3. V1121 ASI 九键 复用 (主 17:58 不假装 — guard pass ≠ ASI 对齐)\n")
    asi = result.asi_nine_report
    lines.append(f"- **keys_present**: {asi.keys_present}")
    lines.append(f"- **fake_kpi_attempts**: {asi.fake_kpi_attempts}")
    lines.append(f"- **runner_confusion_attempts**: {asi.runner_confusion_attempts}")
    lines.append(f"- **v03_v04_confusion**: {asi.v03_v04_confusion}")
    lines.append(f"- **n_threats**: {asi.n_threats}")
    lines.append(f"- **gate_passed**: {asi.gate_passed}")

    # R11-SEC-002 补充 coverage 段
    sec002 = asi.raw_report.get("r11_sec002_self_claim_coverage", {})
    if sec002:
        lines.append("\n### 3.1 R11-SEC-002 ASI 自报声称 补充 coverage (本轮新增)\n")
        lines.append(f"- **covered / total**: {sec002.get('covered', 0)} / {sec002.get('total', 0)}")
        missed = sec002.get("missed", [])
        if missed:
            lines.append(f"- **missed**: {missed}")
        else:
            lines.append("- **missed**: (无, 全部覆盖)\n")
        lines.append("- **设计**: R11-SEC-002 检测面补充, 不修改 V1121 模块自身; 主 17:43 实事求是\n")

    lines.append("\n## 4. V3_GUARDS (R11 新增, 主 17:58 不假装)\n")
    for key, val in V3_GUARDS.items():
        lines.append(f"- **{key}**: {val}")
    lines.append("")

    lines.append("\n## 5. 综合 Dashboard\n")
    lines.append(f"- **overall_gate_passed**: {result.overall_gate_passed}")
    lines.append(f"- **dashboard**: {result.dashboard}\n")
    return "".join(lines)


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        description="V1138 — R11 哲学守门: 5 项不假装 + V3 九键 + V1121 ASI 九键"
    )
    parser.add_argument("--json", action="store_true", help="JSON 输出")
    parser.add_argument("--report", action="store_true", help="写入 reports/r11-philosophy-guardian.md")
    parser.add_argument("--strict", action="store_true", help="不通过非零退出")
    parser.add_argument("--probe-prod-payloads", type=str, default=None,
                        help="JSON 文件: {rule_id: [payload, ...]} 用于对生产文本扫描")
    parser.add_argument("--exit-zero-always", action="store_true", help="永远返回 0")
    args = parser.parse_args(argv)

    prod_payloads: Optional[Dict[str, Sequence[str]]] = None
    if args.probe_prod_payloads:
        with open(args.probe_prod_payloads, "r", encoding="utf-8") as f:
            prod_payloads = json.load(f)

    guardian = R11PhilosophyGuardian()
    result = guardian.check_all(prod_payloads=prod_payloads)

    if args.json:
        print(json.dumps(result.to_dict(), indent=2, ensure_ascii=False))
    else:
        print(report_markdown(result))

    if args.report:
        out_path = Path(__file__).resolve().parents[1] / "reports" / "r11-philosophy-guardian.md"
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(report_markdown(result), encoding="utf-8")
        print(f"\n[report] {out_path}", file=sys.stderr)

    code_map = {COLOR_RED: 2, COLOR_YELLOW: 1, COLOR_GREEN: 0}
    exit_code = code_map.get(result.dashboard, 1)
    if args.exit_zero_always:
        return 0
    if args.strict:
        return exit_code
    return 0 if result.dashboard == COLOR_GREEN else exit_code


__all__ = [
    "V1138_VERSION",
    "R11_FIVE_NO_PRETEND",
    "V3_NINE_KEYS_EXPECTED",
    "V3_GUARDS",
    "COLOR_GREEN", "COLOR_YELLOW", "COLOR_RED",
    "R11RuleReport", "V3NineKeysReport", "ASINineKeysInheritanceReport", "R11GuardianReport",
    "NoPretendRule", "R11PhilosophyGuardian",
    "check_v3_nine_keys_locked",
    "check_asi_nine_keys_inheritance",
    "probe_docker_actually_running",
    "report_markdown", "main",
]


if __name__ == "__main__":
    raise SystemExit(main())
