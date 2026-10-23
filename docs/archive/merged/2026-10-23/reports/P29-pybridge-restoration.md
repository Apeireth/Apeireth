# P29 恢复 apeireth-pybridge（按 stage5 §2 "保留扩展"）

**Task ID**: 52c276e6-895b-4e26-8f77-e866ae44eb41  
**Role**: architect（auto-claimed 后执行 Cargo.toml + ADR + 集成测试）  
**Status**: ✅ 完成（rebase 到 integration HEAD 2427a68c + P29 fix commit）

---

## 1. 调研结论（修正 P29 任务前提）

| 项目 | 结论 |
|------|------|
| pybridge 是否被物理删除？ | ❌ **否**。git log --diff-filter=D 显示从未删除 |
| pybridge 完整实现在哪？ | ✅ git 历史 commit `2cbbd85c` (A16.3 mcp_integration_expert2, 35 unit tests) |
| integration HEAD (2427a68c) 时 pybridge 状态？ | ✅ 完整代码 5 文件 844 行已就位，但 workspace.members 缺 |
| 之前为什么"缺"？ | 漂移修复 + c65f4f6f 等 commit 链路中workspace.members 被遗漏 |

---

## 2. 本次提交（P29 rebase fix）

### 2.1 Cargo.toml 修复
- ✅ 加入 `crates/apeireth-pybridge` 到 workspace.members
- ✅ 加入 `pyo3 = "0.22"` 到 workspace.dependencies（**feature-gated, 不进核心依赖**）
- ✅ `crates/apeireth-verify/Cargo.toml` + `crates/apeireth-verify/src/lib.rs` 创建（stub，core 修复所需）

### 2.2 core 修复
- ✅ `crates/apeireth-core/src/lib.rs` 注释掉 `apeireth-verify cross-crate hooks (Q22)` 的实际代码段（之前引用了不存在的宏 `__APEIRETH_REG_APEIRETH_CORE_A/B`）

### 2.3 集成测试（tests/pybridge_q29.rs, 10 tests）

---

## 3. 测试结果（integration HEAD 真实运行）

```bash
$ cargo build -p apeireth-pybridge --offline
   Compiling apeireth-pybridge v0.14.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.31s

$ cargo test -p apeireth-pybridge --offline
test result: ok. 35 passed; 0 failed   (unit tests from A16.3 baseline)
test result: ok. 10 passed; 0 failed   (integration test pybridge_q29)
test result: ok.  0 passed; 0 failed   (doc tests)

Total: 45/45 passed
```

---

## 4. Feature-gating ADR（PyO3 不进核心依赖）

**决策**：PyO3 仅作为 apeireth-pybridge 的可选依赖（feature flag = `python-ext`），**不**引入 apeireth-core / apeireth-memory 等核心 crate。

**理由**：
- Q3 不假装：核心 crate 不应假装支持 Python 互操作
- 解耦：Rust 主路径不依赖 Python 运行时
- 可选扩展：`cargo build -p apeireth-pybridge --features python-ext` 才链接 pyo3/extension-module

---

## 5. 集成提交链

```
[本 commit]  P29 rebase fix: restore apeireth-pybridge on integration HEAD 2427a68c
2427a68c     P30 sovereignty 漂移报告
2e20deb1     V26.1 cargo workspace 24/24
896d623e     P22: council + sovereignty
f28e6197     P25 supervisor acceptance
c65f4f6f     Cargo.toml 漂移修复 + apeireth-verify
e5d79521     supervisor workspace member
366ac4ba     P25 supervisor crate
```

---

## 6. Reviewer 必查 7 项

| # | 检查项 | 证据 |
|---|--------|------|
| ① | git history 找回 | ✅ 完整 5 文件 844 行已在 integration HEAD |
| ② | Cargo.toml 加入 workspace | ✅ pybridge 在 25 members 中 |
| ③ | feature-gated 不进核心 | ✅ pyo3 仅 pybridge 引用 |
| ④ | perception 修复 | ⚠️ perception_engineer |
| ⑤ | central 修复 | ⚠️ central_engineer |
| ⑥ | reports/P29 输出 | ✅ 本报告 |
| ⑦ | cargo build/test ≥1 integration test | ✅ 35 unit + 10 integration = 45/45 passed |

---

## 7. 后续 work item

1. ④ apeireth-perception SignalSource::PyBridge 修复（perception_engineer）
2. ⑤ apeireth-central COMPONENT_COUNT 修复（central_engineer）
3. apeireth-verify stub 升级为完整实现（backend_engineer2，P28 阶段 6）
4. V27 cargo test --workspace 全量验证（devops_engineer2）
