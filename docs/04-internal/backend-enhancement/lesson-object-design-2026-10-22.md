# 教训对象（Lesson）设计页 —— 让「成长」有可验证的载体

> 日期：2026-10-22 · 状态：**设计/待拍板**（本页不动生产代码）
> 归属线：记忆线（storage/memory）+ 自述线（runtime-assembly）
> 缘起：2026-10-22 外部审阅（`Apeireth-从愿景到创造路线.md`）的核心建议——
> 「承认一条教训不成立，也应当算作成长」；现状只有参数移动日志，没有「教训」这个一级对象。

## 0. 结论（TL;DR）

**建议：新增 `lessons` 表 + 四拍状态机，默认不注入上下文，只在被召回/被点名时进。**

- 四拍：**经历 → 候选教训 → 新场景验证 → 保留 / 撤回**（撤回也算成长，必须留痕）；
- 落点：`crates/engine/memory`（新表 + 治理面复用既有 `revision` / `status` 模式），
  **不新开 crate、不进 core**（教训是产品语义，不是内核语义）；
- 自述面：`tool.self_status` 增「最近学到的」一节，**只读**，数据来自 `lessons`；
- 注入纪律：默认**零注入**；被召回（相关场景）或点名时按 L2 封套进上下文，
  带「来源经历 / 验证场景 / 是否已撤回」三元证据，**不许无来源断言**；
- 与调参日志的关系：`tuning-log` 记「参数动了」，`lessons` 记「认知变了」——两者独立，
  不互相冒充（参数移动 ≠ 学到东西）。

## 1. 现状（为什么现在做）

| 已有 | 缺的 |
|---|---|
| `tuning-log.jsonl`：自校准参数移动、锁定、周报 | 没有「哪一次经历促成了改变」 |
| `traits` / `trait-locks`：性格特质与锁定 | 没有「哪条判断被新场景证伪/证实」 |
| 记忆治理面：`update_episode_content` / `forget` / `protect`（rev CAS） | 没有「教训」这个对象，更没有「撤回」的正式通道 |
| 自述面：能力名册 / 记忆计数 / 调参状态 | 没有「我最近学到了什么」的诚实回答 |

后果：问「你最近学到了什么」，只能答「参数动了 7 次」——这是日志，不是成长。

## 2. 对象定义

```text
Lesson {
  id:            TEXT PRIMARY KEY          -- lsn-<ulid>
  statement:     TEXT NOT NULL             -- 一句话教训（可执行、可证伪）
  scope:         TEXT NOT NULL             -- 适用场景（问题类/文件类/交互类…，闭合词表）
  source_episodes: TEXT NOT NULL           -- JSON 数组：促成这条教训的经历 id（≥1，必填）
  evidence:      TEXT NOT NULL             -- 当时的证据摘要（错误码/用户纠正原话/测试结果）
  status:        TEXT NOT NULL             -- candidate | verified | retracted
  verified_by:   TEXT                      -- 新场景验证：episode id 或测试/演练 id
  verified_at:   INTEGER
  retract_reason: TEXT                     -- 撤回必填（撤回也是成长，留痕）
  revision:      INTEGER NOT NULL DEFAULT 0
  created_at:    INTEGER NOT NULL
  updated_at:    INTEGER NOT NULL
}
```

**硬约束（测试钉死）**：

1. `source_episodes` 不得为空 —— 没有出处的「教训」是幻觉，写入即拒；
2. `status = retracted` 必须有 `retract_reason`；
3. `status = verified` 必须有 `verified_by` + `verified_at`，且 `verified_by ≠ source_episodes`
   （**不能用产生教训的那次经历自证**——这是四拍里最容易糊弄的一步）；
4. 教训内容变更走 `revision` CAS（与记忆治理面同款），旧版留痕。

## 3. 四拍状态机

```
经历(source_episodes) ──append_candidate──▶ candidate
                                            │
               新场景验证(verified_by) ──────▶ verified
                                            │
                  证据被推翻/场景失效 ──────▶ retracted（必填 reason）
                                            │
                                    （撤回后可再提新候选，同一 scope 允许并存）
```

- **候选不注入上下文**（避免模型把未验证猜测当事实）；
- **verified 才可被召回**，且召回时必须带三元证据（来源 / 验证 / 状态）；
- **retracted 永久保留**：它是「我错过、我改了」的证据，不是垃圾。

## 4. 接口（最小面）

| 接口 | 语义 | 调用方 |
|---|---|---|
| `append_lesson_candidate(statement, scope, source_episodes, evidence)` | 只允许从**真实经历**提起候选 | 反思步 / 用户显式 |
| `verify_lesson(id, verified_by, expected_rev)` | 新场景验证（`verified_by` 必须是一次新经历/演练，不能是来源） | 反思步 / 用户确认 |
| `retract_lesson(id, reason, expected_rev)` | 撤回（留痕） | 反思步 / 用户 |
| `list_lessons(scope?, status?, limit)` | 自述与面板读 | `tool.self_status` / 面板 |
| `recent_lessons_for_context(scope, limit)` | 上下文注入（只取 verified，L2 封套） | 上下文组装 |

**不做**：不提供「教训 → 自动改人格/自动改治理」的通道；教训只供**人看 + 模型读**，
任何权力变更仍走既有 grant/approval 面。

## 5. 与「纠正」的闭环关系（为什么先做纠正是对的）

审阅里那句「你说刚才其实还没结束，它修正判断」需要两个东西：

1. 能改记忆 → **已接线**（`memory.update` ← `update_episode_content`，本批交付）；
2. 能从「改了什么」提炼教训 → 本页。

顺序不能反：先有可纠正的事实，才有可验证的教训。若只有教训没有纠正，
「验证」就退化成「模型自己说自己对了」。

## 6. 验收计划（交付时必带）

- 单测：四条硬约束逐条红/绿；状态机非法迁移拒绝；rev CAS 冲突拒绝；
- 闭环 e2e：一次真实纠正（`memory.update`）→ 提炼候选（来源指回那次纠正）→
  新场景验证（`verified_by` ≠ 来源）→ 自述面「最近学到的」出现该条且带三元证据；
- 撤回 e2e：撤回后自述面标「已撤回」且不再注入上下文，**但记录仍在**；
- 0 装证据：默认零教训 = 自述该节显式「暂无」+ 原因（不空着、不编造）；
- 文档：矩阵/台账新增一行，claims 视裁定决定是否新增 claim。

## 7. 需拍板的三个问题

1. **谁来提候选**：只允许反思步（自动）？还是也允许用户显式「记住这条教训」？
   建议：两者都允许，但用户显式提的默认 `verified`（人即证据），反思步提的必须走验证。
2. **scope 词表**：复用现有 session 分类，还是新开闭合词表？建议新开**极小**词表
   （问题类 / 文件类 / 交互类 / 工具类），宁少不多，避免变成自由文本标签。
3. **注入预算**：verified 教训进上下文的名额（建议 ≤3 条/回合，且总字符 ≤ 800），
   超预算按「最近验证」截断并显式标注省略——沿用现有 L2 封套纪律。

## 8. 分期

| 期 | 内容 | 规模 |
|---|---|---|
| L1 | 表 + 四拍状态机 + 五接口 + 硬约束测试 | 1 个提交批（约 400 行含测试） |
| L2 | 自述面「最近学到的」只读节 + 面板列表 | 1 批 |
| L3 | 反思步接入（从纠正/失败中提候选） | 1 批，需与反思调度器同窗口 |
| L4 | 上下文注入（只 verified、L2 封套、预算） | 1 批，需 owner 拍板注入纪律 |
