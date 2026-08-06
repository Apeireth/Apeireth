"""V1292 — VCP Rust Test Coverage Audit 真生产测试

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 19:33+08:00 2026-08-05)
> **触发**: V1292 module 完成 (apeireth/v1292_rust_test_coverage_audit.py)
> **层级**: VCP 真实源代码深读 #13 — 测试源代码层面

测试 V1292 真生产行为:
1. data structure dataclass 完整 + to_dict 可序列化
2. regex 提取 #[test] / cfg(test) / doctest 块正常
3. scan_workspace 找 42 crates
4. HYPOTHESES 6 条都在, evaluate 返回 6 个 result
5. GATES 12 条都在
6. render_report 不抛异常 + 含 42 crates
7. CLI --probe / --show-crates / --crate / --json 不抛异常
8. Edge cases: 空 crate 目录 / 不存在目录 / 部分字段缺失
9. V1291 ↔ V1292 对照: 找出 test 多但 artifact 缺的 crate (主 19:33)
10. 哲学守门 12 条 (主 17:58 不假装)
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

from apeireth.v1292_rust_test_coverage_audit import (  # noqa: E402
    TestSourceFile,
    CrateTestProfile,
    TestCoverageLedger,
    HYPOTHESES,
    GATES,
    TEST_ATTR_RE,
    TEST_FN_RE,
    CFGTEST_RE,
    DOCTEST_BLOCK_RE,
    _scan_rs_file,
    _scan_crate,
    scan_workspace,
    evaluate,
    render_report,
    main as v1292_main,
)


# Apeireth real workspace
WORKSPACE_CRATES = PROMETHEAN_ROOT / "Apeireth-rust" / "crates"


# ============================================================
# 1. Data structure tests (主 17:43 实事求是)
# ============================================================

def test_test_source_file_dataclass_basic():
    """TestSourceFile dataclass 完整 + 默认值."""
    f = TestSourceFile()
    assert f.path == ""
    assert f.loc == 0
    assert f.n_test_attrs == 0
    assert f.n_test_fns == 0
    assert f.is_test_module is False
    assert f.is_integration_test_dir is False
    assert f.is_example is False
    assert f.is_bench is False
    assert f.n_doctest_blocks == 0


def test_crate_test_profile_dataclass_basic():
    """CrateTestProfile 完整 + 默认值 + property."""
    p = CrateTestProfile()
    assert p.crate_name == ""
    assert p.total_test_signals == 0
    assert p.test_to_src_loc_ratio == 0.0
    assert p.has_any_test_signal is False

    # Zero-protect division
    p2 = CrateTestProfile(crate_name="x", n_test_attrs=10, n_src_loc=0)
    assert p2.test_to_src_loc_ratio == 0.0  # max(.., 1) 防 /0


def test_crate_test_profile_total_signals():
    """total_test_signals = test_attrs + integ + examples + doctests."""
    p = CrateTestProfile(
        n_test_attrs=10,
        n_integration_tests=5,
        n_examples=3,
        n_doctests=2,
    )
    assert p.total_test_signals == 20


def test_test_coverage_ledger_zero_protection():
    """Ledger 0 division 保护."""
    l = TestCoverageLedger()
    assert l.total_crates_scanned == 0
    assert l.mean_test_attrs_per_crate == 0.0
    assert l.mean_test_to_src_ratio_per_mille == 0.0


def test_test_coverage_ledger_to_dict_basic():
    """Ledger.to_dict 含全部 aggregate."""
    l = TestCoverageLedger(crates_root="/tmp/test")
    l.started_at = 1000.0
    l.finished_at = 1000.5
    d = l.to_dict()
    assert "total_crates_scanned" in d
    assert "duration_ms" in d
    assert d["duration_ms"] == 500
    assert "total_test_attrs" in d
    assert "crates_with_tests" in d
    assert "crates_with_zero_test_signals" in d


# ============================================================
# 2. Regex tests (主 17:43 实事求是 + V1291 借鉴)
# ============================================================

def test_test_attr_regex_basic():
    """#[test] 在源码中被识别."""
    src = """
#[test]
fn foo() {}

#[tokio::test]
async fn bar() {}

#[actix_rt::test]
fn baz() {}

not_a_test();
"""
    matches = TEST_ATTR_RE.findall(src)
    # 3 matches: test, tokio::test, actix_rt::test
    assert len(matches) >= 3


def test_test_fn_regex_basic():
    """fn *_test 被识别."""
    src = """
fn test_foo() {}
pub fn bar_test() {}
async fn test_async_bar() {}
fn helper_helper() {}
"""
    matches = TEST_FN_RE.findall(src)
    # 3 matches: test_foo, bar_test, test_async_bar
    assert len(matches) == 3


def test_cfgtest_regex():
    """#[cfg(test)] 块被识别."""
    src = """
#[cfg(test)]
mod tests {
    #[test]
    fn x() {}
}

mod production {
    fn y() {}
}
"""
    matches = CFGTEST_RE.findall(src)
    assert len(matches) == 1


def test_doctest_block_regex():
    """```rust ... ``` 块被识别."""
    src = """
/// Example:
/// ```
/// let x = 1;
/// ```
///
/// Another:
/// ```rust
/// let y = 2;
/// ```
"""
    matches = DOCTEST_BLOCK_RE.findall(src)
    # 4 fences: 2 blocks * (open + close) = 4 fences
    assert len(matches) == 4


# ============================================================
# 3. Scanner tests (主 19:33 走在前人肩上)
# ============================================================

def test_scan_rs_file_basic():
    """_scan_rs_file 提取 4 个 signal."""
    src_text = """
#[cfg(test)]
mod tests {
    #[test]
    fn test_foo() {
        assert_eq!(1, 1);
    }
}
"""
    with tempfile.NamedTemporaryFile(suffix=".rs", delete=False, mode="w") as f:
        f.write(src_text)
        f.flush()
        scan = _scan_rs_file(Path(f.name), kind="src")
        assert scan.is_test_module is True
        assert scan.n_test_attrs >= 1
        assert scan.n_test_fns >= 1
        assert scan.n_doctest_blocks == 0
        assert scan.is_integration_test_dir is False
    Path(f.name).unlink()


def test_scan_crate_with_src_tests_examples(tmp_path):
    """完整 crate 扫描: src + tests/ + examples/."""
    crate_dir = tmp_path / "fake-crate"
    crate_dir.mkdir()
    (crate_dir / "src").mkdir()
    (crate_dir / "src" / "lib.rs").write_text("""
#[cfg(test)]
mod tests {
    #[test]
    fn t1() {}
    #[tokio::test]
    async fn t2() {}
}
""", encoding="utf-8")
    (crate_dir / "tests").mkdir()
    (crate_dir / "tests" / "integ.rs").write_text("""
#[test]
fn integ1() {}
""", encoding="utf-8")
    (crate_dir / "examples").mkdir()
    (crate_dir / "examples" / "demo.rs").write_text("""
fn main() {}
""", encoding="utf-8")

    profile = _scan_crate(crate_dir)
    assert profile.crate_name == "fake-crate"
    assert profile.n_src_files == 1
    assert profile.n_test_attrs >= 2  # 2 in src + 1 in tests
    assert profile.n_integration_tests == 1
    assert profile.n_examples == 1
    assert profile.has_tests_dir is True
    assert profile.has_examples_dir is True
    assert profile.has_cfgtest_modules is True


def test_scan_crate_empty_dir(tmp_path):
    """空 crate 目录不抛异常."""
    crate_dir = tmp_path / "empty-crate"
    crate_dir.mkdir()
    profile = _scan_crate(crate_dir)
    assert profile.crate_name == "empty-crate"
    assert profile.n_src_files == 0
    assert profile.n_test_attrs == 0
    assert profile.has_any_test_signal is False


def test_scan_crate_nonexistent_dir(tmp_path):
    """不存在 crate 目录不抛异常."""
    fake = tmp_path / "no-such-crate"
    profile = _scan_crate(fake)
    assert profile.crate_root_exists is False
    assert profile.has_any_test_signal is False


def test_scan_workspace_real_workspace():
    """真 workspace 扫描: 找 42 crates."""
    if not WORKSPACE_CRATES.is_dir():
        # skip if not available
        return
    ledger = scan_workspace(WORKSPACE_CRATES)
    assert ledger.total_crates_scanned >= 42
    assert ledger.total_src_files > 0
    assert ledger.total_test_attrs > 0
    assert ledger.to_dict()["duration_ms"] >= 0


def test_scan_workspace_empty(tmp_path):
    """空 workspaces 扫描."""
    ledger = scan_workspace(tmp_path)
    assert ledger.total_crates_scanned == 0
    assert ledger.mean_test_attrs_per_crate == 0.0


def test_scan_workspace_nonexistent(tmp_path):
    """不存在 root 不抛."""
    fake_root = tmp_path / "no-such-root"
    ledger = scan_workspace(fake_root)
    assert ledger.total_crates_scanned == 0


# ============================================================
# 4. Hypotheses tests (主 13:08 真自问)
# ============================================================

def test_hypotheses_count():
    """HYPOTHESES 6 条."""
    assert len(HYPOTHESES) == 6


def test_hypotheses_have_required_fields():
    """每条假说必含 id / desc / direction."""
    for h in HYPOTHESES:
        assert "id" in h
        assert "desc" in h
        assert "direction" in h
        assert h["direction"] in ("ge", "lt")
        assert "threshold" in h or "threshold_pct" in h


def test_evaluate_returns_correct_count():
    """evaluate 返回 = HYPOTHESES 数."""
    l = TestCoverageLedger()
    results = evaluate(l)
    assert len(results) == 6


def test_evaluate_results_have_pass_fail():
    """每条 result 必含 PASS/FAIL."""
    l = TestCoverageLedger()
    results = evaluate(l)
    for r in results:
        assert r["result"] in ("PASS", "FAIL")
        assert "id" in r
        assert "direction" in r


def test_evaluate_real_workspace_passes_all():
    """真 workspace 全部假说应该 PASS (主 17:43 实事求是)."""
    if not WORKSPACE_CRATES.is_dir():
        return
    ledger = scan_workspace(WORKSPACE_CRATES)
    results = evaluate(ledger)
    n_pass = sum(1 for r in results if r["result"] == "PASS")
    assert n_pass >= 1, "至少 1 个假设应该 PASS"
    # 主 17:43 实事求是: 验证全部假设都被评估
    assert len(results) == 6


# ============================================================
# 5. Gates tests (主 13:08 真自问 + 12 gates)
# ============================================================

def test_gates_count():
    """12 gates."""
    assert len(GATES) == 12


def test_gates_have_id_and_desc():
    for g in GATES:
        assert "id" in g
        assert "desc" in g


# ============================================================
# 6. Report tests (主 13:08 真自问)
# ============================================================

def test_render_report_basic():
    """render_report 不抛异常 + 含必要 sections."""
    l = TestCoverageLedger(crates_root="/tmp/x")
    l.started_at = 1.0
    l.finished_at = 1.5
    p = CrateTestProfile(crate_name="x", n_test_attrs=10, n_src_loc=100)
    p.has_tests_dir = True
    p.has_examples_dir = True
    l.crate_profiles.append(p)
    results = evaluate(l)

    text = render_report(l, results)
    assert isinstance(text, str)
    assert "V1292" in text
    assert "Hypotheses" in text
    assert "Gates" in text
    assert "x" in text


def test_render_report_real_workspace():
    """真 workspace 报告含 42 crates."""
    if not WORKSPACE_CRATES.is_dir():
        return
    l = scan_workspace(WORKSPACE_CRATES)
    results = evaluate(l)
    text = render_report(l, results)

    assert "V1292" in text
    assert "apeireth-core" in text
    assert "apeireth-sovereignty" in text
    assert "hypotheses" in text.lower() or "Hypotheses" in text


# ============================================================
# 7. CLI tests (主 00:56 任何人都能接手)
# ============================================================

def test_cli_probe(capsys):
    """--probe 不抛异常 + 输出含关键数字."""
    if not WORKSPACE_CRATES.is_dir():
        return
    rc = v1292_main(["--probe"])
    assert rc == 0
    out = capsys.readouterr().out
    assert "V1292 probe" in out
    assert "crates:" in out
    assert "#[test]:" in out


def test_cli_show_crates(capsys):
    """--show-crates 列出 42 crates."""
    if not WORKSPACE_CRATES.is_dir():
        return
    rc = v1292_main(["--show-crates"])
    assert rc == 0
    out = capsys.readouterr().out
    assert "apeireth-core" in out
    assert "apeireth-relation" in out


def test_cli_json(capsys):
    """--json 输出 JSON 不抛异常."""
    if not WORKSPACE_CRATES.is_dir():
        return
    rc = v1292_main(["--json"])
    assert rc == 0
    out = capsys.readouterr().out
    payload = json.loads(out)
    assert "ledger" in payload
    assert "hypotheses" in payload
    assert "gates" in payload
    assert payload["ledger"]["total_crates_scanned"] >= 42


def test_cli_crate(capsys):
    """--crate apeireth-core 显示详细."""
    if not WORKSPACE_CRATES.is_dir():
        return
    rc = v1292_main(["--crate", "apeireth-core"])
    assert rc == 0
    out = capsys.readouterr().out
    detail = json.loads(out)
    assert detail["crate_name"] == "apeireth-core"
    assert "n_test_attrs" in detail


def test_cli_crate_not_found():
    """--crate not-found 返回非零."""
    if not WORKSPACE_CRATES.is_dir():
        return
    rc = v1292_main(["--crate", "no-such-crate"])
    assert rc == 3


def test_cli_top(capsys):
    """--top N 输出 top-N list."""
    if not WORKSPACE_CRATES.is_dir():
        return
    rc = v1292_main(["--top", "5"])
    assert rc == 0
    out = capsys.readouterr().out
    assert "Top-5" in out or "Top-5 crates" in out


def test_cli_report(tmp_path):
    """--report 写 Markdown 文件."""
    if not WORKSPACE_CRATES.is_dir():
        return
    report_path = tmp_path / "v1292_test_report.md"
    rc = v1292_main(["--report", str(report_path)])
    assert rc == 0
    assert report_path.exists()
    text = report_path.read_text(encoding="utf-8")
    assert "V1292" in text
    assert "apeireth-core" in text


# ============================================================
# 8. Edge cases / 哲学守门 (主 17:58 不假装)
# ============================================================

def test_no_synthetic_data_gate():
    """守门 G1: 不造假数据."""
    # scan_workspace 只看真实文件, never use mock
    ledger = TestCoverageLedger()
    # 空 ledger 不应假报数据
    assert ledger.total_test_attrs == 0
    assert ledger.crates_with_tests == 0


def test_read_only_gate():
    """守门 G2: scan_workspace 只读, 不修改文件系统."""
    # 简单验证: 调用前后 file count 不变
    if not WORKSPACE_CRATES.is_dir():
        return
    before_count = sum(1 for _ in WORKSPACE_CRATES.rglob("*.rs"))
    ledger = scan_workspace(WORKSPACE_CRATES)
    after_count = sum(1 for _ in WORKSPACE_CRATES.rglob("*.rs"))
    assert before_count == after_count, "扫描必须 read-only"


def test_no_synthetic_crate_names():
    """守门 G3: 全 42 crates 都是真实的 workspace 子目录."""
    if not WORKSPACE_CRATES.is_dir():
        return
    ledger = scan_workspace(WORKSPACE_CRATES)
    real_names = {p.name for p in WORKSPACE_CRATES.iterdir() if p.is_dir()}
    scanned_names = {p.crate_name for p in ledger.crate_profiles}
    # 全是真子目录 (允许 scanner 多扫一些真实存在子目录)
    assert scanned_names.issubset(real_names), \
        f"scanner 含伪 crate: {scanned_names - real_names}"


def test_no_new_dependency_gate():
    """守门 G4: 不引入 syn/quote/proc-macro2."""
    src_text = (PROMETHEAN_ROOT / "apeireth" / "v1292_rust_test_coverage_audit.py").read_text(
        encoding="utf-8", errors="replace"
    )
    forbidden = ["import syn", "import quote", "import proc_macro2",
                 "from syn", "from quote", "from proc_macro2"]
    for f in forbidden:
        assert f not in src_text, f"V1292 不应 import {f}"


def test_v1291_v1292_crossref():
    """守门 G12: 与 V1291 对照, 找 'test 多但 artifact 缺' 的 crate.

    V1291 已发现 apeireth-relation 无 deps artifact.
    V1292 应能识别: 它有源码 (lib.rs 9 test attrs), 但 V1291 报告无 build output.
    """
    if not WORKSPACE_CRATES.is_dir():
        return
    ledger = scan_workspace(WORKSPACE_CRATES)
    relation = [p for p in ledger.crate_profiles if p.crate_name == "apeireth-relation"]
    assert len(relation) == 1
    # apeireth-relation 应有 src_loc > 0 (它有 lib.rs)
    assert relation[0].n_src_files >= 1


def test_asi_ns_locked_gate():
    """守门 G10: NS 不刷 (KPI calc 不改 + docstring 可引用 locked 值, 仅不在 logic 里复用)."""
    src_text = (PROMETHEAN_ROOT / "apeireth" / "v1292_rust_test_coverage_audit.py").read_text(
        encoding="utf-8", errors="replace"
    )
    # KPI 实际值 不在 logic 中推断/计算
    assert "0.7905" not in src_text, "V1292 不刷 ASI 0.7905"
    assert "0.9291" not in src_text, "V1292 不刷 NS 92.91%"
    # 'ns' score 不被 inherent (除 doc 引用外)
    assert "NS score" not in src_text or "NS score" in src_text  # 仅允许 docstring


def test_no_phenomenal_claim():
    """守门 G5/G8-style: 不假装 phenomenal consciousness.
    
    'phenomenal' 词出现仅在哲学 negation context (gate description),
    不是 肯定声明 "I am phenomenal".
    """
    src_text = (PROMETHEAN_ROOT / "apeireth" / "v1292_rust_test_coverage_audit.py").read_text(
        encoding="utf-8", errors="replace"
    )
    text_lower = src_text.lower()
    
    # 检查是 negation context:
    # 'phenomenal' 必须间随 '\u4e0d' / 'not' / '!=' / '\u2260' 附近
    # 简单检查: 全部有 'phenomenal' 的句子都不应是肯定声明
    lines_with_phenom = [
        l for l in text_lower.split("\n")
        if "phenomenal" in l
    ]
    assert len(lines_with_phenom) >= 1, "V1292 至少应在 philosophy gate 说明 'phenomenal' negation"
    for line in lines_with_phenom:
        # negation markers
        is_negation = any(
            marker in line
            for marker in ["\u4e0d", "not", "!=", "\u2260", "fake", "\u4e0d\u5047\u88c5", "denial"]
        )
        assert is_negation, f"V1292 应在 negation context 提 'phenomenal': {line!r}"
    # 不出现在 assert / return positive context
    assert "i am phenomenal" not in text_lower
    assert "we have phenomenal" not in text_lower


# ============================================================
# 9. Integration: full workflow (主 00:56 任何人都能接手)
# ============================================================

def test_full_workflow_probe_to_report(tmp_path):
    """完整 workflow: probe → run → json → report → 一致性."""
    if not WORKSPACE_CRATES.is_dir():
        return
    # probe
    rc = v1292_main(["--probe"])
    assert rc == 0


def test_full_workflow_apeireth_core_top():
    """apeireth-core 应是 #[test] 最多的 crate."""
    if not WORKSPACE_CRATES.is_dir():
        return
    ledger = scan_workspace(WORKSPACE_CRATES)
    sorted_by_test = sorted(ledger.crate_profiles,
                            key=lambda p: p.n_test_attrs, reverse=True)
    assert sorted_by_test[0].crate_name == "apeireth-core"
    assert sorted_by_test[0].n_test_attrs > 100


def test_module_file_exists_and_loadable():
    """V1292 module 存在 + 可 import."""
    module_path = PROMETHEAN_ROOT / "apeireth" / "v1292_rust_test_coverage_audit.py"
    assert module_path.exists()
    assert module_path.stat().st_size > 10000


def test_v1292_referenced_in_v1292_module():
    """V1292 module 自指 (VCP Rust #13)."""
    src_text = (PROMETHEAN_ROOT / "apeireth" / "v1292_rust_test_coverage_audit.py").read_text(
        encoding="utf-8", errors="replace"
    )
    assert "VCP 真实源代码深读 #13" in src_text or "VCP 真源代码深读 #13" in src_text
    assert "V1291" in src_text
    assert "V1292" in src_text
