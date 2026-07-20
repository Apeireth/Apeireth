#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Parse Bocha AI responses — extract sources AND answer."""
import json
import re
from pathlib import Path

data = json.loads(Path('research-ai-deep-search-2026-07-20.json').read_text(encoding='utf-8-sig'))

synthesis = []
for i, r in enumerate(data):
    q = r.get("query", "")
    raw = r.get("raw", "")
    err = r.get("err", "")

    if raw:
        try:
            j = json.loads(raw)
            msgs = j.get("messages", [])

            # Extract sources from message[0] (type=source)
            sources = []
            if msgs and msgs[0].get("type") == "source":
                content = msgs[0].get("content", "")
                if content:
                    try:
                        s = json.loads(content)
                        for v in s.get("value", [])[:5]:
                            sources.append({
                                "name": v.get("name", ""),
                                "url": v.get("url", ""),
                                "summary": v.get("summary", "")[:300],
                            })
                    except Exception:
                        pass

            # Extract answer from message[2+] (type=answer)
            ai_answer = ""
            for m in msgs[1:]:
                if m.get("role") == "assistant" and m.get("type") == "answer":
                    ai_answer = m.get("content", "")
                    break
            if not ai_answer:
                # fallback — find any assistant with non-source content
                for m in msgs[1:]:
                    if m.get("role") == "assistant" and m.get("type") != "source":
                        ai_answer = m.get("content", "")
                        break

            # Strip <text> tags if any
            ai_answer = re.sub(r'</?text>', '', ai_answer).strip()
            ai_answer = re.sub(r'\\u003c/?text\\u003e', '', ai_answer)

            synthesis.append({
                "query": q,
                "answer": ai_answer[:3000] if ai_answer else "(no answer in messages)",
                "n_sources": len(sources),
                "sources": sources,
            })
        except Exception as e:
            synthesis.append({"query": q, "answer": f"(parse err: {e})", "n_sources": 0, "sources": []})
    elif err:
        synthesis.append({"query": q, "answer": f"ERR: {err}", "n_sources": 0, "sources": []})

Path('research-ai-parsed.json').write_text(json.dumps(synthesis, indent=2, ensure_ascii=False), encoding='utf-8')

# Display
for s in synthesis:
    print(f"\n=== Q: {s['query'][:100]} ===")
    print(f"    sources: {s['n_sources']}")
    if s['sources']:
        for src in s['sources'][:3]:
            print(f"    - {src['name'][:80]}")
            print(f"      {src['url']}")
    print(f"    A: {s['answer'][:800]}")
