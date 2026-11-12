# 记忆质量与持久一致性

[Document-Meta]
Document: docs/design/memory-quality-hardening.md
Version: 1.0
Class: SPEC
Last-Modified: 2026-11-12
Status: 🟢 活跃
Author: lead

## 目标与范围

以本项目的用户需求、现有存储契约与可验证不变量补强长期记忆。
沿用 [D0–D5 设计](../01-architecture/memory-knowledge-depth-system-spec.md)
和[既有质量路线](../status/memory-quality-route-map-2026-10-27.md)，不新增平行记忆系统。
事实正确、作用域隔离和故障后的可解释状态优先于召回数量。
本批不导入外部项目代码、文档片段、提示词或评测数据；不改写既有研究历史。

验收沿用十八项质量指标，但必须区分算法库测试、真实装配路径与模型回答质量。
本地确定性测试不能证明真实模型的最终答题能力，也不能证明优于尚未运行的外部系统。

## 不变量

1. 持久写入失败不得把未提交记录发布为工作记忆；重试相同稳定 ID 不应挤占工作环。
2. SQLite 情节、出处及作用域元数据必须在同一事务提交；原文哈希链继续接受完整性验证。
3. 用户显式纠正应保留可追溯证据；模型回复、引用和低可信输入不能仅凭出现顺序覆盖事实。
4. 相同主题不等于相同事实键；主体、作用域、角色及版本必须有确定边界。
5. 未配置、无法测量或测试失败必须如实记录；不调整历史质量阈值来获得通过。

### 判定基础

身份按结构元组 `(class, scope, content)` 比较，显示标签仅用于诊断。
两个 SQLite 与 InMemory 组合入口中，同一稳定 ID、原文和归属的重放满足
幂等条件 `W(e, W(e, state)) = W(e, state)`，
其中 `state` 同时包含持久证据、元数据、链头及工作环；派生访问计数等既存字段保留。
一次组合写入失败时，SQLite 的上述受管持久状态保持提交前值，Working 不发布新项。
本批用明确反例、故障注入和软件回归验证这些性质；尚未执行全状态空间形式化模型检查。

## 批次与验收

| 批次 | 交付 | 验收 |
|---|---|---|
| Q1 持久一致性 | 复用 backend/coordinator，补原子存储与提交后发布 | 故障注入；SQLite 回滚；稳定 ID 重试；重启后元数据与哈希链一致 |
| Q2 中文生产路径评测 | 在既有 Runtime → SQLite → Provider 测试面补中文多轮场景 | 检查真实送达证据、来源和隔离；关键断言旧实现红、新实现绿 |
| Q3 事实纠错 | 基于 Q2 的失败证据选择最小修复 | 显式纠正、生效边界、旧证据保留、错误主体与角色对抗测试 |
| Q4 公平比较 | 固定语料、模型、预算、参数及重复策略 | 双方实例与配置齐备后才出比较；缺一方记未测 |

### 比较口径

比较前固定中文多轮样本与保留样本，按会话划分开发集和测试集；标签不得进入查询、索引或排序参数。
双方使用同一模型版本、输入与输出预算、记忆容量、召回名额、注入预算和可用工具，记录配置与语料摘要。
分别报告事实正确、纠错与时间正确、证据覆盖、噪声鲁棒、权限隔离、重启恢复、延迟与费用。
错误主体、已遗忘事实重新送达或来源伪造属于硬失败，不能被平均分抵消。
有模型随机性时报告多次配对结果及不确定性；未完成双方同条件测试，不发布领先结论。

确定性回归通过率只表示固定案例满足契约。RecordingProvider 只观测真实请求中的证据，
不生成被测答案；短案例也不构成饱和预算或大规模性能验收。

## 当前事实与边界

- `MemoryCoordinator::persist_episode` 在本批开始时先更新工作环，再分别写情节和元数据。
  这一顺序存在部分成功状态；本批通过故障注入验证具体后果后修复。
- 默认物化的一般事实调和未加载持久快照；承诺、人格、双时间关系和人工 CAS 已有实现。
  不把这些已有能力重复建设，也不把字符串相似度当作可信事实身份。
- 新物化 ID 的摘要输入改为结构化编码，既有存储 ID 不迁移；旧记录仍按原 ID 读取。
  本批未验证跨版本重新物化同一输入的 ID 等价性。
- 既有运行时测试已覆盖 SQLite 持久化、重启和 provider 请求中的实际记忆证据。
  新评测延伸这些入口，不另建排名器来替代生产路径。
- 本批不改并行工作线的 gateway 回合结果恢复文件，不实现 OS 驱动或原生桌面动作。

## 验收入口

```sh
cargo test -p apeireth-memory --test memory_writeback_consistency --locked -j 2
cargo test -p apeireth-memory --test memory_reconciliation_identity --locked -j 2
cargo test -p apeireth-memory --test conversation_memory_quality --locked -j 2
cargo test -p apeireth-runtime-assembly --test memory_provider_e2e --locked -j 2
python scripts/check_quality_metrics.py
python scripts/check_golden_baseline.py
```

Windows 跑小说基线时显式设置包含 `novels/` 的仓外数据根，保持 Python 与 Rust 评测器一致：

```powershell
$env:APEIRETH_DATA_DIR = Join-Path $env:USERPROFILE '.apeireth'
python scripts/check_golden_baseline.py
```

这些测试分别覆盖事务故障、身份边界、中文治理生命周期和真实 Provider 接线。
Provider 测试从独立 SQLite 读取发送 ID 对应的治理状态、作用域和出处；
不再把一个集合复制成“检索 / 选择 / 发送”三份后比较自身。
遗忘测试验证隐藏与不再送达，原始审计证据仍保留；它不证明物理销毁或备份清除。

## 进度与验收

| 项目 | 当前状态 | 证据 / 还差什么 |
|---|---|---|
| Q1 | PRODUCTION WIRED | 默认协调器调用组合写；两 SQLite 原文、sidecar、链头同事务，成功新插入后发布 Working；既存 ID 不回填工作环。故障注入、身份冲突、派生元数据保留、淘汰后重试及重开检查通过。legacy 默认顺序写仍可能留下原文，不承诺原子性；尚无 AfterTurn 自动恢复队列接线。 |
| Q2 | 软件回归通过 | 自有中文案例经真实 SQLite/协调器及 Runtime → RecordingProvider；覆盖显式治理纠错、旧 revision 拒绝、角色/主体隔离、重复、重启与遗忘隐藏。尚未测真实模型答案和饱和预算压力。 |
| Q3 | 部分完成 | Working 已应用治理后的正文；调和与物化 ID 使用结构化身份，不再由显示分隔符混淆主体；结构化事实/多值关系保留独立断言，临时 ID 不覆盖快照。一般自然语言自动纠错仍未实现，不能把本批当作完成该能力。 |
| Q4 | 未测 | 暂无外部可运行实例；先完成自有实现与评测 |

### 2026-11-12 本地验收

工具链：`rustc 1.97.1` / `cargo 1.97.1`，Windows；所有 Cargo 命令清空 `RUSTC_WRAPPER`。
`cargo test -p apeireth-memory -p apeireth-plugin -p apeireth-runtime-assembly --all-targets --locked --no-fail-fast -j 2`
退出 0；新增场景均无忽略，现有忽略设置未修改。
`pwsh -NoProfile -File scripts/fmt-check.ps1`、`python scripts/check_crate_budget.py --check`
及 `python scripts/arch_guard.py --strict` 退出 0；未增加依赖、预算或基线。
`cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` 退出 0。
显式设置仓外数据根后，`python scripts/check_golden_baseline.py` 退出 0：历史臂与生产臂均实际跑到语料，
检索指标未低于既有基线，证据送达观察项无变化；基线文件与阈值未修改。
首跑目录不一致导致的失败已保留，不计为通过。
跨平台全仓测试与合并裁决以同一 PR 提交的远端适用检查为准；全部通过才允许合并。

内部知识卷的 `ig-v1-027/030/121/123/127` 随本批源文档纠正而同步修订依据。
其中一题去掉了错误的历史前提，另两题改指向实际承载依据的章节；题号、题数及严格子串校验保留。
该内部卷修订与源语料变化需独立记录，旧内部卷分数仅保留作历史，不能据此声称算法收益。
小说卷、其基线及全部验收阈值未因这次修订而变化。

反向验伪的有效输出：

```text
# 身份修复前：新身份边界测试真实断言失败；修复后通过
test display_labels_do_not_merge_distinct_persona_identities ... FAILED
test display_labels_do_not_merge_distinct_persona_identities ... ok

# 仅恢复旧协调器接线：会返回旧事实且留下部分写入；恢复新接线后通过
test governed_chinese_correction_reaches_every_layer_and_survives_restart ... FAILED
test sqlite_sidecar_failure_rolls_back_evidence_and_head_before_working_publication ... FAILED
test governed_chinese_correction_reaches_every_layer_and_survives_restart ... ok
test sqlite_sidecar_failure_rolls_back_evidence_and_head_before_working_publication ... ok

# 首版环内去重不足：淘汰后重试仍失败；加入存储真实写入结果后通过
test replay_of_an_evicted_stable_id_does_not_reenter_or_displace_working_memory ... FAILED
test replay_of_an_evicted_stable_id_does_not_reenter_or_displace_working_memory ... ok
```

反证只计实际运行到断言的失败；在途源码造成的编译失败不计为红证据。
这些是固定反例与回归结果，不是外部系统比较分数。
全量测试数字仍以[测试真源](../03-reference/test-baseline.md)为准。
