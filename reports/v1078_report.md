# V1078 Cron Self-Audit (2026-07-22T11:28:43+08:00)

**Overall: HEALTHY** 鈥?records=40, idle=0, fallback=0, errors=2

## Volume
- rounds: 31
- records: 40

## Reliability
- idle_timeout incidents: **0** (jsonl keyword match)
- fallback incidents: **0** (jsonl keyword match)
- errors: **2** (jsonl keyword match)

## Latency (research rounds done)
- p50: 64.80s
- p95: 64.80s
- max: 64.80s

## Cadence (gap between rounds)
- mean gap: 1682.3s
- std gap: 2971.4s
- drift z-score (latest gap vs mean): -0.56

## Provider Health
- Bocha AI: 0.0%
- AnySearch: 5.0%
- Errors/exception provider: 0.0%

## Components
| name | value | source | confidence | note |
| --- | --- | --- | --- | --- |
| records_parsed | 40 | jsonl | high | skipped 0 malformed lines |
| rounds_observed | 31 | jsonl | high | max round number in jsonl |
| idle_timeout_count | 0 | jsonl_keyword | medium | matched: idle timeout,idle-timeout,idle_to, ... |
| fallback_count | 0 | jsonl_keyword | medium | matched: fallback,deepseek,switch,replaced |
| error_keyword_count | 2 | jsonl_keyword | medium | matched: error,fail,exception, ... |
| p95_duration | 64.80s | jsonl_done | low | sample size=1 |
| cadence_drift_z | -0.56 | jsonl_timestamps | high | mean gap 1682s, std 2971s |
| provider_diversity | bocha=0% any=5% err=0% | jsonl | high | percentage of records using each provider |

