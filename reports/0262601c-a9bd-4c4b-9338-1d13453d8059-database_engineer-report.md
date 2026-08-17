# 自审报告 — M5 通用记忆层时间有效性（valid_from/valid_until + 问法感知过滤, 记忆调研批⭐）

- **任务 ID**: 0262601c-a9bd-4c4b-9338-1d13453d8059
- **角色**: database_engineer
- **日期**: 2026-08-17
- **代码提交**: 52316ba2（migrations.rs + session_note.rs, 327+/4-）
- **边界遵守**: 只动 memory crate 条目模型 + 检索过滤；**未改 apeireth_core::Note、未改 NoteStore trait、未改 memory_graph 图事实时间语义**（valid_at/invalid_at 保持）

## 1. 设计（数据库工程师视角：兼容演进方案）

### Schema 变更（向后兼容铁律）

```sql
-- V3__notes_validity_window (migrations.rs, append-only 追加)
ALTER TABLE notes ADD COLUMN valid_from INTEGER;   -- NULL = 无下界
ALTER TABLE notes ADD COLUMN valid_until INTEGER;  -- NULL = 永久有效
```

- **存量零迁移**：两列均 NULLable 无 DEFAULT，ALTER 后存量行自动 NULL → 语义 = 永久有效。不做任何 UPDATE 回填（零数据风险、零停机）。
- **回滚方案**：SQLite 无 DROP COLUMN（旧版本）也不影响——旧代码 SELECT 显式列名不含新列，读不受影响；新列数据对旧代码不可见但无害（前向兼容）。新代码打开旧库自动经 V3 迁移升列。
- **索引策略**：不加索引。过滤条件 `(valid_from IS NULL OR valid_from <= ?) AND (valid_until IS NULL OR valid_until > ?)` 含 IS NULL 析取，B-tree 索引非 sargable 收益趋零；notes 表规模（提炼级知识条目）全扫足够。升级路径（表 >10^5 行后）：`CREATE INDEX ... ON notes(valid_until) WHERE valid_until IS NOT NULL` 部分索引 + 改判定为 `COALESCE` 表达式索引，届时评估。
- **迁移机制**：走 crate 既有 `MIGRATIONS` 版本化机制（schema_migrations 幂等判定），V3 追加不改历史 entry。

### 语义定义

- 有效窗口为**半开区间 [valid_from, valid_until)**：`valid_until == as_of` 视为已过期（测试覆盖边界）。
- `as_of` 参考时刻：缺省 = 当前墙钟；可显式注入（确定性测试/回放场景）。
- **问法感知过滤**（确定性关键词规则，0 LLM 0 装）：
  - 当前类词命中 → `CurrentOnly`：现在/当前/目前/如今/眼下/此刻/现阶段/近来
  - 历史类词（以前/曾经/过去/当时/从前/往昔/昔日/早先）→ 与缺省 `All` 同义，无需显式匹配
  - 两类同现 → 当前类优先（"现在和以前比"主问在现在）
  - 中性/空 → `All`（保守缺省，不过滤）
  - 子串匹配的误报风险如实标注（启发式）；升级路径：词表外置配置/分词消歧。

### 接口稳定（M4 呼应）

- `NoteStore` trait **零改动**（three_layer.rs 的 `Arc<dyn NoteStore>` 不受影响）——qa_engineer 的 LongMemEval 时序用例可依赖既有接口。
- 新能力全部走**追加式**：NoteQuery 新字段（Default 覆盖旧 builder 用法）、NoteRecord 新字段（`#[serde(default)]` 覆盖旧 JSON 缓存）、SqliteMemoryStore 固有方法（put_note_with_validity / set_note_validity）。
- apeireth_core::Note 不动（核心类型无时间有效性字段，NoteRecord.into_core 丢弃窗口字段，文档注明）。

## 2. 验证（cargo test -p apeireth-memory -j 4 全绿）

| 套件 | 结果 |
|---|---|
| lib 单测 | **312 passed** / 0 failed（含 5 个新 m5 测试） |
| 集成测试 | 9 + 2 + 6 + 7 passed / 0 failed |
| doc-tests | 3 passed / 4 ignored（存量） |

5 个新测试覆盖：
1. `m5_default_permanent_and_legacy_row_compat`：put_note 缺省永久 + **旧式 6 列 INSERT 行兼容**（模拟存量数据）+ V3 已应用
2. `m5_current_only_filters_expired_and_future`：永久/当前/过期/未生效四态过滤 + All 全返 + 半开区间边界（valid_until==as_of 过期）
3. `m5_write_side_validation_and_update`：反向窗口拒绝（put/set 两处）+ 事后标注 + 恢复永久 + ghost id → Invalid
4. `m5_query_text_awareness_rules`：当前类 5 例 / 历史+中性+空 5 例 / 冲突优先 / builder 集成 2 例
5. `m5_serde_default_for_legacy_json`：旧缓存 JSON（无新字段）反序列化 → None 永久

## 3. 0 装 PASS

- 写入侧"提炼/存入时来源可标注有效期"：**接口已留（put_note_with_validity），提炼管线接线 = 0 装**（任务方向③明示留接口即可）——当前无提炼侧调用方，不虚构接线。
- 问法感知过滤在检索调用方的接线（companion 查询入口传 with_query_text）= 未做（边界：只动 memory crate 内；接线属消费方任务，接口就绪）。
- 图事实（memory_graph）时间语义未触碰 ✓。

## 4. 风险与升级路径

| 项 | 现状 | 升级路径 |
|---|---|---|
| 索引 | 无（非 sargable + 表小） | 部分索引 + COALESCE 表达式索引（表 >10^5 评估） |
| 问法词表 | 硬编码 8 当前词 | 外置配置/分词消歧（误报反馈驱动） |
| apeireth_core::Note | 不含窗口字段 | 若核心类型需感知，届时加 Option 字段（仍需 serde(default) 兼容） |
| 检索接线 | 未接（边界外） | 消费方任务：查询入口调用 with_query_text |

## 5. 提交清单

| hash | 内容 |
|---|---|
| 52316ba2 | feat(M5) 代码（V3 migration + 模型/查询/问法感知/写入侧 + 5 测试） |
| （本提交） | backlog M5 划 ✅ + 本报告 |
