# 模板 · 批次交付文档骨架（Delivery Doc Skeleton）

```text
[Document-Meta]
Document:        <批名>-delivery-<日期>.md
Version:         1.0
Last-Modified:   <YYYY-MM-DD>
Status:          🟢 交付记录 / 🟡 在途
Author:          <施工线/代理 ID>
口径真源:        实形先例 = backend-enhancement/ 各 *-delivery.md 卷宗；宪章 = CONSTRUCTION-DISCIPLINE.md
```

> 给谁看：接续本批的人或 AI——**不读任何对话记录也能接手**。
> 纪律：每节如实；没有的东西写「暂无」；数字随行三件（命令 + 日期 + 工具链）。

## 〇、抬头四注（缺一不可）

- **接缝**：（X 并行线，已兑现其报备 / 本批无跨线接缝）——形状 + 词表 + 谁供谁消费
- **write_scope**：本批动的文件清单（他线在途脏文件 = 未碰，列出以示清白）
- **四级口径**：本批每个能力落在哪一级（IMPLEMENTED / PRODUCTION WIRED / DEFAULT ENABLED / HARDWARE VALIDATED）
- **取材口径**（如涉外部素材）：净化 / 限读 / 剥装饰声明

## 一、本批交付总账

| # | 件 | 落点（文件:行 / commit） | 口径级 | 默认开关 |
|---|---|---|---|---|

## 二、测试表（钉行为不钉字面）

| 测试名 | 钉什么行为 | 例数 |
|---|---|---|

## 三、门禁证据（全量，判定基于落盘 log）

- `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` → `<N> passed / 0 failed`（日期，log 路径）
- `cargo clippy --workspace --all-targets --locked -- -D warnings` → 0 警告（日期）
- `cargo fmt --check` → 0 diff；前端 `pnpm check`/`pnpm test` → <N/N>
- 中性扫描 0 命中（豁免计数：<N>，结构性）；secret-scan 过
- **红账定性行**（如有）：`<症状> → 隔离复跑 <结果> → 定性 <flake 并行扰动/真flake已定根/他线在途> → 归属线`

## 四、数字口径注（链路自洽对账）

真源 [test-baseline.md](../../../03-reference/test-baseline.md) 现行值 ↔ 本批复测值 ↔ 本批表值，逐列可加；
他口径差异（审计者自计/静态计数）如实注明定义不同。**未实跑不发布数字。**

## 五、残余与边界（如实）

| 残余 | 性质（未接线/未实测/待拍板/环境债） | 归口 |
|---|---|---|

## 六、接棒清单（未完）

（每条四要素：落点文件 / 注册条件 / 现成零件 / 验收形状——格式见 resumption-entry.md；
收口后就地标 ✅。无未完写「无」，不留悬空节。）
