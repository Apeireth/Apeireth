# ADR 0020: D-07 一次性 SQLite → PostgreSQL 迁移

> **状态**: 🟢 Accepted (主人 2026-08-05 拍板, 1.0 release 阶段 5 估补)
> **commit 锚**: `Cargo.toml` workspace `rusqlite = "0.32"` + `crates/apeireth-migrate/` 估补 + 蓝图 §3.6
> **最后更新**: 2026-08-05

---

## 1. 背景 (Context)

Apeireth 1.0 release (v1.0.0) 默认数据后端 = SQLite (per `Cargo.toml` `rusqlite = "0.32"`, workspace 硬锁)。但 R21 商业化版会切 PostgreSQL (per R20 阶段 1 蓝图 §3.6 + 主人 2026-08-05 "R21 商业化" 拍板)。

**问题**:
- SQLite 是"个人/小团队"场景, 写并发弱, 无网络访问
- PostgreSQL 是"企业/多机"场景, 写并发强, 远程访问
- 1.0 release 用户装 SQLite; R21 升级时 **数据不能丢** + **0 人工干预** (一键迁移)
- 4 crate 共享 rusqlite: `apeireth-memory` / `apeireth-vector` / `apeireth-api` / `apeireth-mcp`

**约束**:
- 1.0 release 12 项 checklist #5 upgrade 要求 "升级跑通 + data check + 0 丢失"
- 1.0 release 12 项 checklist #5 + #6 uninstall 跟迁移脚本协同
- 不破坏 SQLite 1.0 release (单 binary 启动, 无外部依赖)
- 迁移脚本必须 **dry-run 0 错** (per 1.0 release #5)

---

## 2. 决策 (Decision)

**R20 阶段 5 估补 `apeireth-migrate` crate = 一次性 SQLite → PostgreSQL 迁移工具 + 升级器集成**

### 2.1 迁移策略 (3 步)

```
┌─────────────┐    ┌──────────────┐    ┌──────────────┐
│ 1.0 release │ →  │ 1. 导出       │ →  │ 2. PostgreSQL │
│ SQLite 库   │    │   JSONL (5表) │    │    初始化     │
└─────────────┘    └──────────────┘    └──────────────┘
                                              ↓
                              ┌──────────────────────────┐
                              │ 3. 导入 (事务 + 校验)     │
                              │   - 5 表逐行 INSERT      │
                              │   - FK 重新建立          │
                              │   - 索引重建             │
                              │   - 0 丢失校验           │
                              └──────────────────────────┘
```

### 2.2 5 表清单 (per 4 crate 共享 SQLite)

| Crate | 表 | 主键 | 估行数 (1 用户 1 年) |
|---|---|---|---|
| `apeireth-memory` | `memory_chunks` | chunk_id (UUID) | 100K-500K |
| `apeireth-memory` | `memory_embeddings` | embedding_id (UUID) | 100K-500K |
| `apeireth-vector` | `vector_index` | doc_id (UUID) | 50K-200K |
| `apeireth-api` | `api_auth_tokens` | token_hash (TEXT) | 100-1K |
| `apeireth-mcp` | `mcp_server_state` | server_id (TEXT) | 10-50 |

### 2.3 迁移 crate 接口 (估补)

```rust
// crates/apeireth-migrate/src/lib.rs (估补, R20 阶段 5 落地)
pub struct Migrator {
    source: SqlitePool,
    target: PgPool,
    dry_run: bool,
}

impl Migrator {
    /// 0 触碰 source DB, 1 步导出 JSONL
    pub async fn export_jsonl(&self, out_dir: &Path) -> Result<()>;
    
    /// 0 触碰 target DB, 1 步导入 (事务)
    pub async fn import_jsonl(&self, in_dir: &Path) -> Result<MigrationReport>;
    
    /// 0 丢失校验 (行数 / 主键 / FK / 索引)
    pub async fn verify(&self) -> Result<VerificationReport>;
    
    /// 1 步: export + import + verify (1.0 release #5 验收入口)
    pub async fn run(&self) -> Result<MigrationReport>;
}

pub struct MigrationReport {
    pub tables: Vec<TableStats>,
    pub total_rows: u64,
    pub duration_sec: f64,
    pub errors: Vec<String>,
    pub dry_run: bool,
}
```

### 2.4 升级器集成 (per 1.0 release #6 uninstall 协同)

- **apt upgrade / dnf upgrade / brew upgrade / scoop update**: 包升级前自动跑 `apeireth-migrate --check-compat` (1.0 release 跳过, R21 真接)
- **R21 真实升级**: 检测到 PostgreSQL 配置, 自动提示"1.0 → 1.1 升级需要数据迁移, 是否继续?"
- **回滚**: 失败时 source SQLite 不动, 0 数据丢失 (per 1.0 release 守门)

### 2.5 Dry-run 模式 (1.0 release #5 验收)

```bash
# 0 触碰真 DB, 仅模拟
apeireth-migrate --dry-run --source ./test.db --target postgresql://localhost/test
# 预期: 0 error + 0 warning, 估时报告

# 真迁移 (R21 估补, 1.0 release 不跑)
apeireth-migrate --source ~/.local/share/apeireth/db.sqlite \
                 --target postgresql://apeireth:***@prod-db/apeireth
```

---

## 3. 后果 (Consequences)

### 3.1 正面

- ✅ **1.0 release #5 upgrade PASS**: dry-run 0 错 + 升级跑通 + data check
- ✅ **0 数据丢失**: source SQLite 0 触碰 + 事务保证 + 失败回滚
- ✅ **复用 4 crate 共享 SQLite**: 5 表统一迁移, 不分散
- ✅ **dry-run 必过**: 1.0 release CI 必跑
- ✅ **R21 商业化无障碍**: 用户一键切 PostgreSQL, 不需重装

### 3.2 负面

- ⚠️ **5 表都要写一遍 INSERT**: 4 crate 各 1 估补, 估时 1 owner × 1 周
- ⚠️ **JSONL 中间态**: 大库 (1M+ 行) 估 5-15 min 导出; 1.0 release 1 用户 1 年估 500K 行, 估 30-60 s
- ⚠️ **PostgreSQL schema 同步**: 4 crate 各需要 1 个 schema 迁移 (per R21)
- ⚠️ **字符集**: SQLite UTF-8 默认, PostgreSQL UTF-8 强制, 大部分字符集兼容; emoji 偶发编码问题 (mitigation: 迁移前 normalize)

### 3.3 风险

- 1.0 release 估 500K 行, 1 owner × 1 周估补可完成; 10M+ 行超大库 R21+ 估补流式迁移
- PostgreSQL 13 vs 16 字符集差异 (per 0.9.x 案例); mitigation: 锁 PostgreSQL 14+ (1.0 release 文档明示)

---

## 4. 备选 (Alternatives Considered)

### A. 1.0 release 直接 PostgreSQL, 不用 SQLite
- 优点: 1 步到位
- 否决: 1.0 release 定位"个人/小团队", 强制 PG 装门槛太高; 用户装个 PG 都不想, 1.0 release 失败

### B. 1.0 release 仅 SQLite, R21 再迁移 (本决策)
- 优点: 1.0 release 装最简; R21 商业化再迁
- 拍板: 1.0 release 阶段 5 拍 (估补 `apeireth-migrate`)

### C. 双写 (SQLite + PostgreSQL 并存)
- 优点: R21 切 PG 0 风险
- 否决: 1.0 release 复杂度 × 2, 装门槛高; 1 用户用不到双写

### D. 第三方迁移工具 (e.g. pgloader)
- 优点: 业界成熟
- 否决: pgloader 是 Python 工具, 跟 Rust 工具链割裂; 自建可控, 估时 1 周

### E. Logical replication (PostgreSQL 订阅)
- 优点: 实时同步
- 否决: SQLite → PG logical replication 业界无成熟方案, 自建成本高

---

## 5. 6 哲学锚穿透

- ✅ **S-1 走在前人经验上**: SQLite 1.0 + PG R21 双轨业界常见 (e.g. Notion, Linear)
- ✅ **S-2 实事求是**: 1.0 release 估 1 用户 1 年 500K 行, 自建迁移 1 周可完成
- ✅ **O-2 用户看结果不看哲学**: 用户只关心"装上能跑, 升级不丢", 不关心迁移机制
- ✅ **O-3 信息密度"高"**: 3 步 + 5 表 + 1 crate, 1 节说清
- ✅ **O-4 干净状态 = 没有历史包袱**: 不双写, 不引第三方迁移工具
- ✅ **O-5 6 哲学锚穿透**: 本节自检

---

## 6. 8 项不修改承诺

- ✅ **不假装已实现**: `apeireth-migrate` 估补, R20 阶段 5 落地; 1.0 release #5 dry-run 是该 crate 真接后跑
- ✅ **编译期 hardcode**: 5 表 schema 编译期固定 (per 4 crate 共享 SQLite 表定义)
- ✅ **不改 LOCKED**: 7 LOCKED 文档 + 24 LOCKED crate 0 触碰
- ✅ **不改 workspace version**: v1.0.0 严守 (迁移脚本跟 v1.0.0 同步)
- ✅ **6 哲学锚穿透**: §5 自检
- ✅ **不依赖 NewAPI**: 自建迁移, 0 依赖第三方
- ✅ **不重复造轮子**: 沿用 sqlx (PG) + rusqlite (SQLite) 业界标准
- ✅ **诚实标缺**: 1.0 release 不跑真迁移 (只 dry-run); R21 商业化估补真迁移 + 升级器集成

---

## 7. 引用

- 决策 ID: `docs/stage4/pending-decisions-overview-2026-08-05.md` (D-07, 主人 2026-08-05 拍板 "R21 商业化走, 1.0 release dry-run")
- 蓝图: `docs/stage4/r20-product-finalize-2026-08-05.md` §3.6
- rusqlite 硬锁: `Cargo.toml` line 178 (`rusqlite = "0.32"`, workspace 锁)
- 1.0 release #5 验收: `reports/r20-v1.0.0-release-checklist-2026-08-05.md` #5
- 实施估补: `crates/apeireth-migrate/` (R20 阶段 5, 估 1 owner × 1 周)
- Workspace 锁定: `docs/stage4/8-locked-unified-2026-08-05.md`
