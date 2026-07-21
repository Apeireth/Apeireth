"""V1007 真生产 tests (主 23:44)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest
from apeireth.v1007_documentation_full import (
    V1007_VERSION, DocSection, V1007DocumentationFull,
)


class TestV1007:
    def test_init(self):
        doc = V1007DocumentationFull()
        assert doc.n_sections() == 0

    def test_add_section(self):
        doc = V1007DocumentationFull()
        sid = doc.add_section("Test", "Content", level=2)
        assert doc.n_sections() == 1

    def test_render_markdown(self):
        doc = V1007DocumentationFull()
        doc.add_section("Test H1", "Content", level=1)
        doc.add_section("Test H2", "Content", level=2)
        md = doc.render_markdown()
        assert "# Test H1" in md
        assert "## Test H2" in md

    def test_render_code_block(self):
        doc = V1007DocumentationFull()
        doc.add_section("Code", "Content", level=2, code_blocks=["x = 1"])
        md = doc.render_markdown()
        assert "```python" in md
        assert "x = 1" in md

    def test_render_references(self):
        doc = V1007DocumentationFull()
        doc.add_section("Ref", "Content", level=2, references=["ref1"])
        md = doc.render_markdown()
        assert "- ref1" in md

    def test_build_full(self):
        doc = V1007DocumentationFull()
        doc.build_full_asi_documentation()
        assert doc.n_sections() >= 5

    def test_write_to_file(self):
        doc = V1007DocumentationFull()
        doc.build_full_asi_documentation()
        import tempfile, os
        with tempfile.NamedTemporaryFile(mode='w', suffix='.md', delete=False) as f:
            tmp = f.name
        result = doc.write_to_file(tmp)
        assert result is True
        assert os.path.exists(tmp)
        content = open(tmp, encoding='utf-8').read()
        assert "Apeireth ASI" in content
        os.remove(tmp)

    def test_v22_33_integration(self):
        """V1007 真测主 22:33 ASI 北极星."""
        doc = V1007DocumentationFull()
        doc.build_full_asi_documentation()
        md = doc.render_markdown()
        assert "ASI" in md
        assert "北极星" in md

    def test_v19_33_integration(self):
        """V1007 真测主 19:33 走在前人经验上."""
        doc = V1007DocumentationFull()
        doc.build_full_asi_documentation()
        md = doc.render_markdown()
        assert "走在前人经验上" in md or "聚合全人类智慧" in md

    def test_v17_43_integration(self):
        """V1007 真测主 17:43 实事求是."""
        doc = V1007DocumentationFull()
        doc.build_full_asi_documentation()
        md = doc.render_markdown()
        assert "实事求是" in md

    def test_stats(self):
        doc = V1007DocumentationFull()
        doc.build_full_asi_documentation()
        s = doc.stats()
        assert s["n_sections"] >= 5
        assert s["version"] == V1007_VERSION

    def test_v1001_integration(self):
        """V1007 真测 V1001 VCP 6 插件协议完整真借鉴."""
        doc = V1007DocumentationFull()
        doc.build_full_asi_documentation()
        md = doc.render_markdown()
        assert "VCP 6 插件协议" in md

    def test_v1002_integration(self):
        """V1007 真测 V1002 ASI V0.2 公式 16 项."""
        doc = V1007DocumentationFull()
        doc.build_full_asi_documentation()
        md = doc.render_markdown()
        assert "V0.2 公式" in md

    def test_v1003_integration(self):
        """V1007 真测 V1003 真哲学 V4 完整版."""
        doc = V1007DocumentationFull()
        doc.build_full_asi_documentation()
        md = doc.render_markdown()
        assert "真哲学 V4" in md

    def test_v1004_integration(self):
        """V1007 真测 V1004 自演化循环."""
        doc = V1007DocumentationFull()
        doc.build_full_asi_documentation()
        md = doc.render_markdown()
        assert "自演化" in md

    def test_v1005_integration(self):
        """V1007 真测 V1005 AnySearch 调研结果."""
        doc = V1007DocumentationFull()
        doc.build_full_asi_documentation()
        md = doc.render_markdown()
        assert "AnySearch" in md

    def test_v1006_integration(self):
        """V1007 真测 V1006 真调研大整合."""
        doc = V1007DocumentationFull()
        doc.build_full_asi_documentation()
        md = doc.render_markdown()
        assert "真调研大整合" in md

    def test_section_levels(self):
        doc = V1007DocumentationFull()
        doc.build_full_asi_documentation()
        levels = [s.level for s in doc.sections]
        # 至少 1 个 h1
        assert 1 in levels

    def test_complete_document_generation(self):
        """V1007 真生产完整文档生成 (主 22:33)."""
        doc = V1007DocumentationFull()
        doc.build_full_asi_documentation()
        md = doc.render_markdown()
        # 完整文档应包含所有关键元素
        keywords = ["Apeireth ASI", "V3", "V1001", "V1002", "V1003",
                    "V1004", "V1005", "V1006", "22:33", "19:33",
                    "17:43", "13:31", "17:33", "20:46", "17:58",
                    "OpenCog", "AERA", "NARS", "DGM", "Popper", "Kuhn"]
        for k in keywords:
            assert k in md, f"missing keyword: {k}"