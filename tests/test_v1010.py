"""V1010 真生产 tests (主 23:44)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest
from apeireth.v1010_research_report import (
    V1010_VERSION, ResearchReportSection, V1010ResearchReport,
)


class TestV1010:
    def test_init(self):
        r = V1010ResearchReport()
        assert r.n_sections() == 0

    def test_add_section(self):
        r = V1010ResearchReport()
        sid = r.add_section("Test", "Content", level=2)
        assert r.n_sections() == 1

    def test_render_markdown(self):
        r = V1010ResearchReport()
        r.add_section("H1", "Content", level=1)
        md = r.render_markdown()
        assert "# H1" in md

    def test_findings_in_md(self):
        r = V1010ResearchReport()
        r.add_section("T", "C", findings=["finding 1", "finding 2"])
        md = r.render_markdown()
        assert "finding 1" in md
        assert "finding 2" in md

    def test_references_in_md(self):
        r = V1010ResearchReport()
        r.add_section("T", "C", references=["ref1"])
        md = r.render_markdown()
        assert "ref1" in md

    def test_build_full(self):
        r = V1010ResearchReport()
        r.build_full_research_report()
        assert r.n_sections() >= 7

    def test_write_to_file(self):
        r = V1010ResearchReport()
        r.build_full_research_report()
        import tempfile, os
        with tempfile.NamedTemporaryFile(mode='w', suffix='.md', delete=False) as f:
            tmp = f.name
        result = r.write_to_file(tmp)
        assert result is True
        assert os.path.exists(tmp)
        content = open(tmp, encoding='utf-8').read()
        assert "Apeireth" in content
        os.remove(tmp)

    def test_stats(self):
        r = V1010ResearchReport()
        r.build_full_research_report()
        s = r.stats()
        assert s["n_sections"] >= 7
        assert s["version"] == V1010_VERSION

    def test_v22_33_asi_integration(self):
        """V1010 真测主 22:33 ASI 北极星."""
        r = V1010ResearchReport()
        r.build_full_research_report()
        md = r.render_markdown()
        assert "ASI" in md
        assert "22:33" in md

    def test_v19_33_integration(self):
        """V1010 真测主 19:33 走在前人经验上 + 聚合全人类智慧."""
        r = V1010ResearchReport()
        r.build_full_research_report()
        md = r.render_markdown()
        assert "19:33" in md
        assert "全人类智慧" in md

    def test_v19_17_integration(self):
        """V1010 真测主 19:17 AnySearch 真调研."""
        r = V1010ResearchReport()
        r.build_full_research_report()
        md = r.render_markdown()
        assert "19:17" in md

    def test_v19_28_integration(self):
        """V1010 真测主 19:28 博查 AI Search 真调研."""
        r = V1010ResearchReport()
        r.build_full_research_report()
        md = r.render_markdown()
        assert "19:28" in md

    def test_v18_44_integration(self):
        """V1010 真测主 18:44 VCP 真借鉴."""
        r = V1010ResearchReport()
        r.build_full_research_report()
        md = r.render_markdown()
        assert "18:44" in md
        assert "VCP" in md

    def test_v17_43_integration(self):
        """V1010 真测主 17:43 实事求是."""
        r = V1010ResearchReport()
        r.build_full_research_report()
        md = r.render_markdown()
        assert "17:43" in md
        assert "实事求是" in md

    def test_v17_33_integration(self):
        """V1010 真测主 17:33 放手干到底."""
        r = V1010ResearchReport()
        r.build_full_research_report()
        md = r.render_markdown()
        assert "17:33" in md

    def test_v23_44_integration(self):
        """V1010 真测主 23:44 干到底."""
        r = V1010ResearchReport()
        r.build_full_research_report()
        md = r.render_markdown()
        assert "23:44" in md

    def test_v1001_v1009_integration(self):
        """V1010 真测 V1001-V1009 真整合 (主 22:33 + 主 17:33 + 主 19:33)."""
        r = V1010ResearchReport()
        r.build_full_research_report()
        md = r.render_markdown()
        for v in ["V1001", "V1002", "V1003", "V1004", "V1005",
                  "V1006", "V1007", "V1008", "V1009"]:
            assert v in md, f"missing: {v}"

    def test_complete_integration(self):
        """V1010 真测完整调研报告 (主 22:33 + 主 19:33 + 主 17:43 + 主 17:33 + 主 23:44)."""
        r = V1010ResearchReport()
        r.build_full_research_report()
        md = r.render_markdown()
        keywords = ["Apeireth ASI", "V1001", "V1002", "V1003", "V1004",
                    "V1005", "V1006", "V1007", "V1008", "V1009",
                    "22:33", "19:33", "17:43", "17:33", "23:44",
                    "VCP", "OpenCog", "AERA", "NARS", "DGM", "Popper"]
        for k in keywords:
            assert k in md, f"missing: {k}"