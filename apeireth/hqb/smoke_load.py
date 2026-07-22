"""HQB schema 烟测 — 验证 schema 可创建 + 可插/查/删 (R3-DB-01).

设计: in-memory DB, 每表插 1 行, 校验可读 + 可删 + cascade 关系正确.
不进 V1074 / 不写真生产数据. 仅 schema 层验证.

用法: python -m apeireth.hqb.smoke_load
"""

from __future__ import annotations

import sys
import tempfile
from pathlib import Path

# 让 `python apeireth/hqb/smoke_load.py` 也能跑
_PKG = Path(__file__).resolve().parent.parent.parent
if str(_PKG) not in sys.path:
    sys.path.insert(0, str(_PKG))

from apeireth.hqb import HqbStore, SCHEMA_VERSION  # noqa: E402


def _section(title: str) -> None:
    print(f"\n=== {title} ===")


def _ok(msg: str) -> None:
    print(f"  [OK] {msg}")


def _fail(msg: str) -> None:
    print(f"  [FAIL] {msg}")
    raise SystemExit(1)


def test_in_memory_lifecycle() -> None:
    """用例 1: in-memory DB — 插/查/删主链路."""
    _section("TEST 1: in-memory lifecycle")
    s = HqbStore(":memory:")
    _ok(f"created :memory: store, schema_version={s.schema_version()}")
    assert s.schema_version() == SCHEMA_VERSION

    did = s.record_decision(task_id="t-001", decision="allow", score=0.91,
                            philosophy_guard_status="pass", snapshot_score=0.8816)
    _ok(f"insert decision id={did[:8]}...")
    d = s.get_decision(did)
    assert d is not None and d["decision"] == "allow" and d["score"] == 0.91
    _ok(f"read decision task_id={d['task_id']} guard={d['philosophy_guard_status']}")

    gid = s.record_guard(did, guard_type="philosophy", passed=True, reason="v3 guard ok")
    _ok(f"insert guard_event id={gid[:8]}...")
    assert len(s.list_guards_for(did)) == 1
    _ok("list_guards_for returns 1 row")

    delta_id = s.record_delta(did, asiv0_before=0.8816, asiv0_after=0.8861)
    _ok(f"insert asi_delta id={delta_id[:8]}...")
    deltas = s.list_deltas_for(did)
    assert len(deltas) == 1 and abs(deltas[0]["lift_value"] - 0.0045) < 1e-9
    _ok(f"lift_value={deltas[0]['lift_value']:.4f} (computed correctly)")

    parent = s.record_trace(action="harness_edit_apply", rationale="root")
    child = s.record_trace(action="score_compute", rationale="child of root", parent_id=parent)
    _ok(f"trace chain parent={parent[:8]}... child={child[:8]}...")

    deleted = s.delete_decision(did)
    assert deleted == 1
    _ok("delete_decision removed 1 row")
    assert s.get_decision(did) is None
    _ok("decision gone (cascade: guards + deltas auto-cleaned)")
    assert len(s.list_guards_for(did)) == 0 and len(s.list_deltas_for(did)) == 0
    _ok("cascade verified: 0 guards, 0 deltas after delete")
    s.close()


def test_persistent_file(tmpdir: str) -> None:
    """用例 2: 落盘 DB — 跨实例持久化."""
    _section("TEST 2: persistent file lifecycle")
    db = Path(tmpdir) / "hqb_smoke.db"
    s1 = HqbStore(str(db))
    did = s1.record_decision("t-persist", "flag", 0.75, "skip", 0.88)
    s1.close()
    _ok(f"wrote 1 decision to {db.name}, closed.")

    s2 = HqbStore(str(db))
    d = s2.get_decision(did)
    assert d is not None and d["task_id"] == "t-persist"
    _ok(f"reopened store, decision survived (task_id={d['task_id']})")
    s2.close()


def test_idempotent_schema() -> None:
    """用例 3: 重复 init schema 不应破坏."""
    _section("TEST 3: idempotent CREATE TABLE IF NOT EXISTS")
    s = HqbStore(":memory:")
    s._init_schema()  # 再跑一次
    s._init_schema()
    _ok("init_schema x3 succeeded — no duplicate table errors")
    s.close()


def main() -> int:
    print(f"HQB smoke test (R3-DB-01) | schema_version={SCHEMA_VERSION}")
    test_in_memory_lifecycle()
    with tempfile.TemporaryDirectory() as td:
        test_persistent_file(td)
    test_idempotent_schema()
    print("\n=== ALL SMOKE TESTS PASSED ===")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())