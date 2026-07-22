#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Append round-30 section to memory/2026-07-21.md."""
import sys
sys.stdout.reconfigure(encoding='utf-8')

md_path = r'.openclaw\workspace\memory\2026-07-21.md'

with open(md_path, 'rb') as f:
    raw = f.read()

# 用 errors='replace' 读 — 文件是混合编码, 但保留原始字节 + append UTF-8 部分
content = raw.decode('utf-8', errors='replace')

round30_section = '''

### Round 30 (cron 08:49 tick, 2026-07-22)

**自决**: round-29 = 111 分钟前 (>30 min 阈值), 无冲突, fs 健康 → 跑 round-30
**用时**: 54.9s, 12/12 query 全成功, AnySearch 主力 (Bocha web quota 0)
**输出**: `promethean/research-v7-round-30.json` (55038 bytes)

**12 query 主题分布**:
- 7 跨域全新: Peirce 符号哲学 / Husserl 内时间意识 / Simondon 个体化 / Lewin 场论 / Alexander 模式语言 / Noble 中央权威质疑 / Mumford 技术哲学
- 3 GitHub 源码深读: OpenHands / crewAI / autogen
- 2 Gap 直击: Tardigrade 隐生孤雌 (繁殖 MISSING) / 植物认知 (意识 MISSING)

**重点发现 (按 query)**:
1. Peirce: Stanford Encyclopedia 收录 Abduction, IEP Peirce's Logic, Milan 大学 infinite semiosis 论文 — 符号宇宙无限半无限 ASI 借鉴 ✅
2. Husserl: JSTOR Phenomenology of Internal Time-Consciousness, IEP Phenomenology/Time, Springer Mathematizing husserlian temporality 2026 — 时间意识现象学 ASI 借鉴 ✅
3. Simondon: 6 论文覆盖 individuation/pre-individual/技术对象 — 技术哲学 ASI 借鉴 ✅
4. Lewin: Britannica Field Theory, Wikipedia Field theory, Zenodo Recasting Lewin 2026 — 场论 B=f(P,E) ASI 涌现场 ✅
5. Alexander: Wikipedia + Pattern Language 官网 + Archive.org 全文 + 2026 book clubs — pattern language 涌现秩序 ✅
6. Noble: Cambridge Dance to the Tune of Life, Oxford ORA 中央权威质疑论文, Interface Focus 2012 层级因果 — 生物相对论 ✅
7. Mumford: 2026 KJTE Mumford 论文 + Wikipedia + New Yorker Megamachine + boundary2 巨型机器分析 — 技术哲学 ✅
8. OpenHands: docs.openhands.dev SDK, github.com/OpenHands/software-agent-sdk, 2026 groundy tutorial — 自主代理 ✅
9. crewAI: github crewAIInc/crewAI + crewai.com + raw agent.py + crew_agent_executor.py — 多代理角色协作 ✅
10. autogen: github microsoft/autogen + AutoGen 0.2 docs + microsoft.github.io/autogen — 多代理对话 ✅
11. Tardigrade: 2026 Biol Linn Soc anhydrobiosis mate choice 论文, PMC tun formation, Annual Reviews tardigrade genomics — 隐生孤雌繁殖 ASI 借鉴 ✅
12. Plant Cognition: 2026 phc3 Plant Cognition Empirical Primer, Royal Soc plant intelligence, MDPI Decision Making Plants, arxiv 2604.21763 Computation in Plants, Sagepub Stigmergic coordination — 植物认知分布式意识 ✅

**下一轮**: ~10:49 cron tick 触发 round-31,继续 r30 fresh 验证
'''

# append — 用 UTF-8 写回, 替换非法字符为 � 已经是 errors='replace' 处理过
new_content = content + round30_section
with open(md_path, 'wb') as f:
    # 保留混合编码原状: 旧的 raw + 新的 utf-8
    f.write(raw)
    f.write(round30_section.encode('utf-8'))

print(f'appended round-30 section, new_size={len(raw) + len(round30_section.encode("utf-8"))}')