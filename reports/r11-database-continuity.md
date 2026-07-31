# R11 数据库：ContinuityTracker 真数据契约

> Task: `5ab88b00-5f92-4e2f-b789-add40e8fbb1b`  
> Role: 数据库工程师  
> Status: 完成

## 结论

V1130 dashboard 原先只把 recovery 样本写入 SQLite，ContinuityTracker session 与 V1136 的 continuity/autonomy/transferability 结果均缺少统一、版本化、可查询的持久化契约。本次新增真实 SQLite 契约，保持旧 dashboard API 和旧数据兼容，并且未修改任何 V0.5 分数或权重常数。

## 实现

修改 `apeireth/v1130_continuity_tracker_dashboard.py`：

- 新增 `ContinuitySnapshotStore` 与 `CONTINUITY_SCHEMA_VERSION = 2`。
- 新增三张业务表：
  - `continuity_session`：持久化 V1072 `SessionMarker`，主键 `(identity_id, session_id)`，重复写入使用 upsert。
  - `continuity_snapshot`：append-only 三维测量快照，保存测量版本、契约版本、原始来源 payload。
  - `continuity_snapshot_source`：按维度记录每个真实子测来源、来源版本和 detail JSON。
- 新增 `continuity_schema_meta` 记录 schema version。
- dashboard build 将实际 tracker session 写入 `db_dir/continuity_contract.sqlite3`，再从数据库计数核验；结果通过 `persistence_summary` 暴露。
- `persist_snapshot()` 接受 V1136 `V1136Result` 或兼容 mapping，三维 score、各自 detail/sub_scores、measurement version 全部可追踪。

## 兼容 migration

`migrate()` 只执行 `CREATE TABLE/INDEX IF NOT EXISTS` 和 schema meta upsert：

- 不删除、不改名、不重建旧表。
- 不修改既有 recovery 表。
- 重复执行幂等。
- 真实测试先建立 legacy 表及数据，再连续迁移两次，旧行仍完整存在。

## 约束与索引

数据库边界约束：

- 三维 score：`CHECK(value BETWEEN 0 AND 1)`。
- entries 非负、importance 在 `[0,1]`、active 为布尔整数。
- 来源维度只允许 `continuity/autonomy/transferability`。
- source payload/detail 必须为合法 JSON。
- source 通过 FK 绑定 snapshot，删除快照时级联清理来源。

索引：

- `idx_continuity_session_time(identity_id, started_at DESC)`
- `idx_continuity_snapshot_time(identity_id, measured_at DESC)`
- `idx_continuity_source_dimension(dimension, source_version)`

时间序列查询按 `identity_id` 过滤、`measured_at` 排序；真实 `EXPLAIN QUERY PLAN` 验证命中 `idx_continuity_snapshot_time`。

## 查询契约

- `persist_tracker(identity_id, tracker)`：session 真写入，主键 upsert，更新结束时间、entries、importance 和 active 状态。
- `persist_snapshot(identity_id, measurement)`：保存三维快照及逐来源记录，稳定内容 hash 生成 snapshot id，重复写幂等。
- `timeline(identity_id)`：返回升序时间序列，包含 snapshot、测量版本、契约版本和四项结果字段。

## 非 mock 测试

新增 `tests/test_r11_continuity_data_contract.py`，全部使用磁盘临时 SQLite 文件和真实 SQL：

1. legacy 数据迁移保留及幂等。
2. ContinuityTracker session 真写、更新、回读。
3. V1136 三维快照与来源/版本可追踪。
4. 时间序列顺序与真实查询计划索引命中。
5. dashboard payload 的 session 数与数据库实际行数一致。

验证结果：

```text
R11 新测试 + V1130 原测试: 37 passed in 118.23s
V1122 ContinuityTracker 回归: 16 passed in 37.48s
py_compile: passed
总计: 53 passed
```

## 不变项

- 未修改 V1125/V1136 公式。
- 未修改 `0.85` baseline 或三个 `0.05` 权重。
- 未虚构新的 score。
- 未删除或覆盖旧 dashboard/recovery 数据。
