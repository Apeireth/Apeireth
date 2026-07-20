#!/usr/bin/env python3
"""Fetch READMEs for newly-found repos from GitHub search."""
import urllib.request
import urllib.error
from pathlib import Path

OUT = Path(r".openclaw\workspace\promethean\research-master-list-2026")

candidates = [
    "multica-ai/andrej-karpathy-skills",
    "jo-inc/camofox-browser",
    "tinyhumansai/openhuman",
    "wechat-article/wechat-article-exporter",
    "drl990114/MarkFlowy",
    "baidu/Unlimited-OCR",
]

for repo in candidates:
    safe_name = repo.replace("/", "_")
    out_path = OUT / f"{safe_name}_README.md"
    for branch in ("main", "master"):
        url = f"https://raw.githubusercontent.com/{repo}/{branch}/README.md"
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "ApeirethResearch/2.0"})
            with urllib.request.urlopen(req, timeout=20) as resp:
                data = resp.read().decode("utf-8", errors="replace")
            if data and len(data) > 100:
                out_path.write_text(data, encoding="utf-8")
                print(f"  OK {repo}@{branch}: {len(data)} chars -> {out_path.name}")
                break
        except urllib.error.HTTPError as e:
            if e.code == 404:
                continue
            print(f"  ERR {repo}@{branch}: {e}")
        except Exception as e:
            print(f"  ERR {repo}@{branch}: {e}")
    else:
        print(f"  NOT FOUND {repo}")
