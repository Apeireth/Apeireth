#!/usr/bin/env python3
"""Phase 52 Cron Self-Update test."""
import sys
sys.path.insert(0, '.')

from apeireth.cron_self_update import (
    CRON_SELF_UPDATE_VERSION, CronSelfUpdater,
    git_log_oneline, count_apeireth_modules, compute_v0_1_index
)

print(f'Phase 52 Cron Self-Update: {CRON_SELF_UPDATE_VERSION}')

log = git_log_oneline(5)
print(f'git log: {len(log)} commits')
n = count_apeireth_modules()
print(f'真生产 module: {n}')
idx = compute_v0_1_index()
print(f'ASI Index V0.1: {idx}')

csu = CronSelfUpdater()
stats = csu.stats()
print(f'stats: n_modules={stats["n_modules"]}, asi={stats["asi_index_v0_1"]}')

msg = csu.build_message()
print(f'message length: {len(msg)} chars')
print('OK Phase 52 Cron Self-Update')