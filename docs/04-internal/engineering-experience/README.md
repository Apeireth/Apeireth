# engineering-experience —— 工程经验（提炼层 L3）

> **提炼流水线产物**：从 L4 在飞与 L5 卷宗里提炼出的可复用经验与教训。不是流水账。

**8 篇** + 1 个子目录（templates）。

| 篇 | 是什么 | 矿脉批次 |
|---|---|---|
| [`00-construction-handbook.md`](00-construction-handbook.md) | 施工者主手册（进场 → 施工 → 收工的一站式工序卡） | 总装批 |
| [`01-collaboration-patterns.md`](01-collaboration-patterns.md) | 协同模式矿 · 多 AI 并行施工的实战协同模式 | 批 L1 |
| [`02-failure-anatomy.md`](02-failure-anatomy.md) | 死亡解剖矿 · 会话死法与防复发药方 | 批 L1 |
| [`03-design-verdicts.md`](03-design-verdicts.md) | 设计裁定矿 · 本战役口径裁决集 | 批 L2 |
| [`04-methodology.md`](04-methodology.md) | 方法论矿 · 已验证配方与执行细则 | 批 L2 |
| [`05-product-insights.md`](05-product-insights.md) | 产品洞察矿 · 内测判词与产品级洞察 | 批 L3 |
| [`06-truth-and-guard.md`](06-truth-and-guard.md) | **真源矿** · 四级口径（「已实现 / 已接线」这类话）怎么保持为真（声明 / 棘轮 / 量具 / 知识库） | 批 L4 |
| [`07-workline-and-merge.md`](07-workline-and-merge.md) | **工作线矿** · 认领、去重、合并收口与接棒（工作线登记 / 判重 / 收口 / 交接） | 批 L4 |

---

## 怎么用（30 秒定位）

| 你此刻要做 | 先读哪一篇 |
|---|---|
| 开工前认领一条工作线 / 写进度页 | [07](07-workline-and-merge.md) 条目 1–3 · [00](00-construction-handbook.md) §1 工序卡 |
| 判断「这件事是不是已经有人做过了」 | [07](07-workline-and-merge.md) 条目 4–6 |
| 要写「已实现 / 已接线 / 数字 = N」 | [06](06-truth-and-guard.md) 条目 1–6 · [05](05-product-insights.md) 洞察 2（四级口径阶梯） |
| 要给一份文档配守卫 | [06](06-truth-and-guard.md) 条目 7–9 · [00](00-construction-handbook.md) §2 任务型导航 |
| 造量具 / 写探针 / 判判据 | [06](06-truth-and-guard.md) 条目 4、5、8、10、11 |
| 合并两条线 / 做收口批 | [07](07-workline-and-merge.md) 条目 7–12 |
| 写交付文档 / 做交接 | [04](04-methodology.md) §二 接棒遗嘱格式 · [07](07-workline-and-merge.md) 条目 2、3 |
| 会话要死 / 上下文要爆 | [02](02-failure-anatomy.md) · [04](04-methodology.md) §一 |

## 本体系的守卫（**不许退化成散文**）

| 守卫 | 判什么 | 命令 |
|---|---|---|
| [`scripts/check_experience_anchors.py`](../../../scripts/check_experience_anchors.py) | 条目段型（每型必填段）+ `★` 出处锚点可达 + 索引覆盖 + **人读层 ⇄ 机器层失败模式互引** | `python3 scripts/check_experience_anchors.py --check` |
| [`scripts/check_failure_knowledge.py`](../../../scripts/check_failure_knowledge.py) | 知识库引用的守卫必须**真实存在** | `python3 scripts/check_failure_knowledge.py` |

> **为什么有这两条**：本仓自己的标准是「**没有守卫的文档 = 散文；散文不能当进度用**」
> （见 [workline-registry.md](../../status/workline-registry.md) §三）。
> 本体系在此之前**一条守卫都没有** ⇒ 现在补上：
> 条目写成散段落 / 出处指到已删文件 / 机器层知识库与人读层各说各话 ⇒ **一律红**。

---

上层索引：[`../README.md`](../README.md)。
机器可读短指令：[`../../../AGENTS.md`](../../../AGENTS.md)。
