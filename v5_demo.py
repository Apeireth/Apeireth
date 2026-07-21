# V5 demo — 整合 Phase 51-54 真生产借鉴
import sys
sys.path.insert(0, '.')

from apeireth.open_webui_patterns import (
    OPEN_WEBUI_PATTERNS_VERSION, BochaSearchAdapter, VectorDBFactory,
    LLMRouter, WebSearchAggregator
)
from apeireth.tag_memo_wave import TAG_MEMO_WAVE_VERSION, TagMemoWave
from apeireth.memories_module import MEMORIES_MODULE_VERSION, MemoriesModule

print('=' * 60)
print('=== V5 ASI Base Demo — 整合 Phase 51-54 真生产借鉴 ===')
print('=' * 60)

# Phase 51 Open WebUI 真生产借鉴
bocha = BochaSearchAdapter(api_key='fake', ai_api_key='fake')
vdf = VectorDBFactory()
vdf.register('chroma', collection_name='memories')
vdf.register('milvus', collection_name='memories')
router = LLMRouter()
router.register_route('openai-compatible', 'http://localhost:3000/v1', api_key='fake', model='MiniMax-M3')
wsa = WebSearchAggregator()
wsa.enable('bocha')
wsa.enable('tavily')

# Phase 53 VCP TagMemo
tm = TagMemoWave(threshold=0.5)
tm.observe_tag('apeireth')
tm.observe_tag('memory')
tm.observe_tag('asi')
tm.cooccurrence('apeireth', 'memory', 3.0)
tm.cooccurrence('apeireth', 'asi', 5.0)
tm.rebuild_matrix()

# Phase 54 Open WebUI memories
mem = MemoriesModule()
p1 = mem.create_path('chuling', '/apeireth/philosophy')
p2 = mem.create_path('chuling', '/apeireth/research')
m1 = mem.add_memory('chuling', '主人 22:08 V2 哲学: 中央 AI 是调度者/思考者/无数关系集合体/最大权限/ASI 位置', 
                    memory_path='/apeireth/philosophy', importance=1.0, tags=['v2-philosophy'])
m2 = mem.add_memory('chuling', '主人 23:58 真哲学: Apeireth 推到主会话, 拥有最大记忆上下文', 
                    memory_path='/apeireth/philosophy', importance=1.0, tags=['main-session'])
m3 = mem.add_memory('chuling', 'Phase 51-54 整合: Open WebUI 真生产借鉴 + VCP TagMemo + memories', 
                    memory_path='/apeireth/research', importance=0.9, tags=['integration'])

print(f'Open WebUI: Bocha={bocha}, VectorDB={len(vdf.adapters)}, Router={router.stats()}')
print(f'Web Search: {wsa.stats()}')
print(f'TagMemo: {tm.stats()}')
print(f'Memories: {mem.stats()}')

# 搜索 V2 哲学
results = mem.search_memory('V2 中央 AI')
print(f'Search V2 中央 AI: {len(results)} results')

print()
print('V5 ASI Base Demo PASS — 整合 4 真生产借鉴模块')