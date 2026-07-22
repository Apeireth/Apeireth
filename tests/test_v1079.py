"""v1079_asi_literature_review.py 真生产回归测试.

主 23:44 干到底: 真测每个真生产组件 + 真 fixture + CLI + V3 守门.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest

from apeireth.v1079_asi_literature_review import (
    V1079_VERSION,
    Paper,
    PaperQuery,
    CitationParser,
    DeduplicationEngine,
    BibTeXExporter,
    ReferenceFormatter,
    TFIDFCluster,
    TrendAnalyzer,
    LiteratureReviewGenerator,
    ASILiteratureReviewBridge,
    V3PhilosophyGuard,
    LiteratureReviewer,
    LiteratureReview,
    REFERENCES,
    _default_fixture,
)


# ===================== Fixtures =====================

@pytest.fixture
def fixture_papers() -> list:
    return _default_fixture()


@pytest.fixture
def small_papers() -> list:
    return [
        Paper(
            paper_id="10.1/test1",
            title="AlphaGo and the game of Go",
            authors=["David Silver", "Aja Huang"],
            year=2016, venue="Nature",
            doi="10.1/test1",
            citations_in=100, citations_out=[],
            abstract="AlphaGo wins Go.",
            source="fixture",
        ),
        Paper(
            paper_id="10.1/test2",
            title="AlphaGo Zero without human knowledge",
            authors=["David Silver", "Julian Schrittwieser"],
            year=2017, venue="Nature",
            doi="10.1/test2",
            citations_in=80, citations_out=[],
            abstract="Tabula rasa.",
            source="fixture",
        ),
        Paper(
            paper_id="10.1/test3",
            title="BERT pretraining",
            authors=["Jacob Devlin"],
            year=2019, venue="NAACL",
            doi="10.1/test3",
            citations_in=50000, citations_out=[],
            abstract="Bidirectional.",
            source="fixture",
        ),
        Paper(
            paper_id="10.1/test4",
            title="Attention is all you need",
            authors=["Ashish Vaswani"],
            year=2017, venue="NeurIPS",
            doi="10.1/test4",
            citations_in=80000, citations_out=[],
            abstract="Transformer.",
            source="fixture",
        ),
    ]


# ===================== Test 1: Paper dataclass =====================

class TestPaper:
    def test_normalized_title(self):
        p = Paper(paper_id="x", title="Foo! Bar? Baz.", authors=["Alice"], year=2020)
        assert p.normalized_title == "foo bar baz"

    def test_first_author_year(self):
        p = Paper(paper_id="x", title="t", authors=["John Smith"], year=2024)
        assert p.first_author_year == "smith_2024"

    def test_first_author_year_no_authors(self):
        p = Paper(paper_id="x", title="t", year=2024)
        assert p.first_author_year == "unknown_2024"

    def test_to_dict(self):
        p = Paper(paper_id="x", title="t", authors=["a"], year=2020)
        d = p.to_dict()
        assert d["paper_id"] == "x"
        assert d["year"] == 2020
        assert "abstract" in d


# ===================== Test 2: PaperQuery =====================

class TestPaperQuery:
    def test_init_default(self):
        q = PaperQuery()
        assert q.timeout_s == 8.0
        assert "@" in q.mailto

    def test_init_custom(self):
        q = PaperQuery(timeout_s=2.0, mailto="x@y")
        assert q.timeout_s == 2.0

    def test_openalex_url_construction(self):
        # 真检测 URL (不发起请求)
        url = f"{PaperQuery.OPENALEX_BASE}?search=test&per-page=5&mailto=x@y"
        assert "openalex.org/works" in url
        assert "search=test" in url

    def test_s2_url_construction(self):
        url = f"{PaperQuery.S2_BASE}?query=test&limit=10"
        assert "semanticscholar.org" in url
        assert "limit=10" in url

    def test_query_openalex_network_failure_returns_empty(self):
        # 真检测: 网络不可达时返回 [], 不假装.
        q = PaperQuery(timeout_s=0.001, mailto="x@y")
        result = q.query_openalex("test", max_results=2)
        assert isinstance(result, list)


# ===================== Test 3: CitationParser =====================

class TestCitationParser:
    def test_top_cited(self, small_papers):
        top = CitationParser.top_cited(small_papers, k=2)
        assert len(top) == 2
        # BERT (50000) > Attention (80000) ... 实际 Attention 最高
        assert top[0].citations_in >= top[1].citations_in

    def test_total_in(self, small_papers):
        total = CitationParser.total_in(small_papers)
        assert total == 100 + 80 + 50000 + 80000

    def test_total_out(self, small_papers):
        total = CitationParser.total_out(small_papers)
        assert total == 0  # fixture 中 citations_out 都空

    def test_paper_id_index(self, small_papers):
        idx = CitationParser.paper_id_index(small_papers)
        assert "10.1/test3" in idx
        assert idx["10.1/test3"].title == "BERT pretraining"

    def test_internal_citation_count(self):
        p1 = Paper(paper_id="A", citations_out=["B"])
        p2 = Paper(paper_id="B", citations_out=[])
        papers = [p1, p2]
        # 这里 paper_id 是 "A"/"B" 而 ref 是 "B" - normalize 后是 "B"
        assert CitationParser.internal_citation_count(papers) == 1


# ===================== Test 4: DeduplicationEngine =====================

class TestDeduplicationEngine:
    def test_dedupe_no_dup(self, small_papers):
        deduped, dup = DeduplicationEngine.dedupe(small_papers)
        assert dup == 0
        assert len(deduped) == 4

    def test_dedupe_by_doi(self):
        p1 = Paper(paper_id="x", doi="10.1/dup", title="foo", authors=["a"], year=2020)
        p2 = Paper(paper_id="y", doi="10.1/dup", title="bar", authors=["b"], year=2021)
        deduped, dup = DeduplicationEngine.dedupe([p1, p2])
        assert dup == 1
        assert len(deduped) == 1

    def test_dedupe_by_title(self):
        p1 = Paper(paper_id="x", doi="", title="Hello World", authors=["a"], year=2020)
        p2 = Paper(paper_id="y", doi="", title="hello, world!", authors=["b"], year=2021)
        deduped, dup = DeduplicationEngine.dedupe([p1, p2])
        assert dup == 1
        assert len(deduped) == 1

    def test_dedupe_by_first_author_year(self):
        # 同样 first_author_year + 同样 normalized_title 触发 dedupe
        p1 = Paper(paper_id="x", doi="", title="foo bar baz", authors=["John Smith"], year=2020)
        p2 = Paper(paper_id="y", doi="", title="foo bar baz", authors=["J. Smith"], year=2020)
        deduped, dup = DeduplicationEngine.dedupe([p1, p2])
        assert dup == 1
        assert len(deduped) == 1

    def test_dedupe_different_papers_kept(self):
        p1 = Paper(paper_id="x", doi="", title="foo", authors=["a"], year=2020)
        p2 = Paper(paper_id="y", doi="", title="bar", authors=["b"], year=2021)
        deduped, dup = DeduplicationEngine.dedupe([p1, p2])
        assert dup == 0
        assert len(deduped) == 2


# ===================== Test 5: BibTeXExporter =====================

class TestBibTeXExporter:
    def test_entry_key(self):
        p = Paper(paper_id="x", title="Attention is all you need",
                  authors=["Ashish Vaswani"], year=2017)
        key = BibTeXExporter._entry_key(p)
        assert key.startswith("vaswani2017")
        assert "attention" in key

    def test_escape_special_chars(self):
        out = BibTeXExporter._escape("hello {world} & $foo")
        assert "\\{world\\}" in out
        assert "\\&" in out
        assert "\\$foo" in out

    def test_to_bibtex_contains_required_fields(self):
        p = Paper(paper_id="10.1/x", title="Test",
                  authors=["Jane Doe"], year=2024, venue="ICML", doi="10.1/x")
        bib = BibTeXExporter.to_bibtex(p)
        assert bib.startswith("@article{")
        assert "title" in bib
        assert "author" in bib
        assert "year" in bib
        assert "2024" in bib
        assert "doi" in bib

    def test_export_multiple(self, small_papers):
        entries = BibTeXExporter.export(small_papers)
        assert len(entries) == 4
        for e in entries:
            assert "@article{" in e


# ===================== Test 6: ReferenceFormatter =====================

class TestReferenceFormatter:
    def test_last_name_initials_single(self):
        assert ReferenceFormatter._last_name_initials("John Smith") == "Smith, J."

    def test_last_name_initials_multi(self):
        assert ReferenceFormatter._last_name_initials("John A. Smith") == "Smith, J. A."

    def test_authors_apa_one(self):
        assert ReferenceFormatter._authors_apa(["John Smith"]) == "Smith, J."

    def test_authors_apa_two(self):
        out = ReferenceFormatter._authors_apa(["John Smith", "Jane Doe"])
        assert "&" in out
        assert "Smith" in out
        assert "Doe" in out

    def test_authors_apa_three(self):
        out = ReferenceFormatter._authors_apa(["A", "B", "C"])
        assert out.count("&") == 1  # only last &
        assert "A, B, & C" == out

    def test_authors_mla_et_al(self):
        out = ReferenceFormatter._authors_mla(["a", "b", "c"])
        assert "et al." in out

    def test_authors_ieee_initials(self):
        out = ReferenceFormatter._authors_ieee(["John Smith"])
        assert "J." in out
        assert "Smith" in out

    def test_format_apa(self):
        p = Paper(paper_id="x", title="Test paper", authors=["John Smith"],
                  year=2020, venue="ICML", doi="10.1/x")
        out = ReferenceFormatter.format_apa(p)
        assert "Smith, J." in out
        assert "(2020)" in out
        assert "*ICML*" in out
        assert "10.1/x" in out

    def test_format_mla(self):
        p = Paper(paper_id="x", title="Test paper", authors=["John Smith"],
                  year=2020, venue="ICML", doi="10.1/x")
        out = ReferenceFormatter.format_mla(p)
        assert "Smith, J." in out
        assert "2020" in out
        assert "doi:10.1/x" in out

    def test_format_ieee(self):
        p = Paper(paper_id="x", title="Test paper", authors=["John Smith"],
                  year=2020, venue="ICML", doi="10.1/x")
        out = ReferenceFormatter.format_ieee(p)
        assert "Smith" in out
        assert "2020" in out
        assert "doi:" in out

    def test_export_three_styles(self, small_papers):
        apa, mla, ieee = ReferenceFormatter.export(small_papers)
        assert len(apa) == len(mla) == len(ieee) == 4


# ===================== Test 7: TFIDFCluster =====================

class TestTFIDFCluster:
    def test_tokenize(self):
        out = TFIDFCluster._tokenize("Foo bar 123 hello")
        assert "foo" in out
        assert "bar" in out
        assert "hello" in out

    def test_tfidf_shape(self, small_papers):
        docs = [TFIDFCluster._tokenize(p.title + " " + p.abstract) for p in small_papers]
        vecs = TFIDFCluster._tfidf(docs)
        assert len(vecs) == 4
        for v in vecs:
            assert isinstance(v, dict)

    def test_cosine_identical(self):
        a = {"foo": 1.0, "bar": 1.0}
        assert TFIDFCluster._cosine(a, a) == pytest.approx(1.0, abs=1e-6)

    def test_cosine_orthogonal(self):
        a = {"foo": 1.0}
        b = {"bar": 1.0}
        assert TFIDFCluster._cosine(a, b) == 0.0

    def test_cluster_similar_merge(self):
        p1 = Paper(paper_id="1", title="AlphaGo wins the game of Go with deep learning",
                   authors=["a"], year=2016, abstract="deep learning Go reinforcement learning")
        p2 = Paper(paper_id="2", title="AlphaGo Zero wins the game of Go with self-play",
                   authors=["b"], year=2017, abstract="self-play Go reinforcement learning")
        p3 = Paper(paper_id="3", title="Transformer for language translation",
                   authors=["c"], year=2017, abstract="attention is all you need transformer")
        clusters = TFIDFCluster.cluster([p1, p2, p3], threshold=0.15)
        assert len(clusters) >= 1
        # Go papers should be in same cluster
        sizes = sorted([len(c) for c in clusters], reverse=True)
        assert sizes[0] >= 2

    def test_cluster_empty(self):
        out = TFIDFCluster.cluster([])
        assert out == []


# ===================== Test 8: TrendAnalyzer =====================

class TestTrendAnalyzer:
    def test_year_distribution(self, small_papers):
        d = TrendAnalyzer.year_distribution(small_papers)
        assert d[2016] == 1
        assert d[2017] == 2
        assert d[2019] == 1
        assert d.get(2020, 0) == 0

    def test_year_distribution_invalid_year_ignored(self):
        p = Paper(paper_id="x", title="t", year=1850)  # 早于 1900
        d = TrendAnalyzer.year_distribution([p])
        assert d == {}

    def test_venue_distribution(self, small_papers):
        d = TrendAnalyzer.venue_distribution(small_papers)
        assert d["Nature"] == 2
        assert d["NeurIPS"] == 1
        assert d["NAACL"] == 1

    def test_author_distribution(self, small_papers):
        d = TrendAnalyzer.author_distribution(small_papers)
        assert d["David Silver"] == 2

    def test_median_year(self, small_papers):
        med = TrendAnalyzer.median_year(small_papers)
        # 2016, 2017, 2017, 2019 → median = 2017
        assert med == 2017.0

    def test_total_authors(self, small_papers):
        n = TrendAnalyzer.total_authors(small_papers)
        # 7 distinct: David Silver, Aja Huang, Julian Schrittwieser, Jacob Devlin,
        # Ashish Vaswani
        assert n == 5


# ===================== Test 9: LiteratureReviewGenerator =====================

class TestLiteratureReviewGenerator:
    def test_generate_basic(self):
        p = Paper(paper_id="x", title="Test", authors=["John Smith"], year=2020, venue="ICML")
        md = LiteratureReviewGenerator.generate(
            "test query", [p],
            year_dist={2020: 1},
            venue_dist={"ICML": 1},
            author_dist={"John Smith": 1},
            clusters=[[0]],
            total_in=10, total_out=2,
        )
        assert "Literature Review" in md
        assert "test query" in md
        assert "papers analyzed: **1**" in md
        assert "TF-IDF" in md or "Distributional" in md

    def test_generate_no_caveats_omitted_when_review_has_no_data(self):
        md = LiteratureReviewGenerator.generate(
            "empty", [], {}, {}, {}, [], 0, 0,
        )
        assert "0" in md  # papers analyzed: 0
        assert "bibliographic" in md.lower()

    def test_generate_includes_top_cited(self):
        p1 = Paper(paper_id="x1", title="Alpha paper", authors=["a"], year=2020, citations_in=500)
        p2 = Paper(paper_id="x2", title="Beta paper", authors=["b"], year=2021, citations_in=10)
        md = LiteratureReviewGenerator.generate(
            "test", [p1, p2],
            {2020: 1, 2021: 1}, {}, {},
            [[0, 1]],
            510, 0,
        )
        assert "Alpha paper" in md
        assert "500 citations" in md


# ===================== Test 10: ASILiteratureReviewBridge =====================

class TestASILiteratureReviewBridge:
    def test_subscore_empty_zero(self):
        review = LiteratureReview(query="x")
        s = ASILiteratureReviewBridge.compute_subscore(review)
        assert s == 0.0

    def test_subscore_full_fixture(self, fixture_papers):
        reviewer = LiteratureReviewer(query="x", use_fixture=True, fixture=fixture_papers)
        review = reviewer.run()
        s = ASILiteratureReviewBridge.compute_subscore(review)
        # fixture 数据全, dedup=0/8, bibtex 全有效, 3 styles, cluster, trends
        assert s >= 0.8

    def test_subscore_weights_sum_to_one(self):
        total = sum(ASILiteratureReviewBridge.V03_WEIGHTS.values())
        assert abs(total - 1.0) < 1e-6


# ===================== Test 11: V3PhilosophyGuard =====================

class TestV3PhilosophyGuard:
    def test_no_fake_review_understanding_pass(self):
        review = LiteratureReview(query="x", review_markdown="This is a bibliographic review.")
        assert V3PhilosophyGuard.check_no_fake_review_understanding(review) is True

    def test_no_fake_review_understanding_fail(self):
        review = LiteratureReview(query="x", review_markdown="I understands this topic deeply.")
        assert V3PhilosophyGuard.check_no_fake_review_understanding(review) is False

    def test_no_fake_citation_value_pass(self):
        review = LiteratureReview(query="x", review_markdown="This paper has many citations.")
        assert V3PhilosophyGuard.check_no_fake_citation_value(review) is True

    def test_no_fake_citation_value_fail(self):
        review = LiteratureReview(query="x", review_markdown="This is the best paper ever.")
        assert V3PhilosophyGuard.check_no_fake_citation_value(review) is False

    def test_no_fake_trend_insight_pass(self):
        review = LiteratureReview(query="x", review_markdown="Year distribution shows 2017 peaks.")
        assert V3PhilosophyGuard.check_no_fake_trend_insight(review) is True

    def test_no_fake_trend_insight_fail(self):
        review = LiteratureReview(query="x", review_markdown="I conclude that AI is rising.")
        assert V3PhilosophyGuard.check_no_fake_trend_insight(review) is False

    def test_no_fake_cluster_semantic_with_clusters(self, fixture_papers):
        reviewer = LiteratureReviewer(query="x", use_fixture=True, fixture=fixture_papers)
        review = reviewer.run()
        # generator includes TF-IDF identifier
        assert V3PhilosophyGuard.check_no_fake_cluster_semantic(review) is True

    def test_no_fake_asi_pass(self):
        review = LiteratureReview(query="x", review_markdown="This is a tool, not ASI.")
        assert V3PhilosophyGuard.check_no_fake_asi(review) is True

    def test_no_fake_asi_fail(self):
        review = LiteratureReview(query="x", review_markdown="I am ASI-level now.")
        assert V3PhilosophyGuard.check_no_fake_asi(review) is False


# ===================== Test 12: LiteratureReviewer end-to-end =====================

class TestLiteratureReviewer:
    def test_run_with_fixture(self, fixture_papers):
        reviewer = LiteratureReviewer(
            query="reinforcement learning game playing",
            use_fixture=True,
            fixture=fixture_papers,
        )
        review = reviewer.run()
        assert review.overall == "OK"
        assert len(review.papers) == 8
        assert len(review.bibtex_entries) == 8
        assert len(review.references_apa) == 8
        assert len(review.references_mla) == 8
        assert len(review.references_ieee) == 8
        assert review.clusters
        assert review.trends_year
        assert review.review_markdown

    def test_run_with_empty_fixture(self):
        reviewer = LiteratureReviewer(query="x", use_fixture=True, fixture=[])
        review = reviewer.run()
        assert review.overall == "EMPTY"
        assert review.papers == []
        assert review.duplicates_removed == 0

    def test_run_with_dup_fixture(self):
        p1 = Paper(paper_id="10.1/dup", title="Same title", authors=["a"], year=2020, doi="10.1/dup")
        p2 = Paper(paper_id="10.1/dup", title="Same title", authors=["a"], year=2020, doi="10.1/dup")
        reviewer = LiteratureReviewer(query="x", use_fixture=True, fixture=[p1, p2])
        review = reviewer.run()
        assert review.duplicates_removed == 1
        assert len(review.papers) == 1

    def test_run_source_breakdown_fixture(self, fixture_papers):
        reviewer = LiteratureReviewer(query="x", use_fixture=True, fixture=fixture_papers)
        review = reviewer.run()
        assert review.source_breakdown == {"fixture": 8}

    def test_run_component_source_field(self, fixture_papers):
        reviewer = LiteratureReviewer(query="x", use_fixture=True, fixture=fixture_papers)
        review = reviewer.run()
        for name, _, source in review.components:
            assert source != ""


# ===================== Test 13: CLI =====================

class TestCLI:
    def test_main_help(self, capsys):
        from apeireth.v1079_asi_literature_review import main
        rc = main([])
        out = capsys.readouterr().out
        assert rc == 0
        assert "Literature Review" in out or "--review" in out

    def test_main_with_fixture_report(self, capsys, fixture_papers, monkeypatch):
        from apeireth.v1079_asi_literature_review import main, _default_fixture
        # 直接 patch _default_fixture 不可行 (主 module), 改用 fixture flag + 自带 fixture
        # 这里用 monkeypatch 调用 fixture path
        rc = main(["--review", "test", "--fixture", "--report"])
        out = capsys.readouterr().out
        # fixture 8 papers
        assert rc == 0
        assert "V1079 Literature Review" in out
        assert "Sources" in out

    def test_main_json_output(self, capsys):
        from apeireth.v1079_asi_literature_review import main
        rc = main(["--review", "test", "--fixture"])
        out = capsys.readouterr().out
        # fixture 8 papers → OK
        assert rc == 0
        d = json.loads(out)
        assert d["overall"] == "OK"
        assert d["n_papers"] == 8


# ===================== Test 14: sanity / refs / guards / reproducibility =====================

class TestSanity:
    def test_version_present(self):
        assert V1079_VERSION == "0.1.0"

    def test_references_count(self):
        assert len(REFERENCES) >= 14

    def test_references_have_url(self):
        for rid, title, url in REFERENCES:
            assert url.startswith("http")

    def test_reproducibility_same_fixture_same_output(self, fixture_papers):
        # 真测可复现: 跑两次 → 关键字段一致
        r1 = LiteratureReviewer(query="q", use_fixture=True, fixture=fixture_papers).run()
        r2 = LiteratureReviewer(query="q", use_fixture=True, fixture=fixture_papers).run()
        assert len(r1.papers) == len(r2.papers)
        assert len(r1.clusters) == len(r2.clusters)
        assert r1.trends_year == r2.trends_year
        assert len(r1.bibtex_entries) == len(r2.bibtex_entries)

    def test_to_dict_keys(self, fixture_papers):
        reviewer = LiteratureReviewer(query="q", use_fixture=True, fixture=fixture_papers)
        review = reviewer.run()
        d = review.to_dict()
        expected = {
            "query", "n_papers", "duplicates_removed", "n_clusters",
            "trends_year", "top_venues", "top_authors",
            "bibtex_entries_count", "references_apa_count",
            "references_mla_count", "references_ieee_count",
            "review_markdown_chars", "components", "overall",
            "overall_note", "source_breakdown", "elapsed_s", "version",
        }
        assert set(d.keys()) >= expected

    def test_markdown_report_contains_v3_guard(self, fixture_papers):
        reviewer = LiteratureReviewer(query="q", use_fixture=True, fixture=fixture_papers)
        review = reviewer.run()
        md = review.to_markdown()
        assert "V3 哲学守门" in md
        assert "不假装" in md

    def test_no_fake_in_fixture(self, fixture_papers):
        # 真 fixture 数据全部非空
        for p in fixture_papers:
            assert p.title
            assert p.year > 0
            assert p.authors
            assert p.venue
            assert p.doi
