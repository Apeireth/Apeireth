"""Quick scan: find run_* + main entry for each V14XX audit module."""
import importlib

mods = [
    'v1458_asi_north_star_ceiling_chain_audit',
    'v1457_asi_six_deployment_operational_runbook',
    'v1456_asi_six_deployment_real_execution_parity',
    'v1450_asi_cross_modular_cube_history',
    'v1449_asi_seven_problems_vcp_cross_modular',
    'v1446_asi_seven_philosophical_problems',
    'v1445_asi_v2_position_closure_audit',
    'v1256_evidence_audit',
    'v1455_asi_hypercube_full_source_content_audit_v5',
    'v1259',
]
for m in mods:
    try:
        x = importlib.import_module(f'apeireth.{m}')
        names = [n for n in dir(x) if not n.startswith('_')]
        runs = [n for n in names if n.startswith('run_')]
        mains = [n for n in names if 'main' in n.lower()]
        print(f'{m}: runs={runs[:5]} mains={mains[:5]}')
    except Exception as e:
        print(f'{m}: ERR {e}')