"""Apeireth pytest 全局 fixtures (R8-DevOps 修复 R7 §技术债 #5).

R7 handoff §已知技术债 #5 = ``test_v1058::test_find_api_key_empty`` env-dependent:
只清理 LLM_API_KEY / OPENAI_API_KEY / NEWAPI_API_KEY 三个 env, 其他 LLM key
(``MINIMAX_API_KEY`` / ``*_TOKEN`` / ``ANTHROPIC_API_KEY`` 等) 在测试间会泄漏.
本 fixture 在每个 test 前后快照/恢复所有 ``*API*KEY*`` / ``*_TOKEN`` env,
并把 ``*_API_KEY`` 一律清空 (生产路径上由 CI 显式注入).

主 17:43 实事求是 + 主 17:58 不假装: fixture 真恢复 env, 不让测试互相污染.
"""
from __future__ import annotations

import os
import sys
from typing import Iterator, Set

import pytest


_API_KEY_SUFFIXES: tuple[str, ...] = ("_API_KEY", "API_KEY", "_TOKEN", "_SECRET")
"""env 变量名匹配模式: 任何含这些子串的变量都会被快照/恢复/默认清空."""


def _snapshot() -> dict[str, str | None]:
    """快照所有匹配 *API*KEY* / *_TOKEN 的 env 变量."""
    return {k: os.environ.get(k) for k in list(os.environ) if any(s in k for s in _API_KEY_SUFFIXES)}


def _restore(snapshot: dict[str, str | None]) -> None:
    """恢复快照, 不在快照里的 env-key 一律删除 (防泄漏)."""
    current: Set[str] = {k for k in os.environ if any(s in k for s in _API_KEY_SUFFIXES)}
    for k in current - set(snapshot):
        os.environ.pop(k, None)
    for k, v in snapshot.items():
        if v is None:
            os.environ.pop(k, None)
        else:
            os.environ[k] = v


@pytest.fixture(autouse=True)
def _isolate_api_key_env() -> Iterator[None]:
    """每个 test 前后隔离 *API*KEY* / *_TOKEN env.

    ponytail: ceiling = ``*API*KEY*`` / ``*_TOKEN`` 通配; 升级路径 = 显式
    注入 ``LLM_API_KEY`` 的 env-marker, 隔离策略可按需收缩.
    """
    saved = _snapshot()
    for k in list(saved):
        os.environ[k] = ""  # 测试期间清空, 强制走 fallback
    try:
        yield
    finally:
        _restore(saved)


def pytest_configure(config: pytest.Config) -> None:
    """在 pytest header 标记本 conftest 已激活, 便于不假装守门."""
    print(f"[conftest] api-key env isolation active (python={sys.version.split()[0]})", file=sys.stderr)
