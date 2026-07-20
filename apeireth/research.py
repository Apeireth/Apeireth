"""Apeireth Search — 集成层

主人 13:51 严肃要求解决联网搜索问题, 13:54 让我看 AnySearch 能不能用。
AnySearch = ✅ 完美, GitHub 4555 stars, Apache-2.0, 自带 SKILL.md, 
2.3K→4.5K stars/trend 速度, 17 个 vertical domain routing.

我们 Apeireth 把 AnySearch 作为 L2 Interaction Layer 的联网眼睛。
不直接调用 CLI (慢), 用纯 JSON-RPC over HTTPS (Own RPC format).

ly search()
  batch_search()
  get_sub_domains()
  extract()
  doc() — 这个用于把 SKILL.md 转成更适合 agent 的格式

优先 vertical (P0) search — AnySearch 默认 Path 2 = vertical.
"""
import os
import json
import urllib.request
import urllib.error
import urllib.parse
from pathlib import Path
from typing import Optional


API_BASE = os.environ.get("ANYSEARCH_API_BASE", "https://api.anysearch.com/mcp")
KEY_FILE = Path(__file__).parent / "skills" / "anysearch" / ".env"


class AnySearch:
    """AnySearch JSON-RPC 2.0 client for Apeireth.

    No SDK dependency. Just urllib.
    """

    def __init__(self, api_key: Optional[str] = None):
        self.api_key = api_key or _load_key()
        self._request_id = 0

    def _call(self, method: str, params: dict) -> dict:
        self._request_id += 1
        body = json.dumps({
            "jsonrpc": "2.0",
            "id": self._request_id,
            "method": method,
            "params": params,
        }).encode()

        headers = {"Content-Type": "application/json"}
        if self.api_key:
            headers["Authorization"] = f"Bearer {self.api_key}"

        req = urllib.request.Request(API_BASE, data=body, headers=headers, method="POST")
        try:
            with urllib.request.urlopen(req, timeout=30) as resp:
                data = json.loads(resp.read().decode())
                if "error" in data:
                    return {"ok": False, "error": data["error"]}
                return {"ok": True, "data": data.get("result")}
        except urllib.error.HTTPError as e:
            return {"ok": False, "error": f"HTTP {e.code}", "body": e.read()[:500].decode(errors="ignore")}
        except Exception as e:
            return {"ok": False, "error": str(e)}

    def search(self, query: str, domain: Optional[str] = None,
               sub_domain: Optional[str] = None,
               sdp: Optional[dict] = None,
               max_results: int = 10) -> dict:
        params = {"query": query, "max_results": max_results}
        if domain:
            params["domain"] = domain
        if sub_domain:
            params["sub_domain"] = sub_domain
        if sdp:
            params["sub_domain_params"] = sdp
        return self._call("tools/call", {"name": "search", "arguments": params})

    def batch_search(self, queries: list[dict]) -> dict:
        """queries = [{query, domain?, sub_domain?, sdp?}, ...] 1-5 个."""
        return self._call("tools/call", {"name": "batch_search", "arguments": {"queries": queries}})

    def get_sub_domains(self, domains: list[str]) -> dict:
        return self._call("tools/call", {"name": "get_sub_domains", "arguments": {"domains": domains}})

    def extract(self, url: str) -> dict:
        return self._call("tools/call", {"name": "extract", "arguments": {"url": url}})


def _load_key() -> Optional[str]:
    if not KEY_FILE.exists():
        return None
    for line in KEY_FILE.read_text(encoding="utf-8", errors="ignore").splitlines():
        line = line.strip()
        if line and not line.startswith("#") and "=" in line:
            key, _, val = line.partition("=")
            if key.strip() == "ANYSEARCH_API_KEY" and val.strip():
                return val.strip()
    return None


# ── test_doctest ──────────────────────────────────────────────
if __name__ == "__main__":
    s = AnySearch()
    if s.api_key:
        print(f"✅ API key loaded ({len(s.api_key)} chars)")
    else:
        print("⚠️ No API key — anonymous mode (lower rate limits)")

    # 跑 search 测试
    r = s.search("open source agent framework")
    print(f"\n--- Test search ---")
    if r["ok"]:
        d = r["data"]
        print(f"✅ search ok: {len(str(d))} chars response")
    else:
        print(f"❌ search failed: {r['error']}")
