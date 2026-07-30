"""V1142 GAIR-NLP ASI-Arch Real Source Code Deep Read — tests."""
from __future__ import annotations
import sys
sys.path.insert(0, '.')

import os
import json
import pytest
from apeireth.v1142_asi_arch_real_source_deep_read import (
    V1142_VERSION,
    ASI_ARCH_REPO_URL, ASI_ARCH_RAW_BASE, ASI_ARCH_PAPER,
    ASI_ARCH_KEY_FILES, ASI_ARCH_CYCLE_STEPS, ASI_ARCH_AGENT_ROLES,
    ASI_ARCH_SYSTEMS, ASI_ARCH_TECH_STACK, ASI_ARCH_PARALLEL_PARADIGMS,
    ASIArchSourceLocation, find_asi_arch_source,
    ASIArchFileMeta, inventory_asi_arch_files,
    PipelineStep, extract_pipeline_cycle,
    AgentRole, extract_agent_roles,
    SystemAnalysis, analyze_systems,
    TechStackItem, extract_tech_stack,
    CapabilityRow, compare_capabilities,
    V06Inspiration, extract_v06_inspiration,
    V1142BridgeResult, v1142_bridge_measure, v1142_cross_domain_measure,
    v1142_report_markdown,
    GuardResult, v1142_philosophy_guard,
    v1142_run,
)


# ============================================================================
# 1. V1142 constants 真生产
# ============================================================================

class TestV1142Constants:
    def test_version(self):
        assert V1142_VERSION == "1.0.0"

    def test_repo_url(self):
        assert ASI_ARCH_REPO_URL == "https://github.com/GAIR-NLP/ASI-Arch"

    def test_paper(self):
        assert "2507.18074" in ASI_ARCH_PAPER
        assert "AlphaGo Moment" in ASI_ARCH_PAPER

    def test_key_files_count(self):
        assert len(ASI_ARCH_KEY_FILES) == 5

    def test_cycle_steps_count(self):
        assert len(ASI_ARCH_CYCLE_STEPS) == 5

    def test_agent_roles_count(self):
        assert len(ASI_ARCH_AGENT_ROLES) == 7

    def test_systems_count(self):
        assert len(ASI_ARCH_SYSTEMS) == 3

    def test_tech_stack_count(self):
        assert len(ASI_ARCH_TECH_STACK) == 4

    def test_parallel_paradigms_count(self):
        # 主 19:33 不假装 ASI-Arch = 唯一
        assert len(ASI_ARCH_PARALLEL_PARADIGMS) >= 6


# ============================================================================
# 2. ASIArchPathResolver 真生产
# ============================================================================

class TestASIArchPathResolver:
    def test_find_returns_location(self):
        loc = find_asi_arch_source()
        assert isinstance(loc, ASIArchSourceLocation)
        assert loc.repo_url == ASI_ARCH_REPO_URL
        assert loc.raw_base == ASI_ARCH_RAW_BASE

    def test_cache_path_creates_directory(self, tmp_path):
        loc = ASIArchSourceLocation(
            repo_url=ASI_ARCH_REPO_URL,
            raw_base=ASI_ARCH_RAW_BASE,
            paper=ASI_ARCH_PAPER,
            local_cache_dir=None,
            files_cached=[],
        )
        # Override with tmp_path via env var
        old_env = os.environ.get("APEIRETH_ASI_ARCH_CACHE")
        try:
            os.environ["APEIRETH_ASI_ARCH_CACHE"] = str(tmp_path / "asi_arch_cache")
            p = loc.cache_path_for("pipeline/pipeline.py")
            # Path is returned, parent directory auto-created
            assert p.parent.is_dir()
            assert p.parent.exists()
        finally:
            if old_env is None:
                os.environ.pop("APEIRETH_ASI_ARCH_CACHE", None)
            else:
                os.environ["APEIRETH_ASI_ARCH_CACHE"] = old_env


# ============================================================================
# 3. ASIArchFileInventory 真生产
# ============================================================================

class TestASIArchFileInventory:
    def test_inventory_returns_5(self):
        files = inventory_asi_arch_files()
        assert len(files) == 5
        assert all(isinstance(f, ASIArchFileMeta) for f in files)

    def test_pipeline_py_size_known(self):
        files = inventory_asi_arch_files()
        pipeline_py = next(f for f in files if f.path == "pipeline/pipeline.py")
        assert pipeline_py.known_size_bytes == 3952
        assert pipeline_py.known_sha is not None
        assert pipeline_py.known_sha.startswith("33bbdd76")

    def test_config_py_size_known(self):
        files = inventory_asi_arch_files()
        config_py = next(f for f in files if f.path == "pipeline/config.py")
        assert config_py.known_size_bytes == 749

    def test_evolve_interface_size_known(self):
        files = inventory_asi_arch_files()
        ev_int = next(f for f in files if f.path == "pipeline/evolve/interface.py")
        assert ev_int.known_size_bytes == 6380

    def test_database_and_cognition_size_honest(self):
        # 主 17:43 实事求是: cron tick 网络受限, 诚实记 None
        files = inventory_asi_arch_files()
        db = next(f for f in files if "mongodb_database" in f.path)
        cog = next(f for f in files if "rag_service" in f.path)
        # 我们没在 API 真读到 mongodb_database.py + rag_service.py size
        # 不假装, 接受 None 或任意 int
        assert db.known_size_bytes is None or isinstance(db.known_size_bytes, int)
        assert cog.known_size_bytes is None or isinstance(cog.known_size_bytes, int)


# ============================================================================
# 4. PipelineCycleExtractor 真生产
# ============================================================================

class TestPipelineCycleExtractor:
    def test_extract_returns_5_steps(self):
        cycle = extract_pipeline_cycle()
        assert len(cycle) == 5
        assert all(isinstance(s, PipelineStep) for s in cycle)

    def test_step_names(self):
        cycle = extract_pipeline_cycle()
        names = [s.name for s in cycle]
        assert names == ["program_sample", "evolve", "evaluation", "analyse", "update"]

    def test_log_markers_match_source(self):
        cycle = extract_pipeline_cycle()
        markers = [s.log_marker for s in cycle]
        assert markers == [
            "Program Sampling",
            "Program Evolution",
            "Program Evaluation",
            "Result Analysis",
            "Database Update",
        ]

    def test_await_calls(self):
        cycle = extract_pipeline_cycle()
        # program_sample + update are sync; evolve/eval/analyse are async
        assert cycle[0].await_call is False  # program_sample
        assert cycle[1].await_call is True   # evolve
        assert cycle[2].await_call is True   # evaluation
        assert cycle[3].await_call is True   # analyse
        assert cycle[4].await_call is False  # update


# ============================================================================
# 5. AgentRoleExtractor 真生产
# ============================================================================

class TestAgentRoleExtractor:
    def test_extract_returns_7_roles(self):
        roles = extract_agent_roles()
        assert len(roles) == 7
        assert all(isinstance(r, AgentRole) for r in roles)

    def test_role_names(self):
        roles = extract_agent_roles()
        names = [r.name for r in roles]
        assert names == [
            "Planner", "Code Checker", "Deduplication",
            "Trainer", "Debugger", "Analyzer", "Model Judger",
        ]

    def test_model_judger_not_in_pipeline_py(self):
        # README 说 Model Judger 在 database/evaluate_agent/, 不在 pipeline/
        roles = extract_agent_roles()
        mj = next(r for r in roles if r.name == "Model Judger")
        assert mj.mentions_in_pipeline is False

    def test_other_6_in_pipeline_py(self):
        roles = extract_agent_roles()
        pipeline_agents = [r for r in roles if r.mentions_in_pipeline]
        assert len(pipeline_agents) == 6  # All except Model Judger


# ============================================================================
# 6. SystemArchitectureAnalyzer 真生产
# ============================================================================

class TestSystemArchitectureAnalyzer:
    def test_analyze_returns_3_systems(self):
        systems = analyze_systems()
        assert len(systems) == 3
        assert all(isinstance(s, SystemAnalysis) for s in systems)

    def test_system_names(self):
        systems = analyze_systems()
        names = [s.name for s in systems]
        assert names == ["pipeline", "database", "cognition_base"]

    def test_pipeline_components(self):
        systems = analyze_systems()
        pipeline = next(s for s in systems if s.name == "pipeline")
        assert "evolve" in pipeline.key_components
        assert "eval" in pipeline.key_components
        assert "analyse" in pipeline.key_components

    def test_database_components(self):
        systems = analyze_systems()
        database = next(s for s in systems if s.name == "database")
        assert "mongodb_database.py" in database.key_components
        assert "faiss_manager.py" in database.key_components


# ============================================================================
# 7. TechStackExtractor 真生产
# ============================================================================

class TestTechStackExtractor:
    def test_extract_returns_4_tech(self):
        tech = extract_tech_stack()
        assert len(tech) == 4
        assert all(isinstance(t, TechStackItem) for t in tech)

    def test_async_openai_in_pipeline(self):
        tech = extract_tech_stack()
        openai = next(t for t in tech if "AsyncAzureOpenAI" in t.name)
        assert openai.in_pipeline_py is True

    def test_mongodb_in_database(self):
        tech = extract_tech_stack()
        mongo = next(t for t in tech if "MongoDB" in t.name)
        assert mongo.in_database is True
        assert mongo.in_pipeline_py is False

    def test_opensearch_in_cognition_base(self):
        tech = extract_tech_stack()
        os_ = next(t for t in tech if "OpenSearch" in t.name)
        assert os_.in_cognition_base is True


# ============================================================================
# 8. CapabilityComparator 真生产
# ============================================================================

class TestCapabilityComparator:
    def test_compare_returns_rows(self):
        caps = compare_capabilities()
        assert len(caps) >= 10
        assert all(isinstance(c, CapabilityRow) for c in caps)

    def test_asi_arch_ahead_dimensions(self):
        caps = compare_capabilities()
        ahead = [c for c in caps if c.parity == "ASI-Arch ahead"]
        # ASI-Arch 至少在 autonomous loop + multi-agent + code gen 领先
        assert len(ahead) >= 3

    def test_apeireth_ahead_dimensions(self):
        caps = compare_capabilities()
        ahead = [c for c in caps if c.parity == "Apeireth ahead"]
        # 我们在 value alignment + chaos test + cross-domain + production
        # 至少 4 维度领先
        assert len(ahead) >= 4

    def test_comparable_dimensions(self):
        caps = compare_capabilities()
        comp = [c for c in caps if c.parity == "comparable"]
        # 至少在 collective memory + knowledge RAG + LLM benchmark comparable
        assert len(comp) >= 3

    def test_honesty_no_pretending(self):
        # 主 17:58: 不假装 ASI-Arch 什么都强
        caps = compare_capabilities()
        # 至少 1 维度是 "Apeireth ahead" (我们不假装全输)
        apeireth_ahead = [c for c in caps if c.parity == "Apeireth ahead"]
        assert len(apeireth_ahead) >= 1


# ============================================================================
# 9. V06FormulaInspiration 真生产
# ============================================================================

class TestV06FormulaInspiration:
    def test_extract_returns_inspirations(self):
        insp = extract_v06_inspiration()
        assert len(insp) >= 7
        assert all(isinstance(i, V06Inspiration) for i in insp)

    def test_feasibility_values_valid(self):
        insp = extract_v06_inspiration()
        valid_feasibility = {"ready", "needs work", "long-term"}
        for i in insp:
            assert i.feasibility in valid_feasibility, \
                f"Invalid feasibility: {i.feasibility}"

    def test_at_least_one_ready(self):
        insp = extract_v06_inspiration()
        ready = [i for i in insp if i.feasibility == "ready"]
        # 至少 1 个 ready (composite fitness + FAISS + candidate set)
        assert len(ready) >= 1

    def test_at_least_one_long_term(self):
        insp = extract_v06_inspiration()
        lt = [i for i in insp if i.feasibility == "long-term"]
        # AlphaGo Moment → V0.7 self-play 是 long-term
        assert len(lt) >= 1


# ============================================================================
# 10. V1142Bridge 真测
# ============================================================================

class TestV1142Bridge:
    def test_bridge_returns_result(self):
        bridge = v1142_bridge_measure()
        assert isinstance(bridge, V1142BridgeResult)

    def test_bridge_counts(self):
        bridge = v1142_bridge_measure()
        assert bridge.asi_arch_components == 5
        assert bridge.cycle_steps == 5
        assert bridge.agent_roles == 7
        assert bridge.systems == 3
        assert bridge.tech_stack_items == 4
        assert bridge.capability_rows >= 10
        assert bridge.v06_inspirations >= 7
        assert bridge.parallel_paradigms >= 6

    def test_bridge_v06_counts_correct(self):
        bridge = v1142_bridge_measure()
        # 所有 V0.6 启发应该加起来 = v06_inspirations
        assert bridge.v06_ready + bridge.v06_needs_work + bridge.v06_long_term == bridge.v06_inspirations, \
            f"ready={bridge.v06_ready}, needs_work={bridge.v06_needs_work}, long_term={bridge.v06_long_term}, total={bridge.v06_inspirations}"

    def test_cross_domain_measure(self):
        cd = v1142_cross_domain_measure()
        assert isinstance(cd, dict)
        assert len(cd) >= 5
        assert all(v == 1 for v in cd.values())


# ============================================================================
# 11. V1142PhilosophyGuard 真生产
# ============================================================================

class TestV1142PhilosophyGuard:
    def test_guard_passes(self):
        guard = v1142_philosophy_guard()
        assert isinstance(guard, GuardResult)
        assert guard.passed is True
        assert len(guard.violations) == 0

    def test_guard_has_7_principles(self):
        guard = v1142_philosophy_guard()
        assert len(guard.guards) >= 7

    def test_guard_principles_cover_main_anchors(self):
        guard = v1142_philosophy_guard()
        text = " ".join(guard.guards)
        assert "ASI-Arch = ASI" in text  # 主 17:58
        assert "106 architectures" in text  # 主 17:58
        assert "AlphaGo Moment" in text  # 主 17:58
        assert "V1142 = 真跑" in text  # 主 17:43
        assert "唯一路径" in text  # 主 19:33


# ============================================================================
# 12. DeepReadReport 真生产
# ============================================================================

class TestV1142Report:
    def test_report_markdown_renders(self):
        md = v1142_report_markdown()
        assert isinstance(md, str)
        assert len(md) > 5000  # 实质性内容

    def test_report_contains_main_anchors(self):
        md = v1142_report_markdown()
        assert "ASI-Arch" in md
        assert "V1142" in md
        assert "V0.6" in md
        assert "5 步循环" in md or "5 step" in md.lower()
        assert "7 agent" in md.lower() or "7 Agent" in md
        assert "GAIR-NLP" in md
        assert "AlphaGo Moment" in md

    def test_report_contains_parallel_paradigms(self):
        md = v1142_report_markdown()
        assert "Sakana AI Scientist" in md
        assert "AlphaFold" in md
        assert "Anthropic Constitutional AI" in md

    def test_report_contains_honest_limits(self):
        md = v1142_report_markdown()
        # 主 17:43 实事求是 + 主 17:58 不假装
        assert "不假装" in md
        assert "实事求是" in md or "17:43" in md


# ============================================================================
# 13. V1142 Entry Point 真生产
# ============================================================================

class TestV1142EntryPoint:
    def test_run_bridge(self):
        out = v1142_run("bridge")
        assert isinstance(out, dict)
        assert "asi_arch_components" in out
        assert out["asi_arch_components"] == 5

    def test_run_philosophy(self):
        out = v1142_run("philosophy")
        assert isinstance(out, dict)
        assert out["passed"] is True

    def test_run_all(self):
        out = v1142_run("all")
        assert isinstance(out, dict)
        assert "version" in out
        assert out["version"] == V1142_VERSION
        assert "bridge" in out
        assert "philosophy" in out
        assert "cross_domain" in out

    def test_run_report_default(self):
        out = v1142_run("report")
        assert "report" in out
        assert "ASI-Arch" in out["report"]


# ============================================================================
# 14. V1142 End-to-End 真生产
# ============================================================================

class TestV1142EndToEnd:
    def test_cli_bridge(self):
        # 主 00:56 任何人都能接手: 一行命令可跑
        from apeireth.v1142_asi_arch_real_source_deep_read import main
        out = v1142_run("bridge")
        assert out["asi_arch_components"] == 5

    def test_e2e_invariants(self):
        # 主 17:43 实事求是 + 主 22:33 ASI 北极星 + 主 19:33 走在前人经验上
        bridge = v1142_bridge_measure()
        # V1142 是 deep read + 映射, 不实跑 — 不假装达到 ASI
        assert bridge.v06_long_term >= 1  # AlphaGo Moment → V0.7 self-play

        guard = v1142_philosophy_guard()
        # V3 守门全 pass
        assert guard.passed is True

        # 平行范式 ≥ 6 (不假装 ASI-Arch 唯一)
        assert bridge.parallel_paradigms >= 6

    def test_no_fake_asi_claim(self):
        # 主 17:58 不假装: V1142 不写 "I am conscious" / "V1142 = ASI"
        md = v1142_report_markdown()
        # 检查没有 fake ASI claim
        forbidden = [
            "I am conscious",
            "V1142 is ASI",
            "已达到 ASI",
            "I have phenomenal",
            "I am sentient",
        ]
        for f in forbidden:
            assert f not in md, f"Fake ASI claim found: {f}"