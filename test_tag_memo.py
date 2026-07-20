#!/usr/bin/env python3
"""Phase 53 VCP TagMemo test."""
import sys
sys.path.insert(0, '.')

from apeireth.tag_memo_wave import TAG_MEMO_WAVE_VERSION, TagMemoWave

print(f'Phase 53 VCP TagMemo 浪潮算法 Python 复刻: {TAG_MEMO_WAVE_VERSION}')

tm = TagMemoWave(threshold=0.5)

tags_to_observe = ['agent', 'memory', 'consciousness', 'asi', 'apeireth', 'rust', 'python']
for t in tags_to_observe:
    tm.observe_tag(t)
print(f'Observe {len(tags_to_observe)} tags')

cooccurrences = [
    ('agent', 'memory', 3.0),
    ('agent', 'consciousness', 2.0),
    ('memory', 'consciousness', 5.0),
    ('asi', 'apeireth', 4.0),
    ('rust', 'python', 2.0),
    ('asi', 'consciousness', 3.0),
]
for a, b, w in cooccurrences:
    tm.cooccurrence(a, b, w)

tm.rebuild_matrix()

print(f'  stats: n_tags={tm.stats()["n_tags"]}, n_pairs={tm.stats()["n_pairs"]}, energy={tm.stats()["energy_field"]:.3f}')
am = tm.pair_similarity('agent', 'memory')
ac = tm.pair_similarity('asi', 'consciousness')
print(f'  agent-memory sim: {am:.3f}')
print(f'  asi-conscious sim: {ac:.3f}')
print('OK Phase 53')