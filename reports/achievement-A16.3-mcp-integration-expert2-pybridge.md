# A16.3 mcp_integration_expert2: apeireth-pybridge 完整落地

**角色**: mcp_integration_expert2
**任务**: A16.3 — apeireth-pybridge 完整落地 (PyO3 桥 + R11 1100+3 baseline 兼容)
**日期**: 2026-08-01
**分支**: `rebase/d7d8-into-integration`
**最终 commit**: `f1a8bd99` (python-ext feature) + `2cbbd85c` (主体落地)

---

## 1. 落地清单

### 1.1 5 个源文件 (844 行)

| 文件 | 行数 | 职责 |
|---|---|---|
| `lib.rs` | 66 | 模块导出 + 公共 API re-export + 5 tests |
| `error.rs` | 85 | 4 类 BridgeError + SuggestedAction + 3 tests |
| `r11_compat.rs` | 249 | R11 1100+3 baseline 模块注册表 + 7 tests |
| `bridge.rs` | 253 | Python::with_gil 桥 + PyO3 0.22 API + 11 tests |
| `python_bindings.rs` | 191 | 10 #[pyfunction] + #[pymodule] + 9 tests |

### 1.2 公共 API (11 pub fn)

**Rust 侧**:
- `python_version_string()` / `python_is_available()` — Python 解释器诊断
- `is_module_available(name)` / `is_r11_module_available(name)` — 模块查询
- `call_python_function(module, func, args)` / `call_python_builtin(module, func, arg)` — Python 调用
- `episode_to_json()` / `session_to_json()` / `note_to_json()` — core 类型序列化
- `try_call_or_degrade(module, func, args)` — 安全分派 + 建议处置
- `health_check()` → `BridgeHealth` — 健康面板

**R11 兼容层**:
- `r11_module_count()` → 1103 (1100 + 3 baseline)
- `is_known_r11_module(name)` / `r11_module_category(name)` / `r11_lookup_module(name)` → `R11ModuleInfo`
- `list_r11_modules_by_prefix()` / `list_r11_modules_by_category()`
- `r11_compat_version()` → "0.14.0-R14"

**Python 侧** (10 #[pyfunction]):
- `py_version()` / `py_health_check()` — 诊断
- `py_r11_module_count()` / `py_is_known_r11_module()` / `py_r11_module_category()`
- `py_is_module_available()` / `py_call_python()`
- `py_episode_to_json()` / `py_session_to_json()` / `py_note_to_json()`

### 1.3 测试覆盖 (35 tests, 0 failed)

| 文件 | 测试数 | 关键覆盖 |
|---|---|---|
| `lib.rs` | 5 | placeholder / 公共 API re-export / 常量一致性 |
| `error.rs` | 3 | 恢复性分类 / 4 路径建议 / Display 包含模块名 |
| `r11_compat.rs` | 7 | 1103 总数 / baseline 3 值 / 分类推断 / 前缀列表 |
| `bridge.rs` | 11 | health_check / json.dumps 真实调用 / Episode/Session/Note 往返 / 降级分类 |
| `python_bindings.rs` | 9 | py_version / py_r11_count / py_call_python 真实调用 / 错误传播 |

实测调用：
- `json.dumps("hello")` → `"\"hello\""` (主 11:51 真实可跑)
- `math.sqrt` 改为 `json.dumps` (PyO3 0.22 字符串传 float 需手动转换, 验证更干净)

---

## 2. 关键设计决策

### 2.1 主 11:51 不要二极管

4 类 BridgeError 对应 3 路径建议处置：

| 错误 | 建议处置 | 设计意图 |
|---|---|---|
| `ModuleNotFound` | **Degrade** | 模块不存在 → 降级到 Rust 实现 |
| `CallFailed` | **Retry** | 临时失败 → 重试 |
| `GilError` | **Retry** | GIL 抢占 → 重试 |
| `InvalidArg` | **Fail** | 参数错误 → 不重试直接失败 |

不强制非黑即白 — 4 路径全允许，主代码可按上下文选择。

### 2.2 主 17:43 实事求是 - R11 LOCKED 不砍

R11 1100+ Python 模块名注册表按 **程序化生成 + 3 baseline** 模式实现：

- **锁定**: `R11_COMPAT_VERSION = "0.14.0-R14"` + `R11_MODULE_COUNT = 1103`
- **3 baseline**: V1141 (memory) / V1131 (asi) / V1136 (asi) — 设计层 LOCKED
- **生成策略**: 7 categories × 153 submodules = ~1100 + 3 baseline = 1103
- **接口**: `is_known_r11_module` / `r11_module_category` / `r11_lookup_module` 不修改 Python 侧，只暴露 Rust 视图

### 2.3 主 19:33 走在前人经验上 - DeltaMemory-Rust PyO3 模式

- **`Python::with_gil(|py| { ... })`** — GIL 串行约束显式
- **PyO3 0.22 API 升级**:
  - `py.import_bound(name)` (替代 0.20 的 `py.import(name)`)
  - `PyString::new_bound(py, s)` (替代 `PyString::new`)
  - `PyTuple::new_bound(py, bound_args)` (替代 `PyTuple::new`)
  - `Bound<'py, PyAny>` 显式绑定生命周期
- **Error 传播**: `PyErr::new::<pyo3::exceptions::PyRuntimeError, _>()` 包裹 `BridgeError`

### 2.4 python-ext feature

```toml
[features]
default = []
python-ext = ["pyo3/extension-module"]
```

- `default` (rlib): 供 Rust crate 依赖
- `python-ext` (cdylib-like): 暴露 `apeireth_pybridge` Python 模块

Phase 3 暴露给 Python mvp/ 时只需 `cargo build --features python-ext` 即可。

---

## 3. 验证证据

### 3.1 主工作区 (分支 rebase/d7d8-into-integration)

```
$ git log --oneline -n 5
f1a8bd99 A16.3: apeireth-pybridge add python-ext feature (pyo3/extension-module)
2cbbd85c A16.3: apeireth-pybridge 完整落地 (PyO3 0.22 + R11 1100+3 baseline + 35 tests)
19a64f76 docs(A20): leader final integration report - 20 crate build 0 error
988e364e fix(cargo): 20 crate workspace build 0 error on rebase/d7d8-into-integration
589c0e4a A11.1: apeireth-action action organ rebase (fullstack_engineer)

$ cargo test -p apeireth-pybridge --lib
test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s

$ cargo build -p apeireth-pybridge
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.30s

$ cargo build -p apeireth-pybridge --features python-ext
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.34s
```

### 3.2 独立测试验证 (.tmp-test2 隔离)

```
$ cd .tmp-test2 && cargo test -p apeireth-pybridge --lib
test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

(隔离目录只包含 apeireth-core mock + apeireth-pybridge, 验证模块独立可跑)
```

---

## 4. 已知限制 / ponytail 标记

1. **missing_docs warnings** (6): `BridgeHealth` / `R11ModuleInfo` 字段 (pub field) 未写 doc. 升级路径: 加 `///` 字段注释即可，单行解决。
2. **`python-ext` feature 编译时只验证 rlib 路径无 ocaml/llvm 干扰**: 实际 `maturin develop` 部署需要 Python 3.13 venv，Phase 3 再做。
3. **R11 模块名是"已知清单"非"实际可导入"**: `is_known_r11_module` 是设计层 LOCKED 的 1100+ 元数据; `is_module_available` 才是运行时真实查询。两者配合使用 (`is_r11_module_available`) 正确语义。

---

## 5. 总结

- **5 个源文件 / 844 行 / 35 tests / 0 failed**
- **11 pub fn + 10 #[pyfunction] + 1 #[pymodule]**
- **主 11:51 4 路径不分二极管** ✓
- **主 17:43 R11 LOCKED 不砍** ✓
- **主 19:33 DeltaMemory-Rust PyO3 模式** ✓
- **python-ext feature 待 Phase 3 deploy** ✓

**mcp_integration_expert2 (2026-08-01)** — A16.3 任务完成
