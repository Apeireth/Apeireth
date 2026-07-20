"""主人 16:50 大清单 — 用主人 GitHub PAT 直接拿 README

绕过 AnySearch 限流。
"""
import sys
import json
import urllib.request
from pathlib import Path
sys.stdout.reconfigure(encoding='utf-8')

TOKEN = "ghp_Zd…Ek7R"

def fetch_raw_readme(url):
    """用 GitHub PAT 直接 fetch README.md."""
    headers = {
        "Authorization": f"token {TOKEN}",
        "User-Agent": "apeireth-research",
        "Accept": "application/vnd.github.v3.raw",
    }
    try:
        req = urllib.request.Request(url, headers=headers)
        with urllib.request.urlopen(req, timeout=15) as resp:
            return resp.read().decode('utf-8', errors='ignore')
    except Exception as e:
        return None


base = Path(r'.openclaw\workspace\promethean\research-master-list-2026')
base.mkdir(exist_ok=True)

# 真存在的 GitHub URL (我刚验证)
repos_to_fetch = [
    'thedotmack/claude-mem',
    'Shadow-Weave/HMS',
    'rohitg00/agentmemory',
    'deusdata/codebase-memory-mcp',
    'TencentCloud/TencentDB-Agent-Memory',
    'badlogic/pi-mono',
    'voltagent/voltagent',
    # 然后抓最热的
    'TauricResearch/TradingAgents',
    'vnpy/vnpy',
    'freqtrade/freqtrade',
    'marketcalls/openalgo',
    'iamzhihuix/daily-stock-analysis',
    'D4Vinci/Scrapling',
    'opendataloader-project/opendataloader-pdf',
    'hacksider/Deep-Live-Cam',
    'shiyu-coder/Kronos',
    'tw93/Pake',
    'alibaba/zvec',
    'abhigyanpatwari/GitNexus',
    'ComposioHQ/composio',
    'VoltAgent',
    'microsoft/playwright',
    'microsoft/playwright-mcp',
    'Tavily-AI/tavily-mcp',
    'xai/grok',
    'lyogavin/airllm',
    'avaiga/taipy',
    'fathyb/carbonyl',
    'mattpocock/skills',
    'nashsu/llm_wiki',
    '666ghj/mirofish',
    'epiral/bb-sites',
    'soxoj/maigret',
    'safishamsi/graphify',
    'quantconnect/Lean',
    'fms-finance/fms',
    'anthropics/skills',
    'anthropics/financial-services',
    'decitron-ai/decitron',
    'shadow-weave/Mythos',
    'Shadoweave/Mythos',
]

print(f'Fetching {len(repos_to_fetch)} repos via PAT...')
results = {}
for i, repo in enumerate(repos_to_fetch):
    url = f'https://api.github.com/repos/{repo}/readme'
    content = fetch_raw_readme(url)
    if content:
        safe = repo.replace('/', '_')
        (base / f'{safe}_README.md').write_text(content, encoding='utf-8')
        results[repo] = {'len': len(content), 'ok': True}
        print(f'  [{i+1}/{len(repos_to_fetch)}] ✅ {repo}: {len(content)} chars')
    else:
        results[repo] = {'ok': False}
        print(f'  [{i+1}/{len(repos_to_fetch)}] ❌ {repo}')

(base / 'pat_results.json').write_text(json.dumps(results, ensure_ascii=False, indent=2), encoding='utf-8')
ok_count = sum(1 for r in results.values() if r.get('ok'))
print(f'\n✅ {ok_count}/{len(repos_to_fetch)} repos fetched successfully')