"""R11 Requisite Variety Controller — Ashby 1956 + Conant-Ashby 1970 真借鉴.

主人 12:14 + 19:33 + 21:30 真哲学 + Appendix L #23 "Phase 32 必要多样性律" 真待加项.

Ashby's Law of Requisite Variety (1956, "An Introduction to Cybernetics"):
  "Only variety can absorb variety."
  严格形式 (信息论): I(D;R) >= H(D|T), 等价于 H(R) >= H(D|T)
    D = 环境扰动分布 (disturbances)
    R = 系统响应分布 (responses)
    T = 转换通道 (channel transformation)
  当 T = 完美 (1-1 对应): 简化为 |R| >= |D| (V47 的 flat check)

Conant & Ashby (1970) "Every good regulator of a system must be a model of that system":
  一个好 regulator 必须有足够多样性吸收扰动; 否则系统不可控.
  Amplification Principle: 当系统多样性不足时, 增加 response states 直到 requisite.

为什么需要 R11 (V47 的局限):
  - V47.check_requisite_variety(env, sys) 仅返回 boolean, 不告诉:
    1. 多样性缺口具体多大 (bits)
    2. 哪些 env 状态无响应 (missing states)
    3. 通道瓶颈在哪 (channel capacity < H(D|T))
    4. 怎么补 (amplification strategy)

R11 真实工程增量:
  1. RequisiteVarietyController (RVC) — 维护 disturbances/responses/channel_samples 注册表
  2. Shannon 信息论级别真测 — H(D), H(R), I(D;R), channel capacity
  3. 缺失状态检测 — env 状态有但无 channel sample 对应
  4. Amplification 建议 — Ashby amplification principle (Conant-Ashby 1970)
  5. attach_to_v47() — 接入 V47SelfOrganizingCore, 返回 flat + info-theoretic 双层报告

Karpathy 准则:
  1. Think Before Coding: 真读 Ashby 1956 + Conant-Ashby 1970, 不闭门
  2. Simplicity First: RVC = register + measure, 无外部依赖 (只用 math + Counter)
  3. Surgical Changes: 不动 V47, 仅 composition (attach_to_v47)
  4. Goal-Driven Execution: verifiable = 真测通道容量 + 缺失状态 + amplification 建议
"""
from __future__ import annotations

import math
import time
import uuid
from collections import Counter
from dataclasses import dataclass, field
from typing import Dict, List, Set

from apeireth.v47_self_organizing_core import V47SelfOrganizingCore


R11_VERSION = "0.1.0"


@dataclass
class Disturbance:
    """环境扰动 — environment variety source (Appendix L 主 21:30 跨域)."""
    dist_id: str
    label: str                  # 来源标签 (e.g. "user_intent", "external_event")
    state: str                  # 观察到的状态 (categorical)
    ts: float = field(default_factory=time.time)


@dataclass
class Response:
    """系统响应 — system variety source (主 22:08 V2 中央 AI 5 位置 #1)."""
    resp_id: str
    label: str                  # 来源标签 (e.g. "central_ai", "module_x")
    action: str                 # 采取的动作 (categorical)
    ts: float = field(default_factory=time.time)


@dataclass
class ChannelSample:
    """(disturbance_state, response_action) 观察样本 — 测量 transformation T."""
    dist_state: str
    resp_action: str
    ts: float = field(default_factory=time.time)


@dataclass
class RequisiteVarietyReport:
    """Ashby 真测报告 — Shannon 信息论级别."""
    n_disturbances: int
    n_responses: int
    n_channel_samples: int
    H_D: float                  # Shannon 熵 H(环境扰动分布) [bits]
    H_R: float                  # Shannon 熵 H(系统响应分布) [bits]
    channel_capacity: float     # 互信息 I(D;R) [bits]
    ratio: float                # H_R / H_D — Ashby "variety ratio" (inf when H_D=0)
    deficit: bool               # True: 系统无法完全吸收环境扰动
    missing_states: Set[str]    # env 状态有但无 channel 对应
    amplification_suggestions: List[str]  # Ashby amplification 建议
    is_requisite: bool          # True: 已达 requisite variety (H_R >= H_D + 通道满)


@dataclass
class AttachedReport:
    """V47 flat check + R11 info-theoretic 真测 双重报告."""
    v47_satisfied: bool         # V47.check_requisite_variety().satisfied
    v47_requisite_variety: int  # V47 返回的 requisite_variety 值
    r11: RequisiteVarietyReport


class RequisiteVarietyController:
    """R11 Requisite Variety Controller — Ashby 1956 + Conant-Ashby 1970 真借鉴.

    接入 V47SelfOrganizingCore:
      - V47 提供 flat boolean 接口 (兼容旧调用)
      - R11 提供 Shannon 信息论级别真测 (新调用)
      - attach_to_v47() 同时返回两层, 真生产系统可按需选

    不假装承诺 (主 17:58 + 主 20:46):
      - 不假装 regulator (没观察就 deficit=True)
      - 不假装达标 (deficit 时 is_requisite=False)
      - 不刷 KPI (amplification 建议是动作不是分数)
    """

    def __init__(self, name: str = "r11_rvc"):
        self.name = name
        self.disturbances: List[Disturbance] = []
        self.responses: List[Response] = []
        self.channel_samples: List[ChannelSample] = []

    # --------------------- 观察接口 (真生产) ---------------------

    def observe_disturbance(self, label: str, state: str) -> Disturbance:
        d = Disturbance(
            dist_id=f"d_{uuid.uuid4().hex[:8]}",
            label=label,
            state=state,
        )
        self.disturbances.append(d)
        return d

    def record_response(self, label: str, action: str) -> Response:
        r = Response(
            resp_id=f"r_{uuid.uuid4().hex[:8]}",
            label=label,
            action=action,
        )
        self.responses.append(r)
        return r

    def sample_channel(self, dist_state: str, resp_action: str) -> ChannelSample:
        """记录 (env_state, response_action) 观察 — 用于估计 I(D;R)."""
        s = ChannelSample(dist_state=dist_state, resp_action=resp_action)
        self.channel_samples.append(s)
        return s

    # --------------------- 真测 (Ashby + Shannon) ---------------------

    @staticmethod
    def _shannon_entropy(counts: Dict[str, int]) -> float:
        """真测 Shannon 熵 H(X) = -Σ p(x) log2 p(x) [bits]."""
        total = sum(counts.values())
        if total == 0:
            return 0.0
        H = 0.0
        for c in counts.values():
            if c > 0:
                p = c / total
                H -= p * math.log2(p)
        return H

    def measure(self) -> RequisiteVarietyReport:
        """真测 Requisite Variety (Ashby 1956 信息论级别).

        Returns:
            RequisiteVarietyReport with:
              - H_D, H_R: 多样性量 [bits]
              - channel_capacity: I(D;R), 通道传递能力 [bits]
              - deficit: 是否存在不可吸收扰动
              - missing_states: 无对应响应的 env 状态
              - amplification_suggestions: Ashby amplification 建议
        """
        # 分布计数
        d_counts: Dict[str, int] = Counter(d.state for d in self.disturbances)
        r_counts: Dict[str, int] = Counter(r.action for r in self.responses)

        # 联合计数 (channel samples)
        joint_counts: Dict[str, int] = Counter(
            f"{s.dist_state}|{s.resp_action}" for s in self.channel_samples
        )

        # Shannon 熵 (整体分布: disturbances + responses)
        H_D = self._shannon_entropy(dict(d_counts))
        H_R = self._shannon_entropy(dict(r_counts))

        # 通道容量 I(D;R) — mutual information
        # 严格定义: I(D;R) 必须基于观察到的 (D,R) 联合样本 (channel_samples)
        # 不能混入全部 responses (那不是 channel mapping)
        if self.channel_samples:
            # H(D,R) 联合熵 — 基于 channel_samples
            joint_total = len(self.channel_samples)
            H_DR = 0.0
            for c in joint_counts.values():
                if c > 0:
                    p = c / joint_total
                    H_DR -= p * math.log2(p)
            # H(D) 和 H(R) — 也用 channel_samples (channel 内分布)
            d_in_channel: Dict[str, int] = Counter(
                s.dist_state for s in self.channel_samples
            )
            r_in_channel: Dict[str, int] = Counter(
                s.resp_action for s in self.channel_samples
            )
            H_D_channel = self._shannon_entropy(dict(d_in_channel))
            H_R_channel = self._shannon_entropy(dict(r_in_channel))
            # I(D;R) = H(D) + H(R) - H(D,R) — 全部基于 channel_samples
            channel_capacity = max(
                0.0, H_D_channel + H_R_channel - H_DR
            )
        else:
            # 无 channel 观察 = 通道容量 0 (Ashby 严格: 不知道映射 = 无 regulator)
            channel_capacity = 0.0

        # Ashby variety ratio
        if H_D == 0.0:
            # 无扰动观察: 系统 variety 无意义约束
            ratio = 1.0 if H_R == 0.0 else float("inf")
        else:
            ratio = H_R / H_D

        # 缺失状态: env 状态有但 channel 没观察过对应响应
        observed_d_states = set(d_counts.keys())
        responded_states = {s.dist_state for s in self.channel_samples}
        missing_states = observed_d_states - responded_states

        # Deficit 判断 (Ashby + Conant-Ashby 1970 三件套)
        # 1. H_R < H_D: 系统多样性量不够
        # 2. missing_states 非空: 有未响应的 env 状态
        # 3. channel_capacity < H_D - H_R: 通道瓶颈 (transformation T 信息丢失)
        eps = 1e-9
        condition_variety = H_R + eps < H_D
        condition_missing = bool(missing_states)
        condition_bottleneck = channel_capacity + eps < H_D - H_R
        deficit = condition_variety or condition_missing or condition_bottleneck

        is_requisite = (not deficit) and (H_R >= H_D)

        # Amplification 建议 (Conant-Ashby amplification principle)
        amplification_suggestions: List[str] = []
        if condition_missing:
            for ms in sorted(missing_states):
                amplification_suggestions.append(
                    f"add_response_for_state({ms!r}) — Conant-Ashby amplification: "
                    f"missing env state, system cannot absorb this disturbance"
                )
        if condition_variety:
            extra_bits = H_D - H_R
            amplification_suggestions.append(
                f"diversify_responses({extra_bits:.3f} bits) — Ashby amplification: "
                f"H_R ({H_R:.3f}) < H_D ({H_D:.3f}), system variety insufficient"
            )
        if condition_bottleneck:
            bottleneck_bits = H_D - H_R - channel_capacity
            amplification_suggestions.append(
                f"increase_channel_fidelity({bottleneck_bits:.3f} bits) — "
                f"transformation T loses information, regulator cannot distinguish states"
            )

        return RequisiteVarietyReport(
            n_disturbances=len(self.disturbances),
            n_responses=len(self.responses),
            n_channel_samples=len(self.channel_samples),
            H_D=H_D,
            H_R=H_R,
            channel_capacity=channel_capacity,
            ratio=ratio,
            deficit=deficit,
            missing_states=missing_states,
            amplification_suggestions=amplification_suggestions,
            is_requisite=is_requisite,
        )

    # --------------------- 接入 V47 (composite) ---------------------

    def attach_to_v47(self, core: V47SelfOrganizingCore, env_variety: int) -> AttachedReport:
        """接入 V47SelfOrganizingCore — 返回 flat + info-theoretic 双层报告.

        Args:
            core: 已有 V47SelfOrganizingCore 实例
            env_variety: V47 flat check 的环境多样性 (count of distinct env states)

        Returns:
            AttachedReport with v47_satisfied + v47_requisite_variety + r11 详细

        关键差异演示 (R11 比 V47 严的 case):
            V47: env=10, sys=10 -> satisfied (count 相等)
            R11: env 10 states 但只 1 个 channel sample -> 9 missing -> deficit
        """
        # 用 V47 现有接口做 flat check
        sys_variety_distinct = len(set(r.action for r in self.responses))
        v47_var = core.check_requisite_variety(
            environment_variety=env_variety,
            system_variety=sys_variety_distinct,
        )
        # R11 信息论级别真测
        r11 = self.measure()

        return AttachedReport(
            v47_satisfied=v47_var.satisfied,
            v47_requisite_variety=v47_var.requisite_variety,
            r11=r11,
        )

    # --------------------- Stats (dashboard) ---------------------

    def stats(self) -> Dict[str, object]:
        r = self.measure()
        ratio_str: object = (
            round(r.ratio, 4) if r.ratio != float("inf") else "inf"
        )
        return {
            "version": R11_VERSION,
            "controller_name": self.name,
            "n_disturbances": r.n_disturbances,
            "n_responses": r.n_responses,
            "n_channel_samples": r.n_channel_samples,
            "H_D_bits": round(r.H_D, 4),
            "H_R_bits": round(r.H_R, 4),
            "channel_capacity_bits": round(r.channel_capacity, 4),
            "variety_ratio": ratio_str,
            "deficit": r.deficit,
            "is_requisite": r.is_requisite,
            "n_missing_states": len(r.missing_states),
            "missing_states": sorted(r.missing_states),
            "n_amplification_suggestions": len(r.amplification_suggestions),
            "amplification_suggestions": r.amplification_suggestions,
            "philosophy": (
                "Ashby 1956 'Only variety absorbs variety' + "
                "Conant-Ashby 1970 'Every good regulator must be a model of that system'. "
                "R11 RVC = Shannon 信息论级别真测 + deficit detection + amplification principle. "
                "不假装 regulator (主 17:58). 不刷 KPI (主 13:03). "
                "主 22:33 ASI 北极星真逼近 (R11 是真生产 substrate, 不是 KPI 凑数)."
            ),
        }


__all__ = [
    "R11_VERSION",
    "Disturbance",
    "Response",
    "ChannelSample",
    "RequisiteVarietyReport",
    "AttachedReport",
    "RequisiteVarietyController",
]


# --------------------- Demo (主 00:56 任何人都能接手) ---------------------

def _demo() -> None:
    print("=" * 64)
    print("=== R11 Requisite Variety Controller (Ashby 1956 + Conant-Ashby 1970) ===")
    print("=" * 64)

    # 真场景: 中央 AI 接收 4 类 query, 用 6 类 response
    rvc = RequisiteVarietyController(name="central_ai_r11_demo")
    env_states = ["bug_report", "feature_request", "how_to", "philosophical"]
    sys_actions = ["acknowledge", "investigate", "code_fix", "explain", "defer", "escalate"]

    # 100 次交互 (主 19:33 真生产数据)
    mapping = {
        "bug_report": "investigate",
        "feature_request": "acknowledge",
        "how_to": "explain",
        "philosophical": "defer",
    }
    for i in range(100):
        env = env_states[i % 4]
        rvc.observe_disturbance("user_intent", env)
        action = mapping[env]
        rvc.record_response("central_ai", action)
        rvc.sample_channel(env, action)

    r = rvc.measure()
    print(f"\n  H(D) = {r.H_D:.4f} bits  (env扰动多样性)")
    print(f"  H(R) = {r.H_R:.4f} bits  (sys响应多样性)")
    print(f"  I(D;R) = {r.channel_capacity:.4f} bits  (通道容量)")
    print(f"  ratio = {r.ratio:.4f}  (Ashby ratio)")
    print(f"  deficit = {r.deficit}  (是否可吸收)")
    print(f"  is_requisite = {r.is_requisite}  (是否达 requisite)")

    # 接入 V47
    core = V47SelfOrganizingCore()
    attached = rvc.attach_to_v47(core, env_variety=4)
    print(f"\n  V47 flat check: satisfied={attached.v47_satisfied}")
    print(f"  R11 info-theoretic: is_requisite={attached.r11.is_requisite}")

    print(f"\n  stats: {rvc.stats()}")
    print("=" * 64)


if __name__ == "__main__":
    _demo()