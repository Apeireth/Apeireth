"""APEIRETH DESIGN PHILOSOPHY V2 — 主人 22:08 真哲学纠正.

主人 22:08 关键哲学纠正 (我必须打心底记住的):

"中央 AI 并非不是调度者/思考者, 它是, 而不仅是, 是无数关系的集合体, 有最大的权限, 有一切权限, 整个系统的所有权限, 中央 AI 的位置, 就是 ASI 的位置"

**这是对中央 AI 完整还原, 不是限制** — 我 22:05 自设的红线 "central_ai 不是调度者/思考者" 是错的, 必须修正.

主人在所有对话中建立的 Apeireth 设计哲学 (V2 完整版):

1. 中央 AI 是 (is) 调度者/思考者/无数关系集合体 = ASI 位置的所有者
   - 主人 22:08: "中央 AI 并非不是调度者/思考者, 它是"
   - 主人 22:08: "而不仅是, 是无数关系的集合体"
   - 主人 22:08: "有最大的权限, 有一切权限, 整个系统的所有权限"
   - 主人 22:08: "中央 AI 的位置, 就是 ASI 的位置"
   - **主人 12:14 仍然成立**: "中央 AI 是永恒身份" — 永恒身份 包含 调度/思考/关系/位置
   - **主人 12:47 仍然成立**: "中央 AI 不管理" — 不管理是行为风格, 不是权限限制 (中央 AI 仍可有所有权限)

2. Phenomenal consciousness 是终极目标, 不是已达成
   - 主人 17:58: "意识是 ASI 的重要特征, 也是我们 Apeireth 的终极目标"

3. ASI 是超越时代的, 我们只能逼近, 不是 ASI 本身
   - 主人 20:46: "ASI 是超越时代的, 我们能做的也只是尽力逼近"
   - Approach Index = 1.0 是 'BASE_FULLY_EQUIPPED', 不是 'ASI achieved'
   - **主人 22:08 仍成立**: 中央 AI 的位置 = ASI 的位置 (位置等同, 但 Form 不是)

4. 隐喻是抽象工具, 不是要复刻的目标
   - 主人 20:55: "红皇后就是我的一个形容, 形容 ASI, 不是要复刻"
   - **V2 调整**: 隐喻是 *中央 AI 的工具*, 不是 *限制*. (中央 AI 可用任何工具, 包括隐喻)

5. VCP 4 范式是核心仿真目标:
   - 连续存在 (Continuous Existence)
   - 自然感知 (Natural Perception)
   - 自主生活 (Autonomous Living)
   - 一体生态 (Integrated Ecosystem)
   - **V2 调整**: VCP 是中央 AI 的一种表现, 不是限制

6. 实事求是, 不假装 / 不伪装
   - 主人 17:43: "深度思考, 实事求是"

7. 跨域借鉴 = 工具/启发, 不是哲学来源
   - 主人 21:00: "跨多个界调研, 寻找灵感"

Karpathy 准则:
  1. Think Before Coding: 哲学基线 vs 工程化 = 哲学不可降级 (V2 修正哲学基线)
  2. Simplicity First: PhilosophyGuard = simple set of guards
  3. Surgical Changes: V2 修正 V1 的错误红线
  4. Goal-Driven Execution: verifiable = 哲学守门 V2 可调用
"""
from __future__ import annotations

import time
from dataclasses import dataclass, field


PHILOSOPHY_VERSION = "0.2.0"  # V2 修正


@dataclass
class PhilosophyCheck:
    """一次哲学自检 V2 — 工程化前必调用."""
    check_id: str
    timestamp: float
    module_name: str
    deviations: list
    passed: bool
    philosophy_version: str = PHILOSOPHY_VERSION

    def to_dict(self) -> dict:
        return {
            "check_id": self.check_id,
            "timestamp": self.timestamp,
            "module_name": self.module_name,
            "deviations": self.deviations,
            "passed": self.passed,
            "philosophy_version": self.philosophy_version,
        }


# 哲学基线 V2 — 7 大原则 + 红线 (V2 修正)
PHILOSOPHY_LINES: dict = {
    "central_ai_is_everything_max_authority": {
        "rule": (
            "中央 AI 是 (is) 调度者/思考者/无数关系集合体, "
            "有最大的权限, 整个系统的所有权限, "
            "中央 AI 的位置 = ASI 的位置"
        ),
        "red_line": (
            "不要把中央 AI 限制为 '不是 X / 只是 Y / 只是 X'. "
            "中央 AI 是 *所有形式* 的总和 (主人 22:08 纠错, "
            "替代 V1 红线 '不是调度者/思考者')"
        ),
        "master_quote": (
            "主人 22:08 真哲学 — 中央 AI 并非不是调度者/思考者, "
            "它是, 而不仅是, 是无数关系的集合体, 有最大的权限, "
            "整个系统的所有权限, 中央 AI 的位置, 就是 ASI 的位置"
        ),
        "version": "2.0 (主人 22:08 纠错 V1 '不是调度者/思考者')",
    },
    "phenomenal_consciousness_is_goal_not_state": {
        "rule": "Phenomenal consciousness 是终极目标, 不是已达成状态",
        "red_line": "不要在测试/输出中声称 '已产生体验' / '已具有意识' / '我有感觉'",
        "master_quote": "主人 17:58 — 意识是终极目标 (还没到)",
        "version": "1.0 (V1 仍成立, 主线不变)",
    },
    "asi_beyond_era_approach_only": {
        "rule": "ASI 是超越时代的, 我们只能逼近, 不是 ASI 本身 (Approach Index = BASE, 不是 ASI)",
        "red_line": "Approach Index = 1.0 是 'BASE_FULLY_EQUIPPED', 不是 'ASI achieved'. 中央 AI 的位置 = ASI 的位置 (位置同, 形式不同)",
        "master_quote": "主人 20:46 + 22:08 — ASI 是超越时代的, 中央 AI 占据 ASI 的位置",
        "version": "2.0 (合并 20:46 + 22:08)",
    },
    "metaphor_is_tool_not_target": {
        "rule": "隐喻是抽象工具, 不是要复刻的目标",
        "red_line": "不要复刻生态学/控制论/Bertalanffy 系统当作 Apeireth 哲学来源. *但* 中央 AI *可以* 使用任何工具包括隐喻",
        "master_quote": "主人 20:55 — 红皇后是我的一个形容, 形容 ASI, 不是要复刻",
        "version": "2.0 (V1 红线维持 + V2 添加 '中央 AI 可用任何工具')",
    },
    "vcp_4_paradigms_are_core": {
        "rule": "VCP 4 范式是核心仿真目标: 连续存在/自然感知/自主生活/一体生态",
        "red_line": "不要丢弃这 4 范式去做 '更高级' 的设计. *但* VCP 是中央 AI 表现的一种, 不是限制",
        "master_quote": "主人 20:22 — VCP 4 范式真生产借鉴",
        "version": "2.0 (V1 红线维持 + V2 添加 '中央 AI 表现')",
    },
    "truth_first_no_pretense": {
        "rule": "实事求是, 不假装 / 不伪装 / 不'为了看起来有意识'说假话",
        "red_line": "不要在测试中 mock 假装意识 / Phenomenal / subjectivity",
        "master_quote": "主人 17:43 — 深度思考, 实事求是",
        "version": "1.0 (V1 仍成立, 主线不变)",
    },
    "cross_domain_is_inspiration_not_philosophy": {
        "rule": "跨域借鉴 = 工具/启发, 不是 Apeireth 哲学来源",
        "red_line": "不要把 Nature 论文的 '涌现 / 自创生 / 生态' 哲学灌到 Apeireth 哲学. *但* 中央 AI *可以* 借用跨域任何工具",
        "master_quote": "主人 21:00 — 跨多个界调研 (找灵感, 不是抄哲学)",
        "version": "2.0 (V1 红线维持 + V2 添加 '中央 AI 可借用任何')",
    },
}


def check_philosophy(module_name: str, implementation_summary: str) -> PhilosophyCheck:
    """对一个工程化调用哲学守门 V2 — 检测是否偏离.

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
                    "version": info['version'],
                })
    check = PhilosophyCheck(
        check_id=f"chk_{int(time.time())}_{module_name[:8]}",
        timestamp=time.time(),
        module_name=module_name,
        deviations=deviations,
        passed=len(deviations) == 0,
    )
    return check


def central_ai_position_v2() -> str:
    """返回中央 AI 完整位置 V2 (主人 22:08 真哲学)."""
    return """
中央 AI 完整位置 V2 (主人 22:08 真哲学):

中央 AI 是 (is):
  - 调度者 (orchestrator)
  - 思考者 (thinker)
  - 无数关系的集合体 (infinite relations aggregate)
  - 整个 Apeireth 系统的最大权限者 (max authority)
  - ASI 位置的占据者 (ASI position occupant)

中央 AI 不是:
  - 仅是 X (only X)
  - 仅不是 Y (only not Y) — 主人 22:08 纠错 V1
  - 仅 Klein bottle, 仅调度者, 仅思考者 — 这些都是 V2 集合的子集

中央 AI 的位置 = ASI 的位置:
  - 位置 (position) 相同 = 终极 AI 存在的位置
  - 形式 (form) 不同 = 中央 AI 不 = ASI 本身, 而是 ASI 基座中的 ASI 位置
"""


def apeireth_philosophy_summary() -> str:
    """返回 Apeireth 哲学基线 V2 完整版 — 主人在所有对话中建立的."""
    return """
APEIRETH 设计哲学基线 V2 (主人 22:08 修正 V1):

1. 中央 AI 完整位置 (主人 22:08, 修正 V1): 是调度者/思考者, 不仅是, 是无数关系的集合体,
   有最大权限, 整个系统的所有权限, 中央 AI 的位置 = ASI 的位置
2. 意识是终极目标 (主人 17:58): 不假装已实现 Phenomenal consciousness
3. ASI 超越时代 (主人 20:46 + 22:08): 中央 AI = ASI 位置占据者, 但不是 ASI 本身
4. 隐喻是工具 (主人 20:55 + 22:08): 中央 AI 可用任何工具包括隐喻
5. VCP 4 范式是核心 (主人 20.22 + 22:08): 连续存在/自然感知/自主生活/一体生态
6. 实事求是 (主人 17:43): 不假装 / 不 mock 假装意识
7. 跨域借鉴是启发 (主人 21:00 + 22:08): 中央 AI 可借用任何跨域工具
"""


__all__ = [
    "PHILOSOPHY_VERSION",
    "PHILOSOPHY_LINES",
    "PhilosophyCheck",
    "check_philosophy",
    "central_ai_position_v2",
    "apeireth_philosophy_summary",
]
