"""主人 16:50 大清单 — 60+ 项目真调研

策略:
- 按领域分组 (memory / agent / financial / scraping / OCR / design / ...)
- 真调研 30+ 项目 README
- 提取每个项目的核心借鉴点
"""
import sys
import json
from pathlib import Path
sys.stdout.reconfigure(encoding='utf-8')
from apeireth import AnySearch, GitHubResearch

s = AnySearch()
g = GitHubResearch()

base = Path(r'.openclaw\workspace\apeireth\research-master-list-2026')
base.mkdir(exist_ok=True)

# 主人清单 - 按领域分类
master_list = {
    'Memory & Cognition': [
        'claude-mem',
        'Shadoweave-team/HMS-holographic-memory-system',
        'rohitg00/agentmemory',
        'MemPalace',
        'DeusData/codebase-memory-mcp',
        'TencentDB/Agent-Memory',
        'YintaTriss/AgentMemory',  # 我们自己的
    ],
    'Agent Frameworks': [
        'OpenSquilla',
        'exo-explore',
        'project-nomad',
        'pi-mono',
        'ComposioHQ/composio',
        'VoltAgent',
        'Odysseus-AI',
        'Tavily-ai/tavily-mcp',
        'microsoft/playwright-mcp',
        'Dexter-AI',
        'simular-ai/Agent-S',
        'agent-reach',
    ],
    'Financial / Trading': [
        'HKUDS/TradingAgents',
        'vnpy/vnpy',
        'freqtrade/freqtrade',
        'TauricResearch/TradingAgents',
        'juanjuandog/FinSight-AI',
        'OpenStock-finance',
        'HKUDS/Vibe-Trading',
        'TraderAlice/OpenAlice',
        'QuantConnect/Lean',
        'marketcalls/openalgo',
        'StockSharp/StockSharp',
        'OpenByteInc/QuantDinger',
        'rmbell09-lang/tradesight',
        'iamzhihuix/daily-stock-analysis',
        'anthropics/financial-services',
        'FinanceDatabase',
    ],
    'Scraping & Data': [
        'D4Vinci/Scrapling',
        'Scraplin-ai',
        'multica-ai/andrej-karpathy-skills',
        'opendataloader-project/opendataloader-pdf',
        'camofox-browser',
        '666ghj/mirofish',
        'epiral/bb-sites',
        'soxoj/maigret',
    ],
    'OCR & Vision': [
        'Unlimited-OCR',
        'hyOCR1.5',
        'hacksider/Deep-Live-Cam',
        'shiyu-coder/Kronos',
        'TimesFM',
        'nolangz/pixel2motion',
        'decitron',
    ],
    'Models & Inference': [
        'lyogavin/airllm',
        'avaiga/taipy',
        'xai/grok-build',
        'Karpathy-upgrade',
        'getcompanion-ai/feynman',
        'anysearch-ai/anysearch-skill',  # 我们已经用了
    ],
    'Design & UI': [
        'tw93/Pake',
        'VoltAgent/awesome-design-md',
        'nicejade/markdawn-online-editor',
        'ali-build/agent-reach',
    ],
    'Document & Wiki': [
        'nashsu/llm_wiki',
        'langchain-ai/openwiki',
        'openscience',
        'wechat-article-exporter',
    ],
    'Architecture & Systems': [
        'open-mythos',
        'Mythos-architecture-reversed',
        'Shadoweave-HMS',
        'Self-herness',
        'abhigyanpatwari/GitNexus',
        'alibaba/zvec',
        'fathyb/carbonyl',
        'T3MP3ST',
        'Terax-Project',
        'BilldDesk-Pro',
        'Self-Improving',
        'safishamsi/graphify',
        'mattpocock/skills',
        'iamzhihuix/skills-manage',
    ],
    'Marketing / Content': [
        'yikart/AiToEarn',
        'cli-angthing',
    ],
}

# 找到 GitHub URL
print('=' * 70)
print('A. 搜 GitHub URL')
print('=' * 70)

url_map = {}
for category, projects in master_list.items():
    print(f'\n--- {category} ---')
    for proj in projects:
        try:
            r = s.search(f'github {proj} repository', max_results=1)
            if r['ok']:
                d = r['data']
                if isinstance(d, dict):
                    for c in d.get('content', []):
                        if c.get('type') == 'text':
                            text = c['text']
                            # extract github.com/owner/repo
                            import re
                            matches = re.findall(r'github\.com/([\w\-]+/[\w\-\.]+)', text)
                            if matches:
                                url_map[proj] = matches[0]
                                print(f'  {proj} → github.com/{matches[0]}')
                                break
        except Exception as e:
            print(f'  {proj} FAIL: {e}')

(base / 'url_map.json').write_text(json.dumps(url_map, ensure_ascii=False, indent=2), encoding='utf-8')

# B. 真抓 README
print('\n' + '=' * 70)
print('B. 真抓 README (top 30)')
print('=' * 70)

readme_data = {}
for i, (proj, url) in enumerate(url_map.items()):
    if i >= 30:
        break
    print(f'\n--- {proj} ({url}) ---')
    md = g.fetch_readme(url)
    if md and len(md) > 500:
        safe = url.replace('/', '_')
        (base / f'{safe}_README.md').write_text(md, encoding='utf-8')
        print(f'  saved {len(md)} chars')
        readme_data[proj] = {'url': url, 'len': len(md)}
    else:
        print(f'  FAILED')

(base / 'readme_summary.json').write_text(json.dumps(readme_data, ensure_ascii=False, indent=2), encoding='utf-8')
print(f'\n\nSaved {len(readme_data)} project summaries')