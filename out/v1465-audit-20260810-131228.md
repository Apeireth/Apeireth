# V1465 Audit Report — v1465_asi_lint_gate_http_gateway_cross_audit

- **Module**: `v1465_asi_lint_gate_http_gateway_cross_audit`
- **Version**: `0.1.0`
- **Schema**: `v1465.asi-lint-gate-http-gateway-cross-audit/v1`
- **Date**: `2026-08-10`
- **Verdict**: **PASS**
- **Elapsed**: 6.03s

## Summary

- Endpoints: 9/9 pass, 0 fail, 0 error
- Invariants: 9/9 pass, 0 fail, 0 error
- Happy paths: 6, Sad paths: 3

## V1464 Server Boot

- Outcome: **PASS**
- PID: 44752
- Bind: 127.0.0.1:18080
- Boot time: 638.23 ms
- stderr tail: `\subprocess.py", line 1615, in _readerthread
    buffer.append(fh.read())
                  ~~~~~~~^^
UnicodeDecodeError: 'gbk' codec can't decode byte 0xae in position 10: illegal multibyte sequence
`

## Endpoint Audits

| Method | Path | Status | Outcome | Shape | ms | Note |
|--------|------|--------|---------|-------|----|------|
| GET | `/healthz` | 200 | PASS | ✓ | 1.57 | liveness probe |
| GET | `/status` | 200 | PASS | ✓ | 17.8 | module status cross-check |
| GET | `/pipeline/adversarial` | 200 | PASS | ✓ | 2614.33 | 30/30 adversarial suite |
| POST | `/pipeline/lint?policy=STANDARD` | 200 | PASS | ✓ | 22.84 | safe spec under STANDARD policy |
| POST | `/pipeline/policy-gate?policy=PERMISSIVE` | 200 | PASS | ✓ | 0.84 | bad spec under PERMISSIVE policy |
| POST | `/pipeline/run` | 200 | PASS | ✓ | 42.06 | mixed JSONL run: 1 safe + 1 bad |
| GET | `/no-such-endpoint` | 404 | PASS | ✓ | 25.91 | sad: unknown path returns 404 |
| POST | `/healthz` | 405 | PASS | ✓ | 15.49 | sad: POST on GET-only endpoint returns 405 |
| POST | `/pipeline/lint` | 413 | PASS | ✓ | 0.67 | sad: oversize body rejected (413 or abort) |
  - error: `connection_aborted_but_rejected: ConnectionAbortedError: [WinError 10053] 你的主机中的软件中止了一个已建立的连接。`

## Cross-Module Invariants

| Module | Invariant | Outcome | Expected | Actual |
|--------|-----------|---------|----------|--------|
| v1464 | loopback_default | PASS | `127.0.0.1` | `127.0.0.1` |
| v1464 | body_bounded | PASS | `262144` | `262144` |
| v1464 | n_routes_6 | PASS | `6` | `6` |
| v1463 | adversarial_30_specs | PASS | `30` | `30` |
| v1463 | match_rate_1.0 | PASS | `1.0` | `1.0` |
| v1462 | n_rules_24 | PASS | `24` | `24` |
| v1462 | policy_levels_3 | PASS | `3` | `3` |
| v1461 | sandbox_modes_9 | PASS | `9` | `9` |
| v1460 | stages_12_or_13 | PASS | `12 or 13` | `13` |
