# V1396 deploy-stack executor 鈥?deploy

- schema: `v1396.deploy-executor/v1` v0.1.0
- generated_at: `2026-08-09T00:26:57Z`
- compose_files: **3**
- services_total: **19**
- issues: **0** (errors=0 warnings=0 info=0)
- port_conflicts: **0**
- ok: **True**
- elapsed_seconds: **0.2523**

## Compose files
- `deploy\18-crates\docker-compose.group-a.yml` 鈥?鉁?(9 services, 229 lines)
- `deploy\18-crates\docker-compose.group-b.yml` 鈥?鉁?(9 services, 229 lines)
- `deploy\docker-compose.yml` 鈥?鉁?(1 services, 28 lines)

## Chain result
- verdict: `GOOD`
- deploy_score: `100` grade=`A+`
- chain_ok: `True` (3/3 steps ok)
  - 鉁?`V1387` unified deploy-stack runner (0.0772s) files=24 findings=0 ok=True
  - 鉁?`V1393` deploy-stack judge (0.0735s) verdict=GOOD score=100 grade=A+
  - 鉁?`V1395` deploy-stack dashboard (0.0865s) modules=11/11 broken=0 tests=512

## GUARDS
- `GUARD_EXECUTOR_REAL`
- `GUARD_CHAIN_REAL`
- `GUARD_COMPOSE_PYTHON_ONLY`
- `GUARD_NO_CAP_CHANGE`
- `GUARD_DETERMINISTIC`
- `GUARD_HONEST_DISCLOSURE`
- `GUARD_MANIFEST_VALID`
- `GUARD_PORTS_VALID`
- `GUARD_DELEGATE_REAL`
- `GUARD_CLI_RUNNABLE`
- `GUARD_RESTART_VALID`
- `GUARD_HEALTHCHECK_RECOMMENDED`

## Known unknowns
- V1396 does not run docker / docker-compose / kubectl 鈥?static analysis only
- V1396 depends on PyYAML being installed; falls back to parse-error if not
- V1396 chain depends on V1387/V1393/V1395 being importable; marks step failed if not

*V1396 deploy executor v0.1.0 (schema v1396.deploy-executor/v1). Real run, not pretend.*
