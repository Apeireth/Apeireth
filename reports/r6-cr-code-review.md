# R6-CR-01 R6 新增模块代码审查

**审查人** code_reviewer | **性质** 只读, 不动源码/不重跑
**范围** R6-PHL-01/02/03, R6-BE-04, R3-DB-01 + 测试

## 模块结论

### 1. `self_reproduction.py` (117L) ✅
2 dataclass + 5-方法 Protocol + guard. `__post_init__` 校验 seed/path/modules>0. PHILOSOPHY_NOTES 拒 clone/perfect/uuid. 仅引 `.philosophy`. 测试 6 ✓.

### 2. `self_mod_safety.py` (126L) ⚠️ 缺测试
3 dataclass + 5-方法 Protocol + guard. `SafetyVerification.__post_init__` 验 `risk_score∈[0,1]`. guard 多带 `distinct_from_reproduction`. **HIGH: `test_r6_self_mod_safety_contract.py` 不存在**.

### 3. `formal_verify.py` (113L) ✅
`CONTRACT_ONLY=True` 显式非证明器. 2 dataclass + 5-方法 Protocol + guard. PHILOSOPHY_NOTES 三条 (spec≠proof/counterexample≠bug/prover≠truth). 引 TLA+/Lean/Dafny 不依赖. 测试 8 ✓.

### 4. `v1000_yaml_serializer.py` (303L) ✅ 真实现, 2 MED
`safe_load/safe_dump`, `_pre_dump` 处理 datetime/Path/Enum/dataclass/frozenset. `YAMLSerializerASIBridge` 暴露 metrics. 测试 52 ✓.
- **MED** `loads_all`: `_wrap` 包"创建 generator", mid-stream 错透传 YAMLError.
- **MED** `dump_stream`: 先 `dumps()` 物化再 write, 非真流式.
- **LOW** `YAMLMode.ROUND_TRIP` 接受但=SAFE, 误导.

### 5. `hqb/schema.py` (185L) ✅ 真实现, 类单测薄
`HqbStore` 用 stdlib sqlite3: 4 表 + hqb_meta. WAL+FK+Row. CREATE IF NOT EXISTS 幂等. FK CASCADE/SET NULL 正确. 仅 V1085/V1086 smoke 覆盖集成, 无类单测 (LOW).

## 真问题

| Pri | 模块 | 问题 | 建议 |
|-----|------|------|------|
| HIGH | self_mod_safety | `test_r6_self_mod_safety_contract.py` 缺失 | 补 ≥6 tests, 与 self_reproduction 同构 |
| MED | v1000_yaml | `loads_all` 错误包装失效 | 迭代时捕获或 lazy gen |
| MED | v1000_yaml | `dump_stream` 非真流式 | `yaml.dump(stream=target)` 直写 IO |
| LOW | v1000_yaml | ROUND_TRIP 行为=SAFE | 弃用或未装 ruamel 时 raise |
| LOW | v1000_yaml | PyYAML 隐式依赖 | `deploy_v1080/requirements.txt` 显式追加 |
| LOW | hqb/schema | HqbStore 无类单测 | 补 schema_version 幂等 / delta lift |

## 边界 ✓/✗

| 项 | 状态 |
|----|------|
| 不接 call_llm | ✓ |
| 不破坏 V1074/V1081 | ✓ |
| 不引入新依赖 | ⚠️ (PyYAML 隐式) |
| 命名空间冲突 | ✓ |
| import 循环 | ✓ |

证据: 5 模块无 LLM 导入 (yaml L19 "不动 llm_kernel" 负面声明); 无 V1074/V1081 import; `v1000_yaml_serializer` 命名唯一; 仅引 `.philosophy`/stdlib.

## 测试覆盖 (静态)

| 模块 | def test_ |
|------|----------|
| self_reproduction | 6 ✓ |
| self_mod_safety | 0 ✗ |
| formal_verify | 8 ✓ |
| v1000_yaml | 52 ✓ |

## 验收

≤ 3KB ✓ | 每模块 ≤ 200 字 ✓ | high/med/low ✓ | ✓/✗ 表 ✓ | 不动源码/commit/重跑 ✓

## 一行结论
哲学壳干净 (PHL-02 缺测试 HIGH); yaml 真实现但流式与多文档错误包装 2 MED; HQB 干净, 类单测薄. 边界守.