# N5 artifact_sig 内容寻址缓存门禁 — 自审报告（database_engineer2）

- 任务ID: 5f492ccb-7c0a-4f80-8c09-2139949929fe
- 交付类型: code
- 日期: 2026-08-16
- 文件范围: `crates/apeireth-memory/src/semantic_persist.rs`（仅此一个代码文件 + 2 个文档同步）

## 1. 背景与设计依据

吸收 VCP rust-vexus-lite `memo_artifact_builder.rs:662-802` 的"内容寻址资产"机制
（team-work-doc §8.2/§8.4 P0#1）：artifact_sig 级联签名 → 签名不变跳过重算。

## 2. 设计（机制而非补丁）

- `artifact_sig(content)` = SHA-256(规范化内容 UTF-8 字节) hex64，算法明确可测。
  **纯 Rust 手写 SHA-256（mod sha256_n5, ~50 行）**，0 新依赖：sha2 crate 不在本 crate
  依赖图，且工作区 Cargo.lock 当时已被并行任务弄脏，加 dep 会引入共享文件冲突；
  本 crate 已有手写 SHA-1 先例（hallways.rs sha1_12 mod）。正确性由 NIST FIPS 180-4
  官方测试向量锚定（空串/"abc"/双块 56 字符向量三连）。
- `artifact_sig_many(items)` = 排序级联规范化签名：同集合与插入顺序无关；
  id/content 用 `\0` 分隔 + count 前缀防拼接歧义；格式版本号 `apeireth-artifact-v1`
  供未来 bump（旧签名自然 miss → 重算）。
- 门禁决策 `artifact_gate_decision(stored, current_sig)` 纯函数，失效规则显式：
  1. 无记录 → MissNoRecord（首次构建）
  2. sig 不匹配 → MissContentChanged（**防脏读第一原则：内容变→签名变→强制重算**）
  3. sig 匹配但 normalize_version < CURRENT → StaleNormalize（与已有
     `CURRENT_NORMALIZE_VERSION` + `.normalize.json` sidecar 机制协作：规范化/chunk
     规则升级 → 产物语义变 → 即使内容相同签名也失效）
  4. sig 匹配但 schema_version 不一致 → StaleSchema
  5. 全匹配 → Hit（复用磁盘资产，跳过重算）
- sidecar：`<vector_path>.artifact_sig.json`（JSON {sig, normalize_version,
  schema_version}），沿用 embedder.json 的 tmp+rename 原子写；损坏 JSON → None =
  miss 走重算（0 panic 0 假命中）。
- 重算路径 `reindex_all(eps)`：先全量 embed（失败不动现有资产）→ clear() +
  upsert_batch 全量重建（episode 集收缩/删除 → 0 残留旧向量，防脏读闭环）→
  落盘 sidecar 三件套 + 新签名记录。Hit → 直接返回，0 embed 0 写盘。

## 3. 四路径验收测试（cargo test -p apeireth-memory -j 4）

| 路径 | 测试 | 断言要点 |
|---|---|---|
| 命中 | `n5_reindex_all_miss_then_hit_skips_recompute` | 二次同内容 → Hit，CountingEmbedder 计数不增长（真跳过） |
| 未命中 | 同上（首次）+ `n5_artifact_record_sidecar_roundtrip_and_corruption` | 无记录 → MissNoRecord 真重算；损坏 sidecar → miss 不 panic |
| stale 失效 | `n5_reindex_all_stale_normalize_invalidates_hit` | sig 匹配但 normalize 旧版本 → StaleNormalize 强制重算 → 回 CURRENT → 恢复 Hit |
| 内容微变 | `n5_reindex_all_content_micro_change_forces_recompute` | 一字符改 → MissContentChanged 全量重 embed + 新签名落盘 |

纯函数路径另由 `n5_gate_decision_rules_pure`（5 条规则逐一断言）+
`n5_artifact_sig_matches_sha256_known_vectors`（NIST 向量）+
`n5_artifact_sig_many_order_invariant_and_boundary_safe` 覆盖。

测试结果: **全绿**。`cargo test -p apeireth-memory -j 4` 298 passed / 0 failed
（新增 8 个 n5_ 测试全过；6 个 warning 全为预存 — deprecated save() 旧测试 +
lightmemo 缺文档，非本次引入）。首跑曾抓出 1 个真 bug：`clear()` 会删 vec_meta
并重置 dim → upsert_batch 报 "set_dimension() not called yet"，已修复
（clear 后必须重设 dim），回归全绿。

## 4. 0 装 PASS（§2.4 不假装红线）自审

- ✅ 无假装成功：record_artifact IO 失败传播 Err；check 损坏 sidecar 返回 None=miss
- ✅ 无夸大文档：门禁只覆盖全量重建路径（reindex_all）；增量写入路径
  （index_episode/index_episodes）保持原语义，模块头已诚实标注"0 假装全覆盖"
- ✅ 无静默吞错：所有 vector op 错误映射为 MemoryError::Other 带上下文
- ✅ 无虚构交互：返回 ArtifactDecision 让调用方可观测，测试用 embed 计数证明"真跳过"

## 5. 边界遵守

- 只改 `semantic_persist.rs`（semantic 持久化）；0 触碰 memory_graph 节点评分
  （database_engineer N6）、0 触碰 crawl（agent_orchestrator2 N7）
- 0 改 lib.rs（semantic_persist 已注册，新类型经 `apeireth_memory::semantic_persist::*` 可达）
- 0 改 Cargo.toml / Cargo.lock（手写 SHA-256 决策见 §2）

## 6. 提交与文档同步

- 代码提交: 被流水线历史重写整合进 **f8245f28**（HEAD 已含完整实现：8 个 n5_
  测试 + clear/set_dim 修复，worktree 与 HEAD 零 diff 核实）。本人事后精确补提交
  的仅为文档与本报告。
- backlog N5 划 ✅: 已被流水线台账提交 **2dba1a8e** 吸收（HEAD 核实）。
  注: 其后工作区 backlog.md 曾出现回退为旧快照（纯删已提交内容，无新内容），
  已用 HEAD 恢复，未提交任何回退。
- docs/maintenance-guide.md 模块地图 +semantic_persist 行: 本次精确提交
  （仅本 hunk，未裹入 database_engineer 的 N6 memory_graph 行编辑）。

## 7. 并发协作实况记录（诚实披露）

- Cargo.lock 与多数 crate 文件在工作区长期带他人未提交改动 → 全程只 add
  自己的文件；backlog/guide 的共享冲突用手工 patch 精确切 hunk 解决。
- 流水线历史重写两次改变 commit 归属（代码进 f8245f28、台账 ✅ 进 2dba1a8e），
  均已在 HEAD 逐项核实内容完整，无内容丢失。
