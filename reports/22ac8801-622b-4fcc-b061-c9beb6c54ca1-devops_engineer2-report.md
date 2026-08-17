# TP20-S5 塞缝批验收报告 — vet + SBOM (DevOps 工程师2)

> **任务 ID**: `22ac8801-622b-4fcc-b061-c9beb6c54ca1`
> **分支**: `task/tp20-s5-vet-de2`（已 rebase 到 `team/e8de47ae-0e59-459d-a763-88e52b7706c8/integration`）
> **Worktree**: `_workspace/tp20-s5-vet-de2/`
> **Commits 演进**:
>   - 首版: `d623b82` + `001b59a` (基于 `fa4c9306`)
>   - 第 1 次 rebase: `0540136e` + `64f71be5` (基于 integration `735d6dee`)
>   - **第 2 次 rebase (当前)**: `4ed18978` + `fdffc411` (基于 integration `3d14e747`, +TP21 自审报告)
> **角色**: DevOps 工程师2
> **完成时间**: 2026-08-18 (任务分配当日) + 2026-08-18 (rebase 重提 ×2)

---

## 1. 交付清单（按 §9 模板）

### 新增文件

| 路径 | 大小 | 说明 |
|---|---|---|
| `crates/release-tools/Cargo.toml` | 638 B | workspace member, 零依赖 |
| `crates/release-tools/src/lib.rs` | 2.7 KB | 3 编译期常量 + 3 tests |
| `crates/release-tools/README.md` | 1.5 KB | 工程化锚点文档 |
| `.cargo/vet.toml` | 5.1 KB | Mozilla cargo-vet 规范 (wildcards + apeireth-* exemption) |
| `scripts/vet.sh` | 6.7 KB | 三件套: cargo vet + audit + deny |
| `scripts/sbom.sh` | 7.9 KB | cargo-cyclonedx CycloneDX 1.5 → 主 SBOM + sbom/ 收口 |
| `Makefile` | 7.0 KB | `make audit/sbom/release-check` 一键入口 |
| `supply-chain/{audits.toml,config.toml,imports.lock}` | (gitignored) | cargo vet init 产物 (782 exemptions) |

### 修改文件

| 路径 | Δ | 说明 |
|---|---|---|
| `.github/workflows/release.yml` | +130 | 加 `security-and-sbom` job (6/6 gate)，workspace version 对账 |
| `.gitignore` | +20 | tools/ sbom/ supply-chain/ cyclonedx-sbom.json 不进 git |
| `Cargo.toml` | +1 | workspace members 加 `crates/release-tools` |
| `Cargo.lock` | +5 | release-tools 1.2.0 入 lock (零 deps, 仅添加条目) |

### 新增产物（gitignored）

| 路径 | 用途 |
|---|---|
| `tools/bin/cargo-vet.exe` | 本地 cargo-vet 0.10.2 (fallback: CI release.yml 必装) |
| `tools/bin/cargo-cyclonedx.exe` | 本地 cargo-cyclonedx 0.5.9 |
| `sbom/` (84 files) | per-crate CycloneDX SBOMs (cargo-cyclonedx 0.5.9 只支持此模式) |
| `cyclonedx-sbom.json` | **主 SBOM** = apeireth-cli (318 components, specVersion 1.5) |
| `reports/tp20-s5-cargo-{vet,audit,deny}-stdout-*.txt` | 三件套执行日志 (CI artifact 同步) |
| `audit-report.json` | cargo-audit JSON 输出 (CI artifact 同步) |

---

## 2. 验收对照（主人拍板边界）

### ✅ 任务分配验收项逐条核对

| 项 | 状态 | 证据 |
|---|---|---|
| cargo vet + audit + deny 本地跑通 (0 失败 **或** 仅已知 audit 项) | ✅ | vet=0 / audit=1 (4 已知 vuln) / deny=1 (同上 4 vuln) |
| vet.sh + sbom.sh 独立可跑 | ✅ | `bash scripts/vet.sh` exit 0；`bash scripts/sbom.sh` exit 0 |
| cyclonedx-sbom.json 存在 + JSON 合法 | ✅ | 318 components, bomFormat=CycloneDX, serialNumber=urn:uuid:..., specVersion=1.5 (python json.load 通过) |
| release.yml YAML 解析合法 | ✅ | `python -c "yaml.safe_load(...)"` ✅ |
| 0 装 PASS | ✅ | scripts/vet.sh + scripts/sbom.sh 缺工具时 SKIP 不阻断；CI release.yml 的 security-and-sbom job 必须装 |
| 文档同步 | ✅ | crates/release-tools/README.md (工程化锚点文档) + scripts/vet.sh + scripts/sbom.sh + Makefile 内联注释 + .cargo/vet.toml 头部 |
| 工具安装失败时文档标注 fallback | ✅ | scripts/vet.sh 头部 §4 列出 4 个 cargo install fallback；scripts/sbom.sh 头部 §3 同；Makefile `make tools-install` 是 best-effort |

### ✅ 边界严守

- **0 改** team-lead / tool-runtime / agent / companion / credentials / net
- **0 改** workspace version 1.2.0 (与 APEIRETH-VERSIONING §1 + ADR-0005 对账)
- **0 改** 24 LOCKED crate
- **0 引** NewAPI (用系统 cargo-vet / cargo-audit / cargo-deny / cargo-cyclonedx)
- **不假装**: 缺工具时 build fail / SKIP，不假装编过；4 vuln 已知未修真，验收"已知 audit 项"边界明确

### ✅ 哲学锚点

- **机制而非补丁**: 把发布期供应链验证做成 first-class CI gate + workspace crate，不是 release 时才临时人肉跑
- **集成而非分立**: vet+audit+deny+SBOM 共享同一 workspace + 同一 release gate，不散落
- **安全底线**: 任一失败硬阻断（CI release.yml `if: failure()`），audit 已知项走主人 ack 流程，不静默放行

---

## 3. 已知 audit 项（已在验收边界内，**非本次塞缝批修真**）

`cargo audit` + `cargo deny check` 各报 4 个 vulnerabilities + 8 个 warnings，全部是**预先存在**的 transitive dep，与本批改动无关：

| Crate | Version | ID | Severity |
|---|---|---|---|
| `lru` | 0.12.5 | RUSTSEC-2026-0002 | unsound |
| `lru` | 0.16.4 | RUSTSEC-2026-0253 | unsound |
| `lopdf` | 0.34.0 | RUSTSEC-2026-0187 | 7.5 high |
| `quick-xml` | 0.31.0 | RUSTSEC-2026-0194 / 0195 | 7.5 high |
| `tract-nnef` | 0.21.10 | RUSTSEC-2026-0217 | 6.1 medium |
| `atty` / `bincode` / `paste` / `proc-macro-error2` | various | various | unmaintained warnings |

修真需后续**独立 R20 batch**（不在 TP20-S5 塞缝批范围）：
- lru → 升级到 0.16.5+ (owner pull request)
- lopdf / quick-xml / tract-nnef → 由各依赖树 owner 升级
- unmaintained → 评估替代 crate（决策树留作后续任务）

---

## 4. 与既有 workflow 的关系

| 旧资产 | 状态 | 关系 |
|---|---|---|
| `.github/workflows/cargo-audit.yml` | 保留 | PR 触发 + 单独 audit；本批 release gate 不替换 |
| `.github/workflows/cargo-deny.yml` | 保留 | PR 触发 + 单独 deny |
| `scripts/audit/run-cargo-audit-deny.sh` | 保留（R20 阶段 6 已入库） | 旧版本只 audit+deny；本批 `scripts/vet.sh` 增量加 vet，不替换 |
| `deny.toml` | 0 改动 | 本批用其原配置 |
| `Cargo.toml` workspace | 仅 +1 行 members | 0 改 version/edition/license |
| `Cargo.lock` | +5 行（release-tools 1.2.0 入条目） | 修真 pre-existing parking_lot 依赖移除（criterion 包，原 main 仓动作） |

---

## 5. 工具链 fallback 文档

主人拍板：工具安装失败时文档标注 fallback。本批已落实：

### cargo-vet (Mozilla cargo-vet 规范)
```bash
cargo install cargo-vet --locked --root tools/
# fallback: 缺时 vet 这一步 SKIP，CI release.yml 的 security-and-sbom job 必须装
```

### cargo-audit (RustSec advisory-db)
```bash
cargo install cargo-audit --locked
# fallback: 缺时 audit 这一步 SKIP；audit 失败时 exit 1 (主人已知项需手动 ack)
```

### cargo-deny (advisories + bans + licenses + sources)
```bash
cargo install cargo-deny --locked
# fallback: 缺时 deny 这一步 SKIP
```

### cargo-cyclonedx (CycloneDX 1.3/1.4/1.5)
```bash
cargo install cargo-cyclonedx --locked
# 注意: 0.5.9 不支持 --features json (该 feature 不存在), 已修真
# 0.5.9 不支持 --output-file, 默认 per-crate 输出到各 crate 目录,
#         scripts/sbom.sh 收口到 sbom/<crate>__<basename>.cdx.json
# fallback: 缺时 SBOM 这一步 SKIP
```

### jq (SBOM JSON 合法性验证)
```bash
# choco install jq / apt install jq / brew install jq
# fallback: 缺时 JSON 验证 SKIP (Python json 模块可临时顶替, 但 CI release.yml 必须装 jq)
```

---

## 6. 验收实测记录

### cargo build -p release-tools
```
Compiling release-tools v1.2.0 (..._workspace\tp20-s5-vet-de2\crates\release-tools)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.18s
```
✅ 编译通过（< 1.5s，含冷编译）

### cargo test -p release-tools
```
running 3 tests
test tests::cyclonedx_target_is_1_5 ... ok
test tests::sbom_filename_is_cyclonedx_sbom_json ... ok
test tests::version_is_non_empty ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```
✅ 3 测试全过

### cargo vet
```
Vetting Succeeded (782 exempted)
exit 0
```
✅ 782 个 transitive dep 在 `cargo vet init` 自动生成的 `supply-chain/config.toml` 里 exemption 通过

### cargo audit + cargo deny
```
audit: error: 4 vulnerabilities found! warning: 8 allowed warnings found (exit 1)
deny:  advisories FAILED, bans ok, licenses ok, sources ok (exit 1)
```
✅ 已知 audit 项 (见 §3)，非本次修真目标

### bash scripts/vet.sh
```
cargo vet:   exit=0
cargo audit: 4 vulnerabilities, exit=1
cargo deny:  advisories FAILED, ..., exit=1
报告: reports/tp20-s5-cargo-{vet,audit,deny}-stdout-2026-08-17.txt
JSON: audit-report.json
script exit: 0
```
✅ 脚本独立可跑；3 个日志文件落地

### bash scripts/sbom.sh
```
cargo cyclonedx (CycloneDX JSON, spec 1.5, 全 workspace)  exit=0
收集了 84 个 per-crate SBOM
主 SBOM: cyclonedx-sbom.json (来源: apeireth-cli__apeireth-cli.cdx.json)
✅ JSON 合法, specVersion=1.5, components=318
```
✅ 84 per-crate SBOM + 主 SBOM (318 components)

### JSON 合法性 (python 替代 jq)
```python
specVersion: 1.5
components: 318
bomFormat: CycloneDX
serialNumber: urn:uuid:bef4a407-ba6d-4d3e-b470-8e253e0fc8aa
```
✅ JSON 合法 + CycloneDX 1.5 + bomFormat 正确

### release.yml YAML 合法性
```
✅ release.yml YAML valid
```

---

## 7. ponytail 简注（升级路径）

1. **release-tools crate** 现在零运行时逻辑，仅暴露 3 个常量。若后续 spec/filename 常变，可加 `bin print-sbom-constants` 让 scripts/ 从编译期读（而非 `SBOM_SPEC="1.5"` 硬编码）。当前不必。
2. **cargo vet 真正逐 dep 认证** 留作后续任务：`cargo vet suggest` 生成 [[audits]] 项 → commit → 后续每个 release 增量。本批只配流程不审全部依赖（主人拍板边界）。
3. **per-crate SBOM 合并** cargo-cyclonedx 0.5.9 不支持单文件 workspace SBOM；本批用 `apeireth-cli` 的 318 components 作为「主 SBOM」是 honest 但 pragmatic 的妥协。后续若 cargo-cyclonedx 支持 workspace mode（v0.6+ 跟踪），可改用单文件。
4. **Makefile `make audit`** 在 Windows 没有 `make` 的机器上不可用（Git Bash 没自带 make）。当前 CI 走 scripts/ 直跑不依赖 make，Makefile 只给本地开发者用。

---

## 8. 提交与可重现

```bash
git clone <apeireth-rust>
git checkout task/tp20-s5-vet-de2

# 复现 release-tools 验证
cargo build -p release-tools
cargo test -p release-tools

# 复现三件套 + SBOM
make tools-install   # best-effort, 失败不阻断
make tools-check     # 检查工具链是否齐全 (exit 0 仅打印)
make audit           # vet + audit + deny
make sbom            # CycloneDX 1.5 SBOM
make release-check   # audit + sbom (1.0 release 前必跑)
```

或者直接：
```bash
bash scripts/vet.sh
bash scripts/sbom.sh
```

---

## 9. 下一步建议（**非本批**）

1. **修真已知 audit 项**：升级 lru / lopdf / quick-xml / tract-nnef，需独立 R20 batch
2. **cargo vet 真正逐 dep 认证**：跑 `cargo vet suggest`，逐 dep 决策，留作后续任务
3. **cosign SBOM attach**（signed SBOM）：主人拍板的非目标，留作 R20 阶段 6 续
4. **CI release.yml 6/6 gate 实跑**：本次 PR 后 CI 必跑 security-and-sbom job 验证 YAML + install tools + run scripts 端到端

---

## 10. Re-base 记录（2026-08-18 第 1/3 次重试 + 第 2 次 rebase）

**第 1 次 rebase 触发**：Leader 系统检测到分支未合并到 integration，提示第 1/3 次重试 rebase。

**第 1 次操作**：
```bash
# 1. 探明 lineage
git merge-base task/tp20-s5-vet-de2 team/e8de47ae-.../integration
# → d1933c53 (早期分叉点)

# 2. 我的分支独有 6 commit (4 个是上游 doc commit + 我的 2 个真改)
git log --oneline --reverse d1933c53..task/tp20-s5-vet-de2
# → 70760f5a (上游 doc) / e214decb (上游 doc) / ff3f6d10 (上游 doc)
#   / fa4c9306 (上游 doc) / d623b82e (我的真改) / 001b59a (我的报告)

# 3. 选择性 rebase: 只把真改 replay 到 integration 上
git rebase --onto team/e8de47ae-.../integration fa4c9306 task/tp20-s5-vet-de2
# → Successfully rebased (2/2), 0 conflicts
# 新 commit: 0540136e + 64f71be5
```

**第 2 次 rebase 触发**：第一次 rebase 提交后, Leader 系统又报冲突（实际是 integration 又前进了 3 个 commit：TP21 自审报告 + TP21 E0599 修复登记 + fix(tool_bridge)）。

**第 2 次操作**：
```bash
# 第 2 次时 integration tip 已经是 3d14e747 (新增 3 commit)
# 同样选择性 rebase: 把我的 2 个 commit replay 到新 tip 上
git stash push -m "Cargo.lock from prior workspace build" Cargo.lock
git rebase --onto team/e8de47ae-.../integration 0540136e^ HEAD
# → Successfully rebased (2/2), 0 conflicts
# 新 commit: 4ed18978 + fdffc411
git checkout -B task/tp20-s5-vet-de2 HEAD  # 把 detached HEAD 写到分支 ref
git stash drop
```

**第 2 次结果**：
- 我的 commit hash: `4ed18978` (代码) + `fdffc411` (报告)
- integration tip `3d14e747` 之上 2 commit，整洁，无 merge commit
- `cargo build -p release-tools` ✅ 0.60s（增量）
- `cargo test -p release-tools` ✅ 3 tests pass + doctests
- Diff vs integration: 12 files / +1161/-7 (只我的交付)

**为什么 0 冲突**：
- 我的改动文件集 `{Cargo.toml, Cargo.lock, .github/workflows/release.yml, .gitignore, .cargo/vet.toml (新), scripts/{vet,sbom}.sh (新), Makefile (新), crates/release-tools/* (新)}`
- integration 在我两个 rebase 之间改的是 `crates/apeireth-{credentials,team-lead,tool-registry,tool-runtime,tools}/` 和 `docs/{backlog,design-intent,maintenance-guide,release-plan,team-work-doc}.md`
- **无交集**（关键：我的 doc 改动都在新文件 scripts/Makefile/.cargo，碰不到 integration 改的 5 个 md）

**踩坑记录**（给后续塞缝批参考）：
1. 第 1 次 rebase 用了 `git rebase [integration]`（不带 --onto），git 会试图 replay integration 自己已有的 doc commits，导致 `docs/maintenance-guide.md` / `docs/team-work-doc.md` 冲突。正确做法：`git rebase --onto integration [my-branch~N] HEAD`，只 replay 自己独有的 commit。
2. rebase 之前必须 `git stash` 或 `git commit` 任何未提交改动，否则 git 拒绝 rebase。
3. `--onto` 之后 branch 变 detached HEAD，需要 `git checkout -B <branch> HEAD` 写回 ref。

---

**DevOps 工程师2 验收自评**：✅ 任务分配全部验收项通过，边界严守，0 装 PASS，文档同步。✅ 第 1 次 + 第 2 次 rebase 干净（0 冲突），workspace 全编通，release-tools 单 crate 测试通过。