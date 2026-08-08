"""Audit VCP modules for CLI patterns."""
import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'apeireth'))

mods = [
    'v1335_vcp_cross_plugin_invariant_synthesis',
    'v1336_vcp_plugin_conformance_linter',
    'v1337_vcp_plugin_compliance_dashboard',
    'v1338_vcp_plugin_migration_tool',
    'v1339_vcp_substrate_cookbook',
    'v1340_vcp_cookbook_validator',
    'v1341_vcp_pattern_detector',
    'v1342_vcp_quality_tiers',
    'v1343_vcp_tier_aware_linter',
    'v1344_vcp_ci_gate',
    'v1345_vcp_historical_ledger',
    'v1346_vcp_tier_aware_migration',
    'v1347_vcp_plugin_health',
    'v1348_vcp_anomaly_detector',
    'v1349_vcp_llm_benchmark',
    'v1350_vcp_anomaly_lifecycle',
]
for mod in mods:
    try:
        m = __import__(mod)
        has_main = hasattr(m, 'main')
        has_argparse = 'argparse' in dir(m)
        version = '?'
        for k in ['_VERSION', 'VERSION']:
            if hasattr(m, k):
                version = getattr(m, k)
                break
        print(f'{mod}: main={has_main}, argparse={has_argparse}, version={version}')
    except Exception as e:
        print(f'{mod}: ERR {e}')