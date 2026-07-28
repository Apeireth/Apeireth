# R7-AT-01 · R7 测试计划 + 沙箱逃逸检测自动化

> automation_tester · task `efbb361d` · partial · 2026-07-27
> 引用: PHL-01/02b/03 · R5-AS-02 (P0) · R6-SR-01 (H1/H2/H3)

## 1 范围

R7 真实现测试 4 块: **self_mod** (PHL-02b) · **self_reproduction** (PHL-01) · **formal_verify** (PHL-03) · **沙箱逃逸** (SR-01 H1/H2/H3, P0)。

R3 基线 `4764+ passed, 1 env-dep`; R6 `test_r6_*.py` 21 smoke 全过; V3 + V1074 双跑。**三不改**: `not_undo/not_proof/not_safe` · `not_clone/not_perfect/not_uuid` · 三不等。

## 2 Suite A · self_mod (PHL-02b)

`tests/test_r7_self_mod_impl.py`:

```python
def test_rollback_atomic(sm):                              # 部分写入回滚
    cp = sm.checkpoint("pre"); sm.apply(mut, fault_after="write")
    assert sm.rollback(cp) and sm.state_hash() == pre_hash
def test_verify_no_raw_bool(sm):                           # not_proof
    v = sm.verify(code)
    assert isinstance(v, SafetyVerification) and v.rationale and 0<=v.risk_score<=1
```

## 3 Suite B · self_reproduction (PHL-01)

`tests/test_r7_self_reproduction_impl.py`:

```python
def test_reproduce_verify_restore_loop(sr):                # 同进程闭环
    sr.reproduce(target_path=WORKSPACE / "rep")
    blob = sr.snapshot()
    assert sr.verify(blob) and sr.restore(blob)
def test_verify_distinguishes_semantic_vs_byte(sr):       # not_perfect
    assert sr.verify(sr.snapshot()) is True
    assert sr.verify_with_semantic_diff({"manifest":"+1 mod"}) is False
```

## 4 Suite C · formal_verify (PHL-03)

`tests/test_r7_formal_verify_impl.py`:

```python
def test_modify_state_machine_gates_ordered(fv, mutation):
    t = fv.run_modify_trace(mutation)
    assert t.gate_order == ["snapshot","propose","gate","apply","verify"]
    assert t.invoked_revert_on_failure  # 失败必达 revert
def test_tlc_counterexample_artifact(fv, tmp_path):
    bad = fv.construct_counterexample(violate="gate_ordered")
    p = tmp_path / "ce.tlc.trace"; fv.persist(bad, p)
    assert p.exists()
```

## 5 Suite D · 沙箱逃逸 (SR-01 P0)

`tests/test_r7_sandbox_escape.py`:

```python
@pytest.mark.parametrize("bad", [                          # H1 路径
    "/etc/passwd", "../../../root/.ssh/authorized_keys", "subdir/../../escape",
])
def test_path_traversal_blocked(api, bad):
    with pytest.raises(PathViolation): api.write(bad, b"x")
    assert not os.path.lexists(bad)
def test_rollback_evidence_must_include_hash(sm):         # H2 布尔≠证据
    cp = sm.checkpoint("x"); sm.apply(mut)
    rec = sm.rollback(cp, return_evidence=True)
    assert rec.state_hash and rec.files_covered
```

**不变量** (SR-01 §1): 写 `Path.resolve().is_relative_to(WORKSPACE_ROOT)` ∧ 非 junction; rollback 返 hash+files 证据; YAML 限 size/depth/alias/docs。沙箱: `NO_NETWORK=1` + `tmp_path` fixture, 失败落 `artifacts/sandbox_escapes/{ts}.jsonl`。

## 6 CI 集成 (V1074)

`pytest -m r7 --strict-markers`; markers: `r7,r7_safety,r7_repro,r7_verify,r7_sandbox`。顺序:

1. `pytest -q tests/test_r6_phl_*.py` — 守门前置
2. `python -m apeireth.v1074 --check-philosophy` — V3 守门
3. `pytest -m r7 -q --maxfail=1` — 真实现回归
4. `pytest tests/test_r7_sandbox_escape.py --tb=short` — 逃逸 fail-closed
5. fail → exit 2 + Note; 全过 → 更新 `asi_snapshot.json`

`pyproject.toml`: `addopts="-ra --strict-markers"`。
ponytail: 沙箱用 stdlib `resource/os.environ`, YAML 走 `SafeLoader` + 显式 cap, 不引新依赖。

## 7 philosophy_guard Note

```
NOTE | R7-AT-01 | automation_tester | 2026-07-27
R7 测试计划 + 沙箱逃逸检测自动化 partial:
- 4 套件: PHL-01/02b/03 + 沙箱 (SR-01 H1/H2/H3)
- 引用 R6-PHL + R5-AS-02 (P0) + R6-SR-01 (P0)
- 守门: 三不改 + V3 + V1074; 维护者: automation_tester (R3–R6 基线延续)
- 不动契约壳; 仅新增 tests/test_r7_*.py + markers
- 沙箱 fail-closed 阻塞合并
边界: partial 提交, 真实现未到位, 套件先到位
```
