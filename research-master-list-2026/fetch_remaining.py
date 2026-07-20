#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Fetch remaining GitHub READMEs from master list 16:50."""
import json
import os
import sys
import time
import urllib.request
import urllib.error
import urllib.parse
import base64

OUTPUT_DIR = r".openclaw\workspace\promethean\research-master-list-2026"
UA = "Mozilla/5.0 (PrometheanResearch)"

# (display_name, owners_to_try, skip_if_exists, search_hint)
TARGETS = [
    # (label, primary_url_or_guess, optional_alt_urls)
    ("iamzhihuix/daily-stock-analysis", ["iamzhihuix/daily-stock-analysis", "zhihuix/daily-stock-analysis"], None),
    ("decitron-ai/decitron", ["decitron-ai/decitron"], None),
    ("Shadoweave/Mythos", ["Shadoweave/Mythos", "shadoweave/Mythos", "Shadoweave/mythos", "Shadoweave/Mythos-AI"], None),
    ("shadow-weave/Mythos", ["shadow-weave/Mythos", "Shadow-Weave/Mythos", "Shadowweave/Mythos"], None),
    # openscience -> search
    ("openscience", ["Project-OpenScience/open-science", "open-science-ai/open-science", "open-science/open-science", "brainqub3/open-science"], "search:open-science AI memory"),
    ("opensquilla/opensquilla", ["opensquilla/opensquilla", "OpenSquilla/OpenSquilla"], None),
    # crosstalk-solutions/project-nomad - retry
    ("crosstalk-solutions/project-nomad", ["crosstalk-solutions/project-nomad", "Crosstalk-Solutions/Project-Nomad"], None),
    # exo-explore/exo - retry
    ("exo-explore/exo", ["exo-explore/exo", "exo-explore/Exo"], None),
    # ComposioHQ/composio - already exists, skip
    # VoltAgent -> voltagent/voltagent exists
    # Self-herness -> search
    ("Self-herness", ["self-herness/self-herness", "Self-Herness/self-herness", "selfherness/selfherness"], "search:Self-Herness AI"),
    # MemPalace/mempalace - retry
    ("MemPalace/mempalace", ["MemPalace/mempalace", "memPalace/mempalace", "mem-palace/mempalace"], None),
    # alchaincyf
    ("alchaincyf/deepseek-v4-deep-dive", ["alchaincyf/deepseek-v4-deep-dive", "Alchaincyf/deepseek-v4-deep-dive"], None),
    ("alchaincyf/zhangxuefeng-skill", ["alchaincyf/zhangxuefeng-skill", "Alchaincyf/zhangxuefeng-skill"], None),
    # Openhuman
    ("Openhuman", ["openhumanai/openhuman", "OpenHuman/openhuman", "openhuman/openhuman"], "search:Openhuman AI"),
    # Dexter-AI
    ("Dexter-AI", ["Dexter-AI/Dexter-AI", "dexter-ai/dexter", "DexterAI/dexter"], "search:Dexter-AI agent"),
    # simular-ai/Agent-S
    ("simular-ai/Agent-S", ["simular-ai/Agent-S", "simularai/Agent-S", "simular-ai/agents", "simular-ai/agent-s"], None),
    # HKUDS/Vibe-Trading
    ("HKUDS/Vibe-Trading", ["HKUDS/Vibe-Trading", "hkuds/Vibe-Trading", "HKUDS/VibeTrading"], None),
    # TraderAlice/OpenAlice
    ("TraderAlice/OpenAlice", ["TraderAlice/OpenAlice", "traderalice/OpenAlice", "OpenAlice/openalice"], None),
    # yikart/AiToEarn
    ("yikart/AiToEarn", ["yikart/AiToEarn", "Yikart/AiToEarn", "yikart/aitoearn"], None),
    # juanjuandog/FinSight-AI
    ("juanjuandog/FinSight-AI", ["juanjuandog/FinSight-AI", "Juanjuandog/FinSight-AI", "juanjuandog/FinSightAI"], None),
    # OpenStock-finance
    ("OpenStock-finance", ["OpenStock-finance/OpenStock", "openstock-finance/OpenStock", "OpenStockFinance/openstock"], "search:OpenStock finance"),
    # OpenByteInc/QuantDinger
    ("OpenByteInc/QuantDinger", ["OpenByteInc/QuantDinger", "openbyteinc/QuantDinger"], None),
    # rmbell09-lang/tradesight
    ("rmbell09-lang/tradesight", ["rmbell09-lang/tradesight", "Rmbell09-lang/tradesight", "rmbell09/TradeSight"], None),
    # StockSharp/StockSharp
    ("StockSharp/StockSharp", ["StockSharp/StockSharp", "stocksharp/StockSharp"], None),
    # wechat-article-exporter
    ("wechat-article-exporter", ["jooooel/wechat-article-exporter", "jooojioooj/wechat-article-exporter"], "search:wechat-article-exporter"),
    # nicejade/markdawn-online-editor
    ("nicejade/markdawn-online-editor", ["nicejade/markdown-online-editor", "nicejade/markdawn-online-editor"], "search:nicejade markdown online editor"),
    # T3MP3ST (likely T3mpest or t3st or t3mp3st?)
    ("T3MP3ST", [], "search:T3MP3ST OR T3mp3st OR t3mp3st"),
    # terax-project
    ("terax-project", [], "search:terax-project Rust"),
    # camofox-browser
    ("camofox-browser", ["distriqt/camofox-browser", "camofox/camofox-browser"], "search:camofox-browser"),
    # Unlimited-OCR
    ("Unlimited-OCR", [], "search:Unlimited-OCR"),
    # hyOCR1.5
    ("hyOCR1.5", [], "search:hyOCR1.5 OR hyOCR"),
    # cli-angthing
    ("cli-angthing", [], "search:cli-angthing"),
    # open-mythos
    ("open-mythos", ["open-mythos/open-mythos", "openmythos/open-mythos", "openmythos/mythos"], "search:open-mythos AI"),
    # yinta-triss (YintaTriss repos)
    ("yinta-triss", ["yinta-triss/yinta-triss", "yintatriss/yinta-triss", "YintaTriss/yinta-triss"], "search:YintaTriss github"),
]


def http_get(url, timeout=15):
    req = urllib.request.Request(url, headers={"User-Agent": UA, "Accept": "application/vnd.github.v3+json"})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            data = resp.read()
            return resp.status, data, dict(resp.headers)
    except urllib.error.HTTPError as e:
        return e.code, e.read(), dict(e.headers) if e.headers else {}
    except Exception as e:
        return -1, str(e).encode("utf-8"), {}


def fetch_readme(owner, repo):
    """Fetch README via unauthenticated GitHub API. Returns (status, text, stars)."""
    # First get repo metadata for stars
    status, data, _ = http_get(f"https://api.github.com/repos/{owner}/{repo}")
    stars = None
    if status == 200:
        try:
            meta = json.loads(data)
            stars = meta.get("stargazers_count")
        except Exception:
            pass
    # Then get README
    status, data, _ = http_get(f"https://api.github.com/repos/{owner}/{repo}/readme")
    if status != 200:
        return status, None, stars
    try:
        obj = json.loads(data)
        content = base64.b64decode(obj.get("content", "")).decode("utf-8", errors="replace")
        return 200, content, stars
    except Exception as e:
        return 500, None, stars


def safe_name(owner, repo):
    return f"{owner}_{repo}_README.md".replace("/", "_").replace("\\", "_")


def save_readme(owner, repo, content):
    path = os.path.join(OUTPUT_DIR, safe_name(owner, repo))
    with open(path, "w", encoding="utf-8") as f:
        f.write(content)
    return path


def update_results(results):
    path = os.path.join(OUTPUT_DIR, "remaining_results.json")
    with open(path, "w", encoding="utf-8") as f:
        json.dump(results, f, indent=2, ensure_ascii=False)


def search_github(q):
    """Use /search/repositories to find candidate. Returns list of full_name."""
    url = "https://api.github.com/search/repositories?q=" + urllib.parse.quote(q) + "&sort=stars&order=desc&per_page=5"
    status, data, _ = http_get(url)
    if status != 200:
        return [], status
    try:
        obj = json.loads(data)
        items = obj.get("items", [])
        return [(it["full_name"], it.get("stargazers_count", 0), it.get("html_url", "")) for it in items], 200
    except Exception:
        return [], -1


def main():
    results = {}
    summary = []
    
    for label, urls, hint in TARGETS:
        ok = False
        for u in urls or []:
            owner, _, repo = u.partition("/")
            print(f"[TRY] {label} -> {owner}/{repo}", flush=True)
            status, content, stars = fetch_readme(owner, repo)
            if status == 200 and content and len(content) > 200:
                path = save_readme(owner, repo, content)
                results[label] = {"url": f"{owner}/{repo}", "path": path, "len": len(content), "stars": stars, "ok": True}
                summary.append((label, f"{owner}/{repo}", stars, path))
                print(f"  OK ({len(content)} bytes) stars={stars}", flush=True)
                ok = True
                break
            else:
                print(f"  FAIL status={status}", flush=True)
            time.sleep(0.3)
        
        if not ok and hint and hint.startswith("search:"):
            q = hint[len("search:"):]
            print(f"[SEARCH] {label} q='{q}'", flush=True)
            items, status = search_github(q)
            if status == 200 and items:
                print(f"  search results: {[(n, s) for n, s, _ in items]}", flush=True)
                # Try first match only
                owner_repo, stars_search, html_url = items[0]
                owner, _, repo = owner_repo.partition("/")
                status2, content2, stars2 = fetch_readme(owner, repo)
                if status2 == 200 and content2 and len(content2) > 200:
                    path = save_readme(owner, repo, content2)
                    results[label] = {"url": f"{owner}/{repo}", "path": path, "len": len(content2), "stars": stars2 or stars_search, "ok": True, "via_search": True}
                    summary.append((label, f"{owner}/{repo}", stars2 or stars_search, path))
                    print(f"  OK via search ({len(content2)} bytes) stars={stars2 or stars_search}", flush=True)
                    ok = True
                else:
                    print(f"  search hit but fetch failed: {status2}", flush=True)
            else:
                print(f"  search failed status={status}", flush=True)
        
        if not ok:
            results[label] = {"ok": False, "hint": hint}
            summary.append((label, None, None, None))
        
        update_results(results)
        time.sleep(1.0)  # be nice
    
    # print final
    print("\n=== SUMMARY ===")
    for label, url, stars, path in summary:
        if url:
            print(f"OK  {label} -> {url} stars={stars}")
        else:
            print(f"FAIL {label}")
    
    return results


if __name__ == "__main__":
    main()
