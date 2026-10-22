# 模板 · 子代理 Brief（派工四要素）

```text
[Document-Meta]
Document:        docs/04-internal/engineering-experience/templates/subagent-brief.md
Version:         1.0（可复制形态）
Last-Modified:   2026-10-20
Status:          🟢 活跃
口径真源:        ENGINEER-MANIFESTO.md §6（workflow + 3 问 + 亲验义务）
```

> 派前 3 问（答不清不派）：①明确产出？②明确边界（改什么/不改什么）？③为什么派而不是自己做？
> 派前自己先跑基线；派后**必亲验报告**（子代理报"完成" ≠ 主代理已验证）。

```text
任务: <明确产出，不是模糊方向。例：调研 X 的 5 个缺口真生产路径，写报告 docs/04-internal/<名>.md>
必读: <文档 file:line / 源码路径，按读序排列；注明取材口径（限读/剥装饰/净化）>
必跑: <命令 + 期望输出。例：cargo test -p <crate> --test <t> 期望 N passed>
必写: <报告结构：结论（实测/推断/未验三分标注）+ 证据（命令+日期）+ alternatives + 0 触碰 LOCKED 声明>
不 commit: 等主代理审（Q1 C1 policy）
```

**接回后亲验清单**：
- [ ] 产出物存在且结构符合「必写」
- [ ] 每个数字/结论有命令可复跑，三分标注齐
- [ ] 抽查 1-2 条声明亲跑亲读（防 0 装诱导；ready 无 closing message 时主代理自验，不等）
- [ ] LOCKED 0 触碰自 grep 一遍 diff
