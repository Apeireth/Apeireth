# 模板 · 交接报告骨架（Handoff Report Skeleton）

```text
[Document-Meta]
Document:        handoff-<主题>-<日期>.md
Version:         1.0
Last-Modified:   <YYYY-MM-DD>
Status:          🟢 交付记录
Author:          <交接方>
口径真源:        实形 = handoff-w1-w5-closure-2026-10-10.md / engineering-review-handoff-2026-10-06.md
```

> 给谁看：接续方（人或 AI）——**不需要读过任何对话记录，本文自足**。
> 交接方三件事：复核入口给足 / 未验证边界说清 / 防复蹈教训留全。

## 〇、抬头

- **上一份交接**：<路径>（本文与它的关系：接续/收官/并行）
- **状态总纲**：一句话（全部闭环 / 剩 N 件，阻塞点各是什么）
- **HEAD**：`<sha>`（main，是否已推 origin）；工作树脏面归属（他人 在途 ≠ 本批）
- **口径**：四级诚实口径（system-capabilities.md 头注）——每条「已完成」都落到四级之一

## 一、交付总账（批次表）

| # | 批次 | 一线 | commit | 口径级 |
|---|---|---|---|---|

## 二、各战线终局（四级口径表）

| 战线 | IMPLEMENTED | PRODUCTION WIRED | DEFAULT ENABLED | 开关/命令 | 备注 |
|---|---|---|---|---|---|

## 三、结论复核矩阵（接续方逐条亲验，不要信文字）

> 用法：每条给命令 + **预期输出**；判定栏由接续方填：✅ 证实 / ⚠️ 部分正确（附修正）/ ❌ 推翻（附反证）。
> 判「未接线」走三关（名字扫描 / crate 内部互调 / Option 的 Default 值）。

| # | 结论 | 复核命令 | 预期输出 | 判定 |
|---|---|---|---|---|

## 四、我**没有**验证的东西（边界如实）

- live E2E 未实测项 / 需硬件项 / 需人工点击项 / 挂账行指针（live-verification-ledger §2）

## 五、剩余队列与阻塞点（按可操作性排序）

| 项 | 阻塞点 / 解锁条件 | 建议第一刀 |
|---|---|---|

## 六、防复蹈教训（本 span 真机咬出的坑，按编号可引用）

1. **症状 → 根因 → 修法**（例：AppContainer 环境块三连 203；win.ini AAP 自放行；`forbid(unsafe_code)` crate 里 set_var 违禁……）

## 七、接续方第一步（建议顺序）

1. 跑 §三 全量验证 + `git status` 对脏面归属
2. 读 live-verification-ledger 对应台账行
3. 从 §五 第一行开工
4. 新模块按**五件验收门**：默认关旋钮 / 默认关不变测试 / 效果可见测试 / 四级口径行 / 台账条目
