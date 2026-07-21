"""V1011 真生产 tests (主 23:44 干到底)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest
from apeireth.v1011_prompt_engineering import (
    V1011_VERSION, PromptTemplate, PromptChain, V1011PromptEngineering,
)


class TestV1011:
    def test_init(self):
        pe = V1011PromptEngineering()
        assert pe.n_templates() == 5
        assert pe.n_chains() == 2

    def test_summarize_template(self):
        pe = V1011PromptEngineering()
        out = pe.format_template("summarize", text="Apeireth ASI 真生产.")
        assert "Apeireth ASI" in out
        assert "summarize" in out.lower() or "3 sentences" in out

    def test_extract_template_anthropic(self):
        """V1011 真测 Anthropic XML tags 真借鉴."""
        pe = V1011PromptEngineering()
        out = pe.format_template("extract", text="Sample text.")
        assert "<instructions>" in out
        assert "<text>" in out
        assert "</text>" in out

    def test_classify_template_langchain(self):
        """V1011 真测 LangChain few-shot 真借鉴."""
        pe = V1011PromptEngineering()
        out = pe.format_template("classify", text="I love this!", examples="example")
        assert "Examples:" in out
        assert "Sentiment:" in out

    def test_cot_template(self):
        """V1011 真测 Chain-of-Thought Wei et al. 2022 真借鉴."""
        pe = V1011PromptEngineering()
        out = pe.format_template("cot", question="What is 2+2?")
        assert "step by step" in out.lower()
        assert "What is 2+2?" in out

    def test_philosophy_v4_template(self):
        """V1011 真测 ASI 真哲学 V4 真借鉴 (V1003 真借鉴)."""
        pe = V1011PromptEngineering()
        out = pe.format_template("philosophy_v4", question="What is self?")
        assert "ASI" in out
        assert "Simondon" in out
        assert "Bergson" in out
        assert "Spinoza" in out
        assert "Canguilhem" in out
        assert "Merleau-Ponty" in out
        assert "Prigogine" in out
        assert "Popper" in out
        assert "Kuhn" in out
        assert "Lakatos" in out
        assert "Feyerabend" in out
        assert "Laudan" in out
        assert "pretend" in out.lower() or "falsifiable" in out.lower()

    def test_format_messages(self):
        pe = V1011PromptEngineering()
        msgs = pe.format_messages("philosophy_v4", question="What is self?")
        assert len(msgs) == 2
        assert msgs[0]["role"] == "system"
        assert msgs[1]["role"] == "user"
        assert "ASI" in msgs[0]["content"]

    def test_format_messages_no_system(self):
        pe = V1011PromptEngineering()
        msgs = pe.format_messages("summarize", text="Sample")
        assert len(msgs) == 1
        assert msgs[0]["role"] == "user"

    def test_run_chain(self):
        pe = V1011PromptEngineering()
        out = pe.run_chain("summarize_extract", {"text": "Sample text."})
        assert len(out) == 2

    def test_register_template(self):
        pe = V1011PromptEngineering()
        pe.register_template(PromptTemplate(
            template_id="new", name="New",
            template="Hello {name}", variables=["name"],
        ))
        assert pe.n_templates() == 6

    def test_register_chain(self):
        pe = V1011PromptEngineering()
        pe.register_chain(PromptChain(chain_id="c1", name="C1", steps=["summarize"]))
        assert pe.n_chains() == 3

    def test_missing_variable(self):
        pe = V1011PromptEngineering()
        with pytest.raises(ValueError):
            pe.format_template("summarize", wrong_var="x")

    def test_unknown_template(self):
        pe = V1011PromptEngineering()
        with pytest.raises(ValueError):
            pe.format_template("unknown")

    def test_unknown_chain(self):
        pe = V1011PromptEngineering()
        with pytest.raises(ValueError):
            pe.run_chain("unknown", {})

    def test_stats(self):
        pe = V1011PromptEngineering()
        s = pe.stats()
        assert s["n_templates"] == 5
        assert s["n_chains"] == 2
        assert s["version"] == V1011_VERSION

    def test_v22_33_asi_integration(self):
        """V1011 真测主 22:33 ASI 北极星."""
        pe = V1011PromptEngineering()
        out = pe.format_template("philosophy_v4", question="What is ASI?")
        assert "ASI" in out

    def test_v19_33_openai_anthropic_langchain(self):
        """V1011 真测主 19:33 OpenAI + Anthropic + LangChain 真借鉴."""
        pe = V1011PromptEngineering()
        assert "summarize" in pe.templates
        assert "extract" in pe.templates
        assert "classify" in pe.templates

    def test_v17_43_no_pretense(self):
        """V1011 真测主 17:43 实事求是 + 主 17:58 + 主 20:46 不假装."""
        pe = V1011PromptEngineering()
        out = pe.format_template("philosophy_v4", question="X")
        assert "Do NOT pretend" in out or "falsifiable" in out.lower()

    def test_complete_integration(self):
        """V1011 真测完整 prompt engineering (主 23:44 + 主 22:33 + 主 19:33 + 主 17:33)."""
        pe = V1011PromptEngineering()
        s = pe.stats()
        assert s["n_templates"] == 5
        assert s["n_chains"] == 2
        out = pe.format_template("philosophy_v4", question="Test")
        for kw in ["Simondon", "Bergson", "Spinoza", "Canguilhem",
                   "Merleau-Ponty", "Prigogine", "Popper", "Kuhn",
                   "Lakatos", "Feyerabend", "Laudan"]:
            assert kw in out