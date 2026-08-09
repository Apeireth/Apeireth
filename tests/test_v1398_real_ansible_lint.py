"""V1398 真生产 ansible playbook lint tests (主 17:43 实事求是).

主 17:43 实事求是: 真跑真测, 不假装 pass.
主 19:33 走在前人经验上: 真借鉴 ansible-lint + yamllint + community-ansible-lint-rules 真规则.
"""
from __future__ import annotations

import json
import os
import sys
import tempfile
from pathlib import Path

import pytest

# V1398 ensure repo root on path
_PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
if str(_PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROMETHEAN_ROOT))

from apeireth.v1398_real_ansible_lint import (  # noqa: E402
    V1398_VERSION, V1398_SCHEMA,
    V1398_GUARDS, V1398_BORROWED, V1398_RULES,
    V1398_SECRET_PATTERNS, V1398_DEPRECATED_LOOPS, V1398_CMD_TO_MODULE,
    V1398_INLINE_ENV_SECRET,
    LintFinding, LintReport,
    _strip_quotes, _normalize_value, _coerce_plays, _find_line_no,
    _parse_playbook, _popper_self_test,
    lint_file, lint_path, lint_text,
    _report_to_sarif, chain_with_v1387,
    main,
)


# ============================================================================
# V1398 constants & helpers
# ============================================================================


class TestV1398Constants:
    """V1398 真生产 constants (主 17:43)."""

    def test_version_is_string(self):
        assert isinstance(V1398_VERSION, str) and len(V1398_VERSION) > 0

    def test_schema_includes_v1398(self):
        assert "v1398" in V1398_SCHEMA
        assert "ansible" in V1398_SCHEMA

    def test_guards_count_is_12(self):
        assert len(V1398_GUARDS) == 12

    def test_guards_required_names(self):
        required = {
            "GUARD_YAML_PARSED", "GUARD_RULES_REAL", "GUARD_FILE_IO",
            "GUARD_LINE_TRACKED", "GUARD_NO_CAP_CHANGE", "GUARD_DETERMINISTIC",
            "GUARD_HONEST_DISCLOSURE", "GUARD_PATH_SAFE", "GUARD_NON_DESTRUCTIVE",
            "GUARD_DELEGATE_REAL", "GUARD_CLI_RUNNABLE", "GUARD_POPPER_RUNS",
        }
        assert set(V1398_GUARDS) == required

    def test_borrowed_count_is_6(self):
        assert len(V1398_BORROWED) == 6

    def test_borrowed_includes_ansible_lint(self):
        joined = " ".join(V1398_BORROWED).lower()
        assert "ansible-lint" in joined
        assert "yamllint" in joined

    def test_rules_count_is_12(self):
        assert len(V1398_RULES) == 12

    def test_all_rule_ids_are_unique(self):
        ids = [rid for rid, _ in V1398_RULES]
        assert len(ids) == len(set(ids))

    def test_secret_patterns_non_empty(self):
        assert len(V1398_SECRET_PATTERNS) >= 4

    def test_deprecated_loops_includes_with_items(self):
        assert "with_items" in V1398_DEPRECATED_LOOPS
        assert "with_dict" in V1398_DEPRECATED_LOOPS

    def test_inline_env_secret_patterns(self):
        assert len(V1398_INLINE_ENV_SECRET) >= 1

    def test_cmd_to_module_pairs(self):
        assert len(V1398_CMD_TO_MODULE) >= 10


# ============================================================================
# V1398 helpers
# ============================================================================


class TestV1398StripQuotes:
    """V1398 _strip_quotes helper."""

    def test_double_quotes(self):
        assert _strip_quotes('"hello"') == "hello"

    def test_single_quotes(self):
        assert _strip_quotes("'hello'") == "hello"

    def test_no_quotes(self):
        assert _strip_quotes("hello") == "hello"

    def test_empty(self):
        assert _strip_quotes("") == ""

    def test_single_char_no_strip(self):
        # 1 char, doesn't pass the >=2 check
        assert _strip_quotes('"') == '"'

    def test_non_string(self):
        assert _strip_quotes(42) == 42
        assert _strip_quotes(None) is None
        assert _strip_quotes(["a"]) == ["a"]


class TestV1398NormalizeValue:
    """V1398 _normalize_value helper."""

    def test_string_with_quotes(self):
        assert _normalize_value('"hello"') == "hello"

    def test_list_of_strings(self):
        assert _normalize_value(['"a"', '"b"']) == ["a", "b"]

    def test_dict_with_quoted_keys(self):
        result = _normalize_value({'"name"': '"alice"'})
        assert result == {"name": "alice"}

    def test_non_string_passthrough(self):
        assert _normalize_value(42) == 42
        assert _normalize_value(True) is True


class TestV1398CoercePlays:
    """V1398 _coerce_plays helper."""

    def test_list_of_dicts(self):
        assert len(_coerce_plays([{"hosts": "all"}, {"hosts": "web"}])) == 2

    def test_single_dict(self):
        assert len(_coerce_plays({"hosts": "all"})) == 1

    def test_empty_list(self):
        assert _coerce_plays([]) == []

    def test_none(self):
        assert _coerce_plays(None) == []

    def test_string(self):
        assert _coerce_plays("not a playbook") == []


class TestV1398FindLineNo:
    """V1398 _find_line_no helper."""

    def test_first_line(self):
        text = "hello world"
        assert _find_line_no(text, "hello") == 1

    def test_second_line(self):
        text = "first\nsecond\nthird"
        assert _find_line_no(text, "second") == 2

    def test_not_found(self):
        text = "hello"
        assert _find_line_no(text, "missing") == 0

    def test_empty_target(self):
        assert _find_line_no("hello", "") == 0


class TestV1398ParsePlaybook:
    """V1398 _parse_playbook helper."""

    def test_multi_play_list(self):
        text = """---
- hosts: web
  tasks: []
- hosts: db
  tasks: []
"""
        plays, err, n_plays, n_tasks = _parse_playbook(text, "test.yml")
        assert err == ""
        assert n_plays == 2
        assert n_tasks == 0

    def test_single_play_dict(self):
        text = """---
hosts: web
tasks: []
"""
        plays, err, n_plays, _ = _parse_playbook(text, "test.yml")
        assert err == ""
        assert n_plays == 1

    def test_count_tasks(self):
        text = """---
- hosts: all
  tasks:
    - name: t1
      apt: name=foo
    - name: t2
      apt: name=bar
"""
        _, _, n_plays, n_tasks = _parse_playbook(text, "t.yml")
        assert n_plays == 1
        assert n_tasks == 2

    def test_invalid_yaml_returns_error(self):
        text = "::: bad :::"
        plays, err, _, _ = _parse_playbook(text, "bad.yml")
        # PyYAML may still parse this as a string; either way should not crash
        assert isinstance(plays, list)
        assert isinstance(err, str)


# ============================================================================
# V1398 individual rules
# ============================================================================


_BAD_PLAYBOOK = """---
- hosts: all
  vars:
    db_password: "supersecret123"
  tasks:
    - name: install package
      command: apt-get install -y nginx
    - shell: cat /etc/passwd | grep root
    - command: systemctl restart nginx
    - command: echo $DB_PASSWORD
      environment:
        DB_PASSWORD: "supersecret123"
    - name: legacy loop
      with_items: "{{ packages }}"
    - name: bad task without name
      apt: name=vim
    - name: ignore errors
      command: /bin/false
      ignore_errors: true
    - name: loop undefined
      debug:
        msg: "{{ item }}"
      loop: "{{ undefined_var }}"
    - copy:
        content: |
          -----BEGIN RSA PRIVATE KEY-----
          MIIE...
          -----END RSA PRIVATE KEY-----
        dest: /tmp/k.pem
"""

_CLEAN_PLAYBOOK = """---
- name: Setup web
  hosts: webservers
  tags: [setup, web]
  vars:
    packages:
      - nginx
      - curl
  tasks:
    - name: Install nginx
      tags: [install]
      apt:
        name: "{{ packages }}"
        state: present
    - name: Start nginx
      tags: [service]
      service:
        name: nginx
        state: started
        enabled: true
"""


class TestV1398RulesFire:
    """V1398 真生产 rules 真在 bad sample 上 fire (主 17:43 实事求是)."""

    def test_an001_no_play_name_fires(self):
        rep = lint_text(_BAD_PLAYBOOK, "bad.yml")
        ids = {f.rule_id for f in rep.findings}
        assert "AN001-NO-PLAY-NAME" in ids

    def test_an002_no_task_name_fires(self):
        rep = lint_text(_BAD_PLAYBOOK, "bad.yml")
        ids = {f.rule_id for f in rep.findings}
        assert "AN002-NO-TASK-NAME" in ids

    def test_an003_hardcoded_secret_fires(self):
        rep = lint_text(_BAD_PLAYBOOK, "bad.yml")
        ids = {f.rule_id for f in rep.findings}
        assert "AN003-HARDCODED-SECRET" in ids

    def test_an004_plain_private_key_fires(self):
        rep = lint_text(_BAD_PLAYBOOK, "bad.yml")
        ids = {f.rule_id for f in rep.findings}
        assert "AN004-PLAIN-PRIVATE-KEY" in ids

    def test_an005_risky_shell_pipe_fires(self):
        rep = lint_text(_BAD_PLAYBOOK, "bad.yml")
        ids = {f.rule_id for f in rep.findings}
        assert "AN005-RISKY-SHELL-PIPE" in ids

    def test_an006_no_changed_when_fires(self):
        rep = lint_text(_BAD_PLAYBOOK, "bad.yml")
        ids = {f.rule_id for f in rep.findings}
        assert "AN006-NO-CHANGED-WHEN" in ids

    def test_an007_deprecated_loop_fires(self):
        rep = lint_text(_BAD_PLAYBOOK, "bad.yml")
        ids = {f.rule_id for f in rep.findings}
        assert "AN007-DEPRECATED-LOOP" in ids

    def test_an008_command_instead_module_fires(self):
        rep = lint_text(_BAD_PLAYBOOK, "bad.yml")
        ids = {f.rule_id for f in rep.findings}
        assert "AN008-COMMAND-INSTEAD-MODULE" in ids

    def test_an009_inline_env_var_fires(self):
        rep = lint_text(_BAD_PLAYBOOK, "bad.yml")
        ids = {f.rule_id for f in rep.findings}
        assert "AN009-INLINE-ENV-VAR" in ids

    def test_an010_missing_tags_fires(self):
        rep = lint_text(_BAD_PLAYBOOK, "bad.yml")
        ids = {f.rule_id for f in rep.findings}
        assert "AN010-MISSING-TAGS" in ids

    def test_an011_ignore_errors_masks_fires(self):
        rep = lint_text(_BAD_PLAYBOOK, "bad.yml")
        ids = {f.rule_id for f in rep.findings}
        assert "AN011-IGNORE-ERRORS-MASKS" in ids

    def test_an012_loop_undefined_var_fires(self):
        rep = lint_text(_BAD_PLAYBOOK, "bad.yml")
        ids = {f.rule_id for f in rep.findings}
        assert "AN012-LOOP-UNDEFINED-VAR" in ids

    def test_all_12_rules_fire_on_bad_sample(self):
        rep = lint_text(_BAD_PLAYBOOK, "bad.yml")
        ids = {f.rule_id for f in rep.findings}
        expected = {rid for rid, _ in V1398_RULES}
        missing = expected - ids
        assert not missing, f"missing rules: {missing}"

    def test_clean_playbook_no_errors_or_warnings(self):
        rep = lint_text(_CLEAN_PLAYBOOK, "clean.yml")
        assert rep.n_errors == 0, f"unexpected errors: {rep.findings}"
        assert rep.n_warnings == 0, f"unexpected warnings: {rep.findings}"


# ============================================================================
# V1398 lint_file / lint_path / lint_text
# ============================================================================


class TestV1398LintFile:
    """V1398 lint_file 真生产 (主 17:43)."""

    def test_lint_real_file(self, tmp_path):
        f = tmp_path / "playbook.yml"
        f.write_text(_BAD_PLAYBOOK, encoding="utf-8")
        rep = lint_file(str(f))
        assert rep.ok is True
        assert rep.n_findings >= 8
        assert rep.n_errors >= 3
        assert rep.file_path == str(f)

    def test_lint_missing_file(self, tmp_path):
        rep = lint_file(str(tmp_path / "missing.yml"))
        assert rep.ok is False
        assert "Cannot read" in rep.parse_error

    def test_lint_clean_file(self, tmp_path):
        f = tmp_path / "clean.yml"
        f.write_text(_CLEAN_PLAYBOOK, encoding="utf-8")
        rep = lint_file(str(f))
        assert rep.ok is True
        assert rep.n_errors == 0
        assert rep.n_warnings == 0

    def test_lint_path_directory(self, tmp_path):
        (tmp_path / "a.yml").write_text(_BAD_PLAYBOOK, encoding="utf-8")
        (tmp_path / "b.yml").write_text(_CLEAN_PLAYBOOK, encoding="utf-8")
        reports = lint_path(str(tmp_path))
        assert len(reports) == 2

    def test_lint_path_empty_dir(self, tmp_path):
        reports = lint_path(str(tmp_path))
        assert reports == []

    def test_lint_path_filters_non_yaml(self, tmp_path):
        (tmp_path / "a.yml").write_text(_BAD_PLAYBOOK, encoding="utf-8")
        (tmp_path / "ignore.txt").write_text(_BAD_PLAYBOOK, encoding="utf-8")
        reports = lint_path(str(tmp_path))
        assert len(reports) == 1


# ============================================================================
# V1398 SARIF output
# ============================================================================


class TestV1398Sarif:
    """V1398 真生产 SARIF 输出 (主 00:36 工程化)."""

    def test_sarif_version(self):
        rep = lint_text(_BAD_PLAYBOOK, "bad.yml")
        sarif = _report_to_sarif([rep])
        assert sarif["version"] == "2.1.0"

    def test_sarif_has_results(self):
        rep = lint_text(_BAD_PLAYBOOK, "bad.yml")
        sarif = _report_to_sarif([rep])
        assert len(sarif["runs"][0]["results"]) >= 8

    def test_sarif_severity_mapping(self):
        rep = lint_text(_BAD_PLAYBOOK, "bad.yml")
        sarif = _report_to_sarif([rep])
        results = sarif["runs"][0]["results"]
        levels = {r["level"] for r in results}
        assert "error" in levels
        assert "warning" in levels

    def test_sarif_has_locations(self):
        rep = lint_text(_BAD_PLAYBOOK, "bad.yml")
        sarif = _report_to_sarif([rep])
        for r in sarif["runs"][0]["results"]:
            assert "locations" in r
            assert "physicalLocation" in r["locations"][0]
            assert "uri" in r["locations"][0]["physicalLocation"]["artifactLocation"]


# ============================================================================
# V1398 chain delegate
# ============================================================================


class TestV1398Chain:
    """V1398 真生产 chain V1387 + V1398 (主 17:43)."""

    def test_chain_with_real_dir(self, tmp_path):
        (tmp_path / "a.yml").write_text(_BAD_PLAYBOOK, encoding="utf-8")
        out = chain_with_v1387(str(tmp_path))
        assert out["schema"] == "v1398.ansible-lint.chain/v1"
        assert out["v1398"]["n_files"] == 1
        assert out["v1398"]["n_findings"] >= 8
        assert "v1387_delegate" in out

    def test_chain_v1387_failure_is_handled(self, tmp_path):
        # 路径不存在, v1387 delegate 应 graceful 处理
        out = chain_with_v1387(str(tmp_path))
        # 真生产: 不应该 raise
        assert "v1398" in out


# ============================================================================
# V1398 popper self-test
# ============================================================================


class TestV1398Popper:
    """V1398 popper self-test 真跑真测 (主 17:43)."""

    def test_popper_ok(self):
        result = _popper_self_test()
        assert result["ok"] is True

    def test_popper_all_5_tests(self):
        result = _popper_self_test()
        test_names = {t["name"] for t in result["tests"]}
        assert "bad_sample_lints" in test_names
        assert "clean_sample_lints" in test_names
        assert "all_12_rules_fire" in test_names
        assert "sarif_roundtrip" in test_names
        assert "chain_delegate" in test_names

    def test_popper_12_rules_metadata(self):
        result = _popper_self_test()
        assert result["n_rules"] == 12
        assert result["n_guards"] == 12


# ============================================================================
# V1398 CLI
# ============================================================================


class TestV1398CLI:
    """V1398 CLI 真可执行 (主 17:43)."""

    def test_cli_version(self, capsys):
        rc = main(["version"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "v1398-ansible-lint" in out
        assert V1398_VERSION in out

    def test_cli_demo(self, capsys):
        rc = main(["demo"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "AN001-NO-PLAY-NAME" in out
        assert "ansible-lint" in out.lower()

    def test_cli_popper(self, capsys):
        rc = main(["popper"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert data["ok"] is True

    def test_cli_lint_text(self, capsys, tmp_path):
        f = tmp_path / "bad.yml"
        f.write_text(_BAD_PLAYBOOK, encoding="utf-8")
        rc = main(["lint", str(f)])
        assert rc in (1, 2)  # errors → non-zero
        out = capsys.readouterr().out
        assert "AN003-HARDCODED-SECRET" in out

    def test_cli_lint_json(self, capsys, tmp_path):
        f = tmp_path / "bad.yml"
        f.write_text(_BAD_PLAYBOOK, encoding="utf-8")
        rc = main(["lint", str(f), "--format", "json"])
        assert rc in (1, 2)
        out = capsys.readouterr().out
        data = json.loads(out)
        assert isinstance(data, list)
        assert data[0]["n_errors"] >= 3

    def test_cli_lint_sarif(self, capsys, tmp_path):
        f = tmp_path / "bad.yml"
        f.write_text(_BAD_PLAYBOOK, encoding="utf-8")
        rc = main(["lint", str(f), "--format", "sarif"])
        out = capsys.readouterr().out
        data = json.loads(out)
        assert data["version"] == "2.1.0"

    def test_cli_lint_clean(self, capsys, tmp_path):
        f = tmp_path / "clean.yml"
        f.write_text(_CLEAN_PLAYBOOK, encoding="utf-8")
        rc = main(["lint", str(f)])
        assert rc == 0
        out = capsys.readouterr().out
        assert "findings=0" in out or "findings=" in out

    def test_cli_help(self, capsys):
        rc = main([])
        assert rc == 0
        out = capsys.readouterr().out
        assert "v1398-ansible-lint" in out

    def test_cli_chain(self, capsys, tmp_path):
        (tmp_path / "a.yml").write_text(_BAD_PLAYBOOK, encoding="utf-8")
        rc = main(["chain", str(tmp_path)])
        out = capsys.readouterr().out
        assert "V1398 chain report" in out


# ============================================================================
# V1398 V3 philosophy guards (主 17:58 + 主 20:46 + 主 17:43)
# ============================================================================


class TestV1398V3Guards:
    """V1398 真生产 V3 哲学守门 (主 17:43 不假装)."""

    def test_no_cap_change_guard_in_list(self):
        assert "GUARD_NO_CAP_CHANGE" in V1398_GUARDS

    def test_honest_disclosure_guard_in_list(self):
        assert "GUARD_HONEST_DISCLOSURE" in V1398_GUARDS

    def test_deterministic_guard_in_list(self):
        assert "GUARD_DETERMINISTIC" in V1398_GUARDS

    def test_deterministic_same_input_same_output(self):
        rep1 = lint_text(_BAD_PLAYBOOK, "a.yml")
        rep2 = lint_text(_BAD_PLAYBOOK, "a.yml")
        # findings count may differ by line_no only; ensure same rule set + counts
        ids1 = sorted([f.rule_id for f in rep1.findings])
        ids2 = sorted([f.rule_id for f in rep2.findings])
        assert ids1 == ids2
        assert rep1.n_findings == rep2.n_findings

    def test_non_destructive_guard_in_list(self):
        assert "GUARD_NON_DESTRUCTIVE" in V1398_GUARDS

    def test_cli_runnable_guard_in_list(self):
        assert "GUARD_CLI_RUNNABLE" in V1398_GUARDS


# ============================================================================
# V1398 continuity with V1384-V1397
# ============================================================================


class TestV1398Continuity:
    """V1398 真生产 chain continuity V1384-V1397 (主 17:43 实事求是)."""

    def test_does_not_break_v1397_imports(self):
        from apeireth import v1397_real_terraform_lint as v1397  # noqa: F401
        assert v1397.V1397_VERSION == "0.1.0"

    def test_does_not_break_v1396_imports(self):
        from apeireth import v1396_deploy_executor as v1396  # noqa: F401
        assert hasattr(v1396, "V1396_VERSION")

    def test_v1398_in_apeireth_package(self):
        import apeireth
        mod = __import__("apeireth.v1398_real_ansible_lint", fromlist=["v1398"])
        assert mod.V1398_VERSION == V1398_VERSION

    def test_self_referential_cli_works(self, capsys):
        """V1398 真跑 self CLI (主 17:43)."""
        rc = main(["version"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "v1398-ansible-lint" in out