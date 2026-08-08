"""
V1309 - Test Coverage 真审计 (Post-V1308 Cargo.lock audit)

修真目的: 92 members 中, 哪些有 tests (unit/integration) / 哪些 0 tests / 修真必要
修真结论: 数据驱动分类 (P0-P3), 修真仅当必要 (P0 critical path)
                不假装:
- 真 glob 92 crates/apeireth-* (非 cargo metadata 推测)
- 真 #[test] 计数 (regex grep src/**/*.rs)
- 真 tests/ 目录存在 (glob tests/*.rs)
- 真分类 (P0=0 tests, P1=unit only, P2=unit+integration, P3=完整+coverage)
- 真修真决策: 仅修真 P0 critical path (core/memory/asi/pipeline/cli's audit-dock)
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RUST_CRATES = ROOT / "Apeireth-rust" / "crates"


def count_test_attrs(crate_path: Path) -> dict:
    """Count #[test] / #[tokio::test] / #[async_test] in src/**/*.rs."""
    src_dir = crate_path / "src"
    counts = {"test": 0, "tokio_test": 0, "async_test": 0, "files_scanned": 0, "total_loc": 0}
    if not src_dir.exists():
        return counts
    for rs in src_dir.rglob("*.rs"):
        text = rs.read_text(encoding="utf-8", errors="replace")
        counts["test"] += len(re.findall(r"#\s*\[\s*test\s*\]", text))
        counts["tokio_test"] += len(re.findall(r"#\s*\[\s*tokio::test\s*\]", text))
        counts["async_test"] += len(re.findall(r"#\s*\[\s*async(?:_)?test\s*\]", text))
        counts["files_scanned"] += 1
        counts["total_loc"] += sum(1 for _ in text.splitlines() if _.strip())
    return counts


def has_integration_tests(crate_path: Path) -> tuple[bool, int]:
    """Check tests/ dir for *.rs files."""
    tests_dir = crate_path / "tests"
    if not tests_dir.is_dir():
        return False, 0
    count = sum(1 for _ in tests_dir.rglob("*.rs"))
    return count > 0, count


def has_examples(crate_path: Path) -> tuple[bool, int]:
    examples_dir = crate_path / "examples"
    if not examples_dir.is_dir():
        return False, 0
    return True, sum(1 for _ in examples_dir.rglob("*.rs"))


def has_benches(crate_path: Path) -> tuple[bool, int]:
    benches_dir = crate_path / "benches"
    if not benches_dir.is_dir():
        return False, 0
    return True, sum(1 for _ in benches_dir.rglob("*.rs"))


# Critical path crates (P0 - 必须有 tests, 否则修真)
# 修真范围评估 = 影响 ASI pole-star / 安全 / 数据完整性 的核心 crate
P0_CRITICAL = {
    "apeireth-core",        # 核心类型 (Result/Error/Trait)
    "apeireth-memory",      # 记忆系统 (sqlite/sled) - 数据完整性
    "apeireth-asi",         # ASI 哲学 - pole-star
    "apeireth-pipeline",    # LOCKED 路径 - R17 chat 专用
    "apeireth-pipeline-g5", # 通用 5 阶段
    "apeireth-bus",         # 事件总线 - 影响所有 module
    "apeireth-constraint",  # 约束系统 - 安全
    "apeireth-cron",        # 定时任务 - 主线驱动
    "apeireth-skills",      # 技能系统 - ASI 暴露面
    "apeireth-value",       # 价值系统 - alignment
}


def classify(crate_name: str, unit_count: int, has_tests: bool, has_examples: bool) -> str:
    """Return P0/P1/P2/P3 classification."""
    if crate_name in P0_CRITICAL and unit_count == 0 and not has_tests:
        return "P0"  # critical + 0 tests = must fix
    if unit_count == 0 and not has_tests:
        return "P1"  # 0 tests, non-critical (consider adding)
    if not has_tests and not has_examples:
        return "P2"  # unit only, no integration
    return "P3"  # has integration or examples


def main():
    if not RUST_CRATES.is_dir():
        print(f"FAIL: {RUST_CRATES} not found", file=sys.stderr)
        sys.exit(2)

    crates = sorted([p.name for p in RUST_CRATES.iterdir() if p.is_dir() and p.name.startswith("apeireth-")])
    audit_rows = []
    p0, p1, p2, p3 = [], [], [], []

    for cname in crates:
        cpath = RUST_CRATES / cname
        unit = count_test_attrs(cpath)
        has_int, int_count = has_integration_tests(cpath)
        has_ex, ex_count = has_examples(cpath)
        has_b, bench_count = has_benches(cpath)
        cls = classify(cname, unit["test"] + unit["tokio_test"] + unit["async_test"], has_int, has_ex)

        row = {
            "crate": cname,
            "class": cls,
            "unit_tests": unit["test"] + unit["tokio_test"] + unit["async_test"],
            "test_attr_breakdown": unit,
            "has_integration_tests": has_int,
            "integration_test_files": int_count,
            "has_examples": has_ex,
            "example_files": ex_count,
            "has_benches": has_b,
            "bench_files": bench_count,
            "is_p0_critical": cname in P0_CRITICAL,
        }
        audit_rows.append(row)
        if cls == "P0":
            p0.append(cname)
        elif cls == "P1":
            p1.append(cname)
        elif cls == "P2":
            p2.append(cname)
        else:
            p3.append(cname)

    summary = {
        "workspace_root": str(ROOT),
        "crates_root": str(RUST_CRATES),
        "total_crates_scanned": len(crates),
        "class_counts": {"P0": len(p0), "P1": len(p1), "P2": len(p2), "P3": len(p3)},
        "p0_critical_no_tests": p0,
        "p1_no_tests_non_critical": p1,
        "p2_unit_only_no_integration": p2,
        "p3_well_tested": p3,
        "audit_rows": audit_rows,
    }

    out = ROOT / "apeireth" / "v1309_audit_findings.json"
    out.write_text(json.dumps(summary, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"V1309 audit done: {len(crates)} crates")
    print(f"  P0 (critical + 0 tests):  {len(p0)} {p0}")
    print(f"  P1 (0 tests, non-crit):   {len(p1)}")
    print(f"  P2 (unit only, no int):   {len(p2)}")
    print(f"  P3 (well tested):         {len(p3)}")
    print(f"  → {out}")


if __name__ == "__main__":
    main()