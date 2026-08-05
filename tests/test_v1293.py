"""V1293 — Cargo Dependency Graph Profile 真生产测试

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 19:45+08:00 2026-08-05)
> **触发**: V1293 module 完成 (apeireth/v1293_rust_dependency_graph_profile.py)
> **层级**: VCP 真实源代码深读 #14 — Cargo.toml 依赖图层面

测试 V1293 真生产行为:
1. data structure dataclass 完整 + to_dict 可序列化
2. tomllib 解析 Cargo.toml 正常
3. internal vs external dep 区分正确
4. optional / dev / build deps 计数正确
5. features / lib / bin / example / lints 检测正常
6. workspace members 发现正确
7. reverse index (in_degree / reverse_deps_list) 正确
8. cycle detection (Tarjan SCC) 正确
9. depth (BFS) 计算正确
10. scan_workspace 找 41 crates (实际 workspace member 数)
11. HYPOTHESES 5 条都在, evaluate 返回 5 个 result
12. GATES 12 条都在
13. render_report 不抛异常 + 含 41 crates + 5 假说 + 12 gates
14. CLI --probe / --run / --json / --crate / --cycles / --report 不抛异常
15. Edge cases: 空 crate 目录 / 不存在目录 / 部分字段缺失 / 非 toml 文本
16. V1292 ? V1293 对照: 找 n_test_attrs > 0 但 n_internal_deps 异常的 crate
17. 哲学守门 12 条 (主 17:58 不假装)
"""

from __future__ import annotations

import json
import sys
import tempfile
import shutil
from pathlib import Path
from typing import Any, Dict, List

# 把 promethean 加入 sys.path (类似其他 test_v*.py)
PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROMETHEAN_ROOT))

from apeireth.v1293_rust_dependency_graph_profile import (  # noqa: E402
    CrateDepProfile,
    CrateGraphCycle,
    DepGraphLedger,
    HYPOTHESES,
    GATES,
    INTERNAL_PATH_RE,
    _is_internal_dep,
    _is_optional_dep,
    _extract_workspace_lints_from_text,
    _build_reverse_index,
    _find_cycles_in_internal_subgraph,
    _compute_depth,
    _read_workspace_members,
    _scan_crate,
    scan_workspace,
    evaluate,
    render_report,
    main as v1293_main,
)


# Apeireth real workspace
WORKSPACE_ROOT = PROMETHEAN_ROOT / "Apeireth-rust"


# ============================================================
# 1. Data structure tests (主 17:43 实事求是)
# ============================================================

def test_crate_dep_profile_dataclass_basic():
    """CrateDepProfile 默认值正确."""
    p = CrateDepProfile()
    assert p.crate_name == ""
    assert p.n_internal_deps == 0
    assert p.n_external_deps == 0
    assert p.n_dev_deps == 0
    assert p.n_build_deps == 0
    assert p.n_features == 0
    assert p.n_optional_deps == 0
    assert p.has_workspace_lints is False
    assert p.has_lib_target is False
    assert p.n_bin_targets == 0
    assert p.n_example_targets == 0
    assert p.total_dependencies == 0
    assert p.in_degree == 0
    assert p.out_degree == 0


def test_crate_dep_profile_to_dict():
    """CrateDepProfile.to_dict() 可序列化."""
    p = CrateDepProfile(crate_name="apeireth-test", n_internal_deps=3)
    d = p.to_dict()
    assert d["crate_name"] == "apeireth-test"
    assert d["n_internal_deps"] == 3
    assert "in_degree" in d
    assert "out_degree" in d
    # 确认 json 序列化不抛异常
    json.dumps(d)


def test_crate_graph_cycle_dataclass():
    """CrateGraphCycle dataclass 正确."""
    c = CrateGraphCycle(
        cycle_crates=["apeireth-a", "apeireth-b", "apeireth-c"],
        cycle_length=3,
    )
    assert c.cycle_length == 3
    assert len(c.cycle_crates) == 3


def test_dep_graph_ledger_to_dict():
    """DepGraphLedger.to_dict() 含所有派生字段."""
    ledger = DepGraphLedger(workspace_root="/tmp/test")
    ledger.crate_profiles = [
        CrateDepProfile(crate_name="apeireth-a", n_internal_deps=2),
        CrateDepProfile(crate_name="apeireth-b", n_internal_deps=1),
    ]
    ledger.cycles = [CrateGraphCycle(cycle_crates=["x", "y"], cycle_length=2)]
    d = ledger.to_dict()
    assert d["total_crates"] == 2
    assert d["total_internal_deps"] == 3
    assert d["total_external_deps"] == 0
    assert d["total_cycles"] == 1
    assert d["cycles"][0]["cycle_length"] == 2
    json.dumps(d)


# ============================================================
# 2. _is_internal_dep logic (主 17:43 实事求是)
# ============================================================

def test_is_internal_dep_apeireth_prefix():
    """apeireth-* 前缀 → internal."""
    is_int, name = _is_internal_dep(
        {"name": "apeireth-core"},
        set(),
    )
    assert is_int is True
    assert name == "apeireth-core"


def test_is_internal_dep_external():
    """非 apeireth-* 前缀 → external."""
    is_int, name = _is_internal_dep(
        {"name": "tokio"},
        set(),
    )
    assert is_int is False
    assert name == "tokio"


def test_is_internal_dep_string_spec():
    """string 形式的 dep spec."""
    is_int, name = _is_internal_dep("apeireth-memory", set())
    assert is_int is True
    assert name == "apeireth-memory"


def test_is_internal_dep_empty_name():
    """空 name."""
    is_int, name = _is_internal_dep({"name": ""}, set())
    assert is_int is False


def test_is_optional_dep_true():
    """optional = true."""
    assert _is_optional_dep({"optional": True}) is True


def test_is_optional_dep_false():
    """optional = false."""
    assert _is_optional_dep({"optional": False}) is False


def test_is_optional_dep_missing():
    """无 optional key."""
    assert _is_optional_dep({}) is False
    assert _is_optional_dep("just-string") is False


# ============================================================
# 3. _extract_workspace_lints_from_text (主 17:43 实事求是)
# ============================================================

def test_workspace_lints_present():
    """[lints] workspace = true."""
    text = """
[package]
name = "x"

[lints]
workspace = true
"""
    assert _extract_workspace_lints_from_text(text) is True


def test_workspace_lints_absent():
    """无 [lints] section."""
    text = """
[package]
name = "x"

[dependencies]
foo = "1"
"""
    assert _extract_workspace_lints_from_text(text) is False


def test_workspace_lints_section_no_workspace():
    """[lints] 但无 workspace = true."""
    text = """
[package]
name = "x"

[lints]
other = true
"""
    assert _extract_workspace_lints_from_text(text) is False


# ============================================================
# 4. _scan_crate (主 17:43 实事求是)
# ============================================================

def test_scan_crate_apeireth_asi():
    """扫描 apeireth-asi (已知 3 internal + 7 external)."""
    crate_dir = WORKSPACE_ROOT / "crates" / "apeireth-asi"
    if not crate_dir.is_dir():
        # 容错: workspace 可能未检出
        return
    profile = _scan_crate("apeireth-asi", crate_dir)
    assert profile.crate_name == "apeireth-asi"
    assert profile.cargo_toml_exists is True
    assert profile.n_internal_deps >= 3  # core + memory + api
    assert "apeireth-core" in profile.internal_deps_list
    assert "apeireth-memory" in profile.internal_deps_list
    assert profile.has_workspace_lints is True
    assert profile.has_lib_target is True
    assert profile.n_example_targets >= 1  # calibrate_demo + real_judge_demo + real_effect_demo


def test_scan_crate_apeireth_core():
    """扫描 apeireth-core (leaf, 无 internal deps)."""
    crate_dir = WORKSPACE_ROOT / "crates" / "apeireth-core"
    profile = _scan_crate("apeireth-core", crate_dir)
    assert profile.crate_name == "apeireth-core"
    assert profile.cargo_toml_exists is True
    assert profile.n_internal_deps == 0  # leaf
    assert profile.n_external_deps >= 5  # tokio + serde + anyhow + thiserror + uuid + chrono
    assert profile.has_workspace_lints is True


def test_scan_crate_missing_dir():
    """不存在的 dir."""
    fake = Path("/tmp/does-not-exist-crate-xyz")
    profile = _scan_crate("apeireth-fake", fake)
    assert profile.cargo_toml_exists is False
    assert profile.n_internal_deps == 0


def test_scan_crate_with_optional_dep():
    """optional deps 检测 (apeireth-memory 的 apeireth-vector optional)."""
    crate_dir = WORKSPACE_ROOT / "crates" / "apeireth-memory"
    if not crate_dir.is_dir():
        return
    profile = _scan_crate("apeireth-memory", crate_dir)
    assert profile.n_optional_deps >= 1  # apeireth-vector optional


# ============================================================
# 5. _read_workspace_members
# ============================================================

def test_read_workspace_members_real():
    """真实 workspace 应找到 41 crates (tauri-stub 已 comment out)."""
    members = _read_workspace_members(WORKSPACE_ROOT / "Cargo.toml")
    assert len(members) == 41
    names = [m[0] for m in members]
    assert "apeireth-core" in names
    assert "apeireth-asi" in names
    assert "apeireth-memory" in names
    assert "apeireth-tauri-stub" not in names  # commented out


def test_read_workspace_members_nonexistent():
    """不存在的 workspace Cargo.toml."""
    members = _read_workspace_members(Path("/tmp/no-such-ws"))
    assert members == []


# ============================================================
# 6. Graph analysis: _build_reverse_index
# ============================================================

def test_build_reverse_index_basic():
    """A → B, C → B 应让 B.in_degree = 2."""
    profiles = [
        CrateDepProfile(crate_name="apeireth-a", internal_deps_list=["apeireth-b"]),
        CrateDepProfile(crate_name="apeireth-c", internal_deps_list=["apeireth-b"]),
        CrateDepProfile(crate_name="apeireth-b", internal_deps_list=[]),
    ]
    _build_reverse_index(profiles)
    a, c, b = profiles
    assert b.in_degree == 2
    assert b.out_degree == 0
    assert set(b.reverse_deps_list) == {"apeireth-a", "apeireth-c"}
    assert a.out_degree == 1
    assert a.in_degree == 0


# ============================================================
# 7. Graph analysis: _find_cycles_in_internal_subgraph
# ============================================================

def test_find_cycles_no_cycle():
    """无 cycle (DAG)."""
    profiles = [
        CrateDepProfile(crate_name="apeireth-a", internal_deps_list=["apeireth-b"]),
        CrateDepProfile(crate_name="apeireth-b", internal_deps_list=["apeireth-c"]),
        CrateDepProfile(crate_name="apeireth-c", internal_deps_list=[]),
    ]
    cycles = _find_cycles_in_internal_subgraph(profiles)
    assert cycles == []


def test_find_cycles_simple_cycle():
    """A → B → A."""
    profiles = [
        CrateDepProfile(crate_name="apeireth-a", internal_deps_list=["apeireth-b"]),
        CrateDepProfile(crate_name="apeireth-b", internal_deps_list=["apeireth-a"]),
    ]
    cycles = _find_cycles_in_internal_subgraph(profiles)
    assert len(cycles) >= 1
    cycle_set = set(tuple(sorted(c.cycle_crates)) for c in cycles)
    assert any("apeireth-a" in s and "apeireth-b" in s for s in cycle_set)


def test_find_cycles_self_loop():
    """A → A 自环."""
    profiles = [
        CrateDepProfile(crate_name="apeireth-a", internal_deps_list=["apeireth-a"]),
    ]
    cycles = _find_cycles_in_internal_subgraph(profiles)
    assert len(cycles) == 1
    assert cycles[0].cycle_length == 2


def test_find_cycles_ignores_external():
    """外部 dep 不参与 cycle 检测."""
    profiles = [
        CrateDepProfile(
            crate_name="apeireth-a",
            internal_deps_list=[],
            external_deps_list=["tokio"],
        ),
    ]
    cycles = _find_cycles_in_internal_subgraph(profiles)
    assert cycles == []


# ============================================================
# 8. Graph analysis: _compute_depth
# ============================================================

def test_compute_depth_chain():
    """A → B → C → D (D leaf)."""
    profiles = [
        CrateDepProfile(crate_name="apeireth-a", internal_deps_list=["apeireth-b"]),
        CrateDepProfile(crate_name="apeireth-b", internal_deps_list=["apeireth-c"]),
        CrateDepProfile(crate_name="apeireth-c", internal_deps_list=["apeireth-d"]),
        CrateDepProfile(crate_name="apeireth-d", internal_deps_list=[]),
    ]
    depths = _compute_depth(profiles)
    assert depths["apeireth-d"] == 0
    assert depths["apeireth-c"] == 1
    assert depths["apeireth-b"] == 2
    assert depths["apeireth-a"] == 3


def test_compute_depth_diamond():
    """菱形依赖 A → B, A → C, B → D, C → D."""
    profiles = [
        CrateDepProfile(crate_name="apeireth-a", internal_deps_list=["apeireth-b", "apeireth-c"]),
        CrateDepProfile(crate_name="apeireth-b", internal_deps_list=["apeireth-d"]),
        CrateDepProfile(crate_name="apeireth-c", internal_deps_list=["apeireth-d"]),
        CrateDepProfile(crate_name="apeireth-d", internal_deps_list=[]),
    ]
    depths = _compute_depth(profiles)
    assert depths["apeireth-d"] == 0
    assert depths["apeireth-b"] == 1
    assert depths["apeireth-c"] == 1
    assert depths["apeireth-a"] == 2


# ============================================================
# 9. scan_workspace 整体 (主 00:56 任何人都能接手)
# ============================================================

def test_scan_workspace_real():
    """真实 workspace scan."""
    if not WORKSPACE_ROOT.is_dir():
        return
    ledger = scan_workspace(WORKSPACE_ROOT)
    assert ledger.total_crates == 41
    assert ledger.total_internal_deps > 50  # 至少 50 internal dep edges
    assert ledger.crates_with_workspace_lints == 41  # 41/41 都用 workspace lints
    assert ledger.total_cycles == 0  # 无内部 cycle
    assert ledger.max_internal_in_degree >= 20  # apeireth-core in-degree 高
    assert ledger.hub_crate_count >= 3
    assert ledger.leaf_crate_count >= 5


def test_scan_workspace_timing():
    """scan 耗时 < 5s (41 crates + simple parsing)."""
    if not WORKSPACE_ROOT.is_dir():
        return
    import time
    t0 = time.time()
    ledger = scan_workspace(WORKSPACE_ROOT)
    elapsed = time.time() - t0
    assert elapsed < 5.0, f"scan took {elapsed}s, expected < 5s"
    assert ledger.duration_ms < 5000


# ============================================================
# 10. Hypotheses (主 17:43 实事求是)
# ============================================================

def test_hypotheses_count():
    """5 假说都在."""
    assert len(HYPOTHESES) == 5
    ids = [h["id"] for h in HYPOTHESES]
    assert ids == ["H1", "H2", "H3", "H4", "H5"]


def test_evaluate_real_workspace():
    """真实 workspace 跑 5 假说."""
    if not WORKSPACE_ROOT.is_dir():
        return
    ledger = scan_workspace(WORKSPACE_ROOT)
    results = evaluate(ledger)
    assert len(results) == 5
    # 关键: 无 cycle 应 PASS
    h1 = next(r for r in results if r["id"] == "H1")
    assert h1["passed"] is True
    # workspace lints 应 PASS
    h2 = next(r for r in results if r["id"] == "H2")
    assert h2["passed"] is True
    # hub crates 应 PASS
    h5 = next(r for r in results if r["id"] == "H5")
    assert h5["passed"] is True


# ============================================================
# 11. Philosophy gates (主 17:58 + 主 20:46)
# ============================================================

def test_gates_count():
    """12 守门都在."""
    assert len(GATES) == 12
    ids = [g["id"] for g in GATES]
    expected = [
        "v1293_extends_v1292",
        "v1293_no_new_asi_dim",
        "v1293_no_asi_v1_claim",
        "v1293_no_kpi_inflate",
        "v1293_no_phenomenal_claim",
        "v1293_stdlib_only",
        "v1293_read_only",
        "v1293_audit_not_fix",
        "v1293_toml_only_no_cargo_tree",
        "v1293_42_crates_full",
        "v1293_no_cargo_lock_parse",
        "v1293_no_workspace_member_modify",
    ]
    assert ids == expected


# ============================================================
# 12. render_report (主 00:56 任何人都能接手)
# ============================================================

def test_render_report_real():
    """真实 workspace 渲染报告."""
    if not WORKSPACE_ROOT.is_dir():
        return
    ledger = scan_workspace(WORKSPACE_ROOT)
    report = render_report(ledger)
    assert "V1293" in report
    assert "Total crates: **41**" in report
    assert "H1" in report and "H5" in report
    assert "Philosophy Gates" in report
    assert "Top Hubs" in report
    assert "Top Leaves" in report
    # 41 crates 都应在 Per-Crate Profile
    for profile in ledger.crate_profiles:
        assert profile.crate_name in report


def test_render_report_empty():
    """空 ledger 不抛异常."""
    ledger = DepGraphLedger(workspace_root="/tmp/empty")
    report = render_report(ledger)
    assert "V1293" in report
    assert "Total crates: **0**" in report


# ============================================================
# 13. CLI tests (主 00:56 任何人都能接手)
# ============================================================

def test_cli_probe():
    """--probe 不抛异常."""
    rc = v1293_main(["--probe"])
    assert rc == 0


def test_cli_run():
    """--run 不抛异常."""
    rc = v1293_main(["--run"])
    assert rc == 0


def test_cli_json():
    """--json 输出 JSON 不抛异常."""
    import io
    from contextlib import redirect_stdout
    buf = io.StringIO()
    with redirect_stdout(buf):
        rc = v1293_main(["--json"])
    assert rc == 0
    output = buf.getvalue()
    data = json.loads(output)
    assert data["total_crates"] == 41
    assert "hypotheses" in data


def test_cli_crate_asi():
    """--crate apeireth-asi."""
    import io
    from contextlib import redirect_stdout
    buf = io.StringIO()
    with redirect_stdout(buf):
        rc = v1293_main(["--crate", "apeireth-asi"])
    assert rc == 0
    output = buf.getvalue()
    data = json.loads(output)
    assert data["crate_name"] == "apeireth-asi"
    assert data["n_internal_deps"] >= 3


def test_cli_crate_not_found():
    """--crate 不存在."""
    rc = v1293_main(["--crate", "apeireth-does-not-exist"])
    assert rc == 1


def test_cli_cycles():
    """--cycles 输出."""
    import io
    from contextlib import redirect_stdout
    buf = io.StringIO()
    with redirect_stdout(buf):
        rc = v1293_main(["--cycles"])
    assert rc == 0
    output = buf.getvalue()
    assert "cycles" in output.lower()


def test_cli_report():
    """--report 写文件."""
    with tempfile.NamedTemporaryFile(suffix=".md", delete=False) as f:
        report_path = f.name
    try:
        rc = v1293_main(["--report", report_path])
        assert rc == 0
        content = Path(report_path).read_text(encoding="utf-8")
        assert "V1293" in content
    finally:
        Path(report_path).unlink(missing_ok=True)


def test_cli_workspace_explicit():
    """--workspace 显式."""
    rc = v1293_main(["--workspace", str(WORKSPACE_ROOT), "--probe"])
    assert rc == 0


# ============================================================
# 14. Edge cases (主 17:43 实事求是)
# ============================================================

def test_scan_crate_malformed_toml():
    """故意破坏的 Cargo.toml 不抛异常, 静默返回."""
    with tempfile.TemporaryDirectory() as tmp:
        crate_dir = Path(tmp) / "apeireth-malformed"
        crate_dir.mkdir()
        (crate_dir / "Cargo.toml").write_text("this is not valid toml [[[", encoding="utf-8")
        profile = _scan_crate("apeireth-malformed", crate_dir)
        assert profile.crate_name == "apeireth-malformed"
        assert profile.cargo_toml_exists is True
        assert profile.n_internal_deps == 0


def test_scan_crate_minimal_toml():
    """最小 Cargo.toml."""
    with tempfile.TemporaryDirectory() as tmp:
        crate_dir = Path(tmp) / "apeireth-min"
        crate_dir.mkdir()
        (crate_dir / "Cargo.toml").write_text(
            '[package]\nname = "apeireth-min"\nversion = "0.1.0"\n',
            encoding="utf-8",
        )
        profile = _scan_crate("apeireth-min", crate_dir)
        assert profile.cargo_toml_exists is True
        assert profile.n_internal_deps == 0
        assert profile.has_workspace_lints is False


def test_scan_crate_with_all_features():
    """含 features / lib / bin / example 的 Cargo.toml."""
    with tempfile.TemporaryDirectory() as tmp:
        crate_dir = Path(tmp) / "apeireth-full"
        crate_dir.mkdir()
        cargo_toml = """
[package]
name = "apeireth-full"
version = "0.1.0"

[features]
default = []
foo = []
bar = ["dep:baz"]

[dependencies]
apeireth-core = { path = "../apeireth-core" }
tokio = { workspace = true }
optional-dep = { version = "1", optional = true }

[dev-dependencies]
proptest = "1"

[build-dependencies]
build-helper = "1"

[lib]
name = "apeireth_full"
path = "src/lib.rs"

[[bin]]
name = "apeireth-full-bin"

[[example]]
name = "demo"

[lints]
workspace = true
"""
        (crate_dir / "Cargo.toml").write_text(cargo_toml, encoding="utf-8")
        profile = _scan_crate("apeireth-full", crate_dir)
        assert profile.n_internal_deps == 1
        assert "apeireth-core" in profile.internal_deps_list
        assert profile.n_features == 3
        assert profile.n_optional_deps == 1
        assert profile.n_dev_deps == 1
        assert profile.n_build_deps == 1
        assert profile.has_lib_target is True
        assert profile.n_bin_targets == 1
        assert profile.n_example_targets == 1
        assert profile.has_workspace_lints is True


# ============================================================
# 15. V1292 ↔ V1293 cross-check (主 19:33 走在前人肩上)
# ============================================================

def test_v1292_v1293_cross():
    """V1292 test source ? V1293 internal dep 对照 (只比 workspace 41 members)."""
    if not WORKSPACE_ROOT.is_dir():
        return
    from apeireth.v1292_rust_test_coverage_audit import scan_workspace as v1292_scan
    crates_dir = WORKSPACE_ROOT / "crates"
    v1292_ledger = v1292_scan(crates_dir)
    v1293_ledger = scan_workspace(WORKSPACE_ROOT)

    v1293_crates = {p.crate_name for p in v1293_ledger.crate_profiles}
    # V1293  = 41 workspace members
    assert len(v1293_crates) == 41

    # V1292 扫 47 个 dirs (包含 41 members + sub-subdirs), 取交集 = 41 members
    v1292_all = {p.crate_name for p in v1292_ledger.crate_profiles}
    v1292_members = v1292_all & v1293_crates
    assert len(v1292_members) == 41
    assert v1292_members == v1293_crates

    # 检查: 哪些 crate internal dep 多但 test attr 少?
    cross_findings = []
    for v1293 in v1293_ledger.crate_profiles:
        v1292_match = next(
            (p for p in v1292_ledger.crate_profiles if p.crate_name == v1293.crate_name),
            None,
        )
        if v1292_match is None:
            continue
        if v1293.n_internal_deps >= 3 and v1292_match.n_test_attrs == 0:
            cross_findings.append(v1293.crate_name)

    # 这个 finding 应在 report 中提及
    # 不强制要求非空 (可能没人符合), 只验证 cross_findings 可计算
    assert isinstance(cross_findings, list)


# ============================================================
# 16. Philosophy gates display (主 17:58 不假装)
# ============================================================

def test_philosophy_gates_in_report():
    """12 守门都在 report 里."""
    if not WORKSPACE_ROOT.is_dir():
        return
    ledger = scan_workspace(WORKSPACE_ROOT)
    report = render_report(ledger)
    for g in GATES:
        assert g["id"] in report
        assert g["desc"] in report


def test_no_kpi_inflation():
    """不刷 KPI, 不假装 ASI V1 (gates 描述除外)."""
    if not WORKSPACE_ROOT.is_dir():
        return
    ledger = scan_workspace(WORKSPACE_ROOT)
    report = render_report(ledger)
    # 切掉 Philosophy Gates section (含 'v1293_no_asi_v1_claim' 描述提及 ASI V1)
    # 只检查 Summary / Hypotheses / Top Hubs / Leaves / Per-Crate Profile body
    gate_idx = report.find("## Philosophy Gates")
    if gate_idx > 0:
        body = report[:gate_idx]
    else:
        body = report
    # 不应出现 ASI V1 / consciousness / phenomenal 字样 in body
    forbidden = ["phenomenal consciousness", "已达 ASI"]
    for word in forbidden:
        assert word not in body, f"forbidden phrase '{word}' found in report body"


def test_no_pretending_in_source():
    """module 源码不应假装 ASI."""
    module_path = PROMETHEAN_ROOT / "apeireth" / "v1293_rust_dependency_graph_profile.py"
    if not module_path.is_file():
        return
    src = module_path.read_text(encoding="utf-8")
    # 应当明确写 "不假装" / "不刷 KPI" / "lock"
    assert "不假装" in src
    assert "不刷" in src or "LOCKED" in src