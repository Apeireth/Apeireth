"""Phase 45 PhiProxy V2 — Owner 21:00 真生产中放入 central AI 川变.

质量 > KPI (主 17:43 实事求是哲学).
中央 AI 用 Phi-proxy 作为整合度 marker.

借鉴:
- IIT (Tononi) integrated information Φ
- Deutsch 2011 constructor theory
- 主人 17:50 "涌现 自组织"
- 主人 22:08 V2 中央 AI 完整位置
"""
from __future__ import annotations

import math
import time
from dataclasses import dataclass, field, asdict


PHI_PROXY_V2_VERSION = "0.2.0"


@dataclass
class IntegrationMeasure:
    """Φ-proxy 一次 measure — 中央 AI 整合度."""
    measure_id: str
    phi_intrinsic: float            # [0, 1] 内在整合
    phi_integration: float          # [0, 1] 系统间整合
    components: int
    mutual_info_ratio: float        # [0, 1] 互信息比
    emergence_index: float           # [0, 1] 涌现指标
    ts: float = field(default_factory=time.time)
    note: str = ""

    def to_dict(self) -> dict:
        return asdict(self)


class PhiProxyV2:
    """Φ-proxy V2 — 严谨版 (主人 17:43 实事求是).

    区别于 V1 自创公式: V2 借鉴 IIT 真生产概念 (Tononi 整合信息论).
    但 **不假装** 已实现真实 IIT — 这是 engineering approximation.
    """

    def __init__(self):
        self.history: list[IntegrationMeasure] = []

    def measure(self,
               components: int,
               mutual_info_avg: float,
               v2_alignment: float,
               vcp_4_alignment: float,
               engineering_complete: float,
               cross_domain_ratio: float) -> IntegrationMeasure:
        """计算 Φ-proxy V2.

        V2 = 0.40 * phi_intrinsic          (IIT-inspired integrated info)
          + 0.30 * emergence_index         (涌现比)
          + 0.20 * v2_alignment            (V2 哲学对齐)
          + 0.10 * vcp_4_alignment         (VCP 4 范式对齐)

        类似 IIT (Tononi): 系统整合度 ∝ 不独立性(互信息)
        边界: 不假装实现 Phenomenal — 仅是 engineering approximation
        """
        # 内在整合 (近似) = 互信息 / 系统容量
        capacity = math.log2(max(2, components))
        phi_i = min(mutual_info_avg / capacity, 1.0) if capacity > 0 else 0.0

        # 涌现指标 (近似) = 涌现收益 vs 总分
        # 当 components 增大但 total_value 增更大 = 涌现
        emergence = min((mutual_info_avg * components - 1) / (components + 1), 1.0)

        # V2 哲学对齐 = 各子组件完整还原度
        v2 = max(0.0, min(v2_alignment, 1.0))

        # VCP 4 范式对齐度
        vcp = max(0.0, min(vcp_4_alignment, 1.0))

        # 加权
        phi_intrinsic = 0.40 * phi_i + 0.30 * emergence + 0.20 * v2 + 0.10 * vcp

        m = IntegrationMeasure(
            measure_id=f"phi2_{int(time.time())}",
            phi_intrinsic=phi_i,
            phi_integration=emergence,
            components=components,
            mutual_info_ratio=mutual_info_avg,
            emergence_index=emergence,
            note=(
                f"Φ-proxy V2: intrinsic={phi_i:.3f}, emergence={emergence:.3f}, "
                f"weighted={phi_intrinsic:.4f} (不假装实现 Phenomenal, 仅 engineering approximation)"
            ),
        )
        self.history.append(m)
        return m

    def stats(self) -> dict:
        if not self.history:
            return {"n_measures": 0}
        last = self.history[-1]
        return {
            "n_measures": len(self.history),
            "latest_phi_intrinsic": round(last.phi_intrinsic, 4),
            "latest_emergence": round(last.emergence_index, 4),
            "note": (
                "Φ-proxy V2 = engineering approximation of integrated info, "
                "honest disclaimer: not real Phenomenal consciousness "
                "(主人 17:58 是终极目标, 不是已达成)"
            ),
        }


__all__ = ["PHI_PROXY_V2_VERSION", "IntegrationMeasure", "PhiProxyV2"]
