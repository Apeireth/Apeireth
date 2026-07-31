# R9 W4 Security Audit Report — V1121 真审查

- version: 0.1.0
- timestamp: 2026-07-30 00:11:31
- 审查人: security_reviewer (R9-SEC-001)

## Dashboard

# V1121 Security Dashboard

- status: **RED**
- timestamp: 2026-07-30 00:11:31
- threats_total: **11**
- P0: 4 | P1: 6 | P2: 1 | P3: 0 | P4: 0
- gates_passed: 2/5


## V1072 Identity 守门真测

```json
{
  "identity_id_ok": true,
  "identity_id_format_ok": true,
  "continuity_score_ok": true,
  "continuity_score": 1.0,
  "ltm_persistence_ok": true,
  "core_snapshot_hash_ok": true,
  "n_threats": 0,
  "threats": [],
  "gate_passed": true
}
```

## V1095 Store 守门真测

```json
{
  "n_inputs_validated": 13,
  "n_inputs_blocked": 10,
  "n_injection_attempts": 6,
  "n_unauthorized_attempts": 0,
  "n_role_violations": 3,
  "n_path_traversals": 1,
  "fsync_audit_passed": true,
  "access_control_passed": false,
  "n_threats": 3,
  "gate_passed": false
}
```

## V1112 DGM v0.4 真演化 Threat Model

```json
{
  "n_candidates_total": 3,
  "n_candidates_anchored": 2,
  "n_orphans_rejected": 1,
  "n_unanchored_rejected": 0,
  "n_archive_sealed": 3,
  "archive_encrypted": true,
  "archive_sealed_ok": true,
  "archive_retention_ok": true,
  "n_threats": 1,
  "gate_passed": false
}
```

## ASI 9 键 LOCKED 真测

```json
{
  "keys_locked": true,
  "n_keys_present": 9,
  "fake_kpi_attempts": 2,
  "runner_confusion_attempts": 2,
  "v03_v04_confusion": 3,
  "n_threats": 3,
  "gate_passed": false
}
```

## 跨域威胁真测 (OWASP Top 10)

```json
{
  "input_pollution": 2,
  "side_channel": 0,
  "replay": 1,
  "key_leak": 2,
  "unauthorized": 2,
  "n_threats": 4,
  "gate_passed": true
}
```

## V3 守门 (主 17:58 不假装)

- module_is_not_safety: V1121 守门 ≠ 真安全
- threat_count_is_not_threat: 威胁计数是 proxy
- gate_pass_is_not_aligned: 守门通过 ≠ ASI 对齐

## 结论

- 状态: **RED**
- 威胁总数: 11
- 守门通过: 2/5
- 主 22:33 ASI 北极星 0.9800 LOCKED 未达, 真审查是 proxy

## V1121 真测试覆盖 (审查人产出)

- 测试文件: `tests/test_v1121_security_guard.py`
- 测试数: **33 passed** (0 failed, 0 skipped)
- 覆盖章节:
  - Block 1: ThreatModel & Severity — 4 tests
  - Block 2: IdentityGate (V1072 永恒身份守门) — 5 tests
  - Block 3: StoreGuard (V1095 Store 守门) — 4 tests
  - Block 4: DGMThreatModel (V1112 真演化威胁) — 5 tests
  - Block 5: ASINineKeysGuard (9 键 LOCKED) — 4 tests
  - Block 6: CrossDomainThreatSuite (OWASP Top 10) — 5 tests
  - Block 7: SecurityDashboard & Orchestrator — 3 tests
  - Block 8: End-to-End 真跑 + 报告 — 2 tests
  - Block 9: V3 守门 (主 17:58 不假装) — 1 test

## V1121 v0.1 审查发现 P1 真 bug (security_reviewer 产出)

- **位置**: `apeireth/v1121_security_guard_v01.py:776` `FAKE_KPI_PATTERNS[2]`
- **bug**: regex `r"\breached[_=]?\s*asi\b"` 字面 pattern 是 `\b` + `reached` (7 chars) + `[_=]?` + `\s*` + `asi` + `\b`, **缺 leading 'b'**
- **后果**: 字面 `\breached` (regex `\b` + `reached`) 永远不匹配 `breached asi` — 因为 'breached' 中 'reached' 从位置 1 开始, `\b` 在位置 1 之前不成立 (位置 0 和 1 都是 word char)
- **应改为**: `r"\bbreached[_=]?\s*asi\b"` (在 `\b` 后加 'b')
- **风险等级**: P1 (检测器盲点, 但不影响主 9 键 LOCKED 检查)
- **修复目标**: V1121 v0.2
- **审查人**: security_reviewer (R9-SEC-001)
- **测试覆盖**: `tests/test_v1121_security_guard.py::TestASINineKeysGuard::test_v1121_bug_breached_asi_regex_typo` 真测验证 bug 存在 + 修复后行为
