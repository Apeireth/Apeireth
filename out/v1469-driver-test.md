# V1469 Two-Process Driver Report

- **Module**: `v1469_asi_real_two_process_v1468_client_v1467_server_driver`
- **Version**: `0.1.0`
- **Schema**: `v1469.asi-real-two-process-v1468-client-v1467-server-driver/v1`
- **Date**: `2026-08-10`
- **Verdict**: **PASS**
- **OK**: True
- **Host**: `127.0.0.1:18380`
- **Server PID**: 23336
- **Client PID**: 60040
- **Server boot**: 1.00s
- **Client elapsed**: 0.15s
- **Total elapsed**: 2.21s
- **Endpoints OK**: 6/6
- **Guards passed**: 14/14
- **V3 哲学守门**: 7
- **Borrowed sources**: 5 (v1468, v1467, v1466, v1437, stdlib)

## Endpoints hit via V1468-generated client (subprocess B)

- ✅ `GET /healthz` → `healthz` (30.5ms) payload_keys=['module', 'ok', 'schema', 'ts', 'version']
- ✅ `GET /status` → `status` (14.3ms) payload_keys=['chain', 'endpoints', 'history_count', 'history_path', 'limits', 'stats', 'v1467']
- ✅ `POST /audit/run` → `audit_run` (1.3ms) payload_keys=['audit_host', 'dry_run', 'elapsed_s', 'gateway_elapsed_s', 'module', 'n_endpoints_2xx', 'n_endpoints_total', 'n_invariants_failed', 'n_invariants_total', 'note', 'policy', 'verdict']
- ✅ `GET /audit/history` → `audit_history` (1.5ms) payload_keys=['entries', 'history_path', 'n_entries']
- ✅ `GET /audit/{audit_id}` → `audit_get` (9.8ms) payload_keys=[]
- ✅ `GET /audit/diff` → `audit_diff` (1.5ms) payload_keys=[]

## Errors

(none)

## GUARDS (主 00:44 质量工程化)

- `GUARD_V1468_REUSED`
- `GUARD_V1467_REUSED`
- `GUARD_TWO_PROCESSES`
- `GUARD_PORT_REUSED`
- `GUARD_HTTP_LIVE`
- `GUARD_CLIENT_GENERATED`
- `GUARD_CLIENT_DRIVER_RUNS`
- `GUARD_RESULT_FILE_PARSED`
- `GUARD_ALL_ENDPOINTS_HIT`
- `GUARD_BOUNDED_WALLCLOCK`
- `GUARD_SUBPROCESS_CLEANED`
- `GUARD_LINEAGE_CITED`
- `GUARD_RUNS_ON_WINDOWS`
- `GUARD_DETERMINISTIC`

## V3 哲学守门 (主 17:58 + 主 20:46 不假装)

- `GUARD_DRIVER_NOT_CI`
- `GUARD_DRIVER_NOT_LOAD_TEST`
- `GUARD_DRIVER_NOT_FUZZER`
- `GUARD_DRIVER_NOT_ASI`
- `GUARD_DRIVER_NOT_PHENOMENAL`
- `GUARD_DRIVER_NOT_HUMAN_LEVEL`
- `GUARD_DRIVER_NOT_ORCHESTRATOR`

## 借力 (主 19:33 走在前人经验上)

- v1468
- v1467
- v1466
- v1437
- stdlib

## Artifacts

- V1468-generated client: `AppData\Local\Temp\v1469_8ru1ae60_driver\v1467_client.py`
- Client driver result: `AppData\Local\Temp\v1469_8ru1ae60_driver\client_result.json`
