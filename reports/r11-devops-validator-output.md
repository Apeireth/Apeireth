# V1132 真部署 validator 报告 (主 06:15 V1050+ 真部署方向 + 主 17:43 实事求是)

- report_id: `rpt-0af166d3`
- timestamp: 1785394861.1975942
- docker_daemon_available: **False**
- compose_files_parsed: **2**
- services_seen: **14**
- k8s_manifests_ok: **3**
- dockerfile_valid: **2**
- subprocess_runs_ok / failed: **2** / 0
- health_probes_ok / failed: **0** / 1
- canonical_bundle_valid: **True**
- offline_valid: **True** (static/subprocess only; no container claim)
- runtime_valid: **False** (requires daemon + live canonical health probe)
- passed: **False** (strict runtime verdict)

## Checks

| name | passed | detail |
|------|--------|--------|
| docker_daemon_probe | False | docker CLI not on PATH (host has no docker installed) |
| compose_parse[docker-compose.r8.yml] | True | ok (13 services) (redacted\.openclaw\workspace\promethean\docker-compose.r8.yml) |
| compose_parse[docker-compose.yml] | True | absent (ok): file not found: redacted\.openclaw\workspace\promethean\docker-compose.yml |
| compose_parse[docker-compose.yml] | True | ok (1 services) (redacted\.openclaw\workspace\promethean\deploy\docker-compose.yml) |
| v1008_subprocess_render | True | compose=317chars, k8s=700chars, sh=717chars, k8s_parse=parsed 2 doc(s), kinds=['Deployment', 'Service'] |
| v1032_subprocess_render | True | files=Dockerfile=1329\|docker-compose.yml=972\|k8s-deployment.yaml=1846\|requirements.txt=375; dockerfile=FROM + WORKDIR present (1329 chars); k8s=parsed 3 doc(s), kinds=['Deployment', 'Service', 'Hor |
| canonical_bundle | True | 18/18 semantic checks passed; image=apeireth-asi:0.1.0 port=8765 |
| consistency_check | True | historical generators isolated; canonical bundle governs deploy/: v1008=['v1132-svc'] v1032=['asi-core', 'asi-test'] r8_n=13 |
| probe[canonical-v1075] | False | runtime not verified at http://127.0.0.1:8765/health: URLError: <urlopen error timed out> |

## Notes

- docker daemon not reachable; container-level checks (docker-compose up -d, container healthchecks) are NOT executed. Config-level checks still run.
- historical V1008/V1032/R8 examples use distinct service names; canonical deploy/ bundle is checked separately: v1008=['v1132-svc'] v1032=['asi-core', 'asi-test'] r8_n=13

