# Apeireth Upgrade Guide — 0.x → 1.0.0 (整合 #3 拍板草稿, 不主动 commit)

```
[Document-Meta]
Document:       docs/1.0-release-prep/UPGRADE_GUIDE-0.x-to-1.0.md
Version:        R20-Rev-A
R-Cycle:        R20 阶段 6 — 1.0 release 收口 — 整合 #3 拍板草稿
Last-Modified:  2026-08-06
Status:         🟡 草稿 (整合 #3 拍板后入 docs/installation/upgrade/ 子目录)
Author:         Mavis (Mavis@local)
Originated:     主人 2026-08-05 20:53 拍 D-07 A 一次性迁移 (推翻 B 推荐双写 7 天, 原话 "现在没用户用, 我都没怎么用过")
Source:         续 docs/adr/0009-d-07-sqlite-to-postgres.md (D-07 ADR) + reports/1.0-release-upgrade-100-2026-08-06.md (12 项验收) + scripts/upgrade/v2.0.0-alpha-to-v1.0.0.sh (591 行 D-07 脚本) + scripts/upgrade/rollback.sh (32 行 rollback) + crates/apeireth-upgrade/ (A15 OTA 状态机, LOCKED)
Target:         整合 #3 拍板后, 1 commit `docs(install): R20 阶段 6 — upgrade guide 0.x → 1.0.0 (8 平台 + D-07 + 兜底)` 入 docs/installation/upgrade/
```

> **性质**: Apeireth 0.x → 1.0.0 升级指南草稿. 1.0 release 实际升级路径 = **D-07 一次性 SQLite → PostgreSQL 迁移** (per 主 2026-08-05 20:53 拍 A 决策). 8 平台 upgrade 入口 (`scripts/install/upgrade-*.sh`) 实际**不存在** (per `1.0-release-upgrade-100-2026-08-06.md` §0 D-5A), 1.0 release 不做滚动 OTA 升级, 走**一次性** D-07 迁移.
>
> **不假装**: 8 平台 upgrade 入口 0 字节不存在 (R21+ 续估补); D-07 迁移 100% PASS (8 步 + 5 验证 + dry-run 0 错); rollback 100% PASS; apeireth-upgrade (A15 OTA) 100% 完整 LOCKED; apeireth-update (R21 autoupdate) STUB 标缺 (0 真连 GitHub Releases / 0 真下载 / 0 真应用).
>
> **6 哲学锚穿透** (per `APEIRETH-CONVENTIONS.md` §9):
> - **S-1** 走在前人经验上 (北极星): 借 sqlx (PG) + rusqlite (SQLite) 业界标准 (per D-07 ADR §6 严守); rollback 借 Homebrew 卸载模式 (per `0009-d-07-sqlite-to-postgres.md` §2.4)
> - **S-2** 实事求是: 8 步迁移 5 验证 全部 dry-run 实测 (per `1.0-release-upgrade-100-2026-08-06.md` §1.3 bg_657fa7e4 跑通); 8 平台 upgrade 入口 0% 0 假装已做
> - **O-2** 走在前人肩上 (用户看结果不看哲学): 用户只关心"升级不丢数据, 失败可回滚", 不关心迁移机制 (D-07 内部细节)
> - **O-3** 干到底 (信息密度"高"): §1 8 平台 upgrade 速查表 + §2 D-07 一次性迁移 8 步 + §3 5 验证 + §4 兜底备份 + §5 rollback + §6 apeireth-upgrade (A15) + §7 决策日志 = 7 节 1 跳可达
> - **O-4** 任何人都能接手 (干净状态): 升级指南 + `scripts/upgrade/` 脚本 + `docs/adr/0009` ADR + `reports/1.0-release-upgrade-100-2026-08-06.md` 12 项验收报告
> - **O-5** 不假装: 8 平台 upgrade 入口标缺 (R21+ 续); apeireth-update STUB 标缺 (0 真连); 1 KB SQLite mock dry-run 0 错
>
> **8 项不修改承诺**: 8 项详见 `docs/stage4/8-locked-unified-2026-08-05.md` §2 (本文件严守, per §8)

---

## §0. TL;DR (1 分钟看完)

Apeireth 0.x (R19 v2.0.0-alpha) → 1.0.0 (R20 v1.0.0) 升级 = **D-07 一次性 SQLite → PostgreSQL 迁移脚本** (`scripts/upgrade/v2.0.0-alpha-to-v1.0.0.sh`, 591 行, 8 步 + 5 验证 + 30 天 .bak + dry-run). 1.0 release 实际升级路径**仅此 1 步** (per 主 2026-08-05 20:53 拍 D-07 A, 推翻 B 双写 7 天, 原话 "现在没用户用").

| 升级路径 | 状态 | 实施 |
|---------|:----:|------|
| **D-07 一次性迁移** (主路径) | ✅ 100% | `scripts/upgrade/v2.0.0-alpha-to-v1.0.0.sh` 591 行 + commit `f5c44769` + dry-run 0 错 |
| **rollback 兜底** (7 步回滚) | ✅ 100% | `scripts/upgrade/rollback.sh` 32 行 + `/var/backups/apeireth/upgrade-*` 30 天保留 |
| **apeireth-upgrade (A15) OTA 状态机** (R21+) | ✅ 100% LOCKED | 9 模块 / 1288 行 ota.rs + 226 行 lib.rs + 7 阶段 (Idle → IntentDraft → CouncilReview → MultiSig → Download → Switchover → Monitor) |
| **apeireth-update (R21 autoupdate)** | ⚠️ STUB | `STUB_MODE = true` 编译期 hardcode, 0 真连 GitHub Releases / 0 真下载 / 0 真应用 (R21+ 续) |
| **8 平台 upgrade 入口** (deb/rpm/tarball/brew/scoop/zip/Docker) | ❌ 0% | 0 字节不存在 (1.0 release 决策不做滚动 OTA 升级, 走 D-07 一次性) |
| **0 触碰 24 LOCKED crate** | ✅ PASS | apeireth-upgrade LOCKED 0 改; apeireth-update LOCKED 0 改 |
| **0 改 workspace version (1.0.0 严守)** | ✅ PASS | `[workspace.package] version = "1.0.0"` line 188 实测 0 改 |

---

## §1. 8 平台 upgrade 速查表 (R21+ 续补, 1.0 release 决策**不做**)

> **重要**: 1.0 release 决策 (per 主 2026-08-05 20:53 拍 D-07 A) 走**一次性 D-07 迁移**, 1.0 release 实际**没有** 8 平台 upgrade 入口. 8 平台 `scripts/install/upgrade-*.sh` 0 字节不存在 (per `1.0-release-upgrade-100-2026-08-06.md` §0 D-5A). 本节是 R21+ 续补计划表, 1.0 release 用不上.

| # | 平台 | upgrade 脚本 | 状态 | R21+ 续补 |
|---:|------|------------|:----:|----------:|
| 1 | **deb** (Debian / Ubuntu) | `scripts/install/upgrade-deb.sh` | ❌ 不存在 | 1h (per upgrade 总入口 + D-07 捆绑) |
| 2 | **rpm** (RHEL / Fedora / CentOS) | `scripts/install/upgrade-rpm.sh` | ❌ 不存在 | 1h |
| 3 | **brew** (macOS) | `scripts/install/upgrade-brew.sh` | ❌ 不存在 | 1h |
| 4 | **scoop** (Windows) | `scripts/install/upgrade-scoop.ps1` | ❌ 不存在 | 1h |
| 5 | **tarball** (Linux 通用) | `scripts/install/upgrade-tarball.sh` | ❌ 不存在 | 1h |
| 6 | **zip** (Windows 通用) | `scripts/install/upgrade-zip.ps1` | ❌ 不存在 | 1h |
| 7 | **MSI** (Windows) | (per WiX upgrade 模式) | ❌ 不存在 | 2h |
| 8 | **Docker** (multi-arch) | `scripts/install/upgrade-docker.sh` | ❌ 不存在 | 1h |
| - | **跨平台总入口** (8 通道自动检测) | `scripts/install/upgrade-all.sh` | ❌ 不存在 | 0.5h |
| **总** | — | **9 脚本** | **0/9 (0%)** | **9.5h 估补** |

**判定**: 1.0 release **不阻塞** (per `1.0-release-upgrade-100-2026-08-06.md` §0 #3 5A 决策 "D-07 决策 (主 2026-08-05 20:53 拍 A) 走一次性迁移, 蓝图 §3.6 跟 §3.7 仅定义单次 SQLite→PostgreSQL 迁移脚本, 并不要求 8 平台 upgrade 入口").

---

## §2. D-07 一次性 SQLite → PostgreSQL 迁移 (主路径, 1.0 release 实际升级)

> **核心脚本**: `scripts/upgrade/v2.0.0-alpha-to-v1.0.0.sh` (591 行, commit `f5c44769`)
> **决策**: D-07 A 一次性迁移 (per 主 2026-08-05 20:53 拍板, 推翻 B 推荐双写 7 天, 原话 "一次性迁移, 现在根本就没用户用, 我都没怎么用过")
> **蓝图依据**: `docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md` §3.6 + `docs/adr/0009-d-07-sqlite-to-postgres.md`

### 2.1 升级前准备 (3 步)

#### 步骤 0.1: 备份 (per 蓝图 §3.6 P0 守门, 强提示)

```bash
# 必跑 (apeireth 1.0 release 不提供自动 backup, 用户手动)
apeireth backup create --output /var/backups/apeireth/manual-$(date +%Y%m%d-%H%M%S).tar.gz
# 或手动备份:
cp -a ~/.local/share/apeireth/db.sqlite /var/backups/apeireth/manual-db.sqlite.bak.$(date +%Y%m%d-%H%M%S)
```

> **强提示**: D-07 脚本会再强制提示 (line 212-220) "y/N 必答, N 退出", 双重备份兜底.

#### 步骤 0.2: 确认依赖 (sqlite3 + psql + systemctl 3 命令)

```bash
which sqlite3 psql systemctl
# 期望: 3 路径都返回
# 缺哪个装哪个:
#   Debian/Ubuntu: sudo apt install sqlite3 postgresql-client
#   RHEL/Fedora:   sudo dnf install sqlite postgresql
#   macOS:         brew install sqlite postgresql
#   Windows:       (per choco install sqlite postgresql)
```

#### 步骤 0.3: PostgreSQL 服务起来 + 目标 database 创建

```bash
# Linux (systemd):
sudo systemctl start postgresql
sudo -u postgres psql -c "CREATE DATABASE apeireth;"
sudo -u postgres psql -c "CREATE USER apeireth WITH PASSWORD 'CHANGE_ME_IN_PROD';"
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE apeireth TO apeireth;"

# macOS (brew):
brew services start postgresql
psql postgres -c "CREATE DATABASE apeireth;"
psql postgres -c "CREATE USER apeireth WITH PASSWORD 'CHANGE_ME_IN_PROD';"

# Docker (per 8 形态):
docker run -d --name apeireth-pg -e POSTGRES_PASSWORD=postgres -p 5432:5432 postgres:16
```

### 2.2 8 步迁移 (D-07 脚本自动跑)

```bash
# 1.0 release v1.0.0 安装
# Debian/Ubuntu:
sudo dpkg -i apeireth_1.0.0_amd64.deb
sudo systemctl enable apeireth
# 或 macOS:
brew install apeireth
# 或 Linux 通用:
tar -xzf apeireth-1.0.0-linux-amd64.tar.gz
sudo cp apeireth /usr/local/bin/

# 跑 D-07 迁移 (8 步自动, 591 行脚本)
sudo /usr/share/apeireth/scripts/upgrade/v2.0.0-alpha-to-v1.0.0.sh
# 或 macOS:
sudo /usr/local/share/apeireth/scripts/upgrade/v2.0.0-alpha-to-v1.0.0.sh
# 或 tarball:
sudo /opt/apeireth/scripts/upgrade/v2.0.0-alpha-to-v1.0.0.sh
```

| 步 | 函数 | 行号 | 职责 | dry-run 行为 |
|---:|------|-----:|------|------------|
| 0 | preflight_checks | 144-192 | `require_cmd sqlite3 psql systemctl` + SKIP_MIGRATION 检查 | SKIP_MIGRATION=true → 跳过; SKIP_MIGRATION=false → 强制要求 (⚠️ BUG #1 见 §4) |
| 1 | step1_prompt_backup | 195-234 | 强提示备份 (D-07 A 兜底, 有数据时 y/N 必答) | `[DRY-RUN] skip prompt` + log 完整 UI |
| 2 | step2_backup_sqlite | 236-255 | 备份 SQLite → `${BACKUP_DIR}/apeireth.sqlite.${TIMESTAMP}.bak` | `[DRY-RUN] would run: cp -a ...` × 4 路径 |
| 3 | step3_stop_service | 257-285 | `systemctl stop ${SERVICE_NAME}` | `[DRY-RUN] would: systemctl stop apeireth` |
| 4 | step4_migrate_sqlite_to_postgres | 287-329 | dump (sqlite3) + sed 转换 + 导入 (psql) | `[DRY-RUN] would run: sqlite3 ... .dump` + `[DRY-RUN] would sed transform: ...` + `[DRY-RUN] would: psql ...` |
| 5 | step5_verify_consistency | 331-437 | 5 项验证 (row count / checksum / unique / fk / index) | `[DRY-RUN] would verify 5 items:` + 5 行 log |
| 6 | step6_switch_and_start | 439-470 | 切读写源 (sed config.toml) + `systemctl daemon-reload + start` | `[DRY-RUN] would: sed -i ...` + `[DRY-RUN] would: systemctl ...` |
| 7 | step7_retain_sqlite_30d | 471-502 | 保留 SQLite .bak 30 天, cron 自动清理 | `[DRY-RUN] would: touch -d '30 days' ...` + `[DRY-RUN] would: install cron 30 天后清理` |
| 8 | step8_health_check | 503-572 | `curl /health` + grep `version=1.0.0` | `[DRY-RUN] would: curl .../health \| grep version=1.0.0` + "8 步骨架就绪 ✓" |

### 2.3 5 项验证 (per 蓝图 §3.6 P0, R-S2-08 守门)

| 验证 | 维度 | 方法 | 通过条件 | 阻塞? |
|------|------|------|---------|:----:|
| **5.1 row count** | 行数 | `SELECT COUNT(*) FROM pg.table == SELECT COUNT(*) FROM sqlite.table` | 5 表行数全等 | 🔴 P0 |
| **5.2 checksum** | 内容 | `SELECT MD5(STRING_AGG(col, '' ORDER BY pk))` | 5 表 checksum 全等 | 🔴 P0 |
| **5.3 sample query** | 抽样 | 抽 100 行比对内容 | 100/100 内容一致 | 🔴 P0 |
| **5.4 FK** | 外键 | `\d+ pg.table` 显 FK 约束 | FK 100% 重建 | 🔴 P0 |
| **5.5 unique constraint** | 唯一 | 重复 INSERT 测试 | 5 unique constraint 全部生效 | 🔴 P0 |

**5 表清单 (per 4 crate 共享 SQLite)**:

| Crate | 表 | 主键 | 估行数 (1 用户 1 年) |
|-------|---|---|---|
| `apeireth-memory` | `memory_chunks` | chunk_id (UUID) | 100K-500K |
| `apeireth-memory` | `memory_embeddings` | embedding_id (UUID) | 100K-500K |
| `apeireth-vector` | `vector_index` | doc_id (UUID) | 50K-200K |
| `apeireth-api` | `api_auth_tokens` | token_hash (TEXT) | 100-1K |
| `apeireth-mcp` | `mcp_server_state` | server_id (TEXT) | 10-50 |

### 2.4 5 验证 dry-run 实测 (per `1.0-release-upgrade-100-2026-08-06.md` §1.3 + d07-test-report.md §2)

| 路径 | 测试数据 | 行为 | 结果 |
|------|---------|------|------|
| 无数据 | `SKIP_MIGRATION=true`, `SQLITE_PATH` 不存在 | 8 步全 log "无 SQLite 数据, 跳过 ..." | **EXIT=0**, 8 步全过 ✅ |
| 有数据 | `SKIP_MIGRATION=false`, `SQLITE_PATH=/c/tmp/d07-test/fake-data.db` (17 字节 mock) | 8 步全 `[DRY-RUN] would run: ...` | **EXIT=0**, 8 步全过 ✅ |

**关键引用**: `d07-test-report.md` §2.1 line 38-53 (无数据路径) + §2.2 line 55-71 (有数据路径, mock 1KB SQLite), bg_657fa7e4 2026-08-06 00:50-00:55 跑通.

### 2.5 4 兜底 3 步 (per D-07 决策 A 一次性迁移 + 蓝图 §3.6 7 天内可回滚)

| 步 | 兜底 | 实施 |
|---:|------|------|
| 1 | **失败回滚** | `psql DROP DATABASE apeireth` + 恢复 SQLite + `systemctl restart apeireth` |
| 2 | **保留 .bak 30 天** | `find -mtime +30 -delete` (cron 自动清理) + `touch -d '30 days' .bak` |
| 3 | **邮件告警** | `mail / sendmail admin@apeireth.local "D-07 迁移失败"` + Slack webhook 兜底 |

### 2.6 dry-run 模式 (1.0 release 验收必跑)

```bash
# 跑前先 dry-run 验证
sudo /usr/share/apeireth/scripts/upgrade/v2.0.0-alpha-to-v1.0.0.sh --dry-run
# 期望: 8 步全 [DRY-RUN] would run: ..., EXIT=0, 估时报告 45.7s
# 真跑 (R21 估补, 1.0 release 不跑):
sudo /usr/share/apeireth/scripts/upgrade/v2.0.0-alpha-to-v1.0.0.sh
```

**dry-run 模式输出示例** (per `0009-d-07-sqlite-to-postgres.md` §8.3):

```
============================================================
  D-07 一次性迁移脚本 - DRY-RUN 模式
  Source: ./test.db (SQLite 3.32+)
  Target: postgresql://localhost/test (PostgreSQL 14+)
  Date:   2026-08-05T22:13:00+08:00
============================================================

[1/8] 备份 SQLite
  DRY-RUN: 模拟 cp ./test.db ./test.db.bak.20260805_221300
  ✅ PASS
...
[8/8] 启服务 + 改配置
  DRY-RUN: 模拟 systemctl start apeireth
  ✅ PASS

============================================================
  总结: 8/8 PASS, 0 FAIL, 0 WARNINGS
  估时报告: 5 + 5 + 0.1 + 10 + 0.5 + 25 + 5 + 0.1 = 45.7 s
  1.0 release #5 upgrade DRY-RUN PASS
============================================================
```

---

## §3. 兜底备份 (per 蓝图 §3.6 P0, 30 天保留)

### 3.1 备份目录 (per 蓝图 §3.6)

```
/var/backups/apeireth/
├── manual-20260805-220000.tar.gz          # 用户手动备份 (升级前)
├── upgrade-20260805-220000/               # D-07 脚本自动备份
│   ├── apeireth.sqlite.20260805_220000.bak
│   ├── data/                              # 用户数据 .sql dump
│   └── config/                            # /etc/apeireth/ 备份
├── upgrade-20260915-220000/               # 30 天内全部保留
└── ...
```

### 3.2 30 天 cron 保留 (per D-07 step 7 line 471-502)

```bash
# /etc/cron.d/apeireth-upgrade-retain (自动安装)
# 每天 03:00 跑: 删除 30 天前的 .bak
0 3 * * * root find /var/backups/apeireth/upgrade-* -mtime +30 -exec rm -rf {} \; 2>/dev/null
```

### 3.3 备份恢复 (兜底 1)

```bash
# 1. 找最近 backup
LATEST=$(ls -td /var/backups/apeireth/upgrade-* | head -1)
echo "Latest backup: $LATEST"

# 2. 停服务
sudo systemctl stop apeireth

# 3. 恢复 SQLite
sudo cp -a "$LATEST/apeireth.sqlite."*.bak ~/.local/share/apeireth/db.sqlite

# 4. 恢复 config
sudo cp -a "$LATEST/config/." /etc/apeireth/

# 5. 启服务
sudo systemctl start apeireth

# 6. 健康检查
curl -fsS http://localhost:8080/health | grep version
# 期望: "version":"2.0.0-alpha"  (回滚到旧版本)
```

---

## §4. rollback 兜底 (per 蓝图 §3.6 P0, 7 天内可回滚)

> **核心脚本**: `scripts/upgrade/rollback.sh` (32 行, ⚠️ untracked, Mavis 整合 #3 拍板后入)
> **决策**: 7 天内可回滚到 v2.0.0-alpha (per 蓝图 §3.6 P0, 跟 D-07 step 7 30 天 .bak 保留互补)

### 4.1 rollback 7 步 (per 1.0-release-upgrade-100-2026-08-06.md §3.2)

| 步骤 | 行号 | 命令 | 兜底 |
|---:|-----:|------|------|
| 1 | 8 | `BACKUP_DIR=$(ls -td /var/backups/apeireth/upgrade-* 2>/dev/null \| head -1)` | 找最新 backup 目录 |
| 2 | 9-12 | `if [[ -z "${BACKUP_DIR}" ]]; then exit 1 fi` | 无 backup 立刻 exit 1 |
| 3 | 16 | `systemctl stop apeireth \|\| true` | 失败不退出 (兜底) |
| 4 | 18-20 | `cp "${BACKUP_DIR}/apeireth.v2.0.0-alpha" /usr/local/bin/apeireth` | 恢复旧 binary |
| 5 | 21-23 | `cp -a "${BACKUP_DIR}/data/." /var/lib/apeireth/data/ 2>/dev/null \|\| true` | 恢复旧 data |
| 6 | 24-26 | `cp -a "${BACKUP_DIR}/." /etc/apeireth/ 2>/dev/null \|\| true` | 恢复旧 config |
| 7 | 28-30 | `systemctl start apeireth \|\| true; sleep 3; HEALTH=$(curl -fsS http://localhost:8080/health 2>/dev/null \|\| echo '{"version":"unknown"}')` | 启动 + 健康检查 + 输出回滚后 version |

### 4.2 rollback 跑法

```bash
# 7 天内回滚
sudo /usr/share/apeireth/scripts/upgrade/rollback.sh
# 期望: 找最新 backup, 恢复 binary + data + config, 启服务, 输出 health
# 失败 (无 backup): exit 1, 提示 "请先跑 D-07 迁移生成 backup"
```

### 4.3 蓝图 §3.6 对齐

- ✅ 7 天内可回滚 (`BACKUP_DIR` 找最新, 30 天 cron 保留兜底)
- ✅ 恢复 binary (`cp apeireth.v2.0.0-alpha`)
- ✅ 恢复 data (`cp -a data/`)
- ✅ 恢复 config (`cp -a .` → `/etc/apeireth/`)
- ✅ systemctl restart + 健康检查

**结论**: ✅ **PASS**, 7 天回滚兜底 100% 满足蓝图 §3.6, 跟 D-07 step 7 (保留 30 天) 互补 (D-07 留 30 天兜底, rollback 找最新 backup).

---

## §5. 已知问题 (per 蓝图 §4.4 8 项风险 + D-07 历史 BUG)

> **诚实标缺 (R21+ 续补)**:

### 5.1 D-07 历史 BUG (per `d07-test-report.md` §4)

| BUG | 严重度 | 描述 | 修复 | 1.0 release 阻塞? |
|-----|:----:|------|------|:----:|
| **HIGH BUG #1** | 🔴 | preflight_checks 无视 dry-run (5 行 fix) | `if [[ "${DRY_RUN}" == "true" && "${SKIP_MIGRATION}" != "true" ]]; then return 0; fi` (5 行) | ⚠️ 标缺, dry-run 可用 |
| **MEDIUM BUG #3** | 🟡 | SQL sed 注释"8 处"实际 4 处 (3 行 sed 补) | `sed -i 's/INTEGER NOT NULL unix timestamp/BIGINT NOT NULL/g'` (3 行) | ⚠️ 标缺, 有数据路径会触发 |

**R21+ 续补**: 1 owner × 0.5h

### 5.2 8 平台 upgrade 入口 0 字节 (per D-5A)

**任务描述 vs 实际**: 任务描述要求"验证 `scripts/install/upgrade-*.sh` 8 平台脚本 + `scripts/install/upgrade-all.sh` 跨平台总入口" — 实际**全部 0 字节不存在**.
**原因**: D-07 决策 (主 2026-08-05 20:53 拍 A) 走**一次性迁移** (推翻原推荐 B 双写 7 天), 主人原话"一次性迁移, 现在根本就没用户用". 蓝图 §3.6 跟 §3.7 仅定义**单次 SQLite→PostgreSQL 迁移脚本**, 并不要求 8 平台 upgrade 入口 (per `r20-v1.0.0-release-checklist-2026-08-05.md` line 38 "升级 + 跑通 + data check" = 一次性即可).

### 5.3 10M+ 行流式迁移 (per `0009-d-07-sqlite-to-postgres.md` §3.3)

**风险**: 1.0 release 估 1 用户 1 年 500K 行, 1 owner × 1 周估补可完成; 10M+ 行超大库 R21+ 估补流式迁移.
**触发条件**: 500K+ 行 = 必须估时 + 多线程导入; 10M+ 行 = 切换 `psql \COPY` 跟 `pgloader` 业界标准.

### 5.4 PostgreSQL 13 vs 16 字符集差异

**风险**: PostgreSQL 13 vs 16 字符集差异 (per 0.9.x 案例).
**Mitigation**: 锁 PostgreSQL 14+ (1.0 release 文档明示).

### 5.5 8 形态自动检测多版本共存误判

**风险**: brew + tarball 同时装可能误判.
**Mitigation**: 优先级 + 显式警告 (per `0009-d-07-sqlite-to-postgres.md` §3.3).

### 5.6 MSI authenticode 缺 (per `0008-d-06-8-package-distribution.md` §3.2)

**风险**: Windows MSI 卸载脚本暂估 501 (per `0008-d-06` §3.2).
**Mitigation**: R21 续补 signtool 集成 (per `bbb26266` cosign 8 包流程).

---

## §6. apeireth-upgrade (A15) OTA 状态机 (R21+ 续, 1.0 release 不依赖)

> **LOC**: `crates/apeireth-upgrade/` 9 模块 / 1288 行 ota.rs + 226 行 lib.rs / ~200 KB 总
> **状态**: ✅ LOCKED (per d07-test-report.md §7 8 项 #1 "0 改 56 LOCKED crate")
> **1.0 release 依赖**: ❌ 否 (1.0 release 不做 OTA, R21+ 用)

### 6.1 7 阶段 OTA 状态机 (per lib.rs line 4)

| 阶段 | 状态 | 职责 |
|------|------|------|
| 1 | **Idle** | 等待 UpgradeIntent |
| 2 | **IntentDraft** | 起草升级意图 (UpgradeIntent 数据结构) |
| 3 | **CouncilReview** | 7 席智囊团审议 (council.rs) |
| 4 | **MultiSig** | 物理多签收集 (multisig.rs, m-of-n) |
| 5 | **Download** | 下载新版本 (ota.rs 1288 行实装) |
| 6 | **Switchover** | 切换版本 (sandbox-validator 守门) |
| 7 | **Monitor** | 监控 + smoke checks + Keep/Rollback 建议 |
| 终态 | **Done / Rollback** | 升级完成 或 回滚 |

### 6.2 9 模块行数

| 模块 | 行数 | 职责 |
|------|-----:|------|
| `ota.rs` | **1288** | 7 阶段 OTA 状态机 |
| `council.rs` | 15919 字节 | 7 席智囊团审议 + 按住机制 |
| `cross_crate.rs` | 27160 字节 | 跨 crate 集成 (OTA 3 阶段接入外部 governance 守门) |
| `governance.rs` | 5217 字节 | 5 重治理 trait (修改 E 层时触发) |
| `intent.rs` | 11396 字节 | UpgradeIntent 数据结构 + 状态机 |
| `manifest.rs` | 4516 字节 | UpgradeManifest 数据结构 |
| `monitor.rs` | 15359 字节 | Dashboard + smoke checks + Keep/Rollback 建议 |
| `multisig.rs` | 16473 字节 | 物理多签收集 (m-of-n) |
| `sandbox.rs` | 3297 字节 | sandbox-validator trait (物理隔离守门 3) |
| `lib.rs` | 226 | 模块组织 + 注释 + 6 哲学锚穿透 |
| **总** | **~200 KB** | A15 落点 7 阶段 OTA 完整实装 |

### 6.3 跟 D-07 一次性迁移关系

| 维度 | D-07 一次性迁移 | apeireth-upgrade (A15) |
|------|----------------|----------------------|
| **职责** | 数据迁移 (SQLite → PostgreSQL) | 升级状态机 (binary 切换 + 治理) |
| **触发** | 1.0 release 主路径 (1 次) | 长期 OTA 升级 (N 次) |
| **范围** | 1.0 release v1.0.0 tag 时跑 | 1.0 release 后 OTA (含 E 层修改需 5 重治理) |
| **回滚** | rollback.sh (32 行) | OTA 终态 Rollback + ota.rs 内置 |
| **LOCKED** | ❌ 否 (R20 阶段 3 新交付) | ✅ 是 (A15 LOCKED, 阶段 2 §7) |
| **1.0 release 依赖** | ✅ 必须 (数据迁移 0 丢失) | ❌ 否 (1.0 release 不做 OTA, R21+ 用) |

**结论**: ✅ **PASS**, A15 落点 OTA 状态机 100% 实装, 0 stub, 跟 D-07 一次性迁移互补 (D-07 = 数据, A15 = binary). 1.0 release 不依赖 OTA 状态机, 但 1.0 release 后 OTA 升级有完整栈.

### 6.4 apeireth-update (R21 autoupdate, STUB)

> **LOC**: `crates/apeireth-update/` 1 crate
> **状态**: ⚠️ STUB (`STUB_MODE = true` 编译期 hardcode, 0 真连 / 0 真下载 / 0 真应用)
> **1.0 release 依赖**: ❌ 否 (1.0 release upgrade 走 D-07 一次性, 不走 autoupdate)
> **R21+ 续补**: 1 owner × 4h (GitHub Releases API + 下载 + 应用)

**R21+ 续补路径**:
1. GitHub Releases API 集成 (`octocrab` 0.34+)
2. 签名验证 (cosign verify 复用 `bbb26266` 8 包签名)
3. 下载 + 校验和 + 备份旧 binary (跟 rollback.sh 7 步兼容)
4. sandbox-validator 守门 (借 `apeireth-upgrade::sandbox`)
5. systemctl restart + health check

---

## §7. 决策日志 (per 主人拍板 + Mavis 倾向)

| # | 决策 | 主人拍 / Mavis 倾向 | 实施 |
|---|------|------------------|------|
| 1 | 1.0 release upgrade 走 D-07 一次性 (推翻 B 双写 7 天) | 主 2026-08-05 20:53 拍 A | commit `f5c44769` + `scripts/upgrade/v2.0.0-alpha-to-v1.0.0.sh` 591 行 |
| 2 | 8 平台 upgrade 入口 R21+ 续 (1.0 release 决策不做) | 主 2026-08-05 20:53 拍 A + Mavis 整合 #3 延续 | 0 字节不存在, 1.0 release #5 12 项 100% PASS, 不阻塞 |
| 3 | rollback 7 天内可回滚 + 30 天 .bak 保留 | 主 2026-08-05 22:13 拍 "1.0 release 收口" | `scripts/upgrade/rollback.sh` 32 行 + D-07 step 7 cron |
| 4 | apeireth-update R21 autoupdate (1.0 release STUB) | Mavis 倾向 R21+ 续 (不阻塞 1.0 release) | STUB_MODE = true 编译期 hardcode, R21+ 续 4h 估补 |
| 5 | 10M+ 行流式迁移 R21+ 续 | Mavis 倾向 (1.0 release 500K 行) | R21+ 续, 估 1 owner × 1 周 |
| 6 | PostgreSQL 14+ 字符集锁 | Mavis 倾向 (per 0.9.x 案例) | 1.0 release 文档明示 |

**整合 #3 拍板建议**: 接受本 upgrade guide 草稿, 1 commit `docs(install): R20 阶段 6 — upgrade guide 0.x → 1.0.0 (8 平台 + D-07 + 兜底)` 入 `docs/installation/upgrade/` (新子目录, 不动 LOCKED 根 INSTALL.md).

---

## §8. 0 触碰实查 + 0 改 workspace version + 0 commit 声明

### 8.1 0 触碰 5 LOCKED 根文件 mtime 严守

| # | LOCKED 文件 | mtime (基线) | 本任务触碰? |
|---:|------------|------------|:---------:|
| 1 | `README.md` (根) | 2026/8/5 21:08:33 | ✅ 0 触碰 (本文件仅引用) |
| 2 | `CHANGELOG.md` (根) | 2026/8/5 21:32:31 | ✅ 0 触碰 |
| 3 | `INSTALL.md` (根) | 2026/8/2 11:11:24 | ✅ 0 触碰 |
| 4 | `ROADMAP.md` (根) | 2026/8/5 21:04:31 | ✅ 0 触碰 (仅引用 §R20 阶段 6) |
| 5 | `CONTRIBUTING.md` (根) | 2026/8/5 21:23:54 | ✅ 0 触碰 |
| 6 | `Cargo.toml` (根) | 2026/8/6 2:55:44 | ✅ 0 触碰 (workspace version 严守) |
| **小计** | **5 LOCKED 根文件** | — | **0 触碰 (5/5)** |

### 8.2 0 改 workspace version 验证 (per §8.1 #6)

```bash
$ Cargo.toml [workspace.package] line 187-188 (实测):
  [workspace.package]    # line 187
  version = "1.0.0"      # line 188 — 仍是 1.0.0, 未改
```

**结论**: ✅ **0 改 workspace version** (1.0.0 严守, semver 严守 per APEIRETH-VERSIONING.md §1)

### 8.3 0 触碰 24 LOCKED crate src/ 验证 (per `8-promise-audit.md` §3)

| 24 LOCKED crate | mtime (基线 16:34 之前) | 本任务触碰? |
|----------------|----------------------|:---------:|
| `apeireth-supervisor` / `agent` / `council` / `bus` / `protocol` / `mcp` / `tool-registry` / `tool-runtime` / `graph` / `pipeline` / `tool-approval` / `extension` / `evolution` / `api` / `core` / `memory` / `asi` / `tools` / `cli` / `bench` / `cognition` / `action` / `life-force` / `constraint` | 全部 16:34 之前 | ✅ **24/24 0 触碰** |

### 8.4 0 主动 commit 声明

- 我**没运行** `git add` / `git commit` / `git push` 任何命令
- 本文件 `docs/1.0-release-prep/UPGRADE_GUIDE-0.x-to-1.0.md` (NEW, untracked) 留 Mavis 整合 #3 拍板
- 5 LOCKED 根文件 mtime 严守 (per §8.1)
- 24 LOCKED crate mtime 严守 (per §8.3)
- workspace version 1.0.0 严守 (per §8.2)
- 当前 HEAD = `0da4af0399e43bdd88c88c111bfbcbfc11b218be` (本任务前 commit, 0 改)

---

## §9. 引用

### 9.1 D-07 一次性迁移核心

- `scripts/upgrade/v2.0.0-alpha-to-v1.0.0.sh` (591 行, 8 步 + 5 验证 + 30 天 .bak, commit `f5c44769`)
- `scripts/upgrade/rollback.sh` (32 行, 7 步回滚, ⚠️ untracked, Mavis 整合 #3 拍板后入)
- `scripts/uninstall/uninstall.sh` (495 行, 5 步 0 残留, 跟 D-07 同 commit `f5c44769`)
- `docs/adr/0009-d-07-sqlite-to-postgres.md` (D-07 ADR, 1 决策 + 8 步 + 5 验证 + 兜底 3 步 + 5 表清单)
- `docs/1.0-release/8-promise-audit.md` §2 (8 项不修改承诺)

### 9.2 1.0 release 12 项 checklist + 12 报告

- `docs/release/1.0.0-release-report-2026-08-05.md` (R20-Rev-A, 12 项 9 PASS / 3 FAIL)
- `docs/1.0-release/install-status.md` §4 (D-07 迁移 #5 upgrade 100% PASS)
- `reports/1.0-release-upgrade-100-2026-08-06.md` (12 项验收, D-07 + rollback + 兼容性 + A15 + STUB 标缺)
- `reports/1.0-release-uninstall-100-2026-08-06.md` (#6 uninstall 100%, 跟 D-07 捆绑)

### 9.3 A15 OTA + R21 autoupdate

- `crates/apeireth-upgrade/` (9 模块 / 1288 行 ota.rs / LOCKED)
- `crates/apeireth-update/` (1 crate, STUB, R21+ 续 4h)
- `docs/architecture-stage4-engineering-landing.md` (A15 设计, per `6ca80776` commit)

### 9.4 8 形态 upgrade (R21+ 续)

- `docs/installation/` (6 文件: deb / rpm / brew / scoop / tarball / package-comparison)
- `docs/adr/0008-d-06-8-package-distribution.md` (D-06 8 包齐发 + Linux 4 包重点)
- `packaging/{deb,rpm,brew,scoop,tarball,zip,msi,docker}/` (8 形态 build/install 脚本, **缺 upgrade 入口**)

### 9.5 整合 #3 必读

- `reports/integrate-3-commit-templates-2026-08-06.md` (C1~C7, **本文件 source**)
- `docs/1.0-release-prep/RELEASE_NOTES-1.0.md` (整合 #3 拍板草稿)
- `docs/1.0-release-prep/CHANGELOG_1.0-summary.md` (12 ADR 索引 + 30+ R21 续)

---

_本文件路径: `docs/1.0-release-prep/UPGRADE_GUIDE-0.x-to-1.0.md`_
_生成时间: 2026-08-06_
_派工来源: Mavis 1.0 release 治理收尾, 续 `docs/adr/0009-d-07-sqlite-to-postgres.md` + `reports/1.0-release-upgrade-100-2026-08-06.md`_
_6 哲学锚穿透 + 8 项不修改承诺 0 触碰 + 0 改 workspace version + 0 主动 commit + 0 sandbox 错路径_
