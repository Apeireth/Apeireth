#!/usr/bin/env python3
"""Fix the 3 major data errors caught by peer review"""
from pathlib import Path

TARGET = Path('.openclaw/workspace/promethean/APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md')

content = TARGET.read_text(encoding='utf-8')

# P0-1: 4938 tests → 6394 tests (peer reviewer caught, snapshot is authoritative)
fixes = []

# Comprehensive replacements for 4938 → 6394
mapping_4938 = [
    ('4938 tests', '6394 tests'),
    ('4938 真测试', '6394 真测试'),
    ('4938 真测', '6394 真测'),
    ('= 4938', '= 6394'),
    ('4938 (通过', '6394 (通过'),
    (', 4938 tests, 187 passed', ', 6394 tests, 187 passed'),
]
for old, new in mapping_4938:
    count = content.count(old)
    if count > 0:
        content = content.replace(old, new)
        fixes.append(f"{old} → {new}: {count}x")

# 1152 modules → 1153 modules (peer reviewer caught n_modules mismatch)
mapping_1152 = [
    ('1152 modules (max V1131', '1153 modules (max V1131'),
    ('1152 modules', '1153 modules'),
    ('**1152** (max V1131', '**1153** (max V1131'),
    ('= **1152**', '= **1153**'),
    ('当前真测 **1152**', '当前真测 **1153**'),
]
for old, new in mapping_1152:
    count = content.count(old)
    if count > 0:
        content = content.replace(old, new)
        fixes.append(f"{old} → {new}: {count}x")

# 508 commits → 542 commits (peer reviewer caught n_commits mismatch)
mapping_508 = [
    ('**508** (master HEAD = f17b7ad1)', '**542** (master HEAD = f17b7ad1)'),
    ('1152 modules / 4938 tests / 508 commits', '1153 modules / 6394 tests / 542 commits'),
    ('真生产 commits | **508**', '真生产 commits | **542**'),
    ('| 真 commit | **508**', '| 真 commit | **542**'),
    ('(/working_i/usr)真 commit', '真 commit'),  # bad pattern
    ('1152 / 4938 / 508', '1153 / 6394 / 542'),
    ('1152 modules + 4938 tests + 508 commits', '1153 modules + 6394 tests + 542 commits'),
]
for old, new in mapping_508:
    count = content.count(old)
    if count > 0:
        content = content.replace(old, new)
        fixes.append(f"{old} → {new}: {count}x")

# §6 fix: "11 关键 module" → "12 关键 module"
content = content.replace(
    '### 6. 核心架构能力（11 关键 module）',
    '### 6. 核心架构能力（12 关键 module anchor）'
)
content = content.replace(
    '## 6. 核心架构能力（11 关键 module）',
    '## 6. 核心架构能力（12 关键 module anchor）'
)
fixes.append("§6 '11 module' → '12 module anchor': 1x")

# Appendix B
content = content.replace(
    '**R7 (15 reports)**',
    '**R7 (15 reports)**'
)

# Add as-of timestamp to TL;DR table
asof_marker = '*真测 as of snap_9c80c9165625 (2026-07-30 02:10:51 UTC)*'
content = content.replace(
    '**一句话定位 Apeireth**',
    asof_marker + '\n\n**一句话定位 Apeireth**'
)
fixes.append(f"Added as-of marker to TL;DR")

with TARGET.open('w', encoding='utf-8') as f:
    f.write(content)

print("FIXES APPLIED:")
for fix in fixes:
    print(f"  - {fix}")

# Re-verify
v = TARGET.read_text(encoding='utf-8')
print(f"\nNew file: {TARGET.stat().st_size}B / {sum(1 for _ in TARGET.open(encoding='utf-8'))} lines")
print(f"'6394' count: {v.count('6394')}")
print(f"'508' count: {v.count('508')}")
print(f"'4938' count: {v.count('4938')}")
print(f"'1152' count: {v.count('1152')}")
print(f"'1153' count: {v.count('1153')}")
print(f"'542' count (modules/commits): {v.count('542')}")
