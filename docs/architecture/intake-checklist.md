# 功能准入清单（Architecture Intake Checklist）

> `Status: 📝 v0.1 草案（2026-10-20）`
> 用途：任何新能力/机制/结构进入仓库前，逐条回答；答案写进 commit 正文或 ADR。
> 依据：[架构契约](../architecture/contract.toml) · [ENGINEER-MANIFESTO §5 O-6](../../04-internal/ENGINEER-MANIFESTO.md) · [施工纪律 §一/§六](../../04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md)

## 0. 先分类：你加的是哪一类？

| 类 | 例子 | 通道 |
|---|---|---|
| **A 新能力/机制** | 新工具、新生物机制、新认知模块、新 provider | 复用既有扩展点 + 能力账 + 默认关 + 零变化测试 |
| **B 新结构** | 新 crate、新 trait/注册表、新总线、新 env 开关族、跨层依赖边 | **ADR + owner 拍板 + 改契约**（默认不给） |
| **C 新文档/口径** | 新规范、新状态表、新能力矩阵行 | 生成物或历史卷宗；禁止手抄真源 |

## 1. 准入八问（A 类必答，B 类答完整 ADR）

1. **层与所有权**：属于哪一层/哪个 crate？谁维护？（不得跨层直接依赖，见 contract `[layers]`/`[deps]`）
2. **扩展点**：复用 `[extension_points]` 的哪一个？如果是新扩展点 → 立刻转 B 类（ADR）。
3. **开关**：需要新 `APEIRETH_*` 开关吗？默认必须关；开关名、owner、默认值、层级、用途写进登记表；**关时逐字节零变化**怎么证（golden/zero-change 测试）。
4. **接线点**：信号从哪来（事件源）？状态所有者是谁？写路径/读路径在哪？拒绝"机制入库、信号源待接"的静默半成品。
5. **观测与审计**：`tool.self_status` 哪一节？审计/账本落在哪？没有观测口 = 没有交付。
6. **四级口径**：IMPLEMENTED / PRODUCTION WIRED / DEFAULT ENABLED / HARDWARE VALIDATED 标到哪级？证据（源码路径 + 测试路径 + 提交号）是什么？
7. **零变化与回滚**：功能关掉时行为与开关前逐字节一致吗？回滚路径是什么（删文件/关开关/回退 commit）？
8. **删除路径**：如果它失败，怎么删干净？（新结构必须给出"删除计划"，且新结构净额 ≤ 0）

## 2. B 类（新结构）额外要求

- ADR 必须含：O-6 三阶审查（总体/系统/架构，各带拒的 alternatives + 拒理由）、契约条款影响、迁移步骤、回滚步骤、预算净额（LOC/边/开关/文件数）。
- 同批更新：`contract.toml`（层表/边表/扩展点/预算）、`arch-baseline.txt`（只能缩）、`ARCHITECTURE.md`（生成物）。
- 默认结论是 **"不给"**：先证明现有结构装不下，再谈新增；新增 1 个结构必须同批删 ≥1 个旧结构（净零例外需 owner 特批 + 到期日）。

## 3. commit 正文模板（新增类必带）

```text
- 架构准入:
  - 类别: A 新能力 / B 新结构 / C 文档
  - 扩展点: <contract [extension_points] 条目>
  - 开关: <无 / 名字 + 默认值 + 零变化测试名>
  - 接线: 信号源=<...> 状态所有者=<...> 观测=<self_status 节>
  - 四级口径: <IMPLEMENTED / PRODUCTION WIRED / ...>  证据: <src:line + test:line + commit>
  - 删除路径: <...>
  - LOCKED 5 项 0 触碰 / 0 新外部 dep / arch_guard --strict 通过
```

## 4. 反例（会直接被守卫或评审拦下）

- 新 mechanism 直接读 `std::env::var`，不登记开关；
- 为图快复制一份 `ContextAssembler`/`SessionStore`/调度器；
- adapter 直连 engine 内部类型（跨层边，无 ADR）；
- 能力停在 IMPLEMENTED 级、不给接线点（机制入库、生产无消费者）；
- 新增 crate 但不同批更新契约/基线（结构净增）；
- 文档手写测试数字/能力状态（违反单一真源）。
