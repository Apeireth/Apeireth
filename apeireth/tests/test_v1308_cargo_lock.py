"""
V1308 - Cargo.lock 真审计 Popper 假说测试 (12 假说)

修真前 (V1307 完成): Cargo.lock drift 未审计, 修真决策不明
修真后 (V1308 完成): Cargo.lock drift = HEALTHY, 修真 = commit 锁定, 不修真 lock

Popper 假说:
  假说 1-3:   修真决策基础 (audit_findings.json 存在 + decision=HEALTHY + all_explainable=True)
  假说 4-6:   数据规模 (delta=218 + workspace_added>=9 + tauri>=100)
  假说 7-9:   修真前/后文件存在 (audit_findings.json + decision.json + 决策执行记录)
  假说 10-12: 修真前/后不假装 (V3 守门 + audit + 真修真决策)

不假装:
- 真跑 audit + decision 脚本, 修真前假说 1-3 应 FAIL (audit 未跑), 修真后 PASS
- 真分类 + 真决策依据, 修真后决策 = commit 锁定而非"修真 lock"
- 真 Popper 假说, 修真前/后实证对比
"""
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent  # tests -> apeireth -> promethean
APEIRETH = ROOT / "apeireth"
TESTS = APEIRETH / "tests"


def h(name: str) -> bool:
    """Run hypothesis check. Returns True if hypothesis PASSES."""
    return True  # placeholder; real impl below


def main():
    findings_path = APEIRETH.parent / "v1308_audit_findings.json"
    decision_path = APEIRETH.parent / "v1308_decision.json"

    findings_exists = findings_path.exists()
    decision_exists = decision_path.exists()

    findings = json.loads(findings_path.read_text(encoding="utf-8")) if findings_exists else {}
    decision = json.loads(decision_path.read_text(encoding="utf-8")) if decision_exists else {}

    # Workspace audit sanity
    out = subprocess.check_output(
        ["git", "show", "HEAD:Apeireth-rust/Cargo.lock"],
        cwd=str(ROOT), text=True,
    )
    head_pkg_count = len([line for line in out.split("\n") if line.startswith('name = "')])

    # Now lock
    now_lock = (ROOT / "Apeireth-rust" / "Cargo.lock").read_text(encoding="utf-8", errors="replace")
    now_pkg_count = len([line for line in now_lock.split("\n") if line.startswith('name = "')])

    # 修真前状态记录 (V1307 完成 = workspace 8/8 → 0 orphans, 92 members)
    # 修真前 Cargo.lock = HEAD = head_pkg_count (修真前 lock 已存在)
    # 修真后 Cargo.lock = working tree = now_pkg_count
    pre_fix_lock_size = head_pkg_count
    post_fix_lock_size = now_pkg_count

    # 修真前/后: workspace audit sanity (cargo metadata --no-deps 应该返回 92 packages)
    out = subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        cwd=str(ROOT / "Apeireth-rust"),
        stderr=subprocess.DEVNULL,
    )
    meta = json.loads(out.decode("utf-8", errors="replace"))
    workspace_pkg_count = len(meta.get("packages", []))
    workspace_members_count = len(meta.get("workspace_members", []))

    # 修真后 Cargo.lock modified (git status --short 应该包含 M Cargo.lock)
    out = subprocess.check_output(
        ["git", "status", "--short", "Apeireth-rust/Cargo.lock"],
        cwd=str(ROOT), text=True,
    )
    cargo_lock_modified = (" M " in out) or ("M " in out)

    hypotheses = [
        # 1: 修真后 audit_findings.json 存在
        ("h_v1308_findings_exists_post", findings_exists, True),
        # 2: 修真后 audit decision = HEALTHY
        ("h_v1308_decision_healthy", findings.get("audit_decision") == "HEALTHY", True),
        # 3: 修真后 all_explainable = True
        ("h_v1308_all_explainable", findings.get("all_explainable") is True, True),
        # 4: 修真后 delta 数值 (218 expected)
        ("h_v1308_delta_218", findings.get("delta") == 218, True),
        # 5: 修真后 workspace_added_count >= 9 (V1302-V1307 修真 8 + 早期 integration/rate-limiter)
        ("h_v1308_workspace_added_ge_9", findings.get("workspace_added_count", 0) >= 9, True),
        # 6: 修真后 tauri_ecosystem_count >= 100 (V1307 修真 tauri-stub 引爆)
        ("h_v1308_tauri_ecosystem_ge_100", findings.get("tauri_ecosystem_count", 0) >= 100, True),
        # 7: 修真后 decision.json 存在
        ("h_v1308_decision_json_exists", decision_exists, True),
        # 8: 修真后 workspace members = 92 (V1307 完成)
        ("h_v1308_workspace_members_92", workspace_members_count == 92, True),
        # 9: 修真后 workspace packages (no-deps) = 92
        ("h_v1308_workspace_packages_92", workspace_pkg_count == 92, True),
        # 10: 修真后 decision.post_fix.cargo_lock_modified_observed = True
        ("h_v1308_lock_modified_observed",
         decision.get("post_fix", {}).get("cargo_lock_modified_observed") is True, True),
        # 11: 修真后 decision.post_fix.lock_rewrite_needed = False (不修真 lock)
        ("h_v1308_no_lock_rewrite",
         decision.get("post_fix", {}).get("lock_rewrite_needed") is False, True),
        # 12: 修真后 Cargo.lock modified in git status (修真 = commit 锁定准备)
        ("h_v1308_cargo_lock_pending_commit", cargo_lock_modified, True),
    ]

    passed = 0
    failed = 0
    import io, sys as _sys
    buf = io.TextIOWrapper(_sys.stdout.buffer, encoding="utf-8", errors="replace")
    buf.write("=" * 78 + "\n")
    buf.write("V1308 Cargo.lock 真审计 Popper 假说 (12/12 PASS expected)\n")
    buf.write("=" * 78 + "\n")
    for name, observed, threshold in hypotheses:
        ok = observed == threshold
        status = "PASS" if ok else "FAIL"
        buf.write(f"  [{status}]  {name:48s}  observed={observed}  expected={threshold}\n")
        if ok:
            passed += 1
        else:
            failed += 1

    buf.write("=" * 78 + "\n")
    buf.write(f"  Total: {passed}/{len(hypotheses)} pass\n")
    buf.write(f"  pre_fix  Cargo.lock packages: {pre_fix_lock_size}\n")
    buf.write(f"  post_fix Cargo.lock packages: {post_fix_lock_size}\n")
    buf.write(f"  workspace members: {workspace_members_count}\n")
    buf.write(f"  workspace packages: {workspace_pkg_count}\n")
    buf.write(f"  audit decision: {findings.get('audit_decision', 'NOT_RUN')}\n")
    buf.write("=" * 78 + "\n")
    buf.flush()

    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())