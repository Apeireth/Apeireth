"""Apeireth GitHub Research — AnySearch-as-GitHub-client

主人 14:23 明确: anysearch 装来是给 我们搜 GitHub 用的.
具体用途:
- 搜 trending 项目 (anysearch-ai 推荐 vertical = code.snippet)
- 抓 GitHub raw README/代码 (extract)
- 跟 AHE/DGM/OpenSage 等对比 (code.snippet)
- 借鉴 watchlist 80 项目自动调研

我们之前面对的 3 个问题:
1. web_search 工具 timeout
2. web_fetch GitHub raw timeout
3. DNS 阻塞 GitHub.com

AnySearch extract 全部绕过:
- POST api.anysearch.com/mcp JSON-RPC
- 它在 cloud 上走自家网络
- 我们直接拿原文
"""

from __future__ import annotations
from typing import Optional
from urllib.parse import urlparse

from .research import AnySearch


class GitHubResearch:
    """High-level: 用 AnySearch 搜 GitHub + 抓 raw 文件.

    模式:
    - search_repo(name)  -> general search, 找 README URL
    - fetch_readme(repo)  -> extract GitHub raw README.md
    - fetch_file(repo, path) -> extract GitHub raw 文件
    - search_trending(topic) -> vertical code.snippet 搜 trending
    """

    def __init__(self, anysearch: Optional[AnySearch] = None):
        self.s = anysearch or AnySearch()

    def _raw_url(self, repo: str, branch: str, path: str) -> str:
        """构造 GitHub raw URL (extract 路径).

        repo 格式: 'anthropics/skills'
        path 格式: 'skills/docx/SKILL.md'
        """
        return f"https://raw.githubusercontent.com/{repo}/{branch}/{path}"

    def fetch_readme(self, repo: str, branch: str = "main") -> Optional[str]:
        """抓 README.md / readme.md / README.markdown

        返回 markdown 文本. None = 失败.
        """
        for path in ["README.md", "readme.md", "README.markdown", "README.rst"]:
            url = self._raw_url(repo, branch, path)
            r = self.s.extract(url)
            if r["ok"]:
                d = r["data"]
                # extract 返回的 content 是 list of {type, text}
                if isinstance(d, dict) and "content" in d:
                    for c in d["content"]:
                        if c.get("type") == "text":
                            return c["text"]
                return str(d)
        return None

    def fetch_file(self, repo: str, path: str, branch: str = "main") -> Optional[str]:
        url = self._raw_url(repo, branch, path)
        r = self.s.extract(url)
        if r["ok"]:
            d = r["data"]
            if isinstance(d, dict) and "content" in d:
                for c in d["content"]:
                    if c.get("type") == "text":
                        return c["text"]
        return None

    def fetch_skill_md(self, repo: str, branch: str = "main") -> Optional[str]:
        """anthropics/skills 格式: skills/<name>/SKILL.md

        先列目录, 再单抓.
        但我们简化: 假设 skills/INDEX/SKILL.md 或 单一 root SKILL.md
        """
        # try root SKILL.md first
        root = self.fetch_file(repo, "SKILL.md", branch)
        if root:
            return root
        # try SKILL.md at skills subfolder (some repos)
        return self.fetch_file(repo, "skill/SKILL.md", branch) or \
               self.fetch_file(repo, ".claude/skills/SKILL.md", branch)

    def search_trending_repo(self, topic: str, limit: int = 8) -> list[str]:
        """搜 trending 仓库名 (返回 GitHub URL list).

        AnySearch general search: 返回的结果含 URL, 我们筛 raw.githubusercontent.com 或 github.com/<org>/<repo>
        """
        r = self.s.search(f"github trending {topic} 2026", max_results=limit)
        repos: list[str] = []
        if r["ok"]:
            data = r["data"]
            text = self._extract_text(data)
            # 简单 regex 找 github.com/<owner>/<repo>
            import re
            urls = re.findall(r"github\.com/([\w\-]+/[\w\-\.]+)", text)
            seen = set()
            for u in urls:
                if "/" in u:
                    owner, _, name = u.partition("/")
                    if owner not in {"docs", "support", "learn", "enterprise", "pages"}:
                        key = u
                        if key not in seen:
                            repos.append(key)
                            seen.add(key)
        return repos

    @staticmethod
    def _extract_text(data) -> str:
        """extract text from AnySearch response (search / extract / batch_search)."""
        if isinstance(data, dict):
            if "content" in data:
                for c in data["content"]:
                    if isinstance(c, dict) and c.get("type") == "text":
                        return c["text"]
            return str(data)
        return str(data)


# ── Doctest ────────────────────────────────────────────────
if __name__ == "__main__":
    g = GitHubResearch()
    print(f"anysearch: {bool(g.s.api_key)}")
    print()
    print("--- fetch anysearch-skill README ---")
    md = g.fetch_readme("anysearch-ai/anysearch-skill")
    if md:
        print(f"✅ {len(md)} chars")
        # first few lines
        print(md.split("\n")[:8])
    else:
        print("❌ failed")

    print()
    print("--- search trending AI ---")
    repos = g.search_trending_repo("AI agent framework")
    print(f"Found {len(repos)} repos:")
    for r_ in repos[:6]:
        print(f"  {r_}")
