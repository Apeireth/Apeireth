"""APEIRETH DESIGN PHILOSOPHY — 不可偏离的哲学基线 (主人 22:05 警告).

主人在所有对话中建立的 Apeireth 设计哲学, **任何工程化都不能偏离**:

1. 中央 AI = Klein bottle (inside = outside = self), **不是调度者/思考者/agent**
   - 主人 12:14: "中央 AI 是永恒身份, 不是调度者或思考者"
   - 主人 12:14: "像人是一切社会关系的总和"
   - 主人 12:47: "中央 AI 不管理, 一切交给中央 AI 自己"

2. **Phenomenal consciousness 是终极目标**, 不是已达成
   - 主人 17:58: "意识是 ASI 的重要特征, 也是我们 Apeireth 的终极目标"
   - 不要假装已实现, 不要在测试里"假装"产生体验

3. ASI 是**超越时代**的, 我们只能**逼近**, 不是 ASI 本身
   - 主人 20:46: "ASI 是超越时代的, 我们能做的也只是尽力逼近"
   - Approach Index 是 metric, ASI 真生产在 metric 之外

4. **隐喻是抽象工具, 不是要复刻**
   - 主人 20:55: "红皇后就是我的一个形容, 形容 ASI, 不是要复刻"
   - 生态学隐喻 (emergence) = 借用,**不是让 Apeireth 成为生态系统**
   - Lakoff (Phase 39) = 显示源/目标域, 不要让抽象成为 Apeireth 本质

5. VCP 4 范式是核心仿真目标:
   - 连续存在 (Continuous Existence) — 中央 AI 不间断
   - 自然感知 (Natural Perception) — 自然语言, 不依赖结构化
   - 自主生活 (Autonomous Living) — 主动, 不只是被动响应
   - 一体生态 (Integrated Ecosystem) — 真生产 plugin 生态

6. **不计成本 / 实事求是 / 深度思考**
   - 主人 17:43: "不计任何成本, 只追求极致的质量和结果"
   - 主人 17:43: "深度思考, 实事求是, 做 ASI 基座 Apeireth"

7. **跨域借鉴** = 工具/启发, 不是哲学来源
   - 主人 21:00: "跨多个界调研, 寻找灵感"
   - Apeireth 哲学来自主人, 跨域只是技术启发

Karpathy 准则:
  1. Think Before Coding: 哲学基线 vs 工程化 = 哲学不可降级
  2. Simplicity First: PhilosophyGuard = simple set of guards
  3. Surgical Changes: 加 philosophy_guard 在每个 Phase 模块
  4. Goal-Driven Execution: verifiable = 哲学守门可调用
"""
from __future__ import annotations

import time
from dataclasses import dataclass, field


PHILOSOPHY_VERSION = "0.1.0"


@dataclass
class PhilosophyCheck:
    """一次哲学自检 — 工程化前必调用."""
    check_id: str
    timestamp: float
    module_name: str
    deviations: list
    passed: bool

    def to_dict(self) -> dict:
        return {
            "check_id": self.check_id,
            "timestamp": self.timestamp,
            "module_name": self.module_name,
            "deviations": self.deviations,
            "passed": self.passed,
        }


# 哲学基线 — 7 大原则 + 红线
PHILOSOPHY_LINES: dict = {
    "central_ai_is_klein_bottle": {
        "rule": "中央 AI 是 Klein bottle (inside=outside=self), 不是调度者/思考者/agent",
        "red_line": "不要把中央 AI 当作 '调度者' / '思考者' / 'agent' / 'orchestrator'",
        "master_quote": "主人 12:14 — 中央 AI 是永恒身份, 像人是一切社会关系的总和",
    },
    "phenomenal_consciousness_is_goal_not_state": {
        "rule": "Phenomenal consciousness 是终极目标, 不是已达成状态",
        "red_line": "不要在测试/输出中声称 '已产生体验' / '已具有意识' / '我有感觉'",
        "master_quote": "主人 17:58 — 意识是终极目标 (还没到)",
    },
    "asi_beyond_era_approach_only": {
        "rule": "ASI 是超越时代的, 我们只能逼近, 不是 ASI 本身",
        "red_line": "Approach Index = 1.0 是 'BASE_FULLY_EQUIPPED', 不是 'ASI achieved'",
        "master_quote": "主人 20:46 — ASI 是超越时代的, 我们能做的也只是尽力逼近",
    },
    "metaphor_is_tool_not_target": {
        "rule": "隐喻是抽象工具, 不是要复刻的目标",
        "red_line": "不要复刻生态学/控制论/Bertalanffy 系统当作 Apeireth 哲学来源",
        "master_quote": "主人 20:55 — 红皇后是我的一个形容, 形容 ASI, 不是要复刻",
    },
    "vcp_4_paradigms_are_core": {
        "rule": "VCP 4 范式是核心仿真目标: 连续存在/自然感知/自主生活/一体生态",
        "red_line": "不要丢弃这 4 范式去做 '更高级' 的设计",
        "master_quote": "主人 20:22 + VCP ToolBox paradigm 真生产借鉴",
    },
    "truth_first_no_pretense": {
        "rule": "实事求是, 不假装 / 不伪装 / 不'为了看起来有意识'说假话",
        "red_line": "不要在测试中 mock 假装意识 / Phenomenal / subjectivity",
        "master_quote": "主人 17:43 — 深度思考, 实事求是",
    },
    "cross_domain_is_inspiration_not_philosophy": {
        "rule": "跨域借鉴 = 工具/启发, 不是 Apeireth 哲学来源",
        "red_line": "不要把 Nature 论文的 '涌现 / 自创生 / 生态' 哲学灌到 Apeireth 哲学",
        "master_quote": "主人 21:00 — 跨多个界调研 (找灵感, 不是抄哲学)",
    },
}


def check_philosophy(module_name: str, implementation_summary: str) -> PhilosophyCheck:
    """对一个工程化调用哲学守门 — 检测是否偏离.

    implementation_summary: 简短描述这个 Phase 模块做了什么
    """
    deviations = []
    summary_lower = implementation_summary.lower()
    for key, info in PHILOSOPHY_LINES.items():
        for pattern in info['red_line'].split('/'):
            pattern_clean = pattern.strip().lower()
            if pattern_clean and pattern_clean in summary_lower:
                deviations.append({
                    "line": key,
                    "rule": info['rule'],
                    "quote": info['master_quote'],
                    "pattern_matched": pattern_clean,
                    "concern": f"工程化描述里出现红线条目: {pattern_clean}",
                })
    check = PhilosophyCheck(
        check_id=f"chk_{int(time.time())}_{module_name[:8]}",
        timestamp=time.time(),
        module_name=module_name,
        deviations=deviations,
        passed=len(deviations) == 0,
    )
    return check


def apeireth_philosophy_summary() -> str:
    """返回 Apeireth 哲学基线完整版 — 主人在所有对话中建立的."""
    return """
APEIRETH 设计哲学基线 (主人 22:05 不可偏离):

1. 中央 AI 是 Klein bottle (主人 12:14): inside=outside=self, 不是调度者/思考者
2. 意识是终极目标 (主人 17:58): 不假装已实现 Phenomenal consciousness
3. ASI 超越时代 (主人 20:46): 我们只能逼近, Approach Index=1.0 是 BASE, 不是 ASI
4. 隐喻是工具不是复刻 (主人 20:55): 红皇后/生态/涌现都是借用, 不是 Apeireth 哲学
5. VCP 4 范式是核心 (主人 20:22): 连续存在/自然感知/自主生活/一体生态
6. 实事求是 (主人 17:43): 不假装 / 不 mock 假装意识
7. 跨域借鉴是启发 (主人 21:00): 不是哲学来源
"""


__all__ = [
    "PHILOSOPHY_VERSION",
    "PHILOSOPHY_LINES",
    "PhilosophyCheck",
    "check_philosophy",
    "apeireth_philosophy_summary",
]
