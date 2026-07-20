#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Parse Bocha AI deep search results + synthesize ASI deep research."""
import sys
import json
from pathlib import Path

if hasattr(sys.stdout, 'reconfigure'):
    sys.stdout.reconfigure(encoding='utf-8')

p = Path(r".openclaw\workspace\promethean\research-ai-deep-search-2026-07-20.json")
data = json.loads(p.read_text(encoding="utf-8-sig"))

synthesis = []
for i, r in enumerate(data):
    q = r.get("query", "")
    raw = r.get("raw", "")
    err = r.get("err", "")

    if raw:
        try:
            j = json.loads(raw)
            msgs = j.get("data", {}).get("messages", [])
            # ai-search 通常 messages[2] 是 AI answer
            ai_answer = None
            for m in msgs:
                if m.get("role") == "assistant" and m.get("type") == "answer":
                    ai_answer = m.get("content", "")
                    break
                if m.get("role") == "assistant" and m.get("type") == "content":
                    ai_answer = m.get("content", "")
                    break
            if not ai_answer and len(msgs) >= 2:
                # fallback to second assistant message
                for m in msgs:
                    if m.get("role") == "assistant":
                        ai_answer = m.get("content", "")
                        break

            synthesis.append({
                "query": q,
                "ai_answer": ai_answer[:3000] if ai_answer else "(no answer extracted)",
                "n_sources": len(msgs),
            })
        except Exception as e:
            synthesis.append({"query": q, "ai_answer": f"(parse err: {e})", "n_sources": 0})
    elif err:
        synthesis.append({"query": q, "ai_answer": f"ERR: {err}", "n_sources": 0})

# Save parsed
out = Path(r".openclaw\workspace\promethean\research-ai-deep-search-parsed.json")
out.write_text(json.dumps(synthesis, indent=2, ensure_ascii=False), encoding="utf-8")

# Display
for s in synthesis:
    print(f"\n=== Q: {s['query'][:100]} ===")
    print(f"    (sources: {s['n_sources']})")
    ans = s['ai_answer']
    if ans and ans != "(no answer extracted)" and not ans.startswith("ERR"):
        print(f"    A: {ans[:1200]}")
    else:
        print(f"    {ans[:200]}")
