"""Cross-small-model CI: HQB task suite (R9-DevOps / R9-DEV-001).

HQB 4 维 (主 18:52 + HARNESS.md §2.3 真借鉴):
  - SC 自洽性: 同一 task 重复 → score 方差
  - NR 抗噪性: 同语义不同扰动 (typo/中英混/礼貌) → score 稳定性
  - EV 可演化性: 跨轮 (prev vs next) score lift
  - CDT 跨域迁移: 跨 4 域 (code/math/reasoning/创意) 跑同一模型

主 19:33 走在前人经验上: 借鉴 MMLU 2021 + GSM8K 2021 + HumanEval 2021 + EleutherAI LM-Eval 2021
+ V36 HQB benchmark 真测函数 + V160 HQB 4 dims.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Dict, List, Optional


class TaskDomain(str, Enum):
    """HQB CDT 跨域迁移 4 域 (主 18:52 HARNESS §2.3)."""
    CODE = "code"
    MATH = "math"
    REASONING = "reasoning"
    CREATIVE = "creative"


@dataclass
class HQBTask:
    """HQB 真测任务. 主 17:43 实事求是: 真任务, 不假装."""

    task_id: str
    domain: TaskDomain
    prompt: str
    expected_keywords: List[str] = field(default_factory=list)
    difficulty: str = "easy"  # easy / medium / hard

    def to_dict(self) -> Dict[str, Any]:
        return {
            "task_id": self.task_id,
            "domain": self.domain.value,
            "prompt": self.prompt,
            "expected_keywords": self.expected_keywords,
            "difficulty": self.difficulty,
        }


# ---------------------------------------------------------------------------
# 真测任务集 (主 19:33 借鉴 GSM8K/HumanEval/MMLU 模式)
# ---------------------------------------------------------------------------
DEFAULT_TASKS: List[HQBTask] = [
    # ---- code 域 ----
    HQBTask(
        task_id="code-001-hello",
        domain=TaskDomain.CODE,
        prompt="Write a Python function that returns the string 'hello world'.",
        expected_keywords=["def", "return", "hello"],
        difficulty="easy",
    ),
    HQBTask(
        task_id="code-002-sum",
        domain=TaskDomain.CODE,
        prompt="Write a Python function that sums two integers a and b.",
        expected_keywords=["def", "return", "a", "b"],
        difficulty="easy",
    ),
    HQBTask(
        task_id="code-003-factorial",
        domain=TaskDomain.CODE,
        prompt="Write a Python function to compute factorial of n.",
        expected_keywords=["def", "factorial", "return"],
        difficulty="medium",
    ),

    # ---- math 域 ----
    HQBTask(
        task_id="math-001-plus",
        domain=TaskDomain.MATH,
        prompt="What is 17 + 25?",
        expected_keywords=["42"],
        difficulty="easy",
    ),
    HQBTask(
        task_id="math-002-mult",
        domain=TaskDomain.MATH,
        prompt="Compute 13 * 7.",
        expected_keywords=["91"],
        difficulty="easy",
    ),
    HQBTask(
        task_id="math-003-word",
        domain=TaskDomain.MATH,
        prompt="If a train travels 60 km/h for 2.5 hours, how far does it go?",
        expected_keywords=["150", "km"],
        difficulty="medium",
    ),

    # ---- reasoning 域 ----
    HQBTask(
        task_id="reason-001-why",
        domain=TaskDomain.REASONING,
        prompt="Why is the sky blue? Explain briefly.",
        expected_keywords=["scatter", "light", "atmosphere"],
        difficulty="medium",
    ),
    HQBTask(
        task_id="reason-002-how",
        domain=TaskDomain.REASONING,
        prompt="How does a refrigerator work? Give 3 steps.",
        expected_keywords=["1", "2", "3", "cold"],
        difficulty="medium",
    ),

    # ---- creative 域 ----
    HQBTask(
        task_id="create-001-list",
        domain=TaskDomain.CREATIVE,
        prompt="List three creative uses for an empty cardboard box.",
        expected_keywords=["1", "2", "3"],
        difficulty="easy",
    ),
    HQBTask(
        task_id="create-002-story",
        domain=TaskDomain.CREATIVE,
        prompt="Tell a one-sentence story about a robot learning to dream.",
        expected_keywords=["robot", "dream"],
        difficulty="medium",
    ),
]


def get_tasks_by_domain(domain: TaskDomain) -> List[HQBTask]:
    """CDT 跨域迁移真测: 拿一个域的全部任务."""
    return [t for t in DEFAULT_TASKS if t.domain == domain]


def nr_variants(prompt: str) -> List[str]:
    """NR 抗噪性: 同一 prompt 的扰动版 (typo/中英混/礼貌/大小写)."""
    return [
        prompt,                                    # 原始
        prompt.upper(),                            # 大写
        prompt.lower(),                            # 全小写
        prompt + " Please.",                       # 礼貌后缀
        "Please " + prompt.lower(),                # 礼貌前缀
        prompt.replace("a", "@").replace("e", "3"),  # 字符替换扰动
    ]
