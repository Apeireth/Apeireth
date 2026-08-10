# R20 阶段 6 — cargo audit + cargo deny 扫描报告

**Date**: 2026-08-05
**Stage**: R20 阶段 6 (1.0 release 12 项 checklist #3 security 实施)
**触发**: 主人 2026-08-05 21:18 拍板"真派"
**HEAD**: `deb78518` (mem 21:50 cron tick 之前; 本 commit 在 `code_reviewer/t15-fix-rebase` worktree)
**工具链**: `cargo-audit 0.22.2` (新装, 17min 网络慢), `cargo-deny 0.20.2` (已装)

---

## TL;DR — 现实与期望差异

| 维度 | 任务期望 | 实际扫描 | 决策 |
|---|---|---|---|
| vulnerabilities | 0 | **4 (pyo3 ×2 + quick-xml ×2)** | 诚实报告, 留 R21 收敛 |
| unsound | 0 | **5 (git2 ×3 + lru ×1)** | 诚实报告, 留 R21 收敛 |
| unmaintained | 0 | **3 (bincode + paste + proc-macro-error2)** | warning 级, 业界共识 transitive, 不修 |
| yanked | 0 | **0** | ✓ 与期望一致 |
| packages | 567 (估) | **626** | 主人估错, 实际 626 = 56 internal + 570 external |
| deny.toml | 新建 50 行 | **已存在 153 行** (R19 第 0 阶段第 2 项) | **不重建**, 避免覆盖 R19 决策 |
| bans (多版本) | 0 冲突 | **1 conflict** (tokio-tungstenite 0.24 vs 0.25) | apeireth-bus 引 0.25, axum 0.24, 留 R21 收敛 |
| licenses | 0 violation | **ok** ✓ | 16 类 license allow 列表覆盖实际 15 类 |
| sources | 0 violation | **ok** ✓ | 只允许 crates.io |
| 24 LOCKED 0 触碰 | 0 触碰 | **0 触碰** (实查) | ✓ 本 commit 完全不动 |
| 0 改 workspace version | 0 改 | **0 改** (Cargo.toml 不动) | ✓ |
| NewAPI 0 依赖 | 0 | **0** (8-promise-audit.sh 已验) | ✓ |

---

## 1. 工具链体检

| 工具 | 版本 | 状态 |
|---|---|---|
| cargo | 1.97.1 (c980f4866 2026-06-30) | 系统已装 |
| cargo-audit | 0.22.2 | **本任务新装** (`cargo install --locked`, 17m01s) |
| cargo-deny | 0.20.2 | 系统已装 (R18 round-00 装) |
| RustSec advisory-db | 6d7aef354b4144c1ede046034adfd00246d3b0c0 (2026-08-04) | git clone (空 advisory-db 重新拉) |
| deny.toml | 153 行 | **已存在** (R19 第 0 阶段第 2 项配置, **不重建**) |

**install 阻碍**:
- 首次 `cargo install cargo-audit --locked` 失败: `transfer too slow` (网络限速)
- 重试 1: 17m01s 编译 + 安装成功
- `advisory-db` 目录有 `db.lock` (0 字节) 阻止 git clone: rename `db.lock` → `db.lock.stale` + `git clone --depth 1` 到新目录 + 原子 rename 替换

---

## 2. cargo audit 完整结果 (exit=1, 4 vulnerabilities)

### 2.1 真实 vulnerability (4)

| ID | 包 | 版本 | 类别 | 引入链 | 修复 |
|---|---|---|---|---|---|
| `RUSTSEC-2025-0020` | pyo3 | 0.22.6 | buffer overflow (`PyString::from_object`) | Cargo.lock:4206 (pyo3 0.22.6) | 升 ≥ 0.24.1 |
| `RUSTSEC-2026-0177` | pyo3 | 0.22.6 | missing `Sync` bound (thread-safety) | Cargo.lock:4206 | 升 ≥ 0.29.0 |
| `RUSTSEC-2026-0194` | quick-xml | 0.36.2 | quadratic runtime (DoS) | `apeireth-mcp-winrm v1.0.0` (5 P0 新 crate) | 升 ≥ 0.41.0 |
| `RUSTSEC-2026-0195` | quick-xml | 0.36.2 | unbounded namespace-decl alloc (memory-exhaustion DoS) | `apeireth-mcp-winrm v1.0.0` | 升 ≥ 0.41.0 |

### 2.2 unsound (5, warning 级)

| ID | 包 | 版本 | 类别 | 引入链 |
|---|---|---|---|---|
| `RUSTSEC-2026-0008` | git2 | 0.19.0 | undefined behavior (Buf struct deref) | `apeireth-rollback v1.0.0` (9 skeleton 新 crate) |
| `RUSTSEC-2026-0183` | git2 | 0.19.0 | undefined behavior (Remote::list) | `apeireth-rollback v1.0.0` |
| `RUSTSEC-2026-0184` | git2 | 0.19.0 | undefined behavior (BlameHunk signature) | `apeireth-rollback v1.0.0` |
| `RUSTSEC-2026-0002` | lru | 0.12.5 | Stacked Borrows violation (IterMut) | 多个 internal (agent/api/asi/bench/cli/cognition/team-lead/workflow) |
| (无 ID) | bincode | 1.3.3 | unmaintained (doxxing 团队停更) | 业界共识 transitive, 不修 |

### 2.3 unmaintained (3, warning 级)

- `bincode 1.3.3` (RUSTSEC-2025-0141, doxxing 停更, 业界共识)
- `paste 1.0.15` (RUSTSEC-2024-0436, dtolnay 归档, 用 `pastey` 替代)
- `proc-macro-error2 2.0.1` (RUSTSEC-2026-0173, 作者确认停更, 用 `manyhow` 替代)

**注**: cargo-deny 把 unmaintained 当 error; cargo-audit 当 warning。--deny warnings 会让 cargo-audit 失败。

### 2.4 yanked / checksum

- yanked: **0** ✓
- 100% checksum: ✓ (Cargo.lock 每行带 `checksum = "..."`)

---

## 3. cargo deny check 完整结果 (exit=3)

```
advisories FAILED, bans FAILED, licenses ok, sources ok
```

### 3.1 advisories FAILED

同 2.1 + 2.2 + 2.3 (cargo-deny 把 unmaintained 也算 advisory error)。

### 3.2 bans FAILED

```
error[duplicate]: found 2 duplicate entries for crate 'tokio-tungstenite'
  tokio-tungstenite v0.24.0
    └── axum v0.7.9 → apeireth-api v1.0.0
  tokio-tungstenite v0.25.0
    └── apeireth-bus v1.0.0 → apeireth-team-lead v1.0.0
```

**分析**: `apeireth-bus` 用了更新的 0.25 (transitive), `axum 0.7.9` 还用 0.24。R21 计划: 强制 `apeireth-bus` 用 `workspace = true` 锁 0.25, 让 axum transitive 跟 (或反之)。

### 3.3 licenses ok ✓

实际用到的 15 种 license 全部在 deny.toml 允许列表(16 类, 1 类 `MPL-2.0` 未遇到 → 触发 `license-not-encountered` warning)。

### 3.4 sources ok ✓

只允许 `https://github.com/rust-lang/crates.io-index`, 0 未知源。

### 3.5 skip 配置 warning (20+, 不致命)

`deny.toml` 早期(R19 第 0 阶段)配置的 skip 列表, 部分 crate 已收敛到单一版本(`unnecessary-skip`), 部分 crate 在新 workspace 不用了(`unmatched-skip`):

- unnecessary-skip (10): windows-core, windows-link, windows-result, windows-strings, base64, http-body, hyper, toml_edit, reqwest, rand, rand_core, darling, darling_core, darling_macro, event-listener
- unmatched-skip (8): windows_aarch64_gnullvm, windows_aarch64_msvc, windows_i686_gnu, windows_i686_msvc, windows_x86_64_gnullvm, proc-macro-crate, jni-sys, schemars
- license-not-encountered (1): MPL-2.0 (允许但实际未用)

**R21 minor**: 清理 skip 列表, 反映当前 workspace 状态。

---

## 4. 与任务期望差异 — 主动报告

| 任务原文 | 实际 | 决策 |
|---|---|---|
| 主人说"新建 deny.toml (50 行, 9 类 license)" | 已存在 153 行 (16 类 license) | **不重建**, 避免覆盖 R19 第 0 阶段第 2 项决策 |
| 主人说"0 vulnerability" | **4 vulnerability** | **诚实记录**, 留 R21 收敛 |
| 主人说"567 packages" | 626 packages | 主人估错, 已记录 |
| 主人说"V1295 VCP Rust Cargo.lock 锁文件 #16 真生产" | V1295 不存在 (有 V1296/V1297/V1298 memory audit) | 主人内部编号错, 实际 V1297 是 Cargo Feature Flag Audit, V1298 是 Cargo Workspace Lints Audit |
| 主人说"5-10min skeleton + 1 commit" | cargo install 17min (网络慢) + 5min audit/deny | 慢 1 步, 但 1 commit 落地 (skeleton 范围可控) |

---

## 5. 严守承诺 (实查)

- ✅ 0 触碰 24 LOCKED crate (本 commit 完全不修改 crates/apeireth-{supervisor,agent,council,bus,protocol,mcp,tool-registry,tool-runtime,graph,pipeline,tool-approval,extension,evolution,api,core,memory,asi,tools,cli,bench,cognition,action,life-force,constraint}/)
- ✅ 0 改 workspace version (Cargo.toml 不动)
- ✅ 0 引 NewAPI (8-promise-audit.sh 已验, 0 真依赖)
- ✅ 0 改 deny.toml (R19 第 0 阶段第 2 项决策保留)
- ✅ 0 改 Cargo.lock (cargo audit --json / cargo deny check 都不改 lock)

---

## 6. 本 commit 新增文件 (3 类)

1. **脚本** (2 个, 可重复跑):
   - `scripts/audit/run-cargo-audit-deny.sh` (90 行, bash)
   - `scripts/audit/run-cargo-audit-deny.ps1` (75 行, PowerShell)
2. **JSON 报告** (1 个):
   - `audit-report.json` (24 KB, cargo audit --json 完整输出, 含 4 vuln + 5 unsound + 3 unmaintained 详情)
3. **stdout 报告** (2 个, 团队可见):
   - `reports/r20-cargo-audit-stdout-2026-08-05.txt`
   - `reports/r20-cargo-deny-stdout-2026-08-05.txt`
4. **本报告** (1 个):
   - `reports/r20-阶段-6-cargo-audit-deny-2026-08-05.md` (本文件)

---

## 7. R21 收敛计划 (留作下次)

| 任务 | 估时 | 风险 |
|---|---|---|
| `apeireth-rollback` 升 git2 0.19.0 → 0.20.4 (修 3 unsound) | 30min | 低 (libgit2-sys ABI 一致) |
| `apeireth-mcp-winrm` 升 quick-xml 0.36.2 → 0.41.0 (修 2 vuln) | 1h | 中 (NsReader 行为变化, 需回归测试) |
| pyo3 0.22.6 → 0.24.1 (修 1 vuln) | 1h | 中 (API 收紧, 需 API audit) |
| pyo3 0.22.6 → 0.29.0 (修 Sync bound vuln) | 2h | 高 (API 重大变化) |
| lru 0.12.5 → 0.16.3 (修 1 unsound) | 30min | 低 (IterMut API 微调) |
| apeireth-bus tokio-tungstenite 0.25 → workspace = true 收敛 | 1h | 中 (需协调 axum transitive) |
| deny.toml skip 列表清理 (20+ warning) | 30min | 0 (纯配置) |
| bincode/paste/proc-macro-error2 unmaintained | skip | 业界共识 transitive, 不修 |

**总估时**: 6-7h, 建议 R21 阶段 1 一次性收敛。

---

## 8. 1.0 release 12 项 checklist #3 security 影响

per `scripts/release-1.0-checklist.sh` 第 3 项:

```bash
check 3 "security (5 守门: non-root / API key 不入 image / audit append-only / 鉴权限流 / 内部网络隔离)" "P0" check_security
```

**本任务影响**:
- `check_security` 函数当前**只检查** 5 守门文件 (Dockerfile + docker-compose.yml), 不实际跑 cargo audit
- 1.0 release tag 之前, 应该把 `cargo audit --deny warnings` 集成进 check_security
- R21 计划: `check_security` 加 `cargo audit --deny warnings` 调用, 4 vuln 修完才放 1.0 tag

---

**生成于**: 2026-08-05 22:50
**作者**: Mavis (R20 阶段 6 worker, per 主人 21:18 拍板"真派")
**Co-authored-by**: Mavis <Mavis@local>
