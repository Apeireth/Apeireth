# R11 P0 Acceptance Gate Report

**Generated (UTC):** 2026-07-30T07:36:25.691861+00:00
**Result:** 5/5 gates PASS

| Gate | Status | Reason |
|------|--------|--------|
| `A.v1136/v1074_truth_source` | ✅ PASS | V1136 真测 3-dim + V0.5=0.8682, V1074 V0.3=0.8959 (snap_3346c8999203) |
| `B.dashboard_version_contract` | ✅ PASS | snapshot v0.1.0 level=ASI v03_score=0.8964 (snap_9c80c9165625) 与 report 一致 |
| `C.v3_nine_key_guard` | ✅ PASS | ASI 9 键 全部 LOCKED (9/9) |
| `D.test_evidence` | ✅ PASS | pytest 子集 PASSED (5 files): ============================ 107 passed in 37.93s ============================= |
| `E.git_traceability` | ✅ PASS | git HEAD=cf30a7ef1e1c (20 recent commits, 17 conventional) |

---

## `A.v1136/v1074_truth_source` — PASS

**Reason**: V1136 真测 3-dim + V0.5=0.8682, V1074 V0.3=0.8959 (snap_3346c8999203)

<details><summary>Details (click to expand)</summary>

```json
{
  "workspace": "C:\\Users\\REDACTED\\.openclaw\\workspace\\promethean",
  "v1136_continuity": 0.95,
  "v1136_autonomy": 0.95,
  "v1136_transferability": 0.95,
  "v1136_n_subs_continuity": 8,
  "v1136_n_subs_autonomy": 4,
  "v1136_n_subs_transferability": 4,
  "v1136_v05_total": 0.8682,
  "v1136_v05_v1125_placeholder": 0.8532,
  "v1136_v05_delta": 0.015,
  "v1136_v3_guards_pass": true,
  "v1136_v3_guards_count": 6,
  "v1074_snapshot_id": "snap_3346c8999203",
  "v1074_level": "ASI",
  "v1074_v03_score": 0.8959,
  "v1074_n_modules": 1160,
  "v1074_n_tests": 6585,
  "v1074_n_commits": 564
}
```

</details>

## `B.dashboard_version_contract` — PASS

**Reason**: snapshot v0.1.0 level=ASI v03_score=0.8964 (snap_9c80c9165625) 与 report 一致

<details><summary>Details (click to expand)</summary>

```json
{
  "snapshot_path": "C:\\Users\\REDACTED\\.openclaw\\workspace\\promethean\\artifacts\\asi_snapshot.json",
  "report_path": "C:\\Users\\REDACTED\\.openclaw\\workspace\\promethean\\reports\\asi_report.md",
  "snapshot_id": "snap_9c80c9165625",
  "version": "0.1.0",
  "level": "ASI",
  "v03_score": 0.8964,
  "n_modules": 1153,
  "n_tests": 6394,
  "n_commits": 542,
  "ts_iso": "2026-07-30T02:10:51+00:00",
  "report_size_bytes": 2835
}
```

</details>

## `C.v3_nine_key_guard` — PASS

**Reason**: ASI 9 键 全部 LOCKED (9/9)

<details><summary>Details (click to expand)</summary>

```json
{
  "n_keys": 9,
  "keys": [
    "not_undo",
    "not_proof",
    "not_safe",
    "not_clone",
    "not_perfect",
    "not_uuid",
    "spec_is_not_proof",
    "counterexample_is_not_bug",
    "production_is_not_autonomy"
  ],
  "lock_values": {
    "not_undo": true,
    "not_proof": true,
    "not_safe": true,
    "not_clone": true,
    "not_perfect": true,
    "not_uuid": true,
    "spec_is_not_proof": true,
    "counterexample_is_not_bug": true,
    "production_is_not_autonomy": true
  },
  "verify_or_raise_works": true,
  "verify_or_raise_message": "ASI 9 键 LOCKED 失败: ['not_undo'] (主 17:43 实事求是: dispatcher 拒服)"
}
```

</details>

## `D.test_evidence` — PASS

**Reason**: pytest 子集 PASSED (5 files): ============================ 107 passed in 37.93s =============================

<details><summary>Details (click to expand)</summary>

```json
{
  "workspace": "C:\\Users\\REDACTED\\.openclaw\\workspace\\promethean",
  "test_files": [
    "tests/test_v1136_asi_v05_3dim_real_measurement.py",
    "tests/test_r4_asi_fun_score.py",
    "tests/test_r4_cli_smoke.py",
    "tests/test_r6_formal_verify_contract.py",
    "tests/test_r11_p0_regression_guard.py"
  ],
  "present_test_files": [
    "tests/test_v1136_asi_v05_3dim_real_measurement.py",
    "tests/test_r4_asi_fun_score.py",
    "tests/test_r4_cli_smoke.py",
    "tests/test_r6_formal_verify_contract.py",
    "tests/test_r11_p0_regression_guard.py"
  ],
  "missing_test_files": [],
  "tests_run": true,
  "pytest_returncode": 0,
  "pytest_elapsed_seconds": 38.335,
  "pytest_stdout_tail": "============================= test session starts =============================\ncollected 107 items\n\ntests\\test_v1136_asi_v05_3dim_real_measurement.py ...................... [ 20%]\n..........                                                               [ 29%]\ntests\\test_r4_asi_fun_score.py .....                                     [ 34%]\ntests\\test_r4_cli_smoke.py .....                                         [ 39%]\ntests\\test_r6_formal_verify_contract.py ........                         [ 46%]\ntests\\test_r11_p0_regression_guard.py .................................. [ 78%]\n.......................                                                  [100%]\n\n============================ 107 passed in 37.93s =============================\n",
  "pytest_stderr_tail": "[conftest] api-key env isolation active (python=3.13.14)\n",
  "pytest_summary": [
    "tests\\test_r11_p0_regression_guard.py .................................. [ 78%]",
    ".......................                                                  [100%]",
    "============================ 107 passed in 37.93s ============================="
  ]
}
```

</details>

## `E.git_traceability` — PASS

**Reason**: git HEAD=cf30a7ef1e1c (20 recent commits, 17 conventional)

<details><summary>Details (click to expand)</summary>

```json
{
  "workspace": "C:\\Users\\REDACTED\\.openclaw\\workspace\\promethean",
  "head_sha": "cf30a7ef1e1c3124908cfef00dfde472d2c8ea32",
  "n_recent_commits": 20,
  "recent_commits_tail": [
    "d969d912 docs(team-finalize-status-2026-07-30-v2): 系统强制跳过记录",
    "5a312fb2 docs(omnibus-2026-07-30): 附录 F research-trending-2026 真读补充",
    "6a9afc70 docs(omnibus-2026-07-30): 附录 E 真调研第五轮深度补充",
    "84525b9c round-50 cross-domain: Haken synergetics + Prigogine + CAS + SOC + Damasio + Bonabeau swarm + Edelman-IIT | ray-project/claude-code/open_deep_research | reproduction MISSING gap + consciousness ultimate goal",
    "b2139242 docs(final-status-2026-07-30): 主文档已 git commit 73f92be 落盘 + system bug 透明化"
  ],
  "conventional_commit_count": 17,
  "conventional_commit_ratio": 0.85,
  "git_porcelain_lines": 130,
  "git_porcelain_sample": [
    " M .coverage",
    " M .spectrai-worktrees/integrations/527f21de-e3e3-4dcc-a90d-d022bec6d5e5",
    " M apeireth/cron_self_update.py",
    " M apeireth/serve.py",
    " M apeireth/v1035_streamlit.py"
  ],
  "snapshot_n_commits": 542,
  "git_log_n_commits": 564
}
```

</details>

---

_Generated by apeireth.r11_requirements_gate (5 gates)._