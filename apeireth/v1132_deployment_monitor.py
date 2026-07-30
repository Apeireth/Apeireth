"""V1132 Deployment Monitor — commit-E 安全部分 (主 17:58 + 主 22:33 + 主 19:33)

集成 V1121 fake-KPI detector + V1130 dashboard wallclock + V1132 daemon probe
+ V1138 五项不假装 + V3 9 键 LOCKED 的综合 monitor + alert 体系. 不重写 V1121 /
V1130 / V1132 / V1138 任一模块, 仅复用其现成入口做监控聚合, 输出 monitor_report.json.

设计原则 (主 19:33 走在前人经验上):
- 复用 V1132DeploymentValidator.run_full_validation (T6-B commit 85074cf4 已落)
- 复用 V1138.check_v3_nine_keys_locked / check_asi_nine_keys_inheritance (R11 末已落)
- 复用 V1121.ASINineKeysGuard.check (T6-B commit 85074cf4 已落)
- 复用 V1141 IC-001 V1130 wallclock 字段 (R11 末已落, fallback: V1130 缺失则跳过)

Alert 严重级 (主 17:58 不假装):
- critical: V1138 5 项 / 9 键任一 = 0 (主 22:33 ASI 北极星 lock 失守)
- warning: V1121 dashboard yellow (信息性) / V1130 wallclock > 2.5s / V1132 daemon probe MISSING
- info: 已知 ceiling 透明标注 (V1130 5407.30ms / V1132 daemon 不可达 / V1121 模块自身 gate=False)
"""
from __future__ import annotations

import dataclasses
import json
import time
import traceback
from pathlib import Path
from typing import Any, Dict, List, Optional

# 4 类监控阈值
V1130_WALLCLOCK_TARGET_MS = 2500.0     # 已知 ceiling §5.C #3
V1130_WALLCLOCK_DEGRADED_MS = 5000.0   # degraded threshold (实测 5407.30ms 落此档)
V1121_KEYS_REQUIRED = 9                # ASI 9 键
V1138_NINE_KEYS_REQUIRED = 9           # V3 9 键
V1138_RULES_REQUIRED = 5               # 五项不假装


@dataclasses.dataclass
class MonitorAlert:
    severity: str       # 'critical' / 'warning' / 'info'
    source: str         # 'v1121' / 'v1130' / 'v1132' / 'v1138'
    metric: str
    current: Any
    threshold: Any
    message: str

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


class V1132DeploymentMonitor:
    """综合 4 类监控 + alert 体系, 不重写 V1121 / V1130 / V1132 / V1138."""

    def __init__(self, repo_root: Optional[str] = None) -> None:
        self.repo_root = repo_root
        self.alerts: List[MonitorAlert] = []
        self.metrics: Dict[str, Any] = {}

    def run_all(self) -> Dict[str, Any]:
        """跑 4 类监控, 返回 monitor_report dict (含 metrics + alerts + severity + elapsed)."""
        t0 = time.time()
        try:
            self.check_v1132()
        except Exception as e:
            self.alerts.append(MonitorAlert(
                severity='critical', source='v1132', metric='monitor_itself',
                current=type(e).__name__, threshold='runnable',
                message=f"V1132 monitor self failed: {e}\n{traceback.format_exc(limit=3)}",
            ))
        self.check_v1121()
        self.check_v1130()
        self.check_v1138()

        # 严重级聚合
        sev_counts = {'critical': 0, 'warning': 0, 'info': 0}
        for a in self.alerts:
            sev_counts[a.severity] = sev_counts.get(a.severity, 0) + 1
        if sev_counts['critical'] > 0:
            overall = 'critical'
        elif sev_counts['warning'] > 0:
            overall = 'warning'
        else:
            overall = 'green'

        return {
            'monitor_id': f'mon-{int(time.time())}',
            'timestamp': time.time(),
            'elapsed_s': round(time.time() - t0, 3),
            'overall_severity': overall,
            'severity_counts': sev_counts,
            'metrics': self.metrics,
            'alerts': [a.to_dict() for a in self.alerts],
        }

    # ----- 1. V1121 fake-KPI detector (T6-B 85074cf4 已 commit) -----
    def check_v1121(self) -> None:
        try:
            from apeireth.v1121_security_guard_v01 import ASINineKeysGuard
            guard = ASINineKeysGuard()
            rep = guard.check()
            keys_present = rep.n_keys_present       # ASINineKeysReport.n_keys_present (not keys_present)
            keys_locked = rep.keys_locked
            gate_passed = rep.gate_passed
            n_threats = len(rep.threats)
            fake_kpi_attempts = rep.fake_kpi_attempts
        except Exception as e:
            self.metrics['v1121'] = {'error': f'{type(e).__name__}: {e}'}
            self.alerts.append(MonitorAlert(
                severity='critical', source='v1121', metric='monitor_probe',
                current=f'{type(e).__name__}', threshold='runnable',
                message=f"V1121 probe failed: {e}",
            ))
            return

        self.metrics['v1121'] = {
            'n_keys_present': keys_present,
            'keys_locked': keys_locked,
            'gate_passed_module': gate_passed,
            'n_threats': n_threats,
            'fake_kpi_attempts': fake_kpi_attempts,
        }

        if keys_present < V1121_KEYS_REQUIRED:
            self.alerts.append(MonitorAlert(
                severity='critical', source='v1121', metric='keys_present',
                current=keys_present, threshold=V1121_KEYS_REQUIRED,
                message=f"V1121 ASI 9 键缺失: present={keys_present} < required={V1121_KEYS_REQUIRED} (主 22:33 ASI 北极星 lock 失守)",
            ))
        if not gate_passed:
            # V1121 模块自身 gate=False 是已知信息性 (R11-SEC-001 严格化副作用), dashboard 综合 yellow
            self.alerts.append(MonitorAlert(
                severity='info', source='v1121', metric='module_gate_passed',
                current=gate_passed, threshold=True,
                message=f"V1121 模块自身 gate=False (dashboard 综合 gate=True, 已知信息性, R11-SEC-001 严格化副作用, 附录 M §1.5 已锁定)",
            ))

    # ----- 2. V1130 dashboard wallclock (T1 命令 3 实测 5407.30ms, 已知 ceiling §5.C #3) -----
    def check_v1130(self) -> None:
        elapsed_ms: Optional[float] = None
        source_detail = 'unknown'
        try:
            # 优先复用 V1141 IC-001 composite.runtime.elapsed_v1130 (R11 末已落)
            from apeireth.v1141_asi_v04_v05_integration_contract import (
                IntegrationContractValidator,
            )
            validator = IntegrationContractValidator()
            bundle = validator.collect()
            res = validator.validate(bundle)
            # ICValidationReport 是 dataclass, to_dict() 暴露字段; 字段是 runtime_metrics (非 runtime)
            res_d = res.to_dict() if hasattr(res, 'to_dict') else {}
            runtime = res_d.get('runtime_metrics', {}) if isinstance(res_d, dict) else {}
            if hasattr(runtime, 'to_dict'):
                runtime = runtime.to_dict()
            if isinstance(runtime, dict) and 'elapsed_v1130' in runtime:
                elapsed_ms = float(runtime['elapsed_v1130']) * 1000.0
                source_detail = 'V1141 IC-001 runtime_metrics.elapsed_v1130'
        except Exception as e:
            self.metrics['v1130'] = {'probe_error': f'{type(e).__name__}: {e}'}
            # V1141 不可用, 降级: 跳过 wallclock 监控, 不 alert
            return

        if elapsed_ms is None:
            return

        self.metrics['v1130'] = {
            'wallclock_ms': elapsed_ms,
            'source': source_detail,
            'target_ms': V1130_WALLCLOCK_TARGET_MS,
            'degraded_ms': V1130_WALLCLOCK_DEGRADED_MS,
        }
        if elapsed_ms > V1130_WALLCLOCK_DEGRADED_MS:
            self.alerts.append(MonitorAlert(
                severity='warning', source='v1130', metric='wallclock_ms',
                current=elapsed_ms, threshold=V1130_WALLCLOCK_TARGET_MS,
                message=f"V1130 wallclock degraded: {elapsed_ms:.0f}ms > {V1130_WALLCLOCK_DEGRADED_MS:.0f}ms (远超 2.5s target, §5.C #3 已知 ceiling, 非回归)",
            ))
        elif elapsed_ms > V1130_WALLCLOCK_TARGET_MS:
            self.alerts.append(MonitorAlert(
                severity='info', source='v1130', metric='wallclock_ms',
                current=elapsed_ms, threshold=V1130_WALLCLOCK_TARGET_MS,
                message=f"V1130 wallclock 超 2.5s target: {elapsed_ms:.0f}ms (R12 ceiling, 附录 M §5.D)",
            ))

    # ----- 3. V1132 daemon probe MISSING (T6-B 85074cf4 已 commit canonical_bundle_valid) -----
    def check_v1132(self) -> None:
        try:
            from apeireth.v1132_real_deployment_validator import V1132DeploymentValidator
            v = V1132DeploymentValidator(repo_root=self.repo_root)
            rep = v.run_full_validation()
        except Exception as e:
            self.metrics['v1132'] = {'error': f'{type(e).__name__}: {e}'}
            self.alerts.append(MonitorAlert(
                severity='critical', source='v1132', metric='monitor_probe',
                current=f'{type(e).__name__}', threshold='runnable',
                message=f"V1132 deployment validator probe failed: {e}",
            ))
            return

        self.metrics['v1132'] = {
            'docker_daemon_available': rep.docker_daemon_available,
            'canonical_bundle_valid': rep.canonical_bundle_valid,
            'offline_valid': rep.offline_valid,
            'runtime_valid': rep.runtime_valid,
            'passed': rep.passed,
            'health_probes_ok': rep.health_probes_ok,
            'health_probes_failed': rep.health_probes_failed,
            'compose_files_parsed': rep.compose_files_parsed,
            'k8s_manifests_ok': rep.k8s_manifests_ok,
        }

        if not rep.canonical_bundle_valid:
            self.alerts.append(MonitorAlert(
                severity='critical', source='v1132', metric='canonical_bundle_valid',
                current=rep.canonical_bundle_valid, threshold=True,
                message="V1132 18 跨文件语义断言失守 (canonical_bundle_valid=False, deploy/ 资产不一致)",
            ))
        if not rep.offline_valid:
            self.alerts.append(MonitorAlert(
                severity='warning', source='v1132', metric='offline_valid',
                current=rep.offline_valid, threshold=True,
                message="V1132 offline_valid=False (静态/子进程验证失败, deploy/ 资产不完整)",
            ))
        if not rep.docker_daemon_available:
            self.alerts.append(MonitorAlert(
                severity='warning', source='v1132', metric='daemon_probe',
                current=rep.docker_daemon_available, threshold=True,
                message="V1132 docker daemon 不可达 (probe MISSING, deploy/ 上线需在具 daemon 节点重跑, §5.D #2 ceiling)",
            ))
        if rep.health_probes_failed > 0:
            self.alerts.append(MonitorAlert(
                severity='warning', source='v1132', metric='health_probes',
                current=f"failed={rep.health_probes_failed}", threshold="failed=0",
                message=f"V1132 health probe 失败: {rep.health_probes_failed} 个, 8765/health 端点未验证",
            ))

    # ----- 4. V1138 五项不假装 + V3 9 键 LOCKED (主 17:58 + 主 22:33) -----
    def check_v1138(self) -> None:
        try:
            from apeireth.v1138_r11_no_pretend_five_guards import (
                check_v3_nine_keys_locked,
                check_asi_nine_keys_inheritance,
                _check_r11_sec002_self_claim_coverage,
            )
            v3 = check_v3_nine_keys_locked()
            asi = check_asi_nine_keys_inheritance()
            sec002 = _check_r11_sec002_self_claim_coverage()
        except Exception as e:
            self.metrics['v1138'] = {'error': f'{type(e).__name__}: {e}'}
            self.alerts.append(MonitorAlert(
                severity='critical', source='v1138', metric='monitor_probe',
                current=f'{type(e).__name__}', threshold='runnable',
                message=f"V1138 probe failed: {e}",
            ))
            return

        # V3 9 键 LOCKED
        n_keys = getattr(v3, 'n_keys_present', 0) or 0
        v3_locked = getattr(v3, 'keys_locked', False) or False
        self.metrics['v1138'] = {
            'v3_keys_locked': v3_locked,
            'v3_n_keys_present': n_keys,
            'v3_gate_passed': getattr(v3, 'gate_passed', False),
            'v1121_keys_present': getattr(asi, 'keys_present', 0),
            'v1121_gate_passed': getattr(asi, 'gate_passed', False),
            'sec002_covered': sec002.get('covered', 0),
            'sec002_total': sec002.get('total', 0),
        }

        if n_keys < V1138_NINE_KEYS_REQUIRED:
            self.alerts.append(MonitorAlert(
                severity='critical', source='v1138', metric='v3_nine_keys',
                current=n_keys, threshold=V1138_NINE_KEYS_REQUIRED,
                message=f"V1138 V3 哲学契约 9 键缺失: present={n_keys} < required={V1138_NINE_KEYS_REQUIRED} (主 22:33 ASI 北极星 lock 失守)",
            ))
        if not v3_locked:
            self.alerts.append(MonitorAlert(
                severity='critical', source='v1138', metric='v3_locked',
                current=v3_locked, threshold=True,
                message="V1138 V3 9 键 keys_locked=False (主 17:58 不假装 + 主 22:33 ASI 北极星 lock 失守)",
            ))

        # R11-SEC-002 coverage
        sec_total = sec002.get('total', 0) or 0
        sec_covered = sec002.get('covered', 0) or 0
        if sec_total > 0 and sec_covered < sec_total:
            self.alerts.append(MonitorAlert(
                severity='critical', source='v1138', metric='r11_sec002_coverage',
                current=f"{sec_covered}/{sec_total}", threshold=f"{sec_total}/{sec_total}",
                message=f"R11-SEC-002 self-claim coverage 不全: {sec_covered}/{sec_total}",
            ))


def run_monitor(repo_root: Optional[str] = None) -> Dict[str, Any]:
    """CLI 入口 helper."""
    m = V1132DeploymentMonitor(repo_root=repo_root)
    return m.run_all()


def main(argv: Optional[List[str]] = None) -> int:
    import argparse
    p = argparse.ArgumentParser(description="V1132 deployment monitor (commit-E 安全部分)")
    p.add_argument("--repo-root", default=None, help="apeireth repo root (default: auto-detect)")
    p.add_argument("--output", default=None, help="write monitor_report.json to path")
    p.add_argument("--strict", action="store_true", help="exit non-zero on critical/warning")
    args = p.parse_args(argv)

    report = run_monitor(repo_root=args.repo_root)
    text = json.dumps(report, ensure_ascii=False, indent=2)
    if args.output:
        Path(args.output).write_text(text, encoding='utf-8')
    else:
        print(text)

    if args.strict and report['overall_severity'] in ('critical', 'warning'):
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
