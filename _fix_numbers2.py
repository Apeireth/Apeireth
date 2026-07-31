#!/usr/bin/env python3
"""Final pass - remaining 5 numeric fixes"""
from pathlib import Path

TARGET = Path('.openclaw/workspace/promethean/APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md')

content = TARGET.read_text(encoding='utf-8')

# Replace by exact line substrings (safer)
exact_replacements = [
    # Line 56: TL;DR table
    ('| **真生产 tests** | **4938** (187 passed V1136真测) | crank self-test 187 passed |',
     '| **真生产 tests** | **6394** (187 passed V1136真测子集; snapshot snap_9c80c9165625 n_tests=6394) | `crank self-test` 累计通过测试 |'),

    # Line 334: §4.1 v-modules
    ('| **真生产 v-modules** | **1152** | `ls apeireth/v*.py | wc -l` = 1153 - 1 = 1152 (含 v1132/v1133/v1134/v1135/v1136) |',
     '| **真生产 v-modules** | **1153** | `ls apeireth/v*.py | wc -l` = 1153 (含 v1132/v1133/v1134/v1135/v1136) |'),

    # Line 335: §4.1 tests
    ('| **真生产 tests** | **4938** | `crank self-test` 累计通过测试 |',
     '| **真生产 tests** | **6394** | `crank self-test` 累计通过测试 (snap_9c80c9165625) |'),

    # Line 336: §4.1 commits
    ('| **真生产 commits** | **508** | `git log --oneline | wc -l` = 542 含早期历史; 主分支真生产 = 508 |',
     '| **真生产 commits** | **542** | `git log --oneline | wc -l` = 542 (snap_9c80c9165625 n_commits=542) |'),

    # Line 344: 注释
    ('- ASI 真测累计测试 = **4938** (通过历史累计 cron tick 持续跑)',
     '- ASI 真测累计测试 = **6394** (通过历史累计 cron tick 持续跑; snap_9c80c9165625)'),

    # Line 1441: 总结
    ('- 1153 modules / 6394 tests / 508 commits',
     '- 1153 modules / 6394 tests / 542 commits'),
]

fixes_done = []
for old, new in exact_replacements:
    if old in content:
        content = content.replace(old, new)
        fixes_done.append(f"✅ {old[:60]}...")
    else:
        fixes_done.append(f"❌ NOT FOUND: {old[:60]}...")

# Note: also fix "0.8290"笔误 if found (P0-1)
if '0.8290' in content:
    content = content.replace(
        '0.8290 vs V0.3 = 0.8964',
        '// 注: 原 L251 "0.8290" 是早期占位字, 与 V0.4=0.8031 矛盾, 已删除'
    )

with TARGET.open('w', encoding='utf-8') as f:
    f.write(content)

print("FINAL FIXES APPLIED:")
for fix in fixes_done:
    print(f"  {fix.encode('ascii', 'replace').decode('ascii')}")
print(f"\nFile: {TARGET.stat().st_size}B / {sum(1 for _ in TARGET.open(encoding='utf-8'))} lines")

# Verify
v = TARGET.read_text(encoding='utf-8')
print(f"\nVerification:")
print(f"  '4938' remaining: {v.count('4938')}")
print(f"  '6394' new: {v.count('6394')}")
print(f"  '1152' remaining: {v.count('1152')}")
print(f"  '1153' new: {v.count('1153')}")
print(f"  '508' remaining: {v.count('508')}")
print(f"  '542' new: {v.count('542')}")
print(f"  '0.8290' remaining: {v.count('0.8290')}")
