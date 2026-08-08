# V1383 archive-health dashboard

- **schema:** `v1383.dashboard/v1`
- **generated:** 2026-08-09T05:25:00Z
- **latest tick id:** `tick-2026-08-09T05-25-00Z-f03d`
- **v1383_version:** 0.1.0
- **tag:** `cron-5min-test-2`

## Latest totals

- **archives on disk:** 1
- **indexed (V1375 INDEX.md rows):** 0
- **manifested (V1379 records):** 1
- **integrity:** `OK`

## Tier distribution

| tier | count |
|------|------:|
| HOT | 1 |
| WARM | 0 |
| COLD | 0 |
| FROZEN | 0 |

## Rotation actions (V1381 plan)

| action | count |
|--------|------:|
| keep | 1 |
| compress | 0 |
| prune | 0 |

## Drift (latest vs previous)

- **archives_delta:** `0`
- **integrity_status_delta:** `ok->ok`
- **tier_distribution_delta:** `{'HOT': 0, 'WARM': 0, 'COLD': 0, 'FROZEN': 0}`
- **action_counts_delta:** `{'keep': 0, 'compress': 0, 'prune': 0}`
- **integrity_changed:** `False`
- **archives_changed:** `False`

## Last 2 ticks

| ts | tick_id | archives | integrity | tag |
|----|---------|---------:|-----------|-----|
| 2026-08-09T05:25:00Z | `tick-2026-08-09T05-25-00Z-f03d` | 1 | OK | cron-5min-test-2 |
| 2026-08-08T21:23:46Z | `tick-2026-08-08T21-23-46Z-b5e1` | 1 | OK | cron-5min-test |

## Guards upheld

- `GUARD_CRON_SAFE`
- `GUARD_HISTORY_APPEND_ONLY`
- `GUARD_NO_CAP_CHANGE`
- `GUARD_DETERMINISTIC`
- `GUARD_ATOMIC_WRITE`
- `GUARD_NO_TOUCH_V1382`
- `GUARD_LOCAL_FILESYSTEM_ONLY`
- `GUARD_HONEST_DISCLOSURE`
- `GUARD_DASHBOARD_PURE`
- `GUARD_DRIFT_PURE`

## Known unknowns

- drift only compares archive count + integrity status + tier counts; does not deep-diff archive contents or INDEX.md
- tick_id microsecond hash is deterministic given (schema, ts, microsecond); not a cryptographic fingerprint
- dashboard is a snapshot of recent ticks; does not project future state
