#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""V7 round-14 — 主 00:51 验证 cron 端到端 + 跨域 ASI 真生产借鉴
主 00:49 真务实: 调研不停 + 验证 round5-v3 cron 没有错误
基于 round-13 + 主 00:51 prompt, 替换/深化, 不重复
"""
import sys, json, time
sys.stdout.reconfigure(encoding='utf-8')
sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research

# 12 query = 主 00:46 整合 Apeireth 内涵 + 主 00:49 验证 + 不重复 round-13
# 7 跨域 + 3 GitHub 源码 + 2 Apeireth Gap
QUERIES = [
    # === 跨域 ASI 真生产借鉴 (7) ===
    # 1. 生态学 ASI — 深化 round-13 的 Agentic Hives
    "agentic hives 2026 follow-up work reproduction multi-agent ecology",
    # 2. 系统论 + ASI — 跨域借鉴
    "Ashby requisite variety cybernetics AI control 2026",
    # 3. 涌现 — 平台不调度 (主 13:51 APEIRETH-EXPLAINED Layer 5)
    "emergence self-organization edge of chaos artificial life 2026",
    # 4. 自演化 — 红皇后 (主 20:55 ASI-LIFE-FEATURES-V4)
    "Red Queen hypothesis AI self-improvement arms race 2026",
    # 5. 中央 AI 哲学 — 哲学谱系
    "Nous Anaxagoras active mind organizing cosmos AI",
    # 6. 自创生 Maturana/Varela — 生物学借鉴
    "Maturana autopoiesis self-producing organization AI 2026",
    # 7. 复杂适应系统 — John Holland
    "John Holland complex adaptive systems genetic algorithm AI",

    # === GitHub 优秀项目源码深入 (3) ===
    # 8. ASI-Arch 源码深入 (round-13 已找到, 现在深入)
    "ASI-Arch source code architecture search agent implementation github",
    # 9. DGM Darwin Godel Machine 源码深入
    "Darwin Godel Machine source code archive jennyzzt implementation",
    # 10. HarnessAgent 真生产 (主 00:21 ⭐⭐ Production-grade)
    "HarnessAgent production multi-agent harness memory safety github",

    # === Apeireth Gap 借鉴 (2) ===
    # 11. 繁殖 IdentityCard.export — 主 17:46 12 生命特征最大 gap
    "IdentityCard export seed cross-platform agent reproduction 2026",
    # 12. 意识 Phenomenal — 主 17:58 终极目标 + V4 哲学守门
    "Phenomenal consciousness engineering AI self-awareness hard problem 2026",
]

def main():
    t0 = time.time()
    results = []
    for i, q in enumerate(QUERIES, 1):
        t_q = time.time()
        r = dual_research(q, top_k=5)
        results.append(r)
        bw = len(r['bocha_web'])
        as_ = len(r['anysearch'])
        ai = len(r['bocha_ai_answer'])
        ms = len(r['merged_sources'])
        print(f'[{i:2d}/12] {q[:55]:55s} | bocha_w={bw} anysearch={as_} ai={ai:4d} merged={ms} ({time.time()-t_q:.1f}s)')
    
    out = r'.openclaw\workspace\promethean\research-v7-round-14.json'
    with open(out, 'w', encoding='utf-8') as f:
        json.dump(results, f, indent=2, ensure_ascii=False)
    
    print(f'\n=== round-14 done in {time.time()-t0:.1f}s, saved {len(results)} queries ===')
    print(f'output: {out}')

if __name__ == '__main__':
    main()