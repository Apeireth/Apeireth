#!/usr/bin/env python3
"""Apeireth philosophy V2 test — 主人 22:08 真哲学."""
import sys
sys.path.insert(0, '.')
from apeireth.philosophy import (
    PHILOSOPHY_VERSION, PHILOSOPHY_LINES, check_philosophy,
    central_ai_position_v2, apeireth_philosophy_summary
)
print(f'Philosophy version: {PHILOSOPHY_VERSION}')
print(f'N lines: {len(PHILOSOPHY_LINES)}')

print('\n=== Central AI Position V2 ===')
print(central_ai_position_v2())

print('\n=== Philosophy Summary V2 ===')
print(apeireth_philosophy_summary())

print('\n=== V2 Tests ===')
check1 = check_philosophy(
    'V2_CentralAI',
    '中央 AI 是调度者/思考者, 是无数关系的集合体, 有最大权限, 中央 AI 位置 = ASI 位置',
)
print(f'  V2 test 1 (true): passed={check1.passed}, deviations={len(check1.deviations)}')

check2 = check_philosophy(
    'BadV2',
    '中央 AI 不是调度者, 只是 Klein bottle',
)
print(f'  V2 test 2 (V1 wrong): passed={check2.passed}, deviations={len(check2.deviations)}')
for d in check2.deviations:
    print(f'    line={d["line"]} pattern={d["pattern_matched"]}')
print('OK V2 work')
