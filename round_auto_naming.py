#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round auto-naming — 主 00:49 真务实.
Cron 是"提醒"不是"机械执行", agentTurn 触发时用它自决下一轮编号.

用法:
    python round_auto_naming.py            # 返回下一轮编号 (int)
    python round_auto_naming.py --json     # 返回完整状态 (json)

规则 (主 00:49):
1. 读 promethean/research-v7-round-*.json, 取 max 编号 N
2. 下一轮 = N + 1
3. 如果 research-v7-round-{N+1}.json 已存在且 >1KB, 说明被另一 cron 抢了
   - 默认选 A: 跳过这次, 返回 -1 (agentTurn 报告"已被占")
   - 可选 B: 加时间戳后缀, e.g. round-{N+1}-20260721T0248.json
4. 如果 N+1.json 不存在, 返回 N+1
5. 如果 N+1.json 存在但 <1KB (半成品, 上次 cron 写一半挂了), 返回 N+1 (覆盖)
"""
import sys
import json
import glob
import os
import re
from pathlib import Path

PROMETHEAN = Path(r'.openclaw\workspace\promethean')
PATTERN = str(PROMETHEAN / 'research-v7-round-*.json')


def detect_next_round(allow_stale_overwrite: bool = True):
    """检测下一轮 round 编号.
    
    Returns:
        int: N+1 (正常情况)
        int: -1 (N+1 已被另一 cron 占用, 跳过)
        dict: 含 next, conflict, existing, stale 等字段 (--json 模式)
    """
    files = sorted(glob.glob(PATTERN))
    nums = []
    file_map = {}
    for f in files:
        m = re.search(r'round-(\d+)(?:-(\d+))?\.json', os.path.basename(f))
        if m:
            n = int(m.group(1))
            nums.append(n)
            file_map[n] = {
                'path': f,
                'size': os.path.getsize(f),
                'mtime': os.path.getmtime(f),
                'stale': os.path.getsize(f) < 1024,
            }
    
    if not nums:
        result = {
            'next': 1,
            'conflict': False,
            'existing': {},
            'note': 'first round',
        }
    else:
        n_max = max(nums)
        next_n = n_max + 1
        next_file = PROMETHEAN / f'research-v7-round-{next_n}.json'
        
        if next_file.exists():
            size = next_file.stat().st_size
            if size < 1024 and allow_stale_overwrite:
                # 半成品, 覆盖
                result = {
                    'next': next_n,
                    'conflict': False,
                    'stale_overwrite': True,
                    'existing': file_map,
                    'note': f'round-{next_n}.json stale ({size} bytes), overwrite',
                }
            else:
                # 已被另一 cron 占用, 跳过
                result = {
                    'next': -1,
                    'conflict': True,
                    'existing': file_map,
                    'note': f'round-{next_n}.json taken ({size} bytes), skip',
                }
        else:
            result = {
                'next': next_n,
                'conflict': False,
                'existing': file_map,
                'note': f'round-{next_n} free',
            }
    
    return result


def main():
    if '--json' in sys.argv:
        print(json.dumps(detect_next_round(), indent=2, ensure_ascii=False))
    else:
        r = detect_next_round()
        print(f'next: {r["next"]}')
        print(f'note: {r["note"]}')
        if r.get('existing'):
            print(f'existing: {sorted(r["existing"].keys())}')


if __name__ == '__main__':
    main()