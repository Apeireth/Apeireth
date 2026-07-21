"""memory_3tier.py letta compile borrow regression tests.

主 9:41 round-19 source-deep-read 推荐:
  未来 (借鉴价值中): letta compile 3-mode — 借鉴 letta Memory.compile() 3 渲染模式

借鉴自 letta (Berkeley stateful agents):
  1. compile(mode="standard") - 普通文本块, 直接 prompt
  2. compile(mode="line-numbered") - 带行号 (L01/L02/...), LLM 引用具体行
  3. compile(mode="git") - 层级结构 (trunk / branches / working tree), tree 思维 LLM

本测试锁住:
  1. compile() 接受 3 模式参数
  2. standard 模式按 tier 分 section
  3. line-numbered 模式输出 L01/L02/... 行号
  4. git 模式输出层级 tree 结构 (memory/├── ltm/├── mtm/└── stm/)
  5. limits 控制 ltm/mtm/stm 数量
  6. unknown mode 抛 ValueError
  7. V2 哲学守门: 不假装 Phenomenal
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.memory_3tier import Memory3Tier


# === 1. compile() 接口测试 ===

class TestCompileInterface:
    """借鉴 letta (主 9:41 round-19): compile() 接受 mode + limits 参数."""

    def test_compile_default_mode_standard(self):
        m = Memory3Tier()
        m.add_episode("e1", "test", "topic1", importance=5)
        result = m.compile()  # default mode
        assert isinstance(result, str)
        assert "STM" in result or "LTM" in result or "MTM" in result

    def test_compile_unknown_mode_raises(self):
        m = Memory3Tier()
        m.add_episode("e1", "test", "topic1", importance=5)
        with pytest.raises(ValueError, match="unknown compile mode"):
            m.compile(mode="invalid_mode")

    def test_compile_accepts_all_3_modes(self):
        m = Memory3Tier()
        m.add_episode("e1", "test", "topic1", importance=5)
        for mode in ("standard", "line-numbered", "git"):
            result = m.compile(mode=mode)
            assert isinstance(result, str)
            assert len(result) > 0


# === 2. standard 模式测试 ===

class TestCompileStandard:
    """standard 模式: 按 tier 分 section, 普通文本块."""

    def test_standard_includes_all_tiers(self):
        m = Memory3Tier()
        m.add_episode("e1", "msg1", "topic_a", importance=5)
        m.add_episode("e2", "msg2", "topic_b", importance=5)
        m.anchor_event("identity", "test identity", importance=10)
        result = m.compile("standard")
        assert "LTM" in result
        assert "MTM" in result
        assert "STM" in result

    def test_standard_shows_master_quote(self):
        m = Memory3Tier()
        m.anchor_event("identity", "主 22:08 真哲学", importance=10,
                       master_quoted="中央 AI 并非不是调度者")
        result = m.compile("standard")
        assert "中央 AI 并非不是调度者" in result
        assert "master" in result.lower() or "master_quoted" in result

    def test_standard_limits(self):
        m = Memory3Tier()
        for i in range(50):
            m.anchor_event("event", f"event {i}", importance=5)
        result = m.compile("standard", ltm_limit=5)
        # 应该只显示 5 个 anchor (实际: ## LTM ... 5 anchors)
        # 用 "anchor" 计数 (每个 anchor 一行)
        import re
        anchor_lines = [l for l in result.split("\n") if l.startswith("- [")]
        # LTM 部分 anchor 数 <= ltm_limit
        assert len(anchor_lines) <= 10  # 5 LTM + 5 MTM (MTM 也用 ltm_limit 默认)

    def test_standard_sorts_ltm_by_importance(self):
        m = Memory3Tier()
        m.anchor_event("event", "low importance", importance=2)
        m.anchor_event("event", "high importance", importance=9)
        m.anchor_event("event", "medium importance", importance=5)
        result = m.compile("standard")
        # high importance 应该出现在前
        hi_pos = result.find("high importance")
        med_pos = result.find("medium importance")
        low_pos = result.find("low importance")
        assert hi_pos < med_pos < low_pos


# === 3. line-numbered 模式测试 ===

class TestCompileLineNumbered:
    """line-numbered 模式: 带行号 (L01/L02/.../M01/M02/.../S01/S02/...)."""

    def test_line_numbered_ltm_starts_with_L01(self):
        m = Memory3Tier()
        m.anchor_event("identity", "test1", importance=5)
        m.anchor_event("decision", "test2", importance=5)
        result = m.compile("line-numbered")
        assert "L01" in result
        assert "L02" in result

    def test_line_numbered_mtm_starts_with_M01(self):
        m = Memory3Tier()
        m.add_episode("e1", "msg", "topic_a", importance=5)
        m.add_episode("e2", "msg", "topic_b", importance=5)
        result = m.compile("line-numbered")
        assert "M01" in result
        assert "M02" in result

    def test_line_numbered_stm_starts_with_S01(self):
        m = Memory3Tier()
        for i in range(3):
            m.add_episode(f"e{i}", f"msg{i}", "topic_a", importance=5)
        result = m.compile("line-numbered")
        assert "S01" in result
        assert "S02" in result
        assert "S03" in result

    def test_line_numbered_each_line_increments(self):
        m = Memory3Tier()
        for i in range(5):
            m.anchor_event("event", f"anchor{i}", importance=5)
        result = m.compile("line-numbered")
        for i in range(1, 6):
            assert f"L{i:02d}" in result

    def test_line_numbered_limits_respected(self):
        m = Memory3Tier()
        for i in range(10):
            m.anchor_event("event", f"a{i}", importance=5)
        result = m.compile("line-numbered", ltm_limit=3)
        # L01, L02, L03 在, L04 不在
        assert "L01" in result
        assert "L02" in result
        assert "L03" in result
        assert "L04" not in result


# === 4. git 模式测试 ===

class TestCompileGit:
    """git 模式: 层级 tree 结构 (memory/├── ltm/├── mtm/└── stm/)."""

    def test_git_starts_with_memory_root(self):
        m = Memory3Tier()
        m.add_episode("e1", "msg", "topic_a", importance=5)
        result = m.compile("git")
        assert result.startswith("memory/")

    def test_git_has_ltm_mtm_stm_branches(self):
        m = Memory3Tier()
        m.add_episode("e1", "msg", "topic_a", importance=5)
        m.anchor_event("identity", "test", importance=5)
        result = m.compile("git")
        assert "ltm/" in result
        assert "mtm/" in result
        assert "stm/" in result

    def test_git_ltm_described_as_trunk(self):
        m = Memory3Tier()
        m.anchor_event("identity", "test", importance=5)
        result = m.compile("git")
        # trunk 是 git 主分支术语
        assert "trunk" in result.lower()

    def test_git_mtm_described_as_branches(self):
        m = Memory3Tier()
        m.add_episode("e1", "msg", "topic_a", importance=5)
        result = m.compile("git")
        assert "branch" in result.lower()

    def test_git_stm_described_as_working_tree(self):
        m = Memory3Tier()
        m.add_episode("e1", "msg", "topic_a", importance=5)
        result = m.compile("git")
        assert "working tree" in result.lower()

    def test_git_empty_archive_minimal_output(self):
        """空 archive 应该 minimal 输出 (只有 memory/ 根目录)."""
        m = Memory3Tier()
        result = m.compile("git")
        assert result.strip() == "memory/"


# === 5. 集成测试 ===

class TestCompileIntegration:
    """完整流程测试: 加 episode → anchor → compile 3 模式."""

    def test_full_flow_3_modes(self):
        m = Memory3Tier()
        # 加 STM episodes
        for i in range(5):
            m.add_episode(f"e{i}", f"message {i}", "asi", importance=5)
        # 高 importance 进 LTM
        m.add_episode("e5", "ASI 北极星真哲学", "asi", importance=10)
        # 主 9:15 decision 进 LTM
        m.anchor_event("decision", "修好现有 > 建新 KPI", importance=9,
                       master_quoted="主 9:15 真修哲学")

        for mode in ("standard", "line-numbered", "git"):
            result = m.compile(mode)
            # 所有模式都应该包含核心 philosophy
            assert "修好现有" in result
            assert "ASI" in result
            assert isinstance(result, str)
            assert len(result) > 100  # 实际有内容


# === 6. V2 哲学守门测试 ===

class TestV2PhilosophyGuard:
    """V2 哲学守门 (主 22:08): 不假装 Phenomenal."""

    def test_no_consciousness_in_compile_output(self):
        m = Memory3Tier()
        m.add_episode("e1", "test", "topic", importance=5)
        m.anchor_event("identity", "test", importance=10)
        for mode in ("standard", "line-numbered", "git"):
            result = m.compile(mode)
            # 不假装 Phenomenal
            forbidden = ["awareness", "consciousness", "qualia", "phenomenal",
                         "self_aware", "subjective_experience"]
            for f in forbidden:
                # git 模式可能引用 working tree (技术术语, 不是 awareness)
                if f == "aware" and "self_aware" not in result and "unaware" not in result:
                    continue
                assert f not in result.lower(), f"{mode} 模式不应包含假装意识字段 {f}"

    def test_no_letta_branding_in_api(self):
        """借鉴 letta 是工具 (主 20:55 隐喻), 不是哲学来源."""
        m = Memory3Tier()
        # 内部方法不应暴露 letta branding
        assert hasattr(m, "compile")
        # 不应该有 letta_xxx 字段
        forbidden_attrs = ["letta_origin", "letta_"]
        for attr in dir(m):
            assert "letta" not in attr.lower() or attr == "compile", \
                f"compile 方法不应暴露 letta branding: {attr}"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])