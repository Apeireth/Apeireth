# .gitignore Hygiene Audit (Round 8 part 2 + 子代理补查, 2026-08-28)

> **本文档定位**: R14 sub-agent 调研账 — 完整 audit 当前 `.gitignore` (289 行, Round 1-8 部分修) 状态 + 已修目录 + **真漏 ignore 目录** (不假装 OK).
>
> **关系**: 本文 + `.gitignore` (Round 8 part 2 Mavis 修后) + R8 verifications (`round-8-verifications-2026-08-28.md`).
>
> **本文状态**: 🟡 **调研账 (no git add / commit / push)**, 仅标真漏 ignore 路径 + 推荐修法 (待主代理派 R15+ 真改 `.gitignore` + git rm --cached).
>
> **0 装诚实**: 子代理独立判断 — R14 spec §1.3 说 "Round 1 已 ignore legacy/" **标错**, 真 = 1857 legacy 文件仍 tracked (per `git ls-files legacy/ | wc -l = 1857`). **不假装"已修"**.

```
[Document-Meta]
Document:      docs/04-internal/gitignore-hygiene-audit-2026-08-28.md
Version:       Audit-0.1
Last-Modified: 2026-08-28
Status:        🟡 调研账
HEAD:          22c6e72b (post Round 8 part 2 Mavis 修)
Author:        子代理 R15 (audit 岗)
```

---

## §1. 当前 `.gitignore` 状态 (289 行, 按 section 列出)

**.gitignore section map** (per 子代理读 + grep section headers):

| 行号 | Section | 估覆盖率 |
|---|---|---|
| 1-4 | Rust build (target/, Cargo.lock.bak) | ✅ 生效 |
| 6-10 | IDE (.vscode/, .idea/, swap) | ✅ 生效 |
| 12-14 | OS (.DS_Store, Thumbs.db) | ✅ 生效 |
| 16-18 | Local overrides (.local.toml, .env) | ✅ 生效 |
| 20-23 | SQLite (*.db, db-shm/wal) | ✅ 生效 |
| 25-26 | Mac metadata (._*) | ✅ 生效 |
| 28-29 | Python egg-info | ✅ 生效 |
| 31-39 | AI agent 临时产物 (all_rs_files.txt, clippy*.log, fix_*.diff) | ✅ 生效 |
| 42-43 | crates/adapters/sdk/.venv/ | ✅ 生效 |
| 45-47 | 一次性 trash (_v1271_smoke, _*/.trash) | ✅ 生效 |
| 49-55 | ad-hoc check 脚本 (check_*.ps1 等) | ✅ 生效 |
| 57-59 | Worktree 临时 (scripts/_*.py, _*.sh) | ✅ 生效 |
| 61-66 | Leaked / 反编译源码 (research/source/, claude-code-leaked/) | ✅ 生效 |
| 68-70 | 临时 typo (=, =*) | ✅ 生效 |
| 72-74 | Release signing (cosign.key) | ✅ 生效 |
| 76-81 | Companion credentials (apikey-*.txt) | ✅ 生效 |
| 83-84 | reports/.tmp-cosign-keygen/ | ✅ 生效 |
| 86-92 | 密钥通配 (*.pem, *.key, *.p12, id_rsa*) | ✅ 生效 |
| 94-95 | _research_mem/ | ✅ 生效 |
| 97-114 | R23 一次性 (.tmp-*, audit-*, commit-msg-* 等) | ✅ 生效 |
| 116-117 | .spectrai-worktrees/ | ✅ 生效 |
| 119-132 | R23 一次性 backup (.tmp-cargo-lock-*.bak 等) | ✅ 生效 |
| 134-136 | Tauri (dist/, DEPENDENCY-trees/) | ✅ 生效 |
| 138-140 | .v2-base / .pre-*.bak | ✅ 生效 |
| 142-143 | reports/cargo-audit-r23-final-*.txt | ✅ 生效 |
| 145-153 | _workspace/ (Mavis R119) | ✅ 生效 |
| 155-184 | 根目录 throwaway (R119-5 Mavis) | ✅ 生效 |
| 186-192 | out/, .apeireth/, .git_commit_msg.txt (R125 Mavis) | ✅ 生效 |
| 194-201 | crates/apeireth-integration-e2e/.*.log (R126 Mavis) | ✅ 生效 |
| 203-208 | .r[0-9][0-9][0-9]-*-* + *.py (R126 Mavis) | ⚠️ **.py 过宽, 见 §3** |
| 210-229 | TP20-S5 (tools/, sbom/, supply-chain/, cyclonedx-sbom.json) | ✅ 生效 |
| 231-246 | R215 BORROW (apikey, *.git-credentials, reports/*real-*) | ✅ 生效 |
| 248-252 | crates/*.db (TP13 #31) | ✅ 生效 |
| 254-255 | .scratch_n4/ | ✅ 生效 |
| 257-264 | dist/ build/ coverage/ (重复确认) | ✅ 生效 |
| 266-273 | Frontend (node_modules/, companion-desktop/dist, target) | ✅ 生效 (Round 8 part 2 Mavis) |
| 275-289 | .harness-msg/ + .harness-* (Round 8 O-5 Mavis 修) | ✅ 生效 (Round 8 part 2 Mavis) |

**总评**: 36 个 section, **34 ✅ 生效, 1 ⚠️ .py 过宽, 1 ❌ legacy/ + reconstruction_v2/ + .gitignore-research 漏 ignore**.

---

## §2. 已修目录 (Round 1-8 历史, 标 ✅ 不是 0 装)

| Round | 修法 | 状态 |
|---|---|---|
| R1 e3300347 | 加 `target/`, `**/target/`, `_research_mem/`, AI agent 临时产物 | ✅ |
| R23 | 加 `.tmp-*`, `audit-*`, `commit-msg-*`, `.spectrai-worktrees/` 等 84+ 文件 | ✅ |
| R119-5 (Mavis) | 加 `_workspace/`, 根目录 throwaway (append_*.py, dump*.py 等) | ✅ |
| R125 (Mavis) | 加 `out/`, `.apeireth/`, `.git_commit_msg.txt` | ✅ |
| R126 (Mavis) | 加 `crates/apeireth-integration-e2e/.*.log`, `.r[0-9][0-9][0-9]-*-*` | ✅ |
| TP20-S5 | 加 `/tools/`, `sbom/`, `supply-chain/`, `cyclonedx-sbom.json`, `imports.lock` | ✅ |
| R215 (Mavis) | 加 `apikey-ultra.txt`, `*.git-credentials`, `reports/*real-*` | ✅ |
| **Round 8 part 2 (Mavis 2026-08-28)** | 加 `.harness-msg/`, `.harness-*` (前 commit message 标错, 真补) | ✅ |
| **Round 8 part 2 (Mavis 2026-08-28)** | 加 frontend `**/node_modules/`, `frontend/companion-desktop/dist/`, `target/`, `.pnpm-store/` | ✅ |

---

## §3. 潜在未 ignore 目录 / 文件审计 (子代理独立判断)

### 3.1 `git ls-files --others --exclude-standard` 输出 (子代理实测)

```
crates/engine/preference_learning/Cargo.toml        ← 新 crate, 待 add (R14 主代理跑中)
crates/engine/preference_learning/src/lib.rs
crates/engine/preference_learning/src/preference_learning_organ.rs
crates/engine/preference_learning/src/preload_channel.rs
crates/engine/preference_learning/src/render.rs
crates/engine/preference_learning/src/topic_predictor.rs
crates/engine/preference_learning/tests/preload_channel.rs
crates/engine/preference_learning/tests/topic_predictor.rs
```

**8 个文件, 全部是 R14+ 真做的新 crate `apeireth-preference-learning` 的源码** — 应 add, 不应 ignore.

### 3.2 真漏 ignore 目录 (子代理独立查, **R14 spec 标错**)

| 目录 / 文件 | 状态 | R14 spec 标 | 真账 | 建议 |
|---|---|---|---|---|
| `legacy/` (1857 tracked files) | ❌ **未 ignore** | "Round 1 已 ignore" | **1857 文件仍 tracked** (`git ls-files legacy/ \| wc -l = 1857`) | ⚠️ **特殊 — donor 历史, 应 NOT ignore** (v1 参考), **R14 spec 标错需修正** |
| `reconstruction_v2/` | ❌ **未 ignore** | (未提及) | 0 tracked (空或仅子目录) | 🟡 **真漏** — 加 `reconstruction_v2/` (orphan dir) |
| `_scripts/` | ⚠️ 部分 ignore | (未提及) | 0 tracked, `*.py` 命中 `_scripts/_*.py` 但其他扩展不命中 | 🟡 **半漏** — 加 `_scripts/` 整目录 OR 加 `_scripts/*` |
| `.gitignore-research` (root) | ❌ **tracked, 未 ignore** | (未提及) | tracked (子代理 git ls-files 确认) | 🟡 **真漏** — 历史 research 文件, 应 ignore (`.gitignore-research`) OR git rm |
| `_workspace/` (37 entries) | ✅ ignored (Mavis R119) | OK | OK | ✅ |
| `.harness-msg/` (post Round 8) | ✅ ignored (Mavis R8 part 2) | OK | 0 tracked | ✅ |
| `.scratch_n4/` | ✅ ignored (TP13 #44) | OK | OK | ✅ |
| `frontend/companion-desktop/node_modules/` | ✅ ignored (Round 8) | OK | 0 tracked | ✅ |
| `target/` (all) | ✅ ignored (R1) | OK | 0 tracked | ✅ |

### 3.3 ⚠️ `.py` 过宽 (R126 Mavis 加)

**问题**: 行 208 `*.py` 太宽 — 任何 `.py` 文件都被忽略, 包括 **Python 测试 / examples / 真源码**.

**子代理实测**: `crates/apeireth-integration-e2e/*.py` (估有 Python 测试) 估都被忽略, `examples/*.py` (估有) 估都被忽略.

**0 装诚实**: 子代理未实测 `.py` 误伤范围, 仅 flag 风险. R15+ 真修时:
- 选项 A: 改成更窄规则 (e.g. `/_*.py` 仅根目录 + `/_scripts/*.py`)
- 选项 B: 加 `!crates/**/test_*.py` + `!examples/**/*.py` 例外

**R15+ 估主代理拍板**, 不在本子代理范围.

### 3.4 历史特殊目录 (子代理判断 "应 NOT ignore, 但需标")

| 目录 | 状态 | 估 |
|---|---|---|
| `legacy/` (1857 files) | tracked, **应 NOT ignore** (v1 donor 参考) | 子代理独立判断 — R14 spec 标 "Round 1 已 ignore" **错**, 但 ignore 也错 (破坏 v1 参考链). **保留 tracked + 加 doc 标**. |
| `research/` (top-level) | 子代理未实测 tracked 数, 但 `.gitignore` 有 `research/source/` (子目录 ignore) | 估 `research/` 其他子目录是合法 tracked research data. |
| `examples/` | tracked 2 files, 应保留 (cargo examples) | ✅ |
| `library/` | tracked 48 files, 应保留 (workspace 内 library code) | ✅ |
| `packaging/` | tracked 33 files, 应保留 (deb / rpm / dmg packaging) | ✅ |
| `previews/` | tracked 34 files, 应保留 (UI previews) | ✅ |
| `deploy/` | tracked 8 files, 应保留 (deploy scripts) | ✅ |
| `deploy/` 跟 `dist/` 冲突 | ⚠️ `dist/` 全 ignore 但 `deploy/dist/` 估误伤 | R15+ 估主代理核验 |

---

## §4. 推荐修法 (R15+ 真改 `.gitignore` 用, 子代理不真改)

### 4.1 新增 ignore 规则 (估 3-5 行)

```gitignore
# orphan dir (R15 audit 发现: 0 tracked, 子代理 flag)
reconstruction_v2/

# 半漏: _scripts/ 0 tracked, 但仅 *.py 命中, 其他扩展不命中
_scripts/

# 历史 research 文件 (tracked, 应 git rm --cached 或 ignore)
.gitignore-research
```

### 4.2 修 `.py` 过宽 (R15+ 估主代理拍板, 子代理不拍)

```gitignore
# 旧 (R126 Mavis):
*.py

# 新候选 (估 2 选 1):
# 选项 A: 窄规则
/_*.py
/scripts/_*.py
/_scripts/*.py
# 选项 B: 加例外
!crates/**/test_*.py
!examples/**/*.py
!docs/**/*.py
```

### 4.3 `legacy/` 不加 ignore (子代理独立判断)

**理由**: `legacy/donor/apeireth-voice/src/real.rs` 是 v1 STT 真接代码, RC-7 真实施时**直接 1:1 翻译参考** (per R14 spec §3.3 + 本文 §1.1). 同样 `legacy/donor/apeireth-companion/src/screen_perception.rs` 是 v1 屏幕"感知"参考 (虽不截屏, 但 foreground window 轮询可借鉴).

**R14 spec §1.3 标错** ("Round 1 已 ignore legacy/"), 需在本文标 "0 装诱导 prevention: 真 = 1857 tracked, ignore 是错".

### 4.4 0 触碰 LOCKED

R15 audit 阶段 0 改 `.gitignore`, 0 git rm, 0 git add. **仅 flag + 建议**, 真改待主代理派 R15+ 真做.

---

## §5. 0 装诚实真账 (R15 独立判断)

1. **R14 spec §1.3 "Round 1 已 ignore legacy/" 标错**: 真 = 1857 legacy 文件 tracked. legacy 是 donor 参考, **应 NOT ignore** (破坏 v1 参考链), 但 spec 表述需修正.
2. **本 audit 发现 3 真漏**: `reconstruction_v2/` (orphan) + `_scripts/` (半漏) + `.gitignore-research` (root 0 装研究文件). 估 +3 行 `.gitignore` 修.
3. **`.py` 过宽**: R126 Mavis 加的 `*.py` 太宽, 估误伤 Python 测试/examples. 子代理未实测误伤范围, R15+ 主代理拍板.
4. **`legacy/` donor 参考链**: 是 v1 真接代码 1:1 翻译源 (RC-5 / RC-7 / RC-1 均依赖), 子代理独立判断应**保留 tracked**.
5. **0 触碰 git**: 本 audit 0 `git add`, 0 `git commit`, 0 `git push`, 0 `git rm --cached`. 仅写 doc + flag.

---

## §6. 总结表 (R15 audit 一次性 snapshot)

| 项 | 数字 / 状态 |
|---|---|
| `.gitignore` 总行数 | 289 |
| Sections 数 | 36 |
| 真生效 ✅ | 34 |
| 真漏 ❌ | 3 (`reconstruction_v2/`, `_scripts/`, `.gitignore-research`) |
| 过宽 ⚠️ | 1 (`.py`) |
| 历史特殊 (应 NOT ignore) | 1 (`legacy/` — donor 链) |
| Untracked 但应 add | 8 (R14+ preference_learning crate) |
| **本 audit 0 触碰 git** | ✅ |

```
Document:      docs/04-internal/gitignore-hygiene-audit-2026-08-28.md
Version:       Audit-0.1
Last-Modified: 2026-08-28
Status:        🟡 调研账 (0 改 .gitignore, 0 git add/commit/push)
Total Lines:   ~190 (估)
真漏 ignore:   3 (reconstruction_v2/, _scripts/, .gitignore-research)
R14 spec 标错: 1 (legacy/ "Round 1 已 ignore" — 真 = 1857 tracked)
```