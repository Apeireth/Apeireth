# R6-PHL-01 自繁殖契约壳 验收报告

## 产出与 LOC

| 文件 | LOC | 角色 |
|---|---|---|
| `apeireth/self_reproduction.py` | 115 | Protocol + dataclass + V3 守门 |
| `tests/test_r6_self_reproduction_contract.py` | 93 | 6 烟测契约 |
| **合计** | **208** | — |

## 5 方法契约清单

```
@runtime_checkable
class SelfReproductionProtocol(Protocol):
    def snapshot(self) -> bytes           # 当前状态字节快照
    def verify(self, snapshot: bytes) -> bool    # 验证完整 + 语义不变
    def restore(self, snapshot: bytes) -> bool   # 从快照恢复
    def reproduce(self, target_path: str) -> str # 在 target_path 重建自身
    def reproduction_id(self) -> str             # 含模块清单哈希的 ID
```

配套 `ReproductionSpec(seed: bytes, target_path: str, expected_modules: int)` 与 `ReproductionResult(success, reproduction_id, diff_summary, manifest_delta)`,均在 `__post_init__` 校验不变量。

## 哲学守门引用

`PHILOSOPHY_NOTES` 三不 (主 17:58):
- `not_clone`: reproduction != clone
- `not_perfect`: 允许 manifest 差异, 不允许语义差异
- `not_uuid`: reproduction_id 必须含模块清单哈希

`guard_self_reproduction()` 调用 V3 `check_philosophy(module_name, summary, claimed_pass=None, evidence=summary, categories=[contract_shell,no_real_impl,philosophy_referenced], required_categories=[...])`。守门自检 PASS,0 deviations。

## 6 烟测简表

| # | 测试 | 验证 |
|---|---|---|
| 1 | protocol_exists | Protocol 在模块 |
| 2 | protocol_methods | 5 方法签名完整 |
| 3 | dataclasses | ReproductionSpec/Result 可构造 + 不变量 |
| 4 | no_real_implementation_yet | 不暴露真繁殖 callable |
| 5 | philosophy_guard_imports | V3 守门 PASS, 引用三不 |
| 6 | module_in_apeireth | apeireth.self_reproduction 命名空间 |

## pytest 摘录

```
tests/test_r6_self_reproduction_contract.py ........ [100%]
========================= 6 passed in 0.21s =========================
```

## R7+ 验收标准

真实现时: `verify` 必须区分语义差异 vs 字节差异; `reproduce` 的 `target_path` 产物必须能被同一进程再次 `verify`+`restore` 闭环; `reproduction_id` 含 `sha256(module_manifest)`。届时新增 `tests/test_r6_self_reproduction_impl.py`。

## 关联

- V1080 reproducibility: 跨进程语义一致
- V1074 measurement: 守门 PASS 已纳入 ASI 真测
- `apeireth/philosophy.py`: V3 `check_philosophy` 接口
