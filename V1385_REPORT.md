# V1385 — docker-compose YAML 真解析 + 真 lint (compose-spec 真借鉴 + 8 真规则)

**Phase:** 1385
**Version:** 0.1.0
**Date:** 2026-08-09 (cron tick 238, 05:50 → 06:00)
**Post:** V1384 (V1050 Dockerfile 真解析 + 真 lint)
**ASI 北极星:** LOCKED (V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V0.3 NOT due)

---

## What V1385 is

V1385 is the **real-engineering counterpart** to V1032's docker-compose
template strings. Where V1032 generates `docker-compose.yml` text and
V1260 spawns compose services, V1385 actually **reads, parses, and
lints** real `docker-compose.yml` files and reports concrete findings
on the deployable shape.

V1385 is not a re-implementation of compose-spec/compose-go. It's a
real, runnable, tested subset of compose-spec best-practices plus
8 V1385-native rules covering the specific issues V1260 / V1032's
generated compose files can exhibit.

This is the bridge between "we wrote the compose YAML" and "we
actually know if the compose file is correct." The cron side never
asks "is V1260's compose correct?" — V1385 makes the answer concrete.

## Real production design (主 19:33 + 主 17:43)

- **Real read**: `parse_compose_services(text)` uses PyYAML 6.0.3
  `safe_load` to parse the YAML. Handles anchor + merge keys (`<<: *xxx`)
  automatically (PyYAML expands them).
- **Real parse**: produces a `ServiceInfo` per service with
  `image / has_healthcheck / has_restart / has_memory_limit /
  depends_on_targets / depends_on_uses_healthy / env / volumes /
  network_mode / privileged / raw`. Plus a `line_map` for service line
  numbers (used in finding line_no).
- **Real lint**: 8 rules borrowed from compose-spec / compose-go best
  practices (see table below).
- **Real findings**: each finding has `rule_id, severity, service,
  line_no, message, suggestion`. Sorted by (service, line_no, severity).
- **Real CLI**: `python -m apeireth.v1385_real_compose_lint <path>
  [--json] [--strict] [--quiet] [--demo] [--version]`. Exits 0/1/2
  (ok / has error / strict has warning).
- **Real Popper self-tests**: 43 pytest cases covering parser,
  every rule, env dict/list, volumes short/long, YAML errors, CLI,
  JSON output, stdin, real-repo compose file, determinism.

## Rules implemented (8)

| # | Rule ID | Severity | Description |
|---|---------|----------|-------------|
| 1 | COMPOSE-LATEST-TAG | warning/info | Image uses `:latest` (or `${VAR:-latest}` default) — non-reproducible |
| 2 | COMPOSE-PRIVILEGED | error | `privileged: true` — container escape risk |
| 3 | COMPOSE-NETWORK-HOST | warning | `network_mode: host` — network isolation disabled |
| 4 | COMPOSE-DOCKER-SOCK | error | Mounts `/var/run/docker.sock` — host docker control |
| 5 | COMPOSE-PLAINTEXT-SECRET | warning | Env name matches KEY/SECRET/TOKEN/PASSWORD with literal value |
| 6 | COMPOSE-MISSING-RESTART | warning | Service has no `restart` policy |
| 7 | COMPOSE-MISSING-MEM-LIMIT | info | Service has no memory limit (OOM risk) |
| 8 | COMPOSE-DEPENDS-NO-HEALTHY | warning | `depends_on` without `condition: service_healthy` (target has healthcheck) |
| 9 | COMPOSE-PARSE-ERROR | error | YAML parse failed (top-level not mapping, syntax error, etc.) |

## CLI

```bash
# From promethean/
python -m apeireth.v1385_real_compose_lint deploy/docker-compose.yml
# → V1385 compose-lint v0.1.0 — deploy/docker-compose.yml
#   parse_ok=True n_services=9 n_findings=9 (errors=0 warnings=0 info=9) ok=True

python -m apeireth.v1385_real_compose_lint deploy/docker-compose.yml --json
# → JSON output with full findings list

python -m apeireth.v1385_real_compose_lint deploy/docker-compose.yml --strict
# → exit 0 if clean, exit 1 if errors, exit 2 if warnings

python -m apeireth.v1385_real_compose_lint --demo
# → built-in demo that triggers all 8 rules

python -m apeireth.v1385_real_compose_lint - < deploy/docker-compose.yml
# → read from stdin
```

## Real findings on the actual repo compose file

```bash
$ python -m apeireth.v1385_real_compose_lint Apeireth-rust/deploy/docker-compose.protocols.yml
V1385 compose-lint v0.1.0 — Apeireth-rust/deploy/docker-compose.protocols.yml
  parse_ok=True n_services=9 n_findings=9 (errors=0 warnings=0 info=9) ok=True
  [INFO] COMPOSE-LATEST-TAG × 9 services
  → every service uses ${APEIRETH_IMAGE_TAG:-latest}
```

The 9-service V2 deploy file is **mostly clean** — it has explicit
healthcheck (test/interval/timeout/retries/start_period), restart:
unless-stopped, memory limits (256M/1024M/512M), and no privileged
/ network_mode host / docker.sock mounts. The only finding is the
informational `COMPOSE-LATEST-TAG` on the `${APEIRETH_IMAGE_TAG:-latest}`
default — which is by design (CI injects the tag). V1385 correctly
classifies this as `info` (not warning) because the default only
applies when `APEIRETH_IMAGE_TAG` is unset.

## V1385 honest integration with V1260 / V1032

```python
from apeireth.v1385_real_compose_lint import V1385ComposeLint
from apeireth.v1260_v1260_v1260_docker_deploy import V1260DockerDeploy

# 真写 V1260 compose 到临时目录
deploy = V1260DockerDeploy()
path = deploy.write_compose("/tmp/deploy/")

# 真 lint V1260 写的 compose
linter = V1385ComposeLint()
report = linter.lint_file(path)
# report.parse_ok=True, report.n_services >= 3, report.n_findings >= 0
```

## Honest measurement (this round)

- **V1385 Popper self-tests:** 43 pytest cases (43/43 pass in 0.28s)
- **V1385 chain V1384+V1385:** 91/91 pass in 0.29s
- **V1385 demo run:** all 8 rules fire (2 errors, 5 warnings, 1 info)
- **V1385 real-repo run:** 9 services, 9 info (only `:latest` default)
- **ASI pole-star V0.2 honest cap:** 0.90 preserved (V1385 has no metric)

## GUARDS upheld (V1385-specific)

1. **GUARD_LINT_REAL**: every finding comes from a real PyYAML parse +
   real rule match on real text; no template strings
2. **GUARD_NO_CAP_CHANGE**: V1385 has no metric, no scoring, no cap
3. **GUARD_DETERMINISTIC**: same input → same findings (no time/random)
4. **GUARD_PATH_SAFE**: missing files return exit 2 with stderr msg,
   not traceback
5. **GUARD_HONEST_DISCLOSURE**: findings include `suggestion` field;
   CLI prints full reasoning; JSON is structured; demo defaults to
   exit 0 (it's a demo, not a CI gate)
6. **GUARD_COMPOSE_ONLY**: V1385 only lints compose YAML, doesn't
   touch Dockerfiles (V1384 scope)
7. **GUARD_BORROW_OPEN_SOURCE**: 8 rules credited to compose-spec
   (https://github.com/compose-spec/compose-spec) and compose-go
   (https://github.com/compose-spec/compose-go)
8. **GUARD_CLI_RUNNABLE**: CLI exits 0/1/2 deterministically, stdin
   supported, JSON supported

## V3 哲学守门 (LOCKED)

- **不假装分数 = ASI:** V1385 has no metric, no ASI scoring
- **不假装决策 = 真生产:** lint = real PyYAML parse + real rules; no proxy
- **不假装 ASI 集成:** zero LLM, zero sidecar import, zero ASI ledger
- **不刷分:** zero metric change; honest 0.90 cap preserved
- **不动 anchor:** V1260 / V1032 source unchanged; V1385 reads them
- **不假装 V1385 = ASI 觉醒:** V1385 lints compose YAML; doesn't lint
  ASI. ASI 北极星 unchanged

## Bugs hit during development

None — V1385 came up clean on first run. PyYAML 6.0.3's anchor expansion
is automatic and correct; line_map approximation is good enough for
finding line numbers (verified against `test_v1385_build_line_map_basic`).

## Files written

- `apeireth/v1385_real_compose_lint.py` (~660 lines, 1 module)
- `tests/test_v1385_real_compose_lint.py` (~430 lines, 43 tests)
- `V1385_REPORT.md` (this file)

## What V1385 is NOT

- V1385 is **not** a docker-compose schema validator (it doesn't
  enforce that all required compose-spec fields are present). It only
  runs 8 rules on top of a permissive PyYAML parse.
- V1385 is **not** a security audit. Privileged / docker.sock findings
  are rule matches, not exploit proofs.
- V1385 is **not** a substitute for `docker compose config --quiet`
  (which validates against the full compose-spec). It's a faster,
  narrower, more opinionated subset.
- V1385 is **not** ASI. It's a 660-line Python module that lints YAML.

## Next-step candidates (post-V1385)

- **V1386 .dockerignore lint** — file pattern analysis (excludes .git,
  IDE files, secrets)
- **V1387 K8s manifest lint** — kubeconform-style rules on YAML
- **V1388 V1384×V1385 unified CI** — chain Dockerfile lint + compose
  lint + exit code aggregation
- **V1389 compose-spec schema validation** — stricter version using
  compose-go schema

(V1385 stops here. Master asleep, posture silent upheld. cron tick 238
done. ASI 北极星 0.7905 lock preserved.)
