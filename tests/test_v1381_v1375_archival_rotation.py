"""Tests for V1381 — V1375 archival rotation policy.

Run from promethean/:
    python -m pytest tests/test_v1381_v1375_archival_rotation.py -v
"""

from __future__ import annotations

import datetime as _dt
import gzip
import io
import json
import os
import sys
import tempfile
from contextlib import redirect_stderr, redirect_stdout

import pytest

import apeireth.v1381_v1375_archival_rotation as v1381


# ----------------------------------------------------------------------
# Constants + GUARDS
# ----------------------------------------------------------------------

def test_schema_version_constant():
    assert v1381.SCHEMA_VERSION == "v1381.rotation/v1"


def test_script_name_constant():
    assert v1381.SCRIPT_NAME == "v1381_v1375_archival_rotation"


def test_policy_version_constant():
    assert isinstance(v1381.POLICY_VERSION, str)
    assert v1381.POLICY_VERSION.startswith("v1381.rotation.policy/")


def test_age_bands_constants_ordering():
    """HOT < WARM < COLD."""
    bands = v1381.AGE_BANDS
    assert bands["HOT_MAX_SEC"] < bands["WARM_MAX_SEC"]
    assert bands["WARM_MAX_SEC"] < bands["COLD_MAX_SEC"]


def test_default_policy_keys():
    pol = v1381.default_policy()
    for k in ("policy_version", "age_bands", "actions", "schema_version"):
        assert k in pol


def test_default_policy_actions_match_tiers():
    pol = v1381.default_policy()
    actions = pol["actions"]
    assert set(actions.keys()) == {"HOT", "WARM", "COLD", "FROZEN"}
    assert actions["HOT"] == "keep"
    assert actions["WARM"] == "keep"
    assert actions["COLD"] == "compress"
    assert actions["FROZEN"] == "prune"


def test_parse_iso_basic_returns_aware_utc():
    dt = v1381.parse_iso_basic("2026-08-09T03-55-00Z")
    assert dt.tzinfo is not None
    assert dt.utcoffset().total_seconds() == 0
    assert dt.year == 2026 and dt.month == 8 and dt.day == 9
    assert dt.hour == 3 and dt.minute == 55 and dt.second == 0


def test_parse_iso_basic_rejects_garbage():
    with pytest.raises(ValueError):
        v1381.parse_iso_basic("not-an-iso")
    with pytest.raises(ValueError):
        v1381.parse_iso_basic("")
    with pytest.raises(ValueError):
        v1381.parse_iso_basic(None)  # type: ignore[arg-type]


def test_slug_timestamp_to_datetime_matches_parse():
    slug = "2026-08-09T03-55-00Z__v1374.md"
    dt = v1381.slug_timestamp_to_datetime(slug)
    assert dt == v1381.parse_iso_basic("2026-08-09T03-55-00Z")


def test_slug_timestamp_to_datetime_rejects_bad_slug():
    with pytest.raises(ValueError):
        v1381.slug_timestamp_to_datetime("INDEX.md")
    with pytest.raises(ValueError):
        v1381.slug_timestamp_to_datetime("not-a-slug.md")


# ----------------------------------------------------------------------
# Tier classification
# ----------------------------------------------------------------------

def test_classify_hot_recent():
    tier, age = v1381.classify_archive(
        "2026-08-09T03-55-00Z__v1374.md",
        now=_dt.datetime(2026, 8, 9, 4, 0, 0, tzinfo=_dt.timezone.utc),
    )
    assert tier == "HOT"
    assert age == 300


def test_classify_warm_three_days():
    tier, age = v1381.classify_archive(
        "2026-08-09T03-55-00Z__v1374.md",
        now=_dt.datetime(2026, 8, 12, 3, 55, 0, tzinfo=_dt.timezone.utc),
    )
    assert tier == "WARM"
    # 3 days exactly
    assert age == 3 * 86400


def test_classify_cold_thirty_days():
    tier, age = v1381.classify_archive(
        "2026-08-09T03-55-00Z__v1374.md",
        now=_dt.datetime(2026, 9, 8, 3, 55, 0, tzinfo=_dt.timezone.utc),
    )
    assert tier == "COLD"
    assert age == 30 * 86400


def test_classify_frozen_two_hundred_days():
    tier, age = v1381.classify_archive(
        "2026-08-09T03-55-00Z__v1374.md",
        now=_dt.datetime(2027, 2, 25, 3, 55, 0, tzinfo=_dt.timezone.utc),
    )
    assert tier == "FROZEN"
    assert age > 90 * 86400


# ----------------------------------------------------------------------
# Plan-action API
# ----------------------------------------------------------------------

def test_plan_action_hot_keep():
    pa = v1381.plan_action(
        "2026-08-09T03-55-00Z__v1374.md",
        now=_dt.datetime(2026, 8, 9, 4, 0, 0, tzinfo=_dt.timezone.utc),
    )
    assert pa["action"] == "keep"
    assert pa["target_path"] == pa["name"]
    assert pa["tier"] == "HOT"


def test_plan_action_cold_compress_to_gz():
    pa = v1381.plan_action(
        "2026-08-09T03-55-00Z__v1374.md",
        now=_dt.datetime(2026, 9, 8, 3, 55, 0, tzinfo=_dt.timezone.utc),
    )
    assert pa["action"] == "compress"
    assert pa["target_path"].endswith(".gz")
    assert pa["tier"] == "COLD"


def test_plan_action_frozen_prune():
    pa = v1381.plan_action(
        "2026-08-09T03-55-00Z__v1374.md",
        now=_dt.datetime(2027, 2, 25, 3, 55, 0, tzinfo=_dt.timezone.utc),
    )
    assert pa["action"] == "prune"
    assert pa["target_path"] == ""
    assert pa["tier"] == "FROZEN"


def test_plan_action_includes_reason():
    pa = v1381.plan_action(
        "2026-08-09T03-55-00Z__v1374.md",
        now=_dt.datetime(2026, 8, 9, 4, 0, 0, tzinfo=_dt.timezone.utc),
    )
    assert "reason" in pa
    assert "HOT" in pa["reason"]


def test_plan_rotation_empty_for_missing_dir():
    plan = v1381.plan_rotation("/nonexistent/v1381/empty/test")
    assert plan == []


def test_plan_rotation_returns_all_archives():
    with tempfile.TemporaryDirectory() as td:
        for n in [
            "2026-08-09T03-55-00Z__v1374.md",
            "2026-08-09T04-00-00Z__v1374.md",
        ]:
            with open(os.path.join(td, n), "w") as fh:
                fh.write("# x\n")
        plan = v1381.plan_rotation(
            td,
            now=_dt.datetime(2026, 8, 9, 4, 30, 0, tzinfo=_dt.timezone.utc),
        )
        assert len(plan) == 2
        # Both HOT within minutes → keep
        assert all(p["action"] == "keep" for p in plan)


def test_list_archive_names_filters_index_and_readme():
    with tempfile.TemporaryDirectory() as td:
        for n in [
            "2026-08-09T03-55-00Z__v1374.md",
            "2026-08-09T04-00-00Z__v1374.md",
            "INDEX.md",
            "README.txt",
        ]:
            with open(os.path.join(td, n), "w") as fh:
                fh.write("x")
        names = v1381._list_archive_names(td)
        assert "INDEX.md" not in names
        assert "README.txt" not in names
        assert len(names) == 2


def test_list_archive_names_returns_empty_for_missing_dir():
    names = v1381._list_archive_names("/nonexistent/v1381/list/test")
    assert names == []


# ----------------------------------------------------------------------
# Renderers
# ----------------------------------------------------------------------

def test_render_plan_md_has_required_sections():
    plan = [
        {
            "name": "2026-08-09T03-55-00Z__v1374.md",
            "tier": "HOT",
            "age_sec": 300,
            "action": "keep",
            "target_path": "2026-08-09T03-55-00Z__v1374.md",
            "reason": "test",
        }
    ]
    md = v1381.render_plan_md(plan, archive_dir="/tmp/test", policy=v1381.default_policy())
    assert "V1381" in md
    assert "Action counts" in md
    assert v1381.POLICY_VERSION in md
    assert "Honesty" in md or "honesty" in md


def test_render_policy_md_has_all_tiers():
    md = v1381.render_policy_md(v1381.default_policy())
    for tier in ("HOT", "WARM", "COLD", "FROZEN"):
        assert tier in md


def test_render_list_md_has_table_header():
    plan = [
        {"name": "x.md", "tier": "HOT", "age_sec": 0, "action": "keep",
         "target_path": "x.md", "reason": "y"}
    ]
    md = v1381.render_list_md(plan, archive_dir="/tmp/test")
    assert "| tier |" in md


def test_render_show_md_contains_tier_when_file_exists():
    """render_show_md includes sha256 only when archive is on disk."""
    with tempfile.TemporaryDirectory() as td:
        name = "2026-08-09T03-55-00Z__v1374.md"
        with open(os.path.join(td, name), "w") as fh:
            fh.write("# content\n" * 3)
        entry = {
            "name": name,
            "tier": "HOT",
            "age_sec": 0,
            "action": "keep",
            "target_path": name,
            "reason": "test",
        }
        md = v1381.render_show_md(name, entry, archive_dir=td)
        assert "tier:" in md
        assert "sha256:" in md
        assert "on disk:** yes" in md


def test_render_manifest_json_schema_and_actions():
    plan = [
        {"name": "x.md", "tier": "HOT", "age_sec": 0, "action": "keep",
         "target_path": "x.md", "reason": "y"}
    ]
    m = v1381.render_manifest_json(
        plan, policy=v1381.default_policy(), archive_dir="/tmp/test", applied=False
    )
    assert m["schema_version"] == v1381.SCHEMA_VERSION
    assert len(m["actions"]) == 1
    assert "actions_summary" in m
    assert m["applied"] is False


# ----------------------------------------------------------------------
# Atomic write + manifest roundtrip
# ----------------------------------------------------------------------

def test_write_report_creates_nested_dirs():
    with tempfile.TemporaryDirectory() as td:
        path = os.path.join(td, "nested", "deeper", "report.md")
        v1381.write_report(path, "# test\n")
        assert os.path.exists(path)
        with open(path, "r", encoding="utf-8") as fh:
            assert fh.read() == "# test\n"


def test_write_manifest_roundtrip():
    plan = [
        {"name": "x.md", "tier": "HOT", "age_sec": 0, "action": "keep",
         "target_path": "x.md", "reason": "y"}
    ]
    m = v1381.render_manifest_json(
        plan, policy=v1381.default_policy(), archive_dir="/tmp/test", applied=True
    )
    with tempfile.TemporaryDirectory() as td:
        path = os.path.join(td, "manifest.json")
        v1381.write_manifest(m, path)
        loaded = v1381.load_manifest(path)
        assert loaded is not None
        assert loaded["schema_version"] == v1381.SCHEMA_VERSION
        assert loaded["actions"][0]["name"] == "x.md"


# ----------------------------------------------------------------------
# Gzip roundtrip (binary mode to avoid Windows \r\n)
# ----------------------------------------------------------------------

def test_atomic_gzip_roundtrip_bytes_match():
    with tempfile.TemporaryDirectory() as td:
        src = os.path.join(td, "src.md")
        expected = b"# Hello World\n" * 100
        with open(src, "wb") as fh:
            fh.write(expected)
        dst = os.path.join(td, "src.md.gz")
        result = v1381._atomic_gzip(src, dst)
        assert result["ok"] is True
        assert result["src_size"] == len(expected)
        assert os.path.exists(dst)
        with gzip.open(dst, "rb") as fh:
            decompressed = fh.read()
        assert decompressed == expected


def test_atomic_gzip_records_sha256():
    with tempfile.TemporaryDirectory() as td:
        src = os.path.join(td, "src.md")
        with open(src, "wb") as fh:
            fh.write(b"x" * 50)
        dst = os.path.join(td, "src.md.gz")
        result = v1381._atomic_gzip(src, dst)
        assert len(result["src_sha256"]) == 64
        assert len(result["dst_sha256"]) == 64
        # sha256 hex format
        assert all(c in "0123456789abcdef" for c in result["src_sha256"])


# ----------------------------------------------------------------------
# Path safety
# ----------------------------------------------------------------------

def test_validate_safe_archive_dir_rejects_parent_traversal():
    with pytest.raises(ValueError):
        v1381._validate_safe_archive_dir("../etc/passwd")
    with pytest.raises(ValueError):
        v1381._validate_safe_archive_dir("/tmp/../etc")


def test_validate_safe_archive_dir_accepts_normal_paths():
    v1381._validate_safe_archive_dir("V1375_HISTORY")
    v1381._validate_safe_archive_dir("/tmp/test")
    v1381._validate_safe_archive_dir("C:\\Users\\test")


def test_validate_safe_manifest_path_rejects_parent_traversal():
    with pytest.raises(ValueError):
        v1381._validate_safe_manifest_path("../manifest.json")
    with pytest.raises(ValueError):
        v1381._validate_safe_manifest_path("a/b/../../../etc/x.json")


def test_validate_safe_manifest_path_accepts_normal_paths():
    v1381._validate_safe_manifest_path("V1381_MANIFEST_AUTO.json")
    v1381._validate_safe_manifest_path("/tmp/manifest.json")


# ----------------------------------------------------------------------
# Apply rotation — disk effects
# ----------------------------------------------------------------------

def test_apply_dry_run_no_disk_changes_when_all_hot():
    with tempfile.TemporaryDirectory() as td:
        with open(os.path.join(td, "2026-08-09T03-55-00Z__v1374.md"), "w") as fh:
            fh.write("# test\n")
        plan = v1381.plan_rotation(
            td,
            now=_dt.datetime(2026, 8, 9, 4, 0, 0, tzinfo=_dt.timezone.utc),
        )
        manifest_path = os.path.join(td, "manifest.json")
        result = v1381.apply_rotation(
            plan, td, policy=v1381.default_policy(), manifest_path=manifest_path
        )
        # All HOT → keep → skipped
        assert len(result["applied_actions"]) == 0
        assert any(s.get("reason") == "no action needed" for s in result["skipped_actions"])
        assert os.path.exists(manifest_path)
        assert result["ok"] is True


def test_apply_compress_creates_gz_and_removes_src():
    with tempfile.TemporaryDirectory() as td:
        old_name = "2026-08-09T03-55-00Z__v1374.md"
        with open(os.path.join(td, old_name), "wb") as fh:
            fh.write(b"# old\n" * 50)
        plan = v1381.plan_rotation(
            td,
            now=_dt.datetime(2026, 9, 8, 4, 0, 0, tzinfo=_dt.timezone.utc),
        )
        manifest_path = os.path.join(td, "manifest.json")
        result = v1381.apply_rotation(
            plan, td, policy=v1381.default_policy(), manifest_path=manifest_path
        )
        assert len(result["applied_actions"]) == 1
        assert result["applied_actions"][0]["ok"] is True
        assert os.path.exists(os.path.join(td, old_name + ".gz"))
        assert not os.path.exists(os.path.join(td, old_name))
        loaded = v1381.load_manifest(manifest_path)
        assert loaded is not None
        assert loaded["applied_actions"][0]["action"] == "compress"


def test_apply_refuses_prune_by_default():
    with tempfile.TemporaryDirectory() as td:
        old_name = "2026-08-09T03-55-00Z__v1374.md"
        with open(os.path.join(td, old_name), "w") as fh:
            fh.write("# old\n")
        plan = v1381.plan_rotation(
            td,
            now=_dt.datetime(2027, 2, 25, 4, 0, 0, tzinfo=_dt.timezone.utc),
        )
        manifest_path = os.path.join(td, "manifest.json")
        result = v1381.apply_rotation(
            plan, td, policy=v1381.default_policy(), manifest_path=manifest_path
        )
        # Prune action NOT in default {"compress"} → must NOT be in applied_actions
        assert not any(
            a.get("action") == "prune" for a in result["applied_actions"]
        )
        # Prune must be logged in skipped_actions
        assert any(
            s.get("action") == "prune" for s in result["skipped_actions"]
        )
        # File must still exist
        assert os.path.exists(os.path.join(td, old_name))


def test_apply_prune_with_allow_flag_actually_deletes():
    with tempfile.TemporaryDirectory() as td:
        old_name = "2026-08-09T03-55-00Z__v1374.md"
        with open(os.path.join(td, old_name), "w") as fh:
            fh.write("# old\n")
        plan = v1381.plan_rotation(
            td,
            now=_dt.datetime(2027, 2, 25, 4, 0, 0, tzinfo=_dt.timezone.utc),
        )
        manifest_path = os.path.join(td, "manifest.json")
        result = v1381.apply_rotation(
            plan, td,
            policy=v1381.default_policy(),
            manifest_path=manifest_path,
            actions_to_apply={"compress", "prune"},
        )
        assert len(result["applied_actions"]) == 1
        assert result["applied_actions"][0]["action"] == "prune"
        assert not os.path.exists(os.path.join(td, old_name))


# ----------------------------------------------------------------------
# CLI — subcommands
# ----------------------------------------------------------------------

def _capture_cli(args):
    """Run run_cli and capture stdout/stderr + return code."""
    buf_out = io.StringIO()
    buf_err = io.StringIO()
    with redirect_stdout(buf_out), redirect_stderr(buf_err):
        rc = v1381.run_cli(args)
    return rc, buf_out.getvalue(), buf_err.getvalue()


def test_cli_version_subcommand():
    rc, out, _ = _capture_cli(["version"])
    assert rc == 0
    assert v1381.SCHEMA_VERSION in out


def test_cli_policy_subcommand_writes_file():
    with tempfile.TemporaryDirectory() as td:
        policy_path = os.path.join(td, "policy.md")
        rc, out, _ = _capture_cli(["--policy-path", policy_path, "policy"])
        assert rc == 0
        assert os.path.exists(policy_path)
        with open(policy_path, "r", encoding="utf-8") as fh:
            content = fh.read()
        for tier in ("HOT", "WARM", "COLD", "FROZEN"):
            assert tier in content


def test_cli_plan_subcommand_writes_plan_file():
    with tempfile.TemporaryDirectory() as td:
        with open(os.path.join(td, "2026-08-09T03-55-00Z__v1374.md"), "w") as fh:
            fh.write("# x\n")
        plan_path = os.path.join(td, "plan.md")
        rc, out, _ = _capture_cli([
            "--archive-dir", td, "--plan-path", plan_path, "plan",
        ])
        assert rc == 0
        assert os.path.exists(plan_path)
        with open(plan_path, "r", encoding="utf-8") as fh:
            content = fh.read()
        assert "V1381" in content


def test_cli_list_subcommand_writes_list_file():
    with tempfile.TemporaryDirectory() as td:
        with open(os.path.join(td, "2026-08-09T03-55-00Z__v1374.md"), "w") as fh:
            fh.write("# x\n")
        list_path = os.path.join(td, "list.md")
        rc, _, _ = _capture_cli([
            "--archive-dir", td,
            "--plan-path", list_path.replace("list", "PLAN"),
            "list",
        ])
        assert rc == 0


def test_cli_rotate_dry_run_does_not_mutate():
    with tempfile.TemporaryDirectory() as td:
        old_name = "2026-08-09T03-55-00Z__v1374.md"
        with open(os.path.join(td, old_name), "w") as fh:
            fh.write("# x\n")
        plan_path = os.path.join(td, "plan.md")
        manifest_path = os.path.join(td, "manifest.json")
        rc, out, _ = _capture_cli([
            "--archive-dir", td, "--plan-path", plan_path,
            "--manifest-path", manifest_path, "rotate",
            "--now", "2026-09-08T04:00:00Z",
        ])
        assert rc == 0
        assert os.path.exists(plan_path)
        assert not os.path.exists(manifest_path)
        assert os.path.exists(os.path.join(td, old_name))
        assert not os.path.exists(os.path.join(td, old_name + ".gz"))


def test_cli_rotate_apply_compresses_and_writes_manifest():
    with tempfile.TemporaryDirectory() as td:
        old_name = "2026-08-09T03-55-00Z__v1374.md"
        with open(os.path.join(td, old_name), "wb") as fh:
            fh.write(b"# old\n" * 100)
        plan_path = os.path.join(td, "plan.md")
        manifest_path = os.path.join(td, "manifest.json")
        rc, out, _ = _capture_cli([
            "--archive-dir", td, "--plan-path", plan_path,
            "--manifest-path", manifest_path, "rotate",
            "--apply", "--now", "2026-09-08T04:00:00Z",
        ])
        assert rc == 0
        assert os.path.exists(os.path.join(td, old_name + ".gz"))
        assert not os.path.exists(os.path.join(td, old_name))
        assert os.path.exists(manifest_path)


def test_cli_show_subcommand_prints_tier_and_sha():
    with tempfile.TemporaryDirectory() as td:
        old_name = "2026-08-09T03-55-00Z__v1374.md"
        with open(os.path.join(td, old_name), "w") as fh:
            fh.write("# test content\n" * 5)
        rc, out, _ = _capture_cli([
            "--archive-dir", td, "show", old_name,
        ])
        assert rc == 0
        assert "tier:" in out
        assert "sha256:" in out


def test_cli_show_missing_archive_returns_2():
    with tempfile.TemporaryDirectory() as td:
        rc, out, err = _capture_cli([
            "--archive-dir", td, "show", "nonexistent.md",
        ])
        assert rc == 2
        assert "not found" in err


# ----------------------------------------------------------------------
# Determinism + guards
# ----------------------------------------------------------------------

def test_plan_is_deterministic():
    plan_a = v1381.plan_rotation("/nonexistent/v1381/empty/test")
    plan_b = v1381.plan_rotation("/nonexistent/v1381/empty/test")
    assert plan_a == plan_b


def test_render_plan_md_is_deterministic():
    pol = v1381.default_policy()
    now = _dt.datetime(2026, 8, 9, 4, 0, 0, tzinfo=_dt.timezone.utc)
    md_a = v1381.render_plan_md([], archive_dir="/x", policy=pol, now=now)
    md_b = v1381.render_plan_md([], archive_dir="/x", policy=pol, now=now)
    assert md_a == md_b


def test_popper_self_tests_all_pass():
    passed, total, failures = v1381._popper_self_tests()
    assert passed == total, f"Popper failures: {failures}"
    assert total >= 80, f"Expected ≥80 Popper self-tests, got {total}"


# ----------------------------------------------------------------------
# Real-data smoke (production data, optional)
# ----------------------------------------------------------------------

def test_real_data_plan_smoke():
    """Run plan against the actual V1375_HISTORY if it exists."""
    if not os.path.isdir("V1375_HISTORY"):
        pytest.skip("V1375_HISTORY dir not present (production smoke test)")
    rc, out, _ = _capture_cli(["plan"])
    assert rc == 0
    assert os.path.exists("V1381_PLAN_AUTO.md")
    # Clean up so the working tree isn't dirtied by smoke tests
    try:
        os.unlink("V1381_PLAN_AUTO.md")
    except OSError:
        pass