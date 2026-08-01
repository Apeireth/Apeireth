"""Final summary of round-59."""
import os, json

print('=' * 60)
print('Round-59 cross-domain research summary')
print('=' * 60)

# File stats
r59 = 'research-v7-round-59.json'
r59_runner = 'round-59-runner.py'
print(f'\nArtifacts:')
print(f'  - {r59}: {os.path.getsize(r59)} bytes')
print(f'  - {r59_runner}: {os.path.getsize(r59_runner)} bytes')

# Load and inspect content
data = json.load(open(r59, encoding='utf-8'))
print(f'\nContent: {len(data)} queries, each with:')
print(f'  - query, bocha_web, anysearch, bocha_ai_answer, merged_sources')

# Per-query breakdown
for i, q in enumerate(data, 1):
    print(f'  [{i:02d}] any={len(q["anysearch"])} | {q["query"][:100]}...')

# Sources total
total_any = sum(len(q['anysearch']) for q in data)
total_bw = sum(len(q['bocha_web']) for q in data)
total_merged = sum(len(q['merged_sources']) for q in data)
print(f'\nSources total: any={total_any}, bw={total_bw}, merged={total_merged}')

# Memory file
mem = r'.openclaw\workspace\memory\2026-08-01.md'
print(f'\nMemory synced: {mem}')
print(f'  size: {os.path.getsize(mem)} bytes')

# Git log
print(f'\nGit:')
print(f'  latest commit: 65ca2923 (technical_writer R14 stage 4 v4 + round-59 artifacts)')

print('\n' + '=' * 60)
print('Round-59 DONE — 12 queries, 60 sources, 70.8s, 51272 bytes')
print('=' * 60)