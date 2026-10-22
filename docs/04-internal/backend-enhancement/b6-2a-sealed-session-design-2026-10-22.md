# B6-2a 试点设计：`sessions.data` 列级封缄接入（决策页）

> 日期：2026-10-22 · 状态：**B6-2a 已按方案 A 施工（默认关；仅 `sessions.data` 试点）** · 归属线：安全线（storage）+ 装配线（runtime-assembly）
> 依据：`implemented-to-wired-plan-2026-10-21.md` §2（B6 抬级波 2）、隐患 #6、`f72a6f13`（S3·B6 定级+接缝交付）、`crypto_keys.rs`（波 1 密钥出口，`41e451ca`）

## 0. 结论（TL;DR，建议按此拍板）

**建议：B6-2a 只做 `sessions.data` 一列的试点接线，默认关，KDF 维持 600k，锚定先用「口令 + 恢复件」而不是平台 keychain。**

- 试点范围：`crates/engine/runtime-assembly/src/sqlite_session.rs` 的 `save`/读路径，`sessions.data` 列走 `SealingLayer<SqliteFieldSink>`（storage 侧接缝已由 `f72a6f13` 交付）；
- 开关：新增 `APEIRETH_ENABLE_SESSION_SEAL`（只认字面 `"1"`，默认关；关 = 逐字节零变化）；
- 密钥：复用 B6 波 1 的 `apeireth crypto` 导出物，daemon 通过**受限权限密钥文件**读取解锁后的 `PartitionKey`（密钥文件 0600，路径由 `APEIRETH_SESSION_SEAL_KEY_FILE` 指定）；**不做**交互式口令（daemon 无终端）；
- 检索：`sessions` 只按 `id` 查（主键），`data` 无任何查询语义 ⇒ **无检索损失**，这是选它当试点列的首要原因；
- 迁移：读路径双兼容（明文旧行照读、密文行解封读），写路径开开关后一律封缄；历史行惰性迁移（读时不动、写时升级），另给一次性 `--migrate` 批处理选项（B6-2a 内置在 cli，不强制）；
- 回滚：关开关 → 新写回明文（混合态可读），密钥文件保留即可读旧密文；**不提供"自动解密全部"命令**（避免批量明文导出面），只提供显式 `--rewrite-plaintext` 需二次确认（可选）；
- 明确不做：`memory_items`/`episodes` 等其余 P0 列（等试点绿后再议）、平台 keychain `KekAnchor`、`tool.*` 模型面、全量一次性迁移。

## 1. 现状（2026-10-22 取证）

### 1.1 storage 侧已交付（不是零起点）

`f72a6f13`（安全隐患治理线 S3·B6，2026-10-21）已交付：

| 资产 | 位置 | 作用 |
|---|---|---|
| 定级单一真源 | `crates/engine/storage/src/crypto/classification.rs` | `FieldClass` 六档 + `classify_column(table, column)`；**`("sessions","data")` 已判 `SealedContent`**（P0 必封） |
| 列级接缝 | `crates/engine/storage/src/crypto/sqlite_sink.rs` | `SealedTable` 白名单（含 `Sessions`）+ `SqliteFieldSink::open_file/in_memory` + `seal_with(PartitionKey)` → `SealingLayer` |
| 密钥层级 | `crates/engine/storage/src/crypto/{partition,kdf,sealed,layer}.rs` | 分区 DEK + KEK 包裹 + PBKDF2 600k + `SealedField`（AAD 绑分区） |
| 端到端证 | `crypto/sealed_column_demo`（5 测） | 写读往返 / 重启后读得出 / 落盘翻位拒读 / 库文件零明文+密文魔数在场 / 他分区拒读 |

### 1.2 生产侧现状（零消费 = 本试点要补的洞）

- `git grep -n "SealedTable\|SqliteFieldSink\|seal_with\|SealingLayer" -- crates ':!crates/engine/storage'` → **无输出**（生产零消费）；
- sessions 的真实读写点：`crates/engine/runtime-assembly/src/sqlite_session.rs:112 save()` → `INSERT INTO sessions (id, data)`（`:119`/`:220`）；读路径按 `id` 取 `data` 反序列化 `Session`；
- CLI 密钥出口已有：`apeireth crypto wrap|recovery-export|unlock`（`41e451ca`），`keys.json` 为包裹态、口令解锁。

## 2. 接入点设计（文件:函数级）

### 2.1 开关（生产装配根）

```text
APEIRETH_ENABLE_SESSION_SEAL=1        # 只认字面 "1"（与既有 bio 开关同纪律），默认关
APEIRETH_SESSION_SEAL_KEY_FILE=<path> # 解锁后的 PartitionKey 材料文件（0600）
```

- 解析收在一处：`runtime-assembly::canonical::session_seal`（新小模块，纯函数 + 单测；不在两处读 env）；
- daemon 启动时：开关关 → 不读密钥文件、不构造 SealingLayer（零变化）；开关开 → 读密钥文件失败 = **启动失败**（fail-closed，不降级明文）。

### 2.2 `sqlite_session.rs`

| 函数 | 现状 | 试点改法 |
|---|---|---|
| `save(&self, session)` | `INSERT INTO sessions (id, data) VALUES (?1,?2)`，`data` = Session JSON | 开开关：`data` 先过 `SealingLayer::put_sealed`（hex 密文）；关：原路径逐字节不变 |
| 读路径（按 id 载入） | `SELECT data FROM sessions WHERE id`，JSON 反序列化 | 开开关：先判 hex/密文魔数 → 解封 → JSON；明文旧行（迁移残）照读（过渡兼容，探测到非密文形态即按明文解析，不猜编码）；解封失败 = **显式错误**（不降级明文、不返回空会话） |
| 惰性迁移 | 无 | 写路径天然升级该行；批量迁移为独立 cli 动词（见 2.4） |

- 连接并发：SealingLayer 持 `Connection`，与现有 `Mutex<Connection>` 口径对齐（单写者假设沿用 storage 文档）；
- 审计：本试点**不写审计行**（审计归口治理面 R8-B3，钥匙/封缄行为审计另开，不在本批抢跑）。

### 2.3 密钥供给（关键决策点，建议方案 A）

| 方案 | 描述 | 评价 |
|---|---|---|
| **A（建议）** | 用户先跑 `apeireth crypto unlock --keys <file> --out <plain-key-file>`（新增 `--out`：0600 写解锁后的 DEK+分区材料）；daemon 经 `APEIRETH_SESSION_SEAL_KEY_FILE` 读 | 无终端交互、可 systemd/服务启动；残余=明文密钥文件在盘（0600，明确写进 SECURITY/文档） |
| B | daemon 启动时 stdin 读口令（`systemd` 可不给 stdin） | 不适合常驻服务，且口令进进程内存 |
| C | 平台 keychain `KekAnchor` 实现 | 需要桌面/系统集成，本波不做（另立） |

- 方案 A 的 `unlock --out` 属 B6 波 1 的小扩展（1 个 cli commit + 测试）；B6-2a 依赖它。

### 2.4 迁移/回滚命令（cli）

```text
apeireth crypto session-seal migrate   --db <path> --keys <keyfile> [--batch N]   # 惰性批量：逐行判明文→封缄，幂等
apeireth crypto session-seal rewrite-plaintext --db <path> --keys <keyfile> --yes  # 显式回滚（二次确认），不默认提供
```

- 迁移前自动备份 `<db>.bak.<ts>`（storage `sealed_backup` 已具备）；
- 幂等：已是密文形态的行跳过；迁移只写 `data` 列，不重建表。

## 2.5 施工落地（2026-10-22，方案 A）

| 决策项 | 拍板结果 | 落地位置 |
|---|---|---|
| 试点范围 | 只 `sessions.data` 一列 | `crates/engine/runtime-assembly/src/sqlite_session.rs` |
| daemon 密钥供给 | 方案 A：`crypto unlock --out` 0600 密钥文件 | `crates/adapters/cli/src/crypto_keys.rs`（`--out`）+ `session_seal.rs` |
| KDF / 回滚 | 维持 600k；回滚 = 关开关（混合态可读），不提供自动批量解密 | 惰性迁移（写时升级）+ 双兼容读 |

### 运维口径（开关默认关）

```bash
# 1) 生成包裹密钥（口令 + 恢复件）—— 恢复件请离线保管
apeireth crypto wrap --partition sessions --keys ~/.apeireth/sessions.keys.json \
    --recovery-out ~/.apeireth/sessions.recovery.txt

# 2) 导出解锁后的密钥材料（0600；daemon 无终端，故不走交互口令）
apeireth crypto unlock --keys ~/.apeireth/sessions.keys.json \
    --out /run/apeireth/session-seal.key

# 3) 打开封缄（daemon 环境）
export APEIRETH_ENABLE_SESSION_SEAL=1
export APEIRETH_SESSION_SEAL_KEY_FILE=/run/apeireth/session-seal.key
```

- **密钥丢失 = 会话永久不可解**（无后门）：`keys.json` + 恢复件必须离线保管，
  备份集必须同时备份密钥材料 —— 只备份库文件等于制造一堆永远解不开的密文。
- 开关开而密钥文件缺失/坏 = **启动失败**，不降级明文。
- 关开关回滚：新写回明文，旧的密文行仍可读（密钥文件保留即可），**不提供**
  自动全量解密命令。
- 已知残余：明文旧带留在历史备份/`.bak` 中（加密不追溯已备份文件）；
  `sessions.id` 与时间列仍明文（定级表已拍板）；未做性能基准，不宣称数字。

## 3. 风险与残余（如实登记，不假装没有）

1. **密钥丢失 = 会话永久不可解**（`KEY_LOSS_BOUNDARY`，无后门）：`keys.json` + 恢复件必须离线保管；备份集必须同时备份 keys/锚定记录；
2. **混合态**：迁移期 + 回滚后库内明文/密文混存，读路径需双兼容（有测试钉）；明文旧带残留在备份/`.bak` 中 —— 文档明示"加密不追溯已备份文件"；
3. **性能**：每会话每次 save 一次 AEAD（JSON 体积多数 KB 级；PBKDF2 600k 只在解锁时一次，不在每写）—— 未做基准，B6-2a 不宣称性能数字；
4. **元数据仍明文**：`sessions.id` / 时间列等保明（定级表已拍板），攻击者可看"有多少会话、何时"；
5. **无真盘恢复演练**：不宣称 HARDWARE VALIDATED；跨机恢复演练另立证据。

## 4. 验收计划（B6-2a 交付时必带）

- 单测（runtime-assembly）：开/关双路、往返、重启后读、篡改拒读、明文旧行兼容读、密钥文件缺失 fail-closed、开关只认 `"1"`；
- 集成：`sqlite_session` E2E（真实文件库两次进程启动读写）+ cli `session-seal migrate` 幂等；
- 0 装证据：开关关 = 库文件逐字节同旧路径（对比测试）；`cargo test -p apeireth-runtime-assembly` scoped + `-p apeireth-cli --lib`；clippy/fmt；
- 文档：矩阵 `sec.storage_envelope_crypto` 行按四级口径更新（若只接 sessions 一列 → 写「PRODUCTION WIRED（默认关，仅 sessions.data 试点；其余 P0 列未接）」），claims 抬级由效率线裁定。

## 5. 分阶段建议

| 阶段 | 内容 | 规模/风险 |
|---|---|---|
| **B6-2a（本决策页）** | `sessions.data` 一列 + 开关 + 密钥文件 + 迁移/回滚命令 | 1–2 commits，中风险（试点最小） |
| B6-2b（试点绿后） | `memory_items.data` / `episodes.data` 等 P0 内容列，逐表接入 | 每表 1 commit，需重看检索面 |
| B6-3 | 密封备份并入既有备份出口（`backup.rs` / `backup` 工具面） | 1–2 commits，依赖 2a/2b |

## 6. 需拍板的三个问题（建议答案）

1. **试点范围**：建议只做 `sessions.data`（唯一无查询语义的内容列）。
2. **daemon 密钥供给**：建议方案 A（`crypto unlock --out` 0600 密钥文件 + `APEIRETH_SESSION_SEAL_KEY_FILE`）。
3. **KDF 与回滚**：建议维持 600k（已对齐硬化建议）；回滚 = 关开关 + 可选显式 `rewrite-plaintext`（二次确认），**不提供自动批量解密**。

> 拍板后按 B6-2a 施工；未拍板前不动生产代码（本页之外零改动）。
