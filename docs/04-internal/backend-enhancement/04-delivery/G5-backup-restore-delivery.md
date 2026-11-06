# G5 备份恢复 — 备份格式规格 + 往返矩阵 + 测试证据

```yaml
[Document-Meta]
Document:        G5-backup-restore-delivery.md
Version:         1.0 (G5 备份恢复交付: 引擎 + 双出口 + 治理 + 自述节)
Last-Modified:   2026-10-17
Status:          🟢 活跃 (备份格式的单一真源; 改格式必改本规格 + 同步测试)
Author:          G5 备份恢复施工线
```

> 横切能力: 运行时的**一切记忆/账本可备份、可搬家** (伙伴的「记忆保险箱」)。
> 一个引擎, 两个出口: 模型面 `tool.backup_export` (AI 先用) + CLI 面
> `apeireth backup export|restore|verify` (同一内核的包装出口, 不另起炉灶)。

## 1. 交付面 (write_scope)

| 面 | 文件 | 说明 |
|---|---|---|
| 引擎 + 工具 | `crates/capabilities/tools/src/backup.rs` (新) | 备份格式 + 导出/恢复/校验 + `tool.backup_export` |
| 自述节 | `crates/capabilities/tools/src/self_status.rs` | `BackupStatus` 备份节 (加法, 避让并行批在途区) |
| CLI 面 | `crates/adapters/cli/src/backup.rs` (新) + `main.rs`/`lib.rs` 接线 | 三动词 + 审计行 + 数据源解析 |
| 装配挂载 | `crates/engine/runtime-assembly/` (tool_modules/production/self_status_source/permission_preset/canonical/lib) | 注册即闸 + 名册 + 导出面 + 只读预设执法面 |
| 安全语义 | `crates/engine/guard/src/semantics.rs` + `tests/backup_capability_descriptor.rs` (新) | `tool.backup_export` 规范描述符 (凭据面 false 断言) |

禁区 0 触碰: frontend / CHANGELOG / .github / docs/design / memory 内部 /
orchestration / tools/voice / organ/care / governance / input_security。

## 2. 备份格式规格 (目录快照 + manifest.json)

### 2.1 容器形态 (读码拍板)

**目录快照**, 不用归档容器 (tar/zip 类):

- workspace 纪律是 **0 引新外部依赖**; 归档容器全部需要新外部依赖,
  目录快照零依赖;
- 「损坏可检出」由 **逐文件 SHA-256 摘要** 充分满足 (manifest 逐件记账);
- 逐文件「先写临时再换名」的原子落位比整包解压更贴合恢复语义。

SHA-256 依赖 = `sha2 = { workspace = true }` (workspace 既有包, 非新外部包;
与同批其他施工线给成员 crate 加依赖边同口径, 如实记账)。

### 2.2 bundle 布局

```text
<bundle>/manifest.json                     清单 (见 2.3)
<bundle>/files/<category>/<relpath>        逐文件原字节快照
```

`category` ∈ 七类数据面 (`BackupCategory`):

| category | 数据面 | 源口径 (组装根解析) |
|---|---|---|
| `sessions` | 会话库 | 默认域 `sessions.sqlite3` (+ WAL/SHM 侧车) + 逐域 `users/<id>/data/sessions.sqlite3*` |
| `cognitive` | 认知库 | 默认域 `cognitive.sqlite3` (+ 侧车) |
| `memory` | 记忆库 | 逐域 `users/<id>/data/cognitive.sqlite3*` (每户独立记忆域) |
| `skills` | 技能卡 | `<root>/skills/` 整树 (每卡一 StoredDoc) |
| `care_rules` | 关怀规则 | `<root>/care_rules.json` + `<root>/care_rules/` 整树 |
| `users` | 用户表 | `<root>/users.json` |
| `domains` | 域文档 | `<root>/users/<id>/domain.json` |

缺失源跳过 (类目计数如实为 0, 不编件); 树内**符号链接不追** (记排除面)。

### 2.3 manifest.json (损坏可检出)

```json
{
  "format_version": 1,
  "product_version": "<workspace 版本真值>",
  "created_at_ms": 1729500000000,
  "data_root": "<导出时数据根 (出处真值)>",
  "files": [
    {
      "category": "sessions",
      "path": "files/sessions/sessions.sqlite3",
      "origin": "<导出时原位绝对路径>",
      "relpath": "sessions.sqlite3",
      "bytes": 12345,
      "sha256": "<hex>",
      "mtime_ms": 1729490000000
    }
  ],
  "excluded": [
    { "path": "creds.json", "reason": "credential artifact (explicit exclusion list: 凭据不随行, 密钥/令牌面不进包)" }
  ]
}
```

- **逐文件摘要 + 大小双查** → 任一位翻转可检出;
- **版本 + 时间** 在场 (`format_version` / `created_at_ms` / 逐件 `mtime_ms`);
- `files` 按 `path` 排序 (确定性); `relpath` 是**恢复根相对**的逻辑家
  (搬家恢复按它重组); `origin` 是同机原位恢复的落位目标 (纯出处信息);
- `excluded` 是**显式排除面的逐条记账** (不静默丢件)。

### 2.4 凭据不随行 (显式排除, 测试钉死)

双层收口:

1. **显式排除表** `CREDENTIAL_ARTIFACTS_EXCLUDED` (逐点名): `creds.json`、
   `apeireth-keyring.bin`、`apeireth-keyring.master.key`、`keyring/`、
   `client-tokens.json`、`im-channels.json`;
2. 既有**敏感路径策略** (`sensitive_path`, 同 crate 单一真源): `.env` 族 /
   私钥族 / apikey 族 / 钥匙串导出物等密钥形状件。

排除件逐条记进 manifest `excluded` (含原因); 打包内零秘值、零凭据件名由
测试逐字节全文扫断言 (对抗面: 误列显式源 / 树内藏件 / 数据根裸放, 三层全测)。

## 3. 硬语义钉死 (二选一均标注被拒项)

| # | 语义 | 钉死选择 | 被拒 alternative + 理由 |
|---|---|---|---|
| 1 | **宁拒不损** | 恢复前**全量校验** (清单可解析 + 版本 + 逐件大小/摘要), 任一不符拒恢复, **零字节落盘** | 拒「边验边落」: 半包落地 = 损坏状态, 违宁拒不损 |
| 2 | **原子落位** | 逐件「先写临时再换名」+ 原件 `.bak` 保全; 中途失败**整链回滚**到恢复起始态 | 拒「失败即止不回滚」: 已落位件悬空, 非原状 |
| 3 | **版本兼容 = 显式拒** | `format_version` ≠ 1 (含旧版 0 / 新版 2) 一律**显式拒绝**并报双方版本号 | 拒「迁移语义」: 现只有一版格式, 预建迁移器是无主债; 版本歧义期宁拒不猜 |
| 4 | **增量: 备份后新增不丢不覆盖** | 恢复只落位 manifest **成员集**; 非成员 (备份后新增) 一字不动; 下一份导出是全量 (新增入账) | 拒「恢复前清场」: 清场会抹掉备份后新增数据, 直接违不丢 |
| 5 | **导出不覆盖** | bundle 目标已存在 = 拒绝即帧 (含空目录) | 拒「覆盖续写」: 两份备份互踩 = 静默丢备份 |
| 6 | **路径安全** | 清单 `path`/`relpath` 只认普通部件 (拒 `..`/绝对/盘符/空), 重复落位目标拒绝即帧 | 拒「宽容归一」: 目录穿越是打包面经典逃逸 |

落位目标解析: 缺省 = manifest `origin` (同机原位); `--to <root>` =
`<root>/<relpath>` (**搬家恢复**, 换机重组)。

## 4. 往返矩阵 (操作 × 语义 × 测试)

| 操作 | 语义面 | 断言 | 测试 (证据) |
|---|---|---|---|
| export → verify → restore `--to` | 全量往返 | 逐字节一致 (每件成员) + 七类目齐 | `full_round_trip_restores_every_member_byte_for_byte` (引擎) + `cli_round_trip_covers_the_seven_categories_and_restores_byte_for_byte` (CLI) |
| restore 原位 | 成员回卷 | 改动后的成员回到 bundle 字节态 | `in_place_restore_returns_members_to_their_bundle_bytes` |
| manifest 坏 JSON / 缺失 | 宁拒不损 | 拒恢复 + 拒校验 + 数据面零触碰 | `corrupt_manifest_is_refused_and_nothing_touches_the_data_plane` |
| 逐件字节翻转 | 宁拒不损 | 拒恢复 + 零落盘 | `a_single_flipped_byte_refuses_the_whole_restore` |
| 凭据三层对抗 | 凭据不随行 | 打包内零秘值 + 零凭据件名 + 排除面记账 | `credentials_never_travel_with_a_bundle` + `cli_credentials_stay_out_of_the_bundle` |
| 换名中途失败 | 原子落位 | 整链回滚到恢复起始态 + 占位原样 + 零临时残留 | `mid_placement_failure_rolls_back_to_the_original_state` |
| 版本 0 / 2 | 版本兼容显式拒 | 报双方版本号, 不迁移不猜 | `version_mismatch_is_explicitly_refused_for_old_and_new_formats` |
| verify | 只验不恢复 | 数据面零读写 + 坏包独立报坏 + 零审计行 | `verify_is_independent_and_never_touches_the_data_plane` + `cli_verify_is_read_only_and_refuses_corruption_and_overwrite` |
| export/restore | 审计行 | `backup.export`/`backup.restore` 各一行, 记时间 (ts) 与清单数 (entries=N) | `audit_rows_and_ledger_record_time_and_manifest_counts` + `cli_backup_writes_audit_rows_with_time_and_manifest_counts` |
| 备份后新增 | 增量不丢不覆盖 | 非成员一字不动 + 下一份全量收进 + 已有 bundle 拒覆盖 | `incremental_semantics_are_pinned_end_to_end` |
| 清单穿越路径 | 路径安全 | `../`/绝对路径拒绝即帧 | `hostile_manifest_paths_are_refused` |
| self_status 备份节 | 真值或 null | 尚无备份 = 显式 null + 原因; 导出后 = 时间/件数真值 | `backup_status_reports_truth_or_explicit_null` + `self_status_backup_probe_reads_the_cli_ledger` + `self_status_backup_section_reports_truth_or_explicit_null` (装配级) |
| 工具面挂载 | 注册即闸 | 默认不注册; 配置在场 = 注册 + 名册如实 + 产出可独立校验的 bundle | `production_registers_the_backup_export_tool_only_when_configured` + `the_backup_export_tool_produces_a_manifest_checked_bundle` |
| 只读预设 | 执法面 | `tool.backup_export` 属写面 (read_only 会话拒) | `backup_export_is_a_write_face_for_the_read_only_preset` |
| guard 描述符 | 敏感面断言 | `may_access_credentials=false` (凭据不随行) + 持久写如实 + 名册覆盖率 100% | `backup_export_descriptor_is_honest_about_its_faces` + `backup_export_is_in_the_builtin_canonical_roster` |

## 5. 治理面

- **CLI 面 = 本地操作** (owner 本机运维, 与 `user` 面同域信任模型);
  `backup.export` / `backup.restore` 各记一行审计 (`daemon-audit.jsonl`
  面板审计同一账本, `AuditDto` 同一形状): **时间 (`ts`) + 清单数
  (`detail: entries=N`)**; `verify` 只验不恢复 = 零审计行 (如实注);
- **工具面** (`tool.backup_export`): 旋钮 `APEIRETH_ENABLE_BACKUP_EXPORT=1`
  = 注册 + grant + **require_approval** (全量打包记忆落盘是敏感面, 每次
  导出都停人工审批, 无自动放行档); 默认关 (fail-closed);
- **敏感面 = 凭据排除断言** (§2.4, 测试钉死);
- **只读预设执法面**: `is_write_or_execute_capability` 名单收录
  `tool.backup_export` (打包落盘 + 外带记忆 = 写面, read_only 会话不得执行);
- **guard 规范描述符**: 双面如实 (Read/MemoryRead/Write + MemoryPrivate),
  `may_access_credentials = false` (凭据不随行, 描述符不得谎报凭据入口);
- **域伙伴运行时不注册本工具** (W3-① 跨域拒绝即帧的备份面等价承诺):
  全量打包面能外带整个数据根, 收口到 owner 的 CLI 本地操作; 域伙伴名册
  照实报 `backup_export: false`。

## 6. self_status 备份节 (真值或 null)

`tool.self_status` 新增 `backup` 节 (输出合同: 节恒在场):

```json
"backup": { "last_backup_at_ms": 1729500000000, "manifest_entries": 10, "reason": null }
```

- 真值来源 = 数据根 `last-backup.json` (导出时落账, 与 `manifest.json`
  同源同刻: 时间 = `created_at_ms`, 件数 = 清单件数);
- **尚无备份 = 显式 null + 原因** (「尚无备份记录」), 绝不编时间/编件数;
  探测口缺位 = 全节 null + `DATA_PROBE_NOT_WIRED` (0 装诚实);
- 探测口 `backup_status_probe(ledger_path)` (引擎同文件, CLI 装配注入)。

## 7. 挂载点决策 (「读码定挂哪」结论)

- **工具实现挂 `capabilities/tools`**: 全部 `ToolCapability` 实现都在此
  crate (fs/search/repo/self_status/skill/… 无一例外), 工具与引擎同 crate
  还能复用 `sensitive_path` 敏感路径策略 (pub(crate) 单一真源) 做凭据排除
  第二层 —— 拒「引擎放 runtime-assembly」: 那会逼工具反向依赖装配层
  (循环) 或复制一份敏感件名单 (两套逻辑);
- **注册挂 `ProductionModulesConfig.backup`** (Option 注入 = 注册即闸),
  模块包装 `tool_modules::BackupExportModule` (`module.tool.backup_export`),
  名册行 `backup_export` 按实际注册条件投影 —— 与 content_feed/care_schedule
  同款结构;
- **runtime-assembly 导出面**: `BackupExportModule` + `backup_status_probe`
  (组合根探针构造与引擎同源, 与 `session_quota_probe` 等同面);
- **CLI 面挂 `adapters/cli/src/backup.rs`**: 数据源解析 (七类) 是组装根的
  职责 (env 覆写口径与运行时逐字一致), 引擎只认显式源清单。

## 8. 测试证据 (2026-10-17 实测)

命令口径: `cargo test --locked -j 2` (逐套件) + `cargo test --workspace
--all-targets --locked --no-fail-fast -j 2` (全量门禁); 工具链 rustc 1.97
(2026-10-17)。新增测试 26 项:

| 套件 | 命令 | 实测 |
|---|---|---|
| 引擎往返矩阵 (12) | `cargo test -p apeireth-tools-canonical --test backup` | 12 passed / 0 failed |
| tools 单元 (含备份 3 + 自述节) | `cargo test -p apeireth-tools-canonical --lib` | 376 passed / 0 failed |
| 装配级挂载 (4) | `cargo test -p apeireth-runtime-assembly --test backup_registration` | 4 passed / 0 failed |
| guard 描述符 (2) | `cargo test -p apeireth-guard --test backup_capability_descriptor` | 2 passed / 0 failed |
| CLI 面 (5) | `cargo test -p apeireth-cli --test backup_cli` | 5 passed / 0 failed |

门禁 (2026-10-17 实测, 工具链 rustc 1.97):

| 门禁 | 命令 | 实测 |
|---|---|---|
| 全量测试 | `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` | **207 套件 / 5605 passed / 0 failed / 20 ignored** (含既有 194 套件零回归 + 本批 4 新套件; 各批新套件合计 207) |
| clippy | `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | 0 警告 |
| fmt | `cargo fmt --all -- --check` | 0 diff |
| 中性表述 | `scripts/check-neutral-terms.ps1` | OK (78 changed files: added lines all neutral) |
| secret-scan | `scripts/secret-scan.ps1` | No secrets found |

## 9. 并发施工协调 (报备留痕)

- `tools/src/self_status.rs` / `tools/src/lib.rs` / runtime-assembly 多文件
  为多批共享施工区: 本批只做**加法** (锚点避开 care/voice/redaction 批在途
  区), 同名撞车已避让 (脱敏批已有 `RestoreReport` → 本批改名
  `BackupRestoreReport`);
- 他人在途红曾出现 (标红不代改, 已由各归口线自检收口, 全量门禁现全绿):
  `orchestration/tests/task_queue.rs` 的 let-else clippy 红、
  `tools/tests/voice_speak.rs` 的 `.ledger()` 编译红、task_board 两测的
  fmt diff —— 均非本批文件, 收口后 2026-10-17 全量复测 0 failed。
