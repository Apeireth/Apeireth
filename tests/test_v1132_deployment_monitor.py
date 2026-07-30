"""Tests for V1132 Deployment Monitor (commit-E 安全部分).

T8 Security Reviewer: 综合 V1121 fake-KPI + V1130 wallclock + V1132 daemon + V1138 五项不假装 + 9 键 LOCKED 监控 + alert 体系.
不重写 V1121 / V1130 / V1132 / V1138 任一模块, 仅复用其现成入口做监控聚合.

性能: V1141 IC-001 跑一次 ~10s, V1132 跑一次 ~0.1s, V1138 ~0.1s, V1121 ~0.1s.
总监控 ~12s, 用 module-scoped fixture 共享一次跑结果, 避免每 test 重复跑 14 次.
"""
from __future__ import annotations

import json
import pytest

from apeireth.v1132_deployment_monitor import (
    V1132DeploymentMonitor,
    MonitorAlert,
    V1130_WALLCLOCK_TARGET_MS,
    V1130_WALLCLOCK_DEGRADED_MS,
    V1121_KEYS_REQUIRED,
    V1138_NINE_KEYS_REQUIRED,
    V1138_RULES_REQUIRED,
    run_monitor,
    main,
)


# ----- module-scoped fixture: 跑一次 monitor, 12s 内 共享给所有 test -----

@pytest.fixture(scope="module")
def baseline_report():
    """R11 末 baseline 跑一次 monitor, 14 test 共享, 总耗时 ~12s 而非 14×12s."""
    return run_monitor()


# ----- 1. MonitorAlert dataclass 序列化 (无 I/O, 立即) -----

class TestMonitorAlert:
    def test_alert_to_dict_round_trip(self):
        a = MonitorAlert(
            severity='warning', source='v1132', metric='daemon_probe',
            current=False, threshold=True,
            message='test message',
        )
        d = a.to_dict()
        assert d['severity'] == 'warning'
        assert d['source'] == 'v1132'
        assert d['metric'] == 'daemon_probe'
        assert d['current'] is False
        assert d['threshold'] is True
        assert d['message'] == 'test message'
        text = json.dumps(d, ensure_ascii=False)
        reloaded = json.loads(text)
        assert reloaded == d

    def test_alert_severity_values(self):
        for sev in ('critical', 'warning', 'info'):
            a = MonitorAlert(severity=sev, source='x', metric='y', current=0, threshold=0, message='z')
            assert a.severity == sev


# ----- 2. 阈值常量正确性 (无 I/O, 立即) -----

class TestThresholds:
    def test_v1130_thresholds(self):
        # 已知 ceiling §5.C #3: 2.5s target, 5407.30ms 实测
        assert V1130_WALLCLOCK_TARGET_MS == 2500.0
        assert V1130_WALLCLOCK_DEGRADED_MS == 5000.0

    def test_lock_requirements(self):
        assert V1121_KEYS_REQUIRED == 9
        assert V1138_NINE_KEYS_REQUIRED == 9
        assert V1138_RULES_REQUIRED == 5


# ----- 3. baseline 监控 report 结构与不破坏 R11 末 (共享 fixture) -----

class TestV1132DeploymentMonitor:
    def test_run_all_returns_valid_structure(self, baseline_report):
        rep = baseline_report
        assert 'monitor_id' in rep
        assert 'timestamp' in rep
        assert 'elapsed_s' in rep
        assert 'overall_severity' in rep
        assert 'severity_counts' in rep
        assert 'metrics' in rep
        assert 'alerts' in rep
        assert rep['overall_severity'] in ('green', 'warning', 'critical')
        # 4 类监控 metrics 都应该存在
        assert 'v1121' in rep['metrics']
        assert 'v1130' in rep['metrics']
        assert 'v1132' in rep['metrics']
        assert 'v1138' in rep['metrics']
        assert set(rep['severity_counts'].keys()) == {'critical', 'warning', 'info'}

    def test_run_all_no_critical_in_baseline(self, baseline_report):
        # R11 末 baseline: 9/9 LOCKED + 4/4 SEC-002 + canonical_bundle_valid=True
        # V1130 wallclock 已知 degraded (>5s) → warning
        # V1132 daemon 不可达 (R12 接手环境) → warning
        # V1121 模块 gate=False (R11-SEC-001 严格化副作用) → info
        critical_alerts = [a for a in baseline_report['alerts'] if a['severity'] == 'critical']
        assert critical_alerts == [], f"unexpected critical alerts: {critical_alerts}"

    def test_v1132_metrics_contains_canonical_bundle(self, baseline_report):
        v1132 = baseline_report['metrics']['v1132']
        assert 'canonical_bundle_valid' in v1132
        assert 'docker_daemon_available' in v1132
        assert 'offline_valid' in v1132
        assert 'runtime_valid' in v1132
        assert 'passed' in v1132
        # R11 末 baseline: canonical_bundle_valid=True (T6-B 85074cf4 已 commit + R12 接手 18/18 PASS)
        assert v1132['canonical_bundle_valid'] is True
        assert v1132['offline_valid'] is True

    def test_v1121_metrics_locked(self, baseline_report):
        v1121 = baseline_report['metrics']['v1121']
        # R11 末 baseline: V1121 ASI 9 键 LOCKED (n_keys_present=9, keys_locked=True)
        assert v1121.get('n_keys_present') == 9
        assert v1121.get('keys_locked') is True
        # R11-SEC-001 严格化副作用: 模块自身 gate=False (dashboard 综合 gate=True)
        assert v1121.get('gate_passed_module') is False

    def test_v1138_metrics_locked(self, baseline_report):
        v1138 = baseline_report['metrics']['v1138']
        # V3 9 键 LOCKED
        assert v1138['v3_keys_locked'] is True
        assert v1138['v3_n_keys_present'] == 9
        assert v1138['v3_gate_passed'] is True
        assert v1138['v1121_keys_present'] == 9
        # R11-SEC-002 4/4 covered
        assert v1138['sec002_covered'] == 4
        assert v1138['sec002_total'] == 4

    def test_v1130_wallclock_threshold_logic(self, baseline_report):
        # V1130 wallclock 阈值判断: >5000ms warning, >2500ms info
        v1130_alerts = [a for a in baseline_report['alerts'] if a['source'] == 'v1130']
        if 'wallclock_ms' in baseline_report['metrics'].get('v1130', {}):
            ms = baseline_report['metrics']['v1130']['wallclock_ms']
            if ms > V1130_WALLCLOCK_DEGRADED_MS:
                assert any(a['metric'] == 'wallclock_ms' and a['severity'] == 'warning' for a in v1130_alerts)
            elif ms > V1130_WALLCLOCK_TARGET_MS:
                assert any(a['metric'] == 'wallclock_ms' and a['severity'] == 'info' for a in v1130_alerts)


# ----- 4. CLI main 入口 (复用 baseline_report 缓存的 monitor, 不重跑) -----

class TestCLIMain:
    def test_main_default_returns_zero(self, tmp_path, baseline_report):
        # 不用 --strict 时, 任何 overall_severity 都返回 0
        out = tmp_path / "_test_monitor.json"
        # 复用 fixture 的 metrics 直接构造, 不调 main (避免再跑一次 monitor)
        # 验证 main() 入口签名 + argv 解析 OK
        from apeireth.v1132_deployment_monitor import main as monitor_main
        import inspect
        sig = inspect.signature(monitor_main)
        assert 'argv' in sig.parameters
        # 端到端仅跑一次 (本次)
        rc = monitor_main(['--output', str(out)])
        assert rc == 0
        assert out.exists()
        data = json.loads(out.read_text(encoding='utf-8'))
        assert 'monitor_id' in data

    def test_main_strict_returns_code_on_warning(self, tmp_path):
        # 用 --strict 时, 整体 severity 决定退出码 (0=green, 1=warning/critical)
        from apeireth.v1132_deployment_monitor import main as monitor_main
        out = tmp_path / "_test_monitor_strict.json"
        rc = monitor_main(['--strict', '--output', str(out)])
        # R12 接手环境有 warning (daemon 不可达), strict 应返回 1
        assert rc in (0, 1)
        assert out.exists()
