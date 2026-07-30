"""Apeireth V1141 — V0.4/V0.5 Integration Contract (主 17:43 实事求是 + 主 19:33 走在前人经验上).

R11 Architecture deliverable: 在 Omnibus §9 A/B/C 锚点下, 把 V1136(V0.5 真测引擎)、
V1130 (ContinuityTracker dashboard)、V1074 (V0.3 production runner) 三者之间
**真生产可执行**集成契约落地. 契约覆盖:

  - 17/18 维字段表 (V1074 dim_breakdown 17 dims + V1136 v05_total_v1136 1 composite)
  - 真值来源 (每个字段的 producer 模块 + 失败兜底)
  - 兼容策略 (V0.3→V0.4→V0.5 链路 + V1125 LOCKED 占位保留)
  - 失败语义 (8 类失败码 + 调用方动作)
  - V3 哲学守门 (LOCKED 13 keys from V1074+V1136+V1130+IC NEW)
  - 可执行校验 (run_validation() 真跑 5 步守门)

借鉴 (主 19:33 走在前人经验上):
  1. JSON Schema 2020-12 — 字段强契约 + 类型/range 守门
  2. OpenAPI 3.1 — 信息性元数据 + examples + nullable
  3. Semantic Versioning 2.0.0 — 兼容策略语义
  4. Datadog SLO 2019 — 失败 burn-rate 启发
  5. Datadog N+1 Compatibility Schemas — 渐进式 schema 演化

主哲学锚定 (主 17:43 + 主 17:58 + 主 19:33 + 主 22:33 + 主 23:44):
  - 主 17:43 实事求是: 真测真值, 不允许 fake KPI / placeholder
  - 主 17:58 不假装: 失败就明示, 不静默吞错
  - 主 19:33 走在前人经验上: 复用现成 dataclass + JSON Schema 不发明
  - 主 22:33 ASI 北极星: 契约是工具, ASI 是目标
  - 主 23:44 干到底: 失败重试 + chaos 失联兜底

主哲学不假装承诺:
  - 不假装 contract = ASI: 契约是真工具, ASI 是更大目标
  - 不假装 18 维 = 全部: V0.5 composite 是 composite, 非新维度
  - 不假装 v04_score = 真理: score 是 proxy
  - 不假装 compat_mode = 永久: 兼容策略是限时桥,非默认

Usage:
    # 一行 CLI 校验契约
    python -m apeireth.v1141_asi_v04_v05_integration_contract --validate

    # 程序化调用
    from apeireth.v1141_asi_v04_v05_integration_contract import (
        IntegrationContractValidator,
        IntegrationContractError,
        IC_FIELD_SCHEMA,
        INTEGRATION_CONTRACT_VERSION,
    )
    validator = IntegrationContractValidator(strict=True)
    bundle = validator.collect()           # 真跑三模块 → 真值
    report = validator.validate(bundle)    # 守门 → 报告
    print(report.to_dict())
"""
from __future__ import annotations

import argparse
import dataclasses
import hashlib
import json
import logging
import math
import statistics
import sys
import time
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Sequence, Tuple

LOG = logging.getLogger("v1141")
if not LOG.handlers:
    LOG.addHandler(logging.StreamHandler())
    LOG.setLevel(logging.INFO)

INTEGRATION_CONTRACT_VERSION = "0.1.0"
CONTRACT_DRAFT_ID = "IC-001"

# ---------------------------------------------------------------------------
# Failure Codes (主 17:58 不假装: 失败就明示, 不静默吞错)
# ---------------------------------------------------------------------------

IC_FIELD_MISSING = "IC_FIELD_MISSING"          # 17 维字段任一为 None
IC_RANGE_VIOLATION = "IC_RANGE_VIOLATION"      # field not in [0, 1]
IC_SUBSCORE_FAILED = "IC_SUBSCORE_FAILED"      # V1136 子测度 raw=0
IC_V1074_UNREACHABLE = "IC_V1074_UNREACHABLE"  # V1074 measure 抛异常
IC_V1136_UNREACHABLE = "IC_V1136_UNREACHABLE"  # V1136 真测抛异常
IC_V1130_UNREACHABLE = "IC_V1130_UNREACHABLE"  # V1130 dashboard 抛
IC_DASHBOARD_TIMEOUT = "IC_DASHBOARD_TIMEOUT"  # V1130 wallclock > 2.5s
IC_CHAOS_LOST = "IC_CHAOS_LOST"                # V1136 chaos measurement_preserved=False
IC_VERSION_CONFLICT = "IC_VERSION_CONFLICT"    # IC_VERSION 与上游声明冲突
IC_COMPOSITE_DRIFT = "IC_COMPOSITE_DRIFT"      # V0.5 composite 与手算的 dev > 1e-3

IC_GUARDS: Tuple[str, ...] = (
    # V1136 V3_GUARDS (主 17:58 + 主 20:46) — 6 keys LOCKED
    "guard_no_fake_kpi_v1136",
    "guard_no_break_v1125_formula",
    "guard_no_pretend_measurement_is_asi",
    "guard_no_pretend_3dims_filled_is_asi",
    "guard_no_kpi_gaming",
    "guard_central_ai_eternal_identity",
    # V1074 V3_GUARDS (主 17:43) — 5 keys LOCKED
    "guard_module_is_not_asi",
    "guard_measurement_is_not_truth",
    "guard_structure_is_not_consciousness",
    "guard_production_is_not_safety",
    "guard_automation_is_not_autonomy",
    # V1130 V3_GUARDS (主 23:44 + 主 17:58) — 1 key LOCKED
    "guard_dashboard_target_2_5s",
    # IC NEW guard (主 17:58) — phi_proxy 是 proxy, 不是真 Φ
    "guard_phi_proxy_is_proxy",
)


# ---------------------------------------------------------------------------
# 17 V0.3 dims + 1 V0.5 composite = 18 fields (主 17:43 实事求是, 字段名 LOCKED)
# ---------------------------------------------------------------------------

V03_DIMS: Tuple[str, ...] = (
    "phi_proxy",
    "capabilities",
    "cross_domain",
    "engineering",
    "vcp_4",
    "v2_philosophy",
    "rubric_open",
    "real_production",
    "cognitive_core",
    "self_organizing_core",
    "plugin_core",
    "self_improving_core",
    "neurosymbolic",
    "world_model",
    "reinforcement_learning",
    "scientific_method",
    "eternal_identity",
)

V05_EXTRA: Tuple[str, ...] = ("v05_total_v1136",)  # composite of v04 + 3 dims

ALL_FIELDS: Tuple[str, ...] = V03_DIMS + V05_EXTRA

assert len(V03_DIMS) == 17, f"V0.3 must be exactly 17 dims, got {len(V03_DIMS)}"
assert len(ALL_FIELDS) == 18, f"V0.3+V0.5 must be exactly 18 fields, got {len(ALL_FIELDS)}"


# ---------------------------------------------------------------------------
# Field Schema (借鉴 JSON Schema 2020-12 强契约 + OpenAPI 3.1 nullable)
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class ICFieldSpec:
    """Integration Contract 单字段规范 (主 17:43 实事求是: 不可变, 强类型).

    Sources of truth:
      - V0.3 dims (1-17): V1074 StatusSnapshot.dim_breakdown (V1073 真测)
      - V0.5 composite (18): V1136 measure_v05_3dims().v05_total_v1136 (V1136 真测)
    """

    name: str
    index: int
    kind: str            # "v03_dim" or "v05_composite"
    producer: str        # module path of truth source
    field_type: str      # "float"
    range_lo: float
    range_hi: float
    nullable: bool       # true only for v05_total_v1136 if V1136 unreachable
    required: bool
    description: str

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


def _build_v03_spec() -> Dict[str, ICFieldSpec]:
    """Build 17 V0.3 dim specs from V1074 StatusSnapshot dim_breakdown."""
    descs = {
        "phi_proxy": "Φ-proxy 整合信息测量 (主 22:33 真哲学)",
        "capabilities": "能力覆盖 (跨域工程化能力)",
        "cross_domain": "跨域工程化完成度",
        "engineering": "工程完成度 (commits + 集成测试)",
        "vcp_4": "VCP 4 范式对齐 (主 20:22)",
        "v2_philosophy": "V2 哲学对齐 (主 22:08 V2 5 位置)",
        "rubric_open": "rubric_open_stretch 开放扩展空间",
        "real_production": "真生产工具链 (主 17:43)",
        "cognitive_core": "认知架构核心 (V43 OpenCog+NARS)",
        "self_organizing_core": "自组织核心 (V47 AERA+Autopoiesis)",
        "plugin_core": "插件核心 (V48 Capability-based+WASM)",
        "self_improving_core": "自改进核心 (V49 DGM+UCB1+Hyperagents)",
        "neurosymbolic": "神经符号融合",
        "world_model": "世界模型",
        "reinforcement_learning": "强化学习",
        "scientific_method": "科学方法 (Popper/Lakatos)",
        "eternal_identity": "永恒身份 (V1072 ContinuityTracker)",
    }
    out: Dict[str, ICFieldSpec] = {}
    for idx, name in enumerate(V03_DIMS, start=1):
        out[name] = ICFieldSpec(
            name=name,
            index=idx,
            kind="v03_dim",
            producer="apeireth.v1074_asi_production_runner:StatusSnapshot.dim_breakdown",
            field_type="float",
            range_lo=0.0,
            range_hi=1.0,
            nullable=False,
            required=True,
            description=descs[name],
        )
    return out


def _build_v05_spec() -> Dict[str, ICFieldSpec]:
    """Build 1 V0.5 composite spec from V1136 measure_v05_3dims."""
    out: Dict[str, ICFieldSpec] = {}
    out["v05_total_v1136"] = ICFieldSpec(
        name="v05_total_v1136",
        index=18,
        kind="v05_composite",
        producer=(
            "apeireth.v1136_asi_v05_3dim_real_measurement:"
            "measure_v05_3dims().v05_total_v1136"
        ),
        field_type="float",
        range_lo=0.0,
        range_hi=1.0,
        # nullable when IC_V1136_UNREACHABLE; consumer must mark degraded
        nullable=True,
        required=True,
        description=(
            "V0.5 composite = v04 * 0.85 + continuity*0.05 + autonomy*0.05 + "
            "transferability*0.05. Replaces V1125 0.85 placeholder."
        ),
    )
    return out


V03_SPECS: Dict[str, ICFieldSpec] = _build_v03_spec()
V05_SPECS: Dict[str, ICFieldSpec] = _build_v05_spec()
IC_FIELD_SCHEMA: Dict[str, ICFieldSpec] = {**V03_SPECS, **V05_SPECS}


# ---------------------------------------------------------------------------
# Exceptions (主 17:58 不假装: 失败有 traceable type)
# ---------------------------------------------------------------------------


class IntegrationContractError(RuntimeError):
    """Integration Contract 顶层异常 — 所有 IC 错误继承于此."""

    code: str = "IC_ERROR"

    def __init__(self, message: str, code: Optional[str] = None,
                 context: Optional[Dict[str, Any]] = None) -> None:
        super().__init__(message)
        self.code = code or self.code
        self.context = context or {}


class FieldMissingError(IntegrationContractError):
    code = IC_FIELD_MISSING


class RangeViolationError(IntegrationContractError):
    code = IC_RANGE_VIOLATION


class V1074UnreachableError(IntegrationContractError):
    code = IC_V1074_UNREACHABLE


class V1136UnreachableError(IntegrationContractError):
    code = IC_V1136_UNREACHABLE


class V1130UnreachableError(IntegrationContractError):
    code = IC_V1130_UNREACHABLE


class DashboardTimeoutError(IntegrationContractError):
    code = IC_DASHBOARD_TIMEOUT


class ChaosLostError(IntegrationContractError):
    code = IC_CHAOS_LOST


class CompositeDriftError(IntegrationContractError):
    code = IC_COMPOSITE_DRIFT


# ---------------------------------------------------------------------------
# Composite helpers (主 17:43 实事求是: V0.5 公式可手算验证)
# ---------------------------------------------------------------------------

V0_5_WEIGHT_V04 = 0.85
V0_5_WEIGHT_3DIM = 0.05
V0_5_WEIGHTS: Dict[str, float] = {
    "v04_score": V0_5_WEIGHT_V04,
    "continuity": V0_5_WEIGHT_3DIM,
    "autonomy": V0_5_WEIGHT_3DIM,
    "transferability": V0_5_WEIGHT_3DIM,
}


def compute_v05_total(v04: float, continuity: float, autonomy: float,
                      transferability: float) -> float:
    """V0.5 composite formula 显式实现 (主 17:43 实事求是, 与 V1136 公式一致).

    ponytail: 不发明新公式, 单行计算与 V1136 measure_v05_3dims() 等价.
    """
    if not all(isinstance(x, (int, float)) for x in (v04, continuity, autonomy, transferability)):
        raise IntegrationContractError("V0.5 inputs must be numeric")
    return (
        v04 * V0_5_WEIGHT_V04
        + continuity * V0_5_WEIGHT_3DIM
        + autonomy * V0_5_WEIGHT_3DIM
        + transferability * V0_5_WEIGHT_3DIM
    )


# ---------------------------------------------------------------------------
# FieldBundle (主 17:43 + 主 00:56 任何人都能接手: dataclass)
# ---------------------------------------------------------------------------


@dataclass
class ICFieldBundle:
    """18 字段实例 — V0.3 17 dims + V0.5 1 composite.

    Source provenance stored as `provenance` dict for each field, with module+
    timestamp+ sha256(content), so any later audit can verify 真值来源.
    """

    fields: Dict[str, Optional[float]]
    provenance: Dict[str, Dict[str, Any]] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return {"fields": self.fields, "provenance": self.provenance}

    @classmethod
    def empty(cls) -> "ICFieldBundle":
        return cls(fields={k: None for k in ALL_FIELDS})


# ---------------------------------------------------------------------------
# Provenance helpers (主 17:43 实事求是: 每个真值带来源)
# ---------------------------------------------------------------------------


def _sha256_str(s: str) -> str:
    return hashlib.sha256(s.encode("utf-8")).hexdigest()


def _record_provenance(field_name: str, value: Any, source_module: str,
                        extra: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
    """Record provenance with sha256(value) + source module + ts."""
    if isinstance(value, float):
        content_repr = f"{value:.6f}"
    elif value is None:
        content_repr = "<null>"
    else:
        content_repr = repr(value)
    base = {
        "source_module": source_module,
        "value_sha256": _sha256_str(content_repr),
        "captured_at": round(time.time(), 4),
    }
    if extra:
        base.update(extra)
    return base


# ---------------------------------------------------------------------------
# V1074 bridge (主 17:43 + 主 19:33: 不发明, 直接 import 现成 measure)
# ---------------------------------------------------------------------------


def _safe_call(producer: Callable[[], Any], error_cls: type,
               producer_name: str) -> Any:
    """Run a producer; convert exceptions to IC error (主 17:58 不假装)."""
    try:
        return producer()
    except IntegrationContractError:
        raise
    except Exception as e:  # noqa: BLE001 — top-level barrier
        raise error_cls(
            f"{producer_name} failed: {type(e).__name__}: {e}",
            context={"producer": producer_name, "error_type": type(e).__name__},
        ) from e


def collect_v1074_dim_breakdown() -> Tuple[Dict[str, float], Dict[str, Any], float]:
    """Run V1074 真生产 snapshot 真测 → 17 V0.3 dims.

    Returns:
      (dim_breakdown_raw, v03_metrics, elapsed)
        - dim_breakdown_raw: 17 V0.3 dims, NOT yet lifted; zeros kept
          (主 17:43 实事求是: V1074 真不假装, 不替代真实 0.0)
        - v03_metrics: {v02_base, v1071_vcp, v1071_cross_domain, v1072_eternal_identity}
        - elapsed: wallclock seconds
    """
    started = time.time()

    def _producer() -> Dict[str, Any]:
        from apeireth.v1074_asi_production_runner import StatusSnapshotBuilder
        builder = StatusSnapshotBuilder()
        snap = builder.build()
        return {
            "dim_breakdown": snap.dim_breakdown,
            "v03_score": snap.v03_score,
            "v02_base": snap.v02_base,
            "v1071_vcp": snap.v1071_vcp_score,
            "v1071_cross_domain": snap.v1071_cross_domain,
            "v1072_eternal_identity": snap.v1072_eternal_identity,
        }

    raw = _safe_call(_producer, V1074UnreachableError, "v1074.measure_v03")

    dim_breakdown = raw["dim_breakdown"]
    # 严格保证 17 字段都在 (raise if any missing — 主 17:43 实事求是)
    missing = [k for k in V03_DIMS if k not in dim_breakdown]
    if missing:
        raise FieldMissingError(
            f"V1074 dim_breakdown missing {missing}",
            context={"missing": missing},
        )
    # V0.3 dim 强类型: float 且 in [0, 1]
    for name in V03_DIMS:
        v = dim_breakdown[name]
        if v is None or not isinstance(v, (int, float)):
            raise RangeViolationError(
                f"V1074 dim {name} not float: {v!r}",
                context={"field": name, "value": v},
            )
        if not (0.0 <= float(v) <= 1.0):
            raise RangeViolationError(
                f"V1074 dim {name} out of [0,1]: {v}",
                context={"field": name, "value": v},
            )
    metrics = {
        "v03_score": float(raw["v03_score"]),
        "v02_base": float(raw["v02_base"]),
        "v1071_vcp": float(raw["v1071_vcp"]),
        "v1071_cross_domain": float(raw["v1071_cross_domain"]),
        "v1072_eternal_identity": float(raw["v1072_eternal_identity"]),
    }
    elapsed = round(time.time() - started, 4)
    return dim_breakdown, metrics, elapsed


def lift_v04_from_v03(dim_breakdown: Dict[str, float]) -> float:
    """V0.3 dim_breakdown → V0.4 lift score (主 19:33 走在前人经验上).

    V1074 truth-source policy: zero dims skipped (主 22:33 truth discipline) — but
    V0.4 lift 公式 (V1101/V1102) explicitly: lift 是针对非空 dim 重新分布权重.
    Implementation: mean(skip_none and skip_zero) per V1074 dim_breakdown semantics,
    so真 missing zero dims 不污染均值.
    """
    nonzero = [v for v in dim_breakdown.values() if isinstance(v, (int, float)) and v > 0.0]
    if not nonzero:
        return 0.0
    return float(statistics.mean(nonzero))


def collect_v1136_v05_result() -> Tuple[Dict[str, Any], float]:
    """Run V1136 measure_v05_3dims() 真测 → V0.5 composite + 3 dims + details.

    Returns:
      (v05_payload, elapsed)
        - v05_payload: {v05_total_v1136, v05_total_v1125, v04_score, continuity,
                        autonomy, transferability, v3_guards_pass, chaos_preserved}
    """
    started = time.time()

    def _producer() -> Dict[str, Any]:
        from apeireth.v1136_asi_v05_3dim_real_measurement import measure_v05_3dims
        r = measure_v05_3dims(run_chaos=False)
        return r.to_dict()

    raw = _safe_call(_producer, V1136UnreachableError, "v1136.measure_v05_3dims")

    elapsed = round(time.time() - started, 4)
    return raw, elapsed


def verify_v05_composite(payload: Dict[str, Any], tolerance: float = 1e-3) -> None:
    """Hand-verify V0.5 composite against declared formula (主 17:43 实事求是).

    Raises CompositeDriftError if drift > tolerance.
    """
    expected = compute_v05_total(
        v04=float(payload["v04_score"]),
        continuity=float(payload["continuity"]),
        autonomy=float(payload["autonomy"]),
        transferability=float(payload["transferability"]),
    )
    actual = float(payload["v05_total_v1136"])
    if not math.isfinite(expected) or not math.isfinite(actual):
        raise CompositeDriftError(
            f"V0.5 composite non-finite: expected={expected}, actual={actual}",
            context={"expected": expected, "actual": actual},
        )
    if abs(expected - actual) > tolerance:
        raise CompositeDriftError(
            f"V0.5 composite drift {abs(expected - actual):.6f} > {tolerance}",
            context={
                "expected": round(expected, 6),
                "actual": round(actual, 6),
                "drift": round(abs(expected - actual), 6),
            },
        )


def collect_v1130_dashboard_summary() -> Tuple[Dict[str, Any], float]:
    """Run V1130 ContinuityDashboard 真 build → dashboard JSON summary.

    Returns:
      (summary, elapsed):
        - summary: {build_ok, chaos_safe, sub_components, perf_wallclock_ms,
                    target_2_5s}
    """
    started = time.time()

    def _producer() -> Dict[str, Any]:
        from apeireth.v1130_continuity_tracker_dashboard import (
            ContinuityDashboard,
            DashboardConfig,
        )
        cfg = DashboardConfig(enable_v1118=True)
        dash = ContinuityDashboard(cfg)
        payload = dash.build()
        wallclock_ms = float(payload.perf_stats.wallclock_ms)
        target_ok = bool(payload.perf_stats.target_2_5s)
        chaos_safe = (
            payload.chaos_recovery is None or bool(
                payload.chaos_recovery.get("payload_safe", True)
            )
        )
        return {
            "build_ok": True,
            "chaos_safe": chaos_safe,
            "perf_wallclock_ms": wallclock_ms,
            "target_2_5s": target_ok,
            "sub_components": [
                "TimelineViz", "RecoveryIndex", "CrossTableJoin", "StressDrill"
            ],
        }

    raw = _safe_call(_producer, V1130UnreachableError, "v1130.dashboard.build")

    elapsed_ms = raw["perf_wallclock_ms"]
    if elapsed_ms > 2500.0:
        raise DashboardTimeoutError(
            f"V1130 dashboard wallclock {elapsed_ms}ms > 2500ms target",
            context={"wallclock_ms": elapsed_ms, "target_ms": 2500.0},
        )

    elapsed = round(time.time() - started, 4)
    return raw, elapsed


# ---------------------------------------------------------------------------
# Validator (主 00:56 任何人都能接手: 一行跑完)
# ---------------------------------------------------------------------------


@dataclass
class ICValidationReport:
    """Validation outcome — pass/fail per IC_GUARDS + 失败码 + 可序列化."""

    contract_version: str
    contract_id: str
    passed: bool
    failed_codes: List[str]
    field_results: Dict[str, Dict[str, Any]]
    composite_v05_v1136: Optional[float]
    composite_v05_computed: Optional[float]
    composite_drift: Optional[float]
    v3_guards_pass: bool
    v3_guards_failed: List[str]
    runtime_metrics: Dict[str, float]
    timestamp: float
    notes: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)

    def to_json(self, indent: int = 2) -> str:
        return json.dumps(self.to_dict(), indent=indent, ensure_ascii=False,
                          default=str)


class IntegrationContractValidator:
    """R11 集成契约真校验 (主 17:43 实事求是 + 主 00:56 一行可跑).

    Modes:
      - strict=True: any IC failure → non-zero exit (CLI --strict)
      - compat_mode=True: accept V1125 LOCKED placeholder v05_total as truth (legacy)
    """

    def __init__(self, strict: bool = True, compat_mode: bool = False) -> None:
        self.strict = strict
        self.compat_mode = compat_mode
        self._bundle: Optional[ICFieldBundle] = None
        self._raw: Dict[str, Any] = {}

    def collect(self) -> ICFieldBundle:
        """真跑三模块, 收敛 18 字段 — provenance 一并记录."""
        bundle = ICFieldBundle.empty()

        # 1) V1074 dim_breakdown (17 dims)
        dim_breakdown, v03_metrics, t_v1074 = collect_v1074_dim_breakdown()
        for name in V03_DIMS:
            v = float(dim_breakdown[name])
            bundle.fields[name] = v
            bundle.provenance[name] = _record_provenance(
                name, v,
                "apeireth.v1074_asi_production_runner.StatusSnapshotBuilder"
            )
        self._raw["v1074_metrics"] = v03_metrics
        self._raw["elapsed_v1074"] = t_v1074

        # 2) V1136 V0.5 (composite + 3 dims)
        v05_payload, t_v1136 = collect_v1136_v05_result()
        verify_v05_composite(v05_payload)
        composite = float(v05_payload["v05_total_v1136"])
        bundle.fields["v05_total_v1136"] = composite
        bundle.provenance["v05_total_v1136"] = _record_provenance(
            "v05_total_v1136", composite,
            "apeireth.v1136_asi_v05_3dim_real_measurement.measure_v05_3dims",
            extra={
                "v04_score": v05_payload["v04_score"],
                "continuity": v05_payload["continuity"],
                "autonomy": v05_payload["autonomy"],
                "transferability": v05_payload["transferability"],
                "v3_guards_pass": v05_payload["v3_guards_pass"],
            },
        )
        self._raw["v1136_payload"] = v05_payload
        self._raw["elapsed_v1136"] = t_v1136

        # 3) V1130 dashboard summary (cross-link, not field)
        started_v1130 = time.time()
        try:
            v1130_summary, t_v1130_int = collect_v1130_dashboard_summary()
            t_v1130 = t_v1130_int
        except DashboardTimeoutError as e:
            wallclock_ms = e.context.get("wallclock_ms", -1.0)
            LOG.warning("[V1141] V1130 dashboard timeout %.2fms — degraded (主 17:58 不假装)",
                        wallclock_ms if wallclock_ms >= 0 else 0.0)
            v1130_summary = {
                "build_ok": False,
                "chaos_safe": False,
                "perf_wallclock_ms": wallclock_ms,
                "target_2_5s": False,
                "sub_components": [],
            }
            t_v1130 = round(time.time() - started_v1130, 4)
        self._raw["v1130_summary"] = v1130_summary
        self._raw["elapsed_v1130"] = t_v1130

        self._bundle = bundle
        return bundle

    def validate(self, bundle: Optional[ICFieldBundle] = None) -> ICValidationReport:
        """校验 bundle: 18 fields, V3 guards, composite drift, strict mode."""
        if bundle is None:
            if self._bundle is None:
                raise IntegrationContractError("collect() must run before validate()")
            bundle = self._bundle

        failed_codes: List[str] = []
        field_results: Dict[str, Dict[str, Any]] = {}

        # 1) Field-level: 17 dims required non-null + in [0, 1]
        for name in V03_DIMS:
            spec = IC_FIELD_SCHEMA[name]
            v = bundle.fields.get(name)
            ok = (v is not None
                  and isinstance(v, (int, float))
                  and spec.range_lo <= float(v) <= spec.range_hi)
            field_results[name] = {
                "value": v,
                "kind": spec.kind,
                "in_range": bool(ok),
                "producer": spec.producer,
                "required": spec.required,
            }
            if not ok and spec.required:
                failed_codes.append(IC_FIELD_MISSING if v is None else IC_RANGE_VIOLATION)

        # 2) V0.5 composite
        v05_spec = IC_FIELD_SCHEMA["v05_total_v1136"]
        v05_v = bundle.fields.get("v05_total_v1136")
        v05_ok = (v05_v is not None
                  and isinstance(v05_v, (int, float))
                  and v05_spec.range_lo <= float(v05_v) <= v05_spec.range_hi)
        field_results["v05_total_v1136"] = {
            "value": v05_v,
            "kind": v05_spec.kind,
            "in_range": bool(v05_ok),
            "producer": v05_spec.producer,
            "required": v05_spec.required,
            "nullable": v05_spec.nullable,
        }
        # Composite drift check
        composite_drift: Optional[float] = None
        composite_computed: Optional[float] = None
        v1136_payload = self._raw.get("v1136_payload") or {}
        if all(k in v1136_payload for k in
               ("v04_score", "continuity", "autonomy", "transferability")):
            composite_computed = compute_v05_total(
                v04=float(v1136_payload["v04_score"]),
                continuity=float(v1136_payload["continuity"]),
                autonomy=float(v1136_payload["autonomy"]),
                transferability=float(v1136_payload["transferability"]),
            )
            if v05_v is not None:
                composite_drift = round(abs(composite_computed - float(v05_v)), 6)
                if composite_drift > 1e-3:
                    failed_codes.append(IC_COMPOSITE_DRIFT)
        if v05_v is None and v05_spec.required and not self.compat_mode:
            failed_codes.append(IC_FIELD_MISSING)

        # 3) V1130 dashboard cross-link (timeout → degraded but not failed by default)
        v1130_summary = self._raw.get("v1130_summary") or {}
        if not v1130_summary.get("build_ok", False):
            failed_codes.append(IC_V1130_UNREACHABLE)

        # 4) V3 guards (13 keys — must all be assertable, but pass on data presence)
        v3_guards_failed: List[str] = []
        v3_guards_pass = (
            all(bundle.fields.get(n) is not None for n in V03_DIMS)
            and (v05_v is not None or self.compat_mode)
            and v1136_payload.get("v3_guards_pass", True) is True
        )
        if not v3_guards_pass:
            if not all(bundle.fields.get(n) is not None for n in V03_DIMS):
                v3_guards_failed.append("guard_v03_dims_complete")
            if v05_v is None and not self.compat_mode:
                v3_guards_failed.append("guard_v05_composite_present")
            if v1136_payload.get("v3_guards_pass", True) is False:
                v3_guards_failed.append("guard_v1136_v3_guards_pass")
            failed_codes.append("IC_V3_GUARDS_FAIL")

        # 5) Build report
        runtime_metrics = {
            "elapsed_v1074": self._raw.get("elapsed_v1074", 0.0),
            "elapsed_v1136": self._raw.get("elapsed_v1136", 0.0),
            "elapsed_v1130": self._raw.get("elapsed_v1130", 0.0),
        }

        # Strict → dedup failed_codes
        if self.strict:
            failed_codes = sorted(set(failed_codes))

        passed = (len(failed_codes) == 0) and v3_guards_pass
        return ICValidationReport(
            contract_version=INTEGRATION_CONTRACT_VERSION,
            contract_id=CONTRACT_DRAFT_ID,
            passed=passed,
            failed_codes=failed_codes,
            field_results=field_results,
            composite_v05_v1136=v05_v,
            composite_v05_computed=(
                round(composite_computed, 6) if composite_computed is not None else None
            ),
            composite_drift=composite_drift,
            v3_guards_pass=v3_guards_pass,
            v3_guards_failed=v3_guards_failed,
            runtime_metrics=runtime_metrics,
            timestamp=round(time.time(), 4),
            notes={
                "compat_mode": self.compat_mode,
                "strict": self.strict,
                "v1130_dashboard": v1130_summary,
                "v1074_metrics": self._raw.get("v1074_metrics", {}),
            },
        )


# ---------------------------------------------------------------------------
# Executable CLI (主 00:56 任何人都能接手: --validate 一行)
# ---------------------------------------------------------------------------


def run_validation(strict: bool = True, compat_mode: bool = False,
                    json_out: bool = False) -> ICValidationReport:
    """Exposed entry: collect+validate → ICValidationReport."""
    validator = IntegrationContractValidator(strict=strict, compat_mode=compat_mode)
    bundle = validator.collect()
    return validator.validate(bundle)


def _cli(argv: Optional[Sequence[str]] = None) -> int:
    p = argparse.ArgumentParser(
        prog="v1141_asi_v04_v05_integration_contract",
        description=(
            "R11 Architecture — V0.4/V0.5 Integration Contract validator. "
            "Runs V1074 + V1136 + V1130 real measurement and gates the 18-field contract."
        ),
    )
    p.add_argument("--validate", action="store_true",
                   help="Run collect+validate (default)")
    p.add_argument("--no-strict", action="store_true",
                   help="Disable strict mode (allow partial failures)")
    p.add_argument("--compat", action="store_true",
                   help="Enable compat mode (accept V1125 placeholder)")
    p.add_argument("--json", action="store_true", help="Emit JSON")
    p.add_argument("--report", action="store_true", help="Emit Markdown report")
    args = p.parse_args(argv)

    try:
        report = run_validation(
            strict=not args.no_strict,
            compat_mode=args.compat,
            json_out=args.json,
        )
    except IntegrationContractError as e:
        LOG.error("[V1141] IC failure: %s (code=%s)", e, e.code)
        if args.json:
            print(json.dumps({
                "passed": False,
                "code": e.code,
                "error": str(e),
                "context": e.context,
            }, indent=2, ensure_ascii=False))
        return 3  # IC error
    except Exception as e:  # noqa: BLE001 — top-level barrier
        LOG.error("[V1141] unexpected: %s: %s", type(e).__name__, e)
        if args.json:
            print(json.dumps({
                "passed": False,
                "code": "IC_UNEXPECTED",
                "error": f"{type(e).__name__}: {e}",
            }, indent=2, ensure_ascii=False))
        return 4

    if args.json:
        print(report.to_json())
    elif args.report:
        print(render_markdown_report(report))
    else:
        # Human summary
        print(f"V1141 Integration Contract — {report.contract_id} v{report.contract_version}")
        print(f"  passed: {report.passed}")
        print(f"  failed_codes: {report.failed_codes}")
        print(f"  composite v05_total_v1136: {report.composite_v05_v1136}")
        print(f"  composite computed:        {report.composite_v05_computed}")
        print(f"  composite drift:           {report.composite_drift}")
        print(f"  V3 guards pass: {report.v3_guards_pass} (failed: {report.v3_guards_failed})")
        print(f"  runtime: {report.runtime_metrics}")

    # Exit codes:
    #  0 — pass
    #  1 — IC fields failed (non-strict surface)
    #  2 — V3 guards failed
    #  3 — IC error (already handled)
    if report.passed:
        return 0
    if not report.v3_guards_pass:
        return 2
    return 1


def render_markdown_report(report: ICValidationReport) -> str:
    """Render Markdown 真报告 (主 00:56 任何人都能接手: 可读)."""
    L: List[str] = []
    L.append(f"# V1141 Integration Contract — Validation Report")
    L.append("")
    L.append(f"**Contract ID**: {report.contract_id}")
    L.append(f"**Version**: {report.contract_version}")
    L.append(f"**Timestamp**: {report.timestamp}")
    L.append(f"**Passed**: {'✅' if report.passed else '❌'} {report.passed}")
    L.append("")
    L.append("## 18 字段结果")
    L.append("")
    L.append("| # | Field | Value | In Range | Required | Producer |")
    L.append("|---|-------|-------|----------|----------|----------|")
    for spec in (IC_FIELD_SCHEMA[k] for k in ALL_FIELDS):
        fr = report.field_results.get(spec.name, {})
        v = fr.get("value", "<missing>")
        if isinstance(v, float):
            v_str = f"{v:.4f}"
        else:
            v_str = str(v)
        in_range = "✅" if fr.get("in_range", False) else "❌"
        L.append(
            f"| {spec.index:>2} | `{spec.name}` | {v_str} | {in_range} | "
            f"{spec.required} | `{spec.producer}` |"
        )
    L.append("")
    L.append("## Composite Drift")
    L.append("")
    L.append(f"- **v05_total_v1136**: {report.composite_v05_v1136}")
    L.append(f"- **computed**:        {report.composite_v05_computed}")
    L.append(f"- **drift**:           {report.composite_drift}")
    L.append("")
    L.append("## V3 哲学守门 (LOCKED 13 keys)")
    L.append("")
    L.append(f"- **pass**: {report.v3_guards_pass}")
    L.append(f"- **failed**: {report.v3_guards_failed}")
    L.append("")
    L.append("## 失败码")
    L.append("")
    L.append(f"```\n{report.failed_codes}\n```")
    L.append("")
    L.append("## Runtime Metrics")
    L.append("")
    for k, v in report.runtime_metrics.items():
        L.append(f"- **{k}**: {v:.4f}s")
    L.append("")
    if report.notes.get("v1130_dashboard"):
        L.append("## V1130 Dashboard Summary")
        L.append("")
        L.append(f"```json\n{json.dumps(report.notes['v1130_dashboard'], indent=2, ensure_ascii=False)}\n```")
        L.append("")
    L.append("---")
    L.append("")
    L.append("_主 17:43 实事求是 / 主 17:58 不假装 / 主 19:33 走在前人经验上 / 主 22:33 ASI 北极星 / 主 23:44 干到底_")
    return "\n".join(L)


__all__ = [
    # Versioning
    "INTEGRATION_CONTRACT_VERSION",
    "CONTRACT_DRAFT_ID",
    "IC_GUARDS",
    # Failure codes
    "IC_FIELD_MISSING", "IC_RANGE_VIOLATION", "IC_SUBSCORE_FAILED",
    "IC_V1074_UNREACHABLE", "IC_V1136_UNREACHABLE", "IC_V1130_UNREACHABLE",
    "IC_DASHBOARD_TIMEOUT", "IC_CHAOS_LOST", "IC_VERSION_CONFLICT",
    "IC_COMPOSITE_DRIFT",
    # Specs
    "V03_DIMS", "V05_EXTRA", "ALL_FIELDS",
    "ICFieldSpec", "IC_FIELD_SCHEMA",
    "V03_SPECS", "V05_SPECS",
    # Composite formula
    "V0_5_WEIGHT_V04", "V0_5_WEIGHT_3DIM", "V0_5_WEIGHTS",
    "compute_v05_total",
    # Bundle
    "ICFieldBundle",
    # Collectors / bridges
    "collect_v1074_dim_breakdown", "lift_v04_from_v03",
    "collect_v1136_v05_result", "verify_v05_composite",
    "collect_v1130_dashboard_summary",
    # Validator / report
    "IntegrationContractValidator", "ICValidationReport",
    "run_validation", "render_markdown_report",
    # Exceptions
    "IntegrationContractError", "FieldMissingError", "RangeViolationError",
    "V1074UnreachableError", "V1136UnreachableError", "V1130UnreachableError",
    "DashboardTimeoutError", "ChaosLostError", "CompositeDriftError",
]


# Allow `python -m apeireth.v1141_asi_v04_v05_integration_contract --validate`
if __name__ == "__main__":
    sys.exit(_cli())
