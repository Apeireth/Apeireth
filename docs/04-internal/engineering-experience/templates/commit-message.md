# 模板 · Commit Message（O-6 三阶审查）

```text
[Document-Meta]
Document:        docs/04-internal/engineering-experience/templates/commit-message.md
Version:         1.0（可复制形态）
Last-Modified:   2026-10-20
Status:          🟢 活跃
口径真源:        ENGINEER-MANIFESTO.md §5/§8.2（LOCKED 口径，本模板不改其义）
```

> 用法：复制下方代码块填实；commit-msg hook 会 BLOCK 缺三阶审查三段的 message。
> commit 前 author env 已设（MANIFESTO §13 #12）。

```text
<type>(<scope>): <一句话>

- 0 装诚实真账:
  - 测试: cargo test --workspace --all-targets --locked --no-fail-fast -j 2 = <N> passed / 0 failed（<日期>，log <路径>）
  - clippy: cargo clippy --workspace --all-targets --locked -- -D warnings = 0 警告（<日期>）
  - 0 触碰 LOCKED 5 项（9 哲学锚本体 + 13 键 + 3 不可变脊柱 + workspace.version + R11 baseline）
  - 0 引新外部 dep（Cargo.lock 0 行 diff / 或如实列依赖边与理由）
- O-6 三阶审查:
  - 总体最优: <更大语境（release 路线/工作量/上下游）里为何这是最优切入点？拒的 alternatives + 拒理由>
  - 系统最优: <子系统依赖图（governance → orchestration → memory → runtime → organ）里改动放哪层？拒的 alternatives + 拒理由>
  - 架构最优: <31 members 拓扑 + 单向依赖 + trait object 下 API 形状/crate 边界/0 新 dep 是否最优？拒的 alternatives + 拒理由>
- 影响面: <文件清单 + 行数；他线接缝报备指针>
- 后续 stage: <分阶段则列下一步；无则写「无」>
```

**type** ∈ feat/fix/refactor/docs/test/chore/perf/ci/build/revert；**scope** = crate 名或文档路径。

**反面教材（MANIFESTO §13 #4）**：三阶审查只描述 WHAT 不答 WHY = O-6 失守；不复用 v1 alignment 代替总体最优；每段必须有具体被拒的 alternative。
